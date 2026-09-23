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

<!-- languages end -->

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

Every code block in a guide is spliced from a program CI runs on every change. A
guide page holds a region such as

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
from pamoja.hal import Parity, SerialSettings
from pamoja.modbus import BROADCAST, ModbusClient, ModbusClientError, ModbusLine, ModbusServer


def word(on: bool) -> str:
    """A state as the output prints it."""
    return "on" if on else "off"


# The line: 19200 baud, even parity, one stop bit, the default the Modbus specification sets.
# Eleven bits a character, and 3.5 of them of silence mark where a frame ends.
settings = SerialSettings(19_200, Parity.EVEN)
gap = ModbusClient.frame_gap_nanos(settings) // 1_000
print(f"line         {settings}, {settings.bits_per_character} bits a character, t3.5 is {gap} us")

# Each device's manual gives its unit address and where its values live. The meter keeps its
# measurements in input registers from 0: volts in tenths, amps in hundredths, then a fault
# word. The relay module has four relays as coils 0 to 3, and the tank's low-level float switch
# as discrete input 0, on while the water is below it.
METER = 17
PUMP = 18
meter = ModbusServer(METER)
meter.set_input_registers(0, [2301, 418, 0])
relays = ModbusServer(PUMP)
relays.set_coils(0, [False] * 4)
relays.set_discrete_inputs(0, [True])
line = ModbusLine().attach(meter).attach(relays)

# The devices sit on a simulated line. On a gateway the port is
# SerialPort.open("/dev/ttyUSB0", settings), and nothing after this statement changes.
port = line.port(settings)
client = ModbusClient(port)

# Poll the meter with function 0x04 for three input registers, and scale each one as its manual
# says.
registers = client.read_input_registers(METER, 0, 3)
print(f"meter        {registers[0] / 10:.1f} V, {registers[1] / 100:.2f} A, faults {registers[2]}")

# What that poll cost the line: the request, the reply, and the silence before the request.
out, back = port.written, port.received
line_time = settings.transfer_micros(out) + settings.transfer_micros(back) + gap
print(f"poll         {out} bytes out, {back} back, {line_time / 1_000:.2f} ms of line time")

# Read the float switch, and start the pump on relay 0 when the tank is low.
[low] = client.read_discrete_inputs(PUMP, 0, 1)
print(f"tank         low-level switch {word(low)}")
if low:
    client.write_single_coil(PUMP, 0, True)
states = client.read_coils(PUMP, 0, 4)
print(f"relays       {' '.join(word(on) for on in states)}")

# A broadcast, to unit 0, reaches every device on the line and none answers: here every relay
# off at once. The client waits out the turnaround so each device has carried it out before the
# next request.
client.write_multiple_coils(BROADCAST, 0, [False] * 4)
print(f"broadcast    every relay off, no reply, {round(client.turnaround * 1_000)} ms turnaround")
after = client.read_coils(PUMP, 0, 4)
print(f"relays       {' '.join(word(on) for on in after)}")

# The meter keeps its measurements in input registers. Asking for them as holding registers,
# function 0x03, is the usual mistake with a new device, and the meter refuses it with an
# exception instead of answering.
refused = None
try:
    client.read_holding_registers(METER, 0, 3)
except ModbusClientError as error:
    refused = error
    print(f"refused      {error}")

# A unit that is not on the line never answers. The client gives up after its response timeout,
# one second unless told otherwise, which a simulated line counts instead of sleeping through.
before = port.waited_micros
silent = None
try:
    client.read_input_registers(19, 0, 1)
except ModbusClientError as error:
    silent = error
    waited = (port.waited_micros - before) // 1_000
    print(f"silent       {error}, {waited} ms counted and not slept")
```
<!-- end -->

`cargo xtask docs --check` fails if the spliced text no longer matches the
source, so a drifting example breaks the build rather than the reader's trust. One file
per guide and language:

| Language | File | Run with |
| --- | --- | --- |
| Rust | [`examples/guides/`](https://github.com/molexxxx/pamoja/tree/main/examples/guides)`<name>.rs`, a program with a `main`, declared in `Cargo.toml` | `cargo run -p pamoja-examples --example <name>` |
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
