# Install

`pamoja` is the whole framework in one package, in every language:

```sh
cargo add pamoja                 # Rust
npm install pamoja               # TypeScript and Node
pip install pamoja               # Python
dotnet add package Pamoja        # C# and .NET
```

That is the right default. Every capability is also its own package, and the
sections below list them, but what you gain by picking them differs between
Rust and the bindings. It is worth knowing which before you choose.

Every crate, package, and binding shares one version and is released together,
so `0.1.15` of any package wraps `0.1.15` of every other. The
[changelog](https://github.com/molexxxx/pamoja/blob/main/CHANGELOG.md) covers
all of them in one entry.

## What picking packages changes

In Rust, it changes what gets compiled. A crate you do not name is never built,
and its dependencies are never fetched, so a narrow build is genuinely smaller
and carries less third-party code.

In the bindings it changes what you import, not what you download. Node, Python,
and .NET each load one compiled engine that carries every capability, and every
package depends on it. Choosing packages narrows the API you see, the manifest
you ship, and the code your dependency scanners have to account for. It does not
shrink the engine.

Neither is a workaround. Compiling only what you use is a property of a compiled
language, and the deployments that need it (a microcontroller with kilobytes of
flash) run Rust and could not host a Python or .NET runtime at all.

## The two things called core

A binding has one package you never name and one you sometimes do, and they are
easy to confuse.

The **compiled engine** is `@pamoja/native`, `pamoja-native`, and
`Pamoja.Native`. It is the built Rust library, the generated contract over it,
and the plumbing every facade needs to call it: the handle type, the error every
failed call raises, and string marshalling. Every package declares it, so it
arrives on its own and you never install it by hand. Rust has no equivalent,
because there you compile the crates.

The **engine surface** is `@pamoja/core`, `pamoja-core`, and `Pamoja.Core`, the
counterpart of the `pamoja-core` crate. It is the runtime version and
`Transport`, the abstraction MQTT, CoAP, and the loopback all implement. It is a
capability like any other, listed first in the tables below, and most packages do
not depend on it: only the transports do, because they are the ones that return a
`Transport`. Install it when you want to hold a link behind that interface.

## Rust

```sh
cargo add pamoja                        # every capability, behind a feature each
cargo add pamoja-modbus                 # or one crate on its own
```

<!-- snippet: examples/tests/guides/imports.rs#rust -->
From [`examples/tests/guides/imports.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/imports.rs):

```rust
use pamoja::modbus::Adu; // the same type as pamoja_modbus::Adu
use pamoja_codec::CborCodec;
```
<!-- end -->

Each module of `pamoja` is the crate of the same name, so the two ways in share
one API and one set of documentation, and code moves between them unchanged.

Naming features is what makes a build small. Measured from the resolved
dependency graph, for a `x86_64-unknown-linux-gnu` build:

<!-- table: builds -->
| Build | What you write | Crates compiled | From this workspace | External |
| --- | --- | --- | --- | --- |
| Every capability | `cargo add pamoja` | 107 | 31 | 76 |
| Codecs and identity | `cargo add pamoja --no-default-features --features codec,security` | 36 | 4 | 32 |
| Field I/O | `cargo add pamoja --no-default-features --features field-io` | 6 | 6 | 0 |
| One capability | `cargo add pamoja --no-default-features --features modbus` | 3 | 3 | 0 |
| Bare metal, no `std` | `cargo add pamoja --no-default-features --features modbus,sensors,lora` | 5 | 5 | 0 |
<!-- end -->

`field-io` there is a group feature. Six domains have one, so a build names a
domain rather than listing its parts:

<!-- table: install rust -->
<div class="domains">
<div class="domain">
<div class="domain-what"><strong>Field I/O</strong><p><a href="/docs/guides/serial.html">Serial framing</a>, <a href="/docs/guides/modbus.html">Modbus RTU</a>, <a href="/docs/guides/can.html">CAN and J1939</a>, <a href="/docs/guides/gpio.html">I2C, SPI, and GPIO</a></p></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features field-io</code><button class="copy" type="button" data-copy="cargo add pamoja --features field-io" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong>Sensing and actuation</strong><p><a href="/docs/guides/sensors.html">Sensor drivers</a>, <a href="/docs/guides/actuators.html">Actuator drivers</a></p></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features sensing</code><button class="copy" type="button" data-copy="cargo add pamoja --features sensing" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong>Radio and reach</strong><p><a href="/docs/guides/lora.html">LoRa airtime</a>, <a href="/docs/guides/lorawan.html">LoRaWAN</a>, <a href="/docs/guides/mesh.html">Mesh frames</a>, <a href="/docs/guides/routing.html">Routing</a></p></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features radio</code><button class="copy" type="button" data-copy="cargo add pamoja --features radio" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong>Trust and operation</strong><p><a href="/docs/guides/audit.html">Audit log</a>, <a href="/docs/guides/session.html">Secured session</a>, <a href="/docs/guides/update.html">Signed updates</a>, <a href="/docs/guides/power.html">Power</a>, <a href="/docs/guides/telemetry.html">Telemetry</a></p></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features trust</code><button class="copy" type="button" data-copy="cargo add pamoja --features trust" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong>Transports and testing</strong><p><a href="/docs/guides/mqtt.html">MQTT</a>, <a href="/docs/guides/coap.html">CoAP</a>, <a href="/docs/guides/loopback.html">Loopback</a>, <a href="/docs/guides/sync.html">Store and forward</a>, <a href="/docs/guides/ladder.html">Transport ladder</a>, <a href="/docs/guides/bus.html">Event bus</a>, <a href="/docs/guides/transport.html">Engine surface</a>, <a href="/docs/guides/sim.html">Simulators</a></p></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features transports</code><button class="copy" type="button" data-copy="cargo add pamoja --features transports" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong>Profiles and robotics</strong><p><a href="/docs/guides/profile.html">Device profiles</a>, <a href="/docs/guides/ros2.html">ROS 2 rules</a>, <a href="/docs/guides/zenoh.html">Zenoh keys</a></p></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features profiles</code><button class="copy" type="button" data-copy="cargo add pamoja --features profiles" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<!-- end -->

The narrow builds carry no third-party code at all: `pamoja`, `pamoja-core`, and
the capability crates, and nothing else. Most capability crates are `no_std`, so
the same code runs on a gateway and on a microcontroller. The
[Rust reference](reference/rust.md) lists every crate.

## TypeScript and Node

```sh
npm install pamoja                            # every capability
npm install @pamoja/modbus @pamoja/codec      # or only the packages you use
```

```ts
import { readHoldingRegisters } from '@pamoja/modbus'
import { toCbor, fromCbor } from '@pamoja/codec'
```

Every package depends on `@pamoja/native`, the compiled engine, prebuilt for
Linux (x64, arm64), macOS (x64, arm64), and Windows (x64); npm picks the right
one, and installs on anything else without a binary to load. Alpine and Windows
on ARM are the two that catch people out. It is one binary carrying every capability whichever packages you install,
so the choice is about the API surface and your dependency manifest, not the
download. Node 16 or later.

The same six domains, one package each. A domain package brings in its
capabilities and re-exports each under its own name, so a name two of them share
stays unambiguous:

<!-- table: install node -->
<div class="domains">
<div class="domain">
<div class="domain-what"><strong><a href="https://www.npmjs.com/package/@pamoja/field-io">Field I/O</a></strong><p><a href="/docs/guides/serial.html">Serial framing</a>, <a href="/docs/guides/modbus.html">Modbus RTU</a>, <a href="/docs/guides/can.html">CAN and J1939</a>, <a href="/docs/guides/gpio.html">I2C, SPI, and GPIO</a></p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/field-io</code><button class="copy" type="button" data-copy="npm install @pamoja/field-io" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://www.npmjs.com/package/@pamoja/sensing">Sensing and actuation</a></strong><p><a href="/docs/guides/sensors.html">Sensor drivers</a>, <a href="/docs/guides/actuators.html">Actuator drivers</a></p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/sensing</code><button class="copy" type="button" data-copy="npm install @pamoja/sensing" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://www.npmjs.com/package/@pamoja/radio">Radio and reach</a></strong><p><a href="/docs/guides/lora.html">LoRa airtime</a>, <a href="/docs/guides/lorawan.html">LoRaWAN</a>, <a href="/docs/guides/mesh.html">Mesh frames</a>, <a href="/docs/guides/routing.html">Routing</a></p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/radio</code><button class="copy" type="button" data-copy="npm install @pamoja/radio" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://www.npmjs.com/package/@pamoja/trust">Trust and operation</a></strong><p><a href="/docs/guides/audit.html">Audit log</a>, <a href="/docs/guides/session.html">Secured session</a>, <a href="/docs/guides/update.html">Signed updates</a>, <a href="/docs/guides/power.html">Power</a>, <a href="/docs/guides/telemetry.html">Telemetry</a></p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/trust</code><button class="copy" type="button" data-copy="npm install @pamoja/trust" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://www.npmjs.com/package/@pamoja/transports">Transports and testing</a></strong><p><a href="/docs/guides/mqtt.html">MQTT</a>, <a href="/docs/guides/coap.html">CoAP</a>, <a href="/docs/guides/loopback.html">Loopback</a>, <a href="/docs/guides/sync.html">Store and forward</a>, <a href="/docs/guides/ladder.html">Transport ladder</a>, <a href="/docs/guides/bus.html">Event bus</a>, <a href="/docs/guides/transport.html">Engine surface</a>, <a href="/docs/guides/sim.html">Simulators</a></p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/transports</code><button class="copy" type="button" data-copy="npm install @pamoja/transports" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://www.npmjs.com/package/@pamoja/profiles">Profiles and robotics</a></strong><p><a href="/docs/guides/profile.html">Device profiles</a>, <a href="/docs/guides/ros2.html">ROS 2 rules</a>, <a href="/docs/guides/zenoh.html">Zenoh keys</a></p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/profiles</code><button class="copy" type="button" data-copy="npm install @pamoja/profiles" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<!-- end -->

<!-- table: packages node -->
### Engine

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/transport.html#typescript">Engine surface</a><p>The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/core</code><button class="copy" type="button" data-copy="npm install @pamoja/core" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_core.html"><code>@pamoja/core</code></a></li><li><a href="/docs/guides/transport.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/core">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-core" title="pamoja-core">Rust</a> <a href="https://pypi.org/project/pamoja-core/" title="pamoja-core">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Core" title="Pamoja.Core">C#</a></p>
</div>
</div>

### Identity

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/security.html#typescript">Device identity</a><p>ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/security</code><button class="copy" type="button" data-copy="npm install @pamoja/security" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_security.html"><code>@pamoja/security</code></a></li><li><a href="/docs/guides/security.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/security">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-security" title="pamoja-security">Rust</a> <a href="https://pypi.org/project/pamoja-security/" title="pamoja-security">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Security" title="Pamoja.Security">C#</a></p>
</div>
</div>

### Codecs

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/codec.html#typescript">Codecs</a><p>CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/codec</code><button class="copy" type="button" data-copy="npm install @pamoja/codec" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_codec.html"><code>@pamoja/codec</code></a></li><li><a href="/docs/guides/codec.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/codec">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-codec" title="pamoja-codec">Rust</a> <a href="https://pypi.org/project/pamoja-codec/" title="pamoja-codec">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Codec" title="Pamoja.Codec">C#</a></p>
</div>
</div>

### Helpers

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/kit.html#typescript">Helpers</a><p>Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/kit</code><button class="copy" type="button" data-copy="npm install @pamoja/kit" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_kit.html"><code>@pamoja/kit</code></a></li><li><a href="/docs/guides/kit.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/kit">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-kit" title="pamoja-kit">Rust</a> <a href="https://pypi.org/project/pamoja-kit/" title="pamoja-kit">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Kit" title="Pamoja.Kit">C#</a></p>
</div>
</div>

### Field I/O

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/serial.html#typescript">Serial framing</a><p>SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/serial</code><button class="copy" type="button" data-copy="npm install @pamoja/serial" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_serial.html"><code>@pamoja/serial</code></a></li><li><a href="/docs/guides/serial.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/serial">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-serial" title="pamoja-serial">Rust</a> <a href="https://pypi.org/project/pamoja-serial/" title="pamoja-serial">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Serial" title="Pamoja.Serial">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/modbus.html#typescript">Modbus RTU</a><p>Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/modbus</code><button class="copy" type="button" data-copy="npm install @pamoja/modbus" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_modbus.html"><code>@pamoja/modbus</code></a></li><li><a href="/docs/guides/modbus.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/modbus">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-modbus" title="pamoja-modbus">Rust</a> <a href="https://pypi.org/project/pamoja-modbus/" title="pamoja-modbus">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Modbus" title="Pamoja.Modbus">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/can.html#typescript">CAN and J1939</a><p>CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/can</code><button class="copy" type="button" data-copy="npm install @pamoja/can" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_can.html"><code>@pamoja/can</code></a></li><li><a href="/docs/guides/can.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/can">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-can" title="pamoja-can">Rust</a> <a href="https://pypi.org/project/pamoja-can/" title="pamoja-can">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Can" title="Pamoja.Can">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/gpio.html#typescript">I2C, SPI, and GPIO</a><p>I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/gpio</code><button class="copy" type="button" data-copy="npm install @pamoja/gpio" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_gpio.html"><code>@pamoja/gpio</code></a></li><li><a href="/docs/guides/gpio.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/gpio">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-gpio" title="pamoja-gpio">Rust</a> <a href="https://pypi.org/project/pamoja-gpio/" title="pamoja-gpio">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Gpio" title="Pamoja.Gpio">C#</a></p>
</div>
</div>

### Sensing and actuation

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/sensors.html#typescript">Sensor drivers</a><p>Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/sensors</code><button class="copy" type="button" data-copy="npm install @pamoja/sensors" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_sensors.html"><code>@pamoja/sensors</code></a></li><li><a href="/docs/guides/sensors.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/sensors">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-sensors" title="pamoja-sensors">Rust</a> <a href="https://pypi.org/project/pamoja-sensors/" title="pamoja-sensors">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Sensors" title="Pamoja.Sensors">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/actuators.html#typescript">Actuator drivers</a><p>PCA9685 PWM and servo pulses, and stepper coil sequencing</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/actuators</code><button class="copy" type="button" data-copy="npm install @pamoja/actuators" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_actuators.html"><code>@pamoja/actuators</code></a></li><li><a href="/docs/guides/actuators.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/actuators">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-actuators" title="pamoja-actuators">Rust</a> <a href="https://pypi.org/project/pamoja-actuators/" title="pamoja-actuators">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Actuators" title="Pamoja.Actuators">C#</a></p>
</div>
</div>

### Radio and reach

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/lora.html#typescript">LoRa airtime</a><p>Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/lora</code><button class="copy" type="button" data-copy="npm install @pamoja/lora" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_lora.html"><code>@pamoja/lora</code></a></li><li><a href="/docs/guides/lora.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/lora">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-lora" title="pamoja-lora">Rust</a> <a href="https://pypi.org/project/pamoja-lora/" title="pamoja-lora">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Lora" title="Pamoja.Lora">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/lorawan.html#typescript">LoRaWAN</a><p>LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/lorawan</code><button class="copy" type="button" data-copy="npm install @pamoja/lorawan" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_lorawan.html"><code>@pamoja/lorawan</code></a></li><li><a href="/docs/guides/lorawan.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/lorawan">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-lorawan" title="pamoja-lorawan">Rust</a> <a href="https://pypi.org/project/pamoja-lorawan/" title="pamoja-lorawan">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Lorawan" title="Pamoja.Lorawan">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/mesh.html#typescript">Mesh frames</a><p>Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/mesh</code><button class="copy" type="button" data-copy="npm install @pamoja/mesh" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_mesh.html"><code>@pamoja/mesh</code></a></li><li><a href="/docs/guides/mesh.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/mesh">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-mesh" title="pamoja-mesh">Rust</a> <a href="https://pypi.org/project/pamoja-mesh/" title="pamoja-mesh">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Mesh" title="Pamoja.Mesh">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/routing.html#typescript">Routing</a><p>Reverse-path routing that learns the cheapest route from overheard traffic</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/routing</code><button class="copy" type="button" data-copy="npm install @pamoja/routing" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_routing.html"><code>@pamoja/routing</code></a></li><li><a href="/docs/guides/routing.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/routing">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-routing" title="pamoja-routing">Rust</a> <a href="https://pypi.org/project/pamoja-routing/" title="pamoja-routing">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Routing" title="Pamoja.Routing">C#</a></p>
</div>
</div>

### MAVLink

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/mavlink.html#typescript">MAVLink</a><p>MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/mavlink</code><button class="copy" type="button" data-copy="npm install @pamoja/mavlink" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_mavlink.html"><code>@pamoja/mavlink</code></a></li><li><a href="/docs/guides/mavlink.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/mavlink">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-mavlink" title="pamoja-mavlink">Rust</a> <a href="https://pypi.org/project/pamoja-mavlink/" title="pamoja-mavlink">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Mavlink" title="Pamoja.Mavlink">C#</a></p>
</div>
</div>

### Trust and operation

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/audit.html#typescript">Audit log</a><p>A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/audit</code><button class="copy" type="button" data-copy="npm install @pamoja/audit" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_audit.html"><code>@pamoja/audit</code></a></li><li><a href="/docs/guides/audit.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/audit">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-audit" title="pamoja-audit">Rust</a> <a href="https://pypi.org/project/pamoja-audit/" title="pamoja-audit">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Audit" title="Pamoja.Audit">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/session.html#typescript">Secured session</a><p>X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/session</code><button class="copy" type="button" data-copy="npm install @pamoja/session" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_session.html"><code>@pamoja/session</code></a></li><li><a href="/docs/guides/session.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/session">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-session" title="pamoja-session">Rust</a> <a href="https://pypi.org/project/pamoja-session/" title="pamoja-session">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Session" title="Pamoja.Session">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/update.html#typescript">Signed updates</a><p>Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/update</code><button class="copy" type="button" data-copy="npm install @pamoja/update" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_update.html"><code>@pamoja/update</code></a></li><li><a href="/docs/guides/update.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/update">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-update" title="pamoja-update">Rust</a> <a href="https://pypi.org/project/pamoja-update/" title="pamoja-update">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Update" title="Pamoja.Update">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/power.html#typescript">Power</a><p>Duty cycling and an energy-aware governor that stretches work as the battery drains</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/power</code><button class="copy" type="button" data-copy="npm install @pamoja/power" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_power.html"><code>@pamoja/power</code></a></li><li><a href="/docs/guides/power.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/power">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-power" title="pamoja-power">Rust</a> <a href="https://pypi.org/project/pamoja-power/" title="pamoja-power">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Power" title="Pamoja.Power">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/telemetry.html#typescript">Telemetry</a><p>Observability that ships only what is worth the bytes as link cost rises, while counting everything</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/telemetry</code><button class="copy" type="button" data-copy="npm install @pamoja/telemetry" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_telemetry.html"><code>@pamoja/telemetry</code></a></li><li><a href="/docs/guides/telemetry.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/telemetry">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-telemetry" title="pamoja-telemetry">Rust</a> <a href="https://pypi.org/project/pamoja-telemetry/" title="pamoja-telemetry">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Telemetry" title="Pamoja.Telemetry">C#</a></p>
</div>
</div>

### Transports and testing

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/mqtt.html#typescript">MQTT</a><p>An MQTT client with the topic and wildcard rules, as the core transport</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/mqtt</code><button class="copy" type="button" data-copy="npm install @pamoja/mqtt" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_mqtt.html"><code>@pamoja/mqtt</code></a></li><li><a href="/docs/guides/mqtt.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/mqtt">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-mqtt" title="pamoja-mqtt">Rust</a> <a href="https://pypi.org/project/pamoja-mqtt/" title="pamoja-mqtt">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Mqtt" title="Pamoja.Mqtt">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/coap.html#typescript">CoAP</a><p>A CoAP client over UDP with confirmable delivery and observe</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/coap</code><button class="copy" type="button" data-copy="npm install @pamoja/coap" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_coap.html"><code>@pamoja/coap</code></a></li><li><a href="/docs/guides/coap.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/coap">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-coap" title="pamoja-coap">Rust</a> <a href="https://pypi.org/project/pamoja-coap/" title="pamoja-coap">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Coap" title="Pamoja.Coap">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/loopback.html#typescript">Loopback</a><p>An in-process transport with topic matching and a fault injector, for testing with no broker</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/loopback</code><button class="copy" type="button" data-copy="npm install @pamoja/loopback" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_loopback.html"><code>@pamoja/loopback</code></a></li><li><a href="/docs/guides/loopback.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/loopback">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-loopback" title="pamoja-loopback">Rust</a> <a href="https://pypi.org/project/pamoja-loopback/" title="pamoja-loopback">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Loopback" title="Pamoja.Loopback">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/sync.html#typescript">Store and forward</a><p>Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/sync</code><button class="copy" type="button" data-copy="npm install @pamoja/sync" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_sync.html"><code>@pamoja/sync</code></a></li><li><a href="/docs/guides/sync.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/sync">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-sync" title="pamoja-sync">Rust</a> <a href="https://pypi.org/project/pamoja-sync/" title="pamoja-sync">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Sync" title="Pamoja.Sync">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/ladder.html#typescript">Transport ladder</a><p>Cheapest reachable link first, buffering to a store when every link is down</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/ladder</code><button class="copy" type="button" data-copy="npm install @pamoja/ladder" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_ladder.html"><code>@pamoja/ladder</code></a></li><li><a href="/docs/guides/ladder.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/ladder">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-ladder" title="pamoja-ladder">Rust</a> <a href="https://pypi.org/project/pamoja-ladder/" title="pamoja-ladder">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Ladder" title="Pamoja.Ladder">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/bus.html#typescript">Event bus</a><p>An in-memory typed publish and subscribe event bus</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/bus</code><button class="copy" type="button" data-copy="npm install @pamoja/bus" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_bus.html"><code>@pamoja/bus</code></a></li><li><a href="/docs/guides/bus.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/bus">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-bus" title="pamoja-bus">Rust</a> <a href="https://pypi.org/project/pamoja-bus/" title="pamoja-bus">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Bus" title="Pamoja.Bus">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/sim.html#typescript">Simulators</a><p>Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/sim</code><button class="copy" type="button" data-copy="npm install @pamoja/sim" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_sim.html"><code>@pamoja/sim</code></a></li><li><a href="/docs/guides/sim.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/sim">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-sim" title="pamoja-sim">Rust</a> <a href="https://pypi.org/project/pamoja-sim/" title="pamoja-sim">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Sim" title="Pamoja.Sim">C#</a></p>
</div>
</div>

### Profiles and robotics

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/profile.html#typescript">Device profiles</a><p>Named, ready-to-run device profiles from plain data or a JSON manifest</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/profile</code><button class="copy" type="button" data-copy="npm install @pamoja/profile" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_profile.html"><code>@pamoja/profile</code></a></li><li><a href="/docs/guides/profile.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/profile">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-profile" title="pamoja-profile">Rust</a> <a href="https://pypi.org/project/pamoja-profile/" title="pamoja-profile">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Profile" title="Pamoja.Profile">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/ros2.html#typescript">ROS 2 rules</a><p>ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/ros2</code><button class="copy" type="button" data-copy="npm install @pamoja/ros2" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_ros2.html"><code>@pamoja/ros2</code></a></li><li><a href="/docs/guides/ros2.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/ros2">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-ros2" title="pamoja-ros2">Rust</a> <a href="https://pypi.org/project/pamoja-ros2/" title="pamoja-ros2">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Ros2" title="Pamoja.Ros2">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/zenoh.html#typescript">Zenoh keys</a><p>Zenoh key expressions: validity, canonical form, and wildcard matching</p></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/zenoh</code><button class="copy" type="button" data-copy="npm install @pamoja/zenoh" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/node/modules/_pamoja_zenoh.html"><code>@pamoja/zenoh</code></a></li><li><a href="/docs/guides/zenoh.html#typescript">worked example</a></li><li><a href="https://www.npmjs.com/package/@pamoja/zenoh">npm</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-zenoh" title="pamoja-zenoh">Rust</a> <a href="https://pypi.org/project/pamoja-zenoh/" title="pamoja-zenoh">Python</a> <a href="https://www.nuget.org/packages/Pamoja.Zenoh" title="Pamoja.Zenoh">C#</a></p>
</div>
</div>
<!-- end -->

## Python

```sh
pip install pamoja                          # every capability
pip install pamoja-modbus pamoja-codec      # or only the distributions you use
```

```python
from pamoja.modbus import read_holding_registers
from pamoja.codec import to_cbor, from_cbor
```

`pamoja` is a namespace package: each distribution ships one `pamoja.<name>`
module and they merge on import. Every distribution depends on `pamoja-native`,
the compiled engine, with wheels for the same platforms as the Node engine and
for Python 3.10 and later; elsewhere `pip` builds it from the sdist, which needs
a Rust toolchain.

The same six domains, one package each. A domain package brings in its
capabilities and re-exports each under its own name, so a name two of them share
stays unambiguous:

<!-- table: install python -->
<div class="domains">
<div class="domain">
<div class="domain-what"><strong><a href="https://pypi.org/project/pamoja-field-io/">Field I/O</a></strong><p><a href="/docs/guides/serial.html">Serial framing</a>, <a href="/docs/guides/modbus.html">Modbus RTU</a>, <a href="/docs/guides/can.html">CAN and J1939</a>, <a href="/docs/guides/gpio.html">I2C, SPI, and GPIO</a></p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-field-io</code><button class="copy" type="button" data-copy="pip install pamoja-field-io" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://pypi.org/project/pamoja-sensing/">Sensing and actuation</a></strong><p><a href="/docs/guides/sensors.html">Sensor drivers</a>, <a href="/docs/guides/actuators.html">Actuator drivers</a></p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-sensing</code><button class="copy" type="button" data-copy="pip install pamoja-sensing" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://pypi.org/project/pamoja-radio/">Radio and reach</a></strong><p><a href="/docs/guides/lora.html">LoRa airtime</a>, <a href="/docs/guides/lorawan.html">LoRaWAN</a>, <a href="/docs/guides/mesh.html">Mesh frames</a>, <a href="/docs/guides/routing.html">Routing</a></p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-radio</code><button class="copy" type="button" data-copy="pip install pamoja-radio" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://pypi.org/project/pamoja-trust/">Trust and operation</a></strong><p><a href="/docs/guides/audit.html">Audit log</a>, <a href="/docs/guides/session.html">Secured session</a>, <a href="/docs/guides/update.html">Signed updates</a>, <a href="/docs/guides/power.html">Power</a>, <a href="/docs/guides/telemetry.html">Telemetry</a></p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-trust</code><button class="copy" type="button" data-copy="pip install pamoja-trust" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://pypi.org/project/pamoja-transports/">Transports and testing</a></strong><p><a href="/docs/guides/mqtt.html">MQTT</a>, <a href="/docs/guides/coap.html">CoAP</a>, <a href="/docs/guides/loopback.html">Loopback</a>, <a href="/docs/guides/sync.html">Store and forward</a>, <a href="/docs/guides/ladder.html">Transport ladder</a>, <a href="/docs/guides/bus.html">Event bus</a>, <a href="/docs/guides/transport.html">Engine surface</a>, <a href="/docs/guides/sim.html">Simulators</a></p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-transports</code><button class="copy" type="button" data-copy="pip install pamoja-transports" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://pypi.org/project/pamoja-profiles/">Profiles and robotics</a></strong><p><a href="/docs/guides/profile.html">Device profiles</a>, <a href="/docs/guides/ros2.html">ROS 2 rules</a>, <a href="/docs/guides/zenoh.html">Zenoh keys</a></p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-profiles</code><button class="copy" type="button" data-copy="pip install pamoja-profiles" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<!-- end -->

<!-- table: packages python -->
### Engine

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/transport.html#python">Engine surface</a><p>The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-core</code><button class="copy" type="button" data-copy="pip install pamoja-core" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/core.html"><code>pamoja.core</code></a></li><li><a href="/docs/guides/transport.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-core/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-core" title="pamoja-core">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/core" title="@pamoja/core">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Core" title="Pamoja.Core">C#</a></p>
</div>
</div>

### Identity

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/security.html#python">Device identity</a><p>ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-security</code><button class="copy" type="button" data-copy="pip install pamoja-security" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/security.html"><code>pamoja.security</code></a></li><li><a href="/docs/guides/security.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-security/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-security" title="pamoja-security">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/security" title="@pamoja/security">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Security" title="Pamoja.Security">C#</a></p>
</div>
</div>

### Codecs

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/codec.html#python">Codecs</a><p>CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-codec</code><button class="copy" type="button" data-copy="pip install pamoja-codec" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/codec.html"><code>pamoja.codec</code></a></li><li><a href="/docs/guides/codec.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-codec/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-codec" title="pamoja-codec">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/codec" title="@pamoja/codec">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Codec" title="Pamoja.Codec">C#</a></p>
</div>
</div>

### Helpers

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/kit.html#python">Helpers</a><p>Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-kit</code><button class="copy" type="button" data-copy="pip install pamoja-kit" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/kit.html"><code>pamoja.kit</code></a></li><li><a href="/docs/guides/kit.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-kit/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-kit" title="pamoja-kit">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/kit" title="@pamoja/kit">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Kit" title="Pamoja.Kit">C#</a></p>
</div>
</div>

### Field I/O

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/serial.html#python">Serial framing</a><p>SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-serial</code><button class="copy" type="button" data-copy="pip install pamoja-serial" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/serial.html"><code>pamoja.serial</code></a></li><li><a href="/docs/guides/serial.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-serial/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-serial" title="pamoja-serial">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/serial" title="@pamoja/serial">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Serial" title="Pamoja.Serial">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/modbus.html#python">Modbus RTU</a><p>Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-modbus</code><button class="copy" type="button" data-copy="pip install pamoja-modbus" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/modbus.html"><code>pamoja.modbus</code></a></li><li><a href="/docs/guides/modbus.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-modbus/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-modbus" title="pamoja-modbus">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/modbus" title="@pamoja/modbus">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Modbus" title="Pamoja.Modbus">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/can.html#python">CAN and J1939</a><p>CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-can</code><button class="copy" type="button" data-copy="pip install pamoja-can" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/can.html"><code>pamoja.can</code></a></li><li><a href="/docs/guides/can.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-can/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-can" title="pamoja-can">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/can" title="@pamoja/can">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Can" title="Pamoja.Can">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/gpio.html#python">I2C, SPI, and GPIO</a><p>I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-gpio</code><button class="copy" type="button" data-copy="pip install pamoja-gpio" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/gpio.html"><code>pamoja.gpio</code></a></li><li><a href="/docs/guides/gpio.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-gpio/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-gpio" title="pamoja-gpio">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/gpio" title="@pamoja/gpio">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Gpio" title="Pamoja.Gpio">C#</a></p>
</div>
</div>

### Sensing and actuation

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/sensors.html#python">Sensor drivers</a><p>Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-sensors</code><button class="copy" type="button" data-copy="pip install pamoja-sensors" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/sensors.html"><code>pamoja.sensors</code></a></li><li><a href="/docs/guides/sensors.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-sensors/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-sensors" title="pamoja-sensors">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/sensors" title="@pamoja/sensors">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Sensors" title="Pamoja.Sensors">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/actuators.html#python">Actuator drivers</a><p>PCA9685 PWM and servo pulses, and stepper coil sequencing</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-actuators</code><button class="copy" type="button" data-copy="pip install pamoja-actuators" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/actuators.html"><code>pamoja.actuators</code></a></li><li><a href="/docs/guides/actuators.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-actuators/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-actuators" title="pamoja-actuators">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/actuators" title="@pamoja/actuators">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Actuators" title="Pamoja.Actuators">C#</a></p>
</div>
</div>

### Radio and reach

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/lora.html#python">LoRa airtime</a><p>Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-lora</code><button class="copy" type="button" data-copy="pip install pamoja-lora" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/lora.html"><code>pamoja.lora</code></a></li><li><a href="/docs/guides/lora.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-lora/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-lora" title="pamoja-lora">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/lora" title="@pamoja/lora">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Lora" title="Pamoja.Lora">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/lorawan.html#python">LoRaWAN</a><p>LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-lorawan</code><button class="copy" type="button" data-copy="pip install pamoja-lorawan" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/lorawan.html"><code>pamoja.lorawan</code></a></li><li><a href="/docs/guides/lorawan.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-lorawan/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-lorawan" title="pamoja-lorawan">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/lorawan" title="@pamoja/lorawan">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Lorawan" title="Pamoja.Lorawan">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/mesh.html#python">Mesh frames</a><p>Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-mesh</code><button class="copy" type="button" data-copy="pip install pamoja-mesh" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/mesh.html"><code>pamoja.mesh</code></a></li><li><a href="/docs/guides/mesh.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-mesh/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-mesh" title="pamoja-mesh">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/mesh" title="@pamoja/mesh">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Mesh" title="Pamoja.Mesh">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/routing.html#python">Routing</a><p>Reverse-path routing that learns the cheapest route from overheard traffic</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-routing</code><button class="copy" type="button" data-copy="pip install pamoja-routing" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/routing.html"><code>pamoja.routing</code></a></li><li><a href="/docs/guides/routing.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-routing/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-routing" title="pamoja-routing">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/routing" title="@pamoja/routing">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Routing" title="Pamoja.Routing">C#</a></p>
</div>
</div>

### MAVLink

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/mavlink.html#python">MAVLink</a><p>MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-mavlink</code><button class="copy" type="button" data-copy="pip install pamoja-mavlink" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/mavlink.html"><code>pamoja.mavlink</code></a></li><li><a href="/docs/guides/mavlink.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-mavlink/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-mavlink" title="pamoja-mavlink">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/mavlink" title="@pamoja/mavlink">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Mavlink" title="Pamoja.Mavlink">C#</a></p>
</div>
</div>

### Trust and operation

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/audit.html#python">Audit log</a><p>A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-audit</code><button class="copy" type="button" data-copy="pip install pamoja-audit" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/audit.html"><code>pamoja.audit</code></a></li><li><a href="/docs/guides/audit.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-audit/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-audit" title="pamoja-audit">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/audit" title="@pamoja/audit">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Audit" title="Pamoja.Audit">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/session.html#python">Secured session</a><p>X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-session</code><button class="copy" type="button" data-copy="pip install pamoja-session" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/session.html"><code>pamoja.session</code></a></li><li><a href="/docs/guides/session.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-session/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-session" title="pamoja-session">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/session" title="@pamoja/session">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Session" title="Pamoja.Session">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/update.html#python">Signed updates</a><p>Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-update</code><button class="copy" type="button" data-copy="pip install pamoja-update" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/update.html"><code>pamoja.update</code></a></li><li><a href="/docs/guides/update.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-update/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-update" title="pamoja-update">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/update" title="@pamoja/update">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Update" title="Pamoja.Update">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/power.html#python">Power</a><p>Duty cycling and an energy-aware governor that stretches work as the battery drains</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-power</code><button class="copy" type="button" data-copy="pip install pamoja-power" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/power.html"><code>pamoja.power</code></a></li><li><a href="/docs/guides/power.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-power/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-power" title="pamoja-power">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/power" title="@pamoja/power">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Power" title="Pamoja.Power">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/telemetry.html#python">Telemetry</a><p>Observability that ships only what is worth the bytes as link cost rises, while counting everything</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-telemetry</code><button class="copy" type="button" data-copy="pip install pamoja-telemetry" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/telemetry.html"><code>pamoja.telemetry</code></a></li><li><a href="/docs/guides/telemetry.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-telemetry/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-telemetry" title="pamoja-telemetry">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/telemetry" title="@pamoja/telemetry">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Telemetry" title="Pamoja.Telemetry">C#</a></p>
</div>
</div>

### Transports and testing

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/mqtt.html#python">MQTT</a><p>An MQTT client with the topic and wildcard rules, as the core transport</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-mqtt</code><button class="copy" type="button" data-copy="pip install pamoja-mqtt" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/mqtt.html"><code>pamoja.mqtt</code></a></li><li><a href="/docs/guides/mqtt.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-mqtt/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-mqtt" title="pamoja-mqtt">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/mqtt" title="@pamoja/mqtt">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Mqtt" title="Pamoja.Mqtt">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/coap.html#python">CoAP</a><p>A CoAP client over UDP with confirmable delivery and observe</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-coap</code><button class="copy" type="button" data-copy="pip install pamoja-coap" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/coap.html"><code>pamoja.coap</code></a></li><li><a href="/docs/guides/coap.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-coap/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-coap" title="pamoja-coap">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/coap" title="@pamoja/coap">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Coap" title="Pamoja.Coap">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/loopback.html#python">Loopback</a><p>An in-process transport with topic matching and a fault injector, for testing with no broker</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-loopback</code><button class="copy" type="button" data-copy="pip install pamoja-loopback" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/loopback.html"><code>pamoja.loopback</code></a></li><li><a href="/docs/guides/loopback.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-loopback/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-loopback" title="pamoja-loopback">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/loopback" title="@pamoja/loopback">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Loopback" title="Pamoja.Loopback">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/sync.html#python">Store and forward</a><p>Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-sync</code><button class="copy" type="button" data-copy="pip install pamoja-sync" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/sync.html"><code>pamoja.sync</code></a></li><li><a href="/docs/guides/sync.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-sync/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-sync" title="pamoja-sync">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/sync" title="@pamoja/sync">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Sync" title="Pamoja.Sync">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/ladder.html#python">Transport ladder</a><p>Cheapest reachable link first, buffering to a store when every link is down</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-ladder</code><button class="copy" type="button" data-copy="pip install pamoja-ladder" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/ladder.html"><code>pamoja.ladder</code></a></li><li><a href="/docs/guides/ladder.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-ladder/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-ladder" title="pamoja-ladder">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/ladder" title="@pamoja/ladder">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Ladder" title="Pamoja.Ladder">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/bus.html#python">Event bus</a><p>An in-memory typed publish and subscribe event bus</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-bus</code><button class="copy" type="button" data-copy="pip install pamoja-bus" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/bus.html"><code>pamoja.bus</code></a></li><li><a href="/docs/guides/bus.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-bus/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-bus" title="pamoja-bus">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/bus" title="@pamoja/bus">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Bus" title="Pamoja.Bus">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/sim.html#python">Simulators</a><p>Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-sim</code><button class="copy" type="button" data-copy="pip install pamoja-sim" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/sim.html"><code>pamoja.sim</code></a></li><li><a href="/docs/guides/sim.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-sim/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-sim" title="pamoja-sim">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/sim" title="@pamoja/sim">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Sim" title="Pamoja.Sim">C#</a></p>
</div>
</div>

### Profiles and robotics

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/profile.html#python">Device profiles</a><p>Named, ready-to-run device profiles from plain data or a JSON manifest</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-profile</code><button class="copy" type="button" data-copy="pip install pamoja-profile" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/profile.html"><code>pamoja.profile</code></a></li><li><a href="/docs/guides/profile.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-profile/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-profile" title="pamoja-profile">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/profile" title="@pamoja/profile">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Profile" title="Pamoja.Profile">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/ros2.html#python">ROS 2 rules</a><p>ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-ros2</code><button class="copy" type="button" data-copy="pip install pamoja-ros2" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/ros2.html"><code>pamoja.ros2</code></a></li><li><a href="/docs/guides/ros2.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-ros2/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-ros2" title="pamoja-ros2">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/ros2" title="@pamoja/ros2">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Ros2" title="Pamoja.Ros2">C#</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/zenoh.html#python">Zenoh keys</a><p>Zenoh key expressions: validity, canonical form, and wildcard matching</p></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-zenoh</code><button class="copy" type="button" data-copy="pip install pamoja-zenoh" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/python/pamoja/zenoh.html"><code>pamoja.zenoh</code></a></li><li><a href="/docs/guides/zenoh.html#python">worked example</a></li><li><a href="https://pypi.org/project/pamoja-zenoh/">PyPI</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-zenoh" title="pamoja-zenoh">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/zenoh" title="@pamoja/zenoh">TypeScript</a> <a href="https://www.nuget.org/packages/Pamoja.Zenoh" title="Pamoja.Zenoh">C#</a></p>
</div>
</div>
<!-- end -->

## C# and .NET

```sh
dotnet add package Pamoja                        # every capability
dotnet add package Pamoja.Modbus Pamoja.Codec    # or only the packages you use
```

```csharp
using Pamoja.Modbus;
using Pamoja.Codec;
```

A domain package there brings in its capabilities and ships no assembly of its
own, since C# has no way to re-export a namespace, so a type is named the way it
is when its package is referenced directly.

Each package is one namespace of the same name. Every package depends on
`Pamoja.Native`, which carries the native library for `win-x64`, `linux-x64`,
`linux-arm64`, `osx-x64`, and `osx-arm64`, and targets .NET 8. Those five are the
whole list: on any other runtime identifier, `win-arm64` and the musl-based
distributions among them, the packages restore and compile and then throw
`DllNotFoundException` on the first call, because there is no native library to
load and, unlike Python, no source build to fall back on.

The same six domains, one package each. A domain package brings in its
capabilities and re-exports each under its own name, so a name two of them share
stays unambiguous:

<!-- table: install dotnet -->
<div class="domains">
<div class="domain">
<div class="domain-what"><strong><a href="https://www.nuget.org/packages/Pamoja.FieldIo">Field I/O</a></strong><p><a href="/docs/guides/serial.html">Serial framing</a>, <a href="/docs/guides/modbus.html">Modbus RTU</a>, <a href="/docs/guides/can.html">CAN and J1939</a>, <a href="/docs/guides/gpio.html">I2C, SPI, and GPIO</a></p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.FieldIo</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.FieldIo" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://www.nuget.org/packages/Pamoja.Sensing">Sensing and actuation</a></strong><p><a href="/docs/guides/sensors.html">Sensor drivers</a>, <a href="/docs/guides/actuators.html">Actuator drivers</a></p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Sensing</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Sensing" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://www.nuget.org/packages/Pamoja.Radio">Radio and reach</a></strong><p><a href="/docs/guides/lora.html">LoRa airtime</a>, <a href="/docs/guides/lorawan.html">LoRaWAN</a>, <a href="/docs/guides/mesh.html">Mesh frames</a>, <a href="/docs/guides/routing.html">Routing</a></p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Radio</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Radio" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://www.nuget.org/packages/Pamoja.Trust">Trust and operation</a></strong><p><a href="/docs/guides/audit.html">Audit log</a>, <a href="/docs/guides/session.html">Secured session</a>, <a href="/docs/guides/update.html">Signed updates</a>, <a href="/docs/guides/power.html">Power</a>, <a href="/docs/guides/telemetry.html">Telemetry</a></p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Trust</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Trust" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://www.nuget.org/packages/Pamoja.Transports">Transports and testing</a></strong><p><a href="/docs/guides/mqtt.html">MQTT</a>, <a href="/docs/guides/coap.html">CoAP</a>, <a href="/docs/guides/loopback.html">Loopback</a>, <a href="/docs/guides/sync.html">Store and forward</a>, <a href="/docs/guides/ladder.html">Transport ladder</a>, <a href="/docs/guides/bus.html">Event bus</a>, <a href="/docs/guides/transport.html">Engine surface</a>, <a href="/docs/guides/sim.html">Simulators</a></p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Transports</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Transports" aria-label="Copy the install command">copy</button></div>
</div>
<div class="domain">
<div class="domain-what"><strong><a href="https://www.nuget.org/packages/Pamoja.Profiles">Profiles and robotics</a></strong><p><a href="/docs/guides/profile.html">Device profiles</a>, <a href="/docs/guides/ros2.html">ROS 2 rules</a>, <a href="/docs/guides/zenoh.html">Zenoh keys</a></p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Profiles</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Profiles" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<!-- end -->

<!-- table: packages dotnet -->
### Engine

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/transport.html#c">Engine surface</a><p>The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Core</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Core" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Core.html"><code>Pamoja.Core</code></a></li><li><a href="/docs/guides/transport.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Core">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-core" title="pamoja-core">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/core" title="@pamoja/core">TypeScript</a> <a href="https://pypi.org/project/pamoja-core/" title="pamoja-core">Python</a></p>
</div>
</div>

### Identity

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/security.html#c">Device identity</a><p>ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Security</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Security" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Security.html"><code>Pamoja.Security</code></a></li><li><a href="/docs/guides/security.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Security">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-security" title="pamoja-security">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/security" title="@pamoja/security">TypeScript</a> <a href="https://pypi.org/project/pamoja-security/" title="pamoja-security">Python</a></p>
</div>
</div>

### Codecs

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/codec.html#c">Codecs</a><p>CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Codec</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Codec" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Codec.html"><code>Pamoja.Codec</code></a></li><li><a href="/docs/guides/codec.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Codec">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-codec" title="pamoja-codec">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/codec" title="@pamoja/codec">TypeScript</a> <a href="https://pypi.org/project/pamoja-codec/" title="pamoja-codec">Python</a></p>
</div>
</div>

### Helpers

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/kit.html#c">Helpers</a><p>Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Kit</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Kit" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Kit.html"><code>Pamoja.Kit</code></a></li><li><a href="/docs/guides/kit.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Kit">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-kit" title="pamoja-kit">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/kit" title="@pamoja/kit">TypeScript</a> <a href="https://pypi.org/project/pamoja-kit/" title="pamoja-kit">Python</a></p>
</div>
</div>

### Field I/O

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/serial.html#c">Serial framing</a><p>SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Serial</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Serial" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Serial.html"><code>Pamoja.Serial</code></a></li><li><a href="/docs/guides/serial.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Serial">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-serial" title="pamoja-serial">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/serial" title="@pamoja/serial">TypeScript</a> <a href="https://pypi.org/project/pamoja-serial/" title="pamoja-serial">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/modbus.html#c">Modbus RTU</a><p>Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Modbus</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Modbus" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Modbus.html"><code>Pamoja.Modbus</code></a></li><li><a href="/docs/guides/modbus.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Modbus">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-modbus" title="pamoja-modbus">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/modbus" title="@pamoja/modbus">TypeScript</a> <a href="https://pypi.org/project/pamoja-modbus/" title="pamoja-modbus">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/can.html#c">CAN and J1939</a><p>CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Can</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Can" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Can.html"><code>Pamoja.Can</code></a></li><li><a href="/docs/guides/can.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Can">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-can" title="pamoja-can">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/can" title="@pamoja/can">TypeScript</a> <a href="https://pypi.org/project/pamoja-can/" title="pamoja-can">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/gpio.html#c">I2C, SPI, and GPIO</a><p>I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Gpio</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Gpio" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Gpio.html"><code>Pamoja.Gpio</code></a></li><li><a href="/docs/guides/gpio.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Gpio">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-gpio" title="pamoja-gpio">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/gpio" title="@pamoja/gpio">TypeScript</a> <a href="https://pypi.org/project/pamoja-gpio/" title="pamoja-gpio">Python</a></p>
</div>
</div>

### Sensing and actuation

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/sensors.html#c">Sensor drivers</a><p>Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Sensors</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Sensors" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Sensors.html"><code>Pamoja.Sensors</code></a></li><li><a href="/docs/guides/sensors.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Sensors">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-sensors" title="pamoja-sensors">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/sensors" title="@pamoja/sensors">TypeScript</a> <a href="https://pypi.org/project/pamoja-sensors/" title="pamoja-sensors">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/actuators.html#c">Actuator drivers</a><p>PCA9685 PWM and servo pulses, and stepper coil sequencing</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Actuators</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Actuators" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Actuators.html"><code>Pamoja.Actuators</code></a></li><li><a href="/docs/guides/actuators.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Actuators">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-actuators" title="pamoja-actuators">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/actuators" title="@pamoja/actuators">TypeScript</a> <a href="https://pypi.org/project/pamoja-actuators/" title="pamoja-actuators">Python</a></p>
</div>
</div>

### Radio and reach

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/lora.html#c">LoRa airtime</a><p>Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Lora</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Lora" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Lora.html"><code>Pamoja.Lora</code></a></li><li><a href="/docs/guides/lora.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Lora">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-lora" title="pamoja-lora">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/lora" title="@pamoja/lora">TypeScript</a> <a href="https://pypi.org/project/pamoja-lora/" title="pamoja-lora">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/lorawan.html#c">LoRaWAN</a><p>LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Lorawan</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Lorawan" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Lorawan.html"><code>Pamoja.Lorawan</code></a></li><li><a href="/docs/guides/lorawan.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Lorawan">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-lorawan" title="pamoja-lorawan">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/lorawan" title="@pamoja/lorawan">TypeScript</a> <a href="https://pypi.org/project/pamoja-lorawan/" title="pamoja-lorawan">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/mesh.html#c">Mesh frames</a><p>Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Mesh</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Mesh" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Mesh.html"><code>Pamoja.Mesh</code></a></li><li><a href="/docs/guides/mesh.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Mesh">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-mesh" title="pamoja-mesh">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/mesh" title="@pamoja/mesh">TypeScript</a> <a href="https://pypi.org/project/pamoja-mesh/" title="pamoja-mesh">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/routing.html#c">Routing</a><p>Reverse-path routing that learns the cheapest route from overheard traffic</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Routing</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Routing" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Routing.html"><code>Pamoja.Routing</code></a></li><li><a href="/docs/guides/routing.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Routing">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-routing" title="pamoja-routing">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/routing" title="@pamoja/routing">TypeScript</a> <a href="https://pypi.org/project/pamoja-routing/" title="pamoja-routing">Python</a></p>
</div>
</div>

### MAVLink

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/mavlink.html#c">MAVLink</a><p>MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Mavlink</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Mavlink" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Mavlink.html"><code>Pamoja.Mavlink</code></a></li><li><a href="/docs/guides/mavlink.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Mavlink">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-mavlink" title="pamoja-mavlink">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/mavlink" title="@pamoja/mavlink">TypeScript</a> <a href="https://pypi.org/project/pamoja-mavlink/" title="pamoja-mavlink">Python</a></p>
</div>
</div>

### Trust and operation

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/audit.html#c">Audit log</a><p>A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Audit</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Audit" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Audit.html"><code>Pamoja.Audit</code></a></li><li><a href="/docs/guides/audit.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Audit">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-audit" title="pamoja-audit">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/audit" title="@pamoja/audit">TypeScript</a> <a href="https://pypi.org/project/pamoja-audit/" title="pamoja-audit">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/session.html#c">Secured session</a><p>X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Session</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Session" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Session.html"><code>Pamoja.Session</code></a></li><li><a href="/docs/guides/session.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Session">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-session" title="pamoja-session">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/session" title="@pamoja/session">TypeScript</a> <a href="https://pypi.org/project/pamoja-session/" title="pamoja-session">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/update.html#c">Signed updates</a><p>Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Update</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Update" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Update.html"><code>Pamoja.Update</code></a></li><li><a href="/docs/guides/update.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Update">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-update" title="pamoja-update">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/update" title="@pamoja/update">TypeScript</a> <a href="https://pypi.org/project/pamoja-update/" title="pamoja-update">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/power.html#c">Power</a><p>Duty cycling and an energy-aware governor that stretches work as the battery drains</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Power</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Power" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Power.html"><code>Pamoja.Power</code></a></li><li><a href="/docs/guides/power.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Power">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-power" title="pamoja-power">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/power" title="@pamoja/power">TypeScript</a> <a href="https://pypi.org/project/pamoja-power/" title="pamoja-power">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/telemetry.html#c">Telemetry</a><p>Observability that ships only what is worth the bytes as link cost rises, while counting everything</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Telemetry</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Telemetry" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Telemetry.html"><code>Pamoja.Telemetry</code></a></li><li><a href="/docs/guides/telemetry.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Telemetry">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-telemetry" title="pamoja-telemetry">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/telemetry" title="@pamoja/telemetry">TypeScript</a> <a href="https://pypi.org/project/pamoja-telemetry/" title="pamoja-telemetry">Python</a></p>
</div>
</div>

### Transports and testing

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/mqtt.html#c">MQTT</a><p>An MQTT client with the topic and wildcard rules, as the core transport</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Mqtt</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Mqtt" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Mqtt.html"><code>Pamoja.Mqtt</code></a></li><li><a href="/docs/guides/mqtt.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Mqtt">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-mqtt" title="pamoja-mqtt">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/mqtt" title="@pamoja/mqtt">TypeScript</a> <a href="https://pypi.org/project/pamoja-mqtt/" title="pamoja-mqtt">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/coap.html#c">CoAP</a><p>A CoAP client over UDP with confirmable delivery and observe</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Coap</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Coap" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Coap.html"><code>Pamoja.Coap</code></a></li><li><a href="/docs/guides/coap.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Coap">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-coap" title="pamoja-coap">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/coap" title="@pamoja/coap">TypeScript</a> <a href="https://pypi.org/project/pamoja-coap/" title="pamoja-coap">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/loopback.html#c">Loopback</a><p>An in-process transport with topic matching and a fault injector, for testing with no broker</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Loopback</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Loopback" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Loopback.html"><code>Pamoja.Loopback</code></a></li><li><a href="/docs/guides/loopback.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Loopback">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-loopback" title="pamoja-loopback">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/loopback" title="@pamoja/loopback">TypeScript</a> <a href="https://pypi.org/project/pamoja-loopback/" title="pamoja-loopback">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/sync.html#c">Store and forward</a><p>Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Sync</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Sync" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Sync.html"><code>Pamoja.Sync</code></a></li><li><a href="/docs/guides/sync.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Sync">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-sync" title="pamoja-sync">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/sync" title="@pamoja/sync">TypeScript</a> <a href="https://pypi.org/project/pamoja-sync/" title="pamoja-sync">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/ladder.html#c">Transport ladder</a><p>Cheapest reachable link first, buffering to a store when every link is down</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Ladder</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Ladder" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Ladder.html"><code>Pamoja.Ladder</code></a></li><li><a href="/docs/guides/ladder.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Ladder">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-ladder" title="pamoja-ladder">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/ladder" title="@pamoja/ladder">TypeScript</a> <a href="https://pypi.org/project/pamoja-ladder/" title="pamoja-ladder">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/bus.html#c">Event bus</a><p>An in-memory typed publish and subscribe event bus</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Bus</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Bus" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Bus.html"><code>Pamoja.Bus</code></a></li><li><a href="/docs/guides/bus.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Bus">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-bus" title="pamoja-bus">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/bus" title="@pamoja/bus">TypeScript</a> <a href="https://pypi.org/project/pamoja-bus/" title="pamoja-bus">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/sim.html#c">Simulators</a><p>Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Sim</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Sim" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Sim.html"><code>Pamoja.Sim</code></a></li><li><a href="/docs/guides/sim.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Sim">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-sim" title="pamoja-sim">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/sim" title="@pamoja/sim">TypeScript</a> <a href="https://pypi.org/project/pamoja-sim/" title="pamoja-sim">Python</a></p>
</div>
</div>

### Profiles and robotics

<div class="pkgs">
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/profile.html#c">Device profiles</a><p>Named, ready-to-run device profiles from plain data or a JSON manifest</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Profile</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Profile" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Profile.html"><code>Pamoja.Profile</code></a></li><li><a href="/docs/guides/profile.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Profile">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-profile" title="pamoja-profile">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/profile" title="@pamoja/profile">TypeScript</a> <a href="https://pypi.org/project/pamoja-profile/" title="pamoja-profile">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/ros2.html#c">ROS 2 rules</a><p>ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Ros2</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Ros2" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Ros2.html"><code>Pamoja.Ros2</code></a></li><li><a href="/docs/guides/ros2.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Ros2">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-ros2" title="pamoja-ros2">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/ros2" title="@pamoja/ros2">TypeScript</a> <a href="https://pypi.org/project/pamoja-ros2/" title="pamoja-ros2">Python</a></p>
</div>
<div class="pkg">
<div class="pkg-what"><a href="/docs/guides/zenoh.html#c">Zenoh keys</a><p>Zenoh key expressions: validity, canonical form, and wildcard matching</p></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Zenoh</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Zenoh" aria-label="Copy the install command">copy</button></div>
<ul class="pkg-links"><li><a href="/docs/reference/dotnet/api/Pamoja.Zenoh.html"><code>Pamoja.Zenoh</code></a></li><li><a href="/docs/guides/zenoh.html#c">worked example</a></li><li><a href="https://www.nuget.org/packages/Pamoja.Zenoh">NuGet</a></li></ul>
<p class="pkg-else"><span>Also in</span> <a href="https://crates.io/crates/pamoja-zenoh" title="pamoja-zenoh">Rust</a> <a href="https://www.npmjs.com/package/@pamoja/zenoh" title="@pamoja/zenoh">TypeScript</a> <a href="https://pypi.org/project/pamoja-zenoh/" title="pamoja-zenoh">Python</a></p>
</div>
</div>
<!-- end -->
