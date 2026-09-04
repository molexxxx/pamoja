# pamoja

One memory-safe Rust core with bindings for TypeScript, Python, and C#, for the
devices that watch and control the physical world: sensors, gateways, robots,
and drones, built to run on cheap hardware with weak or no connectivity.

Every capability is a crate in Rust and a package in each binding, and every
guide shows the same task in all four languages. The code in a guide is spliced
from a test that runs in CI, so an example that stops working fails the build
rather than the reader.

## Start here

```sh
cargo add pamoja                 # Rust
npm install pamoja               # TypeScript and Node
pip install pamoja               # Python
dotnet add package Pamoja        # C# and .NET
```

That is the whole framework. To take less than all of it, and to see what a
narrow build actually costs, read [Install](install.md).

## What it covers

<!-- table: chapters -->
| Chapter | Guides | Crates |
| --- | --- | --- |
| Identity | Device identity | [`pamoja-security`](https://docs.rs/pamoja-security) |
| Codecs | Codecs | [`pamoja-codec`](https://docs.rs/pamoja-codec) |
| Helpers | Helpers | [`pamoja-kit`](https://docs.rs/pamoja-kit) |
| Field I/O | Serial framing, Modbus RTU, CAN and J1939, I2C, SPI, and GPIO | [`pamoja-serial`](https://docs.rs/pamoja-serial), [`pamoja-modbus`](https://docs.rs/pamoja-modbus), [`pamoja-can`](https://docs.rs/pamoja-can), [`pamoja-gpio`](https://docs.rs/pamoja-gpio) |
| Sensing and actuation | Sensor drivers, Actuator drivers | [`pamoja-sensors`](https://docs.rs/pamoja-sensors), [`pamoja-actuators`](https://docs.rs/pamoja-actuators) |
| Radio and reach | LoRa airtime, LoRaWAN, Mesh frames, Routing | [`pamoja-lora`](https://docs.rs/pamoja-lora), [`pamoja-lorawan`](https://docs.rs/pamoja-lorawan), [`pamoja-mesh`](https://docs.rs/pamoja-mesh), [`pamoja-routing`](https://docs.rs/pamoja-routing) |
| MAVLink | MAVLink | [`pamoja-mavlink`](https://docs.rs/pamoja-mavlink) |
| Trust and operation | Audit log, Secured session, Signed updates, Power, Telemetry | [`pamoja-audit`](https://docs.rs/pamoja-audit), [`pamoja-session`](https://docs.rs/pamoja-session), [`pamoja-update`](https://docs.rs/pamoja-update), [`pamoja-power`](https://docs.rs/pamoja-power), [`pamoja-telemetry`](https://docs.rs/pamoja-telemetry) |
| Transports and testing | MQTT, CoAP, Loopback, Store and forward, Transport ladder, Event bus, Engine surface, Simulators | [`pamoja-mqtt`](https://docs.rs/pamoja-mqtt), [`pamoja-coap`](https://docs.rs/pamoja-coap), [`pamoja-loopback`](https://docs.rs/pamoja-loopback), [`pamoja-sync`](https://docs.rs/pamoja-sync), [`pamoja-ladder`](https://docs.rs/pamoja-ladder), [`pamoja-bus`](https://docs.rs/pamoja-bus), [`pamoja-sim`](https://docs.rs/pamoja-sim) |
| Profiles and robotics | Device profiles, ROS 2 rules, Zenoh keys | [`pamoja-profile`](https://docs.rs/pamoja-profile), [`pamoja-ros2`](https://docs.rs/pamoja-ros2), [`pamoja-zenoh`](https://docs.rs/pamoja-zenoh) |
| Engine | the traits every capability implements, the C ABI, and the dashboard | [`pamoja-core`](https://docs.rs/pamoja-core), [`pamoja-ffi`](https://docs.rs/pamoja-ffi), [`pamoja-dashboard`](https://docs.rs/pamoja-dashboard) |
| Everything | `cargo add pamoja`: every capability above, behind a feature each | [`pamoja`](https://docs.rs/pamoja) |
<!-- end -->

The guides are grouped by the chapters above, and the reference for each
language is generated from its own source: [Rust](reference/rust.md),
[TypeScript](reference/node.md), [Python](reference/python.md), and
[C#](reference/dotnet.md).
