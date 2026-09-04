# Install

`pamoja` is the whole framework in one package, in every language:

```sh
cargo add pamoja                 # Rust
npm install pamoja               # TypeScript and Node
pip install pamoja               # Python
dotnet add package Pamoja        # C# and .NET
```

That is the right default. Every capability is also its own package, and the
sections below list them, but what you gain by picking them differs between
Rust and the bindings. It is worth knowing which before you choose.

Every crate, package, and binding shares one version and is released together,
so `0.1.15` of any package wraps `0.1.15` of every other. The
[changelog](https://github.com/molexxxx/pamoja/blob/main/CHANGELOG.md) covers
all of them in one entry.

## What picking packages changes

In Rust, it changes what gets compiled. A crate you do not name is never built,
and its dependencies are never fetched, so a narrow build is genuinely smaller
and carries less third-party code.

In the bindings it changes what you import, not what you download. Node, Python,
and .NET each load one compiled engine that carries every capability, and every
package depends on it. Choosing packages narrows the API you see, the manifest
you ship, and the code your dependency scanners have to account for. It does not
shrink the engine.

Neither is a workaround. Compiling only what you use is a property of a compiled
language, and the deployments that need it (a microcontroller with kilobytes of
flash) run Rust and could not host a Python or .NET runtime at all.

## Rust

```sh
cargo add pamoja                        # every capability, behind a feature each
cargo add pamoja-modbus                 # or one crate on its own
```

<!-- snippet: examples/tests/guides/imports.rs#rust -->
From [`examples/tests/guides/imports.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/imports.rs):

```rust
use pamoja::modbus::Adu; // the same type as pamoja_modbus::Adu
use pamoja_codec::CborCodec;
```
<!-- end -->

Each module of `pamoja` is the crate of the same name, so the two ways in share
one API and one set of documentation, and code moves between them unchanged.

Naming features is what makes a build small. Measured from the resolved
dependency graph, for a `x86_64-unknown-linux-gnu` build:

<!-- table: builds -->
| Build | What you write | Crates compiled | From this workspace | External |
| --- | --- | --- | --- | --- |
| Every capability | `cargo add pamoja` | 107 | 31 | 76 |
| Codecs and identity | `--features codec,security` | 36 | 4 | 32 |
| Field I/O | `--features field-io` | 6 | 6 | 0 |
| One capability | `--features modbus` | 3 | 3 | 0 |
| Bare metal, no `std` | `--features modbus,sensors,lora` | 5 | 5 | 0 |
<!-- end -->

`field-io` there is a group feature. Six domains have one, so a build names a
domain rather than listing its parts:

<!-- table: domains rust -->
```sh
cargo add pamoja --features field-io    # Field I/O
cargo add pamoja --features sensing     # Sensing and actuation
cargo add pamoja --features radio       # Radio and reach
cargo add pamoja --features trust       # Trust and operation
cargo add pamoja --features transports  # Transports and testing
cargo add pamoja --features profiles    # Profiles and robotics
```
<!-- end -->

The narrow builds carry no third-party code at all: `pamoja`, `pamoja-core`, and
the capability crates, and nothing else. Most capability crates are `no_std`, so
the same code runs on a gateway and on a microcontroller. The
[Rust reference](reference/rust.md) lists every crate.

## TypeScript and Node

```sh
npm install pamoja                            # every capability
npm install @pamoja/modbus @pamoja/codec      # or only the packages you use
```

```ts
import { readHoldingRegisters } from '@pamoja/modbus'
import { toCbor, fromCbor } from '@pamoja/codec'
```

Every package depends on `@pamoja/native`, the compiled engine, prebuilt for
Linux (x64, arm64), macOS (x64, arm64), and Windows (x64); npm picks the right
one. It is one binary carrying every capability whichever packages you install,
so the choice is about the API surface and your dependency manifest, not the
download. Node 16 or later.

The same six domains, as the packages that make them up. There is no domain
package: naming the capabilities keeps the manifest an honest record of what the
code uses, and `pamoja` is there when you would rather not choose at all.

<!-- table: domains node -->
```sh
npm install @pamoja/serial @pamoja/modbus @pamoja/can @pamoja/gpio                                          # Field I/O
npm install @pamoja/sensors @pamoja/actuators                                                               # Sensing and actuation
npm install @pamoja/lora @pamoja/lorawan @pamoja/mesh @pamoja/routing                                       # Radio and reach
npm install @pamoja/audit @pamoja/session @pamoja/update @pamoja/power @pamoja/telemetry                    # Trust and operation
npm install @pamoja/mqtt @pamoja/coap @pamoja/loopback @pamoja/sync @pamoja/ladder @pamoja/bus @pamoja/sim  # Transports and testing
npm install @pamoja/profile @pamoja/ros2 @pamoja/zenoh                                                      # Profiles and robotics
```
<!-- end -->

<!-- table: binding node -->
| Group | Capability | Import | What it covers |
| --- | --- | --- | --- |
| **Engine** | Engine surface | `@pamoja/core` | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| **Identity** | Device identity | `@pamoja/security` | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| **Codecs** | Codecs | `@pamoja/codec` | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| **Helpers** | Helpers | `@pamoja/kit` | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| **Field I/O** | Serial framing | `@pamoja/serial` | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
|  | Modbus RTU | `@pamoja/modbus` | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
|  | CAN and J1939 | `@pamoja/can` | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
|  | I2C, SPI, and GPIO | `@pamoja/gpio` | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| **Sensing and actuation** | Sensor drivers | `@pamoja/sensors` | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
|  | Actuator drivers | `@pamoja/actuators` | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| **Radio and reach** | LoRa airtime | `@pamoja/lora` | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
|  | LoRaWAN | `@pamoja/lorawan` | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
|  | Mesh frames | `@pamoja/mesh` | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
|  | Routing | `@pamoja/routing` | Reverse-path routing that learns the cheapest route from overheard traffic |
| **MAVLink** | MAVLink | `@pamoja/mavlink` | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| **Trust and operation** | Audit log | `@pamoja/audit` | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
|  | Secured session | `@pamoja/session` | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
|  | Signed updates | `@pamoja/update` | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
|  | Power | `@pamoja/power` | Duty cycling and an energy-aware governor that stretches work as the battery drains |
|  | Telemetry | `@pamoja/telemetry` | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| **Transports and testing** | MQTT | `@pamoja/mqtt` | An MQTT client with the topic and wildcard rules, as the core transport |
|  | CoAP | `@pamoja/coap` | A CoAP client over UDP with confirmable delivery and observe |
|  | Loopback | `@pamoja/loopback` | An in-process transport with topic matching and a fault injector, for testing with no broker |
|  | Store and forward | `@pamoja/sync` | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
|  | Transport ladder | `@pamoja/ladder` | Cheapest reachable link first, buffering to a store when every link is down |
|  | Event bus | `@pamoja/bus` | An in-memory typed publish and subscribe event bus |
|  | Simulators | `@pamoja/sim` | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| **Profiles and robotics** | Device profiles | `@pamoja/profile` | Named, ready-to-run device profiles from plain data or a JSON manifest |
|  | ROS 2 rules | `@pamoja/ros2` | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
|  | Zenoh keys | `@pamoja/zenoh` | Zenoh key expressions: validity, canonical form, and wildcard matching |
<!-- end -->

## Python

```sh
pip install pamoja                          # every capability
pip install pamoja-modbus pamoja-codec      # or only the distributions you use
```

```python
from pamoja.modbus import read_holding_registers
from pamoja.codec import to_cbor, from_cbor
```

`pamoja` is a namespace package: each distribution ships one `pamoja.<name>`
module and they merge on import. Every distribution depends on `pamoja-native`,
the compiled engine, with wheels for the same platforms as the Node engine and
for Python 3.10 and later; elsewhere `pip` builds it from the sdist, which needs
a Rust toolchain.

The same six domains, as the packages that make them up. There is no domain
package: naming the capabilities keeps the manifest an honest record of what the
code uses, and `pamoja` is there when you would rather not choose at all.

<!-- table: domains python -->
```sh
pip install pamoja-serial pamoja-modbus pamoja-can pamoja-gpio                                       # Field I/O
pip install pamoja-sensors pamoja-actuators                                                          # Sensing and actuation
pip install pamoja-lora pamoja-lorawan pamoja-mesh pamoja-routing                                    # Radio and reach
pip install pamoja-audit pamoja-session pamoja-update pamoja-power pamoja-telemetry                  # Trust and operation
pip install pamoja-mqtt pamoja-coap pamoja-loopback pamoja-sync pamoja-ladder pamoja-bus pamoja-sim  # Transports and testing
pip install pamoja-profile pamoja-ros2 pamoja-zenoh                                                  # Profiles and robotics
```
<!-- end -->

<!-- table: binding python -->
| Group | Capability | Module | What it covers |
| --- | --- | --- | --- |
| **Engine** | Engine surface | `pamoja.core` | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| **Identity** | Device identity | `pamoja.security` | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| **Codecs** | Codecs | `pamoja.codec` | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| **Helpers** | Helpers | `pamoja.kit` | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| **Field I/O** | Serial framing | `pamoja.serial` | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
|  | Modbus RTU | `pamoja.modbus` | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
|  | CAN and J1939 | `pamoja.can` | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
|  | I2C, SPI, and GPIO | `pamoja.gpio` | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| **Sensing and actuation** | Sensor drivers | `pamoja.sensors` | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
|  | Actuator drivers | `pamoja.actuators` | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| **Radio and reach** | LoRa airtime | `pamoja.lora` | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
|  | LoRaWAN | `pamoja.lorawan` | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
|  | Mesh frames | `pamoja.mesh` | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
|  | Routing | `pamoja.routing` | Reverse-path routing that learns the cheapest route from overheard traffic |
| **MAVLink** | MAVLink | `pamoja.mavlink` | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| **Trust and operation** | Audit log | `pamoja.audit` | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
|  | Secured session | `pamoja.session` | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
|  | Signed updates | `pamoja.update` | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
|  | Power | `pamoja.power` | Duty cycling and an energy-aware governor that stretches work as the battery drains |
|  | Telemetry | `pamoja.telemetry` | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| **Transports and testing** | MQTT | `pamoja.mqtt` | An MQTT client with the topic and wildcard rules, as the core transport |
|  | CoAP | `pamoja.coap` | A CoAP client over UDP with confirmable delivery and observe |
|  | Loopback | `pamoja.loopback` | An in-process transport with topic matching and a fault injector, for testing with no broker |
|  | Store and forward | `pamoja.sync` | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
|  | Transport ladder | `pamoja.ladder` | Cheapest reachable link first, buffering to a store when every link is down |
|  | Event bus | `pamoja.bus` | An in-memory typed publish and subscribe event bus |
|  | Simulators | `pamoja.sim` | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| **Profiles and robotics** | Device profiles | `pamoja.profile` | Named, ready-to-run device profiles from plain data or a JSON manifest |
|  | ROS 2 rules | `pamoja.ros2` | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
|  | Zenoh keys | `pamoja.zenoh` | Zenoh key expressions: validity, canonical form, and wildcard matching |
<!-- end -->

## C# and .NET

```sh
dotnet add package Pamoja                        # every capability
dotnet add package Pamoja.Modbus Pamoja.Codec    # or only the packages you use
```

```csharp
using Pamoja.Modbus;
using Pamoja.Codec;
```

Each package is one namespace of the same name. Every package depends on
`Pamoja.Native`, which carries the native library for `win-x64`, `linux-x64`,
`linux-arm64`, `osx-x64`, and `osx-arm64`, and targets .NET 8.

The same six domains, as the packages that make them up. There is no domain
package: naming the capabilities keeps the manifest an honest record of what the
code uses, and `pamoja` is there when you would rather not choose at all.

<!-- table: domains dotnet -->
```sh
dotnet add package Pamoja.Serial Pamoja.Modbus Pamoja.Can Pamoja.Gpio                                       # Field I/O
dotnet add package Pamoja.Sensors Pamoja.Actuators                                                          # Sensing and actuation
dotnet add package Pamoja.Lora Pamoja.Lorawan Pamoja.Mesh Pamoja.Routing                                    # Radio and reach
dotnet add package Pamoja.Audit Pamoja.Session Pamoja.Update Pamoja.Power Pamoja.Telemetry                  # Trust and operation
dotnet add package Pamoja.Mqtt Pamoja.Coap Pamoja.Loopback Pamoja.Sync Pamoja.Ladder Pamoja.Bus Pamoja.Sim  # Transports and testing
dotnet add package Pamoja.Profile Pamoja.Ros2 Pamoja.Zenoh                                                  # Profiles and robotics
```
<!-- end -->

<!-- table: binding dotnet -->
| Group | Capability | Package | What it covers |
| --- | --- | --- | --- |
| **Engine** | Engine surface | `Pamoja.Core` | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| **Identity** | Device identity | `Pamoja.Security` | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| **Codecs** | Codecs | `Pamoja.Codec` | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| **Helpers** | Helpers | `Pamoja.Kit` | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| **Field I/O** | Serial framing | `Pamoja.Serial` | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
|  | Modbus RTU | `Pamoja.Modbus` | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
|  | CAN and J1939 | `Pamoja.Can` | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
|  | I2C, SPI, and GPIO | `Pamoja.Gpio` | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| **Sensing and actuation** | Sensor drivers | `Pamoja.Sensors` | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
|  | Actuator drivers | `Pamoja.Actuators` | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| **Radio and reach** | LoRa airtime | `Pamoja.Lora` | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
|  | LoRaWAN | `Pamoja.Lorawan` | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
|  | Mesh frames | `Pamoja.Mesh` | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
|  | Routing | `Pamoja.Routing` | Reverse-path routing that learns the cheapest route from overheard traffic |
| **MAVLink** | MAVLink | `Pamoja.Mavlink` | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| **Trust and operation** | Audit log | `Pamoja.Audit` | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
|  | Secured session | `Pamoja.Session` | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
|  | Signed updates | `Pamoja.Update` | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
|  | Power | `Pamoja.Power` | Duty cycling and an energy-aware governor that stretches work as the battery drains |
|  | Telemetry | `Pamoja.Telemetry` | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| **Transports and testing** | MQTT | `Pamoja.Mqtt` | An MQTT client with the topic and wildcard rules, as the core transport |
|  | CoAP | `Pamoja.Coap` | A CoAP client over UDP with confirmable delivery and observe |
|  | Loopback | `Pamoja.Loopback` | An in-process transport with topic matching and a fault injector, for testing with no broker |
|  | Store and forward | `Pamoja.Sync` | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
|  | Transport ladder | `Pamoja.Ladder` | Cheapest reachable link first, buffering to a store when every link is down |
|  | Event bus | `Pamoja.Bus` | An in-memory typed publish and subscribe event bus |
|  | Simulators | `Pamoja.Sim` | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| **Profiles and robotics** | Device profiles | `Pamoja.Profile` | Named, ready-to-run device profiles from plain data or a JSON manifest |
|  | ROS 2 rules | `Pamoja.Ros2` | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
|  | Zenoh keys | `Pamoja.Zenoh` | Zenoh key expressions: validity, canonical form, and wildcard matching |
<!-- end -->
