//! Python bindings for one serial port.
//!
//! These mirror `pamoja_hal::port`: one serial port shared by a program and every driver on it,
//! over the kernel's serial device on a Linux board, a line looped back on itself, one end of a
//! null-modem pair, or a script of the writes a driver is expected to make. A write, a read,
//! and a wait release the interpreter while the line is busy. The settings cross as a speed, a
//! parity name, and a stop bit count, which the facade gathers into `SerialSettings`.

use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_hal::port::{Parity, PortKind, PortStep, SerialPort as Port, Settings, StopBits};

use crate::PamojaError;

pub(crate) fn settings(baud: u32, parity: &str, stop_bits: u8) -> PyResult<Settings> {
    if baud == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "the speed must be above zero",
        ));
    }
    let parity = match parity {
        "None" => Parity::None,
        "Even" => Parity::Even,
        "Odd" => Parity::Odd,
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "parity {other:?} is not None, Even, or Odd"
            )))
        }
    };
    let stop_bits = match stop_bits {
        1 => StopBits::One,
        2 => StopBits::Two,
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "{other} stop bits is not 1 or 2"
            )))
        }
    };
    Ok(Settings::new(baud)
        .with_parity(parity)
        .with_stop_bits(stop_bits))
}

fn parity_name(parity: Parity) -> &'static str {
    match parity {
        Parity::None => "None",
        Parity::Even => "Even",
        Parity::Odd => "Odd",
    }
}

fn to_py(error: impl ToString) -> PyErr {
    PamojaError::new_err(error.to_string())
}

/// Returns the bits one character takes on the wire: a start bit, eight data bits, the parity
/// bit if there is one, and the stop bits.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn serial_bits_per_character(baud: u32, parity: &str, stop_bits: u8) -> PyResult<u32> {
    Ok(settings(baud, parity, stop_bits)?.bits_per_character())
}

/// Returns how long one character takes on the wire, in nanoseconds, rounded up.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn serial_character_nanos(baud: u32, parity: &str, stop_bits: u8) -> PyResult<u64> {
    Ok(settings(baud, parity, stop_bits)?.character_nanos())
}

/// Returns how long `count` bytes sent back to back take on the wire, in microseconds, rounded
/// up.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn serial_transfer_micros(
    baud: u32,
    parity: &str,
    stop_bits: u8,
    count: usize,
) -> PyResult<u64> {
    Ok(settings(baud, parity, stop_bits)?.transfer_micros(count))
}

/// One step of a scripted port: a write the program is expected to make, or bytes the far end
/// sends.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct SerialStep {
    inner: PortStep,
}

#[gen_stub_pymethods]
#[pymethods]
impl SerialStep {
    /// A write the program is expected to make, in one call.
    #[staticmethod]
    fn write(data: Vec<u8>) -> Self {
        SerialStep {
            inner: PortStep::Write(data),
        }
    }

    /// Bytes the far end sends, readable once every step before them has happened.
    #[staticmethod]
    fn read(data: Vec<u8>) -> Self {
        SerialStep {
            inner: PortStep::Read(data),
        }
    }
}

/// One serial port, shared by the program and every driver built on it.
///
/// A failed write or read raises `PamojaError` with the reason: the script expected another
/// write, or the kernel's own words.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct SerialPort {
    pub(crate) inner: Port,
}

#[gen_stub_pymethods]
#[pymethods]
impl SerialPort {
    /// Opens the kernel's serial device raw: `/dev/serial0` for a Raspberry Pi's own UART,
    /// `/dev/ttyUSB0` or `/dev/ttyACM0` for a USB adapter.
    ///
    /// Raises `PamojaError` anywhere but Linux, for a speed that is not a standard rate from
    /// 1200 to 921600, and when the device cannot be opened.
    #[staticmethod]
    fn open(
        py: Python<'_>,
        path: String,
        baud: u32,
        parity: &str,
        stop_bits: u8,
    ) -> PyResult<SerialPort> {
        let settings = settings(baud, parity, stop_bits)?;
        py.detach(|| Port::open(path, settings))
            .map(|inner| SerialPort { inner })
            .map_err(to_py)
    }

    /// A line looped back on itself: every byte written is waiting to be read.
    #[staticmethod]
    fn looped(baud: u32, parity: &str, stop_bits: u8) -> PyResult<SerialPort> {
        Ok(SerialPort {
            inner: Port::looped(settings(baud, parity, stop_bits)?),
        })
    }

    /// The two ends of a null-modem pair: what one end writes, the other reads.
    #[staticmethod]
    fn pair(baud: u32, parity: &str, stop_bits: u8) -> PyResult<(SerialPort, SerialPort)> {
        let (one, other) = Port::pair(settings(baud, parity, stop_bits)?);
        Ok((SerialPort { inner: one }, SerialPort { inner: other }))
    }

    /// A port that checks each write against the next step of a script, and makes the bytes
    /// the far end sends readable as the script reaches them.
    #[staticmethod]
    fn scripted(
        baud: u32,
        parity: &str,
        stop_bits: u8,
        steps: Vec<PyRef<'_, SerialStep>>,
    ) -> PyResult<SerialPort> {
        let steps: Vec<PortStep> = steps.iter().map(|step| step.inner.clone()).collect();
        Ok(SerialPort {
            inner: Port::scripted(settings(baud, parity, stop_bits)?, steps),
        })
    }

    /// What is on the other end: `"Device"`, `"Looped"`, `"Paired"`, `"Simulated"`, or
    /// `"Scripted"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner.kind() {
            PortKind::Device => "Device",
            PortKind::Looped => "Looped",
            PortKind::Paired => "Paired",
            PortKind::Simulated => "Simulated",
            PortKind::Scripted => "Scripted",
        }
    }

    /// The speed, the parity name, and the stop bit count the port runs at.
    #[getter]
    fn settings(&self) -> (u32, &'static str, u8) {
        let settings = self.inner.settings();
        let stop_bits = match settings.stop_bits {
            StopBits::One => 1,
            StopBits::Two => 2,
        };
        (settings.baud, parity_name(settings.parity), stop_bits)
    }

    /// Writes bytes, returning once they have left the UART.
    fn write(&self, py: Python<'_>, data: Vec<u8>) -> PyResult<()> {
        let port = self.inner.clone();
        py.detach(|| port.write(&data)).map_err(to_py)
    }

    /// Reads up to `size` bytes, waiting up to `timeout_micros` for the first one when nothing
    /// has arrived, and returns what arrived, empty when the timeout passed with nothing.
    fn read<'py>(
        &self,
        py: Python<'py>,
        size: usize,
        timeout_micros: u64,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let port = self.inner.clone();
        let mut buffer = vec![0u8; size];
        let count = py
            .detach(|| port.read(&mut buffer, Duration::from_micros(timeout_micros)))
            .map_err(to_py)?;
        Ok(PyBytes::new(py, &buffer[..count]))
    }

    /// Drops whatever has arrived and not been read.
    fn discard_input(&self) -> PyResult<()> {
        self.inner.discard_input().map_err(to_py)
    }

    /// Waits, really on the kernel's device and anywhere else only counted.
    fn wait(&self, py: Python<'_>, micros: u64) {
        let port = self.inner.clone();
        py.detach(|| port.wait(Duration::from_micros(micros)));
    }

    /// How many bytes have been written through the port.
    #[getter]
    fn written(&self) -> usize {
        self.inner.written()
    }

    /// How many bytes have been read through the port.
    #[getter]
    fn received(&self) -> usize {
        self.inner.received()
    }

    /// How long reads have waited without an answer, and waits have waited, in microseconds,
    /// whether or not the process slept through it.
    #[getter]
    fn waited_micros(&self) -> u64 {
        self.inner.waited_micros()
    }

    /// How many steps a script has left, or `None` when the port is not scripted.
    #[getter]
    fn remaining(&self) -> Option<usize> {
        self.inner.remaining()
    }
}
