//! Identifiers and keys written in hexadecimal, the way a network server shows them.

/// Reads bytes written in hexadecimal, most significant first, as a network server shows a
/// DevEUI, a JoinEUI or a root key.
///
/// It runs at compile time, so a program can take its identifiers from the environment it is
/// built in, with `env!`, rather than keep a key in its source.
///
/// # Arguments
///
/// * `text` - two hexadecimal digits for each byte, in either case, with nothing between.
///
/// # Returns
///
/// The bytes, or `None` when the text is not exactly two digits a byte or holds anything but
/// hexadecimal digits.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::parse_hex;
///
/// // In a firmware build: `parse_hex(env!("LORAWAN_DEV_EUI"))`.
/// const DEV_EUI: [u8; 8] = match parse_hex("70B3D57ED0051234") {
///     Some(eui) => eui,
///     None => panic!("a DevEUI is eight bytes of hexadecimal"),
/// };
/// assert_eq!(DEV_EUI, [0x70, 0xB3, 0xD5, 0x7E, 0xD0, 0x05, 0x12, 0x34]);
///
/// assert_eq!(parse_hex::<8>("70b3d57ed0051234"), Some(DEV_EUI));
/// assert_eq!(parse_hex::<8>("70B3D57ED00512"), None, "a byte short");
/// assert_eq!(parse_hex::<2>("0G12"), None, "not a hexadecimal digit");
/// assert_eq!(parse_hex::<2>("0x12"), None, "no prefix");
/// ```
pub const fn parse_hex<const N: usize>(text: &str) -> Option<[u8; N]> {
    let digits = text.as_bytes();
    if digits.len() != N.saturating_mul(2) {
        return None;
    }
    let mut bytes = [0u8; N];
    let mut index = 0;
    while index < N {
        let (Some(high), Some(low)) = (nibble(digits[2 * index]), nibble(digits[2 * index + 1]))
        else {
            return None;
        };
        bytes[index] = (high << 4) | low;
        index += 1;
    }
    Some(bytes)
}

/// The value of one hexadecimal digit.
const fn nibble(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_digit_reads_as_its_value() {
        let all: [u8; 8] = parse_hex("0123456789abcdef").expect("hex");
        assert_eq!(all, [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF]);
        let upper: [u8; 3] = parse_hex("ABCDEF").expect("hex");
        assert_eq!(upper, [0xAB, 0xCD, 0xEF]);
        assert_eq!(parse_hex::<0>(""), Some([]));
    }

    #[test]
    fn anything_but_two_digits_a_byte_is_refused() {
        assert_eq!(parse_hex::<16>("3333333333333333333333333333333"), None);
        assert_eq!(parse_hex::<16>("333333333333333333333333333333333"), None);
        assert_eq!(parse_hex::<4>("33 33 33 33"), None);
        assert_eq!(parse_hex::<2>("12-4"), None);
        assert_eq!(parse_hex::<1>("é"), None);
    }
}
