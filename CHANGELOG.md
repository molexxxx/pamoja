# Changelog

Notable changes to pamoja, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Every crate, the npm,
PyPI, and NuGet packages, and the language bindings share one version and are
released together, so one entry covers all of them.

## [Unreleased]

### Added

- One I2C bus that a program and every driver on it share, in every language.
  `pamoja_hal::bus::I2cBus` in Rust (the `std` feature) is a handle that clones into each
  driver, over the kernel's adapter (`I2cBus::open`, the `linux` feature), simulated
  parts that answer from their registers, or a script that plays one conversation and
  refuses any other. `delay()` gives a driver a delay that sleeps only for real parts
  and counts every wait, `part` copies what a simulated part holds after a driver wrote
  to it, and `attach` swaps a part in underneath a running driver. A Raspberry Pi's
  controllers report a part that did not acknowledge as `EREMOTEIO`, which the bus reads
  as a missing acknowledge. TypeScript, Python, and C# get it as `I2cBus`, `I2cPart`,
  and `I2cStep` in new `hal` packages: `@pamoja/hal`, `pamoja-hal` (module
  `pamoja.hal`), and `Pamoja.Hal`, where before the buses capability had no package of
  its own in those languages.
- The BME280 driver in TypeScript, Python, and C#, over an `I2cBus`: `Bme280` with
  `init` and `measure`, the part's register map and setting codes, the `ctrl_meas`,
  `ctrl_hum`, and `config` bits both ways, the datasheet's measurement times, and a
  simulated part holding a real part's calibration, or reporting any reading it is
  asked for. In C#, `Bme280` is now a class whose static members are the datasheet and
  whose instances are drivers.
- Every other I2C sensor driver in TypeScript, Python, and C#, over an `I2cBus`: the
  BMP280, TMP117, OPT3001, HDC1080, INA219, INA226, ADS1115, SHT3x, and SCD4x, each with
  its settings, `init`, `measure` (`sample` and `sample_input` on the ADS1115), and what
  else its datasheet gives it: the TMP117's alert limits, the OPT3001's interrupt window,
  the HDC1080's heater, the SHT3x's status register and heater, the SCD4x's data-ready
  poll, temperature offset, altitude and single shot, and the INA226's alert and identity.
  The C ABI carries each driver as a handle. The DS18B20 is read the way a Linux board
  reads it, through the kernel's 1-Wire files, as `Ds18b20Thermometer`: it finds every
  probe the kernel lists, names each by its serial, and reads it.
- A simulated part for every I2C sensor, `sim::part` and `sim::reporting` beside each
  driver, answering as its datasheet says, in every language. `pamoja-hal` gains the two
  shapes of part they need: `WordPart`, sixteen-bit registers behind a pointer byte, which
  is how the TI parts talk, with bits the part keeps for itself marked read-only; and
  `CommandPart`, commands each answered by the reply it leaves, which is how the Sensirion
  parts talk. `sim::Part` holds any of the three and a simulated bus takes any mix.
  TypeScript, Python, and C# get `WordPart`, `CommandPart`, and `SimulatedPart` for any
  of them.
- `ds18b20::w1_slave_text`, the text the kernel's `w1_therm` driver serves for a
  scratchpad, so a program that reads the kernel's files is tested with neither a probe
  nor a kernel; `ds18b20::linux::Thermometer::serial`, the serial the kernel named a
  probe's directory after; `ina219::address`, from the A1 and A0 pins as
  `ina226::address` already was; and `ads1115::Sample::clipped`, true when a conversion
  sits at an end code of the datasheet's Table 7-3 and its voltage is a bound rather than
  a reading. All four reach every language.
- The INA226's averaging, conversion times, and mode by name in TypeScript, Python, and
  C#, as the INA219's settings already were: `ina226.averaging`, `ina226.conversionTime`,
  and `ina226.mode`; `Ina226Averaging`, `Ina226ConversionTime`, and `Ina226Mode`; and
  `Ina226.Averaging`, `Ina226.ConversionTime`, and `Ina226.Mode`, with an `Ina226Config`
  record in C# that the driver takes and `ConfigToRegister` and `UpdateMicros` accept.
  Before, each was a bare register code.
- The sensor drivers guide rewritten around a greenhouse bench, nine parts on one bus and
  a DS18B20 in a pot, printing the same twelve lines in all four languages, with a
  Raspberry Pi logger of an SHT31 and DS18B20 probes in each. Its tables cover where each
  part answers, the sixteen INA219 and INA226 addresses, what a measurement waits, every
  setting and its choices in each language, the ADS1115's ranges, calibrating a current
  monitor, the simulated parts, and what each error means.
- The PCA9685 driver in TypeScript, Python, and C#, over an `I2cBus`: `Pca9685` with its
  frequency, oscillator, and output wiring as settings, and `init`, `set_channel`,
  `set_all`, `sleep`, `wake`, and `software_reset`, loading the four register bytes the
  `pwm` builders make. The C ABI carries it as a handle. Its registers, MODE1 bits, and
  power-on values are named in every language, so a program reads the part back by name.
- A simulated PCA9685, `pca9685::sim::part` and its twin in every language, holding the
  power-on registers of the datasheet's Table 4 and keeping the rules its register
  descriptions set out: PRE_SCALE takes a write only while the oscillator sleeps and never
  loads less than 3, the register pointer moves on only with auto-increment set, one write
  to the ALL_LED registers loads every channel while they read back zero, RESTART clears on
  a written 1 and sets when the part sleeps with a channel running, and EXTCLK stays set. A
  driver that writes the prescale awake leaves the part at 200 Hz, as the part would.
- One serial port that a program and every driver on it share, in every language.
  `pamoja_hal::port::SerialPort` in Rust (the `std` feature) is a handle that clones into
  each holder, over the kernel's serial device (`SerialPort::open`, the `linux` feature),
  opened raw at a speed, a parity, and a stop bit count, with a read that waits in `poll`
  up to its timeout and a write that returns once the bytes have left the UART; a line
  looped back on itself; the two ends of a null-modem pair; a simulated device behind the
  `Peer` trait; or a script. A read anywhere but a real device counts its timeout in
  `waited_micros` without sleeping. `Settings` gives the bits a character, the time one
  takes, and a run's time on the wire. TypeScript, Python, and C# get `SerialPort`,
  `SerialStep`, `Parity`, and the settings in their `hal` packages, with `write` and `read`
  on a worker thread in Node.
- The serial framing guide rewritten around a weather mast whose node sends COBS frames to a
  gateway on a paired port, printing the same ten lines in all four languages, with a
  Raspberry Pi UART self-test on one jumper wire in each. Its tables cover the two framings
  and what they cost, line formats and their time on the wire, the kinds of port, the
  settings in each language, which UART each Raspberry Pi model puts on its header, and what
  each error means.
- A Modbus client and the devices it polls, in every language. `pamoja_modbus::Client`
  (the `port` feature) runs each transaction over a serial port to the Modbus over Serial
  Line specification: it leaves the line silent for 3.5 characters, or 1.75 ms above
  19200 baud, drops stale input, reads the reply to the length the request implies against
  a response timeout, one second unless set, and checks the reply's CRC, unit, and function
  before a value is read out of it. A broadcast write waits out a turnaround, 100 ms unless
  set, instead of a reply, and a refusal comes back as `ClientError::Exception` with the
  device's exception. `Server` (the `alloc` feature) is a device and the four tables it
  serves, answering each of the eight functions, refusing in the order the specification's
  state diagrams check, and staying silent on a frame that fails its CRC, one for another
  unit, and a broadcast, which it still carries out. `Line` puts several on the far end of a
  simulated port, and `Request` reads a request as a device does. TypeScript, Python, and
  C# get `ModbusClient`, `ModbusServer`, and `ModbusLine`, and an error that names what
  failed, with the device's exception code: `ModbusClientError` in TypeScript and Python,
  `ModbusClientException` in C#.
- `Pdu` builds the reply a device sends to each read and to each write of many values, and
  the exception it sends instead, beside the requests; `Exception` prints the name the
  application protocol specification gives it.
- The Modbus guide rewritten around the gateway at a village water pump, polling an energy
  meter and a relay module on one simulated line and printing the same nine lines in all
  four languages, with a Raspberry Pi program in each that scans a real line through a USB
  RS485 adapter. Its tables cover the four data tables and how a manual numbers them, the
  eight functions and their limits, unit addresses, the timing at each speed, the client's
  two waits, the exceptions in the order a device checks them, the client's settings in
  each language, wiring a line, and what each error means.
- A node on a CAN bus, in every language. `pamoja_can::bus::CanBus` (the `bus` feature) is a
  node on a bus simulated inside the program, or with the `linux` feature, a SocketCAN socket
  on a kernel interface such as `can0`, carrying classic, extended, remote, and CAN FD
  frames in the kernel's own layouts. `join` puts another node on the same bus; a node hears
  every frame the others send and none of its own, as a SocketCAN socket does, and keeps the
  ones its filters pass: `Filter::exact`, `Filter::pgn` for one J1939 parameter group from
  any source, or any identifier and mask, with the frame format always part of the match. A
  receive on a simulated bus with nothing waiting returns at once and counts its timeout.
  CI runs the SocketCAN path against the kernel's virtual CAN interface. TypeScript,
  Python, and C# get `CanBus` and `CanFilter` in their `can` packages, with `send` and
  `receive` on a worker thread in Node.
- The CAN guide rewritten around a standby generator's J1939 bus, an engine controller, a
  gateway that filters for engine speed, a service laptop, and a sensor speaking plain CAN,
  printing the same ten lines in all four languages, with a Raspberry Pi monitor in each that
  listens to a real bus through an MCP2515. Its tables cover the kinds of frame, the data
  length code, the fields inside a J1939 identifier, filters, what a SocketCAN socket does by
  default, the kinds of bus, the calls in each language, and what each error means.
- A `Transport` in TypeScript and Python is driven directly, as in C#: `connect`, `send`,
  `subscribe`, and `recv`, so a link from `Transport.mqtt`, `Transport.coap`, or a
  broker's `rung()` works without a ladder around it. A transport with a call running
  refuses to be handed to a ladder or a wrapper, with `this transport is busy with a call`.
- A receive with a time limit in TypeScript and C#, which gives up without taking the next
  message: `recv(timeoutMs)` on `Transport`, `LoopbackTransport`, `Ladder`, `MqttClient`,
  and `CoapClient`, and `ReceiveAsync(limit)` in C#, `RecvAsync(limit)` on `MqttClient`. A
  receive raced against a timer kept running after the race and took the next message
  itself. The C ABI gains `pamoja_transport_recv_within` and its counterparts for the
  loopback link, the ladder, and the MQTT and CoAP clients. Rust and Python already stop
  waiting without a loss, through `tokio::time::timeout` and `asyncio.wait_for`. A negative
  limit throws `a time limit must be 0 ms or more` in TypeScript, where it would otherwise
  reach the native side as a wait of about 49 days.
- An MQTT packet limit: `MqttConfig::max_packet_size`, `maxPacketSize`, `max_packet_size`,
  and `MaxPacketSize`, 10,240 bytes each way unless set, as before. A send whose packet would
  be larger is refused before anything leaves, with the size it would have been.
- The transport, loopback, and MQTT guides give each language its own account of the calls,
  tables of the calls, settings, delivery guarantees, and topic rules, and what each error
  means, citing the sections of MQTT 3.1.1 behind them. The loopback example proves a
  reading did not arrive with a receive that runs out of time, and the MQTT example has a
  node's two-day backlog refused for its size while the node stays connected, each printing
  the same lines in all four languages. The standards register adds MQTT 3.1.1, anchored to
  its own filter examples.
- A CoAP server, the gateway end of a CoAP link, in every language. `pamoja_coap::CoapServer`
  implements the transport traits: its `subscribe` names the paths it takes readings on, a PUT or
  POST to one of them is answered 2.04 Changed and delivered to `recv`, and one to any other path
  4.04 Not Found. Its `send` sets a resource's state, which a GET reads and every observer is
  notified of, as RFC 7641 describes. It answers a retransmitted confirmable request again
  without taking it twice for the 247 seconds of RFC 7252's exchange lifetime, answers a ping
  with a Reset, drops an observer that resets a notification, and counts a resource's
  observers. `CoapPublisher` sets states from another task while a receive waits, so in
  TypeScript, Python, and C# the server's `send` runs beside a waiting `recv`.
- The CoAP guide rewritten around an orchard: a gateway takes moisture readings from the rows
  and holds the irrigation valve, which a row observes and sees open, printing the same ten
  lines in all four languages. Its tables cover the client's settings, the calls on each end,
  the two ways to send, the retransmission schedule, what the server answers, and what each
  error means.
- A publish-only handle on the event bus, in every language. `pamoja_bus::EventPublisher`
  makes a bus with no subscribers yet, or comes from any endpoint's `publisher()`. It has no
  buffer to fill, it clones into every task and callback that announces, and its `publish`
  returns at once with how many subscribers it reached. TypeScript, Python, and C# get it as
  `EventPublisher`, and publishing never waits in any of them, so a callback on another
  thread publishes without an event loop.
- An event bus endpoint counts the events it lost by falling behind: `missed()` in Rust,
  `missed` in TypeScript and Python, and `Missed` in C#. TypeScript and C# get a wait with a
  time limit, `next(timeoutMs)` and `nextText(timeoutMs)`, and `NextAsync(limit)` and
  `NextTextAsync(limit)`, which gives up without taking the next event. The C ABI gains
  `pamoja_event_bus_publisher`, `pamoja_event_bus_next_within`, `pamoja_event_bus_missed`,
  and the `pamoja_event_publisher_*` calls.
- The event bus guide rewritten around a solar weather station: a power monitor and a wind
  sampler announce to a heater, a logger, and a radio that joins late, printing the same
  eight lines in all four languages. Its tables cover the handles and the calls in each
  language, what an endpoint and a publisher each do, how far an endpoint can fall behind
  for a given capacity, and what each error means.
- A link written in TypeScript, Python, or C# may hand over a message whose payload is text.
  A TypeScript `recv` resolves with `{ topic, payload }`, the payload a `Buffer` or a string,
  typed as `DeliveredMessage`; a Python `recv` may return a `(topic, payload)` pair with a text
  payload; and C# gains `new TransportMessage(topic, text)`. Python gains `TransportHandlers`
  and `ReceivingTransportHandlers` in `pamoja.core`, the shape of a link for a type checker.
- The own-link guide rewritten around a moored buoy with a cellular modem and a satellite
  messenger that pamoja has never heard of, on one ladder, printing the same eight lines in
  all four languages: a reading out, a command back, a refused send passed to the satellite,
  a lost session reported, and a reconnect. Its tables cover the contract in each language
  and what a ladder does with what a link does.
- A ladder takes any link as an uplink in TypeScript, Python, and C#, as `uplink` does in
  Rust: `ladder.uplink(transport)` and `ladder.Uplink(transport)`, and
  `pamoja_ladder_uplink` in the C ABI. Before, only a link written without a receive went on
  as one.
- A loopback broker can be taken out of reach and brought back: `set_reachable` and
  `is_reachable` in Rust, `reachable` in TypeScript and Python, `Reachable` in C#, and the
  matching C ABI calls. Every link on it, links a ladder owns included, fails to connect,
  send, or subscribe with `transport error: the broker is out of reach` until it is back, so
  a test takes a network away from a node without reaching into the node.
- A file store can be bounded, as a memory store could: `FileStore::open_with_capacity(dir,
  n)` in Rust, `Store.file(dir, n)` in TypeScript and Python, `Store.File(dir, n)` in C#, and a
  capacity on `pamoja_store_file`. An append that would take it past the bound is refused with
  `io error: store is at capacity`, which keeps a long outage from filling an SD card.
- A store drains onto a transport in TypeScript and Python, as it did in Rust and C#:
  `store.drainTo(transport, topic)` and `store.drain_to(transport, topic)` send every record
  to one topic, oldest first, removing each only once the transport has taken it.
- The simulators guide rewritten around a vineyard rover with nothing built: a replayed range
  finder that ends the loop when it runs out, a drive that records its commands, a pose from
  the kinematics, a seeded soil probe read twice to the same values, and a radio that loses
  every third report, printing the same nine lines in all four languages, where the old
  example printed its numbers differently in each. Its tables cover what each simulator
  stands in for, the noisy sensor's settings, the degraded link's patterns, how the robot
  moves, and what each error means.
- The store-and-forward guide rewritten around a hive scale in a remote apiary: weights queue
  on a bounded file store with no link, survive a reboot, stay in order through a drain that
  loses its uplink part-way, and reach the beekeeper's gateway later, printing the same seven
  lines in all four languages. Its tables cover the two stores, the calls in each language,
  how the file store keeps a record through a power cut, and what each error means.
- The transport ladder guide rewritten around a fishing vessel whose reports go ashore over
  harbor wifi, the coast's cellular network, or a satellite as each falls out of reach, a
  backlog held through a storm, and orders from shore, printing the same nine lines in all
  four languages. Its tables cover the calls in each language, what a send does in each
  state, the stores a ladder buffers into, and what each error means.
- The codec's packers build without the standard library. `pamoja-codec` is `no_std`, and
  with default features off the `Codec` trait, `BytesCodec`, `encode_deltas`,
  `decode_deltas`, and `Quantizer` cross-compile for a Cortex-M4F in CI, so a node packs its
  own batch before it transmits. The CBOR and JSON codecs still use the standard library.
- The codecs guide rewritten around a snow gauge on a ridge that has to fit what it reports
  into the 11 bytes a US915 uplink carries at the slowest data rate: one reading as JSON and
  as CBOR, neither of which fits, then six hours of depths through the quantizer and battery
  voltages through the integer packer, each fitting one uplink, and a batch with a missing
  depth refused, printing the same nine lines in all four languages. Its tables cover the
  calls and the types each language hands in, the three codecs, how a batch is laid out,
  what a step and a CBOR value cost, how to choose a scale, and what each error means.
- The device identity guide gains the signed message, the form a reading usually takes on a
  link, and a message cut short that is refused whole, printing the same seven lines in all
  four languages. Its tables cover the calls on the device and on the gateway in each
  language, the sizes, and what each check says when it fails.
- The windowed helpers keep as many readings as they are told, in every language. In Rust,
  `with_capacity` keeps fewer than the const generic `N`. In the other languages the
  constructor takes a capacity, such as `new Median(5)`, from 1 to 32, or from 2 for a trend
  or an anomaly baseline, which need two readings to answer, and refuses one it cannot keep:
  `capacity must be a whole number from 1 to 32, not 0`. Each reports its `capacity`, a
  window reports whether it is full and its latest and oldest readings, and C# gains
  `Kit.WindowCapacity`.
- A trigger reports its threshold and hysteresis in C# and the C ABI, as it did in the other
  languages, and whether it watches a rising reading in every language.
- The helpers guide rewritten around a village water system: the tower level read off a 4-20
  mA loop, filtered and smoothed, the tower's lean from an accelerometer steadied by a gyro,
  a refill pump and its float switch, a low-water alarm, a booster pump held at pressure by a
  PID behind a soft start, the dew point in its pump house, a countdown through a power cut,
  a leak, a burst main, a flow meter's odd readings, and a tanker truck's district, printing
  the same twenty-three lines in all four languages. Its tables cover every helper in each
  language, what each parameter means and what a value out of range does, the windowed
  capacities, what a reading that is not a number does to each helper, and how to tune them.
- The robot motion helpers in TypeScript, Python, and C#, where they were in Rust alone: the
  differential, skid-steer, Ackermann, and mecanum chassis models, the quadrature decoder
  and its scale, odometry, the waypoint follower and the obstacle stop, the e-stop,
  watchdog, limits, and safety gate, the two-link arm and Denavit-Hartenberg forward
  kinematics, and the servo and ESC pulse maps, each through the C ABI. A twist, a pose, and
  a joint's parameters are plain objects in TypeScript and value types in Python and C#.
  TypeScript and Python refuse a pulse width outside 0 to 65535, and TypeScript a step count
  with a fraction: `minUs must be a whole number of microseconds from 0 to 65535, not -1`.
  `ServoMap` and `Esc` report the pulse widths and travel they were built with, in Rust too.
- The complementary filter, the tilt from an accelerometer, the dew point, and the twelve
  unit conversions in TypeScript, Python, and C#, where they were in Rust alone. In C# the
  conversions are static methods on `Units`, and the tilt and dew point on `Kit`.
- The robot motion guide, around a rover that inspects a solar farm at night: the same turn
  on four chassis, its wheel encoders and odometry, the drive to an inverter cabinet, the
  safety gate every command passes through, and the arm that presses the cabinet's reset
  button, printing the same twenty-one lines in all four languages, with a Raspberry Pi
  program in each that drives the arm's two servos from a PCA9685. Its tables cover the
  frames and units every helper agrees on, the calls in each language, what each parameter
  means and what a value out of range does, what a reading that is not a number does, and
  what each error means.
- Conformance vectors for the motion helpers, the complementary filter, the tilt, the dew
  point, and the unit conversions, checked in all four languages.
- The standards register lists REP-103, the units and axes every twist, pose, and chassis
  model uses, pinned to the odometry test that drives a quarter circle to the left.
- A profile built from its parts, in every language. Rust gains
  `Profile::new(name, topic, control, power)`. TypeScript gains
  `new Profile(name, topic, control, power)`, with the control a plain object and the
  power a `PowerScheduleSettings` whose thresholds default. Python gains
  `Profile(name, topic, control, power)`, `ControlPolicy(kind, ...)` with keyword fields,
  and `PowerScheduleSpec(active, saver, critical)` with default thresholds. C# gains
  `new Profile(name, topic, control, power)`, and its `ControlPolicy` and `PowerSchedule`
  records default what a kind does not use. Custom kinds cross too, through the C ABI's
  `pamoja_profile_new` and `pamoja_profile_new_custom`. Before, a profile outside Rust
  came only from a preset or a manifest, and Rust had no constructor.
- `ControlSpec::custom` builds a custom control that survives a trip through its
  manifest, refusing an empty kind, a built-in kind's name, which a manifest would read
  back as that kind, and a parameter named `kind`, which the manifest keeps for the kind.
- The device guide gains the maker's own profile in every language, built from its parts,
  printing the same eleven lines in all four. Rust runs the parts under a `Node`, and the
  other languages drive the profile's controller by hand. Its tables cover what makes a
  part in each language, the pieces the loop uses, a profile's parts and the calls that
  build one, what a reaction says, and what each error means.
- Conformance vectors for profiles built from their parts, a setpoint one and a custom
  one, pinning the manifest bytes in all four languages.
- The audit log guide breaks the log every way a log gets broken, a record edited, the
  first left out, two swapped, another device's key, then resumes it after a restart and
  shows the one change a chain cannot see, records cut from its end, and how an auditor
  catches that against the device's last reported index, printing the same nine lines in
  all four languages. Its tables cover what a record holds, what each check catches, the
  calls in each language, and what each failure says.
- The secured session guide sends a replayed frame, a frame with its pump id rewritten, a
  frame that arrives late, and the gateway's reply, printing the same seven lines in all
  four languages. Its tables cover what crosses the wire, what the session derives and how
  a nonce is built, what the receiver accepts, the calls in each language, and what each
  refusal says, led by the reused salt.
- The telemetry guide walks a node down all four link costs, from its own network to no
  link at all, and ends on the counts by level, printing the same seven lines in all four
  languages, with tables of the levels, the link costs, a snapshot, and the calls in each
  language.
- The stepper drivers in TypeScript, Python, and C#: `FourWire` for four coil lines
  through a ULN2003 or an H-bridge, and `StepDir` for a step and direction chip such as
  the A4988 or the DRV8825, each over any output line, a `GpioLine` on a board or a
  `PinScript` in a test, and walking the same native coil sequence as the Rust drivers.
  They wait on a delay from each language's `hal` package: `SleepDelay`, which sleeps,
  or `DelayLog`, which counts every wait and sleeps through none, as
  `pamoja_hal::script::DelayLog` does in Rust. The pause after a step and the pulse width
  default to the Rust drivers' 2000 and 10 microseconds, named in every language.
- `Pca9685::channel` reads a channel's setting back from the part, one register a
  transfer, so it works whatever MODE1 holds and changes nothing on the part, in every
  language.
- The actuator drivers guide rewritten around a motion-control time-lapse rig: a tilt servo
  and a status LED on a PCA9685, a slider behind an A4988, and a 28BYJ-48 pan head,
  printing the same thirteen lines in all four languages, with a Raspberry Pi pan and tilt
  head in each. Its tables cover the PCA9685's registers, what each frequency comes out
  as, servo pulse counts, a channel's settings, the coil patterns, step and direction
  timing and microstep pins for the A4988 and DRV8825, every setting in each language,
  and what each error means.
- Rules for a simulated part with byte-wide registers: `pamoja_hal::sim::Rules`, three
  plain functions for what a write does, what a read returns, and where the pointer moves,
  given to a part with `I2cPart::following`. `Rules::MEMORY`, every part's default, is
  plain memory. The PCA9685 is the first part built on them.
- The buses guide rewritten around the shared bus in all four languages, each opening
  with how its language hands out the bus and reports a failure, plus the same program
  on a Raspberry Pi, tables of the bus kinds, the BME280's registers, oversampling and
  measurement times, and what each error means and what to check. The Raspberry Pi
  page's sensor read is now in all four languages too.
- The Raspberry Pi page's relay and LoRa radio programs in TypeScript, Python, and C#,
  beside Rust, each with its own run command. They are compiled in CI against the
  packages each language installs: a `boards` script in Node, a test that loads every
  Python board program, and a `Pamoja.Boards` project in the .NET solution. The ESP32,
  RP2040, gateway, and walkthrough pages now say which languages run where and why: a
  microcontroller runs Rust alone, and the gateway runs as its daemon from any language.
- A GPIO line opened on a Linux board, in every language: `pamoja_gpio::linux::output`
  and `input` in Rust (the `linux` feature), `pamoja_gpio_line_open_output` and its
  companions in the C ABI, and `GpioLine` in TypeScript, Python, and C#. It opens through
  the kernel's GPIO character device, drives its initial level from the moment it is
  taken, and names the chip and the line in every error; anywhere but Linux it refuses
  with a message saying so. TypeScript, Python, and C# also gain `Switch`, `Contact`, and
  `PinScript`, the types Rust already had, so a relay or a float switch is written the
  same way in all four and moves from a test to a real pin by changing the line it is
  given. Before, the bindings had the polarity arithmetic but nothing to hold a line, and
  no way to open one on a board.
- `dialect::mav_sys_status_sensor`, the bits of `SYS_STATUS`'s present, enabled and
  health fields. A ground station waits on `PREARM_CHECK` in the health field before it
  arms; ArduPilot and PX4 both set it once every pre-arm check passes.
- A survey of the band from a gateway's SX1261. `pamoja-gateway` takes a
  `spectral_scan` section under `concentrator.sx1261`, beside `listen_before_talk`,
  naming where the survey starts, how many channels it covers 200 kHz apart, how many
  samples each scan takes and how often one runs. The daemon scans one channel at a
  time between its other work and prints how many samples were at or above each of
  thirty-three levels, four decibels apart, the way Semtech's packet forwarder's scan
  thread does: a scan stands aside for a downlink, one that runs two seconds is
  abandoned, and a downlink that arrives mid-scan abandons it first. The schedule is
  `pamoja_gateway::daemon::scan::Sweep`, which decides which channel is next, whether
  a scan is due and whether one has run too long, so it is tested without a radio.
  The radio's wiring and patch moved under `concentrator.sx1261` with the two jobs
  as sections inside it, since both run from the same patch and the reference lays
  them out that way; a radio named with neither job is refused. The gateway page
  covers both.
- The four LoRaWAN application layer packages, and the update block they carry, in
  every language. TypeScript, Python and C# gain the multicast key chain and the
  parity matrix as plain calls, the clock synchronization package, the firmware
  manager, a fragmentation session that puts a block back together in memory it
  sets aside once, and the code taken over that block as it arrives. One flat
  record reads and writes a command of any of the four, so a server built in any
  of the four languages can drive a session end to end. The conformance vectors
  replay a whole broadcast: the key chain from one root key, the parity matrix,
  every command of each package, and a session that loses every fourth fragment
  and still comes back whole.
- A guide, in four languages, that walks a signed release from a publisher to a
  device that only ever heard it broadcast: the group set up, the block cut into
  fragments, a quarter of them lost, the rest solved for, the block checked, and
  the manifest given the last word.
- A way to carry a signed update inside one block, in `pamoja_update::block`, for a
  transport that moves blocks rather than streams: the signed manifest and the
  image behind a short header that says where each begins, under a descriptor a
  fragmented transport can name the convention by. Nothing in the header is
  trusted; an image that arrives over a broadcast nobody authenticated is held to
  exactly the rules one fetched any other way is, which the tests state by
  altering the image, the manifest and the sequence in the block and watching each
  one be refused.
- Remote multicast setup, TS005-2.0.0, in `pamoja_lorawan::packages::multicast`: how
  a group of devices is given one address, one key and a window in which they all
  listen at once, which is what makes sending a firmware image to a thousand
  devices take one broadcast rather than a thousand. The group key travels wrapped
  under a key encryption key derived from the device's own root key and never
  leaves it in the clear, and the group's session keys come from that key and the
  group's address. The group status, setup and delete commands and the Class C and
  Class B session commands travel on port 200. Tests use ChirpStack's own command
  bytes.
- Fragmented data block transport, TS004-2.0.0, in `pamoja_lorawan::packages::fragment`:
  the way a firmware image crosses a link that carries a couple of hundred bytes at a
  time. A block is cut into fragments and followed by coded ones, each the
  exclusive-or of a pseudo-random half of the originals, so a device that missed
  some solves for them from whatever else arrives. `Fragmenter` produces any
  fragment of a session, `Defragmenter` puts a block back together in storage the
  caller provides and never allocates, and the session setup, status, delete and
  acknowledgment commands travel on port 201. The block's integrity code is taken
  over the image a piece at a time, so a device checks one it never holds twice.
  The coded fragments are seeded by the index among the coded fragments, which is
  what Semtech's LoRa Basics Modem and ChirpStack both do and what the two have to
  agree on; the specification's appendix reads as though the whole-session index
  were the seed. Tests use ChirpStack's own command bytes and an independent
  rendering of the matrix.
- The two LoRaWAN application layer packages a firmware update rests on, in
  `pamoja_lorawan::packages`: clock synchronization, TS003-2.0.0 on port 202, and
  firmware management, TS006-1.0.0 on port 203. A device asks what time it is and
  applies the correction the server sends back, with the four-bit token that makes
  a late answer harmless and the two-step correction the specification works out
  for a clock that starts at zero. A server asks what firmware and hardware a
  device runs, what upgrade image it holds and whether that image can be
  installed, deletes one by version, and programs the single reboot a device
  keeps, as a moment in time or as a countdown, each with its cancellation and its
  refusal. Anchored to both specifications and cross-checked against the command
  sizes and field layouts of Semtech's LoRa Basics Modem.
- A LoRaWAN relay that runs from every language. The C ABI gains a relay handle
  (`pamoja_lorawan_relay_new` and its scan, wake, forward and heard calls) and the
  relay mode of an end device, and Node, Python and .NET wrap both: `LorawanRelay`
  in TypeScript, `lorawan.Relay` in Python and `LorawanRelayNode` in C#, each
  scanning, trusting a device, forwarding an uplink on port 226 and passing the
  answer back, alongside `useRelay`, `heardWorAck` and `noWorAck` on the end
  device and the wake-on-radio exchange every relayed transmission carries. The
  network side crosses too: the notices relays report, the commands that
  configure one, and the trust command that hands over a device's key. The
  LoRaWAN guide gains a third example, in four languages, that carries a sensor
  in a cellar through the relay on the roof, and the conformance vectors replay a
  whole relayed exchange so every binding writes the same wake-up frame.
- A network side that speaks to relays, in `pamoja_gateway::network`. An uplink a relay
  forwards on port 226 is read as if the device itself had sent it, with what the relay
  heard of it carried alongside: a join is admitted and its accept goes back through the
  same relay, an answer to a relayed uplink is wrapped for the relay by `answer` with no
  more asked of the caller, and a relay's report of a device it could not verify waits in
  `notices` until it is read. `command` sends a relay the MAC commands that configure it,
  in the frame options where they fit and a frame of their own where they do not, and
  `trust_command` builds the one that tells a relay to trust a device, with the key that
  lets it verify that device's wake-on-radio frames.
- A relay and a device under one that drive real radios. `pamoja_radios::relay::RelayNode`
  gives a `Relay` a radio and a clock: one `scan` sleeps until the next slot, listens for a
  preamble over a couple of symbols, and only if one is there receives the wake-on-radio
  frame, answers it, takes the uplink behind it, forwards that to the network, and sends
  what comes back in the end device's relay window. `Node` sends every uplink of a device in
  relay mode behind the frame that wakes its relay, reads the acknowledgment in its window,
  and listens in the relay window as well as the usual two. Both chips detect activity for
  it: `Sx126x::detect` with the thresholds Semtech's own radio layer uses for each spreading
  factor and bandwidth, and `Sx127x::detect` over the symbol its detection takes. A
  `Transceiver` that cannot detect activity says so by listening every time, and a received
  frame now carries its signal strength, which a relay forwards with the uplink.
- An end device that sends through a LoRaWAN relay, TS011-1.0.1 chapters 3 and 5.
  `EndDevice::use_relay` turns relay mode on, and every uplink then goes out behind a
  wake-on-radio frame that names it: `Transmission::relay` says when the frame goes,
  with how long a preamble, where the relay's acknowledgment would arrive, when the
  uplink itself follows, and where the third receive window carries a forwarded
  downlink back. `heard_wor_ack` reads the acknowledgment, which tells the device when
  the relay next scans, so the following frames carry only the preamble the two clocks
  could have drifted apart, and holds its payloads to what the relay forwards.
  `no_wor_ack` says whether to send the uplink anyway or wake the relay again, as the
  network's `BackOff` asks, and gives up what the device knows of a relay that stops
  answering. A device left to itself follows appendix 5: it tries a relay on one join in
  four, keeps it only if the join accept comes back through one, turns it on again after
  sixteen unanswered uplinks, and puts it aside after eight frames a relay never
  answered. `EndDeviceConfReq` takes the decision over, sets the second channel and the
  back-off, and a saved state carries all of it, and the wake-on-radio counter, across a
  loss of power.
- A LoRaWAN relay that runs, `pamoja_lorawan::relay::Relay`: an end device that
  also scans for the wake-on-radio frames of TS011-1.0.1, verifies them against
  the counter it expects from each trusted device, answers with a WOR ACK saying
  when it scanned and whether it will forward, listens for the uplink behind the
  frame, forwards it to the network on port 226, and sends the answer back in the
  end device's RXR window. It keeps the tables its network configures with the
  relay commands, each usable on its own: a join request filter decided by the
  longest prefix that matches, sixteen trusted devices, and the token buckets of
  section 8.8, which stop it forwarding and tell a device when to try again. A
  device the relay does not trust is notified to the network once. Anchored to the
  worked filter example of appendix 3 and the scan, limit and counter rules of
  chapters 3, 8 and 10.
- The LoRaWAN relay of TS011-1.0.1, in Rust, C, TypeScript, Python and C#: the
  wake-on-radio frames an end device sends ahead of a join request or an uplink,
  sealed under keys derived from the network session key, the WOR ACK a relay
  answers with, the payload a relay forwards an uplink in on port 226, and the
  arithmetic that shortens a WOR preamble once a device knows when its relay
  scans. The thirteen relay MAC commands are read and written with the others,
  so a device no longer stops reading commands at one, and every channel plan
  carries the WOR channels RP002-1.0.5 gives its region. The frames match a
  line-for-line rendering of Semtech LoRa Basics Modem, the root key matches The
  Things Stack's test vector, the timing follows the specification's worked
  example, and WOR ACK airtimes match RP002-1.0.5 table 128. The
  `EndDeviceConfAns` status follows TS011-1.0.1, which moved bit 3 from BackOffACK
  to SecondChAckOffsetACK.

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
- A LoRa radio on a Linux board, in every language. `pamoja_radios::radio::Radio` holds
  a chip of either family behind one set of calls, and `pamoja_radios::linux` opens one
  over the kernel's spidev and GPIO character devices, resets it, and hands it back
  ready to configure. The C ABI, TypeScript, Python and C# carry that radio as a handle
  with configure, transmit, receive, listen, standby, sleep, and register access; the
  calls compile on every platform and report plainly that only Linux has those devices,
  and each language runs them off its main thread or with the interpreter lock released.
  The Raspberry Pi, RP2040 and ESP32 board pages each gain a radio program built in CI:
  an RFM95W breakout on a Pi's SPI bus, the Waveshare Pico-LoRa-SX1262 on a Pico, and an
  RFM95W on an ESP32-C3, each beaconing under its duty-cycle guard and printing what it
  hears with the levels it heard at. The SX1262 card lists the Pico board at The Pi Hut
  and at Waveshare.
- LoRaWAN gateways in `pamoja-gateway`, a new crate, starting with the Semtech UDP packet
  forwarder protocol on both sides: the PUSH_DATA and PULL_DATA a gateway sends, the
  PUSH_ACK, PULL_ACK and PULL_RESP a server answers with, the TX_ACK that reports what
  became of a downlink, and the `rxpk`, `stat`, `txpk` and `txpk_ack` objects they carry,
  from PROTOCOL.TXT in Semtech's `packet_forwarder`. A frequency crosses in hertz, a
  payload as bytes, and a datarate identifier as the same link settings the airtime and
  range math takes, with the base64 of RFC 4648 and the protocol's two timestamp formats
  written in the crate rather than pulled in. TypeScript, Python and C# build and read the
  same datagrams, checked against new conformance vectors and the protocol's own examples.
- The network and bridge sides of a site in `pamoja-gateway`. `network` is the
  network server of one site: it admits a device from its join request, grants the
  session, follows the frame counter through a wrap and refuses a replay, decrypts
  what a node sent, and works out where and when to answer, with the receive
  windows and delays RP002-1.0.5 recommends for every region and the DLSettings
  and RXDelay bytes of TS001-1.0.4. A frame for an address the site has not
  granted is reported rather than refused, because a gateway hears every network
  in range. `bridge` carries messages between the radio the nodes are on and the
  link that leaves the site, under a prefix naming the site and a direction
  segment that keeps a forwarded reading from coming back as a command, and it
  reports what crossed, so a console is fed by the same pass that carries the
  traffic. A new example, `gateway_fleet`, runs a site end to end and draws it.
- The network side of a site in every language. `pamoja-gateway`'s `network` module
  reaches the C ABI, TypeScript, Python and C#, so a server that admits a device,
  answers its join, decrypts what it sends and builds the downlink for the window
  the uplink opened is written the same way in all four. A network now owns the
  channel plan it runs on rather than borrowing one, which is what lets a binding
  hold it. New conformance vectors pin the whole exchange, so the four languages
  produce the same join request, join accept, uplink frame and downlink bytes, and
  the gateway guide gains a second example that runs in each of them.
- The LoRa Basics Station protocol in `pamoja-gateway`, both sides of it: the
  discovery a station asks for, the router configuration a server answers with,
  and the uplink, join, downlink, transmit confirmation and time synchronization
  messages that follow over the websocket the two hold open. A downlink is placed
  against the station clock rather than the host clock, because the two do not
  agree, and a receive window is measured from the uplink that opened it. The
  protocol reaches the C ABI, TypeScript, Python and C#, with conformance vectors
  pinning every message in all four.
- Interop against a network server somebody else wrote. A continuous integration
  job runs ChirpStack with Postgres, Redis, Mosquitto and the gateway bridge, and
  pamoja carries an OTAA join and an uplink to it over both the packet forwarder
  and Basics Station, then takes the downlink back. `cargo xtask chirpstack` runs
  the same stack locally.
- The Semtech SX1302 and SX1303 concentrators in `pamoja-radios`, which is what
  separates a gateway from a node: eight receivers listening across every
  spreading factor at once rather than one. The crate carries the transfers the
  chip answers, the register map, the firmware the two microcontrollers inside it
  run, the channels the receivers are pointed at, the front ends they listen
  through, the packets they hand back, what a transmission is told to send, and
  the counter a receive window is measured from. A driver walks all of that over
  a real bus, and `pamoja-gateway` gains a daemon that runs a concentrator
  against either upstream from a file it is given. The SX1261 that listens beside
  the concentrator for a carrier check has its commands, its patch loading and
  its decoders carried as well, though nothing drives it over a bus yet.
- The USB versions of the same cards, the RAK5146 and the WM1302 among them, which put
  an STM32 between the host and the concentrator. The bridge does the SPI on the host
  side and drives the card supply and reset pins itself, so `pamoja-radios` carries the
  messages it speaks, a port-backed SPI device and pin that let the same driver run over
  it unchanged, and a Linux opener that brings the card up the way the reference does.
  The gateway daemon takes `usb` in place of the SPI device and its two lines, and
  runs a USB card through the same code as an SPI one.
- The MAC commands a LoRaWAN network and device configure each other with, in
  `pamoja_lorawan::mac`: all ten pairs from section 5 of LoRaWAN 1.0.3, in both
  directions, covering data rate, power, channels, receive windows, duty cycle,
  timing and device status. An identifier names a different command in each
  direction, so reading one takes a direction and has no default. A command
  carries no length, so reading stops at one it does not know and hands back what
  it could not read. The commands reach the C ABI, TypeScript, Python and C#, with
  conformance vectors pinning the bytes of every one.
- A LoRaWAN device that keeps itself reachable when the network goes quiet.
  `pamoja_lorawan::adr::Backoff` counts unanswered uplinks, says when to ask the
  network for an answer, and steps the data rate down after that, as section
  4.3.1.1 lays out. `Backoff::recommended` starts from the limit and delay
  RP002-1.0.5 recommends for every region, and `pamoja_lorawan::defaults` carries
  the rest of that table: the receive and join accept delays, the frame counter
  gap and the retransmission timeout.
- Every bit of a LoRaWAN frame header, in the direction it belongs to. An uplink
  can set the ADR acknowledgment request, and a received uplink reports it along
  with its Class B bit. A frame with no payload can leave its port out, which is
  how a device answers a MAC command with nothing else to send. A join accept
  hands back its channel list, the first window's data rate offset, the second
  window's data rate and the delay to the first window. `CfList` reads and builds
  both forms a channel list takes, frequencies and channel masks, and
  `pamoja_lora::region::FixedChannelList` gives the channel numbers the second form
  refers to their frequencies, with tests anchored to a captured join accept and
  to the worked examples in RP002-1.0.5.
- A LoRaWAN Class A end device in `pamoja_lorawan::device`, which is what a node
  runs to take part in a network. It joins over the air or starts from a
  provisioned session, picks a channel and data rate for each uplink, says when
  and where both receive windows open, and reads what comes back. Told which window
  a frame arrived in, `heard_in` discards one whose MACPayload is longer than that
  window's data rate carries, as TS001-1.0.4 section 4.1 asks. It does what
  every device-side MAC command asks and answers in order, repeating the four
  that change how it listens until a downlink arrives, and creates no channel past
  the 80 a dynamic plan defines. It repeats uplinks as many times as the network
  sets, waits out the retransmission timeout after a confirmed uplink that went
  unacknowledged, whether the windows held nothing or a downlink without the
  acknowledgment, and backs off when the network goes quiet. It keeps the
  region's sub-band duty cycles, the network's aggregated limit, and the join
  back-off of TS001-1.0.4 section 7. It owns no radio and no clock: each step
  takes the time and returns the frame, the carrier, the power and the windows. It
  follows LoRaWAN 1.0.3 or TS001-1.0.4 where the two differ, and covers every
  published plan. On the dynamic plans, EU868, EU433, AS923, KR920, IN865 and RU864,
  it takes the channels a network creates and moves. On US915 and AU915 it joins in
  the passes RP002-1.0.5 section 3.5.2 lays out, eight 125 kHz channels from
  successive groups and then a 500 kHz one until every channel has gone out, and
  answers each channel on the downlink channel its plan numbers. It reads each
  `ChMaskCntl` value by its region's table, including the 900 MHz plans' pairing of a
  bank of eight with its 500 kHz channel, takes a join accept's channel list as the
  channels to enable, and counts US915 power as conducted, taking off only the
  antenna gain above 6 dBi. On CN470 it scans the twenty common join channels and
  follows the plan the one that answered belongs to. Its tests run every exchange
  against this crate's own network half, expect the answer bytes chapter 5 lays out,
  and follow the regional document's own examples: the join passes, both ways
  section 3.5.5 narrows a device to one sub-band, every row of table 49, and the
  96-channel plan answering channel 49 on downlink channel 1.
- A LoRaWAN Class A node in `pamoja_radios::lorawan`, behind the `lorawan`
  feature: an end device driving an SX126x, an SX127x or either through `Radio`.
  `Node` puts each frame on the air, opens both receive windows on time and sees
  an uplink through every repeat. It sizes and centers each window on the
  downlink's preamble the way Semtech's LoRaMac-node does, and tunes the sync
  word and IQ polarity RP002-1.0.5 table 112 gives a device. It needs no
  allocator and no runtime, only a clock and a delay. The ESP32-C3 package gains a
  LoRaWAN node that sends BME280 readings through an RFM95W, built for the chip
  in CI, and the ESP32 page walks through its wiring.
- The uplink events a ChirpStack network server publishes on MQTT, read by
  `pamoja_gateway::chirpstack` into the device, the payload, the port, the
  counter and every gateway that heard it, following the JSON form of ChirpStack's
  `UplinkEvent`. The ChirpStack interop job now reads the live server's events back
  with it, over both the packet forwarder protocol and Basics Station.
- A walkthrough on the site, Node to dashboard, that joins the ESP32-C3 LoRaWAN
  node, a gateway on a Raspberry Pi and a ChirpStack server into one system, and a
  Raspberry Pi program that serves a dashboard of every node the server hears.
- ChirpStack's uplink events in C, TypeScript, Python and C#: the JSON a network
  server publishes on MQTT read into the device, its address, the counter, the port,
  the payload decoded from base64, and every gateway that heard it with the one that
  heard it best, and the topics to subscribe to for one application or all of them.
  New conformance vectors hold the four languages to the event on ChirpStack's own
  documentation page and to what an event may not carry. A LoRa radio on a Linux
  board also draws random numbers from its receiver noise in every language, as it
  already did in Rust.
- A LoRaWAN Class A end device in C, TypeScript, Python and C#: `EndDevice` joins
  over the air or starts personalized, picks each uplink's channel and data rate,
  says when and where both receive windows listen, reads what the network sends
  back, held to the length the window it names carries, does what its MAC
  commands ask, repeats or joins again when nothing came, and saves and resumes
  its state across a power cut. It runs on every published plan, the five
  CN470-510 plans included, and a call that cannot be done says why in each
  language's own way: an `Error` whose `code` is `Wait` with `untilUs` in
  TypeScript, `LorawanDeviceError` with `kind` and `until_us` in Python,
  `LorawanDeviceException` in C#, and a recorded reason in C. The conformance
  vectors replay whole exchanges through all four: a European day of joining,
  confirmed readings, a link check, a status request, a saved state resumed into
  a device that sends the same frame, and a downlink too long for the second
  window but not the first that leaves the device waiting out the retransmission
  timeout; nine US915 joins across the octet passes; a CN470 join that picks its
  plan; and a personalized device that refuses a replayed downlink. The LoRaWAN
  guide gains a second example, a US915 node that joins, is acknowledged in the
  first window, and picks up after a power cut.
- What keeps a LoRaWAN link running, in C, TypeScript, Python and C#: the ADR
  back-off of LoRaWAN 1.0.3 and TS001-1.0.4 section 4.3.1.1, which says when a
  device that stopped hearing its network asks for an answer and which settings it
  gives back; the channel list a join accept carries, built from and read back into
  frequencies or channel mask groups; the defaults RP002-1.0.5 section 3.3
  recommends; and the parts of a join accept that were only reachable from Rust,
  its RX1 offset, RX2 data rate, receive delay and channel list. A frame's ADRACKReq
  and ClassB bits now cross every binding when encoding, decoding and reading a
  header, and a device reports its DevEUI. New conformance vectors hold all four
  languages to TS001-1.0.4 table 9, the channel list and join settings of a
  published EU868 join accept, and the refusals of a frequency a channel list
  cannot carry.
- Saving and resuming a joined LoRaWAN device, `EndDevice::save` and `resume`, so
  a node that sleeps between readings keeps its session instead of joining again.
  The saved bytes carry the session and its keys, both frame counters, every setting
  the network's MAC commands changed, the channels, the answers still owed, and how
  long each duty cycle wait still had to run, which starts again on the clock the
  device wakes to. A CRC-32 refuses storage that corrupted them and a fingerprint
  of the channel plan refuses a state from another region; on CN470 the state names
  the join channel that chose its plan. A resumed device sends the very frame the
  device it was saved from would have.
- `pamoja_lorawan::parse_hex`, a `const fn` that reads a DevEUI, JoinEUI or root key
  written in hexadecimal. The ESP32-C3 LoRaWAN node uses it to take its identifiers
  and key from the environment it is built in, `LORAWAN_DEV_EUI`, `LORAWAN_JOIN_EUI`
  and `LORAWAN_APP_KEY`, rather than from constants in its source.
- All five CN470-510 channel plans in `pamoja-lora`, named by `Cn470Plan`: the
  plans RP002-1.0.5 gives 20 MHz and 26 MHz antennas, each in a type A and B, which
  share twenty common join channels, and the 96-channel plan of the LoRaWAN 1.0.3
  Regional Parameters revision A that RP002-1.0.5 notes is still in wide use.
  `Region::Cn470` stays the first of them.
- Listen before talk on an SX1302 gateway. `pamoja_radios::sx1302::Sx1261` drives
  the SX1261 beside a concentrator on its own SPI device, or through a USB card's
  bridge: it resets the radio, loads Semtech's patch and proves it took, calibrates
  the image, points the receiver at a channel, runs a carrier check and a spectral
  scan, each transfer as Semtech's `sx1302_hal` makes it. `Sx1302::checked_transmission`
  reads from the gain control whether a checked packet went out, and `Sx1302::abort`
  takes back an armed one. The `pamoja-gateway` daemon names the radio and its patch
  under an `sx1261` section, with the threshold and the channels it checks in a
  `listen_before_talk` section inside it; it holds each downlink on a checked channel
  until 80 ms before its window, checks the channel, and reports a busy one rather
  than transmitting into it. The gateway page covers the configuration and what the
  daemon answers.
- A channel plan's rules in C, TypeScript, Python and C#: its kind and channel list
  numbering, whether it answers `TXParamSetupReq`, what each `ChMaskCntl` value
  does, its numbered downlink channels and where the first receive window lands
  after an uplink on any channel, its join order, and whether its power ceiling is
  radiated or conducted. The five CN470-510 plans open by name, and each carries the
  runs of common join channels that select a plan, with where the join accept and
  the second receive window fall for every one. A plan builder sets all of these, so
  a private fixed plan answers what US915 does. The conformance vectors hold every
  published plan, the CN470 plans and a private fixed plan to the same answers in
  all four languages.
- Random numbers from a LoRa radio's receiver noise, `random` on the SX126x and
  SX127x drivers and on `Radio`, following the procedures of Semtech's own
  drivers. LoRaWAN 1.0.3 suggests this source for a join nonce on a device with no
  other, and the ESP32-C3 node draws its nonces and its channel seed this way.
- A register of the standards the code implements, in `docs/standards.toml`: one entry
  per specification with its publisher, its authoritative URL, and the test that pins
  the code to it, rendered on the About page under the same nine chapters as the
  guides. `cargo xtask links` fetches every specification document alongside the
  datasheets, so one that moves fails the build. Each row says what its test asserts:
  a vector the document publishes, a rule of the specification, a live implementation,
  or a round trip alone. Writing the register corrected the page it replaced.
  `CRC-16/CCITT` had named the wrong algorithm, since the mesh checks `0x29B1`, which
  is CCITT-FALSE, and the bare name means KERMIT; ITU-T K.71 governs when an antenna
  may go up without a risk assessment and was cited for bonding and surge, which is
  K.27 with IEC 62305; COBS has no published standard and cites its paper; and nine
  standards the code implements and tests, Ed25519 and CBOR among them, were not
  named at all. The three binding conformance runners each gained a perturbation
  case, so an implementation that agrees with itself and nothing else fails there.
- The hardware page's parts can be found. Each part name is a heading carrying the
  part's key, so it takes an anchor, a table of contents entry and a search row, and
  each group lists its parts as a row of anchors above its cards, so a part is one tap
  away on a phone. Entries that are not things to buy, ArduPilot, PX4, the Pixhawk
  standard, the Cortex-M4 core and ESP-NOW, no longer carry a price and say instead
  what runs them, in a group of their own. The radio and sensor cards link back to
  the pages that explain them, the price is told once, and every card names itself
  to a screen reader.
- The network side of a site in C, TypeScript, Python and C#. A site opens on a
  channel plan, registers the devices it admits, reads what a gateway forwarded, and
  builds the answer for the window the uplink opened. The conformance vectors carry
  the join request, the join accept, the uplink and the downlink, so the four
  languages are held to the same bytes rather than to four descriptions of one idea,
  and the gateway guide gains a second example that prints the same four lines in
  each. A network copies the plan it is built on rather than borrowing it, which is
  what a handle that outlives a call needs.
- An update taken a piece per call can let go of the updater between pieces.
  `Staging::detach` gives a `Transfer` that carries the hash of what has arrived, and
  `Updater::resume_from` takes it up again, checking the release against every rule as
  `resume_at` does. The hash travels only while the slot holds exactly what the transfer
  left in it; once the slot has been opened for anything else, its bytes are read back
  and hashed again.
- `LinkSettings::messages_per_hour` in Rust and `pamoja_lora_messages_per_hour` in C give
  the hourly budget the bindings already offered, and each binding now takes it from the
  core rather than working it out on its own.
- Every language can ask whether a LoRa link uses low data rate optimization, which a
  radio set up from those settings must match: `lowDataRateOptimization` in TypeScript,
  `low_data_rate_optimization` in Python, `LowDataRateOptimization` in C#, and
  `pamoja_lora_low_data_rate_optimization` in C. Only Rust could ask before.
- Simulated LoRa radio chips. `pamoja_radios::sim::Chip`, with the `sim` feature, is an SX126x
  or an SX127x that answers the radio driver over SPI as the part does, and the other languages
  have it as `SimulatedLoraChip`, with `pamoja_lora_sim_chip_*` in C. Its radio is the same one
  that opens a module on a Linux board, with the same calls, so a radio program runs and is
  tested anywhere. The program puts frames on the air for it with `hear`, at the strength and
  SNR they arrive with, and reads back what the chip was tuned to and every frame it sent.
  Nothing is timed: a transmission is done as soon as it starts, and a reception with a
  timeout ends at once when nothing waits.
- The radio in TypeScript, Python, C#, and C listens a few symbols for a preamble with
  `detect`, as a relay's scan does, where only Rust could.
- `LoraBandwidth::from_code` in both radio families' `config` modules names the bandwidth a
  register or command code carries.

### Changed

- `Quantizer::encode` returns a `Result` in Rust, and throws in the other languages, for a
  reading that is not a number, is infinite, or is too large for the scale, such as
  `codec error: reading 1 is NaN, which cannot be quantized`. A scale that is not a positive,
  finite number is refused with the same message in every language: in Rust from `encode`
  and `decode`, and elsewhere from the constructor.
- A packed batch that ends early now says `the batch ends part-way through a value` rather
  than `truncated varint`, and one holding a value past 64 bits says
  `a value in the batch does not fit in 64 bits` rather than `varint is too long`.
- `to_cbor` in Python raises `ValueError` for a document holding a NaN or an infinity, which
  JSON has no way to write, where the core refused the text with a parser error.
- `LoraChannelPlan.maxPayload` in TypeScript takes the data rate first and the table second,
  defaulting to an uplink sent directly, as it does in Python and C#.
- `LoraChannelPlan.DangerousGetHandle` in C# is `Lease`, which holds the plan open until the
  lease is disposed, so a package building on a plan cannot have it freed mid-call.
- A helper's state reads as a property in TypeScript, as it does in Python and C#:
  `smoother.value`, `thermostat.isOn`, `trigger.isSet`, `kalman.estimate`, `debounce.state`,
  `ramp.value`, `median.value`, and `trend.slope`, where each was a method.
- `Trigger.update` in Python returns the `Edge` enum, where it returned the enum's string.
  `Edge` is a `str` enum, so a comparison with `"set"` still holds.
- `Debounce` in TypeScript refuses a count that is not a whole number from 0 to 65535,
  `samples must be a whole number from 0 to 65535, not 2.9`, where it cut a fraction and
  wrapped a negative count to a large one. Python raises `ValueError` with the same words for
  a count out of range, where it raised `OverflowError`.
- On the event bus in TypeScript, Python, and C#, only the waits wait. An endpoint's
  `subscribe` and `publish` return at once rather than a promise or a coroutine in
  TypeScript and Python, and C#'s `PublishAsync` is now `Publish`. A wait gives the event
  rather than an event or null, since an endpoint keeps its bus open and a wait never sees
  it close. `pamoja_event_bus_next` takes its endpoint by shared pointer.
- An event bus holds at most 1,048,576 events, `pamoja_bus::MAX_CAPACITY`, and a larger
  capacity is lowered to it rather than allocated. Its documentation now says a capacity is
  rounded up to the next power of two, which the channel underneath always did.
- `pamoja_store_file` takes a capacity after the directory, 0 for no bound, as
  `pamoja_store_memory` does.
- A ladder's receive in TypeScript and C# gives the message rather than a message or null,
  since a ladder with nothing to listen on reports `resource is closed` rather than ending.
- An exception from a link written in Python reaches the caller as its message,
  `transport error: no signal`, as in the other languages, rather than with its type in
  front; one with no message names its type.
- The CoAP client follows RFC 7252 where it had cut corners. The first wait for an
  acknowledgment is drawn between two and three seconds rather than fixed (section 4.2), the
  first message id is random rather than 0 (section 4.4), and each request's token is four
  random bytes rather than a count from 0 (section 5.3.1). Registering an observation of a path
  again reuses its token, which a server takes as renewing it (RFC 7641 section 4.1). A send
  that ran out of retransmissions says `no acknowledgment after 5 transmissions` rather than
  naming a message id.
- An MQTT client checks a topic to publish to and a filter against the rules of MQTT 3.1.1
  section 4.7 before anything is sent, and says what is wrong, where a wildcard in a topic
  used to fail with `Failed to send mqtt requests to eventloop`.
- An MQTT connection that ends on its own, because the broker went away, another client
  connected with the same id, or a packet over the limit arrived, reports why from one
  receive, as `the connection to the broker ended: ...`, and a receive after that gives
  none. `connect` on a client that holds a connection closes it first.
- In C#, calls on one `Transport`, `Store`, `Ladder`, `LoopbackTransport`, `MqttClient`,
  `CoapClient`, or simulated device run one at a time, and one waiting for its turn holds no
  thread. `Transport.Borrow` is replaced by `LendAsync`, which waits for a call already
  running. `MqttClient` holds a `NativeHandle`, and iterating it stops within a quarter of a
  second of being canceled.
- The install page's build table printed `--features modbus` and its siblings for builds it
  measured with the `std` feature on; each command now names `std`. The page no longer says
  the narrow builds carry no third-party code, since `embedded-hal` is in each.
- A request for more values than one frame carries says so for a read as well as a write.
- `ModbusError` gains `UnitOutOfRange`, for a server made at the broadcast address or a
  reserved one, so a `match` over it needs the new arm.
- A CAN frame that does not fit says what fits: eight bytes for classic CAN and 64 for CAN FD,
  or the lengths CAN FD carries above eight.
- A simulated bus hands a part back as whichever kind it is. In Rust `I2cBus::part` is
  generic over the kind, `bus.part::<I2cPart>(address)`; in TypeScript and Python it
  returns the part as it is; in C# `bus.Part(address)` returns a `SimulatedPart` and
  `bus.Part<I2cPart>(address)` that kind or null. In C#, `Ina219`, `Ads1115`, and
  `Pca9685` are now classes whose static members are the datasheet and whose instances
  are drivers, as `Bme280` became.
- Every guide ends with a Where next generated from the capability map: the guides a
  reader goes to after it, each with what it covers, the pages beside it such as a board
  page, and the rest of its chapter. Before, one guide in 37 had one. The language tabs move
  with the arrow keys, Home and End, keep only the selected tab in the tab order, and show
  keyboard focus on their panels, and a link to a language lands with its tabs in view. A
  guide can now carry more than one set of language tabs, and a panel can carry subheadings
  of its own.
- The hardware page opens each group with a numbered selection table: every part with what
  it does, how it connects, when to pick it over its neighbors, and the lowest price listed,
  linked to its card. The page states the days its prices were read, generated from the
  catalog, in place of a promise about their age. The stepper driver cards no longer say
  their guide covers only PWM and servos, the concentrator cards link the gateway guide, the
  gateway build page and the crates that drive them, and the SX1250 card names its driver.
  The LoRa radios capability now says it covers the SX1302 and SX1303 concentrators, which it
  has since they shipped.
- The hardware page's price refresh runs every day and lands on its own: it closes any
  earlier refresh that never merged, waits for every check on its pull request, and merges
  it, so the day on each card moves with the refresh; one whose checks fail stays open for
  a person. It also lands again, and tells a listing that is gone from a store that
  refused to answer. It had read every store since 2026-09-07 and then
  failed to push, because the checkout's own read-only token took the place of the one
  meant for the push. A store page that answers 404 or 410 now takes its offer off the
  page. Digi-Key, which refuses scripted readers, is read through its Product Information
  API: a part Digi-Key no longer sells leaves the page, and one at the end of its life is
  named in the refresh's report. Mouser is no longer listed or searched.
- The MAVLink SITL job now requires ArduPilot and PX4 to store a mission plan and to
  arm. Before, it accepted a refused arm and, on ArduPilot, a refused upload, and blamed
  the upload on mission storage the headless build lacked. The storage was there. ArduPilot
  SITL does not start booting until a ground station connects, and the test was sending
  within a second of connecting, before the mission library had sized its storage or the
  estimator had started. The test now waits for the pre-arm checks to pass, then asserts
  the plan reads back item for item and the arm is accepted and shows in the heartbeat.
  The hardware catalog said the job flies both autopilots in simulation. It never flew
  anything, and it now says what the job does.
- The ros-bridge CI job now selects `rmw_zenoh` and runs the interop test that publishes
  a `Twist` from a ROS 2 node into a plain pamoja Zenoh peer, so the claim that the bridge
  is checked against `rmw_zenoh` is backed by a passing run. The test had carried a note
  blaming its publisher's lifetime for a failure; it passes as written once the RMW is told
  not to wait for a router and both peers scout by multicast, which is how `cargo xtask ros`
  has run it.
- `relay::t_offset_ms` now takes the end of a wake-on-radio frame's preamble
  rather than a time on air and a symbol time. An end device works out when its
  relay scanned by taking the offset off the end of its preamble, so that is where
  the offset has to end; the fixed allowance appendix 1 writes lands there for no
  frame's actual time on air, and would have devices aim symbols early. This is
  what Semtech LoRa Basics Modem's relay reports.
- A saved LoRaWAN device state is 1678 bytes: every queued command now has room for the
  seven bytes a relay's notification takes, and the state carries the device's relay mode
  and its wake-on-radio counter.
- `ReceiveWindow` has a third window, `Rxr`, which a device under a relay opens when
  neither of the other two held a downlink for it.
- A channel plan in `pamoja-lora` says what kind it is: dynamic, with the
  numbering its region reads a type 1 channel list against, or fixed. It also
  says whether devices on it answer `TXParamSetupReq`, which only AS923 and
  AU915-928 do, what each `LinkADRReq` channel mask control does, the downlink
  channels a fixed plan answers on, the order its join channels are tried in,
  whether its power indexes count radiated or conducted power, and, on CN470-510,
  which plan each join channel selects. All of it comes from RP002-1.0.5 section
  by section. A plan built field by field needs the new fields;
  `ChannelPlanBuilder` starts with the values every dynamic plan shares.
- The network side in `pamoja_gateway::network` answers in the first receive
  window where the channel plan says, `Rx1Channels::Plan`, unless told otherwise,
  so a US902-928, AU915-928 or CN470-510 site answers on its downlink channels
  with no block set by hand, and a CN470-510 join on the frequency table 49 gives
  its channel. The C ABI, Node and .NET default to it too, as
  `PAMOJA_GATEWAY_NETWORK_RX1_PLAN` and `Plan`.
- `pamoja_lorawan::adr::Backoff` follows the revision it is given. LoRaWAN 1.0.3
  steps the data rate down after the limit plus a delay. TS001-1.0.4 table 9
  restores the default power first, then lowers the rate each delay, then
  re-enables the default channels. The back-off reports each step and leaves the
  settings to the device that keeps them.
- The guide examples are programs rather than tests. Each one has a `main`, runs
  with `cargo run -p pamoja-examples --example <name>` or the equivalent in the
  other three languages, and the line printed beside it on the site is the line
  that runs it. They still run on every change; they are no longer spliced out of
  a test harness, which is what the pages had been showing a reader.
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
- A telemetry reporter's counters hold at `u32::MAX` rather than wrap, in every language.
  A node counting a few hundred events a second overflowed a count in about a year, which
  panicked a debug build and wrapped to a small number in a release one.
- The GPIO, audit, session, and telemetry guides print the same bytes in every language:
  where they printed a boolean, Python and C# wrote `True` and `False` beside Rust's and
  TypeScript's `true` and `false`, and each now says what the value means in words.
- `Session.Open` in C# names `message.Tag` when a tag is the wrong length, where it named
  the whole message.
- `Pose` and `Twist` in C# move from `Pamoja.Sim` to `Pamoja.Kit`, beside the motion
  helpers that take them, and `Pamoja.Sim` takes them from there. In Python, `Pose` is a
  frozen value class that compares by value and can be built, each field 0 unless given,
  and `pamoja.sim` returns the same class.
- The signed update guide follows a release past what a device accepts to what it turns
  away: the same release again, a damaged image, and another key's signature, then an
  image that boots and never confirms and is rolled back, and that failed release offered
  a second time. It prints the same twelve lines in every language, and the page gains a
  paragraph for each language, tables of the manifest, the slot states, the boot
  decisions and the calls, and a section on what goes wrong.
- The reference examples read like the guides. A crate's front page and its items build
  what they need with the library and check what it means, where they pasted frames,
  register images, and command bytes: a gateway forwards a reading and is acknowledged, a
  Modbus meter answers with its registers, the radio crate works out what EU868 lets a
  node transmit, every sensor and actuator driver runs on its simulated part, and the
  LoRaWAN MAC commands are built before they are read back. Where a page's subject is the
  wire, as in the bus layer, each value is named for what it is and the datasheet it
  comes from. The byte sequences the examples showed are unit tests now, so each is still
  checked. The Python reference had a few of the same, in its CAN, bus, and MAVLink
  docstrings, and they read the same way now.
- The bus, loopback, store-and-forward, sensor, and actuator crates open with an example,
  where they had none, and the first three run rather than only compile.
- The LoRa radios guide sends its reading through a simulated SX1262 and a simulated RFM95W,
  reading back what each chip was told, where it printed command bytes and decoded chip
  answers typed in as hex. It prints the same twelve lines in every language, with words where
  it printed booleans, and the page gains a paragraph for each language, tables of the two
  families, a radio's calls, a reception's outcomes, the sync words and a simulated chip, and a
  section on what goes wrong.
- The LoRaWAN guide says the network acknowledged the reading rather than printing a
  boolean, and the page gains a paragraph for each language, tables of what each end
  holds, the timings, counters, and ports, what an end device refuses, and the calls, and
  a section on what goes wrong.
- The LoRa guide runs one reading through every LoRa data rate from DR0 to DR5, what it
  costs on air, how many fit in an hour, and how far it is heard, and prints the same
  twenty lines in every language, with words where it printed a boolean. The page gains
  a paragraph for each language, tables of the regions, the data rates, the link
  settings, the budget terms, and the FCC rule, and a section on what goes wrong.
- The power guide takes the charging node's cadence from the mode the panel bought, asks
  a plan about a fuel gauge that did not answer, moves the thresholds for winter, and
  sizes a duty cycle from what a panel harvests. It prints the same twelve lines in every
  language, and the page gains a paragraph for each language, tables of the modes, the
  duty cycle, and the calls, and a section on what goes wrong.

### Fixed

- A quantizer packed a reading that was not a number as 0 and an infinite one as the largest
  64-bit number, so a sensor's missing reading arrived as a real one. It refuses both now.
- A packed batch followed by more bytes decoded as the batch alone, so two batches run
  together lost the second without a word. The bytes are refused now:
  `codec error: 2 bytes follow the batch's last sample`. A value longer than 64 bits was cut
  to 64 rather than refused.
- `packSamples` in TypeScript cut a fraction to its whole part, packed a NaN as 0, and
  rounded a sample past `Number.MAX_SAFE_INTEGER`, and `unpackSamples` rounded one on the way
  back. Both refuse such a sample now, `sample 1 is 10.5, which is not a whole number`.
- A CBOR document that failed to decode said so in the library's debug form, such as
  `Semantic(None, "...")`. The codec puts it into words now:
  `the CBOR ends part-way through a value`, `the CBOR is malformed at byte 3`, or
  `the CBOR has no JSON form:` and why.
- `DeviceIdentity.Verify`, `VerifyMessage`, and `FingerprintOf` in C# read past the end of a
  key shorter than 32 bytes or a signature shorter than 64, since the native call reads a
  fixed length. They throw `ArgumentException` now, and `VerifyMessage` returns null only for
  a message that fails its check, throwing for any other failure.
- Seven more C# calls read past the end of a short key or identifier for the same reason:
  `new Session`, `new AuditVerifier`, `Audit.VerifyChain`, `Update.VerifyEnvelope`,
  `Update.OpenDelegation`, `GatewayNetwork.Register`, and `LorawanRelayNode.Trust`. Each
  throws `ArgumentException` now, as the other bindings already did.
- C# passed an enum value its type does not name, such as `(PowerMode)7`, straight to the
  engine, which reads an enum as one of the values it declares, so the call was undefined
  behavior. Every call that takes one throws `ArgumentOutOfRangeException` now: in power,
  telemetry, GPIO, stepper, ROS 2, session, LoRaWAN MAC command, CoAP, and MQTT. The C
  header says the same of any enum a caller passes in, and `NamedValue.Require` in
  `Pamoja.Native.Interop` makes the check for code that calls `NativeMethods` directly.
- Many C# objects read their native pointer without holding it open, so a finalizer running
  during a call, or a `Dispose` on another thread, could free the native object while the call
  still used it: `LoraChannelPlan`, `GatewayNetwork`, every MAVLink class, and the end device
  and relay constructors that take a plan. Every native call now holds its handle for the
  length of the call, and a call on a disposed object throws `ObjectDisposedException`.
- C# objects whose native state changes with each call had no lock, so two threads could
  reach one at once, which the native side does not allow: `GatewayNetwork`, `LoraRadio`, and
  the MAVLink dialect, parser, signer, verifier, message, mission sender and receiver, and
  command tracker. Their calls run one at a time now.
- `CdrWriter.ToBytes` in C# handed an encoder native code had already consumed back to it when
  called a second time, freeing it again. It throws `PamojaException`, as its documentation
  said. Two threads finishing one `LorawanBlockMic` at once could do the same, and cannot now.
- One reading that was not a number stayed in a helper for good: a NaN from a failed sensor
  turned a smoother, a Kalman filter, a PID's integral, a window's mean, a median, or a trend
  into NaN from then on. Every helper that keeps state ignores such a reading now, in every
  language. A value helper answers with the value it held, a thermostat or a debouncer keeps
  its output, a PID returns its last output, a trigger, a surge, or a depletion countdown
  reports nothing, and an anomaly detector flags the reading and keeps it out of its
  baseline. A parameter that is not a number falls back to a documented value, such as a
  smoother weight of 1 or a hysteresis of 0.
- The motion helpers fail safe on a reading that is not a number. `obstacle_stop` treats a
  range that is NaN as an obstacle, where it drove on; `WaypointFollower::guide` stops for a
  lost fix or heading; a `Watchdog` given a time step that is not finite expires; `Limits`
  eases toward a stop on a command it cannot read; `ServoMap::pulse` sends no pulse and
  `Esc::pulse` the neutral one for an angle or a throttle that is NaN; and `Odometry` and
  `Complementary` keep their estimate. `TwoLinkArm::joints_for` refuses a target that is not
  finite, where it answered with NaN joints.
- `ServoMap::angle` read a reversed servo's pulse, one whose `min_us` is above its `max_us`,
  as the angle at one end.
- A Kalman filter with no process noise and no measurement noise returned NaN from its third
  reading on. It follows each reading now, as a sensor with no noise should.
- `Pid` in Python ignored a limit given alone: `Pid(10, 0, 0, max=40)` had no limit at all.
  It clamps to the one given, and leaves the other side open.
- The helper objects in C# had no lock, so two threads could reach one at once, which the
  native side does not allow. Their calls run one at a time now.
- The documentation of several helpers described something else. A deadband does not shift a
  reading outside the band; it passes it through. A surge reports a change of more than its
  limit, not one of at least it. A window's variance is 0 for one reading rather than absent,
  a depletion countdown reports 0 for a first reading already at the threshold, and an anomaly
  detector judges from its third reading, not once its window is full. The C ABI's
  constructors never return null, except a windowed helper's `_with_capacity` given a
  capacity it cannot keep.
- An event bus endpoint in TypeScript and Python could not publish or subscribe while its own
  wait for the next event was open: the call waited behind the wait, for good if nothing else
  published. In C#, a publish beside a waiting `NextAsync` reached the native endpoint
  alongside it, which the native side does not allow. Publishing and subscribing now run at
  once beside a wait, in every language.
- `new EventBus(-1)` in TypeScript reached the native side as a capacity of about four
  billion events and ended the process trying to allocate it. It now throws
  `a capacity must be 0 or more`.
- A file store append that had returned could be lost to a power cut on Linux, because the
  rename that put the record in place was never flushed to the directory. On Unix systems the
  directory is now flushed after each append. A record a power cut interrupted mid-write left
  a `.rec.tmp` file behind for good; opening the store now deletes it.
- `Store.Memory` in C# took a negative capacity as no bound. It throws
  `ArgumentOutOfRangeException`, as `Store.File` does.
- A method of a link written in TypeScript that threw before returning, rather than
  returning a rejected promise, ended the Node process. The call now fails with the
  method's message.
- A link written in TypeScript, Python, or C#, or through the C ABI, whose receive failed
  went silent: its receiving side ended with nothing reported, and a ladder with no other
  listening link said only `resource is closed`. The next receive now reports the failure,
  and the link has ended after it until it connects again. A Python `recv` returning a
  `(topic, payload)` pair with a text payload ended the link the same way, and the pair is now
  taken; a malformed message from a TypeScript or Python `recv` now reports why.
- A confirmable CoAP request the server reset, or answered with a 4.xx or 5.xx code such as 4.04
  Not Found, counted as delivered. It fails now with the code and its RFC 7252 name, and with
  the server's diagnostic when one came back.
- A CoAP notification from a server that follows RFC 7641 arrived with an empty topic, since
  the client read the path from the notification, which carries only its registration's token.
  The client now delivers each notification under the path it observed, answers one whose token
  it does not know with a Reset, drops one older than the newest it has had, and fails a
  registration the server answered without observing. The test server had echoed the path back,
  which hid it.
- On the documentation site, code in a table cell that starts with a dot, such as a chained
  setter like `.max_retransmits(n)`, could wrap after that dot and leave it alone on a line.
  A leading dot now stays with its name, as a leading slash already did.
- A CoAP client stopped receiving for good once a datagram it sent found no one listening,
  because the operating system reports that on the socket's next receive, which ended the loop.
  It keeps listening, and a send no longer fails on that report.
- A CoAP `subscribe` could return before the resource's current state was queued, so a
  registration made straight after it, such as a node renewing its observation, raced the
  first one and could lose its own copy of the state as a stale repeat. The client queues the
  state before the request returns now.
- The C# binding could reach one native transport, store, ladder, or simulated device from
  two thread-pool calls at once, such as a receive still waiting while a send ran, or a
  ladder taking a transport another call was using, which the native side does not allow.
  Calls on one object now run one at a time, a transport or store with a call running is
  not handed on, and one disposed while a call runs is freed once the call returns.
- An MQTT client whose connection had ended still reported itself connected, a send then
  failed with `Failed to send mqtt requests to eventloop`, and a publish over 10 KiB was
  taken and then ended the connection. It reports itself not connected, a send answers
  `resource is closed`, and an oversized publish is refused with the connection left up.
- The loopback link's documentation promised a receive would end once the broker was
  dropped, which cannot happen while the link holds it. A receive on a connected link waits
  for a message, and a reconnected link keeps its subscriptions, both pinned by tests.
- `Pdu::read_holding_registers_reply` and `read_input_registers_reply` refused more than 123
  registers, the most a write carries, while a read asks for up to 125, which fill a reply's
  250 data bytes.
- A step and direction driver raised STEP straight after setting DIR, while the A4988
  reads the direction on STEP's rising edge and needs it settled 200 ns before, the
  DRV8825 650 ns. On a fast microcontroller the two writes land nanoseconds apart and a
  step after a change of direction could go the old way. The driver now holds DIR for
  the pulse width before the edge, in every language.
- `Pwm::duty(0)` loaded the same count into a channel's on and off registers, which the
  PCA9685 datasheet says never to do, and `duty(4096)` wrapped to the same. A zero duty is
  now the full-off setting and 4096 or more the full-on one, in every language, and the
  bindings no longer claim a zero duty glitches high for one count.
- Building the dashboard crate without its extra locales, as the examples package does,
  warned about an unused variable and unreachable code, so every guide's `cargo run`
  printed both.
- The Raspberry Pi page said `pullup=1` turns on the `w1-gpio` overlay's internal
  pull-up. The overlay turns it on by default and ignores the parameter, as its README
  says, and a DS18B20 still wants the 4.7 kilohm resistor its datasheet shows.
- On the documentation site, a paragraph after a set of language tabs showed under the C#
  tab alone, so a reader on any other language never saw it: the lesson after the
  Raspberry Pi's relay and radio programs, the line after the GPIO guide's board program,
  and the build page's note on `just`. A page now marks where its languages end with
  `<!-- languages end -->`, and what follows is shown for all four. Code in a table cell
  also wraps after `::`, `.`, `(`, `,`, or `/` instead of in the middle of a name.
- The generated C header carried constants no C compiler could use. `cbindgen`
  renders a constant's initializer exactly as written, so one whose value named
  another crate's constant was dropped from the header outright and one whose
  value named an imported constant was emitted as that bare name. Twenty-four
  constants never reached the header, and six more, along with a wake-on-radio
  acknowledgment's length inside a struct, stopped it compiling at all. They are
  literals now, each held to the crate it came from by a compile-time assertion,
  and a test walks the exported surface against the generated header so neither
  can happen again.
- The CN470-510 plan put its second group of uplink channels at 483.9 MHz, where
  the first downlink group starts; RP002-1.0.5 section 3.9.2.1 starts it at 503.5
  MHz. Its beacon is now given on 483.9 MHz, the first downlink channel it hops
  over, rather than on the second receive window's frequency, and its join
  channels are all twenty common join channels of table 49 rather than eight.
- The AU915-928 plan offered every data rate its channels carry for a join.
  Section 3.8.2 joins at DR2 on the 125 kHz channels, which fits the 400 ms dwell
  time a device assumes at first, and at DR6 on the 500 kHz ones.
- The network side read a downlink data rate from the uplink table. The 900 MHz
  plans number their downlink rates apart, so an answer to a US902-928 uplink went
  out at the wrong spreading factor and bandwidth, or found no window at all.
- The AS923 plan carried the payload limits from before RP002-1.0.5, which raised
  DR2 from 59 to 123 bytes. It now has them, along with the table for a 400 ms
  dwell limit, which the plan had left out. It also joined at DR0 and DR1, which
  table 66 leaves out so that a join request fits that limit before the network
  has said whether it applies.
- A received LoRaWAN uplink with its Class B bit set no longer reports that the
  network has more data waiting. That bit means frame pending on a downlink only,
  and a received frame now reads it for the direction it traveled. A frame
  carrying MAC commands in its options on port 0, which TS001-1.0.4 section
  4.3.1.6 forbids, is refused when it is built and when it is read.
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
- `WaypointFollower::guide` turns toward its waypoint. Its yaw rate had the wrong sign, so
  a robot that followed it turned away from a target on either side. A target to the
  robot's right, a positive heading error in compass degrees, now gives a negative yaw
  rate, a right turn, as `Twist` has it, and the `Guidance` fields say which convention
  each one uses.
- The `Odometry::fuse_heading` documentation says the measurement is in the pose's own
  frame, counter-clockwise from the world x axis, and how a compass course converts to it,
  where it listed a GPS course as something to pass in as it came.
- A replay that does not repeat reports closed once it runs out, in every language, but
  the C ABI, TypeScript, and C# documentation said it kept returning its last reading. It
  says what happens now, and Python's says it too.
- The audit log documentation in TypeScript, Python, and C# said signing the index made a
  record removed from the end of a log detectable. It does not: a log cut short at the end
  is still a valid chain. The documentation, `verify_chain`'s included, now says to compare
  the last index and digest with the ones the device last reported, and a test pins the
  behavior.
- The telemetry guide example's header gave `--example telemetry` as its run command,
  which runs an older demo; it is `--example telemetry_guide`.
- The secured session guide split one sentence across three paragraphs.
- The TypeScript, Python, C#, and C updaters read back and hashed every byte already
  written each time a piece arrived, and again for each progress report, so an image
  taken in small pieces cost time in proportion to the square of its length: a megabyte
  in sixteen-byte pieces meant hashing about 34 gigabytes. Each piece now costs only its
  own bytes.
- The TypeScript `Updater` read a time that was not a whole number of seconds, such as
  `NaN` or `-1`, as 0, the start of 1970, so a release past its expiry passed the check,
  and it cut a fractional sequence short. It refuses both now, as in
  `now must be a whole number of seconds, not NaN`.
- The TypeScript `signManifest` had no documentation, because its comment sat above
  `imageDigest` instead.
- A power plan read a state of charge that was not a number, such as one worked out from a
  fuel gauge that failed to answer, as a healthy battery and ran the node at full duty. It
  takes such a charge as critical now, in every language.
- `DutyCycle::from_fraction` panicked on a fraction that was not a number, which aborted the
  process when the call came through C# or the C ABI, and it overflowed on the longest
  period. Such a fraction keeps the node asleep for the whole period now. `DutyCycle::period`
  panicked when its two halves added up past `Duration::MAX`, and holds at the maximum now.
- A duty cycle split from a fraction in TypeScript, Python, C#, or C rounded each half down
  to a whole microsecond on its own, so a tenth of a minute came back one microsecond short
  of a minute in C# and C, and its halves did not add up to its period in TypeScript and
  Python. The awake time is rounded down and the rest of the period is asleep now.
- The TypeScript `DutyCycle` and `PowerPlan` constructors and `DutyCycle.fromFraction` read
  a negative duration or one that was not a number as 0 and cut a fractional one to its
  whole part, so a job that ran longer than its interval slept not at all without a word.
  They refuse such a duration now: `sleepUs must be a whole number of microseconds, not -1`.
- LoRa airtime divided by the bandwidth, so link settings with a bandwidth of 0 panicked,
  which ended the process when the call came from TypeScript, C#, or C. A bandwidth of 0
  counts as one hertz now, as it already did across the link budget.
- A LoRa payload of 2^28 bytes or more overflowed the airtime arithmetic, so 2^29 bytes
  came back shorter on air than ten. The count keeps rising now and holds at the largest
  number rather than wrapping.
- The silence owed after a transmission wrapped round for a duty-cycle limit above 1000
  per mille, asking for about 18 quadrillion microseconds at 1001. A limit of the whole
  of the time or more owes none now.
- The C ABI did not build with only some of its capabilities, as the crate documentation
  shows it built, because its two LoRaWAN relay modules carried no feature gate. They build
  with `lora` and `lorawan` now, and CI checks the documented build.
- The C# LoRa calls read a negative payload length as a vast one. They throw
  `ArgumentOutOfRangeException` now, and `MessagesPerHour` takes the count from the core
  rather than adding two numbers that could wrap.

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
