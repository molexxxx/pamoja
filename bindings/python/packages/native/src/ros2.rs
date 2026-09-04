//! Generated Python bindings for the ROS 2 naming and encoding rules.
//!
//! These mirror the `pamoja-ros2` Rust API: what makes a topic name legal, what
//! it becomes on the DDS wire, the RIHS type hash that identifies a message
//! definition, and the CDR encoding the payload itself is written in.
//!
//! None of it needs a ROS 2 installation, which is the point. A gateway written
//! in Python can validate a name, derive the DDS topic and the Zenoh key an
//! `rmw_zenoh` peer subscribes on, and encode a `geometry_msgs/msg/Twist`
//! without a ROS distribution anywhere near it. Driving a live graph does need
//! one, so the Rust crate's `bridge` feature stays Rust-only.

use std::sync::Mutex;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_ros2::key::entity_key;
use pamoja_ros2::msg::{CdrReader as CoreReader, CdrWriter as CoreWriter, Twist, Vector3};
use pamoja_ros2::name::{dds_topic, is_fully_qualified, is_valid_name, percent_mangle, EntityKind};
use pamoja_ros2::typehash::{dds_type_name, TypeHash};

/// Maps a subsystem name onto the kind it selects.
fn kind_of(kind: &str) -> PyResult<EntityKind> {
    match kind {
        "Topic" => Ok(EntityKind::Topic),
        "ServiceRequest" => Ok(EntityKind::ServiceRequest),
        "ServiceResponse" => Ok(EntityKind::ServiceResponse),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "`{other}` is not one of Topic, ServiceRequest, or ServiceResponse"
        ))),
    }
}

/// Reports whether a string is a valid ROS 2 topic or service name.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ros2_is_valid_name(name: &str) -> bool {
    is_valid_name(name)
}

/// Reports whether a name is fully qualified, so it resolves with no namespace.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ros2_is_fully_qualified(name: &str) -> bool {
    is_fully_qualified(name)
}

/// Returns the DDS topic prefix a subsystem uses.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ros2_entity_kind_prefix(kind: &str) -> PyResult<String> {
    Ok(kind_of(kind)?.prefix().to_owned())
}

/// Returns the DDS topic a fully qualified name maps onto, or `None` if the
/// name is not fully qualified.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ros2_dds_topic(fqn: &str, kind: &str) -> PyResult<Option<String>> {
    Ok(dds_topic(fqn, kind_of(kind)?))
}

/// Percent-mangles a name the way a DDS partition requires.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ros2_percent_mangle(name: &str) -> String {
    percent_mangle(name)
}

/// Returns the DDS type name an interface type maps onto, or `None` if the type
/// is not a valid `package/namespace/Type`.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ros2_dds_type_name(ros_type: &str) -> Option<String> {
    dds_type_name(ros_type)
}

/// Returns the 32-byte digest a RIHS01 hash string carries, or `None` if the
/// string is malformed.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ros2_type_hash_digest(text: &str) -> Option<Vec<u8>> {
    TypeHash::parse(text).map(|hash| hash.digest().to_vec())
}

/// Builds the Zenoh key an `rmw_zenoh` peer publishes an entity on, or `None`
/// if the name, type, or hash is not usable.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ros2_entity_key(
    domain_id: u32,
    fqn: &str,
    ros_type: &str,
    type_hash: &str,
) -> Option<String> {
    let hash = TypeHash::parse(type_hash)?;
    entity_key(domain_id, fqn, ros_type, &hash)
}

/// Encodes a twist into its CDR representation.
///
/// The linear and angular velocities each cross as an `(x, y, z)` triple.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ros2_twist_to_cdr(linear: (f64, f64, f64), angular: (f64, f64, f64)) -> Vec<u8> {
    Twist {
        linear: Vector3::new(linear.0, linear.1, linear.2),
        angular: Vector3::new(angular.0, angular.1, angular.2),
    }
    .to_cdr()
}

/// Decodes a twist from its CDR representation, or `None` if the bytes are not
/// a well-formed twist.
///
/// Returns the linear and angular velocities as two `(x, y, z)` triples.
#[gen_stub_pyfunction]
#[pyfunction]
#[allow(clippy::type_complexity)]
pub fn ros2_twist_from_cdr(data: Vec<u8>) -> Option<((f64, f64, f64), (f64, f64, f64))> {
    Twist::from_cdr(&data).map(|twist| {
        (
            (twist.linear.x, twist.linear.y, twist.linear.z),
            (twist.angular.x, twist.angular.y, twist.angular.z),
        )
    })
}

/// A CDR encoder, which writes primitives with the alignment the wire format
/// requires.
#[gen_stub_pyclass]
#[pyclass]
pub struct CdrWriter {
    inner: Mutex<CoreWriter>,
}

#[gen_stub_pymethods]
#[pymethods]
impl CdrWriter {
    /// Creates an encoder with the encapsulation header already written.
    #[new]
    fn new() -> Self {
        Self {
            inner: Mutex::new(CoreWriter::new()),
        }
    }

    /// Appends a 32-bit signed integer.
    fn write_i32(&self, value: i32) -> PyResult<()> {
        self.locked()?.write_i32(value);
        Ok(())
    }

    /// Appends a 32-bit unsigned integer.
    fn write_u32(&self, value: u32) -> PyResult<()> {
        self.locked()?.write_u32(value);
        Ok(())
    }

    /// Appends a 32-bit float.
    fn write_f32(&self, value: f32) -> PyResult<()> {
        self.locked()?.write_f32(value);
        Ok(())
    }

    /// Appends a 64-bit float.
    fn write_f64(&self, value: f64) -> PyResult<()> {
        self.locked()?.write_f64(value);
        Ok(())
    }

    /// The bytes written so far.
    #[getter]
    fn bytes(&self) -> PyResult<Vec<u8>> {
        Ok(self.locked()?.clone().into_bytes())
    }
}

impl CdrWriter {
    /// Locks the encoder, which a shared reference has to do because Python
    /// hands every method one.
    fn locked(&self) -> PyResult<std::sync::MutexGuard<'_, CoreWriter>> {
        self.inner
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("this writer is poisoned"))
    }
}

/// A CDR decoder, which reads primitives back in the order they were written.
///
/// Reading past the end returns `None` rather than raising, because a short
/// buffer is a wire condition rather than a programming error.
#[gen_stub_pyclass]
#[pyclass]
pub struct CdrReader {
    state: Mutex<Cursor>,
}

/// The bytes a decoder walks and the field widths it has already taken.
///
/// The core reader borrows the buffer and keeps its cursor private, so this
/// holds the bytes and rebuilds the cursor per call, replaying the widths read
/// so far. Alignment follows the width, so replaying by width lands where the
/// original sequence did.
struct Cursor {
    data: Vec<u8>,
    taken: Vec<bool>,
}

#[gen_stub_pymethods]
#[pymethods]
impl CdrReader {
    /// Creates a decoder over encoded bytes.
    ///
    /// Raises `ValueError` if the bytes carry no valid CDR encapsulation header.
    #[new]
    fn new(data: Vec<u8>) -> PyResult<Self> {
        if CoreReader::new(&data).is_none() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "the bytes carry no valid CDR encapsulation header",
            ));
        }
        Ok(Self {
            state: Mutex::new(Cursor {
                data,
                taken: Vec::new(),
            }),
        })
    }

    /// Reads the next 32-bit signed integer, or `None` once exhausted.
    fn read_i32(&self) -> PyResult<Option<i32>> {
        self.read(false, |cursor| cursor.read_i32())
    }

    /// Reads the next 32-bit unsigned integer, or `None` once exhausted.
    fn read_u32(&self) -> PyResult<Option<u32>> {
        self.read(false, |cursor| cursor.read_u32())
    }

    /// Reads the next 32-bit float, or `None` once exhausted.
    fn read_f32(&self) -> PyResult<Option<f32>> {
        self.read(false, |cursor| cursor.read_f32())
    }

    /// Reads the next 64-bit float, or `None` once exhausted.
    fn read_f64(&self) -> PyResult<Option<f64>> {
        self.read(true, |cursor| cursor.read_f64())
    }
}

impl CdrReader {
    /// Reads one field, replaying the fields already taken to reach the cursor.
    fn read<T>(
        &self,
        wide: bool,
        read: impl FnOnce(&mut CoreReader<'_>) -> Option<T>,
    ) -> PyResult<Option<T>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("this reader is poisoned"))?;
        let Some(mut cursor) = CoreReader::new(&state.data) else {
            return Ok(None);
        };
        for &taken_wide in &state.taken {
            let stepped = if taken_wide {
                cursor.read_f64().is_some()
            } else {
                cursor.read_u32().is_some()
            };
            if !stepped {
                return Ok(None);
            }
        }
        match read(&mut cursor) {
            Some(value) => {
                state.taken.push(wide);
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
}
