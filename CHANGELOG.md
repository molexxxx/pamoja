# Changelog

Notable changes to pamoja, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Every crate, the npm,
PyPI, and NuGet packages, and the language bindings share one version and are
released together, so one entry covers all of them.

## [Unreleased]

### Added

- Signed firmware updates with verified rollback in `pamoja-update` (#62).
- LoRaWAN regional parameters: the RP002 channel plans, data rates, and
  duty-cycle limits per region (#70), and the plans in every binding (#71).
- Every capability in the Node, Python, and .NET bindings, with a
  cross-language conformance suite pinning the wire bytes: identity, codecs,
  and the helpers (#61); field I/O (#63); sensors and actuators (#64); radio
  (#65); trust and operation (#66); the async transports (#68); profiles and
  the robotics naming rules (#69); MAVLink framing (#72), named message fields
  (#73), and the mission, command, and offboard protocols (#74).
- `cargo xtask release --plan` derives the crates.io publish order from
  `cargo metadata`, and `cargo xtask version` sets and checks the version in
  every manifest, lockfile, and generated file.
- A preflight every release workflow waits on. A publish cannot be withdrawn, so
  before anything reaches a registry it checks that the tree carries the version
  being tagged, that the commit is on main, and that `ci`, `node`, `python`, and
  `dotnet` all completed successfully on that exact commit. Each release
  workflow also takes a version by hand, so a run that stalled can be resumed
  without inventing a tag.
- A GitHub release for each tag, carrying the changelog's entry for the version
  followed by the pull requests that went into it, grouped by label. Labels come
  from the files a pull request touches.
- A documentation site at [pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/):
  the guides rendered by mdBook and a generated reference for each language
  (rustdoc, typedoc, pdoc, DocFX), built on every pull request and published
  with the showcase. `docs/capabilities.toml` is the one map of what each
  capability covers in every language, checked against the code.
- A `pamoja` crate that bundles every capability behind a feature each, all on
  by default, so `cargo add pamoja` is the whole framework the way
  `npm install pamoja`, `pip install pamoja`, and `dotnet add package Pamoja`
  are. `pamoja::mqtt` is `pamoja-mqtt`; with the default features off, naming
  only the `no_std` capabilities builds for bare metal.
- Guide examples that run as tests in all four languages, spliced into the
  documentation from the test files, and the Python facade's doctests now run
  with its test suite. Each is a program somebody would actually
  write, end to end, rather than a set of assertions: it builds its own fixtures
  from the library, prints what it learned, and keeps its checks below the region
  the page shows. A guide's own wire bytes live in the crate's tests or in the
  generated conformance vectors, so no page asks a reader to decode a constant.
- Reading a value the library can produce, wherever it could only produce one:
  a DS18B20 scratchpad and an INA219 register set can be built as well as
  decoded, Modbus can build the replies it could already parse, a PCA9685 setting
  and a J1939 payload can be read back out, and an identity signs and verifies a
  message without a caller splitting the signature off by hand.
- `cargo xtask builds` measures what each named feature set of the `pamoja`
  crate compiles, resolved for a fixed target so the counts are the same on
  every machine. The install page carries the table, regenerated and
  drift-checked with the rest of the generated documentation.
- Domains: the six chapters of the guides that hold more than one capability are
  installable as a unit in every language. In Rust each is a feature on the
  `pamoja` crate, so it decides what compiles. In the bindings each is a package
  (`@pamoja/field-io`, `pamoja-field-io`, `Pamoja.FieldIo`) that brings in its
  capabilities and, where the language allows it, re-exports each under its own
  name; a name two capabilities share stays reachable and unambiguous, which a
  flat re-export could not manage. Every domain is checked against the
  capability map, so a capability cannot fall out of its own domain.

### Changed

- `pamoja-ffi` exposes every capability behind a default-on feature and now
  depends on the whole workspace.
- Verifying an audit chain reports why it failed in every language, not only in
  Rust. The three bindings collapsed the engine's reason to a bare true or
  false, so a caller could tell that a log had been altered but not which record
  broke it or whether the log had instead been shortened. They now raise the
  reason, the way every other fallible call in them already does.
- A J1939 payload is a value with named signals rather than eight bytes to
  slice, in every language: `Signals` starts filled with the byte the standard
  reserves for a signal a controller is not reporting, the priorities and the
  broadcast address are named, and a broadcast identifier has its own
  constructor. The mesh header length and the two I2C address ranges the
  specification reserves are exported from the bindings as well, since both were
  known to the engine and to no caller.
- The Node binding is split into packages the way the crates are. `pamoja` is
  the whole framework in one package; each capability is its own `@pamoja/<name>`
  for installing only what you use; `@pamoja/core` is the engine's surface, the
  counterpart of `pamoja-core`; and `@pamoja/native` is the compiled engine and
  generated contract every package depends on. The `@pamoja/core/<name>` subpath
  imports are gone: `@pamoja/core/mqtt` is now `@pamoja/mqtt`, and
  `@pamoja/core/raw` is `@pamoja/native`.
- The Python binding is split the same way. `pamoja` is the whole framework in
  one distribution; each capability is `pamoja-<name>`, one module of the
  `pamoja` namespace; `pamoja-core` is the engine's surface (`pamoja.core`); and
  `pamoja-native` is the compiled engine, `pamoja._native`, that every
  distribution depends on. `pamoja` is a namespace package now, so the flat
  `from pamoja import DeviceIdentity` becomes `from pamoja.security import
  DeviceIdentity`, and `pamoja.transport` is `pamoja.core`.
- The .NET binding is split the same way. `Pamoja` is the whole framework in
  one package; each capability is `Pamoja.<Name>`, a package and a namespace of
  the same name; `Pamoja.Core` is the engine's surface; and `Pamoja.Native` is
  the compiled engine and the P/Invoke contract (`Pamoja.Native.Interop`) that
  every package depends on. Types keep their names but move namespaces
  (`Pamoja.Core.MqttClient` is `Pamoja.Mqtt.MqttClient`), and the transport
  factories move next to their clients: `Transport.Mqtt(options)` is
  `MqttTransport.Open(options)` and `Transport.Coap(options)` is
  `CoapTransport.Open(options)`.

### Fixed

- Two blocks of constants were missing or broken in the generated C header.
  `cbindgen` does not read the crates `pamoja-ffi` depends on, so a constant
  defined as another crate's constant was emitted as a bare identifier declared
  nowhere in the header, and the three PCA9685 values were dropped from it
  entirely. Each now carries its value with a compile-time assertion tying it to
  the crate that defines it.
- The capability tables in the install page and every binding README are grouped
  by chapter, so thirty rows read as a handful of domains.
- `Pamoja.Core` was two things at once, the engine's surface and the marshalling
  every facade needs, so all twenty-nine capability packages depended on it. The
  handle type, the error type, the status helpers, and string marshalling move to
  `Pamoja.Native`, where the rest of the P/Invoke contract already lives, and
  `PamojaException` sits in the root `Pamoja` namespace so a facade sees it
  without a using and a consumer catches it with `using Pamoja;`. Only the five
  transport packages depend on `Pamoja.Core` now, matching the Node and Python
  bindings, where a capability package depends on the engine alone.
- The Node facades exported enum constants a TypeScript caller could not pass to
  the facade's own functions. `PinLevel`, `PinEdge`, `PinPolarity`, `StepDrive`,
  `LinkCost`, and `EntityKind` held plain strings, which are not assignable to
  the `const enum` the generated contract takes, so every call needed a cast. The
  smoke suites are JavaScript and never saw it. The constants carry the contract
  type now, and `@pamoja/ros2` exports the contract's `EntityKind` type rather
  than one derived from its own object.
- The Node, Python, and .NET workflows all named their job "build and smoke
  test", so a pull request showed three identical checks, none of which could be
  required and none of which said which binding had failed. Each names its
  language now.
- The MQTT guide proved only that an unreachable broker is refused, which is the
  one thing a reader does not need shown. It runs a real round trip now: a
  gateway subscribes to a wildcard, a node publishes under it, and the reading
  arrives with its topic. The Rust example starts an in-process broker, and the
  three binding workflows start one, which `just broker` also starts locally.
- The gateway pairing code no longer appears in a captured dashboard log (#67).
- Broken intra-doc links in the rustdoc of nine crates, which docs.rs rendered
  as dead links; `cargo doc` now runs with warnings denied.
- `pamoja-lora` with its `std` feature on did not compile outside its own test
  build, because the crate stayed `no_std` regardless; it now links `std` when
  the feature is on, and the `pamoja` crate's default build exercises it.
- The install page described choosing packages as if it shrank a binding's
  download. It does not: each binding loads one engine carrying every
  capability, so the choice narrows the API and the dependency manifest. The
  page now says which of the two applies per language and measures the Rust
  claim, and `pamoja-ffi` documents the feature sets that do shrink the library
  for a C or C++ host that builds it.

### Dependencies

- napi 3.12.2 (#55), pyo3 0.29.2 (#56), and the npm, actions, and cargo minor
  groups (#57, #59, #60).

## [0.1.14] - 2026-08-25

### Changed

- The Node binding facade builds with TypeScript 7 (#53) and the native addon
  with napi-rs 3 (#43).
- The crypto stack moved to the digest 0.11 RustCrypto majors (#52), with
  x25519-dalek 3 (#26), ed25519-dalek 3 (#25), and aes 0.9 (#46).
- PyO3 0.29 clears the list and tuple iterator advisories.
- CodeQL scans through an explicit workflow with a fixed language list.

### Fixed

- Pages deployments no longer cancel each other mid-flight.

Earlier versions are described by their tags on GitHub.
