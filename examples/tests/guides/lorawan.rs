//! The LoRaWAN activation guide example; see docs/guides/lorawan.md.

/// A network admitting a device: the join it grants, the session keys neither side sends,
/// the first uplink read at the far end, and a forged accept getting nowhere.
#[test]
fn a_network_admits_a_device_and_both_derive_the_same_keys() {
    // ANCHOR: example
    use pamoja_lorawan::{Device, JoinGrant, Uplink};

    // The root key is provisioned into the device at the factory and known to the network
    // server. It is the only secret either side starts with; any 16 bytes stand in here.
    let app_key = [7u8; 16];

    // The device asks to join with a nonce it has not used before, which is what stops an
    // old accept being replayed at it.
    let dev_nonce = 1;
    let node = Device::new([0; 8], [0; 8], app_key);

    // The network grants the join. It draws its own nonce, names the network the device is
    // joining, and assigns the address the device will answer to from then on.
    let app_nonce = 2;
    let net_id = 19;
    let dev_addr = 0x2601_2E43;
    let grant = JoinGrant::new(app_nonce, net_id, dev_addr);
    let accept = grant.accept(&app_key, dev_nonce);
    println!(
        "granted   address {dev_addr:#010X} in a {}-byte accept",
        accept.as_bytes().len()
    );

    // The device verifies it against the root key. A join accept carries no device
    // identifier, so only that key decides whether it is for this device.
    let joined = node
        .accept_join(accept.as_bytes(), dev_nonce)
        .expect("the accept verifies under the root key");
    println!(
        "joined    the device took address {:#010X}",
        joined.dev_addr()
    );

    // Neither side transmits a session key. Both derive the same pair from the root key
    // and the two nonces, so the network reads what the device sends without ever having
    // been told how.
    let network = grant.session(&app_key, dev_nonce);
    let reading = Uplink::new(1, 1, b"level=high");
    let uplink = joined
        .session()
        .encode_uplink(&reading)
        .expect("a payload that fits one frame");
    let received = network
        .decode(uplink.as_bytes(), 1)
        .expect("the message integrity code verifies under the derived key");
    println!(
        "uplink    the network read {}",
        String::from_utf8_lossy(received.payload())
    );

    // A single byte changed in the air fails that check, so no one else can admit the
    // device or put words in its mouth.
    let mut forged = accept.as_bytes().to_vec();
    forged[1] ^= 0xFF;
    match node.accept_join(&forged, dev_nonce) {
        Ok(_) => println!("a forged accept was taken, which should never happen"),
        Err(error) => println!("forged    accept refused: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(joined.dev_addr(), dev_addr);
    assert_eq!(received.payload(), b"level=high");
    assert!(node.accept_join(&forged, dev_nonce).is_err());
}
