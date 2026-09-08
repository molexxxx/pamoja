//! CDR serialization and the geometry messages a robot is driven by.
//!
//! ROS 2 and `rmw_zenoh` put messages on the wire as CDR, the OMG Common Data Representation. A
//! CDR stream opens with a four-byte encapsulation header naming the byte order, after which each
//! primitive is written in that byte order and aligned to its own size relative to the start of the
//! body. Getting the alignment padding wrong is the classic CDR bug, so the [`CdrWriter`] and
//! [`CdrReader`] handle it once, and the message types build on them. This slice covers the
//! little-endian encapsulation and the geometry messages used to command motion; more messages and
//! big-endian decoding arrive with the live bridge.

use alloc::vec::Vec;

// The CDR encapsulation header is four bytes: a two-byte representation identifier and two option
// bytes. `00 01` selects classic CDR, little-endian; the options are unused here.
const ENCAPSULATION: [u8; 4] = [0x00, 0x01, 0x00, 0x00];
const ENCAPSULATION_LEN: usize = 4;

/// Writes primitives as little-endian CDR, handling alignment padding.
///
/// The writer starts with the little-endian encapsulation header; each write aligns the cursor to
/// the value's size (measured from the start of the body) before appending the bytes.
///
/// # Examples
///
/// ```
/// use pamoja_ros2::msg::CdrWriter;
///
/// let mut w = CdrWriter::new();
/// w.write_f64(1.0);
/// // Four-byte header plus eight bytes for the double.
/// assert_eq!(w.into_bytes().len(), 12);
/// ```
#[derive(Clone, Debug, Default)]
pub struct CdrWriter {
    buf: Vec<u8>,
}

impl CdrWriter {
    /// Creates a writer primed with the little-endian CDR encapsulation header.
    ///
    /// # Returns
    ///
    /// The writer.
    pub fn new() -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&ENCAPSULATION);
        Self { buf }
    }

    fn align(&mut self, alignment: usize) {
        let offset = self.buf.len() - ENCAPSULATION_LEN;
        let padding = (alignment - (offset % alignment)) % alignment;
        self.buf.resize(self.buf.len() + padding, 0);
    }

    /// Writes a 32-bit signed integer.
    ///
    /// # Arguments
    ///
    /// * `value` - the integer to write.
    pub fn write_i32(&mut self, value: i32) {
        self.align(4);
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a 32-bit unsigned integer.
    ///
    /// # Arguments
    ///
    /// * `value` - the integer to write.
    pub fn write_u32(&mut self, value: u32) {
        self.align(4);
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a 32-bit float.
    ///
    /// # Arguments
    ///
    /// * `value` - the float to write.
    pub fn write_f32(&mut self, value: f32) {
        self.align(4);
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a 64-bit float.
    ///
    /// # Arguments
    ///
    /// * `value` - the float to write.
    pub fn write_f64(&mut self, value: f64) {
        self.align(8);
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Consumes the writer and returns the encoded bytes, header included.
    ///
    /// # Returns
    ///
    /// The CDR-encoded buffer.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

/// Reads primitives from a little-endian CDR buffer, handling alignment padding.
///
/// The reader checks the encapsulation header on construction and then mirrors [`CdrWriter`]'s
/// alignment, so a value written by the writer is read back identically.
pub struct CdrReader<'a> {
    body: &'a [u8],
    pos: usize,
}

impl<'a> CdrReader<'a> {
    /// Creates a reader over a CDR buffer.
    ///
    /// # Arguments
    ///
    /// * `data` - the CDR-encoded buffer, including the four-byte encapsulation header.
    ///
    /// # Returns
    ///
    /// `Some(reader)` if `data` carries a classic little-endian CDR header; `None` otherwise,
    /// including a buffer too short to hold a header or one declaring a byte order this reader does
    /// not decode.
    pub fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < ENCAPSULATION_LEN || data[0] != 0x00 || data[1] != 0x01 {
            return None;
        }
        Some(Self {
            body: &data[ENCAPSULATION_LEN..],
            pos: 0,
        })
    }

    fn align(&mut self, alignment: usize) {
        let padding = (alignment - (self.pos % alignment)) % alignment;
        self.pos += padding;
    }

    fn take<const N: usize>(&mut self, alignment: usize) -> Option<[u8; N]> {
        self.align(alignment);
        let end = self.pos.checked_add(N)?;
        if end > self.body.len() {
            return None;
        }
        let bytes: [u8; N] = self.body[self.pos..end].try_into().ok()?;
        self.pos = end;
        Some(bytes)
    }

    /// Reads a 32-bit signed integer.
    ///
    /// # Returns
    ///
    /// `Some(value)`, or `None` if the buffer is exhausted.
    pub fn read_i32(&mut self) -> Option<i32> {
        self.take::<4>(4).map(i32::from_le_bytes)
    }

    /// Reads a 32-bit unsigned integer.
    ///
    /// # Returns
    ///
    /// `Some(value)`, or `None` if the buffer is exhausted.
    pub fn read_u32(&mut self) -> Option<u32> {
        self.take::<4>(4).map(u32::from_le_bytes)
    }

    /// Reads a 32-bit float.
    ///
    /// # Returns
    ///
    /// `Some(value)`, or `None` if the buffer is exhausted.
    pub fn read_f32(&mut self) -> Option<f32> {
        self.take::<4>(4).map(f32::from_le_bytes)
    }

    /// Reads a 64-bit float.
    ///
    /// # Returns
    ///
    /// `Some(value)`, or `None` if the buffer is exhausted.
    pub fn read_f64(&mut self) -> Option<f64> {
        self.take::<8>(8).map(f64::from_le_bytes)
    }
}

/// A three-dimensional vector (`geometry_msgs/msg/Vector3`): three 64-bit floats.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector3 {
    /// The x component.
    pub x: f64,
    /// The y component.
    pub y: f64,
    /// The z component.
    pub z: f64,
}

impl Vector3 {
    /// Creates a vector from its components.
    ///
    /// # Arguments
    ///
    /// * `x` - the x component.
    /// * `y` - the y component.
    /// * `z` - the z component.
    ///
    /// # Returns
    ///
    /// The vector.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Encodes the vector into a CDR writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - the writer to append to.
    pub fn encode(&self, writer: &mut CdrWriter) {
        writer.write_f64(self.x);
        writer.write_f64(self.y);
        writer.write_f64(self.z);
    }

    /// Decodes a vector from a CDR reader.
    ///
    /// # Arguments
    ///
    /// * `reader` - the reader to consume from.
    ///
    /// # Returns
    ///
    /// `Some(vector)`, or `None` if the buffer is exhausted.
    pub fn decode(reader: &mut CdrReader) -> Option<Self> {
        Some(Self {
            x: reader.read_f64()?,
            y: reader.read_f64()?,
            z: reader.read_f64()?,
        })
    }
}

/// A body velocity command (`geometry_msgs/msg/Twist`): a linear and an angular [`Vector3`].
///
/// This is the message a ROS 2 robot is driven by on `cmd_vel`, the natural target for the body
/// twists the `pamoja-kit` chassis and navigation helpers produce.
///
/// # Examples
///
/// ```
/// use pamoja_ros2::msg::{Twist, Vector3};
///
/// let cmd = Twist {
///     linear: Vector3::new(0.5, 0.0, 0.0),
///     angular: Vector3::new(0.0, 0.0, 0.2),
/// };
/// assert_eq!(Twist::from_cdr(&cmd.to_cdr()), Some(cmd));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Twist {
    /// The linear velocity, in meters per second.
    pub linear: Vector3,
    /// The angular velocity, in radians per second.
    pub angular: Vector3,
}

impl Twist {
    /// Encodes the twist as a CDR message.
    ///
    /// # Returns
    ///
    /// The CDR-encoded bytes, header included.
    pub fn to_cdr(&self) -> Vec<u8> {
        let mut writer = CdrWriter::new();
        self.linear.encode(&mut writer);
        self.angular.encode(&mut writer);
        writer.into_bytes()
    }

    /// Decodes a twist from a CDR message.
    ///
    /// # Arguments
    ///
    /// * `data` - the CDR-encoded bytes, header included.
    ///
    /// # Returns
    ///
    /// `Some(twist)`, or `None` if the buffer is not a valid little-endian CDR twist.
    pub fn from_cdr(data: &[u8]) -> Option<Self> {
        let mut reader = CdrReader::new(data)?;
        let linear = Vector3::decode(&mut reader)?;
        let angular = Vector3::decode(&mut reader)?;
        Some(Self { linear, angular })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_pads_a_double_after_an_int() {
        // A u32 then an f64: the double must align to an 8-byte boundary in the body, so four
        // padding bytes sit between them.
        let mut w = CdrWriter::new();
        w.write_u32(0x0102_0304);
        w.write_f64(1.0);
        let bytes = w.into_bytes();
        assert_eq!(
            bytes,
            [
                0x00, 0x01, 0x00, 0x00, // encapsulation header
                0x04, 0x03, 0x02, 0x01, // u32, little-endian
                0x00, 0x00, 0x00, 0x00, // alignment padding to offset 8
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, // 1.0_f64, little-endian
            ]
        );
    }

    #[test]
    fn twist_matches_a_hand_computed_cdr_vector() {
        let cmd = Twist {
            linear: Vector3::new(1.0, 0.0, 0.0),
            angular: Vector3::new(0.0, 0.0, 0.5),
        };
        let bytes = cmd.to_cdr();
        let mut expected = Vec::new();
        expected.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // header
        expected.extend_from_slice(&1.0_f64.to_le_bytes()); // linear.x
        expected.extend_from_slice(&0.0_f64.to_le_bytes()); // linear.y
        expected.extend_from_slice(&0.0_f64.to_le_bytes()); // linear.z
        expected.extend_from_slice(&0.0_f64.to_le_bytes()); // angular.x
        expected.extend_from_slice(&0.0_f64.to_le_bytes()); // angular.y
        expected.extend_from_slice(&0.5_f64.to_le_bytes()); // angular.z
        assert_eq!(bytes, expected);
        assert_eq!(bytes.len(), 4 + 48);
    }

    #[test]
    fn twist_round_trips_through_cdr() {
        let cmd = Twist {
            linear: Vector3::new(0.5, -1.5, 0.0),
            angular: Vector3::new(0.0, 0.0, 0.25),
        };
        assert_eq!(Twist::from_cdr(&cmd.to_cdr()), Some(cmd));
    }

    #[test]
    fn primitives_round_trip_with_alignment() {
        let mut w = CdrWriter::new();
        w.write_i32(-7);
        w.write_f64(2.5);
        w.write_f32(1.25);
        w.write_u32(42);
        let bytes = w.into_bytes();

        let mut r = CdrReader::new(&bytes).unwrap();
        assert_eq!(r.read_i32(), Some(-7));
        assert_eq!(r.read_f64(), Some(2.5));
        assert_eq!(r.read_f32(), Some(1.25));
        assert_eq!(r.read_u32(), Some(42));
    }

    #[test]
    fn a_short_or_wrong_endian_buffer_is_rejected() {
        assert!(CdrReader::new(&[0x00]).is_none()); // too short for a header
        assert!(CdrReader::new(&[0x00, 0x00, 0x00, 0x00]).is_none()); // big-endian, not decoded here
        assert!(Twist::from_cdr(&[0x00, 0x01, 0x00, 0x00]).is_none()); // header but no body
    }
}
