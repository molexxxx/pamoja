# pamoja

The whole pamoja framework in one package: every capability of one memory-safe Rust core, behind an idiomatic TypeScript facade, for IoT, robotics, and drones. Each capability is also its own package, so an application that needs one thing can depend on `@pamoja/mqtt` alone; this package depends on all of them and re-exports them.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/index.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install pamoja
```

## What it bundles

Each package name opens its reference.

| Package | What it covers |
| --- | --- |
| [`@pamoja/core`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_core.html) | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| [`@pamoja/security`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_security.html) | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| [`@pamoja/codec`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html) | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| [`@pamoja/kit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html) | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| [`@pamoja/serial`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html) | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
| [`@pamoja/modbus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html) | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
| [`@pamoja/can`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html) | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
| [`@pamoja/gpio`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html) | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| [`@pamoja/sensors`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html) | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
| [`@pamoja/actuators`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html) | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| [`@pamoja/lora`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lora.html) | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
| [`@pamoja/lorawan`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html) | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
| [`@pamoja/mesh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html) | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
| [`@pamoja/routing`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html) | Reverse-path routing that learns the cheapest route from overheard traffic |
| [`@pamoja/mavlink`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html) | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| [`@pamoja/audit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html) | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
| [`@pamoja/session`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html) | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
| [`@pamoja/update`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html) | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
| [`@pamoja/power`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html) | Duty cycling and an energy-aware governor that stretches work as the battery drains |
| [`@pamoja/telemetry`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html) | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| [`@pamoja/mqtt`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html) | An MQTT client with the topic and wildcard rules, as the core transport |
| [`@pamoja/coap`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html) | A CoAP client over UDP with confirmable delivery and observe |
| [`@pamoja/loopback`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html) | An in-process transport with topic matching and a fault injector, for testing with no broker |
| [`@pamoja/sync`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html) | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
| [`@pamoja/ladder`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html) | Cheapest reachable link first, buffering to a store when every link is down |
| [`@pamoja/bus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html) | An in-memory typed publish and subscribe event bus |
| [`@pamoja/sim`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html) | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| [`@pamoja/profile`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html) | Named, ready-to-run device profiles from plain data or a JSON manifest |
| [`@pamoja/ros2`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html) | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
| [`@pamoja/zenoh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html) | Zenoh key expressions: validity, canonical form, and wildcard matching |

All of them run on `@pamoja/native`, the compiled engine, which is one binary whichever packages you install.

## Documentation

- [The guides](https://pamoja.molex.cloud/docs/), one page per capability with the same example in Rust, TypeScript, Python, and C#.
- [The TypeScript reference](https://pamoja.molex.cloud/docs/reference/node/index.html), generated from every package.

## License

MIT
