#![cfg_attr(not(feature = "std"), no_std)]

//! Concrete sensor drivers for the pamoja SDK.
//!
//! [`pamoja-gpio`](https://docs.rs/pamoja-gpio) builds the address and mode bytes a
//! controller puts on an I2C or SPI bus; this crate is the layer just above it, the
//! per-part knowledge that turns the raw register bytes a specific sensor returns
//! into a physical reading. Each module is one named part: it knows that part's
//! register map, builds the bytes that configure it, and decodes what it sends back,
//! applying the exact conversion the manufacturer's datasheet specifies.
//!
//! Each module is two layers. The decode layer is pure logic with no I/O: the register
//! map, the bytes that configure the part, and the conversion of what it sends back,
//! so it runs on a microcontroller, on a gateway, and in a test with nothing plugged
//! in. The driver layer (the `embedded-hal` feature, on by default) is a type named
//! after the part that owns a bus from [`pamoja-hal`](https://docs.rs/pamoja-hal),
//! performs the datasheet's transfer sequence, and implements the core
//! [`Sensor`](https://docs.rs/pamoja-core/latest/pamoja_core/trait.Sensor.html)
//! trait, so a `Bme280` reads through any `embedded-hal` I2C or SPI implementation
//! and a `Ds18b20` through any 1-Wire bus. A driver's reading is the part's full
//! measurement; `Sensor::map` selects one channel for a controller that wants a
//! single number. Every driver is tested against its datasheet's own transfer
//! sequence over a scripted bus, and a driver's error converts into the core error so
//! the part slots into a profile or a node unchanged. The shapes the drivers share,
//! a register bus over I2C or SPI and the 16-bit register access of the Texas
//! Instruments parts, are in [`driver`].
//!
//! The conversions are anchored to each datasheet's own reference values, since they
//! are where memory-driven bugs hide. The BME280 compensation is ported from Bosch's
//! published reference code and cross-checked against its floating-point form; the
//! DS18B20 decode is pinned to the datasheet's temperature/data table and its CRC to
//! the Maxim 1-Wire polynomial; the INA219 math is checked against the datasheet's
//! worked design example; the ADS1115 full-scale conversion against its per-gain LSB
//! sizes. The parts added after those follow the same rule: the BMP280 port of Bosch's
//! integer compensation against its floating-point form; the SHT3x and SCD4x
//! conversions against the endpoints of their linear formulas, their command words
//! against the tables, and their CRC-8 against Sensirion's published check value; the
//! TMP117 decode against its temperature data table; the HDC1080 equations against
//! their endpoints and identification registers; the OPT3001 lux decode against its
//! table of worked result words and its full-scale table; the INA226 math against the
//! datasheet's calibration example.
//!
//! - [`bme280`] - Bosch temperature, pressure, and humidity over I2C or SPI.
//! - [`bmp280`] - Bosch pressure and temperature, the BME280 without humidity.
//! - [`ds18b20`] - Maxim 1-Wire digital thermometer, with CRC-checked scratchpads.
//! - [`hdc1080`] - Texas Instruments low-power I2C humidity and temperature sensor.
//! - [`ina219`] - Texas Instruments high-side current, voltage, and power monitor.
//! - [`ina226`] - Texas Instruments high- or low-side current, voltage, and power
//!   monitor with an alert pin.
//! - [`ads1115`] - Texas Instruments 16-bit I2C analog-to-digital converter.
//! - [`opt3001`] - Texas Instruments ambient light sensor with a human-eye response.
//! - [`scd4x`] - Sensirion SCD40 and SCD41 photoacoustic CO2 sensors with humidity and
//!   temperature, over I2C.
//! - [`sht3x`] - Sensirion humidity and temperature sensor, with CRC-checked words.
//! - [`tmp117`] - Texas Instruments ±0.1 °C digital temperature sensor with alert limits.

#[cfg(any(feature = "embedded-hal", test))]
extern crate alloc;

pub mod ads1115;
pub mod bme280;
pub mod bmp280;
pub mod ds18b20;
pub mod hdc1080;
pub mod ina219;
pub mod ina226;
pub mod opt3001;
pub mod scd4x;
pub mod sht3x;
pub mod tmp117;

#[cfg(feature = "embedded-hal")]
pub mod driver;
mod error;

#[cfg(feature = "embedded-hal")]
pub use error::DriverError;
pub use error::SensorError;
