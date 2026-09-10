//! The whole loop on real hardware: a shared profile, a real sensor on the header's I2C
//! bus, a real relay on a GPIO line, and readings published to an MQTT broker.
//!
//! This is the shape a deployed node has. Nothing in it is Pi-specific except the two
//! lines that open the bus and take the GPIO line, which is the point: the same profile,
//! the same driver, and the same control loop run on a microcontroller.
//!
//! Wire a BME280 to the header's I2C pins and a relay board's IN to GPIO17, then run
//! `cargo run --release --bin node -- <broker-host> [profile.json]`. See
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
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

/// The header's I2C bus, once the interface is on.
const I2C_BUS: &str = "/dev/i2c-1";

/// The GPIO chip the header's lines live on, and the line the relay is wired to.
const CHIP: &str = "/dev/gpiochip0";
const RELAY_LINE: u32 = 17;

/// The BME280 measures three things at once. The profile judges one number, so the node
/// hands it the temperature and keeps the rest for the reading it reports alongside.
struct Probe(Bme280<pamoja_sensors::driver::I2cRegisters<linux::I2cdev>, linux::Delay>);

impl Sensor for Probe {
    type Reading = f32;

    async fn read(&mut self) -> pamoja_core::Result<f32> {
        let measurement = self
            .0
            .measure()
            .map_err(|err| pamoja_core::Error::Io(format!("the BME280 did not answer: {err}")))?;
        Ok(measurement.celsius())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let broker = args.next().unwrap_or_else(|| "localhost".to_owned());
    let manifest = args.next();

    // The profile is a file: either one passed on the command line, or the shipped
    // grain-store profile as a starting point. Nothing below repeats a number from it.
    let profile = match &manifest {
        Some(path) => Profile::from_json(&std::fs::read_to_string(path)?)?,
        None => Profile::irrigation_node(),
    };
    println!("running {} into {}", profile.name, profile.topic);

    // The sensor: the kernel's I2C adapter, handed to the driver, which runs the
    // datasheet's sequence over it.
    let mut sensor = Bme280::i2c(linux::i2c(I2C_BUS)?, I2C_ADDRESS_PRIMARY, linux::delay());
    sensor.init()?;

    // The output: one GPIO line, driven to its resting level the moment it is taken, with
    // the relay board's active-low input stated once.
    let line = linux::output(CHIP, RELAY_LINE, "pamoja-node", PinState::High)?;
    let output = Switch::new(line, Polarity::ActiveLow);

    // The link: an MQTT broker, which on a gateway is often this same Pi.
    let mut link = MqttTransport::new(MqttConfig::new(&profile.name, broker, 1883));
    link.connect().await?;

    // That is the node. Each tick reads the probe, decides with the profile, switches the
    // relay, and publishes the reading.
    let mut node = Node::new(profile, Probe(sensor), output, link, JsonCodec);
    loop {
        let reaction = node.tick().await?;
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
