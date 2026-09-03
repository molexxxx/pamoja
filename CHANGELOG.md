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
  with its test suite.

### Changed

- `pamoja-ffi` exposes every capability behind a default-on feature and now
  depends on the whole workspace.
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

- The gateway pairing code no longer appears in a captured dashboard log (#67).
- Broken intra-doc links in the rustdoc of nine crates, which docs.rs rendered
  as dead links; `cargo doc` now runs with warnings denied.
- `pamoja-lora` with its `std` feature on did not compile outside its own test
  build, because the crate stayed `no_std` regardless; it now links `std` when
  the feature is on, and the `pamoja` crate's default build exercises it.

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
