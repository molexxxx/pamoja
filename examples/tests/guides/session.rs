//! The secured session guide example; see docs/guides/session.md.

/// A node and a gateway agreeing a key neither of them sent, then a reading crossing the
/// link sealed, arriving intact, and being refused when it is replayed.
#[test]
fn two_devices_agree_a_key_and_the_gateway_refuses_a_replay() {
    // ANCHOR: example
    use pamoja_session::{AgreementKey, Role, Session};

    // Each device is provisioned with a 32-byte seed and publishes the key it derives. A
    // real seed comes from the factory or a secure element; any 32 bytes stand in here.
    let node = AgreementKey::from_seed(&[7u8; 32]);
    let gateway = AgreementKey::from_seed(&[9u8; 32]);

    // Neither side sends the session key. Both derive it from the shared secret, a salt
    // that travels in the clear, and both public keys, with opposite roles.
    //
    // The salt must be fresh for every session: reusing one derives the same key from the
    // same pair of devices twice. The initiator draws it and sends it in the clear, so the
    // responder uses the salt it received rather than one of its own.
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).expect("the system random source");
    let mut uplink = Session::establish(&node, &gateway.public(), &salt, Role::Initiator);
    let mut downlink = Session::establish(&gateway, &node.public(), &salt, Role::Responder);
    println!("both sides derived a key without sending one");

    // The pump id is authenticated but not encrypted, so a router still reads it while any
    // change to it fails the tag. Sealing replaces the plaintext in the buffer it is given.
    let mut frame = *b"flow=41.2";
    let sealed = uplink.seal(&mut frame, b"pump-3");
    println!(
        "sealed    the reading is no longer readable: {}",
        frame != *b"flow=41.2"
    );

    // The gateway opens it back into the same buffer.
    let mut replayed = frame;
    downlink
        .open(&sealed, &mut frame, b"pump-3")
        .expect("authentic and fresh");
    println!("opened    {}", String::from_utf8_lossy(&frame));

    // The anti-replay window refuses a counter it has already accepted, so a frame
    // captured off the air and sent again is not delivered a second time.
    match downlink.open(&sealed, &mut replayed, b"pump-3") {
        Ok(()) => println!("a replayed frame was accepted, which should never happen"),
        Err(error) => println!("replay    refused: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(&frame, b"flow=41.2");
    assert!(downlink.open(&sealed, &mut replayed, b"pump-3").is_err());
}
