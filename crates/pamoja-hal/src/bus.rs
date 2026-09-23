//! One I2C bus, shared by a program and every driver on it.
//!
//! A driver takes its bus by value, which is the right shape on a microcontroller: the program
//! owns the peripheral and hands it over once. A program on a host works differently. A gateway
//! keeps several parts on `/dev/i2c-1`, gives the same bus to a driver for each, and looks at
//! the bus itself between reads. A language binding needs the same, since it cannot give a
//! driver a Rust type at all. [`I2cBus`] is a handle to one bus that clones cheaply: every
//! clone reaches the same bus, and one transfer runs at a time.
//!
//! Behind the handle is one of three things:
//!
//! - The kernel's adapter, opened with [`I2cBus::open`]. Only Linux has one.
//! - Simulated parts, [`I2cBus::simulated`], each answering at its own address from its
//!   registers the way a [`sim::I2cPart`](crate::sim::I2cPart) does. A transfer to an address
//!   no part holds is not acknowledged, as on a real bus.
//! - A script, [`I2cBus::scripted`], that plays one conversation and refuses any other, the way
//!   an [`I2cScript`] does.
//!
//! A driver also takes a delay. [`I2cBus::delay`] gives it one that suits the bus: the process
//! sleeps when real parts are on the other end, and carries straight on when simulated or
//! scripted ones are, since there is nothing to wait for. Either way the wait is counted, and
//! [`I2cBus::waited_micros`] is the total, so a test checks that a driver waited as long as its
//! datasheet asks without the test taking that long.
//!
//! # Examples
//!
//! ```
//! use pamoja_hal::bus::I2cBus;
//! use pamoja_hal::i2c::I2c;
//! use pamoja_hal::sim::I2cPart;
//!
//! // Two parts on one bus, each holding the id its datasheet gives: a BME280, whose chip
//! // id register reads 0x60, and a TMP117, whose 16-bit device id register reads 0x0117.
//! const BME280: u8 = 0x76;
//! const BME280_CHIP_ID_REGISTER: u8 = 0xD0;
//! const TMP117: u8 = 0x48;
//! const TMP117_DEVICE_ID_REGISTER: u8 = 0x0F;
//! let bus = I2cBus::simulated([
//!     I2cPart::new(BME280).holding(BME280_CHIP_ID_REGISTER, &[0x60]),
//!     I2cPart::new(TMP117).holding(TMP117_DEVICE_ID_REGISTER, &0x0117u16.to_be_bytes()),
//! ]);
//!
//! // A clone is what a driver is given; the program keeps the original.
//! let mut driver = bus.clone();
//! let mut id = [0u8; 1];
//! driver.write_read(BME280, &[BME280_CHIP_ID_REGISTER], &mut id)?;
//! assert_eq!(id, [0x60]);
//!
//! // Nothing answers at the BME280's other address.
//! assert!(driver.write_read(BME280 + 1, &[BME280_CHIP_ID_REGISTER], &mut id).is_err());
//! assert_eq!(bus.transfers(), 2);
//! # Ok::<(), pamoja_hal::bus::BusError>(())
//! ```

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::{
    ErrorKind, ErrorType, I2c, NoAcknowledgeSource, Operation, SevenBitAddress,
};

use crate::script::{I2cScript, ScriptError};
use crate::sim::{FromPart, Part, PartError};

/// What answers on an [`I2cBus`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusKind {
    /// The kernel's adapter, with real parts on real wires.
    Adapter,
    /// Simulated parts, answering from their registers.
    Simulated,
    /// A script of the transfers a driver is expected to make.
    Scripted,
}

/// Why an I2C adapter could not be opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError {
    /// This platform has no I2C character device, or this build left out the `linux` feature
    /// that opens one.
    Unsupported,
    /// The adapter's device file could not be opened.
    Adapter {
        /// The device file.
        path: PathBuf,
        /// Why, in the kernel's words.
        reason: String,
    },
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Unsupported => f.write_str(
                "an I2C adapter is opened through the kernel's i2c-dev interface, which only Linux has",
            ),
            OpenError::Adapter { path, reason } => write!(f, "{}: {reason}", path.display()),
        }
    }
}

impl std::error::Error for OpenError {}

/// A part was put on a bus that is not simulated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotSimulated;

impl fmt::Display for NotSimulated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("only a simulated bus takes parts")
    }
}

impl std::error::Error for NotSimulated {}

/// Why a transfer on an [`I2cBus`] failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BusError {
    /// No simulated part holds the address, so nothing acknowledged it.
    NoAcknowledge {
        /// The address the transfer went to.
        address: u8,
    },
    /// A simulated part refused the transfer.
    Part(PartError),
    /// The script refused the transfer, or failed it on purpose.
    Script(ScriptError),
    /// The kernel's adapter failed the transfer.
    Adapter {
        /// The kind of failure, as `embedded-hal` names it.
        kind: ErrorKind,
        /// The device file and the kernel's reason.
        reason: String,
    },
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BusError::NoAcknowledge { address } => write!(f, "nothing answered at {address:#04x}"),
            BusError::Part(error) => write!(f, "{error}"),
            BusError::Script(error) => write!(f, "{error}"),
            BusError::Adapter { reason, .. } => f.write_str(reason),
        }
    }
}

impl std::error::Error for BusError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BusError::Part(error) => Some(error),
            BusError::Script(error) => Some(error),
            BusError::NoAcknowledge { .. } | BusError::Adapter { .. } => None,
        }
    }
}

impl embedded_hal::i2c::Error for BusError {
    fn kind(&self) -> ErrorKind {
        match self {
            BusError::NoAcknowledge { .. } => {
                ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)
            }
            BusError::Part(error) => error.kind(),
            BusError::Script(error) => error.kind(),
            BusError::Adapter { kind, .. } => *kind,
        }
    }
}

/// One I2C bus, reached by every clone of this handle.
///
/// # Examples
///
/// ```
/// use pamoja_hal::bus::{BusKind, I2cBus};
/// use pamoja_hal::i2c::I2c;
/// use pamoja_hal::script::{I2cScript, I2cStep};
///
/// // A script of one register read, a BME280 answering its chip id, and a driver that
/// // makes it.
/// const BME280: u8 = 0x76;
/// const CHIP_ID_REGISTER: u8 = 0xD0;
/// const BME280_CHIP_ID: u8 = 0x60;
/// let bus = I2cBus::scripted(I2cScript::new([I2cStep::write_read(
///     BME280,
///     [CHIP_ID_REGISTER],
///     [BME280_CHIP_ID],
/// )]));
/// let mut driver = bus.clone();
/// let mut id = [0u8; 1];
/// driver.write_read(BME280, &[CHIP_ID_REGISTER], &mut id)?;
///
/// assert_eq!(bus.kind(), BusKind::Scripted);
/// assert_eq!(bus.remaining(), Some(0), "the script has been played through");
/// # Ok::<(), pamoja_hal::bus::BusError>(())
/// ```
#[derive(Clone)]
pub struct I2cBus {
    shared: Arc<Mutex<Shared>>,
}

struct Shared {
    backend: Backend,
    transfers: usize,
    waited_ns: u64,
}

enum Backend {
    #[cfg(all(feature = "linux", target_os = "linux"))]
    Adapter {
        path: PathBuf,
        device: crate::linux::I2cdev,
    },
    Parts(Vec<Part>),
    Script(I2cScript),
}

impl Backend {
    fn kind(&self) -> BusKind {
        match self {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Adapter { .. } => BusKind::Adapter,
            Backend::Parts(_) => BusKind::Simulated,
            Backend::Script(_) => BusKind::Scripted,
        }
    }
}

fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Puts a part in the list, in place of one at the same address.
fn place(parts: &mut Vec<Part>, part: Part) -> Option<Part> {
    match parts
        .iter_mut()
        .find(|held| held.address() == part.address())
    {
        Some(held) => Some(std::mem::replace(held, part)),
        None => {
            parts.push(part);
            None
        }
    }
}

impl I2cBus {
    fn with(backend: Backend) -> I2cBus {
        I2cBus {
            shared: Arc::new(Mutex::new(Shared {
                backend,
                transfers: 0,
                waited_ns: 0,
            })),
        }
    }

    /// Opens the kernel's I2C adapter, such as `/dev/i2c-1` on a Raspberry Pi.
    ///
    /// # Arguments
    ///
    /// * `path` - the adapter's device file.
    ///
    /// # Returns
    ///
    /// The bus, with the real parts wired to it on the other end.
    ///
    /// # Errors
    ///
    /// [`OpenError::Unsupported`] anywhere but Linux, or in a build without the `linux`
    /// feature, and [`OpenError::Adapter`] when the file cannot be opened as an adapter: the
    /// interface is not turned on, or the process may not use it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pamoja_hal::bus::I2cBus;
    ///
    /// let bus = I2cBus::open("/dev/i2c-1")?;
    /// # Ok::<(), pamoja_hal::bus::OpenError>(())
    /// ```
    pub fn open(path: impl AsRef<Path>) -> Result<I2cBus, OpenError> {
        #[cfg(all(feature = "linux", target_os = "linux"))]
        {
            let path = path.as_ref().to_path_buf();
            let device = crate::linux::I2cdev::new(&path).map_err(|error| OpenError::Adapter {
                path: path.clone(),
                reason: error.to_string(),
            })?;
            Ok(I2cBus::with(Backend::Adapter { path, device }))
        }
        #[cfg(not(all(feature = "linux", target_os = "linux")))]
        {
            let _ = path;
            Err(OpenError::Unsupported)
        }
    }

    /// A bus of simulated parts, each answering at its own address.
    ///
    /// # Arguments
    ///
    /// * `parts` - the parts on the bus, of any of the three kinds, put on in order as
    ///   [`I2cBus::attach`] puts them, so a part at an address an earlier one holds takes its
    ///   place.
    ///
    /// # Returns
    ///
    /// The bus. A transfer to an address no part holds fails with
    /// [`BusError::NoAcknowledge`].
    pub fn simulated<P: Into<Part>>(parts: impl IntoIterator<Item = P>) -> I2cBus {
        let mut placed: Vec<Part> = Vec::new();
        for part in parts {
            place(&mut placed, part.into());
        }
        I2cBus::with(Backend::Parts(placed))
    }

    /// Puts a simulated part on the bus, in place of any part already at its address.
    ///
    /// A driver keeps working across the change, which is how a test moves a reading on: a
    /// part built to report different values takes the old one's place.
    ///
    /// # Arguments
    ///
    /// * `part` - the part, of any of the three kinds.
    ///
    /// # Returns
    ///
    /// The part it replaced, or `None` when the address was free.
    ///
    /// # Errors
    ///
    /// [`NotSimulated`] when the bus is the kernel's adapter or a script.
    pub fn attach(&self, part: impl Into<Part>) -> Result<Option<Part>, NotSimulated> {
        match &mut lock(&self.shared).backend {
            Backend::Parts(parts) => Ok(place(parts, part.into())),
            _ => Err(NotSimulated),
        }
    }

    /// A bus that plays one conversation and refuses any other.
    ///
    /// # Arguments
    ///
    /// * `script` - the transfers a driver is expected to make, in order, and the replies.
    ///
    /// # Returns
    ///
    /// The bus. A transfer that does not match the next step fails with
    /// [`BusError::Script`] and consumes nothing.
    pub fn scripted(script: I2cScript) -> I2cBus {
        I2cBus::with(Backend::Script(script))
    }

    /// What answers on the bus.
    ///
    /// # Returns
    ///
    /// The kind of bus.
    #[must_use]
    pub fn kind(&self) -> BusKind {
        lock(&self.shared).backend.kind()
    }

    /// What a simulated part holds now, as the kind of part the caller names.
    ///
    /// # Arguments
    ///
    /// * `address` - the part's address.
    ///
    /// # Returns
    ///
    /// A copy of the part, with whatever drivers have written to it, or `None` when the bus
    /// is not simulated, no part holds the address, or the part there is another kind. Ask
    /// for a [`Part`] to take any kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_hal::bus::I2cBus;
    /// use pamoja_hal::i2c::I2c;
    /// use pamoja_hal::sim::{I2cPart, WordPart};
    ///
    /// // A driver asks a BME280 for one forced measurement: ctrl_meas takes one temperature
    /// // sample, one pressure sample, and forced mode, in its three fields.
    /// const BME280: u8 = 0x76;
    /// const CTRL_MEAS: u8 = 0xF4;
    /// const FORCED_ONCE: u8 = 0b001_001_01;
    /// let mut bus = I2cBus::simulated([I2cPart::new(BME280)]);
    /// bus.write(BME280, &[CTRL_MEAS, FORCED_ONCE])?;
    ///
    /// // The program looks at what the driver left in the part.
    /// let part: I2cPart = bus.part(BME280).expect("a byte-wide part");
    /// assert_eq!(part.register(CTRL_MEAS), FORCED_ONCE);
    /// assert!(bus.part::<WordPart>(BME280).is_none(), "it is not a word-wide one");
    /// # Ok::<(), pamoja_hal::bus::BusError>(())
    /// ```
    #[must_use]
    pub fn part<P: FromPart>(&self, address: u8) -> Option<P> {
        match &lock(&self.shared).backend {
            Backend::Parts(parts) => parts
                .iter()
                .find(|part| part.address() == address)
                .cloned()
                .and_then(P::from_part),
            _ => None,
        }
    }

    /// How many transfers have been made on the bus, by every clone, including any that
    /// failed.
    ///
    /// # Returns
    ///
    /// The count, one per transaction however many operations it held.
    #[must_use]
    pub fn transfers(&self) -> usize {
        lock(&self.shared).transfers
    }

    /// How many steps a script has left.
    ///
    /// # Returns
    ///
    /// The steps not yet reached, `Some(0)` once a driver has made every transfer the script
    /// expected, or `None` when the bus is not scripted.
    #[must_use]
    pub fn remaining(&self) -> Option<usize> {
        match &lock(&self.shared).backend {
            Backend::Script(script) => Some(script.remaining()),
            _ => None,
        }
    }

    /// How long the drivers on the bus have asked to wait, through [`I2cBus::delay`].
    ///
    /// # Returns
    ///
    /// The total in microseconds, counted whether or not the process slept through it.
    #[must_use]
    pub fn waited_micros(&self) -> u64 {
        lock(&self.shared).waited_ns / 1_000
    }

    /// The delay a driver on this bus waits with.
    ///
    /// # Returns
    ///
    /// A delay that sleeps the process when the bus is the kernel's adapter, returns at once
    /// when it is simulated or scripted, and adds every wait to [`I2cBus::waited_micros`].
    #[must_use]
    pub fn delay(&self) -> BusDelay {
        BusDelay {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl fmt::Debug for I2cBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shared = lock(&self.shared);
        f.debug_struct("I2cBus")
            .field("kind", &shared.backend.kind())
            .field("transfers", &shared.transfers)
            .finish_non_exhaustive()
    }
}

impl ErrorType for I2cBus {
    type Error = BusError;
}

impl I2c<SevenBitAddress> for I2cBus {
    fn transaction(
        &mut self,
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), BusError> {
        let mut shared = lock(&self.shared);
        shared.transfers += 1;
        match &mut shared.backend {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Adapter { path, device } => {
                I2c::<SevenBitAddress>::transaction(device, address, operations)
                    .map_err(|error| adapter_error(path, &error))
            }
            Backend::Parts(parts) => {
                let Some(part) = parts.iter_mut().find(|part| part.address() == address) else {
                    return Err(BusError::NoAcknowledge { address });
                };
                part.transaction(address, operations)
                    .map_err(BusError::Part)
            }
            Backend::Script(script) => script
                .transaction(address, operations)
                .map_err(BusError::Script),
        }
    }
}

/// Names an adapter's failure. The Raspberry Pi's controllers, the BCM2835's and the RP1's
/// DesignWare block, report a part that did not acknowledge as `EREMOTEIO`, which
/// `linux-embedded-hal` leaves as [`ErrorKind::Other`]; it is a missing acknowledge here.
#[cfg(all(feature = "linux", target_os = "linux"))]
fn adapter_error(path: &Path, error: &crate::linux::I2CError) -> BusError {
    use crate::linux::i2cdev::linux::LinuxI2CError;
    use embedded_hal::i2c::Error as _;

    let errno = match error.inner() {
        LinuxI2CError::Errno(errno) => Some(*errno),
        LinuxI2CError::Io(io) => io.raw_os_error(),
    };
    let kind = if errno == Some(nix::errno::Errno::EREMOTEIO as i32) {
        ErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown)
    } else {
        error.kind()
    };
    BusError::Adapter {
        kind,
        reason: format!("{}: {error}", path.display()),
    }
}

/// The delay a driver on an [`I2cBus`] waits with, from [`I2cBus::delay`].
#[derive(Clone)]
pub struct BusDelay {
    shared: Arc<Mutex<Shared>>,
}

impl fmt::Debug for BusDelay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BusDelay").finish_non_exhaustive()
    }
}

impl DelayNs for BusDelay {
    fn delay_ns(&mut self, ns: u32) {
        let sleeps = {
            let mut shared = lock(&self.shared);
            shared.waited_ns = shared.waited_ns.saturating_add(u64::from(ns));
            shared.backend.kind() == BusKind::Adapter
        };
        if sleeps {
            std::thread::sleep(Duration::from_nanos(u64::from(ns)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::I2cStep;
    use crate::sim::{CommandPart, I2cPart, WordPart};
    use embedded_hal::i2c::Error as _;
    use std::time::Instant;

    #[test]
    fn every_clone_reaches_the_same_parts() {
        let bus = I2cBus::simulated([I2cPart::new(0x76), I2cPart::new(0x48)]);
        let mut driver = bus.clone();

        driver.write(0x76, &[0xf4, 0x25]).unwrap();
        driver.write(0x48, &[0x01, 0x60, 0x20]).unwrap();

        let part = bus.part::<I2cPart>(0x76).map(|part| part.register(0xf4));
        assert_eq!(part, Some(0x25));
        let part = bus.part::<I2cPart>(0x48).map(|part| part.register(0x02));
        assert_eq!(part, Some(0x20));
        assert_eq!(bus.transfers(), 2);
        assert_eq!(bus.kind(), BusKind::Simulated);
        assert_eq!(bus.remaining(), None);
    }

    #[test]
    fn a_part_attached_at_a_held_address_takes_its_place() {
        let bus = I2cBus::simulated([
            I2cPart::new(0x76).holding(0xd0, &[0x58]),
            I2cPart::new(0x76).holding(0xd0, &[0x60]),
        ]);
        let held = bus.part::<I2cPart>(0x76).map(|part| part.register(0xd0));
        assert_eq!(held, Some(0x60));

        let replaced = bus
            .attach(I2cPart::new(0x76).holding(0xd0, &[0x61]))
            .expect("a simulated bus takes parts");
        let replaced = replaced
            .and_then(I2cPart::from_part)
            .map(|part| part.register(0xd0));
        assert_eq!(replaced, Some(0x60));
        let held = bus.part::<I2cPart>(0x76).map(|part| part.register(0xd0));
        assert_eq!(held, Some(0x61));

        assert!(bus
            .attach(I2cPart::new(0x48))
            .expect("a free address")
            .is_none());
        assert!(bus.part::<Part>(0x48).is_some());
    }

    #[test]
    fn one_bus_holds_parts_of_every_kind() {
        let mut bus = I2cBus::simulated([
            Part::from(I2cPart::new(0x76).holding(0xd0, &[0x60])),
            WordPart::new(0x48).holding(0x0f, 0x0117).into(),
            CommandPart::new(0x44, 2)
                .answering(&[0xf3, 0x2d], &[0x80, 0x10, 0xe1])
                .into(),
        ]);

        let mut word = [0u8; 2];
        bus.write_read(0x48, &[0x0f], &mut word).unwrap();
        assert_eq!(u16::from_be_bytes(word), 0x0117);
        bus.write(0x44, &[0xf3, 0x2d]).unwrap();
        let mut status = [0u8; 3];
        bus.read(0x44, &mut status).unwrap();
        assert_eq!(status, [0x80, 0x10, 0xe1]);

        assert!(bus.part::<WordPart>(0x48).is_some());
        assert!(
            bus.part::<I2cPart>(0x48).is_none(),
            "a word-wide part is not a byte-wide one"
        );
        let commands: CommandPart = bus.part(0x44).expect("a part that takes commands");
        assert_eq!(commands.received().len(), 1);
        assert_eq!(
            bus.read(0x44, &mut status).unwrap_err(),
            BusError::Part(PartError::NoReply),
            "the reply was taken"
        );
    }

    #[test]
    fn only_a_simulated_bus_takes_parts() {
        let bus = I2cBus::scripted(I2cScript::new([]));
        assert_eq!(bus.attach(I2cPart::new(0x76)).unwrap_err(), NotSimulated);
        assert_eq!(NotSimulated.to_string(), "only a simulated bus takes parts");
    }

    #[test]
    fn an_address_no_part_holds_is_not_acknowledged() {
        let mut bus = I2cBus::simulated([I2cPart::new(0x76)]);

        let error = bus.write(0x77, &[0xd0]).unwrap_err();
        assert_eq!(error, BusError::NoAcknowledge { address: 0x77 });
        assert_eq!(
            error.kind(),
            ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)
        );
        assert_eq!(error.to_string(), "nothing answered at 0x77");
        assert!(bus.part::<Part>(0x77).is_none());
        assert_eq!(bus.transfers(), 1, "a refused transfer still happened");
    }

    #[test]
    fn a_part_refusing_a_read_says_why() {
        let mut bus = I2cBus::simulated([I2cPart::new(0x76)]);
        let mut byte = [0u8; 1];

        let error = bus.read(0x76, &mut byte).unwrap_err();
        assert_eq!(error, BusError::Part(PartError::NoRegister));
    }

    #[test]
    fn a_script_counts_what_is_left_and_refuses_the_unexpected() {
        let mut bus = I2cBus::scripted(I2cScript::new([
            I2cStep::write(0x76, [0xe0, 0xb6]),
            I2cStep::write_read(0x76, [0xd0], [0x60]),
        ]));
        assert_eq!(bus.remaining(), Some(2));

        bus.write(0x76, &[0xe0, 0xb6]).unwrap();
        assert_eq!(bus.remaining(), Some(1));

        let mut id = [0u8; 1];
        let refused = bus.write_read(0x76, &[0xd1], &mut id).unwrap_err();
        assert!(
            matches!(
                refused,
                BusError::Script(ScriptError::Mismatch { step: 1, .. })
            ),
            "{refused:?}"
        );
        assert_eq!(
            bus.remaining(),
            Some(1),
            "a refused transfer consumes nothing"
        );

        bus.write_read(0x76, &[0xd0], &mut id).unwrap();
        assert_eq!(id, [0x60]);
        assert_eq!(bus.remaining(), Some(0));
        assert_eq!(bus.kind(), BusKind::Scripted);
        assert!(bus.part::<Part>(0x76).is_none());
    }

    #[test]
    fn a_simulated_bus_counts_a_wait_without_sleeping_through_it() {
        let bus = I2cBus::simulated(Vec::<Part>::new());
        let mut delay = bus.delay();

        let started = Instant::now();
        delay.delay_ms(500);
        delay.delay_us(250);

        assert!(started.elapsed() < Duration::from_millis(250));
        assert_eq!(bus.waited_micros(), 500_250);
    }

    #[cfg(not(all(feature = "linux", target_os = "linux")))]
    #[test]
    fn opening_an_adapter_needs_linux() {
        let error = I2cBus::open("/dev/i2c-1").unwrap_err();
        assert_eq!(error, OpenError::Unsupported);
        assert!(error.to_string().contains("only Linux"), "{error}");
    }

    #[cfg(all(feature = "linux", target_os = "linux"))]
    #[test]
    fn opening_a_missing_adapter_names_it() {
        let error = I2cBus::open("/dev/i2c-pamoja-absent").unwrap_err();
        assert!(
            error.to_string().starts_with("/dev/i2c-pamoja-absent: "),
            "{error}"
        );
    }
}
