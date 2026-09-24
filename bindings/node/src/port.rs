//! Generated Node bindings for one serial port.
//!
//! These mirror `pamoja_hal::port`: one serial port shared by a program and every driver on it,
//! over the kernel's serial device on a Linux board, a line looped back on itself, one end of a
//! null-modem pair, or a script of the writes a driver is expected to make. A real line takes
//! time, so a write, a read, and a wait run on a worker thread and return promises: a write
//! resolves once the bytes have left the UART, and a read once bytes have arrived or its
//! timeout has passed. On anything but the kernel's device nothing waits, and the time a read
//! would have waited is counted instead.

use crate::checked::{self, OptionalWhole};
use std::time::Duration;

use napi::bindgen_prelude::{spawn_blocking, Buffer};
use napi_derive::napi;
use pamoja_hal::port::{
    Parity as LineParity, PortKind, PortStep, SerialPort as Port, Settings, StopBits,
};

/// The parity bit each character carries.
#[napi(string_enum)]
pub enum Parity {
    /// No parity bit.
    None,
    /// A bit that makes the count of ones even, what Modbus RTU asks for by default.
    Even,
    /// A bit that makes the count of ones odd.
    Odd,
}

/// What is on the other end of a serial port.
#[napi(string_enum, js_name = "SerialPortKind")]
pub enum SerialPortKind {
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

/// A port's speed and character format: eight data bits, with the parity and stop bits given.
#[napi(object)]
pub struct SerialSettings {
    /// The speed, in bits a second.
    pub baud: checked::u32,
    /// The parity bit, none unless given.
    pub parity: Option<Parity>,
    /// 1 or 2 stop bits, 1 unless given.
    pub stop_bits: Option<checked::u8>,
}

impl SerialSettings {
    pub(crate) fn settings(&self) -> napi::Result<Settings> {
        if self.baud.get() == 0 {
            return Err(napi::Error::from_reason("the speed must be above zero"));
        }
        let parity = match self.parity {
            None | Some(Parity::None) => LineParity::None,
            Some(Parity::Even) => LineParity::Even,
            Some(Parity::Odd) => LineParity::Odd,
        };
        let stop_bits = match self.stop_bits.get().unwrap_or(1) {
            1 => StopBits::One,
            2 => StopBits::Two,
            other => {
                return Err(napi::Error::from_reason(format!(
                    "{other} stop bits is not 1 or 2"
                )))
            }
        };
        Ok(Settings::new(self.baud.get())
            .with_parity(parity)
            .with_stop_bits(stop_bits))
    }
}

impl From<Settings> for SerialSettings {
    fn from(settings: Settings) -> Self {
        SerialSettings {
            baud: settings.baud.into(),
            parity: Some(match settings.parity {
                LineParity::None => Parity::None,
                LineParity::Even => Parity::Even,
                LineParity::Odd => Parity::Odd,
            }),
            stop_bits: Some(match settings.stop_bits {
                StopBits::One => 1.into(),
                StopBits::Two => 2.into(),
            }),
        }
    }
}

/// One step of a scripted port: a write the program is expected to make, or bytes the far end
/// sends.
#[napi(js_name = "SerialStep")]
pub struct SerialStep {
    inner: PortStep,
}

#[napi]
impl SerialStep {
    /// A write the program is expected to make, in one call.
    #[napi(factory)]
    pub fn write(bytes: Buffer) -> Self {
        SerialStep {
            inner: PortStep::Write(bytes.to_vec()),
        }
    }

    /// Bytes the far end sends, readable once every step before them has happened.
    #[napi(factory)]
    pub fn read(bytes: Buffer) -> Self {
        SerialStep {
            inner: PortStep::Read(bytes.to_vec()),
        }
    }
}

/// One serial port, shared by the program and every driver built on it.
///
/// `SerialPort.open(path, settings)` opens the kernel's serial device raw on a Linux board and
/// throws anywhere else. `SerialPort.looped(settings)` is a line with TX wired to RX,
/// `SerialPort.pair(settings)` the two ends of a null-modem cable, and
/// `SerialPort.scripted(settings, steps)` a port that checks each write against a script.
/// `write` and `read` return promises that reject with the reason.
#[napi(js_name = "SerialPort")]
pub struct SerialPort {
    pub(crate) inner: Port,
}

#[napi]
impl SerialPort {
    /// Opens the kernel's serial device raw: `/dev/serial0` for a Raspberry Pi's own UART,
    /// `/dev/ttyUSB0` or `/dev/ttyACM0` for a USB adapter. Throws anywhere but Linux, for a
    /// speed that is not a standard rate from 1200 to 921600, and when the device cannot be
    /// opened.
    #[napi(factory)]
    pub fn open(path: String, settings: SerialSettings) -> napi::Result<Self> {
        Port::open(path, settings.settings()?)
            .map(|inner| SerialPort { inner })
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// A line looped back on itself: every byte written is waiting to be read.
    #[napi(factory)]
    pub fn looped(settings: SerialSettings) -> napi::Result<Self> {
        Ok(SerialPort {
            inner: Port::looped(settings.settings()?),
        })
    }

    /// The two ends of a null-modem pair: what one end writes, the other reads.
    #[napi]
    pub fn pair(settings: SerialSettings) -> napi::Result<Vec<SerialPort>> {
        let (one, other) = Port::pair(settings.settings()?);
        Ok(vec![SerialPort { inner: one }, SerialPort { inner: other }])
    }

    /// A port that checks each write against the next step of a script, and makes the bytes
    /// the far end sends readable as the script reaches them.
    #[napi(factory)]
    pub fn scripted(settings: SerialSettings, steps: Vec<&SerialStep>) -> napi::Result<Self> {
        let steps: Vec<PortStep> = steps.into_iter().map(|step| step.inner.clone()).collect();
        Ok(SerialPort {
            inner: Port::scripted(settings.settings()?, steps),
        })
    }

    /// Returns the bits one character takes on the wire: a start bit, eight data bits, the
    /// parity bit if there is one, and the stop bits.
    #[napi(js_name = "bitsPerCharacter")]
    pub fn bits_per_character(settings: SerialSettings) -> napi::Result<u32> {
        Ok(settings.settings()?.bits_per_character())
    }

    /// Returns how long one character takes on the wire, in nanoseconds, rounded up.
    #[napi(js_name = "characterNanos")]
    pub fn character_nanos(settings: SerialSettings) -> napi::Result<f64> {
        Ok(settings.settings()?.character_nanos() as f64)
    }

    /// Returns how long `bytes` sent back to back take on the wire, in microseconds, rounded
    /// up.
    #[napi(js_name = "transferMicros")]
    pub fn transfer_micros(settings: SerialSettings, bytes: checked::u32) -> napi::Result<f64> {
        Ok(settings.settings()?.transfer_micros(bytes.get() as usize) as f64)
    }

    /// Writes settings the way a device's manual does, such as `9600 8E1`.
    #[napi]
    pub fn describe(settings: SerialSettings) -> napi::Result<String> {
        Ok(settings.settings()?.to_string())
    }

    /// What is on the other end of the port.
    #[napi(getter)]
    pub fn kind(&self) -> SerialPortKind {
        match self.inner.kind() {
            PortKind::Device => SerialPortKind::Device,
            PortKind::Looped => SerialPortKind::Looped,
            PortKind::Paired => SerialPortKind::Paired,
            PortKind::Simulated => SerialPortKind::Simulated,
            PortKind::Scripted => SerialPortKind::Scripted,
        }
    }

    /// The speed and character format the port runs at.
    #[napi(getter)]
    pub fn settings(&self) -> SerialSettings {
        self.inner.settings().into()
    }

    /// Writes bytes, resolving once they have left the UART. Rejects when a script expected
    /// another write, or with the kernel's reason.
    #[napi]
    pub async fn write(&self, bytes: Buffer) -> napi::Result<()> {
        let port = self.inner.clone();
        let bytes = bytes.to_vec();
        spawn_blocking(move || port.write(&bytes))
            .await
            .map_err(finished)?
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// Reads up to `max` bytes, waiting up to `timeoutMs` for the first one when nothing has
    /// arrived. Resolves with what arrived, empty when the timeout passed with nothing.
    #[napi]
    pub async fn read(&self, max: checked::u32, timeout_ms: f64) -> napi::Result<Buffer> {
        let port = self.inner.clone();
        let timeout = millis(timeout_ms)?;
        let bytes = spawn_blocking(move || {
            let mut buffer = vec![0u8; max.get() as usize];
            port.read(&mut buffer, timeout).map(|count| {
                buffer.truncate(count);
                buffer
            })
        })
        .await
        .map_err(finished)?
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
        Ok(Buffer::from(bytes))
    }

    /// Drops whatever has arrived and not been read, as a client does before a request so a
    /// stale reply cannot be taken for the new one.
    #[napi(js_name = "discardInput")]
    pub fn discard_input(&self) -> napi::Result<()> {
        self.inner
            .discard_input()
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// Waits, as a protocol does to leave the line silent between frames: really on the
    /// kernel's device, and anywhere else the wait is only counted.
    #[napi]
    pub async fn wait(&self, ms: f64) -> napi::Result<()> {
        let port = self.inner.clone();
        let duration = millis(ms)?;
        spawn_blocking(move || port.wait(duration))
            .await
            .map_err(finished)
    }

    /// How many bytes have been written through the port.
    #[napi(getter)]
    pub fn written(&self) -> f64 {
        self.inner.written() as f64
    }

    /// How many bytes have been read through the port.
    #[napi(getter)]
    pub fn received(&self) -> f64 {
        self.inner.received() as f64
    }

    /// How long reads have waited without an answer, and waits have waited, in microseconds,
    /// whether or not the process slept through it.
    #[napi(getter, js_name = "waitedMicros")]
    pub fn waited_micros(&self) -> f64 {
        self.inner.waited_micros() as f64
    }

    /// How many steps a script has left, or `null` when the port is not scripted.
    #[napi(getter)]
    pub fn remaining(&self) -> Option<u32> {
        self.inner
            .remaining()
            .map(|remaining| u32::try_from(remaining).unwrap_or(u32::MAX))
    }
}

fn millis(ms: f64) -> napi::Result<Duration> {
    if ms.is_finite() && ms >= 0.0 {
        Ok(Duration::from_secs_f64(ms / 1_000.0))
    } else {
        Err(napi::Error::from_reason(
            "a time must be a finite number of milliseconds, zero or more",
        ))
    }
}

fn finished(error: impl std::fmt::Display) -> napi::Error {
    napi::Error::from_reason(format!("the port call did not finish: {error}"))
}
