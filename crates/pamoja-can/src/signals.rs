//! The eight data bytes of a J1939 frame, addressed by the signals inside them.
//!
//! A parameter group places each signal at a fixed byte offset, little-endian, with a
//! scale and an offset the standard publishes. Reading or writing one by hand means
//! slicing the payload and calling `from_le_bytes`, which is where an off-by-one goes
//! unnoticed. This module does that addressing, and starts a payload filled with the byte
//! the standard reserves for a signal the sender is not reporting, so a controller only
//! writes the signals it actually has.

/// The byte a J1939 sender writes for a signal it is not reporting.
///
/// A receiver reads this as "not available" rather than as a measurement, which is why a
/// payload starts filled with it instead of with zeros.
pub const NOT_AVAILABLE: u8 = 0xFF;

/// The eight data bytes of a J1939 frame.
///
/// # Examples
///
/// ```
/// use pamoja_can::Signals;
///
/// // Engine speed sits in bytes 4 and 5 of EEC1, at 0.125 rpm per bit.
/// let mut payload = Signals::new();
/// payload.set_u16(3, (1000.0 / 0.125) as u16);
///
/// assert_eq!(payload.u16(3), Some(8000));
/// assert_eq!(payload.u8(0), Some(pamoja_can::NOT_AVAILABLE));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signals {
    bytes: [u8; 8],
}

impl Default for Signals {
    fn default() -> Self {
        Self::new()
    }
}

impl Signals {
    /// Builds a payload with every signal marked not available.
    ///
    /// # Returns
    ///
    /// Eight bytes of [`NOT_AVAILABLE`], ready for a sender to write only what it has.
    pub fn new() -> Signals {
        Signals {
            bytes: [NOT_AVAILABLE; 8],
        }
    }

    /// Reads a payload received off the bus.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the eight data bytes of a received frame.
    ///
    /// # Returns
    ///
    /// The payload, ready for its signals to be read out.
    pub fn from_bytes(bytes: [u8; 8]) -> Signals {
        Signals { bytes }
    }

    /// Returns the eight data bytes, ready to put in a frame.
    ///
    /// # Returns
    ///
    /// The payload in wire order.
    pub fn as_bytes(&self) -> &[u8; 8] {
        &self.bytes
    }

    /// Writes a one-byte signal.
    ///
    /// # Arguments
    ///
    /// * `at` - the byte offset the parameter group places the signal at, `0..=7`.
    /// * `value` - the raw value, already scaled as the group defines.
    ///
    /// # Returns
    ///
    /// The payload, so writes chain.
    pub fn set_u8(&mut self, at: usize, value: u8) -> &mut Signals {
        if at < 8 {
            self.bytes[at] = value;
        }
        self
    }

    /// Writes a two-byte little-endian signal.
    ///
    /// # Arguments
    ///
    /// * `at` - the offset of the signal's first byte, `0..=6`.
    /// * `value` - the raw value, already scaled as the group defines.
    ///
    /// # Returns
    ///
    /// The payload, so writes chain.
    pub fn set_u16(&mut self, at: usize, value: u16) -> &mut Signals {
        if at + 1 < 8 {
            self.bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
        }
        self
    }

    /// Reads a one-byte signal.
    ///
    /// # Arguments
    ///
    /// * `at` - the byte offset the parameter group places the signal at.
    ///
    /// # Returns
    ///
    /// The raw value, or `None` if `at` is past the payload.
    pub fn u8(&self, at: usize) -> Option<u8> {
        self.bytes.get(at).copied()
    }

    /// Reads a two-byte little-endian signal.
    ///
    /// # Arguments
    ///
    /// * `at` - the offset of the signal's first byte.
    ///
    /// # Returns
    ///
    /// The raw value, or `None` if the signal would run past the payload.
    pub fn u16(&self, at: usize) -> Option<u16> {
        if at + 1 < 8 {
            Some(u16::from_le_bytes([self.bytes[at], self.bytes[at + 1]]))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_payload_reports_nothing() {
        let payload = Signals::new();
        assert_eq!(payload.as_bytes(), &[NOT_AVAILABLE; 8]);
        assert_eq!(payload.u8(0), Some(NOT_AVAILABLE));
        assert_eq!(payload.u16(0), Some(0xFFFF));
    }

    #[test]
    fn a_signal_reads_back_from_where_it_was_written() {
        let mut payload = Signals::new();
        payload.set_u16(3, 8_000).set_u8(2, 125);

        assert_eq!(payload.u16(3), Some(8_000));
        assert_eq!(payload.u8(2), Some(125));
        assert_eq!(payload.u8(0), Some(NOT_AVAILABLE));
        assert_eq!(Signals::from_bytes(*payload.as_bytes()), payload);
    }

    #[test]
    fn a_signal_past_the_payload_is_refused_rather_than_wrapped() {
        let mut payload = Signals::new();
        payload.set_u8(8, 1);
        payload.set_u16(7, 1);

        assert_eq!(payload.as_bytes(), &[NOT_AVAILABLE; 8]);
        assert_eq!(payload.u8(8), None);
        assert_eq!(payload.u16(7), None);
    }
}
