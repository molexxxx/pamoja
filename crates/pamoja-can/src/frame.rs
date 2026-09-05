//! The CAN data frame and the length-to-DLC encoding it uses.

use crate::error::CanError;
use crate::id::CanId;

// The largest payload a CAN-FD frame can carry.
const MAX_FD_DATA: usize = 64;

/// Maps a data length to the data-length code that represents it.
///
/// Classic CAN and the first nine CAN-FD codes are the length itself, 0 through 8. Above
/// that CAN-FD jumps in steps, so a length between two steps rounds up to the next code.
///
/// # Arguments
///
/// * `len` - the data length in bytes.
///
/// # Returns
///
/// The 4-bit data-length code.
///
/// # Examples
///
/// ```
/// use pamoja_can::len_to_dlc;
///
/// assert_eq!(len_to_dlc(8), 8);
/// assert_eq!(len_to_dlc(12), 9);
/// assert_eq!(len_to_dlc(64), 15);
/// ```
pub fn len_to_dlc(len: usize) -> u8 {
    match len {
        0..=8 => len as u8,
        9..=12 => 9,
        13..=16 => 10,
        17..=20 => 11,
        21..=24 => 12,
        25..=32 => 13,
        33..=48 => 14,
        _ => 15,
    }
}

/// Maps a data-length code to the number of bytes it represents.
///
/// # Arguments
///
/// * `dlc` - the data-length code; only its low four bits are used.
///
/// # Returns
///
/// The data length in bytes.
///
/// # Examples
///
/// ```
/// use pamoja_can::dlc_to_len;
///
/// assert_eq!(dlc_to_len(8), 8);
/// assert_eq!(dlc_to_len(15), 64);
/// ```
pub fn dlc_to_len(dlc: u8) -> usize {
    match dlc & 0x0F {
        small @ 0..=8 => small as usize,
        9 => 12,
        10 => 16,
        11 => 20,
        12 => 24,
        13 => 32,
        14 => 48,
        _ => 64,
    }
}

// Reports whether a length is one a CAN-FD frame can carry exactly.
fn is_fd_length(len: usize) -> bool {
    matches!(len, 0..=8 | 12 | 16 | 20 | 24 | 32 | 48 | 64)
}

/// A CAN frame: an identifier and its data.
///
/// Holds a classic CAN 2.0 frame (up to 8 bytes), a CAN-FD frame (up to 64 bytes at the
/// discrete CAN-FD lengths), or a classic remote frame, which requests data and carries
/// none. The data lives in a fixed buffer, so building a frame never allocates.
///
/// # Examples
///
/// ```
/// use pamoja_can::{CanId, Frame};
///
/// let frame = Frame::new(CanId::standard(0x100), &[0x01, 0x02, 0x03]).unwrap();
/// assert_eq!(frame.data(), &[0x01, 0x02, 0x03]);
/// assert_eq!(frame.dlc(), 3);
/// assert!(!frame.is_fd());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    id: CanId,
    data: [u8; MAX_FD_DATA],
    len: usize,
    fd: bool,
    remote: bool,
}

impl Frame {
    /// Builds a classic CAN 2.0 data frame.
    ///
    /// # Arguments
    ///
    /// * `id` - the arbitration identifier.
    /// * `data` - the payload, at most 8 bytes.
    ///
    /// # Returns
    ///
    /// The frame.
    ///
    /// # Errors
    ///
    /// Returns [`CanError::DataTooLong`] if `data` is longer than 8 bytes.
    pub fn new(id: CanId, data: &[u8]) -> Result<Frame, CanError> {
        if data.len() > 8 {
            return Err(CanError::DataTooLong);
        }
        Ok(Self::store(id, data, false, false))
    }

    /// Builds a CAN-FD data frame.
    ///
    /// # Arguments
    ///
    /// * `id` - the arbitration identifier.
    /// * `data` - the payload, at one of the discrete CAN-FD lengths up to 64 bytes.
    ///
    /// # Returns
    ///
    /// The frame.
    ///
    /// # Errors
    ///
    /// Returns [`CanError::DataTooLong`] if `data` is longer than 64 bytes, or
    /// [`CanError::InvalidFdLength`] if its length is not one CAN-FD can carry.
    pub fn fd(id: CanId, data: &[u8]) -> Result<Frame, CanError> {
        if data.len() > MAX_FD_DATA {
            return Err(CanError::DataTooLong);
        }
        if !is_fd_length(data.len()) {
            return Err(CanError::InvalidFdLength);
        }
        Ok(Self::store(id, data, true, false))
    }

    /// Builds a classic remote frame, which requests data of a given length and carries
    /// none.
    ///
    /// # Arguments
    ///
    /// * `id` - the arbitration identifier.
    /// * `len` - the data length being requested, clamped to 8 bytes.
    ///
    /// # Returns
    ///
    /// The remote frame.
    pub fn remote(id: CanId, len: usize) -> Frame {
        Frame {
            id,
            data: [0; MAX_FD_DATA],
            len: len.min(8),
            fd: false,
            remote: true,
        }
    }

    fn store(id: CanId, data: &[u8], fd: bool, remote: bool) -> Frame {
        let mut buf = [0; MAX_FD_DATA];
        buf[..data.len()].copy_from_slice(data);
        Frame {
            id,
            data: buf,
            len: data.len(),
            fd,
            remote,
        }
    }

    /// Returns the arbitration identifier.
    ///
    /// # Returns
    ///
    /// The identifier.
    pub fn id(&self) -> CanId {
        self.id
    }

    /// Returns the frame's data.
    ///
    /// # Returns
    ///
    /// The payload bytes, or an empty slice for a remote frame.
    pub fn data(&self) -> &[u8] {
        if self.remote {
            &[]
        } else {
            &self.data[..self.len]
        }
    }

    /// Reads the frame's data as J1939 signals.
    ///
    /// A J1939 message always carries exactly eight bytes, addressed by the offsets its
    /// parameter group publishes, so this hands back a view that reads them by offset
    /// rather than a slice a caller has to index.
    ///
    /// # Returns
    ///
    /// The eight data bytes as [`Signals`](crate::Signals), or `None` if this frame does
    /// not carry exactly eight, which a J1939 message always does.
    pub fn signals(&self) -> Option<crate::Signals> {
        let bytes: [u8; 8] = self.data().try_into().ok()?;
        Some(crate::Signals::from_bytes(bytes))
    }

    /// Returns the data length: the payload length, or the requested length for a remote
    /// frame.
    ///
    /// # Returns
    ///
    /// The length in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Reports whether the frame carries no data.
    ///
    /// # Returns
    ///
    /// `true` if the length is zero.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the data-length code for this frame's length.
    ///
    /// # Returns
    ///
    /// The 4-bit data-length code.
    pub fn dlc(&self) -> u8 {
        len_to_dlc(self.len)
    }

    /// Reports whether this is a CAN-FD frame.
    ///
    /// # Returns
    ///
    /// `true` for a CAN-FD frame.
    pub fn is_fd(&self) -> bool {
        self.fd
    }

    /// Reports whether this is a remote frame.
    ///
    /// # Returns
    ///
    /// `true` for a remote frame.
    pub fn is_remote(&self) -> bool {
        self.remote
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_classic_frame_holds_its_data() {
        let frame = Frame::new(CanId::standard(0x100), &[1, 2, 3, 4]).unwrap();
        assert_eq!(frame.data(), &[1, 2, 3, 4]);
        assert_eq!(frame.len(), 4);
        assert_eq!(frame.dlc(), 4);
        assert!(!frame.is_fd());
        assert!(!frame.is_remote());
    }

    #[test]
    fn a_classic_frame_rejects_more_than_eight_bytes() {
        assert_eq!(
            Frame::new(CanId::standard(0x100), &[0; 9]),
            Err(CanError::DataTooLong)
        );
    }

    #[test]
    fn an_fd_frame_carries_up_to_sixty_four_bytes() {
        let frame = Frame::fd(CanId::extended(0x1234), &[0xAB; 64]).unwrap();
        assert_eq!(frame.len(), 64);
        assert_eq!(frame.dlc(), 15);
        assert!(frame.is_fd());
    }

    #[test]
    fn an_fd_frame_rejects_a_length_it_cannot_carry() {
        // 9 bytes is not a valid CAN-FD length.
        assert_eq!(
            Frame::fd(CanId::standard(0x1), &[0; 9]),
            Err(CanError::InvalidFdLength)
        );
    }

    #[test]
    fn an_fd_frame_rejects_more_than_sixty_four_bytes() {
        assert_eq!(
            Frame::fd(CanId::standard(0x1), &[0; 65]),
            Err(CanError::DataTooLong)
        );
    }

    #[test]
    fn a_remote_frame_requests_a_length_and_carries_no_data() {
        let frame = Frame::remote(CanId::standard(0x200), 8);
        assert!(frame.is_remote());
        assert_eq!(frame.data(), &[]);
        assert_eq!(frame.len(), 8);
        assert_eq!(frame.dlc(), 8);
    }

    #[test]
    fn the_dlc_encoding_round_trips_at_each_step() {
        for &len in &[0usize, 1, 8, 12, 16, 20, 24, 32, 48, 64] {
            assert_eq!(dlc_to_len(len_to_dlc(len)), len);
        }
    }

    #[test]
    fn a_length_between_steps_rounds_up() {
        assert_eq!(len_to_dlc(9), 9); // the code for 12 bytes
        assert_eq!(dlc_to_len(9), 12);
        assert_eq!(len_to_dlc(33), 14); // the code for 48 bytes
        assert_eq!(dlc_to_len(14), 48);
    }

    #[test]
    fn a_zero_length_fd_frame_is_valid() {
        let frame = Frame::fd(CanId::standard(0x1), &[]).unwrap();
        assert!(frame.is_fd());
        assert!(frame.is_empty());
        assert_eq!(frame.len(), 0);
        assert_eq!(frame.dlc(), 0);
        assert_eq!(frame.data(), &[]);
    }

    #[test]
    fn a_classic_frame_can_be_empty() {
        let frame = Frame::new(CanId::standard(0x1), &[]).unwrap();
        assert!(frame.is_empty());
        assert_eq!(frame.dlc(), 0);
    }
}
