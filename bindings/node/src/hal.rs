//! Generated Node bindings for the bus layer.
//!
//! These mirror `pamoja_hal::bus`: one I2C bus shared by a program and every driver on it,
//! over the kernel's adapter on a Linux board, simulated parts, or a script of the transfers a
//! driver is expected to make. A part and a step are values built here and copied onto a bus,
//! so the program keeps its own and reads a simulated part back off the bus after a driver
//! has written to it.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_hal::bus::{BusError, BusKind, I2cBus as Bus};
use pamoja_hal::i2c::{ErrorKind, I2c, NoAcknowledgeSource};
use pamoja_hal::script::{I2cScript, I2cStep as Step};
use pamoja_hal::sim::I2cPart as Part;

/// What answers on a bus.
#[napi(string_enum, js_name = "I2cBusKind")]
pub enum I2cBusKind {
    /// The kernel's adapter, with real parts on real wires.
    Adapter,
    /// Simulated parts, answering from their registers.
    Simulated,
    /// A script of the transfers a driver is expected to make.
    Scripted,
}

/// How a scripted step fails the transfer that reaches it.
#[napi(string_enum, js_name = "I2cFault")]
pub enum I2cFault {
    /// Nothing acknowledged the address.
    NoAcknowledgeAddress,
    /// The part did not acknowledge a data byte.
    NoAcknowledgeData,
    /// A missing acknowledge, with no telling whether of the address or the data.
    NoAcknowledge,
    /// A bus error, such as a misplaced start or stop condition.
    Bus,
    /// Another controller won the bus.
    ArbitrationLoss,
    /// Data arrived faster than it was taken.
    Overrun,
    /// A failure of no more particular kind.
    Other,
}

/// A part that is not there, answering from 256 registers.
///
/// A write names a register and fills it and the ones after it; a read takes them back from
/// wherever the last write left off. What a driver writes stays written, so a program reads a
/// part's configuration back once the driver is done with it.
#[napi(js_name = "I2cPart")]
pub struct I2cPart {
    pub(crate) inner: Part,
}

#[napi]
impl I2cPart {
    /// A part answering at one address, with every register reading zero.
    #[napi(constructor)]
    pub fn new(address: u8) -> Self {
        I2cPart {
            inner: Part::new(address),
        }
    }

    /// Puts bytes in the part from a register on. Past the last register they wrap to the
    /// first.
    #[napi]
    pub fn load(&mut self, first: u8, bytes: Buffer) {
        self.inner.load(first, &bytes);
    }

    /// What one register holds now.
    #[napi]
    pub fn register(&self, register: u8) -> u8 {
        self.inner.register(register)
    }

    /// What consecutive registers hold, from one register on.
    #[napi]
    pub fn read(&self, first: u8, length: u32) -> Buffer {
        let mut at = first;
        let mut bytes = Vec::with_capacity(length as usize);
        for _ in 0..length {
            bytes.push(self.inner.register(at));
            at = at.wrapping_add(1);
        }
        Buffer::from(bytes)
    }

    /// The address the part answers to.
    #[napi(getter)]
    pub fn address(&self) -> u8 {
        self.inner.address()
    }

    /// How many transfers the part has served.
    #[napi(getter)]
    pub fn transfers(&self) -> u32 {
        count(self.inner.transfers())
    }
}

/// One transfer a script expects, and what the part answers.
#[napi(js_name = "I2cStep")]
pub struct I2cStep {
    inner: Step,
}

#[napi]
impl I2cStep {
    /// The driver writes exactly `bytes` to the address.
    #[napi(factory)]
    pub fn write(address: u8, bytes: Buffer) -> Self {
        I2cStep {
            inner: Step::write(address, bytes.to_vec()),
        }
    }

    /// The driver reads from the address and receives `reply`, whose length is the length it
    /// must ask for.
    #[napi(factory)]
    pub fn read(address: u8, reply: Buffer) -> Self {
        I2cStep {
            inner: Step::read(address, reply.to_vec()),
        }
    }

    /// The driver writes `bytes` and then reads `reply` in one transaction, the shape of a
    /// register read.
    #[napi(factory, js_name = "writeRead")]
    pub fn write_read(address: u8, bytes: Buffer, reply: Buffer) -> Self {
        I2cStep {
            inner: Step::write_read(address, bytes.to_vec(), reply.to_vec()),
        }
    }

    /// The next transfer to the address fails, the way a missing or busy part does.
    #[napi(factory)]
    pub fn fault(address: u8, fault: I2cFault) -> Self {
        I2cStep {
            inner: Step::fault(address, fault.into()),
        }
    }
}

/// One I2C bus, shared by the program and every driver built on it.
///
/// Transfers run one at a time, synchronously; each is quick, the time a few bytes take on
/// the wire. A failed transfer throws with the reason: nothing answered at the address, the
/// script expected something else, or the kernel's own words.
#[napi(js_name = "I2cBus")]
pub struct I2cBus {
    pub(crate) inner: Bus,
}

#[napi]
impl I2cBus {
    /// Opens the kernel's I2C adapter, such as `/dev/i2c-1` on a Raspberry Pi.
    ///
    /// Throws anywhere but Linux, and when the file cannot be opened as an adapter: the
    /// interface is not turned on, or the process may not use it.
    #[napi(factory)]
    pub fn open(path: String) -> napi::Result<Self> {
        Bus::open(path)
            .map(|inner| I2cBus { inner })
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// A bus of simulated parts, each answering at its own address. A later part at an
    /// address an earlier one holds takes its place.
    #[napi(factory)]
    pub fn simulated(parts: Option<Vec<&I2cPart>>) -> Self {
        let parts = parts.unwrap_or_default();
        I2cBus {
            inner: Bus::simulated(parts.into_iter().map(|part| part.inner.clone())),
        }
    }

    /// A bus that plays the steps in order and refuses any transfer that is not the next one.
    #[napi(factory)]
    pub fn scripted(steps: Vec<&I2cStep>) -> Self {
        let steps: Vec<Step> = steps.into_iter().map(|step| step.inner.clone()).collect();
        I2cBus {
            inner: Bus::scripted(I2cScript::new(steps)),
        }
    }

    /// Puts a copy of a part on a simulated bus, in place of any part at its address. Throws
    /// for a bus that is not simulated.
    #[napi]
    pub fn attach(&self, part: &I2cPart) -> napi::Result<()> {
        self.inner
            .attach(part.inner.clone())
            .map(drop)
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// What answers on the bus.
    #[napi(getter)]
    pub fn kind(&self) -> I2cBusKind {
        match self.inner.kind() {
            BusKind::Adapter => I2cBusKind::Adapter,
            BusKind::Simulated => I2cBusKind::Simulated,
            BusKind::Scripted => I2cBusKind::Scripted,
        }
    }

    /// Writes bytes to a part in one transaction: usually a register address and its value.
    #[napi]
    pub fn write(&self, address: u8, bytes: Buffer) -> napi::Result<()> {
        let mut bus = self.inner.clone();
        bus.write(address, &bytes).map_err(to_napi)
    }

    /// Reads `length` bytes from a part in one transaction.
    #[napi]
    pub fn read(&self, address: u8, length: u32) -> napi::Result<Buffer> {
        let mut bus = self.inner.clone();
        let mut bytes = vec![0u8; length as usize];
        bus.read(address, &mut bytes).map_err(to_napi)?;
        Ok(Buffer::from(bytes))
    }

    /// Writes bytes and then reads `length` bytes in one transaction, with a repeated start
    /// between them, which is how a register is read.
    #[napi(js_name = "writeRead")]
    pub fn write_read(&self, address: u8, bytes: Buffer, length: u32) -> napi::Result<Buffer> {
        let mut bus = self.inner.clone();
        let mut reply = vec![0u8; length as usize];
        bus.write_read(address, &bytes, &mut reply)
            .map_err(to_napi)?;
        Ok(Buffer::from(reply))
    }

    /// A copy of what a simulated part holds now, with whatever drivers have written to it,
    /// or `null` when the bus is not simulated or no part holds the address.
    #[napi]
    pub fn part(&self, address: u8) -> Option<I2cPart> {
        self.inner.part(address).map(|inner| I2cPart { inner })
    }

    /// How many transfers have been made on the bus, by the program and every driver on it,
    /// including any that failed.
    #[napi(getter)]
    pub fn transfers(&self) -> u32 {
        count(self.inner.transfers())
    }

    /// How many steps a script has left, or `null` when the bus is not scripted.
    #[napi(getter)]
    pub fn remaining(&self) -> Option<u32> {
        self.inner.remaining().map(count)
    }

    /// How long the drivers on the bus have asked to wait, in microseconds, whether or not
    /// the process slept through it.
    #[napi(getter, js_name = "waitedMicros")]
    pub fn waited_micros(&self) -> f64 {
        self.inner.waited_micros() as f64
    }
}

impl From<I2cFault> for ErrorKind {
    fn from(value: I2cFault) -> Self {
        match value {
            I2cFault::NoAcknowledgeAddress => {
                ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)
            }
            I2cFault::NoAcknowledgeData => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data),
            I2cFault::NoAcknowledge => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown),
            I2cFault::Bus => ErrorKind::Bus,
            I2cFault::ArbitrationLoss => ErrorKind::ArbitrationLoss,
            I2cFault::Overrun => ErrorKind::Overrun,
            I2cFault::Other => ErrorKind::Other,
        }
    }
}

/// A count as JavaScript holds it, saturating rather than wrapping.
fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// A failed transfer as a JavaScript error.
fn to_napi(error: BusError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
