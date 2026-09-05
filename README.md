<div align="center">

<img src="assets/pamoja-logo.svg" alt="pamoja" width="560">

**One memory-safe Rust core. Every language. For the devices that change lives.**

<a href="https://crates.io/users/tonywied17"><img height="22" alt="crates.io" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-crates-pamoja.svg?v=21ebfb2d"></a>
&nbsp;<a href="https://www.npmjs.com/org/pamoja"><img height="22" alt="npm" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-npm-pamoja.svg?v=251ba560"></a>
&nbsp;<a href="https://pypi.org/user/tonywied17/"><img height="22" alt="PyPI" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-pypi-pamoja.svg?v=7b1567cc"></a>
&nbsp;<a href="https://www.nuget.org/profiles/tonywied17"><img height="22" alt="NuGet" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-nuget-pamoja.svg?v=cdd1b61a"></a>
&nbsp;<a href="https://github.com/molexxxx/pamoja/actions/workflows/ci.yml"><img height="22" alt="CI" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-ci-pamoja.svg?v=2d04e663"></a>
&nbsp;<a href="LICENSE-MIT"><img height="22" alt="license MIT" src="https://raw.githubusercontent.com/molexxxx/molexxxx/main/.github/badges/pamoja-license-pamoja.svg?v=79a1d17d"></a>

<a href="https://pamoja.molex.cloud/docs/"><img height="34" alt="documentation" src="https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg"></a>
&nbsp;<a href="https://pamoja.molex.cloud"><img height="34" alt="website" src="https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-website.svg"></a>
&nbsp;<a href="https://pamoja.molex.cloud/dashboard"><img height="34" alt="dashboard demo" src="https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-dashboard.svg"></a>

</div>

pamoja is an SDK for IoT, robotics, and drones: one Rust engine with idiomatic
bindings for TypeScript, Python, and C#. Every capability is a crate in Rust and
a package in each binding, and the same concepts work the same way in all four
languages. Most of it is `no_std`, so the same code runs on a gateway and on a
microcontroller.

It is built for the hard environment first: cheap and salvageable hardware, weak
or no connectivity, solar power. Offline-first store-and-forward, compact codecs,
long-range radio, and power-aware scheduling are first-class, and all of it can be
built and tested with nothing plugged in.

## Install

```sh
cargo add pamoja                 # Rust
npm install pamoja               # TypeScript and Node
pip install pamoja               # Python
dotnet add package Pamoja        # C# and .NET
```

That is the whole framework. Every registry offers three grain sizes, so a
project can take less:

| What you want | Rust | npm | PyPI | NuGet |
| --- | --- | --- | --- | --- |
| Everything | `pamoja` | `pamoja` | `pamoja` | `Pamoja` |
| A domain, six of them | `pamoja --features radio` | `@pamoja/radio` | `pamoja-radio` | `Pamoja.Radio` |
| One capability, thirty | `pamoja-lora` | `@pamoja/lora` | `pamoja-lora` | `Pamoja.Lora` |

In Rust that decides what gets compiled, and a Modbus-only build carries three
crates and no third-party code at all. In the bindings it decides what you
import, because one compiled engine sits under every package. The
[install page](https://pamoja.molex.cloud/docs/install.html) measures both.

## First example

A reading off a wire, smoothed, signed, and packed for a metered link, with
nothing plugged in. Each of these is spliced from a test that runs in CI.

<details open>
<summary><b>Rust</b></summary>

<!-- snippet: examples/tests/guides/quickstart.rs#example -->
From [`examples/tests/guides/quickstart.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/quickstart.rs):

```rust
use pamoja_codec::{decode_deltas, encode_deltas};
use pamoja_kit::Smoother;
use pamoja_security::DeviceIdentity;
use pamoja_sensors::ds18b20::{crc8, Scratchpad};

// The nine bytes a DS18B20 sends, CRC last; a bad CRC is a rejected read.
let mut scratchpad = [0x91, 0x01, 0x4b, 0x46, 0x7f, 0xff, 0x0c, 0x10, 0x00];
scratchpad[8] = crc8(&scratchpad[..8]);
let celsius = Scratchpad::parse(&scratchpad)
    .expect("the CRC matches")
    .temperature_celsius();
assert_eq!(celsius, 25.0625);

// Smooth the noise out of successive readings.
let mut smoother = Smoother::new(0.5);
smoother.update(celsius);
let smoothed = smoother.update(celsius + 1.0);
assert!(smoothed > celsius && smoothed < celsius + 1.0);

// Sign the reading so a gateway can prove which device sent it.
let device = DeviceIdentity::from_seed(&[7u8; 32]);
let payload = format!("{smoothed:.2}");
let signature = device.sign(payload.as_bytes());
let verified = device.public().verify(payload.as_bytes(), &signature);
assert!(verified.is_ok());

// Pack a batch of readings for a link where every byte costs money.
let samples = [2506i64, 2507, 2509, 2508, 2510];
let packed = encode_deltas(&samples);
assert!(packed.len() < samples.len() * 8);
assert_eq!(decode_deltas(&packed).expect("a valid batch"), samples);
```
<!-- end -->

</details>

<details>
<summary><b>TypeScript</b></summary>

<!-- snippet: bindings/node/guides/quickstart.ts#example -->
From [`bindings/node/guides/quickstart.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/quickstart.ts):

```typescript
import assert from 'node:assert/strict'

import { packSamples, unpackSamples } from '@pamoja/codec'
import { Smoother } from '@pamoja/kit'
import { DeviceIdentity, verify } from '@pamoja/security'
import { ds18b20 } from '@pamoja/sensors'

// The nine bytes a DS18B20 sends, CRC last; a bad CRC is a rejected read.
const scratchpad = Buffer.from([0x91, 0x01, 0x4b, 0x46, 0x7f, 0xff, 0x0c, 0x10, 0x00])
scratchpad[8] = ds18b20.crc8(scratchpad.subarray(0, 8))
const celsius = ds18b20.parseScratchpad(scratchpad).microCelsius / 1e6
assert.equal(celsius, 25.0625)

// Smooth the noise out of successive readings.
const smoother = new Smoother(0.5)
smoother.update(celsius)
const smoothed = smoother.update(celsius + 1)
assert.ok(smoothed > celsius && smoothed < celsius + 1)

// Sign the reading so a gateway can prove which device sent it.
const device = DeviceIdentity.fromSeed(Buffer.alloc(32, 7))
const payload = smoothed.toFixed(2)
const signature = device.sign(payload)
assert.ok(verify(device.publicKey(), payload, signature))

// Pack a batch of readings for a link where every byte costs money.
const samples = [2506, 2507, 2509, 2508, 2510]
const packed = packSamples(samples)
assert.ok(packed.length < samples.length * 8)
assert.deepEqual(unpackSamples(packed), samples)
```
<!-- end -->

</details>

<details>
<summary><b>Python</b></summary>

<!-- snippet: bindings/python/guides/quickstart.py#example -->
From [`bindings/python/guides/quickstart.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/quickstart.py):

```python
from pamoja import sensors
from pamoja.codec import pack_samples, unpack_samples
from pamoja.kit import Smoother
from pamoja.security import DeviceIdentity, verify

# The nine bytes a DS18B20 sends, CRC last; a bad CRC is a rejected read.
scratchpad = bytearray([0x91, 0x01, 0x4B, 0x46, 0x7F, 0xFF, 0x0C, 0x10, 0x00])
scratchpad[8] = sensors.ds18b20.crc8(bytes(scratchpad[:8]))
celsius = sensors.ds18b20.parse_scratchpad(bytes(scratchpad)).micro_celsius / 1e6
assert celsius == 25.0625

# Smooth the noise out of successive readings.
smoother = Smoother(0.5)
smoother.update(celsius)
smoothed = smoother.update(celsius + 1.0)
assert celsius < smoothed < celsius + 1.0

# Sign the reading so a gateway can prove which device sent it.
device = DeviceIdentity.from_seed(bytes([7]) * 32)
payload = f"{smoothed:.2f}"
signature = device.sign(payload)
assert verify(device.public_key, payload, signature)

# Pack a batch of readings for a link where every byte costs money.
samples = [2506, 2507, 2509, 2508, 2510]
packed = pack_samples(samples)
assert len(packed) < len(samples) * 8
assert unpack_samples(packed) == samples
```
<!-- end -->

</details>

<details>
<summary><b>C#</b></summary>

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/Quickstart.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/Quickstart.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/Quickstart.cs):

```csharp
// The nine bytes a DS18B20 sends, CRC last; a bad CRC is a rejected read.
byte[] scratchpad = [0x91, 0x01, 0x4B, 0x46, 0x7F, 0xFF, 0x0C, 0x10, 0x00];
scratchpad[8] = Ds18b20.Crc8(scratchpad.AsSpan(0, 8));
float celsius = Ds18b20.ParseScratchpad(scratchpad).MicroCelsius / 1e6f;
Expect(celsius == 25.0625f, "the register decodes to 25.0625 C");

// Smooth the noise out of successive readings.
using var smoother = new Smoother(0.5f);
smoother.Update(celsius);
float smoothed = smoother.Update(celsius + 1.0f);
Expect(smoothed > celsius && smoothed < celsius + 1.0f, "smoothing lags the step");

// Sign the reading so a gateway can prove which device sent it.
byte[] seed = new byte[DeviceIdentity.KeyLength];
Array.Fill(seed, (byte)7);
using var device = new DeviceIdentity(seed);
string payload = smoothed.ToString("F2", CultureInfo.InvariantCulture);
byte[] signature = device.Sign(payload);
Expect(DeviceIdentity.Verify(device.PublicKey, payload, signature), "the signature verifies");

// Pack a batch of readings for a link where every byte costs money.
long[] samples = [2506, 2507, 2509, 2508, 2510];
byte[] packed = Codec.PackSamples(samples);
Expect(packed.Length < samples.Length * 8, "packing beats eight bytes a sample");
Expect(Codec.UnpackSamples(packed).SequenceEqual(samples), "and the batch round-trips");
```
<!-- end -->

</details>

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
| Transports and testing | [MQTT](https://pamoja.molex.cloud/docs/guides/mqtt.html), [CoAP](https://pamoja.molex.cloud/docs/guides/coap.html), [Loopback](https://pamoja.molex.cloud/docs/guides/loopback.html), [Store and forward](https://pamoja.molex.cloud/docs/guides/sync.html), [Transport ladder](https://pamoja.molex.cloud/docs/guides/ladder.html), [Event bus](https://pamoja.molex.cloud/docs/guides/bus.html), [Engine surface](https://pamoja.molex.cloud/docs/guides/transport.html), [Simulators](https://pamoja.molex.cloud/docs/guides/sim.html) | [`pamoja-mqtt`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mqtt/index.html), [`pamoja-coap`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_coap/index.html), [`pamoja-loopback`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html), [`pamoja-sync`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html), [`pamoja-ladder`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html), [`pamoja-bus`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html), [`pamoja-sim`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html) |
| Profiles and robotics | [Device profiles](https://pamoja.molex.cloud/docs/guides/profile.html), [ROS 2 rules](https://pamoja.molex.cloud/docs/guides/ros2.html), [Zenoh keys](https://pamoja.molex.cloud/docs/guides/zenoh.html) | [`pamoja-profile`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [`pamoja-ros2`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html), [`pamoja-zenoh`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html) |
| Engine | the traits every capability implements, the C ABI, and the dashboard | [`pamoja-core`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html), [`pamoja-ffi`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ffi/index.html), [`pamoja-dashboard`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_dashboard/index.html) |
| Everything | `cargo add pamoja`: every capability above, behind a feature each | [`pamoja`](https://pamoja.molex.cloud/docs/reference/rust/pamoja/index.html) |
<!-- end -->

## Documentation

Every guide shows the same task in all four languages, and each capability's
page on any registry links to the same capability on the other three.

<!-- table: references absolute -->
| Language | Install | What it covers | Full API reference |
| --- | --- | --- | --- |
| Rust | `cargo add pamoja` | [every package](https://pamoja.molex.cloud/docs/reference/rust.html) | [Rust reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja/index.html), generated by rustdoc |
| TypeScript | `npm install pamoja` | [every package](https://pamoja.molex.cloud/docs/reference/node.html) | [TypeScript reference](https://pamoja.molex.cloud/docs/reference/node/index.html), generated by typedoc |
| Python | `pip install pamoja` | [every package](https://pamoja.molex.cloud/docs/reference/python.html) | [Python reference](https://pamoja.molex.cloud/docs/reference/python/pamoja.html), generated by pdoc |
| C# | `dotnet add package Pamoja` | [every package](https://pamoja.molex.cloud/docs/reference/dotnet.html) | [C# reference](https://pamoja.molex.cloud/docs/reference/dotnet/index.html), generated by DocFX |
<!-- end -->

- [The guides and the install page](https://pamoja.molex.cloud/docs/).
- [The hardware](https://pamoja.molex.cloud/docs/hardware.html) the drivers were
  written against, the buses and radios the crates implement, and the boards this
  is built and tested on.
- [Why it exists](https://pamoja.molex.cloud/docs/about/why.html),
  [how it is put together](https://pamoja.molex.cloud/docs/about/architecture.html),
  and [which standards it is held to](https://pamoja.molex.cloud/docs/about/standards.html).

## Building and contributing

`cargo build --workspace` and `cargo test --workspace` build and test the engine
and every crate; each binding builds from its own directory. The full layout,
the per-language builds, and how the guide examples are spliced are on the
[building page](https://pamoja.molex.cloud/docs/about/building.html), and
[CONTRIBUTING.md](CONTRIBUTING.md) covers the conventions a change is held to.

## License

MIT. See [LICENSE-MIT](LICENSE-MIT).
