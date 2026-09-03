//! Message shapes a caller owns, for dialects this crate does not ship.
//!
//! A [`MessageDescriptor`] borrows its name and fields, which suits the built-in registry
//! where every string is static. A caller transcribing a vendor's dialect owns those
//! strings instead, so [`OwnedMessageDescriptor`] holds them and lends a borrowed view
//! when one is needed.
//!
//! [`MessageDescriptorBuilder`] takes fields in the order a message definition lists them
//! and puts them in wire order itself, then derives the `CRC_EXTRA` seed from the result.

use alloc::string::String;
use alloc::vec::Vec;

use super::{FieldDescriptor, FieldType, MessageDescriptor};
use crate::error::{MavlinkError, Result};
use crate::frame::MAX_PAYLOAD;

/// One field of an owned message shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedFieldDescriptor {
    /// The field name as the dialect writes it.
    pub name: String,

    /// The field's scalar type, which for an array is its element type.
    pub ty: FieldType,

    /// The element count for an array field, or `0` for a scalar.
    pub array_len: u8,

    /// Whether this is a MAVLink 2 extension field.
    pub extension: bool,
}

/// A message shape a caller owns, for a message from a dialect this crate does not type.
///
/// # Examples
///
/// ```
/// use pamoja_mavlink::dialect::{DynamicMessage, FieldType, MessageDescriptorBuilder};
///
/// // A private message, written the way its definition reads: declaration order, with
/// // the builder putting the fields on the wire largest first.
/// let shape = MessageDescriptorBuilder::new(50_000, "BATTERY_CELLS")
///     .field("cell_mv", FieldType::U16, 6)
///     .field("pack_id", FieldType::U8, 0)
///     .field("uptime_ms", FieldType::U32, 0)
///     .build()?;
///
/// shape.with_descriptor(|shape| -> Result<(), pamoja_mavlink::MavlinkError> {
///     // The 32-bit field leads, then the array, then the byte.
///     assert_eq!(shape.offset_of("uptime_ms"), Some(0));
///     assert_eq!(shape.offset_of("cell_mv"), Some(4));
///     assert_eq!(shape.wire_len(), 17);
///
///     let mut message = DynamicMessage::new(shape)?;
///     message.set_uint("cell_mv", 3, 4_150)?;
///     assert_eq!(message.get_uint("cell_mv", 3)?, 4_150);
///     Ok(())
/// })?;
/// # Ok::<(), pamoja_mavlink::MavlinkError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedMessageDescriptor {
    id: u32,
    name: String,
    crc_extra: u8,
    fields: Vec<OwnedFieldDescriptor>,
}

impl OwnedMessageDescriptor {
    /// Takes an owned copy of a borrowed shape.
    ///
    /// # Arguments
    ///
    /// * `descriptor` - the shape to copy, such as one from the built-in registry.
    ///
    /// # Returns
    ///
    /// The owned shape.
    pub fn from_descriptor(descriptor: &MessageDescriptor<'_>) -> Self {
        Self {
            id: descriptor.id,
            name: String::from(descriptor.name),
            crc_extra: descriptor.crc_extra,
            fields: descriptor
                .fields
                .iter()
                .map(|field| OwnedFieldDescriptor {
                    name: String::from(field.name),
                    ty: field.ty,
                    array_len: field.array_len,
                    extension: field.extension,
                })
                .collect(),
        }
    }

    /// Returns the message id.
    ///
    /// # Returns
    ///
    /// The id on the wire.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Returns the message name.
    ///
    /// # Returns
    ///
    /// The name, such as `"BATTERY_CELLS"`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the `CRC_EXTRA` seed this shape implies.
    ///
    /// # Returns
    ///
    /// The seed, which a frame carrying this message folds into its checksum.
    pub fn crc_extra(&self) -> u8 {
        self.crc_extra
    }

    /// Returns the fields in wire order.
    ///
    /// # Returns
    ///
    /// The base fields largest first, then any extensions.
    pub fn fields(&self) -> &[OwnedFieldDescriptor] {
        &self.fields
    }

    /// Lends a borrowed view of this shape for the duration of a call.
    ///
    /// A [`MessageDescriptor`] borrows a slice of fields that does not exist until it is
    /// assembled, so the view is built for the call rather than stored, which keeps this
    /// type free of self-references.
    ///
    /// # Arguments
    ///
    /// * `query` - what to do with the borrowed shape.
    ///
    /// # Returns
    ///
    /// Whatever `query` returns.
    pub fn with_descriptor<R>(&self, query: impl FnOnce(&MessageDescriptor<'_>) -> R) -> R {
        let fields: Vec<FieldDescriptor<'_>> = self
            .fields
            .iter()
            .map(|field| FieldDescriptor {
                name: &field.name,
                ty: field.ty,
                array_len: field.array_len,
                extension: field.extension,
            })
            .collect();
        query(&MessageDescriptor {
            id: self.id,
            name: &self.name,
            crc_extra: self.crc_extra,
            fields: &fields,
        })
    }
}

/// Builds a message shape from a definition written the way a dialect reads.
///
/// Fields are given in declaration order. MAVLink puts the base fields on the wire largest
/// type first, keeping equal-sized fields in the order declared, and leaves extension
/// fields at the end untouched; [`build`](Self::build) applies that and derives the
/// `CRC_EXTRA` seed from the result, so a transcription error surfaces as a checksum a
/// peer rejects rather than as silently misread fields.
#[derive(Clone, Debug, Default)]
pub struct MessageDescriptorBuilder {
    id: u32,
    name: String,
    fields: Vec<OwnedFieldDescriptor>,
}

impl MessageDescriptorBuilder {
    /// Starts a shape for a message id and name.
    ///
    /// # Arguments
    ///
    /// * `id` - the message id on the wire.
    /// * `name` - the message name, which the `CRC_EXTRA` derivation folds in, so it must
    ///   match the dialect exactly.
    ///
    /// # Returns
    ///
    /// The builder, with no fields yet.
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            fields: Vec::new(),
        }
    }

    /// Adds a base field, in the order the definition declares it.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `ty` - the field's scalar type, or an array's element type.
    /// * `array_len` - the element count for an array, or `0` for a scalar.
    ///
    /// # Returns
    ///
    /// The builder.
    pub fn field(mut self, name: impl Into<String>, ty: FieldType, array_len: u8) -> Self {
        self.fields.push(OwnedFieldDescriptor {
            name: name.into(),
            ty,
            array_len,
            extension: false,
        });
        self
    }

    /// Adds a MAVLink 2 extension field, in the order the definition declares it.
    ///
    /// Extensions keep their declared order, are excluded from the `CRC_EXTRA` seed, and
    /// read as zero from a frame sent by a peer that predates them.
    ///
    /// # Arguments
    ///
    /// * `name` - the field name.
    /// * `ty` - the field's scalar type, or an array's element type.
    /// * `array_len` - the element count for an array, or `0` for a scalar.
    ///
    /// # Returns
    ///
    /// The builder.
    pub fn extension(mut self, name: impl Into<String>, ty: FieldType, array_len: u8) -> Self {
        self.fields.push(OwnedFieldDescriptor {
            name: name.into(),
            ty,
            array_len,
            extension: true,
        });
        self
    }

    /// Puts the fields in wire order and derives the seed.
    ///
    /// # Returns
    ///
    /// The finished shape.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::DuplicateField`] if two fields share a name, and
    /// [`MavlinkError::PayloadTooLong`] if the fields do not fit a MAVLink payload.
    pub fn build(self) -> Result<OwnedMessageDescriptor> {
        let Self {
            id,
            name,
            mut fields,
        } = self;

        for (index, field) in fields.iter().enumerate() {
            if fields[index + 1..]
                .iter()
                .any(|other| other.name == field.name)
            {
                return Err(MavlinkError::DuplicateField);
            }
        }

        let total: usize = fields
            .iter()
            .map(|field| {
                let elements = if field.array_len == 0 {
                    1
                } else {
                    field.array_len as usize
                };
                elements * field.ty.size()
            })
            .sum();
        if total > MAX_PAYLOAD {
            return Err(MavlinkError::PayloadTooLong);
        }

        // MAVLink orders the base fields by type size, largest first, and a stable sort
        // keeps equal-sized fields as declared. Extensions are already at the end and are
        // not reordered, so sorting only the leading base run leaves them where they are.
        let base = fields
            .iter()
            .position(|field| field.extension)
            .unwrap_or(fields.len());
        fields[..base].sort_by_key(|field| core::cmp::Reverse(field.ty.size()));

        let crc_extra = crate::crc::crc_extra_of(
            &name,
            fields
                .iter()
                .filter(|field| !field.extension)
                .map(|field| (field.ty.wire_name(), field.name.as_str(), field.array_len)),
        );

        Ok(OwnedMessageDescriptor {
            id,
            name,
            crc_extra,
            fields,
        })
    }
}

/// A dialect a caller owns: message shapes looked up by id or name.
///
/// This is what makes a whole dialect usable rather than one message at a time. It also
/// resolves the `CRC_EXTRA` a [`Parser`](crate::Parser) needs, so frames from a private
/// dialect check like any other.
///
/// # Examples
///
/// ```
/// use pamoja_mavlink::dialect::{FieldType, MessageDescriptorBuilder, OwnedDialect};
/// use pamoja_mavlink::{Header, Parser};
///
/// let mut dialect = OwnedDialect::new();
/// dialect.insert(
///     MessageDescriptorBuilder::new(50_000, "BATTERY_CELLS")
///         .field("pack_id", FieldType::U8, 0)
///         .field("uptime_ms", FieldType::U32, 0)
///         .build()?,
/// );
///
/// let shape = dialect.by_name("BATTERY_CELLS").expect("just inserted");
/// let frame = shape.with_descriptor(|shape| {
///     let mut message = pamoja_mavlink::dialect::DynamicMessage::new(shape)?;
///     message.set_uint("pack_id", 0, 2)?;
///     message.to_frame(Header::new(9, 1, 0))
/// })?;
///
/// // A parser resolves the seed through the dialect, so the private frame verifies.
/// let resolve = |id| dialect.crc_extra(id);
/// let mut parser = Parser::new();
/// let received = frame
///     .as_bytes()
///     .iter()
///     .filter_map(|byte| parser.push_byte(*byte, &resolve))
///     .next()
///     .expect("a whole frame");
/// assert_eq!(received.message_id(), 50_000);
/// # Ok::<(), pamoja_mavlink::MavlinkError>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct OwnedDialect {
    messages: Vec<OwnedMessageDescriptor>,
}

impl OwnedDialect {
    /// Creates a dialect with no messages.
    ///
    /// # Returns
    ///
    /// The empty dialect.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Adds a message shape, replacing any shape already held for its id.
    ///
    /// # Arguments
    ///
    /// * `descriptor` - the shape to add.
    pub fn insert(&mut self, descriptor: OwnedMessageDescriptor) {
        let id = descriptor.id();
        self.messages.retain(|held| held.id() != id);
        self.messages.push(descriptor);
    }

    /// Looks a message shape up by id.
    ///
    /// # Arguments
    ///
    /// * `msgid` - the message id.
    ///
    /// # Returns
    ///
    /// The shape, or [`None`] if this dialect does not describe that id.
    pub fn get(&self, msgid: u32) -> Option<&OwnedMessageDescriptor> {
        self.messages.iter().find(|held| held.id() == msgid)
    }

    /// Looks a message shape up by name.
    ///
    /// # Arguments
    ///
    /// * `name` - the message name.
    ///
    /// # Returns
    ///
    /// The shape, or [`None`] if this dialect does not describe that name.
    pub fn by_name(&self, name: &str) -> Option<&OwnedMessageDescriptor> {
        self.messages.iter().find(|held| held.name() == name)
    }

    /// Returns the `CRC_EXTRA` for a message id, falling back to the common dialect.
    ///
    /// # Arguments
    ///
    /// * `msgid` - the message id.
    ///
    /// # Returns
    ///
    /// The seed, or [`None`] if neither this dialect nor the common one knows the id.
    pub fn crc_extra(&self, msgid: u32) -> Option<u8> {
        self.get(msgid)
            .map(OwnedMessageDescriptor::crc_extra)
            .or_else(|| crate::dialect::crc_extra(msgid))
    }

    /// Returns how many message shapes this dialect holds.
    ///
    /// # Returns
    ///
    /// The count.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Reports whether this dialect holds no message shapes.
    ///
    /// # Returns
    ///
    /// `true` if it is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}
