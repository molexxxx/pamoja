# Contributing to pamoja

Thanks for your interest in pamoja. The project is a single Rust core with thin
language bindings, built to run on hardware from a two-dollar microcontroller to a
gateway, so contributions are held to a bar that keeps it small, correct, and
portable. This guide covers how to build, test, and submit changes.

## Getting started

pamoja is a Cargo workspace. The engine and capability crates live under `crates/`,
the language bindings under `bindings/`, and runnable end-to-end scenarios under
`examples/`.

```sh
cargo build --workspace      # build the engine and capability crates
cargo test --workspace       # run tests, including doctests
```

You do not need any hardware. The `pamoja-sim` and `pamoja-loopback` crates provide
simulated sensors, actuators, and transports, and the tests and examples run entirely
against them.

## Examples in the documentation

Every code block in a guide, and the first example in the README, is spliced
from a test that runs in CI. Edit the test, not the Markdown, and run
`cargo xtask docs` to re-splice it; `cargo xtask docs --check` fails when a
committed block no longer matches its source. The four runners and the marker
convention are described on the
[building page](https://pamoja.molex.cloud/docs/about/building.html).

## Before you open a pull request

CI runs formatting, linting, tests, a dependency audit, and a set of drift and
footprint checks. Run the same checks locally first:

```sh
cargo fmt --all                                        # format
cargo clippy --workspace --all-targets -- -D warnings  # lint, warnings are errors
cargo test --workspace                                 # test
```

`just ci` runs the full set (formatting, clippy, the `no_std` builds, the dashboard
checks, and the generated-docs sync check) if you have [just](https://github.com/casey/just)
installed. `cargo xtask` lists the workspace tasks. `cargo deny check` audits the
dependency graph if you have [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
installed.

## What the code expects

- Public items are documented. `missing_docs` is denied at build time, so every public
  item needs a rustdoc comment with `# Arguments`, `# Returns`, and `# Errors` sections
  where they apply. Doc comments are the canonical documentation and carry runnable
  examples (doctests).
- The core stays `no_std`. Many crates compile without the standard library so they fit
  a microcontroller; keep `alloc`-free code paths allocation-free, and do not reach for
  `std` in a crate that builds `no_std`. CI builds the `no_std` crates with their
  default features off to catch a regression.
- Comments are professional and sparse. Keep file and module headers and the doc
  comments on public items; do not add inline narration that restates what the next line
  does or records why an edit was made.
- Standards compliance. When you implement something defined by a published standard (an
  RFC, a protocol or wire format, a datasheet, a crypto primitive), work from the current
  authoritative specification. Bit layouts, field orders, reserved bits, and algorithm
  constants are where the subtle bugs hide, and a plausible guess is worse than none.
  Anchor tests to the spec's own published reference vectors or worked examples, not
  just to round-trips, so an incorrect-but-self-consistent implementation is caught.
- Parsers that read untrusted input carry property tests. The framing and codec crates
  are on the network and radio boundary, so a decoder must never panic on arbitrary
  bytes. New parsing code should come with a `proptest` that feeds it arbitrary input
  and, where there is an encode path, a round-trip.

## Commits and pull requests

- Write short, imperative commit subjects ("Add the Modbus exception decoder"), with a
  body when the change needs explanation.
- Keep a pull request focused on one change, and make sure CI is green.
- Add or update tests for the behavior you change, and update the affected crate's
  documentation. Crate READMEs are generated from rustdoc with `cargo xtask docs`, so
  edit the doc comments and regenerate rather than hand-editing a README.

## License

By contributing, you agree that your contributions are licensed under the
[MIT License](LICENSE-MIT), the same license as the project.

## Conduct

Be respectful and constructive. This is a project aimed at helping people build things
that matter in hard environments; keep the community welcoming to newcomers, including
those who are not native English speakers or professional engineers.
