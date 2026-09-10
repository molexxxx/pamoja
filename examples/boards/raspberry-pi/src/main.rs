//! The first program on a Raspberry Pi: a BME280 on the 40-pin header's I2C bus, read
//! through the driver pamoja ships, printed every two seconds.
//!
//! Wire the BME280's SDA to GPIO2, its SCL to GPIO3, VIN to 3V3, and GND to ground, turn
//! the I2C interface on, and run `cargo run --release`. See docs/boards/raspberry-pi.md.

use std::error::Error;
use std::thread;
use std::time::Duration;

// ANCHOR: example
use pamoja_hal::linux;
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

fn main() -> Result<(), Box<dyn Error>> {
    // The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
    let bus = linux::i2c("/dev/i2c-1")?;

    // The driver runs the datasheet's sequence over that bus: reset, identify, read the
    // calibration, configure, and then a forced measurement per read.
    let mut sensor = Bme280::i2c(bus, I2C_ADDRESS_PRIMARY, linux::delay());
    sensor.init()?;

    loop {
        let measurement = sensor.measure()?;
        println!(
            "{:.2} C, {:.2} hPa, {:.2} % humidity",
            measurement.celsius(),
            measurement.hectopascals(),
            measurement.relative_humidity_percent()
        );
        thread::sleep(Duration::from_secs(2));
    }
}
// ANCHOR_END: example
