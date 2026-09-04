# Pamoja

The whole pamoja framework in one package: every capability of one memory-safe Rust core, behind an idiomatic C# facade, for IoT, robotics, and drones. Each capability is also its own package, so an application that needs one thing can depend on `Pamoja.Mqtt` alone; this package depends on all of them.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/index.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja
```

## What it installs

Each package name opens its reference.

| Package | What it covers |
| --- | --- |
| [`Pamoja.Core`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html) | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| [`Pamoja.Security`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Security.html) | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| [`Pamoja.Codec`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html) | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| [`Pamoja.Kit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html) | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| [`Pamoja.Serial`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.html) | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
| [`Pamoja.Modbus`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html) | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
| [`Pamoja.Can`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html) | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
| [`Pamoja.Gpio`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html) | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| [`Pamoja.Sensors`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.html) | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
| [`Pamoja.Actuators`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html) | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| [`Pamoja.Lora`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lora.html) | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
| [`Pamoja.Lorawan`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.html) | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
| [`Pamoja.Mesh`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.html) | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
| [`Pamoja.Routing`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html) | Reverse-path routing that learns the cheapest route from overheard traffic |
| [`Pamoja.Mavlink`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html) | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| [`Pamoja.Audit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html) | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
| [`Pamoja.Session`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html) | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
| [`Pamoja.Update`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.html) | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
| [`Pamoja.Power`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.html) | Duty cycling and an energy-aware governor that stretches work as the battery drains |
| [`Pamoja.Telemetry`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html) | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| [`Pamoja.Mqtt`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html) | An MQTT client with the topic and wildcard rules, as the core transport |
| [`Pamoja.Coap`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Coap.html) | A CoAP client over UDP with confirmable delivery and observe |
| [`Pamoja.Loopback`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html) | An in-process transport with topic matching and a fault injector, for testing with no broker |
| [`Pamoja.Sync`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html) | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
| [`Pamoja.Ladder`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html) | Cheapest reachable link first, buffering to a store when every link is down |
| [`Pamoja.Bus`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html) | An in-memory typed publish and subscribe event bus |
| [`Pamoja.Sim`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html) | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| [`Pamoja.Profile`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html) | Named, ready-to-run device profiles from plain data or a JSON manifest |
| [`Pamoja.Ros2`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.html) | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
| [`Pamoja.Zenoh`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html) | Zenoh key expressions: validity, canonical form, and wildcard matching |

All of them run on `Pamoja.Native`, the compiled engine, which is one library whichever packages you install.

## Documentation

- [The guides](https://pamoja.molex.cloud/docs/), one page per capability with the same example in Rust, TypeScript, Python, and C#.
- [The C# reference](https://pamoja.molex.cloud/docs/reference/dotnet/index.html), generated from every package.

## License

MIT
