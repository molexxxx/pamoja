//! Message shape as data: read and write a message by field name, with no typed struct.
//!
//! A [`Message`](crate::dialect::Message) implementation is the fastest way to send a
//! message this crate types. It is also the only way, which is a problem the moment a
//! caller needs a message from ArduPilot's dialect, PX4's, or a vendor's private one. This
//! module removes that limit: a [`MessageDescriptor`] states a message's id, name, and
//! fields, and [`DynamicMessage`] encodes and decodes against it, so any message from any
//! dialect is usable once its shape is described.
//!
//! Every typed message publishes its own descriptor, so the two layers agree by
//! construction: [`descriptor`] resolves an id from the common dialect, and a test holds
//! each descriptor to the same bytes its typed struct produces.
//!
//! A dialect describes its fields in declaration order, but MAVLink puts them on the wire
//! largest first. [`MessageDescriptorBuilder`] applies that reordering and derives the
//! `CRC_EXTRA` seed from the result, so a caller transcribing a message definition writes
//! it the way the definition reads.

use super::DESCRIPTORS;
use crate::crc::crc_extra_of;
use crate::error::{MavlinkError, Result};
use crate::frame::{Frame, Header, MAX_PAYLOAD};

#[cfg(feature = "alloc")]
mod owned;

#[cfg(feature = "alloc")]
pub use owned::{
    MessageDescriptorBuilder, OwnedDialect, OwnedFieldDescriptor, OwnedMessageDescriptor,
};

/// The scalar type of a message field, as a dialect declares it.
///
/// The name is the C type MAVLink writes in a message definition, which is also what the
/// `CRC_EXTRA` derivation folds in, so the spelling is part of the wire contract rather
/// than a label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldType {
    /// `uint8_t`, one unsigned byte.
    U8,
    /// `int8_t`, one signed byte.
    I8,
    /// `char`, one byte of text; an array of these carries a string.
    Char,
    /// `uint16_t`.
    U16,
    /// `int16_t`.
    I16,
    /// `uint32_t`.
    U32,
    /// `int32_t`.
    I32,
    /// `uint64_t`.
    U64,
    /// `int64_t`.
    I64,
    /// `float`, IEEE 754 single precision.
    F32,
    /// `double`, IEEE 754 double precision.
    F64,
}

impl FieldType {
    /// Returns the size of one element of this type, in bytes.
    ///
    /// # Returns
    ///
    /// The element size, from one to eight bytes.
    pub const fn size(self) -> usize {
        match self {
            Self::U8 | Self::I8 | Self::Char => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 | Self::F32 => 4,
            Self::U64 | Self::I64 | Self::F64 => 8,
        }
    }

    /// Returns the C type name a dialect writes for this type.
    ///
    /// # Returns
    ///
    /// The name, such as `"uint8_t"` or `"float"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_mavlink::dialect::FieldType;
    ///
    /// assert_eq!(FieldType::F32.wire_name(), "float");
    /// assert_eq!(FieldType::U64.wire_name(), "uint64_t");
    /// ```
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::U8 => "uint8_t",
            Self::I8 => "int8_t",
            Self::Char => "char",
            Self::U16 => "uint16_t",
            Self::I16 => "int16_t",
            Self::U32 => "uint32_t",
            Self::I32 => "int32_t",
            Self::U64 => "uint64_t",
            Self::I64 => "int64_t",
            Self::F32 => "float",
            Self::F64 => "double",
        }
    }

    /// Resolves a C type name from a message definition.
    ///
    /// An array declaration carries its length separately, so `"uint8_t[4]"` is written as
    /// `"uint8_t"` with a length of four rather than parsed here.
    ///
    /// # Arguments
    ///
    /// * `name` - the C type name, such as `"uint16_t"`.
    ///
    /// # Returns
    ///
    /// The type, or [`None`] if the name is not one MAVLink defines.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_mavlink::dialect::FieldType;
    ///
    /// assert_eq!(FieldType::from_wire_name("int32_t"), Some(FieldType::I32));
    /// assert_eq!(FieldType::from_wire_name("size_t"), None);
    /// ```
    pub fn from_wire_name(name: &str) -> Option<Self> {
        Some(match name {
            "uint8_t" | "uint8_t_mavlink_version" => Self::U8,
            "int8_t" => Self::I8,
            "char" => Self::Char,
            "uint16_t" => Self::U16,
            "int16_t" => Self::I16,
            "uint32_t" => Self::U32,
            "int32_t" => Self::I32,
            "uint64_t" => Self::U64,
            "int64_t" => Self::I64,
            "float" => Self::F32,
            "double" => Self::F64,
            _ => return None,
        })
    }

    /// Reports whether values of this type are whole numbers.
    ///
    /// # Returns
    ///
    /// `true` for every type but `float` and `double`.
    pub const fn is_integer(self) -> bool {
        !matches!(self, Self::F32 | Self::F64)
    }

    /// Reports whether values of this type can be negative.
    ///
    /// # Returns
    ///
    /// `true` for the signed integer types and the two floating-point types.
    pub const fn is_signed(self) -> bool {
        matches!(
            self,
            Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::F32 | Self::F64
        )
    }
}

/// One field of a message: its name, type, and place in the layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldDescriptor<'a> {
    /// The field name as the dialect writes it, such as `"custom_mode"`.
    pub name: &'a str,

    /// The field's scalar type, which for an array is its element type.
    pub ty: FieldType,

    /// The element count for an array field, or `0` for a scalar.
    pub array_len: u8,

    /// Whether this is a MAVLink 2 extension field.
    ///
    /// Extension fields sit after the base fields in declaration order, are excluded from
    /// the `CRC_EXTRA` seed, and may be absent from a frame sent by an older peer, in
    /// which case they read as zero.
    pub extension: bool,
}

impl FieldDescriptor<'_> {
    /// Returns how many elements the field holds.
    ///
    /// # Returns
    ///
    /// The array length, or `1` for a scalar.
    pub const fn elements(&self) -> usize {
        if self.array_len == 0 {
            1
        } else {
            self.array_len as usize
        }
    }

    /// Returns the total size of the field on the wire, in bytes.
    ///
    /// # Returns
    ///
    /// The element size multiplied by the element count.
    pub const fn size(&self) -> usize {
        self.elements() * self.ty.size()
    }
}

/// The shape of one message: what a sender fills in and a receiver reads back.
///
/// Fields are held in wire order, which for the base fields is largest type first. A
/// descriptor written by hand must already be in that order;
/// [`MessageDescriptorBuilder`] does the reordering for a definition written the way a
/// dialect reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageDescriptor<'a> {
    /// The message id on the wire.
    pub id: u32,

    /// The message name, such as `"HEARTBEAT"`.
    pub name: &'a str,

    /// The `CRC_EXTRA` seed folded into the checksum of a frame carrying this message.
    pub crc_extra: u8,

    /// The fields in wire order: the base fields largest first, then any extensions.
    pub fields: &'a [FieldDescriptor<'a>],
}

impl<'a> MessageDescriptor<'a> {
    /// Returns the full length of the message on the wire, extensions included.
    ///
    /// # Returns
    ///
    /// The sum of every field's size, in bytes.
    pub fn wire_len(&self) -> usize {
        self.fields.iter().map(FieldDescriptor::size).sum()
    }

    /// Returns the length of the message's base fields, in bytes.
    ///
    /// This is what a peer that predates the extensions expects, and the length the
    /// `CRC_EXTRA` seed describes.
    ///
    /// # Returns
    ///
    /// The sum of the base fields' sizes.
    pub fn base_len(&self) -> usize {
        self.fields
            .iter()
            .filter(|field| !field.extension)
            .map(FieldDescriptor::size)
            .sum()
    }

    /// Looks a field up by name.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name to find.
    ///
    /// # Returns
    ///
    /// The field, or [`None`] if the message has no field of that name.
    pub fn field(&self, name: &str) -> Option<&'a FieldDescriptor<'a>> {
        self.fields.iter().find(|field| field.name == name)
    }

    /// Returns the byte offset of a field within the payload.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name to locate.
    ///
    /// # Returns
    ///
    /// The offset, or [`None`] if the message has no field of that name.
    pub fn offset_of(&self, name: &str) -> Option<usize> {
        let mut offset = 0;
        for field in self.fields {
            if field.name == name {
                return Some(offset);
            }
            offset += field.size();
        }
        None
    }

    /// Derives the `CRC_EXTRA` seed from this descriptor's base fields.
    ///
    /// A descriptor whose [`crc_extra`](Self::crc_extra) differs from this has a field
    /// wrong: a mistyped field, a wrong array length, or fields out of wire order.
    ///
    /// # Returns
    ///
    /// The seed the fields imply.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_mavlink::dialect::descriptor;
    ///
    /// let heartbeat = descriptor(0).expect("HEARTBEAT is in the common dialect");
    /// assert_eq!(heartbeat.derived_crc_extra(), heartbeat.crc_extra);
    /// ```
    pub fn derived_crc_extra(&self) -> u8 {
        crc_extra_of(
            self.name,
            self.fields
                .iter()
                .filter(|field| !field.extension)
                .map(|field| (field.ty.wire_name(), field.name, field.array_len)),
        )
    }

    fn locate(&self, name: &str) -> Result<(usize, &'a FieldDescriptor<'a>)> {
        let mut offset = 0;
        for field in self.fields {
            if field.name == name {
                return Ok((offset, field));
            }
            offset += field.size();
        }
        Err(MavlinkError::UnknownField)
    }
}

// The first double past the signed and unsigned 64-bit ranges. A cast saturates instead of
// failing, so the bound is checked before the cast rather than after it.
const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;
const TWO_POW_64: f64 = 18_446_744_073_709_551_616.0;

/// A field's value, whichever of the three kinds its type calls for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FieldValue {
    /// The value of a signed integer field.
    Int(i64),
    /// The value of an unsigned integer or `char` field.
    Uint(u64),
    /// The value of a `float` or `double` field.
    Float(f64),
}

/// A message read and written by field name against a [`MessageDescriptor`].
///
/// This is the counterpart to a typed message: the same bytes, reached by name at runtime
/// rather than through a struct known at compile time. It carries a full-size payload
/// buffer and no allocation, so it works on a microcontroller as well as a ground station.
///
/// # Examples
///
/// ```
/// use pamoja_mavlink::dialect::{descriptor, DynamicMessage, Heartbeat, Message};
/// use pamoja_mavlink::Header;
///
/// let shape = descriptor(Heartbeat::ID).expect("HEARTBEAT is in the common dialect");
///
/// // Fill the message in by name, the way a caller reading a dialect definition would.
/// let mut heartbeat = DynamicMessage::new(shape)?;
/// heartbeat.set_uint("type", 0, 18)?; // MAV_TYPE_ONBOARD_CONTROLLER
/// heartbeat.set_uint("system_status", 0, 4)?; // MAV_STATE_ACTIVE
/// heartbeat.set_uint("mavlink_version", 0, 3)?;
///
/// // It is an ordinary frame, so a typed receiver reads it back unchanged.
/// let frame = heartbeat.to_frame(Header::new(1, 1, 0))?;
/// assert_eq!(Heartbeat::decode(frame.payload())?.system_status, 4);
/// # Ok::<(), pamoja_mavlink::MavlinkError>(())
/// ```
#[derive(Clone, Debug)]
pub struct DynamicMessage<'a> {
    descriptor: &'a MessageDescriptor<'a>,
    payload: [u8; MAX_PAYLOAD],
    len: usize,
}

impl<'a> DynamicMessage<'a> {
    /// Creates a message with every field zero.
    ///
    /// # Arguments
    ///
    /// * `descriptor` - the shape of the message to build.
    ///
    /// # Returns
    ///
    /// The zeroed message, ready for its fields to be set.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::PayloadTooLong`] if the descriptor's fields exceed
    /// [`MAX_PAYLOAD`] bytes, which no message from a valid dialect does.
    pub fn new(descriptor: &'a MessageDescriptor<'a>) -> Result<Self> {
        let len = descriptor.wire_len();
        if len > MAX_PAYLOAD {
            return Err(MavlinkError::PayloadTooLong);
        }
        Ok(Self {
            descriptor,
            payload: [0; MAX_PAYLOAD],
            len,
        })
    }

    /// Reads a message out of a frame payload.
    ///
    /// A short payload is zero-extended, as MAVLink 2 truncation requires, so a frame from
    /// a peer that omitted trailing zeros or predates an extension field still decodes.
    ///
    /// # Arguments
    ///
    /// * `descriptor` - the shape to read the payload as.
    /// * `payload` - the frame payload.
    ///
    /// # Returns
    ///
    /// The decoded message.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::BadPayload`] if the payload is longer than the descriptor
    /// describes, and [`MavlinkError::PayloadTooLong`] if the descriptor itself does not
    /// fit a frame.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_mavlink::dialect::{descriptor, DynamicMessage};
    /// use pamoja_mavlink::Frame;
    ///
    /// let shape = descriptor(0).expect("HEARTBEAT is in the common dialect");
    /// let received = Frame::parse(
    ///     &[0xfd, 0x09, 0, 0, 7, 1, 1, 0, 0, 0, 0, 0, 0, 0, 18, 0, 0, 4, 3, 0x75, 0x3a],
    ///     shape.crc_extra,
    /// )?;
    ///
    /// let heartbeat = DynamicMessage::decode(shape, received.payload())?;
    /// assert_eq!(heartbeat.get_uint("type", 0)?, 18);
    /// # Ok::<(), pamoja_mavlink::MavlinkError>(())
    /// ```
    pub fn decode(descriptor: &'a MessageDescriptor<'a>, payload: &[u8]) -> Result<Self> {
        let len = descriptor.wire_len();
        if len > MAX_PAYLOAD {
            return Err(MavlinkError::PayloadTooLong);
        }
        if payload.len() > len {
            return Err(MavlinkError::BadPayload);
        }
        let mut message = Self {
            descriptor,
            payload: [0; MAX_PAYLOAD],
            len,
        };
        message.payload[..payload.len()].copy_from_slice(payload);
        Ok(message)
    }

    /// Returns the shape this message is read and written against.
    ///
    /// # Returns
    ///
    /// The descriptor.
    pub fn descriptor(&self) -> &'a MessageDescriptor<'a> {
        self.descriptor
    }

    /// Returns the message's bytes as they go on the wire.
    ///
    /// # Returns
    ///
    /// The payload, including any trailing zeros; a frame truncates those itself.
    pub fn payload(&self) -> &[u8] {
        &self.payload[..self.len]
    }

    /// Builds a v2 frame carrying this message.
    ///
    /// # Arguments
    ///
    /// * `header` - the addressing fields to stamp on the frame.
    ///
    /// # Returns
    ///
    /// The frame ready to send.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::PayloadTooLong`] if the message does not fit a frame.
    pub fn to_frame(&self, header: Header) -> Result<Frame> {
        Frame::encode_v2(
            header,
            self.descriptor.id,
            self.payload(),
            self.descriptor.crc_extra,
        )
    }

    /// Reads a field as a signed integer.
    ///
    /// Any integer field can be read this way, whatever its width or sign.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `index` - the element to read, or `0` for a scalar field.
    ///
    /// # Returns
    ///
    /// The value.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field,
    /// [`MavlinkError::FieldIndexOutOfRange`] if the element is past the end of an array,
    /// [`MavlinkError::FieldTypeMismatch`] for a floating-point field, and
    /// [`MavlinkError::ValueOutOfRange`] for a `uint64_t` value above [`i64::MAX`].
    pub fn get_int(&self, name: &str, index: usize) -> Result<i64> {
        let (offset, field) = self.element(name, index)?;
        if !field.ty.is_integer() {
            return Err(MavlinkError::FieldTypeMismatch);
        }
        Ok(match field.ty {
            FieldType::I8 => self.read::<1>(offset)[0] as i8 as i64,
            FieldType::I16 => i16::from_le_bytes(self.read::<2>(offset)) as i64,
            FieldType::I32 => i32::from_le_bytes(self.read::<4>(offset)) as i64,
            FieldType::I64 => i64::from_le_bytes(self.read::<8>(offset)),
            FieldType::U8 | FieldType::Char => self.read::<1>(offset)[0] as i64,
            FieldType::U16 => u16::from_le_bytes(self.read::<2>(offset)) as i64,
            FieldType::U32 => u32::from_le_bytes(self.read::<4>(offset)) as i64,
            FieldType::U64 => i64::try_from(u64::from_le_bytes(self.read::<8>(offset)))
                .map_err(|_| MavlinkError::ValueOutOfRange)?,
            FieldType::F32 | FieldType::F64 => unreachable!(),
        })
    }

    /// Reads a field as an unsigned integer.
    ///
    /// Any integer field can be read this way, whatever its width or sign.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `index` - the element to read, or `0` for a scalar field.
    ///
    /// # Returns
    ///
    /// The value.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field,
    /// [`MavlinkError::FieldIndexOutOfRange`] if the element is past the end of an array,
    /// [`MavlinkError::FieldTypeMismatch`] for a floating-point field, and
    /// [`MavlinkError::ValueOutOfRange`] for a negative value.
    pub fn get_uint(&self, name: &str, index: usize) -> Result<u64> {
        let (offset, field) = self.element(name, index)?;
        if !field.ty.is_integer() {
            return Err(MavlinkError::FieldTypeMismatch);
        }
        if field.ty.is_signed() {
            return u64::try_from(self.get_int(name, index)?)
                .map_err(|_| MavlinkError::ValueOutOfRange);
        }
        Ok(match field.ty {
            FieldType::U8 | FieldType::Char => self.read::<1>(offset)[0] as u64,
            FieldType::U16 => u16::from_le_bytes(self.read::<2>(offset)) as u64,
            FieldType::U32 => u32::from_le_bytes(self.read::<4>(offset)) as u64,
            FieldType::U64 => u64::from_le_bytes(self.read::<8>(offset)),
            _ => unreachable!(),
        })
    }

    /// Reads a floating-point field.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `index` - the element to read, or `0` for a scalar field.
    ///
    /// # Returns
    ///
    /// The value, widened to double precision for a `float` field.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field,
    /// [`MavlinkError::FieldIndexOutOfRange`] if the element is past the end of an array,
    /// and [`MavlinkError::FieldTypeMismatch`] for an integer field.
    pub fn get_float(&self, name: &str, index: usize) -> Result<f64> {
        let (offset, field) = self.element(name, index)?;
        Ok(match field.ty {
            FieldType::F32 => f32::from_le_bytes(self.read::<4>(offset)) as f64,
            FieldType::F64 => f64::from_le_bytes(self.read::<8>(offset)),
            _ => return Err(MavlinkError::FieldTypeMismatch),
        })
    }

    /// Reads a field as whichever kind of value its type calls for.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `index` - the element to read, or `0` for a scalar field.
    ///
    /// # Returns
    ///
    /// The value.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field, and
    /// [`MavlinkError::FieldIndexOutOfRange`] if the element is past the end of an array.
    pub fn get(&self, name: &str, index: usize) -> Result<FieldValue> {
        let (_, field) = self.element(name, index)?;
        Ok(if !field.ty.is_integer() {
            FieldValue::Float(self.get_float(name, index)?)
        } else if field.ty.is_signed() {
            FieldValue::Int(self.get_int(name, index)?)
        } else {
            FieldValue::Uint(self.get_uint(name, index)?)
        })
    }

    /// Writes a signed integer into a field.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `index` - the element to write, or `0` for a scalar field.
    /// * `value` - the value to store.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field,
    /// [`MavlinkError::FieldIndexOutOfRange`] if the element is past the end of an array,
    /// [`MavlinkError::FieldTypeMismatch`] for a floating-point field, and
    /// [`MavlinkError::ValueOutOfRange`] if the value does not fit the field's type.
    pub fn set_int(&mut self, name: &str, index: usize, value: i64) -> Result<()> {
        let (offset, field) = self.element(name, index)?;
        let ty = field.ty;
        if !ty.is_integer() {
            return Err(MavlinkError::FieldTypeMismatch);
        }
        let out = MavlinkError::ValueOutOfRange;
        match ty {
            FieldType::I8 => {
                self.write(offset, &i8::try_from(value).map_err(|_| out)?.to_le_bytes())
            }
            FieldType::I16 => self.write(
                offset,
                &i16::try_from(value).map_err(|_| out)?.to_le_bytes(),
            ),
            FieldType::I32 => self.write(
                offset,
                &i32::try_from(value).map_err(|_| out)?.to_le_bytes(),
            ),
            FieldType::I64 => self.write(offset, &value.to_le_bytes()),
            _ => return self.set_uint(name, index, u64::try_from(value).map_err(|_| out)?),
        }
        Ok(())
    }

    /// Writes an unsigned integer into a field.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `index` - the element to write, or `0` for a scalar field.
    /// * `value` - the value to store.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field,
    /// [`MavlinkError::FieldIndexOutOfRange`] if the element is past the end of an array,
    /// [`MavlinkError::FieldTypeMismatch`] for a floating-point field, and
    /// [`MavlinkError::ValueOutOfRange`] if the value does not fit the field's type.
    pub fn set_uint(&mut self, name: &str, index: usize, value: u64) -> Result<()> {
        let (offset, field) = self.element(name, index)?;
        let ty = field.ty;
        if !ty.is_integer() {
            return Err(MavlinkError::FieldTypeMismatch);
        }
        let out = MavlinkError::ValueOutOfRange;
        match ty {
            FieldType::U8 | FieldType::Char => {
                self.write(offset, &u8::try_from(value).map_err(|_| out)?.to_le_bytes())
            }
            FieldType::U16 => self.write(
                offset,
                &u16::try_from(value).map_err(|_| out)?.to_le_bytes(),
            ),
            FieldType::U32 => self.write(
                offset,
                &u32::try_from(value).map_err(|_| out)?.to_le_bytes(),
            ),
            FieldType::U64 => self.write(offset, &value.to_le_bytes()),
            _ => return self.set_int(name, index, i64::try_from(value).map_err(|_| out)?),
        }
        Ok(())
    }

    /// Writes a floating-point field.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `index` - the element to write, or `0` for a scalar field.
    /// * `value` - the value to store, narrowed to single precision for a `float` field.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field,
    /// [`MavlinkError::FieldIndexOutOfRange`] if the element is past the end of an array,
    /// and [`MavlinkError::FieldTypeMismatch`] for an integer field.
    pub fn set_float(&mut self, name: &str, index: usize, value: f64) -> Result<()> {
        let (offset, field) = self.element(name, index)?;
        match field.ty {
            FieldType::F32 => self.write(offset, &(value as f32).to_le_bytes()),
            FieldType::F64 => self.write(offset, &value.to_le_bytes()),
            _ => return Err(MavlinkError::FieldTypeMismatch),
        }
        Ok(())
    }

    /// Writes a value into a field, whichever kind it is.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `index` - the element to write, or `0` for a scalar field.
    /// * `value` - the value to store.
    ///
    /// # Errors
    ///
    /// Returns the same errors as the typed setter for the value's kind.
    pub fn set(&mut self, name: &str, index: usize, value: FieldValue) -> Result<()> {
        match value {
            FieldValue::Int(value) => self.set_int(name, index, value),
            FieldValue::Uint(value) => self.set_uint(name, index, value),
            FieldValue::Float(value) => self.set_float(name, index, value),
        }
    }

    /// Reads a field as a double, whatever its type.
    ///
    /// This is the reading a host language with one numeric type needs. An integer field
    /// wider than 53 bits can exceed what a double represents exactly, so read those with
    /// [`get_int`](Self::get_int) or [`get_uint`](Self::get_uint) where the exact value
    /// matters.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `index` - the element to read, or `0` for a scalar field.
    ///
    /// # Returns
    ///
    /// The value as a double.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field, and
    /// [`MavlinkError::FieldIndexOutOfRange`] if the element is past the end of an array.
    pub fn get_number(&self, name: &str, index: usize) -> Result<f64> {
        Ok(match self.get(name, index)? {
            FieldValue::Int(value) => value as f64,
            FieldValue::Uint(value) => value as f64,
            FieldValue::Float(value) => value,
        })
    }

    /// Writes a double into a field, converting it to the field's type.
    ///
    /// This is the writing a host language with one numeric type needs. A value bound for
    /// an integer field must be a whole number within that field's range, so a fractional
    /// or oversized value is refused rather than silently truncated.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `index` - the element to write, or `0` for a scalar field.
    /// * `value` - the value to store.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field,
    /// [`MavlinkError::FieldIndexOutOfRange`] if the element is past the end of an array,
    /// and [`MavlinkError::ValueOutOfRange`] if an integer field is given a value that is
    /// fractional, infinite, not a number, or outside the range its width holds.
    pub fn set_number(&mut self, name: &str, index: usize, value: f64) -> Result<()> {
        let (_, field) = self.element(name, index)?;
        if !field.ty.is_integer() {
            return self.set_float(name, index, value);
        }
        // Range first, because a cast saturates rather than failing, then the round trip,
        // which is what rejects a fractional value without needing a floating-point library.
        if field.ty.is_signed() {
            if !(-TWO_POW_63..TWO_POW_63).contains(&value) {
                return Err(MavlinkError::ValueOutOfRange);
            }
            let whole = value as i64;
            if whole as f64 != value {
                return Err(MavlinkError::ValueOutOfRange);
            }
            self.set_int(name, index, whole)
        } else {
            if !(0.0..TWO_POW_64).contains(&value) {
                return Err(MavlinkError::ValueOutOfRange);
            }
            let whole = value as u64;
            if whole as f64 != value {
                return Err(MavlinkError::ValueOutOfRange);
            }
            self.set_uint(name, index, whole)
        }
    }

    /// Copies the raw bytes of a byte-wide array field out.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `out` - the destination, which must be at least the field's length.
    ///
    /// # Returns
    ///
    /// The number of bytes written, which is the field's declared length.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field,
    /// [`MavlinkError::FieldTypeMismatch`] if the field is not an array of `char`,
    /// `uint8_t`, or `int8_t`, and [`MavlinkError::PayloadTooLong`] if `out` is too small.
    pub fn get_bytes(&self, name: &str, out: &mut [u8]) -> Result<usize> {
        let (offset, field) = self.descriptor.locate(name)?;
        if field.ty.size() != 1 || field.array_len == 0 {
            return Err(MavlinkError::FieldTypeMismatch);
        }
        let len = field.elements();
        if out.len() < len {
            return Err(MavlinkError::PayloadTooLong);
        }
        out[..len].copy_from_slice(&self.payload[offset..offset + len]);
        Ok(len)
    }

    /// Writes the raw bytes of a byte-wide array field, zero-padding the rest.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `bytes` - the bytes to store, at most the field's declared length.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field,
    /// [`MavlinkError::FieldTypeMismatch`] if the field is not an array of `char`,
    /// `uint8_t`, or `int8_t`, and [`MavlinkError::PayloadTooLong`] if the bytes are
    /// longer than the field.
    pub fn set_bytes(&mut self, name: &str, bytes: &[u8]) -> Result<()> {
        let (offset, field) = self.descriptor.locate(name)?;
        if field.ty.size() != 1 || field.array_len == 0 {
            return Err(MavlinkError::FieldTypeMismatch);
        }
        let len = field.elements();
        if bytes.len() > len {
            return Err(MavlinkError::PayloadTooLong);
        }
        self.payload[offset..offset + len].fill(0);
        self.payload[offset..offset + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    /// Reads a `char` array as text, stopping at the padding.
    ///
    /// MAVLink carries a string in a fixed-length `char` array, padded with zeros when the
    /// text is shorter and left unterminated when it exactly fills the field.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    ///
    /// # Returns
    ///
    /// The text, without its padding.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field,
    /// [`MavlinkError::FieldTypeMismatch`] if the field is not a `char` array, and
    /// [`MavlinkError::BadPayload`] if the bytes are not valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_mavlink::dialect::{descriptor, DynamicMessage};
    ///
    /// let shape = descriptor(253).expect("STATUSTEXT is in the common dialect");
    /// let mut status = DynamicMessage::new(shape)?;
    /// status.set_text("text", "preflight checks passed")?;
    /// assert_eq!(status.text("text")?, "preflight checks passed");
    /// # Ok::<(), pamoja_mavlink::MavlinkError>(())
    /// ```
    pub fn text(&self, name: &str) -> Result<&str> {
        let (offset, field) = self.descriptor.locate(name)?;
        if field.ty != FieldType::Char || field.array_len == 0 {
            return Err(MavlinkError::FieldTypeMismatch);
        }
        let bytes = &self.payload[offset..offset + field.elements()];
        let end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        core::str::from_utf8(&bytes[..end]).map_err(|_| MavlinkError::BadPayload)
    }

    /// Writes text into a `char` array, padding the rest with zeros.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `text` - the text to store, at most the field's declared length.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::UnknownField`] if the message has no such field,
    /// [`MavlinkError::FieldTypeMismatch`] if the field is not a `char` array, and
    /// [`MavlinkError::PayloadTooLong`] if the text is longer than the field.
    pub fn set_text(&mut self, name: &str, text: &str) -> Result<()> {
        let (_, field) = self.descriptor.locate(name)?;
        if field.ty != FieldType::Char {
            return Err(MavlinkError::FieldTypeMismatch);
        }
        self.set_bytes(name, text.as_bytes())
    }

    fn element(&self, name: &str, index: usize) -> Result<(usize, &'a FieldDescriptor<'a>)> {
        let (offset, field) = self.descriptor.locate(name)?;
        if index >= field.elements() {
            return Err(MavlinkError::FieldIndexOutOfRange);
        }
        Ok((offset + index * field.ty.size(), field))
    }

    fn read<const N: usize>(&self, offset: usize) -> [u8; N] {
        let mut bytes = [0; N];
        bytes.copy_from_slice(&self.payload[offset..offset + N]);
        bytes
    }

    fn write(&mut self, offset: usize, bytes: &[u8]) {
        self.payload[offset..offset + bytes.len()].copy_from_slice(bytes);
    }
}

/// Returns the shape of a common-dialect message, if this crate types it.
///
/// # Arguments
///
/// * `msgid` - the message id to look up.
///
/// # Returns
///
/// The descriptor, or [`None`] for an id outside the typed set; a caller can describe such
/// a message itself with [`MessageDescriptorBuilder`].
///
/// # Examples
///
/// ```
/// use pamoja_mavlink::dialect::descriptor;
///
/// let heartbeat = descriptor(0).expect("HEARTBEAT is in the common dialect");
/// assert_eq!(heartbeat.name, "HEARTBEAT");
/// assert_eq!(heartbeat.wire_len(), 9);
/// assert!(descriptor(50_000).is_none());
/// ```
pub fn descriptor(msgid: u32) -> Option<&'static MessageDescriptor<'static>> {
    DESCRIPTORS.iter().copied().find(|shape| shape.id == msgid)
}

/// Returns the shape of a common-dialect message by name.
///
/// # Arguments
///
/// * `name` - the message name, such as `"GLOBAL_POSITION_INT"`.
///
/// # Returns
///
/// The descriptor, or [`None`] for a name outside the typed set.
///
/// # Examples
///
/// ```
/// use pamoja_mavlink::dialect::descriptor_by_name;
///
/// let position = descriptor_by_name("GLOBAL_POSITION_INT").expect("a typed message");
/// assert_eq!(position.id, 33);
/// ```
pub fn descriptor_by_name(name: &str) -> Option<&'static MessageDescriptor<'static>> {
    DESCRIPTORS.iter().copied().find(|shape| shape.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::{
        AutopilotVersion, BatteryStatus, GlobalPositionInt, Heartbeat, Message, MissionAck,
        Statustext,
    };
    use crate::Header;

    // Holds each typed message and its descriptor to the same shape. A field mistyped in
    // one and not the other changes the bytes on the wire, which this catches at the
    // declaration rather than against a live autopilot.
    macro_rules! assert_same_shape {
        ($( $message:ty ),+ $(,)?) => {
            $(
                let shape = <$message as Message>::DESCRIPTOR;
                assert_eq!(shape.id, <$message as Message>::ID);
                assert_eq!(shape.name, <$message as Message>::NAME);
                assert_eq!(shape.crc_extra, <$message as Message>::CRC_EXTRA);
                assert_eq!(shape.wire_len(), <$message as Message>::WIRE_LEN);
                assert_eq!(shape.derived_crc_extra(), <$message as Message>::CRC_EXTRA);

                let base: Vec<(&str, &str, u8)> = shape
                    .fields
                    .iter()
                    .filter(|field| !field.extension)
                    .map(|field| (field.ty.wire_name(), field.name, field.array_len))
                    .collect();
                assert_eq!(base.as_slice(), <$message as Message>::BASE_FIELDS);
            )+
        };
    }

    #[test]
    fn every_typed_message_agrees_with_its_descriptor() {
        assert_same_shape!(
            crate::dialect::Heartbeat,
            crate::dialect::SysStatus,
            crate::dialect::SystemTime,
            crate::dialect::Ping,
            crate::dialect::SetMode,
            crate::dialect::ParamRequestRead,
            crate::dialect::ParamRequestList,
            crate::dialect::ParamValue,
            crate::dialect::ParamSet,
            crate::dialect::GpsRawInt,
            crate::dialect::Attitude,
            crate::dialect::AttitudeQuaternion,
            crate::dialect::LocalPositionNed,
            crate::dialect::GlobalPositionInt,
            crate::dialect::ServoOutputRaw,
            crate::dialect::MissionRequest,
            crate::dialect::MissionCurrent,
            crate::dialect::MissionRequestList,
            crate::dialect::MissionCount,
            crate::dialect::MissionClearAll,
            crate::dialect::MissionAck,
            crate::dialect::MissionRequestInt,
            crate::dialect::RcChannels,
            crate::dialect::ManualControl,
            crate::dialect::MissionItemInt,
            crate::dialect::VfrHud,
            crate::dialect::CommandInt,
            crate::dialect::CommandLong,
            crate::dialect::CommandAck,
            crate::dialect::SetPositionTargetLocalNed,
            crate::dialect::SetPositionTargetGlobalInt,
            crate::dialect::BatteryStatus,
            crate::dialect::AutopilotVersion,
            crate::dialect::HomePosition,
            crate::dialect::ExtendedSysState,
            crate::dialect::Statustext,
        );
    }

    #[test]
    fn the_registry_covers_exactly_the_typed_messages() {
        assert_eq!(DESCRIPTORS.len(), 36);
        for shape in DESCRIPTORS {
            assert_eq!(crate::dialect::crc_extra(shape.id), Some(shape.crc_extra));
            assert_eq!(descriptor(shape.id), Some(*shape));
            assert_eq!(descriptor_by_name(shape.name), Some(*shape));
        }
        assert!(descriptor(50_000).is_none());
        assert!(descriptor_by_name("BATTERY_CELLS").is_none());
    }

    #[test]
    fn a_descriptor_writes_the_bytes_its_typed_message_does() -> Result<()> {
        let typed = Heartbeat {
            custom_mode: 0x0DF0_AD8B,
            type_: 2,
            autopilot: 3,
            base_mode: 81,
            system_status: 4,
            mavlink_version: 3,
        };
        let mut expected = [0u8; MAX_PAYLOAD];
        let len = typed.encode(&mut expected);

        let mut dynamic = DynamicMessage::new(Heartbeat::DESCRIPTOR)?;
        dynamic.set_uint("custom_mode", 0, 0x0DF0_AD8B)?;
        dynamic.set_uint("type", 0, 2)?;
        dynamic.set_uint("autopilot", 0, 3)?;
        dynamic.set_uint("base_mode", 0, 81)?;
        dynamic.set_uint("system_status", 0, 4)?;
        dynamic.set_uint("mavlink_version", 0, 3)?;

        assert_eq!(dynamic.payload(), &expected[..len]);
        Ok(())
    }

    #[test]
    fn signed_and_floating_fields_round_trip() -> Result<()> {
        let mut position = DynamicMessage::new(GlobalPositionInt::DESCRIPTOR)?;
        position.set_int("lat", 0, -33_856_780)?;
        position.set_int("lon", 0, 151_215_300)?;
        position.set_int("vz", 0, -250)?;
        position.set_uint("hdg", 0, 18_000)?;

        let decoded = GlobalPositionInt::decode(position.payload())?;
        assert_eq!(decoded.lat, -33_856_780);
        assert_eq!(decoded.vz, -250);
        assert_eq!(position.get_int("lon", 0)?, 151_215_300);
        assert_eq!(position.get_uint("hdg", 0)?, 18_000);

        let mut attitude = DynamicMessage::new(crate::dialect::Attitude::DESCRIPTOR)?;
        attitude.set_float("roll", 0, -0.5)?;
        assert_eq!(attitude.get_float("roll", 0)?, -0.5);
        assert_eq!(attitude.get("roll", 0)?, FieldValue::Float(-0.5));
        Ok(())
    }

    #[test]
    fn one_numeric_type_reaches_every_field() -> Result<()> {
        let mut position = DynamicMessage::new(GlobalPositionInt::DESCRIPTOR)?;
        position.set_number("lat", 0, -33_856_780.0)?;
        position.set_number("hdg", 0, 18_000.0)?;
        assert_eq!(position.get_number("lat", 0)?, -33_856_780.0);
        assert_eq!(position.get_number("hdg", 0)?, 18_000.0);

        let mut attitude = DynamicMessage::new(crate::dialect::Attitude::DESCRIPTOR)?;
        attitude.set_number("roll", 0, 0.25)?;
        assert_eq!(attitude.get_number("roll", 0)?, 0.25);

        // An integer field takes only a value it can hold exactly.
        for refused in [1.5, f64::NAN, f64::INFINITY, -1.0, 1e30] {
            assert_eq!(
                position.set_number("hdg", 0, refused).unwrap_err(),
                MavlinkError::ValueOutOfRange
            );
        }
        assert_eq!(position.get_number("hdg", 0)?, 18_000.0);
        Ok(())
    }

    #[test]
    fn arrays_and_text_are_addressed_by_element() -> Result<()> {
        let mut battery = DynamicMessage::new(BatteryStatus::DESCRIPTOR)?;
        battery.set_uint("voltages", 0, 4_150)?;
        battery.set_uint("voltages", 9, 3_990)?;
        assert_eq!(battery.get_uint("voltages", 9)?, 3_990);
        assert_eq!(BatteryStatus::decode(battery.payload())?.voltages[9], 3_990);
        assert_eq!(
            battery.get_uint("voltages", 10),
            Err(MavlinkError::FieldIndexOutOfRange)
        );

        let mut version = DynamicMessage::new(AutopilotVersion::DESCRIPTOR)?;
        version.set_bytes("flight_custom_version", &[1, 2, 3])?;
        let mut out = [0u8; 8];
        assert_eq!(version.get_bytes("flight_custom_version", &mut out)?, 8);
        assert_eq!(out, [1, 2, 3, 0, 0, 0, 0, 0]);

        let mut status = DynamicMessage::new(Statustext::DESCRIPTOR)?;
        status.set_text("text", "battery low")?;
        assert_eq!(status.text("text")?, "battery low");
        assert_eq!(status.get_uint("severity", 0)?, 0);
        Ok(())
    }

    #[test]
    fn extension_fields_sit_after_the_base_fields_and_leave_the_seed_alone() -> Result<()> {
        let shape = MissionAck::DESCRIPTOR;
        let extension = shape
            .field("mission_type")
            .expect("MISSION_ACK carries the extension");
        assert!(extension.extension);
        assert_eq!(shape.offset_of("mission_type"), Some(shape.base_len()));
        assert_eq!(shape.derived_crc_extra(), MissionAck::CRC_EXTRA);

        // A peer that predates the extension sends only the base fields, and it reads zero.
        let base = vec![0u8; shape.base_len()];
        let received = DynamicMessage::decode(shape, &base)?;
        assert_eq!(received.get_uint("mission_type", 0)?, 0);
        Ok(())
    }

    #[test]
    fn a_field_is_rejected_when_it_is_missing_or_misused() -> Result<()> {
        let mut heartbeat = DynamicMessage::new(Heartbeat::DESCRIPTOR)?;
        assert_eq!(
            heartbeat.get_uint("throttle", 0),
            Err(MavlinkError::UnknownField)
        );
        assert_eq!(
            heartbeat.set_float("type", 0, 1.0),
            Err(MavlinkError::FieldTypeMismatch)
        );
        assert_eq!(
            heartbeat.set_uint("type", 0, 300),
            Err(MavlinkError::ValueOutOfRange)
        );
        assert_eq!(
            heartbeat.set_int("type", 0, -1),
            Err(MavlinkError::ValueOutOfRange)
        );
        assert_eq!(heartbeat.text("type"), Err(MavlinkError::FieldTypeMismatch));

        let mut attitude = DynamicMessage::new(crate::dialect::Attitude::DESCRIPTOR)?;
        assert_eq!(
            attitude.set_uint("roll", 0, 1),
            Err(MavlinkError::FieldTypeMismatch)
        );
        assert_eq!(
            attitude.get_int("roll", 0),
            Err(MavlinkError::FieldTypeMismatch)
        );
        Ok(())
    }

    #[test]
    fn a_truncated_payload_reads_its_missing_bytes_as_zero() -> Result<()> {
        let shape = Heartbeat::DESCRIPTOR;
        let frame = DynamicMessage::new(shape)?.to_frame(Header::new(1, 1, 0))?;

        // MAVLink 2 trims trailing zeros but never the whole payload, so an all-zero
        // HEARTBEAT goes out as a single byte and the receiver restores the rest.
        assert_eq!(frame.payload(), &[0]);
        let received = DynamicMessage::decode(shape, frame.payload())?;
        assert_eq!(received.get_uint("custom_mode", 0)?, 0);
        assert_eq!(
            DynamicMessage::decode(shape, &[0u8; 10]).unwrap_err(),
            MavlinkError::BadPayload
        );
        Ok(())
    }

    #[test]
    fn a_builder_puts_declared_fields_into_wire_order() -> Result<()> {
        // HEARTBEAT as its definition declares it: the 32-bit field sits fourth, and wire
        // order pulls it to the front.
        let heartbeat = MessageDescriptorBuilder::new(0, "HEARTBEAT")
            .field("type", FieldType::U8, 0)
            .field("autopilot", FieldType::U8, 0)
            .field("base_mode", FieldType::U8, 0)
            .field("custom_mode", FieldType::U32, 0)
            .field("system_status", FieldType::U8, 0)
            .field("mavlink_version", FieldType::U8, 0)
            .build()?;

        assert_eq!(heartbeat.crc_extra(), Heartbeat::CRC_EXTRA);
        heartbeat.with_descriptor(|shape| {
            assert_eq!(shape.fields, Heartbeat::DESCRIPTOR.fields);
        });

        // SYS_STATUS declares an int8 in the middle of its 16-bit fields, so a stable sort
        // by size is what moves it to the end and nothing else with it.
        let status = MessageDescriptorBuilder::new(1, "SYS_STATUS")
            .field("onboard_control_sensors_present", FieldType::U32, 0)
            .field("onboard_control_sensors_enabled", FieldType::U32, 0)
            .field("onboard_control_sensors_health", FieldType::U32, 0)
            .field("load", FieldType::U16, 0)
            .field("voltage_battery", FieldType::U16, 0)
            .field("current_battery", FieldType::I16, 0)
            .field("battery_remaining", FieldType::I8, 0)
            .field("drop_rate_comm", FieldType::U16, 0)
            .field("errors_comm", FieldType::U16, 0)
            .field("errors_count1", FieldType::U16, 0)
            .field("errors_count2", FieldType::U16, 0)
            .field("errors_count3", FieldType::U16, 0)
            .field("errors_count4", FieldType::U16, 0)
            .build()?;

        assert_eq!(status.crc_extra(), crate::dialect::SysStatus::CRC_EXTRA);
        status.with_descriptor(|shape| {
            assert_eq!(shape.fields, crate::dialect::SysStatus::DESCRIPTOR.fields);
        });
        Ok(())
    }

    #[test]
    fn a_builder_rejects_a_shape_it_cannot_describe() {
        let duplicated = MessageDescriptorBuilder::new(1, "TWICE")
            .field("value", FieldType::U8, 0)
            .field("value", FieldType::U16, 0)
            .build();
        assert_eq!(duplicated.unwrap_err(), MavlinkError::DuplicateField);

        let overlong = MessageDescriptorBuilder::new(1, "HUGE")
            .field("payload", FieldType::U32, 64)
            .build();
        assert_eq!(overlong.unwrap_err(), MavlinkError::PayloadTooLong);
    }

    #[test]
    fn an_owned_dialect_resolves_its_own_ids_and_falls_back_to_the_common_one() -> Result<()> {
        let mut dialect = OwnedDialect::new();
        assert!(dialect.is_empty());
        dialect.insert(
            MessageDescriptorBuilder::new(50_000, "BATTERY_CELLS")
                .field("cell_mv", FieldType::U16, 6)
                .field("pack_id", FieldType::U8, 0)
                .build()?,
        );
        assert_eq!(dialect.len(), 1);

        let private = dialect.get(50_000).expect("just inserted");
        assert_eq!(dialect.crc_extra(50_000), Some(private.crc_extra()));
        assert_eq!(dialect.crc_extra(0), Some(Heartbeat::CRC_EXTRA));
        assert_eq!(dialect.crc_extra(49_999), None);
        assert!(dialect.by_name("BATTERY_CELLS").is_some());

        // Replacing a shape keeps one entry rather than shadowing the first.
        dialect.insert(
            MessageDescriptorBuilder::new(50_000, "BATTERY_CELLS")
                .field("pack_id", FieldType::U8, 0)
                .build()?,
        );
        assert_eq!(dialect.len(), 1);
        Ok(())
    }

    #[test]
    fn an_owned_copy_keeps_the_shape_it_was_taken_from() {
        let owned = OwnedMessageDescriptor::from_descriptor(Statustext::DESCRIPTOR);
        assert_eq!(owned.id(), Statustext::ID);
        assert_eq!(owned.crc_extra(), Statustext::CRC_EXTRA);
        owned.with_descriptor(|shape| {
            assert_eq!(shape.fields, Statustext::DESCRIPTOR.fields);
            assert_eq!(shape.wire_len(), Statustext::WIRE_LEN);
        });
    }
}
