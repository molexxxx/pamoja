//! The signed update guide example; see docs/guides/update.md.

/// A release signed by its publisher, staged into the spare slot on a device anchored to
/// that publisher, booted on trial and confirmed, and the same release from another key
/// getting nowhere.
#[test]
fn a_release_reaches_a_device_and_confirms_itself() {
    // ANCHOR: example
    use pamoja_security::DeviceIdentity;
    use pamoja_update::{
        image_digest, Device, Envelope, Manifest, MemoryStore, PayloadFormat, SlotState, SlotStore,
        Updater, ENVELOPE_MAX, STRUCTURE_VERSION,
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
    println!("booting   {:?}", updater.on_boot().expect("a decision"));
    updater.confirm().expect("it came up");
    let state = updater.store().record(slot).expect("the new slot").state;
    println!("confirmed slot {slot} is now {state:?}");

    // The same release signed by a key this device is not anchored to gets nowhere.
    let impostor = DeviceIdentity::from_seed(&[90u8; 32]);
    let mut forged = [0u8; ENVELOPE_MAX];
    let signed = manifest
        .sign(&impostor, &mut forged)
        .expect("a signed release");
    match updater.stage(&forged[..signed], image) {
        Ok(_) => println!("a forged release was accepted, which should never happen"),
        Err(error) => println!("forged    refused: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(manifest.digest, image_digest(image));
    assert_eq!(received, total);
    assert_eq!(slot, 1);
    assert_eq!(state, SlotState::Confirmed);
    assert!(updater.stage(&forged[..signed], image).is_err());
}
