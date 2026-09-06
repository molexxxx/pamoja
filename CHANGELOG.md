# Changelog

Notable changes to pamoja, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Every crate, the npm,
PyPI, and NuGet packages, and the language bindings share one version and are
released together, so one entry covers all of them.

## [Unreleased]

### Fixed

- Eight capability crates gave their error type a `Display` implementation and
  no `Error` one, so `fn main() -> Result<(), Box<dyn Error>>`, which is the
  first thing most people write, failed to compile the moment it touched mesh,
  Modbus, CAN, GPIO, sensors, serial, session, or LoRaWAN. Every one implements
  `core::error::Error` now, which needs no `std` and so reaches a caller on a
  microcontroller too, and a test carries an error from each of them through `?`
  into one boxed error.
- The .NET packages declared no icon, so all thirty-eight rendered as a blank
  placeholder in the gallery and in Visual Studio.
- The PyPI upload goes in dependency order, the compiled engine first, and stops
  at the first refusal to create a project rather than retrying into the cap; a
  scheduled workflow finishes the set as the cap allows, so a release never
  waits on it and never meets it once every project exists.
- The site's bar over the generated references was drawn but not visible on the
  Python pages, since pdoc's stylesheet fixes every bare `nav` element to the
  viewport as its sidebar. The bar is built from elements no generator styles.
- The hardware page linked the `Sensor` and `Actuator` traits at a rustdoc path
  that does not exist.
- The Rust reference on the site was weeks stale: the build cache kept the last
  run's site tree, and copying the fresh rustdoc output into a directory that
  already existed nested it under the old pages, which then shipped again. The
  site tree starts empty on every build.

### Changed

- The install page says what happens on a platform the compiled engine was not
  built for. The .NET packages restore and compile and then fail on the first
  call, since there is no native library and, unlike Python, no source build to
  fall back on; Alpine and Windows on ARM are the two that catch people out.
- The documentation site is rendered by `cargo xtask site` rather than mdBook: the
  same Markdown pages, with a guide's four languages as tabs that remember the
  choice, search, syntax highlighting done when the site is built, and a link
  check that fails the build on a broken link or anchor, the generated references
  included. The reference pages list each capability with its install line, its
  module, its worked example, and the same capability on the other three
  registries.
- The reference page for each language opens its generated API pages. A button
  at the top browses the whole reference (the umbrella crate's rustdoc, which
  lists every crate beside it, typedoc's package list, pdoc's package page, and
  the root namespace in DocFX), and every capability row carries a button for its
  API pages, its guide, its worked example, and its registry page, with the same
  capability on the other three reference pages one step away. The generated
  landing pages that duplicated the reference page are gone, and each generated
  tree carries the site's bar. A guide opens on its reader's language before
  first paint, and scrollbars everywhere on the site follow the palette.
- Every link in a committed page is absolute, so the reference and install
  tables render and resolve on GitHub as well as on the site; the site's link
  check follows them like its own. A guide's reference section and a crate's
  README link the capability's row on each language's reference page, which is
  where the install line and the registry are.
- The hardware page says where to buy each part: two or three product pages
  per part from the makers' own stores and the larger distributors, the
  cheapest reputable option first, each with the price the page listed on the
  day it was read, and the lowest price in the summary table. The pages are
  read by hand, so the date is part of the record.
- The reference rows, the domain rows, and the front page's capability cards
  share one card anatomy: what the thing is and its name in code, the install
  line beside it, and under a hairline one row of equal buttons for its API
  pages, guide, worked example, and registry. A card's drawer continues its
  border without a seam, the front page's install lines and buttons sit on one
  grid at every width, and every page has the menu on a phone, the front page
  included. The header over the generated references is the site's own header.
- The site's stylesheets and scripts are published minified: the sources under
  `web/` stay readable, and the copies the site serves carry no comments and no
  indentation. A script goes through a real parser on the way, so one that does
  not parse fails the build rather than reaching a browser.
- The hardware page is a set of cards rather than tables and bullet lists. A
  card breaks a part down into labelled facts (interface, each figure from its
  document, and its price band with the lowest listed price), says where to buy
  it with the price each page listed, and keeps that apart from what to read
  and build with: the datasheet, specification, or documentation it was written
  from, the driver's source, its crates, and the guides that use it. Each guide
  links the parts its crates drive.
- The search results fit a phone: on a narrow screen they open as a panel under
  the header rather than a dropdown that ran off the left edge. The search box
  shows the slash key that focuses it, a chosen result closes the panel, and the
  shortcut ignores a slash typed with a modifier or into a field.
- Every stylesheet and script a page names carries a stamp of its contents in
  its address, and so do the hooks the four generated references load, so a
  deploy never leaves a browser on a cached copy of the last one; a page the
  router swaps in replaces a stylesheet whose stamp changed.
- A hardware card is a spec sheet: the facts run down one column behind a label
  gutter, and the foot sets where to buy the part beside what to read and build
  with, each a panel of rows of one shape (what it is and a detail on the left,
  the price or the way out on the right), single-column where there is nothing
  to buy. The cards share one padding with the reference rows.
- `cargo xtask prices` reads every product page the hardware page lists, takes
  the price the page states as Schema.org product data, writes it back with the
  day it was read, and orders each part's offers cheapest first; a page that
  states no price that way, or refuses a scripted reader, keeps its last record
  and is named in the report. A weekly workflow runs it and opens a pull
  request with what moved.
- Every hardware card's foot has two panels. A part with offers keeps "Where
  to buy"; a bus lists the parts on the page that speak it, each a jump to its
  card; a protocol, a specification, or a part no store lists gets "Find parts",
  searches at Adafruit, SparkFun, Digi-Key, and Mouser for its name, under the
  note that says no reputable store lists it and since when. No card shows a
  lone panel stretched across the foot.
- The front page is rendered by `cargo xtask site` from the capability map and
  `web/home.toml`, in the same shell as the documentation: the four install
  lines, the first example in four languages spliced from the tests that run it,
  every capability as a card with its four package pages and its guide, nine
  scenarios played by the consoles, the four languages, the roadmap, and the
  backing preview. The Three.js showcase, its data file, and the font host are
  gone; the typefaces are served from the site.
- Moving between pages of the site no longer reloads the document. A link to
  another page fetches it, swaps the article, sidebar, and page metadata in
  place under a short cross-fade, and pushes the address; the back button
  restores the scroll position and a hovered link is fetched ahead of the click.
  Every page is still a complete document with a canonical address and an Open
  Graph card, and the site publishes a sitemap. The header links GitHub, bug
  reports, feature requests, and releases as icons in place of the dashboard
  link, and the front page's hero, first example, capability cards, and backing
  preview were reworked.
- The architecture page opens with a drawing of how a call reaches a crate: the
  three bindings over the compiled engine, a Rust program straight to the
  crates, and every capability by chapter, each box naming its crates, the ones
  whose manifests build on `pamoja-core` in amber over the core itself, and the
  package that installs the chapter on npm, PyPI, NuGet, and as a feature of
  the `pamoja` crate. It is rendered from the capability map and the manifests
  by `cargo xtask docs`, in a wide layout and one for a phone, so it names every
  chapter and crate the map does and is checked like the tables. The link
  buttons take their colours from the same palette as the site's theme.

## [0.1.16] - 2026-09-05

Publishing fixes. 0.1.15 reached crates.io and NuGet intact, and both are
unchanged here; its npm packages carried no code and its PyPI upload stopped a
tenth of the way through. Anyone who installed 0.1.15 from npm should move to
this version, and the 0.1.15 npm packages are deprecated.

### Fixed

- The npm packages published for 0.1.15 contained no JavaScript. Each facade
  names `dist/index.js` as its entry and lists `dist/` in `files`; `dist/` is
  built by `tsc` and is not in the repository, and the publish job went from
  `npm install` to `npm publish` without building it. npm omits a `files` entry
  that is not on disk rather than failing, so all thirty tarballs shipped
  holding a manifest, a README and a licence. They installed without complaint
  and threw `MODULE_NOT_FOUND` on the first `require`. The publish job builds the
  facades now, and a check that runs before publishing and on every pull request
  confirms every package carries the files its `main` and `types` name.
- The PyPI upload for 0.1.15 stopped after four of thirty-eight distributions.
  PyPI caps how many new projects an account may create in a window, which a
  release introducing a project per capability was always going to reach, but
  uploading the whole directory in one call stops at the first refusal and
  stranded the thirty-four files behind it, including a project that already
  existed and was never capped. Each file uploads on its own now, and a refusal
  is treated as a wait rather than a failure: the upload retries what was
  refused until the cap refills, the way the crates.io publisher already handles
  the new-crate limit.

## [0.1.15] - 2026-09-05

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
- A preflight every release workflow waits on. A publish cannot be withdrawn, so
  before anything reaches a registry it checks that the tree carries the version
  being tagged, that the commit is on main, and that `ci`, `node`, `python`, and
  `dotnet` all completed successfully on that exact commit. Each release
  workflow also takes a version by hand, so a run that stalled can be resumed
  without inventing a tag.
- A GitHub release for each tag, carrying the changelog's entry for the version
  followed by the pull requests that went into it, grouped by label. Labels come
  from the files a pull request touches.
- A documentation site at [pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/):
  the guides rendered by mdBook and a generated reference for each language
  (rustdoc, typedoc, pdoc, DocFX), built on every pull request and published
  with the showcase. `docs/capabilities.toml` is the one map of what each
  capability covers in every language, checked against the code.
- A `pamoja` crate that bundles every capability behind a feature each, all on
  by default, so `cargo add pamoja` is the whole framework the way
  `npm install pamoja`, `pip install pamoja`, and `dotnet add package Pamoja`
  are. `pamoja::mqtt` is `pamoja-mqtt`; with the default features off, naming
  only the `no_std` capabilities builds for bare metal.
- Guide examples that run as tests in all four languages, spliced into the
  documentation from the test files, and the Python facade's doctests now run
  with its test suite. Each is a program somebody would actually
  write, end to end, rather than a set of assertions: it builds its own fixtures
  from the library, prints what it learned, and keeps its checks below the region
  the page shows. A guide's own wire bytes live in the crate's tests or in the
  generated conformance vectors, so no page asks a reader to decode a constant.
- Reading a value the library can produce, wherever it could only produce one:
  a DS18B20 scratchpad and an INA219 register set can be built as well as
  decoded, Modbus can build the replies it could already parse, a PCA9685 setting
  and a J1939 payload can be read back out, and an identity signs and verifies a
  message without a caller splitting the signature off by hand.
- `cargo xtask builds` measures what each named feature set of the `pamoja`
  crate compiles, resolved for a fixed target so the counts are the same on
  every machine. The install page carries the table, regenerated and
  drift-checked with the rest of the generated documentation.
- Domains: the six chapters of the guides that hold more than one capability are
  installable as a unit in every language. In Rust each is a feature on the
  `pamoja` crate, so it decides what compiles. In the bindings each is a package
  (`@pamoja/field-io`, `pamoja-field-io`, `Pamoja.FieldIo`) that brings in its
  capabilities and, where the language allows it, re-exports each under its own
  name; a name two capabilities share stays reachable and unambiguous, which a
  flat re-export could not manage. Every domain is checked against the
  capability map, so a capability cannot fall out of its own domain.

### Changed

- `pamoja-ffi` exposes every capability behind a default-on feature and now
  depends on the whole workspace.
- Verifying an audit chain reports why it failed in every language, not only in
  Rust. The three bindings collapsed the engine's reason to a bare true or
  false, so a caller could tell that a log had been altered but not which record
  broke it or whether the log had instead been shortened. They now raise the
  reason, the way every other fallible call in them already does.
- A J1939 payload is a value with named signals rather than eight bytes to
  slice, in every language: `Signals` starts filled with the byte the standard
  reserves for a signal a controller is not reporting, the priorities and the
  broadcast address are named, and a broadcast identifier has its own
  constructor. The mesh header length and the two I2C address ranges the
  specification reserves are exported from the bindings as well, since both were
  known to the engine and to no caller.
- The Node binding is split into packages the way the crates are. `pamoja` is
  the whole framework in one package; each capability is its own `@pamoja/<name>`
  for installing only what you use; `@pamoja/core` is the engine's surface, the
  counterpart of `pamoja-core`; and `@pamoja/native` is the compiled engine and
  generated contract every package depends on. The `@pamoja/core/<name>` subpath
  imports are gone: `@pamoja/core/mqtt` is now `@pamoja/mqtt`, and
  `@pamoja/core/raw` is `@pamoja/native`.
- The Python binding is split the same way. `pamoja` is the whole framework in
  one distribution; each capability is `pamoja-<name>`, one module of the
  `pamoja` namespace; `pamoja-core` is the engine's surface (`pamoja.core`); and
  `pamoja-native` is the compiled engine, `pamoja._native`, that every
  distribution depends on. `pamoja` is a namespace package now, so the flat
  `from pamoja import DeviceIdentity` becomes `from pamoja.security import
  DeviceIdentity`, and `pamoja.transport` is `pamoja.core`.
- The .NET binding is split the same way. `Pamoja` is the whole framework in
  one package; each capability is `Pamoja.<Name>`, a package and a namespace of
  the same name; `Pamoja.Core` is the engine's surface; and `Pamoja.Native` is
  the compiled engine and the P/Invoke contract (`Pamoja.Native.Interop`) that
  every package depends on. Types keep their names but move namespaces
  (`Pamoja.Core.MqttClient` is `Pamoja.Mqtt.MqttClient`), and the transport
  factories move next to their clients: `Transport.Mqtt(options)` is
  `MqttTransport.Open(options)` and `Transport.Coap(options)` is
  `CoapTransport.Open(options)`.

### Fixed

- Two blocks of constants were missing or broken in the generated C header.
  `cbindgen` does not read the crates `pamoja-ffi` depends on, so a constant
  defined as another crate's constant was emitted as a bare identifier declared
  nowhere in the header, and the three PCA9685 values were dropped from it
  entirely. Each now carries its value with a compile-time assertion tying it to
  the crate that defines it.
- The capability tables in the install page and every binding README are grouped
  by chapter, so thirty rows read as a handful of domains.
- `Pamoja.Core` was two things at once, the engine's surface and the marshalling
  every facade needs, so all twenty-nine capability packages depended on it. The
  handle type, the error type, the status helpers, and string marshalling move to
  `Pamoja.Native`, where the rest of the P/Invoke contract already lives, and
  `PamojaException` sits in the root `Pamoja` namespace so a facade sees it
  without a using and a consumer catches it with `using Pamoja;`. Only the five
  transport packages depend on `Pamoja.Core` now, matching the Node and Python
  bindings, where a capability package depends on the engine alone.
- The Node facades exported enum constants a TypeScript caller could not pass to
  the facade's own functions. `PinLevel`, `PinEdge`, `PinPolarity`, `StepDrive`,
  `LinkCost`, and `EntityKind` held plain strings, which are not assignable to
  the `const enum` the generated contract takes, so every call needed a cast. The
  smoke suites are JavaScript and never saw it. The constants carry the contract
  type now, and `@pamoja/ros2` exports the contract's `EntityKind` type rather
  than one derived from its own object.
- The Node, Python, and .NET workflows all named their job "build and smoke
  test", so a pull request showed three identical checks, none of which could be
  required and none of which said which binding had failed. Each names its
  language now.
- The MQTT guide proved only that an unreachable broker is refused, which is the
  one thing a reader does not need shown. It runs a real round trip now: a
  gateway subscribes to a wildcard, a node publishes under it, and the reading
  arrives with its topic. The Rust example starts an in-process broker, and the
  three binding workflows start one, which `just broker` also starts locally.
- The gateway pairing code no longer appears in a captured dashboard log (#67).
- Broken intra-doc links in the rustdoc of nine crates, which docs.rs rendered
  as dead links; `cargo doc` now runs with warnings denied.
- `pamoja-lora` with its `std` feature on did not compile outside its own test
  build, because the crate stayed `no_std` regardless; it now links `std` when
  the feature is on, and the `pamoja` crate's default build exercises it.
- The install page described choosing packages as if it shrank a binding's
  download. It does not: each binding loads one engine carrying every
  capability, so the choice narrows the API and the dependency manifest. The
  page now says which of the two applies per language and measures the Rust
  claim, and `pamoja-ffi` documents the feature sets that do shrink the library
  for a C or C++ host that builds it.

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
