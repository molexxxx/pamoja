# pamoja

One memory-safe Rust core with bindings for TypeScript, Python, and C#, for the
devices that watch and control the physical world: sensors, gateways, robots,
and drones, built to run on cheap hardware with weak or no connectivity.

Every capability is a crate in Rust and a module in each binding, and every
guide here shows the same task in all four languages. The code in every guide
is spliced from a test that runs in CI, so an example that stops working
fails the build rather than the reader.

- [Install](install.md) the core in the language you work in.
- Read the guide for the capability you need, below.
- Look a symbol up in the [Rust](reference/rust.md), [TypeScript](reference/node.md),
  [Python](reference/python.md), or [C#](reference/dotnet.md) reference.
- See [why it exists](about/why.md), [how it is put together](about/architecture.md),
  and [which standards it is held to](about/standards.md).

## Guides

<!-- table: guides -->
### Identity

Signing a payload and checking it, the way a gateway verifies a reading.

- Device identity - ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic

### Codecs

Moving a document to the compact form a metered link should carry, and back.

- Codecs - CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links

### Helpers

The helper math a field node runs between reading a sensor and acting on it.

- Helpers - Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo

### Field I/O

The wires a gateway actually has: framed serial packets, an RS485 request and the reply it draws, a CAN frame, and the address a chip answers on.

- Serial framing - SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets
- Modbus RTU - Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices
- CAN and J1939 - CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose
- I2C, SPI, and GPIO - I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins

### Sensing and actuation

The parts wired to a board: a thermometer that checks its own bytes, a servo pulse, and a stepper walking its coils.

- Sensor drivers - Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115
- Actuator drivers - PCA9685 PWM and servo pulses, and stepper coil sequencing

### Radio and reach

Budgeting airtime, framing a mesh packet, routing it, and securing a LoRaWAN uplink: everything a node needs to reach a network it cannot see.

- LoRa airtime - Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to
- LoRaWAN - LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join
- Mesh frames - Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once
- Routing - Reverse-path routing that learns the cheapest route from overheard traffic

### MAVLink

Talking to an autopilot: framing a message, reading it back off a link that splits and garbles it, proving a signed frame came from who it claims, and moving a plan across one frame at a time.

- MAVLink - MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols

### Trust and operation

Proving what a node did, saying it in confidence, fixing it in the field, and deciding how often it can afford to do any of that.

- Audit log - A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification
- Secured session - X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack
- Signed updates - Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own
- Power - Duty cycling and an energy-aware governor that stretches work as the battery drains
- Telemetry - Observability that ships only what is worth the bytes as link cost rises, while counting everything

### Transports and testing

Reaching the network when no single link always works, and testing all of it with nothing plugged in.

- MQTT - An MQTT client with the topic and wildcard rules, as the core transport
- CoAP - A CoAP client over UDP with confirmable delivery and observe
- Loopback - An in-process transport with topic matching and a fault injector, for testing with no broker
- Store and forward - Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss
- Transport ladder - Cheapest reachable link first, buffering to a store when every link is down
- Event bus - An in-memory typed publish and subscribe event bus
- Engine surface - The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version
- Simulators - Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose

### Profiles and robotics

A node instantiated by name with its policy and schedule, and the naming and encoding rules a robot's topics follow, with no ROS 2 or Zenoh installed.

- Device profiles - Named, ready-to-run device profiles from plain data or a JSON manifest
- ROS 2 rules - ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed
- Zenoh keys - Zenoh key expressions: validity, canonical form, and wildcard matching
<!-- end -->
