//! The device identity guide example; see docs/guides/security.md.

/// A device signing its own reading and a gateway checking it, anchored to the ed25519 test
/// vector RFC 8032 publishes, so the key derivation and the signature are pinned rather than
/// round-tripped against themselves.
#[test]
fn a_signed_reading_and_the_gateway_that_checks_it() {
    // ANCHOR: example
    use pamoja_security::{DeviceIdentity, PublicIdentity};

    // The seed is provisioned into the device and never leaves it. This one is RFC 8032 test
    // vector 2, so the key it derives and the signature below are published constants rather
    // than values checked against themselves.
    let device = DeviceIdentity::from_seed(&[
        0x4c, 0xcd, 0x08, 0x9b, 0x28, 0xff, 0x96, 0xda, 0x9d, 0xb6, 0xc3, 0x46, 0xec, 0x11, 0x4e,
        0x0f, 0x5b, 0x8a, 0x31, 0x9f, 0x35, 0xab, 0xa6, 0x24, 0xda, 0x8c, 0xf6, 0xed, 0x4f, 0xb8,
        0xa6, 0xfb,
    ]);
    assert_eq!(
        device.sign(&[0x72]).to_bytes(),
        [
            0x92, 0xa0, 0x09, 0xa9, 0xf0, 0xd4, 0xca, 0xb8, 0x72, 0x0e, 0x82, 0x0b, 0x5f, 0x64,
            0x25, 0x40, 0xa2, 0xb2, 0x7b, 0x54, 0x16, 0x50, 0x3f, 0x8f, 0xb3, 0x76, 0x22, 0x23,
            0xeb, 0xdb, 0x69, 0xda, 0x08, 0x5a, 0xc1, 0xe4, 0x3e, 0x15, 0x99, 0x6e, 0x45, 0x8f,
            0x36, 0x13, 0xd0, 0xf1, 0x1d, 0x8c, 0x38, 0x7b, 0x2e, 0xae, 0xb4, 0x30, 0x2a, 0xee,
            0xb0, 0x0d, 0x29, 0x16, 0x12, 0xbb, 0x0c, 0x00,
        ]
    );

    // Only the 32-byte public key travels to the gateway.
    let gateway = PublicIdentity::from_bytes(&device.public().to_bytes()).expect("a valid key");
    assert_eq!(gateway.fingerprint(), "3d4017c3e843895a");

    // Signing is deterministic, so the same reading always yields the same 64 bytes; there is
    // no randomness to get wrong on a microcontroller.
    let reading = b"meter-4 1182.750 kWh";
    let signature = device.sign(reading);
    assert_eq!(device.sign(reading), signature);
    assert!(gateway.verify(reading, &signature).is_ok());

    // A digit changed in transit fails, and so does a signature offered under another device's
    // key.
    assert!(gateway.verify(b"meter-4 1082.750 kWh", &signature).is_err());
    let impostor = DeviceIdentity::from_seed(&[0x5a; 32]);
    assert!(impostor.public().verify(reading, &signature).is_err());
    // ANCHOR_END: example
}
