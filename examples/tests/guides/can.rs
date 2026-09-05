//! The CAN and J1939 guide example; see docs/guides/can.md.

/// A J1939 broadcast and an addressed request, built from the fields the standard defines
/// rather than from a packed identifier, then carried in the frames a controller puts on
/// the bus.
#[test]
fn a_j1939_broadcast_and_the_frame_that_carries_it() {
    // ANCHOR: example
    use pamoja_can::{CanId, Frame, J1939Id};

    // J1939 keeps its addressing inside the CAN identifier: a priority, a parameter group
    // that says what the message is, and the address of whatever sent it. Building one
    // from those fields is what saves a caller packing 29 bits by hand.
    const ENGINE: u8 = 0x00;
    const EEC1: u32 = 61_444; // electronic engine controller 1, which carries engine speed
    let broadcast = J1939Id::from_parts(3, EEC1, ENGINE, 0xFF);
    let (priority, group) = (broadcast.priority(), broadcast.pgn());
    let to_one_node = !broadcast.is_broadcast();
    println!("broadcast priority {priority} pgn {group}");
    println!("addressed to one node: {to_one_node}");

    // A parameter group below the PDU1 limit is addressed rather than broadcast, so those
    // eight identifier bits carry a destination instead of extending the group number.
    const REQUEST: u32 = 59_904;
    const GATEWAY: u8 = 0x01;
    const TRANSMISSION: u8 = 0x21;
    let request = J1939Id::from_parts(6, REQUEST, GATEWAY, TRANSMISSION);
    let asked_for = request.pgn();
    println!("request   pgn {asked_for} to node {TRANSMISSION:#04X}");

    // Reading one back off the bus is the same thing in reverse.
    let heard = J1939Id::from_id(request.to_id()).expect("an extended identifier");
    let from = heard.source();
    let for_node = heard.destination().unwrap_or(0xFF);
    println!("heard     from {from:#04X} for {for_node:#04X}");

    // J1939 never rides an 11-bit identifier, so a standard frame is not one.
    let eleven_bit = J1939Id::from_id(CanId::standard(0x123)).is_some();
    println!("an 11-bit identifier is J1939: {eleven_bit}");

    // The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
    // parameter group at 0.125 rpm per bit, and every signal this controller is not
    // reporting is filled with the not-available byte the standard reserves.
    let mut payload = [0xFF; 8];
    payload[3..5].copy_from_slice(&((1000.0 / 0.125) as u16).to_le_bytes());
    let eec1 = Frame::new(broadcast.to_id(), &payload).expect("eight bytes fit");
    let speed = u16::from_le_bytes([eec1.data()[3], eec1.data()[4]]);
    let rpm = f64::from(speed) * 0.125;
    let carried = eec1.dlc();
    println!("engine    {rpm} rpm in {carried} bytes");

    // Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
    // classic frame still refuses a ninth byte.
    let wide = Frame::fd(broadcast.to_id(), &[0; 32]).expect("a CAN-FD length");
    println!("32 bytes carries length code {}", wide.dlc());
    match Frame::new(broadcast.to_id(), &[0; 9]) {
        Ok(_) => println!("a classic frame took nine bytes, which should never happen"),
        Err(error) => println!("classic   refused nine bytes: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(broadcast.priority(), 3);
    assert_eq!(broadcast.pgn(), EEC1);
    assert!(broadcast.is_broadcast() && broadcast.destination().is_none());
    assert_eq!(request.pgn(), REQUEST);
    assert_eq!(request.destination(), Some(TRANSMISSION));
    assert_eq!(heard.source(), GATEWAY);
    assert_eq!(J1939Id::from_id(CanId::standard(0x123)), None);
    assert_eq!(eec1.dlc(), 8);
    assert_eq!(f64::from(speed) * 0.125, 1000.0);
    assert_eq!(wide.dlc(), 13);
    assert!(Frame::new(broadcast.to_id(), &[0; 9]).is_err());
}
