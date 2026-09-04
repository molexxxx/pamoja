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
| Identity | [Device identity](https://pamoja.molex.cloud/docs/guides/security.html) | [`pamoja-security`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_security/index.html) |
| Codecs | [Codecs](https://pamoja.molex.cloud/docs/guides/codec.html) | [`pamoja-codec`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html) |
| Helpers | [Helpers](https://pamoja.molex.cloud/docs/guides/kit.html) | [`pamoja-kit`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html) |
| Field I/O | [Serial framing](https://pamoja.molex.cloud/docs/guides/serial.html), [Modbus RTU](https://pamoja.molex.cloud/docs/guides/modbus.html), [CAN and J1939](https://pamoja.molex.cloud/docs/guides/can.html), [I2C, SPI, and GPIO](https://pamoja.molex.cloud/docs/guides/gpio.html) | [`pamoja-serial`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html), [`pamoja-modbus`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html), [`pamoja-can`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html), [`pamoja-gpio`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html) |
| Sensing and actuation | [Sensor drivers](https://pamoja.molex.cloud/docs/guides/sensors.html), [Actuator drivers](https://pamoja.molex.cloud/docs/guides/actuators.html) | [`pamoja-sensors`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html), [`pamoja-actuators`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html) |
| Radio and reach | [LoRa airtime](https://pamoja.molex.cloud/docs/guides/lora.html), [LoRaWAN](https://pamoja.molex.cloud/docs/guides/lorawan.html), [Mesh frames](https://pamoja.molex.cloud/docs/guides/mesh.html), [Routing](https://pamoja.molex.cloud/docs/guides/routing.html) | [`pamoja-lora`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html), [`pamoja-lorawan`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html), [`pamoja-mesh`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mesh/index.html), [`pamoja-routing`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_routing/index.html) |
| MAVLink | [MAVLink](https://pamoja.molex.cloud/docs/guides/mavlink.html) | [`pamoja-mavlink`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html) |
| Trust and operation | [Audit log](https://pamoja.molex.cloud/docs/guides/audit.html), [Secured session](https://pamoja.molex.cloud/docs/guides/session.html), [Signed updates](https://pamoja.molex.cloud/docs/guides/update.html), [Power](https://pamoja.molex.cloud/docs/guides/power.html), [Telemetry](https://pamoja.molex.cloud/docs/guides/telemetry.html) | [`pamoja-audit`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html), [`pamoja-session`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html), [`pamoja-update`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_update/index.html), [`pamoja-power`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_power/index.html), [`pamoja-telemetry`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html) |
| Transports and testing | MQTT, CoAP, Loopback, Store and forward, Transport ladder, Event bus, Engine surface, Simulators | [`pamoja-mqtt`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mqtt/index.html), [`pamoja-coap`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_coap/index.html), [`pamoja-loopback`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html), [`pamoja-sync`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html), [`pamoja-ladder`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html), [`pamoja-bus`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html), [`pamoja-sim`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html) |
| Profiles and robotics | Device profiles, [ROS 2 rules](https://pamoja.molex.cloud/docs/guides/ros2.html), [Zenoh keys](https://pamoja.molex.cloud/docs/guides/zenoh.html) | [`pamoja-profile`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [`pamoja-ros2`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html), [`pamoja-zenoh`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html) |
| Engine | the traits every capability implements, the C ABI, and the dashboard | [`pamoja-core`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html), [`pamoja-ffi`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ffi/index.html), [`pamoja-dashboard`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_dashboard/index.html) |
| Everything | `cargo add pamoja`: every capability above, behind a feature each | [`pamoja`](https://pamoja.molex.cloud/docs/reference/rust/pamoja/index.html) |
<!-- end -->

## Reference

Every language has a full API reference, generated from its own source and hosted
here. The page on this site names each package and links it to its page there.

<!-- table: references -->
| Language | Install | What it covers | Full API reference |
| --- | --- | --- | --- |
| Rust | `cargo add pamoja` | [every package](reference/rust.md) | [Rust reference](reference/rust/pamoja/index.html), generated by rustdoc |
| TypeScript | `npm install pamoja` | [every package](reference/node.md) | [TypeScript reference](reference/node/index.html), generated by typedoc |
| Python | `pip install pamoja` | [every package](reference/python.md) | [Python reference](reference/python/pamoja.html), generated by pdoc |
| C# | `dotnet add package Pamoja` | [every package](reference/dotnet.md) | [C# reference](reference/dotnet/index.html), generated by DocFX |
<!-- end -->
