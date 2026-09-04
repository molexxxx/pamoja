//! The CAN and J1939 guide example; see docs/guides/can.md.

/// A J1939 broadcast and an addressed request, decoded from the identifiers the standard
/// fixes for them, then carried in the frames a controller would put on the bus.
#[test]
fn a_j1939_broadcast_and_the_frame_that_carries_it() {
    // ANCHOR: example
    use pamoja_can::{CanId, Frame, J1939Id};

    // The engine-speed broadcast a J1939 engine or genset puts on the bus. J1939 keeps its
    // addressing in the identifier: a priority, a parameter group, a source address.
    let engine = J1939Id::from_id(CanId::extended(0x0CF0_0400)).expect("an extended identifier");
    assert_eq!(engine.priority(), 3);
    assert_eq!(engine.pgn(), 61_444);
    assert!(engine.is_broadcast() && engine.destination().is_none());

    // A PDU format below 0xF0 is addressed rather than broadcast, so those eight bits hold a
    // destination instead of extending the parameter group. 59904 is the request group.
    let request = J1939Id::from_id(CanId::extended(0x18EA_2101)).expect("an extended identifier");
    assert_eq!(request.pgn(), 59_904);
    assert_eq!(request.destination(), Some(0x21));
    let composed = J1939Id::from_parts(6, 59_904, 0x01, 0x21);
    assert_eq!(composed.to_id().raw(), 0x18EA_2101);

    // J1939 never rides an 11-bit identifier.
    assert_eq!(J1939Id::from_id(CanId::standard(0x123)), None);

    // The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
    // parameter group, little-endian at 0.125 rpm per bit, so 0x1F40 reads as 1000 rpm.
    let payload = [0xF0, 0x7D, 0x7D, 0x40, 0x1F, 0x00, 0xF0, 0xFF];
    let eec1 = Frame::new(CanId::extended(0x0CF0_0400), &payload).expect("eight bytes fit");
    assert_eq!(eec1.dlc(), 8);
    let speed = u16::from_le_bytes([eec1.data()[3], eec1.data()[4]]);
    assert_eq!(f64::from(speed) * 0.125, 1000.0);

    // Above eight bytes CAN-FD encodes the length in steps, so 32 bytes is code 13, while a
    // classic frame still refuses a ninth byte.
    let wide = Frame::fd(CanId::extended(0x0CF0_0400), &[0; 32]).expect("a CAN-FD length");
    assert_eq!(wide.dlc(), 13);
    assert!(Frame::new(CanId::extended(0x0CF0_0400), &[0; 9]).is_err());
    // ANCHOR_END: example
}
