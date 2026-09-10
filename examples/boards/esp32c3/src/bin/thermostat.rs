//! A thermostat on the chip itself: a BME280 on I2C0 decides, a relay on one GPIO acts,
//! and a button on another forces the output on whatever the temperature says.
//!
//! This is a whole node in one file with no operating system and no allocator beyond the
//! small heap the core's types need. The decision is `pamoja-kit`'s thermostat, the same
//! deadband a profile's setpoint policy runs on a gateway, so the logic that keeps a
//! brooder at temperature is the same logic either side of the wire.
//!
//! Wire the BME280 to GPIO6 and GPIO7, the relay board's IN to GPIO10, and a button
//! between GPIO3 and ground; on a XIAO ESP32C3 those four are D4, D5, D10, and D1.
//! Neither GPIO10 nor GPIO3 is a strapping pin, which is why they are the two chosen.
//! Plug the board in and run
//! `cargo run --release --bin thermostat`. See docs/boards/esp32.md.

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
/// of the board. `esp-hal` keeps its own behind the `unstable` feature.
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
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::{Config, I2c};
use esp_println::println;
use pamoja_gpio::pin::Polarity;
use pamoja_gpio::switch::{Contact, Switch};
use pamoja_kit::{Debounce, Thermostat};
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

/// Hold this temperature, switching a heater on half a degree below it and off half a
/// degree above. The deadband is what stops a relay chattering at the threshold.
const SETPOINT_C: f32 = 21.0;
const DEADBAND_C: f32 = 0.5;

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // The core's own types hold an owned topic and payload, so the binary carries a heap;
    // the driver and the control math never allocate.
    esp_alloc::heap_allocator!(size: 8 * 1024);

    // The chip's GPIO matrix routes I2C0 to any two pins.
    let i2c = I2c::new(peripherals.I2C0, Config::default())
        .expect("a valid I2C configuration")
        .with_sda(peripherals.GPIO6)
        .with_scl(peripherals.GPIO7);
    let mut sensor = Bme280::i2c(i2c, I2C_ADDRESS_PRIMARY, Delay);
    sensor.init().expect("the BME280 answers on I2C0");

    // The relay. Its initial level is what the pin drives the instant it is configured,
    // so starting it at the resting level is what keeps a heater off through boot. Most
    // relay boards energize on a low input, which `ActiveLow` states once here.
    let relay = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let mut heater = Switch::new(relay, Polarity::ActiveLow);

    // The button, wired to ground, so the chip's own pull-up holds the line high until it
    // is pressed. Without that pull the line floats and reads as noise.
    let pin = Input::new(peripherals.GPIO3, InputConfig::default().with_pull(Pull::Up));
    let mut button = Contact::new(pin, Polarity::ActiveLow);
    let mut settled = Debounce::new(4, false);

    // The decision. A heating thermostat switches on below the band and off above it.
    let mut control = Thermostat::heating(SETPOINT_C, DEADBAND_C);
    let mut override_on = false;
    let mut was_pressed = false;

    loop {
        // The button is polled every 50 ms for the second between measurements, so a
        // press is caught rather than missed while the sensor is being read.
        for _ in 0..20 {
            let pressed = settled.update(button.is_asserted().unwrap_or(false));
            if pressed && !was_pressed {
                override_on = !override_on;
                println!("override {}", if override_on { "on" } else { "off" });
            }
            was_pressed = pressed;
            Delay.delay_ms(50);
        }

        let measurement = sensor.measure().expect("a measurement");
        let celsius = measurement.celsius();

        // The thermostat is fed every reading, so its deadband keeps working even while
        // the override holds the relay; releasing it resumes mid-cycle, not from cold.
        let wanted = control.update(celsius);
        let on = wanted || override_on;
        heater.set(on).expect("the relay line takes it");

        println!(
            "{celsius:.2} C, {:.1} % humidity, heater {}{}",
            measurement.relative_humidity_percent(),
            if on { "on" } else { "off" },
            if override_on { " (override)" } else { "" }
        );
    }
}
// ANCHOR_END: example
