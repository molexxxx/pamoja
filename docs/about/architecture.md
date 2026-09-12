# Architecture

Every domain capability is a separate crate behind a trait defined in the core.
The core knows about `Transport`, `Device`, `Sensor`, `Actuator`, `Store`, and
the event bus; it knows nothing about MQTT or CAN specifically. Concrete crates
implement those traits and are pulled in only when needed, so nobody pays for
what they do not use, and on a microcontroller you compile in two crates and
nothing else. The [install page](../install.md) measures that claim per feature
set, down to a single-capability build that carries no third-party code at all.

<!-- table: architecture -->
<div class="bd" role="img" aria-label="How a call reaches a crate: three bindings over one compiled engine, a Rust program straight to the crates, every capability by chapter, and every crate over pamoja-core.">
<div class="bd-doors"><div class="bd-door"><b>TypeScript</b><code>@pamoja/&lt;name&gt;</code><span>over napi-rs</span></div><div class="bd-door"><b>Python</b><code>pamoja-&lt;name&gt;</code><span>over PyO3</span></div><div class="bd-door"><b>C#</b><code>Pamoja.&lt;Name&gt;</code><span>over cbindgen and P/Invoke</span></div></div>
<p class="bd-flow" aria-hidden="true"><span></span></p>
<div class="bd-engine">
<div><b>Compiled engine</b><p>pamoja-ffi over the C ABI: one library carrying every capability</p></div>
<div class="bd-engine-end"><code>@pamoja/native, pamoja-native, Pamoja.Native</code><span>A package narrows the API, not the download.</span></div>
</div>
<p class="bd-flow" aria-hidden="true"><span></span></p>
<div class="bd-caps">
<p class="bd-caps-head"><b>Capabilities by chapter</b><code>Rust: cargo add pamoja-&lt;name&gt;, the crates themselves</code></p>
<div class="bd-grid">
<div class="bd-cell">
<p class="bd-cell-title">Identity</p>
<ul class="bd-crates"><li class="on-core">security</li></ul>
<ul class="bd-names"><li>@pamoja/security</li><li>pamoja-security</li><li>Pamoja.Security</li><li>pamoja -F security</li></ul>
</div>
<div class="bd-cell">
<p class="bd-cell-title">Codecs</p>
<ul class="bd-crates"><li class="on-core">codec</li></ul>
<ul class="bd-names"><li>@pamoja/codec</li><li>pamoja-codec</li><li>Pamoja.Codec</li><li>pamoja -F codec</li></ul>
</div>
<div class="bd-cell">
<p class="bd-cell-title">Helpers</p>
<ul class="bd-crates"><li>kit</li></ul>
<ul class="bd-names"><li>@pamoja/kit</li><li>pamoja-kit</li><li>Pamoja.Kit</li><li>pamoja -F kit</li></ul>
</div>
<div class="bd-cell">
<p class="bd-cell-title">Field I/O</p>
<ul class="bd-crates"><li>serial</li><li>modbus</li><li>can</li><li class="on-core">gpio</li><li>hal</li></ul>
<ul class="bd-names"><li>@pamoja/field-io</li><li>pamoja-field-io</li><li>Pamoja.FieldIo</li><li>pamoja -F field-io</li></ul>
</div>
<div class="bd-cell">
<p class="bd-cell-title">Sensing and actuation</p>
<ul class="bd-crates"><li class="on-core">sensors</li><li class="on-core">actuators</li></ul>
<ul class="bd-names"><li>@pamoja/sensing</li><li>pamoja-sensing</li><li>Pamoja.Sensing</li><li>pamoja -F sensing</li></ul>
</div>
<div class="bd-cell">
<p class="bd-cell-title">Radio and reach</p>
<ul class="bd-crates"><li>lora</li><li>lorawan</li><li class="on-core">radios</li><li class="on-core">gateway</li><li>mesh</li><li>routing</li></ul>
<ul class="bd-names"><li>@pamoja/radio</li><li>pamoja-radio</li><li>Pamoja.Radio</li><li>pamoja -F radio</li></ul>
</div>
<div class="bd-cell">
<p class="bd-cell-title">MAVLink</p>
<ul class="bd-crates"><li class="on-core">mavlink</li></ul>
<ul class="bd-names"><li>@pamoja/mavlink</li><li>pamoja-mavlink</li><li>Pamoja.Mavlink</li><li>pamoja -F mavlink</li></ul>
</div>
<div class="bd-cell">
<p class="bd-cell-title">Trust and operation</p>
<ul class="bd-crates"><li class="on-core">audit</li><li>session</li><li class="on-core">update</li><li>power</li><li>telemetry</li></ul>
<ul class="bd-names"><li>@pamoja/trust</li><li>pamoja-trust</li><li>Pamoja.Trust</li><li>pamoja -F trust</li></ul>
</div>
<div class="bd-cell">
<p class="bd-cell-title">Transports and testing</p>
<ul class="bd-crates"><li class="on-core">mqtt</li><li class="on-core">coap</li><li class="on-core">loopback</li><li class="on-core">sync</li><li class="on-core">ladder</li><li class="on-core">bus</li><li class="on-core">sim</li></ul>
<ul class="bd-names"><li>@pamoja/transports</li><li>pamoja-transports</li><li>Pamoja.Transports</li><li>pamoja -F transports</li></ul>
</div>
<div class="bd-cell">
<p class="bd-cell-title">Profiles and robotics</p>
<ul class="bd-crates"><li class="on-core">profile</li><li class="on-core">ros2</li><li class="on-core">zenoh</li></ul>
<ul class="bd-names"><li>@pamoja/profiles</li><li>pamoja-profiles</li><li>Pamoja.Profiles</li><li>pamoja -F profiles</li></ul>
</div>
</div>
<div class="bd-core">
<b>pamoja-core</b>
<p>Transport, Device, Sensor, Actuator, Store, and the event bus; <code>no_std</code>, so it runs on a microcontroller</p>
<p class="bd-key">The names in the vendor ink build on it; the rest are pure logic with no dependency.</p>
</div>
</div>
<p class="bd-foot">A chapter's package brings its capabilities with it. Everything at once: <code>npm install pamoja</code>, <code>pip install pamoja</code>, <code>dotnet add package Pamoja</code>, or <code>cargo add pamoja</code>.</p>
</div>
<!-- end -->

This separation is literal in Rust: `pamoja-core` defines the traits, and each
transport (`pamoja-mqtt`, `pamoja-coap`) is its own crate, so Rust code pulls
`MqttTransport` from `pamoja-mqtt`, not from the core. Every registry offers the
same three grain sizes, and the guides' chapters are the domains:

| What you want | Rust | npm | PyPI | NuGet |
| --- | --- | --- | --- | --- |
| Everything | `pamoja` | `pamoja` | `pamoja` | `Pamoja` |
| A domain, six of them | `pamoja --features radio` | `@pamoja/radio` | `pamoja-radio` | `Pamoja.Radio` |
| One capability, thirty | `pamoja-lora` | `@pamoja/lora` | `pamoja-lora` | `Pamoja.Lora` |

Underneath the three bindings, and nowhere in Rust, is the compiled engine:
`@pamoja/native`, `pamoja-native`, and `Pamoja.Native`. It is the built library,
the generated contract over it, and the plumbing a facade needs to call it, which
is the handle type, the error every failed call raises, and string marshalling.
Every package declares it, so it arrives on its own and nobody installs it by
hand. A Rust build has no equivalent because it compiles the crates.

`pamoja-core` is a different thing with a similar name. It is the engine's own
surface, the runtime version and the `Transport` every link implements, and in
the bindings it is a capability like the others: `@pamoja/mqtt` returns something
that satisfies it, so the transports depend on it and the rest do not.

That per-package shape means different things on the two sides of the C ABI. A
Rust build compiles only the crates it names, which the
[install page](../install.md) measures per feature set. A binding loads one
compiled engine carrying every capability, so choosing packages there narrows the
API and the dependency manifest rather than the download. Compiling away what you
do not use is a property of a compiled language, and the targets that need it run
Rust rather than a managed runtime. A C or C++ host that builds `pamoja-ffi`
itself gets the Rust behavior, because the capabilities are cargo features
there: dropping the seven that need an async runtime halves the library.

A domain package brings in its capabilities and, where the language allows it,
re-exports each under its own name rather than flattening them, because two
capabilities of a domain can export the same name: `pamoja-lorawan` and
`pamoja-mesh` both define a maximum frame size, and flattening those silently
resolves to one of them. Naming the capability is how Rust and C# already read,
so the bindings read that way too.

## Two tiers in every binding

Each binding has a generated contract and a hand-written facade. The contract
is produced from the Rust source (napi-rs for Node, PyO3 with a generated type
stub for Python, cbindgen and P/Invoke for .NET) and is drift-checked in CI, so
it cannot fall behind the core. The facade is written in the language's own
idiom on top of it: `async for` in Python, `IAsyncEnumerable` in C#, a package
per capability in TypeScript. The facade adds ergonomics only; every
operation delegates to the Rust core.

A single file of conformance vectors, generated from the Rust implementation,
is asserted by every binding's test suite, so the four languages cannot quietly
disagree about what the same call returns.

## What stays in Rust

Two things do not cross the bindings on purpose. The live ROS 2 and Zenoh
bridges need a ROS 2 or Zenoh installation, so only their naming and encoding
rules cross; the bridges themselves stay in Rust. And the MAVLink vehicle
model, which drives a real autopilot over serial, UDP, or TCP, stays in Rust
too; the framing, the message shapes, and the mission, command, and offboard
protocols cross, so a ground station in any language can run the exchange
over its own link.
