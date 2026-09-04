//! The secured session guide example; see docs/guides/session.md.

/// Two devices agreeing a key neither of them sends, exchanging a reading over it, and the
/// gateway refusing the same frame twice, anchored to the X25519 test vector RFC 7748
/// publishes so the key agreement is pinned rather than round-tripped against itself.
#[test]
fn two_devices_agree_a_key_and_the_gateway_refuses_a_replay() {
    // ANCHOR: example
    use pamoja_session::{AgreementKey, Role, Session};

    // Each device is provisioned with a 32-byte seed and publishes the key it derives. These
    // are the X25519 pair RFC 7748 section 6.1 publishes, so the derivation is pinned to the
    // specification rather than checked against itself.
    let node = AgreementKey::from_seed(&[
        0x77, 0x07, 0x6D, 0x0A, 0x73, 0x18, 0xA5, 0x7D, 0x3C, 0x16, 0xC1, 0x72, 0x51, 0xB2, 0x66,
        0x45, 0xDF, 0x4C, 0x2F, 0x87, 0xEB, 0xC0, 0x99, 0x2A, 0xB1, 0x77, 0xFB, 0xA5, 0x1D, 0xB9,
        0x2C, 0x2A,
    ]);
    let gateway = AgreementKey::from_seed(&[
        0x5D, 0xAB, 0x08, 0x7E, 0x62, 0x4A, 0x8A, 0x4B, 0x79, 0xE1, 0x7F, 0x8B, 0x83, 0x80, 0x0E,
        0xE6, 0x6F, 0x3B, 0xB1, 0x29, 0x26, 0x18, 0xB6, 0xFD, 0x1C, 0x2F, 0x8B, 0x27, 0xFF, 0x88,
        0xE0, 0xEB,
    ]);
    assert_eq!(
        node.public().to_bytes(),
        [
            0x85, 0x20, 0xF0, 0x09, 0x89, 0x30, 0xA7, 0x54, 0x74, 0x8B, 0x7D, 0xDC, 0xB4, 0x3E,
            0xF7, 0x5A, 0x0D, 0xBF, 0x3A, 0x0D, 0x26, 0x38, 0x1A, 0xF4, 0xEB, 0xA4, 0xA9, 0x8E,
            0xAA, 0x9B, 0x4E, 0x6A,
        ]
    );

    // Neither side sends the session key. Both derive it from the shared secret, a salt that
    // travels in the clear, and both public keys. The roles have to be opposite.
    //
    // The salt must be fresh for every session: reusing one derives the same key from the
    // same pair of devices twice. The initiator draws it and sends it in the clear, so the
    // responder here uses the salt it received rather than one of its own.
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).expect("the system random source");
    let mut uplink = Session::establish(&node, &gateway.public(), &salt, Role::Initiator);
    let mut downlink = Session::establish(&gateway, &node.public(), &salt, Role::Responder);

    // The pump id is authenticated but not encrypted, so a router still reads it while any
    // change to it fails the tag. Sealing replaces the plaintext in the buffer it is given.
    let mut frame = *b"flow=41.2";
    let sealed = uplink.seal(&mut frame, b"pump-3");
    assert_ne!(&frame, b"flow=41.2");

    let mut captured = frame;
    downlink
        .open(&sealed, &mut frame, b"pump-3")
        .expect("authentic and fresh");
    assert_eq!(&frame, b"flow=41.2");

    // The anti-replay window refuses a counter it has already accepted, so a frame captured
    // off the air and sent again is not delivered a second time.
    assert!(downlink.open(&sealed, &mut captured, b"pump-3").is_err());
    // ANCHOR_END: example
}
