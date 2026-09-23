//! One serial port, shared by a program and every driver on it.
//!
//! A UART carries bytes one character at a time and knows nothing of messages: a framing such
//! as SLIP or COBS from `pamoja-serial`, or the silences of Modbus RTU, is what marks where one
//! ends. [`SerialPort`] is a handle to one port that clones cheaply, as
//! [`I2cBus`](crate::bus::I2cBus) is to one bus, so a program, a thread that reads, and a
//! protocol client can all hold the same port. Behind the handle is one of five things:
//!
//! - The kernel's serial device, opened raw with [`SerialPort::open`]: `/dev/serial0` for a
//!   Raspberry Pi's own UART, `/dev/ttyUSB0` or `/dev/ttyACM0` for a USB adapter. Only Linux
//!   has one here.
//! - A line looped back on itself, [`SerialPort::looped`]: every byte written is read back,
//!   as with TX wired to RX.
//! - One end of a null-modem pair, [`SerialPort::pair`]: what one end writes, the other reads.
//! - A simulated device, [`SerialPort::simulated`], that takes every write and answers with
//!   bytes of its own: anything that implements [`Peer`], such as a Modbus server.
//! - A script, [`SerialPort::scripted`], of the writes a driver is expected to make and the
//!   bytes the far end sends back.
//!
//! A read waits for bytes up to a timeout. On the kernel's device the process waits. On every
//! other port nothing waits, since nothing else is coming, and the time a read would have
//! waited is added to [`SerialPort::waited_micros`], so a test of a device that never answers
//! runs at once and still says how long a real one would have taken.
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//!
//! use pamoja_hal::port::{SerialPort, Settings};
//!
//! // Two ends of one line: a gateway and the node it listens to.
//! let (gateway, node) = SerialPort::pair(Settings::new(115_200));
//! node.write(b"t=21.5")?;
//!
//! let mut buffer = [0u8; 16];
//! let got = gateway.read(&mut buffer, Duration::from_millis(100))?;
//! assert_eq!(&buffer[..got], b"t=21.5");
//!
//! // Nothing more is coming: the read gives up after its timeout, which is counted, not slept.
//! assert_eq!(gateway.read(&mut buffer, Duration::from_millis(100))?, 0);
//! assert_eq!(gateway.waited_micros(), 100_000);
//! # Ok::<(), pamoja_hal::port::PortError>(())
//! ```

use std::collections::VecDeque;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// The parity bit each character carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Parity {
    /// No parity bit.
    None,
    /// A bit that makes the count of ones even, which Modbus RTU asks for by default.
    Even,
    /// A bit that makes the count of ones odd.
    Odd,
}

/// How many stop bits end each character.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StopBits {
    /// One stop bit.
    One,
    /// Two stop bits.
    Two,
}

/// A port's speed and character format: eight data bits, with the parity and stop bits given.
///
/// # Examples
///
/// ```
/// use pamoja_hal::port::{Parity, Settings};
///
/// // A Modbus RTU meter at 9600 baud, with the even parity the protocol defaults to: one
/// // stop bit, and eleven bits a character.
/// let meter = Settings::new(9_600).with_parity(Parity::Even);
/// assert_eq!(meter.to_string(), "9600 8E1");
/// assert_eq!(meter.bits_per_character(), 11);
/// assert_eq!(meter.character_nanos(), 1_145_834);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Settings {
    /// The speed, in bits a second.
    pub baud: u32,
    /// The parity bit each character carries.
    pub parity: Parity,
    /// How many stop bits end each character.
    pub stop_bits: StopBits,
}

impl Settings {
    /// Eight data bits, no parity, and one stop bit, the format most devices start in.
    ///
    /// # Arguments
    ///
    /// * `baud` - the speed, in bits a second.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn new(baud: u32) -> Settings {
        Settings {
            baud,
            parity: Parity::None,
            stop_bits: StopBits::One,
        }
    }

    /// Sets the parity bit.
    ///
    /// # Arguments
    ///
    /// * `parity` - the parity each character carries.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn with_parity(mut self, parity: Parity) -> Settings {
        self.parity = parity;
        self
    }

    /// Sets the stop bits.
    ///
    /// # Arguments
    ///
    /// * `stop_bits` - how many stop bits end each character.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn with_stop_bits(mut self, stop_bits: StopBits) -> Settings {
        self.stop_bits = stop_bits;
        self
    }

    /// Returns the bits one character takes on the wire: a start bit, eight data bits, the
    /// parity bit if there is one, and the stop bits.
    pub const fn bits_per_character(&self) -> u32 {
        let parity = match self.parity {
            Parity::None => 0,
            Parity::Even | Parity::Odd => 1,
        };
        let stop = match self.stop_bits {
            StopBits::One => 1,
            StopBits::Two => 2,
        };
        1 + 8 + parity + stop
    }

    /// Returns how long one character takes on the wire, in nanoseconds, rounded up.
    pub const fn character_nanos(&self) -> u64 {
        let baud = if self.baud == 0 { 1 } else { self.baud as u64 };
        (self.bits_per_character() as u64 * 1_000_000_000).div_ceil(baud)
    }

    /// Returns how long a run of bytes takes on the wire, sent back to back, in microseconds,
    /// rounded up.
    ///
    /// # Arguments
    ///
    /// * `bytes` - how many bytes.
    ///
    /// # Returns
    ///
    /// The time on the wire.
    pub const fn transfer_micros(&self, bytes: usize) -> u64 {
        (self.character_nanos() * bytes as u64).div_ceil(1_000)
    }
}

impl fmt::Display for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parity = match self.parity {
            Parity::None => 'N',
            Parity::Even => 'E',
            Parity::Odd => 'O',
        };
        let stop = match self.stop_bits {
            StopBits::One => 1,
            StopBits::Two => 2,
        };
        write!(f, "{} 8{parity}{stop}", self.baud)
    }
}

/// What is on the other end of a [`SerialPort`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortKind {
    /// The kernel's serial device, with a real line on the other end.
    Device,
    /// The port's own output, looped back to its input.
    Looped,
    /// The other end of a null-modem pair.
    Paired,
    /// A simulated device that answers each write.
    Simulated,
    /// A script of the writes a driver is expected to make.
    Scripted,
}

/// A device at the far end of a simulated serial line.
///
/// Every write to a [`SerialPort::simulated`] port reaches [`Peer::receive`], and whatever it
/// returns is what the device sends back, waiting for the next read. A program that wants to
/// look at the device afterward shares it: `Arc<Mutex<P>>` is a peer whenever `P` is.
///
/// # Examples
///
/// ```
/// use std::sync::{Arc, Mutex};
/// use std::time::Duration;
///
/// use pamoja_hal::port::{Peer, SerialPort, Settings};
///
/// /// A device that answers every write with the same bytes, upper-cased.
/// #[derive(Default)]
/// struct Shouter {
///     heard: usize,
/// }
///
/// impl Peer for Shouter {
///     fn receive(&mut self, bytes: &[u8]) -> Vec<u8> {
///         self.heard += bytes.len();
///         bytes.to_ascii_uppercase()
///     }
/// }
///
/// let device = Arc::new(Mutex::new(Shouter::default()));
/// let port = SerialPort::simulated(Settings::new(9_600), Arc::clone(&device));
/// port.write(b"ping")?;
///
/// let mut reply = [0u8; 8];
/// let got = port.read(&mut reply, Duration::from_millis(50))?;
/// assert_eq!(&reply[..got], b"PING");
/// assert_eq!(device.lock().unwrap().heard, 4);
/// # Ok::<(), pamoja_hal::port::PortError>(())
/// ```
pub trait Peer: Send {
    /// Takes the bytes the port wrote and returns the bytes the device sends back.
    ///
    /// # Arguments
    ///
    /// * `bytes` - one write's bytes.
    ///
    /// # Returns
    ///
    /// The device's answer, empty when it says nothing.
    fn receive(&mut self, bytes: &[u8]) -> Vec<u8>;
}

impl<P: Peer> Peer for Arc<Mutex<P>> {
    fn receive(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .receive(bytes)
    }
}

/// One step of a [`SerialPort::scripted`] port.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortStep {
    /// Bytes the program is expected to write, in one call.
    Write(Vec<u8>),
    /// Bytes the far end sends, readable once every step before this one has happened.
    Read(Vec<u8>),
}

impl PortStep {
    /// A write the program is expected to make.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the bytes of the one write.
    ///
    /// # Returns
    ///
    /// The step.
    pub fn write(bytes: impl Into<Vec<u8>>) -> PortStep {
        PortStep::Write(bytes.into())
    }

    /// Bytes the far end sends.
    ///
    /// # Arguments
    ///
    /// * `bytes` - what arrives.
    ///
    /// # Returns
    ///
    /// The step.
    pub fn read(bytes: impl Into<Vec<u8>>) -> PortStep {
        PortStep::Read(bytes.into())
    }
}

/// Why a serial device could not be opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError {
    /// This platform has no serial device this crate opens, or this build left out the
    /// `linux` feature that opens one.
    Unsupported,
    /// The device file could not be opened or set up as a raw byte pipe.
    Device {
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
                "a serial device is opened through the kernel's terminal interface, which only Linux has here",
            ),
            OpenError::Device { path, reason } => write!(f, "{}: {reason}", path.display()),
        }
    }
}

impl std::error::Error for OpenError {}

/// Why a read or a write on a [`SerialPort`] failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortError {
    /// A scripted port was written something other than what its script expected next.
    Unexpected {
        /// The write the script expected, or none when the script had no writes left.
        expected: Option<Vec<u8>>,
        /// What was written.
        written: Vec<u8>,
    },
    /// The kernel's device failed.
    Device {
        /// The device file and the kernel's reason.
        reason: String,
    },
}

impl fmt::Display for PortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortError::Unexpected {
                expected: Some(expected),
                written,
            } => write!(
                f,
                "the script expected {} to be written, not {}",
                hex(expected),
                hex(written)
            ),
            PortError::Unexpected {
                expected: None,
                written,
            } => write!(
                f,
                "the script expected no more writes, but {} was written",
                hex(written)
            ),
            PortError::Device { reason } => f.write_str(reason),
        }
    }
}

impl std::error::Error for PortError {}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 3);
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            text.push(' ');
        }
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

/// One serial port, reached by every clone of this handle.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use pamoja_hal::port::{PortKind, SerialPort, Settings};
///
/// // TX wired to RX: what goes out comes straight back.
/// let port = SerialPort::looped(Settings::new(115_200));
/// port.write(b"PING\r\n")?;
///
/// let mut echo = [0u8; 6];
/// assert_eq!(port.read(&mut echo, Duration::from_millis(10))?, 6);
/// assert_eq!(&echo, b"PING\r\n");
/// assert_eq!(port.kind(), PortKind::Looped);
/// assert_eq!((port.written(), port.received()), (6, 6));
/// # Ok::<(), pamoja_hal::port::PortError>(())
/// ```
#[derive(Clone)]
pub struct SerialPort {
    shared: Arc<Mutex<Shared>>,
}

struct Shared {
    backend: Backend,
    settings: Settings,
    written: usize,
    received: usize,
    waited_ns: u64,
}

type Line = Arc<Mutex<VecDeque<u8>>>;

enum Backend {
    #[cfg(all(feature = "linux", target_os = "linux"))]
    Device {
        path: PathBuf,
        file: Arc<std::fs::File>,
    },
    Looped(VecDeque<u8>),
    Paired {
        inbox: Line,
        outbox: Line,
    },
    Simulated {
        peer: Box<dyn Peer>,
        inbox: VecDeque<u8>,
    },
    Scripted {
        steps: VecDeque<PortStep>,
        inbox: VecDeque<u8>,
    },
}

impl Backend {
    fn kind(&self) -> PortKind {
        match self {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Device { .. } => PortKind::Device,
            Backend::Looped(_) => PortKind::Looped,
            Backend::Paired { .. } => PortKind::Paired,
            Backend::Simulated { .. } => PortKind::Simulated,
            Backend::Scripted { .. } => PortKind::Scripted,
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn take(queue: &mut VecDeque<u8>, buffer: &mut [u8]) -> usize {
    let count = buffer.len().min(queue.len());
    for (slot, byte) in buffer.iter_mut().zip(queue.drain(..count)) {
        *slot = byte;
    }
    count
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

/// Moves the reads a script has reached into the inbox.
fn release(steps: &mut VecDeque<PortStep>, inbox: &mut VecDeque<u8>) {
    while let Some(PortStep::Read(bytes)) = steps.front() {
        inbox.extend(bytes);
        steps.pop_front();
    }
}

impl SerialPort {
    fn with(backend: Backend, settings: Settings) -> SerialPort {
        SerialPort {
            shared: Arc::new(Mutex::new(Shared {
                backend,
                settings,
                written: 0,
                received: 0,
                waited_ns: 0,
            })),
        }
    }

    /// Opens the kernel's serial device, raw: no echo, no line editing, no translation, just
    /// bytes at the given speed and character format.
    ///
    /// Whatever the device received before it was opened is dropped.
    ///
    /// # Arguments
    ///
    /// * `path` - the device file: `/dev/serial0` for a Raspberry Pi's own UART,
    ///   `/dev/ttyUSB0` or `/dev/ttyACM0` for a USB adapter.
    /// * `settings` - the speed, one of the standard rates from 1200 to 921600, and the
    ///   character format.
    ///
    /// # Returns
    ///
    /// The port, with the real line on the other end.
    ///
    /// # Errors
    ///
    /// [`OpenError::Unsupported`] anywhere but Linux, or in a build without the `linux`
    /// feature, and [`OpenError::Device`] when the file cannot be opened, the rate is not a
    /// standard one, or the terminal settings cannot be applied.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pamoja_hal::port::{SerialPort, Settings};
    ///
    /// let port = SerialPort::open("/dev/serial0", Settings::new(115_200))?;
    /// # Ok::<(), pamoja_hal::port::OpenError>(())
    /// ```
    pub fn open(path: impl AsRef<Path>, settings: Settings) -> Result<SerialPort, OpenError> {
        #[cfg(all(feature = "linux", target_os = "linux"))]
        {
            let path = path.as_ref().to_path_buf();
            let file = device::open(&path, settings).map_err(|error| OpenError::Device {
                path: path.clone(),
                reason: error.to_string(),
            })?;
            Ok(SerialPort::with(
                Backend::Device {
                    path,
                    file: Arc::new(file),
                },
                settings,
            ))
        }
        #[cfg(not(all(feature = "linux", target_os = "linux")))]
        {
            let _ = (path, settings);
            Err(OpenError::Unsupported)
        }
    }

    /// A line looped back on itself: every byte written is waiting to be read, as with TX
    /// wired to RX.
    ///
    /// # Arguments
    ///
    /// * `settings` - the speed and character format the line runs at.
    ///
    /// # Returns
    ///
    /// The port.
    pub fn looped(settings: Settings) -> SerialPort {
        SerialPort::with(Backend::Looped(VecDeque::new()), settings)
    }

    /// The two ends of a null-modem pair: what one end writes, the other reads.
    ///
    /// # Arguments
    ///
    /// * `settings` - the speed and character format both ends run at.
    ///
    /// # Returns
    ///
    /// The two ends.
    pub fn pair(settings: Settings) -> (SerialPort, SerialPort) {
        let one: Line = Arc::default();
        let other: Line = Arc::default();
        (
            SerialPort::with(
                Backend::Paired {
                    inbox: Arc::clone(&one),
                    outbox: Arc::clone(&other),
                },
                settings,
            ),
            SerialPort::with(
                Backend::Paired {
                    inbox: other,
                    outbox: one,
                },
                settings,
            ),
        )
    }

    /// A simulated device on the other end of the line, answering each write.
    ///
    /// # Arguments
    ///
    /// * `settings` - the speed and character format the line runs at.
    /// * `peer` - the device.
    ///
    /// # Returns
    ///
    /// The port.
    pub fn simulated(settings: Settings, peer: impl Peer + 'static) -> SerialPort {
        SerialPort::with(
            Backend::Simulated {
                peer: Box::new(peer),
                inbox: VecDeque::new(),
            },
            settings,
        )
    }

    /// A port that plays a script: each write has to be the one the script expects next, and
    /// the bytes the far end sends become readable as the script reaches them.
    ///
    /// # Arguments
    ///
    /// * `settings` - the speed and character format the line runs at.
    /// * `steps` - the writes and reads, in order. Reads at the start are readable at once.
    ///
    /// # Returns
    ///
    /// The port.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use pamoja_hal::port::{PortStep, SerialPort, Settings};
    ///
    /// let port = SerialPort::scripted(
    ///     Settings::new(9_600),
    ///     [PortStep::write(*b"?"), PortStep::read(*b"42")],
    /// );
    /// port.write(b"?")?;
    /// let mut answer = [0u8; 2];
    /// port.read(&mut answer, Duration::from_millis(10))?;
    /// assert_eq!(&answer, b"42");
    /// assert_eq!(port.remaining(), Some(0));
    /// assert!(port.write(b"?").is_err(), "the script is done");
    /// # Ok::<(), pamoja_hal::port::PortError>(())
    /// ```
    pub fn scripted(settings: Settings, steps: impl IntoIterator<Item = PortStep>) -> SerialPort {
        let mut steps: VecDeque<PortStep> = steps.into_iter().collect();
        let mut inbox = VecDeque::new();
        release(&mut steps, &mut inbox);
        SerialPort::with(Backend::Scripted { steps, inbox }, settings)
    }

    /// Returns the speed and character format the port runs at.
    pub fn settings(&self) -> Settings {
        lock(&self.shared).settings
    }

    /// Returns what is on the other end.
    pub fn kind(&self) -> PortKind {
        lock(&self.shared).backend.kind()
    }

    /// Writes bytes, and on the kernel's device waits until they have left the UART.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the bytes, in order.
    ///
    /// # Errors
    ///
    /// [`PortError::Unexpected`] when a scripted port is written something its script does
    /// not expect next, and [`PortError::Device`] when the kernel's device fails.
    pub fn write(&self, bytes: &[u8]) -> Result<(), PortError> {
        let mut shared = lock(&self.shared);
        match &mut shared.backend {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Device { path, file } => {
                let (path, file) = (path.clone(), Arc::clone(file));
                drop(shared);
                device::write(&file, bytes).map_err(|error| PortError::Device {
                    reason: format!("{}: {error}", path.display()),
                })?;
                shared = lock(&self.shared);
            }
            Backend::Looped(inbox) => inbox.extend(bytes),
            Backend::Paired { outbox, .. } => lock(outbox).extend(bytes),
            Backend::Simulated { peer, inbox } => inbox.extend(peer.receive(bytes)),
            Backend::Scripted { steps, inbox } => match steps.front() {
                Some(PortStep::Write(expected)) if expected == bytes => {
                    steps.pop_front();
                    release(steps, inbox);
                }
                next => {
                    let expected = match next {
                        Some(PortStep::Write(expected)) => Some(expected.clone()),
                        _ => None,
                    };
                    return Err(PortError::Unexpected {
                        expected,
                        written: bytes.to_vec(),
                    });
                }
            },
        }
        shared.written += bytes.len();
        Ok(())
    }

    /// Reads what has arrived, waiting up to `timeout` for the first byte when nothing has.
    ///
    /// On any port but the kernel's device the read does not wait: when nothing has arrived it
    /// returns at once, and the timeout is added to [`SerialPort::waited_micros`].
    ///
    /// # Arguments
    ///
    /// * `buffer` - where the bytes go; at most its length is read.
    /// * `timeout` - how long to wait for the first byte.
    ///
    /// # Returns
    ///
    /// How many bytes were read, zero when the timeout passed with nothing.
    ///
    /// # Errors
    ///
    /// [`PortError::Device`] when the kernel's device fails.
    pub fn read(&self, buffer: &mut [u8], timeout: Duration) -> Result<usize, PortError> {
        let mut shared = lock(&self.shared);
        let count = match &mut shared.backend {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Device { path, file } => {
                let (path, file) = (path.clone(), Arc::clone(file));
                drop(shared);
                let count =
                    device::read(&file, buffer, timeout).map_err(|error| PortError::Device {
                        reason: format!("{}: {error}", path.display()),
                    })?;
                shared = lock(&self.shared);
                count
            }
            Backend::Looped(inbox)
            | Backend::Simulated { inbox, .. }
            | Backend::Scripted { inbox, .. } => take(inbox, buffer),
            Backend::Paired { inbox, .. } => take(&mut lock(inbox), buffer),
        };
        if count == 0 && !buffer.is_empty() {
            shared.waited_ns = shared.waited_ns.saturating_add(nanos(timeout));
        }
        shared.received += count;
        Ok(count)
    }

    /// Drops whatever has arrived and not been read, as a client does before it sends a
    /// request so a stale reply cannot be taken for the new one.
    ///
    /// # Errors
    ///
    /// [`PortError::Device`] when the kernel's device fails.
    pub fn discard_input(&self) -> Result<(), PortError> {
        let mut shared = lock(&self.shared);
        match &mut shared.backend {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Device { path, file } => {
                device::discard(file).map_err(|error| PortError::Device {
                    reason: format!("{}: {error}", path.display()),
                })?;
            }
            Backend::Looped(inbox)
            | Backend::Simulated { inbox, .. }
            | Backend::Scripted { inbox, .. } => inbox.clear(),
            Backend::Paired { inbox, .. } => lock(inbox).clear(),
        }
        Ok(())
    }

    /// Waits, as a protocol does to leave the line silent between frames: the process sleeps
    /// on the kernel's device, and anywhere else the wait is only counted.
    ///
    /// # Arguments
    ///
    /// * `duration` - how long.
    pub fn wait(&self, duration: Duration) {
        let sleeps = {
            let mut shared = lock(&self.shared);
            shared.waited_ns = shared.waited_ns.saturating_add(nanos(duration));
            shared.backend.kind() == PortKind::Device
        };
        if sleeps {
            std::thread::sleep(duration);
        }
    }

    /// Returns how many bytes have been written through this port and every clone of it.
    pub fn written(&self) -> usize {
        lock(&self.shared).written
    }

    /// Returns how many bytes have been read through this port and every clone of it.
    pub fn received(&self) -> usize {
        lock(&self.shared).received
    }

    /// Returns how long reads have waited without an answer, and [`SerialPort::wait`] has
    /// waited, in microseconds, whether or not the process slept through it.
    pub fn waited_micros(&self) -> u64 {
        lock(&self.shared).waited_ns / 1_000
    }

    /// Returns how many steps a script has left, or `None` when the port is not scripted.
    pub fn remaining(&self) -> Option<usize> {
        match &lock(&self.shared).backend {
            Backend::Scripted { steps, .. } => Some(steps.len()),
            _ => None,
        }
    }
}

impl fmt::Debug for SerialPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shared = lock(&self.shared);
        f.debug_struct("SerialPort")
            .field("kind", &shared.backend.kind())
            .field("settings", &shared.settings)
            .field("written", &shared.written)
            .field("received", &shared.received)
            .finish_non_exhaustive()
    }
}

/// The kernel's terminal interface, set up as a raw byte pipe.
#[cfg(all(feature = "linux", target_os = "linux"))]
mod device {
    use std::fs::{File, OpenOptions};
    use std::io::{self, Read, Write};
    use std::os::fd::AsFd;
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::Path;
    use std::time::{Duration, Instant};

    use nix::errno::Errno;
    use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
    use nix::sys::termios::{
        self, BaudRate, ControlFlags, FlushArg, InputFlags, SetArg, SpecialCharacterIndices,
    };

    use super::{Parity, Settings, StopBits};

    pub(super) fn open(path: &Path, settings: Settings) -> io::Result<File> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(nix::libc::O_NOCTTY)
            .open(path)?;
        let speed = match settings.baud {
            1_200 => BaudRate::B1200,
            2_400 => BaudRate::B2400,
            4_800 => BaudRate::B4800,
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
        let mut tty = termios::tcgetattr(&file)?;
        termios::cfmakeraw(&mut tty);
        termios::cfsetispeed(&mut tty, speed)?;
        termios::cfsetospeed(&mut tty, speed)?;
        tty.control_flags |= ControlFlags::CLOCAL | ControlFlags::CREAD;
        tty.control_flags &= !(ControlFlags::PARENB
            | ControlFlags::PARODD
            | ControlFlags::CSTOPB
            | ControlFlags::CRTSCTS
            | ControlFlags::CSIZE);
        tty.control_flags |= ControlFlags::CS8;
        match settings.parity {
            Parity::None => {}
            Parity::Even => tty.control_flags |= ControlFlags::PARENB,
            Parity::Odd => tty.control_flags |= ControlFlags::PARENB | ControlFlags::PARODD,
        }
        if settings.parity != Parity::None {
            tty.input_flags |= InputFlags::INPCK | InputFlags::IGNPAR;
        }
        if settings.stop_bits == StopBits::Two {
            tty.control_flags |= ControlFlags::CSTOPB;
        }
        tty.control_chars[SpecialCharacterIndices::VMIN as usize] = 0;
        tty.control_chars[SpecialCharacterIndices::VTIME as usize] = 0;
        termios::tcsetattr(&file, SetArg::TCSANOW, &tty)?;
        termios::tcflush(&file, FlushArg::TCIOFLUSH)?;
        Ok(file)
    }

    pub(super) fn write(file: &File, bytes: &[u8]) -> io::Result<()> {
        let mut writer = file;
        writer.write_all(bytes)?;
        termios::tcdrain(file)?;
        Ok(())
    }

    pub(super) fn read(file: &File, buffer: &mut [u8], timeout: Duration) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let millis = i32::try_from(left.as_micros().div_ceil(1_000)).unwrap_or(i32::MAX);
            let wait = PollTimeout::try_from(millis).unwrap_or(PollTimeout::MAX);
            let mut fds = [PollFd::new(file.as_fd(), PollFlags::POLLIN)];
            match poll(&mut fds, wait) {
                Ok(0) => return Ok(0),
                Ok(_) => {
                    let mut reader = file;
                    return reader.read(buffer);
                }
                Err(Errno::EINTR) => continue,
                Err(error) => return Err(error.into()),
            }
        }
    }

    pub(super) fn discard(file: &File) -> io::Result<()> {
        termios::tcflush(file, FlushArg::TCIFLUSH)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_character_is_its_start_data_parity_and_stop_bits() {
        let eight_n_one = Settings::new(115_200);
        assert_eq!(eight_n_one.bits_per_character(), 10);
        assert_eq!(eight_n_one.character_nanos(), 86_806, "10 bits at 115200");
        assert_eq!(eight_n_one.to_string(), "115200 8N1");

        // Modbus over Serial Line V1.02, 2.5.1: eleven bits a character, even parity by
        // default, and two stop bits in place of a parity bit.
        let modbus = Settings::new(9_600).with_parity(Parity::Even);
        assert_eq!(modbus.bits_per_character(), 11);
        let no_parity = Settings::new(9_600).with_stop_bits(StopBits::Two);
        assert_eq!(no_parity.bits_per_character(), 11);
        assert_eq!(no_parity.to_string(), "9600 8N2");
        assert_eq!(
            Settings::new(19_200).with_parity(Parity::Odd).to_string(),
            "19200 8O1"
        );

        // 3.5 characters at 9600 8E1 is the 4.01 ms silence that ends an RTU frame.
        assert_eq!((modbus.character_nanos() * 7).div_ceil(2), 4_010_419);
        assert_eq!(modbus.transfer_micros(8), 9_167, "an eight-byte request");
    }

    #[test]
    fn a_looped_line_reads_back_what_it_wrote() {
        let port = SerialPort::looped(Settings::new(9_600));
        port.write(b"abc").unwrap();
        let mut buffer = [0u8; 2];
        assert_eq!(port.read(&mut buffer, Duration::from_millis(5)).unwrap(), 2);
        assert_eq!(&buffer, b"ab");
        assert_eq!(port.read(&mut buffer, Duration::from_millis(5)).unwrap(), 1);
        assert_eq!(buffer[0], b'c');
        assert_eq!(port.read(&mut buffer, Duration::from_millis(5)).unwrap(), 0);
        assert_eq!(port.waited_micros(), 5_000, "only the empty read waited");
        assert_eq!((port.written(), port.received()), (3, 3));
    }

    #[test]
    fn each_end_of_a_pair_reads_what_the_other_wrote() {
        let (left, right) = SerialPort::pair(Settings::new(115_200));
        left.write(b"ping").unwrap();
        right.write(b"pong").unwrap();
        let mut buffer = [0u8; 8];
        let got = right.read(&mut buffer, Duration::ZERO).unwrap();
        assert_eq!(&buffer[..got], b"ping");
        let got = left.read(&mut buffer, Duration::ZERO).unwrap();
        assert_eq!(&buffer[..got], b"pong");
        assert_eq!(left.kind(), PortKind::Paired);

        let reader = right.clone();
        left.write(b"!").unwrap();
        assert_eq!(reader.read(&mut buffer, Duration::ZERO).unwrap(), 1);
        assert_eq!(right.received(), 5, "a clone reaches the same end");
    }

    struct Counter(usize);

    impl Peer for Counter {
        fn receive(&mut self, bytes: &[u8]) -> Vec<u8> {
            self.0 += 1;
            vec![bytes.len() as u8, self.0 as u8]
        }
    }

    #[test]
    fn a_simulated_device_answers_each_write_and_stays_reachable() {
        let device = Arc::new(Mutex::new(Counter(0)));
        let port = SerialPort::simulated(Settings::new(9_600), Arc::clone(&device));
        port.write(b"one").unwrap();
        port.write(b"three").unwrap();
        let mut buffer = [0u8; 8];
        let got = port.read(&mut buffer, Duration::from_millis(1)).unwrap();
        assert_eq!(&buffer[..got], &[3, 1, 5, 2]);
        assert_eq!(device.lock().unwrap().0, 2);
        assert_eq!(port.kind(), PortKind::Simulated);
    }

    #[test]
    fn stale_input_is_discarded() {
        let port = SerialPort::looped(Settings::new(9_600));
        port.write(b"stale").unwrap();
        port.discard_input().unwrap();
        let mut buffer = [0u8; 8];
        assert_eq!(port.read(&mut buffer, Duration::from_millis(2)).unwrap(), 0);
        assert_eq!(port.received(), 0);
    }

    #[test]
    fn a_script_checks_each_write_and_releases_what_follows() {
        let port = SerialPort::scripted(
            Settings::new(9_600),
            [
                PortStep::read(*b"hello"),
                PortStep::write(*b"?"),
                PortStep::read(*b"4"),
                PortStep::read(*b"2"),
                PortStep::write(*b"bye"),
            ],
        );
        let mut buffer = [0u8; 8];
        let got = port.read(&mut buffer, Duration::ZERO).unwrap();
        assert_eq!(
            &buffer[..got],
            b"hello",
            "reads at the start are there at once"
        );
        assert_eq!(
            port.write(b"!"),
            Err(PortError::Unexpected {
                expected: Some(b"?".to_vec()),
                written: b"!".to_vec(),
            })
        );
        port.write(b"?").unwrap();
        let got = port.read(&mut buffer, Duration::ZERO).unwrap();
        assert_eq!(&buffer[..got], b"42");
        assert_eq!(port.remaining(), Some(1));
        port.write(b"bye").unwrap();
        let error = port.write(b"again").unwrap_err();
        assert_eq!(
            error.to_string(),
            "the script expected no more writes, but 61 67 61 69 6e was written"
        );
        assert_eq!(port.written(), 4, "a refused write is not counted");
    }

    #[test]
    fn a_wait_is_counted_and_not_slept_off_a_real_device() {
        let port = SerialPort::looped(Settings::new(9_600));
        let started = std::time::Instant::now();
        port.wait(Duration::from_secs(3));
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(port.waited_micros(), 3_000_000);
    }

    #[cfg(all(feature = "linux", target_os = "linux"))]
    #[test]
    fn a_pseudo_terminal_opens_raw_and_carries_bytes_both_ways() {
        use nix::fcntl::OFlag;
        use nix::pty::{grantpt, posix_openpt, ptsname_r, unlockpt};
        use std::io::{Read, Write};
        use std::time::Instant;

        let mut far_end = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY).unwrap();
        grantpt(&far_end).unwrap();
        unlockpt(&far_end).unwrap();
        let path = ptsname_r(&far_end).unwrap();
        let port = SerialPort::open(&path, Settings::new(9_600).with_parity(Parity::Even)).unwrap();
        assert_eq!(port.kind(), PortKind::Device);

        // A NUL, the XON and XOFF flow control bytes, and a carriage return: a terminal left
        // cooked would swallow or translate every one of them.
        let awkward = [0x00, 0x11, 0x13, 0x0D];
        far_end.write_all(&awkward).unwrap();
        let mut buffer = [0u8; 8];
        let mut got = 0;
        while got < awkward.len() {
            let count = port
                .read(&mut buffer[got..], Duration::from_millis(500))
                .unwrap();
            assert!(count > 0, "the pseudo-terminal went quiet");
            got += count;
        }
        assert_eq!(&buffer[..got], &awkward);

        port.write(b"ok").unwrap();
        let mut back = [0u8; 2];
        far_end.read_exact(&mut back).unwrap();
        assert_eq!(&back, b"ok");

        let started = Instant::now();
        assert_eq!(
            port.read(&mut buffer, Duration::from_millis(40)).unwrap(),
            0
        );
        assert!(
            started.elapsed() >= Duration::from_millis(40),
            "a real read waits"
        );
        assert_eq!(port.waited_micros(), 40_000);

        far_end.write_all(b"stale").unwrap();
        std::thread::sleep(Duration::from_millis(20));
        port.discard_input().unwrap();
        assert_eq!(
            port.read(&mut buffer, Duration::from_millis(20)).unwrap(),
            0
        );

        assert!(matches!(
            SerialPort::open(&path, Settings::new(12_345)),
            Err(OpenError::Device { .. })
        ));
    }

    #[cfg(not(all(feature = "linux", target_os = "linux")))]
    #[test]
    fn only_linux_opens_a_device() {
        assert_eq!(
            SerialPort::open("/dev/serial0", Settings::new(115_200)).unwrap_err(),
            OpenError::Unsupported
        );
    }
}
