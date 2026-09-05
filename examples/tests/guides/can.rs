//! The CAN and J1939 guide example; see docs/guides/can.md.

/// An engine broadcasting its speed and a gateway asking a gearbox a question: both
/// identifiers built from the fields the standard names, and the payload addressed by the
/// signals inside it.
#[test]
fn a_j1939_broadcast_and_the_frame_that_carries_it() {
    // ANCHOR: example
    use pamoja_can::{priority, CanId, Frame, J1939Id, Signals};

    // The nodes on this bus, by the address each answers to, and the two parameter groups
    // in play. J1939 publishes both, so naming them is what makes the traffic readable.
    const ENGINE: u8 = 0;
    const GATEWAY: u8 = 1;
    const GEARBOX: u8 = 33;
    const ENGINE_CONTROLLER_1: u32 = 61_444; // carries engine speed
    const REQUEST: u32 = 59_904; // asks another node for a parameter group

    // J1939 keeps its addressing inside the CAN identifier: a priority, the parameter
    // group, and the address of whatever sent it. A broadcast has no destination, so it
    // is its own constructor rather than a magic address a caller has to know.
    let speed_id = J1939Id::broadcast(priority::CONTROL, ENGINE_CONTROLLER_1, ENGINE);
    let (group, sent_at) = (speed_id.pgn(), speed_id.priority());
    println!("broadcast pgn {group} at priority {sent_at}");

    // A parameter group below the PDU1 limit is addressed rather than broadcast, so those
    // eight identifier bits carry a destination instead of extending the group number.
    let request_id = J1939Id::from_parts(priority::DEFAULT, REQUEST, GATEWAY, GEARBOX);
    let asked_for = request_id.pgn();
    println!("request   pgn {asked_for} addressed to node {GEARBOX}");

    // Reading one back off the bus is the same thing in reverse, so a receiver never
    // unpacks 29 bits by hand.
    let heard = J1939Id::from_id(request_id.to_id()).expect("an extended identifier");
    let (from, to) = (heard.source(), heard.destination().unwrap());
    println!("heard     from node {from} for node {to}");

    // The payload. Every signal starts marked not available, and this controller reports
    // only engine speed, which that group places at byte offset three, two bytes wide, at
    // 0.125 rpm per bit.
    let mut reported = Signals::new();
    reported.set_u16(3, (1000.0 / 0.125) as u16);
    let frame = Frame::new(speed_id.to_id(), reported.as_bytes()).expect("eight bytes fit");

    // The receiving node reads the same offset back, so neither end slices the payload.
    let signals = frame.signals().expect("a J1939 frame carries eight bytes");
    let rpm = f64::from(signals.u16(3).expect("engine speed")) * 0.125;
    println!("engine    {rpm} rpm, carried in {} bytes", frame.dlc());

    // Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
    // classic frame still refuses a ninth byte.
    let wide = Frame::fd(speed_id.to_id(), &[0; 32]).expect("a CAN-FD length");
    println!("32 bytes carries length code {}", wide.dlc());
    match Frame::new(speed_id.to_id(), &[0; 9]) {
        Ok(_) => println!("a classic frame took nine bytes, which should never happen"),
        Err(error) => println!("classic   refused nine bytes: {error}"),
    }

    // J1939 never rides an 11-bit identifier, so a standard frame is not one of its
    // messages however its bits happen to line up.
    let short_id = J1939Id::from_id(CanId::standard(291));
    println!("an 11-bit identifier is J1939: {}", short_id.is_some());
    // ANCHOR_END: example

    assert_eq!(speed_id.priority(), priority::CONTROL);
    assert_eq!(speed_id.pgn(), ENGINE_CONTROLLER_1);
    assert!(speed_id.is_broadcast() && speed_id.destination().is_none());
    assert_eq!(request_id.pgn(), REQUEST);
    assert_eq!(request_id.destination(), Some(GEARBOX));
    assert_eq!(heard.source(), GATEWAY);
    assert_eq!(rpm, 1000.0);
    assert_eq!(frame.dlc(), 8);
    assert_eq!(wide.dlc(), 13);
    assert!(Frame::new(speed_id.to_id(), &[0; 9]).is_err());
    assert_eq!(short_id, None);
}
