# pamoja-dashboard

<a href="https://crates.io/crates/pamoja-dashboard"><img height="28" alt="crates.io" src="https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-cratesio.svg"></a>
<a href="https://docs.rs/pamoja-dashboard"><img height="28" alt="docs.rs" src="https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docsrs.svg"></a>

A local-first dashboard a device serves over its own WiFi hotspot. A clinic worker, a
farmer, or a water committee opens a page on whatever cheap phone they have and sees their
fleet - sensors, alarms, battery, signal - in their own language, with no internet and no
app to install. The same dashboard a professional runs on a gateway in a city runs on a
two-dollar microcontroller in a village.

## The mental model

One rule makes all of this work: **the device ships plain data, the page does everything
else.** The device serves a small, language-neutral JSON snapshot of the fleet; the browser
draws the gauges, formats the numbers, translates the labels, and lays out right-to-left.
That keeps the device's job tiny and the page's job rich.

```
device  ──GET /state──▶  browser   (renders, formats, translates with its own Intl/CLDR)
        ◀─POST /command──           (authenticated control: open a valve, add a sensor)
```

The whole dashboard talks to one seam, the `StateSource` trait:

- `snapshot() -> State` - the fleet the page renders.
- `command(&Command)` - an authenticated control action to apply.

Anything that implements `StateSource` can drive the dashboard. Two do:

- **`Mock`** - a hardware-free demo fleet for development and the hosted showcase. Behind the
  `mock` feature, off by default (turn it on with `--features mock`), so a default build carries
  no demo scaffolding.
- **`Fleet`** - the real source a project fills. This is what you use on a real device.

## How it reaches a phone over a radio mesh

A phone's browser needs an IP link to load the page, and a long-range radio mesh (LoRa,
Meshtastic) is not IP. So the gateway bridges the two: it serves the dashboard over its own WiFi
access point - what the phone connects to, with no internet and no app to install - while the
mesh or LoRa link is the **backhaul** that fills the fleet. Field readings arrive over the radio,
the project's sampling loop pushes them into the `Fleet`, and the page renders them; the radio
never carries the page itself.

That split is why a two-dollar microcontroller can host this. The page is served once over the
local hotspot (gzip-encoded, well under 150 KB); only the small `State` snapshot moves after
that. The HTTP/1.1 server runs over a pluggable byte transport (plain TCP today), so a capable
tier can later supply a TLS transport and the browser upgrades to HTTPS - and with it HTTP/2 -
without touching the request logic.

## Using it with your project

A real project owns its own sensing (it ticks its profiles/sensors on their power schedule)
and **pushes** results into a `Fleet`; the dashboard reads them and hands control commands
back for the project to apply. This push model is why the dashboard works with any project
and stays synchronous and dependency-light.

```rust
use pamoja_dashboard::{Assets, Fleet, LinkKind, Reading, Sensor, Server, Status};

// 1. Declare the fleet's shape. The reading here is only the starting value shown until the
//    first real sample arrives - it is not a fixed value; live values are fed in step 2.
let fleet = Fleet::builder()
    .org("farm", "Pamoja farm")
    .group("farm", "field", "Field node", LinkKind::Lora)
    .sensor("field", Sensor::new("soil", Reading::new("soil_moisture", 60.0, "percent").with_band(40.0, 80.0)))
    .build();

// 2. From your own sampling loop, feed each real reading in. The Fleet keeps the rolling
//    history (the sparkline) for you, and queues any control commands for you to apply.
let worker = fleet.clone();
loop {
    let value = read_soil_sensor();  // your driver, or a pamoja-sensors decoder
    worker.report_reading(
        "field",
        "soil",
        Reading::new("soil_moisture", value, "percent")
            .with_band(40.0, 80.0)
            .with_status(if value < 40.0 { Status::Warn } else { Status::Ok }),
    );
    for command in worker.take_commands() { /* drive real hardware, then report the result */ }
    // wait for the sensor's duty cycle
}

// 3. Serve it (from another thread or task; `run` blocks).
Server::new(fleet, Assets::Embedded).with_pairing_secret(secret).run("0.0.0.0:80").unwrap();
```

The builder *declares* what exists (and a starting reading); `report_reading` *feeds* live
values and grows the history automatically. That split is why the dashboard works with any
project - it never reaches into your sensors; you push what you have.

A complete, runnable version is in [`examples/gateway.rs`](examples/gateway.rs) (driven by a
real `pamoja-profile` controller, with discovery and persistence). Run it:

```
cargo run -p pamoja-dashboard --example gateway
```

It also shows the gaps a real deployment fills: an added sensor carries an optional hardware
**binding** (`i2c:0x76`, `gpio:4`, `lora:ab12`) for the gateway to bind a driver;
`Fleet::add_sensor`/`add_group` surface a node the moment it is discovered; and
`Fleet::from_state` + `State::from_json` restore a fleet across restarts.

For the other shape - a district console rather than one node -
[`examples/fleet.rs`](examples/fleet.rs) reads every manifest in [`profiles/`](../../profiles)
and stands the whole set up side by side, one group per deployment, with each group's sensors
built from the elements its own profile declares:

```
cargo run -p pamoja-dashboard --example fleet
```

## Custom sensors and node stats (profile-driven)

The page draws a built-in set of sensor types out of the box. When a deployment measures
something beyond that set, the profile declares it - no page change. A `pamoja-profile` `Profile`
carries a `Presentation` of `ElementSpec`s: each names a stable key and unit, the graphic to draw
it with (`Viz` - a gauge, bar, dial, sparkline, switch, valve, and so on), its safe band, a
label, and which groups it is offered on (`Scope`). The gateway turns those into the catalog it
serves:

```rust
use pamoja_dashboard::{Assets, Catalog, ElementSpec, Scope, Server, Viz};
use pamoja_profile::Profile;

// `with_element` adds to what the profile already declares; `with_presentation`
// replaces the lot, which drops the elements a shipped profile came with.
let profile = Profile::well_level().with_element(
    ElementSpec::new("water_turbidity", "ntu", "Turbidity", Viz::Gauge)
        .with_band(0.0, 5.0)
        .on(Scope::Links(vec!["mesh".into()])),
);

Server::new(fleet, Assets::Embedded)
    .with_catalog(Catalog::from_profiles(&[&profile]))
    .run("0.0.0.0:80").unwrap();
```

The page fetches `GET /catalog` on boot and folds the custom presets in beside its own, so the
add-sensor dialog offers them (only on the groups their scope allows) and renders each with the
chosen graphic and label. A live reading can also pin its own graphic -
`Reading::new(...).with_viz(Viz::Gauge)` - so a value flows straight into the instrument the
profile intends. A small `Theme` on the presentation tints the console (accent and status
colors), and `Presentation::with_message` localizes any custom state or event code the profile
emits, so the page shows words instead of raw codes.

**Only the sensors a device can bind.** The hardware-free demo lets you add any sensor type (it
fills with placeholder data); a real device should not. Call `Fleet::allow_sensors([...])` and a
client add of anything outside that set is refused with a clear "device does not support that
sensor" message instead of a tile that never reports. Gateway discovery - `Fleet::add_sensor`,
how a real node surfaces a sensor it actually found - stays unrestricted.

A complete, runnable version is in [`examples/gateway.rs`](examples/gateway.rs). For the full
profile authoring guide - presets, JSON manifests, control policies, presentation, theme, and
messages, from the simplest use to a fully themed catalog - see the
[`pamoja-profile`](../pamoja-profile/README.md) crate.

## Authenticated control

Reading is anonymous; moving an actuator or changing the fleet is not, because the hotspot is
open. The device shows a pairing secret out of band (its screen, a QR, or the dev console);
the browser mixes it with a server nonce into a session key (HKDF) and signs every command
with a counter and an HMAC, so an on-network attacker can neither forge a command nor replay
a captured one, and the secret never crosses the network. The keyed-hash primitives are
reused from `pamoja-session`; the browser ships a tiny pure-JS SHA-256/HMAC because WebCrypto
is unavailable over plain `http://`.

## Localization

Translations live once, as one JSON file per locale under
[`web/app/i18n/`](web/app/i18n/) - the single source the browser fetches and renders with its
own CLDR-backed `Intl` (plurals, numbering systems, right-to-left). There is no generation
step and nothing to keep in sync by hand. `cargo xtask dashboard i18n` validates the bundles
(key, placeholder, and metadata parity; gzipped footprint). A constrained build embeds only
the locales it needs (see [Capability tiers](#capability-tiers)).

## Performance

The device serves its assets gzip-encoded, and `cargo xtask dashboard footprint` enforces a
gzipped page-load budget per tier (the full tiers well under 150 KB including one locale; the
Tier C floor page a few KB against a 50 KB ceiling), so the bundle stays small over a weak
link. Add `--tier <a|b|c>` to check one tier. First paint needs no round trip after the
initial load.

## Capability tiers

One design serves hardware from a city gateway down to a microcontroller, chosen with a
compile-time tier feature. The `GET /state` contract is identical across all of them, so a
page written for one tier reads another tier's data.

| Tier | Feature | What ships | Budget |
| --- | --- | --- | --- |
| A | `tier-a` (default) | the full localized app: hand-built visuals, every seed locale, history, authenticated control | ~150 KB gzipped page load, including one locale |
| B | `tier-b` | the same full app, but only the locales a deployment selects, to fit constrained flash | same page load; smaller flash image |
| C | `tier-c` | a single self-contained **floor page** for the smallest hardware | ~50 KB gzipped page load |

The floor page renders the status table with the smallest possible script; when scripting is
off entirely it falls back to `GET /lite`, a server-rendered, meta-refreshing table with no
script at all. It is plain, but it is legible and it works on any browser. The full app links
to it from the top bar's **Lite** control (and the floor links back), so a viewer on a weak
phone can drop to it; the floor page is embedded in every tier for that reason.

Build a non-default tier with `--no-default-features`. Tier B selects its locales with the
`locale-*` features (English is always embedded as the fallback):

```
# the floor page only
cargo build -p pamoja-dashboard --no-default-features --features "serve,tier-c"
# the full app, English + Swahili only, to fit flash
cargo build -p pamoja-dashboard --no-default-features --features "serve,tier-b,locale-sw"
```

The page asks the device which languages it embedded (`GET /locales`) and offers only those,
so a dropped locale never appears in the switcher. A static host with no device keeps the full
built-in list. The embedded image size is guarded by a flash budget in the crate's tests.

## Embedded targets (ESP32)

There are two ways onto a microcontroller, and this is honest about where each stands today.

**std on ESP32 (works today).** The ESP-IDF Rust toolchain gives the chip a real `std` with
`TcpListener` and threads (over lwIP and pthreads), so `Server` runs unchanged - implement the
`Transport` seam over the board's TCP stack, or use the plain-TCP default. Build a constrained
image with the tier and locale features, for example the floor page only, or the full app with
just the languages that fit flash:

```
cargo build -p pamoja-dashboard --no-default-features --features "serve,tier-c"
cargo build -p pamoja-dashboard --no-default-features --features "serve,tier-b,locale-sw"
```

The `serve` feature pulls `flate2` (gzip, pure-Rust `miniz_oxide` backend) and `getrandom`
(the pairing-secret RNG); both have ESP-IDF support, so this path needs no extra glue. The
per-tier footprint and the flash-budget test keep the image small, and the server is
thread-per-connection, which is ample for the handful of clients a node sees over its hotspot.

**Bare-metal no_std (not yet, but the surface is ready).** The crate is `std` today - the
threaded `Server` and `serde_json`'s default `std`. The portable surface is already isolated,
though: the data contract (`state`, `source`, `command`) and the floor renderer (`lite`) use no
`std::` paths (only `alloc`'s `String`/`Vec`), and `Assets::Embedded` is just `include_bytes!`
slices. A bare-metal port reuses those and the request/response writing, driven from the
device's own single-threaded socket loop rather than `Server::run_on` (which spawns a thread
per connection). Reaching it means turning on `serde_json`'s `alloc` feature, supplying a
no_std `getrandom` source, and building `flate2`/`miniz_oxide` in no_std+alloc mode.

## Build modes: real vs demo

The crate is feature-gated so a real firmware build ships no demo:

| Build | Features | What you get |
| --- | --- | --- |
| Real device | `--no-default-features --features "serve,<locales>"` | `Fleet`, `Server`, control - no mock fleet, no scenario switcher. Add the `locale-*` features (or `all-locales`) for the languages to embed; English alone if none. |
| Development / showcase | default (`serve`, `mock`, `tier-a`, `all-locales`) | `Mock` + the dev server + the static snapshot generator, every locale |

Snapshots from the mock carry a `demo` flag; the page shows demo-only affordances (the
scenario switcher) only when it is set, so a real device never exposes them.

## Web app

The page is a multi-file [zQuery](../../) app under [`web/`](web/): `index.html`,
`global.css`, the vendored `zquery.min.js`, and `app/` (entry, store, router, the live feed,
i18n, the pairing/crypto helpers, the visualizations, and the components). In development
`Assets::Dir` serves it from disk with hot reload; in production `Assets::Embedded` bakes it
into the binary with `include_bytes!`. The floor is a separate single-file page,
[`web/lite.html`](web/lite.html), with its styles and script inline; it is embedded in every
tier and reachable from the top bar's Lite control.

## Commands

- `cargo xtask dashboard dev [scenario]` - run the mock-backed dev server (hot reload).
- `cargo run -p pamoja-dashboard --example gateway` - run the real-`Fleet` reference gateway.
- `cargo run -p pamoja-dashboard --example fleet` - run every shipped profile on one console.
- `cargo xtask dashboard i18n --check` - validate the locale bundles.
- `cargo xtask dashboard footprint [--tier a|b|c]` - check each tier's gzipped page-load budget.
- `cargo xtask docs` - regenerate the crate READMEs and the workspace API index under `docs/`.

## API reference

The canonical per-item API is on docs.rs once the crate is published. `cargo xtask docs`
regenerates each crate's README from its rustdoc, plus a workspace API index that links every
crate's docs, under [`docs/`](../../docs/README.md).
