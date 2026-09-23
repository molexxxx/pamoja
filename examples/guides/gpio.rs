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
    use pamoja_gpio::pin::{Edge, Level};
    use pamoja_gpio::spi::Mode;
    use pamoja_gpio::switch::{Contact, Switch};
    use pamoja_hal::digital::PinState;
    use pamoja_hal::script::PinScript;

    // Most relay boards energize when their input is pulled low, and a float switch wired
    // to ground closes the same way. Saying "active low" once, here, is what keeps the
    // inversion out of every line below it.
    let mut pump = Switch::active_low(PinScript::new([]));
    let mut float = Contact::active_low(PinScript::new([PinState::High, PinState::Low]));
    let runs_on = pump.polarity().level(true);
    println!("a pump on an active-low relay runs when its line is {runs_on:?}");

    // The pump runs while the tank fills. The scripted line answers open and then closed,
    // so this is the real loop with nothing plugged in; on a board the same two lines take
    // a pin from the board's GPIO library instead.
    pump.apply(true).await.expect("the relay takes it");
    let while_filling = float.read().await.expect("the line reads");
    let once_filled = float.read().await.expect("the line reads");
    let full = |closed: bool| if closed { "full" } else { "not full" };
    println!(
        "the float reads {}, then {}",
        full(while_filling),
        full(once_filled)
    );

    // The moment the float closes is that line going low, which is a falling edge. A watch
    // armed for the rising one would sleep through the tank filling.
    let closing = Edge::Falling.triggered_by(Level::High, Level::Low);
    let edge = if closing {
        "a falling edge"
    } else {
        "not a falling edge"
    };
    println!("the float closing is {edge} on that line");

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
    let owner = if reserved {
        "reserved by the specification"
    } else {
        "free for a device"
    };
    println!("{RESERVED_FROM:#04X} is {owner}");

    // And a datasheet quotes SPI's clock polarity and phase as one mode number.
    let (idles_high, trailing_edge) = Mode::Mode3.cpol_cpha();
    let idle = if idles_high { "high" } else { "low" };
    let edge = if trailing_edge { "trailing" } else { "leading" };
    println!("SPI mode 3 idles {idle} and samples on the {edge} edge");
    // ANCHOR_END: example

    assert_eq!(runs_on, Level::Low);
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

    if let Ok(chip) = std::env::var("PAMOJA_GPIO_CHIP") {
        on_a_board(&chip)?;
    }
    Ok(())
}

// ANCHOR: board
/// The same pump and float on a Raspberry Pi: the relay board's input on GPIO17 and the
/// float switch between GPIO27 and ground. Only the two lines change.
fn on_a_board(chip: &str) -> std::result::Result<(), Box<dyn Error>> {
    use std::time::{Duration, Instant};

    use pamoja_gpio::linux;
    use pamoja_gpio::pin::Level;
    use pamoja_gpio::switch::{Contact, Switch};

    // The relay energizes on a low input, so its line is taken high and the pump stays off
    // until it is asked to run. The float closes to ground against a pull-up.
    let mut pump = Switch::active_low(linux::output(chip, 17, Level::High)?);
    let mut float = Contact::active_low(linux::input(chip, 27)?);

    // Run the pump until the float closes, and stop it whatever happens: a pump left
    // running on a failed float is the fault this whole program exists to prevent.
    pump.set(true)?;
    let deadline = Instant::now() + Duration::from_secs(600);
    let filled = (|| -> std::result::Result<(), Box<dyn Error>> {
        while !float.is_asserted()? {
            if Instant::now() >= deadline {
                return Err(
                    "the tank did not fill in ten minutes; check the float and the supply".into(),
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    })();
    pump.set(false)?;
    filled?;
    println!("the tank is full and the pump is off");
    Ok(())
}
// ANCHOR_END: board
