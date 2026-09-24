# Community

pamoja means together. The people who know a field, a river, a clinic, or a workshop
can add what they know without becoming Rust engineers, and what they add is held to
the same bar as the rest: it runs, CI checks it, and it is documented where the next
person will look.

| Add | What it is | Checked by | No toolchain? |
| --- | --- | --- | --- |
| [A profile](#share-a-profile) | one JSON file | `cargo xtask profiles` | [share a profile](https://github.com/molexxxx/pamoja/issues/new?template=profile.yml) |
| [An example](#share-an-example) | one program that runs with nothing plugged in | CI runs it on every change | [share an example](https://github.com/molexxxx/pamoja/issues/new?template=example.yml) |
| [A driver](#contribute-a-driver) | a decoder written from a datasheet | the audit checklist below | [contribute a driver](https://github.com/molexxxx/pamoja/issues/new?template=driver.yml) |
| [A board](#add-a-board) | a page and a program that compiles | CI builds the program | [capability request](https://github.com/molexxxx/pamoja/issues/new?template=capability.yml) |

The rules for code are in
[CONTRIBUTING.md](https://github.com/molexxxx/pamoja/blob/main/CONTRIBUTING.md), and
the [building page](about/building.md) runs everything locally.

## Share a profile

A [profile](profiles.md) is a node's behavior as data, so sharing one needs no code.

1. **Start from the closest one** in the [catalog](profiles.md) and change the numbers
   to the ones that worked: the setpoint and its deadband, the safe band, the sampling
   intervals your battery allowed. Give it a `name` in lowercase words joined by
   hyphens, a `reads`, a `topic`, and a one or two sentence `description`. Keep the
   `$schema` line, and an editor such as VS Code checks every field as you type.
2. **Run it.** [`pamoja-node`](run.md) plays readings through it with nothing wired,
   or on the real part, and the [device profiles guide](guides/profile.md) loads it in
   any of the four languages.
3. **Check it.** Save it as `profiles/<name>.json` and run `cargo xtask profiles`,
   which reads it with the parser a device uses and rewrites it into the library's own
   form. `cargo xtask docs` then adds it to the catalog.
4. **Open a pull request** with the file and the regenerated page. The template asks
   how it was tested; "ran on a Pi in a chicken house for a week" is the right kind of
   answer.

Without a Rust toolchain, paste the manifest into the
[share a profile](https://github.com/molexxxx/pamoja/issues/new?template=profile.yml)
form, and a maintainer runs the steps and credits you in the pull request.

What every listed profile has been held to:

- **Fields:** every field is one the [published schema](https://pamoja.molex.cloud/schema/profile-1.json)
  has.
- **Identity:** the name matches the file and is unique, and the description reads as a
  sentence.
- **Reading:** `reads` names the quantity and unit, and the topic is one publishable
  path with no wildcards.
- **Policy:** a setpoint has a deadband above zero and a safe band no narrower than it;
  a level warns at least one sample ahead; a surge has a limit above zero; a custom kind
  is named in lowercase words joined by underscores.
- **Power:** the sampling intervals do not shorten as the battery drains, and the two
  thresholds sit between zero and one in order.
- **Drawing:** every dashboard element has a snake_case key used once, a unit, a label,
  a band with its low end first, and a starting state the dashboard has words for,
  shipped or supplied under `messages` with an `en` text.

## Share an example

An [example](examples.md) is a complete program with a `main`, written to be read top to
bottom, that runs with nothing plugged in because a simulator or the loopback transport
stands in for the hardware. CI runs every one on every change.

1. Write it as `examples/community/<name>.rs`. The module doc's first paragraph is what
   the examples page shows, and a line reading `Run with:` followed by a command in
   backticks is the command beside it. Say who wrote it in the doc.
2. Register it in `examples/Cargo.toml` as an `[[example]]` whose `path` is
   `community/<name>.rs`, and run it with
   `cargo run -p pamoja-examples --example <name>`. It must finish on its own and print
   what it found.
3. Run `cargo xtask docs`, which lists it under "Community programs", and open a pull
   request.

A program in TypeScript, Python, or C# is welcome too: name the language in the
[share an example](https://github.com/molexxxx/pamoja/issues/new?template=example.yml)
form, and it lands beside that language's guide examples.

## Contribute a driver

A driver decodes a part from the manufacturer's datasheet and nothing else, runs it
over the [bus traits](guides/hal.md), and implements `Sensor` or `Actuator`, so the
kit, the profiles, and the dashboard take it like the ones that ship. The
[your own device guide](guides/device.md) covers a part that stays in your own tree;
this is the path for one that ships with pamoja.

Claim the part first with the
[contribute a driver](https://github.com/molexxxx/pamoja/issues/new?template=driver.yml)
form and a link to the datasheet, so two people do not write the same decoder. Then
work through the audit every shipped driver went through:

1. **Source.** The datasheet is the manufacturer's own, linked from a new entry in
   `docs/hardware.toml` with the figures the [hardware page](hardware.md) shows, a price
   band, and two or three places to buy it. `cargo xtask links` must fetch it; a vendor
   page that refuses scripted readers is marked as such, never swapped for a mirror.
2. **Decoder.** The module in `pamoja-sensors` or `pamoja-actuators` is `no_std`, has
   no dependencies, and names every register, command word, timing, and formula.
3. **Constants.** Each one is found in the datasheet text in every spelling a datasheet
   uses (hex with and without `0x`, zero-padded, decimal, scaled). A value split across
   table columns is checked by reading the table.
4. **Formulas.** Each is recomputed from the datasheet in a script, without reading the
   module first, and reproduces the values the tests assert.
5. **Registers.** Every layout is checked field by field against the datasheet's table,
   reserved bits included.
6. **Tests.** They anchor to the datasheet's own worked examples and check values, not
   only round trips. When code and datasheet disagree, find which other printed value
   the datasheet's number matches before blaming the code; a misprint has been found
   this way.
7. **Driver.** It runs the part's transfer sequence over the bus traits, tested against
   a scripted bus playing the part's side.
8. **Docs.** The hardware entry claims the module, so `cargo xtask docs --check` ties
   the two together, and the [sensors](guides/sensors.md) or
   [actuators](guides/actuators.md) guide names the part.
9. **Bindings.** A decoded value that crosses into the bindings reaches all three, with
   a conformance vector every language asserts.
10. **Changelog.** The entry says what the part is and which document it was written
    from.

## Add a board

A board is documentation and a program, not a type:

- **A page** under `docs/boards/` with the board's pins for each bus, wiring for one
  shipped part, the settings or toolchain it needs, and every figure linked to the
  maker's own documentation.
- **A program** under `examples/boards/`, a standalone package with its own empty
  `[workspace]` table and a `.cargo/config.toml` naming its target and runner, which
  reads the part through the shipped driver. CI builds it from its directory on every
  change.
- **A card** in `docs/hardware.toml` whose `page` field names the page.

The [ESP32](boards/esp32.md) page is the shape to copy for a chip with its own Rust
hardware layer, and the [RP2040](boards/rp2040.md) page for one on a community layer.

## Report and request

| You found | Where it goes |
| --- | --- |
| Something that behaves differently from its documentation | [bug](https://github.com/molexxxx/pamoja/issues/new?template=bug.yml) |
| A part, protocol, or job pamoja cannot do yet | [capability request](https://github.com/molexxxx/pamoja/issues/new?template=capability.yml) |
| A page that is wrong, unclear, or missing | [documentation problem](https://github.com/molexxxx/pamoja/issues/new?template=docs.yml) |
| A vulnerability | the private channel in [SECURITY.md](https://github.com/molexxxx/pamoja/blob/main/SECURITY.md), never a public issue |
