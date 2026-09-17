//! A dashboard of every LoRaWAN node a network server hears, served from the gateway itself.
//!
//! A ChirpStack server publishes each uplink it accepts on MQTT. This subscribes to those
//! events, reads the seven-byte BME280 reading the ESP32-C3 LoRaWAN node sends, and draws
//! each node as it is first heard: its temperature, humidity and pressure. It prints how well
//! the best gateway heard each uplink. The dashboard is read-only, since nothing here commands
//! a node.
//!
//! Run `cargo run --release --bin dashboard -- [broker-host] [listen-address]`, naming the
//! machine ChirpStack's MQTT broker runs on, then open the address in a browser. See
//! docs/boards/walkthrough.md.

use std::collections::HashSet;
use std::error::Error;
use std::thread;

// ANCHOR: example
use pamoja_core::{Receive, Transport};
use pamoja_dashboard::{Assets, Fleet, LinkKind, Reading, Sensor, Server};
use pamoja_gateway::chirpstack::{UplinkEvent, UPLINK_TOPIC};
use pamoja_mqtt::{MqttConfig, MqttTransport};

/// The group every node is drawn in.
const GROUP: &str = "nodes";

/// The port the node sends its readings on.
const READINGS_PORT: u8 = 2;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let broker = args.next().unwrap_or_else(|| "localhost".to_owned());
    let listen = args.next().unwrap_or_else(|| "0.0.0.0:8080".to_owned());

    // The fleet starts empty; a node appears the first time the server hears it.
    let fleet = Fleet::builder()
        .org("site", "LoRaWAN site")
        .group("site", GROUP, "Field nodes", LinkKind::Lora)
        .build();

    // The dashboard serves from its own threads, reading the same fleet this loop writes.
    let served = fleet.clone();
    let address = listen.clone();
    thread::spawn(move || {
        if let Err(error) = Server::new(served, Assets::Embedded).run(&address) {
            eprintln!("could not serve on {address}: {error}");
        }
    });
    println!("serving the dashboard on http://{listen}");

    let mut events = MqttTransport::new(MqttConfig::new("pamoja-dashboard", broker, 1883));
    events.connect().await?;
    events.subscribe(UPLINK_TOPIC).await?;

    let mut drawn = HashSet::new();
    while let Some(message) = events.recv().await? {
        let Ok(event) = std::str::from_utf8(&message.payload)
            .map_err(|error| error.to_string())
            .and_then(|text| UplinkEvent::from_json(text).map_err(|error| error.to_string()))
        else {
            continue;
        };
        if event.fport != Some(READINGS_PORT) {
            continue;
        }
        let Some(reading) = decode(&event.data) else {
            println!(
                "{}: {} bytes that are not a reading",
                event.dev_eui,
                event.data.len()
            );
            continue;
        };

        let node = event.dev_eui.to_hex();
        let values = [
            ("temperature", reading.celsius, "celsius"),
            ("humidity", reading.humidity_percent, "percent"),
            ("pressure", reading.hectopascals, "hectopascal"),
        ];
        if drawn.insert(node.clone()) {
            for (key, value, unit) in values {
                fleet.add_sensor(
                    GROUP,
                    Sensor::new(format!("{node}/{key}"), Reading::new(key, value, unit)),
                );
            }
        }
        for (key, value, unit) in values {
            fleet.report_reading(
                GROUP,
                &format!("{node}/{key}"),
                Reading::new(key, value, unit),
            );
        }

        let heard = event
            .best_reception()
            .map(|best| format!("{} dBm, {} dB SNR", best.rssi_dbm, best.snr_db))
            .unwrap_or_else(|| "no gateway named".to_owned());
        println!(
            "{node} uplink {}: {:.2} C, {:.2} % humidity, {:.2} hPa ({heard})",
            event.fcnt, reading.celsius, reading.humidity_percent, reading.hectopascals
        );
    }
    Ok(())
}

/// One BME280 reading as the node sends it.
struct NodeReading {
    celsius: f32,
    humidity_percent: f32,
    hectopascals: f32,
}

/// Reads the node's seven bytes, most significant first: the temperature in hundredths of a
/// degree, signed, the humidity in hundredths of a percent, and the pressure in pascals in 24
/// bits.
fn decode(data: &[u8]) -> Option<NodeReading> {
    let bytes: &[u8; 7] = data.try_into().ok()?;
    let celsius = i16::from_be_bytes([bytes[0], bytes[1]]);
    let humidity = u16::from_be_bytes([bytes[2], bytes[3]]);
    let pascals = u32::from_be_bytes([0, bytes[4], bytes[5], bytes[6]]);
    Some(NodeReading {
        celsius: f32::from(celsius) / 100.0,
        humidity_percent: f32::from(humidity) / 100.0,
        hectopascals: pascals as f32 / 100.0,
    })
}
// ANCHOR_END: example
