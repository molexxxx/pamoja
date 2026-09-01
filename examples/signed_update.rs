//! Updating a device in the field, including the update that goes wrong.
//!
//! A water-point controller is running version 1. Version 2 is released, carried
//! to it however it can be carried, verified, tried, and confirmed. Then version 3
//! turns out to hang on boot, and the device brings itself back on its own.
//!
//! Every step is recorded in a tamper-evident log, so the device carries its own
//! account of what it ran and when, which is what an auditor asks for after a
//! failure. Nothing here needs hardware or a network.
//!
//! Run with: `cargo run -p pamoja-examples --example signed_update`

use pamoja_audit::AuditLog;
use pamoja_security::DeviceIdentity;
use pamoja_update::{
    Boot, Device, Manifest, MemoryStore, PayloadFormat, Updater, ENVELOPE_MAX, STRUCTURE_VERSION,
};
use sha2::{Digest, Sha256};

/// Who builds this firmware.
const VENDOR: [u8; 16] = [0x70; 16];

/// What kind of device it is.
const CLASS: [u8; 16] = [0x77; 16];

fn main() {
    // The author's key lives on the release machine and never on the device. The
    // device holds only the public half, which is enough to tell a real release
    // from anything else.
    let author = DeviceIdentity::from_seed(&[0x21; 32]);
    let device_identity = DeviceIdentity::from_seed(&[0x42; 32]);

    let mut log = AuditLog::new(device_identity);
    let mut recorded = 0usize;
    let mut updater = Updater::new(
        Device {
            vendor_id: VENDOR,
            class_id: CLASS,
            author: author.public(),
        },
        MemoryStore::new(2, 8192),
    );

    // The device left the factory running version 1 in slot 0.
    updater.provision(0, 1).expect("provision");
    println!("device: running version 1 from slot 0\n");

    // A good update.
    println!("release: version 2");
    let image = b"firmware v2: fixes the flow sensor calibration";
    let slot = ship(&mut updater, &author, image, 2, 1).expect("version 2 should install");
    log.append(b"staged v2");
    recorded += 1;
    println!("  staged into slot {slot}");

    match updater.on_boot().expect("boot") {
        Boot::Trying(slot) => println!("  booting slot {slot} on trial"),
        other => panic!("expected a trial boot, got {other:?}"),
    }
    updater.confirm().expect("confirm");
    log.append(b"confirmed v2");
    recorded += 1;
    println!("  v2 came up and reported healthy, so it is now the fallback\n");

    // An update that does not come up.
    println!("release: version 3");
    let broken = b"firmware v3: hangs before it can report in";
    let slot = ship(&mut updater, &author, broken, 3, 0).expect("version 3 should install");
    log.append(b"staged v3");
    recorded += 1;
    println!("  staged into slot {slot}");

    match updater.on_boot().expect("boot") {
        Boot::Trying(slot) => println!("  booting slot {slot} on trial"),
        other => panic!("expected a trial boot, got {other:?}"),
    }
    println!("  v3 hangs, so it never confirms, and the device resets");

    match updater.on_boot().expect("boot") {
        Boot::Reverted { failed, fallback } => {
            log.append(b"reverted v3");
            recorded += 1;
            println!("  slot {failed} failed; falling back to slot {fallback}");
        }
        other => panic!("expected a revert, got {other:?}"),
    }
    println!("  the device is back on v2 without anyone visiting it\n");

    // What an untrusted carrier cannot do.
    println!("attempts an operator would want refused:");
    let impostor = DeviceIdentity::from_seed(&[0x99; 32]);
    report(
        "someone else's release",
        ship(&mut updater, &impostor, image, 4, 0),
    );
    report(
        "replaying the old v2 image",
        ship(&mut updater, &author, image, 2, 0),
    );

    let mut tampered = *b"firmware v4: with one byte changed in transit";
    let manifest = describe(&tampered, 4, 0);
    tampered[9] ^= 0x01;
    report(
        "an image altered on the way",
        stage(&mut updater, &author, &manifest, &tampered),
    );

    println!("\naudit log: {} entries, chained and signed", recorded);
}

/// Describes an image for a release.
fn describe(image: &[u8], sequence: u64, slot: u8) -> Manifest {
    Manifest {
        structure_version: STRUCTURE_VERSION,
        sequence,
        vendor_id: VENDOR,
        class_id: CLASS,
        format: PayloadFormat::Raw,
        storage: slot,
        digest: Sha256::digest(image).into(),
        size: image.len() as u32,
    }
}

/// Signs a release and offers it to the device.
fn ship(
    updater: &mut Updater<MemoryStore>,
    by: &DeviceIdentity,
    image: &[u8],
    sequence: u64,
    slot: u8,
) -> pamoja_update::Result<u8> {
    let manifest = describe(image, sequence, slot);
    stage(updater, by, &manifest, image)
}

/// Signs a manifest and stages the image it describes.
fn stage(
    updater: &mut Updater<MemoryStore>,
    by: &DeviceIdentity,
    manifest: &Manifest,
    image: &[u8],
) -> pamoja_update::Result<u8> {
    let mut envelope = [0u8; ENVELOPE_MAX];
    let written = manifest.sign(by, &mut envelope)?;
    updater.stage(&envelope[..written], image)
}

/// Prints how an attempt was refused.
fn report(what: &str, outcome: pamoja_update::Result<u8>) {
    match outcome {
        Ok(slot) => println!("  {what}: ACCEPTED into slot {slot}, which is a bug"),
        Err(refusal) => println!("  {what}: refused, {}", refusal.reason()),
    }
}
