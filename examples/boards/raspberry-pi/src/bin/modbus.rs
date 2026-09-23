//! A Modbus line scan: asks every unit address on an RS485 line, through a USB adapter, which
//! devices are there.
//!
//! Wire the adapter's A and B to every device's A and B, join the grounds, and set the line's
//! format below to the one the devices use. Then run `cargo run --release --bin modbus`. See
//! docs/guides/modbus.md.

use std::error::Error;
use std::time::Duration;

// ANCHOR: example
use pamoja_hal::port::{Parity, SerialPort, Settings};
use pamoja_modbus::{Client, ClientError};

/// A USB RS485 adapter. The kernel names the first one it finds `ttyUSB0`, or `ttyACM0` for an
/// adapter that presents itself as a modem.
const PORT: &str = "/dev/ttyUSB0";

fn main() -> Result<(), Box<dyn Error>> {
    // The format every device on the line uses, from their manuals: 19200 8E1 is the default
    // the specification sets, and many meters ship at 9600 8N1 instead.
    let settings = Settings::new(19_200).with_parity(Parity::Even);
    let port = SerialPort::open(PORT, settings)?;

    // A device that is there answers within a few milliseconds, so a short response timeout
    // keeps the scan of all 247 addresses under half a minute.
    let mut client = Client::new(port).with_response_timeout(Duration::from_millis(100));

    let mut found = 0;
    for unit in 1..=247 {
        // Holding register 0 is a question any device can answer, with its value or with an
        // exception, and either proves the device is there.
        match client.read_holding_registers(unit, 0, 1) {
            Ok(values) => {
                println!("unit {unit:3}  holding register 0 is {}", values[0]);
                found += 1;
            }
            Err(ClientError::Exception { exception, .. }) => {
                let code = exception.code();
                println!("unit {unit:3}  there, and refused register 0 with exception {code:#04x}");
                found += 1;
            }
            Err(ClientError::Timeout { received: 0, .. }) => {}
            Err(error) => println!("unit {unit:3}  {error}: check the format and the wiring"),
        }
    }
    println!("{found} units answered on {PORT} at {settings}");
    Ok(())
}
// ANCHOR_END: example
