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

use nix::sys::termios::{self, BaudRate, ControlFlags, FlushArg, SetArg, SpecialCharacterIndices};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
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
    /// A serial port could not be opened, or could not be set up as a raw byte pipe.
    Serial(std::io::Error),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::I2c(error) => write!(f, "opening the i2c adapter: {error}"),
            OpenError::Spi(error) => write!(f, "opening the spi device: {error}"),
            OpenError::Gpio(error) => write!(f, "opening the gpio line: {error}"),
            OpenError::SpiMode(mode) => write!(f, "spi mode {mode} is not 0, 1, 2, or 3"),
            OpenError::Serial(error) => write!(f, "opening the serial port: {error}"),
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
            OpenError::Serial(error) => Some(error),
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

/// The serial port a USB card's bridge answers on, opened raw.
///
/// A CDC-ACM device such as `/dev/ttyACM0` is a terminal as far as the kernel is concerned,
/// and a terminal echoes, translates line endings, and buffers by line, every one of which
/// would mangle a binary protocol. This opens the port and turns all of that off: eight data
/// bits, no parity, one stop bit, no flow control, and a read that returns once at least one
/// byte has arrived. Whatever the card sent before anyone was listening is dropped.
///
/// The speed is nominal, since a USB serial port has none, but the port has to be given one.
///
/// # Arguments
///
/// * `path` - the device file.
/// * `baud` - the nominal speed, one of the standard rates from 9600 to 921600.
///
/// # Returns
///
/// The open port, which reads and writes bytes.
///
/// # Errors
///
/// [`OpenError::Serial`] when the file cannot be opened, the rate is not a standard one, or
/// the terminal settings cannot be read or applied.
///
/// # Examples
///
/// ```no_run
/// use pamoja_hal::linux;
///
/// let port = linux::serial("/dev/ttyACM0", 115_200)?;
/// # Ok::<(), linux::OpenError>(())
/// ```
pub fn serial(path: impl AsRef<Path>, baud: u32) -> Result<File, OpenError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(OpenError::Serial)?;
    raw(&file, baud).map_err(OpenError::Serial)?;
    Ok(file)
}

// Turns a terminal into a byte pipe at one speed.
fn raw(file: &File, baud: u32) -> io::Result<()> {
    let speed = match baud {
        9_600 => BaudRate::B9600,
        19_200 => BaudRate::B19200,
        38_400 => BaudRate::B38400,
        57_600 => BaudRate::B57600,
        115_200 => BaudRate::B115200,
        230_400 => BaudRate::B230400,
        460_800 => BaudRate::B460800,
        921_600 => BaudRate::B921600,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a standard serial rate",
            ))
        }
    };

    let mut tty = termios::tcgetattr(file)?;
    termios::cfmakeraw(&mut tty);
    termios::cfsetispeed(&mut tty, speed)?;
    termios::cfsetospeed(&mut tty, speed)?;
    tty.control_flags |= ControlFlags::CLOCAL | ControlFlags::CREAD;
    tty.control_flags &= !(ControlFlags::PARENB | ControlFlags::CSTOPB | ControlFlags::CRTSCTS);
    tty.control_flags &= !ControlFlags::CSIZE;
    tty.control_flags |= ControlFlags::CS8;
    tty.control_chars[SpecialCharacterIndices::VMIN as usize] = 1;
    tty.control_chars[SpecialCharacterIndices::VTIME as usize] = 1;
    termios::tcsetattr(file, SetArg::TCSANOW, &tty)?;
    termios::tcflush(file, FlushArg::TCIOFLUSH)?;
    Ok(())
}
