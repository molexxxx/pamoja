//! The first program on a gateway: bring the concentrator up and print what it hears.
//!
//! A concentrator listens to a whole band at once, so the first thing worth knowing about a
//! new card is whether it hears anything at all. This reads the same configuration file the
//! gateway daemon takes, starts the board, and prints every packet in the terms a network
//! server would be told them in, a carrier in hertz and levels in decibels, rather than the
//! counts the chip keeps them as.
//!
//! Seat the card, wire its reset line, and run `cargo run --release -- gateway.json`.
//! See docs/boards/gateway.md.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

// ANCHOR: example
use pamoja_gateway::daemon::{forward, image, walk, Config};
use pamoja_radios::linux::{self, Wiring};
use pamoja_radios::sx1302::channel::Plan;
use pamoja_radios::sx1302::rx::{self, BUFFER_LEN};
use pamoja_radios::sx1302::timestamp::Counter;

/// How long to wait between asking the concentrator what it heard.
const POLL: Duration = Duration::from_millis(10);

fn main() -> Result<(), Box<dyn Error>> {
    let named = env::args()
        .nth(1)
        .ok_or("usage: board-gateway <configuration.json>")?;
    let config = Config::parse(&fs::read_to_string(&named)?)?;

    // Semtech distributes these as C source rather than as files of bytes. Either form is
    // read here, so there is no conversion step between downloading one and running this.
    let gain_control = image(Path::new(&config.concentrator.gain_control_firmware))?;
    let arbiter = image(Path::new(&config.concentrator.arbiter_firmware))?;

    let wiring = Wiring::new(
        &config.concentrator.spi,
        &config.concentrator.gpio_chip,
        config.concentrator.reset_line,
    );
    let mut chip = linux::open_sx1302(&wiring)?;

    // A concentrator tunes once and listens around that carrier, so a channel is an offset
    // from it rather than a frequency of its own.
    let plan = Plan::new(config.radio.carrier_hz, &config.radio.channels)
        .looking_for(&config.radio.spreading_factors);

    // One call walks the whole start-up order. A board that stops partway names the step it
    // stopped on, which is the difference between a wiring fault and a firmware one.
    if let Some(model) = walk(&mut chip, &config, &plan, &gain_control, &arbiter)? {
        println!("{model:?} answering on {}", config.concentrator.spi);
    }
    println!(
        "listening on {} channels around {} Hz",
        config.radio.channels.len(),
        config.radio.carrier_hz
    );

    let mut counter = Counter::new();
    let mut buffer = [0u8; BUFFER_LEN];

    loop {
        // Read on every pass, so the rollover count stays current. It is what widens a
        // packet timestamp past the 27 bits the chip keeps it in.
        chip.counter(&mut counter)?;

        for packet in rx::packets(chip.receive(&mut buffer)?) {
            // The same translation the daemon forwards with: the channel becomes a carrier
            // in hertz, the chip's quarter-decibel counts become decibels, and the timestamp
            // is widened past its rollover.
            let Some(heard) = forward::heard(
                &packet,
                config.radio.carrier_hz,
                &config.radio.channels,
                &counter,
            ) else {
                continue;
            };

            let snr = heard
                .snr_db
                .map_or_else(|| "unknown".to_owned(), |ratio| ratio.to_string());
            println!(
                "{} Hz  SF{}  {} bytes  {} RSSI  {snr} SNR  crc {:?}",
                heard.frequency_hz,
                packet.datarate,
                heard.payload.len(),
                heard.rssi_dbm,
                heard.crc,
            );
        }

        thread::sleep(POLL);
    }
}
// ANCHOR_END: example
