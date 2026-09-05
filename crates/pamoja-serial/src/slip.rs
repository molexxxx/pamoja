//! SLIP framing, the Serial Line Internet Protocol of RFC 1055.
//!
//! SLIP is the oldest and simplest way to put packets on a serial line. A single byte,
//! [`END`], marks the end of a packet. When that byte (or the escape byte [`ESC`]) appears
//! in the payload it is replaced by a two-byte escape sequence, so the delimiter can never
//! be mistaken for data. That is the whole protocol; its appeal is that it fits in a
//! handful of bytes of code on the smallest microcontroller.
//!
//! The four byte values are fixed by RFC 1055: [`END`] is `0xC0`, [`ESC`] is `0xDB`, and
//! the escape sequences are `ESC` `ESC_END` for a literal `END` and `ESC` `ESC_ESC` for a
//! literal `ESC`.

use crate::SerialError;

/// The SLIP frame delimiter, RFC 1055 `END` (octal 300).
pub const END: u8 = 0xC0;
/// The SLIP escape byte, RFC 1055 `ESC` (octal 333).
pub const ESC: u8 = 0xDB;
/// The byte that follows [`ESC`] to encode a literal [`END`], RFC 1055 `ESC_END` (octal 334).
pub const ESC_END: u8 = 0xDC;
/// The byte that follows [`ESC`] to encode a literal [`ESC`], RFC 1055 `ESC_ESC` (octal 335).
pub const ESC_ESC: u8 = 0xDD;

/// Returns an output length that is always large enough to hold the SLIP encoding of a
/// payload of `payload_len` bytes.
///
/// The worst case is a payload made entirely of [`END`] or [`ESC`] bytes, where every byte
/// becomes a two-byte escape sequence, plus the one trailing [`END`] delimiter.
///
/// # Arguments
///
/// * `payload_len` - the length of the payload to be encoded.
///
/// # Returns
///
/// The maximum number of bytes [`encode`] can write for that payload.
///
/// # Examples
///
/// ```
/// use pamoja_serial::slip;
///
/// assert_eq!(slip::max_encoded_len(10), 21);
/// ```
#[must_use]
pub const fn max_encoded_len(payload_len: usize) -> usize {
    payload_len * 2 + 1
}

/// Encodes a payload into a SLIP frame, terminated by an [`END`] byte.
///
/// Every [`END`] in the payload is written as `ESC` `ESC_END` and every [`ESC`] as `ESC`
/// `ESC_ESC`; all other bytes pass through unchanged. A single [`END`] is appended to mark
/// the end of the frame. A sender that wants RFC 1055's noise-flushing behaviour may
/// prepend an extra [`END`] of its own; [`decode`] and [`SlipDecoder`] ignore it.
///
/// # Arguments
///
/// * `payload` - the bytes to frame.
/// * `output` - the buffer the frame is written into; size it with [`max_encoded_len`].
///
/// # Returns
///
/// The number of bytes written to `output`.
///
/// # Errors
///
/// Returns [`SerialError::BufferTooSmall`] if `output` cannot hold the whole frame.
///
/// # Examples
///
/// ```
/// use pamoja_serial::slip::{self, END, ESC, ESC_END};
///
/// // A payload containing the delimiter byte is escaped, never emitted raw.
/// let mut frame = [0u8; 8];
/// let n = slip::encode(&[0x01, END, 0x02], &mut frame)?;
/// assert_eq!(&frame[..n], &[0x01, ESC, ESC_END, 0x02, END]);
/// # Ok::<(), pamoja_serial::SerialError>(())
/// ```
pub fn encode(payload: &[u8], output: &mut [u8]) -> Result<usize, SerialError> {
    let mut write = 0usize;
    for &byte in payload {
        match byte {
            END => {
                push(output, &mut write, ESC)?;
                push(output, &mut write, ESC_END)?;
            }
            ESC => {
                push(output, &mut write, ESC)?;
                push(output, &mut write, ESC_ESC)?;
            }
            other => push(output, &mut write, other)?,
        }
    }
    push(output, &mut write, END)?;
    Ok(write)
}

/// Decodes a single SLIP frame, recovering the original payload.
///
/// A leading [`END`] (RFC 1055's flush byte) and any empty run before the payload are
/// skipped; decoding stops at the [`END`] that closes the frame, or at the end of the
/// slice if it carries no trailing delimiter.
///
/// # Arguments
///
/// * `frame` - the framed bytes, with or without the trailing [`END`].
/// * `output` - the buffer the payload is written into; it never needs more room than
///   `frame`.
///
/// # Returns
///
/// The number of payload bytes written to `output`.
///
/// # Errors
///
/// Returns [`SerialError::InvalidEscape`] if an [`ESC`] is followed by an unexpected byte,
/// [`SerialError::TruncatedFrame`] if the frame ends in the middle of an escape sequence,
/// and [`SerialError::BufferTooSmall`] if `output` cannot hold the payload.
///
/// # Examples
///
/// ```
/// use pamoja_serial::slip::{self, END};
///
/// let mut payload = [0u8; 4];
/// // A leading END is tolerated and the trailing one closes the frame.
/// let n = slip::decode(&[END, b'h', b'i', END], &mut payload)?;
/// assert_eq!(&payload[..n], b"hi");
/// # Ok::<(), pamoja_serial::SerialError>(())
/// ```
pub fn decode(frame: &[u8], output: &mut [u8]) -> Result<usize, SerialError> {
    let mut write = 0usize;
    let mut in_escape = false;
    for &byte in frame {
        if byte == END {
            if in_escape {
                return Err(SerialError::TruncatedFrame);
            }
            // A delimiter with nothing buffered is a leading or repeated flush byte; keep
            // reading. Otherwise it closes the frame.
            if write == 0 {
                continue;
            }
            return Ok(write);
        }
        if in_escape {
            let decoded = match byte {
                ESC_END => END,
                ESC_ESC => ESC,
                _ => return Err(SerialError::InvalidEscape),
            };
            push(output, &mut write, decoded)?;
            in_escape = false;
        } else if byte == ESC {
            in_escape = true;
        } else {
            push(output, &mut write, byte)?;
        }
    }
    if in_escape {
        return Err(SerialError::TruncatedFrame);
    }
    Ok(write)
}

/// A streaming SLIP decoder that reassembles whole frames from a serial byte stream.
///
/// A serial read returns whatever bytes have arrived, which is rarely a whole packet, so a
/// real receive loop feeds bytes in as they come and acts on each frame as it completes.
/// `SlipDecoder` buffers up to `N` payload bytes; [`push`](SlipDecoder::push) returns the
/// finished payload when an [`END`] closes the frame, and `None` while one is still being
/// assembled.
///
/// # Examples
///
/// ```
/// use pamoja_serial::slip::{SlipDecoder, END};
///
/// let mut decoder: SlipDecoder<32> = SlipDecoder::new();
/// let mut frames = 0;
/// // Two packets arrive back to back in one read.
/// for &byte in &[b'o', b'k', END, b'g', b'o', END] {
///     if let Some(frame) = decoder.push(byte)? {
///         frames += 1;
///         assert!(frame == b"ok" || frame == b"go");
///     }
/// }
/// assert_eq!(frames, 2);
/// # Ok::<(), pamoja_serial::SerialError>(())
/// ```
#[derive(Debug)]
pub struct SlipDecoder<const N: usize> {
    buffer: [u8; N],
    len: usize,
    in_escape: bool,
    complete: bool,
}

impl<const N: usize> SlipDecoder<N> {
    /// Creates an empty decoder with room for an `N`-byte payload.
    ///
    /// # Returns
    ///
    /// A decoder ready to receive the first byte.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [0u8; N],
            len: 0,
            in_escape: false,
            complete: false,
        }
    }

    /// Discards any partly assembled frame, returning the decoder to its initial state.
    pub fn reset(&mut self) {
        self.len = 0;
        self.in_escape = false;
        self.complete = false;
    }

    /// Feeds one byte from the stream into the decoder.
    ///
    /// # Arguments
    ///
    /// * `byte` - the next byte received on the serial line.
    ///
    /// # Returns
    ///
    /// `Some(payload)` when this byte completed a frame, or `None` while a frame is still
    /// being assembled. An empty frame (a stray [`END`] with nothing buffered) is treated
    /// as a flush and yields `None`.
    ///
    /// # Errors
    ///
    /// Returns [`SerialError::InvalidEscape`] if an [`ESC`] is followed by an unexpected
    /// byte, [`SerialError::TruncatedFrame`] if an [`END`] arrives mid-escape, and
    /// [`SerialError::BufferTooSmall`] if the payload exceeds `N` bytes. After any error
    /// the partial frame is discarded and the decoder resumes at the next byte.
    pub fn push(&mut self, byte: u8) -> Result<Option<&[u8]>, SerialError> {
        if self.complete {
            self.reset();
        }
        if byte == END {
            if self.in_escape {
                self.reset();
                return Err(SerialError::TruncatedFrame);
            }
            if self.len == 0 {
                return Ok(None);
            }
            self.complete = true;
            return Ok(Some(&self.buffer[..self.len]));
        }
        if self.in_escape {
            let decoded = match byte {
                ESC_END => END,
                ESC_ESC => ESC,
                _ => {
                    self.reset();
                    return Err(SerialError::InvalidEscape);
                }
            };
            self.in_escape = false;
            self.store(decoded)?;
        } else if byte == ESC {
            self.in_escape = true;
        } else {
            self.store(byte)?;
        }
        Ok(None)
    }

    fn store(&mut self, byte: u8) -> Result<(), SerialError> {
        if self.len >= N {
            self.reset();
            return Err(SerialError::BufferTooSmall);
        }
        self.buffer[self.len] = byte;
        self.len += 1;
        Ok(())
    }
}

impl<const N: usize> Default for SlipDecoder<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Writes one byte into `output` at `write`, advancing it, or reports the buffer is full.
fn push(output: &mut [u8], write: &mut usize, byte: u8) -> Result<(), SerialError> {
    if *write >= output.len() {
        return Err(SerialError::BufferTooSmall);
    }
    output[*write] = byte;
    *write += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_rfc_1055() {
        // RFC 1055 fixes these in octal: 300, 333, 334, 335.
        assert_eq!(END, 0o300);
        assert_eq!(ESC, 0o333);
        assert_eq!(ESC_END, 0o334);
        assert_eq!(ESC_ESC, 0o335);
    }

    #[test]
    fn plain_payload_just_gets_a_trailing_end() {
        let mut frame = [0u8; 8];
        let n = encode(b"hi", &mut frame).unwrap();
        assert_eq!(&frame[..n], &[b'h', b'i', END]);
    }

    #[test]
    fn a_literal_end_byte_is_escaped() {
        let mut frame = [0u8; 8];
        let n = encode(&[END], &mut frame).unwrap();
        assert_eq!(&frame[..n], &[ESC, ESC_END, END]);
    }

    #[test]
    fn a_literal_esc_byte_is_escaped() {
        let mut frame = [0u8; 8];
        let n = encode(&[ESC], &mut frame).unwrap();
        assert_eq!(&frame[..n], &[ESC, ESC_ESC, END]);
    }

    #[test]
    fn both_specials_in_one_payload() {
        let mut frame = [0u8; 16];
        let n = encode(&[END, ESC, 0x01], &mut frame).unwrap();
        assert_eq!(&frame[..n], &[ESC, ESC_END, ESC, ESC_ESC, 0x01, END]);
    }

    #[test]
    fn round_trips_every_byte_value() {
        let payload: [u8; 256] = core::array::from_fn(|i| i as u8);
        let mut frame = [0u8; max_encoded_len(256)];
        let n = encode(&payload, &mut frame).unwrap();
        let mut out = [0u8; 256];
        let m = decode(&frame[..n], &mut out).unwrap();
        assert_eq!(&out[..m], &payload[..]);
    }

    #[test]
    fn decode_skips_a_leading_flush_end() {
        let mut out = [0u8; 4];
        let n = decode(&[END, b'h', b'i', END], &mut out).unwrap();
        assert_eq!(&out[..n], b"hi");
    }

    #[test]
    fn decode_tolerates_a_missing_trailing_end() {
        let mut out = [0u8; 4];
        let n = decode(b"hi", &mut out).unwrap();
        assert_eq!(&out[..n], b"hi");
    }

    #[test]
    fn an_invalid_escape_is_rejected() {
        let mut out = [0u8; 4];
        assert_eq!(
            decode(&[ESC, 0x01, END], &mut out),
            Err(SerialError::InvalidEscape)
        );
    }

    #[test]
    fn an_escape_at_the_end_is_truncated() {
        let mut out = [0u8; 4];
        assert_eq!(
            decode(&[0x01, ESC], &mut out),
            Err(SerialError::TruncatedFrame)
        );
    }

    #[test]
    fn encode_reports_a_full_buffer() {
        let mut frame = [0u8; 2];
        assert_eq!(encode(&[END], &mut frame), Err(SerialError::BufferTooSmall));
    }

    #[test]
    fn decode_reports_a_full_buffer() {
        let mut out = [0u8; 1];
        assert_eq!(decode(b"hi", &mut out), Err(SerialError::BufferTooSmall));
    }

    #[test]
    fn streaming_decoder_splits_back_to_back_frames() {
        let mut decoder: SlipDecoder<16> = SlipDecoder::new();
        // Two packets in one read: "ok", then a single escaped END byte.
        let stream = [b'o', b'k', END, ESC, ESC_END, END];
        let expected: [&[u8]; 2] = [b"ok", &[END]];
        let mut seen = 0;
        for &byte in &stream {
            if let Some(frame) = decoder.push(byte).unwrap() {
                assert_eq!(frame, expected[seen]);
                seen += 1;
            }
        }
        assert_eq!(seen, 2);
    }

    #[test]
    fn streaming_decoder_ignores_empty_frames() {
        let mut decoder: SlipDecoder<8> = SlipDecoder::new();
        // Repeated ENDs (line-noise flushes) yield no frames.
        assert!(decoder.push(END).unwrap().is_none());
        assert!(decoder.push(END).unwrap().is_none());
        assert!(decoder.push(b'x').unwrap().is_none());
        assert_eq!(decoder.push(END).unwrap(), Some(&b"x"[..]));
    }

    #[test]
    fn streaming_decoder_reports_overflow_then_recovers() {
        let mut decoder: SlipDecoder<2> = SlipDecoder::new();
        assert!(decoder.push(b'a').unwrap().is_none());
        assert!(decoder.push(b'b').unwrap().is_none());
        assert_eq!(decoder.push(b'c'), Err(SerialError::BufferTooSmall));
        // After the overflow the next frame decodes cleanly.
        assert!(decoder.push(b'z').unwrap().is_none());
        assert_eq!(decoder.push(END).unwrap(), Some(&b"z"[..]));
    }

    #[test]
    fn a_payload_carrying_reserved_bytes_is_stuffed_as_rfc_1055_fixes() {
        // RFC 1055 replaces an END byte inside a payload with ESC ESC_END, and an ESC byte
        // with ESC ESC_ESC, then ends the frame with END.
        let payload = [0x01, END, ESC, 0x02];
        let mut frame = [0u8; max_encoded_len(4)];
        let n = encode(&payload, &mut frame).expect("room for the frame");
        assert_eq!(&frame[..n], &[0x01, ESC, ESC_END, ESC, ESC_ESC, 0x02, END]);

        let mut restored = [0u8; 4];
        let m = decode(&frame[..n], &mut restored).expect("a well-formed frame");
        assert_eq!(&restored[..m], &payload);
    }
}
