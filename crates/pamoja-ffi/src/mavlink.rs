//! The C ABI for the MAVLink wire protocol.
//!
//! MAVLink is the language drones speak, so talking to a PX4 or ArduPilot
//! autopilot means putting exactly the right bytes on the wire and trusting the
//! bytes that come back. This is that byte layer: assembling and parsing v1 and
//! v2 frames, the CRC-16/MCRF4XX checksum every frame carries, the per-message
//! `CRC_EXTRA` seed that catches a frame whose shape does not match what the
//! receiver expects, and MAVLink 2 signing.
//!
//! # Any dialect, not only the common one
//!
//! A receiver has to know a message's `CRC_EXTRA` before it can check the frame
//! carrying it. The common dialect's seeds are built in, but a vehicle running a
//! vendor or private dialect uses ids this build has never heard of. Two things
//! keep those reachable: [`pamoja_mavlink_message_crc_extra`] derives a seed from
//! a message definition the way the specification does, and a
//! [`PamojaMavlinkDialect`] table carries the results, taking precedence over the
//! built-in registry. Nothing here is limited to the ids this crate happens to
//! type.

use pamoja_mavlink::dialect::{crc_extra as common_crc_extra, RawMessage};
use pamoja_mavlink::{
    crc16_mcrf4xx, message_crc_extra, signing, Frame, Header, MavlinkError, Parser, Signer,
    Verifier, Version, MAX_PAYLOAD, SIGNATURE_LEN,
};

use crate::{read_bytes, read_str, set_last_error, PamojaStatus};

/// The start marker of a v1 frame.
pub const PAMOJA_MAVLINK_MAGIC_V1: u8 = pamoja_mavlink::MAGIC_V1;
/// The start marker of a v2 frame.
pub const PAMOJA_MAVLINK_MAGIC_V2: u8 = pamoja_mavlink::MAGIC_V2;
/// The incompatibility flag that marks a v2 frame as signed.
pub const PAMOJA_MAVLINK_IFLAG_SIGNED: u8 = pamoja_mavlink::IFLAG_SIGNED;
/// The largest payload a frame can carry, in bytes.
pub const PAMOJA_MAVLINK_MAX_PAYLOAD: usize = MAX_PAYLOAD;
/// The largest frame, in bytes, header, checksum and signature included.
pub const PAMOJA_MAVLINK_MAX_FRAME: usize = pamoja_mavlink::MAX_FRAME;
/// The length of a v2 signature block, in bytes.
pub const PAMOJA_MAVLINK_SIGNATURE_LEN: usize = SIGNATURE_LEN;
/// The length of a signing key, in bytes.
pub const PAMOJA_MAVLINK_KEY_LEN: usize = signing::KEY_LEN;
/// The default window a verifier accepts a timestamp within, in microseconds.
pub const PAMOJA_MAVLINK_DEFAULT_TIMESTAMP_WINDOW: u64 = signing::DEFAULT_TIMESTAMP_WINDOW;
/// The Unix time MAVLink counts signing timestamps from, in seconds.
pub const PAMOJA_MAVLINK_EPOCH_OFFSET_SECS: u64 = signing::MAVLINK_EPOCH_OFFSET_SECS;

/// The original wire format, with a six-byte header.
pub const PAMOJA_MAVLINK_VERSION_V1: u8 = 1;
/// The current wire format, with a 24-bit message id, flags, and optional signing.
pub const PAMOJA_MAVLINK_VERSION_V2: u8 = 2;

/// The addressing fields a sender stamps on every frame.
///
/// A frame says who sent it, a system and a component, and where it sits in that
/// sender's stream, so a receiver can tell a dropped frame from a quiet link.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaMavlinkHeader {
    /// The sending system's id.
    pub system_id: u8,
    /// The sending component's id.
    pub component_id: u8,
    /// The sender's sequence number, which wraps at 256.
    pub sequence: u8,
}

impl From<PamojaMavlinkHeader> for Header {
    fn from(header: PamojaMavlinkHeader) -> Self {
        Header::new(header.system_id, header.component_id, header.sequence)
    }
}

/// One field of a message definition, as the `CRC_EXTRA` derivation reads it.
///
/// The seed folds in each field's type name and field name in wire order, plus
/// the element count for an array field, which is what makes it catch a peer
/// whose idea of the message shape differs.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PamojaMavlinkField {
    /// The field's type name as the dialect writes it, such as `uint8_t`.
    pub type_name: *const std::os::raw::c_char,
    /// The field's name as the dialect writes it, such as `custom_mode`.
    pub field_name: *const std::os::raw::c_char,
    /// The element count for an array field, or `0` for a scalar.
    pub array_len: u8,
}

/// Maps a MAVLink error onto the matching status code.
///
/// # Arguments
///
/// * `error` - the error the wire layer returned.
///
/// # Returns
///
/// The status, with the message left in the last-error slot.
fn status_of(error: MavlinkError) -> PamojaStatus {
    set_last_error(error.to_string());
    match error {
        MavlinkError::FrameTooShort
        | MavlinkError::BadMagic(_)
        | MavlinkError::Truncated
        | MavlinkError::CrcMismatch { .. }
        | MavlinkError::UnknownMessage(_)
        | MavlinkError::BadPayload => PamojaStatus::Codec,
        MavlinkError::PayloadTooLong => PamojaStatus::InvalidArgument,
        MavlinkError::Unsigned | MavlinkError::BadSignature | MavlinkError::ReplayedTimestamp => {
            PamojaStatus::Auth
        }
        MavlinkError::Closed => PamojaStatus::Closed,
        _ => PamojaStatus::Other,
    }
}

/// Returns the CRC-16/MCRF4XX checksum of a byte string.
///
/// This is the checksum every MAVLink frame carries, exposed because a host that
/// implements part of the protocol itself still needs the same arithmetic.
///
/// # Arguments
///
/// * `bytes` - the data to checksum.
/// * `bytes_len` - how many bytes `bytes` holds.
///
/// # Returns
///
/// The checksum, or `0` if `bytes` is null with a non-zero length.
///
/// # Safety
///
/// When `bytes_len` is non-zero, `bytes` must point to at least `bytes_len`
/// readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_crc16_mcrf4xx(bytes: *const u8, bytes_len: usize) -> u16 {
    match read_bytes(bytes, bytes_len) {
        Ok(bytes) => crc16_mcrf4xx(&bytes),
        Err(_) => 0,
    }
}

/// Derives the `CRC_EXTRA` seed of a message from its definition.
///
/// This is what makes a dialect this build has never seen usable: given a
/// message's name and its fields in wire order, the seed comes out the same as
/// the one the dialect publishes, and a frame carrying that message then checks
/// like any other.
///
/// # Arguments
///
/// * `name` - the message name, such as `HEARTBEAT`.
/// * `fields` - the base fields in wire order; extension fields are excluded from
///   the seed and must not be listed.
/// * `field_count` - how many fields `fields` holds.
/// * `out_crc_extra` - set to the seed on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if any pointer is null or any name
/// is not valid UTF-8.
///
/// # Safety
///
/// `name` must be a valid null-terminated string, `fields` must point to
/// `field_count` readable entries whose own pointers are valid null-terminated
/// strings, and `out_crc_extra` must point at writable storage for one byte.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_crc_extra(
    name: *const std::os::raw::c_char,
    fields: *const PamojaMavlinkField,
    field_count: usize,
    out_crc_extra: *mut u8,
) -> PamojaStatus {
    if out_crc_extra.is_null() || (field_count != 0 && fields.is_null()) {
        set_last_error("fields and out_crc_extra must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(name) = read_str(name, "name") else {
        return PamojaStatus::InvalidArgument;
    };

    let described = std::slice::from_raw_parts(fields, field_count);
    let mut parsed: Vec<(&str, &str, u8)> = Vec::with_capacity(field_count);
    for field in described {
        let (Some(type_name), Some(field_name)) = (
            read_str(field.type_name, "a field type"),
            read_str(field.field_name, "a field name"),
        ) else {
            return PamojaStatus::InvalidArgument;
        };
        parsed.push((type_name, field_name, field.array_len));
    }

    *out_crc_extra = message_crc_extra(name, &parsed);
    PamojaStatus::Ok
}

/// Returns the `CRC_EXTRA` the common dialect publishes for a message id.
///
/// # Arguments
///
/// * `msgid` - the message id to look up.
/// * `out_crc_extra` - set to the seed on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_crc_extra` is null, and
/// [`PamojaStatus::Unsupported`] if the id is outside the common dialect, which
/// is what a [`PamojaMavlinkDialect`] table is for.
///
/// # Safety
///
/// `out_crc_extra` must point at writable storage for one byte.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_known_crc_extra(
    msgid: u32,
    out_crc_extra: *mut u8,
) -> PamojaStatus {
    if out_crc_extra.is_null() {
        set_last_error("out_crc_extra must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(crc) = common_crc_extra(msgid) else {
        set_last_error(format!("message {msgid} is not in the common dialect"));
        return PamojaStatus::Unsupported;
    };
    *out_crc_extra = crc;
    PamojaStatus::Ok
}

/// The `CRC_EXTRA` seeds of a dialect beyond the common one.
///
/// A handle the caller must release with [`pamoja_mavlink_dialect_free`].
/// Entries added here are consulted before the built-in common-dialect registry,
/// so a private dialect may also override an id the common one defines.
pub struct PamojaMavlinkDialect {
    seeds: Vec<(u32, u8)>,
}

impl PamojaMavlinkDialect {
    /// Returns the seed for a message id, preferring this table.
    ///
    /// # Arguments
    ///
    /// * `msgid` - the message id to look up.
    ///
    /// # Returns
    ///
    /// The seed, or `None` if neither this table nor the common dialect has one.
    fn crc_extra(&self, msgid: u32) -> Option<u8> {
        self.seeds
            .iter()
            .find(|(id, _)| *id == msgid)
            .map(|(_, crc)| *crc)
            .or_else(|| common_crc_extra(msgid))
    }
}

/// Looks a message id up in an optional dialect table, then the common dialect.
///
/// # Arguments
///
/// * `dialect` - the table to prefer, or null for the common dialect alone.
/// * `msgid` - the message id to look up.
///
/// # Returns
///
/// The seed, or `None` if neither has one.
///
/// # Safety
///
/// `dialect` must be a live dialect handle, or null.
unsafe fn lookup(dialect: *const PamojaMavlinkDialect, msgid: u32) -> Option<u8> {
    match dialect.as_ref() {
        Some(dialect) => dialect.crc_extra(msgid),
        None => common_crc_extra(msgid),
    }
}

/// Creates an empty dialect table.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_mavlink_dialect_free`].
#[no_mangle]
pub extern "C" fn pamoja_mavlink_dialect_new() -> *mut PamojaMavlinkDialect {
    Box::into_raw(Box::new(PamojaMavlinkDialect { seeds: Vec::new() }))
}

/// Adds or replaces the `CRC_EXTRA` seed for a message id.
///
/// # Arguments
///
/// * `dialect` - the table to extend.
/// * `msgid` - the message id.
/// * `crc_extra` - the seed, usually from
///   [`pamoja_mavlink_message_crc_extra`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `dialect` is null.
///
/// # Safety
///
/// `dialect` must be a live dialect handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_dialect_add(
    dialect: *mut PamojaMavlinkDialect,
    msgid: u32,
    crc_extra: u8,
) -> PamojaStatus {
    let Some(dialect) = dialect.as_mut() else {
        set_last_error("dialect must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match dialect.seeds.iter_mut().find(|(id, _)| *id == msgid) {
        Some(entry) => entry.1 = crc_extra,
        None => dialect.seeds.push((msgid, crc_extra)),
    }
    PamojaStatus::Ok
}

/// Returns the seed a dialect resolves a message id to.
///
/// # Arguments
///
/// * `dialect` - the table to search, or null for the common dialect alone.
/// * `msgid` - the message id to look up.
/// * `out_crc_extra` - set to the seed on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_crc_extra` is null, and
/// [`PamojaStatus::Unsupported`] if neither the table nor the common dialect
/// knows the id.
///
/// # Safety
///
/// `dialect` must be a live dialect handle or null, and `out_crc_extra` must
/// point at writable storage for one byte.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_dialect_crc_extra(
    dialect: *const PamojaMavlinkDialect,
    msgid: u32,
    out_crc_extra: *mut u8,
) -> PamojaStatus {
    if out_crc_extra.is_null() {
        set_last_error("out_crc_extra must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(crc) = lookup(dialect, msgid) else {
        set_last_error(format!("no dialect here defines message {msgid}"));
        return PamojaStatus::Unsupported;
    };
    *out_crc_extra = crc;
    PamojaStatus::Ok
}

/// Releases a dialect table.
///
/// # Arguments
///
/// * `dialect` - the handle to release; null is ignored.
///
/// # Safety
///
/// `dialect` must have come from [`pamoja_mavlink_dialect_new`] and must not be
/// used afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_dialect_free(dialect: *mut PamojaMavlinkDialect) {
    if !dialect.is_null() {
        drop(Box::from_raw(dialect));
    }
}

/// One MAVLink frame, assembled or received.
///
/// A handle the caller must release with [`pamoja_mavlink_frame_free`].
pub struct PamojaMavlinkFrame {
    inner: Frame,
}

impl PamojaMavlinkFrame {
    /// Moves a frame onto the heap and hands the caller its handle.
    fn into_handle(inner: Frame) -> *mut Self {
        Box::into_raw(Box::new(Self { inner }))
    }
}

/// Assembles a frame carrying a message.
///
/// # Arguments
///
/// * `version` - [`PAMOJA_MAVLINK_VERSION_V1`] or [`PAMOJA_MAVLINK_VERSION_V2`].
/// * `header` - the addressing fields to stamp on the frame.
/// * `msgid` - the message id; a v1 frame only carries ids below 256.
/// * `payload` - the message payload.
/// * `payload_len` - how many bytes `payload` holds.
/// * `crc_extra` - the seed for this message id.
/// * `out_frame` - set to the frame handle on success, and to null otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_frame` is null, the version
/// is neither constant, or the payload does not fit a frame.
///
/// # Safety
///
/// `payload` must point to `payload_len` readable bytes when the length is
/// non-zero, and `out_frame` must point at writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_encode(
    version: u8,
    header: PamojaMavlinkHeader,
    msgid: u32,
    payload: *const u8,
    payload_len: usize,
    crc_extra: u8,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_frame;
    *slot = std::ptr::null_mut();

    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };

    let built = match version {
        PAMOJA_MAVLINK_VERSION_V1 => Frame::encode_v1(header.into(), msgid, &payload, crc_extra),
        PAMOJA_MAVLINK_VERSION_V2 => Frame::encode_v2(header.into(), msgid, &payload, crc_extra),
        other => {
            set_last_error(format!("{other} is not a MAVLink version"));
            return PamojaStatus::InvalidArgument;
        }
    };
    match built {
        Ok(frame) => {
            *slot = PamojaMavlinkFrame::into_handle(frame);
            PamojaStatus::Ok
        }
        Err(error) => status_of(error),
    }
}

/// Parses one frame, checking it against a known `CRC_EXTRA`.
///
/// # Arguments
///
/// * `bytes` - the frame as received.
/// * `bytes_len` - how many bytes `bytes` holds.
/// * `crc_extra` - the seed for the message the frame carries.
/// * `out_frame` - set to the frame handle on success, and to null otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_frame` is null, and
/// [`PamojaStatus::Codec`] if the bytes are not a whole frame or the checksum
/// does not match, which is what rejects a frame mangled in transit.
///
/// # Safety
///
/// `bytes` must point to `bytes_len` readable bytes when the length is non-zero,
/// and `out_frame` must point at writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_parse(
    bytes: *const u8,
    bytes_len: usize,
    crc_extra: u8,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_frame;
    *slot = std::ptr::null_mut();

    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match Frame::parse(&bytes, crc_extra) {
        Ok(frame) => {
            *slot = PamojaMavlinkFrame::into_handle(frame);
            PamojaStatus::Ok
        }
        Err(error) => status_of(error),
    }
}

/// Parses one frame, looking its `CRC_EXTRA` up as it goes.
///
/// This is what a receiver holding many message types uses: the id comes out of
/// the frame, and the seed comes from the dialect table or the common registry.
///
/// # Arguments
///
/// * `bytes` - the frame as received.
/// * `bytes_len` - how many bytes `bytes` holds.
/// * `dialect` - the dialect to prefer, or null for the common one alone.
/// * `out_frame` - set to the frame handle on success, and to null otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_frame` is null, and
/// [`PamojaStatus::Codec`] if the bytes are not a whole frame, the checksum does
/// not match, or no dialect here knows the message id.
///
/// # Safety
///
/// `bytes` must point to `bytes_len` readable bytes when the length is non-zero,
/// `dialect` must be a live dialect handle or null, and `out_frame` must point at
/// writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_parse_known(
    bytes: *const u8,
    bytes_len: usize,
    dialect: *const PamojaMavlinkDialect,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_frame;
    *slot = std::ptr::null_mut();

    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match Frame::parse_with(&bytes, |msgid| lookup(dialect, msgid)) {
        Ok(frame) => {
            *slot = PamojaMavlinkFrame::into_handle(frame);
            PamojaStatus::Ok
        }
        Err(error) => status_of(error),
    }
}

/// Releases a frame.
///
/// # Arguments
///
/// * `frame` - the handle to release; null is ignored.
///
/// # Safety
///
/// `frame` must have come from one of the frame constructors and must not be used
/// afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_free(frame: *mut PamojaMavlinkFrame) {
    if !frame.is_null() {
        drop(Box::from_raw(frame));
    }
}

/// Returns the wire format a frame uses.
///
/// # Arguments
///
/// * `frame` - the frame to read.
///
/// # Returns
///
/// [`PAMOJA_MAVLINK_VERSION_V1`] or [`PAMOJA_MAVLINK_VERSION_V2`], or `0` if
/// `frame` is null.
///
/// # Safety
///
/// `frame` must be a live frame handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_version(frame: *const PamojaMavlinkFrame) -> u8 {
    match frame.as_ref() {
        Some(frame) => match frame.inner.version() {
            Version::V1 => PAMOJA_MAVLINK_VERSION_V1,
            Version::V2 => PAMOJA_MAVLINK_VERSION_V2,
        },
        None => 0,
    }
}

/// Returns the addressing fields a frame carries.
///
/// # Arguments
///
/// * `frame` - the frame to read.
/// * `out_header` - set to the header on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `frame` must be a live frame handle and `out_header` must point at writable
/// storage for one [`PamojaMavlinkHeader`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_header(
    frame: *const PamojaMavlinkFrame,
    out_header: *mut PamojaMavlinkHeader,
) -> PamojaStatus {
    let (Some(frame), false) = (frame.as_ref(), out_header.is_null()) else {
        set_last_error("frame and out_header must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_header = PamojaMavlinkHeader {
        system_id: frame.inner.system_id(),
        component_id: frame.inner.component_id(),
        sequence: frame.inner.sequence(),
    };
    PamojaStatus::Ok
}

/// Returns the id of the message a frame carries.
///
/// # Arguments
///
/// * `frame` - the frame to read.
///
/// # Returns
///
/// The message id, or `0` if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live frame handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_message_id(frame: *const PamojaMavlinkFrame) -> u32 {
    frame.as_ref().map_or(0, |frame| frame.inner.message_id())
}

/// Returns the incompatibility flags a v2 frame declares.
///
/// # Arguments
///
/// * `frame` - the frame to read.
///
/// # Returns
///
/// The flags, or `0` for a v1 frame or a null handle.
///
/// # Safety
///
/// `frame` must be a live frame handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_incompat_flags(
    frame: *const PamojaMavlinkFrame,
) -> u8 {
    frame
        .as_ref()
        .map_or(0, |frame| frame.inner.incompat_flags())
}

/// Reports whether a frame carries a signature.
///
/// A signature only says the frame was signed, not that the signature is good;
/// [`pamoja_mavlink_verifier_verify`] decides that.
///
/// # Arguments
///
/// * `frame` - the frame to read.
///
/// # Returns
///
/// `1` if the frame is signed, `0` otherwise.
///
/// # Safety
///
/// `frame` must be a live frame handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_is_signed(frame: *const PamojaMavlinkFrame) -> u8 {
    frame
        .as_ref()
        .map_or(0, |frame| u8::from(frame.inner.is_signed()))
}

/// Returns a pointer to a frame's payload.
///
/// The pointer is valid until the frame is released.
///
/// # Arguments
///
/// * `frame` - the frame to read.
/// * `out_len` - set to the payload length in bytes.
///
/// # Returns
///
/// A pointer to the payload, or null if either argument is null.
///
/// # Safety
///
/// `frame` must be a live frame handle and `out_len` must point at writable
/// storage for one length.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_payload(
    frame: *const PamojaMavlinkFrame,
    out_len: *mut usize,
) -> *const u8 {
    let (Some(frame), false) = (frame.as_ref(), out_len.is_null()) else {
        set_last_error("frame and out_len must not be null".to_owned());
        return std::ptr::null();
    };
    let payload = frame.inner.payload();
    *out_len = payload.len();
    payload.as_ptr()
}

/// Returns a pointer to a frame's bytes, ready to put on the wire.
///
/// The pointer is valid until the frame is released.
///
/// # Arguments
///
/// * `frame` - the frame to read.
/// * `out_len` - set to the frame length in bytes.
///
/// # Returns
///
/// A pointer to the frame, or null if either argument is null.
///
/// # Safety
///
/// `frame` must be a live frame handle and `out_len` must point at writable
/// storage for one length.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_bytes(
    frame: *const PamojaMavlinkFrame,
    out_len: *mut usize,
) -> *const u8 {
    let (Some(frame), false) = (frame.as_ref(), out_len.is_null()) else {
        set_last_error("frame and out_len must not be null".to_owned());
        return std::ptr::null();
    };
    let bytes = frame.inner.as_bytes();
    *out_len = bytes.len();
    bytes.as_ptr()
}

/// Copies a frame's signature block out.
///
/// # Arguments
///
/// * `frame` - the frame to read.
/// * `out_signature` - filled with [`PAMOJA_MAVLINK_SIGNATURE_LEN`] bytes on
///   success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, and
/// [`PamojaStatus::Unsupported`] if the frame carries no signature.
///
/// # Safety
///
/// `frame` must be a live frame handle and `out_signature` must point at writable
/// storage for [`PAMOJA_MAVLINK_SIGNATURE_LEN`] bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_frame_signature(
    frame: *const PamojaMavlinkFrame,
    out_signature: *mut u8,
) -> PamojaStatus {
    let (Some(frame), false) = (frame.as_ref(), out_signature.is_null()) else {
        set_last_error("frame and out_signature must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(signature) = frame.inner.signature() else {
        set_last_error("this frame is not signed".to_owned());
        return PamojaStatus::Unsupported;
    };
    std::ptr::copy_nonoverlapping(signature.as_ptr(), out_signature, SIGNATURE_LEN);
    PamojaStatus::Ok
}

/// A streaming frame parser, and the frames it has completed.
///
/// A handle the caller must release with [`pamoja_mavlink_parser_free`].
pub struct PamojaMavlinkParser {
    parser: Parser,
    ready: std::collections::VecDeque<Frame>,
}

/// Creates a parser with an empty buffer.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_mavlink_parser_free`].
#[no_mangle]
pub extern "C" fn pamoja_mavlink_parser_new() -> *mut PamojaMavlinkParser {
    Box::into_raw(Box::new(PamojaMavlinkParser {
        parser: Parser::new(),
        ready: std::collections::VecDeque::new(),
    }))
}

/// Feeds bytes off a link into the parser.
///
/// Whatever a serial port or socket delivers can be pushed as it arrives, however
/// it is split. Frames that complete are queued for
/// [`pamoja_mavlink_parser_next`]. Noise between frames is skipped rather than
/// reported, which is what lets a parser join a stream already in progress.
///
/// # Arguments
///
/// * `parser` - the parser to feed.
/// * `bytes` - the bytes just read off the link.
/// * `bytes_len` - how many bytes `bytes` holds.
/// * `dialect` - the dialect to prefer, or null for the common one alone.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `parser` is null.
///
/// # Safety
///
/// `parser` must be a live parser handle, `bytes` must point to `bytes_len`
/// readable bytes when the length is non-zero, and `dialect` must be a live
/// dialect handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_parser_push(
    parser: *mut PamojaMavlinkParser,
    bytes: *const u8,
    bytes_len: usize,
    dialect: *const PamojaMavlinkDialect,
) -> PamojaStatus {
    let Some(parser) = parser.as_mut() else {
        set_last_error("parser must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let lookup = |msgid: u32| lookup(dialect, msgid);
    for byte in bytes {
        if let Some(frame) = parser.parser.push_byte(byte, &lookup) {
            parser.ready.push_back(frame);
        }
    }
    PamojaStatus::Ok
}

/// Takes the next completed frame out of the parser.
///
/// # Arguments
///
/// * `parser` - the parser to drain.
/// * `out_frame` - set to the frame handle, or to null when none is waiting.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] whether or not a frame was waiting; a null `out_frame`
/// means the parser needs more bytes.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `parser` must be a live parser handle and `out_frame` must point at writable
/// storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_parser_next(
    parser: *mut PamojaMavlinkParser,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_frame;
    *slot = std::ptr::null_mut();

    let Some(parser) = parser.as_mut() else {
        set_last_error("parser must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if let Some(frame) = parser.ready.pop_front() {
        *slot = PamojaMavlinkFrame::into_handle(frame);
    }
    PamojaStatus::Ok
}

/// Returns how many completed frames are waiting to be taken.
///
/// # Arguments
///
/// * `parser` - the parser to inspect.
///
/// # Returns
///
/// The number of frames waiting, or `0` if `parser` is null.
///
/// # Safety
///
/// `parser` must be a live parser handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_parser_pending(
    parser: *const PamojaMavlinkParser,
) -> usize {
    parser.as_ref().map_or(0, |parser| parser.ready.len())
}

/// Releases a parser.
///
/// # Arguments
///
/// * `parser` - the handle to release; null is ignored.
///
/// # Safety
///
/// `parser` must have come from [`pamoja_mavlink_parser_new`] and must not be
/// used afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_parser_free(parser: *mut PamojaMavlinkParser) {
    if !parser.is_null() {
        drop(Box::from_raw(parser));
    }
}

/// Converts Unix time into the timestamp MAVLink signing counts in.
///
/// # Arguments
///
/// * `unix_micros` - the time in microseconds since the Unix epoch.
///
/// # Returns
///
/// The MAVLink signing timestamp, in units of ten microseconds since 2015.
#[no_mangle]
pub extern "C" fn pamoja_mavlink_timestamp_from_unix_micros(unix_micros: u64) -> u64 {
    signing::timestamp_from_unix_micros(unix_micros)
}

/// A signing key and the monotonic timestamp that goes with it.
///
/// A handle the caller must release with [`pamoja_mavlink_signer_free`].
pub struct PamojaMavlinkSigner {
    inner: Signer,
}

/// Creates a signer.
///
/// # Arguments
///
/// * `key` - the shared signing key, [`PAMOJA_MAVLINK_KEY_LEN`] bytes.
/// * `link_id` - which link this sender signs on, so two links from one system
///   do not look like replays of each other.
/// * `timestamp` - the timestamp to start from, usually from
///   [`pamoja_mavlink_timestamp_from_unix_micros`].
/// * `out_signer` - set to the signer handle on success, and to null otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `key` must point to [`PAMOJA_MAVLINK_KEY_LEN`] readable bytes and
/// `out_signer` must point at writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_signer_new(
    key: *const u8,
    link_id: u8,
    timestamp: u64,
    out_signer: *mut *mut PamojaMavlinkSigner,
) -> PamojaStatus {
    if out_signer.is_null() || key.is_null() {
        set_last_error("key and out_signer must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_signer;
    let mut bytes = [0u8; signing::KEY_LEN];
    std::ptr::copy_nonoverlapping(key, bytes.as_mut_ptr(), signing::KEY_LEN);
    *slot = Box::into_raw(Box::new(PamojaMavlinkSigner {
        inner: Signer::new(bytes, link_id, timestamp),
    }));
    PamojaStatus::Ok
}

/// Signs a message into a v2 frame.
///
/// Each call advances the signer's timestamp, which is what makes a replayed
/// frame detectable.
///
/// # Arguments
///
/// * `signer` - the signer to use.
/// * `header` - the addressing fields to stamp on the frame.
/// * `msgid` - the message id.
/// * `payload` - the message payload.
/// * `payload_len` - how many bytes `payload` holds.
/// * `crc_extra` - the seed for this message id.
/// * `out_frame` - set to the signed frame on success, and to null otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `signer` or `out_frame` is null,
/// or the payload does not fit a frame.
///
/// # Safety
///
/// `signer` must be a live signer handle, `payload` must point to `payload_len`
/// readable bytes when the length is non-zero, and `out_frame` must point at
/// writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_signer_sign(
    signer: *mut PamojaMavlinkSigner,
    header: PamojaMavlinkHeader,
    msgid: u32,
    payload: *const u8,
    payload_len: usize,
    crc_extra: u8,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_frame;
    *slot = std::ptr::null_mut();

    let Some(signer) = signer.as_mut() else {
        set_last_error("signer must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    match signer.inner.sign(header.into(), msgid, &payload, crc_extra) {
        Ok(frame) => {
            *slot = PamojaMavlinkFrame::into_handle(frame);
            PamojaStatus::Ok
        }
        Err(error) => status_of(error),
    }
}

/// Returns the link a signer signs on.
///
/// # Arguments
///
/// * `signer` - the signer to read.
///
/// # Returns
///
/// The link id, or `0` if `signer` is null.
///
/// # Safety
///
/// `signer` must be a live signer handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_signer_link_id(signer: *const PamojaMavlinkSigner) -> u8 {
    signer.as_ref().map_or(0, |signer| signer.inner.link_id())
}

/// Releases a signer.
///
/// # Arguments
///
/// * `signer` - the handle to release; null is ignored.
///
/// # Safety
///
/// `signer` must have come from [`pamoja_mavlink_signer_new`] and must not be
/// used afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_signer_free(signer: *mut PamojaMavlinkSigner) {
    if !signer.is_null() {
        drop(Box::from_raw(signer));
    }
}

/// A signing key and the timestamps it has already accepted.
///
/// A handle the caller must release with [`pamoja_mavlink_verifier_free`].
pub struct PamojaMavlinkVerifier {
    inner: Verifier,
}

/// Creates a verifier.
///
/// # Arguments
///
/// * `key` - the shared signing key, [`PAMOJA_MAVLINK_KEY_LEN`] bytes.
/// * `out_verifier` - set to the handle on success, and to null otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `key` must point to [`PAMOJA_MAVLINK_KEY_LEN`] readable bytes and
/// `out_verifier` must point at writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_verifier_new(
    key: *const u8,
    out_verifier: *mut *mut PamojaMavlinkVerifier,
) -> PamojaStatus {
    if out_verifier.is_null() || key.is_null() {
        set_last_error("key and out_verifier must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_verifier;
    let mut bytes = [0u8; signing::KEY_LEN];
    std::ptr::copy_nonoverlapping(key, bytes.as_mut_ptr(), signing::KEY_LEN);
    *slot = Box::into_raw(Box::new(PamojaMavlinkVerifier {
        inner: Verifier::new(bytes),
    }));
    PamojaStatus::Ok
}

/// Sets how far a timestamp may run ahead of the last one accepted.
///
/// A wider window tolerates a noisier link; a narrower one narrows the chance of
/// a replay landing inside it.
///
/// # Arguments
///
/// * `verifier` - the verifier to set.
/// * `window` - the window in timestamp units, ten microseconds each.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `verifier` is null.
///
/// # Safety
///
/// `verifier` must be a live verifier handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_verifier_set_window(
    verifier: *mut PamojaMavlinkVerifier,
    window: u64,
) -> PamojaStatus {
    let Some(verifier) = verifier.as_mut() else {
        set_last_error("verifier must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let held = std::mem::replace(&mut verifier.inner, Verifier::new([0u8; signing::KEY_LEN]));
    verifier.inner = held.with_window(window);
    PamojaStatus::Ok
}

/// Checks a frame's signature and its place in the timestamp sequence.
///
/// # Arguments
///
/// * `verifier` - the verifier to use.
/// * `frame` - the frame to check.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] if the frame is authentic and not a replay.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, and
/// [`PamojaStatus::Auth`] if the frame is unsigned, the signature does not match
/// the key, or the timestamp has been seen before.
///
/// # Safety
///
/// `verifier` must be a live verifier handle and `frame` must be a live frame
/// handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_verifier_verify(
    verifier: *mut PamojaMavlinkVerifier,
    frame: *const PamojaMavlinkFrame,
) -> PamojaStatus {
    let (Some(verifier), Some(frame)) = (verifier.as_mut(), frame.as_ref()) else {
        set_last_error("verifier and frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match verifier.inner.verify(&frame.inner) {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => status_of(error),
    }
}

/// Releases a verifier.
///
/// # Arguments
///
/// * `verifier` - the handle to release; null is ignored.
///
/// # Safety
///
/// `verifier` must have come from [`pamoja_mavlink_verifier_new`] and must not be
/// used afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_verifier_free(verifier: *mut PamojaMavlinkVerifier) {
    if !verifier.is_null() {
        drop(Box::from_raw(verifier));
    }
}

/// Assembles a v2 frame carrying a message this build does not type.
///
/// This is the escape hatch a private dialect needs: supply the id, the payload,
/// and the seed, and the frame is built and checked like any other.
///
/// # Arguments
///
/// * `header` - the addressing fields to stamp on the frame.
/// * `msgid` - the message id.
/// * `crc_extra` - the seed for this message id.
/// * `payload` - the message payload.
/// * `payload_len` - how many bytes `payload` holds.
/// * `out_frame` - set to the frame handle on success, and to null otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_frame` is null or the
/// payload does not fit a frame.
///
/// # Safety
///
/// `payload` must point to `payload_len` readable bytes when the length is
/// non-zero, and `out_frame` must point at writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_raw_message_to_frame(
    header: PamojaMavlinkHeader,
    msgid: u32,
    crc_extra: u8,
    payload: *const u8,
    payload_len: usize,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_frame;
    *slot = std::ptr::null_mut();

    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    let raw = RawMessage {
        msgid,
        crc_extra,
        payload: &payload,
    };
    match raw.to_frame(header.into()) {
        Ok(frame) => {
            *slot = PamojaMavlinkFrame::into_handle(frame);
            PamojaStatus::Ok
        }
        Err(error) => status_of(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;

    /// The HEARTBEAT payload an onboard controller announces itself with.
    const HEARTBEAT: [u8; 9] = [0, 0, 0, 0, 18, 0, 0, 4, 3];

    #[test]
    fn a_frame_round_trips_through_the_boundary() {
        unsafe {
            let header = PamojaMavlinkHeader {
                system_id: 1,
                component_id: 1,
                sequence: 7,
            };
            let mut frame = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_frame_encode(
                    PAMOJA_MAVLINK_VERSION_V2,
                    header,
                    0,
                    HEARTBEAT.as_ptr(),
                    HEARTBEAT.len(),
                    50,
                    &mut frame
                ),
                PamojaStatus::Ok
            );

            let mut len = 0;
            let bytes = pamoja_mavlink_frame_bytes(frame, &mut len);
            let wire = std::slice::from_raw_parts(bytes, len).to_vec();

            let mut received = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_frame_parse(wire.as_ptr(), wire.len(), 50, &mut received),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_mavlink_frame_message_id(received), 0);
            assert_eq!(pamoja_mavlink_frame_version(received), 2);
            assert_eq!(pamoja_mavlink_frame_is_signed(received), 0);

            let mut got = PamojaMavlinkHeader {
                system_id: 0,
                component_id: 0,
                sequence: 0,
            };
            assert_eq!(
                pamoja_mavlink_frame_header(received, &mut got),
                PamojaStatus::Ok
            );
            assert_eq!(got, header);

            pamoja_mavlink_frame_free(frame);
            pamoja_mavlink_frame_free(received);
        }
    }

    #[test]
    fn a_frame_mangled_in_transit_is_refused() {
        unsafe {
            let header = PamojaMavlinkHeader {
                system_id: 1,
                component_id: 1,
                sequence: 0,
            };
            let mut frame = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_frame_encode(
                    PAMOJA_MAVLINK_VERSION_V2,
                    header,
                    0,
                    HEARTBEAT.as_ptr(),
                    HEARTBEAT.len(),
                    50,
                    &mut frame
                ),
                PamojaStatus::Ok
            );
            let mut len = 0;
            let bytes = pamoja_mavlink_frame_bytes(frame, &mut len);
            let mut wire = std::slice::from_raw_parts(bytes, len).to_vec();
            pamoja_mavlink_frame_free(frame);

            wire[12] ^= 0xFF;
            let mut received = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_frame_parse(wire.as_ptr(), wire.len(), 50, &mut received),
                PamojaStatus::Codec
            );
            assert!(received.is_null());
        }
    }

    #[test]
    fn the_parser_finds_frames_split_across_reads_and_skips_noise() {
        unsafe {
            let header = PamojaMavlinkHeader {
                system_id: 2,
                component_id: 1,
                sequence: 3,
            };
            let mut frame = ptr::null_mut();
            pamoja_mavlink_frame_encode(
                PAMOJA_MAVLINK_VERSION_V2,
                header,
                0,
                HEARTBEAT.as_ptr(),
                HEARTBEAT.len(),
                50,
                &mut frame,
            );
            let mut len = 0;
            let bytes = pamoja_mavlink_frame_bytes(frame, &mut len);
            let wire = std::slice::from_raw_parts(bytes, len).to_vec();
            pamoja_mavlink_frame_free(frame);

            let parser = pamoja_mavlink_parser_new();

            // Noise, then the frame split mid-payload, the way a serial port
            // actually delivers it.
            let noise = [0x11u8, 0x22, 0x33];
            pamoja_mavlink_parser_push(parser, noise.as_ptr(), noise.len(), ptr::null());
            pamoja_mavlink_parser_push(parser, wire.as_ptr(), 5, ptr::null());
            assert_eq!(pamoja_mavlink_parser_pending(parser), 0);
            pamoja_mavlink_parser_push(parser, wire.as_ptr().add(5), wire.len() - 5, ptr::null());
            assert_eq!(pamoja_mavlink_parser_pending(parser), 1);

            let mut received = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_parser_next(parser, &mut received),
                PamojaStatus::Ok
            );
            assert!(!received.is_null());
            assert_eq!(pamoja_mavlink_frame_message_id(received), 0);
            pamoja_mavlink_frame_free(received);

            // Draining an empty parser is not an error; it means feed it more.
            let mut empty = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_parser_next(parser, &mut empty),
                PamojaStatus::Ok
            );
            assert!(empty.is_null());

            pamoja_mavlink_parser_free(parser);
        }
    }

    #[test]
    fn a_private_dialect_is_parsed_once_its_seed_is_known() {
        unsafe {
            // An id no common dialect defines, with a seed derived from its own
            // definition the way the specification does.
            let name = CString::new("PRIVATE_STATUS").expect("name");
            let type_name = CString::new("uint32_t").expect("type");
            let field_name = CString::new("uptime").expect("field");
            let fields = [PamojaMavlinkField {
                type_name: type_name.as_ptr(),
                field_name: field_name.as_ptr(),
                array_len: 0,
            }];
            let mut seed = 0;
            assert_eq!(
                pamoja_mavlink_message_crc_extra(name.as_ptr(), fields.as_ptr(), 1, &mut seed),
                PamojaStatus::Ok
            );

            let header = PamojaMavlinkHeader {
                system_id: 9,
                component_id: 1,
                sequence: 0,
            };
            let payload = 42u32.to_le_bytes();
            let mut frame = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_raw_message_to_frame(
                    header,
                    50_000,
                    seed,
                    payload.as_ptr(),
                    payload.len(),
                    &mut frame
                ),
                PamojaStatus::Ok
            );
            let mut len = 0;
            let bytes = pamoja_mavlink_frame_bytes(frame, &mut len);
            let wire = std::slice::from_raw_parts(bytes, len).to_vec();
            pamoja_mavlink_frame_free(frame);

            // The common registry alone cannot check it.
            let mut refused = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_frame_parse_known(
                    wire.as_ptr(),
                    wire.len(),
                    ptr::null(),
                    &mut refused
                ),
                PamojaStatus::Codec
            );
            assert!(refused.is_null());

            // Told the seed, it parses like any other frame.
            let dialect = pamoja_mavlink_dialect_new();
            assert_eq!(
                pamoja_mavlink_dialect_add(dialect, 50_000, seed),
                PamojaStatus::Ok
            );
            let mut received = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_frame_parse_known(wire.as_ptr(), wire.len(), dialect, &mut received),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_mavlink_frame_message_id(received), 50_000);

            // MAVLink 2 drops trailing zero bytes, so a four-byte field whose
            // value fits in one arrives as one byte; a decoder zero-extends it.
            let mut payload_len = 0;
            let payload = pamoja_mavlink_frame_payload(received, &mut payload_len);
            assert_eq!(std::slice::from_raw_parts(payload, payload_len), [42]);

            pamoja_mavlink_frame_free(received);
            pamoja_mavlink_dialect_free(dialect);
        }
    }

    #[test]
    fn the_common_registry_answers_for_the_ids_it_knows() {
        unsafe {
            let mut seed = 0;
            assert_eq!(
                pamoja_mavlink_known_crc_extra(0, &mut seed),
                PamojaStatus::Ok
            );
            assert_eq!(seed, 50, "HEARTBEAT");
            assert_eq!(
                pamoja_mavlink_known_crc_extra(9999, &mut seed),
                PamojaStatus::Unsupported
            );
        }
    }

    #[test]
    fn a_signed_frame_verifies_once_and_a_replay_is_refused() {
        unsafe {
            let key = [7u8; PAMOJA_MAVLINK_KEY_LEN];
            let mut signer = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_signer_new(key.as_ptr(), 1, 1_000, &mut signer),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_mavlink_signer_link_id(signer), 1);

            let header = PamojaMavlinkHeader {
                system_id: 1,
                component_id: 1,
                sequence: 0,
            };
            let mut frame = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_signer_sign(
                    signer,
                    header,
                    0,
                    HEARTBEAT.as_ptr(),
                    HEARTBEAT.len(),
                    50,
                    &mut frame
                ),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_mavlink_frame_is_signed(frame), 1);

            let mut signature = [0u8; PAMOJA_MAVLINK_SIGNATURE_LEN];
            assert_eq!(
                pamoja_mavlink_frame_signature(frame, signature.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(signature[0], 1, "the link id leads the block");

            let mut verifier = ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_verifier_new(key.as_ptr(), &mut verifier),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_mavlink_verifier_verify(verifier, frame),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_mavlink_verifier_verify(verifier, frame),
                PamojaStatus::Auth,
                "the same timestamp a second time is a replay"
            );

            // A different key is a different sender.
            let mut stranger = ptr::null_mut();
            pamoja_mavlink_verifier_new([9u8; PAMOJA_MAVLINK_KEY_LEN].as_ptr(), &mut stranger);
            assert_eq!(
                pamoja_mavlink_verifier_verify(stranger, frame),
                PamojaStatus::Auth
            );

            pamoja_mavlink_verifier_free(stranger);
            pamoja_mavlink_verifier_free(verifier);
            pamoja_mavlink_frame_free(frame);
            pamoja_mavlink_signer_free(signer);
        }
    }

    #[test]
    fn an_unsigned_frame_has_no_signature_to_report() {
        unsafe {
            let header = PamojaMavlinkHeader {
                system_id: 1,
                component_id: 1,
                sequence: 0,
            };
            let mut frame = ptr::null_mut();
            pamoja_mavlink_frame_encode(
                PAMOJA_MAVLINK_VERSION_V2,
                header,
                0,
                HEARTBEAT.as_ptr(),
                HEARTBEAT.len(),
                50,
                &mut frame,
            );
            let mut signature = [0u8; PAMOJA_MAVLINK_SIGNATURE_LEN];
            assert_eq!(
                pamoja_mavlink_frame_signature(frame, signature.as_mut_ptr()),
                PamojaStatus::Unsupported
            );
            pamoja_mavlink_frame_free(frame);
        }
    }

    #[test]
    fn the_checksum_matches_the_catalogue_check_value() {
        unsafe {
            let data = b"123456789";
            assert_eq!(
                pamoja_mavlink_crc16_mcrf4xx(data.as_ptr(), data.len()),
                crc16_mcrf4xx(data)
            );
        }
    }
}
