//! The CRC-16/MCRF4XX checksum every MAVLink frame carries, and the `CRC_EXTRA`
//! seed derived from a message's shape.
//!
//! MAVLink names its checksum "X.25", but the accumulator it actually uses omits the
//! final inversion that true CRC-16/X-25 applies, which makes it CRC-16/MCRF4XX. The
//! distinction is not cosmetic: the two share a polynomial, initial value, and
//! reflection and differ only in that final XOR, so a frame checked with the wrong one
//! is silently rejected. This module pins the MCRF4XX parameters and is anchored to the
//! catalogue check value, so that trap is closed.

// The reflected form of the CRC-16/CCITT polynomial 0x1021, used because the checksum
// reflects its input and output and so processes each byte least-significant bit first.
const POLY_REFLECTED: u16 = 0x8408;

/// Folds more bytes into a running CRC-16/MCRF4XX.
///
/// # Arguments
///
/// * `crc` - the running value; start a fresh checksum from `0xFFFF`.
/// * `data` - the bytes to fold in.
///
/// # Returns
///
/// The updated CRC.
pub const fn accumulate(mut crc: u16, data: &[u8]) -> u16 {
    let mut i = 0;
    while i < data.len() {
        crc ^= data[i] as u16;
        let mut bit = 0;
        while bit < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ POLY_REFLECTED;
            } else {
                crc >>= 1;
            }
            bit += 1;
        }
        i += 1;
    }
    crc
}

/// Computes the CRC-16/MCRF4XX of a byte slice.
///
/// The parameters are width 16, polynomial `0x1021`, initial value `0xFFFF`, input and
/// output reflected, and no final inversion (`xorout = 0x0000`). True CRC-16/X-25 differs
/// only in that final inversion, so this checks `0x6F91` over `"123456789"` where X-25
/// checks `0x906E`.
///
/// # Arguments
///
/// * `data` - the bytes to check.
///
/// # Returns
///
/// The 16-bit CRC.
///
/// # Examples
///
/// ```
/// use pamoja_mavlink::crc16_mcrf4xx;
///
/// // The catalogue check value for CRC-16/MCRF4XX.
/// assert_eq!(crc16_mcrf4xx(b"123456789"), 0x6F91);
/// ```
pub const fn crc16_mcrf4xx(data: &[u8]) -> u16 {
    accumulate(0xFFFF, data)
}

/// Computes the checksum a frame carries: the CRC over its bytes with the message's
/// `CRC_EXTRA` folded in last.
///
/// # Arguments
///
/// * `frame` - the frame bytes the checksum covers: everything after the start marker up
///   to but not including the two checksum bytes (and not the signature).
/// * `crc_extra` - the `CRC_EXTRA` seed for the frame's message id.
///
/// # Returns
///
/// The 16-bit checksum to append, low byte first.
pub const fn checksum(frame: &[u8], crc_extra: u8) -> u16 {
    accumulate(accumulate(0xFFFF, frame), &[crc_extra])
}

/// Derives a message's `CRC_EXTRA` from its name and base fields.
///
/// MAVLink computes this over the message name, then each base (non-extension) field's
/// type and name in wire order, with an array field's length folded in as one byte, and
/// reduces the 16-bit result to a byte. A receiver folds the seed into every frame's
/// checksum, so a sender and receiver that disagree about a message's shape reject each
/// other's frames instead of silently misreading them. Extension fields are excluded,
/// which is what lets a message gain extension fields without breaking compatibility.
///
/// # Arguments
///
/// * `name` - the message name, such as `"HEARTBEAT"`.
/// * `fields` - the base fields in wire order, each as `(type, name, array_len)`, where
///   `type` is the MAVLink C type such as `"uint16_t"` and `array_len` is `0` for a
///   scalar field.
///
/// # Returns
///
/// The `CRC_EXTRA` byte.
///
/// # Examples
///
/// ```
/// use pamoja_mavlink::message_crc_extra;
///
/// // HEARTBEAT, in wire order: the 4-byte field first, then the five bytes.
/// let crc_extra = message_crc_extra(
///     "HEARTBEAT",
///     &[
///         ("uint32_t", "custom_mode", 0),
///         ("uint8_t", "type", 0),
///         ("uint8_t", "autopilot", 0),
///         ("uint8_t", "base_mode", 0),
///         ("uint8_t", "system_status", 0),
///         ("uint8_t", "mavlink_version", 0),
///     ],
/// );
/// assert_eq!(crc_extra, 50);
/// ```
pub fn message_crc_extra(name: &str, fields: &[(&str, &str, u8)]) -> u8 {
    crc_extra_of(name, fields.iter().copied())
}

// The seed derivation itself, over any sequence of `(type, name, array_len)` fields, so
// the descriptor layer derives the same value without collecting its fields into a slice.
pub(crate) fn crc_extra_of<'f>(
    name: &str,
    fields: impl IntoIterator<Item = (&'f str, &'f str, u8)>,
) -> u8 {
    let mut crc = accumulate(0xFFFF, name.as_bytes());
    crc = accumulate(crc, b" ");
    for (ty, field, array_len) in fields {
        crc = accumulate(crc, ty.as_bytes());
        crc = accumulate(crc, b" ");
        crc = accumulate(crc, field.as_bytes());
        crc = accumulate(crc, b" ");
        if array_len != 0 {
            crc = accumulate(crc, &[array_len]);
        }
    }
    ((crc & 0xFF) ^ (crc >> 8)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_mcrf4xx_catalogue_check_value() {
        assert_eq!(crc16_mcrf4xx(b"123456789"), 0x6F91);
    }

    #[test]
    fn is_not_the_x25_check_value() {
        // True CRC-16/X-25 applies a final XOR and checks 0x906E; using it would make
        // every frame fail its checksum against a real autopilot.
        assert_ne!(crc16_mcrf4xx(b"123456789"), 0x906E);
    }

    #[test]
    fn an_empty_slice_is_the_initial_value() {
        assert_eq!(crc16_mcrf4xx(&[]), 0xFFFF);
    }

    #[test]
    fn heartbeat_crc_extra_matches_the_dialect() {
        // The official common-dialect CRC_EXTRA for HEARTBEAT is 50.
        let crc_extra = message_crc_extra(
            "HEARTBEAT",
            &[
                ("uint32_t", "custom_mode", 0),
                ("uint8_t", "type", 0),
                ("uint8_t", "autopilot", 0),
                ("uint8_t", "base_mode", 0),
                ("uint8_t", "system_status", 0),
                ("uint8_t", "mavlink_version", 0),
            ],
        );
        assert_eq!(crc_extra, 50);
    }
}
