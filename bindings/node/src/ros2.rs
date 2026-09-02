//! Generated Node bindings for the ROS 2 naming and encoding rules.
//!
//! These mirror the `pamoja-ros2` Rust API: what makes a topic name legal, what
//! it becomes on the DDS wire, the RIHS type hash that identifies a message
//! definition, and the CDR encoding the payload itself is written in.
//!
//! None of it needs a ROS 2 installation, which is the point. A gateway written
//! in TypeScript can validate a name, derive the DDS topic and the Zenoh key an
//! `rmw_zenoh` peer subscribes on, and encode a `geometry_msgs/msg/Twist`
//! without a ROS distribution anywhere near it. Driving a live graph does need
//! one, so the Rust crate's `bridge` feature stays Rust-only.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_ros2::key::entity_key;
use pamoja_ros2::msg::{CdrReader as CoreReader, CdrWriter as CoreWriter, Twist, Vector3};
use pamoja_ros2::name::{dds_topic, is_fully_qualified, is_valid_name, percent_mangle, EntityKind};
use pamoja_ros2::typehash::{dds_type_name, TypeHash};

/// The ROS 2 subsystem a name belongs to, which fixes its DDS prefix.
#[napi(string_enum)]
pub enum EntityKindName {
    /// A topic, which takes the `rt` prefix.
    Topic,
    /// The request side of a service, which takes the `rq` prefix.
    ServiceRequest,
    /// The reply side of a service, which takes the `rr` prefix.
    ServiceResponse,
}

impl From<EntityKindName> for EntityKind {
    fn from(kind: EntityKindName) -> Self {
        match kind {
            EntityKindName::Topic => EntityKind::Topic,
            EntityKindName::ServiceRequest => EntityKind::ServiceRequest,
            EntityKindName::ServiceResponse => EntityKind::ServiceResponse,
        }
    }
}

/// A three-dimensional vector, matching `geometry_msgs/msg/Vector3`.
#[napi(object)]
pub struct Ros2Vector3 {
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
/// chassis or navigation helper publishes into a ROS graph.
#[napi(object)]
pub struct Ros2Twist {
    /// The linear velocity in metres per second.
    pub linear: Ros2Vector3,
    /// The angular velocity in radians per second.
    pub angular: Ros2Vector3,
}

/// Reports whether a string is a valid ROS 2 topic or service name.
#[napi]
pub fn ros2_is_valid_name(name: String) -> bool {
    is_valid_name(&name)
}

/// Reports whether a name is fully qualified, so it resolves with no namespace.
#[napi]
pub fn ros2_is_fully_qualified(name: String) -> bool {
    is_fully_qualified(&name)
}

/// Returns the DDS topic prefix a subsystem uses.
#[napi]
pub fn ros2_entity_kind_prefix(kind: EntityKindName) -> String {
    EntityKind::from(kind).prefix().to_owned()
}

/// Returns the DDS topic a fully qualified name maps onto, or `null` if the
/// name is not fully qualified.
#[napi]
pub fn ros2_dds_topic(fqn: String, kind: EntityKindName) -> Option<String> {
    dds_topic(&fqn, kind.into())
}

/// Percent-mangles a name the way a DDS partition requires.
#[napi]
pub fn ros2_percent_mangle(name: String) -> String {
    percent_mangle(&name)
}

/// Returns the DDS type name an interface type maps onto, or `null` if the type
/// is not a valid `package/namespace/Type`.
#[napi]
pub fn ros2_dds_type_name(ros_type: String) -> Option<String> {
    dds_type_name(&ros_type)
}

/// Returns the 32-byte digest a RIHS01 hash string carries, or `null` if the
/// string is malformed.
#[napi]
pub fn ros2_type_hash_digest(text: String) -> Option<Buffer> {
    TypeHash::parse(&text).map(|hash| Buffer::from(hash.digest().to_vec()))
}

/// Builds the Zenoh key an `rmw_zenoh` peer publishes an entity on, or `null`
/// if the name, type, or hash is not usable.
///
/// @param domainId - the ROS 2 domain.
/// @param fqn - the fully qualified entity name.
/// @param rosType - the interface type as `package/namespace/Type`.
/// @param typeHash - the message type hash as its `RIHS01_` string.
#[napi]
pub fn ros2_entity_key(
    domain_id: u32,
    fqn: String,
    ros_type: String,
    type_hash: String,
) -> Option<String> {
    let hash = TypeHash::parse(&type_hash)?;
    entity_key(domain_id, &fqn, &ros_type, &hash)
}

/// Encodes a twist into its CDR representation.
#[napi]
pub fn ros2_twist_to_cdr(twist: Ros2Twist) -> Buffer {
    let twist = Twist {
        linear: Vector3::new(twist.linear.x, twist.linear.y, twist.linear.z),
        angular: Vector3::new(twist.angular.x, twist.angular.y, twist.angular.z),
    };
    Buffer::from(twist.to_cdr())
}

/// Decodes a twist from its CDR representation, or `null` if the bytes are not
/// a well-formed twist.
#[napi]
pub fn ros2_twist_from_cdr(data: Buffer) -> Option<Ros2Twist> {
    Twist::from_cdr(&data).map(|twist| Ros2Twist {
        linear: Ros2Vector3 {
            x: twist.linear.x,
            y: twist.linear.y,
            z: twist.linear.z,
        },
        angular: Ros2Vector3 {
            x: twist.angular.x,
            y: twist.angular.y,
            z: twist.angular.z,
        },
    })
}

/// A CDR encoder, which writes primitives with the alignment the wire format
/// requires.
#[napi]
pub struct CdrWriter {
    inner: CoreWriter,
}

#[napi]
impl CdrWriter {
    /// Creates an encoder with the encapsulation header already written.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreWriter::new(),
        }
    }

    /// Appends a 32-bit signed integer.
    #[napi]
    pub fn write_i32(&mut self, value: i32) {
        self.inner.write_i32(value);
    }

    /// Appends a 32-bit unsigned integer.
    #[napi]
    pub fn write_u32(&mut self, value: u32) {
        self.inner.write_u32(value);
    }

    /// Appends a 32-bit float.
    #[napi]
    pub fn write_f32(&mut self, value: f64) {
        self.inner.write_f32(value as f32);
    }

    /// Appends a 64-bit float.
    #[napi]
    pub fn write_f64(&mut self, value: f64) {
        self.inner.write_f64(value);
    }

    /// The bytes written so far.
    #[napi(getter)]
    pub fn bytes(&self) -> Buffer {
        Buffer::from(self.inner.clone().into_bytes())
    }
}

impl Default for CdrWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// A CDR decoder, which reads primitives back in the order they were written.
///
/// Reading past the end returns `null` rather than throwing, because a short
/// buffer is a wire condition rather than a programming error.
#[napi]
pub struct CdrReader {
    data: Vec<u8>,
    taken: Vec<bool>,
}

#[napi]
impl CdrReader {
    /// Creates a decoder over encoded bytes.
    ///
    /// Throws if the bytes carry no valid CDR encapsulation header.
    #[napi(constructor)]
    pub fn new(data: Buffer) -> napi::Result<Self> {
        let data = data.to_vec();
        if CoreReader::new(&data).is_none() {
            return Err(napi::Error::from_reason(
                "the bytes carry no valid CDR encapsulation header",
            ));
        }
        Ok(Self {
            data,
            taken: Vec::new(),
        })
    }

    /// Reads the next 32-bit signed integer, or `null` once exhausted.
    #[napi]
    pub fn read_i32(&mut self) -> Option<i32> {
        self.read(false, |cursor| cursor.read_i32())
    }

    /// Reads the next 32-bit unsigned integer, or `null` once exhausted.
    #[napi]
    pub fn read_u32(&mut self) -> Option<u32> {
        self.read(false, |cursor| cursor.read_u32())
    }

    /// Reads the next 32-bit float, or `null` once exhausted.
    #[napi]
    pub fn read_f32(&mut self) -> Option<f64> {
        self.read(false, |cursor| cursor.read_f32()).map(f64::from)
    }

    /// Reads the next 64-bit float, or `null` once exhausted.
    #[napi]
    pub fn read_f64(&mut self) -> Option<f64> {
        self.read(true, |cursor| cursor.read_f64())
    }
}

impl CdrReader {
    /// Reads one field, replaying the fields already taken to reach the cursor.
    ///
    /// The core reader borrows the buffer it walks and keeps its cursor private,
    /// so this holds the bytes and the widths read so far and rebuilds the
    /// cursor per call. Alignment follows the width, so replaying by width lands
    /// where the original sequence did.
    fn read<T>(
        &mut self,
        wide: bool,
        read: impl FnOnce(&mut CoreReader<'_>) -> Option<T>,
    ) -> Option<T> {
        let mut cursor = CoreReader::new(&self.data)?;
        for &taken_wide in &self.taken {
            if taken_wide {
                cursor.read_f64()?;
            } else {
                cursor.read_u32()?;
            }
        }
        let value = read(&mut cursor)?;
        self.taken.push(wide);
        Some(value)
    }
}
