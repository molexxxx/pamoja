//! A CAN bus monitor: listens on the Pi's CAN interface, through an MCP2515, and names each
//! J1939 message it hears.
//!
//! Load the controller's overlay, bring the interface up at the bus's bit rate, and run
//! `cargo run --release --bin can`. See docs/guides/can.md.

use std::error::Error;
use std::time::{Duration, Instant};

// ANCHOR: example
use pamoja_can::bus::CanBus;
use pamoja_can::J1939Id;

/// The interface the MCP2515 overlay makes.
const INTERFACE: &str = "can0";

/// Engine speed's parameter group, where the speed sits in it, and its scale.
const ENGINE_CONTROLLER_1: u32 = 61_444;
const ENGINE_SPEED_AT: usize = 3;
const RPM_PER_BIT: f64 = 0.125;

fn main() -> Result<(), Box<dyn Error>> {
    // The monitor only listens and sends nothing, so it is safe on a running machine's bus.
    let bus = CanBus::open(INTERFACE)?;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        let Some(frame) = bus.receive(Duration::from_secs(1))? else {
            println!("quiet for a second: check the bit rate, the wiring, and the termination");
            continue;
        };
        let at = started.elapsed().as_secs_f64();
        match J1939Id::from_id(frame.id()) {
            Some(id) if id.pgn() == ENGINE_CONTROLLER_1 => {
                let raw = frame
                    .signals()
                    .and_then(|signals| signals.u16(ENGINE_SPEED_AT));
                let rpm = raw.map_or(0.0, |raw| f64::from(raw) * RPM_PER_BIT);
                println!(
                    "{at:7.3} s  pgn {} from {}: {rpm:.1} rpm",
                    id.pgn(),
                    id.source()
                );
            }
            Some(id) => println!(
                "{at:7.3} s  pgn {} from {}, {} bytes",
                id.pgn(),
                id.source(),
                frame.len()
            ),
            None => println!(
                "{at:7.3} s  0x{:03X}, {} bytes, an 11-bit identifier",
                frame.id().raw(),
                frame.len()
            ),
        }
    }
    println!("{} frames in ten seconds on {INTERFACE}", bus.received());
    Ok(())
}
// ANCHOR_END: example
