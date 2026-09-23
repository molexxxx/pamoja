//! The Modbus RTU guide example; see docs/guides/modbus.md.
//!
//! Run: `cargo run -p pamoja-examples --example modbus`

use std::error::Error;

/// A gateway at a village water pump polls an energy meter and a relay module that share one
/// RS485 line. Both devices are simulated, so the polling loop runs end to end with nothing on
/// the line.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_hal::port::{Parity, SerialPort, Settings};
    use pamoja_modbus::{Client, Line, Server, BROADCAST};

    fn word(on: bool) -> &'static str {
        if on {
            "on"
        } else {
            "off"
        }
    }

    // The line: 19200 baud, even parity, one stop bit, the default the Modbus specification
    // sets. Eleven bits a character, and 3.5 of them of silence mark where a frame ends.
    let settings = Settings::new(19_200).with_parity(Parity::Even);
    let gap = Client::frame_gap(settings).as_micros() as u64;
    println!(
        "line         {settings}, {} bits a character, t3.5 is {gap} us",
        settings.bits_per_character()
    );

    // Each device's manual gives its unit address and where its values live. The meter keeps
    // its measurements in input registers from 0: volts in tenths, amps in hundredths, then a
    // fault word. The relay module has four relays as coils 0 to 3, and the tank's low-level
    // float switch as discrete input 0, on while the water is below it.
    const METER: u8 = 17;
    const PUMP: u8 = 18;
    let mut line = Line::new();
    line.attach(Server::new(METER)?.with_input_registers(0, &[2301, 418, 0]));
    let relays = line.attach(
        Server::new(PUMP)?
            .with_coils(0, &[false; 4])
            .with_discrete_inputs(0, &[true]),
    );

    // The devices sit on a simulated line. On a gateway the port is
    // SerialPort::open("/dev/ttyUSB0", settings), and nothing after this statement changes.
    let port = SerialPort::simulated(settings, line);
    let mut client = Client::new(port.clone());

    // Poll the meter with function 0x04 for three input registers, and scale each one as its
    // manual says.
    let registers = client.read_input_registers(METER, 0, 3)?;
    println!(
        "meter        {:.1} V, {:.2} A, faults {}",
        f32::from(registers[0]) / 10.0,
        f32::from(registers[1]) / 100.0,
        registers[2]
    );

    // What that poll cost the line: the request, the reply, and the silence before the request.
    let (out, back) = (port.written(), port.received());
    let line_time = settings.transfer_micros(out) + settings.transfer_micros(back) + gap;
    println!(
        "poll         {out} bytes out, {back} back, {:.2} ms of line time",
        line_time as f64 / 1_000.0
    );

    // Read the float switch, and start the pump on relay 0 when the tank is low.
    let low = client.read_discrete_inputs(PUMP, 0, 1)?[0];
    println!("tank         low-level switch {}", word(low));
    if low {
        client.write_single_coil(PUMP, 0, true)?;
    }
    let states = client.read_coils(PUMP, 0, 4)?;
    let words: Vec<&str> = states.iter().map(|&on| word(on)).collect();
    println!("relays       {}", words.join(" "));

    // A broadcast, to unit 0, reaches every device on the line and none answers: here every
    // relay off at once. The client waits out the turnaround so each device has carried it out
    // before the next request.
    client.write_multiple_coils(BROADCAST, 0, &[false; 4])?;
    println!(
        "broadcast    every relay off, no reply, {} ms turnaround",
        client.turnaround().as_millis()
    );
    let after: Vec<&str> = client
        .read_coils(PUMP, 0, 4)?
        .iter()
        .map(|&on| word(on))
        .collect();
    println!("relays       {}", after.join(" "));

    // The meter keeps its measurements in input registers. Asking for them as holding
    // registers, function 0x03, is the usual mistake with a new device, and the meter refuses
    // it with an exception instead of answering.
    let refused = client.read_holding_registers(METER, 0, 3);
    if let Err(error) = &refused {
        println!("refused      {error}");
    }

    // A unit that is not on the line never answers. The client gives up after its response
    // timeout, one second unless told otherwise, which a simulated line counts instead of
    // sleeping through.
    let before = port.waited_micros();
    let silent = client.read_input_registers(19, 0, 1);
    if let Err(error) = &silent {
        println!(
            "silent       {error}, {} ms counted and not slept",
            (port.waited_micros() - before) / 1_000
        );
    }
    // ANCHOR_END: example

    // The frames the specification fixes are pinned in the crate's own tests, so the guide
    // asserts behavior instead.
    assert_eq!(registers, [2301, 418, 0]);
    assert_eq!((out, back), (8, 11));
    assert!(low);
    assert_eq!(states, [true, false, false, false]);
    assert!(relays
        .lock()
        .is_ok_and(|relays| relays.coil(0) == Some(false)));
    assert!(matches!(
        refused,
        Err(pamoja_modbus::ClientError::Exception {
            exception: pamoja_modbus::Exception::IllegalDataAddress,
            ..
        })
    ));
    assert!(matches!(
        silent,
        Err(pamoja_modbus::ClientError::Timeout { unit: 19, .. })
    ));

    Ok(())
}
