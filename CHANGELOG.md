# Changelog

Notable changes to pamoja, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Every crate, the npm,
PyPI, and NuGet packages, and the language bindings share one version and are
released together, so one entry covers all of them.

## [Unreleased]

### Added

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

### Changed

- The hardware page opens each group with a numbered selection table: every part with what
  it does, how it connects, when to pick it over its neighbors, and the lowest price listed,
  linked to its card. The page states the days its prices were read, generated from the
  catalog, in place of a promise about their age. The stepper driver cards no longer say
  their guide covers only PWM and servos, the concentrator cards link the gateway guide, the
  gateway build page and the crates that drive them, and the SX1250 card names its driver.
  The LoRa radios capability now says it covers the SX1302 and SX1303 concentrators, which it
  has since they shipped.
- The hardware page's weekly price refresh lands again, and tells a listing that is gone
  from a store that refused to answer. It had read every store since 2026-09-07 and then
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

### Fixed

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
