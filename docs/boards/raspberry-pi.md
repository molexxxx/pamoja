# Raspberry Pi

A Raspberry Pi is the gateway-class board: it runs Linux, so it runs the full
`std` build of pamoja, every transport, and the dashboard, and it reaches the
parts on its 40-pin header through the kernel's own bus drivers. Everything on
this page holds for any model with that header, from the
[Pi 5](../hardware.md#raspberry-pi-5) to the
[Zero 2 W](../hardware.md#raspberry-pi-zero-2-w).

Four programs build this up: a sensor read, a relay and a switch, a LoRa radio,
and then a whole node that ties them to a profile and a broker. All four are in the
Rust package at
[`examples/boards/raspberry-pi`](https://github.com/molexxxx/pamoja/tree/main/examples/boards/raspberry-pi),
built in CI on every change. The sensor read, the relay, and the radio are also in
TypeScript, Python, and C#, compiled in CI against the packages each language installs.
The whole node is written here in Rust. The [device profiles guide](../guides/profile.md)
runs the same node in TypeScript, Python, and C#, and [`pamoja-node`](../run.md) runs it
from a wiring file with no program at all.

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
1-Wire driver on GPIO4, and `dtoverlay=w1-gpio,gpiopin=17` moves it. The
overlay turns the pin's internal pull-up on by default and now ignores its
`pullup` parameter; a DS18B20 still wants the 4.7 kilohm resistor its
datasheet shows, as the [sensor drivers guide](../guides/sensors.md#on-a-board)
wires it. Once on, the buses are files: `/dev/i2c-1` for the header's I2C,
`/dev/spidev0.0` for SPI, `/dev/gpiochip0` for the GPIO lines, and
`/sys/bus/w1/devices/` for every 1-Wire device the kernel found.

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
and prints a compensated reading every two seconds. The bus is one handle that
the program and every driver on it share, so a second part on the same two wires
is a second driver on the same bus. The [buses guide](../guides/hal.md) runs the
same driver against a simulated part and a script of the datasheet's sequence,
with nothing plugged in, and lists what each error on this bus means.

### Rust

<!-- snippet: examples/boards/raspberry-pi/src/main.rs#example -->
From [`examples/boards/raspberry-pi/src/main.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/main.rs):

```rust
use pamoja_hal::bus::I2cBus;
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

fn main() -> Result<(), Box<dyn Error>> {
    // The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
    let bus = I2cBus::open("/dev/i2c-1")?;

    // The driver runs the datasheet's sequence over that bus: reset, identify, read the
    // calibration, configure, and then a forced measurement per read.
    let mut sensor = Bme280::i2c(bus.clone(), I2C_ADDRESS_PRIMARY, bus.delay());
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

### TypeScript

<!-- snippet: bindings/node/boards/raspberry-pi/sensor.ts#example -->
From [`bindings/node/boards/raspberry-pi/sensor.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/boards/raspberry-pi/sensor.ts):

```typescript
import { setTimeout as sleep } from 'node:timers/promises'
import { I2cBus } from '@pamoja/hal'
import { Bme280, bme280 } from '@pamoja/sensors'

async function main(): Promise<void> {
  // The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
  const bus = I2cBus.open('/dev/i2c-1')

  // The driver runs the datasheet's sequence over that bus: reset, identify, read the
  // calibration, configure, and then a forced measurement per read.
  const sensor = new Bme280(bus, bme280.addressPrimary)
  await sensor.init()

  for (;;) {
    const reading = await sensor.measure()
    console.log(
      `${reading.celsius.toFixed(2)} C, ${reading.hectopascals.toFixed(2)} hPa, ` +
        `${reading.relativeHumidityPercent.toFixed(2)} % humidity`,
    )
    await sleep(2000)
  }
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
```
<!-- end -->

```sh
npm --prefix bindings/node run boards
node bindings/node/build/boards/raspberry-pi/sensor.js
```

### Python

<!-- snippet: bindings/python/boards/raspberry_pi/sensor.py#example -->
From [`bindings/python/boards/raspberry_pi/sensor.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/boards/raspberry_pi/sensor.py):

```python
import time

from pamoja.hal import I2cBus
from pamoja.sensors import Bme280, bme280


def main() -> None:
    # The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
    bus = I2cBus.open("/dev/i2c-1")

    # The driver runs the datasheet's sequence over that bus: reset, identify, read the
    # calibration, configure, and then a forced measurement per read.
    sensor = Bme280(bus, bme280.ADDRESS_PRIMARY)
    sensor.init()

    while True:
        reading = sensor.measure()
        print(
            f"{reading.celsius:.2f} C, {reading.hectopascals:.2f} hPa, "
            f"{reading.relative_humidity_percent:.2f} % humidity"
        )
        time.sleep(2)
```
<!-- end -->

```sh
python bindings/python/boards/raspberry_pi/sensor.py
```

### C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Sensor.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Sensor.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Sensor.cs):

```csharp
using System.Globalization;

using Pamoja.Hal;
using Pamoja.Sensors;

namespace Boards.RaspberryPi;

/// <summary>
/// The first program on a Raspberry Pi: a BME280 on the 40-pin header's I2C bus, read through
/// the driver pamoja ships, printed every two seconds. Wire the BME280's SDA to GPIO2, its SCL
/// to GPIO3, VIN to 3V3, and GND to ground, and turn the I2C interface on.
/// </summary>
public static class Sensor
{
    /// <summary>Runs until the process is stopped.</summary>
    public static void Run()
    {
        // The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
        using I2cBus bus = I2cBus.Open("/dev/i2c-1");

        // The driver runs the datasheet's sequence over that bus: reset, identify, read the
        // calibration, configure, and then a forced measurement per read.
        using var sensor = new Bme280(bus, Bme280.AddressPrimary);
        sensor.Init();

        while (true)
        {
            Bme280Measurement reading = sensor.Measure();
            Console.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"{reading.Celsius:F2} C, {reading.Hectopascals:F2} hPa, {reading.RelativeHumidityPercent:F2} % humidity"));
            Thread.Sleep(TimeSpan.FromSeconds(2));
        }
    }
}
```
<!-- end -->

```sh
dotnet run --project bindings/dotnet/samples/Pamoja.Boards -- raspberry-pi/sensor
```

<!-- languages end -->

The driver runs the datasheet's whole sequence over the bus, so the same driver
that ran here runs unchanged on a microcontroller's I2C peripheral or over a
simulated part in a test. In Rust, `pamoja_hal::linux::i2c` opens the same
adapter as a plain `embedded-hal` bus for a single driver that owns it outright.

## Driving a relay and reading a switch

The second program is the other half of a node: an output that acts and an
input that reports. Wire the relay board's IN to GPIO17 and its own VCC and GND
to the header's 5V and ground, and wire a limit switch between GPIO27 and a
ground pin with `gpio=27=ip,pu` in `config.txt`.

### Rust

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

### TypeScript

<!-- snippet: bindings/node/boards/raspberry-pi/relay.ts#example -->
From [`bindings/node/boards/raspberry-pi/relay.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/boards/raspberry-pi/relay.ts):

```typescript
import { Contact, GpioLine, PinLevel, Switch } from '@pamoja/gpio'
import { Debounce } from '@pamoja/kit'

// The GPIO chip the header's lines live on. Every current model exposes them here, and the
// line numbers below are the BCM numbers the documentation and the kernel both use.
const CHIP = '/dev/gpiochip0'
const RELAY_LINE = 17
const SWITCH_LINE = 27

// Taking the line as an output also says what to drive the moment it is taken. Until then
// every GPIO is an input, so a relay board sees whatever its own pull gives it; driving the
// resting level immediately is what keeps a vent from opening at boot. Most relay boards
// energize on a low input, which is what `activeLow` says once so that nothing below this
// line has to think about the inversion again.
const relay = Switch.activeLow(GpioLine.openOutput(CHIP, RELAY_LINE, PinLevel.High))

// The switch is wired to pull the line down when it closes, so it is active low too.
const limit = Contact.activeLow(GpioLine.openInput(CHIP, SWITCH_LINE))

// A mechanical contact bounces for a few milliseconds as it closes. Sampling every 20 ms and
// requiring three agreeing samples means the state has to hold for 60 ms before it counts,
// which is longer than the bounce and shorter than a person.
const settled = new Debounce(3, false)
let wasClosed = false

console.log(`watching GPIO${SWITCH_LINE}, driving GPIO${RELAY_LINE}; Ctrl-C to stop`)
setInterval(() => {
  const closed = settled.update(limit.isAsserted())
  if (closed !== wasClosed) {
    console.log(`the limit switch ${closed ? 'closed' : 'opened'}`)
    // The relay follows the switch. A real vent would run its motor until the limit closes
    // and then stop; this is the same two calls either way.
    relay.set(!closed)
    wasClosed = closed
  }
}, 20)
```
<!-- end -->

```sh
npm --prefix bindings/node run boards
node bindings/node/build/boards/raspberry-pi/relay.js
```

### Python

<!-- snippet: bindings/python/boards/raspberry_pi/relay.py#example -->
From [`bindings/python/boards/raspberry_pi/relay.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/boards/raspberry_pi/relay.py):

```python
import time

from pamoja.gpio import Contact, GpioLine, Level, Switch
from pamoja.kit import Debounce

# The GPIO chip the header's lines live on. Every current model exposes them here, and the
# line numbers below are the BCM numbers the documentation and the kernel both use.
CHIP = "/dev/gpiochip0"
RELAY_LINE = 17
SWITCH_LINE = 27


def main() -> None:
    # Taking the line as an output also says what to drive the moment it is taken. Until
    # then every GPIO is an input, so a relay board sees whatever its own pull gives it;
    # driving the resting level immediately is what keeps a vent from opening at boot.
    # Most relay boards energize on a low input, which is what `active_low` says once so
    # that nothing below this line has to think about the inversion again.
    relay = Switch.active_low(GpioLine.open_output(CHIP, RELAY_LINE, Level.HIGH))

    # The switch is wired to pull the line down when it closes, so it is active low too.
    limit = Contact.active_low(GpioLine.open_input(CHIP, SWITCH_LINE))

    # A mechanical contact bounces for a few milliseconds as it closes. Sampling every
    # 20 ms and requiring three agreeing samples means the state has to hold for 60 ms
    # before it counts, which is longer than the bounce and shorter than a person.
    settled = Debounce(3, False)
    was_closed = False

    print(f"watching GPIO{SWITCH_LINE}, driving GPIO{RELAY_LINE}; Ctrl-C to stop")
    while True:
        closed = settled.update(limit.is_asserted())
        if closed != was_closed:
            print(f"the limit switch {'closed' if closed else 'opened'}")
            # The relay follows the switch. A real vent would run its motor until the
            # limit closes and then stop; this is the same two calls either way.
            relay.set(not closed)
            was_closed = closed
        time.sleep(0.02)
```
<!-- end -->

```sh
python bindings/python/boards/raspberry_pi/relay.py
```

### C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Relay.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Relay.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Relay.cs):

```csharp
using Pamoja.Gpio;
using Pamoja.Kit;

namespace Boards.RaspberryPi;

/// <summary>
/// A relay board on GPIO17 that follows a limit switch on GPIO27, debounced. Wire the relay
/// board's IN to GPIO17 and its VCC and GND to the header's 5V and ground, and the switch
/// between GPIO27 and ground with <c>gpio=27=ip,pu</c> in config.txt.
/// </summary>
public static class Relay
{
    // The GPIO chip the header's lines live on. Every current model exposes them here, and
    // the line numbers below are the BCM numbers the documentation and the kernel both use.
    private const string Chip = "/dev/gpiochip0";
    private const uint RelayLine = 17;
    private const uint SwitchLine = 27;

    /// <summary>Runs until the process is stopped.</summary>
    public static void Run()
    {
        // Taking the line as an output also says what to drive the moment it is taken.
        // Until then every GPIO is an input, so a relay board sees whatever its own pull
        // gives it; driving the resting level immediately is what keeps a vent from opening
        // at boot. Most relay boards energize on a low input, which is what `ActiveLow`
        // says once so that nothing below this line has to think about the inversion again.
        using GpioLine relayLine = GpioLine.OpenOutput(Chip, RelayLine, PinLevel.High);
        var relay = Switch.ActiveLow(relayLine);

        // The switch is wired to pull the line down when it closes, so it is active low too.
        using GpioLine switchLine = GpioLine.OpenInput(Chip, SwitchLine);
        var limit = Contact.ActiveLow(switchLine);

        // A mechanical contact bounces for a few milliseconds as it closes. Sampling every
        // 20 ms and requiring three agreeing samples means the state has to hold for 60 ms
        // before it counts, which is longer than the bounce and shorter than a person.
        using var settled = new Debounce(3, false);
        bool wasClosed = false;

        Console.WriteLine($"watching GPIO{SwitchLine}, driving GPIO{RelayLine}; Ctrl-C to stop");
        while (true)
        {
            bool closed = settled.Update(limit.IsAsserted());
            if (closed != wasClosed)
            {
                Console.WriteLine($"the limit switch {(closed ? "closed" : "opened")}");

                // The relay follows the switch. A real vent would run its motor until the
                // limit closes and then stop; this is the same two calls either way.
                relay.Set(!closed);
                wasClosed = closed;
            }

            Thread.Sleep(20);
        }
    }
}
```
<!-- end -->

```sh
dotnet run --project bindings/dotnet/samples/Pamoja.Boards -- raspberry-pi/relay
```

<!-- languages end -->

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
straight to a profile's control loop, which the last program on this page does.

## A LoRa radio on the SPI bus

The third program puts the Pi on the air. An
[RFM95W breakout](../hardware.md#sx1276), which carries an SX1276, wires to SPI0:
VIN to a 3V3 pin, GND to ground, SCK to GPIO11, MISO to GPIO9, MOSI to GPIO10,
CS to GPIO8, and RST to GPIO25. The breakout regulates its own supply and level
shifts every input, so 3.3 V logic reaches it unchanged. Screw the antenna on
before powering it: a transmitter with nothing on its output reflects its own
power back into the amplifier.

`dtparam=spi=on` makes CS the kernel's own chip select on `/dev/spidev0.0`, and
the reset line is an ordinary GPIO the driver pulses.

### Rust

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

### TypeScript

<!-- snippet: bindings/node/boards/raspberry-pi/radio.ts#example -->
From [`bindings/node/boards/raspberry-pi/radio.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/boards/raspberry-pi/radio.ts):

```typescript
import { LoraRegion, linkBudget, maxTransmitPowerDbm, planFor } from '@pamoja/lora'
import { DutyCycle, LoraRadio, ReceptionOutcome, sx127x } from '@pamoja/radios'

// The header's first SPI chip select, the GPIO chip its lines are on, and the line the
// breakout's reset pin is wired to.
const SPI = '/dev/spidev0.0'
const CHIP = '/dev/gpiochip0'
const RESET_LINE = 25

// The channel this node uses, the data rate it sends at, and how long it listens between
// beacons.
const FREQUENCY_HZ = 868_100_000
const DATA_RATE = 3
const LISTEN_US = 10_000_000

async function main(): Promise<void> {
  // The regional plan decides the channel's power ceiling and its duty cycle, so no limit
  // below is a number anyone has to remember.
  const plan = planFor(LoraRegion.Eu868)
  const link = plan.linkSettings(DATA_RATE)!
  const ceilingDbm = plan.maxEirpDbm(FREQUENCY_HZ)
  const permille = plan.dutyCyclePermille(FREQUENCY_HZ)!

  // A 2.15 dBi whip on half a decibel of pigtail. The antenna's gain counts against the
  // ceiling and the pigtail's loss counts for it, so the amplifier takes what is left.
  const whip = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
  const outputDbm = Math.floor(maxTransmitPowerDbm(whip, ceilingDbm))

  // Opening resets the chip and reads its version back, so a wiring mistake is caught here
  // rather than on the first frame.
  const radio = LoraRadio.openSx127x(
    { spi: SPI, gpioChip: CHIP, resetLine: RESET_LINE },
    { output: sx127x.PaOutput.PaBoost },
  )
  await radio.configure({ frequencyHz: FREQUENCY_HZ, link, outputDbm })
  console.log(
    `beacon on ${FREQUENCY_HZ} Hz at DR${DATA_RATE}, ${outputDbm} dBm under a ${ceilingDbm} dBm ceiling`,
  )

  // The duty cycle is the radio's other budget: each frame buys silence in proportion to its
  // airtime, and the guard says when the next one may go out.
  const duty = new DutyCycle(permille)
  const started = process.hrtime.bigint()
  const nowUs = (): number => Number((process.hrtime.bigint() - started) / 1000n)
  let reading = 0

  for (;;) {
    // Listening returns as soon as a frame arrives, and a frame comes with the levels it was
    // heard at: how strong it was, and how far above the noise.
    const heard = await radio.receive(LISTEN_US)
    if (heard.outcome === ReceptionOutcome.Frame) {
      console.log(
        `heard  ${heard.payload?.toString()} at ${heard.rssiDbm?.toFixed(0)} dBm, SNR ${heard.snrDb?.toFixed(1)} dB`,
      )
    } else if (heard.outcome === ReceptionOutcome.Corrupt) {
      console.log('heard  a frame whose CRC failed')
    }

    const now = nowUs()
    if (duty.ready(now)) {
      const frame = `pi reading ${reading}`
      const airtimeUs = await radio.transmit(Buffer.from(frame))
      duty.transmitted(now, link, frame.length)
      console.log(`sent   ${frame} in ${airtimeUs} us on air`)
      reading += 1
    }
  }
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
```
<!-- end -->

```sh
npm --prefix bindings/node run boards
node bindings/node/build/boards/raspberry-pi/radio.js
```

### Python

<!-- snippet: bindings/python/boards/raspberry_pi/radio.py#example -->
From [`bindings/python/boards/raspberry_pi/radio.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/boards/raspberry_pi/radio.py):

```python
import math
import time

from pamoja.lora import LinkBudget, plan_for
from pamoja.radios import DutyCycle, LoraRadio, sx127x

# The header's first SPI chip select, the GPIO chip its lines are on, and the line the
# breakout's reset pin is wired to.
SPI = "/dev/spidev0.0"
CHIP = "/dev/gpiochip0"
RESET_LINE = 25

# The channel this node uses, the data rate it sends at, and how long it listens between
# beacons.
FREQUENCY_HZ = 868_100_000
DATA_RATE = 3
LISTEN_US = 10_000_000


def main() -> None:
    # The regional plan decides the channel's power ceiling and its duty cycle, so no limit
    # below is a number anyone has to remember.
    plan = plan_for("EU868")
    link = plan.link_settings(DATA_RATE)
    ceiling_dbm = plan.max_eirp_dbm(FREQUENCY_HZ)
    permille = plan.duty_cycle_permille(FREQUENCY_HZ)

    # A 2.15 dBi whip on half a decibel of pigtail. The antenna's gain counts against the
    # ceiling and the pigtail's loss counts for it, so the amplifier takes what is left.
    whip = LinkBudget(transmit_antenna_gain_dbi=2.15, transmit_cable_loss_db=0.5)
    output_dbm = math.floor(whip.max_transmit_power_dbm(ceiling_dbm))

    # Opening resets the chip and reads its version back, so a wiring mistake is caught
    # here rather than on the first frame.
    radio = LoraRadio.open_sx127x(SPI, CHIP, RESET_LINE, sx127x.PaOutput.PA_BOOST)
    with radio:
        radio.configure(FREQUENCY_HZ, link, output_dbm)
        print(
            f"beacon on {FREQUENCY_HZ} Hz at DR{DATA_RATE}, "
            f"{output_dbm} dBm under a {ceiling_dbm} dBm ceiling"
        )

        # The duty cycle is the radio's other budget: each frame buys silence in proportion
        # to its airtime, and the guard says when the next one may go out.
        duty = DutyCycle(permille)
        started = time.monotonic_ns()
        reading = 0

        while True:
            # Listening returns as soon as a frame arrives, and a frame comes with the
            # levels it was heard at: how strong it was, and how far above the noise.
            heard = radio.receive(LISTEN_US)
            if heard.outcome == "Frame":
                print(
                    f"heard  {heard.payload.decode(errors='replace')} at "
                    f"{heard.rssi_dbm:.0f} dBm, SNR {heard.snr_db:.1f} dB"
                )
            elif heard.outcome == "Corrupt":
                print("heard  a frame whose CRC failed")

            now_us = (time.monotonic_ns() - started) // 1000
            if duty.ready(now_us):
                frame = f"pi reading {reading}"
                airtime_us = radio.transmit(frame.encode())
                duty.transmitted(now_us, link, len(frame))
                print(f"sent   {frame} in {airtime_us} us on air")
                reading += 1
```
<!-- end -->

```sh
python bindings/python/boards/raspberry_pi/radio.py
```

### C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Radio.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Radio.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Radio.cs):

```csharp
using System.Diagnostics;
using System.Text;

using Pamoja.Lora;
using Pamoja.Radios;

namespace Boards.RaspberryPi;

/// <summary>
/// A LoRa radio on the header: an RFM95W breakout on SPI0, beaconing a reading and printing
/// every frame it hears in between. Wire the breakout's VIN to a 3V3 pin, GND to ground,
/// SCK to GPIO11, MISO to GPIO9, MOSI to GPIO10, CS to GPIO8 (CE0), and RST to GPIO25, and
/// screw on an antenna for the band before powering it. Two boards running it hear each
/// other.
/// </summary>
public static class Radio
{
    // The header's first SPI chip select, the GPIO chip its lines are on, and the line the
    // breakout's reset pin is wired to.
    private const string Spi = "/dev/spidev0.0";
    private const string Chip = "/dev/gpiochip0";
    private const uint ResetLine = 25;

    // The channel this node uses, the data rate it sends at, and how long it listens
    // between beacons.
    private const uint FrequencyHz = 868_100_000;
    private const byte DataRate = 3;
    private static readonly TimeSpan Listen = TimeSpan.FromSeconds(10);

    /// <summary>Runs until the process is stopped.</summary>
    public static void Run()
    {
        // The regional plan decides the channel's power ceiling and its duty cycle, so no
        // limit below is a number anyone has to remember.
        using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        LoraLink link = plan.LinkSettings(DataRate)!;
        sbyte ceilingDbm = plan.MaxEirpDbm(FrequencyHz);
        uint permille = plan.DutyCyclePermille(FrequencyHz)!.Value;

        // A 2.15 dBi whip on half a decibel of pigtail. The antenna's gain counts against
        // the ceiling and the pigtail's loss counts for it, so the amplifier takes what is
        // left.
        var whip = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };
        sbyte outputDbm = (sbyte)Math.Floor(whip.MaxTransmitPowerDbm(ceilingDbm));

        // Opening resets the chip and reads its version back, so a wiring mistake is caught
        // here rather than on the first frame.
        using LoraRadio radio = LoraRadio.OpenSx127x(
            new LoraRadioWiring(Spi, Chip, ResetLine), new Sx127xBoard(Sx127xPaOutput.PaBoost));
        radio.Configure(new LoraRadioConfig(FrequencyHz, link, outputDbm));
        Console.WriteLine(
            $"beacon on {FrequencyHz} Hz at DR{DataRate}, {outputDbm} dBm under a {ceilingDbm} dBm ceiling");

        // The duty cycle is the radio's other budget: each frame buys silence in proportion
        // to its airtime, and the guard says when the next one may go out.
        using var duty = new RadioDutyCycle(permille);
        var clock = Stopwatch.StartNew();
        int reading = 0;

        while (true)
        {
            // Listening returns as soon as a frame arrives, and a frame comes with the
            // levels it was heard at: how strong it was, and how far above the noise.
            LoraReception heard = radio.Receive(Listen);
            if (heard.Outcome == LoraReceptionOutcome.Frame)
            {
                Console.WriteLine(
                    $"heard  {Encoding.UTF8.GetString(heard.Payload!)} at {heard.RssiDbm:F0} dBm, SNR {heard.SnrDb:F1} dB");
            }
            else if (heard.Outcome == LoraReceptionOutcome.Corrupt)
            {
                Console.WriteLine("heard  a frame whose CRC failed");
            }

            ulong nowUs = (ulong)(clock.Elapsed.Ticks / (TimeSpan.TicksPerMillisecond / 1000));
            if (duty.Ready(nowUs))
            {
                string frame = $"pi reading {reading}";
                ulong airtimeUs = radio.Transmit(Encoding.UTF8.GetBytes(frame));
                duty.Transmitted(nowUs, link, frame.Length);
                Console.WriteLine($"sent   {frame} in {airtimeUs} us on air");
                reading++;
            }
        }
    }
}
```
<!-- end -->

```sh
dotnet run --project bindings/dotnet/samples/Pamoja.Boards -- raspberry-pi/radio
```

<!-- languages end -->

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
[radio guide](../guides/radios.md) walks through every setting it uses.

## The whole node

The fourth program is what a deployed node looks like. It loads a profile from a
file, reads the BME280 through the driver, lets the profile's policy decide,
switches the relay, and publishes each reading to an MQTT broker on the Pi, waiting
whatever the profile's power schedule says between samples. The first tick that
fails ends it, and the service below starts it again.

<!-- snippet: examples/boards/raspberry-pi/src/bin/node.rs#example -->
From [`examples/boards/raspberry-pi/src/bin/node.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/bin/node.rs):

```rust
use pamoja_codec::JsonCodec;
use pamoja_core::{Sensor, Transport};
use pamoja_gpio::{pin::Polarity, switch::Switch};
use pamoja_hal::{digital::PinState, linux};
use pamoja_mqtt::{MqttConfig, MqttTransport};
use pamoja_profile::{Node, Profile};
use pamoja_sensors::bme280::{Bme280, Measurement, I2C_ADDRESS_PRIMARY};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let profile = Profile::from_json(&std::fs::read_to_string("brooder-heater.json")?)?;

    let bus = linux::i2c("/dev/i2c-1")?;
    let mut bme280 = Bme280::i2c(bus, I2C_ADDRESS_PRIMARY, linux::delay());
    bme280.init()?;
    let probe = bme280.map(|measurement: Measurement| measurement.celsius());

    let relay = linux::output("/dev/gpiochip0", 17, "brooder", PinState::High)?;
    let lamp = Switch::new(relay, Polarity::ActiveLow);

    let mut broker = MqttTransport::new(MqttConfig::new("coop-2", "localhost", 1883));
    broker.connect().await?;

    // On mains the charge is full; a node on a panel reads its charge controller here.
    let mut node = Node::new(profile, probe, lamp, broker, JsonCodec)?;
    node.run(|| (1.0, true), tokio::time::sleep).await?;
    Ok(())
}
```
<!-- end -->

```sh
curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/brooder-heater.json
cargo run --release --bin node
```

Nothing in it names a threshold, a deadband, or an interval. Those are in the
manifest, which anyone can read, edit, and share back, and the same manifest
runs on a microcontroller with a different two lines of setup at the top. The
[device profile guide](../guides/profile.md) says what a manifest holds and how a
controller decides with it, and [`examples/brooder_node.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/brooder_node.rs)
is a worked version with a flaky uplink and a second node driven by rules.

This program is the one to start from when a node needs something of its own: a
second probe, a display, a policy the library does not ship. When it does not, the
stock runner does the same work with nothing to compile. Its wiring file names the
BME280's bus and address, the relay's line, and the broker:

```json
{
  "$schema": "https://pamoja.molex.cloud/schema/wiring-1.json",
  "site": "coop-2",
  "profile": "brooder-heater.json",
  "sensor": { "part": "bme280", "bus": "/dev/i2c-1", "address": "0x77" },
  "output": { "gpio": "/dev/gpiochip0", "line": 17, "active_low": true },
  "link": { "mqtt": "192.168.1.10" }
}
```

[Running a profile](../run.md) installs it, tries it with nothing wired, and lists
the parts it reads.

## Running it as a service

A node has to come back after a power cut without anyone logging in, which on a
Pi means a systemd unit. The shape, for the runner:

```ini
[Unit]
Description=pamoja node
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/usr/local/bin/pamoja-node /etc/pamoja/coop-2.json
Restart=on-failure
RestartSec=10
DynamicUser=yes
SupplementaryGroups=i2c gpio

[Install]
WantedBy=multi-user.target
```

Written to `/etc/systemd/system/pamoja-node.service` and enabled with
`sudo systemctl enable --now pamoja-node`. For the program above, `ExecStart`
names its binary instead, and `WorkingDirectory=/etc/pamoja` points it at the folder
that holds the profile. The service runs as an account of its own
in the `i2c` and `gpio` groups, which keeps the earlier point honest: it never
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
- [Sensor drivers](../guides/sensors.md#on-a-board) and
  [actuator drivers](../guides/actuators.md#on-a-board), for every shipped part,
  each with a program for this board: an air sensor with soil probes, and a pan
  and tilt head from a PCA9685 and a 28BYJ-48.
- [Serial framing](../guides/serial.md#on-a-board), for the UART on GPIO14 and
  GPIO15, checked with one jumper wire, and which UART each model puts there.
- [Modbus RTU](../guides/modbus.md#on-a-board), for an RS485 line through a USB
  adapter, and a scan that lists every device on it.
- [CAN and J1939](../guides/can.md#on-a-board), for a CAN bus through an MCP2515 on
  the SPI bus, and a monitor that names each J1939 message it hears.
- [Your own device](../guides/device.md), for a part pamoja has never heard of.
- [Device profiles](../guides/profile.md), for the read-decide-act-publish loop
  a node runs, and the `gateway` and `fleet` examples in `pamoja-dashboard`,
  which serve the dashboard from a Pi.
- [Running a profile](../run.md), for the stock runner and every field of its
  wiring file.
- [Node to dashboard](walkthrough.md), for a Pi that is both a LoRaWAN gateway and
  the dashboard of the nodes its network server hears.

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
