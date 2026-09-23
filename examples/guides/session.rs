//! The secured session guide example; see docs/guides/session.md.
//!
//! Run: `cargo run -p pamoja-examples --example session`

use std::error::Error;

/// A node and a gateway agreeing a key neither of them sent, then readings crossing the link
/// sealed: one arriving intact, one replayed, one with its pump id rewritten, one arriving
/// late, and the gateway's reply.
fn main() -> std::result::Result<(), Box<dyn Error>> {
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
    println!("agreed    both sides derived a key without sending one");

    // The pump id is authenticated but not encrypted, so a router still reads it while any
    // change to it fails the tag. Sealing replaces the plaintext in the buffer it is given.
    let mut frame = *b"flow=41.2";
    let sealed = uplink.seal(&mut frame, b"pump-3");
    let hidden = if frame != *b"flow=41.2" {
        "no longer"
    } else {
        "still"
    };
    println!(
        "sealed    counter {}, and what goes on the wire is {hidden} the reading",
        sealed.counter
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

    // A router that rewrites the pump id breaks the tag, so the gateway refuses the frame
    // rather than file the reading under the wrong pump. A frame that fails to open leaves
    // its counter unused.
    let mut later = *b"flow=41.3";
    let later_sealed = uplink.seal(&mut later, b"pump-3");
    let mut rewritten = later;
    match downlink.open(&later_sealed, &mut rewritten, b"pump-4") {
        Ok(()) => println!("a rewritten pump id was accepted, which should never happen"),
        Err(error) => println!("altered   refused: {error}"),
    }

    // Radio frames can arrive out of order. The window accepts any counter it has not seen
    // among the 64 below the newest, so the frame that was held up still opens.
    let mut newest = *b"flow=41.5";
    let newest_sealed = uplink.seal(&mut newest, b"pump-3");
    downlink
        .open(&newest_sealed, &mut newest, b"pump-3")
        .expect("the newest frame");
    downlink
        .open(&later_sealed, &mut later, b"pump-3")
        .expect("a late frame inside the window");
    println!(
        "late      counter {} opened first, then counter {}: {}, then {}",
        newest_sealed.counter,
        later_sealed.counter,
        String::from_utf8_lossy(&newest),
        String::from_utf8_lossy(&later)
    );

    // The gateway answers on the same session. Its frames carry the other direction in
    // their nonce, so a reply can never be taken for, or replayed as, one from the node.
    let mut order = *b"valve=close";
    let order_sealed = downlink.seal(&mut order, b"pump-3");
    uplink
        .open(&order_sealed, &mut order, b"pump-3")
        .expect("the gateway's reply");
    println!(
        "reply     {}, sealed by the gateway and opened by the node",
        String::from_utf8_lossy(&order)
    );
    // ANCHOR_END: example

    assert_eq!(&frame, b"flow=41.2");
    assert_eq!(&later, b"flow=41.3");
    assert!(downlink.open(&sealed, &mut replayed, b"pump-3").is_err());

    Ok(())
}
