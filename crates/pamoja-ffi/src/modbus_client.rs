//! The C ABI for a Modbus client on a serial port, and the devices it polls.
//!
//! A client runs each transaction to the Modbus over Serial Line specification over a
//! [`PamojaSerialPort`]: the kernel's serial device on a Linux board, or a port whose far end
//! is a [`PamojaModbusLine`] of simulated devices. A device is a [`PamojaModbusServer`], a unit
//! address and the four tables it serves, which also answers a single frame on its own. The
//! program keeps its handle to each device after putting it on a line, so it can look at what
//! a write did.
//!
//! A failed transaction returns a status with the reason in the last error message, and fills
//! a [`PamojaModbusClientError`] when the caller passes one, so a program can tell a device's
//! exception from a timeout without reading the message.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use pamoja_hal::port::SerialPort;
use pamoja_modbus::{Client, ClientError, Line, Server};

use crate::modbus::{failed, out_slot, panicked, read_values};
use crate::port::{PamojaSerialPort, PamojaSerialSettings};
use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};

/// A client error kind: the call succeeded.
pub const PAMOJA_MODBUS_CLIENT_OK: u8 = 0;

/// A client error kind: the request could not be built, from a quantity outside what one
/// request carries.
pub const PAMOJA_MODBUS_CLIENT_REQUEST: u8 = 1;

/// A client error kind: a read was addressed to the broadcast address, which no device answers.
pub const PAMOJA_MODBUS_CLIENT_BROADCAST_READ: u8 = 2;

/// A client error kind: the serial port failed.
pub const PAMOJA_MODBUS_CLIENT_PORT: u8 = 3;

/// A client error kind: no complete reply arrived within the response timeout.
pub const PAMOJA_MODBUS_CLIENT_TIMEOUT: u8 = 4;

/// A client error kind: the reply failed its CRC or is not the shape its function gives.
pub const PAMOJA_MODBUS_CLIENT_FRAME: u8 = 5;

/// A client error kind: a reply came back from another unit.
pub const PAMOJA_MODBUS_CLIENT_WRONG_UNIT: u8 = 6;

/// A client error kind: a reply answered another function.
pub const PAMOJA_MODBUS_CLIENT_WRONG_FUNCTION: u8 = 7;

/// A client error kind: a well-formed reply that does not answer the request.
pub const PAMOJA_MODBUS_CLIENT_MISMATCH: u8 = 8;

/// A client error kind: the device refused the request with an exception.
pub const PAMOJA_MODBUS_CLIENT_EXCEPTION: u8 = 9;

/// Why a client call failed, for a program that branches on it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PamojaModbusClientError {
    /// One of the `PAMOJA_MODBUS_CLIENT_*` kinds, [`PAMOJA_MODBUS_CLIENT_OK`] on success.
    pub kind: u8,
    /// The unit the call asked.
    pub unit: u8,
    /// The function the call asked, for an exception or a reply to another function.
    pub function: u8,
    /// What the reply named instead: the unit that answered, or the function it answered.
    pub found: u8,
    /// The exception code the device answered with.
    pub exception: u8,
    /// How many bytes of a reply had arrived before a timeout.
    pub received: u32,
}

/// A Modbus device: a unit address and the four tables it serves. Opaque; release it with
/// [`pamoja_modbus_server_free`]. A line it is on keeps its own share of it.
pub struct PamojaModbusServer {
    server: Arc<Mutex<Server>>,
}

/// Several devices on one simulated line. Opaque; release it with
/// [`pamoja_modbus_line_free`]. A port made from it keeps its own share of it.
pub struct PamojaModbusLine {
    line: Arc<Mutex<Line>>,
}

/// A Modbus RTU client on a serial port. Opaque; release it with [`pamoja_modbus_client_free`].
pub struct PamojaModbusClient {
    client: Mutex<Client>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Makes a device at a unit address, with every table empty.
///
/// # Arguments
///
/// * `unit` - its address, 1 to 247.
/// * `out_server` - receives the device.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null `out_server` or a unit
/// outside 1 to 247, with the reason in the last error message.
///
/// # Safety
///
/// `out_server` must be a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_new(
    unit: u8,
    out_server: *mut *mut PamojaModbusServer,
) -> PamojaStatus {
    let out_server = match out_slot(out_server, "out_server") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    match Server::new(unit) {
        Ok(server) => {
            *out_server = Box::into_raw(Box::new(PamojaModbusServer {
                server: Arc::new(Mutex::new(server)),
            }));
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Runs a call on a device, turning a null handle or a panic into a status.
///
/// # Safety
///
/// `server` must be a live handle or null.
unsafe fn on_server(
    server: *const PamojaModbusServer,
    call: impl FnOnce(&mut Server),
) -> PamojaStatus {
    let Some(server) = server.as_ref() else {
        set_last_error("server must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match catch_unwind(AssertUnwindSafe(|| call(&mut lock(&server.server)))) {
        Ok(()) => PamojaStatus::Ok,
        Err(_) => panicked(),
    }
}

/// Sets coils from an address on, adding any the device did not have.
///
/// # Arguments
///
/// * `server` - the device.
/// * `start` - the first coil's address.
/// * `values` - one byte per coil, non-zero for on.
/// * `len` - how many coils.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `server` must be a live handle or null, and `values` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_set_coils(
    server: *const PamojaModbusServer,
    start: u16,
    values: *const u8,
    len: usize,
) -> PamojaStatus {
    match read_values(values, len, "values") {
        Ok(values) => {
            let values: Vec<bool> = values.into_iter().map(|value| value != 0).collect();
            on_server(server, |server| server.set_coils(start, &values))
        }
        Err(status) => status,
    }
}

/// Sets discrete inputs from an address on, adding any the device did not have.
///
/// # Arguments
///
/// * `server` - the device.
/// * `start` - the first input's address.
/// * `values` - one byte per input, non-zero for on.
/// * `len` - how many inputs.
///
/// # Returns
///
/// As [`pamoja_modbus_server_set_coils`].
///
/// # Safety
///
/// As [`pamoja_modbus_server_set_coils`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_set_discrete_inputs(
    server: *const PamojaModbusServer,
    start: u16,
    values: *const u8,
    len: usize,
) -> PamojaStatus {
    match read_values(values, len, "values") {
        Ok(values) => {
            let values: Vec<bool> = values.into_iter().map(|value| value != 0).collect();
            on_server(server, |server| server.set_discrete_inputs(start, &values))
        }
        Err(status) => status,
    }
}

/// Sets holding registers from an address on, adding any the device did not have.
///
/// # Arguments
///
/// * `server` - the device.
/// * `start` - the first register's address.
/// * `values` - the values, in address order.
/// * `len` - how many registers.
///
/// # Returns
///
/// As [`pamoja_modbus_server_set_coils`].
///
/// # Safety
///
/// `server` must be a live handle or null, and `values` must point to `len` readable values.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_set_holding_registers(
    server: *const PamojaModbusServer,
    start: u16,
    values: *const u16,
    len: usize,
) -> PamojaStatus {
    match read_values(values, len, "values") {
        Ok(values) => on_server(server, |server| {
            server.set_holding_registers(start, &values)
        }),
        Err(status) => status,
    }
}

/// Sets input registers from an address on, adding any the device did not have.
///
/// # Arguments
///
/// * `server` - the device.
/// * `start` - the first register's address.
/// * `values` - the values, in address order.
/// * `len` - how many registers.
///
/// # Returns
///
/// As [`pamoja_modbus_server_set_coils`].
///
/// # Safety
///
/// As [`pamoja_modbus_server_set_holding_registers`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_set_input_registers(
    server: *const PamojaModbusServer,
    start: u16,
    values: *const u16,
    len: usize,
) -> PamojaStatus {
    match read_values(values, len, "values") {
        Ok(values) => on_server(server, |server| server.set_input_registers(start, &values)),
        Err(status) => status,
    }
}

/// Reads one entry of a device's table, reporting whether the device has it.
///
/// # Safety
///
/// `server` must be a live handle or null, and `out` a writable pointer or null.
unsafe fn entry<T>(
    server: *const PamojaModbusServer,
    out: *mut T,
    read: impl FnOnce(&Server) -> Option<T>,
) -> bool {
    let (Some(server), Some(out)) = (server.as_ref(), out.as_mut()) else {
        return false;
    };
    match read(&lock(&server.server)) {
        Some(value) => {
            *out = value;
            true
        }
        None => false,
    }
}

/// Reads a coil.
///
/// # Arguments
///
/// * `server` - the device.
/// * `address` - the coil's address.
/// * `out_on` - receives its state.
///
/// # Returns
///
/// `true` with the state in `out_on` when the device has the coil; `false` when it does not,
/// or for a null argument.
///
/// # Safety
///
/// `server` must be a live handle or null, and `out_on` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_coil(
    server: *const PamojaModbusServer,
    address: u16,
    out_on: *mut bool,
) -> bool {
    entry(server, out_on, |server| server.coil(address))
}

/// Reads a discrete input.
///
/// # Arguments
///
/// * `server` - the device.
/// * `address` - the input's address.
/// * `out_on` - receives its state.
///
/// # Returns
///
/// As [`pamoja_modbus_server_coil`].
///
/// # Safety
///
/// As [`pamoja_modbus_server_coil`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_discrete_input(
    server: *const PamojaModbusServer,
    address: u16,
    out_on: *mut bool,
) -> bool {
    entry(server, out_on, |server| server.discrete_input(address))
}

/// Reads a holding register.
///
/// # Arguments
///
/// * `server` - the device.
/// * `address` - the register's address.
/// * `out_value` - receives its value.
///
/// # Returns
///
/// `true` with the value in `out_value` when the device has the register; `false` when it
/// does not, or for a null argument.
///
/// # Safety
///
/// `server` must be a live handle or null, and `out_value` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_holding_register(
    server: *const PamojaModbusServer,
    address: u16,
    out_value: *mut u16,
) -> bool {
    entry(server, out_value, |server| server.holding_register(address))
}

/// Reads an input register.
///
/// # Arguments
///
/// * `server` - the device.
/// * `address` - the register's address.
/// * `out_value` - receives its value.
///
/// # Returns
///
/// As [`pamoja_modbus_server_holding_register`].
///
/// # Safety
///
/// As [`pamoja_modbus_server_holding_register`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_input_register(
    server: *const PamojaModbusServer,
    address: u16,
    out_value: *mut u16,
) -> bool {
    entry(server, out_value, |server| server.input_register(address))
}

/// Returns a device's unit address, or 0 for a null handle.
///
/// # Safety
///
/// `server` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_unit(server: *const PamojaModbusServer) -> u8 {
    server
        .as_ref()
        .map_or(0, |server| lock(&server.server).unit())
}

/// Returns how many requests a device has carried out, broadcasts included and refusals not,
/// or 0 for a null handle.
///
/// # Safety
///
/// `server` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_served(server: *const PamojaModbusServer) -> usize {
    server
        .as_ref()
        .map_or(0, |server| lock(&server.server).served())
}

/// Answers one RTU frame, as the device on the line does.
///
/// # Arguments
///
/// * `server` - the device.
/// * `frame` - the frame as it came off the line, CRC included.
/// * `len` - its length.
/// * `out_buffer` - receives the frame to send back, or null when the device stays silent: the
///   frame failed its CRC, is for another unit, or is a broadcast, whose write the device still
///   carries out.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `server` must be a live handle or null, `frame` must point to `len` readable bytes, and
/// `out_buffer` must be a writable pointer or null. A returned buffer is the caller's to
/// release with [`pamoja_buffer_free`](crate::pamoja_buffer_free).
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_answer(
    server: *const PamojaModbusServer,
    frame: *const u8,
    len: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    let frame = match read_bytes(frame, len) {
        Ok(frame) => frame,
        Err(status) => return status,
    };
    on_server(server, |server| {
        if let Some(reply) = server.answer(&frame) {
            *out_buffer = PamojaBuffer::into_raw(reply.as_bytes().to_vec());
        }
    })
}

/// Releases the caller's handle to a device. A line it is on keeps its own share. A null
/// pointer is ignored.
///
/// # Safety
///
/// `server` must be a handle that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_server_free(server: *mut PamojaModbusServer) {
    if !server.is_null() {
        drop(Box::from_raw(server));
    }
}

/// Makes a line with no devices on it.
///
/// # Returns
///
/// The line, which the caller releases with [`pamoja_modbus_line_free`].
#[no_mangle]
pub extern "C" fn pamoja_modbus_line_new() -> *mut PamojaModbusLine {
    Box::into_raw(Box::new(PamojaModbusLine {
        line: Arc::new(Mutex::new(Line::new())),
    }))
}

/// Puts a device on a line. The line shares the device, so the caller's handle still reads
/// and changes it.
///
/// # Arguments
///
/// * `line` - the line.
/// * `server` - the device.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `line` and `server` must be live handles or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_line_attach(
    line: *const PamojaModbusLine,
    server: *const PamojaModbusServer,
) -> PamojaStatus {
    let (Some(line), Some(server)) = (line.as_ref(), server.as_ref()) else {
        set_last_error("line and server must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    lock(&line.line).attach_shared(Arc::clone(&server.server));
    PamojaStatus::Ok
}

/// Returns how many devices are on a line, or 0 for a null handle.
///
/// # Safety
///
/// `line` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_line_len(line: *const PamojaModbusLine) -> usize {
    line.as_ref()
        .map_or(0, |line| lock(&line.line).units().len())
}

/// Makes a serial port with a line on its far end: every frame written reaches each device,
/// and whatever they answer waits to be read.
///
/// # Arguments
///
/// * `line` - the line; devices put on it later are on the port too.
/// * `settings` - the speed and character format the line runs at.
/// * `out_port` - receives the port, which the caller releases with
///   [`pamoja_serial_port_free`](crate::port::pamoja_serial_port_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument or settings
/// the port does not have.
///
/// # Safety
///
/// `line` must be a live handle or null, and `out_port` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_line_port(
    line: *const PamojaModbusLine,
    settings: PamojaSerialSettings,
    out_port: *mut *mut PamojaSerialPort,
) -> PamojaStatus {
    let out_port = match out_slot(out_port, "out_port") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    let Some(line) = line.as_ref() else {
        set_last_error("line must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let settings = match settings.settings() {
        Ok(settings) => settings,
        Err(status) => return status,
    };
    let port = SerialPort::simulated(settings, Arc::clone(&line.line));
    *out_port = Box::into_raw(Box::new(PamojaSerialPort { port }));
    PamojaStatus::Ok
}

/// Releases the caller's handle to a line. A port made from it keeps its own share. A null
/// pointer is ignored.
///
/// # Safety
///
/// `line` must be a handle that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_line_free(line: *mut PamojaModbusLine) {
    if !line.is_null() {
        drop(Box::from_raw(line));
    }
}

/// Returns the silence that separates two frames at a line's speed and format: 3.5
/// characters, and a fixed 1750 microseconds above 19200 baud.
///
/// # Arguments
///
/// * `settings` - the line's speed and character format.
/// * `out_nanos` - receives the silence, in nanoseconds.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null `out_nanos` or
/// settings the port does not have.
///
/// # Safety
///
/// `out_nanos` must be a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_frame_gap_nanos(
    settings: PamojaSerialSettings,
    out_nanos: *mut u64,
) -> PamojaStatus {
    let Some(out_nanos) = out_nanos.as_mut() else {
        set_last_error("out_nanos must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match settings.settings() {
        Ok(settings) => {
            *out_nanos = u64::try_from(Client::frame_gap(settings).as_nanos()).unwrap_or(u64::MAX);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Makes a client on a port, with a one-second response timeout and a 100 ms turnaround.
///
/// # Arguments
///
/// * `port` - the line; the client holds its own share of it.
/// * `out_client` - receives the client.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `port` must be a live handle or null, and `out_client` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_new(
    port: *const PamojaSerialPort,
    out_client: *mut *mut PamojaModbusClient,
) -> PamojaStatus {
    let out_client = match out_slot(out_client, "out_client") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    let Some(port) = port.as_ref() else {
        set_last_error("port must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_client = Box::into_raw(Box::new(PamojaModbusClient {
        client: Mutex::new(Client::new(port.port.clone())),
    }));
    PamojaStatus::Ok
}

/// Sets how long a client waits for a whole reply once a request has gone out.
///
/// # Arguments
///
/// * `client` - the client.
/// * `micros` - the response timeout.
///
/// # Safety
///
/// `client` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_set_response_timeout(
    client: *const PamojaModbusClient,
    micros: u64,
) {
    if let Some(client) = client.as_ref() {
        lock(&client.client).set_response_timeout(Duration::from_micros(micros));
    }
}

/// Sets how long a client leaves the line quiet after a broadcast.
///
/// # Arguments
///
/// * `client` - the client.
/// * `micros` - the turnaround delay.
///
/// # Safety
///
/// `client` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_set_turnaround(
    client: *const PamojaModbusClient,
    micros: u64,
) {
    if let Some(client) = client.as_ref() {
        lock(&client.client).set_turnaround(Duration::from_micros(micros));
    }
}

/// Returns a client's response timeout, in microseconds, or 0 for a null handle.
///
/// # Safety
///
/// `client` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_response_timeout_micros(
    client: *const PamojaModbusClient,
) -> u64 {
    client
        .as_ref()
        .map_or(0, |client| micros(lock(&client.client).response_timeout()))
}

/// Returns a client's turnaround delay, in microseconds, or 0 for a null handle.
///
/// # Safety
///
/// `client` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_turnaround_micros(
    client: *const PamojaModbusClient,
) -> u64 {
    client
        .as_ref()
        .map_or(0, |client| micros(lock(&client.client).turnaround()))
}

fn micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

/// Runs a transaction, filling the caller's error and the last error message when it fails.
///
/// # Safety
///
/// `client` must be a live handle or null, and `out_error` a writable pointer or null.
unsafe fn transact<T>(
    client: *const PamojaModbusClient,
    unit: u8,
    out_error: *mut PamojaModbusClientError,
    call: impl FnOnce(&mut Client) -> Result<T, ClientError>,
) -> Result<T, PamojaStatus> {
    let mut report = PamojaModbusClientError {
        unit,
        ..PamojaModbusClientError::default()
    };
    let Some(client) = client.as_ref() else {
        set_last_error("client must not be null".to_owned());
        return Err(PamojaStatus::InvalidArgument);
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| call(&mut lock(&client.client))));
    let result = match outcome {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => {
            set_last_error(error.to_string());
            let status = match error {
                ClientError::Request(_) | ClientError::BroadcastRead => {
                    PamojaStatus::InvalidArgument
                }
                ClientError::Port(_) | ClientError::Timeout { .. } => PamojaStatus::Io,
                ClientError::Exception { .. } => PamojaStatus::Other,
                _ => PamojaStatus::Codec,
            };
            describe(&error, &mut report);
            Err(status)
        }
        Err(_) => Err(panicked()),
    };
    if let Some(out_error) = out_error.as_mut() {
        *out_error = report;
    }
    result
}

fn describe(error: &ClientError, report: &mut PamojaModbusClientError) {
    report.kind = match *error {
        ClientError::Request(_) => PAMOJA_MODBUS_CLIENT_REQUEST,
        ClientError::BroadcastRead => PAMOJA_MODBUS_CLIENT_BROADCAST_READ,
        ClientError::Port(_) => PAMOJA_MODBUS_CLIENT_PORT,
        ClientError::Timeout { received, .. } => {
            report.received = u32::try_from(received).unwrap_or(u32::MAX);
            PAMOJA_MODBUS_CLIENT_TIMEOUT
        }
        ClientError::Frame(_) => PAMOJA_MODBUS_CLIENT_FRAME,
        ClientError::WrongUnit { found, .. } => {
            report.found = found;
            PAMOJA_MODBUS_CLIENT_WRONG_UNIT
        }
        ClientError::WrongFunction { expected, found } => {
            report.function = expected;
            report.found = found;
            PAMOJA_MODBUS_CLIENT_WRONG_FUNCTION
        }
        ClientError::Mismatch { .. } => PAMOJA_MODBUS_CLIENT_MISMATCH,
        ClientError::Exception {
            function,
            exception,
            ..
        } => {
            report.function = function;
            report.exception = exception.code();
            PAMOJA_MODBUS_CLIENT_EXCEPTION
        }
    };
}

/// Copies what a read returned into the caller's array, which holds `quantity` values.
///
/// # Safety
///
/// `out` must point to `quantity` writable values, or be null when `quantity` is 0.
unsafe fn read_into<T: Copy, U>(
    client: *const PamojaModbusClient,
    unit: u8,
    quantity: u16,
    out: *mut U,
    out_error: *mut PamojaModbusClientError,
    convert: impl Fn(T) -> U,
    call: impl FnOnce(&mut Client) -> Result<Vec<T>, ClientError>,
) -> PamojaStatus {
    if out.is_null() && quantity > 0 {
        set_last_error("out_values must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match transact(client, unit, out_error, call) {
        Ok(values) => {
            let out = std::slice::from_raw_parts_mut(out, values.len().min(usize::from(quantity)));
            for (slot, value) in out.iter_mut().zip(values) {
                *slot = convert(value);
            }
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Reads coils, function `0x01`.
///
/// # Arguments
///
/// * `client` - the client.
/// * `unit` - the device, 1 to 247.
/// * `start` - the first coil's address.
/// * `quantity` - how many, 1 to 2000.
/// * `out_values` - receives one byte per coil, 1 for on, in address order.
/// * `out_error` - receives why the call failed, or null.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a request that cannot be sent;
/// [`PamojaStatus::Io`] for a port failure or a timeout; [`PamojaStatus::Codec`] for a reply
/// that fails its checks; or [`PamojaStatus::Other`] for the device's exception. The reason
/// is in the last error message and, in detail, in `out_error`.
///
/// # Safety
///
/// `client` must be a live handle or null, `out_values` must point to `quantity` writable
/// bytes, and `out_error` must be a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_read_coils(
    client: *const PamojaModbusClient,
    unit: u8,
    start: u16,
    quantity: u16,
    out_values: *mut u8,
    out_error: *mut PamojaModbusClientError,
) -> PamojaStatus {
    read_into(
        client,
        unit,
        quantity,
        out_values,
        out_error,
        u8::from,
        |client| client.read_coils(unit, start, quantity),
    )
}

/// Reads discrete inputs, function `0x02`.
///
/// # Arguments
///
/// As [`pamoja_modbus_client_read_coils`], for inputs.
///
/// # Returns
///
/// As [`pamoja_modbus_client_read_coils`].
///
/// # Safety
///
/// As [`pamoja_modbus_client_read_coils`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_read_discrete_inputs(
    client: *const PamojaModbusClient,
    unit: u8,
    start: u16,
    quantity: u16,
    out_values: *mut u8,
    out_error: *mut PamojaModbusClientError,
) -> PamojaStatus {
    read_into(
        client,
        unit,
        quantity,
        out_values,
        out_error,
        u8::from,
        |client| client.read_discrete_inputs(unit, start, quantity),
    )
}

/// Reads holding registers, function `0x03`.
///
/// # Arguments
///
/// * `client` - the client.
/// * `unit` - the device, 1 to 247.
/// * `start` - the first register's address.
/// * `quantity` - how many, 1 to 125.
/// * `out_values` - receives the values, in address order.
/// * `out_error` - receives why the call failed, or null.
///
/// # Returns
///
/// As [`pamoja_modbus_client_read_coils`].
///
/// # Safety
///
/// `client` must be a live handle or null, `out_values` must point to `quantity` writable
/// values, and `out_error` must be a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_read_holding_registers(
    client: *const PamojaModbusClient,
    unit: u8,
    start: u16,
    quantity: u16,
    out_values: *mut u16,
    out_error: *mut PamojaModbusClientError,
) -> PamojaStatus {
    read_into(
        client,
        unit,
        quantity,
        out_values,
        out_error,
        |v| v,
        |client| client.read_holding_registers(unit, start, quantity),
    )
}

/// Reads input registers, function `0x04`.
///
/// # Arguments
///
/// As [`pamoja_modbus_client_read_holding_registers`], for input registers.
///
/// # Returns
///
/// As [`pamoja_modbus_client_read_coils`].
///
/// # Safety
///
/// As [`pamoja_modbus_client_read_holding_registers`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_read_input_registers(
    client: *const PamojaModbusClient,
    unit: u8,
    start: u16,
    quantity: u16,
    out_values: *mut u16,
    out_error: *mut PamojaModbusClientError,
) -> PamojaStatus {
    read_into(
        client,
        unit,
        quantity,
        out_values,
        out_error,
        |v| v,
        |client| client.read_input_registers(unit, start, quantity),
    )
}

/// Writes one coil, function `0x05`.
///
/// # Arguments
///
/// * `client` - the client.
/// * `unit` - the device, 1 to 247, or 0 to broadcast to every device.
/// * `address` - the coil's address.
/// * `on` - the state to write.
/// * `out_error` - receives why the call failed, or null.
///
/// # Returns
///
/// As [`pamoja_modbus_client_read_coils`].
///
/// # Safety
///
/// `client` must be a live handle or null, and `out_error` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_write_single_coil(
    client: *const PamojaModbusClient,
    unit: u8,
    address: u16,
    on: bool,
    out_error: *mut PamojaModbusClientError,
) -> PamojaStatus {
    status(transact(client, unit, out_error, |client| {
        client.write_single_coil(unit, address, on)
    }))
}

/// Writes one holding register, function `0x06`.
///
/// # Arguments
///
/// * `client` - the client.
/// * `unit` - the device, 1 to 247, or 0 to broadcast to every device.
/// * `address` - the register's address.
/// * `value` - the value to write.
/// * `out_error` - receives why the call failed, or null.
///
/// # Returns
///
/// As [`pamoja_modbus_client_read_coils`].
///
/// # Safety
///
/// As [`pamoja_modbus_client_write_single_coil`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_write_single_register(
    client: *const PamojaModbusClient,
    unit: u8,
    address: u16,
    value: u16,
    out_error: *mut PamojaModbusClientError,
) -> PamojaStatus {
    status(transact(client, unit, out_error, |client| {
        client.write_single_register(unit, address, value)
    }))
}

/// Writes a run of coils, function `0x0F`.
///
/// # Arguments
///
/// * `client` - the client.
/// * `unit` - the device, 1 to 247, or 0 to broadcast to every device.
/// * `start` - the first coil's address.
/// * `values` - one byte per coil, non-zero for on, 1 to 1968 of them.
/// * `len` - how many coils.
/// * `out_error` - receives why the call failed, or null.
///
/// # Returns
///
/// As [`pamoja_modbus_client_read_coils`].
///
/// # Safety
///
/// `client` must be a live handle or null, `values` must point to `len` readable bytes, and
/// `out_error` must be a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_write_multiple_coils(
    client: *const PamojaModbusClient,
    unit: u8,
    start: u16,
    values: *const u8,
    len: usize,
    out_error: *mut PamojaModbusClientError,
) -> PamojaStatus {
    let values: Vec<bool> = match read_values(values, len, "values") {
        Ok(values) => values.into_iter().map(|value| value != 0).collect(),
        Err(status) => return status,
    };
    status(transact(client, unit, out_error, |client| {
        client.write_multiple_coils(unit, start, &values)
    }))
}

/// Writes a run of holding registers, function `0x10`.
///
/// # Arguments
///
/// * `client` - the client.
/// * `unit` - the device, 1 to 247, or 0 to broadcast to every device.
/// * `start` - the first register's address.
/// * `values` - the values, 1 to 123 of them, in address order.
/// * `len` - how many registers.
/// * `out_error` - receives why the call failed, or null.
///
/// # Returns
///
/// As [`pamoja_modbus_client_read_coils`].
///
/// # Safety
///
/// `client` must be a live handle or null, `values` must point to `len` readable values, and
/// `out_error` must be a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_write_multiple_registers(
    client: *const PamojaModbusClient,
    unit: u8,
    start: u16,
    values: *const u16,
    len: usize,
    out_error: *mut PamojaModbusClientError,
) -> PamojaStatus {
    let values = match read_values(values, len, "values") {
        Ok(values) => values,
        Err(status) => return status,
    };
    status(transact(client, unit, out_error, |client| {
        client.write_multiple_registers(unit, start, &values)
    }))
}

fn status(result: Result<(), PamojaStatus>) -> PamojaStatus {
    result.err().unwrap_or(PamojaStatus::Ok)
}

/// Releases a client. Its port stays open while another holder has it. A null pointer is
/// ignored.
///
/// # Safety
///
/// `client` must be a handle that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_client_free(client: *mut PamojaModbusClient) {
    if !client.is_null() {
        drop(Box::from_raw(client));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{
        pamoja_serial_port_free, pamoja_serial_port_waited_micros, PAMOJA_PARITY_EVEN,
    };
    use std::ptr;

    fn line_settings() -> PamojaSerialSettings {
        PamojaSerialSettings {
            baud: 19_200,
            parity: PAMOJA_PARITY_EVEN,
            stop_bits: 1,
        }
    }

    #[test]
    fn a_client_polls_devices_on_a_simulated_line_through_the_c_abi() {
        unsafe {
            let mut meter = ptr::null_mut();
            assert_eq!(pamoja_modbus_server_new(17, &mut meter), PamojaStatus::Ok);
            let registers = [2301u16, 418, 0];
            assert_eq!(
                pamoja_modbus_server_set_holding_registers(meter, 107, registers.as_ptr(), 3),
                PamojaStatus::Ok
            );
            let line = pamoja_modbus_line_new();
            assert_eq!(pamoja_modbus_line_attach(line, meter), PamojaStatus::Ok);
            assert_eq!(pamoja_modbus_line_len(line), 1);
            let mut port = ptr::null_mut();
            assert_eq!(
                pamoja_modbus_line_port(line, line_settings(), &mut port),
                PamojaStatus::Ok
            );
            let mut client = ptr::null_mut();
            assert_eq!(
                pamoja_modbus_client_new(port, &mut client),
                PamojaStatus::Ok
            );

            let mut values = [0u16; 3];
            let mut error = PamojaModbusClientError::default();
            assert_eq!(
                pamoja_modbus_client_read_holding_registers(
                    client,
                    17,
                    107,
                    3,
                    values.as_mut_ptr(),
                    &mut error
                ),
                PamojaStatus::Ok
            );
            assert_eq!(values, [2301, 418, 0]);
            assert_eq!(error.kind, PAMOJA_MODBUS_CLIENT_OK);

            assert_eq!(
                pamoja_modbus_client_write_single_register(client, 17, 109, 5, &mut error),
                PamojaStatus::Ok
            );
            let mut value = 0u16;
            assert!(pamoja_modbus_server_holding_register(
                meter, 109, &mut value
            ));
            assert_eq!(value, 5);
            assert!(!pamoja_modbus_server_holding_register(
                meter, 110, &mut value
            ));

            assert_eq!(
                pamoja_modbus_client_read_holding_registers(
                    client,
                    17,
                    108,
                    3,
                    values.as_mut_ptr(),
                    &mut error
                ),
                PamojaStatus::Other
            );
            assert_eq!(error.kind, PAMOJA_MODBUS_CLIENT_EXCEPTION);
            assert_eq!(
                (error.unit, error.function, error.exception),
                (17, 0x03, 0x02)
            );

            pamoja_modbus_client_set_response_timeout(client, 250_000);
            assert_eq!(
                pamoja_modbus_client_response_timeout_micros(client),
                250_000
            );
            let before = pamoja_serial_port_waited_micros(port);
            assert_eq!(
                pamoja_modbus_client_read_holding_registers(
                    client,
                    18,
                    107,
                    1,
                    values.as_mut_ptr(),
                    &mut error
                ),
                PamojaStatus::Io
            );
            assert_eq!(error.kind, PAMOJA_MODBUS_CLIENT_TIMEOUT);
            assert_eq!(
                pamoja_serial_port_waited_micros(port) - before,
                2_005 + 250_000
            );
            assert_eq!(pamoja_modbus_server_served(meter), 2);

            pamoja_modbus_client_free(client);
            pamoja_serial_port_free(port);
            pamoja_modbus_line_free(line);
            pamoja_modbus_server_free(meter);
        }
    }

    #[test]
    fn a_device_answers_a_frame_on_its_own_and_stays_silent_for_another_unit() {
        unsafe {
            let mut meter = ptr::null_mut();
            assert_eq!(pamoja_modbus_server_new(17, &mut meter), PamojaStatus::Ok);
            let coils = [1u8, 0, 1];
            assert_eq!(
                pamoja_modbus_server_set_coils(meter, 0, coils.as_ptr(), 3),
                PamojaStatus::Ok
            );
            let request = pamoja_modbus::Pdu::read_coils(0, 3).to_adu(17);
            let mut reply = ptr::null_mut();
            assert_eq!(
                pamoja_modbus_server_answer(
                    meter,
                    request.as_bytes().as_ptr(),
                    request.as_bytes().len(),
                    &mut reply
                ),
                PamojaStatus::Ok
            );
            assert!(!reply.is_null());
            crate::pamoja_buffer_free(reply);

            let other = pamoja_modbus::Pdu::read_coils(0, 3).to_adu(18);
            assert_eq!(
                pamoja_modbus_server_answer(
                    meter,
                    other.as_bytes().as_ptr(),
                    other.as_bytes().len(),
                    &mut reply
                ),
                PamojaStatus::Ok
            );
            assert!(reply.is_null());
            let mut on = false;
            assert!(pamoja_modbus_server_coil(meter, 2, &mut on));
            assert!(on);

            let mut refused = ptr::null_mut();
            assert_eq!(
                pamoja_modbus_server_new(248, &mut refused),
                PamojaStatus::InvalidArgument
            );
            assert!(refused.is_null());
            pamoja_modbus_server_free(meter);
        }
    }

    #[test]
    fn the_frame_gap_crosses_in_nanoseconds() {
        let mut nanos = 0;
        unsafe {
            assert_eq!(
                pamoja_modbus_frame_gap_nanos(line_settings(), &mut nanos),
                PamojaStatus::Ok
            );
        }
        assert_eq!(nanos, 2_005_210);
    }
}
