//! A LoRa radio on the header: an RFM95W breakout on SPI0, beaconing a reading and printing
//! every frame it hears in between.
//!
//! Wire the breakout's VIN to a 3V3 pin, GND to ground, SCK to GPIO11, MISO to GPIO9, MOSI to
//! GPIO10, CS to GPIO8 (CE0), and RST to GPIO25, and screw on an antenna for the band before
//! powering it. Turn SPI on, then run `cargo run --release --bin radio`. Two boards running it
//! hear each other. See docs/boards/raspberry-pi.md.

use std::error::Error;
use std::time::{Duration, Instant};

// ANCHOR: example
use pamoja_lora::budget::{Decibels, LinkBudget};
use pamoja_lora::region::Region;
use pamoja_radios::duty::DutyCycle;
use pamoja_radios::linux::{self, Wiring};
use pamoja_radios::radio::{RadioConfig, Reception};
use pamoja_radios::sx127x::config::PaOutput;
use pamoja_radios::sx127x::Board;

/// The header's first SPI chip select, the GPIO chip its lines are on, and the line the
/// breakout's reset pin is wired to.
const SPI: &str = "/dev/spidev0.0";
const CHIP: &str = "/dev/gpiochip0";
const RESET_LINE: u32 = 25;

/// The channel this node uses, the data rate it sends at, and how long it listens between
/// beacons.
const FREQUENCY_HZ: u32 = 868_100_000;
const DATA_RATE: u8 = 3;
const LISTEN: Duration = Duration::from_secs(10);

fn main() -> Result<(), Box<dyn Error>> {
    // The regional plan decides the channel's power ceiling and its duty cycle, so no limit
    // below is a number anyone has to remember.
    let plan = Region::Eu868.plan();
    let link = plan
        .link_settings(DATA_RATE)
        .expect("every LoRa data rate of the plan has link settings");
    let ceiling_dbm = plan.max_eirp_dbm(FREQUENCY_HZ);
    let permille = plan
        .duty_cycle_permille(FREQUENCY_HZ)
        .expect("the plan holds this channel");

    // A 2.15 dBi whip on half a decibel of pigtail. The antenna's gain counts against the
    // ceiling and the pigtail's loss counts for it, so the amplifier takes what is left.
    let whip = LinkBudget {
        transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
        transmit_cable_loss_db: Decibels::from_tenths(5),
        ..LinkBudget::default()
    };
    let output_dbm = whip
        .max_transmit_power_dbm(Decibels::from_db(i32::from(ceiling_dbm)))
        .floor_db() as i8;

    // Opening resets the chip and reads its version back, so a wiring mistake is caught here
    // rather than on the first frame.
    let mut radio = linux::open_sx127x(
        &Wiring::new(SPI, CHIP, RESET_LINE),
        Board::new(PaOutput::PaBoost),
    )?;
    radio.configure(RadioConfig::new(FREQUENCY_HZ, link, output_dbm))?;
    println!("beacon on {FREQUENCY_HZ} Hz at DR{DATA_RATE}, {output_dbm} dBm under a {ceiling_dbm} dBm ceiling");

    // The duty cycle is the radio's other budget: each frame buys silence in proportion to its
    // airtime, and the guard says when the next one may go out.
    let mut duty = DutyCycle::new(permille);
    let clock = Instant::now();
    let mut buffer = [0u8; 255];
    let mut reading = 0u32;

    loop {
        // Listening returns as soon as a frame arrives, and a frame comes with the levels it
        // was heard at: how strong it was, and how far above the noise.
        match radio.receive(&mut buffer, LISTEN.as_micros() as u64)? {
            Reception::Frame { len, levels } => println!(
                "heard  {} at {:.0} dBm, SNR {:.1} dB",
                String::from_utf8_lossy(&buffer[..len]),
                decibels(levels.rssi_dbm),
                decibels(levels.snr_db)
            ),
            Reception::Corrupt => println!("heard  a frame whose CRC failed"),
            Reception::Timeout => {}
        }

        let now_us = clock.elapsed().as_micros() as u64;
        if duty.ready(now_us) {
            let frame = format!("pi reading {reading}");
            let airtime_us = radio.transmit(frame.as_bytes())?;
            duty.transmitted(now_us, &link, frame.len());
            println!("sent   {frame} in {airtime_us} us on air");
            reading += 1;
        }
    }
}

/// Returns a level in decibels, for printing.
fn decibels(value: Decibels) -> f64 {
    f64::from(value.hundredths()) / 100.0
}
// ANCHOR_END: example
