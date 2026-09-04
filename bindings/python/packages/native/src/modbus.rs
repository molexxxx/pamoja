//! Generated Python bindings for Modbus RTU framing.
//!
//! These mirror the `pamoja-modbus` Rust API. Each request builder returns a
//! complete RTU frame, CRC included, ready to write to an RS485 port: a caller
//! reaching the SDK from Python has no use for a bare PDU, so the PDU and the
//! frame around it are one step here.
//!
//! A received frame goes back through `parse_frame`, which validates the CRC
//! before anything can be read from it, and the values a device returned come out
//! as integers and booleans rather than bytes to unpack.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_modbus::{crc16, Adu, ModbusError, Pdu, Response};

use crate::PamojaError;

/// A received Modbus RTU frame whose CRC has been verified.
#[gen_stub_pyclass]
#[pyclass]
pub struct ModbusFrame {
    /// The unit (slave) address the frame is addressed to or came from.
    #[pyo3(get)]
    address: u8,
    /// The function code. An exception response carries the request's code with
    /// its high bit set, as it appeared on the wire.
    #[pyo3(get)]
    function_code: u8,
    /// The exception code a device reported, or `None` when the frame is not an
    /// exception response.
    #[pyo3(get)]
    exception: Option<u8>,
    /// The protocol data unit: the function code and its data.
    pdu: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ModbusFrame {
    /// The protocol data unit: the function code and its data, without the
    /// address or the CRC.
    #[getter]
    fn pdu<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.pdu)
    }

    /// Reads the 16-bit registers out of a read-registers reply.
    fn registers(&self) -> PyResult<Vec<u16>> {
        Response::new(&self.pdu)
            .registers()
            .map(Iterator::collect)
            .map_err(to_py)
    }

    /// Reads `count` coils or discrete inputs out of a read-bits reply.
    fn coils(&self, count: u16) -> PyResult<Vec<bool>> {
        Response::new(&self.pdu)
            .coils(count)
            .map(Iterator::collect)
            .map_err(to_py)
    }
}

/// Computes the CRC-16/MODBUS that every RTU frame ends with.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_crc16(data: Vec<u8>) -> u16 {
    crc16(&data)
}

/// Builds a read-coils request frame (function `0x01`).
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_read_coils<'py>(
    py: Python<'py>,
    address: u8,
    start: u16,
    count: u16,
) -> Bound<'py, PyBytes> {
    frame(py, Pdu::read_coils(start, count), address)
}

/// Builds a read-discrete-inputs request frame (function `0x02`).
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_read_discrete_inputs<'py>(
    py: Python<'py>,
    address: u8,
    start: u16,
    count: u16,
) -> Bound<'py, PyBytes> {
    frame(py, Pdu::read_discrete_inputs(start, count), address)
}

/// Builds a read-holding-registers request frame (function `0x03`).
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_read_holding_registers<'py>(
    py: Python<'py>,
    address: u8,
    start: u16,
    count: u16,
) -> Bound<'py, PyBytes> {
    frame(py, Pdu::read_holding_registers(start, count), address)
}

/// Builds a read-input-registers request frame (function `0x04`).
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_read_input_registers<'py>(
    py: Python<'py>,
    address: u8,
    start: u16,
    count: u16,
) -> Bound<'py, PyBytes> {
    frame(py, Pdu::read_input_registers(start, count), address)
}

/// Builds a write-single-coil request frame (function `0x05`).
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_write_single_coil<'py>(
    py: Python<'py>,
    address: u8,
    coil: u16,
    on: bool,
) -> Bound<'py, PyBytes> {
    frame(py, Pdu::write_single_coil(coil, on), address)
}

/// Builds a write-single-register request frame (function `0x06`).
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_write_single_register<'py>(
    py: Python<'py>,
    address: u8,
    register: u16,
    value: u16,
) -> Bound<'py, PyBytes> {
    frame(py, Pdu::write_single_register(register, value), address)
}

/// Builds a write-multiple-registers request frame (function `0x10`).
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_write_multiple_registers<'py>(
    py: Python<'py>,
    address: u8,
    start: u16,
    values: Vec<u16>,
) -> PyResult<Bound<'py, PyBytes>> {
    Pdu::write_multiple_registers(start, &values)
        .map(|pdu| frame(py, pdu, address))
        .map_err(to_py)
}

/// Builds a write-multiple-coils request frame (function `0x0F`).
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_write_multiple_coils<'py>(
    py: Python<'py>,
    address: u8,
    start: u16,
    values: Vec<bool>,
) -> PyResult<Bound<'py, PyBytes>> {
    Pdu::write_multiple_coils(start, &values)
        .map(|pdu| frame(py, pdu, address))
        .map_err(to_py)
}

/// Builds a request frame from a raw function code and data.
///
/// This is the escape hatch for the function codes the SDK does not name.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_raw<'py>(
    py: Python<'py>,
    address: u8,
    function_code: u8,
    data: Vec<u8>,
) -> PyResult<Bound<'py, PyBytes>> {
    Pdu::raw(function_code, &data)
        .map(|pdu| frame(py, pdu, address))
        .map_err(to_py)
}

/// Parses a received RTU frame, verifying its CRC.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_parse_frame(data: Vec<u8>) -> PyResult<ModbusFrame> {
    let adu = Adu::parse(&data).map_err(to_py)?;
    Ok(ModbusFrame {
        address: adu.address(),
        function_code: adu.function_code(),
        exception: adu.exception().map(pamoja_modbus::Exception::code),
        pdu: adu.pdu().to_vec(),
    })
}

/// Wraps a PDU into an addressed RTU frame and hands back its bytes.
fn frame<'py>(py: Python<'py>, pdu: Pdu, address: u8) -> Bound<'py, PyBytes> {
    PyBytes::new(py, pdu.to_adu(address).as_bytes())
}

/// Maps a Modbus error onto the SDK's Python exception.
fn to_py(error: ModbusError) -> PyErr {
    PamojaError::new_err(error.to_string())
}
