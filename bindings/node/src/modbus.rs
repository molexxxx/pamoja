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

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_modbus::{crc16, Adu, ModbusError, Pdu, Response};

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
pub fn modbus_read_holding_registers_reply(
    address: u8,
    values: Vec<u16>,
) -> napi::Result<Buffer> {
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
