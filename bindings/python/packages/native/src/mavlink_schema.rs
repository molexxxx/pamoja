//! Generated Python bindings for MAVLink message shapes.
//!
//! [`mavlink`](crate::mavlink) carries any message's bytes, which is enough to
//! move traffic but leaves the caller hand-packing payloads against a message
//! definition. This is the layer above: a schema states a message's fields, and
//! a message reads and writes them by name, so a caller works in `custom_mode`
//! and `lat` rather than byte offsets.
//!
//! Every message the engine types is published, so a schema for the common
//! dialect needs nothing declared. A message from ArduPilot's dialect, PX4's, or
//! a vendor's private one is described through a builder, which puts the fields
//! in wire order and derives the `CRC_EXTRA` seed, so a caller transcribes a
//! definition as it reads.

use std::sync::Mutex;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_mavlink::dialect::{
    descriptor, descriptor_by_name, DynamicMessage, FieldType, MessageDescriptor,
    MessageDescriptorBuilder, OwnedMessageDescriptor, DESCRIPTORS,
};
use pamoja_mavlink::{Header, MavlinkError};

use crate::mavlink::{MavlinkFrame, MavlinkHeader};

/// Turns a MAVLink error into the exception a caller sees.
fn error_of(error: MavlinkError) -> PyErr {
    PyValueError::new_err(error.to_string())
}

/// Resolves a field type given either its code or the name a dialect writes.
fn type_of(field_type: &Bound<'_, PyAny>) -> PyResult<FieldType> {
    if let Ok(code) = field_type.extract::<u32>() {
        return match code {
            1 => Ok(FieldType::U8),
            2 => Ok(FieldType::I8),
            3 => Ok(FieldType::Char),
            4 => Ok(FieldType::U16),
            5 => Ok(FieldType::I16),
            6 => Ok(FieldType::U32),
            7 => Ok(FieldType::I32),
            8 => Ok(FieldType::U64),
            9 => Ok(FieldType::I64),
            10 => Ok(FieldType::F32),
            11 => Ok(FieldType::F64),
            _ => Err(PyValueError::new_err(format!(
                "{code} is not a MAVLink field type"
            ))),
        };
    }
    let name: String = field_type.extract()?;
    FieldType::from_wire_name(&name)
        .ok_or_else(|| PyValueError::new_err(format!("{name} is not a MAVLink field type")))
}

/// The code a field type is written as, matching `pamoja.mavlink.FieldType`.
fn code_of(ty: FieldType) -> u32 {
    match ty {
        FieldType::U8 => 1,
        FieldType::I8 => 2,
        FieldType::Char => 3,
        FieldType::U16 => 4,
        FieldType::I16 => 5,
        FieldType::U32 => 6,
        FieldType::I32 => 7,
        FieldType::U64 => 8,
        FieldType::I64 => 9,
        FieldType::F32 => 10,
        FieldType::F64 => 11,
    }
}

/// One field of a message shape.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct MavlinkFieldInfo {
    /// The field name as the dialect writes it, such as `custom_mode`.
    #[pyo3(get)]
    name: String,
    /// The field's type name as the dialect writes it, such as `uint32_t`.
    #[pyo3(get)]
    type_name: String,
    /// The field's type, one of the `FieldType` values.
    #[pyo3(get)]
    field_type: u32,
    /// The element count for an array field, or `0` for a scalar.
    #[pyo3(get)]
    array_len: u8,
    /// Whether this is a MAVLink 2 extension field.
    #[pyo3(get)]
    extension: bool,
    /// The field's byte offset within the payload.
    #[pyo3(get)]
    offset: usize,
}

#[gen_stub_pymethods]
#[pymethods]
impl MavlinkFieldInfo {
    /// Returns a readable form for logs and the interpreter.
    fn __repr__(&self) -> String {
        format!(
            "MavlinkFieldInfo(name={:?}, type_name={:?}, array_len={}, extension={}, offset={})",
            self.name, self.type_name, self.array_len, self.extension, self.offset
        )
    }
}

/// The shape of one message: its id, name, seed, and fields.
#[gen_stub_pyclass]
#[pyclass]
pub struct MessageSchema {
    shape: OwnedMessageDescriptor,
}

impl MessageSchema {
    /// Returns the shape this schema wraps, for another module in this crate.
    pub(crate) fn shape(&self) -> &OwnedMessageDescriptor {
        &self.shape
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl MessageSchema {
    /// Returns the shape of a message the engine types, by id.
    ///
    /// Raises `ValueError` if this build does not type that id, which is what a
    /// builder is for.
    #[staticmethod]
    fn for_id(msgid: u32) -> PyResult<Self> {
        let shape = descriptor(msgid).ok_or_else(|| {
            PyValueError::new_err(format!("message {msgid} is not one this build types"))
        })?;
        Ok(Self {
            shape: OwnedMessageDescriptor::from_descriptor(shape),
        })
    }

    /// Returns the shape of a message the engine types, by name.
    ///
    /// Raises `ValueError` if this build does not type that name.
    #[staticmethod]
    fn for_name(name: &str) -> PyResult<Self> {
        let shape = descriptor_by_name(name).ok_or_else(|| {
            PyValueError::new_err(format!("{name} is not a message this build types"))
        })?;
        Ok(Self {
            shape: OwnedMessageDescriptor::from_descriptor(shape),
        })
    }

    /// The id of the message this schema describes.
    #[getter]
    fn id(&self) -> u32 {
        self.shape.id()
    }

    /// The name of the message this schema describes.
    #[getter]
    fn name(&self) -> String {
        self.shape.name().to_owned()
    }

    /// The `CRC_EXTRA` seed a frame carrying this message folds into its checksum.
    #[getter]
    fn crc_extra(&self) -> u8 {
        self.shape.crc_extra()
    }

    /// The length of the message on the wire, in bytes, extensions included.
    #[getter]
    fn wire_len(&self) -> usize {
        self.shape.with_descriptor(|shape| shape.wire_len())
    }

    /// The fields in wire order: the base fields largest first, then extensions.
    #[getter]
    fn fields(&self) -> Vec<MavlinkFieldInfo> {
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
                        offset,
                    };
                    offset += field.size();
                    info
                })
                .collect()
        })
    }

    /// Returns a readable form for logs and the interpreter.
    fn __repr__(&self) -> String {
        format!(
            "MessageSchema(id={}, name={:?}, crc_extra={})",
            self.shape.id(),
            self.shape.name(),
            self.shape.crc_extra()
        )
    }
}

/// Returns the names of every message this build types, in message-id order.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_known_messages() -> Vec<String> {
    DESCRIPTORS
        .iter()
        .map(|shape| shape.name.to_owned())
        .collect()
}

/// Describes a message this build does not type, one field at a time.
///
/// Fields are added in the order the message definition lists them; building
/// puts them in wire order and derives the `CRC_EXTRA` seed from the result.
#[gen_stub_pyclass]
#[pyclass]
pub struct MessageSchemaBuilder {
    builder: Mutex<Option<MessageDescriptorBuilder>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MessageSchemaBuilder {
    /// Starts describing a message with an id and the name its dialect uses.
    #[new]
    fn new(msgid: u32, name: &str) -> Self {
        Self {
            builder: Mutex::new(Some(MessageDescriptorBuilder::new(msgid, name))),
        }
    }

    /// Adds a base field, in the order the definition declares it.
    ///
    /// The type is either a `FieldType` value or the name a dialect writes, such
    /// as `"uint32_t"`. Raises `ValueError` if it is neither, or if the shape has
    /// already been built.
    #[pyo3(signature = (name, field_type, array_len = 0))]
    fn field(&self, name: &str, field_type: &Bound<'_, PyAny>, array_len: u8) -> PyResult<()> {
        self.add(name, field_type, array_len, false)
    }

    /// Adds a MAVLink 2 extension field, in the order the definition declares it.
    ///
    /// Extensions keep their declared order, stay out of the `CRC_EXTRA` seed,
    /// and read as zero from a frame sent by a peer that predates them.
    #[pyo3(signature = (name, field_type, array_len = 0))]
    fn extension(&self, name: &str, field_type: &Bound<'_, PyAny>, array_len: u8) -> PyResult<()> {
        self.add(name, field_type, array_len, true)
    }

    /// Puts the declared fields in wire order and finishes the shape.
    ///
    /// Raises `ValueError` if two fields share a name, the fields do not fit a
    /// MAVLink payload, or the shape has already been built.
    fn build(&self) -> PyResult<MessageSchema> {
        let mut held = self
            .builder
            .lock()
            .expect("the builder lock is never poisoned");
        let builder = held
            .take()
            .ok_or_else(|| PyValueError::new_err("this builder has already been built"))?;
        let shape = builder.build().map_err(error_of)?;
        Ok(MessageSchema { shape })
    }
}

impl MessageSchemaBuilder {
    fn add(
        &self,
        name: &str,
        field_type: &Bound<'_, PyAny>,
        array_len: u8,
        extension: bool,
    ) -> PyResult<()> {
        let ty = type_of(field_type)?;
        let mut held = self
            .builder
            .lock()
            .expect("the builder lock is never poisoned");
        let builder = held
            .take()
            .ok_or_else(|| PyValueError::new_err("this builder has already been built"))?;
        *held = Some(if extension {
            builder.extension(name, ty, array_len)
        } else {
            builder.field(name, ty, array_len)
        });
        Ok(())
    }
}

/// A message read and written by field name against a schema.
#[gen_stub_pyclass]
#[pyclass]
pub struct MavlinkMessage {
    shape: OwnedMessageDescriptor,
    payload: Mutex<Vec<u8>>,
}

impl MavlinkMessage {
    /// Wraps a message the engine decoded, for another module in this crate to hand back.
    pub(crate) fn from_typed(shape: &MessageDescriptor<'static>, payload: Vec<u8>) -> Self {
        Self {
            shape: OwnedMessageDescriptor::from_descriptor(shape),
            payload: Mutex::new(payload),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl MavlinkMessage {
    /// Creates a message with every field zero.
    ///
    /// Raises `ValueError` if the shape does not fit a MAVLink payload.
    #[staticmethod]
    fn empty(schema: &MessageSchema) -> PyResult<Self> {
        let shape = schema.shape().clone();
        let payload = shape
            .with_descriptor(|view| DynamicMessage::new(view).map(|m| m.payload().to_vec()))
            .map_err(error_of)?;
        Ok(Self {
            shape,
            payload: Mutex::new(payload),
        })
    }

    /// Reads a message out of a frame payload.
    ///
    /// A payload shorter than the shape is zero-extended, as MAVLink 2
    /// truncation requires, so a frame from a peer that trimmed trailing zeros or
    /// predates an extension field still decodes. Raises `ValueError` if the
    /// payload is longer than the shape describes.
    #[staticmethod]
    fn decode(schema: &MessageSchema, payload: Vec<u8>) -> PyResult<Self> {
        let shape = schema.shape().clone();
        let payload = shape
            .with_descriptor(|view| {
                DynamicMessage::decode(view, &payload).map(|m| m.payload().to_vec())
            })
            .map_err(error_of)?;
        Ok(Self {
            shape,
            payload: Mutex::new(payload),
        })
    }

    /// The id of the message this carries.
    #[getter]
    fn message_id(&self) -> u32 {
        self.shape.id()
    }

    /// The name of the message this carries.
    #[getter]
    fn name(&self) -> String {
        self.shape.name().to_owned()
    }

    /// The message's bytes as they go on the wire.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let held = self
            .payload
            .lock()
            .expect("the payload lock is never poisoned");
        PyBytes::new(py, &held)
    }

    /// Builds a v2 frame carrying this message.
    ///
    /// Raises `ValueError` if the message does not fit a frame.
    fn to_frame(&self, header: MavlinkHeader) -> PyResult<MavlinkFrame> {
        let held = self
            .payload
            .lock()
            .expect("the payload lock is never poisoned");
        let frame = self
            .shape
            .with_descriptor(|view| {
                DynamicMessage::decode(view, &held)?.to_frame(Header::from(header))
            })
            .map_err(error_of)?;
        Ok(MavlinkFrame::from_frame(frame))
    }

    /// Reads a field as a number.
    ///
    /// Every field reads this way. An integer field wider than 53 bits can exceed
    /// what a float holds exactly; read those with `get_int` where the exact value
    /// matters.
    #[pyo3(signature = (field, index = 0))]
    fn get(&self, field: &str, index: usize) -> PyResult<f64> {
        self.read(|message| message.get_number(field, index))
    }

    /// Reads an integer field exactly, whatever its width or sign.
    #[pyo3(signature = (field, index = 0))]
    fn get_int(&self, field: &str, index: usize) -> PyResult<i64> {
        self.read(|message| message.get_int(field, index))
    }

    /// Writes a number into a field, converting it to the field's type.
    ///
    /// A value bound for an integer field must be a whole number within that
    /// field's range, so a fractional or oversized value is refused rather than
    /// silently truncated.
    #[pyo3(signature = (field, value, index = 0))]
    fn set(&self, field: &str, value: f64, index: usize) -> PyResult<()> {
        self.write(|message| message.set_number(field, index, value))
    }

    /// Writes an integer into a field exactly, whatever its width or sign.
    #[pyo3(signature = (field, value, index = 0))]
    fn set_int(&self, field: &str, value: i64, index: usize) -> PyResult<()> {
        self.write(|message| message.set_int(field, index, value))
    }

    /// Copies the raw bytes of a byte-wide array field out, padding included.
    fn get_bytes<'py>(&self, py: Python<'py>, field: &str) -> PyResult<Bound<'py, PyBytes>> {
        let held = self
            .payload
            .lock()
            .expect("the payload lock is never poisoned");
        let mut out = vec![0u8; pamoja_mavlink::MAX_PAYLOAD];
        let len = self
            .shape
            .with_descriptor(|view| DynamicMessage::decode(view, &held)?.get_bytes(field, &mut out))
            .map_err(error_of)?;
        Ok(PyBytes::new(py, &out[..len]))
    }

    /// Writes the raw bytes of a byte-wide array field, zero-padding the rest.
    fn set_bytes(&self, field: &str, data: Vec<u8>) -> PyResult<()> {
        self.write(|message| message.set_bytes(field, &data))
    }

    /// Reads a `char` array as text, stopping at the padding.
    fn get_text(&self, field: &str) -> PyResult<String> {
        self.read(|message| message.text(field).map(str::to_owned))
    }

    /// Writes text into a `char` array, padding the rest with zeros.
    fn set_text(&self, field: &str, text: &str) -> PyResult<()> {
        self.write(|message| message.set_text(field, text))
    }

    /// Returns a readable form for logs and the interpreter.
    fn __repr__(&self) -> String {
        format!(
            "MavlinkMessage(name={:?}, id={})",
            self.shape.name(),
            self.shape.id()
        )
    }
}

impl MavlinkMessage {
    fn read<T>(
        &self,
        query: impl FnOnce(&DynamicMessage<'_>) -> pamoja_mavlink::Result<T>,
    ) -> PyResult<T> {
        let held = self
            .payload
            .lock()
            .expect("the payload lock is never poisoned");
        self.shape
            .with_descriptor(|view| query(&DynamicMessage::decode(view, &held)?))
            .map_err(error_of)
    }

    fn write(
        &self,
        step: impl FnOnce(&mut DynamicMessage<'_>) -> pamoja_mavlink::Result<()>,
    ) -> PyResult<()> {
        let mut held = self
            .payload
            .lock()
            .expect("the payload lock is never poisoned");
        let updated = self.shape.with_descriptor(|view| {
            let mut message = DynamicMessage::decode(view, &held)?;
            step(&mut message)?;
            Ok::<Vec<u8>, MavlinkError>(message.payload().to_vec())
        });
        match updated {
            Ok(payload) => {
                *held = payload;
                Ok(())
            }
            Err(error) => Err(error_of(error)),
        }
    }
}
