# Security Policy

pamoja is built for deployments where a failure has real consequences: cold-chain
alarms, water and power monitoring, flood warning, and the vehicles and robots the
drone and robotics crates talk to. Security is a project pillar, not an afterthought,
so vulnerability reports are taken seriously and handled promptly.

## Supported versions

pamoja is pre-1.0 and all crates share one workspace version. Security fixes land on
`main` and ship in the next patch release across every registry (crates.io, npm, PyPI,
NuGet). Only the latest published `0.1.x` release is supported; if you are on an older
version, the fix is to upgrade.

## Reporting a vulnerability

Please report suspected vulnerabilities privately, not through a public issue, pull
request, or discussion.

- Preferred: open a private report through GitHub's "Report a vulnerability" button
  under the repository's Security tab
  (https://github.com/molexxxx/pamoja/security/advisories/new). This keeps the report
  confidential until a fix is available and gives us a private channel to coordinate.

A good report includes the affected crate and version, the platform, a description of
the issue and its impact, and a minimal reproduction (a byte sequence, a code snippet,
or a failing test) where possible.

## What to expect

- Acknowledgement of your report within a few days.
- An initial assessment of severity and affected versions, and a private channel to
  work through the details with you.
- A coordinated fix and a patch release across the affected registries, with a
  published advisory once users have a version to upgrade to.
- Credit for the report, if you would like it.

Please give us a reasonable window to release a fix before any public disclosure.

## Scope and areas of concern

The highest-risk surfaces, and where a report is most valuable:

- The parsers that read untrusted bytes off a wire or radio link: the MAVLink,
  LoRaWAN, mesh, Modbus, CAN/J1939, and serial (SLIP/COBS) framing, and the codecs.
  These are exercised with property tests, but a crafted input that panics, hangs, or
  misparses is exactly the kind of issue we want to hear about.
- The `unsafe` boundary in `pamoja-ffi`, the single crate that exposes the C ABI.
- The cryptographic code in `pamoja-security` (ed25519 identity), `pamoja-session`
  (X25519, HKDF, ChaCha20-Poly1305), `pamoja-audit` (hash-chained log), and the
  LoRaWAN AES-CMAC and AES paths.
- The dashboard's authenticated control surface in `pamoja-dashboard`.

The build tooling and the dependency graph are audited in CI with `cargo deny`
(licenses and security advisories) on every change.
