//! The C ABI for MAVLink message shapes: reading and writing a message by field name.
//!
//! [`mavlink`](crate::mavlink) carries any message's bytes, which is enough to move traffic
//! but leaves the caller hand-packing payloads against a message definition. This is the
//! layer above: a schema states a message's fields, and a message reads and writes them by
//! name, so a caller works in `custom_mode` and `lat` rather than byte offsets.
//!
//! Two sources of shape are supported and behave identically. Every message the engine
//! types is published, so [`pamoja_mavlink_schema_for_id`] resolves the common dialect with
//! nothing to declare. A message from ArduPilot's dialect, PX4's, or a vendor's private one
//! is described through [`PamojaMavlinkSchemaBuilder`], which puts the fields in wire order
//! and derives the `CRC_EXTRA` seed, so a caller transcribes a definition as it reads.

use std::ffi::{c_char, CString};

use pamoja_mavlink::dialect::{
    descriptor, descriptor_by_name, DynamicMessage, FieldType, MessageDescriptor,
    MessageDescriptorBuilder, OwnedMessageDescriptor, DESCRIPTORS,
};
use pamoja_mavlink::{Header, Result as MavlinkResult};

use crate::mavlink::{status_of, PamojaMavlinkDialect, PamojaMavlinkFrame, PamojaMavlinkHeader};
use crate::{read_bytes, read_str, set_last_error, PamojaStatus};

/// A `uint8_t` field.
pub const PAMOJA_MAVLINK_FIELD_UINT8: u32 = 1;
/// An `int8_t` field.
pub const PAMOJA_MAVLINK_FIELD_INT8: u32 = 2;
/// A `char` field; an array of these carries text.
pub const PAMOJA_MAVLINK_FIELD_CHAR: u32 = 3;
/// A `uint16_t` field.
pub const PAMOJA_MAVLINK_FIELD_UINT16: u32 = 4;
/// An `int16_t` field.
pub const PAMOJA_MAVLINK_FIELD_INT16: u32 = 5;
/// A `uint32_t` field.
pub const PAMOJA_MAVLINK_FIELD_UINT32: u32 = 6;
/// An `int32_t` field.
pub const PAMOJA_MAVLINK_FIELD_INT32: u32 = 7;
/// A `uint64_t` field.
pub const PAMOJA_MAVLINK_FIELD_UINT64: u32 = 8;
/// An `int64_t` field.
pub const PAMOJA_MAVLINK_FIELD_INT64: u32 = 9;
/// A `float` field.
pub const PAMOJA_MAVLINK_FIELD_FLOAT: u32 = 10;
/// A `double` field.
pub const PAMOJA_MAVLINK_FIELD_DOUBLE: u32 = 11;

/// Maps a field type onto its stable code.
///
/// The codes are written out rather than taken from a Rust enum's discriminants, so a
/// value that crosses the boundary means the same thing in every build.
fn code_of(ty: FieldType) -> u32 {
    match ty {
        FieldType::U8 => PAMOJA_MAVLINK_FIELD_UINT8,
        FieldType::I8 => PAMOJA_MAVLINK_FIELD_INT8,
        FieldType::Char => PAMOJA_MAVLINK_FIELD_CHAR,
        FieldType::U16 => PAMOJA_MAVLINK_FIELD_UINT16,
        FieldType::I16 => PAMOJA_MAVLINK_FIELD_INT16,
        FieldType::U32 => PAMOJA_MAVLINK_FIELD_UINT32,
        FieldType::I32 => PAMOJA_MAVLINK_FIELD_INT32,
        FieldType::U64 => PAMOJA_MAVLINK_FIELD_UINT64,
        FieldType::I64 => PAMOJA_MAVLINK_FIELD_INT64,
        FieldType::F32 => PAMOJA_MAVLINK_FIELD_FLOAT,
        FieldType::F64 => PAMOJA_MAVLINK_FIELD_DOUBLE,
    }
}

/// Resolves a field type code, reporting an unknown one as an error.
fn type_of(code: u32) -> Result<FieldType, PamojaStatus> {
    Ok(match code {
        PAMOJA_MAVLINK_FIELD_UINT8 => FieldType::U8,
        PAMOJA_MAVLINK_FIELD_INT8 => FieldType::I8,
        PAMOJA_MAVLINK_FIELD_CHAR => FieldType::Char,
        PAMOJA_MAVLINK_FIELD_UINT16 => FieldType::U16,
        PAMOJA_MAVLINK_FIELD_INT16 => FieldType::I16,
        PAMOJA_MAVLINK_FIELD_UINT32 => FieldType::U32,
        PAMOJA_MAVLINK_FIELD_INT32 => FieldType::I32,
        PAMOJA_MAVLINK_FIELD_UINT64 => FieldType::U64,
        PAMOJA_MAVLINK_FIELD_INT64 => FieldType::I64,
        PAMOJA_MAVLINK_FIELD_FLOAT => FieldType::F32,
        PAMOJA_MAVLINK_FIELD_DOUBLE => FieldType::F64,
        _ => {
            set_last_error(format!("{code} is not a MAVLink field type"));
            return Err(PamojaStatus::InvalidArgument);
        }
    })
}

/// One field of a message shape, as read back from a schema.
///
/// Both names point into the schema they came from and stay valid until it is released, so
/// a caller reads them in place rather than freeing them.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PamojaMavlinkFieldInfo {
    /// The field name as the dialect writes it, such as `custom_mode`.
    pub name: *const c_char,
    /// The field's type name as the dialect writes it, such as `uint32_t`.
    pub type_name: *const c_char,
    /// The field's type, one of the `PAMOJA_MAVLINK_FIELD_*` codes.
    pub field_type: u32,
    /// The element count for an array field, or `0` for a scalar.
    pub array_len: u8,
    /// `1` for a MAVLink 2 extension field, `0` for a base field.
    pub extension: u8,
    /// The field's byte offset within the payload.
    pub offset: usize,
}

/// The shape of one message: its id, name, seed, and fields.
///
/// A schema is what turns bytes into named fields. It comes either from the built-in
/// registry of typed messages or from a [`PamojaMavlinkSchemaBuilder`], and behaves the
/// same way whichever it is.
pub struct PamojaMavlinkSchema {
    shape: OwnedMessageDescriptor,
    name: CString,
    field_names: Vec<CString>,
}

impl PamojaMavlinkSchema {
    /// Moves a shape onto the heap, interning its names for the C side to borrow.
    fn into_handle(shape: OwnedMessageDescriptor) -> Result<*mut Self, PamojaStatus> {
        let interior = || {
            set_last_error("a message or field name contains an interior null byte".to_owned());
            PamojaStatus::InvalidArgument
        };
        let name = CString::new(shape.name()).map_err(|_| interior())?;
        let field_names = shape
            .fields()
            .iter()
            .map(|field| CString::new(field.name.as_str()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| interior())?;
        Ok(Box::into_raw(Box::new(Self {
            shape,
            name,
            field_names,
        })))
    }
}

/// Returns the shape of a message the engine types, by id.
///
/// # Arguments
///
/// * `msgid` - the message id to look up.
/// * `out_schema` - set to a new schema handle on success, which the caller releases with
///   [`pamoja_mavlink_schema_free`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_schema` is null, and
/// [`PamojaStatus::Unsupported`] for an id this build does not type, which is what
/// [`PamojaMavlinkSchemaBuilder`] is for.
///
/// # Safety
///
/// `out_schema` must point at writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_for_id(
    msgid: u32,
    out_schema: *mut *mut PamojaMavlinkSchema,
) -> PamojaStatus {
    if out_schema.is_null() {
        set_last_error("out_schema must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(shape) = descriptor(msgid) else {
        set_last_error(format!("message {msgid} is not one this build types"));
        return PamojaStatus::Unsupported;
    };
    publish(shape, out_schema)
}

/// Returns the shape of a message the engine types, by name.
///
/// # Arguments
///
/// * `name` - the message name, such as `GLOBAL_POSITION_INT`.
/// * `out_schema` - set to a new schema handle on success, which the caller releases with
///   [`pamoja_mavlink_schema_free`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or `name` is not
/// UTF-8, and [`PamojaStatus::Unsupported`] for a name this build does not type.
///
/// # Safety
///
/// `name` must be a null-terminated C string, and `out_schema` must point at writable
/// storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_for_name(
    name: *const c_char,
    out_schema: *mut *mut PamojaMavlinkSchema,
) -> PamojaStatus {
    if out_schema.is_null() {
        set_last_error("out_schema must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(name) = read_str(name, "name") else {
        return PamojaStatus::InvalidArgument;
    };
    let Some(shape) = descriptor_by_name(name) else {
        set_last_error(format!("{name} is not a message this build types"));
        return PamojaStatus::Unsupported;
    };
    publish(shape, out_schema)
}

/// Returns how many messages this build types.
///
/// Together with [`pamoja_mavlink_schema_at`] this enumerates the built-in registry, so a
/// caller can discover what is available rather than guessing ids.
///
/// # Returns
///
/// The count.
#[no_mangle]
pub extern "C" fn pamoja_mavlink_schema_count() -> usize {
    DESCRIPTORS.len()
}

/// Returns the shape of the message at a position in the built-in registry.
///
/// # Arguments
///
/// * `index` - the position, below [`pamoja_mavlink_schema_count`].
/// * `out_schema` - set to a new schema handle on success, which the caller releases with
///   [`pamoja_mavlink_schema_free`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_schema` is null or `index` is past
/// the end of the registry.
///
/// # Safety
///
/// `out_schema` must point at writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_at(
    index: usize,
    out_schema: *mut *mut PamojaMavlinkSchema,
) -> PamojaStatus {
    if out_schema.is_null() {
        set_last_error("out_schema must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(shape) = DESCRIPTORS.get(index) else {
        set_last_error(format!("{index} is past the end of the registry"));
        return PamojaStatus::InvalidArgument;
    };
    publish(shape, out_schema)
}

unsafe fn publish(
    shape: &MessageDescriptor<'static>,
    out_schema: *mut *mut PamojaMavlinkSchema,
) -> PamojaStatus {
    match PamojaMavlinkSchema::into_handle(OwnedMessageDescriptor::from_descriptor(shape)) {
        Ok(handle) => {
            *out_schema = handle;
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Returns the id of the message a schema describes.
///
/// # Arguments
///
/// * `schema` - the shape to read.
///
/// # Returns
///
/// The message id, or `0` if `schema` is null.
///
/// # Safety
///
/// `schema` must be a live schema handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_id(schema: *const PamojaMavlinkSchema) -> u32 {
    schema.as_ref().map_or(0, |schema| schema.shape.id())
}

/// Returns the name of the message a schema describes.
///
/// The pointer borrows the schema and stays valid until it is released.
///
/// # Arguments
///
/// * `schema` - the shape to read.
///
/// # Returns
///
/// The name, or null if `schema` is null.
///
/// # Safety
///
/// `schema` must be a live schema handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_name(
    schema: *const PamojaMavlinkSchema,
) -> *const c_char {
    schema
        .as_ref()
        .map_or(std::ptr::null(), |schema| schema.name.as_ptr())
}

/// Returns the `CRC_EXTRA` seed a schema implies.
///
/// # Arguments
///
/// * `schema` - the shape to read.
///
/// # Returns
///
/// The seed, or `0` if `schema` is null.
///
/// # Safety
///
/// `schema` must be a live schema handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_crc_extra(schema: *const PamojaMavlinkSchema) -> u8 {
    schema.as_ref().map_or(0, |schema| schema.shape.crc_extra())
}

/// Returns the length of the message on the wire, extensions included.
///
/// # Arguments
///
/// * `schema` - the shape to read.
///
/// # Returns
///
/// The length in bytes, or `0` if `schema` is null.
///
/// # Safety
///
/// `schema` must be a live schema handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_wire_len(
    schema: *const PamojaMavlinkSchema,
) -> usize {
    schema.as_ref().map_or(0, |schema| {
        schema.shape.with_descriptor(|shape| shape.wire_len())
    })
}

/// Returns how many fields a message has.
///
/// # Arguments
///
/// * `schema` - the shape to read.
///
/// # Returns
///
/// The field count, or `0` if `schema` is null.
///
/// # Safety
///
/// `schema` must be a live schema handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_field_count(
    schema: *const PamojaMavlinkSchema,
) -> usize {
    schema
        .as_ref()
        .map_or(0, |schema| schema.shape.fields().len())
}

/// Describes one field of a message.
///
/// # Arguments
///
/// * `schema` - the shape to read.
/// * `index` - the field position, in wire order, below
///   [`pamoja_mavlink_schema_field_count`].
/// * `out_field` - set to the field's description on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or `index` is past
/// the end of the field list.
///
/// # Safety
///
/// `schema` must be a live schema handle, and `out_field` must point at writable storage
/// for one [`PamojaMavlinkFieldInfo`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_field(
    schema: *const PamojaMavlinkSchema,
    index: usize,
    out_field: *mut PamojaMavlinkFieldInfo,
) -> PamojaStatus {
    let Some(schema) = schema.as_ref() else {
        set_last_error("schema must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if out_field.is_null() {
        set_last_error("out_field must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(field) = schema.shape.fields().get(index) else {
        set_last_error(format!("{index} is past the end of the field list"));
        return PamojaStatus::InvalidArgument;
    };
    let offset = schema
        .shape
        .with_descriptor(|shape| shape.offset_of(&field.name))
        .expect("a field of this shape has an offset in it");

    *out_field = PamojaMavlinkFieldInfo {
        name: schema.field_names[index].as_ptr(),
        type_name: TYPE_NAMES[type_index(field.ty)].as_ptr().cast(),
        field_type: code_of(field.ty),
        array_len: field.array_len,
        extension: u8::from(field.extension),
        offset,
    };
    PamojaStatus::Ok
}

// The type names handed back as borrowed C strings. They are null-terminated here so a
// caller can read them in place, the way it reads a field name out of a schema.
const TYPE_NAMES: [&[u8]; 11] = [
    b"uint8_t\0",
    b"int8_t\0",
    b"char\0",
    b"uint16_t\0",
    b"int16_t\0",
    b"uint32_t\0",
    b"int32_t\0",
    b"uint64_t\0",
    b"int64_t\0",
    b"float\0",
    b"double\0",
];

fn type_index(ty: FieldType) -> usize {
    code_of(ty) as usize - 1
}

/// Releases a schema.
///
/// # Arguments
///
/// * `schema` - the handle to release; null is ignored.
///
/// # Safety
///
/// `schema` must have come from one of the schema constructors and must not be used
/// afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_free(schema: *mut PamojaMavlinkSchema) {
    if !schema.is_null() {
        drop(Box::from_raw(schema));
    }
}

/// Adds a schema's message to a dialect table, so frames carrying it check.
///
/// # Arguments
///
/// * `dialect` - the table to extend.
/// * `schema` - the shape whose id and seed to add.
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
/// `dialect` must be a live dialect handle and `schema` a live schema handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_dialect_add_schema(
    dialect: *mut PamojaMavlinkDialect,
    schema: *const PamojaMavlinkSchema,
) -> PamojaStatus {
    let Some(schema) = schema.as_ref() else {
        set_last_error("schema must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    crate::mavlink::pamoja_mavlink_dialect_add(dialect, schema.shape.id(), schema.shape.crc_extra())
}

/// Describes a message this build does not type, one field at a time.
///
/// Fields are added in the order the message definition lists them;
/// [`pamoja_mavlink_schema_builder_build`] puts them in wire order and derives the
/// `CRC_EXTRA` seed from the result.
pub struct PamojaMavlinkSchemaBuilder {
    builder: Option<MessageDescriptorBuilder>,
}

/// Starts describing a message.
///
/// # Arguments
///
/// * `msgid` - the message id on the wire.
/// * `name` - the message name, which the seed derivation folds in, so it must match the
///   dialect exactly.
///
/// # Returns
///
/// A builder the caller releases with [`pamoja_mavlink_schema_builder_free`] or consumes
/// with [`pamoja_mavlink_schema_builder_build`], or null if `name` is null or not UTF-8.
///
/// # Safety
///
/// `name` must be a null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_builder_new(
    msgid: u32,
    name: *const c_char,
) -> *mut PamojaMavlinkSchemaBuilder {
    let Some(name) = read_str(name, "name") else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(PamojaMavlinkSchemaBuilder {
        builder: Some(MessageDescriptorBuilder::new(msgid, name)),
    }))
}

/// Adds a base field, in the order the definition declares it.
///
/// # Arguments
///
/// * `builder` - the description to extend.
/// * `name` - the field name.
/// * `field_type` - the field's type, one of the `PAMOJA_MAVLINK_FIELD_*` codes.
/// * `array_len` - the element count for an array, or `0` for a scalar.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, `name` is not UTF-8, or
/// `field_type` is not a MAVLink field type.
///
/// # Safety
///
/// `builder` must be a live builder handle and `name` a null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_builder_field(
    builder: *mut PamojaMavlinkSchemaBuilder,
    name: *const c_char,
    field_type: u32,
    array_len: u8,
) -> PamojaStatus {
    add_field(builder, name, field_type, array_len, false)
}

/// Adds a MAVLink 2 extension field, in the order the definition declares it.
///
/// Extensions keep their declared order, stay out of the `CRC_EXTRA` seed, and read as zero
/// from a frame sent by a peer that predates them.
///
/// # Arguments
///
/// * `builder` - the description to extend.
/// * `name` - the field name.
/// * `field_type` - the field's type, one of the `PAMOJA_MAVLINK_FIELD_*` codes.
/// * `array_len` - the element count for an array, or `0` for a scalar.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, `name` is not UTF-8, or
/// `field_type` is not a MAVLink field type.
///
/// # Safety
///
/// `builder` must be a live builder handle and `name` a null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_builder_extension(
    builder: *mut PamojaMavlinkSchemaBuilder,
    name: *const c_char,
    field_type: u32,
    array_len: u8,
) -> PamojaStatus {
    add_field(builder, name, field_type, array_len, true)
}

unsafe fn add_field(
    builder: *mut PamojaMavlinkSchemaBuilder,
    name: *const c_char,
    field_type: u32,
    array_len: u8,
    extension: bool,
) -> PamojaStatus {
    let Some(handle) = builder.as_mut() else {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(name) = read_str(name, "name") else {
        return PamojaStatus::InvalidArgument;
    };
    let ty = match type_of(field_type) {
        Ok(ty) => ty,
        Err(status) => return status,
    };
    let Some(builder) = handle.builder.take() else {
        set_last_error("this builder has already been built".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    handle.builder = Some(if extension {
        builder.extension(name, ty, array_len)
    } else {
        builder.field(name, ty, array_len)
    });
    PamojaStatus::Ok
}

/// Puts the declared fields in wire order and finishes the shape.
///
/// The builder is consumed and released whether or not the shape is valid, so it must not
/// be used again.
///
/// # Arguments
///
/// * `builder` - the description to finish.
/// * `out_schema` - set to a new schema handle on success, which the caller releases with
///   [`pamoja_mavlink_schema_free`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, if two fields share a
/// name, or if the fields do not fit a MAVLink payload.
///
/// # Safety
///
/// `builder` must be a live builder handle, and `out_schema` must point at writable
/// storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_builder_build(
    builder: *mut PamojaMavlinkSchemaBuilder,
    out_schema: *mut *mut PamojaMavlinkSchema,
) -> PamojaStatus {
    if builder.is_null() {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    if out_schema.is_null() {
        set_last_error("out_schema must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let handle = Box::from_raw(builder);
    let Some(builder) = handle.builder else {
        set_last_error("this builder has already been built".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let shape = match builder.build() {
        Ok(shape) => shape,
        Err(error) => return status_of(error),
    };
    match PamojaMavlinkSchema::into_handle(shape) {
        Ok(schema) => {
            *out_schema = schema;
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Releases a builder that was never built.
///
/// # Arguments
///
/// * `builder` - the handle to release; null is ignored.
///
/// # Safety
///
/// `builder` must have come from [`pamoja_mavlink_schema_builder_new`], must not already
/// have been built, and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_schema_builder_free(
    builder: *mut PamojaMavlinkSchemaBuilder,
) {
    if !builder.is_null() {
        drop(Box::from_raw(builder));
    }
}

/// A message being written or read field by field against a schema.
pub struct PamojaMavlinkMessage {
    shape: OwnedMessageDescriptor,
    payload: Vec<u8>,
}

impl PamojaMavlinkMessage {
    /// Wraps a message the engine decoded, for another module in this crate to hand back.
    pub(crate) fn from_typed(shape: &MessageDescriptor<'static>, payload: Vec<u8>) -> *mut Self {
        Box::into_raw(Box::new(Self {
            shape: OwnedMessageDescriptor::from_descriptor(shape),
            payload,
        }))
    }
}

/// Runs a read against a message, writing the result through `out`.
unsafe fn get<T: Copy>(
    message: *const PamojaMavlinkMessage,
    name: *const c_char,
    out: *mut T,
    query: impl FnOnce(&DynamicMessage<'_>, &str) -> MavlinkResult<T>,
) -> PamojaStatus {
    let Some(message) = message.as_ref() else {
        set_last_error("message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if out.is_null() {
        set_last_error("the output pointer must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(name) = read_str(name, "field") else {
        return PamojaStatus::InvalidArgument;
    };
    let result = message.shape.with_descriptor(|shape| {
        let dynamic = DynamicMessage::decode(shape, &message.payload)?;
        query(&dynamic, name)
    });
    match result {
        Ok(value) => {
            *out = value;
            PamojaStatus::Ok
        }
        Err(error) => status_of(error),
    }
}

/// Runs a write against a message, keeping its bytes unchanged if the write fails.
unsafe fn set(
    message: *mut PamojaMavlinkMessage,
    name: *const c_char,
    step: impl FnOnce(&mut DynamicMessage<'_>, &str) -> MavlinkResult<()>,
) -> PamojaStatus {
    let Some(message) = message.as_mut() else {
        set_last_error("message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(name) = read_str(name, "field") else {
        return PamojaStatus::InvalidArgument;
    };
    let payload = std::mem::take(&mut message.payload);
    let result = message.shape.with_descriptor(|shape| {
        let mut dynamic = DynamicMessage::decode(shape, &payload)?;
        step(&mut dynamic, name)?;
        Ok(dynamic.payload().to_vec())
    });
    match result {
        Ok(updated) => {
            message.payload = updated;
            PamojaStatus::Ok
        }
        Err(error) => {
            message.payload = payload;
            status_of(error)
        }
    }
}

/// Creates a message with every field zero.
///
/// # Arguments
///
/// * `schema` - the shape of the message to build.
/// * `out_message` - set to a new message handle on success, which the caller releases
///   with [`pamoja_mavlink_message_free`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or the shape does
/// not fit a MAVLink payload.
///
/// # Safety
///
/// `schema` must be a live schema handle, and `out_message` must point at writable storage
/// for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_new(
    schema: *const PamojaMavlinkSchema,
    out_message: *mut *mut PamojaMavlinkMessage,
) -> PamojaStatus {
    let Some(schema) = schema.as_ref() else {
        set_last_error("schema must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if out_message.is_null() {
        set_last_error("out_message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let built = schema.shape.with_descriptor(|shape| {
        DynamicMessage::new(shape).map(|message| message.payload().to_vec())
    });
    match built {
        Ok(payload) => {
            *out_message = Box::into_raw(Box::new(PamojaMavlinkMessage {
                shape: schema.shape.clone(),
                payload,
            }));
            PamojaStatus::Ok
        }
        Err(error) => status_of(error),
    }
}

/// Reads a message out of a frame payload.
///
/// A payload shorter than the shape is zero-extended, as MAVLink 2 truncation requires, so
/// a frame from a peer that trimmed trailing zeros or predates an extension field decodes.
///
/// # Arguments
///
/// * `schema` - the shape to read the payload as.
/// * `payload` - the frame payload.
/// * `payload_len` - the payload length in bytes.
/// * `out_message` - set to a new message handle on success, which the caller releases
///   with [`pamoja_mavlink_message_free`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, and
/// [`PamojaStatus::Codec`] if the payload is longer than the shape describes.
///
/// # Safety
///
/// `schema` must be a live schema handle, `payload` must point at `payload_len` readable
/// bytes, and `out_message` must point at writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_decode(
    schema: *const PamojaMavlinkSchema,
    payload: *const u8,
    payload_len: usize,
    out_message: *mut *mut PamojaMavlinkMessage,
) -> PamojaStatus {
    let Some(schema) = schema.as_ref() else {
        set_last_error("schema must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if out_message.is_null() {
        set_last_error("out_message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    let decoded = schema.shape.with_descriptor(|shape| {
        DynamicMessage::decode(shape, &payload).map(|message| message.payload().to_vec())
    });
    match decoded {
        Ok(payload) => {
            *out_message = Box::into_raw(Box::new(PamojaMavlinkMessage {
                shape: schema.shape.clone(),
                payload,
            }));
            PamojaStatus::Ok
        }
        Err(error) => status_of(error),
    }
}

/// Returns a pointer to a message's payload bytes.
///
/// The pointer borrows the message and stays valid until it is written to or released.
///
/// # Arguments
///
/// * `message` - the message to read.
/// * `out_len` - set to the payload length in bytes.
///
/// # Returns
///
/// A pointer to the bytes, or null if either pointer is null.
///
/// # Safety
///
/// `message` must be a live message handle, and `out_len` must point at writable storage
/// for one length.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_payload(
    message: *const PamojaMavlinkMessage,
    out_len: *mut usize,
) -> *const u8 {
    let Some(message) = message.as_ref() else {
        return std::ptr::null();
    };
    if out_len.is_null() {
        return std::ptr::null();
    }
    *out_len = message.payload.len();
    message.payload.as_ptr()
}

/// Builds a v2 frame carrying a message.
///
/// # Arguments
///
/// * `message` - the message to send.
/// * `header` - the addressing fields to stamp on the frame.
/// * `out_frame` - set to a new frame handle on success, which the caller releases with
///   `pamoja_mavlink_frame_free`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or the message does
/// not fit a frame.
///
/// # Safety
///
/// `message` must be a live message handle, and `out_frame` must point at writable storage
/// for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_to_frame(
    message: *const PamojaMavlinkMessage,
    header: PamojaMavlinkHeader,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    let Some(message) = message.as_ref() else {
        set_last_error("message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let built = message.shape.with_descriptor(|shape| {
        DynamicMessage::decode(shape, &message.payload)?.to_frame(Header::from(header))
    });
    match built {
        Ok(frame) => {
            *out_frame = PamojaMavlinkFrame::into_handle(frame);
            PamojaStatus::Ok
        }
        Err(error) => status_of(error),
    }
}

/// Releases a message.
///
/// # Arguments
///
/// * `message` - the handle to release; null is ignored.
///
/// # Safety
///
/// `message` must have come from [`pamoja_mavlink_message_new`] or
/// [`pamoja_mavlink_message_decode`] and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_free(message: *mut PamojaMavlinkMessage) {
    if !message.is_null() {
        drop(Box::from_raw(message));
    }
}

/// Reads a field as a signed integer.
///
/// Any integer field reads this way, whatever its width or sign.
///
/// # Arguments
///
/// * `message` - the message to read.
/// * `field` - the field name.
/// * `index` - the element to read, or `0` for a scalar field.
/// * `out_value` - set to the value on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the message has no such
/// field, the element is past the end of an array, the field is floating-point, or a
/// `uint64_t` value is above the signed range.
///
/// # Safety
///
/// `message` must be a live message handle, `field` a null-terminated C string, and
/// `out_value` must point at writable storage for one 64-bit integer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_get_int(
    message: *const PamojaMavlinkMessage,
    field: *const c_char,
    index: usize,
    out_value: *mut i64,
) -> PamojaStatus {
    get(message, field, out_value, |dynamic, name| {
        dynamic.get_int(name, index)
    })
}

/// Reads a field as an unsigned integer.
///
/// Any integer field reads this way, whatever its width or sign.
///
/// # Arguments
///
/// * `message` - the message to read.
/// * `field` - the field name.
/// * `index` - the element to read, or `0` for a scalar field.
/// * `out_value` - set to the value on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the message has no such
/// field, the element is past the end of an array, the field is floating-point, or the
/// value is negative.
///
/// # Safety
///
/// `message` must be a live message handle, `field` a null-terminated C string, and
/// `out_value` must point at writable storage for one 64-bit integer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_get_uint(
    message: *const PamojaMavlinkMessage,
    field: *const c_char,
    index: usize,
    out_value: *mut u64,
) -> PamojaStatus {
    get(message, field, out_value, |dynamic, name| {
        dynamic.get_uint(name, index)
    })
}

/// Reads a floating-point field.
///
/// # Arguments
///
/// * `message` - the message to read.
/// * `field` - the field name.
/// * `index` - the element to read, or `0` for a scalar field.
/// * `out_value` - set to the value on success, widened from `float` where needed.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the message has no such
/// field, the element is past the end of an array, or the field is an integer.
///
/// # Safety
///
/// `message` must be a live message handle, `field` a null-terminated C string, and
/// `out_value` must point at writable storage for one double.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_get_float(
    message: *const PamojaMavlinkMessage,
    field: *const c_char,
    index: usize,
    out_value: *mut f64,
) -> PamojaStatus {
    get(message, field, out_value, |dynamic, name| {
        dynamic.get_float(name, index)
    })
}

/// Writes a signed integer into a field.
///
/// # Arguments
///
/// * `message` - the message to write.
/// * `field` - the field name.
/// * `index` - the element to write, or `0` for a scalar field.
/// * `value` - the value to store.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the message has no such
/// field, the element is past the end of an array, the field is floating-point, or the
/// value does not fit the field's type.
///
/// # Safety
///
/// `message` must be a live message handle and `field` a null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_set_int(
    message: *mut PamojaMavlinkMessage,
    field: *const c_char,
    index: usize,
    value: i64,
) -> PamojaStatus {
    set(message, field, |dynamic, name| {
        dynamic.set_int(name, index, value)
    })
}

/// Writes an unsigned integer into a field.
///
/// # Arguments
///
/// * `message` - the message to write.
/// * `field` - the field name.
/// * `index` - the element to write, or `0` for a scalar field.
/// * `value` - the value to store.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the message has no such
/// field, the element is past the end of an array, the field is floating-point, or the
/// value does not fit the field's type.
///
/// # Safety
///
/// `message` must be a live message handle and `field` a null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_set_uint(
    message: *mut PamojaMavlinkMessage,
    field: *const c_char,
    index: usize,
    value: u64,
) -> PamojaStatus {
    set(message, field, |dynamic, name| {
        dynamic.set_uint(name, index, value)
    })
}

/// Writes a floating-point field.
///
/// # Arguments
///
/// * `message` - the message to write.
/// * `field` - the field name.
/// * `index` - the element to write, or `0` for a scalar field.
/// * `value` - the value to store, narrowed to `float` where the field is one.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the message has no such
/// field, the element is past the end of an array, or the field is an integer.
///
/// # Safety
///
/// `message` must be a live message handle and `field` a null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_set_float(
    message: *mut PamojaMavlinkMessage,
    field: *const c_char,
    index: usize,
    value: f64,
) -> PamojaStatus {
    set(message, field, |dynamic, name| {
        dynamic.set_float(name, index, value)
    })
}

/// Reads a field as a double, whatever its type.
///
/// This is the reading a host language with one numeric type needs. An integer field wider
/// than 53 bits can exceed what a double holds exactly, so read those with
/// [`pamoja_mavlink_message_get_int`] or [`pamoja_mavlink_message_get_uint`] where the
/// exact value matters.
///
/// # Arguments
///
/// * `message` - the message to read.
/// * `field` - the field name.
/// * `index` - the element to read, or `0` for a scalar field.
/// * `out_value` - set to the value on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the message has no such
/// field, or the element is past the end of an array.
///
/// # Safety
///
/// `message` must be a live message handle, `field` a null-terminated C string, and
/// `out_value` must point at writable storage for one double.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_get_number(
    message: *const PamojaMavlinkMessage,
    field: *const c_char,
    index: usize,
    out_value: *mut f64,
) -> PamojaStatus {
    get(message, field, out_value, |dynamic, name| {
        dynamic.get_number(name, index)
    })
}

/// Writes a double into a field, converting it to the field's type.
///
/// This is the writing a host language with one numeric type needs. A value bound for an
/// integer field must be a whole number within that field's range, so a fractional or
/// oversized value is refused rather than silently truncated.
///
/// # Arguments
///
/// * `message` - the message to write.
/// * `field` - the field name.
/// * `index` - the element to write, or `0` for a scalar field.
/// * `value` - the value to store.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the message has no such
/// field, the element is past the end of an array, or an integer field is given a value
/// that is fractional, infinite, not a number, or outside the range its width holds.
///
/// # Safety
///
/// `message` must be a live message handle and `field` a null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_set_number(
    message: *mut PamojaMavlinkMessage,
    field: *const c_char,
    index: usize,
    value: f64,
) -> PamojaStatus {
    set(message, field, |dynamic, name| {
        dynamic.set_number(name, index, value)
    })
}

/// Copies the raw bytes of a byte-wide array field out.
///
/// This is how a `char` array carrying text is read: the bytes come back padded with zeros,
/// and the caller stops at the first one.
///
/// # Arguments
///
/// * `message` - the message to read.
/// * `field` - the field name.
/// * `out_bytes` - the destination, which must hold at least the field's length.
/// * `out_bytes_len` - the space available at `out_bytes`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the message has no such
/// field, the field is not a byte-wide array, or the destination is too small.
///
/// # Safety
///
/// `message` must be a live message handle, `field` a null-terminated C string, and
/// `out_bytes` must point at `out_bytes_len` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_get_bytes(
    message: *const PamojaMavlinkMessage,
    field: *const c_char,
    out_bytes: *mut u8,
    out_bytes_len: usize,
) -> PamojaStatus {
    let Some(message) = message.as_ref() else {
        set_last_error("message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(name) = read_str(field, "field") else {
        return PamojaStatus::InvalidArgument;
    };
    let out = std::slice::from_raw_parts_mut(out_bytes, out_bytes_len);
    let read = message.shape.with_descriptor(|shape| {
        DynamicMessage::decode(shape, &message.payload)?
            .get_bytes(name, out)
            .map(|_| ())
    });
    match read {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => status_of(error),
    }
}

/// Writes the raw bytes of a byte-wide array field, zero-padding the rest.
///
/// This is how a `char` array carrying text is written: pass the text's bytes and the field
/// is padded to its declared length.
///
/// # Arguments
///
/// * `message` - the message to write.
/// * `field` - the field name.
/// * `bytes` - the bytes to store, at most the field's declared length.
/// * `bytes_len` - the number of bytes to store.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the message has no such
/// field, the field is not a byte-wide array, or the bytes are longer than the field.
///
/// # Safety
///
/// `message` must be a live message handle, `field` a null-terminated C string, and
/// `bytes` must point at `bytes_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_message_set_bytes(
    message: *mut PamojaMavlinkMessage,
    field: *const c_char,
    bytes: *const u8,
    bytes_len: usize,
) -> PamojaStatus {
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    set(message, field, |dynamic, name| {
        dynamic.set_bytes(name, &bytes)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    unsafe fn schema_for(name: &str) -> *mut PamojaMavlinkSchema {
        let mut schema = std::ptr::null_mut();
        let wanted = CString::new(name).unwrap();
        assert_eq!(
            pamoja_mavlink_schema_for_name(wanted.as_ptr(), &mut schema),
            PamojaStatus::Ok
        );
        schema
    }

    #[test]
    fn a_typed_message_is_described_and_filled_in_by_name() {
        unsafe {
            let schema = schema_for("HEARTBEAT");
            assert_eq!(pamoja_mavlink_schema_id(schema), 0);
            assert_eq!(pamoja_mavlink_schema_crc_extra(schema), 50);
            assert_eq!(pamoja_mavlink_schema_wire_len(schema), 9);
            assert_eq!(pamoja_mavlink_schema_field_count(schema), 6);

            // The 32-bit field leads, as wire order puts it.
            let mut field = std::mem::zeroed::<PamojaMavlinkFieldInfo>();
            assert_eq!(
                pamoja_mavlink_schema_field(schema, 0, &mut field),
                PamojaStatus::Ok
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(field.name).to_str().unwrap(),
                "custom_mode"
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(field.type_name).to_str().unwrap(),
                "uint32_t"
            );
            assert_eq!(field.field_type, PAMOJA_MAVLINK_FIELD_UINT32);
            assert_eq!(field.offset, 0);
            assert_eq!(field.extension, 0);

            let mut message = std::ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_message_new(schema, &mut message),
                PamojaStatus::Ok
            );
            let kind = CString::new("type").unwrap();
            assert_eq!(
                pamoja_mavlink_message_set_uint(message, kind.as_ptr(), 0, 18),
                PamojaStatus::Ok
            );
            let mut value = 0u64;
            assert_eq!(
                pamoja_mavlink_message_get_uint(message, kind.as_ptr(), 0, &mut value),
                PamojaStatus::Ok
            );
            assert_eq!(value, 18);

            // Out of range for a uint8_t, and the message keeps its old bytes.
            assert_eq!(
                pamoja_mavlink_message_set_uint(message, kind.as_ptr(), 0, 300),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_mavlink_message_get_uint(message, kind.as_ptr(), 0, &mut value),
                PamojaStatus::Ok
            );
            assert_eq!(value, 18);

            let mut len = 0;
            let payload = pamoja_mavlink_message_payload(message, &mut len);
            assert_eq!(std::slice::from_raw_parts(payload, len)[4], 18);

            pamoja_mavlink_message_free(message);
            pamoja_mavlink_schema_free(schema);
        }
    }

    #[test]
    fn a_private_message_is_described_and_carried_like_any_other() {
        unsafe {
            let name = CString::new("BATTERY_CELLS").unwrap();
            let builder = pamoja_mavlink_schema_builder_new(50_000, name.as_ptr());
            assert!(!builder.is_null());

            let cells = CString::new("cell_mv").unwrap();
            let pack = CString::new("pack_id").unwrap();
            let uptime = CString::new("uptime_ms").unwrap();
            assert_eq!(
                pamoja_mavlink_schema_builder_field(
                    builder,
                    cells.as_ptr(),
                    PAMOJA_MAVLINK_FIELD_UINT16,
                    6
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_mavlink_schema_builder_field(
                    builder,
                    pack.as_ptr(),
                    PAMOJA_MAVLINK_FIELD_UINT8,
                    0
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_mavlink_schema_builder_field(
                    builder,
                    uptime.as_ptr(),
                    PAMOJA_MAVLINK_FIELD_UINT32,
                    0
                ),
                PamojaStatus::Ok
            );

            let mut schema = std::ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_schema_builder_build(builder, &mut schema),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_mavlink_schema_wire_len(schema), 17);

            // Wire order pulls the 32-bit field to the front.
            let mut field = std::mem::zeroed::<PamojaMavlinkFieldInfo>();
            assert_eq!(
                pamoja_mavlink_schema_field(schema, 0, &mut field),
                PamojaStatus::Ok
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(field.name).to_str().unwrap(),
                "uptime_ms"
            );

            let mut message = std::ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_message_new(schema, &mut message),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_mavlink_message_set_uint(message, cells.as_ptr(), 3, 4_150),
                PamojaStatus::Ok
            );
            let mut value = 0u64;
            assert_eq!(
                pamoja_mavlink_message_get_uint(message, cells.as_ptr(), 3, &mut value),
                PamojaStatus::Ok
            );
            assert_eq!(value, 4_150);

            pamoja_mavlink_message_free(message);
            pamoja_mavlink_schema_free(schema);
        }
    }

    #[test]
    fn an_unknown_message_or_field_is_reported_rather_than_guessed() {
        unsafe {
            let mut schema = std::ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_schema_for_id(50_000, &mut schema),
                PamojaStatus::Unsupported
            );

            let heartbeat = schema_for("HEARTBEAT");
            let mut message = std::ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_message_new(heartbeat, &mut message),
                PamojaStatus::Ok
            );
            let missing = CString::new("throttle").unwrap();
            let mut value = 0u64;
            assert_eq!(
                pamoja_mavlink_message_get_uint(message, missing.as_ptr(), 0, &mut value),
                PamojaStatus::InvalidArgument
            );
            pamoja_mavlink_message_free(message);
            pamoja_mavlink_schema_free(heartbeat);
        }
    }
}
