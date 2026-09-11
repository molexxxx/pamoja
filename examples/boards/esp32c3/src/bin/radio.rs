//! An RFM95W on an ESP32-C3: the SX1276 breakout on the chip's SPI2, beaconing a reading and
//! printing every frame it hears over the USB serial port.
//!
//! Wire the breakout's VIN to 3V3, GND to ground, SCK to GPIO4, MOSI to GPIO5, MISO to GPIO6,
//! CS to GPIO7, and RST to GPIO10, and screw on an antenna for the band before powering it.
//! None of those is a strapping pin, which is why they are the six chosen. Plug the board in
//! and run `cargo run --release --bin radio`. See docs/boards/esp32.md.

#![no_std]
#![no_main]

use embedded_hal::delay::DelayNs;
use esp_hal::main;
use esp_hal::time::{Duration, Instant};

/// A panic resets the chip, which is the honest state for a node with nobody watching.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    esp_hal::system::software_reset()
}

/// A blocking delay over the chip's microsecond system timer, which is all a driver asks of
/// the board.
#[derive(Clone, Copy)]
struct Delay;

impl DelayNs for Delay {
    fn delay_ns(&mut self, ns: u32) {
        let start = Instant::now();
        let wait = Duration::from_micros(u64::from(ns.div_ceil(1000)));
        while start.elapsed() < wait {}
    }
}

// ANCHOR: example
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::spi::master::{Config, Spi};
use esp_hal::spi::Mode;
use esp_hal::time::Rate;
use esp_println::println;
use pamoja_lora::LinkSettings;
use pamoja_radios::duty::DutyCycle;
use pamoja_radios::sx127x::config::{PaOutput, TxPower};
use pamoja_radios::sx127x::{Board, RadioConfig, Reception, Sx127x};

/// The channel this node uses, what it sends at, and the share of time it may hold the band.
const FREQUENCY_HZ: u32 = 868_100_000;
const OUTPUT_DBM: i8 = 14;
const DUTY_CYCLE_PERMILLE: u32 = 10;

/// How long the radio listens before it looks at its duty-cycle budget again, in microseconds.
const LISTEN_US: u64 = 10_000_000;

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // The chip's GPIO matrix routes SPI2 to any pins, so the six above are chosen for being
    // free rather than for being an SPI block. Two megahertz is well under what the radio
    // accepts and is what the reference drivers use.
    let bus = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(2))
            .with_mode(Mode::_0),
    )
    .expect("a valid SPI configuration")
    .with_sck(peripherals.GPIO4)
    .with_mosi(peripherals.GPIO5)
    .with_miso(peripherals.GPIO6);

    // The chip select is a plain output the bus drives around each transaction, resting high
    // so the radio ignores the bus until a transaction starts.
    let cs = Output::new(peripherals.GPIO7, Level::High, OutputConfig::default());
    let device = ExclusiveDevice::new(bus, cs, Delay).expect("the chip select takes a level");

    // An RFM95W wires the SX1276's PA_BOOST output to its antenna and clocks it from a
    // crystal, which is what the board description says here.
    let reset = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let mut radio = Sx127x::new(device, reset, Delay, Board::new(PaOutput::PaBoost));
    radio.init().expect("the RFM95W answers on SPI2");

    // SF9 at 125 kHz, which is DR3 in the European plan, at 14 dBm on PA_BOOST.
    let link = LinkSettings::new(9, 125_000);
    let power = TxPower::for_output(PaOutput::PaBoost, OUTPUT_DBM);
    radio
        .configure(RadioConfig::new(FREQUENCY_HZ, link, power))
        .expect("the chip takes the settings");
    println!("beacon on {FREQUENCY_HZ} Hz at SF9, {OUTPUT_DBM} dBm");

    // Each frame buys silence in proportion to its airtime, and the guard says when the next
    // one may go out, counted on the chip's own microsecond timer.
    let mut duty = DutyCycle::new(DUTY_CYCLE_PERMILLE);
    let mut buffer = [0u8; 255];
    let mut reading = 0u32;

    loop {
        match radio.receive(&mut buffer, LISTEN_US) {
            Ok(Reception::Frame { len, status }) => {
                let heard = core::str::from_utf8(&buffer[..len]).unwrap_or("(not text)");
                println!(
                    "heard  {heard} at {} dBm, SNR {} dB",
                    status.rssi_dbm.round_db(),
                    status.snr_db.round_db()
                );
            }
            Ok(Reception::Corrupt) => println!("heard  a frame whose CRC failed"),
            Ok(Reception::Timeout) => {}
            Err(_) => println!("the radio stopped answering"),
        }

        let now_us = Instant::now().duration_since_epoch().as_micros();
        if duty.ready(now_us) {
            let mut frame = [0u8; 32];
            let len = beacon(&mut frame, reading);
            match radio.transmit(&frame[..len]) {
                Ok(airtime_us) => {
                    duty.transmitted(now_us, &link, len);
                    println!("sent   reading {reading} in {airtime_us} us on air");
                    reading += 1;
                }
                Err(_) => println!("the frame did not go out"),
            }
        }
    }
}

/// Writes a beacon into a buffer without allocating, and returns its length.
fn beacon(frame: &mut [u8; 32], reading: u32) -> usize {
    use core::fmt::Write;

    let mut cursor = Cursor { frame, len: 0 };
    let _ = write!(cursor, "esp32 reading {reading}");
    cursor.len
}

/// A writer over a fixed buffer, so a `no_std` node can format a frame.
struct Cursor<'a> {
    frame: &'a mut [u8; 32],
    len: usize,
}

impl core::fmt::Write for Cursor<'_> {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        for byte in text.as_bytes() {
            if self.len == self.frame.len() {
                return Err(core::fmt::Error);
            }
            self.frame[self.len] = *byte;
            self.len += 1;
        }
        Ok(())
    }
}
// ANCHOR_END: example
