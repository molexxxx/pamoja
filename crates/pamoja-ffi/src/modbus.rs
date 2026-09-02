//! The C ABI for Modbus RTU framing.
//!
//! These functions wrap [`pamoja_modbus`] for callers that reach the SDK through
//! the flat C boundary. Each request builder produces a complete RTU frame, CRC
//! included, ready to put on an RS485 line: the PDU and the frame around it are
//! one step here rather than two, because a caller crossing this boundary has no
//! use for a PDU on its own.
//!
//! A received frame goes the other way through [`pamoja_modbus_frame_parse`],
//! which validates the CRC before anything else can be read from it. The values a
//! device returned then come out as typed series, registers as `uint16` and coils
//! as one byte per bit, rather than as bytes the caller would have to unpack.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use pamoja_modbus::{crc16, Adu, ModbusError, Pdu};

use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};

/// An opaque handle to a parsed Modbus RTU frame with a verified CRC.
///
/// Read it with the `pamoja_modbus_frame_*` calls, then release it with
/// [`pamoja_modbus_frame_free`].
pub struct PamojaModbusFrame {
    adu: Adu,
}

/// An opaque handle to the 16-bit registers a device returned.
///
/// Read it with [`pamoja_registers_data`] and [`pamoja_registers_len`], then
/// release it with [`pamoja_registers_free`].
pub struct PamojaRegisters {
    registers: Vec<u16>,
}

/// Computes the CRC-16/MODBUS that every RTU frame ends with.
///
/// # Returns
///
/// The checksum over `bytes`, or the checksum of an empty input when `bytes` is
/// null and `bytes_len` is 0.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes, or be null when
/// `bytes_len` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_crc16(bytes: *const u8, bytes_len: usize) -> u16 {
    match read_bytes(bytes, bytes_len) {
        Ok(bytes) => crc16(&bytes),
        Err(_) => 0,
    }
}

/// Builds a read-coils request frame (function `0x01`).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// holding the frame, which the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Safety
///
/// `out_buffer` must point to a writable `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_read_coils(
    address: u8,
    start: u16,
    count: u16,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    request(out_buffer, address, || Ok(Pdu::read_coils(start, count)))
}

/// Builds a read-discrete-inputs request frame (function `0x02`).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// holding the frame.
///
/// # Safety
///
/// `out_buffer` must point to a writable `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_read_discrete_inputs(
    address: u8,
    start: u16,
    count: u16,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    request(out_buffer, address, || {
        Ok(Pdu::read_discrete_inputs(start, count))
    })
}

/// Builds a read-holding-registers request frame (function `0x03`).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// holding the frame.
///
/// # Safety
///
/// `out_buffer` must point to a writable `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_read_holding_registers(
    address: u8,
    start: u16,
    count: u16,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    request(out_buffer, address, || {
        Ok(Pdu::read_holding_registers(start, count))
    })
}

/// Builds a read-input-registers request frame (function `0x04`).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// holding the frame.
///
/// # Safety
///
/// `out_buffer` must point to a writable `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_read_input_registers(
    address: u8,
    start: u16,
    count: u16,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    request(out_buffer, address, || {
        Ok(Pdu::read_input_registers(start, count))
    })
}

/// Builds a write-single-coil request frame (function `0x05`).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// holding the frame.
///
/// # Safety
///
/// `out_buffer` must point to a writable `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_write_single_coil(
    address: u8,
    coil: u16,
    on: bool,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    request(out_buffer, address, || Ok(Pdu::write_single_coil(coil, on)))
}

/// Builds a write-single-register request frame (function `0x06`).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// holding the frame.
///
/// # Safety
///
/// `out_buffer` must point to a writable `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_write_single_register(
    address: u8,
    register: u16,
    value: u16,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    request(out_buffer, address, || {
        Ok(Pdu::write_single_register(register, value))
    })
}

/// Builds a write-multiple-registers request frame (function `0x10`).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// holding the frame, or [`PamojaStatus::InvalidArgument`] if `count` is zero or
/// beyond what one request may carry.
///
/// # Safety
///
/// `values` must point to at least `count` readable `uint16` values, or be null
/// when `count` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_write_multiple_registers(
    address: u8,
    start: u16,
    values: *const u16,
    count: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    let values = match read_values(values, count, "values") {
        Ok(values) => values,
        Err(status) => return status,
    };
    request(out_buffer, address, || {
        Pdu::write_multiple_registers(start, &values)
    })
}

/// Builds a write-multiple-coils request frame (function `0x0F`).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// holding the frame, or [`PamojaStatus::InvalidArgument`] if `count` is zero or
/// beyond what one request may carry.
///
/// # Safety
///
/// `values` must point to at least `count` readable bytes, one per coil and
/// non-zero for on, or be null when `count` is 0, and `out_buffer` must point to
/// a writable `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_write_multiple_coils(
    address: u8,
    start: u16,
    values: *const u8,
    count: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    let values = match read_values(values, count, "values") {
        Ok(values) => values,
        Err(status) => return status,
    };
    let coils: Vec<bool> = values.into_iter().map(|value| value != 0).collect();
    request(out_buffer, address, || {
        Pdu::write_multiple_coils(start, &coils)
    })
}

/// Builds a request frame from a raw function code and data.
///
/// This is the escape hatch for the function codes the SDK does not name.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// holding the frame, or [`PamojaStatus::InvalidArgument`] if the data is longer
/// than a PDU may be.
///
/// # Safety
///
/// `data` must point to at least `data_len` readable bytes, or be null when
/// `data_len` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_raw(
    address: u8,
    function: u8,
    data: *const u8,
    data_len: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    let data = match read_bytes(data, data_len) {
        Ok(data) => data,
        Err(status) => return status,
    };
    request(out_buffer, address, || Pdu::raw(function, &data))
}

/// Parses a received RTU frame, verifying its CRC.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a new handle the
/// caller must release with [`pamoja_modbus_frame_free`], or
/// [`PamojaStatus::Codec`] if the frame is truncated, oversized, or its CRC does
/// not match its contents.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes, or be null when
/// `bytes_len` is 0, and `out_frame` must point to a writable
/// `*mut PamojaModbusFrame`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_frame_parse(
    bytes: *const u8,
    bytes_len: usize,
    out_frame: *mut *mut PamojaModbusFrame,
) -> PamojaStatus {
    let out_frame = match out_slot(out_frame, "out_frame") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| Adu::parse(&bytes))) {
        Ok(Ok(adu)) => {
            *out_frame = Box::into_raw(Box::new(PamojaModbusFrame { adu }));
            PamojaStatus::Ok
        }
        Ok(Err(error)) => failed(error),
        Err(_) => panicked(),
    }
}

/// Returns the unit address a frame is addressed to or came from.
///
/// # Returns
///
/// The address, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from [`pamoja_modbus_frame_parse`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_frame_address(frame: *const PamojaModbusFrame) -> u8 {
    if frame.is_null() {
        return 0;
    }
    (*frame).adu.address()
}

/// Returns a frame's function code.
///
/// An exception response carries the request's function code with its high bit
/// set, so this returns that byte as it appeared on the wire.
///
/// # Returns
///
/// The function code, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from [`pamoja_modbus_frame_parse`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_frame_function(frame: *const PamojaModbusFrame) -> u8 {
    if frame.is_null() {
        return 0;
    }
    (*frame).adu.function_code()
}

/// Returns the exception code a device reported.
///
/// # Returns
///
/// The exception code, or 0 when the frame is not an exception response, is not
/// one this SDK names, or `frame` is null. Zero is not a defined exception code,
/// so it is unambiguous.
///
/// # Safety
///
/// `frame` must be a live handle from [`pamoja_modbus_frame_parse`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_frame_exception(frame: *const PamojaModbusFrame) -> u8 {
    if frame.is_null() {
        return 0;
    }
    (*frame)
        .adu
        .exception()
        .map_or(0, pamoja_modbus::Exception::code)
}

/// Returns a pointer to a frame's PDU: the function code and its data, without
/// the address or the CRC.
///
/// Use [`pamoja_modbus_frame_pdu_len`] for the length. The pointer is valid until
/// the handle is freed.
///
/// # Returns
///
/// A pointer to the PDU, or null if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from [`pamoja_modbus_frame_parse`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_frame_pdu(frame: *const PamojaModbusFrame) -> *const u8 {
    if frame.is_null() {
        return ptr::null();
    }
    (*frame).adu.pdu().as_ptr()
}

/// Returns the length in bytes of a frame's PDU.
///
/// # Returns
///
/// The length, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from [`pamoja_modbus_frame_parse`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_frame_pdu_len(frame: *const PamojaModbusFrame) -> usize {
    if frame.is_null() {
        return 0;
    }
    (*frame).adu.pdu().len()
}

/// Reads the 16-bit registers out of a read-registers response.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_registers` set to a new handle the
/// caller must release with [`pamoja_registers_free`], or
/// [`PamojaStatus::Codec`] if the response is not a well-formed read-registers
/// reply.
///
/// # Safety
///
/// `frame` must be a live handle from [`pamoja_modbus_frame_parse`] or null, and
/// `out_registers` must point to a writable `*mut PamojaRegisters`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_frame_registers(
    frame: *const PamojaModbusFrame,
    out_registers: *mut *mut PamojaRegisters,
) -> PamojaStatus {
    let out_registers = match out_slot(out_registers, "out_registers") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    if frame.is_null() {
        set_last_error("frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let adu = (*frame).adu;
    match catch_unwind(AssertUnwindSafe(|| {
        adu.response()
            .registers()
            .map(|registers| registers.collect::<Vec<u16>>())
    })) {
        Ok(Ok(registers)) => {
            *out_registers = Box::into_raw(Box::new(PamojaRegisters { registers }));
            PamojaStatus::Ok
        }
        Ok(Err(error)) => failed(error),
        Err(_) => panicked(),
    }
}

/// Reads the coils or discrete inputs out of a read-bits response.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// holding one byte per coil, `1` for on and `0` for off, which the caller must
/// release with [`pamoja_buffer_free`](crate::pamoja_buffer_free), or
/// [`PamojaStatus::Codec`] if the response does not carry `count` bits.
///
/// # Safety
///
/// `frame` must be a live handle from [`pamoja_modbus_frame_parse`] or null, and
/// `out_buffer` must point to a writable `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_frame_coils(
    frame: *const PamojaModbusFrame,
    count: u16,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    if frame.is_null() {
        set_last_error("frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let adu = (*frame).adu;
    match catch_unwind(AssertUnwindSafe(|| {
        adu.response()
            .coils(count)
            .map(|coils| coils.map(u8::from).collect::<Vec<u8>>())
    })) {
        Ok(Ok(coils)) => {
            *out_buffer = PamojaBuffer::into_raw(coils);
            PamojaStatus::Ok
        }
        Ok(Err(error)) => failed(error),
        Err(_) => panicked(),
    }
}

/// Releases a parsed frame handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `frame` must be a handle from [`pamoja_modbus_frame_parse`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_modbus_frame_free(frame: *mut PamojaModbusFrame) {
    if !frame.is_null() {
        drop(Box::from_raw(frame));
    }
}

/// Returns a pointer to the registers a device returned.
///
/// Use [`pamoja_registers_len`] for the count. The pointer is valid until the
/// handle is freed.
///
/// # Returns
///
/// A pointer to the registers, or null if `registers` is null.
///
/// # Safety
///
/// `registers` must be a live handle from [`pamoja_modbus_frame_registers`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_registers_data(registers: *const PamojaRegisters) -> *const u16 {
    if registers.is_null() {
        return ptr::null();
    }
    (*registers).registers.as_ptr()
}

/// Returns how many registers a device returned.
///
/// # Returns
///
/// The count, or 0 if `registers` is null.
///
/// # Safety
///
/// `registers` must be a live handle from [`pamoja_modbus_frame_registers`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_registers_len(registers: *const PamojaRegisters) -> usize {
    if registers.is_null() {
        return 0;
    }
    (*registers).registers.len()
}

/// Releases a register series.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `registers` must be a handle from [`pamoja_modbus_frame_registers`] that has
/// not already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_registers_free(registers: *mut PamojaRegisters) {
    if !registers.is_null() {
        drop(Box::from_raw(registers));
    }
}

/// Builds a PDU, wraps it in a frame for `address`, and hands back the bytes.
///
/// # Safety
///
/// `out_buffer` must point to a writable `*mut PamojaBuffer`.
unsafe fn request(
    out_buffer: &mut *mut PamojaBuffer,
    address: u8,
    build: impl FnOnce() -> Result<Pdu, ModbusError>,
) -> PamojaStatus {
    match catch_unwind(AssertUnwindSafe(|| {
        build().map(|pdu| pdu.to_adu(address).as_bytes().to_vec())
    })) {
        Ok(Ok(bytes)) => {
            *out_buffer = PamojaBuffer::into_raw(bytes);
            PamojaStatus::Ok
        }
        Ok(Err(error)) => failed(error),
        Err(_) => panicked(),
    }
}

/// Rejects a null out-pointer and borrows the slot it names, cleared.
///
/// # Safety
///
/// `out` must be null or point to a writable `*mut T` that outlives the call.
unsafe fn out_slot<'a, T>(out: *mut *mut T, name: &str) -> Result<&'a mut *mut T, PamojaStatus> {
    if out.is_null() {
        set_last_error(format!("{name} must not be null"));
        return Err(PamojaStatus::InvalidArgument);
    }
    let slot = &mut *out;
    *slot = ptr::null_mut();
    Ok(slot)
}

/// Copies a borrowed array of `count` values, treating a zero count as empty.
///
/// # Safety
///
/// When `count` is non-zero, `ptr` must point to at least `count` readable `T`
/// values.
unsafe fn read_values<T: Copy>(
    ptr: *const T,
    count: usize,
    name: &str,
) -> Result<Vec<T>, PamojaStatus> {
    if count == 0 {
        Ok(Vec::new())
    } else if ptr.is_null() {
        set_last_error(format!(
            "{name} must not be null when its count is non-zero"
        ));
        Err(PamojaStatus::InvalidArgument)
    } else {
        Ok(std::slice::from_raw_parts(ptr, count).to_vec())
    }
}

/// Records a Modbus error and maps it onto its status.
fn failed(error: ModbusError) -> PamojaStatus {
    set_last_error(error.to_string());
    match error {
        ModbusError::InvalidValueCount => PamojaStatus::InvalidArgument,
        ModbusError::FrameTooShort
        | ModbusError::FrameTooLong
        | ModbusError::CrcMismatch { .. }
        | ModbusError::MalformedResponse => PamojaStatus::Codec,
    }
}

/// Records a caught panic and reports it as [`PamojaStatus::Panic`].
fn panicked() -> PamojaStatus {
    set_last_error("panic at the FFI boundary".to_owned());
    PamojaStatus::Panic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len};

    /// Copies a buffer handle's bytes out and releases the handle.
    ///
    /// # Safety
    ///
    /// `buffer` must be a live handle that has not already been freed.
    unsafe fn take(buffer: *mut PamojaBuffer) -> Vec<u8> {
        let bytes =
            std::slice::from_raw_parts(pamoja_buffer_data(buffer), pamoja_buffer_len(buffer))
                .to_vec();
        pamoja_buffer_free(buffer);
        bytes
    }

    #[test]
    fn a_read_request_matches_the_specification_example() {
        let mut out = ptr::null_mut();
        // Safety: the out-pointer is writable.
        let frame = unsafe {
            assert_eq!(
                pamoja_modbus_read_holding_registers(0x11, 0x006B, 3, &mut out),
                PamojaStatus::Ok
            );
            take(out)
        };
        assert_eq!(
            frame,
            vec![0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87],
            "the frame carries the address, the PDU, and the CRC"
        );
    }

    #[test]
    fn a_reply_parses_into_its_registers() {
        let on_wire = Adu::from_pdu(0x11, &[0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64])
            .expect("assemble");
        let bytes = on_wire.as_bytes();
        let mut frame = ptr::null_mut();
        let mut registers = ptr::null_mut();

        // Safety: the input is a valid slice and the out-pointers are writable.
        unsafe {
            assert_eq!(
                pamoja_modbus_frame_parse(bytes.as_ptr(), bytes.len(), &mut frame),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_modbus_frame_address(frame), 0x11);
            assert_eq!(pamoja_modbus_frame_function(frame), 0x03);
            assert_eq!(pamoja_modbus_frame_exception(frame), 0);
            assert_eq!(
                pamoja_modbus_frame_registers(frame, &mut registers),
                PamojaStatus::Ok
            );
            let values = std::slice::from_raw_parts(
                pamoja_registers_data(registers),
                pamoja_registers_len(registers),
            )
            .to_vec();
            pamoja_registers_free(registers);
            pamoja_modbus_frame_free(frame);
            assert_eq!(values, vec![0x022B, 0x0000, 0x0064]);
        }
    }

    #[test]
    fn a_bit_reply_unpacks_one_byte_per_coil() {
        // A read-coils reply carrying the bits 1,0,1,1 in one byte.
        let on_wire = Adu::from_pdu(0x11, &[0x01, 0x01, 0b0000_1101]).expect("assemble");
        let bytes = on_wire.as_bytes();
        let mut frame = ptr::null_mut();
        let mut coils = ptr::null_mut();

        // Safety: the input is a valid slice and the out-pointers are writable.
        unsafe {
            assert_eq!(
                pamoja_modbus_frame_parse(bytes.as_ptr(), bytes.len(), &mut frame),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_modbus_frame_coils(frame, 4, &mut coils),
                PamojaStatus::Ok
            );
            let values = take(coils);
            pamoja_modbus_frame_free(frame);
            assert_eq!(values, vec![1, 0, 1, 1]);
        }
    }

    #[test]
    fn a_corrupt_frame_is_refused() {
        let mut bytes = Adu::from_pdu(0x11, &[0x03, 0x00, 0x6B, 0x00, 0x03])
            .expect("assemble")
            .as_bytes()
            .to_vec();
        bytes[2] ^= 0xFF;
        let mut frame = ptr::null_mut();

        // Safety: the input is a valid slice and the out-pointer is writable.
        let status = unsafe { pamoja_modbus_frame_parse(bytes.as_ptr(), bytes.len(), &mut frame) };
        assert_eq!(
            status,
            PamojaStatus::Codec,
            "a frame mangled on the wire must not reach the application"
        );
        assert!(frame.is_null());
    }

    #[test]
    fn an_exception_reply_reports_its_code() {
        // Function 0x03 with the high bit set, then exception 0x02.
        let on_wire = Adu::from_pdu(0x11, &[0x83, 0x02]).expect("assemble");
        let bytes = on_wire.as_bytes();
        let mut frame = ptr::null_mut();

        // Safety: the input is a valid slice and the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_modbus_frame_parse(bytes.as_ptr(), bytes.len(), &mut frame),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_modbus_frame_exception(frame), 0x02);
            assert_eq!(pamoja_modbus_frame_pdu_len(frame), 2);
            pamoja_modbus_frame_free(frame);
        }
    }

    #[test]
    fn an_empty_write_is_rejected() {
        let mut out = ptr::null_mut();
        // Safety: a null values pointer is allowed when the count is zero.
        let status =
            unsafe { pamoja_modbus_write_multiple_registers(0x11, 0, ptr::null(), 0, &mut out) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
        assert!(out.is_null());
    }

    #[test]
    fn the_checksum_matches_the_frame_it_ends() {
        let frame = Adu::from_pdu(0x11, &[0x03, 0x00, 0x6B, 0x00, 0x03]).expect("assemble");
        let bytes = frame.as_bytes();
        let split = bytes.len() - 2;
        // Safety: the input is a valid slice.
        let computed = unsafe { pamoja_modbus_crc16(bytes.as_ptr(), split) };
        assert_eq!(computed.to_le_bytes(), bytes[split..]);
    }
}
