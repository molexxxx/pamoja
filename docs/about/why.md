# Why it exists

pamoja is free software for building things that watch and control the
physical world: a fridge that warns before its contents spoil, a pump that runs
when a tank gets low, a sensor that keeps working when the internet does not.
It is built to run on cheap, solar-powered hardware and the ordinary phones
people already have, in places with little money and weak or no connectivity.
It costs nothing and works offline.

For developers, that means a single modular SDK for IoT, robotics, and drones:
one memory-safe Rust engine at the core, with idiomatic bindings for
TypeScript, Python, C#, and Rust itself. Control and communicate with physical
things from the language you already work in, with native performance and
memory safety, without hand-rolling FFI. You install only the capabilities you
need, and the same concepts work the same way everywhere.

## The hard environment first

The places where connected devices can do the most good, smallholder farms,
off-grid villages, rural clinics, disaster zones, are exactly the places with
the least money, the worst connectivity, and the cheapest hardware. Most IoT
and robotics stacks quietly assume the opposite of all of that.

pamoja is built for the hard environment first. If it runs well on a
two-dollar microcontroller on a solar panel with an intermittent radio link,
it runs well anywhere. That single constraint makes the library better for
everyone. In practice:

- Cheap and salvageable hardware, down to microcontrollers with a few hundred
  KB of RAM.
- Offline first: local buffering and store-and-forward, so a device
  disconnected for days loses nothing.
- Low bandwidth and long range: compact codecs and radio (LoRa, mesh) treated
  as first-class.
- Low power: async duty cycling and energy-aware scheduling for battery and
  solar.
- Free and unencumbered, so cost is never a barrier to use.
- Reachable: many languages, plus a plain-language helper layer, so you do not
  need to be an engineer to build something that works.

## The pillars

- Performant: native Rust, async first, small enough to run `no_std` on
  microcontrollers.
- Secure: memory safety by construction, device identity, a secured channel,
  signed updates, and a tamper-evident log.
- One consistent API in every language, with a high-level facade plus a
  low-level escape hatch.
- Easy to adopt: opt-in scoped packages, strong defaults, and simulators, so
  everything here can be built and tested with nothing plugged in.

## Where things stand

Released and installable, not a prototype:

- 32 crates on crates.io, with the Node, Python, and .NET bindings on npm,
  PyPI, and NuGet, all versioned in lockstep.
- Tests pinned, wherever a standard exists, to that standard's own published
  vectors rather than to round-trips, so an implementation that is wrong but
  self-consistent still fails.
- Checked against the real thing in CI: MAVLink against live ArduPilot and PX4
  SITL, the ROS 2 bridge against ROS 2 Jazzy with rmw_zenoh, and every
  `no_std` crate cross-compiled for a Cortex-M4F microcontroller.
- Audited on every change: rustfmt, clippy at `-D warnings`, CodeQL over five
  languages, and a license and security-advisory sweep of the dependency graph
  a consumer actually installs.

Generated surfaces (the language binding contracts, the C header, the crate
READMEs, and every example in these guides) are drift-checked against the
source, so they cannot quietly fall behind it.
