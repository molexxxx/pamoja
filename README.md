<div align="center">

<img src="assets/pamoja-logo.svg" alt="pamoja" width="560">

**One memory-safe Rust core. Every language. For the devices that change lives.**

<a href="https://crates.io/users/tonywied17"><img height="22" alt="crates.io" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-crates-pamoja.svg?v=21ebfb2d"></a>
&nbsp;<a href="https://www.npmjs.com/org/pamoja"><img height="22" alt="npm" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-npm-pamoja.svg?v=251ba560"></a>
&nbsp;<a href="https://pypi.org/user/tonywied17/"><img height="22" alt="PyPI" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-pypi-pamoja.svg?v=7b1567cc"></a>
&nbsp;<a href="https://www.nuget.org/profiles/tonywied17"><img height="22" alt="NuGet" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-nuget-pamoja.svg?v=cdd1b61a"></a>
&nbsp;<a href="https://github.com/molexxxx/pamoja/actions/workflows/ci.yml"><img height="22" alt="CI" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-ci-pamoja.svg?v=2d04e663"></a>
&nbsp;<a href="LICENSE-MIT"><img height="22" alt="license MIT" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-license-pamoja.svg?v=79a1d17d"></a>

<a href="https://pamoja.molex.cloud"><img height="34" alt="website" src="https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-website.svg"></a>
&nbsp;<a href="https://pamoja.molex.cloud/dashboard"><img height="34" alt="dashboard demo" src="https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-dashboard.svg"></a>
&nbsp;<a href="https://github.com/molexxxx/pamoja/tree/main/docs#api-reference"><img height="34" alt="API docs" src="https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg"></a>

</div>

## In plain words

pamoja is free software for building things that watch and control the physical world - a
fridge that warns you before vaccines spoil, a pump that runs when a tank gets low, a sensor
that keeps working when the internet does not. It is built to run on cheap, solar-powered
hardware and the ordinary phones people already have, in places with little money and weak or
no connectivity. It costs nothing and works offline.

You do not have to be an engineer to use it, and you do not give anything up if you are one.

**Where to start**

- **Just want to install it?** Go to the [quick start](#quick-start): one command per language, and the same example in each.
- **Try it with no hardware.** The simulators (`pamoja-sim`) stand in for real sensors, radios, and even a robot, so you can build and test with nothing plugged in.
- **Building something?** Skip to the [crate list](#engine-and-capability-crates) and add only the pieces you need - each crate's README is its getting-started guide.
- **On a microcontroller or in a rural clinic?** That is the design target, not an afterthought - see [Why it exists](#why-it-exists).

## What is pamoja

pamoja is a single, modular SDK for IoT, robotics, and drones: one memory-safe Rust engine at the core, with idiomatic bindings for the languages a device developer actually uses. You install only the capabilities you need, and the same concepts work the same way in every language.

Control and communicate with physical things - sensors, robots, drones, gateways - from TypeScript, Python, C#, Lua, or Rust itself, with C-class performance and memory safety, without hand-rolling FFI.

## Quick start

Install the core plus only the capabilities you need. Every binding wraps the same Rust engine, so the same concepts carry across languages.

```sh
cargo add pamoja-core pamoja-mqtt   # Rust
npm install @pamoja/core            # TypeScript / Node
pip install pamoja-core             # Python
dotnet add package Pamoja.Core      # C# / .NET
```

Publish a reading and read messages back:

<details open>
<summary><b>Rust</b></summary>

```rust
use pamoja_core::Transport;
use pamoja_mqtt::{MqttConfig, MqttTransport};

let mut transport = MqttTransport::new(MqttConfig::new("sensor-1", "localhost", 1883));
transport.connect().await?;
transport.subscribe("sensors/+/temperature").await?;
transport.send("sensors/1/temperature", b"21.5").await?;

if let Some(message) = transport.recv().await? {
    println!("{}: {} bytes", message.topic, message.payload.len());
}
```

</details>

<details>
<summary><b>TypeScript / Node</b></summary>

```ts
import { MqttClient } from '@pamoja/core'

const client = new MqttClient({ clientId: 'sensor-1', host: 'localhost', port: 1883 })
await client.connect()
await client.subscribe('sensors/+/temperature')
await client.publish('sensors/1/temperature', '21.5')
for await (const message of client) {
  console.log(message.topic, message.payload.toString())
}
```

</details>

<details>
<summary><b>Python</b></summary>

```python
import asyncio
from pamoja import MqttClient

async def main():
    async with MqttClient(client_id="sensor-1", host="localhost", port=1883) as client:
        await client.subscribe("sensors/+/temperature")
        await client.publish("sensors/1/temperature", "21.5")
        async for message in client:
            print(message.topic, message.payload.decode())

asyncio.run(main())
```

</details>

<details>
<summary><b>C# / .NET</b></summary>

```csharp
using Pamoja.Core;

await using var client = new MqttClient(new MqttClientOptions
{
    ClientId = "sensor-1",
    Host = "localhost",
    Port = 1883,
});

await client.ConnectAsync();
await client.SubscribeAsync("sensors/+/temperature");
await client.PublishAsync("sensors/1/temperature", "21.5");

await foreach (var message in client)
{
    Console.WriteLine($"{message.Topic}: {message.Payload.Length} bytes");
}
```

</details>

No hardware needed: `pamoja-sim` and `pamoja-loopback` stand in for sensors, radios, degraded links, and even a robot, so all of the above runs with nothing plugged in.

## Why it exists

The places where connected devices can do the most good - smallholder farms, off-grid villages, rural clinics, disaster zones - are exactly the places with the least money, the worst connectivity, and the cheapest hardware. Most IoT and robotics stacks quietly assume the opposite of all of that.

pamoja is built for the hard environment first. If it runs well on a two-dollar microcontroller on a solar panel with an intermittent radio link, it runs well anywhere. That single constraint makes the library better for everyone.

What that means in practice:

- Cheap and salvageable hardware, down to microcontrollers with a few hundred KB of RAM.
- Offline-first: local buffering and store-and-forward, so a device disconnected for days loses nothing.
- Low bandwidth and long range: compact codecs and radio (LoRa, mesh) treated as first-class.
- Low power: async duty-cycling and energy-aware scheduling for battery and solar.
- Free and unencumbered, so cost is never a barrier to use.
- Reachable: many languages, plus a plain-language helper layer so you do not need to be an engineer to build something that works.

## The pillars

- Performant - native Rust, async-first, small enough to run `no_std` on microcontrollers.
- Secure - memory safety by construction, TLS 1.3 / DTLS, device identity, signed OTA.
- Quality of life - one consistent API in every language, with a high-level ergonomic facade plus a low-level escape hatch.
- Easy to adopt - opt-in scoped packages, strong defaults, and simulators so you can build and test with zero hardware.

## Where things stand

Released and installable, not a prototype:

- **31 crates on crates.io**, with the Node, Python, and .NET bindings on npm, PyPI, and NuGet, all versioned in lockstep.
- **870 tests across 82 targets**, pinned wherever a standard exists to that standard's own published vectors rather than to round-trips, so an implementation that is wrong but self-consistent still fails.
- **Checked against the real thing in CI**, not just mocked: MAVLink against live ArduPilot and PX4 SITL, the ROS 2 bridge against ROS 2 Jazzy with rmw_zenoh, and every `no_std` crate cross-compiled for a Cortex-M4F microcontroller.
- **Audited on every change**: rustfmt, clippy at `-D warnings`, CodeQL over five languages, and a license and security-advisory sweep of the dependency graph a consumer actually installs.

Generated surfaces (the language binding contracts and the C header) are drift-checked against the Rust source, so they cannot quietly fall behind it.

## Engine and capability crates

| Crate | Area | What it does |
| --- | --- | --- |
| [`pamoja-core`](crates/pamoja-core/README.md#pamoja-core) | core | The device model: `Transport`, `Device`, `Sensor`, `Actuator`, `Store`, event-bus, and error traits. |
| [`pamoja-codec`](crates/pamoja-codec/README.md#pamoja-codec) | serialize | CBOR, JSON, and raw codecs behind one trait, plus delta+varint batch packing and an `f32` quantizer for metered links. |
| [`pamoja-mqtt`](crates/pamoja-mqtt/README.md#pamoja-mqtt) | messaging | An MQTT client implementing the core `Transport` trait, tested against an embedded broker. |
| [`pamoja-coap`](crates/pamoja-coap/README.md#pamoja-coap) | messaging | A CoAP client over UDP with confirmable and non-confirmable delivery and RFC 7641 observe. |
| [`pamoja-ladder`](crates/pamoja-ladder/README.md#pamoja-ladder) | resilience | A cost-aware transport ladder: cheapest reachable rung first, buffering to a `Store` when every link is down. |
| [`pamoja-sync`](crates/pamoja-sync/README.md#pamoja-sync) | resilience | Offline-first store-and-forward queues: in-memory, plus a crash-safe on-disk queue that survives power loss. |
| [`pamoja-dashboard`](crates/pamoja-dashboard/README.md#pamoja-dashboard) | resilience | A local-first fleet dashboard a node serves over its own hotspot - multilingual and fully offline, with a hardware-free mock for development - so a community can see its own data with no cloud; it scales from a full gateway build down to a microcontroller no-JavaScript floor page, and profiles can declare custom sensors, node stats, and a theme the page renders with no code change. |
| [`pamoja-bus`](crates/pamoja-bus/README.md#pamoja-bus) | core | An in-memory typed publish/subscribe event bus implementing the core `EventBus` trait. |
| [`pamoja-loopback`](crates/pamoja-loopback/README.md#pamoja-loopback) | testing | An in-process `Transport` with topic matching and a fault injector, exercising the full path with no broker. |
| [`pamoja-sim`](crates/pamoja-sim/README.md#pamoja-sim) | testing | Hardware-free simulators: noisy and replay sensors, a recording actuator, a degraded-link transport, and a simulated robot that turns velocity commands into a dead-reckoned pose. |
| [`pamoja-power`](crates/pamoja-power/README.md#pamoja-power) | energy | Duty cycling plus an energy-aware governor that stretches work as the battery drains and eases off while charging. |
| [`pamoja-security`](crates/pamoja-security/README.md#pamoja-security) | trust | ed25519 device identity: sign a device's telemetry and verify it, so a gateway can prove a reading is authentic. |
| [`pamoja-audit`](crates/pamoja-audit/README.md#pamoja-audit) | trust | A `no_std` tamper-evident, SHA-256 hash-chained log; altering, reordering, or dropping any record breaks verification. |
| [`pamoja-update`](crates/pamoja-update/README.md#pamoja-update) | trust | Signed firmware updates: an RFC 9124 manifest, streaming image verification, and A/B slots that fall back on their own when an image does not come up; a transfer cut off by a dead link resumes where it stopped. |
| [`pamoja-session`](crates/pamoja-session/README.md#pamoja-session) | trust | A secured channel - X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window - so two nodes get confidentiality and integrity over a hostile link without a TLS stack. |
| [`pamoja-telemetry`](crates/pamoja-telemetry/README.md#pamoja-telemetry) | observe | Allocation-free observability that ships only what is worth the bytes as link cost rises, while counting everything. |
| [`pamoja-lora`](crates/pamoja-lora/README.md#pamoja-lora) | radio | The exact LoRa time-on-air of a payload and the duty-cycle off-time it forces, so a node stays in regulation and budget. |
| [`pamoja-lorawan`](crates/pamoja-lorawan/README.md#pamoja-lorawan) | radio | LoRaWAN 1.0.x MAC framing with AES-CMAC and AES encryption and OTAA join, against the FIPS-197 and RFC 4493 vectors. |
| [`pamoja-mesh`](crates/pamoja-mesh/README.md#pamoja-mesh) | mesh | Addressed, hop-limited, CRC-checked frames plus duplicate suppression that floods a packet across the mesh exactly once. |
| [`pamoja-routing`](crates/pamoja-routing/README.md#pamoja-routing) | mesh | Reverse-path routing that learns the cheapest route from overheard traffic, saving the airtime flooding wastes. |
| [`pamoja-modbus`](crates/pamoja-modbus/README.md#pamoja-modbus) | field I/O | Modbus RTU framing (CRC-16/Modbus) with request builders and reply decoders for RS485 field sensors. |
| [`pamoja-can`](crates/pamoja-can/README.md#pamoja-can) | field I/O | CAN 2.0 and CAN-FD frames (11- and 29-bit IDs) plus J1939 decode and compose for trucks, tractors, and gensets. |
| [`pamoja-serial`](crates/pamoja-serial/README.md#pamoja-serial) | field I/O | SLIP (RFC 1055) and COBS byte-stuffing with streaming frame decoders, so a raw UART byte stream carries discrete packets to motor controllers, GPS, and LiDAR. |
| [`pamoja-gpio`](crates/pamoja-gpio/README.md#pamoja-gpio) | field I/O | On-board bus logic: I2C 7- and 10-bit address frames (NXP UM10204) with reserved-range checks, the four SPI clock modes, and active-high/active-low GPIO pins. |
| [`pamoja-sensors`](crates/pamoja-sensors/README.md#pamoja-sensors) | field I/O | Datasheet-anchored, `no_std` decoders for common, cheap parts: BME280 (temp/humidity/pressure), DS18B20, INA219 power, and the ADS1115 ADC. |
| [`pamoja-actuators`](crates/pamoja-actuators/README.md#pamoja-actuators) | field I/O | `no_std` drivers for cheap outputs: PCA9685 16-channel PWM with servo-angle helpers, plus a stepper driver. |
| [`pamoja-zenoh`](crates/pamoja-zenoh/README.md#pamoja-zenoh) | robotics | A Zenoh transport plus a key-expression engine (validity, canonical form, wildcard matching) so fleets and robots share data over Zenoh, with or without ROS 2. |
| [`pamoja-ros2`](crates/pamoja-ros2/README.md#pamoja-ros2) | robotics | A ROS 2 bridge - topics, services, and actions - with ROS 2 name, RIHS01 type-hash, and CDR handling plus rmw_zenoh key assembly, so a robot appears as an ordinary pamoja device; interoperates with rmw_zenoh, routerless. |
| [`pamoja-mavlink`](crates/pamoja-mavlink/README.md#pamoja-mavlink) | drones | MAVLink v1/v2 framing with the CRC-16/MCRF4XX checksum, per-message CRC_EXTRA, and MAVLink 2 SHA-256 signing, a typed common dialect with MAVLink 2 extension fields, the mission, command, and offboard protocols as sans-IO state machines, and a vehicle modeled as a pamoja `Device` driven over real serial, UDP, and TCP links - verified in CI against real ArduPilot and PX4 SITL. |
| [`pamoja-kit`](crates/pamoja-kit/README.md#pamoja-kit) | ergonomics | Plain-language helpers that name the goal over the math: smoothing/filtering (EMA, median, Kalman, complementary, debounce), calibration, units and deadband shaping, PID and on/off control with ramping, trend/surge/depletion and anomaly prediction, rolling-window stats, wheel kinematics (differential, Ackermann, skid-steer, mecanum), odometry, waypoint guidance and motion safety (e-stop, watchdog, limits), two-link arm forward/inverse kinematics, and geo (distance/bearing/geofence), IMU tilt, and dew-point helpers. |
| [`pamoja-profile`](crates/pamoja-profile/README.md#pamoja-profile) | ergonomics | Named, ready-to-run device profiles from plain data or a JSON manifest; assembled and testable with no hardware. |
| [`pamoja-ffi`](crates/pamoja-ffi/README.md#pamoja-ffi) | bindings | The curated C ABI over the core and MQTT, with a `cbindgen`-generated, drift-checked `pamoja.h`. |

## Language bindings

One engine, many front doors. A version tag publishes every binding to its registry at once, so the four never drift apart.

| Language | Package | Status |
| --- | --- | --- |
| Rust | `pamoja-core`, `pamoja-mqtt`, and 29 more | available - the engine itself |
| TypeScript / Node | `@pamoja/core` | available - generated contract plus a hand-written facade (napi-rs) |
| Python | `pamoja-core` | available - generated, type-stubbed contract plus an async facade (PyO3 + maturin) |
| C# / .NET | `Pamoja.Core` | available - P/Invoke interop plus an async facade with `IAsyncEnumerable` streams |
| Lua | embeddable | planned |
| WebAssembly | browser / npm | planned |
| Kotlin, Swift, Go | platform-native | planned |

Device identity, the wire codecs, and the helper math reach all three bindings alongside the MQTT transport, so a reading can be smoothed, signed, packed for a metered link, and published without leaving the language you started in. A single file of conformance vectors, generated from the Rust implementation, is asserted by every binding's test suite, so the four cannot quietly disagree about what the same call returns.

## Standards and conformance

Anything defined by a published standard is implemented from the authoritative specification itself, and its tests are anchored to that specification's own reference vectors. Bit layouts, field orders, reserved bits, and algorithm constants are where the subtle bugs hide, and a plausible guess is worse than none.

| Area | Anchored to |
| --- | --- |
| Crypto | FIPS-197 (AES-128), RFC 4493 (AES-CMAC), FIPS-180 (SHA-256), RFC 2104 and RFC 4231 (HMAC-SHA256), RFC 5869 (HKDF), RFC 7748 (X25519), RFC 8439 (ChaCha20-Poly1305) |
| Messaging | MQTT topic and wildcard rules, RFC 7252 and RFC 7641 (CoAP with observe) |
| Radio and mesh | LoRaWAN 1.0.x MAC framing and OTAA join, LoRa time-on-air and duty cycle, CRC-16/CCITT frames |
| Field I/O | RFC 1055 (SLIP) and COBS, CRC-16/MODBUS, CAN 2.0 and CAN-FD with SAE J1939, NXP UM10204 (I2C) |
| Drones | MAVLink v1/v2 framing, CRC-16/MCRF4XX, per-message CRC_EXTRA, MAVLink 2 signing |
| Robotics | ROS 2 names, RIHS01 type hashes, CDR encoding, rmw_zenoh key expressions |

That rigor is also what makes dependency upgrades safe to take. When the primitives underneath change, every vector still matches or the build fails.

A second set of vectors, in `conformance/`, does the same job across languages rather than against a specification: generated from the Rust implementation and asserted by every binding's test suite, so a facade that drifts fails instead of quietly returning something else.

## Architecture

Every domain capability is a separate crate behind a trait defined in the core. The core knows about `Transport`, `Device`, `Sensor`, `Actuator`, `Store`, and the event bus; it knows nothing about MQTT or CAN specifically. Concrete crates implement those traits and are pulled in only when needed, so nobody pays for what they do not use, and on a microcontroller you compile in two crates and nothing else.

This separation is literal in Rust: `pamoja-core` defines the traits, and each transport (`pamoja-mqtt`, `pamoja-coap`) is its own crate, so Rust code pulls `MqttTransport` from `pamoja-mqtt`, not from the core. The language bindings are heading to the same shape, with capability-scoped packages (`@pamoja/mqtt`, `pamoja-mqtt`, `Pamoja.Mqtt`) sitting next to the core package. Today, while the polyglot release pipeline is being proven end to end with a single capability, that first transport ships inside each language's `core` package, so the bindings import `MqttClient` from core for now. Splitting the bindings into scoped packages is on the roadmap.

```
        bindings (two tiers: generated contract + hand-written facade)
   npm @pamoja/*   PyPI pamoja-*   NuGet Pamoja.*   Lua / WASM / Kotlin / Swift
        |                |               |                    |
        +----------------+---------------+--------------------+
                                  |
                         +--------+--------+   async runtime, device model,
                         |   pamoja-core   |   event bus, error model, codecs
                         +--------+--------+
                                  |  trait-based abstraction layer
   messaging   hardware I/O   robotics    drones    security   resilience   power
   mqtt/coap   serial/can/    ros2/       mavlink   tls/       store-and-   duty-
   lora/mesh   gpio/rs485     zenoh                 identity   forward      cycling
```

## Roadmap

Messaging and radio. MQTT and CoAP work today, behind a cost-aware transport ladder that tries the cheapest link first and buffers when there is none. LoRa and LoRaWAN long-range radio, and a CRC-checked mesh frame with reverse-path routing, now ship as further rungs. Next: the cheap-radio drivers they ride on (ESP-NOW, nRF24), a Meshtastic bridge for off-grid networks, and cellular and satellite uplinks for the most remote telemetry.

Hardware and sensors. Serial (SLIP/COBS), CAN with J1939, RS485/Modbus, and on-board GPIO/I2C/SPI ship today for field wiring, alongside datasheet-anchored decoders for common, salvageable parts (BME280, DS18B20, INA219, ADS1115) and actuator drivers (PCA9685 PWM/servo, stepper). You can also instantiate a node by name with a device profile (an irrigation node, a well-level monitor) instead of wiring pins. Next: a broader driver catalog.

Resilience and power. Offline-first store-and-forward, energy-aware duty cycling for solar and battery, and a local-first dashboard a node serves over its own hotspot - multilingual, fully offline, with a hardware-free mock - all work today. The dashboard now also renders custom sensors and node stats a profile declares, and reaches any phone over the gateway's own WiFi while the radio mesh carries the data behind it - the shape of a pre-flashed field kit. Next is data-mule sync for places with no link at all.

Robotics and drones. A ROS 2 bridge - topics, services, and actions - over a Zenoh transport ships today, interoperating with rmw_zenoh, routerless; the kit adds wheel kinematics, odometry, waypoint guidance, motion safety, and arm forward/inverse kinematics, and a simulated robot exercises it all with no hardware. MAVLink ships too: a vehicle is an ordinary pamoja device you arm, command, and fly missions through, over real serial, UDP, and TCP links, with the whole path verified in CI against real ArduPilot and PX4 SITL. Next: multi-device fleet orchestration, and the vehicle model surfaced through the language bindings.

Security. Memory safety by construction today, with ed25519 device identity, a tamper-evident hash-chained audit log, and a secured channel (X25519 key agreement and ChaCha20-Poly1305 with anti-replay) already shipping. Signed firmware updates ship too: a device checks a release against the manifest its author signed, stages it beside the running image, and falls back on its own if the new one never reports healthy. Because the signature reaches the image through the manifest's digest, an update is safe to carry over a link nobody trusts - a radio mesh, a passing phone, a USB stick. Next: TLS 1.3 and DTLS, X.509 device identity, then attestation and delta updates.

Reach. Python and C#/.NET ship alongside Node today, each with the same capability set behind a facade written in that language's idiom and held to shared conformance vectors. Next: Lua, WebAssembly, Kotlin, Swift, and Go. The plain-language helper layer (`pamoja-kit`) is broad today - smooth a noisy reading, hold a value with a PID, warn before a tank runs dry, steer by wheel kinematics - each naming the goal over the math with the real algorithm one layer down. And an offline-first community cookbook so the SDK reaches the people it is built for.

## Repository layout

```
crates/      Rust engine and capability crates (each crate's README is its doc landing)
bindings/    per-language bindings (Node, Python, .NET today; more to come)
examples/    runnable end-to-end scenarios, including a cross-crate conformance test
conformance/ the vectors every binding asserts, so the languages cannot disagree
docs/        a generated API index linking each crate's README (cargo xtask docs)
sitl/        ArduPilot and PX4 SITL images for the MAVLink interop job
web/         the showcase site and the hosted dashboard demo
assets/      brand and logo
```

Device and transport simulators live in `pamoja-sim` and `pamoja-loopback`, so the examples and tests run with no hardware.

## Building from source

```sh
cargo build --workspace      # build the engine and capability crates
cargo test --workspace       # run tests, including doctests and the MQTT round-trip

cd bindings/node
npm install && npm run build  # build the native addon and the TypeScript facade
npm test                      # smoke-test the binding

cd ../python
python -m venv .venv && . .venv/bin/activate
pip install maturin pytest && maturin develop  # build the extension and install the facade
pytest                                          # smoke-test the binding

cd ../..
cargo build -p pamoja-ffi --release                       # build the native C ABI and refresh pamoja.h
dotnet build bindings/dotnet/Pamoja.Core.sln -c Release    # build the .NET interop and facade
dotnet run --project bindings/dotnet/tests/Pamoja.Core.Smoke -c Release  # smoke-test the binding
```

The local toolchain needs no extra components; formatting and clippy run in CI.

## Contributing

Contributions are welcome. [CONTRIBUTING.md](CONTRIBUTING.md) covers how to build, test,
and submit changes, and the conventions the code holds to (documented public items, the
`no_std` constraint, standards-anchored tests). To report a security issue, see
[SECURITY.md](SECURITY.md); please report vulnerabilities privately rather than in a
public issue.

## License

Released under the [MIT License](LICENSE-MIT). Free to use, with no legal or financial barrier, because cost should never be the reason a good idea does not get built.
