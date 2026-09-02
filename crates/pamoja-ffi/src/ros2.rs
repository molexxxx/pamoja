//! The C ABI for the ROS 2 naming and encoding rules.
//!
//! These wrap [`pamoja_ros2`] for callers that reach the SDK through the flat C
//! boundary: what makes a topic name legal, what it becomes on the DDS wire, the
//! RIHS type hash that identifies a message definition, and the CDR encoding the
//! payload itself is written in.
//!
//! None of it needs a ROS 2 installation, which is the point. A gateway can
//! validate a name, derive the DDS topic and the Zenoh key an `rmw_zenoh` peer
//! subscribes on, and encode a `geometry_msgs/msg/Twist` without linking against
//! anything from a ROS distribution. Driving a live graph does need one, so the
//! `bridge` feature stays Rust-only.

use std::ffi::c_char;
use std::fmt::Write as _;
use std::ptr;

use pamoja_ros2::key::entity_key;
use pamoja_ros2::msg::{CdrReader, CdrWriter, Twist, Vector3};
use pamoja_ros2::name::{dds_topic, is_fully_qualified, is_valid_name, percent_mangle, EntityKind};
use pamoja_ros2::typehash::{dds_type_name, TypeHash};

use crate::{read_bytes, read_str, set_last_error, PamojaBuffer, PamojaStatus, PamojaString};

/// The number of bytes in a RIHS01 type hash digest.
pub const PAMOJA_TYPE_HASH_LEN: usize = 32;

/// The ROS 2 subsystem a name belongs to, which fixes its DDS prefix.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaEntityKind {
    /// A topic, which takes the `rt` prefix.
    Topic = 0,
    /// The request side of a service, which takes the `rq` prefix.
    ServiceRequest = 1,
    /// The reply side of a service, which takes the `rr` prefix.
    ServiceResponse = 2,
}

impl From<PamojaEntityKind> for EntityKind {
    fn from(kind: PamojaEntityKind) -> Self {
        match kind {
            PamojaEntityKind::Topic => EntityKind::Topic,
            PamojaEntityKind::ServiceRequest => EntityKind::ServiceRequest,
            PamojaEntityKind::ServiceResponse => EntityKind::ServiceResponse,
        }
    }
}

/// A RIHS01 type hash: the 32-byte digest that identifies a message definition.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaTypeHash {
    /// The SHA-256 digest the hash carries.
    pub digest: [u8; PAMOJA_TYPE_HASH_LEN],
}

/// A three-dimensional vector, matching `geometry_msgs/msg/Vector3`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaVector3 {
    /// The x component.
    pub x: f64,
    /// The y component.
    pub y: f64,
    /// The z component.
    pub z: f64,
}

/// A body velocity command, matching `geometry_msgs/msg/Twist`.
///
/// This is what a ROS 2 robot is driven by on `cmd_vel`, so it is the shape a
/// chassis or navigation helper from `pamoja-kit` publishes into a ROS graph.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaRos2Twist {
    /// The linear velocity in metres per second.
    pub linear: PamojaVector3,
    /// The angular velocity in radians per second.
    pub angular: PamojaVector3,
}

impl From<Vector3> for PamojaVector3 {
    fn from(vector: Vector3) -> Self {
        Self {
            x: vector.x,
            y: vector.y,
            z: vector.z,
        }
    }
}

impl From<PamojaVector3> for Vector3 {
    fn from(vector: PamojaVector3) -> Self {
        Vector3::new(vector.x, vector.y, vector.z)
    }
}

impl PamojaTypeHash {
    /// Renders the digest as its RIHS01 string.
    fn text(&self) -> String {
        let mut text = String::from("RIHS01_");
        for byte in self.digest {
            let _ = write!(text, "{byte:02x}");
        }
        text
    }
}

/// Reports whether a string is a valid ROS 2 topic or service name.
///
/// # Arguments
///
/// * `name` - the candidate name, as null-terminated UTF-8.
///
/// # Returns
///
/// `true` when the name obeys the ROS 2 rules, or `false` if it does not or
/// `name` is null.
///
/// # Safety
///
/// `name` must be a valid null-terminated UTF-8 string for the duration of the
/// call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ros2_name_is_valid(name: *const c_char) -> bool {
    match read_str(name, "name") {
        Some(name) => is_valid_name(name),
        None => false,
    }
}

/// Reports whether a name is fully qualified, so it resolves with no namespace.
///
/// # Arguments
///
/// * `name` - the candidate name, as null-terminated UTF-8.
///
/// # Returns
///
/// `true` when the name is fully qualified, or `false` if it is not or `name` is
/// null.
///
/// # Safety
///
/// `name` must be a valid null-terminated UTF-8 string for the duration of the
/// call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ros2_name_is_fully_qualified(name: *const c_char) -> bool {
    match read_str(name, "name") {
        Some(name) => is_fully_qualified(name),
        None => false,
    }
}

/// Returns the DDS topic prefix a subsystem uses.
///
/// # Arguments
///
/// * `kind` - the subsystem.
///
/// # Returns
///
/// A null-terminated string with static lifetime, which the caller does not free.
#[no_mangle]
pub extern "C" fn pamoja_ros2_entity_kind_prefix(kind: PamojaEntityKind) -> *const c_char {
    match kind {
        PamojaEntityKind::Topic => c"rt".as_ptr(),
        PamojaEntityKind::ServiceRequest => c"rq".as_ptr(),
        PamojaEntityKind::ServiceResponse => c"rr".as_ptr(),
    }
}

/// Returns the DDS topic a fully qualified ROS 2 name maps onto.
///
/// # Arguments
///
/// * `fqn` - the fully qualified name, as null-terminated UTF-8.
/// * `kind` - which subsystem the name belongs to, which fixes the prefix.
///
/// # Returns
///
/// A string the caller must release with
/// [`pamoja_string_free`](crate::pamoja_string_free), or null if the name is not
/// fully qualified or `fqn` is null.
///
/// # Safety
///
/// `fqn` must be a valid null-terminated UTF-8 string for the duration of the
/// call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ros2_dds_topic(
    fqn: *const c_char,
    kind: PamojaEntityKind,
) -> *mut PamojaString {
    let Some(fqn) = read_str(fqn, "fqn") else {
        return ptr::null_mut();
    };
    match dds_topic(fqn, kind.into()) {
        Some(topic) => PamojaString::into_raw(topic),
        None => {
            set_last_error(format!("`{fqn}` is not a fully qualified ROS 2 name"));
            ptr::null_mut()
        }
    }
}

/// Percent-mangles a name the way a DDS partition requires.
///
/// # Arguments
///
/// * `name` - the name to mangle, as null-terminated UTF-8.
///
/// # Returns
///
/// A string the caller must release with
/// [`pamoja_string_free`](crate::pamoja_string_free), or null if `name` is null.
///
/// # Safety
///
/// `name` must be a valid null-terminated UTF-8 string for the duration of the
/// call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ros2_percent_mangle(name: *const c_char) -> *mut PamojaString {
    let Some(name) = read_str(name, "name") else {
        return ptr::null_mut();
    };
    PamojaString::into_raw(percent_mangle(name))
}

/// Returns the DDS type name a ROS 2 interface type maps onto.
///
/// # Arguments
///
/// * `ros_type` - the interface type as `package/namespace/Type`, such as
///   `std_msgs/msg/String`, as null-terminated UTF-8.
///
/// # Returns
///
/// A string such as `std_msgs::msg::dds_::String_`, which the caller must
/// release with [`pamoja_string_free`](crate::pamoja_string_free), or null if
/// the type is not a valid three-part interface type or `ros_type` is null.
///
/// # Safety
///
/// `ros_type` must be a valid null-terminated UTF-8 string for the duration of
/// the call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ros2_dds_type_name(ros_type: *const c_char) -> *mut PamojaString {
    let Some(ros_type) = read_str(ros_type, "ros_type") else {
        return ptr::null_mut();
    };
    match dds_type_name(ros_type) {
        Some(name) => PamojaString::into_raw(name),
        None => {
            set_last_error(format!(
                "`{ros_type}` is not a `package/namespace/Type` interface type"
            ));
            ptr::null_mut()
        }
    }
}

/// Parses a RIHS01 type hash string.
///
/// # Arguments
///
/// * `text` - the candidate hash, expected as `RIHS01_` plus 64 lowercase hex
///   digits, as null-terminated UTF-8.
/// * `out_hash` - receives the parsed hash.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once parsed, or [`PamojaStatus::InvalidArgument`] if the
/// text is malformed or either pointer is null.
///
/// # Safety
///
/// `text` must be a valid null-terminated UTF-8 string for the duration of the
/// call, and `out_hash` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ros2_type_hash_parse(
    text: *const c_char,
    out_hash: *mut PamojaTypeHash,
) -> PamojaStatus {
    if out_hash.is_null() {
        set_last_error("out_hash must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(text) = read_str(text, "text") else {
        return PamojaStatus::InvalidArgument;
    };
    match TypeHash::parse(text) {
        Some(hash) => {
            *out_hash = PamojaTypeHash {
                digest: hash.digest(),
            };
            PamojaStatus::Ok
        }
        None => {
            set_last_error(format!("`{text}` is not a well-formed RIHS01 hash"));
            PamojaStatus::InvalidArgument
        }
    }
}

/// Renders a type hash back to its RIHS01 string.
///
/// # Arguments
///
/// * `hash` - the hash to render.
///
/// # Returns
///
/// A string the caller must release with
/// [`pamoja_string_free`](crate::pamoja_string_free).
#[no_mangle]
pub extern "C" fn pamoja_ros2_type_hash_to_string(hash: PamojaTypeHash) -> *mut PamojaString {
    PamojaString::into_raw(hash.text())
}

/// Builds the Zenoh key an `rmw_zenoh` peer publishes an entity on.
///
/// # Arguments
///
/// * `domain_id` - the ROS 2 domain.
/// * `fqn` - the fully qualified entity name, as null-terminated UTF-8.
/// * `ros_type` - the interface type as `package/namespace/Type`, as
///   null-terminated UTF-8.
/// * `hash` - the message type hash.
///
/// # Returns
///
/// A key such as `0/chatter/std_msgs::msg::dds_::String_/RIHS01_...`, which the
/// caller must release with [`pamoja_string_free`](crate::pamoja_string_free),
/// or null if the name is not fully qualified, the type is not a valid interface
/// type, or either string is null.
///
/// # Safety
///
/// `fqn` and `ros_type` must be valid null-terminated UTF-8 strings for the
/// duration of the call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ros2_entity_key(
    domain_id: u32,
    fqn: *const c_char,
    ros_type: *const c_char,
    hash: PamojaTypeHash,
) -> *mut PamojaString {
    let (Some(fqn), Some(ros_type)) = (read_str(fqn, "fqn"), read_str(ros_type, "ros_type")) else {
        return ptr::null_mut();
    };
    let Some(parsed) = TypeHash::parse(&hash.text()) else {
        set_last_error("the type hash is malformed".to_owned());
        return ptr::null_mut();
    };
    match entity_key(domain_id, fqn, ros_type, &parsed) {
        Some(key) => PamojaString::into_raw(key),
        None => {
            set_last_error(format!(
                "no entity key for `{fqn}` of type `{ros_type}` in domain {domain_id}"
            ));
            ptr::null_mut()
        }
    }
}

/// Encodes a twist into its CDR representation.
///
/// # Arguments
///
/// * `twist` - the command to encode.
///
/// # Returns
///
/// A buffer the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free).
#[no_mangle]
pub extern "C" fn pamoja_ros2_twist_to_cdr(twist: PamojaRos2Twist) -> *mut PamojaBuffer {
    let twist = Twist {
        linear: twist.linear.into(),
        angular: twist.angular.into(),
    };
    PamojaBuffer::into_raw(twist.to_cdr())
}

/// Decodes a twist from its CDR representation.
///
/// # Arguments
///
/// * `data` - the encoded bytes.
/// * `data_len` - the length of `data`.
/// * `out_twist` - receives the decoded command.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once decoded, or [`PamojaStatus::InvalidArgument`] if the
/// bytes are not a well-formed twist or `out_twist` is null.
///
/// # Safety
///
/// `data` must point to at least `data_len` readable bytes or be null when that
/// length is 0, and `out_twist` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ros2_twist_from_cdr(
    data: *const u8,
    data_len: usize,
    out_twist: *mut PamojaRos2Twist,
) -> PamojaStatus {
    if out_twist.is_null() {
        set_last_error("out_twist must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let data = match read_bytes(data, data_len) {
        Ok(data) => data,
        Err(status) => return status,
    };
    match Twist::from_cdr(&data) {
        Some(twist) => {
            *out_twist = PamojaRos2Twist {
                linear: twist.linear.into(),
                angular: twist.angular.into(),
            };
            PamojaStatus::Ok
        }
        None => {
            set_last_error("the bytes are not a well-formed CDR twist".to_owned());
            PamojaStatus::InvalidArgument
        }
    }
}

/// An opaque handle to a CDR encoder.
pub struct PamojaCdrWriter {
    inner: CdrWriter,
}

/// Creates a CDR encoder with the encapsulation header already written.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_cdr_writer_free`] or consume
/// with [`pamoja_cdr_writer_into_bytes`].
#[no_mangle]
pub extern "C" fn pamoja_cdr_writer_new() -> *mut PamojaCdrWriter {
    Box::into_raw(Box::new(PamojaCdrWriter {
        inner: CdrWriter::new(),
    }))
}

/// Appends a 32-bit signed integer.
///
/// # Arguments
///
/// * `writer` - the encoder.
/// * `value` - the value to append.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written, or [`PamojaStatus::InvalidArgument`] if
/// `writer` is null.
///
/// # Safety
///
/// `writer` must be a live handle from [`pamoja_cdr_writer_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_writer_write_i32(
    writer: *mut PamojaCdrWriter,
    value: i32,
) -> PamojaStatus {
    let Some(writer) = writer_handle(writer) else {
        return PamojaStatus::InvalidArgument;
    };
    writer.inner.write_i32(value);
    PamojaStatus::Ok
}

/// Appends a 32-bit unsigned integer.
///
/// # Arguments
///
/// * `writer` - the encoder.
/// * `value` - the value to append.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written, or [`PamojaStatus::InvalidArgument`] if
/// `writer` is null.
///
/// # Safety
///
/// `writer` must be a live handle from [`pamoja_cdr_writer_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_writer_write_u32(
    writer: *mut PamojaCdrWriter,
    value: u32,
) -> PamojaStatus {
    let Some(writer) = writer_handle(writer) else {
        return PamojaStatus::InvalidArgument;
    };
    writer.inner.write_u32(value);
    PamojaStatus::Ok
}

/// Appends a 32-bit float.
///
/// # Arguments
///
/// * `writer` - the encoder.
/// * `value` - the value to append.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written, or [`PamojaStatus::InvalidArgument`] if
/// `writer` is null.
///
/// # Safety
///
/// `writer` must be a live handle from [`pamoja_cdr_writer_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_writer_write_f32(
    writer: *mut PamojaCdrWriter,
    value: f32,
) -> PamojaStatus {
    let Some(writer) = writer_handle(writer) else {
        return PamojaStatus::InvalidArgument;
    };
    writer.inner.write_f32(value);
    PamojaStatus::Ok
}

/// Appends a 64-bit float.
///
/// # Arguments
///
/// * `writer` - the encoder.
/// * `value` - the value to append.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written, or [`PamojaStatus::InvalidArgument`] if
/// `writer` is null.
///
/// # Safety
///
/// `writer` must be a live handle from [`pamoja_cdr_writer_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_writer_write_f64(
    writer: *mut PamojaCdrWriter,
    value: f64,
) -> PamojaStatus {
    let Some(writer) = writer_handle(writer) else {
        return PamojaStatus::InvalidArgument;
    };
    writer.inner.write_f64(value);
    PamojaStatus::Ok
}

/// Takes the encoded bytes, consuming the encoder.
///
/// # Arguments
///
/// * `writer` - the encoder, which this call frees.
///
/// # Returns
///
/// A buffer the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or null if `writer` is
/// null.
///
/// # Safety
///
/// `writer` must be a live handle from [`pamoja_cdr_writer_new`], or null. After
/// this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_writer_into_bytes(
    writer: *mut PamojaCdrWriter,
) -> *mut PamojaBuffer {
    if writer.is_null() {
        set_last_error("writer must not be null".to_owned());
        return ptr::null_mut();
    }
    let writer = Box::from_raw(writer);
    PamojaBuffer::into_raw(writer.inner.into_bytes())
}

/// Releases a CDR encoder handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `writer` must be a handle from [`pamoja_cdr_writer_new`] that has not already
/// been freed or consumed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_writer_free(writer: *mut PamojaCdrWriter) {
    if !writer.is_null() {
        drop(Box::from_raw(writer));
    }
}

/// The width of a field already taken from a decoder.
///
/// The core reader borrows the buffer it walks and keeps its cursor private, so
/// this handle owns the bytes and replays the reads made so far to reach the
/// cursor again. Alignment follows the width, so replaying by width lands in
/// exactly the position the original sequence did.
#[derive(Clone, Copy)]
enum Field {
    /// A four-byte field, aligned to four.
    Word,
    /// An eight-byte field, aligned to eight.
    Double,
}

/// An opaque handle to a CDR decoder.
pub struct PamojaCdrReader {
    data: Vec<u8>,
    taken: Vec<Field>,
}

/// Creates a CDR decoder over encoded bytes.
///
/// # Arguments
///
/// * `data` - the encoded bytes, which are copied.
/// * `data_len` - the length of `data`.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_cdr_reader_free`], or null if
/// the bytes carry no valid encapsulation header.
///
/// # Safety
///
/// `data` must point to at least `data_len` readable bytes, or be null when that
/// length is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_reader_new(
    data: *const u8,
    data_len: usize,
) -> *mut PamojaCdrReader {
    let Ok(data) = read_bytes(data, data_len) else {
        return ptr::null_mut();
    };
    if CdrReader::new(&data).is_none() {
        set_last_error("the bytes carry no valid CDR encapsulation header".to_owned());
        return ptr::null_mut();
    }
    Box::into_raw(Box::new(PamojaCdrReader {
        data,
        taken: Vec::new(),
    }))
}

/// Reads the next 32-bit signed integer.
///
/// # Arguments
///
/// * `reader` - the decoder.
/// * `out_value` - receives the value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read, or [`PamojaStatus::InvalidArgument`] if the
/// buffer is exhausted or either pointer is null.
///
/// # Safety
///
/// `reader` must be a live handle from [`pamoja_cdr_reader_new`] and `out_value`
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_reader_read_i32(
    reader: *mut PamojaCdrReader,
    out_value: *mut i32,
) -> PamojaStatus {
    read_field(reader, out_value, Field::Word, |cursor| cursor.read_i32())
}

/// Reads the next 32-bit unsigned integer.
///
/// # Arguments
///
/// * `reader` - the decoder.
/// * `out_value` - receives the value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read, or [`PamojaStatus::InvalidArgument`] if the
/// buffer is exhausted or either pointer is null.
///
/// # Safety
///
/// `reader` must be a live handle from [`pamoja_cdr_reader_new`] and `out_value`
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_reader_read_u32(
    reader: *mut PamojaCdrReader,
    out_value: *mut u32,
) -> PamojaStatus {
    read_field(reader, out_value, Field::Word, |cursor| cursor.read_u32())
}

/// Reads the next 32-bit float.
///
/// # Arguments
///
/// * `reader` - the decoder.
/// * `out_value` - receives the value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read, or [`PamojaStatus::InvalidArgument`] if the
/// buffer is exhausted or either pointer is null.
///
/// # Safety
///
/// `reader` must be a live handle from [`pamoja_cdr_reader_new`] and `out_value`
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_reader_read_f32(
    reader: *mut PamojaCdrReader,
    out_value: *mut f32,
) -> PamojaStatus {
    read_field(reader, out_value, Field::Word, |cursor| cursor.read_f32())
}

/// Reads the next 64-bit float.
///
/// # Arguments
///
/// * `reader` - the decoder.
/// * `out_value` - receives the value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read, or [`PamojaStatus::InvalidArgument`] if the
/// buffer is exhausted or either pointer is null.
///
/// # Safety
///
/// `reader` must be a live handle from [`pamoja_cdr_reader_new`] and `out_value`
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_reader_read_f64(
    reader: *mut PamojaCdrReader,
    out_value: *mut f64,
) -> PamojaStatus {
    read_field(reader, out_value, Field::Double, |cursor| cursor.read_f64())
}

/// Releases a CDR decoder handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `reader` must be a handle from [`pamoja_cdr_reader_new`] that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cdr_reader_free(reader: *mut PamojaCdrReader) {
    if !reader.is_null() {
        drop(Box::from_raw(reader));
    }
}

/// Borrows a writer handle, rejecting a null pointer.
///
/// # Safety
///
/// `writer` must be a live handle from [`pamoja_cdr_writer_new`], or null.
unsafe fn writer_handle<'a>(writer: *mut PamojaCdrWriter) -> Option<&'a mut PamojaCdrWriter> {
    if writer.is_null() {
        set_last_error("writer must not be null".to_owned());
        return None;
    }
    Some(&mut *writer)
}

/// Reads one field, replaying the fields already taken to reach the cursor.
///
/// # Safety
///
/// `reader` must be a live handle from [`pamoja_cdr_reader_new`] and `out_value`
/// must be writable.
unsafe fn read_field<T>(
    reader: *mut PamojaCdrReader,
    out_value: *mut T,
    width: Field,
    read: impl FnOnce(&mut CdrReader<'_>) -> Option<T>,
) -> PamojaStatus {
    if reader.is_null() {
        set_last_error("reader must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    if out_value.is_null() {
        set_last_error("out_value must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let handle = &mut *reader;
    let Some(mut cursor) = CdrReader::new(&handle.data) else {
        set_last_error("the bytes carry no valid CDR encapsulation header".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    for field in &handle.taken {
        let stepped = match field {
            Field::Word => cursor.read_u32().is_some(),
            Field::Double => cursor.read_f64().is_some(),
        };
        if !stepped {
            set_last_error("the CDR buffer is exhausted".to_owned());
            return PamojaStatus::InvalidArgument;
        }
    }
    match read(&mut cursor) {
        Some(value) => {
            *out_value = value;
            handle.taken.push(width);
            PamojaStatus::Ok
        }
        None => {
            set_last_error("the CDR buffer is exhausted".to_owned());
            PamojaStatus::InvalidArgument
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};

    use super::*;

    const CHATTER_HASH: &str =
        "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18";

    fn text_of(string: *mut PamojaString) -> String {
        assert!(!string.is_null(), "the call produced no string");
        let text = unsafe { CStr::from_ptr(crate::pamoja_string_data(string)) }
            .to_str()
            .expect("utf-8")
            .to_owned();
        unsafe { crate::pamoja_string_free(string) };
        text
    }

    #[test]
    fn a_topic_maps_onto_its_dds_name() {
        let fqn = CString::new("/robot1/cmd_vel").expect("static");
        let topic = unsafe { pamoja_ros2_dds_topic(fqn.as_ptr(), PamojaEntityKind::Topic) };
        assert_eq!(text_of(topic), "rt/robot1/cmd_vel");
    }

    #[test]
    fn a_name_that_breaks_the_rules_is_refused() {
        let bad = CString::new("/2foo").expect("static");
        assert!(!unsafe { pamoja_ros2_name_is_valid(bad.as_ptr()) });
        let good = CString::new("/robot1/camera_left/image_raw").expect("static");
        assert!(unsafe { pamoja_ros2_name_is_valid(good.as_ptr()) });
    }

    #[test]
    fn an_entity_key_matches_the_published_example() {
        let text = CString::new(CHATTER_HASH).expect("static");
        let mut hash = PamojaTypeHash {
            digest: [0u8; PAMOJA_TYPE_HASH_LEN],
        };
        assert_eq!(
            unsafe { pamoja_ros2_type_hash_parse(text.as_ptr(), &mut hash) },
            PamojaStatus::Ok
        );
        assert_eq!(text_of(pamoja_ros2_type_hash_to_string(hash)), CHATTER_HASH);

        let fqn = CString::new("/chatter").expect("static");
        let ros_type = CString::new("std_msgs/msg/String").expect("static");
        let key = unsafe { pamoja_ros2_entity_key(0, fqn.as_ptr(), ros_type.as_ptr(), hash) };
        assert_eq!(
            text_of(key),
            format!("0/chatter/std_msgs::msg::dds_::String_/{CHATTER_HASH}")
        );
    }

    #[test]
    fn a_twist_survives_a_cdr_round_trip() {
        let sent = PamojaRos2Twist {
            linear: PamojaVector3 {
                x: 1.5,
                y: 0.0,
                z: 0.0,
            },
            angular: PamojaVector3 {
                x: 0.0,
                y: 0.0,
                z: -0.25,
            },
        };
        let buffer = pamoja_ros2_twist_to_cdr(sent);
        assert!(!buffer.is_null());

        let mut received = PamojaRos2Twist {
            linear: PamojaVector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            angular: PamojaVector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        };
        let status = unsafe {
            pamoja_ros2_twist_from_cdr(
                crate::pamoja_buffer_data(buffer),
                crate::pamoja_buffer_len(buffer),
                &mut received,
            )
        };
        unsafe { crate::pamoja_buffer_free(buffer) };
        assert_eq!(status, PamojaStatus::Ok);
        assert_eq!(received, sent);
    }

    #[test]
    fn mixed_width_fields_read_back_in_order() {
        let writer = pamoja_cdr_writer_new();
        unsafe {
            assert_eq!(pamoja_cdr_writer_write_u32(writer, 7), PamojaStatus::Ok);
            assert_eq!(pamoja_cdr_writer_write_f64(writer, 2.5), PamojaStatus::Ok);
            assert_eq!(pamoja_cdr_writer_write_i32(writer, -3), PamojaStatus::Ok);
        }
        let buffer = unsafe { pamoja_cdr_writer_into_bytes(writer) };
        assert!(!buffer.is_null());

        let reader = unsafe {
            pamoja_cdr_reader_new(
                crate::pamoja_buffer_data(buffer),
                crate::pamoja_buffer_len(buffer),
            )
        };
        assert!(!reader.is_null());

        let mut word = 0u32;
        let mut double = 0f64;
        let mut signed = 0i32;
        unsafe {
            assert_eq!(
                pamoja_cdr_reader_read_u32(reader, &mut word),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_cdr_reader_read_f64(reader, &mut double),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_cdr_reader_read_i32(reader, &mut signed),
                PamojaStatus::Ok
            );
            pamoja_cdr_reader_free(reader);
            crate::pamoja_buffer_free(buffer);
        }
        assert_eq!(word, 7, "the first word reads back");
        assert_eq!(double, 2.5, "an eight-byte field keeps its alignment");
        assert_eq!(signed, -3, "and the field after it is not skewed");
    }

    #[test]
    fn a_null_argument_is_rejected_rather_than_dereferenced() {
        assert!(!unsafe { pamoja_ros2_name_is_valid(ptr::null()) });
        assert!(!unsafe { pamoja_ros2_name_is_fully_qualified(ptr::null()) });
        assert!(unsafe { pamoja_ros2_percent_mangle(ptr::null()) }.is_null());
        assert!(unsafe { pamoja_ros2_dds_type_name(ptr::null()) }.is_null());
        assert_eq!(
            unsafe { pamoja_ros2_type_hash_parse(ptr::null(), ptr::null_mut()) },
            PamojaStatus::InvalidArgument
        );
        assert_eq!(
            unsafe { pamoja_cdr_writer_write_u32(ptr::null_mut(), 0) },
            PamojaStatus::InvalidArgument
        );
        assert!(unsafe { pamoja_cdr_writer_into_bytes(ptr::null_mut()) }.is_null());
    }
}
