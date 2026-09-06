# Architecture

Every domain capability is a separate crate behind a trait defined in the core.
The core knows about `Transport`, `Device`, `Sensor`, `Actuator`, `Store`, and
the event bus; it knows nothing about MQTT or CAN specifically. Concrete crates
implement those traits and are pulled in only when needed, so nobody pays for
what they do not use, and on a microcontroller you compile in two crates and
nothing else. The [install page](../install.md) measures that claim per feature
set, down to a single-capability build that carries no third-party code at all.

[![How a call reaches a crate: the three bindings over the compiled engine, Rust straight to the crates, and every capability crate over pamoja-core](../assets/architecture.svg)](../assets/architecture.svg)

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
itself gets the Rust behaviour, because the capabilities are cargo features
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
