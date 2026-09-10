//! Driving and reading the header's GPIO lines: a relay on one line and a limit switch
//! on another, through the kernel's GPIO character device.
//!
//! Wire the relay board's IN to GPIO17, its VCC to 5V and its GND to ground; wire the
//! limit switch between GPIO27 and a ground pin. Add `gpio=27=ip,pu` to `config.txt` and
//! reboot, so the line is pulled up and the switch pulls it down when it closes. Then run
//! `cargo run --release --bin relay`. See docs/boards/raspberry-pi.md.

use std::error::Error;
use std::thread;
use std::time::Duration;

// ANCHOR: example
use pamoja_gpio::pin::Polarity;
use pamoja_gpio::switch::{Contact, Switch};
use pamoja_hal::digital::PinState;
use pamoja_hal::linux;
use pamoja_kit::Debounce;

/// The GPIO chip the header's lines live on. Every current model exposes them here, and
/// the line numbers below are the BCM numbers the documentation and the kernel both use.
const CHIP: &str = "/dev/gpiochip0";

/// The line the relay board's input is wired to.
const RELAY_LINE: u32 = 17;

/// The line the limit switch is wired to.
const SWITCH_LINE: u32 = 27;

fn main() -> Result<(), Box<dyn Error>> {
    // Taking the line as an output also says what to drive the moment it is taken. Until
    // then every GPIO is an input, so a relay board sees whatever its own pull gives it;
    // driving the resting level immediately is what keeps a vent from opening at boot.
    // Most relay boards energize on a low input, which is what `ActiveLow` says once so
    // that nothing below this line has to think about the inversion again.
    let line = linux::output(CHIP, RELAY_LINE, "pamoja-relay", PinState::High)?;
    let mut relay = Switch::new(line, Polarity::ActiveLow);

    // The switch is wired to pull the line down when it closes, so it is active low too.
    let line = linux::input(CHIP, SWITCH_LINE, "pamoja-limit")?;
    let mut limit = Contact::new(line, Polarity::ActiveLow);

    // A mechanical contact bounces for a few milliseconds as it closes. Sampling every
    // 20 ms and requiring three agreeing samples means the state has to hold for 60 ms
    // before it counts, which is longer than the bounce and shorter than a person.
    let mut settled = Debounce::new(3, false);
    let mut was_closed = false;

    println!("watching GPIO{SWITCH_LINE}, driving GPIO{RELAY_LINE}; Ctrl-C to stop");
    loop {
        let closed = settled.update(limit.is_asserted()?);
        if closed != was_closed {
            println!(
                "the limit switch {}",
                if closed { "closed" } else { "opened" }
            );
            // The relay follows the switch. A real vent would run its motor until the
            // limit closes and then stop; this is the same two calls either way.
            relay.set(!closed)?;
            was_closed = closed;
        }
        thread::sleep(Duration::from_millis(20));
    }
}
// ANCHOR_END: example
