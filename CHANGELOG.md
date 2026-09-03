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

### Changed

- `pamoja-ffi` exposes every capability behind a default-on feature and now
  depends on the whole workspace.

### Fixed

- The gateway pairing code no longer appears in a captured dashboard log (#67).

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
