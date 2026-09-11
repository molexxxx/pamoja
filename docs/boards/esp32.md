# ESP32

The ESP32 family is the microcontroller behind most small sensor nodes: Wi-Fi
and Bluetooth on the chip, the buses a sensor needs, and boards that cost a few
dollars. There is no operating system, so a program is a `no_std` binary built
for the chip, and pamoja's `no_std` crates, the drivers among them, run on it
over the chip's own peripherals through `esp-hal`, Espressif's Rust hardware
layer.

Two toolchains cover the family. The original ESP32 and the S2 and S3 have
Xtensa cores, which need Espressif's own build of the Rust compiler, installed
with `espup`. The C series and the H2 and P4 have RISC-V cores, which the
stock Rust toolchain targets on its stable channel. This page works through
the ESP32-C3, the RISC-V part most breakout boards carry, and its two programs
are the ones built in CI.

## The ESP32-C3

From the chip's datasheet: a 32-bit RISC-V single-core processor at up to 160
MHz, 22 programmable GPIOs, two UARTs, three SPI controllers, an I2C
controller, a TWAI controller compatible with ISO 11898-1 (CAN), a LED PWM
controller with six channels, two 12-bit SAR ADCs, and 2.4 GHz Wi-Fi with
Bluetooth 5 (LE). The chip's I/O is 3.3 V, and its GPIO matrix routes the I2C
controller to any pair of pins, so a board decides which two carry the bus.

`esp-hal` names the target for this chip `riscv32imc-unknown-none-elf`, and
its I2C and GPIO drivers implement the `embedded-hal` traits every pamoja
driver is written against.

## Which pins are actually free

The GPIO matrix will route almost anything anywhere, which makes it easy to
pick a pin that works on the bench and breaks the board on the next reset.
Three groups of pins on this chip are spoken for, and `esp-hal`'s own chip
metadata marks each one:

| Pins | What they are | Safe to use? |
| --- | --- | --- |
| GPIO11 to GPIO17 | the SPI flash the program runs from | never |
| GPIO2, GPIO8, GPIO9 | strapping pins, read at reset | only with care |
| GPIO18, GPIO19 | the USB Serial/JTAG interface | not while flashing over USB |
| GPIO20, GPIO21 | UART0, the boot log | not while watching the log |
| GPIO0 to GPIO7, GPIO10 | general purpose | yes |

The strapping pins are the trap. Their level at the moment of reset selects the
boot mode, and GPIO9 is the one that puts the chip into download mode when it
is held low: a button wired between GPIO9 and ground, pressed while the board
resets, stops the program from starting at all. That is why the program below
puts its button on GPIO3 and its relay on GPIO10, neither of which is read at
reset.

The analog inputs split the same way. ADC1 is on GPIO0 to GPIO4 and is the one
to use; ADC2, on GPIO5, shares hardware with the Wi-Fi radio and returns
nonsense while the radio is on, which the board's own documentation warns
about.

## Wiring a BME280

On a Seeed XIAO ESP32C3, the board's own documentation labels GPIO6 as D4 and
GPIO7 as D5, its I2C pair. Wire the BME280's SDA to GPIO6, its SCL to GPIO7,
VIN to the board's 3V3, and GND to ground. On another ESP32-C3 board any two
free GPIOs serve; change the two pin names in the program to match. The chip
answers at `0x76` unless the breakout's jumper moves it to `0x77`.

## The toolchain

```sh
rustup target add riscv32imc-unknown-none-elf
cargo install espflash
```

`espflash` flashes the chip over its USB serial port and opens a monitor on it;
the program's `.cargo/config.toml` names it as the runner, so `cargo run` does
both. For a project of your own, Espressif's generator,
`cargo install esp-generate --locked`, lays out a package for any chip in the
family; the package below is the complete, smaller shape for this one.

## Reading the sensor

The first program is a complete package at
[`examples/boards/esp32c3`](https://github.com/molexxxx/pamoja/tree/main/examples/boards/esp32c3),
cross-compiled in CI on every change. It brings up the chip, opens the I2C
controller on GPIO6 and GPIO7, hands it to the BME280 driver, and prints a
compensated reading over the USB serial port every two seconds.

<!-- snippet: examples/boards/esp32c3/src/main.rs#example -->
From [`examples/boards/esp32c3/src/main.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/esp32c3/src/main.rs):

```rust
use esp_hal::i2c::master::{Config, I2c};
use esp_println::println;
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // The core's own types hold an owned topic and payload, so the binary carries a heap;
    // the driver itself never allocates.
    esp_alloc::heap_allocator!(size: 8 * 1024);

    // The chip routes I2C0 to any two pins; GPIO6 and GPIO7 are the pair the XIAO ESP32C3
    // labels D4 and D5. The default configuration is the 100 kHz standard mode.
    let i2c = I2c::new(peripherals.I2C0, Config::default())
        .expect("a valid I2C configuration")
        .with_sda(peripherals.GPIO6)
        .with_scl(peripherals.GPIO7);

    let mut sensor = Bme280::i2c(i2c, I2C_ADDRESS_PRIMARY, Delay);
    sensor.init().expect("the BME280 answers on I2C0");

    loop {
        let measurement = sensor.measure().expect("a measurement");
        println!(
            "{:.2} C, {:.2} hPa, {:.2} % humidity",
            measurement.celsius(),
            measurement.hectopascals(),
            measurement.relative_humidity_percent()
        );
        Delay.delay_ms(2000);
    }
}
```
<!-- end -->

Plug the board in and run it from that directory:

```sh
cd examples/boards/esp32c3
cargo run --release
```

Three things in the source, outside the snippet or at the top of `main`, are
the chip's own requirements. A panic resets the chip, which is the honest state
for a node with nobody watching. The delay the driver takes is written in six
lines over the chip's microsecond timer, since the driver asks only for the
`embedded-hal` trait and `esp-hal` 1.2 keeps its ready-made delay behind its
`unstable` feature, along with most of the peripherals: GPIO, I2C, SPI, UART,
and timekeeping are the stable set, so a program built on those keeps working
across releases.

## A thermostat on the chip

The second program is a whole node. It reads the same sensor, decides with the
deadband math from `pamoja-kit`, switches a relay on a GPIO line, and takes a
button as a manual override. There is no gateway in it and no network: the
chip decides for itself, which is what a node has to do when the link is down.

<!-- snippet: examples/boards/esp32c3/src/bin/thermostat.rs#example -->
From [`examples/boards/esp32c3/src/bin/thermostat.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/esp32c3/src/bin/thermostat.rs):

```rust
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::{Config, I2c};
use esp_println::println;
use pamoja_gpio::pin::Polarity;
use pamoja_gpio::switch::{Contact, Switch};
use pamoja_kit::{Debounce, Thermostat};
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

/// Hold this temperature, switching a heater on half a degree below it and off half a
/// degree above. The deadband is what stops a relay chattering at the threshold.
const SETPOINT_C: f32 = 21.0;
const DEADBAND_C: f32 = 0.5;

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // The core's own types hold an owned topic and payload, so the binary carries a heap;
    // the driver and the control math never allocate.
    esp_alloc::heap_allocator!(size: 8 * 1024);

    // The chip's GPIO matrix routes I2C0 to any two pins.
    let i2c = I2c::new(peripherals.I2C0, Config::default())
        .expect("a valid I2C configuration")
        .with_sda(peripherals.GPIO6)
        .with_scl(peripherals.GPIO7);
    let mut sensor = Bme280::i2c(i2c, I2C_ADDRESS_PRIMARY, Delay);
    sensor.init().expect("the BME280 answers on I2C0");

    // The relay. Its initial level is what the pin drives the instant it is configured,
    // so starting it at the resting level is what keeps a heater off through boot. Most
    // relay boards energize on a low input, which `ActiveLow` states once here.
    let relay = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let mut heater = Switch::new(relay, Polarity::ActiveLow);

    // The button, wired to ground, so the chip's own pull-up holds the line high until it
    // is pressed. Without that pull the line floats and reads as noise.
    let pin = Input::new(peripherals.GPIO3, InputConfig::default().with_pull(Pull::Up));
    let mut button = Contact::new(pin, Polarity::ActiveLow);
    let mut settled = Debounce::new(4, false);

    // The decision. A heating thermostat switches on below the band and off above it.
    let mut control = Thermostat::heating(SETPOINT_C, DEADBAND_C);
    let mut override_on = false;
    let mut was_pressed = false;

    loop {
        // The button is polled every 50 ms for the second between measurements, so a
        // press is caught rather than missed while the sensor is being read.
        for _ in 0..20 {
            let pressed = settled.update(button.is_asserted().unwrap_or(false));
            if pressed && !was_pressed {
                override_on = !override_on;
                println!("override {}", if override_on { "on" } else { "off" });
            }
            was_pressed = pressed;
            Delay.delay_ms(50);
        }

        let measurement = sensor.measure().expect("a measurement");
        let celsius = measurement.celsius();

        // The thermostat is fed every reading, so its deadband keeps working even while
        // the override holds the relay; releasing it resumes mid-cycle, not from cold.
        let wanted = control.update(celsius);
        let on = wanted || override_on;
        heater.set(on).expect("the relay line takes it");

        println!(
            "{celsius:.2} C, {:.1} % humidity, heater {}{}",
            measurement.relative_humidity_percent(),
            if on { "on" } else { "off" },
            if override_on { " (override)" } else { "" }
        );
    }
}
```
<!-- end -->

```sh
cargo run --release --bin thermostat
```

Four things in it are worth reading twice.

**The output's initial level.** `Output::new` takes the level to drive the
instant the pin is configured. Getting that wrong runs a heater through every
boot, and on an active-low relay board the resting level is the high one.

**The pull on the input.** A button wired to ground leaves the line floating
when it is not pressed, and a floating input reads as noise. The chip's own
pull-up holds it high, which is what `InputConfig::default().with_pull(Pull::Up)`
turns on; `Pull::Down` is the mirror image for a button wired to 3.3 V.

**The polarity, stated once.** `Polarity::ActiveLow` on the relay and on the
button says that a low line means asserted, and nothing after that line inverts
anything by hand. A relay board that energizes on a high input is a one-word
change.

**The debounce.** A mechanical button makes and breaks contact several times
over a few milliseconds, so a raw read turns one press into a burst. Requiring
four agreeing samples 50 ms apart turns the burst back into one press.

The same `Switch` and `Contact` types run on a Raspberry Pi over the kernel's
GPIO character device, and the same `Thermostat` is what a profile's setpoint
policy runs on a gateway. That is the point of the layering: what changes
between a five-dollar chip and a Linux box is the two lines that open the bus
and take the pin.

## Sleeping between samples

A node on a battery spends almost all of its life asleep. `esp-hal` keeps deep
sleep behind its `unstable` feature, so it is not in the program above, but the
shape a profile expects is already here: `pamoja-power`'s schedule answers how
long to wait given the charge left, and that interval is what a chip sleeps for
rather than what it spins in a delay loop. The
[power guide](../guides/power.md) covers the schedule; the wake-up itself is
`esp-hal`'s, through its low-power interfaces.

## Where next

- [Buses and links](../buses.md), for what each bus on the chip is for, and
  the [bus layer guide](../guides/hal.md) for the traits and the scripted bus a
  driver is tested against with nothing plugged in.
- [Sensor drivers](../guides/sensors.md) and
  [actuator drivers](../guides/actuators.md), for every shipped part.
- [Raspberry Pi](raspberry-pi.md), for the same two programs on a board that
  runs Linux, plus the whole profile-driven node.
- [LoRa airtime](../guides/lora.md) and [LoRaWAN](../guides/lorawan.md), for a
  node that reports over a radio rather than Wi-Fi; the
  [hardware page](../hardware.md#radios-and-long-range-links) lists an ESP32
  board with a LoRa radio on it.
- [Your own device](../guides/device.md), for a part pamoja has never heard of.

## Sources

- [ESP32-C3 Series Datasheet](https://documentation.espressif.com/esp32-c3_datasheet_en.pdf),
  version 2.4, for the chip's core, peripherals, and I/O.
- [`esp-hal`](https://docs.rs/esp-hal/latest/esp_hal/) on docs.rs, version
  1.2.1, for the entry point, the I2C and GPIO drivers and their `embedded-hal`
  implementations, and which modules are stable rather than behind `unstable`;
  and its
  [chip metadata](https://github.com/esp-rs/esp-hal/blob/main/esp-metadata/devices/esp32c3/gpio.toml)
  for the target triple and for which pins are strapping pins or belong to the
  SPI flash.
- [`esp-generate`](https://github.com/esp-rs/esp-generate), for the project
  layout and the toolchain per chip family.
- [Seeed Studio XIAO ESP32C3](https://wiki.seeedstudio.com/XIAO_ESP32C3_Getting_Started/),
  the board's own documentation, for its pin labels, its boot button on GPIO9,
  and the warning about ADC2.
