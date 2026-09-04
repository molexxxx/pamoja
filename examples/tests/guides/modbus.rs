//! The Modbus RTU guide example; see docs/guides/modbus.md.

/// A request to a field device and the reply it draws, checked against the frame bytes the
/// Modbus specification fixes, so the CRC and the field order are pinned rather than
/// round-tripped against themselves.
#[test]
fn a_request_and_the_reply_it_draws() {
    // ANCHOR: example
    use pamoja_modbus::{Adu, Pdu};

    // Ask unit 0x11 for three holding registers starting at 0x006B. The last two bytes are
    // the CRC-16/MODBUS, so this is the frame exactly as it goes out on the wire.
    let request = Pdu::read_holding_registers(0x006B, 3).to_adu(0x11);
    assert_eq!(
        request.as_bytes(),
        &[0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87]
    );

    // The device answers with three 16-bit registers. A reply carries its own checksum, so
    // the receiver validates the frame before reading any value out of it.
    let reply = Adu::from_pdu(0x11, &[0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64])
        .expect("a well-formed reply");
    let parsed = Adu::parse(reply.as_bytes()).expect("the checksum matches");
    let registers: Vec<u16> = parsed
        .response()
        .registers()
        .expect("a register reply")
        .collect();
    assert_eq!(registers, [0x022B, 0x0000, 0x0064]);

    // One flipped bit anywhere in the frame fails the checksum, which is the whole point of
    // carrying one over a long RS485 run.
    let mut corrupt = reply.as_bytes().to_vec();
    corrupt[2] ^= 0xFF;
    assert!(Adu::parse(&corrupt).is_err());
    // ANCHOR_END: example
}
