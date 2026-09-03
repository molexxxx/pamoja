//! MAVLink message shapes: reading and writing a message by field name.
//!
//! [`mavlink`](crate::mavlink) carries any message's bytes, which is enough to move traffic
//! but leaves the caller hand-packing payloads against a message definition. This is the
//! layer above: a schema states a message's fields, and a message reads and writes them by
//! name, so a caller works in `custom_mode` and `lat` rather than byte offsets.
//!
//! Every message the engine types is published, so a schema for the common dialect needs
//! nothing declared. A message from ArduPilot's dialect, PX4's, or a vendor's private one is
//! described through a builder, which puts the fields in wire order and derives the
//! `CRC_EXTRA` seed, so a caller transcribes a definition as it reads.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use pamoja_mavlink::dialect::{
    descriptor, descriptor_by_name, DynamicMessage, FieldType, MessageDescriptor,
    MessageDescriptorBuilder, OwnedMessageDescriptor, DESCRIPTORS,
};
use pamoja_mavlink::{Header, MavlinkError};

use crate::mavlink::{error_of, MavlinkFrame, MavlinkHeader};

/// A `uint8_t` field.
#[napi]
pub const MAVLINK_FIELD_UINT8: u32 = 1;
/// An `int8_t` field.
#[napi]
pub const MAVLINK_FIELD_INT8: u32 = 2;
/// A `char` field; an array of these carries text.
#[napi]
pub const MAVLINK_FIELD_CHAR: u32 = 3;
/// A `uint16_t` field.
#[napi]
pub const MAVLINK_FIELD_UINT16: u32 = 4;
/// An `int16_t` field.
#[napi]
pub const MAVLINK_FIELD_INT16: u32 = 5;
/// A `uint32_t` field.
#[napi]
pub const MAVLINK_FIELD_UINT32: u32 = 6;
/// An `int32_t` field.
#[napi]
pub const MAVLINK_FIELD_INT32: u32 = 7;
/// A `uint64_t` field.
#[napi]
pub const MAVLINK_FIELD_UINT64: u32 = 8;
/// An `int64_t` field.
#[napi]
pub const MAVLINK_FIELD_INT64: u32 = 9;
/// A `float` field.
#[napi]
pub const MAVLINK_FIELD_FLOAT: u32 = 10;
/// A `double` field.
#[napi]
pub const MAVLINK_FIELD_DOUBLE: u32 = 11;

/// Maps a field type onto the stable code a caller sees.
fn code_of(ty: FieldType) -> u32 {
    match ty {
        FieldType::U8 => MAVLINK_FIELD_UINT8,
        FieldType::I8 => MAVLINK_FIELD_INT8,
        FieldType::Char => MAVLINK_FIELD_CHAR,
        FieldType::U16 => MAVLINK_FIELD_UINT16,
        FieldType::I16 => MAVLINK_FIELD_INT16,
        FieldType::U32 => MAVLINK_FIELD_UINT32,
        FieldType::I32 => MAVLINK_FIELD_INT32,
        FieldType::U64 => MAVLINK_FIELD_UINT64,
        FieldType::I64 => MAVLINK_FIELD_INT64,
        FieldType::F32 => MAVLINK_FIELD_FLOAT,
        FieldType::F64 => MAVLINK_FIELD_DOUBLE,
    }
}

/// Resolves a field type code, reporting an unknown one as an error.
fn type_of(code: u32) -> Result<FieldType> {
    Ok(match code {
        MAVLINK_FIELD_UINT8 => FieldType::U8,
        MAVLINK_FIELD_INT8 => FieldType::I8,
        MAVLINK_FIELD_CHAR => FieldType::Char,
        MAVLINK_FIELD_UINT16 => FieldType::U16,
        MAVLINK_FIELD_INT16 => FieldType::I16,
        MAVLINK_FIELD_UINT32 => FieldType::U32,
        MAVLINK_FIELD_INT32 => FieldType::I32,
        MAVLINK_FIELD_UINT64 => FieldType::U64,
        MAVLINK_FIELD_INT64 => FieldType::I64,
        MAVLINK_FIELD_FLOAT => FieldType::F32,
        MAVLINK_FIELD_DOUBLE => FieldType::F64,
        _ => {
            return Err(Error::new(
                Status::InvalidArg,
                format!("{code} is not a MAVLink field type"),
            ))
        }
    })
}

/// One field of a message shape.
#[napi(object)]
pub struct MavlinkFieldInfo {
    /// The field name as the dialect writes it, such as `custom_mode`.
    pub name: String,
    /// The field's type name as the dialect writes it, such as `uint32_t`.
    pub type_name: String,
    /// The field's type, one of the `MAVLINK_FIELD_*` codes.
    pub field_type: u32,
    /// The element count for an array field, or `0` for a scalar.
    pub array_len: u8,
    /// Whether this is a MAVLink 2 extension field.
    pub extension: bool,
    /// The field's byte offset within the payload.
    pub offset: u32,
}

/// The shape of one message: its id, name, seed, and fields.
#[napi]
pub struct MessageSchema {
    shape: OwnedMessageDescriptor,
}

impl MessageSchema {
    /// Returns the shape this schema wraps, for another type in this crate to read.
    pub(crate) fn shape(&self) -> &OwnedMessageDescriptor {
        &self.shape
    }
}

#[napi]
impl MessageSchema {
    /// Returns the shape of a message the engine types, by id.
    ///
    /// @param msgid - The message id to look up.
    /// @returns The shape.
    /// @throws If this build does not type that id, which is what a builder is for.
    #[napi(factory)]
    pub fn for_id(msgid: u32) -> Result<Self> {
        let Some(shape) = descriptor(msgid) else {
            return Err(Error::new(
                Status::InvalidArg,
                format!("message {msgid} is not one this build types"),
            ));
        };
        Ok(Self {
            shape: OwnedMessageDescriptor::from_descriptor(shape),
        })
    }

    /// Returns the shape of a message the engine types, by name.
    ///
    /// @param name - The message name, such as `GLOBAL_POSITION_INT`.
    /// @returns The shape.
    /// @throws If this build does not type that name.
    #[napi(factory)]
    pub fn for_name(name: String) -> Result<Self> {
        let Some(shape) = descriptor_by_name(&name) else {
            return Err(Error::new(
                Status::InvalidArg,
                format!("{name} is not a message this build types"),
            ));
        };
        Ok(Self {
            shape: OwnedMessageDescriptor::from_descriptor(shape),
        })
    }

    /// The id of the message this schema describes.
    #[napi(getter)]
    pub fn id(&self) -> u32 {
        self.shape.id()
    }

    /// The name of the message this schema describes.
    #[napi(getter)]
    pub fn name(&self) -> String {
        self.shape.name().to_owned()
    }

    /// The `CRC_EXTRA` seed a frame carrying this message folds into its checksum.
    #[napi(getter)]
    pub fn crc_extra(&self) -> u8 {
        self.shape.crc_extra()
    }

    /// The length of the message on the wire, in bytes, extensions included.
    #[napi(getter)]
    pub fn wire_len(&self) -> u32 {
        self.shape.with_descriptor(|shape| shape.wire_len()) as u32
    }

    /// The fields in wire order: the base fields largest first, then any extensions.
    #[napi(getter)]
    pub fn fields(&self) -> Vec<MavlinkFieldInfo> {
        self.shape.with_descriptor(|shape| {
            let mut offset = 0;
            shape
                .fields
                .iter()
                .map(|field| {
                    let info = MavlinkFieldInfo {
                        name: field.name.to_owned(),
                        type_name: field.ty.wire_name().to_owned(),
                        field_type: code_of(field.ty),
                        array_len: field.array_len,
                        extension: field.extension,
                        offset: offset as u32,
                    };
                    offset += field.size();
                    info
                })
                .collect()
        })
    }
}

/// The names of every message this build types, in message-id order.
///
/// @returns The message names, each usable with `MessageSchema.forName`.
#[napi]
pub fn mavlink_known_messages() -> Vec<String> {
    DESCRIPTORS
        .iter()
        .map(|shape| shape.name.to_owned())
        .collect()
}

/// Describes a message this build does not type, one field at a time.
///
/// Fields are added in the order the message definition lists them; building puts them in
/// wire order and derives the `CRC_EXTRA` seed from the result.
#[napi]
pub struct MessageSchemaBuilder {
    builder: Option<MessageDescriptorBuilder>,
}

#[napi]
impl MessageSchemaBuilder {
    /// Starts describing a message.
    ///
    /// @param msgid - The message id on the wire.
    /// @param name - The message name, which the seed derivation folds in, so it must
    ///   match the dialect exactly.
    #[napi(constructor)]
    pub fn new(msgid: u32, name: String) -> Self {
        Self {
            builder: Some(MessageDescriptorBuilder::new(msgid, name)),
        }
    }

    /// Adds a base field, in the order the definition declares it.
    ///
    /// @param name - The field name.
    /// @param fieldType - The field's type, one of the `MAVLINK_FIELD_*` codes.
    /// @param arrayLen - The element count for an array, or `0` for a scalar.
    /// @throws If the type is not a MAVLink field type, or the shape is already built.
    #[napi]
    pub fn field(&mut self, name: String, field_type: u32, array_len: Option<u8>) -> Result<()> {
        self.add(name, field_type, array_len.unwrap_or(0), false)
    }

    /// Adds a MAVLink 2 extension field, in the order the definition declares it.
    ///
    /// Extensions keep their declared order, stay out of the `CRC_EXTRA` seed, and read as
    /// zero from a frame sent by a peer that predates them.
    ///
    /// @param name - The field name.
    /// @param fieldType - The field's type, one of the `MAVLINK_FIELD_*` codes.
    /// @param arrayLen - The element count for an array, or `0` for a scalar.
    /// @throws If the type is not a MAVLink field type, or the shape is already built.
    #[napi]
    pub fn extension(
        &mut self,
        name: String,
        field_type: u32,
        array_len: Option<u8>,
    ) -> Result<()> {
        self.add(name, field_type, array_len.unwrap_or(0), true)
    }

    /// Puts the declared fields in wire order and finishes the shape.
    ///
    /// @returns The finished schema.
    /// @throws If two fields share a name, the fields do not fit a MAVLink payload, or the
    ///   shape is already built.
    #[napi]
    pub fn build(&mut self) -> Result<MessageSchema> {
        let Some(builder) = self.builder.take() else {
            return Err(Error::new(
                Status::InvalidArg,
                "this builder has already been built",
            ));
        };
        let shape = builder.build().map_err(error_of)?;
        Ok(MessageSchema { shape })
    }

    fn add(&mut self, name: String, field_type: u32, array_len: u8, extension: bool) -> Result<()> {
        let ty = type_of(field_type)?;
        let Some(builder) = self.builder.take() else {
            return Err(Error::new(
                Status::InvalidArg,
                "this builder has already been built",
            ));
        };
        self.builder = Some(if extension {
            builder.extension(name, ty, array_len)
        } else {
            builder.field(name, ty, array_len)
        });
        Ok(())
    }
}

/// A message read and written by field name against a schema.
#[napi]
pub struct MavlinkMessage {
    shape: OwnedMessageDescriptor,
    payload: Vec<u8>,
}

impl MavlinkMessage {
    /// Wraps a message the engine decoded, for another module in this crate to hand back.
    pub(crate) fn from_typed(shape: &MessageDescriptor<'static>, payload: Vec<u8>) -> Self {
        Self {
            shape: OwnedMessageDescriptor::from_descriptor(shape),
            payload,
        }
    }
}

#[napi]
impl MavlinkMessage {
    /// Creates a message with every field zero.
    ///
    /// @param schema - The shape of the message to build.
    /// @returns The zeroed message, ready for its fields to be set.
    /// @throws If the shape does not fit a MAVLink payload.
    #[napi(factory)]
    pub fn empty(schema: &MessageSchema) -> Result<Self> {
        let shape = schema.shape().clone();
        let payload = shape
            .with_descriptor(|view| DynamicMessage::new(view).map(|m| m.payload().to_vec()))
            .map_err(error_of)?;
        Ok(Self { shape, payload })
    }

    /// Reads a message out of a frame payload.
    ///
    /// A payload shorter than the shape is zero-extended, as MAVLink 2 truncation requires,
    /// so a frame from a peer that trimmed trailing zeros or predates an extension field
    /// still decodes.
    ///
    /// @param schema - The shape to read the payload as.
    /// @param payload - The frame payload.
    /// @returns The decoded message.
    /// @throws If the payload is longer than the shape describes.
    #[napi(factory)]
    pub fn decode(schema: &MessageSchema, payload: Buffer) -> Result<Self> {
        let shape = schema.shape().clone();
        let payload = shape
            .with_descriptor(|view| {
                DynamicMessage::decode(view, &payload).map(|m| m.payload().to_vec())
            })
            .map_err(error_of)?;
        Ok(Self { shape, payload })
    }

    /// The id of the message this carries.
    #[napi(getter)]
    pub fn message_id(&self) -> u32 {
        self.shape.id()
    }

    /// The name of the message this carries.
    #[napi(getter)]
    pub fn name(&self) -> String {
        self.shape.name().to_owned()
    }

    /// The message's bytes as they go on the wire.
    #[napi(getter)]
    pub fn payload(&self) -> Buffer {
        Buffer::from(self.payload.clone())
    }

    /// Builds a v2 frame carrying this message.
    ///
    /// @param header - The addressing fields to stamp on the frame.
    /// @returns The frame ready to send.
    /// @throws If the message does not fit a frame.
    #[napi]
    pub fn to_frame(&self, header: MavlinkHeader) -> Result<MavlinkFrame> {
        let frame = self
            .shape
            .with_descriptor(|view| {
                DynamicMessage::decode(view, &self.payload)?.to_frame(Header::from(header))
            })
            .map_err(error_of)?;
        Ok(MavlinkFrame::from_frame(frame))
    }

    /// Reads a field as a number.
    ///
    /// Every field reads this way. An integer field wider than 53 bits can exceed what a
    /// JavaScript number holds exactly, which no common-dialect field reaches in practice.
    ///
    /// @param field - The field name.
    /// @param index - The element to read, or `0` for a scalar field.
    /// @returns The value.
    /// @throws If the message has no such field, or the element is past the end of an array.
    #[napi]
    pub fn get(&self, field: String, index: Option<u32>) -> Result<f64> {
        self.read(|message| message.get_number(&field, index.unwrap_or(0) as usize))
    }

    /// Writes a number into a field, converting it to the field's type.
    ///
    /// A value bound for an integer field must be a whole number within that field's range,
    /// so a fractional or oversized value is refused rather than silently truncated.
    ///
    /// @param field - The field name.
    /// @param value - The value to store.
    /// @param index - The element to write, or `0` for a scalar field.
    /// @throws If the message has no such field, the element is past the end of an array,
    ///   or an integer field cannot hold the value exactly.
    #[napi]
    pub fn set(&mut self, field: String, value: f64, index: Option<u32>) -> Result<()> {
        self.write(|message| message.set_number(&field, index.unwrap_or(0) as usize, value))
    }

    /// Copies the raw bytes of a byte-wide array field out.
    ///
    /// @param field - The field name.
    /// @returns The bytes, including the zero padding.
    /// @throws If the message has no such field, or it is not a byte-wide array.
    #[napi]
    pub fn get_bytes(&self, field: String) -> Result<Buffer> {
        let mut out = vec![0u8; pamoja_mavlink::MAX_PAYLOAD];
        let len = self
            .shape
            .with_descriptor(|view| {
                DynamicMessage::decode(view, &self.payload)?.get_bytes(&field, &mut out)
            })
            .map_err(error_of)?;
        out.truncate(len);
        Ok(Buffer::from(out))
    }

    /// Writes the raw bytes of a byte-wide array field, zero-padding the rest.
    ///
    /// @param field - The field name.
    /// @param bytes - The bytes to store, at most the field's declared length.
    /// @throws If the message has no such field, it is not a byte-wide array, or the bytes
    ///   are longer than the field.
    #[napi]
    pub fn set_bytes(&mut self, field: String, bytes: Buffer) -> Result<()> {
        self.write(|message| message.set_bytes(&field, &bytes))
    }

    /// Reads a `char` array as text, stopping at the padding.
    ///
    /// @param field - The field name.
    /// @returns The text, without its padding.
    /// @throws If the message has no such field, it is not a `char` array, or the bytes are
    ///   not valid UTF-8.
    #[napi]
    pub fn get_text(&self, field: String) -> Result<String> {
        self.read(|message| message.text(&field).map(str::to_owned))
    }

    /// Writes text into a `char` array, padding the rest with zeros.
    ///
    /// @param field - The field name.
    /// @param text - The text to store, at most the field's declared length.
    /// @throws If the message has no such field, it is not a `char` array, or the text is
    ///   longer than the field.
    #[napi]
    pub fn set_text(&mut self, field: String, text: String) -> Result<()> {
        self.write(|message| message.set_text(&field, &text))
    }

    fn read<T>(
        &self,
        query: impl FnOnce(&DynamicMessage<'_>) -> pamoja_mavlink::Result<T>,
    ) -> Result<T> {
        self.shape
            .with_descriptor(|view| query(&DynamicMessage::decode(view, &self.payload)?))
            .map_err(error_of)
    }

    fn write(
        &mut self,
        step: impl FnOnce(&mut DynamicMessage<'_>) -> pamoja_mavlink::Result<()>,
    ) -> Result<()> {
        let updated = self.shape.with_descriptor(|view| {
            let mut message = DynamicMessage::decode(view, &self.payload)?;
            step(&mut message)?;
            Ok::<Vec<u8>, MavlinkError>(message.payload().to_vec())
        });
        match updated {
            Ok(payload) => {
                self.payload = payload;
                Ok(())
            }
            Err(error) => Err(error_of(error)),
        }
    }
}
