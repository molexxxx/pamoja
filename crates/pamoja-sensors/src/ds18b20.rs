//! Maxim DS18B20 1-Wire digital thermometer.
//!
//! The DS18B20 reports temperature as a 16-bit two's-complement number in a nine-byte
//! scratchpad, with a CRC byte that covers the rest. This module decodes that
//! temperature exactly as the datasheet's temperature/data table specifies, reads the
//! resolution out of the configuration byte, and verifies the scratchpad's CRC with
//! the Maxim 1-Wire polynomial so a read corrupted on the bus is caught rather than
//! trusted.
//!
//! It is pure logic: a caller drives the 1-Wire transactions (convert, then read
//! scratchpad) and hands the nine bytes to [`Scratchpad::parse`].

use crate::SensorError;

/// The 1-Wire family code that identifies a DS18B20 in the first ROM byte.
pub const FAMILY_CODE: u8 = 0x28;

/// The conversion resolution, selected by the R1/R0 bits of the configuration byte.
///
/// Higher resolution resolves smaller steps but takes longer to convert, a tradeoff
/// the datasheet spells out. The temperature register always carries 1/16 °C per
/// count; at lower resolutions the unused low bits simply read as zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// 9-bit, 0.5 °C steps, up to 93.75 ms per conversion.
    Bits9,
    /// 10-bit, 0.25 °C steps, up to 187.5 ms per conversion.
    Bits10,
    /// 11-bit, 0.125 °C steps, up to 375 ms per conversion.
    Bits11,
    /// 12-bit, 0.0625 °C steps, up to 750 ms per conversion. The power-on default.
    Bits12,
}

impl Resolution {
    /// Returns the number of significant bits this resolution produces.
    ///
    /// # Returns
    ///
    /// `9`, `10`, `11`, or `12`.
    pub fn bits(self) -> u8 {
        match self {
            Resolution::Bits9 => 9,
            Resolution::Bits10 => 10,
            Resolution::Bits11 => 11,
            Resolution::Bits12 => 12,
        }
    }

    /// Returns the configuration-register byte that selects this resolution.
    ///
    /// The byte places R1/R0 in bits 6:5 over the datasheet's fixed surrounding
    /// pattern (bit 7 clear, bit 4 set, bits 3:0 set), so 12-bit is `0x7F`, 11-bit
    /// `0x5F`, 10-bit `0x3F`, and 9-bit `0x1F`.
    ///
    /// # Returns
    ///
    /// The configuration byte written to scratchpad byte 4.
    pub fn config_byte(self) -> u8 {
        let r1r0 = match self {
            Resolution::Bits9 => 0b00,
            Resolution::Bits10 => 0b01,
            Resolution::Bits11 => 0b10,
            Resolution::Bits12 => 0b11,
        };
        0b0001_1111 | (r1r0 << 5)
    }

    /// Reads the resolution out of a configuration byte's R1/R0 bits.
    ///
    /// # Arguments
    ///
    /// * `byte` - the configuration register (scratchpad byte 4).
    ///
    /// # Returns
    ///
    /// The resolution selected by bits 6:5.
    pub fn from_config_byte(byte: u8) -> Resolution {
        match (byte >> 5) & 0b11 {
            0b00 => Resolution::Bits9,
            0b01 => Resolution::Bits10,
            0b10 => Resolution::Bits11,
            _ => Resolution::Bits12,
        }
    }

    /// Returns the temperature step this resolution resolves, in micro-degrees Celsius.
    ///
    /// # Returns
    ///
    /// `500000` (0.5 °C) for 9-bit down to `62500` (0.0625 °C) for 12-bit.
    pub fn step_micro_celsius(self) -> u32 {
        match self {
            Resolution::Bits9 => 500_000,
            Resolution::Bits10 => 250_000,
            Resolution::Bits11 => 125_000,
            Resolution::Bits12 => 62_500,
        }
    }

    /// Returns the datasheet's maximum conversion time, in microseconds.
    ///
    /// # Returns
    ///
    /// `93750` for 9-bit, doubling up to `750000` for 12-bit.
    pub fn max_conversion_micros(self) -> u32 {
        match self {
            Resolution::Bits9 => 93_750,
            Resolution::Bits10 => 187_500,
            Resolution::Bits11 => 375_000,
            Resolution::Bits12 => 750_000,
        }
    }
}

/// Converts a raw temperature register value to micro-degrees Celsius, exactly.
///
/// Each count is 1/16 °C, which is 62500 micro-degrees, so the conversion is exact in
/// integer arithmetic and needs no floating point.
///
/// # Arguments
///
/// * `raw` - the 16-bit two's-complement temperature register, as a signed value.
///
/// # Returns
///
/// The temperature in micro-degrees Celsius (millionths of a degree).
pub fn temperature_to_micro_celsius(raw: i16) -> i32 {
    raw as i32 * 62_500
}

/// Converts a raw temperature register value to degrees Celsius.
///
/// # Arguments
///
/// * `raw` - the 16-bit two's-complement temperature register, as a signed value.
///
/// # Returns
///
/// The temperature in degrees Celsius.
pub fn temperature_to_celsius(raw: i16) -> f32 {
    raw as f32 / 16.0
}

/// Converts micro-degrees Celsius to a raw temperature register value.
///
/// The result is truncated to the step `resolution` resolves, since the datasheet
/// specifies that the bits below a selected resolution read as zero.
///
/// # Arguments
///
/// * `micro_celsius` - the temperature in millionths of a degree Celsius.
/// * `resolution` - the resolution the part is configured for.
///
/// # Returns
///
/// The 16-bit two's-complement temperature register, as a signed value.
pub fn temperature_from_micro_celsius(micro_celsius: i32, resolution: Resolution) -> i16 {
    let counts = micro_celsius.div_euclid(62_500) as i16;
    let unused = 12 - resolution.bits();
    (counts >> unused) << unused
}

/// Converts degrees Celsius to a raw temperature register value.
///
/// # Arguments
///
/// * `celsius` - the temperature in degrees Celsius.
/// * `resolution` - the resolution the part is configured for.
///
/// # Returns
///
/// The 16-bit two's-complement temperature register, as a signed value.
pub fn temperature_from_celsius(celsius: f32, resolution: Resolution) -> i16 {
    temperature_from_micro_celsius((celsius * 1_000_000.0) as i32, resolution)
}

/// Computes the Maxim 1-Wire CRC-8 over `data`.
///
/// This is the CRC the DS18B20 (and every Maxim 1-Wire device) appends to its ROM
/// code and scratchpad. The polynomial is X^8 + X^5 + X^4 + 1, processed
/// least-significant-bit first from a zero shift register, which is the reflected
/// form `0x8C`.
///
/// # Arguments
///
/// * `data` - the bytes the CRC covers, in transmission order.
///
/// # Returns
///
/// The 8-bit CRC; for a correctly received message followed by its CRC byte, running
/// this over all of them yields zero.
pub fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &byte in data {
        let mut bits = byte;
        for _ in 0..8 {
            let mix = (crc ^ bits) & 0x01;
            crc >>= 1;
            if mix != 0 {
                crc ^= 0x8C;
            }
            bits >>= 1;
        }
    }
    crc
}

/// A decoded, CRC-verified DS18B20 scratchpad.
///
/// The scratchpad is nine bytes: temperature LSB and MSB, the high and low alarm
/// thresholds, the configuration byte, three reserved bytes, and a CRC. [`parse`]
/// checks the CRC before exposing any of it.
///
/// [`parse`]: Scratchpad::parse
///
/// # Examples
///
/// ```
/// use pamoja_sensors::ds18b20::{temperature_from_celsius, Resolution, Scratchpad};
///
/// // What a part sitting at +25.0625 °C, set to 12-bit steps with its thresholds at
/// // +75/-10 °C, puts on the bus. A driver reads these nine bytes off the wire.
/// let bytes = Scratchpad::new(
///     temperature_from_celsius(25.0625, Resolution::Bits12),
///     Resolution::Bits12,
///     75,
///     -10,
/// )
/// .to_bytes();
///
/// let scratchpad = Scratchpad::parse(&bytes)?;
/// assert_eq!(scratchpad.temperature_micro_celsius(), 25_062_500);
/// assert_eq!(scratchpad.resolution(), Resolution::Bits12);
/// # Ok::<(), pamoja_sensors::SensorError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scratchpad {
    raw_temperature: i16,
    alarm_high: i8,
    alarm_low: i8,
    resolution: Resolution,
}

impl Scratchpad {
    /// Parses and CRC-checks a nine-byte scratchpad.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the nine scratchpad bytes in the order the device sends them, the
    ///   ninth being the CRC.
    ///
    /// # Returns
    ///
    /// The decoded scratchpad.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::Crc`] if the CRC byte does not match the first eight,
    /// which means the read was corrupted and should be repeated.
    pub fn parse(bytes: &[u8; 9]) -> Result<Scratchpad, SensorError> {
        if crc8(&bytes[..8]) != bytes[8] {
            return Err(SensorError::Crc);
        }
        let raw_temperature = i16::from_le_bytes([bytes[0], bytes[1]]);
        Ok(Scratchpad {
            raw_temperature,
            alarm_high: bytes[2] as i8,
            alarm_low: bytes[3] as i8,
            resolution: Resolution::from_config_byte(bytes[4]),
        })
    }

    /// Builds the scratchpad a part in the given state reports.
    ///
    /// This is the inverse of [`parse`](Self::parse), so a node can be developed and
    /// tested against the bytes a thermometer would send without one attached.
    ///
    /// # Arguments
    ///
    /// * `raw_temperature` - the 16-bit two's-complement temperature register, as a
    ///   signed value; [`temperature_from_celsius`] builds one from a temperature.
    /// * `resolution` - the resolution the part is configured for.
    /// * `alarm_high` - the high alarm threshold in whole degrees Celsius (TH).
    /// * `alarm_low` - the low alarm threshold in whole degrees Celsius (TL).
    ///
    /// # Returns
    ///
    /// The scratchpad holding those values.
    pub fn new(
        raw_temperature: i16,
        resolution: Resolution,
        alarm_high: i8,
        alarm_low: i8,
    ) -> Scratchpad {
        Scratchpad {
            raw_temperature,
            alarm_high,
            alarm_low,
            resolution,
        }
    }

    /// Returns the nine bytes a part holding this scratchpad puts on the bus.
    ///
    /// Bytes 5 to 7 are the reserved bytes, which a part reports but which carry no
    /// reading; they are filled with the values its scratchpad figure shows so the
    /// frame is the one a device would actually send. The ninth byte is the CRC over
    /// the other eight, so the result parses.
    ///
    /// # Returns
    ///
    /// The nine scratchpad bytes in transmission order, CRC last.
    pub fn to_bytes(&self) -> [u8; 9] {
        let temperature = self.raw_temperature.to_le_bytes();
        let mut bytes = [
            temperature[0],
            temperature[1],
            self.alarm_high as u8,
            self.alarm_low as u8,
            self.resolution.config_byte(),
            0xFF,
            0x0C,
            0x10,
            0x00,
        ];
        bytes[8] = crc8(&bytes[..8]);
        bytes
    }

    /// Returns the raw temperature register value.
    ///
    /// # Returns
    ///
    /// The 16-bit two's-complement register, as a signed value.
    pub fn raw_temperature(&self) -> i16 {
        self.raw_temperature
    }

    /// Returns the temperature in micro-degrees Celsius.
    ///
    /// # Returns
    ///
    /// The temperature, exact, in millionths of a degree Celsius.
    pub fn temperature_micro_celsius(&self) -> i32 {
        temperature_to_micro_celsius(self.raw_temperature)
    }

    /// Returns the temperature in degrees Celsius.
    ///
    /// # Returns
    ///
    /// The temperature in degrees Celsius.
    pub fn temperature_celsius(&self) -> f32 {
        temperature_to_celsius(self.raw_temperature)
    }

    /// Returns the configured conversion resolution.
    pub fn resolution(&self) -> Resolution {
        self.resolution
    }

    /// Returns the high temperature alarm threshold (TH), in whole degrees Celsius.
    pub fn alarm_high(&self) -> i8 {
        self.alarm_high
    }

    /// Returns the low temperature alarm threshold (TL), in whole degrees Celsius.
    pub fn alarm_low(&self) -> i8 {
        self.alarm_low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature_table_matches_the_datasheet() {
        // The DS18B20 datasheet's temperature/data relationship table, in 1/16 °C.
        let table: &[(i16, i32)] = &[
            (0x07D0, 125_000_000),
            (0x0550, 85_000_000),
            (0x0191, 25_062_500),
            (0x00A2, 10_125_000),
            (0x0008, 500_000),
            (0x0000, 0),
            (i16::from_le_bytes([0xF8, 0xFF]), -500_000),
            (i16::from_le_bytes([0x5E, 0xFF]), -10_125_000),
            (i16::from_le_bytes([0x6F, 0xFE]), -25_062_500),
            (i16::from_le_bytes([0x90, 0xFC]), -55_000_000),
        ];
        for &(raw, micro) in table {
            assert_eq!(temperature_to_micro_celsius(raw), micro, "raw {raw:#06x}");
        }
    }

    #[test]
    fn crc8_matches_the_published_check_value() {
        // CRC-8/MAXIM-DOW check value for the ASCII string "123456789" is 0xA1.
        assert_eq!(crc8(b"123456789"), 0xA1);
        // An empty message leaves the zero-initialised register untouched.
        assert_eq!(crc8(&[]), 0x00);
    }

    #[test]
    fn crc_over_a_message_and_its_crc_is_zero() {
        let data = [0x28, 0xFF, 0x64, 0x1E, 0x0C, 0x00, 0x00, 0x00];
        let crc = crc8(&data);
        let mut with_crc = [0u8; 9];
        with_crc[..8].copy_from_slice(&data);
        with_crc[8] = crc;
        assert_eq!(crc8(&with_crc), 0x00);
    }

    #[test]
    fn a_scratchpad_round_trips_through_parse() {
        let mut bytes = [0x91, 0x01, 75, 0xF6, 0x7F, 0xFF, 0x00, 0x10, 0x00];
        bytes[8] = crc8(&bytes[..8]);
        let scratchpad = Scratchpad::parse(&bytes).expect("valid crc");
        assert_eq!(scratchpad.raw_temperature(), 0x0191);
        assert_eq!(scratchpad.temperature_micro_celsius(), 25_062_500);
        assert_eq!(scratchpad.resolution(), Resolution::Bits12);
        assert_eq!(scratchpad.alarm_high(), 75);
        assert_eq!(scratchpad.alarm_low(), -10);
    }

    #[test]
    fn a_corrupted_scratchpad_fails_the_crc() {
        let mut bytes = [0x91, 0x01, 75, 0xF6, 0x7F, 0xFF, 0x00, 0x10, 0x00];
        bytes[8] = crc8(&bytes[..8]);
        bytes[0] ^= 0x01; // flip a temperature bit after the CRC was computed
        assert_eq!(Scratchpad::parse(&bytes), Err(SensorError::Crc));
    }

    #[test]
    fn resolution_config_bytes_match_the_datasheet() {
        assert_eq!(Resolution::Bits9.config_byte(), 0x1F);
        assert_eq!(Resolution::Bits10.config_byte(), 0x3F);
        assert_eq!(Resolution::Bits11.config_byte(), 0x5F);
        assert_eq!(Resolution::Bits12.config_byte(), 0x7F);
        for resolution in [
            Resolution::Bits9,
            Resolution::Bits10,
            Resolution::Bits11,
            Resolution::Bits12,
        ] {
            assert_eq!(
                Resolution::from_config_byte(resolution.config_byte()),
                resolution
            );
        }
    }
    #[test]
    fn a_built_scratchpad_parses_back_to_what_it_was_built_from() {
        let built = Scratchpad::new(
            temperature_from_celsius(25.0625, Resolution::Bits12),
            Resolution::Bits12,
            75,
            -10,
        );
        let parsed = Scratchpad::parse(&built.to_bytes()).expect("a built scratchpad is valid");
        assert_eq!(parsed, built);
        assert_eq!(parsed.raw_temperature(), 0x0191);
        assert_eq!(parsed.temperature_micro_celsius(), 25_062_500);
        assert_eq!(parsed.alarm_high(), 75);
        assert_eq!(parsed.alarm_low(), -10);
    }

    #[test]
    fn a_built_scratchpad_carries_the_datasheet_temperature_bytes() {
        // The +25.0625 °C row of the datasheet's temperature/data table is 0x0191, sent
        // least-significant byte first, and byte 4 selects 12-bit resolution.
        let bytes = Scratchpad::new(0x0191, Resolution::Bits12, 75, -10).to_bytes();
        assert_eq!(bytes[0], 0x91);
        assert_eq!(bytes[1], 0x01);
        assert_eq!(bytes[4], 0x7F);
        assert_eq!(bytes[8], crc8(&bytes[..8]));
    }

    #[test]
    fn a_temperature_truncates_to_the_step_the_resolution_resolves() {
        // 12-bit resolves a sixteenth of a degree; the coarser settings read the bits
        // below their step as zero, so the same temperature lands on a lower count.
        assert_eq!(temperature_from_celsius(25.0625, Resolution::Bits12), 401);
        assert_eq!(temperature_from_celsius(25.0625, Resolution::Bits11), 400);
        assert_eq!(temperature_from_celsius(25.0625, Resolution::Bits10), 400);
        assert_eq!(temperature_from_celsius(25.0625, Resolution::Bits9), 400);
        assert_eq!(
            temperature_from_micro_celsius(-10_062_500, Resolution::Bits12),
            -161
        );
    }

    #[test]
    fn every_temperature_register_survives_a_round_trip() {
        for raw in [-880i16, -161, -1, 0, 1, 401, 1250] {
            let micro = temperature_to_micro_celsius(raw);
            assert_eq!(
                temperature_from_micro_celsius(micro, Resolution::Bits12),
                raw
            );
        }
    }
}
