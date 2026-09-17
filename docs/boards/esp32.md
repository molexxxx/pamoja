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

## A LoRa radio on SPI2

The third program puts the chip on the air. An
[RFM95W breakout](../hardware.md#sx1276), which carries an SX1276, wires to six
free pins: SCK to GPIO4, MOSI to GPIO5, MISO to GPIO6, CS to GPIO7, RST to
GPIO10, VIN to 3V3, and GND to ground. None of those is a strapping pin, which is
the reason for choosing them over the pins a board silkscreens as its SPI: the
GPIO matrix routes SPI2 anywhere, and a radio holding GPIO9 low through a reset
would stop the program from starting at all. Screw the antenna on before powering
it.

<!-- snippet: examples/boards/esp32c3/src/bin/radio.rs#example -->
From [`examples/boards/esp32c3/src/bin/radio.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/esp32c3/src/bin/radio.rs):

```rust
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::spi::master::{Config, Spi};
use esp_hal::spi::Mode;
use esp_hal::time::Rate;
use esp_println::println;
use pamoja_lora::LinkSettings;
use pamoja_radios::duty::DutyCycle;
use pamoja_radios::sx127x::config::{PaOutput, TxPower};
use pamoja_radios::sx127x::{Board, RadioConfig, Reception, Sx127x};

/// The channel this node uses, what it sends at, and the share of time it may hold the band.
const FREQUENCY_HZ: u32 = 868_100_000;
const OUTPUT_DBM: i8 = 14;
const DUTY_CYCLE_PERMILLE: u32 = 10;

/// How long the radio listens before it looks at its duty-cycle budget again, in microseconds.
const LISTEN_US: u64 = 10_000_000;

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // The chip's GPIO matrix routes SPI2 to any pins, so the six above are chosen for being
    // free rather than for being an SPI block. Two megahertz is well under what the radio
    // accepts and is what the reference drivers use.
    let bus = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(2))
            .with_mode(Mode::_0),
    )
    .expect("a valid SPI configuration")
    .with_sck(peripherals.GPIO4)
    .with_mosi(peripherals.GPIO5)
    .with_miso(peripherals.GPIO6);

    // The chip select is a plain output the bus drives around each transaction, resting high
    // so the radio ignores the bus until a transaction starts.
    let cs = Output::new(peripherals.GPIO7, Level::High, OutputConfig::default());
    let device = ExclusiveDevice::new(bus, cs, Delay).expect("the chip select takes a level");

    // An RFM95W wires the SX1276's PA_BOOST output to its antenna and clocks it from a
    // crystal, which is what the board description says here.
    let reset = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let mut radio = Sx127x::new(device, reset, Delay, Board::new(PaOutput::PaBoost));
    radio.init().expect("the RFM95W answers on SPI2");

    // SF9 at 125 kHz, which is DR3 in the European plan, at 14 dBm on PA_BOOST.
    let link = LinkSettings::new(9, 125_000);
    let power = TxPower::for_output(PaOutput::PaBoost, OUTPUT_DBM);
    radio
        .configure(RadioConfig::new(FREQUENCY_HZ, link, power))
        .expect("the chip takes the settings");
    println!("beacon on {FREQUENCY_HZ} Hz at SF9, {OUTPUT_DBM} dBm");

    // Each frame buys silence in proportion to its airtime, and the guard says when the next
    // one may go out, counted on the chip's own microsecond timer.
    let mut duty = DutyCycle::new(DUTY_CYCLE_PERMILLE);
    let mut buffer = [0u8; 255];
    let mut reading = 0u32;

    loop {
        match radio.receive(&mut buffer, LISTEN_US) {
            Ok(Reception::Frame { len, status }) => {
                let heard = core::str::from_utf8(&buffer[..len]).unwrap_or("(not text)");
                println!(
                    "heard  {heard} at {} dBm, SNR {} dB",
                    status.rssi_dbm.round_db(),
                    status.snr_db.round_db()
                );
            }
            Ok(Reception::Corrupt) => println!("heard  a frame whose CRC failed"),
            Ok(Reception::Timeout) => {}
            Err(_) => println!("the radio stopped answering"),
        }

        let now_us = Instant::now().duration_since_epoch().as_micros();
        if duty.ready(now_us) {
            let mut frame = [0u8; 32];
            let len = beacon(&mut frame, reading);
            match radio.transmit(&frame[..len]) {
                Ok(airtime_us) => {
                    duty.transmitted(now_us, &link, len);
                    println!("sent   reading {reading} in {airtime_us} us on air");
                    reading += 1;
                }
                Err(_) => println!("the frame did not go out"),
            }
        }
    }
}

/// Writes a beacon into a buffer without allocating, and returns its length.
fn beacon(frame: &mut [u8; 32], reading: u32) -> usize {
    use core::fmt::Write;

    let mut cursor = Cursor { frame, len: 0 };
    let _ = write!(cursor, "esp32 reading {reading}");
    cursor.len
}

/// A writer over a fixed buffer, so a `no_std` node can format a frame.
struct Cursor<'a> {
    frame: &'a mut [u8; 32],
    len: usize,
}

impl core::fmt::Write for Cursor<'_> {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        for byte in text.as_bytes() {
            if self.len == self.frame.len() {
                return Err(core::fmt::Error);
            }
            self.frame[self.len] = *byte;
            self.len += 1;
        }
        Ok(())
    }
}
```
<!-- end -->

```sh
cargo run --release --bin radio
```

The chip select is the one part the radio driver does not own. `embedded-hal`
separates a bus from a device on it, so an `ExclusiveDevice` wraps the SPI bus
with that pin and drives it around each transaction; a second chip on the same
bus would take its own. The rest is the same code a Raspberry Pi runs over
`/dev/spidev0.0`, which is the point of writing drivers against the traits.

## A LoRaWAN node

The fourth program puts the first and third together and joins a network. The
BME280 keeps its I2C pair, so the radio moves over: its MISO to GPIO3, NSS to
GPIO10, and RESET to GPIO20, with SCK and MOSI where they were.

| Part | Pin | GPIO | XIAO ESP32C3 label |
| --- | --- | --- | --- |
| BME280 | SDA | GPIO6 | D4 |
| BME280 | SCL | GPIO7 | D5 |
| RFM95W | SCK | GPIO4 | D2 |
| RFM95W | MOSI | GPIO5 | D3 |
| RFM95W | MISO | GPIO3 | D1 |
| RFM95W | NSS | GPIO10 | D10 |
| RFM95W | RESET | GPIO20 | D7 |

Seven of the nine general purpose pins, and none that is read at reset. GPIO20
is UART0's receive line, which a program that only prints has no use for; the
log goes out over USB.

The device's identifiers and root key come from the environment the program is
built in, as hexadecimal the way a network server shows them: `LORAWAN_DEV_EUI`,
`LORAWAN_JOIN_EUI` and `LORAWAN_APP_KEY`. No key is kept in the source, and a
missing or malformed value stops the build and names the variable. A device takes
the identifiers and key its network registered; the local stack
`cargo xtask chirpstack` raises registers these test values:

```sh
export LORAWAN_DEV_EUI=1111111111111111
export LORAWAN_JOIN_EUI=2222222222222222
export LORAWAN_APP_KEY=33333333333333333333333333333333
```

<!-- snippet: examples/boards/esp32c3/src/bin/lorawan.rs#example -->
From [`examples/boards/esp32c3/src/bin/lorawan.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/esp32c3/src/bin/lorawan.rs):

```rust
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::spi::Mode;
use esp_hal::time::Rate;
use esp_println::println;
use pamoja_lora::region::Region;
use pamoja_lorawan::device::{DeviceError, EndDevice, Settings};
use pamoja_lorawan::{parse_hex, Device, Version};
use pamoja_radios::lorawan::{Node, NodeError, Timer};
use pamoja_radios::sx127x::config::PaOutput;
use pamoja_radios::sx127x::{Board, Sx127x};
use pamoja_sensors::bme280::{Bme280, Measurement, I2C_ADDRESS_PRIMARY};

/// Who the device is to its network, and the root key it shares with it, read from the build
/// environment so that no key is kept in the source. A missing or malformed value stops the
/// build.
const DEV_EUI: [u8; 8] = from_build(env!("LORAWAN_DEV_EUI", "set LORAWAN_DEV_EUI in hex"));
const JOIN_EUI: [u8; 8] = from_build(env!("LORAWAN_JOIN_EUI", "set LORAWAN_JOIN_EUI in hex"));
const APP_KEY: [u8; 16] = from_build(env!("LORAWAN_APP_KEY", "set LORAWAN_APP_KEY in hex"));

/// Reads an identifier or key given in hexadecimal when the program was built.
const fn from_build<const N: usize>(hex: &str) -> [u8; N] {
    match parse_hex(hex) {
        Some(bytes) => bytes,
        None => panic!("an identifier or key takes two hex digits a byte"),
    }
}

/// The application port readings go out on, and how long to wait between them.
const PORT: u8 = 2;
const INTERVAL_US: u64 = 5 * 60 * 1_000_000;

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // The core's own types hold an owned topic and payload, so the binary carries a heap;
    // neither driver, the device, nor the node allocates.
    esp_alloc::heap_allocator!(size: 8 * 1024);

    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .expect("a valid I2C configuration")
        .with_sda(peripherals.GPIO6)
        .with_scl(peripherals.GPIO7);
    let mut sensor = Bme280::i2c(i2c, I2C_ADDRESS_PRIMARY, Delay);
    sensor.init().expect("the BME280 answers on I2C0");

    let bus = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(2))
            .with_mode(Mode::_0),
    )
    .expect("a valid SPI configuration")
    .with_sck(peripherals.GPIO4)
    .with_mosi(peripherals.GPIO5)
    .with_miso(peripherals.GPIO3);
    let cs = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let device = ExclusiveDevice::new(bus, cs, Delay).expect("the chip select takes a level");
    let reset = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());
    let mut radio = Sx127x::new(device, reset, Delay, Board::new(PaOutput::PaBoost));
    radio.init().expect("the RFM95W answers on SPI2");

    // The chip has no entropy of its own without its Wi-Fi radio running, so the node takes it
    // from the LoRa receiver's noise, as LoRaWAN 1.0.3 suggests for the join nonce. The network
    // registration says 1.0.3, whose nonce is random rather than a stored count.
    let seed = radio.random().expect("the RFM95W listens");
    let (min_dbm, max_dbm) = PaOutput::PaBoost.range_dbm();
    let settings = Settings::new(min_dbm, max_dbm)
        .with_version(Version::V1_0_3)
        .with_seed(seed);
    let credentials = Device::new(DEV_EUI, JOIN_EUI, APP_KEY);
    let device = EndDevice::new(Region::Eu868.plan(), credentials, settings)
        .expect("EU868 fits a device's channel table");

    // The node sends, opens both receive windows on time, and sees each uplink through its
    // repeats. The clock and the delay it times them with are the chip's own timer.
    let clock = Timer::new(|| Instant::now().duration_since_epoch().as_micros(), Delay);
    let mut node = Node::new(device, radio, clock);

    while !node.device().is_joined() {
        let nonce = node.radio_mut().random().map_or(0, |noise| noise as u16);
        match node.join(nonce) {
            Ok(true) => println!("joined as {:08X}", node.device().dev_addr().unwrap_or(0)),
            Ok(false) => println!("no join accept yet"),
            Err(NodeError::Device(DeviceError::Wait { until_us })) => node.wait_until(until_us),
            Err(error) => println!("join failed: {error}"),
        }
    }

    loop {
        let started = Instant::now().duration_since_epoch().as_micros();
        match sensor.measure() {
            Ok(measurement) => {
                let payload = encode(&measurement);
                match node.send(PORT, &payload, false) {
                    Ok(report) => {
                        println!(
                            "sent {:.2} C, {:.2} hPa, {:.2} % as uplink {} ({} transmissions)",
                            measurement.celsius(),
                            measurement.hectopascals(),
                            measurement.relative_humidity_percent(),
                            node.device().fcnt_up() - 1,
                            report.transmissions,
                        );
                        if let Some(delivery) = report.delivery {
                            println!(
                                "heard {} bytes on port {:?}",
                                delivery.payload().len(),
                                delivery.port()
                            );
                        }
                    }
                    Err(NodeError::Device(DeviceError::Wait { until_us })) => {
                        node.wait_until(until_us);
                        continue;
                    }
                    Err(error) => println!("the uplink failed: {error}"),
                }
            }
            Err(_) => println!("the BME280 did not answer"),
        }
        node.wait_until(started + INTERVAL_US);
    }
}

/// Packs a reading into seven bytes, most significant first: the temperature in hundredths
/// of a degree as a signed 16-bit value, the relative humidity in hundredths of a percent as
/// an unsigned 16-bit value, and the pressure in pascals in 24 bits.
fn encode(measurement: &Measurement) -> [u8; 7] {
    let celsius = hundredths(measurement.celsius()) as i16;
    let humidity = hundredths(measurement.relative_humidity_percent()) as u16;
    let pascals = measurement.pascals().min(0x00FF_FFFF);

    let mut payload = [0u8; 7];
    payload[..2].copy_from_slice(&celsius.to_be_bytes());
    payload[2..4].copy_from_slice(&humidity.to_be_bytes());
    payload[4..].copy_from_slice(&pascals.to_be_bytes()[1..]);
    payload
}

/// A value in hundredths, rounded to the nearest one.
fn hundredths(value: f32) -> i32 {
    let scaled = value * 100.0;
    if scaled >= 0.0 {
        (scaled + 0.5) as i32
    } else {
        (scaled - 0.5) as i32
    }
}
```
<!-- end -->

```sh
cargo run --release --bin lorawan
```

Three pieces do the work. `EndDevice`, from `pamoja-lorawan`, is the LoRaWAN
device with no radio attached: it picks a channel and data rate for each frame,
keeps the region's duty cycle, answers the network's MAC commands and says when
the two receive windows open. `Node`, from `pamoja-radios`, puts each frame on the
air and opens those windows on time, sized as Semtech's LoRaMac-node sizes them to
catch the downlink's preamble despite a few milliseconds of timing error either
way. And the radio itself supplies the join nonce: without Wi-Fi running, the
chip has no source of entropy in `esp-hal`'s stable set, so the program reads it
from the receiver's noise, which is what LoRaWAN 1.0.3 suggests for a device with
no other.

The region is one argument. A node on a 915 MHz network fits an RFM95W built for
that band, names `Region::Us915` or `Region::Au915`, and swaps `pamoja-lora`'s
`eu868` feature for that region's. The device then joins the way those plans lay
out, eight 125 kHz channels from successive groups and then a 500 kHz one, and
listens for each answer on the downlink channel its uplink maps to. A network in
China names one of the five plans of `Cn470Plan`.

The seven bytes a reading travels in are the temperature in hundredths of a
degree, the humidity in hundredths of a percent and the pressure in pascals,
each most significant byte first. A downlink comes back in the same call that
sent the uplink, and the program prints its port and length.

## Sleeping between samples

A node on a battery spends almost all of its life asleep. `esp-hal` keeps deep
sleep behind its `unstable` feature, so it is not in the program above, but the
shape a profile expects is already here: `pamoja-power`'s schedule answers how
long to wait given the charge left, and that interval is what a chip sleeps for
rather than what it spins in a delay loop. The
[power guide](../guides/power.md) covers the schedule; the wake-up itself is
`esp-hal`'s, through its low-power interfaces. A LoRaWAN node keeps its session
through the sleep: `EndDevice::save` turns what the device settled with its network
into bytes to keep in retained memory or flash, and `resume` puts them back on
waking, so it carries on with its counters and channels rather than joining again.
The bytes hold the session keys, so they belong wherever the keys would be safe.

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
- [Gateway](gateway.md), for the other end of the LoRaWAN node's link: a
  concentrator on a Raspberry Pi forwarding what it hears to a network server.
- [Node to dashboard](walkthrough.md), for the whole path the node's readings
  take: gateway, ChirpStack, and a dashboard.
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
- [TS001-1.0.4](../about/standards.md#ts001-1-0-4) and
  [RP002-1.0.5](../about/standards.md#rp002-1-0-5), for the device the LoRaWAN
  node runs, and LoRaWAN 1.0.3 section 6.2.4 for drawing its join nonce from
  receiver noise.
- [LoRaMac-node](https://github.com/Lora-net/LoRaMac-node), Semtech's reference
  implementation, for the receive window sizing and for reading random numbers
  from the SX1276 and SX126x.
