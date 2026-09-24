//! The whole node on real hardware: a profile from a file, a BME280 on the header's I2C
//! bus, a relay on a GPIO line, and readings published to an MQTT broker.
//!
//! Nothing in it repeats a number from the profile, and nothing is Pi-specific except the
//! lines that open the bus and take the GPIO line, so the same profile, driver, and loop
//! run on a microcontroller.
//!
//! Wire a BME280 to the header's I2C pins and a relay board's IN to GPIO17, put
//! profiles/brooder-heater.json in the working directory, and run
//! `cargo run --release --bin node` with a broker on the Pi. See
//! docs/boards/raspberry-pi.md.

use std::error::Error;

// ANCHOR: example
use pamoja_codec::JsonCodec;
use pamoja_core::{Sensor, Transport};
use pamoja_gpio::{pin::Polarity, switch::Switch};
use pamoja_hal::{digital::PinState, linux};
use pamoja_mqtt::{MqttConfig, MqttTransport};
use pamoja_profile::{Node, Profile};
use pamoja_sensors::bme280::{Bme280, Measurement, I2C_ADDRESS_PRIMARY};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let profile = Profile::from_json(&std::fs::read_to_string("brooder-heater.json")?)?;

    let bus = linux::i2c("/dev/i2c-1")?;
    let mut bme280 = Bme280::i2c(bus, I2C_ADDRESS_PRIMARY, linux::delay());
    bme280.init()?;
    let probe = bme280.map(|measurement: Measurement| measurement.celsius());

    let relay = linux::output("/dev/gpiochip0", 17, "brooder", PinState::High)?;
    let lamp = Switch::new(relay, Polarity::ActiveLow);

    let mut broker = MqttTransport::new(MqttConfig::new("coop-2", "localhost", 1883));
    broker.connect().await?;

    // On mains the charge is full; a node on a panel reads its charge controller here.
    let mut node = Node::new(profile, probe, lamp, broker, JsonCodec)?;
    node.run(|| (1.0, true), tokio::time::sleep).await?;
    Ok(())
}
// ANCHOR_END: example
