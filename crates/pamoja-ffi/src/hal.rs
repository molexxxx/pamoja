//! The C ABI for the bus layer: one I2C bus that a program and every driver on it share.
//!
//! A bus is the kernel's adapter on a Linux board, a set of simulated parts, or a script of
//! the transfers a driver is expected to make, and the same calls work on each: a write, a
//! read, and a write then a read in one transaction, which is how a register is read. A
//! driver built on a bus, such as the BME280 in [`crate::sensors_driver`], holds a share of
//! it, so the caller keeps its own handle, reads back what a simulated part holds after a
//! driver wrote to it, and frees its handle whenever it likes; the bus closes when the last
//! holder lets go.
//!
//! A part and a script are built here and copied onto a bus, so the handles that built them
//! stay the caller's to free. Every failed transfer returns [`PamojaStatus::Io`] with the
//! reason in the last error message: nothing answered at the address, the script expected
//! something else, or the kernel's own words for what went wrong.
//!
//! A simulated part is one of three kinds, and one handle holds any of them:
//! [`PAMOJA_I2C_PART_BYTES`], registers a byte wide, as Bosch's parts have;
//! [`PAMOJA_I2C_PART_WORDS`], registers sixteen bits wide, as Texas Instruments' parts have;
//! and [`PAMOJA_I2C_PART_COMMANDS`], commands that leave replies, as Sensirion's parts take.
//! A call made on the wrong kind of part is refused with [`PamojaStatus::InvalidArgument`],
//! or reads as zero where the call returns a value.

use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use pamoja_hal::bus::{BusKind, I2cBus, OpenError};
use pamoja_hal::i2c::{ErrorKind, I2c, NoAcknowledgeSource};
use pamoja_hal::script::{I2cScript, I2cStep};
use pamoja_hal::sim::{CommandPart, I2cPart, Part, WordPart};

use crate::{read_bytes, read_str, set_last_error, PamojaBuffer, PamojaStatus};

/// A bus kind: the kernel's adapter, with real parts on real wires.
pub const PAMOJA_I2C_BUS_ADAPTER: u8 = 0;

/// A bus kind: simulated parts answering from their registers.
pub const PAMOJA_I2C_BUS_SIMULATED: u8 = 1;

/// A bus kind: a script of the transfers a driver is expected to make.
pub const PAMOJA_I2C_BUS_SCRIPTED: u8 = 2;

/// A scripted failure: nothing acknowledged the address.
pub const PAMOJA_I2C_FAULT_NO_ACKNOWLEDGE_ADDRESS: u8 = 0;

/// A scripted failure: the part did not acknowledge a data byte.
pub const PAMOJA_I2C_FAULT_NO_ACKNOWLEDGE_DATA: u8 = 1;

/// A scripted failure: a missing acknowledge, with no telling whether of the address or data.
pub const PAMOJA_I2C_FAULT_NO_ACKNOWLEDGE: u8 = 2;

/// A scripted failure: a bus error, such as a misplaced start or stop condition.
pub const PAMOJA_I2C_FAULT_BUS: u8 = 3;

/// A scripted failure: another controller won the bus.
pub const PAMOJA_I2C_FAULT_ARBITRATION_LOSS: u8 = 4;

/// A scripted failure: data arrived faster than it was taken.
pub const PAMOJA_I2C_FAULT_OVERRUN: u8 = 5;

/// A scripted failure of no more particular kind.
pub const PAMOJA_I2C_FAULT_OTHER: u8 = 6;

/// A part kind: 256 registers a byte wide.
pub const PAMOJA_I2C_PART_BYTES: u8 = 0;

/// A part kind: 256 registers sixteen bits wide, each traveling most significant byte first.
pub const PAMOJA_I2C_PART_WORDS: u8 = 1;

/// A part kind: commands, each leaving the reply it was given for a read to take.
pub const PAMOJA_I2C_PART_COMMANDS: u8 = 2;

/// A part that is not there, answering from its registers or its commands. Opaque; release it
/// with [`pamoja_i2c_part_free`].
pub struct PamojaI2cPart {
    pub(crate) part: Part,
}

impl PamojaI2cPart {
    /// Boxes any kind of simulated part for the caller to own.
    ///
    /// # Arguments
    ///
    /// * `part` - the part.
    ///
    /// # Returns
    ///
    /// A raw handle the caller releases with [`pamoja_i2c_part_free`].
    pub(crate) fn into_raw(part: impl Into<Part>) -> *mut PamojaI2cPart {
        Box::into_raw(Box::new(PamojaI2cPart { part: part.into() }))
    }
}

/// The transfers a driver is expected to make, in order, and the replies. Opaque; release it
/// with [`pamoja_i2c_script_free`].
pub struct PamojaI2cScript {
    steps: Vec<I2cStep>,
}

/// One I2C bus, shared with every driver built on it. Opaque; release it with
/// [`pamoja_i2c_bus_free`].
pub struct PamojaI2cBus {
    pub(crate) bus: I2cBus,
}

/// Creates a part answering at one address from 256 registers a byte wide, every one reading
/// zero.
///
/// # Arguments
///
/// * `address` - the 7-bit address it answers to.
///
/// # Returns
///
/// The part, which the caller releases with [`pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_i2c_part_new(address: u8) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(I2cPart::new(address))
}

/// Creates a part answering at one address from 256 registers sixteen bits wide, every one
/// reading zero. A pointer byte names a register, a write of a word stores it and leaves the
/// pointer where it was, and a read takes words from the pointer on.
///
/// # Arguments
///
/// * `address` - the 7-bit address it answers to.
///
/// # Returns
///
/// The part, which the caller releases with [`pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_i2c_word_part_new(address: u8) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(WordPart::new(address))
}

/// Creates a part answering at one address that takes commands and has been given no replies
/// yet. A write sends a command and any arguments after it; a read takes the reply that
/// command left, once, padded with `0xFF`; a read with no reply waiting is not acknowledged.
///
/// # Arguments
///
/// * `address` - the 7-bit address it answers to.
/// * `width` - how many bytes a command takes: two for Sensirion's 16-bit commands.
///
/// # Returns
///
/// The part, which the caller releases with [`pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_i2c_command_part_new(address: u8, width: usize) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(CommandPart::new(address, width))
}

/// Returns which kind of part a handle holds.
///
/// # Arguments
///
/// * `part` - the part.
///
/// # Returns
///
/// [`PAMOJA_I2C_PART_BYTES`], [`PAMOJA_I2C_PART_WORDS`], or [`PAMOJA_I2C_PART_COMMANDS`]; the
/// bytes code for a null part.
///
/// # Safety
///
/// `part` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_kind(part: *const PamojaI2cPart) -> u8 {
    if part.is_null() {
        return PAMOJA_I2C_PART_BYTES;
    }
    match (*part).part {
        Part::Bytes(_) => PAMOJA_I2C_PART_BYTES,
        Part::Words(_) => PAMOJA_I2C_PART_WORDS,
        Part::Commands(_) => PAMOJA_I2C_PART_COMMANDS,
    }
}

/// Puts bytes in a part whose registers are a byte wide, from a register on. Past the last
/// register they wrap to the first.
///
/// # Arguments
///
/// * `part` - the part.
/// * `first` - the register the bytes start at.
/// * `bytes` - what to put there.
/// * `len` - how many bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null part, a null `bytes`
/// with a nonzero length, or a part of another kind.
///
/// # Safety
///
/// `part` must be a live handle or null, and `bytes` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_load(
    part: *mut PamojaI2cPart,
    first: u8,
    bytes: *const u8,
    len: usize,
) -> PamojaStatus {
    let bytes = match read_bytes(bytes, len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match byte_part(part) {
        Ok(part) => {
            part.load(first, &bytes);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Reads what one register of a part whose registers are a byte wide holds.
///
/// # Arguments
///
/// * `part` - the part.
/// * `register` - which register.
///
/// # Returns
///
/// Its value, which is what a driver wrote if it wrote one, or 0 for a null part or a part of
/// another kind.
///
/// # Safety
///
/// `part` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_register(part: *const PamojaI2cPart, register: u8) -> u8 {
    match part.as_ref().map(|part| &part.part) {
        Some(Part::Bytes(part)) => part.register(register),
        _ => 0,
    }
}

/// Reads consecutive registers of a part whose registers are a byte wide, from one register
/// on.
///
/// # Arguments
///
/// * `part` - the part.
/// * `first` - the first register.
/// * `out` - receives one byte per register. Past the last register it wraps to the first.
/// * `len` - how many registers to read.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument or a part of
/// another kind.
///
/// # Safety
///
/// `part` must be a live handle or null, and `out` must point to `len` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_read(
    part: *const PamojaI2cPart,
    first: u8,
    out: *mut u8,
    len: usize,
) -> PamojaStatus {
    let Some(out) = out_bytes(out, len) else {
        return PamojaStatus::InvalidArgument;
    };
    let part = match byte_part(part.cast_mut()) {
        Ok(part) => part,
        Err(status) => return status,
    };
    let mut at = first;
    for slot in out {
        *slot = part.register(at);
        at = at.wrapping_add(1);
    }
    PamojaStatus::Ok
}

/// Puts a value in one register of a part whose registers are sixteen bits wide, read-only
/// bits included, the way the part itself would.
///
/// # Arguments
///
/// * `part` - the part.
/// * `register` - the register.
/// * `value` - what it holds.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null part or a part of
/// another kind.
///
/// # Safety
///
/// `part` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_set_word(
    part: *mut PamojaI2cPart,
    register: u8,
    value: u16,
) -> PamojaStatus {
    match word_part(part) {
        Ok(part) => {
            part.set(register, value);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Marks bits of one register of a part whose registers are sixteen bits wide as the part's to
/// set: a driver's write leaves them as the part holds them, as a conversion-ready flag is.
///
/// # Arguments
///
/// * `part` - the part.
/// * `register` - the register.
/// * `mask` - the bits that are the part's.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null part or a part of
/// another kind.
///
/// # Safety
///
/// `part` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_read_only(
    part: *mut PamojaI2cPart,
    register: u8,
    mask: u16,
) -> PamojaStatus {
    match word_part(part) {
        Ok(held) => {
            *held = held.clone().read_only(register, mask);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Reads what one register of a part whose registers are sixteen bits wide holds.
///
/// # Arguments
///
/// * `part` - the part.
/// * `register` - which register.
///
/// # Returns
///
/// Its value, which is what a driver wrote there apart from the read-only bits, or 0 for a
/// null part or a part of another kind.
///
/// # Safety
///
/// `part` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_word(part: *const PamojaI2cPart, register: u8) -> u16 {
    match part.as_ref().map(|part| &part.part) {
        Some(Part::Words(part)) => part.word(register),
        _ => 0,
    }
}

/// Gives a part that takes commands the reply one command leaves, in place of any reply given
/// before.
///
/// # Arguments
///
/// * `part` - the part.
/// * `command` - the command's bytes.
/// * `command_len` - how many.
/// * `reply` - what a read after the command returns.
/// * `reply_len` - how many bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument or a part of
/// another kind.
///
/// # Safety
///
/// `part` must be a live handle or null, `command` must point to `command_len` readable bytes,
/// and `reply` to `reply_len`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_answer(
    part: *mut PamojaI2cPart,
    command: *const u8,
    command_len: usize,
    reply: *const u8,
    reply_len: usize,
) -> PamojaStatus {
    let command = match read_bytes(command, command_len) {
        Ok(command) => command,
        Err(status) => return status,
    };
    let reply = match read_bytes(reply, reply_len) {
        Ok(reply) => reply,
        Err(status) => return status,
    };
    match command_part(part) {
        Ok(part) => {
            part.answer(&command, &reply);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Returns how many writes a part that takes commands has received.
///
/// # Arguments
///
/// * `part` - the part.
///
/// # Returns
///
/// The count, or 0 for a null part or a part of another kind.
///
/// # Safety
///
/// `part` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_received_count(part: *const PamojaI2cPart) -> usize {
    match part.as_ref().map(|part| &part.part) {
        Some(Part::Commands(part)) => part.received().len(),
        _ => 0,
    }
}

/// Copies one write a part that takes commands received: a command and any arguments after
/// it.
///
/// # Arguments
///
/// * `part` - the part.
/// * `index` - which write, oldest first, below [`pamoja_i2c_part_received_count`].
///
/// # Returns
///
/// The write's bytes, which the caller releases with [`crate::pamoja_buffer_free`], or null for
/// a null part, a part of another kind, or an index past the last write.
///
/// # Safety
///
/// `part` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_received(
    part: *const PamojaI2cPart,
    index: usize,
) -> *mut PamojaBuffer {
    match part.as_ref().map(|part| &part.part) {
        Some(Part::Commands(part)) => match part.received().get(index) {
            Some(write) => PamojaBuffer::into_raw(write.clone()),
            None => std::ptr::null_mut(),
        },
        _ => std::ptr::null_mut(),
    }
}

/// Returns the address a part answers to.
///
/// # Arguments
///
/// * `part` - the part.
///
/// # Returns
///
/// The 7-bit address, or 0 for a null part.
///
/// # Safety
///
/// `part` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_address(part: *const PamojaI2cPart) -> u8 {
    if part.is_null() {
        return 0;
    }
    (*part).part.address()
}

/// Returns how many transfers a part has served.
///
/// # Arguments
///
/// * `part` - the part.
///
/// # Returns
///
/// The count, or 0 for a null part.
///
/// # Safety
///
/// `part` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_transfers(part: *const PamojaI2cPart) -> usize {
    if part.is_null() {
        return 0;
    }
    (*part).part.transfers()
}

/// Releases a part. A null pointer is ignored.
///
/// # Safety
///
/// `part` must be a handle that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_part_free(part: *mut PamojaI2cPart) {
    if !part.is_null() {
        drop(Box::from_raw(part));
    }
}

/// Creates an empty script.
///
/// # Returns
///
/// The script, which the caller fills with the step functions and releases with
/// [`pamoja_i2c_script_free`].
#[no_mangle]
pub extern "C" fn pamoja_i2c_script_new() -> *mut PamojaI2cScript {
    Box::into_raw(Box::new(PamojaI2cScript { steps: Vec::new() }))
}

/// Adds a step: the driver writes exactly these bytes to the address.
///
/// # Arguments
///
/// * `script` - the script.
/// * `address` - the 7-bit address the write must go to.
/// * `bytes` - the bytes the driver must send.
/// * `len` - how many.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `script` must be a live handle or null, and `bytes` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_script_write(
    script: *mut PamojaI2cScript,
    address: u8,
    bytes: *const u8,
    len: usize,
) -> PamojaStatus {
    add_step(script, || {
        Ok(I2cStep::write(address, read_bytes(bytes, len)?))
    })
}

/// Adds a step: the driver reads from the address and receives the reply, whose length is
/// the length it must ask for.
///
/// # Arguments
///
/// * `script` - the script.
/// * `address` - the 7-bit address the read must come from.
/// * `reply` - the bytes the part answers with.
/// * `len` - how many.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `script` must be a live handle or null, and `reply` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_script_read(
    script: *mut PamojaI2cScript,
    address: u8,
    reply: *const u8,
    len: usize,
) -> PamojaStatus {
    add_step(script, || {
        Ok(I2cStep::read(address, read_bytes(reply, len)?))
    })
}

/// Adds a step: the driver writes these bytes and then reads the reply in one transaction,
/// the shape of a register read.
///
/// # Arguments
///
/// * `script` - the script.
/// * `address` - the 7-bit address of the part.
/// * `bytes` - the bytes the driver must send first, usually a register address.
/// * `len` - how many.
/// * `reply` - the bytes the part answers with.
/// * `reply_len` - how many.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `script` must be a live handle or null, `bytes` must point to `len` readable bytes, and
/// `reply` to `reply_len`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_script_write_read(
    script: *mut PamojaI2cScript,
    address: u8,
    bytes: *const u8,
    len: usize,
    reply: *const u8,
    reply_len: usize,
) -> PamojaStatus {
    add_step(script, || {
        Ok(I2cStep::write_read(
            address,
            read_bytes(bytes, len)?,
            read_bytes(reply, reply_len)?,
        ))
    })
}

/// Adds a step: the next transfer to the address fails, the way a missing or busy part does.
///
/// # Arguments
///
/// * `script` - the script.
/// * `address` - the 7-bit address the failing transfer must go to.
/// * `fault` - one of the `PAMOJA_I2C_FAULT_` codes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null script or a code
/// that names no fault.
///
/// # Safety
///
/// `script` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_script_fault(
    script: *mut PamojaI2cScript,
    address: u8,
    fault: u8,
) -> PamojaStatus {
    add_step(script, || {
        let kind = fault_kind(fault).ok_or_else(|| {
            set_last_error(format!("{fault} is not an I2C fault code"));
            PamojaStatus::InvalidArgument
        })?;
        Ok(I2cStep::fault(address, kind))
    })
}

/// Returns how many steps a script holds.
///
/// # Arguments
///
/// * `script` - the script.
///
/// # Returns
///
/// The count, or 0 for a null script.
///
/// # Safety
///
/// `script` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_script_len(script: *const PamojaI2cScript) -> usize {
    if script.is_null() {
        return 0;
    }
    (*script).steps.len()
}

/// Releases a script. A null pointer is ignored.
///
/// # Safety
///
/// `script` must be a handle that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_script_free(script: *mut PamojaI2cScript) {
    if !script.is_null() {
        drop(Box::from_raw(script));
    }
}

/// Opens the kernel's I2C adapter, such as `/dev/i2c-1` on a Raspberry Pi.
///
/// # Arguments
///
/// * `path` - the adapter's device file.
/// * `out_bus` - receives the bus, or null when opening fails.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the bus in `out_bus`; [`PamojaStatus::Unsupported`] on any
/// platform but Linux; [`PamojaStatus::InvalidArgument`] for a null or non-UTF-8 argument; or
/// [`PamojaStatus::Io`] when the file cannot be opened as an adapter, with a last error
/// message that names it.
///
/// # Safety
///
/// `path` must be a null-terminated string or null, and `out_bus` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_open(
    path: *const c_char,
    out_bus: *mut *mut PamojaI2cBus,
) -> PamojaStatus {
    if out_bus.is_null() {
        set_last_error("out_bus must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_bus = std::ptr::null_mut();
    let Some(path) = read_str(path, "path") else {
        return PamojaStatus::InvalidArgument;
    };
    match catch_unwind(AssertUnwindSafe(|| I2cBus::open(path))) {
        Ok(Ok(bus)) => {
            *out_bus = Box::into_raw(Box::new(PamojaI2cBus { bus }));
            PamojaStatus::Ok
        }
        Ok(Err(error)) => {
            set_last_error(error.to_string());
            match error {
                OpenError::Unsupported => PamojaStatus::Unsupported,
                OpenError::Adapter { .. } => PamojaStatus::Io,
            }
        }
        Err(_) => panicked(),
    }
}

/// Creates a simulated bus with no parts on it yet; [`pamoja_i2c_bus_attach`] puts them on.
///
/// # Returns
///
/// The bus, which the caller releases with [`pamoja_i2c_bus_free`]. Until a part is attached
/// every transfer finds nothing at its address.
#[no_mangle]
pub extern "C" fn pamoja_i2c_bus_simulated() -> *mut PamojaI2cBus {
    Box::into_raw(Box::new(PamojaI2cBus {
        bus: I2cBus::simulated(Vec::<Part>::new()),
    }))
}

/// Puts a copy of a part of any kind on a simulated bus, in place of any part already at its
/// address.
///
/// # Arguments
///
/// * `bus` - the bus.
/// * `part` - the part to copy onto it; the caller still owns the handle.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument or a bus
/// that is not simulated.
///
/// # Safety
///
/// `bus` and `part` must be live handles or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_attach(
    bus: *const PamojaI2cBus,
    part: *const PamojaI2cPart,
) -> PamojaStatus {
    if bus.is_null() || part.is_null() {
        set_last_error("bus and part must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match (*bus).bus.attach((*part).part.clone()) {
        Ok(_) => PamojaStatus::Ok,
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::InvalidArgument
        }
    }
}

/// Creates a bus that plays a script and refuses any transfer that is not its next step.
///
/// # Arguments
///
/// * `script` - the steps to copy; the caller still owns the handle.
///
/// # Returns
///
/// The bus, which the caller releases with [`pamoja_i2c_bus_free`], or null for a null
/// script.
///
/// # Safety
///
/// `script` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_scripted(
    script: *const PamojaI2cScript,
) -> *mut PamojaI2cBus {
    if script.is_null() {
        set_last_error("script must not be null".to_owned());
        return std::ptr::null_mut();
    }
    let steps = (*script).steps.clone();
    Box::into_raw(Box::new(PamojaI2cBus {
        bus: I2cBus::scripted(I2cScript::new(steps)),
    }))
}

/// Returns what answers on a bus.
///
/// # Arguments
///
/// * `bus` - the bus.
///
/// # Returns
///
/// [`PAMOJA_I2C_BUS_ADAPTER`], [`PAMOJA_I2C_BUS_SIMULATED`], or [`PAMOJA_I2C_BUS_SCRIPTED`];
/// the simulated code for a null bus.
///
/// # Safety
///
/// `bus` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_kind(bus: *const PamojaI2cBus) -> u8 {
    if bus.is_null() {
        return PAMOJA_I2C_BUS_SIMULATED;
    }
    match (*bus).bus.kind() {
        BusKind::Adapter => PAMOJA_I2C_BUS_ADAPTER,
        BusKind::Simulated => PAMOJA_I2C_BUS_SIMULATED,
        BusKind::Scripted => PAMOJA_I2C_BUS_SCRIPTED,
    }
}

/// Writes bytes to a part, in one transaction.
///
/// # Arguments
///
/// * `bus` - the bus.
/// * `address` - the part's 7-bit address.
/// * `bytes` - what to write, usually a register address and then its value.
/// * `len` - how many bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null argument; or
/// [`PamojaStatus::Io`] when the transfer fails, with the reason in the last error message.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `bytes` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_write(
    bus: *const PamojaI2cBus,
    address: u8,
    bytes: *const u8,
    len: usize,
) -> PamojaStatus {
    let bytes = match read_bytes(bytes, len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    on_bus(bus, |bus| bus.write(address, &bytes))
}

/// Reads bytes from a part, in one transaction.
///
/// # Arguments
///
/// * `bus` - the bus.
/// * `address` - the part's 7-bit address.
/// * `out` - receives the bytes.
/// * `len` - how many bytes to read.
///
/// # Returns
///
/// As [`pamoja_i2c_bus_write`].
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out` must point to `len` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_read(
    bus: *const PamojaI2cBus,
    address: u8,
    out: *mut u8,
    len: usize,
) -> PamojaStatus {
    let Some(out) = out_bytes(out, len) else {
        return PamojaStatus::InvalidArgument;
    };
    on_bus(bus, |bus| bus.read(address, out))
}

/// Writes bytes to a part and reads its reply in one transaction, with a repeated start
/// between them, which is how a register is read.
///
/// # Arguments
///
/// * `bus` - the bus.
/// * `address` - the part's 7-bit address.
/// * `bytes` - what to write first, usually the register address.
/// * `len` - how many bytes to write.
/// * `out` - receives the reply.
/// * `out_len` - how many bytes to read.
///
/// # Returns
///
/// As [`pamoja_i2c_bus_write`].
///
/// # Safety
///
/// `bus` must be a live handle or null, `bytes` must point to `len` readable bytes, and `out`
/// to `out_len` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_write_read(
    bus: *const PamojaI2cBus,
    address: u8,
    bytes: *const u8,
    len: usize,
    out: *mut u8,
    out_len: usize,
) -> PamojaStatus {
    let bytes = match read_bytes(bytes, len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Some(out) = out_bytes(out, out_len) else {
        return PamojaStatus::InvalidArgument;
    };
    on_bus(bus, |bus| bus.write_read(address, &bytes, out))
}

/// Returns how many transfers have been made on a bus, by the caller and every driver on it,
/// including any that failed.
///
/// # Arguments
///
/// * `bus` - the bus.
///
/// # Returns
///
/// The count, or 0 for a null bus.
///
/// # Safety
///
/// `bus` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_transfers(bus: *const PamojaI2cBus) -> usize {
    if bus.is_null() {
        return 0;
    }
    (*bus).bus.transfers()
}

/// Reports how many steps a scripted bus has left.
///
/// # Arguments
///
/// * `bus` - the bus.
/// * `out_remaining` - receives the steps not yet reached.
///
/// # Returns
///
/// `true` with the count in `out_remaining` for a scripted bus; `false` for any other bus, a
/// null bus, or a null `out_remaining`.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_remaining` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_remaining(
    bus: *const PamojaI2cBus,
    out_remaining: *mut usize,
) -> bool {
    if bus.is_null() || out_remaining.is_null() {
        return false;
    }
    match (*bus).bus.remaining() {
        Some(remaining) => {
            *out_remaining = remaining;
            true
        }
        None => false,
    }
}

/// Returns how long the drivers on a bus have asked to wait.
///
/// # Arguments
///
/// * `bus` - the bus.
///
/// # Returns
///
/// The total in microseconds, counted whether or not the process slept through it, or 0 for a
/// null bus.
///
/// # Safety
///
/// `bus` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_waited_micros(bus: *const PamojaI2cBus) -> u64 {
    if bus.is_null() {
        return 0;
    }
    (*bus).bus.waited_micros()
}

/// Copies what a simulated part holds now, with whatever drivers have written to it.
///
/// # Arguments
///
/// * `bus` - the bus.
/// * `address` - the part's address.
///
/// # Returns
///
/// A new part of the kind that answers at the address, which [`pamoja_i2c_part_kind`] names
/// and the caller releases with [`pamoja_i2c_part_free`], or null when the bus is not
/// simulated, holds no part at the address, or is null.
///
/// # Safety
///
/// `bus` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_part(
    bus: *const PamojaI2cBus,
    address: u8,
) -> *mut PamojaI2cPart {
    if bus.is_null() {
        return std::ptr::null_mut();
    }
    match (*bus).bus.part::<Part>(address) {
        Some(part) => PamojaI2cPart::into_raw(part),
        None => std::ptr::null_mut(),
    }
}

/// Releases the caller's share of a bus. The bus closes when no driver holds it either. A
/// null pointer is ignored.
///
/// # Safety
///
/// `bus` must be a handle that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_bus_free(bus: *mut PamojaI2cBus) {
    if !bus.is_null() {
        drop(Box::from_raw(bus));
    }
}

/// Adds a step built by `build` to a script, turning a null script into a status.
///
/// # Safety
///
/// `script` must be a live handle or null.
unsafe fn add_step(
    script: *mut PamojaI2cScript,
    build: impl FnOnce() -> Result<I2cStep, PamojaStatus>,
) -> PamojaStatus {
    if script.is_null() {
        set_last_error("script must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match build() {
        Ok(step) => {
            (*script).steps.push(step);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// A part handle as a part whose registers are a byte wide, or the status that refuses it.
///
/// # Safety
///
/// `part` must be a live handle or null.
unsafe fn byte_part<'a>(part: *mut PamojaI2cPart) -> Result<&'a mut I2cPart, PamojaStatus> {
    match part.as_mut().map(|part| &mut part.part) {
        Some(Part::Bytes(part)) => Ok(part),
        other => Err(wrong_kind(other.is_some(), "registers a byte wide")),
    }
}

/// A part handle as a part whose registers are sixteen bits wide, or the status that refuses
/// it.
///
/// # Safety
///
/// `part` must be a live handle or null.
unsafe fn word_part<'a>(part: *mut PamojaI2cPart) -> Result<&'a mut WordPart, PamojaStatus> {
    match part.as_mut().map(|part| &mut part.part) {
        Some(Part::Words(part)) => Ok(part),
        other => Err(wrong_kind(other.is_some(), "registers sixteen bits wide")),
    }
}

/// A part handle as a part that takes commands, or the status that refuses it.
///
/// # Safety
///
/// `part` must be a live handle or null.
unsafe fn command_part<'a>(part: *mut PamojaI2cPart) -> Result<&'a mut CommandPart, PamojaStatus> {
    match part.as_mut().map(|part| &mut part.part) {
        Some(Part::Commands(part)) => Ok(part),
        other => Err(wrong_kind(other.is_some(), "commands")),
    }
}

/// Records why a part handle was refused: it was null, or it held another kind of part.
fn wrong_kind(present: bool, wanted: &str) -> PamojaStatus {
    if present {
        set_last_error(format!("only a part with {wanted} takes that call"));
    } else {
        set_last_error("part must not be null".to_owned());
    }
    PamojaStatus::InvalidArgument
}

/// The failure a fault code names.
fn fault_kind(code: u8) -> Option<ErrorKind> {
    Some(match code {
        PAMOJA_I2C_FAULT_NO_ACKNOWLEDGE_ADDRESS => {
            ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)
        }
        PAMOJA_I2C_FAULT_NO_ACKNOWLEDGE_DATA => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data),
        PAMOJA_I2C_FAULT_NO_ACKNOWLEDGE => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown),
        PAMOJA_I2C_FAULT_BUS => ErrorKind::Bus,
        PAMOJA_I2C_FAULT_ARBITRATION_LOSS => ErrorKind::ArbitrationLoss,
        PAMOJA_I2C_FAULT_OVERRUN => ErrorKind::Overrun,
        PAMOJA_I2C_FAULT_OTHER => ErrorKind::Other,
        _ => return None,
    })
}

/// A caller's buffer as a slice, or `None` after recording why it cannot be one.
///
/// # Safety
///
/// `out` must point to `len` writable bytes, or be null.
unsafe fn out_bytes<'a>(out: *mut u8, len: usize) -> Option<&'a mut [u8]> {
    if len == 0 {
        return Some(&mut []);
    }
    if out.is_null() {
        set_last_error("out must not be null".to_owned());
        return None;
    }
    Some(std::slice::from_raw_parts_mut(out, len))
}

/// Runs a transfer on a bus, turning a refusal or a panic into a status.
///
/// # Safety
///
/// `bus` must be a live handle or null.
unsafe fn on_bus(
    bus: *const PamojaI2cBus,
    transfer: impl FnOnce(&mut I2cBus) -> Result<(), pamoja_hal::bus::BusError>,
) -> PamojaStatus {
    if bus.is_null() {
        set_last_error("bus must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let mut bus = (*bus).bus.clone();
    match catch_unwind(AssertUnwindSafe(|| transfer(&mut bus))) {
        Ok(Ok(())) => PamojaStatus::Ok,
        Ok(Err(error)) => {
            set_last_error(error.to_string());
            PamojaStatus::Io
        }
        Err(_) => panicked(),
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
    use std::ffi::{CStr, CString};
    use std::ptr;

    fn last_error() -> String {
        let message = crate::pamoja_last_error_message();
        if message.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn a_part_on_a_simulated_bus_answers_and_keeps_what_was_written() {
        unsafe {
            let part = pamoja_i2c_part_new(0x76);
            assert_eq!(
                pamoja_i2c_part_load(part, 0xd0, [0x60].as_ptr(), 1),
                PamojaStatus::Ok
            );
            let bus = pamoja_i2c_bus_simulated();
            assert_eq!(pamoja_i2c_bus_attach(bus, part), PamojaStatus::Ok);
            pamoja_i2c_part_free(part);

            let mut id = [0u8; 1];
            assert_eq!(
                pamoja_i2c_bus_write_read(bus, 0x76, [0xd0].as_ptr(), 1, id.as_mut_ptr(), 1),
                PamojaStatus::Ok
            );
            assert_eq!(id, [0x60]);
            assert_eq!(
                pamoja_i2c_bus_write(bus, 0x76, [0xf4, 0x25].as_ptr(), 2),
                PamojaStatus::Ok
            );

            let held = pamoja_i2c_bus_part(bus, 0x76);
            assert!(!held.is_null());
            assert_eq!(pamoja_i2c_part_register(held, 0xf4), 0x25);
            let mut block = [0u8; 2];
            assert_eq!(
                pamoja_i2c_part_read(held, 0xd0, block.as_mut_ptr(), 2),
                PamojaStatus::Ok
            );
            assert_eq!(block, [0x60, 0x00]);
            pamoja_i2c_part_free(held);

            assert_eq!(pamoja_i2c_bus_kind(bus), PAMOJA_I2C_BUS_SIMULATED);
            assert_eq!(pamoja_i2c_bus_transfers(bus), 2);
            let mut remaining = 7;
            assert!(!pamoja_i2c_bus_remaining(bus, &mut remaining));
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn nothing_at_an_address_is_an_io_error_that_says_so() {
        unsafe {
            let bus = pamoja_i2c_bus_simulated();
            let mut byte = [0u8; 1];
            assert_eq!(
                pamoja_i2c_bus_write_read(bus, 0x77, [0xd0].as_ptr(), 1, byte.as_mut_ptr(), 1),
                PamojaStatus::Io
            );
            assert_eq!(last_error(), "nothing answered at 0x77");
            assert!(pamoja_i2c_bus_part(bus, 0x77).is_null());
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn a_script_counts_down_and_refuses_what_it_did_not_expect() {
        unsafe {
            let script = pamoja_i2c_script_new();
            assert_eq!(
                pamoja_i2c_script_write(script, 0x76, [0xe0, 0xb6].as_ptr(), 2),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_i2c_script_write_read(script, 0x76, [0xd0].as_ptr(), 1, [0x60].as_ptr(), 1),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_i2c_script_fault(script, 0x76, PAMOJA_I2C_FAULT_BUS),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_i2c_script_fault(script, 0x76, 42),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(pamoja_i2c_script_len(script), 3);

            let bus = pamoja_i2c_bus_scripted(script);
            pamoja_i2c_script_free(script);
            assert_eq!(pamoja_i2c_bus_kind(bus), PAMOJA_I2C_BUS_SCRIPTED);

            let mut remaining = 0;
            assert!(pamoja_i2c_bus_remaining(bus, &mut remaining));
            assert_eq!(remaining, 3);

            assert_eq!(
                pamoja_i2c_bus_write(bus, 0x76, [0xe0, 0xb7].as_ptr(), 2),
                PamojaStatus::Io
            );
            assert!(
                last_error().starts_with("i2c script step 0"),
                "{}",
                last_error()
            );
            assert_eq!(
                pamoja_i2c_bus_write(bus, 0x76, [0xe0, 0xb6].as_ptr(), 2),
                PamojaStatus::Ok
            );
            let mut id = [0u8; 1];
            assert_eq!(
                pamoja_i2c_bus_write_read(bus, 0x76, [0xd0].as_ptr(), 1, id.as_mut_ptr(), 1),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_i2c_bus_read(bus, 0x76, id.as_mut_ptr(), 1),
                PamojaStatus::Io
            );
            assert!(
                last_error().starts_with("i2c script fault"),
                "{}",
                last_error()
            );
            assert!(pamoja_i2c_bus_remaining(bus, &mut remaining));
            assert_eq!(remaining, 0);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn only_a_simulated_bus_takes_a_part() {
        unsafe {
            let script = pamoja_i2c_script_new();
            let bus = pamoja_i2c_bus_scripted(script);
            let part = pamoja_i2c_part_new(0x76);
            assert_eq!(
                pamoja_i2c_bus_attach(bus, part),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(last_error(), "only a simulated bus takes parts");
            pamoja_i2c_part_free(part);
            pamoja_i2c_bus_free(bus);
            pamoja_i2c_script_free(script);
            assert!(pamoja_i2c_bus_scripted(ptr::null()).is_null());
        }
    }

    #[test]
    fn word_and_command_parts_answer_in_their_own_shape() {
        unsafe {
            let words = pamoja_i2c_word_part_new(0x48);
            assert_eq!(pamoja_i2c_part_kind(words), PAMOJA_I2C_PART_WORDS);
            assert_eq!(
                pamoja_i2c_part_set_word(words, 0x01, 0x2000),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_i2c_part_read_only(words, 0x01, 0xF000),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_i2c_part_load(words, 0x00, [0x01].as_ptr(), 1),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                last_error(),
                "only a part with registers a byte wide takes that call"
            );

            let commands = pamoja_i2c_command_part_new(0x44, 2);
            assert_eq!(pamoja_i2c_part_kind(commands), PAMOJA_I2C_PART_COMMANDS);
            assert_eq!(
                pamoja_i2c_part_answer(
                    commands,
                    [0xF3, 0x2D].as_ptr(),
                    2,
                    [0x80, 0x10, 0xE1].as_ptr(),
                    3
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_i2c_part_set_word(commands, 0x00, 0),
                PamojaStatus::InvalidArgument
            );

            let bus = pamoja_i2c_bus_simulated();
            assert_eq!(pamoja_i2c_bus_attach(bus, words), PamojaStatus::Ok);
            assert_eq!(pamoja_i2c_bus_attach(bus, commands), PamojaStatus::Ok);
            pamoja_i2c_part_free(words);
            pamoja_i2c_part_free(commands);

            assert_eq!(
                pamoja_i2c_bus_write(bus, 0x48, [0x01, 0x00, 0x20].as_ptr(), 3),
                PamojaStatus::Ok
            );
            let mut word = [0u8; 2];
            assert_eq!(
                pamoja_i2c_bus_write_read(bus, 0x48, [0x01].as_ptr(), 1, word.as_mut_ptr(), 2),
                PamojaStatus::Ok
            );
            assert_eq!(
                word,
                [0x20, 0x20],
                "the flag the part keeps, then the write"
            );

            let mut status = [0u8; 3];
            assert_eq!(
                pamoja_i2c_bus_write(bus, 0x44, [0xF3, 0x2D].as_ptr(), 2),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_i2c_bus_read(bus, 0x44, status.as_mut_ptr(), 3),
                PamojaStatus::Ok
            );
            assert_eq!(status, [0x80, 0x10, 0xE1]);
            assert_eq!(
                pamoja_i2c_bus_read(bus, 0x44, status.as_mut_ptr(), 3),
                PamojaStatus::Io,
                "the reply was taken"
            );

            let held = pamoja_i2c_bus_part(bus, 0x48);
            assert_eq!(pamoja_i2c_part_kind(held), PAMOJA_I2C_PART_WORDS);
            assert_eq!(pamoja_i2c_part_word(held, 0x01), 0x2020);
            assert_eq!(pamoja_i2c_part_register(held, 0x01), 0);
            pamoja_i2c_part_free(held);

            let held = pamoja_i2c_bus_part(bus, 0x44);
            assert_eq!(pamoja_i2c_part_received_count(held), 1);
            let write = pamoja_i2c_part_received(held, 0);
            assert_eq!(crate::pamoja_buffer_len(write), 2);
            assert_eq!(*crate::pamoja_buffer_data(write), 0xF3);
            crate::pamoja_buffer_free(write);
            assert!(pamoja_i2c_part_received(held, 1).is_null());
            pamoja_i2c_part_free(held);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn opening_an_adapter_off_linux_is_unsupported() {
        let path = CString::new("/dev/i2c-1").expect("no interior null");
        let mut bus = ptr::null_mut();
        let opened = unsafe { pamoja_i2c_bus_open(path.as_ptr(), &mut bus) };
        assert_eq!(opened, PamojaStatus::Unsupported);
        assert!(bus.is_null());
        assert!(last_error().contains("only Linux"), "{}", last_error());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn opening_a_missing_adapter_is_an_io_error_that_names_it() {
        let path = CString::new("/dev/i2c-pamoja-absent").expect("no interior null");
        let mut bus = ptr::null_mut();
        let opened = unsafe { pamoja_i2c_bus_open(path.as_ptr(), &mut bus) };
        assert_eq!(opened, PamojaStatus::Io);
        assert!(bus.is_null());
        assert!(
            last_error().starts_with("/dev/i2c-pamoja-absent: "),
            "{}",
            last_error()
        );
    }
}
