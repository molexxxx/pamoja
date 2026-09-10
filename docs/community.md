# Community

pamoja means together. The library is written so that the people who know a field,
a river, a clinic, or a workshop can add what they know without becoming Rust
engineers, and so that what they add is held to the same bar as the rest: it runs,
it is checked in CI, and it is documented where the next person will look. Four
things are open to anyone. A profile is a JSON file. An example is one program. A
driver is a decoder written from a datasheet, with a checklist that keeps it
honest. A board is a page and a program that compiles. Each has a path below, a
check that runs before it merges, and an issue form for the reader who has the
thing but not the toolchain to run the check. The rules for code itself are in
[CONTRIBUTING.md](https://github.com/molexxxx/pamoja/blob/main/CONTRIBUTING.md),
and the [building page](about/building.md) says how to run everything locally.

## Share a profile

A [profile](profiles.md) is the whole behavior of a node as data, and sharing one
needs no code.

1. Start from the manifest closest to what you built, from the
   [catalog](profiles.md), and change the numbers to the ones that worked: the
   setpoint and its deadband, the safe band an alert waits for, the sampling
   intervals your battery allowed. Give it a `name` in lowercase words joined by
   hyphens, a `topic`, and a `description` of a sentence or two saying what it
   watches or holds and what it does about it.
2. Run it. Load it with `Profile::from_json` (or `fromJson`, `from_json`,
   `FromJson`) and let its controller decide a few readings, as the
   [device profiles guide](guides/profile.md) does, or run it on the node.
3. Save it as `profiles/<name>.json`, where the file name is the profile's name,
   and run `cargo xtask profiles`. The task reads the file with the parser a
   device uses, checks it, and rewrites it into the form the library writes. Then
   `cargo xtask docs` adds it to the catalog.
4. Open a pull request with the file and the regenerated page. The pull request
   template asks how it was tested; "ran on a Pi in a chicken house for a week"
   is the right kind of answer.

Without a Rust toolchain, file the
[share a profile](https://github.com/molexxxx/pamoja/issues/new?template=profile.yml)
form with the manifest pasted in, and a maintainer runs the steps above and
credits you in the pull request.

What the check enforces, so a reader knows what a listed profile has been held to:
the name matches the file and is unique; the description is there and reads as a
sentence; the topic is one publishable path, with no wildcards; a custom kind is
named in lowercase words joined by underscores; a setpoint policy
has a deadband above zero and a safe band no narrower than it; a level policy
warns at least one sample ahead; a surge policy has a limit above zero; the
sampling intervals do not shorten as the battery drains, and the two thresholds
sit between zero and one in the right order; every dashboard element has a
snake_case key used once, a unit, a label, a band with its low end first, and a
starting state the dashboard has words for, either one it ships or one the
manifest supplies under `messages`, where a per-locale message carries an `en`
text for the locales it does not name.

## Share an example

An [example](examples.md) is a complete program with a `main`, written to be read
top to bottom, that runs with nothing plugged in because a simulator or the
loopback transport stands in for the hardware. Every one runs in CI on every
change, so an example on the page is never one that worked once.

1. Write it as `examples/community/<name>.rs`. The module doc's first paragraph
   is what the examples page shows, and a line reading `Run with:` followed by
   the command in backticks is the line beside it; without one the page shows the
   default `cargo run` line. Say who wrote it in the doc, since the page credits
   the file and its history, not a list of names.
2. Register it in `examples/Cargo.toml` as an `[[example]]` whose `path` is
   `community/<name>.rs`, and run it: `cargo run -p pamoja-examples --example
   <name>`. It must finish on its own and print what it found.
3. Run `cargo xtask docs`, which lists it under "Community programs", and open a
   pull request.

A program in TypeScript, Python, or C# is welcome too; say which language in the
[share an example](https://github.com/molexxxx/pamoja/issues/new?template=example.yml)
form, and it lands beside the guide examples for that language.

## Contribute a driver

A driver is a decoder for a part, written from the manufacturer's datasheet and
nothing else, with a driver type over the [bus traits](guides/hal.md) that runs it
on real hardware and implements `Sensor` or `Actuator` so the kit, the profiles,
and the dashboard take it as they take the ones that ship. The
[your own device guide](guides/device.md) shows the shape for a part that stays in
your own tree; this is the path for one that ships with pamoja.

Claim the part first with the
[contribute a driver](https://github.com/molexxxx/pamoja/issues/new?template=driver.yml)
form, with a link to the datasheet, so two people do not write the same decoder in
the same month. Then the checklist, which is the same audit every shipped driver
went through:

1. The datasheet is the manufacturer's own, linked from a new entry in
   `docs/hardware.toml` with the figures the [hardware page](hardware.md) shows,
   the price band, and two or three places to buy it with the price on the day.
   `cargo xtask links` must fetch the source; a vendor page that refuses scripted
   readers is marked as such rather than replaced with a mirror.
2. The decode module in `pamoja-sensors` or `pamoja-actuators` is `no_std`, takes
   no dependencies, and carries every register address, command word, timing, and
   compensation formula as a named constant or a documented function.
3. Every constant is searched for in the datasheet text, in the spellings a
   datasheet uses (hex with and without `0x`, zero-padded, decimal, scaled), and a
   value that a table splits across columns is checked by reading the table, not
   by loosening the search.
4. Every formula is recomputed independently, by transcribing the datasheet's
   formula into a script and reproducing the values the tests assert, without
   reading the module first.
5. Every register layout is checked field by field against the datasheet's field
   table, including reserved bits that must keep a fixed pattern.
6. The tests anchor to the datasheet's own worked examples and published check
   values, not only to round trips, so a decoder that is wrong but self-consistent
   is caught. When the datasheet and the code disagree, find which other value in
   the datasheet the printed one matches before assuming the code is wrong; a
   misprint has been found this way.
7. The driver type runs the part's transfer sequence over the bus traits and is
   tested against a scripted bus that plays the part's side of the conversation.
8. The hardware entry claims the module, so `cargo xtask docs --check` ties the
   two together, and the [sensors](guides/sensors.md) or
   [actuators](guides/actuators.md) guide names the part.
9. Where a decoded value crosses into the bindings, it reaches all three, with a
   conformance vector every language asserts.
10. The CHANGELOG entry says what the part is and what document it was written
    from.

## Add a board

A board is documentation and a program, not a type: a page under `docs/boards/`
with the board's pins for each bus, wiring for one shipped part, the settings or
toolchain the board needs, and a first program that reads the part through the
shipped driver, every figure from the maker's own documentation and linked at the
end of the page. The program is a standalone package under `examples/boards/`
with its own empty `[workspace]` table and a `.cargo/config.toml` naming its
target and runner; CI builds it from inside its directory on every change. The
board's card in `docs/hardware.toml` names the page in its `page` field, and the
[ESP32](boards/esp32.md) and [RP2040](boards/rp2040.md) pages are the two shapes
to copy, for a chip with its own Rust hardware layer and for one on a community
one.

## Report and request

Something that behaves differently from its documentation is a
[bug](https://github.com/molexxxx/pamoja/issues/new?template=bug.yml). A part,
a protocol, or a job pamoja cannot do yet is a
[capability request](https://github.com/molexxxx/pamoja/issues/new?template=capability.yml).
A page that is wrong, unclear, or missing is a
[documentation problem](https://github.com/molexxxx/pamoja/issues/new?template=docs.yml).
A vulnerability goes through the private channel in
[SECURITY.md](https://github.com/molexxxx/pamoja/blob/main/SECURITY.md), never a
public issue.
