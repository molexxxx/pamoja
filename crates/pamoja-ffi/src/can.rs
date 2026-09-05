//! The C ABI for CAN bus framing.
//!
//! These functions wrap [`pamoja_can`] for callers that reach the SDK through the
//! flat C boundary: classic CAN 2.0 and CAN-FD frames, the length encoding CAN-FD
//! uses above eight bytes, and the J1939 identifier that trucks, tractors, and
//! gensets ride on top of it.
//!
//! A frame carries a payload of up to 64 bytes, so it crosses as an opaque handle
//! like every other payload-bearing type here. A J1939 identifier is only scalars,
//! so it crosses by value as [`PamojaJ1939Id`], which keeps decoding an identifier
//! free of any allocation.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use pamoja_can::{
    dlc_to_len, len_to_dlc, priority, CanError, CanId, Frame, J1939Id, Signals, BROADCAST_ADDRESS,
    NOT_AVAILABLE,
};

use crate::{read_bytes, set_last_error, PamojaStatus};

/// An opaque handle to a CAN frame.
///
/// Read it with the `pamoja_can_frame_*` calls, then release it with
/// [`pamoja_can_frame_free`].
pub struct PamojaCanFrame {
    frame: Frame,
}

/// The fields J1939 packs into an extended CAN identifier.
///
/// Every field is a scalar, so this crosses the boundary by value. `addressed` is
/// `1` for a PDU1 message, where `destination` names the node the message is for,
/// and `0` for a PDU2 broadcast, where `destination` carries no meaning.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaJ1939Id {
    /// The parameter group number, which names what the message carries.
    pub pgn: u32,
    /// The message priority, 0 (highest) to 7.
    pub priority: u8,
    /// The source address: the node that sent the message.
    pub source: u8,
    /// The PDU format byte of the parameter group.
    pub pdu_format: u8,
    /// The destination address, meaningful only when `addressed` is `1`.
    pub destination: u8,
    /// `1` for an addressed (PDU1) message, `0` for a broadcast (PDU2) one.
    pub addressed: u8,
}

/// The eight data bytes of a J1939 frame, addressed by the signals inside them.
///
/// A parameter group places each signal at a fixed byte offset, little-endian. The
/// payload is only bytes, so it crosses the boundary by value, which keeps reading
/// and writing a signal free of any allocation.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaJ1939Signals {
    /// The eight data bytes, in wire order.
    pub bytes: [u8; 8],
}

/// The byte a J1939 sender writes for a signal it is not reporting.
pub const PAMOJA_J1939_NOT_AVAILABLE: u8 = 0xFF;

/// The destination address every node on the bus reads.
pub const PAMOJA_J1939_BROADCAST_ADDRESS: u8 = 0xFF;

/// The priority a control message takes, ahead of ordinary traffic.
pub const PAMOJA_J1939_PRIORITY_CONTROL: u8 = 3;

/// The priority ordinary traffic takes.
pub const PAMOJA_J1939_PRIORITY_DEFAULT: u8 = 6;

/// The priority that yields to everything else on the bus.
pub const PAMOJA_J1939_PRIORITY_LOWEST: u8 = 7;

// The header generator does not read the crates this one depends on, so these
// carry their value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_J1939_NOT_AVAILABLE == NOT_AVAILABLE);
const _: () = assert!(PAMOJA_J1939_BROADCAST_ADDRESS == BROADCAST_ADDRESS);
const _: () = assert!(PAMOJA_J1939_PRIORITY_CONTROL == priority::CONTROL);
const _: () = assert!(PAMOJA_J1939_PRIORITY_DEFAULT == priority::DEFAULT);
const _: () = assert!(PAMOJA_J1939_PRIORITY_LOWEST == priority::LOWEST);

/// Builds a classic CAN 2.0 frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a new handle the
/// caller must release with [`pamoja_can_frame_free`], or
/// [`PamojaStatus::InvalidArgument`] if the data is longer than the eight bytes a
/// classic frame carries.
///
/// # Safety
///
/// `data` must point to at least `data_len` readable bytes, or be null when
/// `data_len` is 0, and `out_frame` must point to a writable
/// `*mut PamojaCanFrame`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_new(
    id: u32,
    extended: bool,
    data: *const u8,
    data_len: usize,
    out_frame: *mut *mut PamojaCanFrame,
) -> PamojaStatus {
    build(id, extended, data, data_len, out_frame, Frame::new)
}

/// Builds a CAN-FD frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a new handle the
/// caller must release with [`pamoja_can_frame_free`], or
/// [`PamojaStatus::InvalidArgument`] if the data is longer than 64 bytes or is
/// not one of the discrete lengths CAN-FD can carry.
///
/// # Safety
///
/// `data` must point to at least `data_len` readable bytes, or be null when
/// `data_len` is 0, and `out_frame` must point to a writable
/// `*mut PamojaCanFrame`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_fd(
    id: u32,
    extended: bool,
    data: *const u8,
    data_len: usize,
    out_frame: *mut *mut PamojaCanFrame,
) -> PamojaStatus {
    build(id, extended, data, data_len, out_frame, Frame::fd)
}

/// Builds a remote transmission request, which asks another node to send.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a new handle the
/// caller must release with [`pamoja_can_frame_free`].
///
/// # Safety
///
/// `out_frame` must point to a writable `*mut PamojaCanFrame`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_remote(
    id: u32,
    extended: bool,
    len: usize,
    out_frame: *mut *mut PamojaCanFrame,
) -> PamojaStatus {
    let out_frame = match out_slot(out_frame, "out_frame") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| {
        Frame::remote(identifier(id, extended), len)
    })) {
        Ok(frame) => {
            *out_frame = Box::into_raw(Box::new(PamojaCanFrame { frame }));
            PamojaStatus::Ok
        }
        Err(_) => panicked(),
    }
}

/// Returns a frame's identifier, already masked to 11 or 29 bits.
///
/// # Returns
///
/// The identifier, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from one of the frame constructors, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_id(frame: *const PamojaCanFrame) -> u32 {
    if frame.is_null() {
        return 0;
    }
    (*frame).frame.id().raw()
}

/// Reports whether a frame carries a 29-bit extended identifier.
///
/// # Returns
///
/// `true` for an extended identifier, `false` for a standard one or a null frame.
///
/// # Safety
///
/// `frame` must be a live handle from one of the frame constructors, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_is_extended(frame: *const PamojaCanFrame) -> bool {
    !frame.is_null() && (*frame).frame.id().is_extended()
}

/// Reports whether a frame is CAN-FD rather than classic CAN 2.0.
///
/// # Returns
///
/// `true` for a CAN-FD frame, `false` otherwise or for a null frame.
///
/// # Safety
///
/// `frame` must be a live handle from one of the frame constructors, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_is_fd(frame: *const PamojaCanFrame) -> bool {
    !frame.is_null() && (*frame).frame.is_fd()
}

/// Reports whether a frame is a remote transmission request.
///
/// # Returns
///
/// `true` for a remote frame, `false` otherwise or for a null frame.
///
/// # Safety
///
/// `frame` must be a live handle from one of the frame constructors, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_is_remote(frame: *const PamojaCanFrame) -> bool {
    !frame.is_null() && (*frame).frame.is_remote()
}

/// Returns how many payload bytes a frame carries.
///
/// A remote frame reports the length it requests while carrying no data.
///
/// # Returns
///
/// The length, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from one of the frame constructors, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_len(frame: *const PamojaCanFrame) -> usize {
    if frame.is_null() {
        return 0;
    }
    (*frame).frame.len()
}

/// Returns a frame's data length code, the length as it appears on the wire.
///
/// # Returns
///
/// The code, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from one of the frame constructors, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_dlc(frame: *const PamojaCanFrame) -> u8 {
    if frame.is_null() {
        return 0;
    }
    (*frame).frame.dlc()
}

/// Returns a pointer to a frame's payload bytes.
///
/// Use [`pamoja_can_frame_data_len`] for the length, not
/// [`pamoja_can_frame_len`]: a remote frame reports the length it requests while
/// carrying no payload at all.
///
/// # Returns
///
/// A pointer to the payload, or null if `frame` is null or carries no bytes.
///
/// # Safety
///
/// `frame` must be a live handle from one of the frame constructors, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_data(frame: *const PamojaCanFrame) -> *const u8 {
    if frame.is_null() {
        return ptr::null();
    }
    let data = (*frame).frame.data();
    if data.is_empty() {
        ptr::null()
    } else {
        data.as_ptr()
    }
}

/// Returns how many bytes [`pamoja_can_frame_data`] points to.
///
/// This is the frame's length for an ordinary frame and 0 for a remote one,
/// which requests a length without carrying the bytes.
///
/// # Returns
///
/// The payload length, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from one of the frame constructors, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_data_len(frame: *const PamojaCanFrame) -> usize {
    if frame.is_null() {
        return 0;
    }
    (*frame).frame.data().len()
}

/// Releases a frame handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `frame` must be a handle from one of the frame constructors that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_frame_free(frame: *mut PamojaCanFrame) {
    if !frame.is_null() {
        drop(Box::from_raw(frame));
    }
}

/// Returns the data length code that encodes a payload length.
///
/// # Returns
///
/// The code for `len`, rounding up to the next length CAN-FD can carry.
#[no_mangle]
pub extern "C" fn pamoja_can_len_to_dlc(len: usize) -> u8 {
    len_to_dlc(len)
}

/// Returns the payload length a data length code encodes.
///
/// # Returns
///
/// The length in bytes.
#[no_mangle]
pub extern "C" fn pamoja_can_dlc_to_len(dlc: u8) -> usize {
    dlc_to_len(dlc)
}

/// Decodes the J1939 fields out of an extended CAN identifier.
///
/// # Returns
///
/// `true` when `extended` is set, with `*out_message` filled in; `false` for a
/// standard 11-bit identifier, which J1939 does not use, leaving `*out_message`
/// untouched.
///
/// # Safety
///
/// `out_message` must point to a writable `PamojaJ1939Id`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_j1939_decode(
    id: u32,
    extended: bool,
    out_message: *mut PamojaJ1939Id,
) -> bool {
    if out_message.is_null() {
        set_last_error("out_message must not be null".to_owned());
        return false;
    }
    let Some(message) = J1939Id::from_id(identifier(id, extended)) else {
        return false;
    };
    *out_message = PamojaJ1939Id {
        pgn: message.pgn(),
        priority: message.priority(),
        source: message.source(),
        pdu_format: message.pdu_format(),
        destination: message.destination().unwrap_or(0),
        addressed: u8::from(!message.is_broadcast()),
    };
    true
}

/// Composes the extended CAN identifier a set of J1939 fields describes.
///
/// # Returns
///
/// The 29-bit identifier. `destination` is used only for an addressed (PDU1)
/// parameter group and ignored for a broadcast (PDU2) one.
#[no_mangle]
pub extern "C" fn pamoja_can_j1939_compose(
    priority: u8,
    pgn: u32,
    source: u8,
    destination: u8,
) -> u32 {
    J1939Id::from_parts(priority, pgn, source, destination)
        .to_id()
        .raw()
}

/// Composes the identifier of a J1939 broadcast, which every node on the bus reads.
///
/// # Returns
///
/// The 29-bit identifier. Most parameter groups are broadcast, so this is the
/// common case, and it saves a caller knowing the broadcast address.
#[no_mangle]
pub extern "C" fn pamoja_can_j1939_broadcast(priority: u8, pgn: u32, source: u8) -> u32 {
    J1939Id::broadcast(priority, pgn, source).to_id().raw()
}

/// Builds a J1939 payload with every signal marked not available.
///
/// # Returns
///
/// Eight bytes of [`PAMOJA_J1939_NOT_AVAILABLE`], ready for a sender to write only
/// the signals it has.
#[no_mangle]
pub extern "C" fn pamoja_can_signals_new() -> PamojaJ1939Signals {
    PamojaJ1939Signals {
        bytes: *Signals::new().as_bytes(),
    }
}

/// Writes a one-byte signal at the offset its parameter group defines.
///
/// # Returns
///
/// The payload with the signal written, or unchanged if `at` is past its end.
#[no_mangle]
pub extern "C" fn pamoja_can_signals_set_u8(
    signals: PamojaJ1939Signals,
    at: usize,
    value: u8,
) -> PamojaJ1939Signals {
    let mut payload = Signals::from_bytes(signals.bytes);
    payload.set_u8(at, value);
    PamojaJ1939Signals {
        bytes: *payload.as_bytes(),
    }
}

/// Writes a two-byte little-endian signal at the offset its group defines.
///
/// # Returns
///
/// The payload with the signal written, or unchanged if the signal would run past
/// its end.
#[no_mangle]
pub extern "C" fn pamoja_can_signals_set_u16(
    signals: PamojaJ1939Signals,
    at: usize,
    value: u16,
) -> PamojaJ1939Signals {
    let mut payload = Signals::from_bytes(signals.bytes);
    payload.set_u16(at, value);
    PamojaJ1939Signals {
        bytes: *payload.as_bytes(),
    }
}

/// Reads a one-byte signal at the offset its parameter group defines.
///
/// # Returns
///
/// `true` with `*out_value` set, or `false` if `at` is past the payload.
///
/// # Safety
///
/// `out_value` must point to a writable `uint8_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_signals_u8(
    signals: PamojaJ1939Signals,
    at: usize,
    out_value: *mut u8,
) -> bool {
    if out_value.is_null() {
        return false;
    }
    match Signals::from_bytes(signals.bytes).u8(at) {
        Some(value) => {
            ptr::write(out_value, value);
            true
        }
        None => false,
    }
}

/// Reads a two-byte little-endian signal at the offset its group defines.
///
/// # Returns
///
/// `true` with `*out_value` set, or `false` if the signal would run past the
/// payload.
///
/// # Safety
///
/// `out_value` must point to a writable `uint16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_signals_u16(
    signals: PamojaJ1939Signals,
    at: usize,
    out_value: *mut u16,
) -> bool {
    if out_value.is_null() {
        return false;
    }
    match Signals::from_bytes(signals.bytes).u16(at) {
        Some(value) => {
            ptr::write(out_value, value);
            true
        }
        None => false,
    }
}

/// Builds a frame with one of the constructors and hands back a handle.
///
/// # Safety
///
/// `data` must point to at least `data_len` readable bytes, or be null when
/// `data_len` is 0, and `out_frame` must point to a writable
/// `*mut PamojaCanFrame`.
unsafe fn build(
    id: u32,
    extended: bool,
    data: *const u8,
    data_len: usize,
    out_frame: *mut *mut PamojaCanFrame,
    construct: fn(CanId, &[u8]) -> Result<Frame, CanError>,
) -> PamojaStatus {
    let out_frame = match out_slot(out_frame, "out_frame") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    let data = match read_bytes(data, data_len) {
        Ok(data) => data,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| {
        construct(identifier(id, extended), &data)
    })) {
        Ok(Ok(frame)) => {
            *out_frame = Box::into_raw(Box::new(PamojaCanFrame { frame }));
            PamojaStatus::Ok
        }
        Ok(Err(error)) => failed(error),
        Err(_) => panicked(),
    }
}

/// Builds an identifier of the requested width, masking the value to fit it.
fn identifier(id: u32, extended: bool) -> CanId {
    if extended {
        CanId::extended(id)
    } else {
        CanId::standard(id as u16)
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

/// Records a framing error and maps it onto its status.
fn failed(error: CanError) -> PamojaStatus {
    set_last_error(error.to_string());
    PamojaStatus::InvalidArgument
}

/// Records a caught panic and reports it as [`PamojaStatus::Panic`].
fn panicked() -> PamojaStatus {
    set_last_error("panic at the FFI boundary".to_owned());
    PamojaStatus::Panic
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zeroed identifier, so a test can tell a written field from an untouched one.
    fn blank() -> PamojaJ1939Id {
        PamojaJ1939Id {
            pgn: 0,
            priority: 0,
            source: 0,
            pdu_format: 0,
            destination: 0,
            addressed: 0,
        }
    }

    #[test]
    fn a_classic_frame_carries_its_payload() {
        let data = [0x01u8, 0xF4];
        let mut frame = ptr::null_mut();

        // Safety: the input is a valid slice and the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_can_frame_new(0x20A, false, data.as_ptr(), data.len(), &mut frame),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_can_frame_id(frame), 0x20A);
            assert!(!pamoja_can_frame_is_extended(frame));
            assert!(!pamoja_can_frame_is_fd(frame));
            assert!(!pamoja_can_frame_is_remote(frame));
            assert_eq!(pamoja_can_frame_len(frame), 2);
            assert_eq!(pamoja_can_frame_dlc(frame), 2);
            let payload = std::slice::from_raw_parts(
                pamoja_can_frame_data(frame),
                pamoja_can_frame_data_len(frame),
            );
            assert_eq!(payload, data);
            pamoja_can_frame_free(frame);
        }
    }

    #[test]
    fn a_classic_frame_refuses_more_than_eight_bytes() {
        let data = [0u8; 9];
        let mut frame = ptr::null_mut();
        // Safety: the input is a valid slice and the out-pointer is writable.
        let status =
            unsafe { pamoja_can_frame_new(0x20A, false, data.as_ptr(), data.len(), &mut frame) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
        assert!(frame.is_null());
    }

    #[test]
    fn a_fd_frame_takes_a_length_classic_can_cannot() {
        let data = [0u8; 32];
        let mut frame = ptr::null_mut();

        // Safety: the input is a valid slice and the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_can_frame_fd(0x1234_5678, true, data.as_ptr(), data.len(), &mut frame),
                PamojaStatus::Ok
            );
            assert!(pamoja_can_frame_is_fd(frame));
            assert!(pamoja_can_frame_is_extended(frame));
            assert_eq!(pamoja_can_frame_len(frame), 32);
            assert_eq!(pamoja_can_frame_dlc(frame), 13);
            pamoja_can_frame_free(frame);
        }
    }

    #[test]
    fn a_length_between_the_fd_steps_is_refused() {
        let data = [0u8; 13];
        let mut frame = ptr::null_mut();
        // Safety: the input is a valid slice and the out-pointer is writable.
        let status =
            unsafe { pamoja_can_frame_fd(0x100, true, data.as_ptr(), data.len(), &mut frame) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
        assert!(frame.is_null());
    }

    #[test]
    fn a_remote_frame_asks_for_a_length_it_does_not_carry() {
        let mut frame = ptr::null_mut();
        // Safety: the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_can_frame_remote(0x20A, false, 4, &mut frame),
                PamojaStatus::Ok
            );
            assert!(pamoja_can_frame_is_remote(frame));
            assert_eq!(pamoja_can_frame_len(frame), 4, "the length it asks for");
            assert_eq!(
                pamoja_can_frame_data_len(frame),
                0,
                "a remote frame carries no bytes to read"
            );
            assert!(pamoja_can_frame_data(frame).is_null());
            pamoja_can_frame_free(frame);
        }
    }

    #[test]
    fn an_engine_broadcast_decodes_to_its_parameter_group() {
        let mut message = blank();
        // Safety: the out-pointer is writable.
        let decoded = unsafe { pamoja_can_j1939_decode(0x0CF0_0400, true, &mut message) };
        assert!(decoded);
        assert_eq!(message.pgn, 61_444, "electronic engine controller 1");
        assert_eq!(message.priority, 3);
        assert_eq!(message.source, 0x00);
        assert_eq!(message.addressed, 0, "a PDU2 message is a broadcast");
    }

    #[test]
    fn a_standard_identifier_is_not_a_j1939_message() {
        let mut message = blank();
        // Safety: the out-pointer is writable.
        let decoded = unsafe { pamoja_can_j1939_decode(0x123, false, &mut message) };
        assert!(!decoded, "J1939 never rides an 11-bit identifier");
        assert_eq!(message, blank(), "a refused decode writes nothing");
    }

    #[test]
    fn an_addressed_message_round_trips_through_its_identifier() {
        // A request PGN (0x0EA00) sent from 0x21 to 0x0A at priority 6.
        let id = pamoja_can_j1939_compose(6, 0x0EA00, 0x21, 0x0A);
        let mut message = blank();
        // Safety: the out-pointer is writable.
        assert!(unsafe { pamoja_can_j1939_decode(id, true, &mut message) });
        assert_eq!(message.priority, 6);
        assert_eq!(message.pgn, 0x0EA00);
        assert_eq!(message.source, 0x21);
        assert_eq!(message.addressed, 1);
        assert_eq!(message.destination, 0x0A);
    }

    #[test]
    fn the_length_encoding_round_trips() {
        for len in [0usize, 8, 12, 16, 20, 24, 32, 48, 64] {
            assert_eq!(pamoja_can_dlc_to_len(pamoja_can_len_to_dlc(len)), len);
        }
    }

    #[test]
    fn a_payload_starts_with_every_signal_not_available() {
        let payload = pamoja_can_signals_new();
        assert_eq!(payload.bytes, [PAMOJA_J1939_NOT_AVAILABLE; 8]);
    }

    #[test]
    fn a_signal_reads_back_from_where_it_was_written() {
        // Engine speed sits at byte offset three of EEC1, at 0.125 rpm per bit.
        let payload = pamoja_can_signals_set_u16(pamoja_can_signals_new(), 3, 8_000);
        let mut speed = 0u16;
        // Safety: the out-pointer is writable.
        assert!(unsafe { pamoja_can_signals_u16(payload, 3, &mut speed) });
        assert_eq!(speed, 8_000);

        let mut untouched = 0u8;
        // Safety: the out-pointer is writable.
        assert!(unsafe { pamoja_can_signals_u8(payload, 0, &mut untouched) });
        assert_eq!(untouched, PAMOJA_J1939_NOT_AVAILABLE);
    }

    #[test]
    fn a_signal_past_the_payload_is_refused_rather_than_wrapped() {
        let payload = pamoja_can_signals_set_u8(pamoja_can_signals_new(), 8, 1);
        assert_eq!(payload.bytes, [PAMOJA_J1939_NOT_AVAILABLE; 8]);

        let mut value = 0u16;
        // Safety: the out-pointer is writable.
        assert!(!unsafe { pamoja_can_signals_u16(payload, 7, &mut value) });
    }

    #[test]
    fn a_broadcast_carries_no_destination() {
        let id = pamoja_can_j1939_broadcast(PAMOJA_J1939_PRIORITY_CONTROL, 61_444, 0);
        let mut message = blank();
        // Safety: the out-pointer is writable.
        assert!(unsafe { pamoja_can_j1939_decode(id, true, &mut message) });
        assert_eq!(message.priority, PAMOJA_J1939_PRIORITY_CONTROL);
        assert_eq!(message.addressed, 0);
    }
}
