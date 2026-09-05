//! The Modbus RTU guide example; see docs/guides/modbus.md.

/// A request to a field device and the reply it draws, both built by the library, so a
/// gateway's polling loop can be exercised end to end with nothing on the RS485 line.
#[test]
fn a_request_and_the_reply_it_draws() {
    // ANCHOR: example
    use pamoja_modbus::{Adu, Pdu};

    // The device this gateway polls: a power meter at unit 17, whose manual says the
    // three registers holding voltage, current and a fault word start at address 107.
    const METER: u8 = 17;
    const FIRST_REGISTER: u16 = 107;

    // Ask it for those three registers. The frame is complete, checksum included, exactly
    // as it goes out on the wire.
    let request = Pdu::read_holding_registers(FIRST_REGISTER, 3).to_adu(METER);
    let sent = request.as_bytes().len();
    println!("polling unit {METER}, {sent} bytes out");

    // A stand-in for the meter. On a running gateway this frame arrives over RS485; here
    // the library builds what a meter reporting those three values would send back.
    let from_the_meter = Pdu::read_holding_registers_reply(&[2301, 418, 0])
        .expect("three registers fit one reply")
        .to_adu(METER);

    // Everything below is the gateway's own code. A reply carries its own checksum, so
    // the frame is validated before any value is read out of it.
    let reply = Adu::parse(from_the_meter.as_bytes()).expect("the checksum matches");
    let registers: Vec<u16> = reply
        .response()
        .registers()
        .expect("a register reply")
        .collect();
    let volts = f32::from(registers[0]) / 10.0;
    let amps = f32::from(registers[1]) / 100.0;
    println!("voltage   {volts:.1} V");
    println!("current   {amps:.2} A");
    println!("faults    {}", registers[2]);

    // One flipped bit anywhere in the frame fails the checksum, which is the whole point
    // of carrying one over a long RS485 run.
    let mut mangled = from_the_meter.as_bytes().to_vec();
    mangled[2] ^= 0xFF;
    match Adu::parse(&mangled) {
        Ok(_) => println!("mangled frame accepted, which should never happen"),
        Err(error) => println!("mangled frame rejected: {error}"),
    }
    // ANCHOR_END: example

    // The request and reply frames the specification fixes are pinned in the crate's own
    // tests, so a guide asserts behaviour instead.
    assert_eq!(sent, 8);
    assert_eq!(registers, [2301, 418, 0]);
    assert!(Adu::parse(&mangled).is_err());
}
