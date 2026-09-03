# Architecture

Every domain capability is a separate crate behind a trait defined in the core.
The core knows about `Transport`, `Device`, `Sensor`, `Actuator`, `Store`, and
the event bus; it knows nothing about MQTT or CAN specifically. Concrete crates
implement those traits and are pulled in only when needed, so nobody pays for
what they do not use, and on a microcontroller you compile in two crates and
nothing else.

This separation is literal in Rust: `pamoja-core` defines the traits, and each
transport (`pamoja-mqtt`, `pamoja-coap`) is its own crate, so Rust code pulls
`MqttTransport` from `pamoja-mqtt`, not from the core. The bindings follow the
same shape: on npm each capability is its own package (`@pamoja/mqtt`),
`@pamoja/core` is the engine's surface, and `pamoja` is the whole framework in
one package. The compiled engine itself is `@pamoja/native`, a build artifact
every package depends on and nobody installs by hand. On PyPI the same four
kinds are `pamoja-<name>`, `pamoja-core`, `pamoja`, and `pamoja-native`, merged
into one `pamoja` namespace on import. On NuGet they are `Pamoja.<Name>`,
`Pamoja.Core`, `Pamoja`, and `Pamoja.Native`, each package a namespace of the
same name.

```
   npm  pamoja, @pamoja/<capability>      PyPI  pamoja, pamoja-<capability>
   NuGet  Pamoja, Pamoja.<Capability>     crates.io  pamoja, pamoja-<capability>
        |                |                     |                    |
   @pamoja/native   pamoja-native        Pamoja.Native          (source)
        +----------------+---------------------+--------------------+
                                     |  the compiled engine: every crate below
                            +--------+--------+   device model, event bus,
                            |   pamoja-core   |   error model, transports
                            +--------+--------+
                                     |  trait-based abstraction layer
   messaging   hardware I/O   robotics    drones    trust      resilience   power
   mqtt/coap   serial/can/    ros2/       mavlink   identity/  store-and-   duty-
   lora/mesh   gpio/rs485     zenoh                 session/   forward      cycling
                                                    update
```

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
