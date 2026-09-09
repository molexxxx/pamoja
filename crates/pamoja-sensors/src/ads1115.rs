//! Texas Instruments ADS1115 16-bit I2C analog-to-digital converter.
//!
//! The ADS1115 turns an analog signal (a soil-moisture probe, a pH electrode, a
//! divider) into a 16-bit reading, and a single Config register selects the input,
//! the gain, the rate, and the comparator. This module builds and parses that Config
//! register field by field and converts a raw conversion result into a voltage at the
//! selected full-scale range, following the datasheet's register tables and per-gain
//! LSB sizes.

#[cfg(feature = "embedded-hal")]
mod driver;

#[cfg(feature = "embedded-hal")]
pub use driver::{Ads1115, CONVERSION_POLLS};

/// The I2C addresses the ADDR pin selects, by what it is tied to.
pub mod address {
    /// ADDR tied to GND: `1001000b`.
    pub const GND: u8 = 0x48;
    /// ADDR tied to VDD: `1001001b`.
    pub const VDD: u8 = 0x49;
    /// ADDR tied to SDA: `1001010b`.
    pub const SDA: u8 = 0x4A;
    /// ADDR tied to SCL: `1001011b`.
    pub const SCL: u8 = 0x4B;
}

/// The time from a stable supply until the part accepts a transfer, in microseconds.
pub const POWER_UP_MICROS: u32 = 50;

/// The ADS1115 register addresses, selected by the address pointer.
pub mod register {
    /// Conversion register: the last 16-bit result, two's complement.
    pub const CONVERSION: u8 = 0x00;
    /// Config register: input, gain, mode, data rate, and comparator settings.
    pub const CONFIG: u8 = 0x01;
    /// Low threshold register for the comparator.
    pub const LO_THRESH: u8 = 0x02;
    /// High threshold register for the comparator.
    pub const HI_THRESH: u8 = 0x03;
}

/// The power-on value of the Config register (0x8583).
pub const CONFIG_RESET: u16 = 0x8583;

/// The input multiplexer setting (Config bits 14:12).
///
/// The ADS1115 measures either of two differential pairs against `AIN3`, the pair
/// `AIN0`/`AIN1`, or any one input against ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mux {
    /// AINP = AIN0, AINN = AIN1 (the default).
    Ain0Ain1,
    /// AINP = AIN0, AINN = AIN3.
    Ain0Ain3,
    /// AINP = AIN1, AINN = AIN3.
    Ain1Ain3,
    /// AINP = AIN2, AINN = AIN3.
    Ain2Ain3,
    /// AINP = AIN0, AINN = GND (single-ended).
    Ain0Gnd,
    /// AINP = AIN1, AINN = GND (single-ended).
    Ain1Gnd,
    /// AINP = AIN2, AINN = GND (single-ended).
    Ain2Gnd,
    /// AINP = AIN3, AINN = GND (single-ended).
    Ain3Gnd,
}

impl Mux {
    /// Returns the 3-bit field code for this multiplexer setting.
    pub fn code(self) -> u8 {
        match self {
            Mux::Ain0Ain1 => 0b000,
            Mux::Ain0Ain3 => 0b001,
            Mux::Ain1Ain3 => 0b010,
            Mux::Ain2Ain3 => 0b011,
            Mux::Ain0Gnd => 0b100,
            Mux::Ain1Gnd => 0b101,
            Mux::Ain2Gnd => 0b110,
            Mux::Ain3Gnd => 0b111,
        }
    }

    /// Builds a multiplexer setting from a 3-bit field code (the low three bits used).
    pub fn from_code(code: u8) -> Mux {
        match code & 0b111 {
            0b000 => Mux::Ain0Ain1,
            0b001 => Mux::Ain0Ain3,
            0b010 => Mux::Ain1Ain3,
            0b011 => Mux::Ain2Ain3,
            0b100 => Mux::Ain0Gnd,
            0b101 => Mux::Ain1Gnd,
            0b110 => Mux::Ain2Gnd,
            _ => Mux::Ain3Gnd,
        }
    }
}

/// The programmable gain amplifier's full-scale range (Config bits 11:9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pga {
    /// Full-scale range ±6.144 V.
    Fsr6_144,
    /// Full-scale range ±4.096 V.
    Fsr4_096,
    /// Full-scale range ±2.048 V (the default).
    Fsr2_048,
    /// Full-scale range ±1.024 V.
    Fsr1_024,
    /// Full-scale range ±0.512 V.
    Fsr0_512,
    /// Full-scale range ±0.256 V.
    Fsr0_256,
}

impl Pga {
    /// Returns the 3-bit field code for this gain setting.
    pub fn code(self) -> u8 {
        match self {
            Pga::Fsr6_144 => 0b000,
            Pga::Fsr4_096 => 0b001,
            Pga::Fsr2_048 => 0b010,
            Pga::Fsr1_024 => 0b011,
            Pga::Fsr0_512 => 0b100,
            Pga::Fsr0_256 => 0b101,
        }
    }

    /// Builds a gain setting from a 3-bit field code.
    ///
    /// Codes `110` and `111` are documented as also selecting ±0.256 V and map here
    /// to [`Pga::Fsr0_256`].
    pub fn from_code(code: u8) -> Pga {
        match code & 0b111 {
            0b000 => Pga::Fsr6_144,
            0b001 => Pga::Fsr4_096,
            0b010 => Pga::Fsr2_048,
            0b011 => Pga::Fsr1_024,
            0b100 => Pga::Fsr0_512,
            _ => Pga::Fsr0_256,
        }
    }

    /// Returns the positive full-scale input voltage for this gain, in microvolts.
    pub fn full_scale_microvolts(self) -> u32 {
        match self {
            Pga::Fsr6_144 => 6_144_000,
            Pga::Fsr4_096 => 4_096_000,
            Pga::Fsr2_048 => 2_048_000,
            Pga::Fsr1_024 => 1_024_000,
            Pga::Fsr0_512 => 512_000,
            Pga::Fsr0_256 => 256_000,
        }
    }
}

/// The conversion mode (Config bit 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Convert continuously.
    Continuous,
    /// Convert once per request, then power down (the default).
    SingleShot,
}

/// The output data rate (Config bits 7:5), in samples per second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataRate {
    /// 8 samples per second.
    Sps8,
    /// 16 samples per second.
    Sps16,
    /// 32 samples per second.
    Sps32,
    /// 64 samples per second.
    Sps64,
    /// 128 samples per second (the default).
    Sps128,
    /// 250 samples per second.
    Sps250,
    /// 475 samples per second.
    Sps475,
    /// 860 samples per second.
    Sps860,
}

impl DataRate {
    /// Returns the 3-bit field code for this data rate.
    pub fn code(self) -> u8 {
        match self {
            DataRate::Sps8 => 0b000,
            DataRate::Sps16 => 0b001,
            DataRate::Sps32 => 0b010,
            DataRate::Sps64 => 0b011,
            DataRate::Sps128 => 0b100,
            DataRate::Sps250 => 0b101,
            DataRate::Sps475 => 0b110,
            DataRate::Sps860 => 0b111,
        }
    }

    /// Builds a data rate from a 3-bit field code.
    pub fn from_code(code: u8) -> DataRate {
        match code & 0b111 {
            0b000 => DataRate::Sps8,
            0b001 => DataRate::Sps16,
            0b010 => DataRate::Sps32,
            0b011 => DataRate::Sps64,
            0b100 => DataRate::Sps128,
            0b101 => DataRate::Sps250,
            0b110 => DataRate::Sps475,
            _ => DataRate::Sps860,
        }
    }

    /// Returns the rate in samples per second.
    pub fn samples_per_second(self) -> u16 {
        match self {
            DataRate::Sps8 => 8,
            DataRate::Sps16 => 16,
            DataRate::Sps32 => 32,
            DataRate::Sps64 => 64,
            DataRate::Sps128 => 128,
            DataRate::Sps250 => 250,
            DataRate::Sps475 => 475,
            DataRate::Sps860 => 860,
        }
    }
}

/// The comparator mode (Config bit 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparatorMode {
    /// Traditional comparator with hysteresis (the default).
    Traditional,
    /// Window comparator.
    Window,
}

/// The ALERT/RDY pin polarity (Config bit 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparatorPolarity {
    /// Active low (the default).
    ActiveLow,
    /// Active high.
    ActiveHigh,
}

/// Whether the comparator latches (Config bit 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparatorLatch {
    /// Non-latching: the pin clears once readings return within the thresholds (the
    /// default).
    NonLatching,
    /// Latching: the pin stays asserted until the conversion data is read.
    Latching,
}

/// The comparator queue, or disable (Config bits 1:0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparatorQueue {
    /// Assert after one conversion beyond a threshold.
    AfterOne,
    /// Assert after two conversions beyond a threshold.
    AfterTwo,
    /// Assert after four conversions beyond a threshold.
    AfterFour,
    /// Disable the comparator and set ALERT/RDY high-impedance (the default).
    Disabled,
}

impl ComparatorQueue {
    /// Returns the 2-bit field code for this queue setting.
    pub fn code(self) -> u8 {
        match self {
            ComparatorQueue::AfterOne => 0b00,
            ComparatorQueue::AfterTwo => 0b01,
            ComparatorQueue::AfterFour => 0b10,
            ComparatorQueue::Disabled => 0b11,
        }
    }

    /// Builds a queue setting from a 2-bit field code.
    pub fn from_code(code: u8) -> ComparatorQueue {
        match code & 0b11 {
            0b00 => ComparatorQueue::AfterOne,
            0b01 => ComparatorQueue::AfterTwo,
            0b10 => ComparatorQueue::AfterFour,
            _ => ComparatorQueue::Disabled,
        }
    }
}

/// A decoded ADS1115 Config register.
///
/// Build one, set the fields, and turn it into the 16-bit register value with
/// [`bits`](Config::bits); or parse a register read with [`from_bits`](Config::from_bits).
/// [`Config::default`] is the power-on state, `0x8583`.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::ads1115::{Config, Mux, Pga};
///
/// // Read AIN0 against ground at the ±4.096 V range, leaving everything else default.
/// let config = Config {
///     mux: Mux::Ain0Gnd,
///     pga: Pga::Fsr4_096,
///     ..Config::default()
/// };
/// // The high byte then low byte are written to the Config register.
/// let [hi, lo] = config.bits().to_be_bytes();
/// assert_eq!(Config::from_bits(u16::from_be_bytes([hi, lo])), config);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Start a single conversion when written; reads as "not converting" when set.
    pub start_conversion: bool,
    /// The input multiplexer setting.
    pub mux: Mux,
    /// The full-scale range.
    pub pga: Pga,
    /// The conversion mode.
    pub mode: Mode,
    /// The output data rate.
    pub data_rate: DataRate,
    /// The comparator mode.
    pub comparator_mode: ComparatorMode,
    /// The ALERT/RDY pin polarity.
    pub comparator_polarity: ComparatorPolarity,
    /// Whether the comparator latches.
    pub comparator_latch: ComparatorLatch,
    /// The comparator queue, or disable.
    pub comparator_queue: ComparatorQueue,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            start_conversion: true,
            mux: Mux::Ain0Ain1,
            pga: Pga::Fsr2_048,
            mode: Mode::SingleShot,
            data_rate: DataRate::Sps128,
            comparator_mode: ComparatorMode::Traditional,
            comparator_polarity: ComparatorPolarity::ActiveLow,
            comparator_latch: ComparatorLatch::NonLatching,
            comparator_queue: ComparatorQueue::Disabled,
        }
    }
}

impl Config {
    /// Assembles the 16-bit Config register value.
    ///
    /// # Returns
    ///
    /// The register value to write, most significant bit first.
    pub fn bits(self) -> u16 {
        let mut bits = 0u16;
        bits |= u16::from(self.start_conversion) << 15;
        bits |= u16::from(self.mux.code()) << 12;
        bits |= u16::from(self.pga.code()) << 9;
        bits |= u16::from(matches!(self.mode, Mode::SingleShot)) << 8;
        bits |= u16::from(self.data_rate.code()) << 5;
        bits |= u16::from(matches!(self.comparator_mode, ComparatorMode::Window)) << 4;
        bits |= u16::from(matches!(
            self.comparator_polarity,
            ComparatorPolarity::ActiveHigh
        )) << 3;
        bits |= u16::from(matches!(self.comparator_latch, ComparatorLatch::Latching)) << 2;
        bits |= u16::from(self.comparator_queue.code());
        bits
    }

    /// Parses a 16-bit Config register value.
    ///
    /// # Arguments
    ///
    /// * `bits` - the register value, as read from the device.
    ///
    /// # Returns
    ///
    /// The decoded configuration.
    pub fn from_bits(bits: u16) -> Config {
        Config {
            start_conversion: bits & (1 << 15) != 0,
            mux: Mux::from_code((bits >> 12) as u8),
            pga: Pga::from_code((bits >> 9) as u8),
            mode: if bits & (1 << 8) != 0 {
                Mode::SingleShot
            } else {
                Mode::Continuous
            },
            data_rate: DataRate::from_code((bits >> 5) as u8),
            comparator_mode: if bits & (1 << 4) != 0 {
                ComparatorMode::Window
            } else {
                ComparatorMode::Traditional
            },
            comparator_polarity: if bits & (1 << 3) != 0 {
                ComparatorPolarity::ActiveHigh
            } else {
                ComparatorPolarity::ActiveLow
            },
            comparator_latch: if bits & (1 << 2) != 0 {
                ComparatorLatch::Latching
            } else {
                ComparatorLatch::NonLatching
            },
            comparator_queue: ComparatorQueue::from_code(bits as u8),
        }
    }
}

/// Converts a raw conversion result to nanovolts at the selected full-scale range.
///
/// The result is a 16-bit two's-complement code spanning plus or minus the full
/// scale, so the voltage is `code * full_scale / 32768`. Working in nanovolts keeps
/// the conversion exact in integer arithmetic across every gain setting.
///
/// # Arguments
///
/// * `pga` - the gain the conversion was taken at.
/// * `raw` - the signed conversion register value.
///
/// # Returns
///
/// The measured voltage in nanovolts.
pub fn to_nanovolts(pga: Pga, raw: i16) -> i64 {
    raw as i64 * (pga.full_scale_microvolts() as i64 * 1000) / 32768
}

/// Converts a raw conversion result to volts at the selected full-scale range.
///
/// # Arguments
///
/// * `pga` - the gain the conversion was taken at.
/// * `raw` - the signed conversion register value.
///
/// # Returns
///
/// The measured voltage in volts.
pub fn to_volts(pga: Pga, raw: i16) -> f32 {
    to_nanovolts(pga, raw) as f32 / 1_000_000_000.0
}

/// The longest one conversion takes at a data rate, in microseconds.
///
/// A conversion takes one period of the data rate, and the datasheet allows the
/// rate to vary by ten percent, so the period is stretched by that much. A driver
/// waits this long after starting a single-shot conversion before it polls the
/// operational-status bit.
///
/// # Arguments
///
/// * `rate` - the data rate the conversion runs at.
///
/// # Returns
///
/// The period in microseconds, with the ten percent margin.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::ads1115::{conversion_micros, DataRate};
///
/// assert_eq!(conversion_micros(DataRate::Sps128), 8_595);
/// assert_eq!(conversion_micros(DataRate::Sps860), 1_280);
/// ```
pub fn conversion_micros(rate: DataRate) -> u32 {
    let period = 1_000_000u32.div_ceil(u32::from(rate.samples_per_second()));
    period + period.div_ceil(10)
}

/// One conversion result with the gain it was taken at, so it converts to volts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    /// The signed 16-bit conversion register value.
    pub raw: i16,
    /// The full-scale range the conversion ran at.
    pub pga: Pga,
}

impl Sample {
    /// Returns the measured voltage in volts.
    pub fn volts(&self) -> f32 {
        to_volts(self.pga, self.raw)
    }

    /// Returns the measured voltage in nanovolts, exact in integer arithmetic.
    pub fn nanovolts(&self) -> i64 {
        to_nanovolts(self.pga, self.raw)
    }

    /// Returns the conversion register bytes for this sample, most significant first.
    ///
    /// This is what the part sends when the register is read, so a test can script it.
    pub fn to_bytes(&self) -> [u8; 2] {
        self.raw.to_be_bytes()
    }
}

#[cfg(test)]
mod driver_support_tests {
    use super::*;

    #[test]
    fn conversion_time_is_one_period_plus_the_ten_percent_variation() {
        assert_eq!(conversion_micros(DataRate::Sps8), 137_500);
        assert_eq!(conversion_micros(DataRate::Sps128), 8_595);
        assert_eq!(conversion_micros(DataRate::Sps860), 1_280);
    }

    #[test]
    fn a_sample_converts_at_its_gain_and_scripts_its_bytes() {
        let sample = Sample {
            raw: 3200,
            pga: Pga::Fsr2_048,
        };
        assert!((sample.volts() - 0.2).abs() < 1e-6);
        assert_eq!(sample.nanovolts(), 200_000_000);
        assert_eq!(sample.to_bytes(), [0x0C, 0x80]);
    }

    #[test]
    fn the_addresses_follow_the_addr_pin_table() {
        assert_eq!(address::GND, 0b100_1000);
        assert_eq!(address::VDD, 0b100_1001);
        assert_eq!(address::SDA, 0b100_1010);
        assert_eq!(address::SCL, 0b100_1011);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_the_datasheet_reset_value() {
        assert_eq!(Config::default().bits(), CONFIG_RESET);
        assert_eq!(Config::from_bits(CONFIG_RESET), Config::default());
    }

    #[test]
    fn config_round_trips_through_bits() {
        let config = Config {
            start_conversion: false,
            mux: Mux::Ain2Gnd,
            pga: Pga::Fsr0_256,
            mode: Mode::Continuous,
            data_rate: DataRate::Sps860,
            comparator_mode: ComparatorMode::Window,
            comparator_polarity: ComparatorPolarity::ActiveHigh,
            comparator_latch: ComparatorLatch::Latching,
            comparator_queue: ComparatorQueue::AfterFour,
        };
        assert_eq!(Config::from_bits(config.bits()), config);
    }

    #[test]
    fn pga_codes_match_the_datasheet() {
        assert_eq!(Pga::Fsr6_144.code(), 0b000);
        assert_eq!(Pga::Fsr2_048.code(), 0b010);
        assert_eq!(Pga::Fsr0_256.code(), 0b101);
        // The reserved 110 and 111 codes also select ±0.256 V.
        assert_eq!(Pga::from_code(0b110), Pga::Fsr0_256);
        assert_eq!(Pga::from_code(0b111), Pga::Fsr0_256);
    }

    #[test]
    fn full_scale_conversion_matches_the_per_gain_lsb() {
        // One count at ±4.096 V is 125 µV; at ±6.144 V it is 187.5 µV.
        assert_eq!(to_nanovolts(Pga::Fsr4_096, 1), 125_000);
        assert_eq!(to_nanovolts(Pga::Fsr6_144, 1), 187_500);
        // Two counts at ±0.256 V is 15.625 µV, exact in nanovolts.
        assert_eq!(to_nanovolts(Pga::Fsr0_256, 2), 15_625);
        // Positive full-scale code at ±4.096 V is just under the 4.096 V range.
        assert_eq!(to_nanovolts(Pga::Fsr4_096, 0x7FFF), 4_095_875_000);
        // Negative codes scale symmetrically.
        assert_eq!(to_nanovolts(Pga::Fsr2_048, -16384), -1_024_000_000);
    }
}
