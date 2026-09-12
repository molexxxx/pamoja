//! The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md.
//!
//! Run: `cargo run -p pamoja-examples --example gpio`

use std::error::Error;

/// A pump on a relay and a float switch in a tank, driven and read through the pin
/// model, then the two facts a datasheet asks for before a part on a shared bus answers
/// at all.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_core::{Actuator, Sensor};
    use pamoja_gpio::i2c::{Address, Direction, RESERVED_FROM};
    use pamoja_gpio::pin::{Edge, Level, Polarity};
    use pamoja_gpio::spi::Mode;
    use pamoja_gpio::switch::{Contact, Switch};
    use pamoja_hal::digital::PinState;
    use pamoja_hal::script::PinScript;

    // Most relay boards energize when their input is pulled low, and a float switch wired
    // to ground closes the same way. Saying "active low" once, here, is what keeps the
    // inversion out of every line below it.
    let mut pump = Switch::new(PinScript::new([]), Polarity::ActiveLow);
    let mut float = Contact::new(
        PinScript::new([PinState::High, PinState::Low]),
        Polarity::ActiveLow,
    );
    println!(
        "a pump on an active-low relay runs when its line is {:?}",
        Polarity::ActiveLow.level(true)
    );

    // The pump runs while the tank fills. The scripted line answers open and then closed,
    // so this is the real loop with nothing plugged in; on a board the same two lines take
    // a pin from the host's GPIO library instead.
    pump.apply(true).await.expect("the relay takes it");
    let while_filling = float.read().await.expect("the line reads");
    let once_filled = float.read().await.expect("the line reads");
    println!("the float reads full: {while_filling}, then {once_filled}");

    // The moment the float closes is that line going low, which is a falling edge. A watch
    // armed for the rising one would sleep through the tank filling.
    let closing = Edge::Falling.triggered_by(Level::High, Level::Low);
    println!("the float closing is a falling edge on that line: {closing}");

    // Full, so the pump stops. Releasing the switch hands the line back, and the levels it
    // was driven to are the whole conversation the board saw.
    pump.apply(false).await.expect("the relay takes it");
    let line = pump.release();
    let (ran, stopped) = (line.driven()[0], line.driven()[1]);
    println!("running drove the line {ran:?} and stopping drove it {stopped:?}");

    // A part on a shared bus answers to an address, and the byte on the wire is not the
    // address the datasheet prints: it shifts up one and the low bit says read or write.
    let sensor = Address::seven_bit(0x76).expect("a 7-bit address");
    let to_write = sensor.frame(Direction::Write).as_bytes()[0];
    let to_read = sensor.frame(Direction::Read).as_bytes()[0];
    println!("a part at 0x76 is written to as {to_write:#04X} and read from as {to_read:#04X}");

    // Two ranges belong to the specification itself, so a part answering in either is a
    // wiring mistake rather than a device.
    let reserved = Address::seven_bit(RESERVED_FROM)
        .expect("in range")
        .is_reserved();
    println!("{RESERVED_FROM:#04X} is reserved by the specification: {reserved}");

    // And a datasheet quotes SPI's clock polarity and phase as one mode number.
    let (idles_high, trailing_edge) = Mode::Mode3.cpol_cpha();
    println!("SPI mode 3 idles high: {idles_high}, samples on the trailing edge: {trailing_edge}");
    // ANCHOR_END: example

    assert_eq!(Polarity::ActiveLow.level(true), Level::Low);
    assert!(!while_filling);
    assert!(once_filled);
    assert!(closing);
    assert!(!Edge::Rising.triggered_by(Level::High, Level::Low));
    assert_eq!((ran, stopped), (PinState::Low, PinState::High));
    assert_eq!((to_write, to_read), (0xEC, 0xED));
    assert!(!sensor.is_reserved());
    assert!(reserved);
    assert!(idles_high && trailing_edge);
    assert_eq!(Mode::from_cpol_cpha(true, false).number(), 2);

    Ok(())
}
