//! The first program on an ESP32-C3: a BME280 on the chip's I2C controller, read through
//! the driver pamoja ships, printed over the USB serial port every two seconds.
//!
//! Wire the BME280's SDA to GPIO6, its SCL to GPIO7, VIN to 3V3, and GND to ground; on a
//! XIAO ESP32C3 those are the pins labeled D4 and D5. Plug the board in and run
//! `cargo run --release`, which flashes it through espflash and opens the monitor. See
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

/// A blocking delay over the chip's microsecond system timer, which is all a driver asks
/// of the board.
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
use esp_hal::i2c::master::{Config, I2c};
use esp_println::println;
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // The core's own types hold an owned topic and payload, so the binary carries a heap;
    // the driver itself never allocates.
    esp_alloc::heap_allocator!(size: 8 * 1024);

    // The chip routes I2C0 to any two pins; GPIO6 and GPIO7 are the pair the XIAO ESP32C3
    // labels D4 and D5. The default configuration is the 100 kHz standard mode.
    let i2c = I2c::new(peripherals.I2C0, Config::default())
        .expect("a valid I2C configuration")
        .with_sda(peripherals.GPIO6)
        .with_scl(peripherals.GPIO7);

    let mut sensor = Bme280::i2c(i2c, I2C_ADDRESS_PRIMARY, Delay);
    sensor.init().expect("the BME280 answers on I2C0");

    loop {
        let measurement = sensor.measure().expect("a measurement");
        println!(
            "{:.2} C, {:.2} hPa, {:.2} % humidity",
            measurement.celsius(),
            measurement.hectopascals(),
            measurement.relative_humidity_percent()
        );
        Delay.delay_ms(2000);
    }
}
// ANCHOR_END: example
