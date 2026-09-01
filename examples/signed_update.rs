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
    Boot, Delegation, Device, Manifest, MemoryStore, PayloadFormat, Updater, DELEGATION_MAX,
    ENVELOPE_MAX, STRUCTURE_VERSION,
};
use sha2::{Digest, Sha256};

/// Who builds this firmware.
const VENDOR: [u8; 16] = [0x70; 16];

/// What kind of device it is.
const CLASS: [u8; 16] = [0x77; 16];

fn main() {
    // The anchor authorises who may release, and otherwise stays in a safe. The
    // release key is the one that actually signs, and the one that will eventually
    // need replacing.
    let author = DeviceIdentity::from_seed(&[0x21; 32]);
    let release = DeviceIdentity::from_seed(&[0x31; 32]);
    let successor = DeviceIdentity::from_seed(&[0x32; 32]);
    let device_identity = DeviceIdentity::from_seed(&[0x42; 32]);

    let mut log = AuditLog::new(device_identity);
    let mut recorded = 0usize;
    let mut updater = Updater::new(
        Device {
            vendor_id: VENDOR,
            class_id: CLASS,
            anchor: author.public(),
        },
        MemoryStore::new(2, 8192),
    );

    // The device left the factory running version 1 in slot 0.
    updater.provision(0, 1).expect("provision");
    println!("device: running version 1 from slot 0\n");

    // The anchor hands day-to-day signing to a release key, then goes back in the
    // safe. It is needed again only to authorise a rotation.
    let mut grant = [0u8; DELEGATION_MAX];
    let granted = Delegation {
        epoch: 1,
        release_key: release.public().to_bytes(),
        expires: 0,
    }
    .sign(&author, &mut grant)
    .expect("sign");
    updater.adopt(&grant[..granted], None).expect("adopt");
    println!("device: releases are signed by the delegated key, epoch 1");

    // A good update.
    println!("release: version 2");
    let image = b"firmware v2: fixes the flow sensor calibration";
    let slot = ship(&mut updater, &release, image, 2, 1).expect("version 2 should install");
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

    // The link drops partway through, which on a slow radio is the normal case.
    println!("release: version 3, over a link that keeps dropping");
    let broken = b"firmware v3: hangs before it can report in";
    let manifest = describe(broken, 3, 0);
    let mut envelope = [0u8; ENVELOPE_MAX];
    let written = manifest.sign(&release, &mut envelope).expect("sign");
    {
        let mut staging = updater.begin(&envelope[..written]).expect("begin");
        staging.write(&broken[..12]).expect("write");
        let (had, total) = staging.progress();
        println!("  link dropped after {had} of {total} bytes");
    }
    {
        let mut staging = updater
            .resume_at(&envelope[..written], None)
            .expect("resume");
        let (had, total) = staging.progress();
        println!("  reconnected; resuming at {had} of {total} rather than starting over");
        staging.write(&broken[12..]).expect("write");
        staging.finish().expect("finish");
    }
    log.append(b"staged v3");
    recorded += 1;
    println!(
        "  staged into slot 0
"
    );

    // An update that does not come up.
    println!("trying version 3");
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
        ship(&mut updater, &release, image, 2, 0),
    );

    let mut tampered = *b"firmware v4: with one byte changed in transit";
    let manifest = describe(&tampered, 4, 0);
    tampered[9] ^= 0x01;
    report(
        "an image altered on the way",
        stage(&mut updater, &release, &manifest, &tampered),
    );

    // The case a sequence number cannot catch on its own: this release really is
    // newer than what the device runs, so only an expiry gives it grounds to say no.
    let stale = b"firmware v4: newer than v2, but superseded long ago";
    let mut expiring = describe(stale, 4, 0);
    expiring.expires = 1_600_000_000;
    let mut envelope = [0u8; ENVELOPE_MAX];
    let written = expiring.sign(&release, &mut envelope).expect("sign");
    report(
        "a stale release aimed at a device that was offline",
        updater.stage_at(&envelope[..written], stale, Some(1_700_000_000)),
    );

    // The release key is suspected, so the anchor appoints its successor. No one
    // has to visit the device for that to take effect.
    println!(
        "
rotating the release key:"
    );
    let mut grant = [0u8; DELEGATION_MAX];
    let granted = Delegation {
        epoch: 2,
        release_key: successor.public().to_bytes(),
        expires: 0,
    }
    .sign(&author, &mut grant)
    .expect("sign");
    updater.adopt(&grant[..granted], None).expect("adopt");
    log.append(b"rotated to epoch 2");
    recorded += 1;
    println!("  epoch 2 adopted; the previous key is retired");

    let next = b"firmware v4: signed by the new release key";
    report(
        "a release from the retired key",
        ship(&mut updater, &release, next, 4, 0),
    );
    accepted(
        "a release from the new key",
        ship(&mut updater, &successor, next, 4, 0),
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
        expires: 0,
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

/// Prints an outcome that is supposed to succeed.
fn accepted(what: &str, outcome: pamoja_update::Result<u8>) {
    match outcome {
        Ok(slot) => println!("  {what}: accepted into slot {slot}"),
        Err(refusal) => println!("  {what}: REFUSED ({}), which is a bug", refusal.reason()),
    }
}

/// Prints how an attempt was refused.
fn report(what: &str, outcome: pamoja_update::Result<u8>) {
    match outcome {
        Ok(slot) => println!("  {what}: ACCEPTED into slot {slot}, which is a bug"),
        Err(refusal) => println!("  {what}: refused, {}", refusal.reason()),
    }
}
