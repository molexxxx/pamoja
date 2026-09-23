//! The signed update guide example; see docs/guides/update.md.
//!
//! Run: `cargo run -p pamoja-examples --example update`

use std::error::Error;

/// A release signed by its publisher, staged into the spare slot on a device anchored to
/// that publisher, booted on trial and confirmed; then the releases a device has to turn
/// away, and a bad release that boots, never confirms, and is rolled back.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_security::DeviceIdentity;
    use pamoja_update::{
        image_digest, Boot, Device, Envelope, Manifest, MemoryStore, PayloadFormat, SlotState,
        SlotStore, Updater, ENVELOPE_MAX, STRUCTURE_VERSION,
    };

    // The publisher's key signs releases; devices in the field are anchored to its public
    // half and will take firmware from nobody else.
    let publisher = DeviceIdentity::from_seed(&[7u8; 32]);

    // Who the release is for. Both identifiers are sixteen bytes a vendor assigns itself,
    // and a device takes firmware only for the pair it was built as.
    const VENDOR: [u8; 16] = [10; 16];
    const FLOW_METER: [u8; 16] = [11; 16];

    // The release. A manifest says who the image is for, which slot it belongs in, how big
    // it is and what it hashes to; nothing about the image itself is taken on trust.
    let image = b"firmware for a flow meter, version two";
    let manifest = Manifest {
        structure_version: STRUCTURE_VERSION,
        sequence: 2,
        vendor_id: VENDOR,
        class_id: FLOW_METER,
        format: PayloadFormat::Raw,
        storage: 1,
        digest: image_digest(image),
        size: image.len() as u32,
        expires: 0,
    };

    // Signing it produces the envelope that travels with the image.
    let mut buf = [0u8; ENVELOPE_MAX];
    let written = manifest
        .sign(&publisher, &mut buf)
        .expect("a signed release");
    let envelope = &buf[..written];
    let sequence = manifest.sequence;
    println!("published sequence {sequence} in a {written}-byte envelope");

    // On the device. It checks the envelope against the key it was anchored to before it
    // accepts a single byte of the image.
    let device = Device {
        vendor_id: manifest.vendor_id,
        class_id: manifest.class_id,
        anchor: publisher.public(),
    };
    let opened = Envelope::decode(envelope).expect("a well-formed envelope");
    match opened.verify(&device.anchor) {
        Ok(release) => println!("accepted  a release for slot {}", release.storage),
        Err(error) => println!("refused   {error}"),
    }

    // It left the factory running sequence 1 from slot 0, so the release goes to the spare
    // slot and the image it is running stays where it is.
    let mut updater = Updater::new(device, MemoryStore::new(2, 4096));
    updater.provision(0, 1).expect("the shipped image");
    let mut staging = updater.begin(envelope).expect("a release for this device");
    for piece in image.chunks(16) {
        staging.write(piece).expect("the next piece");
    }
    let (received, total) = staging.progress();
    println!("staged    {received} of {total} bytes");
    let slot = staging.finish().expect("the image matched its digest");
    println!("written   to slot {slot}, leaving the running image alone");

    // The first boot into a new image is a trial. It reverts on the next boot unless the
    // device confirms that it came up, which is what makes a bad release survivable.
    let said = |decision: Boot| match decision {
        Boot::Trying(slot) => format!("slot {slot} on trial"),
        Boot::Confirmed(slot) => format!("slot {slot}, already confirmed"),
        Boot::Reverted { failed, fallback } => {
            format!("slot {failed} never confirmed, so the device runs slot {fallback} again")
        }
    };
    let decision = updater.on_boot().expect("a decision");
    println!("booting   {}", said(decision));
    updater.confirm().expect("it came up");
    let state = updater.store().record(slot).expect("the new slot").state;
    println!("confirmed slot {slot} is now {state:?}");

    // The same release offered again would take the device nowhere new, so it is refused
    // as a rollback, and so would any older one.
    match updater.stage(envelope, image) {
        Ok(_) => println!("an old release was accepted, which should never happen"),
        Err(error) => println!("old       refused: {error}"),
    }

    // The next release goes to the slot the device is not running, slot 0 now. An image
    // damaged on the way still arrives in full, but it does not hash to what was signed.
    let upgrade = b"firmware for a flow meter, version three";
    let third = Manifest {
        sequence: 3,
        storage: 0,
        digest: image_digest(upgrade),
        size: upgrade.len() as u32,
        ..manifest
    };
    let mut buf = [0u8; ENVELOPE_MAX];
    let written = third.sign(&publisher, &mut buf).expect("a signed release");
    let release = &buf[..written];
    let mut damaged = upgrade.to_vec();
    damaged[0] ^= 0xFF;
    match updater.stage(release, &damaged) {
        Ok(_) => println!("a damaged image was accepted, which should never happen"),
        Err(error) => println!("corrupt   refused: {error}"),
    }

    // The same release signed by a key this device is not anchored to gets nowhere.
    let impostor = DeviceIdentity::from_seed(&[90u8; 32]);
    let mut forged = [0u8; ENVELOPE_MAX];
    let signed = third
        .sign(&impostor, &mut forged)
        .expect("a signed release");
    match updater.stage(&forged[..signed], upgrade) {
        Ok(_) => println!("a forged release was accepted, which should never happen"),
        Err(error) => println!("forged    refused: {error}"),
    }

    // The genuine release stages and boots on trial, but never confirms: the next boot
    // fails it and goes back to the image that worked.
    updater
        .stage(release, upgrade)
        .expect("the genuine release");
    let trial = updater.on_boot().expect("a decision");
    println!(
        "booting   {}, running sequence {}",
        said(trial),
        third.sequence
    );
    let after = updater.on_boot().expect("a decision");
    println!("reverted  {}", said(after));

    // A release that failed cannot be offered again, or a captured image could be
    // replayed; the fix goes out as sequence 4.
    match updater.stage(release, upgrade) {
        Ok(_) => println!("a failed release was accepted again, which should never happen"),
        Err(error) => println!("again     refused: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(manifest.digest, image_digest(image));
    assert_eq!(received, total);
    assert_eq!(slot, 1);
    assert_eq!(state, SlotState::Confirmed);
    assert_eq!(trial, Boot::Trying(0));
    assert_eq!(
        after,
        Boot::Reverted {
            failed: 0,
            fallback: 1
        }
    );
    assert_eq!(
        updater.store().record(0).expect("slot 0").state,
        SlotState::Failed
    );
    assert_eq!(updater.installed_sequence().expect("the slots"), 3);

    Ok(())
}
