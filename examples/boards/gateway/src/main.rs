//! The first program on a gateway: bring the concentrator up and print what it hears.
//!
//! A concentrator listens to a whole band at once, so the first thing worth knowing about a
//! new card is whether it hears anything at all. This reads the same configuration file the
//! gateway daemon takes, starts the board, and prints every packet in the terms a network
//! server would be told them in, a carrier in hertz and levels in decibels, rather than the
//! counts the chip keeps them as.
//!
//! Seat the card, wire its reset line or plug it in, and run
//! `cargo run --release -- gateway.json`. See docs/boards/gateway.md.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

// ANCHOR: example
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiDevice;
use pamoja_gateway::daemon::{forward, image, walk, Bus, Config};
use pamoja_radios::linux::{self, Wiring};
use pamoja_radios::sx1302::channel::Plan;
use pamoja_radios::sx1302::rx::{self, BUFFER_LEN};
use pamoja_radios::sx1302::timestamp::Counter;
use pamoja_radios::sx1302::Sx1302;

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

    // A card on the host's SPI is reached through a bus and two lines. A USB card is reached
    // through its port, and the bridge on the card does the SPI and drives the pins. Once
    // open, both are the same driver, so everything past this point is shared.
    match &config.concentrator.bus {
        Bus::Spi {
            spi,
            gpio_chip,
            reset_line,
            power_enable_line,
        } => {
            let mut wiring = Wiring::new(spi, gpio_chip, *reset_line);
            if let Some(line) = power_enable_line {
                wiring = wiring.with_power_enable_line(*line);
            }

            // Some boards gate the concentrator's supply behind a line. The handle holding it
            // stays bound while the card listens, because dropping it releases the line and
            // the card with it.
            let (chip, _supply) = linux::open_sx1302(&wiring)?;
            listen(chip, &config, spi, &gain_control, &arbiter)
        }
        Bus::Usb { port } => {
            let card = linux::open_usb_sx1302(port)?;
            listen(card.concentrator, &config, port, &gain_control, &arbiter)
        }
    }
}

/// Starts a concentrator and prints every packet it hears, whichever bus it was opened on.
fn listen<SPI, RESET, D>(
    mut chip: Sx1302<SPI, RESET, D>,
    config: &Config,
    reached_on: &str,
    gain_control: &[u8],
    arbiter: &[u8],
) -> Result<(), Box<dyn Error>>
where
    SPI: SpiDevice,
    SPI::Error: core::fmt::Debug + 'static,
    RESET: OutputPin,
    D: DelayNs,
{
    // A concentrator tunes once and listens around that carrier, so a channel is an offset
    // from it rather than a frequency of its own.
    let plan = Plan::new(config.radio.carrier_hz, &config.radio.channels)
        .looking_for(&config.radio.spreading_factors)
        .network(config.radio.lorawan_public);

    // One call walks the whole start-up order. A board that stops partway names the step it
    // stopped on, which is the difference between a wiring fault and a firmware one.
    if let Some(model) = walk(&mut chip, config, &plan, gain_control, arbiter)? {
        println!("{model:?} answering on {reached_on}");
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
