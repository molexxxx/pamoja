//! Opening a GPIO line on a Linux board.
//!
//! On a Raspberry Pi or any Linux board a pin is a line on a GPIO chip, a device file such as
//! `/dev/gpiochip0`, and a line's number on that chip is the one a board's pinout calls its
//! GPIO or BCM number. [`output`] and [`input`] open one through the GPIO character device
//! backend of `pamoja-hal` and hand it back as a [`Line`], which a
//! [`Switch`](crate::switch::Switch) or a [`Contact`](crate::switch::Contact) sits over as
//! it would over any other pin. The language bindings open lines through the same two
//! functions, so a relay is driven the same way from Rust, TypeScript, Python, and C#.
//!
//! A line is held by one process at a time, and the kernel names the holder, which
//! `gpioinfo` prints; a line held by another program or claimed by a kernel driver cannot be
//! opened until it is let go. On Raspberry Pi OS a user in the `gpio` group can open lines
//! without root.
//!
//! Only Linux has the GPIO character device. Everywhere else this module still builds and
//! [`Line`] still names a type, but opening a line returns [`OpenError::Unsupported`], so a
//! program and the bindings compile on any platform and say plainly where no line opens.
//!
//! # Examples
//!
//! ```no_run
//! use pamoja_gpio::linux;
//! use pamoja_gpio::pin::Level;
//! use pamoja_gpio::switch::{Contact, Switch};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // A relay board on GPIO17 that energizes on a low input, opened high so the pump stays
//! // off until it is asked to run, and a float switch to ground on GPIO27 with a pull-up.
//! let mut pump = Switch::active_low(linux::output("/dev/gpiochip0", 17, Level::High)?);
//! let mut float = Contact::active_low(linux::input("/dev/gpiochip0", 27)?);
//! pump.set(!float.is_asserted()?)?;
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::path::{Path, PathBuf};

use embedded_hal::digital::{self, ErrorType, InputPin, OutputPin};

use crate::pin::Level;

/// The name the kernel shows as holding a line opened here, which `gpioinfo` prints.
pub const CONSUMER: &str = "pamoja";

#[cfg(target_os = "linux")]
type Handle = pamoja_hal::linux::CdevPin;

#[cfg(not(target_os = "linux"))]
type Handle = core::convert::Infallible;

/// A GPIO line opened on a Linux board, as an output or an input.
///
/// It implements the `embedded-hal` pin traits, so it goes under a
/// [`Switch`](crate::switch::Switch), a [`Contact`](crate::switch::Contact), or any driver
/// written against them, and [`Line::drive`] and [`Line::read`] work it directly.
pub struct Line {
    handle: Handle,
    chip: PathBuf,
    offset: u32,
}

impl fmt::Debug for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Line")
            .field("chip", &self.chip)
            .field("offset", &self.offset)
            .finish_non_exhaustive()
    }
}

/// Why a line could not be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    /// The platform has no GPIO character device, which is any platform but Linux.
    Unsupported,
    /// The chip or the line could not be opened.
    Gpio {
        /// The chip's device file.
        chip: PathBuf,
        /// The line's number on it.
        line: u32,
        /// Why, which is most often a missing chip, a line number past the chip's last, a
        /// line another program holds, or a user outside the `gpio` group.
        reason: String,
    },
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Unsupported => f.write_str(
                "a GPIO line is opened through the GPIO character device, which only Linux has",
            ),
            OpenError::Gpio { chip, line, reason } => {
                write!(f, "{} line {line}: {reason}", chip.display())
            }
        }
    }
}

impl std::error::Error for OpenError {}

/// Why an open line could not be driven or read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineError {
    /// The chip's device file.
    pub chip: PathBuf,
    /// The line's number on it.
    pub line: u32,
    /// What the kernel reported.
    pub reason: String,
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} line {}: {}",
            self.chip.display(),
            self.line,
            self.reason
        )
    }
}

impl std::error::Error for LineError {}

impl digital::Error for LineError {
    fn kind(&self) -> digital::ErrorKind {
        digital::ErrorKind::Other
    }
}

/// Opens a line as an output, driving `initial` from the moment it is taken.
///
/// Opening at the level that leaves the part off matters for anything that should not
/// twitch at start-up: an active-low relay is opened [`Level::High`].
///
/// # Arguments
///
/// * `chip` - the GPIO chip's device file, `/dev/gpiochip0` on most boards.
/// * `line` - the line's number on that chip, the GPIO or BCM number on a Raspberry Pi.
/// * `initial` - the level to drive as soon as the line is taken.
///
/// # Returns
///
/// The line.
///
/// # Errors
///
/// [`OpenError::Unsupported`] on any platform but Linux, or [`OpenError::Gpio`] when the
/// chip or the line cannot be opened.
pub fn output(chip: impl AsRef<Path>, line: u32, initial: Level) -> Result<Line, OpenError> {
    open(chip.as_ref(), line, Some(initial))
}

/// Opens a line as an input.
///
/// # Arguments
///
/// * `chip` - the GPIO chip's device file.
/// * `line` - the line's number on that chip.
///
/// # Returns
///
/// The line.
///
/// # Errors
///
/// [`OpenError::Unsupported`] on any platform but Linux, or [`OpenError::Gpio`] when the
/// chip or the line cannot be opened.
pub fn input(chip: impl AsRef<Path>, line: u32) -> Result<Line, OpenError> {
    open(chip.as_ref(), line, None)
}

#[cfg(target_os = "linux")]
fn open(chip: &Path, line: u32, initial: Option<Level>) -> Result<Line, OpenError> {
    use embedded_hal::digital::PinState;
    use pamoja_hal::linux;

    let opened = match initial {
        Some(level) => linux::output(chip, line, CONSUMER, PinState::from(level == Level::High)),
        None => linux::input(chip, line, CONSUMER),
    };
    let handle = opened.map_err(|error| OpenError::Gpio {
        chip: chip.to_path_buf(),
        line,
        reason: error.to_string(),
    })?;
    Ok(Line {
        handle,
        chip: chip.to_path_buf(),
        offset: line,
    })
}

#[cfg(not(target_os = "linux"))]
fn open(_chip: &Path, _line: u32, _initial: Option<Level>) -> Result<Line, OpenError> {
    Err(OpenError::Unsupported)
}

impl Line {
    /// The chip the line is on.
    ///
    /// # Returns
    ///
    /// The chip's device file.
    pub fn chip(&self) -> &Path {
        &self.chip
    }

    /// The line's number on its chip.
    ///
    /// # Returns
    ///
    /// The offset it was opened at.
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// Drives the line to a level. The line must have been opened with [`output`].
    ///
    /// # Arguments
    ///
    /// * `level` - the level to drive.
    ///
    /// # Errors
    ///
    /// A [`LineError`] naming the line when the kernel refuses the write, which is what an
    /// input line answers.
    pub fn drive(&mut self, level: Level) -> Result<(), LineError> {
        self.write(level)
    }

    /// Reads the level on the line now.
    ///
    /// # Returns
    ///
    /// The level.
    ///
    /// # Errors
    ///
    /// A [`LineError`] naming the line when the kernel refuses the read.
    pub fn read(&mut self) -> Result<Level, LineError> {
        self.sample()
    }

    #[cfg(target_os = "linux")]
    fn write(&mut self, level: Level) -> Result<(), LineError> {
        let written = match level {
            Level::High => self.handle.set_high(),
            Level::Low => self.handle.set_low(),
        };
        written.map_err(|error| self.failed(error))
    }

    #[cfg(target_os = "linux")]
    fn sample(&mut self) -> Result<Level, LineError> {
        match self.handle.is_high() {
            Ok(high) => Ok(Level::from_bool(high)),
            Err(error) => Err(self.failed(error)),
        }
    }

    #[cfg(target_os = "linux")]
    fn failed(&self, error: impl fmt::Display) -> LineError {
        LineError {
            chip: self.chip.clone(),
            line: self.offset,
            reason: error.to_string(),
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn write(&mut self, _level: Level) -> Result<(), LineError> {
        match self.handle {}
    }

    #[cfg(not(target_os = "linux"))]
    fn sample(&mut self) -> Result<Level, LineError> {
        match self.handle {}
    }
}

impl ErrorType for Line {
    type Error = LineError;
}

impl OutputPin for Line {
    fn set_low(&mut self) -> Result<(), LineError> {
        self.drive(Level::Low)
    }

    fn set_high(&mut self) -> Result<(), LineError> {
        self.drive(Level::High)
    }
}

impl InputPin for Line {
    fn is_high(&mut self) -> Result<bool, LineError> {
        self.read().map(|level| level == Level::High)
    }

    fn is_low(&mut self) -> Result<bool, LineError> {
        self.read().map(|level| level == Level::Low)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn opening_off_linux_is_unsupported() {
        let refused = output("/dev/gpiochip0", 17, Level::High).unwrap_err();
        assert_eq!(refused, OpenError::Unsupported);
        assert!(refused.to_string().contains("only Linux"), "{refused}");
        assert_eq!(
            input("/dev/gpiochip0", 27).unwrap_err(),
            OpenError::Unsupported
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_missing_chip_is_named_in_the_error() {
        let refused = output("/dev/gpiochip-pamoja-absent", 17, Level::High).unwrap_err();
        assert!(
            matches!(&refused, OpenError::Gpio { line: 17, .. }),
            "{refused:?}"
        );
        assert!(
            refused
                .to_string()
                .starts_with("/dev/gpiochip-pamoja-absent line 17: "),
            "{refused}"
        );
        assert!(input("/dev/gpiochip-pamoja-absent", 27).is_err());
    }

    #[test]
    fn a_line_error_names_the_line() {
        let error = LineError {
            chip: PathBuf::from("/dev/gpiochip0"),
            line: 17,
            reason: "Operation not permitted".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "/dev/gpiochip0 line 17: Operation not permitted"
        );
        assert_eq!(digital::Error::kind(&error), digital::ErrorKind::Other);
    }
}
