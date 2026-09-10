# Raspberry Pi

A Raspberry Pi is the gateway-class board: it runs Linux, so it runs the full
`std` build of pamoja, every transport, and the dashboard, and it reaches the
parts on its 40-pin header through the kernel's own bus drivers. Everything on
this page holds for any model with that header, from the
[Pi 5](../hardware.md#raspberry-pi-5) to the
[Zero 2 W](../hardware.md#raspberry-pi-zero-2-w).

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
| UART | TX | GPIO14 |
| UART | RX | GPIO15 |
| PWM | hardware channels | GPIO12, GPIO13, GPIO18, GPIO19 |
| 1-Wire | data, by default | GPIO4 |

GPIO2 and GPIO3 have fixed pull-up resistors on the board, so an I2C breakout
needs none of its own. Physical pin numbers are a different numbering that runs
down the header in pairs; the `pinout` command, run in a terminal on the Pi,
prints the whole header with both numberings, and is the reference to have open
while wiring.

## Wiring a BME280

A BME280 breakout has four pins that matter. Wire VIN to a 3V3 pin, GND to a
ground pin, SDA to GPIO2, and SCL to GPIO3. The chip answers at `0x76` unless
the breakout's address jumper moves it to `0x77`; the driver's
`I2C_ADDRESS_PRIMARY` and `I2C_ADDRESS_SECONDARY` are those two.

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

## The first program

The program below is a complete package at
[`examples/boards/raspberry-pi`](https://github.com/molexxxx/pamoja/tree/main/examples/boards/raspberry-pi),
built in CI on every change. It opens the header's I2C bus, hands it to the
BME280 driver, and prints a compensated reading every two seconds.

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

## Where next

- [Buses and links](../buses.md), for what each bus on the header is for,
  and the [bus layer guide](../guides/hal.md) for the traits, the scripted bus,
  and the Linux backend in detail.
- [Sensor drivers](../guides/sensors.md) and
  [actuator drivers](../guides/actuators.md), for every shipped part.
- [Your own device](../guides/device.md), for a part pamoja has never heard of.
- [Device profiles](../guides/profile.md), for the read-decide-act-publish loop
  a node runs, and the `gateway` example in `pamoja-dashboard`, which serves
  the dashboard from a Pi.

## Sources

- [GPIO on Raspberry Pi](https://github.com/raspberrypi/documentation/blob/master/documentation/asciidoc/computers/raspberry-pi/gpio-on-raspberry-pi.adoc),
  the source of the Raspberry Pi documentation, for the header's GPIO numbers,
  the pull-ups, the voltage, and the `pinout` command.
- [SPI on Raspberry Pi](https://github.com/raspberrypi/documentation/blob/master/documentation/asciidoc/computers/raspberry-pi/spi-bus-on-raspberry-pi.adoc),
  for the device files, the pins, and how the kernel driver handles chip select.
- The firmware's
  [overlay reference](https://github.com/raspberrypi/firmware/blob/master/boot/overlays/README),
  for the `i2c_arm`, `spi`, and `w1-gpio` settings and their defaults.
