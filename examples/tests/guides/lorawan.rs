//! The LoRaWAN guide example; see docs/guides/lorawan.md.

/// Both halves of an over-the-air activation, anchored to a join accept captured off a live
/// network and to the session keys an independent implementation derived from it, so the
/// encryption, the MIC, and the key derivation are pinned rather than round-tripped against
/// themselves.
#[test]
fn a_network_admits_a_device_and_both_derive_the_same_keys() {
    // ANCHOR: example
    use pamoja_lorawan::{Device, JoinGrant, Session, Uplink};

    // A join accept captured off a live EU868 network, the root key it was signed under, and
    // the session keys an independent implementation derived from it. Published at
    // https://github.com/anthonykirby/lora-packet/issues/10
    let captured = [
        0x20, 0x4D, 0xD8, 0x5A, 0xE6, 0x08, 0xB8, 0x7F, 0xC4, 0x88, 0x99, 0x70, 0xB7, 0xD2, 0x04,
        0x2C, 0x9E, 0x72, 0x95, 0x9B, 0x00, 0x57, 0xAE, 0xD6, 0x09, 0x4B, 0x16, 0x00, 0x3D, 0xF1,
        0x2D, 0xE1, 0x45,
    ];
    let app_key = [
        0xB6, 0xB5, 0x3F, 0x4A, 0x16, 0x8A, 0x7A, 0x88, 0xBD, 0xF7, 0xEA, 0x13, 0x5C, 0xE9, 0xCF,
        0xCA,
    ];
    let dev_nonce = 0xCC85;

    // The network half: the address and radio settings this network grants, encrypted and
    // signed under the root key, are the frame that was captured.
    let cflist = [
        0x18, 0x4F, 0x84, 0xE8, 0x56, 0x84, 0xB8, 0x5E, 0x84, 0x88, 0x66, 0x84, 0x58, 0x6E, 0x84,
        0x00,
    ];
    let offer = JoinGrant::new(0x00E5_063A, 0x13, 0x2601_2E43)
        .with_dl_settings(0x03)
        .with_rx_delay(0x01)
        .with_cflist(cflist);
    assert_eq!(offer.accept(&app_key, dev_nonce).as_bytes(), &captured[..]);

    // The device half. A join accept carries no EUI, so only the root key decides whether it
    // verifies.
    let node = Device::new([0; 8], [0; 8], app_key);
    let accepted = node
        .accept_join(&captured, dev_nonce)
        .expect("the captured accept verifies");
    assert_eq!(accepted.dev_addr(), 0x2601_2E43);

    // Neither side transmits a session key; both derive it from the two nonces. What the
    // device derived is read back by a session holding the keys published with the capture.
    let nwk_skey = [
        0x2C, 0x96, 0xF7, 0x02, 0x81, 0x84, 0xBB, 0x0B, 0xE8, 0xAA, 0x49, 0x27, 0x52, 0x90, 0xD4,
        0xFC,
    ];
    let app_skey = [
        0xF3, 0xA5, 0xC8, 0xF0, 0x23, 0x2A, 0x38, 0xC1, 0x44, 0x02, 0x9C, 0x16, 0x58, 0x65, 0x80,
        0x2C,
    ];
    let gateway = Session::new(0x2601_2E43, nwk_skey, app_skey);
    let probe = Uplink::new(1, 1, b"real");
    let uplink = accepted
        .session()
        .encode_uplink(&probe)
        .expect("a payload that fits one frame");
    let rx = gateway
        .decode(uplink.as_bytes(), 1)
        .expect("the MIC verifies under the derived key");
    assert_eq!(rx.payload(), b"real");

    // A single byte changed in the air fails the MIC, so no one else can admit the device.
    let mut forged = captured;
    forged[1] ^= 0xFF;
    assert!(node.accept_join(&forged, dev_nonce).is_err());
    // ANCHOR_END: example
}
