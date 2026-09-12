//! Reading and writing the concentrator's registers directly.
//!
//! The listening program drives the board through the library's own start-up order. This is
//! the other half of the same toolkit, for when a board behaves in a way that order does not
//! explain: it resets the chip, says what answered, and then reads or writes any byte of the
//! register map by address.
//!
//! Run `cargo run --release --bin registers -- /dev/spidev0.0 /dev/gpiochip0 23` for what the
//! chip reports about itself, add an address such as `0x5600` to read one byte, and add a
//! value after it to write one. See docs/boards/gateway.md.

use std::env;
use std::error::Error;

// ANCHOR: example
use pamoja_radios::linux::{self, Wiring};
use pamoja_radios::sx1302::register::Register;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args().skip(1);
    let (Some(spi), Some(gpio_chip), Some(line)) =
        (arguments.next(), arguments.next(), arguments.next())
    else {
        return Err("usage: registers <spi> <gpio-chip> <reset-line> [address] [value]".into());
    };

    let wiring = Wiring::new(
        &spi,
        &gpio_chip,
        number(&line).ok_or("the reset line is a number")?,
    );
    let mut chip = linux::open_sx1302(&wiring)?;

    // Opening pulses reset, so the chip is answering by here and nothing else has been asked
    // of it. Both of these read the chip's own identity rather than anything configured.
    chip.check()?;
    println!("version {}, part {:?}", chip.version()?, chip.identify()?);

    let Some(address) = arguments.next() else {
        return Ok(());
    };
    let address = u16::try_from(number(&address).ok_or("the address is a number")?)?;

    // A whole byte, which is what an address on its own names. Register::new also takes a bit
    // offset and a width, which is how the driver addresses the fields packed inside a byte.
    let whole = Register::new(address, 0, 8, false);

    if let Some(value) = arguments.next() {
        let value = u8::try_from(number(&value).ok_or("the value is a number")?)?;
        chip.write_register(whole, value)?;
    }

    println!("{address:#06x} = {:#04x}", chip.read_register(whole)?);
    Ok(())
}

/// Reads a number written either in decimal or with a `0x` prefix.
fn number(text: &str) -> Option<u32> {
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}
// ANCHOR_END: example
