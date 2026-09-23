//! Generated Python bindings for the bus layer.
//!
//! These mirror `pamoja_hal::bus`: one I2C bus shared by a program and every driver on it,
//! over the kernel's adapter on a Linux board, simulated parts, or a script of the transfers a
//! driver is expected to make. A part and a step are values built here and copied onto a bus,
//! so the program keeps its own and reads a simulated part back off the bus after a driver has
//! written to it.
//!
//! A bus kind and a scripted fault cross as plain strings, which the facade turns back into
//! Python enum members.
//!
//! A simulated part is one of three classes, as in Rust: `I2cPart` with registers a byte wide,
//! `WordPart` with registers sixteen bits wide, and `CommandPart` with commands that leave
//! replies. A bus takes any of them and gives each back as its own class.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use pyo3_stub_gen::impl_stub_type;

use pamoja_hal::bus::{BusError, BusKind, I2cBus as Bus};
use pamoja_hal::i2c::{ErrorKind, I2c, NoAcknowledgeSource};
use pamoja_hal::script::{I2cScript, I2cStep as Step};
use pamoja_hal::sim::{CommandPart as Commands, I2cPart as Part, Part as Any, WordPart as Words};

use crate::PamojaError;

/// Any simulated part, as a bus takes it.
#[derive(FromPyObject)]
pub enum AnyPart<'py> {
    /// A part whose registers are a byte wide.
    Bytes(PyRef<'py, I2cPart>),
    /// A part whose registers are sixteen bits wide.
    Words(PyRef<'py, WordPart>),
    /// A part that takes commands.
    Commands(PyRef<'py, CommandPart>),
}

impl_stub_type!(AnyPart<'_> = I2cPart | WordPart | CommandPart);

impl AnyPart<'_> {
    /// A copy of the part as the bus holds it.
    fn copy(&self) -> Any {
        match self {
            AnyPart::Bytes(part) => part.inner.clone().into(),
            AnyPart::Words(part) => part.inner.clone().into(),
            AnyPart::Commands(part) => part.inner.clone().into(),
        }
    }
}

/// A simulated part given back as the class of part it is.
///
/// It lives only until pyo3 turns it into a Python object, so its size does not matter.
#[derive(IntoPyObject)]
#[allow(clippy::large_enum_variant)]
pub enum HeldPart {
    /// A part whose registers are a byte wide.
    Bytes(I2cPart),
    /// A part whose registers are sixteen bits wide.
    Words(WordPart),
    /// A part that takes commands.
    Commands(CommandPart),
}

impl_stub_type!(HeldPart = I2cPart | WordPart | CommandPart);

/// A part that is not there, answering from 256 registers.
///
/// A write names a register and fills it and the ones after it; a read takes them back from
/// wherever the last write left off. What a driver writes stays written.
#[gen_stub_pyclass]
#[pyclass]
pub struct I2cPart {
    pub(crate) inner: Part,
}

#[gen_stub_pymethods]
#[pymethods]
impl I2cPart {
    /// A part answering at one address, with every register reading zero.
    #[new]
    fn new(address: u8) -> Self {
        I2cPart {
            inner: Part::new(address),
        }
    }

    /// Puts bytes in the part from a register on. Past the last register they wrap to the
    /// first.
    fn load(&mut self, first: u8, data: Vec<u8>) {
        self.inner.load(first, &data);
    }

    /// What one register holds now.
    fn register(&self, register: u8) -> u8 {
        self.inner.register(register)
    }

    /// What consecutive registers hold, from one register on.
    fn read<'py>(&self, py: Python<'py>, first: u8, length: usize) -> Bound<'py, PyBytes> {
        let mut at = first;
        let mut bytes = Vec::with_capacity(length);
        for _ in 0..length {
            bytes.push(self.inner.register(at));
            at = at.wrapping_add(1);
        }
        PyBytes::new(py, &bytes)
    }

    /// The address the part answers to.
    #[getter]
    fn address(&self) -> u8 {
        self.inner.address()
    }

    /// How many transfers the part has served.
    #[getter]
    fn transfers(&self) -> usize {
        self.inner.transfers()
    }
}

/// A part that is not there, answering from 256 registers sixteen bits wide.
///
/// A pointer byte names a register and a register travels most significant byte first. Bits
/// the part sets for itself, such as a conversion-ready flag, are marked with `read_only` and
/// keep the part's value whatever a driver writes.
#[gen_stub_pyclass]
#[pyclass]
pub struct WordPart {
    pub(crate) inner: Words,
}

#[gen_stub_pymethods]
#[pymethods]
impl WordPart {
    /// A part answering at one address, with every register reading zero.
    #[new]
    fn new(address: u8) -> Self {
        WordPart {
            inner: Words::new(address),
        }
    }

    /// Puts a value in one register, read-only bits included, the way the part itself would.
    fn set(&mut self, register: u8, value: u16) {
        self.inner.set(register, value);
    }

    /// Marks bits of one register as the part's to set: a driver's write leaves them as the
    /// part holds them.
    fn read_only(&mut self, register: u8, mask: u16) {
        self.inner = self.inner.clone().read_only(register, mask);
    }

    /// What one register holds now, which is what a driver wrote there apart from the
    /// read-only bits.
    fn word(&self, register: u8) -> u16 {
        self.inner.word(register)
    }

    /// The address the part answers to.
    #[getter]
    fn address(&self) -> u8 {
        self.inner.address()
    }

    /// How many transfers the part has served.
    #[getter]
    fn transfers(&self) -> usize {
        self.inner.transfers()
    }
}

/// A part that is not there, answering commands with the replies it was given.
///
/// A write sends a command and any arguments after it; a read takes the reply that command
/// left, once, padded with `0xFF`. A read with no reply waiting is not acknowledged.
#[gen_stub_pyclass]
#[pyclass]
pub struct CommandPart {
    pub(crate) inner: Commands,
}

#[gen_stub_pymethods]
#[pymethods]
impl CommandPart {
    /// A part answering at one address that has been given no replies yet; a command takes
    /// `width` bytes.
    #[new]
    #[pyo3(signature = (address, width = 2))]
    fn new(address: u8, width: usize) -> Self {
        CommandPart {
            inner: Commands::new(address, width),
        }
    }

    /// Answers one command with a reply from now on, in place of any reply given before.
    fn answer(&mut self, command: Vec<u8>, reply: Vec<u8>) {
        self.inner.answer(&command, &reply);
    }

    /// Every write the part has received, oldest first: a command and any arguments after it.
    #[getter]
    fn received<'py>(&self, py: Python<'py>) -> Vec<Bound<'py, PyBytes>> {
        self.inner
            .received()
            .iter()
            .map(|write| PyBytes::new(py, write))
            .collect()
    }

    /// The address the part answers to.
    #[getter]
    fn address(&self) -> u8 {
        self.inner.address()
    }

    /// How many transfers the part has served.
    #[getter]
    fn transfers(&self) -> usize {
        self.inner.transfers()
    }
}

/// One transfer a script expects, and what the part answers.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct I2cStep {
    inner: Step,
}

#[gen_stub_pymethods]
#[pymethods]
impl I2cStep {
    /// The driver writes exactly `data` to the address.
    #[staticmethod]
    fn write(address: u8, data: Vec<u8>) -> Self {
        I2cStep {
            inner: Step::write(address, data),
        }
    }

    /// The driver reads from the address and receives `reply`, whose length is the length it
    /// must ask for.
    #[staticmethod]
    fn read(address: u8, reply: Vec<u8>) -> Self {
        I2cStep {
            inner: Step::read(address, reply),
        }
    }

    /// The driver writes `data` and then reads `reply` in one transaction, the shape of a
    /// register read.
    #[staticmethod]
    fn write_read(address: u8, data: Vec<u8>, reply: Vec<u8>) -> Self {
        I2cStep {
            inner: Step::write_read(address, data, reply),
        }
    }

    /// The next transfer to the address fails, the way a missing or busy part does. The fault
    /// is `"NoAcknowledgeAddress"`, `"NoAcknowledgeData"`, `"NoAcknowledge"`, `"Bus"`,
    /// `"ArbitrationLoss"`, `"Overrun"`, or `"Other"`.
    #[staticmethod]
    fn fault(address: u8, fault: &str) -> PyResult<Self> {
        Ok(I2cStep {
            inner: Step::fault(address, read_fault(fault)?),
        })
    }
}

/// One I2C bus, shared by the program and every driver built on it.
///
/// A failed transfer raises `PamojaError` with the reason: nothing answered at the address,
/// the script expected something else, or the kernel's own words.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct I2cBus {
    pub(crate) inner: Bus,
}

#[gen_stub_pymethods]
#[pymethods]
impl I2cBus {
    /// Opens the kernel's I2C adapter, such as `/dev/i2c-1` on a Raspberry Pi.
    ///
    /// Raises `PamojaError` anywhere but Linux, and when the file cannot be opened as an
    /// adapter: the interface is not turned on, or the process may not use it.
    #[staticmethod]
    fn open(py: Python<'_>, path: String) -> PyResult<I2cBus> {
        py.detach(|| Bus::open(path))
            .map(|inner| I2cBus { inner })
            .map_err(|error| PamojaError::new_err(error.to_string()))
    }

    /// A bus of simulated parts of any kind, each answering at its own address. A later part
    /// at an address an earlier one holds takes its place.
    #[staticmethod]
    #[pyo3(signature = (parts = None))]
    fn simulated(parts: Option<Vec<AnyPart<'_>>>) -> I2cBus {
        let parts = parts.unwrap_or_default();
        I2cBus {
            inner: Bus::simulated(parts.iter().map(AnyPart::copy)),
        }
    }

    /// A bus that plays the steps in order and refuses any transfer that is not the next one.
    #[staticmethod]
    fn scripted(steps: Vec<PyRef<'_, I2cStep>>) -> I2cBus {
        let steps: Vec<Step> = steps.iter().map(|step| step.inner.clone()).collect();
        I2cBus {
            inner: Bus::scripted(I2cScript::new(steps)),
        }
    }

    /// Puts a copy of a part of any kind on a simulated bus, in place of any part at its
    /// address. Raises `PamojaError` for a bus that is not simulated.
    fn attach(&self, part: AnyPart<'_>) -> PyResult<()> {
        self.inner
            .attach(part.copy())
            .map(drop)
            .map_err(|error| PamojaError::new_err(error.to_string()))
    }

    /// What answers on the bus: `"Adapter"`, `"Simulated"`, or `"Scripted"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner.kind() {
            BusKind::Adapter => "Adapter",
            BusKind::Simulated => "Simulated",
            BusKind::Scripted => "Scripted",
        }
    }

    /// Writes bytes to a part in one transaction: usually a register address and its value.
    fn write(&self, py: Python<'_>, address: u8, data: Vec<u8>) -> PyResult<()> {
        let mut bus = self.inner.clone();
        py.detach(|| bus.write(address, &data)).map_err(to_py)
    }

    /// Reads `length` bytes from a part in one transaction.
    fn read<'py>(
        &self,
        py: Python<'py>,
        address: u8,
        length: usize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let mut bus = self.inner.clone();
        let mut bytes = vec![0u8; length];
        py.detach(|| bus.read(address, &mut bytes)).map_err(to_py)?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Writes bytes and then reads `length` bytes in one transaction, with a repeated start
    /// between them, which is how a register is read.
    fn write_read<'py>(
        &self,
        py: Python<'py>,
        address: u8,
        data: Vec<u8>,
        length: usize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let mut bus = self.inner.clone();
        let mut reply = vec![0u8; length];
        py.detach(|| bus.write_read(address, &data, &mut reply))
            .map_err(to_py)?;
        Ok(PyBytes::new(py, &reply))
    }

    /// A copy of what a simulated part holds now, with whatever drivers have written to it,
    /// as the class of part it is, or `None` when the bus is not simulated or no part holds
    /// the address.
    fn part(&self, address: u8) -> Option<HeldPart> {
        self.inner.part::<Any>(address).map(|part| match part {
            Any::Bytes(inner) => HeldPart::Bytes(I2cPart { inner }),
            Any::Words(inner) => HeldPart::Words(WordPart { inner: *inner }),
            Any::Commands(inner) => HeldPart::Commands(CommandPart { inner }),
        })
    }

    /// How many transfers have been made on the bus, by the program and every driver on it,
    /// including any that failed.
    #[getter]
    fn transfers(&self) -> usize {
        self.inner.transfers()
    }

    /// How many steps a script has left, or `None` when the bus is not scripted.
    #[getter]
    fn remaining(&self) -> Option<usize> {
        self.inner.remaining()
    }

    /// How long the drivers on the bus have asked to wait, in microseconds, whether or not the
    /// process slept through it.
    #[getter]
    fn waited_micros(&self) -> u64 {
        self.inner.waited_micros()
    }
}

/// Reads a scripted fault back from its name.
fn read_fault(fault: &str) -> PyResult<ErrorKind> {
    Ok(match fault {
        "NoAcknowledgeAddress" => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address),
        "NoAcknowledgeData" => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data),
        "NoAcknowledge" => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown),
        "Bus" => ErrorKind::Bus,
        "ArbitrationLoss" => ErrorKind::ArbitrationLoss,
        "Overrun" => ErrorKind::Overrun,
        "Other" => ErrorKind::Other,
        _ => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "{fault} is not an I2C fault"
            )))
        }
    })
}

/// A failed transfer as a Python exception.
fn to_py(error: BusError) -> PyErr {
    PamojaError::new_err(error.to_string())
}
