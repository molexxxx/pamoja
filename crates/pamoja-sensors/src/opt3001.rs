//! Texas Instruments OPT3001 ambient light sensor.
//!
//! The OPT3001 is a single-chip lux meter with an optical filter matched to the
//! photopic response of the human eye. It reports illuminance as a 16-bit word holding
//! a 4-bit exponent and a 12-bit mantissa, across twelve binary-weighted full-scale
//! ranges the part can select on its own; the low- and high-limit registers that drive
//! its interrupt pin share that encoding. This module decodes the word, encodes a lux
//! threshold back into it, and builds and parses the configuration register field by
//! field, following the datasheet's register tables and its table of worked decoding
//! examples.
//!
//! Illuminance is returned in integer milli-lux, which holds every register value
//! exactly since the smallest LSB is 0.01 lux; an `f32` lux convenience sits beside it.

/// The I2C address with the ADDR pin tied to GND.
pub const I2C_ADDRESS_GND: u8 = 0x44;
/// The I2C address with the ADDR pin tied to VDD.
pub const I2C_ADDRESS_VDD: u8 = 0x45;
/// The I2C address with the ADDR pin tied to SDA.
pub const I2C_ADDRESS_SDA: u8 = 0x46;
/// The I2C address with the ADDR pin tied to SCL.
pub const I2C_ADDRESS_SCL: u8 = 0x47;

/// The value the manufacturer ID register returns: `0x5449`, the ASCII bytes "TI".
pub const MANUFACTURER_ID: u16 = 0x5449;
/// The value the device ID register returns for an OPT3001.
pub const DEVICE_ID: u16 = 0x3001;

/// The OPT3001 register addresses. Every register is 16 bits, sent most significant
/// byte first.
pub mod register {
    /// Result register: the exponent and mantissa of the latest conversion.
    pub const RESULT: u8 = 0x00;
    /// Configuration register: range, conversion time, mode, status flags, and the
    /// interrupt reporting settings.
    pub const CONFIGURATION: u8 = 0x01;
    /// Low-limit register, in the result register's encoding.
    pub const LOW_LIMIT: u8 = 0x02;
    /// High-limit register, in the result register's encoding.
    pub const HIGH_LIMIT: u8 = 0x03;
    /// Manufacturer ID register; reads [`super::MANUFACTURER_ID`].
    pub const MANUFACTURER_ID: u8 = 0x7E;
    /// Device ID register; reads [`super::DEVICE_ID`] for an OPT3001.
    pub const DEVICE_ID: u8 = 0x7F;
}

/// The power-on value of the configuration register (0xC810): automatic full-scale
/// range, 800 ms conversion time, shutdown mode, latched window-style comparison, an
/// active-low INT pin, the exponent not masked, and a fault count of one.
pub const CONFIGURATION_RESET: u16 = 0xC810;
/// The power-on value of the low-limit register: exponent 0, mantissa 0, or 0 lux.
pub const LOW_LIMIT_RESET: u16 = 0x0000;
/// The power-on value of the high-limit register: exponent 11, mantissa 0xFFF, the
/// largest encodable threshold of 83865.60 lux.
pub const HIGH_LIMIT_RESET: u16 = 0xBFFF;
/// The low-limit register value that selects end-of-conversion mode, where the INT
/// pin and the flags report every completed conversion: the exponent's two most
/// significant bits set to `11b`.
pub const LOW_LIMIT_END_OF_CONVERSION: u16 = 0xC000;

/// The range number that selects automatic full-scale setting, `1100b`. Range numbers
/// `0` to `11` select one of the fixed full-scale ranges; `13` to `15` are reserved.
pub const RANGE_AUTOMATIC: u8 = 0b1100;
/// The largest range number that selects a fixed full-scale range.
pub const RANGE_MAX: u8 = 11;

/// Returns the LSB size, in milli-lux, of a result or limit register at an exponent.
///
/// The datasheet's `LSB_Size = 0.01 lux * 2^E`, so 10 milli-lux doubled per step.
///
/// # Arguments
///
/// * `exponent` - the 4-bit exponent, a range number from `0` to `11`.
///
/// # Returns
///
/// The LSB size in milli-lux, or `None` if `exponent` is above [`RANGE_MAX`].
pub fn lsb_milli_lux(exponent: u8) -> Option<u32> {
    (exponent <= RANGE_MAX).then(|| 10u32 << exponent)
}

/// Returns the full-scale illuminance, in milli-lux, of a range number.
///
/// Full scale is the 12-bit mantissa's maximum, 4095, at that range's LSB size, so
/// 40.95 lux at range `0` up to 83865.60 lux at range `11`.
///
/// # Arguments
///
/// * `range_number` - the range number, `0` to `11`.
///
/// # Returns
///
/// The full-scale illuminance in milli-lux, or `None` for the automatic and reserved
/// range numbers, which have no single full scale.
pub fn full_scale_milli_lux(range_number: u8) -> Option<u32> {
    lsb_milli_lux(range_number).map(|lsb| lsb * 0xFFF)
}

/// Decodes a result or limit register word to milli-lux.
///
/// The datasheet's `lux = 0.01 * 2^E[3:0] * R[11:0]`, exact in integer milli-lux for
/// every possible word.
///
/// # Arguments
///
/// * `raw` - the 16-bit register word, exponent in bits 15:12 and mantissa in 11:0.
///
/// # Returns
///
/// The illuminance in milli-lux.
pub fn milli_lux(raw: u16) -> u32 {
    let exponent = raw >> 12;
    let mantissa = u32::from(raw & 0x0FFF);
    (10u32 << exponent) * mantissa
}

/// Decodes a result or limit register word to lux.
///
/// # Arguments
///
/// * `raw` - the 16-bit register word.
///
/// # Returns
///
/// The illuminance in lux.
pub fn lux(raw: u16) -> f32 {
    milli_lux(raw) as f32 / 1000.0
}

/// Encodes an illuminance into the result and limit registers' exponent-and-mantissa
/// word, at the smallest exponent that holds the value.
///
/// The inverse of [`milli_lux`]. The smallest exponent gives the finest LSB, so the
/// encoded threshold is the closest one the part can compare against; the mantissa is
/// truncated to that LSB. An illuminance above the largest full scale, 83865.60 lux,
/// saturates to [`HIGH_LIMIT_RESET`], the largest encodable word.
///
/// # Arguments
///
/// * `milli_lux` - the illuminance in milli-lux.
///
/// # Returns
///
/// The 16-bit register word.
pub fn raw_from_milli_lux(milli_lux: u32) -> u16 {
    for exponent in 0..=RANGE_MAX {
        let mantissa = milli_lux / (10u32 << exponent);
        if mantissa <= 0x0FFF {
            return (u16::from(exponent) << 12) | mantissa as u16;
        }
    }
    HIGH_LIMIT_RESET
}

/// Assembles a register word from the two bytes the part sends, most significant
/// byte first.
///
/// # Arguments
///
/// * `bytes` - the two data bytes of a register read.
///
/// # Returns
///
/// The 16-bit register word.
pub fn word_from_bytes(bytes: [u8; 2]) -> u16 {
    u16::from_be_bytes(bytes)
}

/// Splits a register word into the two bytes written to the part, most significant
/// byte first.
///
/// # Arguments
///
/// * `word` - the 16-bit register word.
///
/// # Returns
///
/// The two data bytes of a register write.
pub fn word_to_bytes(word: u16) -> [u8; 2] {
    word.to_be_bytes()
}

/// The conversion time (configuration bit 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionTime {
    /// 100 ms; on ranges `0` to `5` this drops one to three bits of resolution.
    Ms100,
    /// 800 ms, the full specified resolution on every range (the default).
    Ms800,
}

impl ConversionTime {
    /// Returns the conversion time in milliseconds.
    pub fn millis(self) -> u16 {
        match self {
            ConversionTime::Ms100 => 100,
            ConversionTime::Ms800 => 800,
        }
    }
}

/// The mode of conversion operation (configuration bits 10:9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Shutdown; the flags and INT pin keep their last state (the default).
    Shutdown,
    /// One conversion, after which the field reads back as shutdown.
    SingleShot,
    /// Continuous conversions.
    Continuous,
}

impl Mode {
    /// Returns the 2-bit field code for this mode.
    pub fn code(self) -> u8 {
        match self {
            Mode::Shutdown => 0b00,
            Mode::SingleShot => 0b01,
            Mode::Continuous => 0b10,
        }
    }

    /// Builds a mode from a 2-bit field code.
    ///
    /// Codes `10` and `11` both select continuous conversion and map to
    /// [`Mode::Continuous`].
    pub fn from_code(code: u8) -> Mode {
        match code & 0b11 {
            0b00 => Mode::Shutdown,
            0b01 => Mode::SingleShot,
            _ => Mode::Continuous,
        }
    }
}

/// The interrupt reporting style, the latch field (configuration bit 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Latch {
    /// Transparent hysteresis-style comparison: the INT pin and flags follow the
    /// comparison directly, with no clearing event.
    TransparentHysteresis,
    /// Latched window-style comparison: the INT pin and flags hold until the
    /// configuration register is read (the default).
    LatchedWindow,
}

/// The INT pin polarity (configuration bit 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    /// INT pulls low on an interrupt event (the default).
    ActiveLow,
    /// INT goes high impedance on an interrupt event, to be pulled high.
    ActiveHigh,
}

/// The number of consecutive fault events that trigger a report (configuration
/// bits 1:0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultCount {
    /// One fault (the default).
    One,
    /// Two consecutive faults.
    Two,
    /// Four consecutive faults.
    Four,
    /// Eight consecutive faults.
    Eight,
}

impl FaultCount {
    /// Returns the 2-bit field code for this fault count.
    pub fn code(self) -> u8 {
        match self {
            FaultCount::One => 0b00,
            FaultCount::Two => 0b01,
            FaultCount::Four => 0b10,
            FaultCount::Eight => 0b11,
        }
    }

    /// Builds a fault count from a 2-bit field code.
    pub fn from_code(code: u8) -> FaultCount {
        match code & 0b11 {
            0b00 => FaultCount::One,
            0b01 => FaultCount::Two,
            0b10 => FaultCount::Four,
            _ => FaultCount::Eight,
        }
    }

    /// Returns the number of consecutive faults this setting requires.
    pub fn count(self) -> u8 {
        match self {
            FaultCount::One => 1,
            FaultCount::Two => 2,
            FaultCount::Four => 4,
            FaultCount::Eight => 8,
        }
    }
}

/// A decoded OPT3001 configuration register.
///
/// Build one, set the fields, and turn it into the 16-bit register value with
/// [`bits`](Configuration::bits); or parse a register read with
/// [`from_bits`](Configuration::from_bits). [`Configuration::default`] is the power-on
/// state, [`CONFIGURATION_RESET`]. The four status fields are read-only on the part
/// and are ignored by [`bits`](Configuration::bits).
///
/// # Examples
///
/// ```
/// use pamoja_sensors::opt3001::{Configuration, Mode};
///
/// // Convert continuously with the range set automatically, everything else default.
/// let config = Configuration {
///     mode: Mode::Continuous,
///     ..Configuration::default()
/// };
/// assert_eq!(config.bits(), 0xCC10);
/// assert_eq!(Configuration::from_bits(0xCC10), config);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Configuration {
    /// The range number (bits 15:12): `0` to `11` for a fixed full-scale range, or
    /// [`RANGE_AUTOMATIC`] to let the part choose and report its choice in the
    /// result's exponent.
    pub range_number: u8,
    /// The conversion time.
    pub conversion_time: ConversionTime,
    /// The mode of conversion operation.
    pub mode: Mode,
    /// Read-only: the last conversion overflowed its full-scale range.
    pub overflow: bool,
    /// Read-only: a conversion has completed since the register was last read or
    /// written with a non-shutdown mode.
    pub conversion_ready: bool,
    /// Read-only: the result exceeded the high limit for the fault count.
    pub flag_high: bool,
    /// Read-only: the result fell below the low limit for the fault count.
    pub flag_low: bool,
    /// The interrupt reporting style.
    pub latch: Latch,
    /// The INT pin polarity.
    pub polarity: Polarity,
    /// Force the result's exponent to zero on a fixed range, so the mantissa alone
    /// is the reading at that range's LSB.
    pub mask_exponent: bool,
    /// The consecutive faults required to trigger a report.
    pub fault_count: FaultCount,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            range_number: RANGE_AUTOMATIC,
            conversion_time: ConversionTime::Ms800,
            mode: Mode::Shutdown,
            overflow: false,
            conversion_ready: false,
            flag_high: false,
            flag_low: false,
            latch: Latch::LatchedWindow,
            polarity: Polarity::ActiveLow,
            mask_exponent: false,
            fault_count: FaultCount::One,
        }
    }
}

impl Configuration {
    /// Assembles the 16-bit configuration register value.
    ///
    /// The read-only status bits are written as zero, and the range number is
    /// truncated to its four bits.
    ///
    /// # Returns
    ///
    /// The register value to write, most significant byte first.
    pub fn bits(self) -> u16 {
        let mut bits = 0u16;
        bits |= u16::from(self.range_number & 0x0F) << 12;
        bits |= u16::from(matches!(self.conversion_time, ConversionTime::Ms800)) << 11;
        bits |= u16::from(self.mode.code()) << 9;
        bits |= u16::from(matches!(self.latch, Latch::LatchedWindow)) << 4;
        bits |= u16::from(matches!(self.polarity, Polarity::ActiveHigh)) << 3;
        bits |= u16::from(self.mask_exponent) << 2;
        bits |= u16::from(self.fault_count.code());
        bits
    }

    /// Parses a 16-bit configuration register value.
    ///
    /// # Arguments
    ///
    /// * `bits` - the register value, as read from the device.
    ///
    /// # Returns
    ///
    /// The decoded configuration, status flags included.
    pub fn from_bits(bits: u16) -> Configuration {
        Configuration {
            range_number: (bits >> 12) as u8,
            conversion_time: if bits & (1 << 11) != 0 {
                ConversionTime::Ms800
            } else {
                ConversionTime::Ms100
            },
            mode: Mode::from_code((bits >> 9) as u8),
            overflow: bits & (1 << 8) != 0,
            conversion_ready: bits & (1 << 7) != 0,
            flag_high: bits & (1 << 6) != 0,
            flag_low: bits & (1 << 5) != 0,
            latch: if bits & (1 << 4) != 0 {
                Latch::LatchedWindow
            } else {
                Latch::TransparentHysteresis
            },
            polarity: if bits & (1 << 3) != 0 {
                Polarity::ActiveHigh
            } else {
                Polarity::ActiveLow
            },
            mask_exponent: bits & (1 << 2) != 0,
            fault_count: FaultCount::from_code(bits as u8),
        }
    }

    /// Returns whether the range number selects automatic full-scale setting.
    pub fn is_automatic_range(&self) -> bool {
        self.range_number == RANGE_AUTOMATIC
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_follow_the_addr_pin_table() {
        // Table 1: 1000100b with ADDR to GND, then VDD, SDA, SCL.
        assert_eq!(I2C_ADDRESS_GND, 0b100_0100);
        assert_eq!(I2C_ADDRESS_VDD, 0b100_0101);
        assert_eq!(I2C_ADDRESS_SDA, 0b100_0110);
        assert_eq!(I2C_ADDRESS_SCL, 0b100_0111);
    }

    #[test]
    fn register_map_and_ids_match_the_datasheet() {
        // Table 6 register map; Tables 14 and 15 for the ID values.
        assert_eq!(register::RESULT, 0x00);
        assert_eq!(register::CONFIGURATION, 0x01);
        assert_eq!(register::LOW_LIMIT, 0x02);
        assert_eq!(register::HIGH_LIMIT, 0x03);
        assert_eq!(register::MANUFACTURER_ID, 0x7E);
        assert_eq!(register::DEVICE_ID, 0x7F);
        assert_eq!(MANUFACTURER_ID, 0x5449);
        assert_eq!(MANUFACTURER_ID.to_be_bytes(), *b"TI");
        assert_eq!(DEVICE_ID, 0x3001);
    }

    #[test]
    fn result_register_decodes_per_the_datasheet_examples() {
        // Table 9, every row: register word, LSB weight, and resulting lux.
        let rows: [(u16, u32, u32); 10] = [
            (0x0001, 10, 10),
            (0x0FFF, 10, 40_950),
            (0x3456, 80, 88_800),
            (0x789A, 1_280, 2_818_560),
            (0x8800, 2_560, 5_242_880),
            (0x9400, 5_120, 5_242_880),
            (0xA200, 10_240, 5_242_880),
            (0xB100, 20_480, 5_242_880),
            (0xB001, 20_480, 20_480),
            (0xBFFF, 20_480, 83_865_600),
        ];
        for (raw, lsb, expected) in rows {
            assert_eq!(
                lsb_milli_lux((raw >> 12) as u8),
                Some(lsb),
                "lsb {raw:#06x}"
            );
            assert_eq!(milli_lux(raw), expected, "milli-lux {raw:#06x}");
            assert!(
                (lux(raw) - expected as f32 / 1000.0).abs() < 0.001,
                "lux {raw:#06x}"
            );
        }
    }

    #[test]
    fn full_scale_table_matches_each_range_number() {
        // Table 8: full-scale range and LSB size for exponents 0000b to 1011b.
        let rows: [(u8, u32, u32); 12] = [
            (0, 40_950, 10),
            (1, 81_900, 20),
            (2, 163_800, 40),
            (3, 327_600, 80),
            (4, 655_200, 160),
            (5, 1_310_400, 320),
            (6, 2_620_800, 640),
            (7, 5_241_600, 1_280),
            (8, 10_483_200, 2_560),
            (9, 20_966_400, 5_120),
            (10, 41_932_800, 10_240),
            (11, 83_865_600, 20_480),
        ];
        for (range, full_scale, lsb) in rows {
            assert_eq!(
                full_scale_milli_lux(range),
                Some(full_scale),
                "range {range}"
            );
            assert_eq!(lsb_milli_lux(range), Some(lsb), "range {range}");
            assert_eq!(milli_lux((u16::from(range) << 12) | 0x0FFF), full_scale);
        }
        // The automatic and reserved range numbers have no full scale of their own.
        for range in RANGE_AUTOMATIC..=0x0F {
            assert_eq!(full_scale_milli_lux(range), None);
            assert_eq!(lsb_milli_lux(range), None);
        }
        // Electrical Characteristics: 0.01 lux resolution, 83865.6 lux full scale.
        assert_eq!(lsb_milli_lux(0), Some(10));
        assert_eq!(full_scale_milli_lux(RANGE_MAX), Some(83_865_600));
    }

    #[test]
    fn encoder_picks_the_smallest_exponent_that_holds_the_value() {
        // Table 9 lists four words for 5242.88 lux; 08h/800h is the smallest exponent.
        assert_eq!(raw_from_milli_lux(5_242_880), 0x8800);
        // 88.80 lux fits a 0.04 lux LSB, one step finer than Table 9's 03h/456h.
        assert_eq!(raw_from_milli_lux(88_800), 0x28AC);
        assert_eq!(milli_lux(0x28AC), 88_800);
        assert_eq!(raw_from_milli_lux(10), 0x0001);
        assert_eq!(raw_from_milli_lux(40_950), 0x0FFF);
        assert_eq!(raw_from_milli_lux(40_960), 0x1800);
        assert_eq!(raw_from_milli_lux(0), 0x0000);
        // The largest encodable threshold, and saturation above it.
        assert_eq!(raw_from_milli_lux(83_865_600), HIGH_LIMIT_RESET);
        assert_eq!(raw_from_milli_lux(83_865_601), HIGH_LIMIT_RESET);
        assert_eq!(raw_from_milli_lux(u32::MAX), HIGH_LIMIT_RESET);
        // A value between two LSB steps truncates to the step below.
        assert_eq!(raw_from_milli_lux(15), 0x0001);
    }

    #[test]
    fn every_result_word_survives_a_round_trip_through_the_encoder() {
        for raw in 0..=HIGH_LIMIT_RESET {
            let value = milli_lux(raw);
            let encoded = raw_from_milli_lux(value);
            assert_eq!(milli_lux(encoded), value, "word {raw:#06x}");
            assert!(
                encoded >> 12 <= raw >> 12,
                "word {raw:#06x} re-encoded {encoded:#06x}"
            );
        }
    }

    #[test]
    fn integer_decode_tracks_the_floating_point_reference() {
        // Equation 3, transcribed: lux = 0.01 * 2^E[3:0] * R[11:0].
        for raw in 0..=HIGH_LIMIT_RESET {
            let exponent = f64::from(raw >> 12);
            let mantissa = f64::from(raw & 0x0FFF);
            let reference = 0.01 * exponent.exp2() * mantissa;
            let integer = f64::from(milli_lux(raw)) / 1000.0;
            assert!(
                (integer - reference).abs() < 1e-6,
                "word {raw:#06x}: {integer} vs {reference}"
            );
        }
    }

    #[test]
    fn register_words_travel_most_significant_byte_first() {
        // Figures 20 and 21: data MSByte then data LSByte.
        assert_eq!(word_from_bytes([0x34, 0x56]), 0x3456);
        assert_eq!(word_to_bytes(0x3456), [0x34, 0x56]);
        assert_eq!(milli_lux(word_from_bytes([0x78, 0x9A])), 2_818_560);
        assert_eq!(word_from_bytes(word_to_bytes(0xC810)), 0xC810);
    }

    #[test]
    fn default_configuration_is_the_datasheet_reset_value() {
        // Section 7.6.1.1.2: configuration register reset C810h, Table 10 per field.
        assert_eq!(Configuration::default().bits(), CONFIGURATION_RESET);
        assert_eq!(
            Configuration::from_bits(CONFIGURATION_RESET),
            Configuration::default()
        );
        let reset = Configuration::default();
        assert_eq!(reset.range_number, 0b1100);
        assert!(reset.is_automatic_range());
        assert_eq!(reset.conversion_time, ConversionTime::Ms800);
        assert_eq!(reset.mode, Mode::Shutdown);
        assert_eq!(reset.latch, Latch::LatchedWindow);
        assert_eq!(reset.polarity, Polarity::ActiveLow);
        assert!(!reset.mask_exponent);
        assert_eq!(reset.fault_count, FaultCount::One);
    }

    #[test]
    fn limit_registers_reset_per_the_datasheet() {
        // Table 11: LE = 0h, TL = 000h. Table 13: HE = Bh, TH = FFFh.
        assert_eq!(LOW_LIMIT_RESET, 0x0000);
        assert_eq!(milli_lux(LOW_LIMIT_RESET), 0);
        assert_eq!(HIGH_LIMIT_RESET, 0xBFFF);
        assert_eq!(milli_lux(HIGH_LIMIT_RESET), 83_865_600);
        // Section 7.4.2.3: end-of-conversion mode is LE[3:2] = 11b.
        assert_eq!(LOW_LIMIT_END_OF_CONVERSION >> 14, 0b11);
    }

    #[test]
    fn configuration_field_codes_match_the_datasheet() {
        // Table 10: CT 1 = 800 ms; M 01 = single-shot, 10 and 11 = continuous;
        // FC 00, 01, 10, 11 = one, two, four, eight faults.
        assert_eq!(ConversionTime::Ms100.millis(), 100);
        assert_eq!(ConversionTime::Ms800.millis(), 800);
        assert_eq!(Mode::SingleShot.code(), 0b01);
        assert_eq!(Mode::from_code(0b10), Mode::Continuous);
        assert_eq!(Mode::from_code(0b11), Mode::Continuous);
        assert_eq!(FaultCount::Eight.code(), 0b11);
        assert_eq!(FaultCount::from_code(0b10).count(), 4);
        // Continuous conversion on the automatic range is the reset word with M = 10b.
        let continuous = Configuration {
            mode: Mode::Continuous,
            ..Configuration::default()
        };
        assert_eq!(continuous.bits(), 0xCC10);
        // A read with OVF, CRF, and FH set: the flags decode and are not written back.
        let status = Configuration::from_bits(0xCDD0);
        assert!(status.overflow);
        assert!(status.conversion_ready);
        assert!(status.flag_high);
        assert!(!status.flag_low);
        assert_eq!(status.bits(), 0xCC10);
    }

    #[test]
    fn configuration_round_trips_through_bits() {
        let config = Configuration {
            range_number: 6,
            conversion_time: ConversionTime::Ms100,
            mode: Mode::SingleShot,
            latch: Latch::TransparentHysteresis,
            polarity: Polarity::ActiveHigh,
            mask_exponent: true,
            fault_count: FaultCount::Four,
            ..Configuration::default()
        };
        assert_eq!(config.bits(), 0x620E);
        assert_eq!(Configuration::from_bits(config.bits()), config);
        assert!(!config.is_automatic_range());
        assert_eq!(full_scale_milli_lux(config.range_number), Some(2_620_800));
    }
}
