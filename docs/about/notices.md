# Notices and contact

Who holds the copyright, what the project uses that it did not write, and how to reach a
person.

## Copyright and license

Copyright 2026 Anthony Wiedman. Released under the
[MIT license](https://github.com/molexxxx/pamoja/blob/main/LICENSE-MIT), which travels
with every crate, every package, and this site. The [terms](terms.md) say what that means
in practice.

## The typefaces

Both faces are served from this site rather than from a font host, and their licenses are
served beside them.

| Typeface | Used for | License |
| --- | --- | --- |
| [Archivo](https://github.com/Omnibus-Type/Archivo) | Every heading, label, and run of reading text | [SIL Open Font License 1.1](/fonts/LICENSE-Archivo.txt) |
| [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) | Anything measured, installed, or typed | [SIL Open Font License 1.1](/fonts/LICENSE-JetBrainsMono.txt) |

## Dependencies

The engine is written to need very little. A build that names one capability compiles that
crate, `pamoja-core`, and nothing else from outside the workspace, which the table on the
[install page](../install.md) measures per feature set.

What the wider builds do pull in is audited on every change. Continuous integration runs
`cargo-deny` over the graph a user actually installs, refusing a dependency whose license
is not on the allowed list and failing on any security advisory against it. The policy is
[`deny.toml`](https://github.com/molexxxx/pamoja/blob/main/deny.toml) in the repository,
and the licenses of the crates in any particular build are listed by `cargo deny list`
against your own lockfile.

The bindings add a runtime each: napi-rs for TypeScript, PyO3 for Python, and cbindgen
with P/Invoke for C#. Each carries its own license, declared in its own package.

## Standards and third-party names

The capabilities implement published specifications, and the hardware reference cites the
datasheets and standards the drivers were written from. Names such as Modbus, CAN, J1939,
LoRa, LoRaWAN, MAVLink, ROS 2, Zenoh, and the names of the parts and boards documented
here belong to their respective owners. They appear to say what a capability speaks to.
None of them is affiliated with this project, and none of them endorses it.

## Security

Report a suspected vulnerability privately, never in a public issue or pull request. The
[security policy](https://github.com/molexxxx/pamoja/blob/main/SECURITY.md) says what to
send and what happens next; the fastest route is
[a private advisory](https://github.com/molexxxx/pamoja/security/advisories/new) on the
repository.

## Conduct

The project follows a [code of conduct](https://github.com/molexxxx/pamoja/blob/main/CODE_OF_CONDUCT.md)
that applies to the repository, the issues, and anywhere the project is represented.

## Contact

There is one maintainer, and these are the ways to reach them.

| For | Where | Public |
| --- | --- | --- |
| A bug, a question, or a capability request | [Open an issue](https://github.com/molexxxx/pamoja/issues/new/choose) | Yes |
| A change you want to contribute | [Open a pull request](https://github.com/molexxxx/pamoja/pulls), after reading [CONTRIBUTING](https://github.com/molexxxx/pamoja/blob/main/CONTRIBUTING.md) | Yes |
| A suspected vulnerability | [A private advisory](https://github.com/molexxxx/pamoja/security/advisories/new) | No |
| A conduct report, or anything else private | <molex@sent.com> | No |

An issue is the right default. It is public, so the next person with the same question
finds the answer, and it is where the work is tracked.

## This page

Like every page here, this one is a file in the repository and changes by a commit. The
revision in the footer is the version you are reading.
