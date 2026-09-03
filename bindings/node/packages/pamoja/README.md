# pamoja

The whole pamoja framework in one package: every capability of one memory-safe Rust core, behind an idiomatic TypeScript facade, for IoT, robotics, and drones. Each capability is also its own package, so an application that needs one thing can depend on `@pamoja/mqtt` alone; this package depends on all of them and re-exports them.

## Install

```sh
npm install pamoja
```

## What it bundles

| Package | What it covers |
| --- | --- |
| `@pamoja/security` | ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic |
| `@pamoja/codec` | CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links |
| `@pamoja/kit` | Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo |
| `@pamoja/serial` | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
| `@pamoja/modbus` | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
| `@pamoja/can` | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
| `@pamoja/gpio` | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |
| `@pamoja/sensors` | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
| `@pamoja/actuators` | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| `@pamoja/lora` | Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to |
| `@pamoja/lorawan` | LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join |
| `@pamoja/mesh` | Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once |
| `@pamoja/routing` | Reverse-path routing that learns the cheapest route from overheard traffic |
| `@pamoja/mavlink` | MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols |
| `@pamoja/audit` | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
| `@pamoja/session` | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
| `@pamoja/update` | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
| `@pamoja/power` | Duty cycling and an energy-aware governor that stretches work as the battery drains |
| `@pamoja/telemetry` | Observability that ships only what is worth the bytes as link cost rises, while counting everything |
| `@pamoja/mqtt` | An MQTT client with the topic and wildcard rules, as the core transport |
| `@pamoja/coap` | A CoAP client over UDP with confirmable delivery and observe |
| `@pamoja/loopback` | An in-process transport with topic matching and a fault injector, for testing with no broker |
| `@pamoja/sync` | Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss |
| `@pamoja/ladder` | Cheapest reachable link first, buffering to a store when every link is down |
| `@pamoja/bus` | An in-memory typed publish and subscribe event bus |
| `@pamoja/core` | The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version |
| `@pamoja/sim` | Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose |
| `@pamoja/profile` | Named, ready-to-run device profiles from plain data or a JSON manifest |
| `@pamoja/ros2` | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
| `@pamoja/zenoh` | Zenoh key expressions: validity, canonical form, and wildcard matching |

All of them run on `@pamoja/native`, the compiled engine, which is one binary whichever packages you install.

## Documentation

- [The guides](https://pamoja.molex.cloud/docs/), one page per capability with the same example in Rust, TypeScript, Python, and C#.
- [The TypeScript reference](https://pamoja.molex.cloud/docs/reference/node/index.html), generated from every package.

## License

MIT
