# Building

Everything builds and tests with nothing plugged in. A first pass through the Rust
workspace takes a few minutes; each binding adds its own toolchain on top. When you are
done, [open a pull request](https://github.com/molexxxx/pamoja/blob/main/CONTRIBUTING.md).

## Repository layout

```text
crates/      the Rust engine and the capability crates
bindings/    per-language bindings: node, python, dotnet
examples/    end-to-end scenarios and the conformance generator
profiles/    the shared device profiles, one JSON manifest each
conformance/ the vectors every binding asserts
docs/        this site: guides, the capability map, about
sitl/        ArduPilot and PX4 images for the MAVLink interop job
web/         stylesheets, scripts, typefaces, front-page data
assets/      brand and logo
```

Device and transport simulators live in
[`pamoja-sim`](https://github.com/molexxxx/pamoja/tree/main/crates/pamoja-sim) and
[`pamoja-loopback`](https://github.com/molexxxx/pamoja/tree/main/crates/pamoja-loopback), so
the examples and tests run with no hardware.

## From source

Each language builds on its own, from the repository root. Every path below is relative to
it, so no step depends on where a previous one left you.

### Rust

```sh
cargo build --workspace   # the engine and the capability crates
cargo test --workspace    # tests, including doctests
```

### TypeScript

```sh
npm --prefix bindings/node install
npm --prefix bindings/node run build   # the native addon and the TypeScript facade
npm --prefix bindings/node test        # smoke and conformance tests
```

### Python

```sh
python -m venv .venv
. .venv/bin/activate                   # Windows: .venv\Scripts\activate
pip install maturin pytest
maturin develop -m bindings/python/packages/native/Cargo.toml   # the engine, pamoja-native
pip install bindings/python/packages/*/ --no-deps               # every pure distribution
python -m pytest bindings/python                                # smoke and conformance tests
```

### C#

```sh
cargo build -p pamoja-ffi --release                  # the C ABI, and refresh pamoja.h
dotnet build bindings/dotnet/Pamoja.sln -c Release   # the interop and the facade
dotnet run --project bindings/dotnet/tests/Pamoja.Smoke -c Release
```

[`just`](https://github.com/molexxxx/pamoja/blob/main/justfile) lists the recipes CI runs:
`just ci` runs everything the main job does, and `just guides` runs the four guide suites.
`cargo xtask` lists the workspace tasks.

## Generated files

Several committed files are generated and checked in CI. Edit the source they come from,
then run the command that rebuilds them. Four commands cover everything:

| Run | Rebuilds | From |
| --- | --- | --- |
| `cargo xtask docs` | the crate READMEs, `docs/SUMMARY.md`, the capability tables on this site, and each binding package's manifest and README | each crate's `lib.rs` rustdoc, and [`docs/capabilities.toml`](https://github.com/molexxxx/pamoja/blob/main/docs/capabilities.toml) |
| `cargo build -p pamoja-ffi` | `crates/pamoja-ffi/include/pamoja.h` | the `pamoja-ffi` source |
| `npm run build` in `bindings/node` | `packages/native/index.js` and `index.d.ts` | the Node binding source |
| `cargo run --bin stub_gen` in `bindings/python/packages/native` | `python/pamoja/_native/__init__.pyi` | the Python binding source |

Two more stand on their own: `cargo run -p pamoja-examples --example conformance_vectors`
rebuilds [`conformance/vectors.json`](https://github.com/molexxxx/pamoja/blob/main/conformance/vectors.json)
from the Rust implementation, and `cargo xtask profiles` rewrites `profiles/*.json` in
canonical form through `Profile::to_json`.

## Guide examples

Every code block in a guide is spliced from a test that ran in CI. A guide page
holds a region such as

```md
<!-- snippet: bindings/python/guides/modbus.py#example -->
<!-- end -->
```

and `cargo xtask docs` fills it with the lines between `# ANCHOR: example` and
`# ANCHOR_END: example` in
[`bindings/python/guides/modbus.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/modbus.py),
dedented, under a link to the file. The region above produces exactly this, which is
the [Modbus guide](../guides/modbus.md)'s Python listing, spliced here by the same
mechanism so this page cannot describe something it does not do:

<!-- snippet: bindings/python/guides/modbus.py#example -->
From [`bindings/python/guides/modbus.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/modbus.py):

```python
from pamoja.core import PamojaError
from pamoja.modbus import parse_frame, read_holding_registers, read_holding_registers_reply

# The device this gateway polls: a power meter at unit 17, whose manual says the three
# registers holding voltage, current and a fault word start at address 107.
METER = 17
FIRST_REGISTER = 107

# Ask it for those three registers. The frame is complete, checksum included, exactly as
# it goes out on the wire.
request = read_holding_registers(METER, FIRST_REGISTER, 3)
print(f"polling unit {METER}, {len(request)} bytes out")

# A stand-in for the meter. On a running gateway this frame arrives over RS485; here the
# library builds what a meter reporting those three values would send back.
from_the_meter = read_holding_registers_reply(METER, [2301, 418, 0])

# Everything below is the gateway's own code. A reply carries its own checksum, so the
# frame is validated before any value is read out of it.
reply = parse_frame(from_the_meter)
registers = reply.registers()
print(f"voltage   {registers[0] / 10:.1f} V")
print(f"current   {registers[1] / 100:.2f} A")
print(f"faults    {registers[2]}")

# One flipped bit anywhere in the frame fails the checksum, which is the whole point of
# carrying one over a long RS485 run.
mangled = bytearray(from_the_meter)
mangled[2] ^= 0xFF
try:
    parse_frame(bytes(mangled))
    print("mangled frame accepted, which should never happen")
except PamojaError as error:
    print(f"mangled frame rejected: {error}")
```
<!-- end -->

`cargo xtask docs --check` fails if the spliced text no longer matches the
source, so a drifting example breaks the build rather than the reader's trust. One file
per guide and language:

| Language | File | Run with |
| --- | --- | --- |
| Rust | [`examples/tests/guides/`](https://github.com/molexxxx/pamoja/tree/main/examples/tests/guides)`<name>.rs`, one `#[test]`, declared in `main.rs` | `cargo test -p pamoja-examples --test guides` |
| TypeScript | [`bindings/node/guides/`](https://github.com/molexxxx/pamoja/tree/main/bindings/node/guides)`<name>.ts`, top-level statements with `node:assert/strict` | `npm run test:guides` in `bindings/node` |
| Python | [`bindings/python/guides/`](https://github.com/molexxxx/pamoja/tree/main/bindings/python/guides)`<name>.py`, a script with plain `assert` | `python -m pytest tests/test_guides.py` in `bindings/python` |
| C# | [`bindings/dotnet/samples/Pamoja.Guides/`](https://github.com/molexxxx/pamoja/tree/main/bindings/dotnet/samples/Pamoja.Guides)`<Name>Guide.cs`, a static `Run()` called from `Program.cs` | `dotnet run --project bindings/dotnet/samples/Pamoja.Guides` |

`just guides` runs all four. No example needs a broker, a server, or hardware:
where a capability is a network client, the example covers what is decidable
without one, and the loopback transport carries the round trips. The TypeScript
files import the `@pamoja/<name>`
packages through the workspace links under `node_modules`, so they see each
package the way a user does; build the facade first. The C# project has a plain
`Guides` namespace rather than a `Pamoja.*` one, so its examples name types the
way an application does.

## This site

The pages are rendered from `docs/` by `cargo xtask site`, a static-site generator
in the workspace task runner, and the four references are generated into the same
tree: rustdoc for the crates, typedoc for the Node packages, pdoc for the Python
packages, and DocFX for the .NET packages. The reference page for each language
is the way into its generated tree, whose own root pages redirect to it. Every
code block is highlighted when the site is rendered, every link is checked
(references included, once they are in place), and the docs workflow does all of
it on every pull request; the Pages workflow publishes the same tree under
`/docs`. Locally, the generators run first so the pages can overwrite their roots:

```sh
cargo xtask docs                 # the generated regions, checked in
mkdir -p target/site/docs/reference

# Rust
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps \
  --exclude xtask --exclude pamoja-examples
cp -r target/doc target/site/docs/reference/rust

# TypeScript, Python, C#, each into its own tree under docs/reference
(cd bindings/node/docs && npm ci && npx typedoc)
(cd bindings/python && pip install pdoc==16.0 \
   && pdoc pamoja '!pamoja._native' --docformat restructuredtext \
      -o ../../target/site/docs/reference/python)
dotnet tool install -g docfx --version 2.78.5
docfx bindings/dotnet/docs/docfx.json

cargo xtask site                 # the pages, into target/site
cargo xtask site --verify        # every link resolves, references included
node web/serve.mjs --root target/site    # localhost:8099, live reload
```

## Formatting and lints

CI runs `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets
-- -D warnings`, and the same two over the Node and Python binding crates,
which sit outside the workspace. Run both before pushing; the `just ci` recipe
runs everything the main CI job does.

The likeliest first failure is `cargo xtask docs --check`, which fails when a generated
file no longer matches its source. Run `cargo xtask docs` and commit what it writes.

## Sending a change

[CONTRIBUTING.md](https://github.com/molexxxx/pamoja/blob/main/CONTRIBUTING.md) covers what
a pull request needs, [SECURITY.md](https://github.com/molexxxx/pamoja/blob/main/SECURITY.md)
covers reporting a vulnerability privately, and the
[community page](../community.md) covers contributing a profile, an example, a driver, or a
board.
