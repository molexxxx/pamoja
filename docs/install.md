# Install

`pamoja` is the whole framework in one package, in every language. Install it
when you want everything, or install only the packages you use: every
capability is its own package, and the lists below say which.

```sh
cargo add pamoja                 # Rust
npm install pamoja               # TypeScript and Node
pip install pamoja               # Python
dotnet add package Pamoja        # C# and .NET
```

Every crate, package, and binding shares one version and is released together,
so `0.1.15` of any package wraps `0.1.15` of every other. The
[changelog](https://github.com/molexxxx/pamoja/blob/main/CHANGELOG.md) covers
all of them in one entry.

## Rust

```sh
cargo add pamoja                        # every capability, behind a feature each
cargo add pamoja-core pamoja-codec      # only the crates you use
```

Most capability crates are `no_std`, so the same code runs on a gateway and on
a microcontroller, and a build carries only the crates it names. The
[Rust reference](reference/rust.md) lists every crate.

## TypeScript and Node

```sh
npm install pamoja                            # every capability
npm install @pamoja/security @pamoja/codec   # only the packages you use
```

```ts
import { DeviceIdentity } from '@pamoja/security'
import { toCbor, fromCbor } from '@pamoja/codec'
```

Every package depends on `@pamoja/native`, the compiled engine, prebuilt for
Linux (x64, arm64), macOS (x64, arm64), and Windows (x64); npm picks the right
one. It is one binary that carries every capability whichever packages you
install, so picking packages narrows the API and the dependencies you take on,
not the download. Node 16 or later.

<!-- table: binding node -->
| Capability | Import | What it covers |
| --- | --- | --- |
| Device identity | `@pamoja/security` | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| Codecs | `@pamoja/codec` | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| Helpers | `@pamoja/kit` | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| Serial framing | `@pamoja/serial` | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
| Modbus RTU | `@pamoja/modbus` | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
| CAN and J1939 | `@pamoja/can` | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
| I2C, SPI, and GPIO | `@pamoja/gpio` | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| Sensor drivers | `@pamoja/sensors` | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
| Actuator drivers | `@pamoja/actuators` | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| LoRa airtime | `@pamoja/lora` | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
| LoRaWAN | `@pamoja/lorawan` | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
| Mesh frames | `@pamoja/mesh` | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
| Routing | `@pamoja/routing` | Reverse-path routing that learns the cheapest route from overheard traffic |
| MAVLink | `@pamoja/mavlink` | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| Audit log | `@pamoja/audit` | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
| Secured session | `@pamoja/session` | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
| Signed updates | `@pamoja/update` | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
| Power | `@pamoja/power` | Duty cycling and an energy-aware governor that stretches work as the battery drains |
| Telemetry | `@pamoja/telemetry` | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| MQTT | `@pamoja/mqtt` | An MQTT client with the topic and wildcard rules, as the core transport |
| CoAP | `@pamoja/coap` | A CoAP client over UDP with confirmable delivery and observe |
| Loopback | `@pamoja/loopback` | An in-process transport with topic matching and a fault injector, for testing with no broker |
| Store and forward | `@pamoja/sync` | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
| Transport ladder | `@pamoja/ladder` | Cheapest reachable link first, buffering to a store when every link is down |
| Event bus | `@pamoja/bus` | An in-memory typed publish and subscribe event bus |
| Engine surface | `@pamoja/core` | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| Simulators | `@pamoja/sim` | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| Device profiles | `@pamoja/profile` | Named, ready-to-run device profiles from plain data or a JSON manifest |
| ROS 2 rules | `@pamoja/ros2` | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
| Zenoh keys | `@pamoja/zenoh` | Zenoh key expressions: validity, canonical form, and wildcard matching |
<!-- end -->

## Python

```sh
pip install pamoja                # every capability
pip install pamoja-core           # the engine's surface, and the capabilities as modules today
```

```python
from pamoja import security, codec
```

Wheels exist for the same platforms as the Node engine, for Python 3.10 and
later; on any other platform `pip` builds the extension from the sdist, which
needs a Rust toolchain.

<!-- table: binding python -->
| Capability | Module | What it covers |
| --- | --- | --- |
| Device identity | `pamoja.security` | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| Codecs | `pamoja.codec` | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| Helpers | `pamoja.kit` | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| Serial framing | `pamoja.serial` | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
| Modbus RTU | `pamoja.modbus` | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
| CAN and J1939 | `pamoja.can` | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
| I2C, SPI, and GPIO | `pamoja.gpio` | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| Sensor drivers | `pamoja.sensors` | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
| Actuator drivers | `pamoja.actuators` | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| LoRa airtime | `pamoja.lora` | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
| LoRaWAN | `pamoja.lorawan` | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
| Mesh frames | `pamoja.mesh` | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
| Routing | `pamoja.routing` | Reverse-path routing that learns the cheapest route from overheard traffic |
| MAVLink | `pamoja.mavlink` | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| Audit log | `pamoja.audit` | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
| Secured session | `pamoja.session` | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
| Signed updates | `pamoja.update` | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
| Power | `pamoja.power` | Duty cycling and an energy-aware governor that stretches work as the battery drains |
| Telemetry | `pamoja.telemetry` | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| MQTT | `pamoja.mqtt` | An MQTT client with the topic and wildcard rules, as the core transport |
| CoAP | `pamoja.coap` | A CoAP client over UDP with confirmable delivery and observe |
| Loopback | `pamoja.loopback` | An in-process transport with topic matching and a fault injector, for testing with no broker |
| Store and forward | `pamoja.sync` | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
| Transport ladder | `pamoja.ladder` | Cheapest reachable link first, buffering to a store when every link is down |
| Event bus | `pamoja.bus` | An in-memory typed publish and subscribe event bus |
| Engine surface | `pamoja.transport` | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| Simulators | `pamoja.sim` | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| Device profiles | `pamoja.profile` | Named, ready-to-run device profiles from plain data or a JSON manifest |
| ROS 2 rules | `pamoja.ros2` | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
| Zenoh keys | `pamoja.zenoh` | Zenoh key expressions: validity, canonical form, and wildcard matching |
<!-- end -->

## C# and .NET

```sh
dotnet add package Pamoja         # every capability
dotnet add package Pamoja.Core    # the engine's surface, and the capabilities as types today
```

```csharp
using Pamoja.Core;
```

The package carries the native library for `win-x64`, `linux-x64`,
`linux-arm64`, `osx-x64`, and `osx-arm64` and targets .NET 8.

<!-- table: binding dotnet -->
| Capability | Types | What it covers |
| --- | --- | --- |
| Device identity | `DeviceIdentity` | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| Codecs | `Codec`, `Quantizer` | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| Helpers | `Kit`, `Smoother`, `Median`, `Kalman`, `Calibration`, `Pid`, `Ramp`, `Thermostat`, `Debounce`, `Boundary`, `Coordinate`, `Geofence`, `Window`, `Trend`, `Surge`, `Depletion`, `Anomaly` | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| Serial framing | `Serial`, `SlipDecoder`, `CobsDecoder` | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
| Modbus RTU | `Modbus`, `ModbusFrame` | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
| CAN and J1939 | `Can`, `CanFrame`, `J1939Message` | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
| I2C, SPI, and GPIO | `I2c`, `Spi`, `SpiClock`, `Pin`, `PinLevel`, `PinEdge`, `PinPolarity` | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| Sensor drivers | `Bme280`, `Bme280Calibration`, `Bme280Measurement`, `Ds18b20`, `Ds18b20Reading`, `Ina219`, `Ads1115`, `Ads1115Config` | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
| Actuator drivers | `Pca9685`, `Pwm`, `Stepper`, `StepDrive`, `StepDirection` | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| LoRa airtime | `LoraLink`, `LoraChannelPlan`, `LoraPlanBuilder`, `LoraPlanInfo`, `LoraRegion`, `LoraDataRate`, `LoraChannelBlock`, `LoraChannelSet`, `LoraSubBand`, `LoraRx2`, `LoraBeacon`, `LoraMaxPayload`, `LoraPayloadTable`, `LoraModulation`, `LoraDirection` | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
| LoRaWAN | `Lorawan`, `LorawanDevice`, `LorawanSession`, `LorawanHeader`, `LorawanJoinRequest`, `LorawanJoinAccept`, `LorawanGrant`, `LorawanOptions`, `LorawanRxData`, `LorawanMessageType`, `LorawanDirection` | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
| Mesh frames | `Mesh`, `MeshFrame`, `SeenPackets` | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
| Routing | `Router`, `Route`, `ForwardDecision`, `ForwardAction` | Reverse-path routing that learns the cheapest route from overheard traffic |
| MAVLink | `Mavlink`, `MavlinkFrame`, `MavlinkHeader`, `MavlinkParser`, `MavlinkSigner`, `MavlinkVerifier`, `MavlinkVersion`, `MavlinkDialect`, `MavlinkField`, `MavlinkSchema`, `MavlinkSchemaBuilder`, `MavlinkMessage`, `MavlinkFieldInfo`, `MavlinkFieldType`, `MavlinkMissionReceiver`, `MavlinkMissionSender`, `MavlinkReceiverStep`, `MavlinkSenderStep`, `MavlinkReceiverKind`, `MavlinkSenderKind`, `MavlinkCommand`, `MavlinkAckOutcome`, `MavlinkAckKind`, `MavlinkOffboard`, `MavlinkTypeMask` | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| Audit log | `Audit`, `AuditLog`, `AuditEntry`, `AuditVerifier` | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
| Secured session | `Session`, `SessionRole`, `AgreementKey`, `SealedMessage` | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
| Signed updates | `Update`, `Updater`, `Manifest`, `ImageVerifier`, `Progress`, `Delegation`, `SlotRecord`, `SlotState`, `BootDecision`, `BootAction` | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
| Power | `DutyCycle`, `PowerPlan`, `PowerMode` | Duty cycling and an energy-aware governor that stretches work as the battery drains |
| Telemetry | `Reporter`, `TelemetryEvent`, `TelemetryLevel`, `TelemetrySnapshot`, `LinkCost` | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| MQTT | `MqttClient`, `MqttClientOptions`, `MqttMessage`, `Qos` | An MQTT client with the topic and wildcard rules, as the core transport |
| CoAP | `CoapClient`, `CoapClientOptions`, `Reliability` | A CoAP client over UDP with confirmable delivery and observe |
| Loopback | `LoopbackBroker`, `LoopbackTransport` | An in-process transport with topic matching and a fault injector, for testing with no broker |
| Store and forward | `Store` | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
| Transport ladder | `Ladder`, `Delivery` | Cheapest reachable link first, buffering to a store when every link is down |
| Event bus | `EventBus` | An in-memory typed publish and subscribe event bus |
| Engine surface | `Transport`, `TransportMessage` | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| Simulators | `SimulatedSensor`, `Replay`, `RecordingActuator`, `SimulatedRobot`, `Pose`, `Twist` | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| Device profiles | `Profile`, `ControlPolicy`, `ControlKind`, `Controller`, `Reaction`, `Alert`, `AlertKind`, `PowerSchedule` | Named, ready-to-run device profiles from plain data or a JSON manifest |
| ROS 2 rules | `Ros2`, `Ros2Twist`, `Vector3`, `CdrReader`, `CdrWriter`, `EntityKind` | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
| Zenoh keys | `KeyExpression` | Zenoh key expressions: validity, canonical form, and wildcard matching |
<!-- end -->
