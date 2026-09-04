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

## The two things called core

A binding has one package you never name and one you sometimes do, and they are
easy to confuse.

The **compiled engine** is `@pamoja/native`, `pamoja-native`, and
`Pamoja.Native`. It is the built Rust library, the generated contract over it,
and the plumbing every facade needs to call it: the handle type, the error every
failed call raises, and string marshalling. Every package declares it, so it
arrives on its own and you never install it by hand. Rust has no equivalent,
because there you compile the crates.

The **engine surface** is `@pamoja/core`, `pamoja-core`, and `Pamoja.Core`, the
counterpart of the `pamoja-core` crate. It is the runtime version and
`Transport`, the abstraction MQTT, CoAP, and the loopback all implement. It is a
capability like any other, listed first in the tables below, and most packages do
not depend on it: only the transports do, because they are the ones that return a
`Transport`. Install it when you want to hold a link behind that interface.

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

The same six domains, one package each. A domain package brings in its
capabilities and re-exports each under its own name, so a name two of them share
stays unambiguous:

<!-- table: domains node -->
```sh
npm install @pamoja/field-io    # Field I/O
npm install @pamoja/sensing     # Sensing and actuation
npm install @pamoja/radio       # Radio and reach
npm install @pamoja/trust       # Trust and operation
npm install @pamoja/transports  # Transports and testing
npm install @pamoja/profiles    # Profiles and robotics
```
<!-- end -->

<!-- table: binding node -->
| Group | Capability | Import | What it covers |
| --- | --- | --- | --- |
| **Engine** | [Engine surface](https://pamoja.molex.cloud/docs/guides/transport.html) | [`@pamoja/core`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_core.html) | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| **Identity** | [Device identity](https://pamoja.molex.cloud/docs/guides/security.html) | [`@pamoja/security`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_security.html) | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| **Codecs** | [Codecs](https://pamoja.molex.cloud/docs/guides/codec.html) | [`@pamoja/codec`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html) | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| **Helpers** | [Helpers](https://pamoja.molex.cloud/docs/guides/kit.html) | [`@pamoja/kit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html) | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| **Field I/O** | [Serial framing](https://pamoja.molex.cloud/docs/guides/serial.html) | [`@pamoja/serial`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html) | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
|  | [Modbus RTU](https://pamoja.molex.cloud/docs/guides/modbus.html) | [`@pamoja/modbus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html) | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
|  | [CAN and J1939](https://pamoja.molex.cloud/docs/guides/can.html) | [`@pamoja/can`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html) | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
|  | [I2C, SPI, and GPIO](https://pamoja.molex.cloud/docs/guides/gpio.html) | [`@pamoja/gpio`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html) | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| **Sensing and actuation** | [Sensor drivers](https://pamoja.molex.cloud/docs/guides/sensors.html) | [`@pamoja/sensors`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html) | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
|  | [Actuator drivers](https://pamoja.molex.cloud/docs/guides/actuators.html) | [`@pamoja/actuators`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html) | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| **Radio and reach** | [LoRa airtime](https://pamoja.molex.cloud/docs/guides/lora.html) | [`@pamoja/lora`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lora.html) | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
|  | [LoRaWAN](https://pamoja.molex.cloud/docs/guides/lorawan.html) | [`@pamoja/lorawan`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html) | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
|  | [Mesh frames](https://pamoja.molex.cloud/docs/guides/mesh.html) | [`@pamoja/mesh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html) | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
|  | [Routing](https://pamoja.molex.cloud/docs/guides/routing.html) | [`@pamoja/routing`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html) | Reverse-path routing that learns the cheapest route from overheard traffic |
| **MAVLink** | [MAVLink](https://pamoja.molex.cloud/docs/guides/mavlink.html) | [`@pamoja/mavlink`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html) | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| **Trust and operation** | [Audit log](https://pamoja.molex.cloud/docs/guides/audit.html) | [`@pamoja/audit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html) | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
|  | [Secured session](https://pamoja.molex.cloud/docs/guides/session.html) | [`@pamoja/session`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html) | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
|  | [Signed updates](https://pamoja.molex.cloud/docs/guides/update.html) | [`@pamoja/update`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html) | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
|  | [Power](https://pamoja.molex.cloud/docs/guides/power.html) | [`@pamoja/power`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html) | Duty cycling and an energy-aware governor that stretches work as the battery drains |
|  | [Telemetry](https://pamoja.molex.cloud/docs/guides/telemetry.html) | [`@pamoja/telemetry`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html) | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| **Transports and testing** | [MQTT](https://pamoja.molex.cloud/docs/guides/mqtt.html) | [`@pamoja/mqtt`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html) | An MQTT client with the topic and wildcard rules, as the core transport |
|  | [CoAP](https://pamoja.molex.cloud/docs/guides/coap.html) | [`@pamoja/coap`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html) | A CoAP client over UDP with confirmable delivery and observe |
|  | [Loopback](https://pamoja.molex.cloud/docs/guides/loopback.html) | [`@pamoja/loopback`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html) | An in-process transport with topic matching and a fault injector, for testing with no broker |
|  | [Store and forward](https://pamoja.molex.cloud/docs/guides/sync.html) | [`@pamoja/sync`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html) | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
|  | [Transport ladder](https://pamoja.molex.cloud/docs/guides/ladder.html) | [`@pamoja/ladder`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html) | Cheapest reachable link first, buffering to a store when every link is down |
|  | [Event bus](https://pamoja.molex.cloud/docs/guides/bus.html) | [`@pamoja/bus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html) | An in-memory typed publish and subscribe event bus |
|  | [Simulators](https://pamoja.molex.cloud/docs/guides/sim.html) | [`@pamoja/sim`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html) | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| **Profiles and robotics** | [Device profiles](https://pamoja.molex.cloud/docs/guides/profile.html) | [`@pamoja/profile`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html) | Named, ready-to-run device profiles from plain data or a JSON manifest |
|  | [ROS 2 rules](https://pamoja.molex.cloud/docs/guides/ros2.html) | [`@pamoja/ros2`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html) | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
|  | [Zenoh keys](https://pamoja.molex.cloud/docs/guides/zenoh.html) | [`@pamoja/zenoh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html) | Zenoh key expressions: validity, canonical form, and wildcard matching |
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

The same six domains, one package each. A domain package brings in its
capabilities and re-exports each under its own name, so a name two of them share
stays unambiguous:

<!-- table: domains python -->
```sh
pip install pamoja-field-io    # Field I/O
pip install pamoja-sensing     # Sensing and actuation
pip install pamoja-radio       # Radio and reach
pip install pamoja-trust       # Trust and operation
pip install pamoja-transports  # Transports and testing
pip install pamoja-profiles    # Profiles and robotics
```
<!-- end -->

<!-- table: binding python -->
| Group | Capability | Module | What it covers |
| --- | --- | --- | --- |
| **Engine** | [Engine surface](https://pamoja.molex.cloud/docs/guides/transport.html) | [`pamoja.core`](https://pamoja.molex.cloud/docs/reference/python/pamoja/core.html) | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| **Identity** | [Device identity](https://pamoja.molex.cloud/docs/guides/security.html) | [`pamoja.security`](https://pamoja.molex.cloud/docs/reference/python/pamoja/security.html) | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| **Codecs** | [Codecs](https://pamoja.molex.cloud/docs/guides/codec.html) | [`pamoja.codec`](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html) | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| **Helpers** | [Helpers](https://pamoja.molex.cloud/docs/guides/kit.html) | [`pamoja.kit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html) | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| **Field I/O** | [Serial framing](https://pamoja.molex.cloud/docs/guides/serial.html) | [`pamoja.serial`](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html) | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
|  | [Modbus RTU](https://pamoja.molex.cloud/docs/guides/modbus.html) | [`pamoja.modbus`](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html) | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
|  | [CAN and J1939](https://pamoja.molex.cloud/docs/guides/can.html) | [`pamoja.can`](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html) | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
|  | [I2C, SPI, and GPIO](https://pamoja.molex.cloud/docs/guides/gpio.html) | [`pamoja.gpio`](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html) | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| **Sensing and actuation** | [Sensor drivers](https://pamoja.molex.cloud/docs/guides/sensors.html) | [`pamoja.sensors`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sensors.html) | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
|  | [Actuator drivers](https://pamoja.molex.cloud/docs/guides/actuators.html) | [`pamoja.actuators`](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html) | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| **Radio and reach** | [LoRa airtime](https://pamoja.molex.cloud/docs/guides/lora.html) | [`pamoja.lora`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lora.html) | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
|  | [LoRaWAN](https://pamoja.molex.cloud/docs/guides/lorawan.html) | [`pamoja.lorawan`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html) | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
|  | [Mesh frames](https://pamoja.molex.cloud/docs/guides/mesh.html) | [`pamoja.mesh`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mesh.html) | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
|  | [Routing](https://pamoja.molex.cloud/docs/guides/routing.html) | [`pamoja.routing`](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html) | Reverse-path routing that learns the cheapest route from overheard traffic |
| **MAVLink** | [MAVLink](https://pamoja.molex.cloud/docs/guides/mavlink.html) | [`pamoja.mavlink`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html) | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| **Trust and operation** | [Audit log](https://pamoja.molex.cloud/docs/guides/audit.html) | [`pamoja.audit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html) | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
|  | [Secured session](https://pamoja.molex.cloud/docs/guides/session.html) | [`pamoja.session`](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html) | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
|  | [Signed updates](https://pamoja.molex.cloud/docs/guides/update.html) | [`pamoja.update`](https://pamoja.molex.cloud/docs/reference/python/pamoja/update.html) | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
|  | [Power](https://pamoja.molex.cloud/docs/guides/power.html) | [`pamoja.power`](https://pamoja.molex.cloud/docs/reference/python/pamoja/power.html) | Duty cycling and an energy-aware governor that stretches work as the battery drains |
|  | [Telemetry](https://pamoja.molex.cloud/docs/guides/telemetry.html) | [`pamoja.telemetry`](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html) | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| **Transports and testing** | [MQTT](https://pamoja.molex.cloud/docs/guides/mqtt.html) | [`pamoja.mqtt`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mqtt.html) | An MQTT client with the topic and wildcard rules, as the core transport |
|  | [CoAP](https://pamoja.molex.cloud/docs/guides/coap.html) | [`pamoja.coap`](https://pamoja.molex.cloud/docs/reference/python/pamoja/coap.html) | A CoAP client over UDP with confirmable delivery and observe |
|  | [Loopback](https://pamoja.molex.cloud/docs/guides/loopback.html) | [`pamoja.loopback`](https://pamoja.molex.cloud/docs/reference/python/pamoja/loopback.html) | An in-process transport with topic matching and a fault injector, for testing with no broker |
|  | [Store and forward](https://pamoja.molex.cloud/docs/guides/sync.html) | [`pamoja.sync`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html) | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
|  | [Transport ladder](https://pamoja.molex.cloud/docs/guides/ladder.html) | [`pamoja.ladder`](https://pamoja.molex.cloud/docs/reference/python/pamoja/ladder.html) | Cheapest reachable link first, buffering to a store when every link is down |
|  | [Event bus](https://pamoja.molex.cloud/docs/guides/bus.html) | [`pamoja.bus`](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html) | An in-memory typed publish and subscribe event bus |
|  | [Simulators](https://pamoja.molex.cloud/docs/guides/sim.html) | [`pamoja.sim`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sim.html) | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| **Profiles and robotics** | [Device profiles](https://pamoja.molex.cloud/docs/guides/profile.html) | [`pamoja.profile`](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html) | Named, ready-to-run device profiles from plain data or a JSON manifest |
|  | [ROS 2 rules](https://pamoja.molex.cloud/docs/guides/ros2.html) | [`pamoja.ros2`](https://pamoja.molex.cloud/docs/reference/python/pamoja/ros2.html) | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
|  | [Zenoh keys](https://pamoja.molex.cloud/docs/guides/zenoh.html) | [`pamoja.zenoh`](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html) | Zenoh key expressions: validity, canonical form, and wildcard matching |
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

A domain package there brings in its capabilities and ships no assembly of its
own, since C# has no way to re-export a namespace, so a type is named the way it
is when its package is referenced directly.

Each package is one namespace of the same name. Every package depends on
`Pamoja.Native`, which carries the native library for `win-x64`, `linux-x64`,
`linux-arm64`, `osx-x64`, and `osx-arm64`, and targets .NET 8.

The same six domains, one package each. A domain package brings in its
capabilities and re-exports each under its own name, so a name two of them share
stays unambiguous:

<!-- table: domains dotnet -->
```sh
dotnet add package Pamoja.FieldIo     # Field I/O
dotnet add package Pamoja.Sensing     # Sensing and actuation
dotnet add package Pamoja.Radio       # Radio and reach
dotnet add package Pamoja.Trust       # Trust and operation
dotnet add package Pamoja.Transports  # Transports and testing
dotnet add package Pamoja.Profiles    # Profiles and robotics
```
<!-- end -->

<!-- table: binding dotnet -->
| Group | Capability | Package | What it covers |
| --- | --- | --- | --- |
| **Engine** | [Engine surface](https://pamoja.molex.cloud/docs/guides/transport.html) | [`Pamoja.Core`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html) | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| **Identity** | [Device identity](https://pamoja.molex.cloud/docs/guides/security.html) | [`Pamoja.Security`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Security.html) | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| **Codecs** | [Codecs](https://pamoja.molex.cloud/docs/guides/codec.html) | [`Pamoja.Codec`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html) | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| **Helpers** | [Helpers](https://pamoja.molex.cloud/docs/guides/kit.html) | [`Pamoja.Kit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html) | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| **Field I/O** | [Serial framing](https://pamoja.molex.cloud/docs/guides/serial.html) | [`Pamoja.Serial`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.html) | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
|  | [Modbus RTU](https://pamoja.molex.cloud/docs/guides/modbus.html) | [`Pamoja.Modbus`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html) | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
|  | [CAN and J1939](https://pamoja.molex.cloud/docs/guides/can.html) | [`Pamoja.Can`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html) | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
|  | [I2C, SPI, and GPIO](https://pamoja.molex.cloud/docs/guides/gpio.html) | [`Pamoja.Gpio`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html) | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| **Sensing and actuation** | [Sensor drivers](https://pamoja.molex.cloud/docs/guides/sensors.html) | [`Pamoja.Sensors`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.html) | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
|  | [Actuator drivers](https://pamoja.molex.cloud/docs/guides/actuators.html) | [`Pamoja.Actuators`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html) | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| **Radio and reach** | [LoRa airtime](https://pamoja.molex.cloud/docs/guides/lora.html) | [`Pamoja.Lora`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lora.html) | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
|  | [LoRaWAN](https://pamoja.molex.cloud/docs/guides/lorawan.html) | [`Pamoja.Lorawan`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.html) | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
|  | [Mesh frames](https://pamoja.molex.cloud/docs/guides/mesh.html) | [`Pamoja.Mesh`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.html) | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
|  | [Routing](https://pamoja.molex.cloud/docs/guides/routing.html) | [`Pamoja.Routing`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html) | Reverse-path routing that learns the cheapest route from overheard traffic |
| **MAVLink** | [MAVLink](https://pamoja.molex.cloud/docs/guides/mavlink.html) | [`Pamoja.Mavlink`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html) | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| **Trust and operation** | [Audit log](https://pamoja.molex.cloud/docs/guides/audit.html) | [`Pamoja.Audit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html) | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
|  | [Secured session](https://pamoja.molex.cloud/docs/guides/session.html) | [`Pamoja.Session`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html) | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
|  | [Signed updates](https://pamoja.molex.cloud/docs/guides/update.html) | [`Pamoja.Update`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.html) | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
|  | [Power](https://pamoja.molex.cloud/docs/guides/power.html) | [`Pamoja.Power`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.html) | Duty cycling and an energy-aware governor that stretches work as the battery drains |
|  | [Telemetry](https://pamoja.molex.cloud/docs/guides/telemetry.html) | [`Pamoja.Telemetry`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html) | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| **Transports and testing** | [MQTT](https://pamoja.molex.cloud/docs/guides/mqtt.html) | [`Pamoja.Mqtt`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html) | An MQTT client with the topic and wildcard rules, as the core transport |
|  | [CoAP](https://pamoja.molex.cloud/docs/guides/coap.html) | [`Pamoja.Coap`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Coap.html) | A CoAP client over UDP with confirmable delivery and observe |
|  | [Loopback](https://pamoja.molex.cloud/docs/guides/loopback.html) | [`Pamoja.Loopback`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html) | An in-process transport with topic matching and a fault injector, for testing with no broker |
|  | [Store and forward](https://pamoja.molex.cloud/docs/guides/sync.html) | [`Pamoja.Sync`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html) | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
|  | [Transport ladder](https://pamoja.molex.cloud/docs/guides/ladder.html) | [`Pamoja.Ladder`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html) | Cheapest reachable link first, buffering to a store when every link is down |
|  | [Event bus](https://pamoja.molex.cloud/docs/guides/bus.html) | [`Pamoja.Bus`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html) | An in-memory typed publish and subscribe event bus |
|  | [Simulators](https://pamoja.molex.cloud/docs/guides/sim.html) | [`Pamoja.Sim`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html) | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| **Profiles and robotics** | [Device profiles](https://pamoja.molex.cloud/docs/guides/profile.html) | [`Pamoja.Profile`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html) | Named, ready-to-run device profiles from plain data or a JSON manifest |
|  | [ROS 2 rules](https://pamoja.molex.cloud/docs/guides/ros2.html) | [`Pamoja.Ros2`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.html) | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
|  | [Zenoh keys](https://pamoja.molex.cloud/docs/guides/zenoh.html) | [`Pamoja.Zenoh`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html) | Zenoh key expressions: validity, canonical form, and wildcard matching |
<!-- end -->
