# Gateway

A gateway is two parts: a Linux host and a concentrator card. The host is an
ordinary computer, usually a [Raspberry Pi](raspberry-pi.md), and everything on
that page about its buses, its permissions, and running a program as a service
holds here too. The card is what makes it a gateway. A node's radio hears one
channel at a time; a concentrator hears a whole band at once, on eight channels
and every spreading factor, behind its own RF front ends.

Three cards in the catalog do this, and all three are mini PCIe with the standard
52-pin edge connector: the [RAK2287](../hardware.md#rak2287), an SX1302 with two
SX1250 front ends and a GPS; the [RAK5146](../hardware.md#rak5146), an SX1303 with
an SX126x for listen before talk; and the
[WM1302](../hardware.md#wm1302), an SX1302 in SPI and USB versions. The connector
is mini PCIe in shape only. It carries SPI or USB and a 3.3 V supply, not PCI
Express, so the card needs a carrier board or a hat rather than a PCIe slot.

Two programs build this up, both in the package at
[`examples/boards/gateway`](https://github.com/molexxxx/pamoja/tree/main/examples/boards/gateway),
built in CI on every change: the first brings the card up and prints what it
hears, and the second reads and writes its registers directly.

## What the host has to provide

Four things, and the configuration file names all of them.

| What | Why |
| --- | --- |
| An SPI device | The card answers on it, as `/dev/spidev0.0` on a Pi with SPI0 enabled. |
| A GPIO line for reset | The concentrator does not reset itself, and nothing it answers means anything until it has. |
| 3.3 V at a few hundred milliamps | Transmitting draws far more than receiving, and a supply that sags resets the card mid-packet. |
| Two firmware images | The chip's two microcontrollers start empty. |

The reset line is the one number that differs by carrier board rather than by
card, so take it from the document for the board the card is seated in: the
[RAK2287](https://docs.rakwireless.com/product-categories/wislink/rak2287/datasheet/)
and [RAK5146](https://docs.rakwireless.com/product-categories/wislink/rak5146/datasheet/)
datasheets and the [WM1302](https://wiki.seeedstudio.com/WM1302_module/) wiki each
give the pin their own hats and adapters wire it to. Turning SPI on is the same
`raspi-config` step the [Raspberry Pi](raspberry-pi.md#turning-the-buses-on) page
covers, and the same `gpio` and `spi` group membership keeps the program from
needing root.

The USB versions of the RAK5146 and the WM1302 talk to their host through an
STM32 bridge rather than over SPI, which this driver does not speak yet. Use an
SPI version.

## The firmware images

The SX1302 does not receive anything by itself. Two microcontrollers inside it do
the work: one sets the front end gains, the other shares the radios between the
receivers. Both hold eight kilobytes and both start empty.

Those images belong to Semtech, not to pamoja, so they are not in this repository.
They ship with the reference implementation, as C source rather than as files of
bytes:

- [`agc_fw_sx1250.var`](https://github.com/Lora-net/sx1302_hal/blob/master/libloragw/src/agc_fw_sx1250.var),
  the gain control image for a board with SX1250 front ends.
- [`agc_fw_sx1257.var`](https://github.com/Lora-net/sx1302_hal/blob/master/libloragw/src/agc_fw_sx1257.var),
  the same for the older SX1255 and SX1257 front ends.
- [`arb_fw.var`](https://github.com/Lora-net/sx1302_hal/blob/master/libloragw/src/arb_fw.var),
  the arbiter image, which is the same for either.

Point the configuration straight at whichever pair matches the card. pamoja reads
that form as it is, so there is no conversion step: a file that is exactly 8192
bytes is taken as the image, and anything else is read as the array it is written
in. The BSD 3-Clause notice those files carry is reproduced in
[notices](../about/notices.md).

## The configuration

One JSON file, which both the daemon and the first program read. Everything
without a default is required.

| Field | Value | Default |
| --- | --- | --- |
| `gateway` | The identifier this gateway reports itself by, sixteen hexadecimal characters. Colons and dashes are ignored. | |
| `concentrator.spi` | The SPI device the card answers on. | |
| `concentrator.gpio_chip` | The GPIO character device the reset line is on. | |
| `concentrator.reset_line` | The line number within that chip. | |
| `concentrator.firmware.gain_control` | The gain control image. | |
| `concentrator.firmware.arbiter` | The arbiter image. | |
| `concentrator.front_end` | `sx1250`, or `sx1255`, `sx1257`, `sx125x` for the older boards. | `sx1250` |
| `concentrator.clock` | Which chain the concentrator takes its clock from, `a` or `b`. | `a` |
| `concentrator.single_input` | Whether the board wires its front ends single ended rather than differential. | `false` |
| `concentrator.listen_before_talk` | Whether an SX1261 is fitted beside the concentrator. | `false` |
| `radio.carrier_hz` | The carrier every channel offset is measured from. | |
| `radio.channels` | One to eight offsets from it, in hertz, signed. | |
| `radio.spreading_factors` | Which factors to look for, each 5 to 12. | all of them |
| `radio.lorawan_public` | Whether the network is public, which picks the sync word the receivers look for. | `true` |
| `radio.tx_freq_min_hz` | The lowest frequency the board may transmit on. Name both bounds or neither. | not checked |
| `radio.tx_freq_max_hz` | The highest. | not checked |
| `radio.duty_cycle_permille` | The share of time the band allows a transmitter, in parts per thousand, so `10` is 1%. | not held to one |
| `radio.gain_table` | What the board reaches at each power, strongest last. | the reference design's |
| `radio.dual_demodulation` | A mask of factors to demodulate twice over, one bit each from SF5. | `0` |
| `upstream.forwarder` | The host to send uplinks to. | |
| `upstream.port` | Its port. | `1700` |

A gateway on the European band, listening on the eight channels around 867.5 MHz:

```json
{
  "gateway": "b827ebfffe010203",
  "concentrator": {
    "spi": "/dev/spidev0.0",
    "gpio_chip": "/dev/gpiochip0",
    "reset_line": 23,
    "firmware": {
      "gain_control": "/usr/share/pamoja/agc_fw_sx1250.var",
      "arbiter": "/usr/share/pamoja/arb_fw.var"
    }
  },
  "radio": {
    "carrier_hz": 867800000,
    "channels": [-700000, -500000, -300000, -100000, 100000, 300000, 500000, 700000]
  },
  "upstream": { "forwarder": "router.example.net" }
}
```

Channels are offsets rather than frequencies because a concentrator tunes its
front ends once and listens around them. The eight above are the standard EU868
plan, 867.1 MHz through 868.5 MHz, with the carrier in the middle of them.

Putting the carrier in the middle is not a stylistic choice. A radio hears
1.6 MHz in total, so a 125 kHz channel has to sit within 737.5 kHz of the
carrier, counting half the channel's own width. Tuning to 867.5 MHz and reaching
for 868.5 MHz asks for an offset of a megahertz, which is outside that window:
the chip accepts the number, and those channels then hear nothing at all.

Anything the file is missing or cannot use is refused by name, so a gateway that
will not start says which field to fix rather than that the file is wrong.

`listen_before_talk` tells the gain control that an SX1261 is fitted. The scan
itself is not driven yet, so leave it off for now.

Naming a `station` upstream instead of a `forwarder` runs the other protocol.
The gateway asks that address where its network server is, opens the websocket
it names, and is told the region, the band and the data rates to count in. After
that a packet is reported by a data rate index rather than a spreading factor,
and a downlink is timed against the uplink it answers rather than against a
concentrator count, so the two upstreams need no different wiring and no
different configuration beyond the address.

A gain table entry says what the board reaches and how. `radiated_dbm` is the
reference implementation's `rf_power`, `amplifier` is its `pa_gain`,
`power_index` is its `pwr_idx`, and `digital_gain` is its `dig_gain`, so a table
can be copied across from a Semtech configuration a field at a time. The two
calibrated offsets, `offset_i` and `offset_q`, default to zero. A power asked for
above the table transmits at the most the board reaches rather than being
refused, which is what the reference does.

## What the gateway can say about a downlink it refused

The packet forwarder protocol answers a downlink with one of eight words, and
not every reason has one. The daemon reports what it can:

| What happened | What it answers |
| --- | --- |
| Sent | `NONE` |
| The window is within 42.5 ms, too close to program a chain | `TOO_LATE` |
| The window is more than 512 seconds out | `TOO_EARLY` |
| The chain is still holding the packet before it | `COLLISION_PACKET` |
| The duty cycle owes the band silence | `COLLISION_PACKET` |
| The carrier is outside `tx_freq_min_hz` to `tx_freq_max_hz` | `TX_FREQ` |
| A GPS time was asked for, and this gateway drives no GPS | `GPS_UNLOCKED` |

The two thresholds are the reference forwarder's. Below 42.5 ms a chain cannot
be configured and loaded before its moment passes, and beyond 512 seconds a
timestamp is wrong rather than early, since a class A window is a second or two
out and a class B one falls inside 128 seconds.

Two of these are approximations, and it is worth knowing which. A duty cycle
that is not yet spent is reported as a collision because the protocol has no
word for it, and the slot is genuinely taken, by the silence the last
transmission owes. A modulation the daemon cannot drive has no word at all.
`TX_POWER` is never answered, because a power above the table is transmitted at
the board's strongest setting rather than refused.

## Bringing it up and listening

The order a concentrator is started in is not a matter of taste, and getting it
wrong gives you a board that answers every register, passes every check, and hears
nothing. The front ends are tuned before the clock is taken from one of them,
because a front end that is not listening clocks nothing. The receivers are given
their channels before anything is switched on. The two microcontrollers are
configured after their firmware is loaded rather than before.

That order lives in the library, so a program walks it in one call:

```rust
use pamoja_gateway::daemon::{forward, image, walk, Config};
use pamoja_radios::linux::{self, Wiring};
use pamoja_radios::sx1302::channel::Plan;
use pamoja_radios::sx1302::rx::{self, BUFFER_LEN};
use pamoja_radios::sx1302::timestamp::Counter;

/// How long to wait between asking the concentrator what it heard.
const POLL: Duration = Duration::from_millis(10);

fn main() -> Result<(), Box<dyn Error>> {
    let named = env::args()
        .nth(1)
        .ok_or("usage: board-gateway <configuration.json>")?;
    let config = Config::parse(&fs::read_to_string(&named)?)?;

    // Semtech distributes these as C source rather than as files of bytes. Either form is
    // read here, so there is no conversion step between downloading one and running this.
    let gain_control = image(Path::new(&config.concentrator.gain_control_firmware))?;
    let arbiter = image(Path::new(&config.concentrator.arbiter_firmware))?;

    let wiring = Wiring::new(
        &config.concentrator.spi,
        &config.concentrator.gpio_chip,
        config.concentrator.reset_line,
    );
    let mut chip = linux::open_sx1302(&wiring)?;

    // A concentrator tunes once and listens around that carrier, so a channel is an offset
    // from it rather than a frequency of its own.
    let plan = Plan::new(config.radio.carrier_hz, &config.radio.channels)
        .looking_for(&config.radio.spreading_factors);

    // One call walks the whole start-up order. A board that stops partway names the step it
    // stopped on, which is the difference between a wiring fault and a firmware one.
    if let Some(model) = walk(&mut chip, &config, &plan, &gain_control, &arbiter)? {
        println!("{model:?} answering on {}", config.concentrator.spi);
    }
    println!(
        "listening on {} channels around {} Hz",
        config.radio.channels.len(),
        config.radio.carrier_hz
    );

    let mut counter = Counter::new();
    let mut buffer = [0u8; BUFFER_LEN];

    loop {
        // Read on every pass, so the rollover count stays current. It is what widens a
        // packet timestamp past the 27 bits the chip keeps it in.
        chip.counter(&mut counter)?;

        for packet in rx::packets(chip.receive(&mut buffer)?) {
            // The same translation the daemon forwards with: the channel becomes a carrier
            // in hertz, the chip's quarter-decibel counts become decibels, and the timestamp
            // is widened past its rollover.
            let Some(heard) = forward::heard(
                &packet,
                config.radio.carrier_hz,
                &config.radio.channels,
                &counter,
            ) else {
                continue;
            };

            let snr = heard
                .snr_db
                .map_or_else(|| "unknown".to_owned(), |ratio| ratio.to_string());
            println!(
                "{} Hz  SF{}  {} bytes  {} RSSI  {snr} SNR  crc {:?}",
                heard.frequency_hz,
                packet.datarate,
                heard.payload.len(),
                heard.rssi_dbm,
                heard.crc,
            );
        }

        thread::sleep(POLL);
    }
}
```

`cargo run --release -- gateway.json`, with a node transmitting nearby:

```text
Sx1302 answering on /dev/spidev0.0
listening on 8 channels around 867500000 Hz
868100000 Hz  SF7  23 bytes  -87 dB RSSI  9.25 dB SNR  crc Ok
868300000 Hz  SF9  18 bytes  -104 dB RSSI  -2.50 dB SNR  crc Ok
```

Nothing there is in the chip's own units. The channel became a carrier in hertz,
the quarter-decibel counts became decibels, and the 27-bit timestamp was widened
past its rollover, which is the same translation the daemon forwards with.

If it stops partway, the step it stopped on is in the message:

```text
Error: reading the version register: the SPI device failed: Io
```

That is a bus problem, not a firmware one. `loading the arbiter firmware: the
firmware read back differs at byte 4096` is the opposite: the bytes are going out
and coming back wrong, which is wiring or clock speed rather than the image.

## Reading registers directly

The program above drives the board through the library's own order. When a board
does something that order does not explain, the same driver reads and writes any
byte of the register map:

```rust
use pamoja_radios::linux::{self, Wiring};
use pamoja_radios::sx1302::register::Register;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args().skip(1);
    let (Some(spi), Some(gpio_chip), Some(line)) =
        (arguments.next(), arguments.next(), arguments.next())
    else {
        return Err("usage: registers <spi> <gpio-chip> <reset-line> [address] [value]".into());
    };

    let wiring = Wiring::new(&spi, &gpio_chip, number(&line).ok_or("the reset line is a number")?);
    let mut chip = linux::open_sx1302(&wiring)?;

    // Opening pulses reset, so the chip is answering by here and nothing else has been asked
    // of it. Both of these read the chip's own identity rather than anything configured.
    chip.check()?;
    println!("version {}, part {:?}", chip.version()?, chip.identify()?);

    let Some(address) = arguments.next() else {
        return Ok(());
    };
    let address = u16::try_from(number(&address).ok_or("the address is a number")?)?;

    // A whole byte, which is what an address on its own names. Register::new also takes a bit
    // offset and a width, which is how the driver addresses the fields packed inside a byte.
    let whole = Register::new(address, 0, 8, false);

    if let Some(value) = arguments.next() {
        let value = u8::try_from(number(&value).ok_or("the value is a number")?)?;
        chip.write_register(whole, value)?;
    }

    println!("{address:#06x} = {:#04x}", chip.read_register(whole)?);
    Ok(())
}
```

`cargo run --release --bin registers -- /dev/spidev0.0 /dev/gpiochip0 23` asks the
chip what it is, which is the shortest check that the bus and the reset line are
both right:

```text
version 16, part Sx1302
```

A version of 16 and a part it recognizes means the card is wired correctly and
answering, whatever else is wrong. Add an address to read one byte, and a value
after it to write one.

## Running it as a gateway

The listening program proves the card works. Forwarding to a network server is
the [`pamoja-gateway`](https://github.com/molexxxx/pamoja/tree/main/crates/pamoja-gateway)
daemon, which reads the same file:

```sh
cargo install pamoja-gateway --features daemon
pamoja-gateway /etc/pamoja/gateway.json
```

It brings the card up the same way, forwards every packet it hears to the host in
`upstream`, holds its route open, reports how it is doing every thirty seconds,
and transmits the downlinks that come back at the microsecond they are asked for.
The protocol it speaks is Semtech's packet forwarder protocol, which has no
authentication and no retries, so it belongs on a private network or inside a
tunnel. The [gateway guide](../guides/gateway.md) covers that protocol from both
sides, in four languages, along with the network side of a single site.

Running it as a service is the same systemd unit the
[Raspberry Pi](raspberry-pi.md#running-it-as-a-service) page shows, with
`ExecStart=/usr/local/bin/pamoja-gateway /etc/pamoja/gateway.json`.

## The antenna is most of the range

A concentrator only decodes what reaches it. A gateway antenna at height, with a
feed line that is not throwing the gain away, is worth more than anything in the
configuration file above.
[Radios and antennas](../radio.md) covers the antenna types and their patterns,
connectors and pigtail loss, feed line, matching and VSWR, height and Fresnel
clearance, lightning and bonding, and the power limits and duty cycle each region
sets. The [LoRa guide](../guides/lora.md) works out the airtime, the channel
plans, and the link budget in code.

## Where next

- [LoRaWAN gateways](../guides/gateway.md), for the packet forwarder protocol and
  the Basics Station protocol from both sides, and a single site that admits a
  join and answers an uplink.
- [Radio drivers](../guides/radios.md), for the node side: the SX126x and SX127x
  chips a device transmits with.
- [Radios and antennas](../radio.md), for everything outside the connector.
- [Raspberry Pi](raspberry-pi.md), for the host: its buses, permissions, running a
  program as a service, and cross-compiling for it.

## Sources

- [RAK2287 WisLink LPWAN Concentrator Datasheet](https://docs.rakwireless.com/product-categories/wislink/rak2287/datasheet/),
  for the chips, the bands, the interface, and the GPS whose pulse per second is
  wired to the SX1302.
- [RAK5146 WisLink LPWAN Concentrator Datasheet](https://docs.rakwireless.com/product-categories/wislink/rak5146/datasheet/),
  for the SX1303, the SX126x it adds for listen before talk, and its SPI and USB
  versions.
- [LoRaWAN Gateway Module WM1302](https://wiki.seeedstudio.com/WM1302_module/),
  for the module's versions, its form factor, and the bands it covers.
- The [`sx1302_hal` readme](https://github.com/Lora-net/sx1302_hal/blob/master/readme.md),
  for the reference implementation the driver follows and the firmware images it
  distributes.
- [SPI on Raspberry Pi](https://github.com/raspberrypi/documentation/blob/master/documentation/asciidoc/computers/raspberry-pi/spi-bus-on-raspberry-pi.adoc),
  for the device files and how the kernel driver handles chip select.
