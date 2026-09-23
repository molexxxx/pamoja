//! Generated Node bindings for Modbus RTU framing.
//!
//! These mirror the `pamoja-modbus` Rust API. Each request builder returns a
//! complete RTU frame, CRC included, ready to write to an RS485 port: a caller
//! reaching the SDK from JavaScript has no use for a bare PDU, so the PDU and the
//! frame around it are one step here.
//!
//! A received frame goes back through `parseFrame`, which validates the CRC
//! before anything can be read from it, and the values a device returned come out
//! as numbers and booleans rather than bytes to unpack.
//!
//! A `ModbusClient` runs whole transactions over a `SerialPort` on a worker thread,
//! and a `ModbusServer` is a device a `ModbusLine` puts on the far end of a simulated
//! port. A transaction resolves with an outcome the facade turns into values or a thrown
//! `ModbusClientError`, so a program can tell a device's exception from a timeout.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use napi::bindgen_prelude::{spawn_blocking, Buffer};
use napi_derive::napi;
use pamoja_hal::port::SerialPort as Port;
use pamoja_modbus::{crc16, Adu, Client, ClientError, Line, ModbusError, Pdu, Response, Server};

use crate::port::{SerialPort, SerialSettings};

/// A received Modbus RTU frame whose CRC has been verified.
#[napi(object)]
pub struct ModbusFrame {
    /// The unit (slave) address the frame is addressed to or came from.
    pub address: u8,
    /// The function code. An exception response carries the request's code with
    /// its high bit set, as it appeared on the wire.
    pub function_code: u8,
    /// The exception code a device reported, or `null` when the frame is not an
    /// exception response.
    pub exception: Option<u8>,
    /// The protocol data unit: the function code and its data, without the
    /// address or the CRC.
    pub pdu: Buffer,
}

/// Computes the CRC-16/MODBUS that every RTU frame ends with.
#[napi]
pub fn modbus_crc16(bytes: Buffer) -> u16 {
    crc16(bytes.as_ref())
}

/// Builds a read-coils request frame (function `0x01`).
#[napi]
pub fn modbus_read_coils(address: u8, start: u16, count: u16) -> Buffer {
    Pdu::read_coils(start, count)
        .to_adu(address)
        .as_bytes()
        .into()
}

/// Builds a read-discrete-inputs request frame (function `0x02`).
#[napi]
pub fn modbus_read_discrete_inputs(address: u8, start: u16, count: u16) -> Buffer {
    Pdu::read_discrete_inputs(start, count)
        .to_adu(address)
        .as_bytes()
        .into()
}

/// Builds a read-holding-registers request frame (function `0x03`).
#[napi]
pub fn modbus_read_holding_registers(address: u8, start: u16, count: u16) -> Buffer {
    Pdu::read_holding_registers(start, count)
        .to_adu(address)
        .as_bytes()
        .into()
}

/// Builds the reply a device sends to a read-holding-registers request.
#[napi]
pub fn modbus_read_holding_registers_reply(address: u8, values: Vec<u16>) -> napi::Result<Buffer> {
    let pdu = Pdu::read_holding_registers_reply(&values).map_err(to_napi)?;
    Ok(pdu.to_adu(address).as_bytes().into())
}

/// Builds the reply a device sends to a read-input-registers request.
#[napi]
pub fn modbus_read_input_registers_reply(address: u8, values: Vec<u16>) -> napi::Result<Buffer> {
    let pdu = Pdu::read_input_registers_reply(&values).map_err(to_napi)?;
    Ok(pdu.to_adu(address).as_bytes().into())
}

/// Builds a read-input-registers request frame (function `0x04`).
#[napi]
pub fn modbus_read_input_registers(address: u8, start: u16, count: u16) -> Buffer {
    Pdu::read_input_registers(start, count)
        .to_adu(address)
        .as_bytes()
        .into()
}

/// Builds a write-single-coil request frame (function `0x05`).
#[napi]
pub fn modbus_write_single_coil(address: u8, coil: u16, on: bool) -> Buffer {
    Pdu::write_single_coil(coil, on)
        .to_adu(address)
        .as_bytes()
        .into()
}

/// Builds a write-single-register request frame (function `0x06`).
#[napi]
pub fn modbus_write_single_register(address: u8, register: u16, value: u16) -> Buffer {
    Pdu::write_single_register(register, value)
        .to_adu(address)
        .as_bytes()
        .into()
}

/// Builds a write-multiple-registers request frame (function `0x10`).
#[napi]
pub fn modbus_write_multiple_registers(
    address: u8,
    start: u16,
    values: Vec<u16>,
) -> napi::Result<Buffer> {
    Pdu::write_multiple_registers(start, &values)
        .map(|pdu| pdu.to_adu(address).as_bytes().into())
        .map_err(to_napi)
}

/// Builds a write-multiple-coils request frame (function `0x0F`).
#[napi]
pub fn modbus_write_multiple_coils(
    address: u8,
    start: u16,
    values: Vec<bool>,
) -> napi::Result<Buffer> {
    Pdu::write_multiple_coils(start, &values)
        .map(|pdu| pdu.to_adu(address).as_bytes().into())
        .map_err(to_napi)
}

/// Builds a request frame from a raw function code and data.
///
/// This is the escape hatch for the function codes the SDK does not name.
#[napi]
pub fn modbus_raw(address: u8, function_code: u8, data: Buffer) -> napi::Result<Buffer> {
    Pdu::raw(function_code, data.as_ref())
        .map(|pdu| pdu.to_adu(address).as_bytes().into())
        .map_err(to_napi)
}

/// Parses a received RTU frame, verifying its CRC.
#[napi]
pub fn modbus_parse_frame(bytes: Buffer) -> napi::Result<ModbusFrame> {
    let adu = Adu::parse(bytes.as_ref()).map_err(to_napi)?;
    Ok(ModbusFrame {
        address: adu.address(),
        function_code: adu.function_code(),
        exception: adu.exception().map(pamoja_modbus::Exception::code),
        pdu: adu.pdu().into(),
    })
}

/// Reads the 16-bit registers out of a read-registers response PDU.
#[napi]
pub fn modbus_registers(pdu: Buffer) -> napi::Result<Vec<u16>> {
    Response::new(pdu.as_ref())
        .registers()
        .map(Iterator::collect)
        .map_err(to_napi)
}

/// Reads `count` coils or discrete inputs out of a read-bits response PDU.
#[napi]
pub fn modbus_coils(pdu: Buffer, count: u16) -> napi::Result<Vec<bool>> {
    Response::new(pdu.as_ref())
        .coils(count)
        .map(Iterator::collect)
        .map_err(to_napi)
}

/// Maps a Modbus error onto a thrown exception.
fn to_napi(error: ModbusError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A Modbus device: a unit address and the four tables it serves, coils, discrete inputs,
/// holding registers, and input registers, each holding only the addresses it was given.
///
/// `answer` takes one RTU frame and returns the frame the device sends back, or `null` when it
/// stays silent: the frame failed its CRC, is for another unit, or is a broadcast, whose write
/// the device still carries out. A request it cannot serve is answered with the exception the
/// specification gives.
#[napi(js_name = "ModbusServer")]
pub struct ModbusServer {
    inner: Arc<Mutex<Server>>,
}

#[napi]
impl ModbusServer {
    /// A device at a unit address, 1 to 247, with every table empty. Throws for 0, the
    /// broadcast address, and for 248 to 255, which the specification reserves.
    #[napi(constructor)]
    pub fn new(unit: u8) -> napi::Result<Self> {
        Server::new(unit)
            .map(|server| ModbusServer {
                inner: Arc::new(Mutex::new(server)),
            })
            .map_err(to_napi)
    }

    /// The device's unit address.
    #[napi(getter)]
    pub fn unit(&self) -> u8 {
        lock(&self.inner).unit()
    }

    /// How many requests the device has carried out, broadcasts included and refusals not.
    #[napi(getter)]
    pub fn served(&self) -> u32 {
        u32::try_from(lock(&self.inner).served()).unwrap_or(u32::MAX)
    }

    /// Sets coils from an address on, adding any the device did not have.
    #[napi]
    pub fn set_coils(&self, start: u16, values: Vec<bool>) {
        lock(&self.inner).set_coils(start, &values);
    }

    /// Sets discrete inputs from an address on, adding any the device did not have.
    #[napi]
    pub fn set_discrete_inputs(&self, start: u16, values: Vec<bool>) {
        lock(&self.inner).set_discrete_inputs(start, &values);
    }

    /// Sets holding registers from an address on, adding any the device did not have.
    #[napi]
    pub fn set_holding_registers(&self, start: u16, values: Vec<u16>) {
        lock(&self.inner).set_holding_registers(start, &values);
    }

    /// Sets input registers from an address on, adding any the device did not have.
    #[napi]
    pub fn set_input_registers(&self, start: u16, values: Vec<u16>) {
        lock(&self.inner).set_input_registers(start, &values);
    }

    /// A coil's state, or `null` when the device has no coil there.
    #[napi]
    pub fn coil(&self, address: u16) -> Option<bool> {
        lock(&self.inner).coil(address)
    }

    /// A discrete input's state, or `null` when the device has no input there.
    #[napi]
    pub fn discrete_input(&self, address: u16) -> Option<bool> {
        lock(&self.inner).discrete_input(address)
    }

    /// A holding register's value, or `null` when the device has no register there.
    #[napi]
    pub fn holding_register(&self, address: u16) -> Option<u16> {
        lock(&self.inner).holding_register(address)
    }

    /// An input register's value, or `null` when the device has no register there.
    #[napi]
    pub fn input_register(&self, address: u16) -> Option<u16> {
        lock(&self.inner).input_register(address)
    }

    /// Answers one RTU frame, as the device on the line does, or returns `null` when it stays
    /// silent.
    #[napi]
    pub fn answer(&self, frame: Buffer) -> Option<Buffer> {
        lock(&self.inner)
            .answer(frame.as_ref())
            .map(|reply| reply.as_bytes().into())
    }
}

/// Several devices on one simulated line, as devices share an RS485 pair: every frame reaches
/// all of them, the one it is addressed to answers, and each carries out a broadcast write.
/// The line shares each device, so the program's `ModbusServer` still reads and changes it.
#[napi(js_name = "ModbusLine")]
pub struct ModbusLine {
    inner: Arc<Mutex<Line>>,
}

#[napi]
impl ModbusLine {
    /// A line with no devices on it.
    #[napi(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        ModbusLine {
            inner: Arc::new(Mutex::new(Line::new())),
        }
    }

    /// Puts a device on the line.
    #[napi]
    pub fn attach(&self, server: &ModbusServer) {
        lock(&self.inner).attach_shared(Arc::clone(&server.inner));
    }

    /// How many devices are on the line.
    #[napi(getter)]
    pub fn count(&self) -> u32 {
        u32::try_from(lock(&self.inner).units().len()).unwrap_or(u32::MAX)
    }

    /// A serial port with the line on its far end: every frame written reaches each device,
    /// and whatever they answer waits to be read. Devices put on the line later are on the
    /// port too.
    #[napi]
    pub fn port(&self, settings: SerialSettings) -> napi::Result<SerialPort> {
        Ok(SerialPort {
            inner: Port::simulated(settings.settings()?, Arc::clone(&self.inner)),
        })
    }
}

/// Why a transaction failed, for the facade to throw as a `ModbusClientError`.
#[napi(object, js_name = "ModbusFailure")]
pub struct ModbusFailure {
    /// `Request`, `BroadcastRead`, `Port`, `Timeout`, `Frame`, `WrongUnit`, `WrongFunction`,
    /// `Mismatch`, or `Exception`.
    pub kind: String,
    /// The reason, in words.
    pub message: String,
    /// The unit the transaction asked.
    pub unit: u8,
    /// For an exception or a reply to another function, the function asked.
    pub function: Option<u8>,
    /// What the reply named instead: the unit that answered, or the function it answered.
    pub found: Option<u8>,
    /// For an exception, the code the device answered with.
    pub exception: Option<u8>,
    /// For a timeout, how many bytes of a reply had arrived.
    pub received: Option<u32>,
}

/// What one transaction came to: the values a read returned, or why it failed.
#[napi(object, js_name = "ModbusOutcome")]
pub struct ModbusOutcome {
    /// The registers a register read returned.
    pub registers: Option<Vec<u16>>,
    /// The states a coil or discrete input read returned.
    pub bits: Option<Vec<bool>>,
    /// Why the transaction failed, or absent when it succeeded.
    pub failure: Option<ModbusFailure>,
}

fn failure(unit: u8, error: &ClientError) -> ModbusFailure {
    let mut out = ModbusFailure {
        kind: String::new(),
        message: error.to_string(),
        unit,
        function: None,
        found: None,
        exception: None,
        received: None,
    };
    out.kind = match *error {
        ClientError::Request(_) => "Request",
        ClientError::BroadcastRead => "BroadcastRead",
        ClientError::Port(_) => "Port",
        ClientError::Timeout { received, .. } => {
            out.received = Some(u32::try_from(received).unwrap_or(u32::MAX));
            "Timeout"
        }
        ClientError::Frame(_) => "Frame",
        ClientError::WrongUnit { found, .. } => {
            out.found = Some(found);
            "WrongUnit"
        }
        ClientError::WrongFunction { expected, found } => {
            out.function = Some(expected);
            out.found = Some(found);
            "WrongFunction"
        }
        ClientError::Mismatch { .. } => "Mismatch",
        ClientError::Exception {
            function,
            exception,
            ..
        } => {
            out.function = Some(function);
            out.exception = Some(exception.code());
            "Exception"
        }
    }
    .to_owned();
    out
}

enum Values {
    None,
    Registers(Vec<u16>),
    Bits(Vec<bool>),
}

/// A Modbus RTU client on a serial line: the gateway that sends each request and waits for its
/// reply, holding each transaction to the Modbus over Serial Line specification.
#[napi(js_name = "ModbusClient")]
pub struct ModbusClient {
    inner: Arc<Mutex<Client>>,
}

#[napi]
impl ModbusClient {
    /// A client on a port, with a one-second response timeout and a 100 ms turnaround.
    #[napi(constructor)]
    pub fn new(port: &SerialPort) -> Self {
        ModbusClient {
            inner: Arc::new(Mutex::new(Client::new(port.inner.clone()))),
        }
    }

    /// The silence that separates two frames, in nanoseconds: 3.5 characters at the line's
    /// speed and format, and a fixed 1750 microseconds above 19200 baud.
    #[napi(js_name = "frameGapNanos")]
    pub fn frame_gap_nanos(settings: SerialSettings) -> napi::Result<f64> {
        Ok(Client::frame_gap(settings.settings()?).as_nanos() as f64)
    }

    /// How long the client waits for a whole reply, in milliseconds.
    #[napi(getter, js_name = "responseTimeoutMs")]
    pub fn response_timeout_ms(&self) -> f64 {
        lock(&self.inner).response_timeout().as_secs_f64() * 1_000.0
    }

    /// Sets how long the client waits for a whole reply, in milliseconds.
    #[napi(js_name = "setResponseTimeout")]
    pub fn set_response_timeout(&self, ms: f64) -> napi::Result<()> {
        lock(&self.inner).set_response_timeout(millis(ms)?);
        Ok(())
    }

    /// How long the client leaves the line quiet after a broadcast, in milliseconds.
    #[napi(getter, js_name = "turnaroundMs")]
    pub fn turnaround_ms(&self) -> f64 {
        lock(&self.inner).turnaround().as_secs_f64() * 1_000.0
    }

    /// Sets how long the client leaves the line quiet after a broadcast, in milliseconds.
    #[napi(js_name = "setTurnaround")]
    pub fn set_turnaround(&self, ms: f64) -> napi::Result<()> {
        lock(&self.inner).set_turnaround(millis(ms)?);
        Ok(())
    }

    /// Reads coils, function `0x01`.
    #[napi]
    pub async fn read_coils(
        &self,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> napi::Result<ModbusOutcome> {
        transact(&self.inner, unit, move |client| {
            client.read_coils(unit, start, quantity).map(Values::Bits)
        })
        .await
    }

    /// Reads discrete inputs, function `0x02`.
    #[napi]
    pub async fn read_discrete_inputs(
        &self,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> napi::Result<ModbusOutcome> {
        transact(&self.inner, unit, move |client| {
            client
                .read_discrete_inputs(unit, start, quantity)
                .map(Values::Bits)
        })
        .await
    }

    /// Reads holding registers, function `0x03`.
    #[napi]
    pub async fn read_holding_registers(
        &self,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> napi::Result<ModbusOutcome> {
        transact(&self.inner, unit, move |client| {
            client
                .read_holding_registers(unit, start, quantity)
                .map(Values::Registers)
        })
        .await
    }

    /// Reads input registers, function `0x04`.
    #[napi]
    pub async fn read_input_registers(
        &self,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> napi::Result<ModbusOutcome> {
        transact(&self.inner, unit, move |client| {
            client
                .read_input_registers(unit, start, quantity)
                .map(Values::Registers)
        })
        .await
    }

    /// Writes one coil, function `0x05`; unit 0 broadcasts it to every device.
    #[napi]
    pub async fn write_single_coil(
        &self,
        unit: u8,
        address: u16,
        on: bool,
    ) -> napi::Result<ModbusOutcome> {
        transact(&self.inner, unit, move |client| {
            client
                .write_single_coil(unit, address, on)
                .map(|()| Values::None)
        })
        .await
    }

    /// Writes one holding register, function `0x06`; unit 0 broadcasts it to every device.
    #[napi]
    pub async fn write_single_register(
        &self,
        unit: u8,
        address: u16,
        value: u16,
    ) -> napi::Result<ModbusOutcome> {
        transact(&self.inner, unit, move |client| {
            client
                .write_single_register(unit, address, value)
                .map(|()| Values::None)
        })
        .await
    }

    /// Writes a run of coils, function `0x0F`; unit 0 broadcasts them to every device.
    #[napi]
    pub async fn write_multiple_coils(
        &self,
        unit: u8,
        start: u16,
        values: Vec<bool>,
    ) -> napi::Result<ModbusOutcome> {
        transact(&self.inner, unit, move |client| {
            client
                .write_multiple_coils(unit, start, &values)
                .map(|()| Values::None)
        })
        .await
    }

    /// Writes a run of holding registers, function `0x10`; unit 0 broadcasts them to every
    /// device.
    #[napi]
    pub async fn write_multiple_registers(
        &self,
        unit: u8,
        start: u16,
        values: Vec<u16>,
    ) -> napi::Result<ModbusOutcome> {
        transact(&self.inner, unit, move |client| {
            client
                .write_multiple_registers(unit, start, &values)
                .map(|()| Values::None)
        })
        .await
    }
}

/// Runs one transaction on a blocking worker thread.
async fn transact(
    inner: &Arc<Mutex<Client>>,
    unit: u8,
    call: impl FnOnce(&mut Client) -> Result<Values, ClientError> + Send + 'static,
) -> napi::Result<ModbusOutcome> {
    let inner = Arc::clone(inner);
    let outcome = spawn_blocking(move || call(&mut lock(&inner)))
        .await
        .map_err(|error| {
            napi::Error::from_reason(format!("the transaction did not finish: {error}"))
        })?;
    Ok(match outcome {
        Ok(Values::None) => ModbusOutcome {
            registers: None,
            bits: None,
            failure: None,
        },
        Ok(Values::Registers(registers)) => ModbusOutcome {
            registers: Some(registers),
            bits: None,
            failure: None,
        },
        Ok(Values::Bits(bits)) => ModbusOutcome {
            registers: None,
            bits: Some(bits),
            failure: None,
        },
        Err(error) => ModbusOutcome {
            registers: None,
            bits: None,
            failure: Some(failure(unit, &error)),
        },
    })
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
