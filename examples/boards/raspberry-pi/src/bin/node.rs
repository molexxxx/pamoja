//! The whole loop on real hardware: a shared profile, a real sensor on the header's I2C
//! bus, a real relay on a GPIO line, and readings published to an MQTT broker.
//!
//! This is the shape a deployed node has. Nothing in it is Pi-specific except the two
//! lines that open the bus and take the GPIO line, which is the point: the same profile,
//! the same driver, and the same control loop run on a microcontroller.
//!
//! Wire a BME280 to the header's I2C pins and a relay board's IN to GPIO17, then run
//! `cargo run --release --bin node -- <broker-host> <profile.json>` with a profile that
//! judges a temperature, such as profiles/brooder-heater.json. See
//! docs/boards/raspberry-pi.md.

use std::error::Error;
use std::time::Duration;

// ANCHOR: example
use pamoja_codec::JsonCodec;
use pamoja_core::{Sensor, Transport};
use pamoja_gpio::pin::Polarity;
use pamoja_gpio::switch::Switch;
use pamoja_hal::digital::PinState;
use pamoja_hal::linux;
use pamoja_mqtt::{MqttConfig, MqttTransport};
use pamoja_profile::{Node, Profile};
use pamoja_sensors::bme280::{Bme280, Measurement, I2C_ADDRESS_PRIMARY};

/// The header's I2C bus, once the interface is on.
const I2C_BUS: &str = "/dev/i2c-1";

/// The GPIO chip the header's lines live on, and the line the relay is wired to.
const CHIP: &str = "/dev/gpiochip0";
const RELAY_LINE: u32 = 17;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let broker = args.next().unwrap_or_else(|| "localhost".to_owned());
    let manifest = args
        .next()
        .ok_or("pass the profile to run, such as profiles/brooder-heater.json")?;

    // The profile is a file, and nothing below repeats a number from it. The BME280 feeds
    // it a temperature, so it has to be a profile that judges one.
    let profile = Profile::from_json(&std::fs::read_to_string(&manifest)?)?;
    println!("running {} into {}", profile.name, profile.topic);

    // The sensor: the kernel's I2C adapter, handed to the driver, which runs the
    // datasheet's sequence over it. The BME280 measures three things at once and the
    // profile judges one number, so `map` hands it the temperature.
    let mut bme280 = Bme280::i2c(linux::i2c(I2C_BUS)?, I2C_ADDRESS_PRIMARY, linux::delay());
    bme280.init()?;
    let sensor = bme280.map(|measurement: Measurement| measurement.celsius());

    // The output: one GPIO line, driven to its resting level the moment it is taken, with
    // the relay board's active-low input stated once.
    let line = linux::output(CHIP, RELAY_LINE, "pamoja-node", PinState::High)?;
    let output = Switch::new(line, Polarity::ActiveLow);

    // The link: an MQTT broker, which on a gateway is often this same Pi.
    let mut link = MqttTransport::new(MqttConfig::new(&profile.name, broker, 1883));
    link.connect().await?;

    // That is the node. Each tick reads the probe, decides with the profile, switches the
    // relay, and publishes the reading.
    let mut node = Node::new(profile, sensor, output, link, JsonCodec)?;
    loop {
        let reaction = node.tick().await?.reaction;
        if let Some(alert) = reaction.alert {
            println!("alert: {}", alert.kind());
        }

        // The profile says how long to wait, given what the battery has left. A node on
        // mains reports a full charge; one on a panel reads its own charge controller.
        let (_, wait) = node.schedule(1.0, true);
        tokio::time::sleep(wait.min(Duration::from_secs(60))).await;
    }
}
// ANCHOR_END: example
