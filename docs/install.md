# Install

`pamoja` is the whole framework in one package, in every language:

```sh
cargo add pamoja                 # Rust
npm install pamoja               # TypeScript and Node
pip install pamoja               # Python
dotnet add package Pamoja        # C# and .NET
```

That is the right default. Every capability is also a package of its own, and
what you gain by picking them differs between Rust and the bindings, so it is
worth knowing which before you choose. This page is about that choice; the
[reference](https://pamoja.molex.cloud/docs/reference/index.html) lists every
package in every language, and each domain has a page of its own.

Every crate, package, and binding shares one version and is released together,
so any package of a release wraps the same release of every other. The
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
capability like any other, listed first on every reference page, and most packages do
not depend on it: only the transports do, because they are the ones that return a
`Transport`. Install it when you want to hold a link behind that interface.

## By domain

Six of the ten headings hold more than one capability, and each of those is
also one thing to install: a feature in Rust, and a package in every binding
that brings in its capabilities and re-exports each under its own name, so a
name two of them share stays unambiguous. Pick a language:

<!-- table: install all -->
<div class="langs">
<div class="lang-tabs" role="tablist" aria-label="Language">
<button class="lang-tab" role="tab" type="button" id="domains-tab-rust" aria-controls="domains-rust" aria-selected="false" data-lang="rust">Rust</button>
<button class="lang-tab" role="tab" type="button" id="domains-tab-typescript" aria-controls="domains-typescript" aria-selected="false" data-lang="typescript">TypeScript</button>
<button class="lang-tab" role="tab" type="button" id="domains-tab-python" aria-controls="domains-python" aria-selected="false" data-lang="python">Python</button>
<button class="lang-tab" role="tab" type="button" id="domains-tab-c" aria-controls="domains-c" aria-selected="false" data-lang="c">C#</button>
</div>
<section class="lang-panel" id="domains-rust" role="tabpanel" aria-labelledby="domains-tab-rust" data-lang="rust" tabindex="0">
<div class="domains">
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/rust.html#field-io">Field I/O</a></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features field-io</code><button class="copy" type="button" data-copy="cargo add pamoja --features field-io" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/serial.html">Serial framing</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/modbus.html">Modbus RTU</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/can.html">CAN and J1939</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/gpio.html">I2C, SPI, and GPIO</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/hal.html">Buses</a></li></ul>
</details><a class="pkg-btn api rust" href="https://pamoja.molex.cloud/docs/reference/rust.html#field-io">API reference</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/rust.html#sensing-and-actuation">Sensing and actuation</a></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features sensing</code><button class="copy" type="button" data-copy="cargo add pamoja --features sensing" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">3</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/sensors.html">Sensor drivers</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/actuators.html">Actuator drivers</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/device.html">Your own device</a></li></ul>
</details><a class="pkg-btn api rust" href="https://pamoja.molex.cloud/docs/reference/rust.html#sensing-and-actuation">API reference</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/rust.html#radio-and-reach">Radio and reach</a></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features radio</code><button class="copy" type="button" data-copy="cargo add pamoja --features radio" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/lora.html">LoRa airtime and range</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/lorawan.html">LoRaWAN</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/radios.html">LoRa radios</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/mesh.html">Mesh frames</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/routing.html">Routing</a></li></ul>
</details><a class="pkg-btn api rust" href="https://pamoja.molex.cloud/docs/reference/rust.html#radio-and-reach">API reference</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/rust.html#trust-and-operation">Trust and operation</a></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features trust</code><button class="copy" type="button" data-copy="cargo add pamoja --features trust" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/audit.html">Audit log</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/session.html">Secured session</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/update.html">Signed updates</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/power.html">Power</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/telemetry.html">Telemetry</a></li></ul>
</details><a class="pkg-btn api rust" href="https://pamoja.molex.cloud/docs/reference/rust.html#trust-and-operation">API reference</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/rust.html#transports-and-testing">Transports and testing</a></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features transports</code><button class="copy" type="button" data-copy="cargo add pamoja --features transports" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">9</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/mqtt.html">MQTT</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/coap.html">CoAP</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/loopback.html">Loopback</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/sync.html">Store and forward</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/ladder.html">Transport ladder</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/bus.html">Event bus</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/transport.html">Engine surface</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/link.html">Your own link</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/sim.html">Simulators</a></li></ul>
</details><a class="pkg-btn api rust" href="https://pamoja.molex.cloud/docs/reference/rust.html#transports-and-testing">API reference</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/rust.html#profiles-and-robotics">Profiles and robotics</a></div>
<div class="pkg-get"><code class="cmd">cargo add pamoja --features profiles</code><button class="copy" type="button" data-copy="cargo add pamoja --features profiles" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">4</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/profile.html">Device profiles</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/rules.html">Rules</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/ros2.html">ROS 2 rules</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/zenoh.html">Zenoh keys</a></li></ul>
</details><a class="pkg-btn api rust" href="https://pamoja.molex.cloud/docs/reference/rust.html#profiles-and-robotics">API reference</a></div></div>
</div>
</div>
</section>
<section class="lang-panel" id="domains-typescript" role="tabpanel" aria-labelledby="domains-tab-typescript" data-lang="typescript" tabindex="0">
<div class="domains">
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/node.html#field-io">Field I/O</a><code class="pkg-import">@pamoja/field-io</code></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/field-io</code><button class="copy" type="button" data-copy="npm install @pamoja/field-io" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/serial.html">Serial framing</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/modbus.html">Modbus RTU</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/can.html">CAN and J1939</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/gpio.html">I2C, SPI, and GPIO</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/hal.html">Buses</a></li></ul>
</details><a class="pkg-btn api node" href="https://pamoja.molex.cloud/docs/reference/node.html#field-io">API reference</a><a class="pkg-btn ext" href="https://www.npmjs.com/package/@pamoja/field-io">npm</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/node.html#sensing-and-actuation">Sensing and actuation</a><code class="pkg-import">@pamoja/sensing</code></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/sensing</code><button class="copy" type="button" data-copy="npm install @pamoja/sensing" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">3</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/sensors.html">Sensor drivers</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/actuators.html">Actuator drivers</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/device.html">Your own device</a></li></ul>
</details><a class="pkg-btn api node" href="https://pamoja.molex.cloud/docs/reference/node.html#sensing-and-actuation">API reference</a><a class="pkg-btn ext" href="https://www.npmjs.com/package/@pamoja/sensing">npm</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/node.html#radio-and-reach">Radio and reach</a><code class="pkg-import">@pamoja/radio</code></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/radio</code><button class="copy" type="button" data-copy="npm install @pamoja/radio" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/lora.html">LoRa airtime and range</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/lorawan.html">LoRaWAN</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/radios.html">LoRa radios</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/mesh.html">Mesh frames</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/routing.html">Routing</a></li></ul>
</details><a class="pkg-btn api node" href="https://pamoja.molex.cloud/docs/reference/node.html#radio-and-reach">API reference</a><a class="pkg-btn ext" href="https://www.npmjs.com/package/@pamoja/radio">npm</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/node.html#trust-and-operation">Trust and operation</a><code class="pkg-import">@pamoja/trust</code></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/trust</code><button class="copy" type="button" data-copy="npm install @pamoja/trust" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/audit.html">Audit log</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/session.html">Secured session</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/update.html">Signed updates</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/power.html">Power</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/telemetry.html">Telemetry</a></li></ul>
</details><a class="pkg-btn api node" href="https://pamoja.molex.cloud/docs/reference/node.html#trust-and-operation">API reference</a><a class="pkg-btn ext" href="https://www.npmjs.com/package/@pamoja/trust">npm</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/node.html#transports-and-testing">Transports and testing</a><code class="pkg-import">@pamoja/transports</code></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/transports</code><button class="copy" type="button" data-copy="npm install @pamoja/transports" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">9</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/mqtt.html">MQTT</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/coap.html">CoAP</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/loopback.html">Loopback</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/sync.html">Store and forward</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/ladder.html">Transport ladder</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/bus.html">Event bus</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/transport.html">Engine surface</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/link.html">Your own link</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/sim.html">Simulators</a></li></ul>
</details><a class="pkg-btn api node" href="https://pamoja.molex.cloud/docs/reference/node.html#transports-and-testing">API reference</a><a class="pkg-btn ext" href="https://www.npmjs.com/package/@pamoja/transports">npm</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/node.html#profiles-and-robotics">Profiles and robotics</a><code class="pkg-import">@pamoja/profiles</code></div>
<div class="pkg-get"><code class="cmd">npm install @pamoja/profiles</code><button class="copy" type="button" data-copy="npm install @pamoja/profiles" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">4</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/profile.html">Device profiles</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/rules.html">Rules</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/ros2.html">ROS 2 rules</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/zenoh.html">Zenoh keys</a></li></ul>
</details><a class="pkg-btn api node" href="https://pamoja.molex.cloud/docs/reference/node.html#profiles-and-robotics">API reference</a><a class="pkg-btn ext" href="https://www.npmjs.com/package/@pamoja/profiles">npm</a></div></div>
</div>
</div>
</section>
<section class="lang-panel" id="domains-python" role="tabpanel" aria-labelledby="domains-tab-python" data-lang="python" tabindex="0">
<div class="domains">
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/python.html#field-io">Field I/O</a><code class="pkg-import">pamoja.field_io</code></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-field-io</code><button class="copy" type="button" data-copy="pip install pamoja-field-io" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/serial.html">Serial framing</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/modbus.html">Modbus RTU</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/can.html">CAN and J1939</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/gpio.html">I2C, SPI, and GPIO</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/hal.html">Buses</a></li></ul>
</details><a class="pkg-btn api python" href="https://pamoja.molex.cloud/docs/reference/python.html#field-io">API reference</a><a class="pkg-btn ext" href="https://pypi.org/project/pamoja-field-io/">PyPI</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/python.html#sensing-and-actuation">Sensing and actuation</a><code class="pkg-import">pamoja.sensing</code></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-sensing</code><button class="copy" type="button" data-copy="pip install pamoja-sensing" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">3</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/sensors.html">Sensor drivers</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/actuators.html">Actuator drivers</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/device.html">Your own device</a></li></ul>
</details><a class="pkg-btn api python" href="https://pamoja.molex.cloud/docs/reference/python.html#sensing-and-actuation">API reference</a><a class="pkg-btn ext" href="https://pypi.org/project/pamoja-sensing/">PyPI</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/python.html#radio-and-reach">Radio and reach</a><code class="pkg-import">pamoja.radio</code></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-radio</code><button class="copy" type="button" data-copy="pip install pamoja-radio" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/lora.html">LoRa airtime and range</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/lorawan.html">LoRaWAN</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/radios.html">LoRa radios</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/mesh.html">Mesh frames</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/routing.html">Routing</a></li></ul>
</details><a class="pkg-btn api python" href="https://pamoja.molex.cloud/docs/reference/python.html#radio-and-reach">API reference</a><a class="pkg-btn ext" href="https://pypi.org/project/pamoja-radio/">PyPI</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/python.html#trust-and-operation">Trust and operation</a><code class="pkg-import">pamoja.trust</code></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-trust</code><button class="copy" type="button" data-copy="pip install pamoja-trust" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/audit.html">Audit log</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/session.html">Secured session</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/update.html">Signed updates</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/power.html">Power</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/telemetry.html">Telemetry</a></li></ul>
</details><a class="pkg-btn api python" href="https://pamoja.molex.cloud/docs/reference/python.html#trust-and-operation">API reference</a><a class="pkg-btn ext" href="https://pypi.org/project/pamoja-trust/">PyPI</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/python.html#transports-and-testing">Transports and testing</a><code class="pkg-import">pamoja.transports</code></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-transports</code><button class="copy" type="button" data-copy="pip install pamoja-transports" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">9</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/mqtt.html">MQTT</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/coap.html">CoAP</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/loopback.html">Loopback</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/sync.html">Store and forward</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/ladder.html">Transport ladder</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/bus.html">Event bus</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/transport.html">Engine surface</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/link.html">Your own link</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/sim.html">Simulators</a></li></ul>
</details><a class="pkg-btn api python" href="https://pamoja.molex.cloud/docs/reference/python.html#transports-and-testing">API reference</a><a class="pkg-btn ext" href="https://pypi.org/project/pamoja-transports/">PyPI</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/python.html#profiles-and-robotics">Profiles and robotics</a><code class="pkg-import">pamoja.profiles</code></div>
<div class="pkg-get"><code class="cmd">pip install pamoja-profiles</code><button class="copy" type="button" data-copy="pip install pamoja-profiles" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">4</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/profile.html">Device profiles</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/rules.html">Rules</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/ros2.html">ROS 2 rules</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/zenoh.html">Zenoh keys</a></li></ul>
</details><a class="pkg-btn api python" href="https://pamoja.molex.cloud/docs/reference/python.html#profiles-and-robotics">API reference</a><a class="pkg-btn ext" href="https://pypi.org/project/pamoja-profiles/">PyPI</a></div></div>
</div>
</div>
</section>
<section class="lang-panel" id="domains-c" role="tabpanel" aria-labelledby="domains-tab-c" data-lang="c" tabindex="0">
<div class="domains">
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#field-io">Field I/O</a></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.FieldIo</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.FieldIo" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/serial.html">Serial framing</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/modbus.html">Modbus RTU</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/can.html">CAN and J1939</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/gpio.html">I2C, SPI, and GPIO</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/hal.html">Buses</a></li></ul>
</details><a class="pkg-btn api dotnet" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#field-io">API reference</a><a class="pkg-btn ext" href="https://www.nuget.org/packages/Pamoja.FieldIo">NuGet</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#sensing-and-actuation">Sensing and actuation</a></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Sensing</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Sensing" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">3</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/sensors.html">Sensor drivers</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/actuators.html">Actuator drivers</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/device.html">Your own device</a></li></ul>
</details><a class="pkg-btn api dotnet" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#sensing-and-actuation">API reference</a><a class="pkg-btn ext" href="https://www.nuget.org/packages/Pamoja.Sensing">NuGet</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#radio-and-reach">Radio and reach</a></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Radio</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Radio" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/lora.html">LoRa airtime and range</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/lorawan.html">LoRaWAN</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/radios.html">LoRa radios</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/mesh.html">Mesh frames</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/routing.html">Routing</a></li></ul>
</details><a class="pkg-btn api dotnet" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#radio-and-reach">API reference</a><a class="pkg-btn ext" href="https://www.nuget.org/packages/Pamoja.Radio">NuGet</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#trust-and-operation">Trust and operation</a></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Trust</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Trust" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">5</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/audit.html">Audit log</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/session.html">Secured session</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/update.html">Signed updates</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/power.html">Power</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/telemetry.html">Telemetry</a></li></ul>
</details><a class="pkg-btn api dotnet" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#trust-and-operation">API reference</a><a class="pkg-btn ext" href="https://www.nuget.org/packages/Pamoja.Trust">NuGet</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#transports-and-testing">Transports and testing</a></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Transports</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Transports" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">9</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/mqtt.html">MQTT</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/coap.html">CoAP</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/loopback.html">Loopback</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/sync.html">Store and forward</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/ladder.html">Transport ladder</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/bus.html">Event bus</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/transport.html">Engine surface</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/link.html">Your own link</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/sim.html">Simulators</a></li></ul>
</details><a class="pkg-btn api dotnet" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#transports-and-testing">API reference</a><a class="pkg-btn ext" href="https://www.nuget.org/packages/Pamoja.Transports">NuGet</a></div></div>
</div>
<div class="domain">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#profiles-and-robotics">Profiles and robotics</a></div>
<div class="pkg-get"><code class="cmd">dotnet add package Pamoja.Profiles</code><button class="copy" type="button" data-copy="dotnet add package Pamoja.Profiles" aria-label="Copy the install command">copy</button></div>
</div>
<div class="pkg-foot"><div class="pkg-btns"><details class="guide-menu">
<summary><span class="guide-menu-n">4</span> guides<span class="guide-menu-caret" aria-hidden="true"></span></summary>
<ul class="guide-menu-list"><li><a href="https://pamoja.molex.cloud/docs/guides/profile.html">Device profiles</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/rules.html">Rules</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/ros2.html">ROS 2 rules</a></li><li><a href="https://pamoja.molex.cloud/docs/guides/zenoh.html">Zenoh keys</a></li></ul>
</details><a class="pkg-btn api dotnet" href="https://pamoja.molex.cloud/docs/reference/dotnet.html#profiles-and-robotics">API reference</a><a class="pkg-btn ext" href="https://www.nuget.org/packages/Pamoja.Profiles">NuGet</a></div></div>
</div>
</div>
</section>
</div>
<!-- end -->

Each row opens the section of that language's reference page that lists what the
domain brings in, with the install line and the API pages for every capability
in it. Every capability, on its own, is on the
[reference](https://pamoja.molex.cloud/docs/reference/index.html).

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
| Every capability | `cargo add pamoja` | 110 | 33 | 77 |
| Codecs and identity | `cargo add pamoja --no-default-features --features codec,security` | 36 | 4 | 32 |
| Field I/O | `cargo add pamoja --no-default-features --features field-io` | 8 | 7 | 1 |
| One capability | `cargo add pamoja --no-default-features --features modbus` | 3 | 3 | 0 |
| Bare metal, no `std` | `cargo add pamoja --no-default-features --features modbus,sensors,lora` | 7 | 6 | 1 |
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

