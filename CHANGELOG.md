# Changelog

Notable changes to pamoja, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Every crate, the npm,
PyPI, and NuGet packages, and the language bindings share one version and are
released together, so one entry covers all of them.

## [Unreleased]

### Added

- LoRa link budgets in `pamoja_lora::budget`, in every language: the EIRP an
  antenna and cable leave, the free-space loss of ITU-R P.525-5, the first Fresnel
  zone of ITU-R P.526-16, receiver sensitivity from the thermal noise floor of
  Semtech AN1200.22 and the demodulator SNR of the SX1261/2 datasheet, the margin
  a path leaves, the transmit power a regional EIRP ceiling allows behind an
  antenna, and the conducted power limit and antenna gain rule of 47 CFR 15.247.
  The math is integer and `no_std`, held to a hundredth of a decibel, and its
  tests are anchored to the ITU equations, the FCC text, and the SX1261/2 and
  SX1276 sensitivity tables. The LoRa guide gains a second example that works out
  how far a reading reaches from a European node.
- A Radios and antennas page on the site: concentrators, antenna gain and patterns,
  ground planes, SMA, RP-SMA and U.FL connectors with pigtail loss, LMR feed line
  loss, VSWR and testing with a network analyzer, a paired field test, Fresnel
  clearance, lightning bonding under ITU-T K.71, weatherproofing, and the power
  limits of ETSI EN 300 220-2, CEPT ERC Recommendation 70-03 and 47 CFR 15.247,
  with the nine LoRaWAN regional plans pamoja carries from RP002-1.0.5. The
  hardware page gains the LLCC68, SX1302, SX1303 and SX1250, the RAK2287, RAK5146
  and WM1302 concentrator cards, and a new group for gateway antennas, pigtails
  and lightning arrestors, each figure quoted from its manufacturer's document.
  The parts are priced beyond the RAKwireless store where others stock them:
  SparkFun for the gateway antennas, Rokland for the concentrators and the
  arrestor, The Pi Hut for an LLCC68 unit, Pimoroni and Adafruit for a u.FL to SMA
  cable, and L-com for a gas tube arrestor.
- LoRa radios in `pamoja-radios`, starting with the Semtech SX126x family: the
  SX1261, SX1262 and SX1268, and the LLCC68 that shares their commands. Every
  command the chip takes is built to its bytes and every answer decoded, from the
  IRQ and status bytes to the RSSI and SNR of a received frame, with the frequency
  word, timeout steps, image calibration and amplifier tables of the datasheet.
  The crate picks the amplifier setting a regional EIRP ceiling allows behind an
  antenna, and a duty-cycle guard holds the radio silent for the off time each
  frame owes. In Rust, an `embedded-hal` driver brings the chip up, transmits and
  receives, and `MeshRadio` carries `pamoja-mesh` frames over it as a pamoja
  transport that relays what it hears. The command set, the decoders and the
  guard reach TypeScript, Python and C# as `@pamoja/radios`, `pamoja-radios` and
  `Pamoja.Radios`, checked against shared conformance vectors, and a new guide
  plans one transmission in all four languages. The SX1262 and LLCC68 cards on
  the hardware page name the driver.
- The Semtech SX1276, SX1277, SX1278 and SX1279 in `pamoja-radios`, the family inside
  most RFM95W modules. The register map, the carrier word, the modem settings a link
  turns into, the RFO and PA_BOOST amplifier settings with the +20 dBm mode and a
  current limit to match, the IRQ flags, and the packet RSSI and SNR with each RF
  port's offset come from the SX1276/77/78/79 datasheet. The receiver's image
  calibration runs at the first carrier on each port, and the 500 kHz sensitivity and
  spurious reception errata are applied as Semtech's LoRaMac-node applies them. The IQ
  polarity bit of the transmit path follows LoRaMac-node, RadioLib and arduino-LoRa
  rather than the datasheet, which describes it the wrong way round. An `embedded-hal`
  driver brings the chip up in LoRa mode, transmits, and receives in single or
  continuous mode, and `MeshRadio` runs over it as it does over an SX1262. The register
  values and decoders reach TypeScript, Python and C#, checked against new conformance
  vectors, and the SX1276 card on the hardware page names the driver.
- An LLCC68 check on the SX126x driver: `Board::with_llcc68` holds a link to the rates
  the LLCC68 supports, up to SF9 at 125 kHz, SF10 at 250 kHz and SF11 at 500 kHz, from
  its datasheet and Semtech's LLCC68 driver, and every binding can ask the same
  question.

### Changed

- The version badges on the front page are drawn on the same sheet as the
  buttons beside them, which they had not been: they were the old palette at a
  different height, generated in another repository. `cargo xtask docs` writes
  them now, in the site's own inks, as a datasheet prints a rated value, with the
  parameter on the tint and the version beside it. Each carries both color
  schemes, so a reader on a dark registry page gets the dark sheet.
- The front page reads on a phone. The buttons were glued together with
  non-breaking spaces, which stopped the row wrapping where it should and left
  ragged gaps down a narrow screen, and there were nine of them where three are
  ways in and six were links the page already carries further down. It keeps the
  three, at the height every other README asks for.
- The architecture drawing is on the architecture page, which is where the walk
  through it is, rather than also in the middle of the front page.
- The front page says what a node is: the profile as a file, the four pieces that
  sit around it, and the two programs that put the whole thing together.
- The light sheet is printed on warm coated stock. The paper, its tint, the rule,
  and the caution wash moved toward tan, and the tint now stands apart from the
  paper where it had nearly vanished into it. The site, the four reference
  generators, the badges, and the dashboard take the new values from the same
  palette.
- A group card on the dashboard's two sheets is drawn with a hairline rule rather
  than a heavy ink frame, and the reading tiles inside it with their tint alone,
  so a card no longer reads as a window inside a window. Stat cards drop their
  accent stripe and their hover shadow there too. A card or tile in warning or
  alarm still takes its status color, and the panels that float over the page
  keep their frame.
- `cargo xtask docs` writes the dashboard's copies of the palette, in its
  stylesheet and in the two pages that render without scripts, from the site's
  palette, and `--check` fails when a copy drifts.
- The capability map on the front page carries the four bindings in one strip
  across its top, over the engine, a cell for each heading, and the dashboard. Every heading
  lists its capabilities, each linked to its guide, and the grid holds four,
  three, two, or one column, so no row is left short. The engine cell names
  `pamoja-core` alone. `docs/capabilities.toml` now says which engine crate is
  the C ABI and which is the dashboard, so the C# binding names the first and
  the dashboard has a cell of its own. The language bindings section is gone
  with that, and Direction and Backing are sections 6 and 7.
- Direction draws each track as a lane: what ships today on the tint, a line at
  today, and what is committed next and later beyond it, with a count of each.
  Every committed item is listed, so Robotics and drones shows fleet and swarm
  orchestration next, and mission planning, numeric inverse kinematics for
  longer arms, and micro-ROS later, and the hardware lane names its parts. Reach
  points at the bindings rather than listing the four languages again.

### Fixed

- Two examples still taught that `with_presentation` adds to a shipped preset.
  Since 0.1.18 every preset carries a presentation, so the call on
  `Profile::well_level()` replaced the well-level bar and the battery stat, and
  the asserts passed only because the originals were gone. Both examples now say
  the call replaces, check the element count, and point at `with_element` for
  adding. One of them generates the crates.io page for `pamoja-profile`.
- The Python reference can be navigated on a phone. A height meant for the
  desktop sidebar left a tall empty band above the page, and the site bar
  covered the menu button. The height applies on the desktop only, and the
  button sits below the bar.
- The Python reference no longer opens on a page carrying only the package name.
  The `pamoja` namespace has no documentation or members of its own, so its page
  lists the submodules with the first line of each one's documentation.
- A dashboard link with `?theme=`, `?locale=`, or `?scenario=` opens on that
  view. The dev server had always said it would, but nothing read those
  parameters. The link changes what is shown, not the saved choice, and a locale
  the build lacks falls back to the saved one. The dev server no longer offers
  `?tier=`, since the tier is a build feature.
- The remove button on a dashboard tile takes a tap across 44 pixels instead of
  20, and a text field shows the focus outline again when reached from the
  keyboard.
- The sheets no longer glow. The alarm count, a gauge bar, and a newly seen link
  each drew a fixed glow that ignored the theme, and they now take the theme's
  own. Faint text on a tile on the light sheet reaches a 4.9:1 contrast.
- The ink frame around the capability map showed only on its right and bottom
  edges, because the hairlines between cells were drawn over the other two. The
  cells sit on a hairline ground now, so the frame holds on all four sides.
- Direction's tags printed at 10px on a phone, from a size token that was never
  defined. They print at 11px, the smallest label size the sheet uses.
- Links in the capability map, the application scenes, the bindings strip, and
  Direction are underlined at rest. Color alone had told them apart from the
  text beside them, at 1.06:1.

## [0.1.18] - 2026-09-10

What a node does between reading and reporting, and what a console makes of a
fleet of them. Rules between nodes are a file: one node's reading crossing a
line drives another node's actuator, with a release band so it fires once per
crossing. A policy of your own runs in a profile's node over whatever a driver
reads and whatever an actuator takes, and a manifest may name a control kind the
library never shipped. Every shipped profile now says how it should be drawn, so
a dashboard built from the eight of them draws each with its own graphics, safe
bands, and French and Swahili labels. A message reads as text or a number and a
link sends text, so a reading written out as words needs no encoding step. Two
new examples show the two shapes a deployment takes, one node done properly and
a district console running all eight profiles at once, and the Raspberry Pi and
ESP32-C3 pages go past a single sensor read into what a GPIO line can drive,
which pins are safe to use, and the whole loop on real hardware.

### Added

- Rules between nodes, as a file. A rule watches one node's topic for a reading
  crossing a line, with a release band so it fires once per crossing, and the
  moment it does drives an actuator held by name or publishes a message over the
  same link; `then` runs as the condition sets and `otherwise` as it clears.
  `pamoja-profile` gains `Rule`, `Condition`, `Action`, `Rules` (a JSON file with
  `from_json` and `to_json`), and `RuleEngine`, which subscribes to each rule's
  topic, decodes readings in the nodes' codec, and runs the file off any link
  that can receive. The condition is `pamoja_kit::Trigger`, a threshold with
  hysteresis that reports its `Edge`, bound in every language with a conformance
  vector. A rules guide runs the same file in four languages: the engine in
  Rust, and the trigger with the program's own loop in the others.
- A message reads as text or a number, and a link sends text. `Message::text`
  and `Message::number` in the core, `send_text` on every `Transport`, and in
  the bindings `text` and `number` on every received message and a `send` (or
  `publish`) that takes a string beside bytes, so a reading or a command written
  out as words needs no encoding step in a program.
- A policy of your own runs in a profile's node. `Policy` in `pamoja-profile` is
  the decision half of a node, generic over the reading it decides on and the
  command it issues, and `Controller` implements it over an `f32` and a `bool`;
  `Node::with_policy` runs any policy in the same read, decide, act, publish
  loop, so a node can read a driver's whole measurement and command an actuator
  that takes more than on and off, and `Reaction` is generic over the command.
  `Alert` gains `Custom { code, value }` for a condition a policy names itself.
  A manifest may name a control kind the library never shipped, with its
  parameters beside it as every built-in kind carries its own: `ControlSpec`
  gains `Custom { kind, params }` with `Params` of numbers, flags, and text, and
  `PolicyRegistry` resolves the four built-in kinds to a controller and a
  custom kind to the factory registered under its name. The futures the core
  `Sensor`, `Actuator`, `Device`, and `Telemetry` traits return are `Send`, as a
  transport's are, so a node built from any sensor and actuator can be driven
  from a spawned task; a driver generic over a bus says so with a `Send` bound.
  Every binding reads a custom kind as its name and parameters and a custom alert
  as its code and value, over `pamoja_profile_control_kind` and
  `pamoja_profile_control_params_json` in the C ABI, and the conformance vectors
  carry a manifest with a kind the library never shipped.
- Every shipped profile says how it should be drawn. All eight manifests in
  `profiles/` now carry a presentation naming their own elements, each with its
  graphic, its safe band, and French and Swahili labels beside the English: the
  fridge draws a thermometer, a cooler switch, and a compressor-duty stat; the
  irrigation node a soil droplet and a drip valve; the well a level bar and a
  battery stat; the flood sensor a wide river wave and a rainfall bar. Naming the
  element also fixes what a bare threshold could not say, since the flood
  sensor's limit of 0.3 is meters and a reading in millimeters would have tripped
  it every sample; the dashboard learns the two unit symbols that go with that,
  and the brooder's heat lamp stops reporting itself as "Online".
- Builders for adding to a profile rather than replacing what it declares.
  `Profile::with_element` and `Profile::with_message` add one element or one
  state's wording, `Presentation::with_elements` declares several at once, and
  `LocalizedText::per_locale` (or an array of locale and text pairs, which
  converts into one) writes text in several languages without building a map.
  `Viz::name` and `Viz::from_name` are the word a manifest writes, which differs
  from the renderer's own token for three of the graphics.
- A node hands back the parts it was assembled from. `Node::sensor_mut`,
  `actuator_mut`, and `transport_mut` reach the components a running node holds,
  which is how a node publishing through a transport ladder flushes its own
  buffer once the link is back, and `Node::into_parts` takes the whole thing
  apart. `Reading::from_element` in `pamoja-dashboard` turns a profile's
  declaration into the first reading a gateway reports.
- Two examples for the two shapes a deployment takes.
  `examples/brooder_node.rs` is one small node end to end: a shared manifest read
  off disk, a probe, a heat lamp on an active-low relay, an uplink that comes and
  goes so the cold hours are held and drained in order, a vent fan at the far end
  of a rule file, and the sampling interval stretching as the battery sags.
  `pamoja-dashboard`'s `fleet` example is the district console: every manifest in
  `profiles/` stood up side by side, one group per deployment, each group's
  sensors built from the elements its own profile declares.
- The board pages go past a single sensor read. The Raspberry Pi package gains a
  program that drives a relay board on one GPIO line and reads a debounced limit
  switch on another, and one that runs the whole profile-driven loop on real
  hardware and publishes over MQTT; its page gains what a pin can actually drive,
  why every GPIO reverts to an input at power-on, the groups behind almost every
  permission error, how to set an input's pull at boot, and how to run a node
  under systemd without root. The ESP32-C3 package gains a thermostat that
  decides on the chip, with a relay and a debounced override button; its page
  gains the pin table that matters, since GPIO11 to GPIO17 are the SPI flash and
  GPIO2, GPIO8, and GPIO9 are strapping pins, GPIO9 being the boot-mode strap.
- Every shipped manifest is proved to run, not only to parse. A test in
  `pamoja-profile` loads each one with the parser a device uses, feeds it
  readings that cross its own lines, and checks the output switches both ways,
  the alerts its policy promises fire, and the sampling interval stretches as the
  battery drains.

### Changed

- `ControlSpec` is no longer `Copy`, since a custom kind carries its name and
  parameters; clone it or borrow it.
- `cargo xtask profiles` writes a preset's manifest from the constructor that
  builds it, so the code stays the single source for the four profiles the crate
  also builds in Rust and a hand edit to those files cannot drift away from it.

- A path for what people build. `profiles/` holds one JSON manifest per shared
  profile, the four presets among them, read by the parser a device uses and
  checked by `cargo xtask profiles` for what a hand-written manifest gets wrong:
  the name and file agree, a description is present, the topic is one publishable
  path, a policy can act, the schedule slows as the battery drains, and every
  dashboard element has a key, a unit, a label, a band with its low end first,
  and a starting state the page has words for. The task rewrites each into the
  form the library writes, and CI holds them there. A profiles page catalogs
  them with what each does in words and the line that downloads it, and a
  community page lays out the path for a profile, an example, a driver (the
  datasheet audit checklist every shipped driver went through), and a board,
  with an issue form for each. `examples/community/` is the place for shared
  programs, listed on the examples page and run in CI. A profile gains a
  `description`, carried in the manifest and by the presets, with
  `with_description` in Rust and a getter in every binding. The dashboard
  presentation a profile carries is typed in every binding: `Presentation`,
  `ElementSpec`, `Theme`, and the `Viz` graphics in TypeScript, Python, and C#,
  read from `presentation` and set with `withPresentation`, `with_presentation`,
  and `WithPresentation`, over `pamoja_profile_presentation_json` and
  `pamoja_profile_with_presentation_json` in the C ABI; the profile guide
  declares one element in all four languages.
- A page per board, each ending in a program that compiles in CI. The
  Raspberry Pi page wires a BME280 to the header, names the kernel settings
  that turn the buses on, and runs the driver over the kernel's device files;
  the ESP32 page works through the ESP32-C3 on `esp-hal`, and the RP2040 page
  through the Pico on `rp2040-hal`, each with the board's own pins, toolchain,
  and flash step from the maker's documentation. The three programs live as
  standalone packages under `examples/boards`, outside the workspace, and CI
  builds each for its target on every change. A buses page explains I2C, SPI,
  1-Wire, UART, PWM, ADC, RS-485 with Modbus, CAN, and LoRa in the terms the
  guides use, what to get right on each, and which crate carries the logic,
  citing the specification or reference manual for every figure. The hardware
  reference gains a card for the Raspberry Pi Pico, and every board card links
  its page.
- The dashboard takes a sensor it has never seen and a number it can set. The
  add-sensor picker offers a custom entry beside the presets: a name, a key, a
  unit, a graphic, and a safe band, and the sensor is created with them, labeled
  by the name it was given, which travels in the reading as `label` so every
  client and the no-script page show it. A reading gains `range`, the numeric
  span a device lets a client command, and a `set` command carries a value
  inside it; the page shows a slider and a `Set` button for such a reading, the
  fleet applies and queues the command for the gateway, and the mock holds the
  value across ticks. The demo fleet's own elements, its field-kit sensors and
  its map positions, come from the mock's catalog, served by the dev server and
  carried in the static showcase's first frame, so the page ships only the
  physical quantities and node stats any deployment may add; the graphic
  heuristic keys on units and generic key patterns rather than the demo's names,
  and a label no bundle or catalog carries reads as its key with the underscores
  taken out. The catalog gains `from_presentations` and `with_site_position`,
  and the demo farm node gains a pump speed to set.
- A link written in the host language is a transport. The C ABI takes a table of
  callbacks and a `user_data` pointer through `pamoja_transport_from_callbacks`,
  runs each callback off the runtime's blocking pool, asks an optional `recv`
  callback for messages from a background thread and queues them so a receive
  stays cancel-safe, and calls `release` only after the last callback in flight
  has returned; `pamoja_message_new` builds what `recv` delivers and
  `pamoja_last_error_set` attaches a reason to a status. On that, .NET gains
  `Transport.FromHandlers` over `ITransportHandlers` and
  `IReceivingTransportHandlers`; Node gains `Transport.fromHandlers` over an
  object whose methods are called on it, so a class instance works as it is, each
  method plain or returning a promise; Python gains `Transport.from_handlers` over
  an object whose methods are plain or coroutine functions, awaited on the loop
  the ladder was driven from, with `recv` returning a `Message`, a
  `(topic, payload)` pair, or `None`. A host transport without a receive is an
  uplink: every ladder adds it as one and never listens on it. The Python
  `Message` gains a constructor. A guide, `docs/guides/link.md`, writes a link
  over two queues in each language and runs it as a ladder rung.
- The inbound half of a link, `Receive`, beside `Transport` in the core, with one
  `Message` type for what any link delivers. MQTT, CoAP, Zenoh, and the loopback
  transport implement it, the fault injector and the degraded link pass it
  through, and its contract names the cancel safety a ladder and a `select!`
  depend on. A link that only sends, a LoRa uplink or a satellite messenger,
  implements `Transport` alone, as before.
- The transport ladder is a link in its own right: it implements `Transport` and
  `Receive`, so a profile node, a rule, or any loop written against one link runs
  over a ladder unchanged and gains its buffering. A subscription placed on the
  ladder goes onto every rung that listens and is kept, so a rung that is down
  when it is placed receives it when it next connects; a receive polls every
  listening rung together, starting from a different one each call, and hands
  up whichever delivers first. The ladder now tracks which rungs are up: a rung
  that reports itself closed, on a send or a receive, is left out until the next
  connect, and connect no longer reconnects rungs that are already up. A
  send-only link is added with `uplink` rather than `rung` and is never listened
  on. The ladder in every binding gains `subscribe` and `recv`, and the
  composable transport handle in the C ABI and .NET gains a receive.
- A bus layer, `pamoja-hal`: the `embedded-hal` 1.0 I2C, SPI, GPIO, and delay
  traits every driver is written against, re-exported in one place; a bit-banged
  1-Wire bus over any pin with the reset and presence handshake, the ROM
  commands, and the search that enumerates a bus; scripted I2C and SPI buses, a
  scripted pin, and a recording delay, so a driver is tested against its
  datasheet's own transfer sequence with nothing plugged in; and, behind the
  `linux` feature, the kernel's i2c-dev, spidev, and GPIO character devices
  opened as those same traits.
- Drivers that reach the parts: every sensor and actuator module gains a type
  named after its part that owns a bus, performs the datasheet's transfer
  sequence with the datasheet's timings, and implements the core `Sensor` or
  `Actuator` trait. The BME280 and BMP280 run over I2C or SPI through a shared
  register-bus abstraction; the SHT3x, SCD4x, TMP117, HDC1080, OPT3001, INA219,
  INA226, and ADS1115 over I2C; the DS18B20 over any 1-Wire bus; the PCA9685 over
  I2C; and a four-wire stepper or a step/direction driver chip over output pins.
  Each is tested against its datasheet's own sequence on a scripted bus,
  including the identity check, the timeout, and the bus fault. The decode
  layers gain what the drivers needed: the BME280 control registers,
  measurement-time formula, and register builders; the BMP280 measurement
  times; the DS18B20 function commands, the parser for the Linux kernel's
  `w1_slave` text, and a thermometer that reads the kernel's file as a
  `Sensor`; the ADS1115 addresses, conversion time, and sample type; the INA219
  configuration register and reading type; reading types for the INA226,
  TMP117, and OPT3001; the TMP117 alert flags; and the PCA9685 MODE2 bits,
  software reset, and oscillator start-up time.
- A guide for the bus layer, `docs/guides/hal.md`: a BME280 read through its
  driver over a scripted bus in Rust, and the same datasheet conversation driven
  from a bus shaped like the host I2C library in TypeScript, Python, and C#,
  with pamoja compensating the burst.
- `Sensor::map` and `Actuator::map_command` in the core, with the `Map` and
  `MapCommand` adapters, so a driver's multi-channel reading feeds a controller
  that wants one number and a part's own command shape is driven by a `bool` or
  a percentage.
- `Switch` and `Contact` in `pamoja-gpio`: a relay, valve, or lamp driven as a
  core `Actuator`, and a button, float switch, or motion detector read as a core
  `Sensor`, each carrying its polarity in the type.

- The seven new sensor drivers reach Python. Each is a module-level object beside
  the four already there, with the datasheet constants, the frame parsers and
  their builders, the configuration registers, and every conversion in both
  directions, documented in the style the other parts use. The smoke suite checks
  a datasheet figure per part and the input each one refuses, and the conformance
  suite asserts the same vectors Rust and Node do.
- The seven new sensor drivers reach TypeScript and Node. Each is a package-level
  object beside the four that were already there, with the datasheet constants,
  the frame parsers and their builders, the configuration registers, and every
  conversion in both directions. The smoke suite checks a datasheet figure per
  part and the input each one refuses, and the conformance runner asserts the
  same vectors Rust does, so a part that decodes differently in one language
  fails the build.
- The seven new sensor drivers have cross-language conformance vectors: the
  generator writes 94 values for them, from the decoded frames and their physical
  readings to the register tables the datasheets print, each with the input the
  vectors mark as bad, and the Rust runner asserts every one. The three bindings
  assert the same file, so a part that decodes differently in one language fails
  the build rather than the reader.
- Seven more sensor drivers, taking `pamoja-sensors` from four parts to eleven: the
  BMP280 pressure sensor, the SHT3x and HDC1080 humidity and temperature sensors, the
  SCD4x carbon dioxide sensor, the TMP117 thermometer, the OPT3001 ambient light
  sensor, and the INA226 power monitor. Each is the same decode-and-configure layer
  the existing four are, `no_std` and allocation-free, and each was written from the
  manufacturer's datasheet and then checked back against it: every constant located in
  the document, every formula recomputed independently, and the register layouts read
  field by field. The BMP280 pressure path is a port of Bosch's own integer
  compensation, and the INA226 reproduces its datasheet's worked example exactly. One
  finding went the other way: the SCD4x datasheet prints the wrong checksum byte for
  the first word of its example frame, which the driver's CRC, anchored to that same
  datasheet's published check value, does not reproduce.

### Fixed

- The store's text accessors named `String` without importing it, so
  `pamoja-core` did not build for a target with no operating system. Only the
  bare-metal build would have caught it, and it did.
- British spellings had spread through the routing and mesh doc comments, the
  update crate's trust prose, the dashboard's reading keys and locale labels, and
  the package READMEs that reach npm, PyPI, and NuGet. The dashboard's mesh stats
  rename with them: the reading keys `neighbours` and `neighbour_mesh` are
  `neighbors` and `neighbor_mesh`, and every locale file renames the key while
  keeping its own translated words.
- The Python type stub for `Message` had not caught up with the payload that
  takes text as well as bytes.
- The hardware page's purchase offers were never actually sorted cheapest first.
  The sort ran, but on the array alone: a TOML table is written back where its
  recorded position says, not where the array now puts it, so the file kept the
  order it already had and the test that covered this only ever inspected the
  array. Six parts listed a dearer offer above a cheaper one once the currencies
  were converted. The positions travel with the sort now, and the test reads the
  rendered document.
- PyPI refused the compiled engine's source distribution: its metadata named
  `LICENSE-MIT`, which maturin had placed beside the vendored crate rather than
  at the archive's root once the package carried path dependencies, and PyPI
  checks that a named license file is in the archive. The package declares the
  file explicitly, which puts it at the root, and the upload pass no longer
  stops at a file PyPI refuses: it prints PyPI's full answer, carries on with
  the rest, and fails at the end naming what it could not place.

### Changed

- Receiving is the `Receive` trait's `recv` rather than a method each transport
  defined for itself, so a caller brings `pamoja_core::Receive` into scope the
  way it already does `Transport`. The per-crate `Message` types of the MQTT,
  CoAP, Zenoh, and loopback transports are the core `Message`, re-exported under
  their old paths; a Zenoh sample's key expression is its `topic` field, where it
  was `key`. The `Store` trait's futures are `Send`, as `Transport`'s already
  were, which a store written as `async fn` over `Send` state already satisfies.
- The architecture drawing opens over the page at full size when clicked, the
  phone layout on a phone, closed by the button, a click outside it, or Escape,
  rather than leaving the page for the file; on a wide screen it also runs a
  little past the text column. The site's background is a faint grain over the
  color washes rather than a grid of dots.

## [0.1.17] - 2026-09-06

The website, rebuilt. The documentation site and the front page are rendered by
`cargo xtask site` from the same Markdown and the capability map, in one design:
reference pages that open the generated API pages for each language, a hardware
page of spec-sheet cards that says where to buy each part at prices read on a
schedule, an examples page, a reference hub, an architecture drawing, stamped and
minified assets, and navigation that swaps pages without a reload. In the code,
eight capability crates' error types implement `core::error::Error`, so `?` into
a boxed error compiles on the first try, and the .NET packages carry an icon.

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
  not parse fails the build rather than reaching a browser. The dashboard demo
  at `/dashboard` is minified the same way as it is copied in, by
  `cargo xtask minify <dir>`.
- The hardware page is a set of cards rather than tables and bullet lists. A
  card breaks a part down into labeled facts (interface, each figure from its
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
- An examples page lists every complete program under `examples/` with what
  it shows and the line that runs it, and every guide's example by chapter with
  what it proves and the four files that run it in CI, each a link to the file.
  A reference hub introduces the four generated references and how they are
  made; "Reference" in the header, the front page, and the menus leads there.
  Both are rendered from the code and the guides, so they cannot drift.
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
  buttons take their colors from the same palette as the site's theme.

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
  holding a manifest, a README and a license. They installed without complaint
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
