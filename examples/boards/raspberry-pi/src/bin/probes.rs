//! A greenhouse logger on a Raspberry Pi: the air from an SHT31 on the header's I2C bus, and
//! every DS18B20 soil probe the kernel's 1-Wire driver has found, printed every ten seconds.
//!
//! Wire the SHT31's SDA to GPIO2, its SCL to GPIO3, VIN to 3V3, and GND to ground, and turn
//! the I2C interface on. Wire each probe's red lead to 3V3, its black lead to ground, and its
//! yellow data lead to GPIO4, with one 4.7 kilohm resistor from GPIO4 to 3V3 for the whole
//! bus; add `dtoverlay=w1-gpio` to `config.txt` and reboot. Then run
//! `cargo run --release --bin probes`. See docs/guides/sensors.md.

use std::error::Error;
use std::thread;
use std::time::Duration;

// ANCHOR: example
use pamoja_hal::bus::I2cBus;
use pamoja_sensors::ds18b20::linux::Thermometer;
use pamoja_sensors::sht3x::{Sht3x, I2C_ADDRESS_A};

fn main() -> Result<(), Box<dyn Error>> {
    // The air sensor, on the header's I2C bus: /dev/i2c-1 on every model.
    let bus = I2cBus::open("/dev/i2c-1")?;
    let mut air = Sht3x::new(bus.clone(), I2C_ADDRESS_A, bus.delay());

    // The kernel lists each DS18B20 it has found on GPIO4 as a directory named for its
    // serial. The list is taken once, so a probe plugged in later needs a restart.
    let probes = Thermometer::discover()?;
    if probes.is_empty() {
        return Err(
            "no DS18B20 under /sys/bus/w1/devices: check the pull-up and the overlay".into(),
        );
    }

    loop {
        let now = air.measure()?;
        println!(
            "air           {:.2} C, {:.1} %",
            now.temperature_celsius(),
            now.relative_humidity()
        );

        // Reading a probe makes the kernel run a conversion, 750 ms at 12 bits. A reading
        // corrupted on a long lead fails its checksum, and the logger says so and reads the
        // probe again next time rather than stopping.
        for probe in &probes {
            let serial = probe.serial().unwrap_or_default();
            match probe.read_scratchpad() {
                Ok(soil) => println!("{serial}  {:.2} C", soil.temperature_celsius()),
                Err(error) => println!("{serial}  {error}"),
            }
        }
        thread::sleep(Duration::from_secs(10));
    }
}
// ANCHOR_END: example
