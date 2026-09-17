//! Opening a LoRa radio on a Linux board.
//!
//! On a Raspberry Pi or any Linux board a radio module's SPI bus is a file such as
//! `/dev/spidev0.0`, and its reset and BUSY pins are lines on a GPIO chip such as
//! `/dev/gpiochip0`. [`open_sx126x`] and [`open_sx127x`] open them through the Linux
//! backends of `pamoja-hal`, reset the chip, and hand it back as a [`LinuxRadio`], so a
//! gateway, a Python script, or a C# service drives a radio with no bus code of its own.
//! The kernel's SPI interface has to be turned on first; on a Raspberry Pi that is
//! `dtparam=spi=on` in `config.txt`.
//!
//! Only Linux has spidev and the GPIO character device. Everywhere else this module still
//! builds and [`LinuxRadio`] still names a type, but opening a radio returns
//! [`OpenError::Unsupported`], so a program and the language bindings compile on any
//! platform and say plainly where no radio can be reached.
//!
//! # Examples
//!
//! ```no_run
//! use pamoja_lora::LinkSettings;
//! use pamoja_radios::linux::{self, Wiring};
//! use pamoja_radios::radio::RadioConfig;
//! use pamoja_radios::sx127x::config::PaOutput;
//! use pamoja_radios::sx127x::Board;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // An RFM95W on the Raspberry Pi's SPI0, chip select CE0, with its reset pin on GPIO25.
//! let wiring = Wiring::new("/dev/spidev0.0", "/dev/gpiochip0", 25);
//! let mut radio = linux::open_sx127x(&wiring, Board::new(PaOutput::PaBoost))?;
//! radio.configure(RadioConfig::new(868_100_000, LinkSettings::new(9, 125_000), 14))?;
//! let airtime_us = radio.transmit(b"21.5")?;
//! println!("sent in {airtime_us} us");
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::path::{Path, PathBuf};

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{self, InputPin, OutputPin};
use embedded_hal::spi::{self, Operation, SpiDevice};

use crate::radio::{Radio, RadioError};
use crate::sx1302::bridge::Identity;
use crate::sx1302::{usb, Sx1261, Sx1302};
use crate::{sx126x, sx127x};

/// The SPI mode both families take: CPOL 0 and CPHA 0, the clock idling low and data sampled
/// on its rising edge, as section 8.2 of the SX126x family's datasheets gives it and as
/// RadioLib opens both families.
pub const SPI_MODE: u8 = 0;

/// The SPI clock a radio is opened at unless its wiring names another, in hertz.
///
/// RadioLib opens both families at 2 MHz by default. A slower clock tolerates longer leads
/// between the board and the module, and [`Wiring::with_spi_hz`] sets another.
pub const DEFAULT_SPI_HZ: u32 = 2_000_000;

/// The name the kernel shows as holding a radio's GPIO lines, which `gpioinfo` prints.
pub const CONSUMER: &str = "pamoja-radio";

/// The SPI device a radio is opened on: the kernel's spidev device.
#[cfg(target_os = "linux")]
pub type Spi = pamoja_hal::linux::SpidevDevice;

/// The SPI device a radio is opened on, which only Linux has.
#[cfg(not(target_os = "linux"))]
pub type Spi = Unavailable;

/// A GPIO line a radio's reset or BUSY pin is on: a line of the GPIO character device.
#[cfg(target_os = "linux")]
pub type Line = pamoja_hal::linux::CdevPin;

/// A GPIO line a radio's reset or BUSY pin is on, which only Linux opens.
#[cfg(not(target_os = "linux"))]
pub type Line = Unavailable;

/// The delay a radio's driver waits with: the process sleeping.
#[cfg(target_os = "linux")]
pub type Delay = pamoja_hal::linux::Delay;

/// The delay a radio's driver waits with, which no opened radio needs off Linux.
#[cfg(not(target_os = "linux"))]
pub type Delay = Unavailable;

/// What the SPI device reports when a transfer fails.
#[cfg(target_os = "linux")]
pub type SpiError = pamoja_hal::linux::SPIError;

/// What the SPI device reports when a transfer fails.
#[cfg(not(target_os = "linux"))]
pub type SpiError = spi::ErrorKind;

/// Why the SPI device or a GPIO line could not be opened.
#[cfg(target_os = "linux")]
pub type BusError = pamoja_hal::linux::OpenError;

/// Why the SPI device or a GPIO line could not be opened, which never happens off Linux
/// because nothing is opened there.
#[cfg(not(target_os = "linux"))]
pub type BusError = Unavailable;

/// A radio opened on a Linux board.
pub type LinuxRadio = Radio<Spi, Line, Line, Delay>;

/// A concentrator opened on a Linux board.
///
/// A gateway part rather than a node one: it listens on many channels at once and is driven
/// as a register map, so it is a handle of its own rather than a [`LinuxRadio`].
pub type LinuxConcentrator = Sx1302<Spi, Line, Delay>;

/// The port a USB card's bridge answers on.
#[cfg(target_os = "linux")]
pub type Port = std::fs::File;

/// On any other platform there is no port to open, and this stands in for one.
#[cfg(not(target_os = "linux"))]
pub type Port = Unavailable;

/// A concentrator on a USB card: the same driver as [`LinuxConcentrator`], reached through
/// the card's bridge rather than a bus of the host's own.
pub type UsbConcentrator = Sx1302<usb::BridgeSpi<Port>, usb::BridgePin<Port>, Delay>;

/// The SX1261 beside a concentrator card on the host's SPI, which checks a channel before the
/// gateway talks on it.
pub type LinuxListener = Sx1261<Spi, Line, Delay>;

/// The SX1261 on a USB card, reached through the card's bridge.
pub type UsbListener = Sx1261<usb::BridgeSpi<Port>, usb::BridgePin<Port>, Delay>;

/// A USB card, opened: its concentrator, and the bridge everything on the card shares.
pub struct UsbCard {
    /// The concentrator, reset and answering.
    pub concentrator: UsbConcentrator,
    /// The SX1261 on the card, for a card that carries one, not yet brought up.
    ///
    /// It shares the bridge with the concentrator under its own target and reset pin, so it
    /// is driven while the concentrator runs. [`Sx1261::bring_up`] readies it; a card without
    /// one simply never answers it.
    pub listener: UsbListener,
    /// The bridge, for the radio beside the concentrator and for the card's own status.
    pub bridge: usb::Shared<Port>,
    /// Who the bridge said it was, which a caller compares against the firmware this crate
    /// was written for with [`Identity::matches_firmware`].
    pub identity: Identity,
}

/// The bus, line, and delay of a platform with no spidev or GPIO character device.
///
/// No value of it exists, so a [`LinuxRadio`] can be named on any platform and opened only on
/// Linux.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unavailable {}

impl fmt::Display for Unavailable {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl std::error::Error for Unavailable {}

impl spi::ErrorType for Unavailable {
    type Error = spi::ErrorKind;
}

impl SpiDevice for Unavailable {
    fn transaction(&mut self, _: &mut [Operation<'_, u8>]) -> Result<(), spi::ErrorKind> {
        match *self {}
    }
}

impl digital::ErrorType for Unavailable {
    type Error = digital::ErrorKind;
}

impl InputPin for Unavailable {
    fn is_high(&mut self) -> Result<bool, digital::ErrorKind> {
        match *self {}
    }

    fn is_low(&mut self) -> Result<bool, digital::ErrorKind> {
        match *self {}
    }
}

impl OutputPin for Unavailable {
    fn set_low(&mut self) -> Result<(), digital::ErrorKind> {
        match *self {}
    }

    fn set_high(&mut self) -> Result<(), digital::ErrorKind> {
        match *self {}
    }
}

impl DelayNs for Unavailable {
    fn delay_ns(&mut self, _: u32) {
        match *self {}
    }
}

impl std::io::Read for Unavailable {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        match *self {}
    }
}

impl std::io::Write for Unavailable {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        match *self {}
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match *self {}
    }
}

/// Where a radio module is wired on a Linux board.
///
/// # Examples
///
/// ```
/// use pamoja_radios::linux::{Wiring, DEFAULT_SPI_HZ};
///
/// // An SX1262 board on SPI0's second chip select, BUSY on GPIO24 and reset on GPIO25.
/// let wiring = Wiring::new("/dev/spidev0.1", "/dev/gpiochip0", 25).with_busy_line(24);
/// assert_eq!(wiring.busy_line, Some(24));
/// assert_eq!(wiring.spi_hz, DEFAULT_SPI_HZ);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Wiring {
    /// The SPI device file, one per chip select, such as `/dev/spidev0.0`.
    pub spi: PathBuf,
    /// The SPI clock in hertz.
    pub spi_hz: u32,
    /// The GPIO chip the lines are on, `/dev/gpiochip0` for a Raspberry Pi's header.
    pub gpio_chip: PathBuf,
    /// The line the module's reset pin is on: its offset on the chip, which on a Raspberry Pi
    /// is the BCM GPIO number.
    pub reset_line: u32,
    /// The line an SX126x's BUSY pin is on. The SX127x has no BUSY pin.
    pub busy_line: Option<u32>,
    /// The line that switches a concentrator's supply on, for a board that gates it.
    ///
    /// A card wired this way answers nothing at all until the line is raised, and it has to
    /// stay raised for as long as the card is used.
    pub power_enable_line: Option<u32>,
}

impl Wiring {
    /// Describes a module on an SPI device with its reset pin on a GPIO line, clocked at
    /// [`DEFAULT_SPI_HZ`] and with no BUSY line.
    ///
    /// # Arguments
    ///
    /// * `spi` - the SPI device file.
    /// * `gpio_chip` - the GPIO chip the lines are on.
    /// * `reset_line` - the line the reset pin is on.
    ///
    /// # Returns
    ///
    /// The wiring.
    pub fn new(spi: impl Into<PathBuf>, gpio_chip: impl Into<PathBuf>, reset_line: u32) -> Wiring {
        Wiring {
            spi: spi.into(),
            spi_hz: DEFAULT_SPI_HZ,
            gpio_chip: gpio_chip.into(),
            reset_line,
            busy_line: None,
            power_enable_line: None,
        }
    }

    /// Returns the wiring with a concentrator's supply gated behind a line.
    ///
    /// # Arguments
    ///
    /// * `line` - the line that switches the supply on.
    ///
    /// # Returns
    ///
    /// The wiring.
    pub fn with_power_enable_line(mut self, line: u32) -> Wiring {
        self.power_enable_line = Some(line);
        self
    }

    /// Returns the wiring with an SX126x's BUSY pin on a line.
    ///
    /// # Arguments
    ///
    /// * `line` - the line the BUSY pin is on.
    ///
    /// # Returns
    ///
    /// The wiring.
    pub fn with_busy_line(mut self, line: u32) -> Wiring {
        self.busy_line = Some(line);
        self
    }

    /// Returns the wiring with another SPI clock.
    ///
    /// # Arguments
    ///
    /// * `hz` - the clock in hertz.
    ///
    /// # Returns
    ///
    /// The wiring.
    pub fn with_spi_hz(mut self, hz: u32) -> Wiring {
        self.spi_hz = hz;
        self
    }
}

/// Why a radio could not be opened.
#[derive(Debug)]
pub enum OpenError {
    /// The platform has no spidev or GPIO character device, which is any platform but Linux.
    Unsupported,
    /// An SX126x was opened on wiring that names no BUSY line.
    NoBusyLine,
    /// A device file could not be opened: the SPI device or the GPIO chip.
    Bus {
        /// The file that could not be opened.
        device: PathBuf,
        /// Why, which is most often a kernel interface left off or a missing group.
        error: BusError,
    },
    /// The chip did not come up after its reset, which is most often a wiring or a power
    /// problem.
    Radio(RadioError<SpiError>),
    /// The reset line was opened but could not be driven, so the chip was never reset.
    ResetLine {
        /// The GPIO chip the line is on.
        device: PathBuf,
    },
    /// The bridge on a USB card did not answer, or refused, while the card was brought up.
    Bridge {
        /// The port the card is on.
        device: PathBuf,
        /// What went wrong.
        error: usb::UsbError,
    },
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Unsupported => f.write_str(
                "a LoRa radio or concentrator is opened through spidev and the GPIO character device, which only Linux has",
            ),
            OpenError::ResetLine { device } => write!(
                f,
                "{}: the reset line opened but could not be driven",
                device.display()
            ),
            OpenError::Bridge { device, error } => {
                write!(f, "{}: the bridge on the card: {error}", device.display())
            }
            OpenError::NoBusyLine => {
                f.write_str("an SX126x needs its BUSY line, and the wiring names none")
            }
            OpenError::Bus { device, error } => write!(f, "{}: {error}", device.display()),
            OpenError::Radio(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenError::Bridge { error, .. } => Some(error),
            OpenError::Bus { error, .. } => Some(error),
            OpenError::Radio(error) => Some(error),
            OpenError::Unsupported | OpenError::NoBusyLine | OpenError::ResetLine { .. } => None,
        }
    }
}

/// Opens an SX1261, SX1262, SX1268, or LLCC68 module and resets it.
///
/// # Arguments
///
/// * `wiring` - the SPI device and the lines, which must name the BUSY line.
/// * `board` - how the module wires the chip: its amplifier, clock, antenna switch, and
///   regulator.
///
/// # Returns
///
/// The radio, reset and in standby, ready for [`Radio::configure`].
///
/// # Errors
///
/// Returns [`OpenError::NoBusyLine`] if the wiring names no BUSY line,
/// [`OpenError::Unsupported`] on any platform but Linux, [`OpenError::Bus`] if the SPI device
/// or a line cannot be opened, and [`OpenError::Radio`] if no SX126x answers.
pub fn open_sx126x(wiring: &Wiring, board: sx126x::Board) -> Result<LinuxRadio, OpenError> {
    let busy_line = wiring.busy_line.ok_or(OpenError::NoBusyLine)?;
    platform::open_sx126x(wiring, busy_line, board)
}

/// Opens an SX1276, SX1277, SX1278, or SX1279 module, such as an RFM95W, and resets it into
/// LoRa mode.
///
/// # Arguments
///
/// * `wiring` - the SPI device and the reset line.
/// * `board` - how the module wires the chip: its amplifier output and clock.
///
/// # Returns
///
/// The radio, reset and in standby, ready for [`Radio::configure`].
///
/// # Errors
///
/// Returns [`OpenError::Unsupported`] on any platform but Linux, [`OpenError::Bus`] if the
/// SPI device or the reset line cannot be opened, and [`OpenError::Radio`] if no SX127x
/// answers.
pub fn open_sx127x(wiring: &Wiring, board: sx127x::Board) -> Result<LinuxRadio, OpenError> {
    platform::open_sx127x(wiring, board)
}

/// Opens an SX1302 or SX1303 concentrator and pulses its reset line.
///
/// Unlike a transceiver, a concentrator is not ready when this returns. It answers on the bus,
/// which is what the reset buys, but it hears nothing until its two microcontrollers are given
/// firmware, and those images belong to the caller. So this hands back a handle to check with
/// [`Sx1302::check`] and load with [`Sx1302::load_firmware`], rather than pretending to an
/// initialization it cannot finish.
///
/// # Arguments
///
/// * `wiring` - the SPI device and the reset line. The concentrator has no BUSY pin, so
///   [`busy_line`](Wiring::busy_line) is ignored.
///
/// # Returns
///
/// The concentrator, reset and answering, and the supply line for a board that gates one.
///
/// That second value has to be kept for as long as the concentrator is used. A GPIO line is
/// released when the handle holding it is dropped, so letting it go switches the card off
/// again, and the card then answers nothing while every other part of the configuration
/// looks right. Bind it to a name rather than to `_`.
///
/// # Errors
///
/// Returns [`OpenError::Unsupported`] on any platform but Linux, and [`OpenError::Bus`] if
/// the SPI device, the reset line, or the supply line cannot be opened.
pub fn open_sx1302(wiring: &Wiring) -> Result<(LinuxConcentrator, Option<Line>), OpenError> {
    platform::open_sx1302(wiring)
}

/// Opens the SX1261 beside a concentrator card on SPI.
///
/// The radio has its own chip select, `/dev/spidev0.1` on Semtech's reference card, and its
/// own reset line, GPIO 22 in the reference's `reset_lgw.sh`, and no BUSY line. Its reset line
/// is left high, which is the radio running; [`Sx1261::bring_up`] pulses it low.
///
/// # Arguments
///
/// * `wiring` - the radio's SPI device and reset line. The BUSY and supply lines are ignored.
///
/// # Returns
///
/// The radio, not yet brought up.
///
/// # Errors
///
/// Returns [`OpenError::Unsupported`] on any platform but Linux, and [`OpenError::Bus`] if
/// the SPI device or the reset line cannot be opened.
pub fn open_sx1261(wiring: &Wiring) -> Result<LinuxListener, OpenError> {
    platform::open_sx1261(wiring)
}

/// Opens a concentrator on a USB card, and brings the card up.
///
/// The card enumerates as a serial device, `/dev/ttyACM0` on a Raspberry Pi with nothing
/// else plugged in. The port is opened raw, the bridge is asked who it is and how it is, the
/// supply is switched on and both resets are pulsed in the order the reference does them,
/// and then the concentrator is reset once more through the driver, so it is in the same
/// state a concentrator on SPI is left in by [`open_sx1302`].
///
/// A bridge running a different firmware than this crate was written for is not refused;
/// the reference warns and carries on, and the caller has [`UsbCard::identity`] to do the
/// same.
///
/// # Arguments
///
/// * `port` - the serial device the card is on.
///
/// # Returns
///
/// The card, with its concentrator answering.
///
/// # Errors
///
/// [`OpenError::Unsupported`] on any platform but Linux, [`OpenError::Bus`] when the port
/// cannot be opened, [`OpenError::Bridge`] when the bridge does not answer or refuses a pin,
/// and [`OpenError::ResetLine`] when the concentrator cannot be reset through it.
pub fn open_usb_sx1302(port: impl AsRef<Path>) -> Result<UsbCard, OpenError> {
    platform::open_usb_sx1302(port.as_ref())
}

#[cfg(target_os = "linux")]
mod platform {
    use std::path::Path;

    use embedded_hal::digital::PinState;
    use pamoja_hal::linux;

    use super::{
        Line, LinuxConcentrator, LinuxRadio, OpenError, UsbCard, Wiring, CONSUMER, SPI_MODE,
    };
    use crate::radio::Radio;
    use crate::sx1302::{bridge, usb, Sx1261, Sx1302};
    use crate::{sx126x, sx127x};

    pub(super) fn open_sx126x(
        wiring: &Wiring,
        busy_line: u32,
        board: sx126x::Board,
    ) -> Result<LinuxRadio, OpenError> {
        let spi = spi(wiring)?;
        let busy = linux::input(&wiring.gpio_chip, busy_line, CONSUMER)
            .map_err(|error| bus(&wiring.gpio_chip, error))?;
        let reset = reset(wiring)?;
        start(Radio::Sx126x(sx126x::Sx126x::new(
            spi,
            busy,
            reset,
            linux::delay(),
            board,
        )))
    }

    pub(super) fn open_sx127x(
        wiring: &Wiring,
        board: sx127x::Board,
    ) -> Result<LinuxRadio, OpenError> {
        let spi = spi(wiring)?;
        let reset = reset(wiring)?;
        start(Radio::Sx127x(sx127x::Sx127x::new(
            spi,
            reset,
            linux::delay(),
            board,
        )))
    }

    pub(super) fn open_sx1302(
        wiring: &Wiring,
    ) -> Result<(LinuxConcentrator, Option<Line>), OpenError> {
        // The supply comes first, before the bus and before the reset, which is the order the
        // reference platform script uses. A board that gates its concentrator answers nothing
        // until this is raised.
        let power = match wiring.power_enable_line {
            None => None,
            Some(line) => Some(
                linux::output(&wiring.gpio_chip, line, CONSUMER, PinState::High)
                    .map_err(|error| bus(&wiring.gpio_chip, error))?,
            ),
        };

        let spi = spi(wiring)?;
        let reset = reset(wiring)?;
        let mut concentrator = Sx1302::new(spi, reset, linux::delay());

        // The chip does not reset itself, and nothing it answers means anything until it has.
        concentrator.reset().map_err(|_| OpenError::ResetLine {
            device: wiring.gpio_chip.clone(),
        })?;
        Ok((concentrator, power))
    }

    pub(super) fn open_usb_sx1302(port: &Path) -> Result<UsbCard, OpenError> {
        let file = linux::serial(port, bridge::BAUD).map_err(|error| bus(port, error))?;
        let mut over = usb::Bridge::new(file);
        let failed = |error: usb::UsbError| OpenError::Bridge {
            device: port.to_path_buf(),
            error,
        };

        // The reference asks who the bridge is and how it is before touching a pin; an
        // answer to both is the proof that a bridge, and not something else, is on the port.
        let identity = over.ping().map_err(failed)?;
        over.status().map_err(failed)?;
        over.power_up().map_err(failed)?;

        let shared = usb::Shared::new(over);
        let mut concentrator = Sx1302::new(
            shared.concentrator(),
            shared.pin(bridge::PIN_RESET),
            linux::delay(),
        );
        concentrator.reset().map_err(|_| OpenError::ResetLine {
            device: port.to_path_buf(),
        })?;
        let listener = Sx1261::new(
            shared.listener(),
            shared.pin(bridge::PIN_RADIO_RESET),
            linux::delay(),
        );
        Ok(UsbCard {
            concentrator,
            listener,
            bridge: shared,
            identity,
        })
    }

    pub(super) fn open_sx1261(wiring: &Wiring) -> Result<super::LinuxListener, OpenError> {
        Ok(Sx1261::new(spi(wiring)?, reset(wiring)?, linux::delay()))
    }

    fn spi(wiring: &Wiring) -> Result<linux::SpidevDevice, OpenError> {
        linux::spi(&wiring.spi, SPI_MODE, wiring.spi_hz).map_err(|error| bus(&wiring.spi, error))
    }

    fn reset(wiring: &Wiring) -> Result<linux::CdevPin, OpenError> {
        linux::output(
            &wiring.gpio_chip,
            wiring.reset_line,
            CONSUMER,
            PinState::High,
        )
        .map_err(|error| bus(&wiring.gpio_chip, error))
    }

    fn start(mut radio: LinuxRadio) -> Result<LinuxRadio, OpenError> {
        radio.init().map_err(OpenError::Radio)?;
        Ok(radio)
    }

    fn bus(device: &Path, error: linux::OpenError) -> OpenError {
        OpenError::Bus {
            device: device.to_path_buf(),
            error,
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use super::{LinuxConcentrator, LinuxRadio, OpenError, Wiring};
    use crate::{sx126x, sx127x};

    pub(super) fn open_sx126x(
        _: &Wiring,
        _: u32,
        _: sx126x::Board,
    ) -> Result<LinuxRadio, OpenError> {
        Err(OpenError::Unsupported)
    }

    pub(super) fn open_sx127x(_: &Wiring, _: sx127x::Board) -> Result<LinuxRadio, OpenError> {
        Err(OpenError::Unsupported)
    }

    pub(super) fn open_sx1302(
        _: &Wiring,
    ) -> Result<(LinuxConcentrator, Option<super::Line>), OpenError> {
        Err(OpenError::Unsupported)
    }

    pub(super) fn open_usb_sx1302(_: &std::path::Path) -> Result<super::UsbCard, OpenError> {
        Err(OpenError::Unsupported)
    }

    pub(super) fn open_sx1261(_: &Wiring) -> Result<super::LinuxListener, OpenError> {
        Err(OpenError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sx126x::config::PowerAmplifier;
    use crate::sx127x::config::PaOutput;

    #[test]
    fn wiring_starts_at_the_default_clock_with_no_busy_line() {
        let wiring = Wiring::new("/dev/spidev0.0", "/dev/gpiochip0", 25);
        assert_eq!(wiring.spi_hz, 2_000_000);
        assert_eq!(wiring.busy_line, None);
        let wiring = wiring.with_busy_line(24).with_spi_hz(8_000_000);
        assert_eq!((wiring.busy_line, wiring.spi_hz), (Some(24), 8_000_000));
    }

    #[test]
    fn an_sx126x_is_not_opened_without_its_busy_line() {
        let wiring = Wiring::new("/dev/spidev0.0", "/dev/gpiochip0", 25);
        let refused = open_sx126x(&wiring, sx126x::Board::new(PowerAmplifier::HighPower)).err();
        assert!(matches!(refused, Some(OpenError::NoBusyLine)));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_missing_spi_device_is_named_in_the_error() {
        let wiring = Wiring::new("/dev/spidev-pamoja-absent", "/dev/gpiochip0", 25);
        let refused = open_sx127x(&wiring, sx127x::Board::new(PaOutput::PaBoost))
            .err()
            .expect("no such device exists");
        assert!(matches!(refused, OpenError::Bus { .. }));
        assert!(
            refused
                .to_string()
                .starts_with("/dev/spidev-pamoja-absent: "),
            "{refused}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_concentrator_names_the_device_it_could_not_open() {
        let wiring = Wiring::new("/dev/spidev-pamoja-absent", "/dev/gpiochip0", 23);
        let refused = open_sx1302(&wiring).err().expect("no such device exists");
        assert!(matches!(refused, OpenError::Bus { .. }));
        assert!(
            refused
                .to_string()
                .starts_with("/dev/spidev-pamoja-absent: "),
            "{refused}"
        );
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn only_linux_opens_a_concentrator() {
        // A concentrator needs no BUSY line, so the wiring that opens one names none.
        let wiring = Wiring::new("/dev/spidev0.0", "/dev/gpiochip0", 23);
        let refused = open_sx1302(&wiring).err();
        assert!(matches!(refused, Some(OpenError::Unsupported)));
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn only_linux_opens_a_radio() {
        let wiring = Wiring::new("/dev/spidev0.0", "/dev/gpiochip0", 25);
        let refused = open_sx127x(&wiring, sx127x::Board::new(PaOutput::PaBoost)).err();
        assert!(matches!(refused, Some(OpenError::Unsupported)));
        assert!(OpenError::Unsupported.to_string().contains("only Linux"));
    }
}
