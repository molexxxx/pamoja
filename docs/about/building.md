# Building

## Repository layout

```
crates/      Rust engine and capability crates (each crate's README is its landing page)
bindings/    per-language bindings: node, python, dotnet
examples/    runnable end-to-end scenarios and the cross-language conformance generator
conformance/ the vectors every binding asserts, so the languages cannot disagree
docs/        this site: the guides, the capability map, and the pages about the project
sitl/        ArduPilot and PX4 SITL images for the MAVLink interop job
web/         the showcase site and the hosted dashboard demo
assets/      brand and logo
```

Device and transport simulators live in `pamoja-sim` and `pamoja-loopback`, so
the examples and tests run with no hardware.

## From source

```sh
cargo build --workspace      # build the engine and capability crates
cargo test --workspace       # run tests, including doctests and the MQTT round-trip

cd bindings/node
npm install && npm run build  # build the native addon and the TypeScript facade
npm test                      # smoke and conformance tests

cd ../python
python -m venv .venv && . .venv/bin/activate
pip install maturin pytest
maturin develop -m packages/native/Cargo.toml                          # build the engine, pamoja-native
pip install $(find packages -mindepth 1 -maxdepth 1 -type d ! -name native)  # every pure distribution
pytest                                                                  # smoke and conformance tests

cd ../..
cargo build -p pamoja-ffi --release                       # build the native C ABI and refresh pamoja.h
dotnet build bindings/dotnet/Pamoja.sln -c Release    # build the .NET interop and facade
dotnet run --project bindings/dotnet/tests/Pamoja.Smoke -c Release  # smoke and conformance tests
```

`just` lists the recipes CI runs, and `cargo xtask` lists the workspace tasks.

## Generated files

Several committed files are generated and checked in CI, so edit the source
they come from and regenerate:

| File | Source | Regenerate with |
| --- | --- | --- |
| `crates/*/README.md` | each crate's `lib.rs` rustdoc | `cargo xtask docs` |
| `docs/SUMMARY.md` and the tables in the READMEs and this site | `docs/capabilities.toml` | `cargo xtask docs` |
| `crates/pamoja-ffi/include/pamoja.h` | the `pamoja-ffi` source | `cargo build -p pamoja-ffi` |
| `bindings/node/packages/native/index.js` and `index.d.ts` | the Node binding source | `npm run build` in `bindings/node` |
| `bindings/node/packages/*/package.json`, `tsconfig.json`, and `README.md` | `docs/capabilities.toml` and each package's imports | `cargo xtask docs` |
| `bindings/python/packages/native/python/pamoja/_native/__init__.pyi` | the Python binding source | `cargo run --bin stub_gen` in `bindings/python/packages/native` |
| `bindings/python/packages/*/pyproject.toml`, `README.md`, and `py.typed` | `docs/capabilities.toml` and each portion's imports | `cargo xtask docs` |
| `conformance/vectors.json` | the Rust implementation | `cargo run -p pamoja-examples --example conformance_vectors` |

## This site

The guides are rendered by [mdBook](https://rust-lang.github.io/mdBook/) from
`docs/`, and the four references are generated beside them: rustdoc for the
crates, typedoc for the Node facade, pdoc for the Python facade, and DocFX for
the .NET facade. The docs workflow builds all of it on every pull request and
the Pages workflow publishes the same tree under `/docs`. Locally:

```sh
cargo install mdbook --version 0.5.4 --locked
cargo xtask docs && mdbook build          # the guides, into target/docs/site
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --exclude xtask --exclude pamoja-examples
cp -r target/doc target/docs/site/reference/rust
cd bindings/node/docs && npm ci && npx typedoc     # target/docs/site/reference/node
cd bindings/python && pip install pdoc==16.0 && pdoc pamoja '!pamoja._native' -o ../../target/docs/site/reference/python --docformat restructuredtext
dotnet tool install -g docfx --version 2.78.5 && docfx bindings/dotnet/docs/docfx.json
```

## Formatting and lints

CI runs `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets
-- -D warnings`, and the same two over the Node and Python binding crates,
which sit outside the workspace. Run both before pushing; the `just ci` recipe
runs everything the main CI job does.
