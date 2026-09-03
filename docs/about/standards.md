# Standards and conformance

Anything defined by a published standard is implemented from the
authoritative specification itself, and its tests are anchored to that
specification's own reference vectors. Bit layouts, field orders, reserved
bits, and algorithm constants are where the subtle bugs hide, and a plausible
guess is worse than none.

| Area | Anchored to |
| --- | --- |
| Crypto | FIPS-197 (AES-128), RFC 4493 (AES-CMAC), FIPS-180 (SHA-256), RFC 2104 and RFC 4231 (HMAC-SHA256), RFC 5869 (HKDF), RFC 7748 (X25519), RFC 8439 (ChaCha20-Poly1305) |
| Messaging | MQTT topic and wildcard rules, RFC 7252 and RFC 7641 (CoAP with observe) |
| Radio and mesh | LoRaWAN 1.0.x MAC framing and OTAA join, the LoRa Alliance RP002 regional parameters, LoRa time-on-air and duty cycle, CRC-16/CCITT frames |
| Field I/O | RFC 1055 (SLIP) and COBS, CRC-16/MODBUS, CAN 2.0 and CAN-FD with SAE J1939, NXP UM10204 (I2C) |
| Sensors and actuators | The BME280, DS18B20, INA219, ADS1115, and PCA9685 datasheets |
| Updates | RFC 9124 (the firmware manifest information model) |
| Drones | MAVLink v1 and v2 framing, CRC-16/MCRF4XX, per-message CRC_EXTRA, MAVLink 2 signing |
| Robotics | ROS 2 names, RIHS01 type hashes, CDR encoding, rmw_zenoh key expressions |

That rigor is also what makes dependency upgrades safe to take. When the
primitives underneath change, every vector still matches or the build fails.

## Across languages

A second set of vectors, in `conformance/vectors.json`, does the same job
across languages rather than against a specification. It is generated from
the Rust implementation and asserted by every binding's test suite, so a
facade that drifts fails instead of quietly returning something else. Each
vector pins exact wire bytes, and the suites are checked by perturbation: a
deliberately wrong vector fails in all four runners.

## Against the real thing

Where a specification has a live implementation to talk to, CI talks to it.
The MAVLink layer arms, commands, and flies missions against ArduPilot and PX4
SITL. The ROS 2 bridge exchanges topics, services, and actions with ROS 2
Jazzy under rmw_zenoh. Every `no_std` crate is cross-compiled for a Cortex-M4F
microcontroller, since a host `no_std` build still links the host `std`.
