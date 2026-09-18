//! Carrying a signed update inside one data block, for a transport that moves blocks.
//!
//! A fragmented transport such as LoRaWAN's TS004 hands a device one block of bytes and says
//! nothing about what is in it: the specification leaves that to the vendor, and gives the
//! server a four-byte descriptor to name whatever convention the two ends share. This is that
//! convention for a pamoja update.
//!
//! A block is the signed manifest and the image it describes, one after the other, behind a
//! short header that says which is which:
//!
//! ```text
//! "PJU1" | envelope length, two bytes | envelope | image
//! ```
//!
//! Nothing here is trusted. The header only says where the two halves are; every rule that
//! decides whether the image runs is the manifest's, checked by the updater as always, so a
//! block that arrives over a broadcast nobody authenticated is no more dangerous than one
//! fetched over a wire.
//!
//! # Examples
//!
//! A server frames an update, and a device stages what came out of its transport:
//!
//! ```
//! use pamoja_update::block::{frame, split, DESCRIPTOR};
//!
//! let envelope = b"a signed manifest";
//! let image = b"the firmware";
//! let mut block = [0u8; 64];
//! let len = frame(envelope, image, &mut block)?;
//!
//! let (read_envelope, read_image) = split(&block[..len])?;
//! assert_eq!(read_envelope, envelope);
//! assert_eq!(read_image, image);
//! assert_eq!(&DESCRIPTOR, b"PJU1", "what the transport calls this block");
//! # Ok::<(), pamoja_update::Refusal>(())
//! ```

use crate::{Refusal, Result};

/// What a transport calls a block in this shape, for a field that names the convention.
pub const DESCRIPTOR: [u8; 4] = *b"PJU1";

/// The bytes the header takes: the descriptor and the envelope length.
pub const HEADER_LEN: usize = 6;

/// Writes a signed update into one block.
///
/// # Arguments
///
/// * `envelope` - the signed manifest.
/// * `image` - the image it describes.
/// * `out` - where to write the block.
///
/// # Returns
///
/// How many bytes the block takes.
///
/// # Errors
///
/// Returns [`Refusal::Size`] when the envelope is longer than two bytes can say, or when the
/// buffer is too small for the block.
pub fn frame(envelope: &[u8], image: &[u8], out: &mut [u8]) -> Result<usize> {
    let len = HEADER_LEN + envelope.len() + image.len();
    if envelope.len() > u16::MAX as usize || out.len() < len {
        return Err(Refusal::Size);
    }
    out[..4].copy_from_slice(&DESCRIPTOR);
    out[4..6].copy_from_slice(&(envelope.len() as u16).to_le_bytes());
    out[6..6 + envelope.len()].copy_from_slice(envelope);
    out[6 + envelope.len()..len].copy_from_slice(image);
    Ok(len)
}

/// Reads a block back into the signed manifest and the image.
///
/// # Arguments
///
/// * `block` - the block a transport reassembled, with any padding already removed.
///
/// # Returns
///
/// The envelope and the image, borrowed from the block.
///
/// # Errors
///
/// Returns [`Refusal::Malformed`] when the block does not carry this convention's header, and
/// [`Refusal::Size`] when the header says the envelope is longer than the block.
pub fn split(block: &[u8]) -> Result<(&[u8], &[u8])> {
    let header = block.get(..HEADER_LEN).ok_or(Refusal::Malformed)?;
    if header[..4] != DESCRIPTOR {
        return Err(Refusal::Malformed);
    }
    let len = usize::from(u16::from_le_bytes([header[4], header[5]]));
    let envelope = block
        .get(HEADER_LEN..HEADER_LEN + len)
        .ok_or(Refusal::Size)?;
    Ok((envelope, &block[HEADER_LEN + len..]))
}
