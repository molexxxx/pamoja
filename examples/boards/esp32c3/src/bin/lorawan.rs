//! A LoRaWAN node on an ESP32-C3: a BME280 reading every five minutes, sent through an RFM95W
//! to whichever network knows its keys, and printed with whatever the network answered.
//!
//! Wire the BME280's SDA to GPIO6 and SCL to GPIO7, the pair a XIAO ESP32C3 labels D4 and D5.
//! Wire the RFM95W's SCK to GPIO4, MOSI to GPIO5, MISO to GPIO3, NSS to GPIO10, and RESET to
//! GPIO20, which on a XIAO are D2, D3, D1, D10 and D7. Both breakouts take 3V3 and ground,
//! and the radio an 868 MHz antenna before it is powered. None of those pins is read at
//! reset, and GPIO20 is UART0's receive line, which a program that only prints never uses.
//!
//! The device's identifiers and root key come from the environment it is built in, in
//! hexadecimal: `LORAWAN_DEV_EUI`, `LORAWAN_JOIN_EUI` and `LORAWAN_APP_KEY`. Set them to what
//! the network registered, then run `cargo run --release --bin lorawan`. See
//! docs/boards/esp32.md.

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
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::spi::Mode;
use esp_hal::time::Rate;
use esp_println::println;
use pamoja_lora::region::Region;
use pamoja_lorawan::device::{DeviceError, EndDevice, Settings};
use pamoja_lorawan::{parse_hex, Device, Version};
use pamoja_radios::lorawan::{Node, NodeError, Timer};
use pamoja_radios::sx127x::config::PaOutput;
use pamoja_radios::sx127x::{Board, Sx127x};
use pamoja_sensors::bme280::{Bme280, Measurement, I2C_ADDRESS_PRIMARY};

/// Who the device is to its network, and the root key it shares with it, read from the build
/// environment so that no key is kept in the source. A missing or malformed value stops the
/// build.
const DEV_EUI: [u8; 8] = from_build(env!("LORAWAN_DEV_EUI", "set LORAWAN_DEV_EUI in hex"));
const JOIN_EUI: [u8; 8] = from_build(env!("LORAWAN_JOIN_EUI", "set LORAWAN_JOIN_EUI in hex"));
const APP_KEY: [u8; 16] = from_build(env!("LORAWAN_APP_KEY", "set LORAWAN_APP_KEY in hex"));

/// Reads an identifier or key given in hexadecimal when the program was built.
const fn from_build<const N: usize>(hex: &str) -> [u8; N] {
    match parse_hex(hex) {
        Some(bytes) => bytes,
        None => panic!("an identifier or key takes two hex digits a byte"),
    }
}

/// The application port readings go out on, and how long to wait between them.
const PORT: u8 = 2;
const INTERVAL_US: u64 = 5 * 60 * 1_000_000;

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // The core's own types hold an owned topic and payload, so the binary carries a heap;
    // neither driver, the device, nor the node allocates.
    esp_alloc::heap_allocator!(size: 8 * 1024);

    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .expect("a valid I2C configuration")
        .with_sda(peripherals.GPIO6)
        .with_scl(peripherals.GPIO7);
    let mut sensor = Bme280::i2c(i2c, I2C_ADDRESS_PRIMARY, Delay);
    sensor.init().expect("the BME280 answers on I2C0");

    let bus = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(2))
            .with_mode(Mode::_0),
    )
    .expect("a valid SPI configuration")
    .with_sck(peripherals.GPIO4)
    .with_mosi(peripherals.GPIO5)
    .with_miso(peripherals.GPIO3);
    let cs = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let device = ExclusiveDevice::new(bus, cs, Delay).expect("the chip select takes a level");
    let reset = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());
    let mut radio = Sx127x::new(device, reset, Delay, Board::new(PaOutput::PaBoost));
    radio.init().expect("the RFM95W answers on SPI2");

    // The chip has no entropy of its own without its Wi-Fi radio running, so the node takes it
    // from the LoRa receiver's noise, as LoRaWAN 1.0.3 suggests for the join nonce. The network
    // registration says 1.0.3, whose nonce is random rather than a stored count.
    let seed = radio.random().expect("the RFM95W listens");
    let (min_dbm, max_dbm) = PaOutput::PaBoost.range_dbm();
    let settings = Settings::new(min_dbm, max_dbm)
        .with_version(Version::V1_0_3)
        .with_seed(seed);
    let credentials = Device::new(DEV_EUI, JOIN_EUI, APP_KEY);
    let device = EndDevice::new(Region::Eu868.plan(), credentials, settings)
        .expect("EU868 fits a device's channel table");

    // The node sends, opens both receive windows on time, and sees each uplink through its
    // repeats. The clock and the delay it times them with are the chip's own timer.
    let clock = Timer::new(|| Instant::now().duration_since_epoch().as_micros(), Delay);
    let mut node = Node::new(device, radio, clock);

    while !node.device().is_joined() {
        let nonce = node.radio_mut().random().map_or(0, |noise| noise as u16);
        match node.join(nonce) {
            Ok(true) => println!("joined as {:08X}", node.device().dev_addr().unwrap_or(0)),
            Ok(false) => println!("no join accept yet"),
            Err(NodeError::Device(DeviceError::Wait { until_us })) => node.wait_until(until_us),
            Err(error) => println!("join failed: {error}"),
        }
    }

    loop {
        let started = Instant::now().duration_since_epoch().as_micros();
        match sensor.measure() {
            Ok(measurement) => {
                let payload = encode(&measurement);
                match node.send(PORT, &payload, false) {
                    Ok(report) => {
                        println!(
                            "sent {:.2} C, {:.2} hPa, {:.2} % as uplink {} ({} transmissions)",
                            measurement.celsius(),
                            measurement.hectopascals(),
                            measurement.relative_humidity_percent(),
                            node.device().fcnt_up() - 1,
                            report.transmissions,
                        );
                        if let Some(delivery) = report.delivery {
                            println!(
                                "heard {} bytes on port {:?}",
                                delivery.payload().len(),
                                delivery.port()
                            );
                        }
                    }
                    Err(NodeError::Device(DeviceError::Wait { until_us })) => {
                        node.wait_until(until_us);
                        continue;
                    }
                    Err(error) => println!("the uplink failed: {error}"),
                }
            }
            Err(_) => println!("the BME280 did not answer"),
        }
        node.wait_until(started + INTERVAL_US);
    }
}

/// Packs a reading into seven bytes, most significant first: the temperature in hundredths
/// of a degree as a signed 16-bit value, the relative humidity in hundredths of a percent as
/// an unsigned 16-bit value, and the pressure in pascals in 24 bits.
fn encode(measurement: &Measurement) -> [u8; 7] {
    let celsius = hundredths(measurement.celsius()) as i16;
    let humidity = hundredths(measurement.relative_humidity_percent()) as u16;
    let pascals = measurement.pascals().min(0x00FF_FFFF);

    let mut payload = [0u8; 7];
    payload[..2].copy_from_slice(&celsius.to_be_bytes());
    payload[2..4].copy_from_slice(&humidity.to_be_bytes());
    payload[4..].copy_from_slice(&pascals.to_be_bytes()[1..]);
    payload
}

/// A value in hundredths, rounded to the nearest one.
fn hundredths(value: f32) -> i32 {
    let scaled = value * 100.0;
    if scaled >= 0.0 {
        (scaled + 0.5) as i32
    } else {
        (scaled - 0.5) as i32
    }
}
// ANCHOR_END: example
