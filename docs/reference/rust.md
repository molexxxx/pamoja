# Rust reference

Every crate's rustdoc, built from this commit, is under
[`reference/rust/`](rust/pamoja_core/index.html); the same documentation for
each published version is on [docs.rs](https://docs.rs/pamoja-core). Each crate's
README, on crates.io and in the repository, is its overview and getting-started
page, generated from the same rustdoc.

The `pamoja` crate is every capability crate behind a feature each, all on by
default; `pamoja::mqtt` is `pamoja-mqtt`, with the same types and documentation.
Six of its features name a domain instead of a single capability, one per chapter
below that holds more than one:

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

Naming features is what makes a Rust build small, and the
[install page](../install.md) measures how small per feature set. The engine
crates define the traits (`Transport`, `Device`, `Sensor`, `Actuator`, `Store`,
the event bus) that every capability crate implements, the C ABI the .NET
binding rides on, and the fleet dashboard.

<!-- table: crates -->
| Chapter | Crate | What it does |
| --- | --- | --- |
| **Everything** | [`pamoja`](https://docs.rs/pamoja) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja/index.html)) | The whole pamoja device SDK in one crate: every capability behind a feature, all on by default, for IoT, robotics, and drones. |
| **Engine** | [`pamoja-core`](https://docs.rs/pamoja-core) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html)) | Core engine for the pamoja device SDK: device model, transport, event bus, and error types. |
|  | [`pamoja-ffi`](https://docs.rs/pamoja-ffi) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ffi/index.html)) | Curated C ABI surface over the pamoja device SDK, for C, C++, and .NET. |
|  | [`pamoja-dashboard`](https://docs.rs/pamoja-dashboard) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_dashboard/index.html)) | Local-first device dashboard for pamoja: a node serves a hand-built, localized web UI over its own hotspot from a language-neutral state snapshot, fully offline. |
| **Identity** | [`pamoja-security`](https://docs.rs/pamoja-security) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_security/index.html)) | Device identity and signed telemetry for pamoja: ed25519 keys that sign payloads and verify them, for data integrity and audit trails. |
| **Codecs** | [`pamoja-codec`](https://docs.rs/pamoja-codec) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html)) | Serialization and framing for pamoja: pluggable wire formats behind a common Codec trait. |
| **Helpers** | [`pamoja-kit`](https://docs.rs/pamoja-kit) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html)) | Goal-named helper math for the pamoja device SDK: smoothing/filtering, calibration and units, PID and on/off control, prediction and anomaly detection, rolling stats, wheel kinematics (differential, Ackermann, skid-steer, mecanum), odometry, waypoint guidance, motion safety, and geo/IMU/weather helpers; no_std-friendly. |
| **Field I/O** | [`pamoja-serial`](https://docs.rs/pamoja-serial) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html)) | Serial-line packet framing for pamoja: SLIP (RFC 1055) and COBS byte-stuffing with streaming frame decoders, so a raw UART byte stream carries discrete, self-delimiting packets to and from motor controllers, GPS, and LiDAR, no_std and allocation-free. The framing half ahead of the serial driver. |
|  | [`pamoja-modbus`](https://docs.rs/pamoja-modbus) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html)) | Modbus RTU framing for pamoja: CRC-16/Modbus, the RTU ADU envelope, the standard request PDUs, and response decoding, so a long-cable RS485 field sensor speaks Modbus, no_std and allocation-free. The framing half ahead of the serial driver. |
|  | [`pamoja-can`](https://docs.rs/pamoja-can) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html)) | CAN bus framing for pamoja: classic CAN 2.0 and CAN-FD frames with 11-bit and 29-bit identifiers and the discrete CAN-FD length encoding, plus J1939 identifier decoding for trucks, tractors, and gensets, no_std and allocation-free. The framing half ahead of the CAN controller. |
|  | [`pamoja-gpio`](https://docs.rs/pamoja-gpio) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html)) | On-board bus addressing and pin logic for pamoja: I2C 7-bit and 10-bit address-frame encoding (NXP UM10204) with reserved-range checks, the four SPI clock modes from CPOL/CPHA, and a GPIO pin model with active-high/active-low logical levels, no_std and allocation-free. The addressing-and-mode half ahead of the GPIO/I2C/SPI driver. |
| **Sensing and actuation** | [`pamoja-sensors`](https://docs.rs/pamoja-sensors) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html)) | Concrete sensor drivers for pamoja: the decode-and-configure half of common parts - BME280 temperature/pressure/humidity (Bosch compensation), DS18B20 1-Wire thermometer (datasheet temperature table + Maxim CRC-8), INA219 current/voltage/power monitor (TI calibration math), and ADS1115 ADC (config register and full-scale conversion) - each turning raw register bytes into physical readings exactly as the manufacturer datasheet specifies, no_std and allocation-free. The decode half ahead of the bus driver. |
|  | [`pamoja-actuators`](https://docs.rs/pamoja-actuators) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html)) | Concrete actuator drivers for pamoja: the command-encode half of common parts - the PCA9685 16-channel PWM/servo/LED controller (mode and prescale registers, the frequency-to-prescale formula, and 12-bit on/off channel words) and stepper-motor coil sequencing (wave, full-step, and half-step drive plus a step/direction position model) - turning a desired output into the bytes and steps a driver applies against the manufacturer datasheet, no_std and allocation-free. The command-encode half ahead of the bus driver. |
| **Radio and reach** | [`pamoja-lora`](https://docs.rs/pamoja-lora) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html)) | LoRa link math for pamoja: exact time-on-air and duty-cycle off-time, so a long-range node stays within regulations and budgets its power, no_std and allocation-free. |
|  | [`pamoja-lorawan`](https://docs.rs/pamoja-lorawan) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html)) | LoRaWAN 1.0.x MAC framing for pamoja: build and parse data-frame PHYPayloads with the message integrity code and payload encryption the spec mandates, plus the over-the-air-activation join exchange, so a long-range node speaks LoRaWAN, no_std and allocation-free. The secured-packet half ahead of the radio driver. |
|  | [`pamoja-mesh`](https://docs.rs/pamoja-mesh) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mesh/index.html)) | Mesh packet framing for pamoja: an addressed, hop-limited, CRC-checked frame for cheap local and mesh radio (ESP-NOW and nRF24 style), with the relay and duplicate-suppression primitives that turn it into a flooding mesh, no_std and allocation-free. The framing half ahead of the radio driver. |
|  | [`pamoja-routing`](https://docs.rs/pamoja-routing) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_routing/index.html)) | Cost-aware mesh routing for pamoja: a bounded routing table that learns reverse-path routes from the traffic it hears and decides whether to deliver, relay toward a destination, or fall back to flooding, so a mesh forwards instead of blindly flooding, no_std and allocation-free. |
| **MAVLink** | [`pamoja-mavlink`](https://docs.rs/pamoja-mavlink) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html)) | MAVLink for pamoja: build, parse, and sign v1/v2 frames (CRC-16/MCRF4XX, per-message CRC_EXTRA, MAVLink 2 SHA-256 signing), a typed common dialect with MAVLink 2 extension fields, the mission, command, and offboard protocols as sans-IO state machines, and a vehicle modelled as a pamoja Device driven over real serial, UDP, and TCP links. Hand-written from the mavlink.io spec, no_std and allocation-free at the core, and exercised against ArduPilot and PX4 SITL. |
| **Trust and operation** | [`pamoja-audit`](https://docs.rs/pamoja-audit) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html)) | Tamper-evident audit logs for pamoja: signed, hash-chained entries so altering, reordering, or dropping a record is detectable. |
|  | [`pamoja-session`](https://docs.rs/pamoja-session) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html)) | Encrypted, authenticated sessions for pamoja: X25519 key agreement (RFC 7748) and HKDF-SHA256 (RFC 5869) derive a session key, then ChaCha20-Poly1305 (RFC 8439) protects each message with counter nonces and an anti-replay window, so two devices that know each other's keys share a confidential, tamper-evident link, no_std and allocation-free. The secured-channel half ahead of the rustls/DTLS driver. |
|  | [`pamoja-update`](https://docs.rs/pamoja-update) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_update/index.html)) | Signed firmware updates for pamoja: an RFC 9124 manifest, image verification, and A/B slots with verified rollback, so a device can be fixed in the field. |
|  | [`pamoja-power`](https://docs.rs/pamoja-power) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_power/index.html)) | Power-aware scheduling for the pamoja device SDK: duty cycling and an energy-aware governor, no_std-friendly. |
|  | [`pamoja-telemetry`](https://docs.rs/pamoja-telemetry) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html)) | Device-side observability for pamoja: structured leveled events and counters that degrade gracefully on metered links, no_std and allocation-free. |
| **Transports and testing** | [`pamoja-mqtt`](https://docs.rs/pamoja-mqtt) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mqtt/index.html)) | MQTT transport for the pamoja device SDK, built on rumqttc. |
|  | [`pamoja-coap`](https://docs.rs/pamoja-coap) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_coap/index.html)) | CoAP transport for the pamoja device SDK, built on coap-lite over UDP. |
|  | [`pamoja-loopback`](https://docs.rs/pamoja-loopback) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html)) | In-process loopback transport for pamoja: a broker-free Transport for tests, examples, and simulators. |
|  | [`pamoja-sync`](https://docs.rs/pamoja-sync) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html)) | Offline-first store-and-forward buffering for pamoja: durable queues behind the core Store trait. |
|  | [`pamoja-ladder`](https://docs.rs/pamoja-ladder) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html)) | Cost-aware transport ladder for the pamoja device SDK: try the cheapest link first, buffer offline. |
|  | [`pamoja-bus`](https://docs.rs/pamoja-bus) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html)) | In-memory typed publish/subscribe event bus implementing the core EventBus trait. |
|  | [`pamoja-sim`](https://docs.rs/pamoja-sim) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html)) | Hardware-free device simulators for pamoja: fake sensors with configurable noise and drift, a recording actuator, a lossy-link transport, and a drivable differential-drive robot, behind the core Sensor and Actuator traits. |
| **Profiles and robotics** | [`pamoja-profile`](https://docs.rs/pamoja-profile) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html)) | Named, ready-to-run device profiles for pamoja: assemble a sensor, actuator, transport, codec, and power schedule into a working node. |
|  | [`pamoja-ros2`](https://docs.rs/pamoja-ros2) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html)) | ROS 2 bridge logic for the pamoja SDK: topic/service name validation and DDS mapping, RIHS01 type-hash and DDS type-name handling, rmw_zenoh key-expression assembly, and CDR message encoding, no_std and allocation-light. The pure-logic half ahead of the live r2r/Zenoh bridge. |
|  | [`pamoja-zenoh`](https://docs.rs/pamoja-zenoh) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html)) | Zenoh key-expression logic for the pamoja SDK: validity, canonical form, and matching of the chunk-based key-expression language (the `*`, `**`, and `$*` wildcards), no_std and allocation-light. The pure-logic half ahead of the Zenoh transport. |
<!-- end -->
