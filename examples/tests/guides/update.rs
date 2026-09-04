//! The signed update guide example; see docs/guides/update.md.

/// A release signed, carried in pieces, tried, and confirmed, anchored to the SHA-256 that
/// FIPS 180-4 publishes for the image, so the digest a manifest commits to is a published
/// constant rather than one checked against itself.
#[test]
fn a_release_reaches_a_device_and_confirms_itself() {
    // ANCHOR: example
    use pamoja_security::DeviceIdentity;
    use pamoja_update::{
        Boot, Device, Envelope, Manifest, MemoryStore, PayloadFormat, SlotState, SlotStore,
        Updater, ENVELOPE_MAX, STRUCTURE_VERSION,
    };

    let publisher = DeviceIdentity::from_seed(&[0x31; 32]);

    // The image stands in for firmware. It is the 56-byte message FIPS 180-4 hashes in its
    // second worked example, so the digest the manifest commits to is a published constant.
    let image = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    let size = image.len() as u32;
    let manifest = Manifest {
        structure_version: STRUCTURE_VERSION,
        sequence: 2,
        vendor_id: [0x0A; 16],
        class_id: [0x0B; 16],
        format: PayloadFormat::Raw,
        storage: 1,
        digest: [
            0x24, 0x8d, 0x6a, 0x61, 0xd2, 0x06, 0x38, 0xb8, 0xe5, 0xc0, 0x26, 0x93, 0x0c, 0x3e,
            0x60, 0x39, 0xa3, 0x3c, 0xe4, 0x59, 0x64, 0xff, 0x21, 0x67, 0xf6, 0xec, 0xed, 0xd4,
            0x19, 0xdb, 0x06, 0xc1,
        ],
        size,
        expires: 0,
    };

    // A release says who it is for, which slot it belongs in, and what it hashes to. The
    // publisher signs that statement; nothing else about the image is taken on trust.
    let mut buf = [0u8; ENVELOPE_MAX];
    let written = manifest
        .sign(&publisher, &mut buf)
        .expect("a signed release");
    let envelope = &buf[..written];
    let opened = Envelope::decode(envelope).expect("a well-formed envelope");
    assert_eq!(
        opened.verify(&publisher.public()).expect("the signature"),
        manifest
    );

    // The device left the factory running sequence 1 from slot 0, so the release goes to the
    // spare slot and the image it runs today stays where it is.
    let device = Device {
        vendor_id: manifest.vendor_id,
        class_id: manifest.class_id,
        anchor: publisher.public(),
    };
    let mut updater = Updater::new(device, MemoryStore::new(2, 4096));
    updater.provision(0, 1).expect("the shipped image");
    let mut staging = updater.begin(envelope).expect("a release for this device");
    for piece in image.chunks(16) {
        staging.write(piece).expect("the next piece");
    }
    assert_eq!(staging.progress(), (size, size));
    assert_eq!(staging.finish().expect("the image matched its digest"), 1);

    // The first boot into a new image is a trial. It reverts to slot 0 on the next boot
    // unless it confirms itself.
    assert_eq!(updater.on_boot().expect("a decision"), Boot::Trying(1));
    assert_eq!(updater.confirm().expect("it came up"), 1);
    assert_eq!(
        updater.store().record(1).expect("slot 1").state,
        SlotState::Confirmed
    );

    // The same release, signed by a key this device is not anchored to, gets nowhere.
    let impostor = DeviceIdentity::from_seed(&[0x32; 32]);
    let mut forged = [0u8; ENVELOPE_MAX];
    let signed = manifest
        .sign(&impostor, &mut forged)
        .expect("a signed release");
    assert!(updater.stage(&forged[..signed], image).is_err());
    // ANCHOR_END: example
}
