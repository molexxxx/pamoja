# Raspberry Pi

A Raspberry Pi is the gateway-class board: it runs Linux, so it runs the full
`std` build of pamoja, every transport, and the dashboard, and it reaches the
parts on its 40-pin header through the kernel's own bus drivers. Everything on
this page holds for any model with that header, from the
[Pi 5](../hardware.md#raspberry-pi-5) to the
[Zero 2 W](../hardware.md#raspberry-pi-zero-2-w).

Three programs build this up, and all three are in the package at
[`examples/boards/raspberry-pi`](https://github.com/molexxxx/pamoja/tree/main/examples/boards/raspberry-pi),
built in CI on every change: a sensor read, a relay and a switch, and then a
whole node that ties them to a profile and a broker.

## The header

The header's pins are 3.3 V logic. The Raspberry Pi documentation is direct
about it: do not use 5 V for 3.3 V components. The pins that matter for a
sensor, with the BCM GPIO numbers the documentation and the kernel use:

| Bus | Signal | GPIO |
| --- | --- | --- |
| I2C | SDA | GPIO2 |
| I2C | SCL | GPIO3 |
| SPI0 | MOSI | GPIO10 |
| SPI0 | MISO | GPIO9 |
| SPI0 | SCLK | GPIO11 |
| SPI0 | CE0 | GPIO8 |
| SPI0 | CE1 | GPIO7 |
| SPI1 | MOSI, MISO, SCLK | GPIO20, GPIO19, GPIO21 |
| SPI1 | CE0, CE1, CE2 | GPIO18, GPIO17, GPIO16 |
| UART | TX | GPIO14 |
| UART | RX | GPIO15 |
| PWM | hardware channels | GPIO12, GPIO13, GPIO18, GPIO19 |
| 1-Wire | data, by default | GPIO4 |

GPIO2 and GPIO3 have fixed pull-up resistors on the board, so an I2C breakout
needs none of its own. Every other pin's pull is set in software, which the
[inputs](#inputs-and-their-pulls) section covers. Physical pin numbers are a
different numbering that runs down the header in pairs; the `pinout` command,
run in a terminal on the Pi, prints the whole header with both numberings, and
is the reference to have open while wiring.

## What a pin can and cannot drive

This is the part that ends in smoke if it is skipped. A pin set high tries to
drive its output to 3.3 V and a pin set low tries to drive it to ground; the
drive strength is not a current limit, only the current at which the pad still
meets its voltage specification. The numbers from the hardware documentation:

| Rating | Value |
| --- | --- |
| Design target per pin | about 3 mA |
| Safe current per pin | 16 mA |
| Default drive strength | 8 mA (4 mA on 4-series) |
| Guaranteed output high | at least 3.0 V |
| Guaranteed output low | at most 0.14 V |
| Internal pull resistor | 50 to 65 kilohms |

Load every pin at 16 mA and the total is 272 mA, which collapses the 3.3 V
supply. The documentation is explicit that the spikes bounce into everything
near them, the SD card included. So an LED gets a series resistor, and anything
with a coil or a motor in it, a relay, a solenoid valve, a pump, gets a driver
board or an H-bridge between it and the pin. The program below drives a relay
board, which is the ordinary answer: the pin switches a transistor on the board,
and the board's own supply switches the load.

One more thing decided by the hardware: every GPIO reverts to an input on
power-on reset, and most pins have a default pull. A relay board therefore sees
whatever that pull gives it from the moment power arrives until a program takes
the line, which for an active-low board can mean energized. Taking the line and
naming its resting level in the same call is what closes that window.

## Turning the buses on

The kernel drivers for the header's buses are off by default. Either run
`sudo raspi-config` and turn them on under Interface Options, or edit the
firmware's `config.txt` and reboot. The firmware's own overlay reference names
the settings:

```text
dtparam=i2c_arm=on
dtparam=spi=on
dtoverlay=w1-gpio
```

The first turns on the ARM's I2C interface; `dtparam=i2c_arm_baudrate=400000`
raises it from the default 100000. The second turns on SPI0, which appears as
`/dev/spidev0.0` and `/dev/spidev0.1`, one per chip select, and the kernel's
driver drives the select lines as plain GPIO. The third loads the kernel's
1-Wire driver on GPIO4; `dtoverlay=w1-gpio,gpiopin=17` moves it, and
`pullup=1` turns on the internal pull-up. Once on, the buses are files:
`/dev/i2c-1` for the header's I2C, `/dev/spidev0.0` for SPI, `/dev/gpiochip0`
for the GPIO lines, and `/sys/bus/w1/devices/` for every 1-Wire device the
kernel found.

## Permissions

Opening those files is a permission, not a privilege escalation. A user has to
be in the group that owns each one. The default account is in all of them
already; any other account is added by hand and has to log out and back in:

```sh
sudo usermod -a -G gpio,i2c,spi $USER
```

If a program fails with a permission error on `/dev/i2c-1` or
`/dev/gpiochip0`, this is almost always why. Running it under `sudo` also
works and is the wrong habit: a node that has to be root to read a thermometer
is a node that runs as root forever.

## Inputs and their pulls

An input pin left unconnected floats, and a floating pin reads as noise. A
switch wired between a pin and ground needs the pin pulled up, so it reads high
until the switch closes it to ground. GPIO2 and GPIO3 have that pull in
hardware; every other pin is set in software, and the firmware sets it at boot
through the `gpio` directive in `config.txt`:

```text
gpio=27=ip,pu
```

The directive takes a pin, a range like `3-4`, or a list like `3-4,6,8`,
followed by attributes: `ip` and `op` for input and output, `dh` and `dl` for
an output's level, `pu`, `pd`, and `pn` for the pull. Later lines override
earlier ones. Two caveats from the documentation are worth knowing before
trusting it: the settings take a few seconds after power to apply, longer when
booting over the network, and they can be overridden later by device-tree
`pinctrl` entries or by the `pinctrl` utility. An external 10 kilohm resistor
to 3.3 V does the same job and cannot be overridden by anything.

The kernel's GPIO character device is what a program then opens. `gpioinfo`
prints every line on every chip with its direction and the name of whatever
holds it, which is the fastest way to find out that another process, or a
kernel driver, already has the line a program is failing to take.

## Wiring a BME280

A BME280 breakout has four pins that matter. Wire VIN to a 3V3 pin, GND to a
ground pin, SDA to GPIO2, and SCL to GPIO3. The chip answers at `0x76` unless
the breakout's address jumper moves it to `0x77`; the driver's
`I2C_ADDRESS_PRIMARY` and `I2C_ADDRESS_SECONDARY` are those two. Before running
anything, `i2cdetect -y 1` prints the addresses that answer on the bus; a part
that does not appear there is a wiring problem, not a software one.

## Reading the sensor

The first program opens the header's I2C bus, hands it to the BME280 driver,
and prints a compensated reading every two seconds.

<!-- snippet: examples/boards/raspberry-pi/src/main.rs#example -->
From [`examples/boards/raspberry-pi/src/main.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/main.rs):

```rust
use pamoja_hal::linux;
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

fn main() -> Result<(), Box<dyn Error>> {
    // The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
    let bus = linux::i2c("/dev/i2c-1")?;

    // The driver runs the datasheet's sequence over that bus: reset, identify, read the
    // calibration, configure, and then a forced measurement per read.
    let mut sensor = Bme280::i2c(bus, I2C_ADDRESS_PRIMARY, linux::delay());
    sensor.init()?;

    loop {
        let measurement = sensor.measure()?;
        println!(
            "{:.2} C, {:.2} hPa, {:.2} % humidity",
            measurement.celsius(),
            measurement.hectopascals(),
            measurement.relative_humidity_percent()
        );
        thread::sleep(Duration::from_secs(2));
    }
}
```
<!-- end -->

With a Rust toolchain on the Pi, which [rustup](https://rustup.rs) installs
in one line, clone the repository and run it from that directory:

```sh
cd examples/boards/raspberry-pi
cargo run --release
```

The `linux` feature of `pamoja-hal` opens the kernel's I2C, SPI, and GPIO
character devices as the `embedded-hal` traits every driver takes, so the same
driver that ran here runs unchanged on a microcontroller or over a scripted bus
in a test.

## Driving a relay and reading a switch

The second program is the other half of a node: an output that acts and an
input that reports. Wire the relay board's IN to GPIO17 and its own VCC and GND
to the header's 5V and ground, and wire a limit switch between GPIO27 and a
ground pin with `gpio=27=ip,pu` in `config.txt`.

<!-- snippet: examples/boards/raspberry-pi/src/bin/relay.rs#example -->
From [`examples/boards/raspberry-pi/src/bin/relay.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/bin/relay.rs):

```rust
use pamoja_gpio::pin::Polarity;
use pamoja_gpio::switch::{Contact, Switch};
use pamoja_hal::digital::PinState;
use pamoja_hal::linux;
use pamoja_kit::Debounce;

/// The GPIO chip the header's lines live on. Every current model exposes them here, and
/// the line numbers below are the BCM numbers the documentation and the kernel both use.
const CHIP: &str = "/dev/gpiochip0";

/// The line the relay board's input is wired to.
const RELAY_LINE: u32 = 17;

/// The line the limit switch is wired to.
const SWITCH_LINE: u32 = 27;

fn main() -> Result<(), Box<dyn Error>> {
    // Taking the line as an output also says what to drive the moment it is taken. Until
    // then every GPIO is an input, so a relay board sees whatever its own pull gives it;
    // driving the resting level immediately is what keeps a vent from opening at boot.
    // Most relay boards energize on a low input, which is what `ActiveLow` says once so
    // that nothing below this line has to think about the inversion again.
    let line = linux::output(CHIP, RELAY_LINE, "pamoja-relay", PinState::High)?;
    let mut relay = Switch::new(line, Polarity::ActiveLow);

    // The switch is wired to pull the line down when it closes, so it is active low too.
    let line = linux::input(CHIP, SWITCH_LINE, "pamoja-limit")?;
    let mut limit = Contact::new(line, Polarity::ActiveLow);

    // A mechanical contact bounces for a few milliseconds as it closes. Sampling every
    // 20 ms and requiring three agreeing samples means the state has to hold for 60 ms
    // before it counts, which is longer than the bounce and shorter than a person.
    let mut settled = Debounce::new(3, false);
    let mut was_closed = false;

    println!("watching GPIO{SWITCH_LINE}, driving GPIO{RELAY_LINE}; Ctrl-C to stop");
    loop {
        let closed = settled.update(limit.is_asserted()?);
        if closed != was_closed {
            println!(
                "the limit switch {}",
                if closed { "closed" } else { "opened" }
            );
            // The relay follows the switch. A real vent would run its motor until the
            // limit closes and then stop; this is the same two calls either way.
            relay.set(!closed)?;
            was_closed = closed;
        }
        thread::sleep(Duration::from_millis(20));
    }
}
```
<!-- end -->

```sh
cargo run --release --bin relay
```

Three things in it are the whole lesson. The polarity is stated once, at the
top, so no line below it inverts anything by hand; a relay board that energizes
on a high input is a one-word change. The initial level is passed when the line
is taken, which is what closes the power-on window described above. And the
contact is debounced: a mechanical switch makes and breaks contact several
times over a few milliseconds as it closes, so a raw read produces a burst of
edges, and requiring three agreeing samples 20 ms apart turns that burst into
one clean change.

`Switch` and `Contact` implement the same `Actuator` and `Sensor` traits as
every other output and input in pamoja, so the relay on GPIO17 can be handed
straight to a profile's control loop. Which is the next program.

## A LoRa radio on the SPI bus

The fourth program puts the Pi on the air. An
[RFM95W breakout](../hardware.md#sx1276), which carries an SX1276, wires to SPI0:
VIN to a 3V3 pin, GND to ground, SCK to GPIO11, MISO to GPIO9, MOSI to GPIO10,
CS to GPIO8, and RST to GPIO25. The breakout regulates its own supply and level
shifts every input, so 3.3 V logic reaches it unchanged. Screw the antenna on
before powering it: a transmitter with nothing on its output reflects its own
power back into the amplifier.

`dtparam=spi=on` makes CS the kernel's own chip select on `/dev/spidev0.0`, and
the reset line is an ordinary GPIO the driver pulses.

<!-- snippet: examples/boards/raspberry-pi/src/bin/radio.rs#example -->
From [`examples/boards/raspberry-pi/src/bin/radio.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/bin/radio.rs):

```rust
use pamoja_lora::budget::{Decibels, LinkBudget};
use pamoja_lora::region::Region;
use pamoja_radios::duty::DutyCycle;
use pamoja_radios::linux::{self, Wiring};
use pamoja_radios::radio::{RadioConfig, Reception};
use pamoja_radios::sx127x::config::PaOutput;
use pamoja_radios::sx127x::Board;

/// The header's first SPI chip select, the GPIO chip its lines are on, and the line the
/// breakout's reset pin is wired to.
const SPI: &str = "/dev/spidev0.0";
const CHIP: &str = "/dev/gpiochip0";
const RESET_LINE: u32 = 25;

/// The channel this node uses, the data rate it sends at, and how long it listens between
/// beacons.
const FREQUENCY_HZ: u32 = 868_100_000;
const DATA_RATE: u8 = 3;
const LISTEN: Duration = Duration::from_secs(10);

fn main() -> Result<(), Box<dyn Error>> {
    // The regional plan decides the channel's power ceiling and its duty cycle, so no limit
    // below is a number anyone has to remember.
    let plan = Region::Eu868.plan();
    let link = plan
        .link_settings(DATA_RATE)
        .expect("every LoRa data rate of the plan has link settings");
    let ceiling_dbm = plan.max_eirp_dbm(FREQUENCY_HZ);
    let permille = plan
        .duty_cycle_permille(FREQUENCY_HZ)
        .expect("the plan holds this channel");

    // A 2.15 dBi whip on half a decibel of pigtail. The antenna's gain counts against the
    // ceiling and the pigtail's loss counts for it, so the amplifier takes what is left.
    let whip = LinkBudget {
        transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
        transmit_cable_loss_db: Decibels::from_tenths(5),
        ..LinkBudget::default()
    };
    let output_dbm = whip
        .max_transmit_power_dbm(Decibels::from_db(i32::from(ceiling_dbm)))
        .floor_db() as i8;

    // Opening resets the chip and reads its version back, so a wiring mistake is caught here
    // rather than on the first frame.
    let mut radio = linux::open_sx127x(
        &Wiring::new(SPI, CHIP, RESET_LINE),
        Board::new(PaOutput::PaBoost),
    )?;
    radio.configure(RadioConfig::new(FREQUENCY_HZ, link, output_dbm))?;
    println!("beacon on {FREQUENCY_HZ} Hz at DR{DATA_RATE}, {output_dbm} dBm under a {ceiling_dbm} dBm ceiling");

    // The duty cycle is the radio's other budget: each frame buys silence in proportion to its
    // airtime, and the guard says when the next one may go out.
    let mut duty = DutyCycle::new(permille);
    let clock = Instant::now();
    let mut buffer = [0u8; 255];
    let mut reading = 0u32;

    loop {
        // Listening returns as soon as a frame arrives, and a frame comes with the levels it
        // was heard at: how strong it was, and how far above the noise.
        match radio.receive(&mut buffer, LISTEN.as_micros() as u64)? {
            Reception::Frame { len, levels } => println!(
                "heard  {} at {:.0} dBm, SNR {:.1} dB",
                String::from_utf8_lossy(&buffer[..len]),
                decibels(levels.rssi_dbm),
                decibels(levels.snr_db)
            ),
            Reception::Corrupt => println!("heard  a frame whose CRC failed"),
            Reception::Timeout => {}
        }

        let now_us = clock.elapsed().as_micros() as u64;
        if duty.ready(now_us) {
            let frame = format!("pi reading {reading}");
            let airtime_us = radio.transmit(frame.as_bytes())?;
            duty.transmitted(now_us, &link, frame.len());
            println!("sent   {frame} in {airtime_us} us on air");
            reading += 1;
        }
    }
}

/// Returns a level in decibels, for printing.
fn decibels(value: Decibels) -> f64 {
    f64::from(value.hundredths()) / 100.0
}
```
<!-- end -->

```sh
cargo run --release --bin radio
```

Four things in it are the whole lesson. The regional plan decides the channel's
power ceiling and its duty cycle, so the program names neither. The link budget
takes the antenna's gain off that ceiling and adds the pigtail's loss back, which
leaves what the amplifier may be set to. Opening the radio resets the chip and
reads its version back, so a swapped MISO and MOSI is caught there rather than on
the first frame. And every frame buys silence in proportion to its airtime, which
the duty-cycle guard hands back as the earliest time the next may go out.

Two Pis running this hear each other, and each prints the other's frames with the
RSSI and SNR it heard them at. That pair is the field test the
[radio page](../radio.md) describes, and the
[radio guide](../guides/radios.md) opens the same radio from TypeScript, Python,
and C#.

## The whole node

The third program is what a deployed node looks like. It loads a profile from a
file, reads the BME280 through the driver, lets the profile's policy decide,
switches the relay, and publishes each reading to an MQTT broker, waiting
whatever the profile's power schedule says between samples.

<!-- snippet: examples/boards/raspberry-pi/src/bin/node.rs#example -->
From [`examples/boards/raspberry-pi/src/bin/node.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/bin/node.rs):

```rust
use pamoja_codec::JsonCodec;
use pamoja_core::{Sensor, Transport};
use pamoja_gpio::pin::Polarity;
use pamoja_gpio::switch::Switch;
use pamoja_hal::digital::PinState;
use pamoja_hal::linux;
use pamoja_mqtt::{MqttConfig, MqttTransport};
use pamoja_profile::{Node, Profile};
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

/// The header's I2C bus, once the interface is on.
const I2C_BUS: &str = "/dev/i2c-1";

/// The GPIO chip the header's lines live on, and the line the relay is wired to.
const CHIP: &str = "/dev/gpiochip0";
const RELAY_LINE: u32 = 17;

/// The BME280 measures three things at once. The profile judges one number, so the node
/// hands it the temperature and keeps the rest for the reading it reports alongside.
struct Probe(Bme280<pamoja_sensors::driver::I2cRegisters<linux::I2cdev>, linux::Delay>);

impl Sensor for Probe {
    type Reading = f32;

    async fn read(&mut self) -> pamoja_core::Result<f32> {
        let measurement = self
            .0
            .measure()
            .map_err(|err| pamoja_core::Error::Io(format!("the BME280 did not answer: {err}")))?;
        Ok(measurement.celsius())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let broker = args.next().unwrap_or_else(|| "localhost".to_owned());
    let manifest = args.next();

    // The profile is a file: either one passed on the command line, or the shipped
    // grain-store profile as a starting point. Nothing below repeats a number from it.
    let profile = match &manifest {
        Some(path) => Profile::from_json(&std::fs::read_to_string(path)?)?,
        None => Profile::irrigation_node(),
    };
    println!("running {} into {}", profile.name, profile.topic);

    // The sensor: the kernel's I2C adapter, handed to the driver, which runs the
    // datasheet's sequence over it.
    let mut sensor = Bme280::i2c(linux::i2c(I2C_BUS)?, I2C_ADDRESS_PRIMARY, linux::delay());
    sensor.init()?;

    // The output: one GPIO line, driven to its resting level the moment it is taken, with
    // the relay board's active-low input stated once.
    let line = linux::output(CHIP, RELAY_LINE, "pamoja-node", PinState::High)?;
    let output = Switch::new(line, Polarity::ActiveLow);

    // The link: an MQTT broker, which on a gateway is often this same Pi.
    let mut link = MqttTransport::new(MqttConfig::new(&profile.name, broker, 1883));
    link.connect().await?;

    // That is the node. Each tick reads the probe, decides with the profile, switches the
    // relay, and publishes the reading.
    let mut node = Node::new(profile, Probe(sensor), output, link, JsonCodec);
    loop {
        let reaction = node.tick().await?;
        if let Some(alert) = reaction.alert {
            println!("alert: {}", alert.kind());
        }

        // The profile says how long to wait, given what the battery has left. A node on
        // mains reports a full charge; one on a panel reads its own charge controller.
        let (_, wait) = node.schedule(1.0, true);
        tokio::time::sleep(wait.min(Duration::from_secs(60))).await;
    }
}
```
<!-- end -->

```sh
cargo run --release --bin node -- broker.example.org profiles/irrigation-node.json
```

Nothing in it names a threshold, a deadband, or an interval. Those are in the
manifest, which anyone can read, edit, and share back, and the same manifest
runs on a microcontroller with a different two lines of setup at the top. The
[device profile guide](../guides/profile.md) is the full account of that loop,
and [`examples/brooder_node.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/brooder_node.rs)
is a worked version with a flaky uplink and a second node driven by rules.

## Running it as a service

A node has to come back after a power cut without anyone logging in, which on a
Pi means a systemd unit. The shape:

```ini
[Unit]
Description=pamoja node
After=network-online.target

[Service]
ExecStart=/home/pi/node broker.example.org /etc/pamoja/profile.json
Restart=always
RestartSec=5
User=pi

[Install]
WantedBy=multi-user.target
```

Written to `/etc/systemd/system/pamoja-node.service` and enabled with
`sudo systemctl enable --now pamoja-node`. `User=pi` keeps the earlier point
honest: the account is in the `gpio` and `i2c` groups, so the service never
needs root. `journalctl -u pamoja-node -f` follows what it prints.

## Building on something faster

A Zero 2 W compiles slowly. Two ways around it, both ordinary: build on the Pi
once and copy the binary afterward, since a release binary has no build-time
dependencies, or cross-compile from a laptop with
`rustup target add aarch64-unknown-linux-gnu` for a 64-bit OS, or
`armv7-unknown-linux-gnueabihf` for a 32-bit one, plus a linker for that target.
The crates in this package are pure Rust, so nothing here needs a cross
toolchain for C.

## Where next

- [Buses and links](../buses.md), for what each bus on the header is for,
  and the [bus layer guide](../guides/hal.md) for the traits, the scripted bus,
  and the Linux backend in detail.
- [Sensor drivers](../guides/sensors.md) and
  [actuator drivers](../guides/actuators.md), for every shipped part.
- [Your own device](../guides/device.md), for a part pamoja has never heard of.
- [Device profiles](../guides/profile.md), for the read-decide-act-publish loop
  a node runs, and the `gateway` and `fleet` examples in `pamoja-dashboard`,
  which serve the dashboard from a Pi.

## Sources

- [GPIO and the 40-pin header](https://github.com/raspberrypi/documentation/blob/master/documentation/asciidoc/computers/raspberry-pi/gpio-on-raspberry-pi.adoc),
  the source of the Raspberry Pi documentation, for the header's GPIO numbers,
  the fixed pull-ups, the alternate functions, the `gpio` group, the power-on
  state, and the voltage and current tables.
- [GPIO pads control](https://github.com/raspberrypi/documentation/blob/master/documentation/asciidoc/computers/raspberry-pi/gpio-pad-controls.adoc),
  for the drive-strength model, the 16 mA safe current, and what happens to the
  3.3 V rail under load.
- [`config.txt` GPIO control](https://github.com/raspberrypi/documentation/blob/master/documentation/asciidoc/computers/config_txt/gpio.adoc),
  for the `gpio` directive's attributes, its ordering, and its two caveats.
- [SPI on Raspberry Pi](https://github.com/raspberrypi/documentation/blob/master/documentation/asciidoc/computers/raspberry-pi/spi-bus-on-raspberry-pi.adoc),
  for the device files, the pins, and how the kernel driver handles chip select.
- The firmware's
  [overlay reference](https://github.com/raspberrypi/firmware/blob/master/boot/overlays/README),
  for the `i2c_arm`, `spi`, and `w1-gpio` settings and their defaults.
