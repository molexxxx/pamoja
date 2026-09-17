# Node to dashboard

This page puts four programs together into one LoRaWAN system. A sensor node
measures the air. A gateway hears it from kilometers away. A network server
decides the frame is genuine and whose it is. A dashboard draws the reading. Every
program here is pamoja except the network server, which is
[ChirpStack](https://www.chirpstack.io/), so the parts meet a server someone else
wrote rather than one written to agree with them.

| Hop | What runs there | What it speaks |
| --- | --- | --- |
| Node | An ESP32-C3 with an RFM95W and a BME280, running `EndDevice` and `Node` | LoRaWAN over LoRa at 868 MHz |
| Gateway | A Raspberry Pi with a concentrator card, running `pamoja-gateway` | Semtech's packet forwarder protocol over UDP |
| Network server | ChirpStack, in Docker | MQTT, one JSON event per uplink |
| Dashboard | The same Pi, running a program that reads those events | HTTP to a browser |

## The parts

| Part | For | In the catalog |
| --- | --- | --- |
| ESP32-C3 board, such as a Seeed XIAO ESP32C3 | the node | [ESP32](../hardware.md#esp32) |
| RFM95W breakout, 868 MHz | the node's radio | [SX1276](../hardware.md#sx1276) |
| BME280 breakout | the node's sensor | [BME280](../hardware.md#bme280) |
| Raspberry Pi | the gateway host and the dashboard | [Raspberry Pi 5](../hardware.md#raspberry-pi-5) |
| RAK2287 or WM1302 on a Pi hat | the gateway's concentrator | [RAK2287](../hardware.md#rak2287), [WM1302](../hardware.md#wm1302) |
| An antenna for each radio, matched to 868 MHz | range | [Antennas](../hardware.md#antennas-cables-and-protection) |

The network server runs anywhere Docker does: the Pi itself, or another machine
on the same network.

## The network server

ChirpStack publishes a Docker Compose project that brings up the server, its
database, an MQTT broker and two gateway bridges, one for the packet forwarder
protocol on UDP port 1700 and one for Basics Station on port 3001. Both bridges
start configured for EU868, which is the plan the node uses:

```sh
git clone https://github.com/chirpstack/chirpstack-docker
cd chirpstack-docker
docker compose up
```

Its web interface is then on port 8080. Four things go in it, in this order:

- **The gateway**, by the identifier the gateway reports itself by: the
  `gateway` field of the gateway's configuration file.
- **A device profile** for the node: region EU868, MAC version LoRaWAN 1.0.3,
  regional parameters revision RP002-1.0.5, over-the-air activation. The node
  follows LoRaWAN 1.0.3, because that revision draws its join nonce at random,
  and the node draws it from radio noise rather than keeping a count in flash.
- **An application**, which is what MQTT events are grouped under.
- **The device**, in that application with that profile: its DevEUI, its
  JoinEUI, and its root key. ChirpStack's API calls the root key of a LoRaWAN
  1.0.x device `nwkKey`; `appKey` is the LoRaWAN 1.1 field, and a key put there
  makes every join fail with no error anywhere.

The repository's own `chirpstack` directory is a different thing: a stack
that `cargo xtask chirpstack` raises for the interop test and tears down when
the test ends. It proves the two sides agree, and it keeps nothing.

## The gateway

The [gateway page](gateway.md) covers the concentrator card, its firmware images
and its configuration file. The one field this walkthrough decides is where
uplinks go: the machine running ChirpStack, on the packet forwarder bridge's port.

```json
"upstream": { "forwarder": "chirpstack.local", "port": 1700 }
```

Start the daemon with the file:

```sh
pamoja-gateway /etc/pamoja/gateway.json
```

## The node

The [ESP32 page](esp32.md#a-lorawan-node) covers the wiring and the program.
Before flashing, set its `DEV_EUI`, `JOIN_EUI` and `APP_KEY` to the device
registered above; the values it ships with are the test identifiers the
interop stack registers. Then:

```sh
cd examples/boards/esp32c3
cargo run --release --bin lorawan
```

The node prints `no join accept yet` until the network answers, and then
`joined as` and its address. After that it prints each reading it sends,
how many times each frame went out, and anything the network sent back.
Joining can take a few attempts: each goes out at a slower data rate than the
one before, and the node holds its join airtime to the back-off of TS001-1.0.4
section 7, so it waits longer between tries the longer the network stays quiet.

## The dashboard

The last program runs on the Pi. It subscribes to ChirpStack's uplink events
over MQTT, reads each one with `pamoja_gateway::chirpstack`, turns the node's
seven bytes back into a reading, and serves a dashboard that draws each node
the first time the network hears it.

<!-- snippet: examples/boards/raspberry-pi/src/bin/dashboard.rs#example -->
From [`examples/boards/raspberry-pi/src/bin/dashboard.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/bin/dashboard.rs):

```rust
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
```
<!-- end -->

Give it the machine the MQTT broker runs on, and optionally where to listen:

```sh
cd examples/boards/raspberry-pi
cargo run --release --bin dashboard -- chirpstack.local 0.0.0.0:8080
```

The dashboard is read-only. It serves no pairing secret, because nothing here
sends a command back to a node; a program that does would add one, as the
[dashboard's gateway example](https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-dashboard/examples/gateway.rs)
shows.

## What is proven and what is not

Continuous integration runs every piece that can run without a radio:

- The node's LoRaWAN device, through each exchange with a network played by
  `pamoja-lorawan`'s own network half, down to the bytes of every MAC answer.
- The node's radio side, sending, listening in both windows and repeating on a
  scripted radio and clock, with each window opened where LoRaMac-node opens it.
- A real ChirpStack server admitting a join and an uplink forwarded both over the
  packet forwarder protocol and over Basics Station, with its MQTT events read back
  by the same `chirpstack` module the dashboard uses.
- The node program built for the ESP32-C3, and the gateway and dashboard programs
  built for Linux.

What it cannot run is radio frequency: a frame leaving the RFM95W and arriving
at the concentrator. That is the hardware checklist, and it is open until the
parts are on a bench.

## Sources

- [chirpstack-docker](https://github.com/chirpstack/chirpstack-docker), ChirpStack's
  Docker Compose project, for the services, ports and regions it starts with.
- [ChirpStack integration events](https://www.chirpstack.io/docs/chirpstack/integrations/events.html),
  for the uplink event the dashboard reads.
- [TS001-1.0.4](../about/standards.md#ts001-1-0-4) and
  [RP002-1.0.5](../about/standards.md#rp002-1-0-5), for the device, its receive
  windows and its region.
