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
//!
//! A `ModbusClient` runs whole transactions over a `SerialPort`, releasing the
//! interpreter while the line is busy, and a `ModbusServer` is a device a `ModbusLine`
//! puts on the far end of a simulated port. A failed transaction raises
//! `ModbusClientError`, whose `kind` tells a device's exception from a timeout.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_hal::port::SerialPort as Port;
use pamoja_modbus::{crc16, Adu, Client, ClientError, Line, ModbusError, Pdu, Response, Server};

use crate::port::SerialPort;
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

/// Builds the reply a device sends to a read-holding-registers request.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_read_holding_registers_reply<'py>(
    py: Python<'py>,
    address: u8,
    values: Vec<u16>,
) -> PyResult<Bound<'py, PyBytes>> {
    let pdu = Pdu::read_holding_registers_reply(&values).map_err(to_py)?;
    Ok(frame(py, pdu, address))
}

/// Builds the reply a device sends to a read-input-registers request.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn modbus_read_input_registers_reply<'py>(
    py: Python<'py>,
    address: u8,
    values: Vec<u16>,
) -> PyResult<Bound<'py, PyBytes>> {
    let pdu = Pdu::read_input_registers_reply(&values).map_err(to_py)?;
    Ok(frame(py, pdu, address))
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

pyo3::create_exception!(
    pamoja,
    ModbusClientError,
    PamojaError,
    "Raised when a Modbus transaction fails. `kind` names why: `request`, `broadcast_read`, `port`, `timeout`, `frame`, `wrong_unit`, `wrong_function`, `mismatch`, or `exception`. `unit` is the unit asked, and `function_code`, `found`, `exception`, and `received` carry what goes with the kind, and are `None` otherwise."
);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn client_error(unit: u8, error: &ClientError) -> PyErr {
    let raised = ModbusClientError::new_err(error.to_string());
    let (kind, function_code, found, exception, received) = match *error {
        ClientError::Request(_) => ("request", None, None, None, None),
        ClientError::BroadcastRead => ("broadcast_read", None, None, None, None),
        ClientError::Port(_) => ("port", None, None, None, None),
        ClientError::Timeout { received, .. } => ("timeout", None, None, None, Some(received)),
        ClientError::Frame(_) => ("frame", None, None, None, None),
        ClientError::WrongUnit { found, .. } => ("wrong_unit", None, Some(found), None, None),
        ClientError::WrongFunction { expected, found } => {
            ("wrong_function", Some(expected), Some(found), None, None)
        }
        ClientError::Mismatch { .. } => ("mismatch", None, None, None, None),
        ClientError::Exception {
            function,
            exception,
            ..
        } => (
            "exception",
            Some(function),
            None,
            Some(exception.code()),
            None,
        ),
    };
    Python::attach(|py| {
        let value = raised.value(py);
        let _ = value.setattr("kind", kind);
        let _ = value.setattr("unit", unit);
        let _ = value.setattr("function_code", function_code);
        let _ = value.setattr("found", found);
        let _ = value.setattr("exception", exception);
        let _ = value.setattr("received", received);
    });
    raised
}

/// A Modbus device: a unit address and the four tables it serves, coils, discrete inputs,
/// holding registers, and input registers, each holding only the addresses it was given.
///
/// `answer` takes one RTU frame and returns the frame the device sends back, or `None` when
/// it stays silent: the frame failed its CRC, is for another unit, or is a broadcast, whose
/// write the device still carries out.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct ModbusServer {
    inner: Arc<Mutex<Server>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ModbusServer {
    /// A device at a unit address, 1 to 247, with every table empty. Raises `PamojaError`
    /// for 0, the broadcast address, and for 248 to 255, which the specification reserves.
    #[new]
    fn new(unit: u8) -> PyResult<Self> {
        Server::new(unit)
            .map(|server| ModbusServer {
                inner: Arc::new(Mutex::new(server)),
            })
            .map_err(to_py)
    }

    /// The device's unit address.
    #[getter]
    fn unit(&self) -> u8 {
        lock(&self.inner).unit()
    }

    /// How many requests the device has carried out, broadcasts included and refusals not.
    #[getter]
    fn served(&self) -> usize {
        lock(&self.inner).served()
    }

    /// Sets coils from an address on, adding any the device did not have.
    fn set_coils(&self, start: u16, values: Vec<bool>) {
        lock(&self.inner).set_coils(start, &values);
    }

    /// Sets discrete inputs from an address on, adding any the device did not have.
    fn set_discrete_inputs(&self, start: u16, values: Vec<bool>) {
        lock(&self.inner).set_discrete_inputs(start, &values);
    }

    /// Sets holding registers from an address on, adding any the device did not have.
    fn set_holding_registers(&self, start: u16, values: Vec<u16>) {
        lock(&self.inner).set_holding_registers(start, &values);
    }

    /// Sets input registers from an address on, adding any the device did not have.
    fn set_input_registers(&self, start: u16, values: Vec<u16>) {
        lock(&self.inner).set_input_registers(start, &values);
    }

    /// A coil's state, or `None` when the device has no coil there.
    fn coil(&self, address: u16) -> Option<bool> {
        lock(&self.inner).coil(address)
    }

    /// A discrete input's state, or `None` when the device has no input there.
    fn discrete_input(&self, address: u16) -> Option<bool> {
        lock(&self.inner).discrete_input(address)
    }

    /// A holding register's value, or `None` when the device has no register there.
    fn holding_register(&self, address: u16) -> Option<u16> {
        lock(&self.inner).holding_register(address)
    }

    /// An input register's value, or `None` when the device has no register there.
    fn input_register(&self, address: u16) -> Option<u16> {
        lock(&self.inner).input_register(address)
    }

    /// Answers one RTU frame, as the device on the line does, or returns `None` when it
    /// stays silent.
    fn answer<'py>(&self, py: Python<'py>, frame: Vec<u8>) -> Option<Bound<'py, PyBytes>> {
        lock(&self.inner)
            .answer(&frame)
            .map(|reply| PyBytes::new(py, reply.as_bytes()))
    }
}

/// Several devices on one simulated line, as devices share an RS485 pair: every frame reaches
/// all of them, the one it is addressed to answers, and each carries out a broadcast write.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct ModbusLine {
    inner: Arc<Mutex<Line>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ModbusLine {
    /// A line with no devices on it.
    #[new]
    fn new() -> Self {
        ModbusLine {
            inner: Arc::new(Mutex::new(Line::new())),
        }
    }

    /// Puts a device on the line, which shares it.
    fn attach(&self, server: PyRef<'_, ModbusServer>) {
        lock(&self.inner).attach_shared(Arc::clone(&server.inner));
    }

    /// How many devices are on the line.
    fn __len__(&self) -> usize {
        lock(&self.inner).units().len()
    }

    /// A serial port with the line on its far end, at a speed, a parity name, and a stop bit
    /// count. Devices put on the line later are on the port too.
    fn port(&self, baud: u32, parity: &str, stop_bits: u8) -> PyResult<SerialPort> {
        let settings = crate::port::settings(baud, parity, stop_bits)?;
        Ok(SerialPort {
            inner: Port::simulated(settings, Arc::clone(&self.inner)),
        })
    }
}

/// A Modbus RTU client on a serial line: the gateway that sends each request and waits for its
/// reply, holding each transaction to the Modbus over Serial Line specification.
///
/// Each call releases the interpreter while the line is busy, and a failure raises
/// `ModbusClientError`.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct ModbusClient {
    inner: Arc<Mutex<Client>>,
}

impl ModbusClient {
    fn run<T: Send>(
        &self,
        py: Python<'_>,
        unit: u8,
        call: impl FnOnce(&mut Client) -> Result<T, ClientError> + Send,
    ) -> PyResult<T> {
        let inner = Arc::clone(&self.inner);
        py.detach(|| call(&mut lock(&inner)))
            .map_err(|error| client_error(unit, &error))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl ModbusClient {
    /// A client on a port, with a one-second response timeout and a 100 ms turnaround.
    #[new]
    fn new(port: PyRef<'_, SerialPort>) -> Self {
        ModbusClient {
            inner: Arc::new(Mutex::new(Client::new(port.inner.clone()))),
        }
    }

    /// The silence that separates two frames, in nanoseconds, for a speed, a parity name,
    /// and a stop bit count: 3.5 characters, and a fixed 1750 microseconds above 19200 baud.
    #[staticmethod]
    fn frame_gap_nanos(baud: u32, parity: &str, stop_bits: u8) -> PyResult<u64> {
        let settings = crate::port::settings(baud, parity, stop_bits)?;
        Ok(u64::try_from(Client::frame_gap(settings).as_nanos()).unwrap_or(u64::MAX))
    }

    /// How long the client waits for a whole reply, in microseconds.
    #[getter]
    fn response_timeout_micros(&self) -> u64 {
        u64::try_from(lock(&self.inner).response_timeout().as_micros()).unwrap_or(u64::MAX)
    }

    /// Sets how long the client waits for a whole reply, in microseconds.
    fn set_response_timeout_micros(&self, micros: u64) {
        lock(&self.inner).set_response_timeout(Duration::from_micros(micros));
    }

    /// How long the client leaves the line quiet after a broadcast, in microseconds.
    #[getter]
    fn turnaround_micros(&self) -> u64 {
        u64::try_from(lock(&self.inner).turnaround().as_micros()).unwrap_or(u64::MAX)
    }

    /// Sets how long the client leaves the line quiet after a broadcast, in microseconds.
    fn set_turnaround_micros(&self, micros: u64) {
        lock(&self.inner).set_turnaround(Duration::from_micros(micros));
    }

    /// Reads coils, function `0x01`.
    fn read_coils(
        &self,
        py: Python<'_>,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> PyResult<Vec<bool>> {
        self.run(py, unit, |client| client.read_coils(unit, start, quantity))
    }

    /// Reads discrete inputs, function `0x02`.
    fn read_discrete_inputs(
        &self,
        py: Python<'_>,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> PyResult<Vec<bool>> {
        self.run(py, unit, |client| {
            client.read_discrete_inputs(unit, start, quantity)
        })
    }

    /// Reads holding registers, function `0x03`.
    fn read_holding_registers(
        &self,
        py: Python<'_>,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> PyResult<Vec<u16>> {
        self.run(py, unit, |client| {
            client.read_holding_registers(unit, start, quantity)
        })
    }

    /// Reads input registers, function `0x04`.
    fn read_input_registers(
        &self,
        py: Python<'_>,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> PyResult<Vec<u16>> {
        self.run(py, unit, |client| {
            client.read_input_registers(unit, start, quantity)
        })
    }

    /// Writes one coil, function `0x05`; unit 0 broadcasts it to every device.
    fn write_single_coil(&self, py: Python<'_>, unit: u8, address: u16, on: bool) -> PyResult<()> {
        self.run(py, unit, |client| {
            client.write_single_coil(unit, address, on)
        })
    }

    /// Writes one holding register, function `0x06`; unit 0 broadcasts it to every device.
    fn write_single_register(
        &self,
        py: Python<'_>,
        unit: u8,
        address: u16,
        value: u16,
    ) -> PyResult<()> {
        self.run(py, unit, |client| {
            client.write_single_register(unit, address, value)
        })
    }

    /// Writes a run of coils, function `0x0F`; unit 0 broadcasts them to every device.
    fn write_multiple_coils(
        &self,
        py: Python<'_>,
        unit: u8,
        start: u16,
        values: Vec<bool>,
    ) -> PyResult<()> {
        self.run(py, unit, |client| {
            client.write_multiple_coils(unit, start, &values)
        })
    }

    /// Writes a run of holding registers, function `0x10`; unit 0 broadcasts them to every
    /// device.
    fn write_multiple_registers(
        &self,
        py: Python<'_>,
        unit: u8,
        start: u16,
        values: Vec<u16>,
    ) -> PyResult<()> {
        self.run(py, unit, |client| {
            client.write_multiple_registers(unit, start, &values)
        })
    }
}
