//! Linux backends: the kernel's I2C, SPI, and GPIO character devices as the bus traits.
//!
//! On a Raspberry Pi or any Linux single-board computer the buses are files:
//! `/dev/i2c-1`, `/dev/spidev0.0`, `/dev/gpiochip0`. These functions open them as the
//! traits every driver is written against, through `linux-embedded-hal`, so a driver
//! moves from a scripted test to a gateway by being handed a different bus and
//! nothing else. The kernel drivers have to be enabled first: on a Raspberry Pi that
//! is `raspi-config`, or `dtparam=i2c_arm=on` and `dtparam=spi=on` in `config.txt`.
//!
//! A DS18B20 is the one part that should not be bit-banged from a Linux process,
//! because user space cannot hold the microsecond slot timing the bus needs. Enable
//! the kernel's own `w1-gpio` driver instead (`dtoverlay=w1-gpio`) and read the
//! thermometer through the sysfs file it exposes, which the DS18B20 driver in
//! `pamoja-sensors` does.
//!
//! # Examples
//!
//! ```no_run
//! use pamoja_hal::i2c::I2c;
//! use pamoja_hal::linux;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // The BME280 on the Raspberry Pi's user I2C bus answers its chip id at 0xD0.
//! const BME280: u8 = 0x76;
//! let mut bus = linux::i2c("/dev/i2c-1")?;
//! let mut id = [0u8; 1];
//! bus.write_read(BME280, &[0xD0], &mut id)?;
//! println!("chip id 0x{:02X}", id[0]);
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::path::Path;

use embedded_hal::digital::PinState;
use linux_embedded_hal::gpio_cdev::{Chip, LineRequestFlags};
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};

pub use linux_embedded_hal::{gpio_cdev, i2cdev, spidev};
pub use linux_embedded_hal::{
    CdevPin, CdevPinError, Delay, I2CError, I2cdev, SPIError, SpidevBus, SpidevDevice,
};

/// Why a bus or a line could not be opened.
#[derive(Debug)]
pub enum OpenError {
    /// The I2C adapter could not be opened.
    I2c(i2cdev::linux::LinuxI2CError),
    /// The SPI device could not be opened or configured.
    Spi(SPIError),
    /// The GPIO chip or line could not be opened or requested.
    Gpio(gpio_cdev::errors::Error),
    /// The SPI mode was not 0, 1, 2, or 3.
    SpiMode(u8),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::I2c(error) => write!(f, "opening the i2c adapter: {error}"),
            OpenError::Spi(error) => write!(f, "opening the spi device: {error}"),
            OpenError::Gpio(error) => write!(f, "opening the gpio line: {error}"),
            OpenError::SpiMode(mode) => write!(f, "spi mode {mode} is not 0, 1, 2, or 3"),
        }
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenError::I2c(error) => Some(error),
            OpenError::Spi(error) => Some(error),
            OpenError::Gpio(error) => Some(error),
            OpenError::SpiMode(_) => None,
        }
    }
}

/// Opens an I2C adapter, such as `/dev/i2c-1`.
///
/// # Arguments
///
/// * `path` - the adapter's device file.
///
/// # Returns
///
/// The adapter, implementing the 7-bit and 10-bit [`I2c`](embedded_hal::i2c::I2c)
/// traits.
///
/// # Errors
///
/// Returns [`OpenError::I2c`] if the file cannot be opened, which usually means the
/// kernel driver is not enabled or the process lacks permission.
pub fn i2c(path: impl AsRef<Path>) -> Result<I2cdev, OpenError> {
    I2cdev::new(path).map_err(OpenError::I2c)
}

/// Opens an SPI device, such as `/dev/spidev0.0`, at a clock mode and speed.
///
/// # Arguments
///
/// * `path` - the device file, one per chip-select line.
/// * `mode` - the clock mode, 0 to 3, as the part's datasheet quotes it.
/// * `max_speed_hz` - the fastest clock the part accepts.
///
/// # Returns
///
/// The device, implementing [`SpiDevice`](embedded_hal::spi::SpiDevice) with 8-bit
/// words.
///
/// # Errors
///
/// Returns [`OpenError::SpiMode`] for a mode above 3, or [`OpenError::Spi`] if the
/// device cannot be opened or configured.
pub fn spi(path: impl AsRef<Path>, mode: u8, max_speed_hz: u32) -> Result<SpidevDevice, OpenError> {
    let flags = match mode {
        0 => SpiModeFlags::SPI_MODE_0,
        1 => SpiModeFlags::SPI_MODE_1,
        2 => SpiModeFlags::SPI_MODE_2,
        3 => SpiModeFlags::SPI_MODE_3,
        other => return Err(OpenError::SpiMode(other)),
    };
    let mut device = SpidevDevice::open(path).map_err(OpenError::Spi)?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(max_speed_hz)
        .mode(flags)
        .build();
    device
        .0
        .configure(&options)
        .map_err(|error| OpenError::Spi(error.into()))?;
    Ok(device)
}

/// Requests a GPIO line as an output.
///
/// # Arguments
///
/// * `chip` - the GPIO chip's device file, `/dev/gpiochip0` on a Raspberry Pi.
/// * `line` - the line offset on that chip, the BCM number on a Raspberry Pi.
/// * `consumer` - the name the kernel shows as holding the line.
/// * `initial` - the level to drive as soon as the line is taken.
///
/// # Returns
///
/// The line, implementing [`OutputPin`](embedded_hal::digital::OutputPin).
///
/// # Errors
///
/// Returns [`OpenError::Gpio`] if the chip or line cannot be opened, or the line is
/// already held by another process or a kernel driver.
pub fn output(
    chip: impl AsRef<Path>,
    line: u32,
    consumer: &str,
    initial: PinState,
) -> Result<CdevPin, OpenError> {
    let handle = Chip::new(chip)
        .and_then(|mut chip| chip.get_line(line))
        .and_then(|line| {
            line.request(
                LineRequestFlags::OUTPUT,
                u8::from(initial == PinState::High),
                consumer,
            )
        })
        .map_err(OpenError::Gpio)?;
    CdevPin::new(handle).map_err(OpenError::Gpio)
}

/// Requests a GPIO line as an input.
///
/// # Arguments
///
/// * `chip` - the GPIO chip's device file.
/// * `line` - the line offset on that chip.
/// * `consumer` - the name the kernel shows as holding the line.
///
/// # Returns
///
/// The line, implementing [`InputPin`](embedded_hal::digital::InputPin).
///
/// # Errors
///
/// Returns [`OpenError::Gpio`] if the chip or line cannot be opened or requested.
pub fn input(chip: impl AsRef<Path>, line: u32, consumer: &str) -> Result<CdevPin, OpenError> {
    let handle = Chip::new(chip)
        .and_then(|mut chip| chip.get_line(line))
        .and_then(|line| line.request(LineRequestFlags::INPUT, 0, consumer))
        .map_err(OpenError::Gpio)?;
    CdevPin::new(handle).map_err(OpenError::Gpio)
}

/// Returns the delay that sleeps the process, for pacing a driver on a gateway.
///
/// # Returns
///
/// A [`DelayNs`](embedded_hal::delay::DelayNs) over `std::thread::sleep`.
pub fn delay() -> Delay {
    Delay
}
