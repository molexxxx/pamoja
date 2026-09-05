//! I2C device addressing per the NXP I2C-bus specification (UM10204).
//!
//! An I2C transfer begins with the controller sending the device's address. The exact
//! bytes are pinned down by the specification, and they are easy to get subtly wrong: the
//! 7-bit address shares its byte with the read/write bit, so the value a datasheet prints
//! is not the byte that goes on the wire, and the 10-bit extension spends a reserved
//! prefix and spreads its bits across two bytes. This module builds those bytes exactly
//! and rejects an address that is out of range or, on request, one the specification
//! reserves.

use crate::GpioError;

/// Largest valid 7-bit address (inclusive).
const MAX_SEVEN_BIT: u16 = 0x7F;
/// Largest valid 10-bit address (inclusive).
const MAX_TEN_BIT: u16 = 0x3FF;
/// The five-bit `11110` prefix (in the top of the first byte) that marks a 10-bit address.
const TEN_BIT_PREFIX: u8 = 0xF0;

/// Whether an I2C transfer reads from or writes to the device.
///
/// The direction rides in the least-significant bit of the address byte: `0` for a write,
/// `1` for a read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// The controller writes to the device. R/W bit `0`.
    Write,
    /// The controller reads from the device. R/W bit `1`.
    Read,
}

impl Direction {
    /// Returns the R/W bit this direction places in the low bit of the address byte.
    ///
    /// # Returns
    ///
    /// `0` for [`Write`](Direction::Write), `1` for [`Read`](Direction::Read).
    pub fn rw_bit(self) -> u8 {
        match self {
            Direction::Write => 0,
            Direction::Read => 1,
        }
    }
}

/// An I2C device address, 7-bit or 10-bit, validated to its range.
///
/// I2C addresses come in two widths. The original 7-bit address shares its byte with the
/// R/W bit, so it lands on the wire as `(address << 1) | r/w`. The later 10-bit extension
/// stays backward compatible by spending the reserved `11110xx` prefix: the first byte is
/// `11110`, then the top two address bits, then the R/W bit, and the second byte is the
/// low eight address bits. Construct an address with [`seven_bit`](Address::seven_bit) or
/// [`ten_bit`](Address::ten_bit), which reject out-of-range values, then turn it into the
/// bytes a controller sends with [`write_frame`](Address::write_frame).
///
/// # Examples
///
/// ```
/// use pamoja_gpio::i2c::{Address, Direction};
///
/// // 7-bit: a BME280 at 0x76 writes as 0xEC and reads as 0xED.
/// let bme = Address::seven_bit(0x76)?;
/// let mut buf = [0u8; 2];
/// assert_eq!(bme.write_frame(Direction::Write, &mut buf)?, 1);
/// assert_eq!(buf[0], 0xEC);
/// assert_eq!(bme.write_frame(Direction::Read, &mut buf)?, 1);
/// assert_eq!(buf[0], 0xED);
/// # Ok::<(), pamoja_gpio::GpioError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Address {
    value: u16,
    ten_bit: bool,
}

impl Address {
    /// Creates a 7-bit I2C address.
    ///
    /// The whole range is accepted, including the addresses the specification reserves;
    /// those are still legal on the wire (the general call address `0x00` is a broadcast,
    /// for instance). Use [`is_reserved`](Address::is_reserved) to test for them.
    ///
    /// # Arguments
    ///
    /// * `address` - the 7-bit device address, `0x00..=0x7F`.
    ///
    /// # Returns
    ///
    /// The validated address.
    ///
    /// # Errors
    ///
    /// [`GpioError::AddressOutOfRange`] if `address` exceeds `0x7F`.
    pub fn seven_bit(address: u8) -> Result<Address, GpioError> {
        if address as u16 > MAX_SEVEN_BIT {
            return Err(GpioError::AddressOutOfRange);
        }
        Ok(Address {
            value: address as u16,
            ten_bit: false,
        })
    }

    /// Creates a 10-bit I2C address.
    ///
    /// # Arguments
    ///
    /// * `address` - the 10-bit device address, `0x000..=0x3FF`.
    ///
    /// # Returns
    ///
    /// The validated address.
    ///
    /// # Errors
    ///
    /// [`GpioError::AddressOutOfRange`] if `address` exceeds `0x3FF`.
    pub fn ten_bit(address: u16) -> Result<Address, GpioError> {
        if address > MAX_TEN_BIT {
            return Err(GpioError::AddressOutOfRange);
        }
        Ok(Address {
            value: address,
            ten_bit: true,
        })
    }

    /// Returns the address value, without the R/W bit.
    ///
    /// # Returns
    ///
    /// The 7- or 10-bit address as passed to the constructor.
    pub fn value(self) -> u16 {
        self.value
    }

    /// Returns `true` if this is a 10-bit address.
    pub fn is_ten_bit(self) -> bool {
        self.ten_bit
    }

    /// Returns the number of bytes [`write_frame`](Address::write_frame) emits.
    ///
    /// # Returns
    ///
    /// `1` for a 7-bit address, `2` for a 10-bit address.
    pub fn frame_len(self) -> usize {
        if self.ten_bit {
            2
        } else {
            1
        }
    }

    /// Returns `true` if a 7-bit address falls in a range the I2C specification reserves.
    ///
    /// UM10204 reserves `0x00..=0x07` (general call and START byte, CBUS, a bus-format
    /// code, a future code, and the Hs-mode master codes) and `0x78..=0x7F` (the 10-bit
    /// addressing prefix and the device-ID codes), leaving `0x08..=0x77` for ordinary
    /// devices. A 10-bit address is not reserved in this sense, so this returns `false`
    /// for one.
    ///
    /// # Returns
    ///
    /// `true` if this is a 7-bit address in `0x00..=0x07` or `0x78..=0x7F`.
    pub fn is_reserved(self) -> bool {
        !self.ten_bit && (self.value <= 0x07 || self.value >= 0x78)
    }

    /// Returns `true` if this is the general call address `0x00`, the broadcast every
    /// device on the bus listens to.
    pub fn is_general_call(self) -> bool {
        !self.ten_bit && self.value == 0x00
    }

    /// Returns the addressing frame this address puts on the bus.
    ///
    /// The same bytes [`write_frame`](Address::write_frame) produces, as a value, so a
    /// caller does not have to size and pass a scratch buffer to find out what an address
    /// looks like on the wire.
    ///
    /// # Arguments
    ///
    /// * `direction` - whether the transfer reads or writes, which sets the R/W bit.
    ///
    /// # Returns
    ///
    /// The frame, one byte for a 7-bit address and two for a 10-bit one.
    pub fn frame(self, direction: Direction) -> AddressFrame {
        let mut bytes = [0u8; 2];
        let len = self.write_frame(direction, &mut bytes).unwrap_or_default();
        AddressFrame { bytes, len }
    }

    /// Writes the address byte(s) a controller puts on the bus for a transfer.
    ///
    /// For a 7-bit address this is the single byte `(address << 1) | r/w`. For a 10-bit
    /// address it is two bytes: `11110` then the top two address bits then the R/W bit,
    /// followed by the low eight address bits. A 10-bit read in practice first addresses
    /// the device with a write frame and then, after a repeated START, re-sends this first
    /// byte with the read bit set; this method emits the bytes for the `direction` asked
    /// for, leaving the START/repeated-START sequencing to the driver.
    ///
    /// # Arguments
    ///
    /// * `direction` - whether the transfer reads or writes, which sets the R/W bit.
    /// * `out` - the buffer the frame is written into; it must hold at least
    ///   [`frame_len`](Address::frame_len) bytes.
    ///
    /// # Returns
    ///
    /// The number of bytes written: `1` for a 7-bit address, `2` for a 10-bit address.
    ///
    /// # Errors
    ///
    /// [`GpioError::BufferTooSmall`] if `out` is shorter than [`frame_len`](Address::frame_len).
    pub fn write_frame(self, direction: Direction, out: &mut [u8]) -> Result<usize, GpioError> {
        let rw = direction.rw_bit();
        if self.ten_bit {
            if out.len() < 2 {
                return Err(GpioError::BufferTooSmall);
            }
            let high = ((self.value >> 8) as u8) & 0x03;
            out[0] = TEN_BIT_PREFIX | (high << 1) | rw;
            out[1] = (self.value & 0xFF) as u8;
            Ok(2)
        } else {
            if out.is_empty() {
                return Err(GpioError::BufferTooSmall);
            }
            out[0] = ((self.value as u8) << 1) | rw;
            Ok(1)
        }
    }
}

/// The bytes an [`Address`] puts on the bus to address a device.
///
/// One byte for a 7-bit address and two for a 10-bit one, held inline so producing one
/// allocates nothing and needs no caller-supplied buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AddressFrame {
    bytes: [u8; 2],
    len: usize,
}

impl AddressFrame {
    /// Returns the frame bytes, in the order they go on the bus.
    ///
    /// # Returns
    ///
    /// One byte for a 7-bit address, two for a 10-bit one.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns how many bytes the frame occupies.
    ///
    /// # Returns
    ///
    /// `1` for a 7-bit address, `2` for a 10-bit one.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the frame is empty, which it never is.
    ///
    /// # Returns
    ///
    /// Always `false`; an address always addresses something.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the frame and asserts it equals the expected bytes on the wire.
    fn assert_frame(address: Address, direction: Direction, expected: &[u8]) {
        let mut buf = [0u8; 2];
        let n = address.write_frame(direction, &mut buf).unwrap();
        assert_eq!(&buf[..n], expected);
    }

    #[test]
    fn seven_bit_frames_match_known_device_bytes() {
        // Reference bytes printed on real datasheets, where the 7-bit address shifts up
        // one and the R/W bit fills the low bit.
        // DS3231 RTC / MPU-6050 at 0x68.
        assert_frame(Address::seven_bit(0x68).unwrap(), Direction::Write, &[0xD0]);
        assert_frame(Address::seven_bit(0x68).unwrap(), Direction::Read, &[0xD1]);
        // SSD1306 OLED at 0x3C.
        assert_frame(Address::seven_bit(0x3C).unwrap(), Direction::Write, &[0x78]);
        assert_frame(Address::seven_bit(0x3C).unwrap(), Direction::Read, &[0x79]);
        // AT24C32 EEPROM / PCF8574 at 0x50.
        assert_frame(Address::seven_bit(0x50).unwrap(), Direction::Write, &[0xA0]);
        assert_frame(Address::seven_bit(0x50).unwrap(), Direction::Read, &[0xA1]);
    }

    #[test]
    fn ten_bit_frame_matches_spec_worked_example() {
        // UM10204's worked 10-bit example, address 0x2A5 = 0b10_1010_0101:
        // first byte 11110 10 0 = 0xF4, second byte 1010_0101 = 0xA5.
        let addr = Address::ten_bit(0x2A5).unwrap();
        assert_frame(addr, Direction::Write, &[0xF4, 0xA5]);
        assert_frame(addr, Direction::Read, &[0xF5, 0xA5]);
    }

    #[test]
    fn ten_bit_frame_at_the_range_bounds() {
        // 0x000: prefix only, low byte zero.
        assert_frame(
            Address::ten_bit(0x000).unwrap(),
            Direction::Write,
            &[0xF0, 0x00],
        );
        // 0x3FF: top two bits set (11110 11 r/w), low byte all ones.
        assert_frame(
            Address::ten_bit(0x3FF).unwrap(),
            Direction::Write,
            &[0xF6, 0xFF],
        );
        assert_frame(
            Address::ten_bit(0x3FF).unwrap(),
            Direction::Read,
            &[0xF7, 0xFF],
        );
    }

    #[test]
    fn out_of_range_addresses_are_rejected() {
        assert_eq!(Address::seven_bit(0x80), Err(GpioError::AddressOutOfRange));
        assert_eq!(Address::ten_bit(0x400), Err(GpioError::AddressOutOfRange));
        // The top of each range is accepted.
        assert!(Address::seven_bit(0x7F).is_ok());
        assert!(Address::ten_bit(0x3FF).is_ok());
    }

    #[test]
    fn reserved_ranges_match_the_spec() {
        // Reserved: 0x00..=0x07 and 0x78..=0x7F.
        for addr in (0x00..=0x07).chain(0x78..=0x7F) {
            assert!(
                Address::seven_bit(addr).unwrap().is_reserved(),
                "{addr:#04x}"
            );
        }
        // The usable range is everything between.
        for addr in 0x08..=0x77 {
            assert!(
                !Address::seven_bit(addr).unwrap().is_reserved(),
                "{addr:#04x}"
            );
        }
        // A 10-bit address is never reserved in the 7-bit sense.
        assert!(!Address::ten_bit(0x002).unwrap().is_reserved());
    }

    #[test]
    fn general_call_is_address_zero() {
        assert!(Address::seven_bit(0x00).unwrap().is_general_call());
        assert!(!Address::seven_bit(0x01).unwrap().is_general_call());
        assert!(!Address::ten_bit(0x000).unwrap().is_general_call());
    }

    #[test]
    fn frame_len_and_too_small_buffers() {
        assert_eq!(Address::seven_bit(0x40).unwrap().frame_len(), 1);
        assert_eq!(Address::ten_bit(0x100).unwrap().frame_len(), 2);

        let mut empty = [];
        assert_eq!(
            Address::seven_bit(0x40)
                .unwrap()
                .write_frame(Direction::Write, &mut empty),
            Err(GpioError::BufferTooSmall)
        );
        let mut one = [0u8; 1];
        assert_eq!(
            Address::ten_bit(0x100)
                .unwrap()
                .write_frame(Direction::Write, &mut one),
            Err(GpioError::BufferTooSmall)
        );
    }
}
