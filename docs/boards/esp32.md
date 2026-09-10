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
the ESP32-C3, the RISC-V part most breakout boards carry, and its program is
the one built in CI.

## The ESP32-C3

From the chip's datasheet: a 32-bit RISC-V single-core processor at up to 160
MHz, 22 programmable GPIOs, two UARTs, three SPI controllers, an I2C
controller, a TWAI controller compatible with ISO 11898-1 (CAN), a LED PWM
controller with six channels, two 12-bit SAR ADCs, and 2.4 GHz Wi-Fi with
Bluetooth 5 (LE). The chip's I/O is 3.3 V, and its GPIO matrix routes the I2C
controller to any pair of pins, so a board decides which two carry the bus.

`esp-hal` names the target for this chip `riscv32imc-unknown-none-elf`, and
its I2C driver implements the `embedded-hal` traits every pamoja driver is
written against.

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

## The first program

The program below is a complete package at
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
`unstable` feature. And the binary declares a small heap through `esp-alloc`:
the core's own types hold an owned topic and payload, so a `no_std` binary that
links them needs an allocator, though the driver itself never allocates.

## Where next

- [Buses and links](../buses.md), for what each bus on the chip is for, and
  the [bus layer guide](../guides/hal.md) for the traits and the scripted bus a
  driver is tested against with nothing plugged in.
- [Sensor drivers](../guides/sensors.md) and
  [actuator drivers](../guides/actuators.md), for every shipped part.
- [LoRa airtime](../guides/lora.md) and [LoRaWAN](../guides/lorawan.md), for a
  node that reports over a radio rather than Wi-Fi; the
  [hardware page](../hardware.md#radios-and-long-range-links) lists an ESP32
  board with a LoRa radio on it.
- [Your own device](../guides/device.md), for a part pamoja has never heard of.

## Sources

- [ESP32-C3 Series Datasheet](https://documentation.espressif.com/esp32-c3_datasheet_en.pdf),
  version 2.4, for the chip's core, peripherals, and I/O.
- [`esp-hal`](https://docs.rs/esp-hal/latest/esp_hal/) on docs.rs, version
  1.2.1, for the entry point, the I2C driver and its `embedded-hal`
  implementation, and the delay; and its
  [chip metadata](https://github.com/esp-rs/esp-hal/blob/main/esp-metadata/devices/esp32c3/soc.toml)
  for the target triple.
- [`esp-generate`](https://github.com/esp-rs/esp-generate), for the project
  layout and the toolchain per chip family.
- [Seeed Studio XIAO ESP32C3](https://wiki.seeedstudio.com/XIAO_ESP32C3_Getting_Started/),
  the board's own documentation, for its pin labels.
