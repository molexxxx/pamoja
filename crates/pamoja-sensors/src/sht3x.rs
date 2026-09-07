//! Sensirion SHT3x-DIS humidity and temperature sensor (SHT30, SHT31, SHT35).
//!
//! The SHT3x returns fully calibrated, linearised 16-bit temperature and humidity
//! words over I2C, each followed by a CRC-8, and converts to physical units with two
//! fixed linear formulas. This module holds the command words, verifies the CRC,
//! decodes a six-byte measurement frame, applies the datasheet's conversion formulas
//! in integer arithmetic, and decodes the status register, so a node reads degrees and
//! percent relative humidity without carrying a floating-point library.
//!
//! A caller writes a word from [`command`] most-significant byte first, waits the
//! measurement duration for the chosen repeatability, reads six bytes, and hands them
//! to [`Measurement::parse`]. In periodic mode the same six bytes follow
//! [`command::FETCH_DATA`].
//!
//! Temperatures are returned in millidegrees and humidity in thousandths of a percent,
//! rounded to nearest, which is finer than the part's 0.01 °C and 0.01 %RH resolution.

use crate::SensorError;

/// I2C address A, selected with the ADDR pin at logic low; the default.
pub const I2C_ADDRESS_A: u8 = 0x44;
/// I2C address B, selected with the ADDR pin at logic high.
pub const I2C_ADDRESS_B: u8 = 0x45;

/// The shortest gap between two commands, in microseconds: the part needs 1 ms after
/// a command before it accepts another.
pub const MIN_COMMAND_GAP_MICROS: u32 = 1_000;

/// The 16-bit command words, sent most-significant byte first.
///
/// Each word already carries the part's 3-bit command checksum, so it is written as
/// two bytes with nothing appended.
pub mod command {
    /// Single shot, high repeatability, clock stretching enabled.
    pub const SINGLE_SHOT_HIGH_STRETCH: u16 = 0x2C06;
    /// Single shot, medium repeatability, clock stretching enabled.
    pub const SINGLE_SHOT_MEDIUM_STRETCH: u16 = 0x2C0D;
    /// Single shot, low repeatability, clock stretching enabled.
    pub const SINGLE_SHOT_LOW_STRETCH: u16 = 0x2C10;
    /// Single shot, high repeatability, clock stretching disabled.
    pub const SINGLE_SHOT_HIGH: u16 = 0x2400;
    /// Single shot, medium repeatability, clock stretching disabled.
    pub const SINGLE_SHOT_MEDIUM: u16 = 0x240B;
    /// Single shot, low repeatability, clock stretching disabled.
    pub const SINGLE_SHOT_LOW: u16 = 0x2416;

    /// Periodic, 0.5 measurements per second, high repeatability.
    pub const PERIODIC_0_5_MPS_HIGH: u16 = 0x2032;
    /// Periodic, 0.5 measurements per second, medium repeatability.
    pub const PERIODIC_0_5_MPS_MEDIUM: u16 = 0x2024;
    /// Periodic, 0.5 measurements per second, low repeatability.
    pub const PERIODIC_0_5_MPS_LOW: u16 = 0x202F;
    /// Periodic, 1 measurement per second, high repeatability.
    pub const PERIODIC_1_MPS_HIGH: u16 = 0x2130;
    /// Periodic, 1 measurement per second, medium repeatability.
    pub const PERIODIC_1_MPS_MEDIUM: u16 = 0x2126;
    /// Periodic, 1 measurement per second, low repeatability.
    pub const PERIODIC_1_MPS_LOW: u16 = 0x212D;
    /// Periodic, 2 measurements per second, high repeatability.
    pub const PERIODIC_2_MPS_HIGH: u16 = 0x2236;
    /// Periodic, 2 measurements per second, medium repeatability.
    pub const PERIODIC_2_MPS_MEDIUM: u16 = 0x2220;
    /// Periodic, 2 measurements per second, low repeatability.
    pub const PERIODIC_2_MPS_LOW: u16 = 0x222B;
    /// Periodic, 4 measurements per second, high repeatability.
    pub const PERIODIC_4_MPS_HIGH: u16 = 0x2334;
    /// Periodic, 4 measurements per second, medium repeatability.
    pub const PERIODIC_4_MPS_MEDIUM: u16 = 0x2322;
    /// Periodic, 4 measurements per second, low repeatability.
    pub const PERIODIC_4_MPS_LOW: u16 = 0x2329;
    /// Periodic, 10 measurements per second, high repeatability. Self-heating can
    /// occur at this rate.
    pub const PERIODIC_10_MPS_HIGH: u16 = 0x2737;
    /// Periodic, 10 measurements per second, medium repeatability.
    pub const PERIODIC_10_MPS_MEDIUM: u16 = 0x2721;
    /// Periodic, 10 measurements per second, low repeatability.
    pub const PERIODIC_10_MPS_LOW: u16 = 0x272A;
    /// Periodic acquisition with the accelerated response time (ART) feature, which
    /// samples at 4 Hz.
    pub const PERIODIC_ART: u16 = 0x2B32;

    /// Reads the latest periodic-mode data pair; the read is NACKed if none is ready,
    /// and the data memory is cleared once it is fetched.
    pub const FETCH_DATA: u16 = 0xE000;
    /// Stops periodic acquisition, aborting any measurement in progress, and returns
    /// the part to single-shot mode within 1 ms.
    pub const BREAK: u16 = 0x3093;
    /// Resets the system controller and reloads the calibration data without removing
    /// power; the part is idle again within 1.5 ms.
    pub const SOFT_RESET: u16 = 0x30A2;
    /// The I2C general-call reset: address byte 0x00 followed by 0x06. It resets every
    /// device on the bus that honours the general call, not just this part.
    pub const GENERAL_CALL_RESET: u16 = 0x0006;
    /// Switches the plausibility-check heater on.
    pub const HEATER_ENABLE: u16 = 0x306D;
    /// Switches the heater off, which is its state after any reset.
    pub const HEATER_DISABLE: u16 = 0x3066;
    /// Reads the status register as one CRC-protected word.
    pub const READ_STATUS: u16 = 0xF32D;
    /// Clears the alert-pending, tracking-alert, and reset-detected flags.
    pub const CLEAR_STATUS: u16 = 0x3041;
}

/// The measurement repeatability, which sets noise, duration, and energy per sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repeatability {
    /// 0.21 %RH and 0.15 °C typical noise, 4 ms maximum measurement.
    Low,
    /// 0.15 %RH and 0.08 °C typical noise, 6 ms maximum measurement.
    Medium,
    /// 0.08 %RH and 0.04 °C typical noise, 15 ms maximum measurement.
    High,
}

impl Repeatability {
    /// Returns the longest a measurement takes at this repeatability, in microseconds.
    ///
    /// These are the datasheet's maxima for a supply of 2.4 V to 5.5 V; below 2.4 V
    /// each is 500 µs longer.
    ///
    /// # Returns
    ///
    /// `4000`, `6000`, or `15000`.
    pub fn max_measurement_micros(self) -> u32 {
        match self {
            Repeatability::Low => 4_000,
            Repeatability::Medium => 6_000,
            Repeatability::High => 15_000,
        }
    }

    /// Returns the typical measurement duration at this repeatability, in microseconds.
    ///
    /// # Returns
    ///
    /// `2500`, `4500`, or `12500`.
    pub fn typical_measurement_micros(self) -> u32 {
        match self {
            Repeatability::Low => 2_500,
            Repeatability::Medium => 4_500,
            Repeatability::High => 12_500,
        }
    }
}

/// The periodic-mode acquisition rate, in measurements per second (mps).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rate {
    /// One measurement every two seconds.
    HalfMps,
    /// One measurement per second.
    OneMps,
    /// Two measurements per second.
    TwoMps,
    /// Four measurements per second.
    FourMps,
    /// Ten measurements per second; self-heating can occur at this rate.
    TenMps,
}

impl Rate {
    /// Returns the interval between measurements at this rate, in microseconds.
    ///
    /// # Returns
    ///
    /// `2000000` for half a measurement per second down to `100000` for ten.
    pub fn interval_micros(self) -> u32 {
        match self {
            Rate::HalfMps => 2_000_000,
            Rate::OneMps => 1_000_000,
            Rate::TwoMps => 500_000,
            Rate::FourMps => 250_000,
            Rate::TenMps => 100_000,
        }
    }
}

/// Returns the single-shot measurement command for a repeatability and clock mode.
///
/// With clock stretching the part holds SCL low until the result is ready, so the
/// read that follows the command blocks instead of being NACKed.
///
/// # Arguments
///
/// * `repeatability` - the repeatability to measure at.
/// * `clock_stretching` - whether the part may stretch the clock during the read.
///
/// # Returns
///
/// The 16-bit command word, one of the `SINGLE_SHOT_*` constants in [`command`].
pub fn single_shot(repeatability: Repeatability, clock_stretching: bool) -> u16 {
    match (repeatability, clock_stretching) {
        (Repeatability::High, true) => command::SINGLE_SHOT_HIGH_STRETCH,
        (Repeatability::Medium, true) => command::SINGLE_SHOT_MEDIUM_STRETCH,
        (Repeatability::Low, true) => command::SINGLE_SHOT_LOW_STRETCH,
        (Repeatability::High, false) => command::SINGLE_SHOT_HIGH,
        (Repeatability::Medium, false) => command::SINGLE_SHOT_MEDIUM,
        (Repeatability::Low, false) => command::SINGLE_SHOT_LOW,
    }
}

/// Returns the periodic-mode command for a repeatability and rate.
///
/// # Arguments
///
/// * `repeatability` - the repeatability to measure at.
/// * `rate` - how many measurements per second the part should take.
///
/// # Returns
///
/// The 16-bit command word, one of the `PERIODIC_*_MPS_*` constants in [`command`].
pub fn periodic(repeatability: Repeatability, rate: Rate) -> u16 {
    use Repeatability::{High, Low, Medium};
    match (rate, repeatability) {
        (Rate::HalfMps, High) => command::PERIODIC_0_5_MPS_HIGH,
        (Rate::HalfMps, Medium) => command::PERIODIC_0_5_MPS_MEDIUM,
        (Rate::HalfMps, Low) => command::PERIODIC_0_5_MPS_LOW,
        (Rate::OneMps, High) => command::PERIODIC_1_MPS_HIGH,
        (Rate::OneMps, Medium) => command::PERIODIC_1_MPS_MEDIUM,
        (Rate::OneMps, Low) => command::PERIODIC_1_MPS_LOW,
        (Rate::TwoMps, High) => command::PERIODIC_2_MPS_HIGH,
        (Rate::TwoMps, Medium) => command::PERIODIC_2_MPS_MEDIUM,
        (Rate::TwoMps, Low) => command::PERIODIC_2_MPS_LOW,
        (Rate::FourMps, High) => command::PERIODIC_4_MPS_HIGH,
        (Rate::FourMps, Medium) => command::PERIODIC_4_MPS_MEDIUM,
        (Rate::FourMps, Low) => command::PERIODIC_4_MPS_LOW,
        (Rate::TenMps, High) => command::PERIODIC_10_MPS_HIGH,
        (Rate::TenMps, Medium) => command::PERIODIC_10_MPS_MEDIUM,
        (Rate::TenMps, Low) => command::PERIODIC_10_MPS_LOW,
    }
}

/// Computes the CRC-8 the part appends to every data word.
///
/// Polynomial 0x31 (x^8 + x^5 + x^4 + 1), initial value 0xFF, no input or output
/// reflection, and no final XOR; the datasheet's check value is `CRC(0xBEEF) = 0x92`.
/// The part covers exactly the two data bytes that precede each CRC byte.
///
/// # Arguments
///
/// * `bytes` - the bytes the CRC covers, in transmission order.
///
/// # Returns
///
/// The 8-bit CRC.
pub fn crc(bytes: &[u8]) -> u8 {
    let mut register = 0xFFu8;
    for &byte in bytes {
        register ^= byte;
        for _ in 0..8 {
            register = if register & 0x80 != 0 {
                (register << 1) ^ 0x31
            } else {
                register << 1
            };
        }
    }
    register
}

/// Decodes one CRC-protected 16-bit word as the part sends it.
///
/// # Arguments
///
/// * `bytes` - the most-significant data byte, the least-significant data byte, and
///   the CRC over the two.
///
/// # Returns
///
/// The 16-bit word.
///
/// # Errors
///
/// Returns [`SensorError::Crc`] if the CRC byte does not match the two data bytes, so
/// the read was corrupted and must be repeated.
pub fn word(bytes: &[u8; 3]) -> Result<u16, SensorError> {
    if crc(&bytes[..2]) != bytes[2] {
        return Err(SensorError::Crc);
    }
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

/// Builds the three bytes that carry a 16-bit word with its CRC.
///
/// The inverse of [`word`]. The part only accepts written data that is followed by a
/// correct CRC, so a write of any data word goes through this.
///
/// # Arguments
///
/// * `value` - the 16-bit word.
///
/// # Returns
///
/// The word most-significant byte first, then its CRC.
pub fn word_bytes(value: u16) -> [u8; 3] {
    let [msb, lsb] = value.to_be_bytes();
    [msb, lsb, crc(&[msb, lsb])]
}

/// Converts a raw temperature word to millidegrees Celsius.
///
/// This is the datasheet's `T = -45 + 175 * S_T / (2^16 - 1)` in integer arithmetic,
/// rounded to the nearest millidegree.
///
/// # Arguments
///
/// * `raw` - the 16-bit temperature word.
///
/// # Returns
///
/// The temperature in millidegrees Celsius, from `-45000` to `130000`.
pub fn milli_celsius(raw: u16) -> i32 {
    ((175_000 * raw as u64 + 32_767) / 65_535) as i32 - 45_000
}

/// Converts a raw temperature word to degrees Celsius.
///
/// # Arguments
///
/// * `raw` - the 16-bit temperature word.
///
/// # Returns
///
/// The temperature in degrees Celsius.
pub fn celsius(raw: u16) -> f32 {
    -45.0 + 175.0 * raw as f32 / 65_535.0
}

/// Converts a raw temperature word to millidegrees Fahrenheit.
///
/// This is the datasheet's `T = -49 + 315 * S_T / (2^16 - 1)` in integer arithmetic,
/// rounded to the nearest millidegree.
///
/// # Arguments
///
/// * `raw` - the 16-bit temperature word.
///
/// # Returns
///
/// The temperature in millidegrees Fahrenheit, from `-49000` to `266000`.
pub fn milli_fahrenheit(raw: u16) -> i32 {
    ((315_000 * raw as u64 + 32_767) / 65_535) as i32 - 49_000
}

/// Converts a raw temperature word to degrees Fahrenheit.
///
/// # Arguments
///
/// * `raw` - the 16-bit temperature word.
///
/// # Returns
///
/// The temperature in degrees Fahrenheit.
pub fn fahrenheit(raw: u16) -> f32 {
    -49.0 + 315.0 * raw as f32 / 65_535.0
}

/// Converts a raw humidity word to thousandths of a percent relative humidity.
///
/// This is the datasheet's `RH = 100 * S_RH / (2^16 - 1)` in integer arithmetic,
/// rounded to the nearest thousandth of a percent.
///
/// # Arguments
///
/// * `raw` - the 16-bit humidity word.
///
/// # Returns
///
/// The relative humidity in thousandths of a percent, from `0` to `100000`.
pub fn milli_percent(raw: u16) -> u32 {
    ((100_000 * raw as u64 + 32_767) / 65_535) as u32
}

/// Converts a raw humidity word to percent relative humidity.
///
/// # Arguments
///
/// * `raw` - the 16-bit humidity word.
///
/// # Returns
///
/// The relative humidity in percent.
pub fn relative_humidity(raw: u16) -> f32 {
    100.0 * raw as f32 / 65_535.0
}

/// Builds the raw temperature word a part reports for a temperature.
///
/// The inverse of [`milli_celsius`], rounded to the nearest count and clamped to the
/// part's -45 °C to 130 °C output range, so a node can be tested against what a
/// sensor would send without one attached.
///
/// # Arguments
///
/// * `milli_celsius` - the temperature in millidegrees Celsius.
///
/// # Returns
///
/// The 16-bit temperature word.
pub fn temperature_raw_from_milli_celsius(milli_celsius: i32) -> u16 {
    let offset = (milli_celsius.clamp(-45_000, 130_000) + 45_000) as u64;
    ((offset * 65_535 + 87_500) / 175_000) as u16
}

/// Builds the raw temperature word a part reports for a temperature in Celsius.
///
/// # Arguments
///
/// * `celsius` - the temperature in degrees Celsius.
///
/// # Returns
///
/// The 16-bit temperature word.
pub fn temperature_raw_from_celsius(celsius: f32) -> u16 {
    temperature_raw_from_milli_celsius(round_to_milli(celsius))
}

/// Builds the raw temperature word a part reports for a temperature in Fahrenheit.
///
/// The inverse of [`milli_fahrenheit`], rounded to the nearest count and clamped to
/// the part's -49 °F to 266 °F output range.
///
/// # Arguments
///
/// * `milli_fahrenheit` - the temperature in millidegrees Fahrenheit.
///
/// # Returns
///
/// The 16-bit temperature word.
pub fn temperature_raw_from_milli_fahrenheit(milli_fahrenheit: i32) -> u16 {
    let offset = (milli_fahrenheit.clamp(-49_000, 266_000) + 49_000) as u64;
    ((offset * 65_535 + 157_500) / 315_000) as u16
}

/// Builds the raw humidity word a part reports for a relative humidity.
///
/// The inverse of [`milli_percent`], rounded to the nearest count and clamped to 0 to
/// 100 %RH.
///
/// # Arguments
///
/// * `milli_percent` - the relative humidity in thousandths of a percent.
///
/// # Returns
///
/// The 16-bit humidity word.
pub fn humidity_raw_from_milli_percent(milli_percent: u32) -> u16 {
    let clamped = milli_percent.min(100_000) as u64;
    ((clamped * 65_535 + 50_000) / 100_000) as u16
}

/// Builds the raw humidity word a part reports for a relative humidity in percent.
///
/// # Arguments
///
/// * `percent` - the relative humidity in percent.
///
/// # Returns
///
/// The 16-bit humidity word.
pub fn humidity_raw_from_relative_humidity(percent: f32) -> u16 {
    humidity_raw_from_milli_percent(round_to_milli(percent).max(0) as u32)
}

// Rounds a value in whole units to the nearest thousandth without `f32::round`, which
// is not available without `std`.
fn round_to_milli(value: f32) -> i32 {
    let scaled = value * 1_000.0;
    (if scaled >= 0.0 {
        scaled + 0.5
    } else {
        scaled - 0.5
    }) as i32
}

/// One temperature and humidity data pair, as the part returns it.
///
/// The part sends the temperature word, its CRC, the humidity word, and its CRC, in
/// that order, after a single-shot command or [`command::FETCH_DATA`]. [`parse`]
/// checks both CRCs before exposing either word.
///
/// [`parse`]: Measurement::parse
///
/// # Examples
///
/// ```
/// use pamoja_sensors::sht3x::{
///     humidity_raw_from_milli_percent, temperature_raw_from_milli_celsius, Measurement,
/// };
///
/// // What a part at 25.0 °C and 60.0 %RH puts on the bus after a single-shot command.
/// let bytes = Measurement {
///     temperature_raw: temperature_raw_from_milli_celsius(25_000),
///     humidity_raw: humidity_raw_from_milli_percent(60_000),
/// }
/// .to_bytes();
/// assert_eq!(bytes, [0x66, 0x66, 0x93, 0x99, 0x99, 0xBE]);
///
/// let measurement = Measurement::parse(&bytes)?;
/// assert_eq!(measurement.temperature_milli_celsius(), 25_000);
/// assert_eq!(measurement.humidity_milli_percent(), 60_000);
/// # Ok::<(), pamoja_sensors::SensorError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measurement {
    /// The 16-bit temperature word, `S_T` in the datasheet.
    pub temperature_raw: u16,
    /// The 16-bit humidity word, `S_RH` in the datasheet.
    pub humidity_raw: u16,
}

impl Measurement {
    /// Parses and CRC-checks a six-byte measurement frame.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the six bytes in the order the part sends them: temperature MSB,
    ///   LSB, CRC, then humidity MSB, LSB, CRC.
    ///
    /// # Returns
    ///
    /// The two raw words.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::Crc`] if either CRC byte does not match its two data
    /// bytes, so the read was corrupted and must be repeated.
    pub fn parse(bytes: &[u8; 6]) -> Result<Measurement, SensorError> {
        Ok(Measurement {
            temperature_raw: word(&[bytes[0], bytes[1], bytes[2]])?,
            humidity_raw: word(&[bytes[3], bytes[4], bytes[5]])?,
        })
    }

    /// Returns the six bytes a part holding this pair puts on the bus.
    ///
    /// The inverse of [`parse`](Self::parse), with both CRCs filled in so the result
    /// parses.
    ///
    /// # Returns
    ///
    /// Temperature MSB, LSB, CRC, then humidity MSB, LSB, CRC.
    pub fn to_bytes(&self) -> [u8; 6] {
        let [t0, t1, t2] = word_bytes(self.temperature_raw);
        let [h0, h1, h2] = word_bytes(self.humidity_raw);
        [t0, t1, t2, h0, h1, h2]
    }

    /// Returns the temperature in millidegrees Celsius.
    ///
    /// # Returns
    ///
    /// The temperature, rounded to the nearest millidegree.
    pub fn temperature_milli_celsius(&self) -> i32 {
        milli_celsius(self.temperature_raw)
    }

    /// Returns the temperature in degrees Celsius.
    ///
    /// # Returns
    ///
    /// The temperature in degrees Celsius.
    pub fn temperature_celsius(&self) -> f32 {
        celsius(self.temperature_raw)
    }

    /// Returns the temperature in millidegrees Fahrenheit.
    ///
    /// # Returns
    ///
    /// The temperature, rounded to the nearest millidegree.
    pub fn temperature_milli_fahrenheit(&self) -> i32 {
        milli_fahrenheit(self.temperature_raw)
    }

    /// Returns the temperature in degrees Fahrenheit.
    ///
    /// # Returns
    ///
    /// The temperature in degrees Fahrenheit.
    pub fn temperature_fahrenheit(&self) -> f32 {
        fahrenheit(self.temperature_raw)
    }

    /// Returns the relative humidity in thousandths of a percent.
    ///
    /// # Returns
    ///
    /// The relative humidity, rounded to the nearest thousandth of a percent.
    pub fn humidity_milli_percent(&self) -> u32 {
        milli_percent(self.humidity_raw)
    }

    /// Returns the relative humidity in percent.
    ///
    /// # Returns
    ///
    /// The relative humidity in percent.
    pub fn relative_humidity(&self) -> f32 {
        relative_humidity(self.humidity_raw)
    }
}

/// The status register, read with [`command::READ_STATUS`] as one CRC-protected word.
///
/// It reports the heater, the alert state, whether a reset has happened, and how the
/// last command and write were received. The flag bits (alert pending, the two
/// tracking alerts, and reset detected) are cleared by [`command::CLEAR_STATUS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    bits: u16,
}

impl Status {
    /// Bit 15: at least one alert is pending.
    pub const ALERT_PENDING: u16 = 1 << 15;
    /// Bit 13: the heater is on.
    pub const HEATER_ON: u16 = 1 << 13;
    /// Bit 11: a relative-humidity tracking alert.
    pub const HUMIDITY_TRACKING_ALERT: u16 = 1 << 11;
    /// Bit 10: a temperature tracking alert.
    pub const TEMPERATURE_TRACKING_ALERT: u16 = 1 << 10;
    /// Bit 4: a hard reset, soft reset, or supply failure happened since the register
    /// was last cleared.
    pub const RESET_DETECTED: u16 = 1 << 4;
    /// Bit 1: the last command was not processed, being invalid or failing the
    /// command checksum.
    pub const COMMAND_FAILED: u16 = 1 << 1;
    /// Bit 0: the checksum of the last write transfer failed.
    pub const WRITE_CHECKSUM_FAILED: u16 = 1 << 0;

    /// The register's value after power-up (0x8010): alert pending and reset detected
    /// set, everything else clear. The datasheet leaves bits 9:5 unspecified; they are
    /// zero here.
    pub const DEFAULT: u16 = Self::ALERT_PENDING | Self::RESET_DETECTED;

    /// Wraps a raw status word.
    ///
    /// # Arguments
    ///
    /// * `bits` - the 16-bit register value, for example the [`Self::DEFAULT`] word
    ///   or a combination of the bit constants.
    ///
    /// # Returns
    ///
    /// The status holding those bits.
    pub fn from_bits(bits: u16) -> Status {
        Status { bits }
    }

    /// Parses and CRC-checks the three bytes returned by [`command::READ_STATUS`].
    ///
    /// # Arguments
    ///
    /// * `bytes` - the register most-significant byte first, then its CRC.
    ///
    /// # Returns
    ///
    /// The decoded status.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::Crc`] if the CRC byte does not match the register bytes.
    pub fn parse(bytes: &[u8; 3]) -> Result<Status, SensorError> {
        word(bytes).map(Status::from_bits)
    }

    /// Returns the three bytes a part in this state answers a status read with.
    ///
    /// The inverse of [`parse`](Self::parse).
    ///
    /// # Returns
    ///
    /// The register most-significant byte first, then its CRC.
    pub fn to_bytes(&self) -> [u8; 3] {
        word_bytes(self.bits)
    }

    /// Returns the raw register value.
    pub fn bits(&self) -> u16 {
        self.bits
    }

    /// Returns whether at least one alert is pending (bit 15).
    pub fn alert_pending(&self) -> bool {
        self.bits & Self::ALERT_PENDING != 0
    }

    /// Returns whether the heater is on (bit 13).
    pub fn heater_on(&self) -> bool {
        self.bits & Self::HEATER_ON != 0
    }

    /// Returns whether a relative-humidity tracking alert is raised (bit 11).
    pub fn humidity_tracking_alert(&self) -> bool {
        self.bits & Self::HUMIDITY_TRACKING_ALERT != 0
    }

    /// Returns whether a temperature tracking alert is raised (bit 10).
    pub fn temperature_tracking_alert(&self) -> bool {
        self.bits & Self::TEMPERATURE_TRACKING_ALERT != 0
    }

    /// Returns whether a reset has happened since the register was last cleared
    /// (bit 4).
    pub fn reset_detected(&self) -> bool {
        self.bits & Self::RESET_DETECTED != 0
    }

    /// Returns whether the last command was rejected (bit 1).
    pub fn command_failed(&self) -> bool {
        self.bits & Self::COMMAND_FAILED != 0
    }

    /// Returns whether the last write transfer failed its checksum (bit 0).
    pub fn write_checksum_failed(&self) -> bool {
        self.bits & Self::WRITE_CHECKSUM_FAILED != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_the_datasheet_check_value() {
        // Table 20: CRC(0xBEEF) = 0x92, polynomial 0x31, initialisation 0xFF.
        assert_eq!(crc(&[0xBE, 0xEF]), 0x92);
        // With no input the register is left at its initial value.
        assert_eq!(crc(&[]), 0xFF);
    }

    #[test]
    fn a_word_is_returned_only_when_its_crc_matches() {
        assert_eq!(word(&[0xBE, 0xEF, 0x92]), Ok(0xBEEF));
        assert_eq!(word(&[0xBE, 0xEF, 0x93]), Err(SensorError::Crc));
        assert_eq!(word(&[0xBE, 0xEE, 0x92]), Err(SensorError::Crc));
    }

    #[test]
    fn word_bytes_carry_the_datasheet_crc_and_parse_back() {
        assert_eq!(word_bytes(0xBEEF), [0xBE, 0xEF, 0x92]);
        for value in [0x0000, 0x0001, 0x6666, 0x8010, 0x9999, 0xFFFF] {
            assert_eq!(word(&word_bytes(value)), Ok(value));
        }
    }

    #[test]
    fn the_conversion_endpoints_match_the_datasheet_formulas() {
        // Section 4.13: T = -45 + 175 * S_T / (2^16 - 1), so 0 is -45 °C and 65535
        // is 130 °C; T = -49 + 315 * S_T / (2^16 - 1) in Fahrenheit; RH = 100 * S_RH
        // / (2^16 - 1), so 0 is 0 %RH and 65535 is 100 %RH.
        assert_eq!(milli_celsius(0), -45_000);
        assert_eq!(milli_celsius(65_535), 130_000);
        assert_eq!(milli_fahrenheit(0), -49_000);
        assert_eq!(milli_fahrenheit(65_535), 266_000);
        assert_eq!(milli_percent(0), 0);
        assert_eq!(milli_percent(65_535), 100_000);
        assert_eq!(celsius(0), -45.0);
        assert!((celsius(65_535) - 130.0).abs() < 1e-4);
        assert_eq!(fahrenheit(0), -49.0);
        assert!((fahrenheit(65_535) - 266.0).abs() < 1e-4);
        assert_eq!(relative_humidity(0), 0.0);
        assert!((relative_humidity(65_535) - 100.0).abs() < 1e-4);
    }

    #[test]
    fn exact_fractions_of_full_scale_decode_to_whole_degrees_and_percent() {
        // 0x6666 is exactly 0.4 of 65535, so T = -45 + 70 = 25 °C and 77 °F; 0x9999
        // is exactly 0.6, so RH = 60 %.
        assert_eq!(milli_celsius(0x6666), 25_000);
        assert_eq!(milli_fahrenheit(0x6666), 77_000);
        assert_eq!(milli_percent(0x9999), 60_000);
        assert!((celsius(0x6666) - 25.0).abs() < 1e-4);
        assert!((fahrenheit(0x6666) - 77.0).abs() < 1e-4);
        assert!((relative_humidity(0x9999) - 60.0).abs() < 1e-4);
    }

    #[test]
    fn integer_conversion_tracks_the_floating_point_formula() {
        // Every raw word, against a direct transcription of the section 4.13
        // formulas; rounding to nearest keeps the integer result within half a
        // millidegree or half a thousandth of a percent.
        for raw in 0..=u16::MAX {
            let t = -45.0 + 175.0 * raw as f64 / 65_535.0;
            let f = -49.0 + 315.0 * raw as f64 / 65_535.0;
            let rh = 100.0 * raw as f64 / 65_535.0;
            assert!(
                (milli_celsius(raw) as f64 - t * 1_000.0).abs() <= 0.5,
                "raw {raw:#06x}: {} vs {t}",
                milli_celsius(raw)
            );
            assert!(
                (milli_fahrenheit(raw) as f64 - f * 1_000.0).abs() <= 0.5,
                "raw {raw:#06x}: {} vs {f}",
                milli_fahrenheit(raw)
            );
            assert!(
                (milli_percent(raw) as f64 - rh * 1_000.0).abs() <= 0.5,
                "raw {raw:#06x}: {} vs {rh}",
                milli_percent(raw)
            );
        }
    }

    #[test]
    fn every_raw_word_survives_a_round_trip_through_its_builder() {
        // One count is 2.67 millidegrees Celsius, 4.81 millidegrees Fahrenheit, and
        // 1.53 thousandths of a percent, all wider than the rounding in the decode,
        // so the builders recover every word exactly.
        for raw in 0..=u16::MAX {
            assert_eq!(temperature_raw_from_milli_celsius(milli_celsius(raw)), raw);
            assert_eq!(
                temperature_raw_from_milli_fahrenheit(milli_fahrenheit(raw)),
                raw
            );
            assert_eq!(humidity_raw_from_milli_percent(milli_percent(raw)), raw);
        }
    }

    #[test]
    fn the_builders_clamp_to_the_output_range() {
        assert_eq!(temperature_raw_from_milli_celsius(-45_000), 0);
        assert_eq!(temperature_raw_from_milli_celsius(-100_000), 0);
        assert_eq!(temperature_raw_from_milli_celsius(130_000), 65_535);
        assert_eq!(temperature_raw_from_milli_celsius(200_000), 65_535);
        assert_eq!(temperature_raw_from_milli_fahrenheit(-49_000), 0);
        assert_eq!(temperature_raw_from_milli_fahrenheit(300_000), 65_535);
        assert_eq!(humidity_raw_from_milli_percent(0), 0);
        assert_eq!(humidity_raw_from_milli_percent(100_000), 65_535);
        assert_eq!(humidity_raw_from_milli_percent(150_000), 65_535);
    }

    #[test]
    fn the_floating_point_builders_land_on_the_same_words() {
        assert_eq!(temperature_raw_from_celsius(25.0), 0x6666);
        assert_eq!(temperature_raw_from_celsius(-45.0), 0);
        assert_eq!(temperature_raw_from_celsius(130.0), 65_535);
        assert_eq!(humidity_raw_from_relative_humidity(60.0), 0x9999);
        assert_eq!(humidity_raw_from_relative_humidity(-1.0), 0);
        assert_eq!(humidity_raw_from_relative_humidity(100.0), 65_535);
        for celsius_in in [-40.0f32, -10.5, 0.0, 23.73, 85.0, 125.0] {
            let back = celsius(temperature_raw_from_celsius(celsius_in));
            assert!((back - celsius_in).abs() < 0.002, "{celsius_in} vs {back}");
        }
        for percent_in in [0.0f32, 12.5, 50.0, 63.37, 99.99] {
            let back = relative_humidity(humidity_raw_from_relative_humidity(percent_in));
            assert!((back - percent_in).abs() < 0.001, "{percent_in} vs {back}");
        }
    }

    #[test]
    fn a_measurement_frame_carries_temperature_then_humidity() {
        // Section 4.4: the temperature word and its CRC come first, then humidity.
        let bytes = [0x66, 0x66, 0x93, 0x99, 0x99, 0xBE];
        let measurement = Measurement::parse(&bytes).expect("valid crcs");
        assert_eq!(measurement.temperature_raw, 0x6666);
        assert_eq!(measurement.humidity_raw, 0x9999);
        assert_eq!(measurement.temperature_milli_celsius(), 25_000);
        assert_eq!(measurement.temperature_milli_fahrenheit(), 77_000);
        assert_eq!(measurement.humidity_milli_percent(), 60_000);
        assert!((measurement.temperature_celsius() - 25.0).abs() < 1e-4);
        assert!((measurement.temperature_fahrenheit() - 77.0).abs() < 1e-4);
        assert!((measurement.relative_humidity() - 60.0).abs() < 1e-4);
        assert_eq!(measurement.to_bytes(), bytes);
    }

    #[test]
    fn a_built_measurement_parses_back_to_what_it_was_built_from() {
        let built = Measurement {
            temperature_raw: temperature_raw_from_milli_celsius(23_732),
            humidity_raw: humidity_raw_from_milli_percent(63_368),
        };
        let parsed = Measurement::parse(&built.to_bytes()).expect("a built frame is valid");
        assert_eq!(parsed, built);
        assert_eq!(parsed.temperature_raw, 0x648B);
        assert_eq!(parsed.humidity_raw, 0xA238);
        assert_eq!(built.to_bytes(), [0x64, 0x8B, 0xC7, 0xA2, 0x38, 0xDB]);
        assert_eq!(parsed.temperature_milli_celsius(), 23_732);
        assert_eq!(parsed.humidity_milli_percent(), 63_368);
    }

    #[test]
    fn a_corrupted_measurement_frame_fails_the_crc() {
        let good = [0x66, 0x66, 0x93, 0x99, 0x99, 0xBE];
        let mut temperature_hit = good;
        temperature_hit[1] ^= 0x01;
        assert_eq!(Measurement::parse(&temperature_hit), Err(SensorError::Crc));
        let mut humidity_hit = good;
        humidity_hit[4] ^= 0x80;
        assert_eq!(Measurement::parse(&humidity_hit), Err(SensorError::Crc));
        let mut crc_hit = good;
        crc_hit[5] = 0x00;
        assert_eq!(Measurement::parse(&crc_hit), Err(SensorError::Crc));
    }

    #[test]
    fn single_shot_commands_match_table_9() {
        // Table 9: MSB 0x2C with clock stretching, 0x24 without; the datasheet's own
        // example is 0x2C06 for high repeatability with clock stretching.
        assert_eq!(single_shot(Repeatability::High, true), 0x2C06);
        assert_eq!(single_shot(Repeatability::Medium, true), 0x2C0D);
        assert_eq!(single_shot(Repeatability::Low, true), 0x2C10);
        assert_eq!(single_shot(Repeatability::High, false), 0x2400);
        assert_eq!(single_shot(Repeatability::Medium, false), 0x240B);
        assert_eq!(single_shot(Repeatability::Low, false), 0x2416);
    }

    #[test]
    fn periodic_commands_match_table_10() {
        // Table 10, one MSB per rate; the datasheet's own example is 0x2130 for one
        // high-repeatability measurement per second.
        use Repeatability::{High, Low, Medium};
        assert_eq!(periodic(High, Rate::OneMps), 0x2130);
        let table: &[(Rate, u16, u16, u16)] = &[
            (Rate::HalfMps, 0x2032, 0x2024, 0x202F),
            (Rate::OneMps, 0x2130, 0x2126, 0x212D),
            (Rate::TwoMps, 0x2236, 0x2220, 0x222B),
            (Rate::FourMps, 0x2334, 0x2322, 0x2329),
            (Rate::TenMps, 0x2737, 0x2721, 0x272A),
        ];
        for &(rate, high, medium, low) in table {
            assert_eq!(periodic(High, rate), high, "{rate:?} high");
            assert_eq!(periodic(Medium, rate), medium, "{rate:?} medium");
            assert_eq!(periodic(Low, rate), low, "{rate:?} low");
        }
        assert_eq!(command::PERIODIC_ART, 0x2B32);
    }

    #[test]
    fn control_commands_match_tables_11_to_19() {
        assert_eq!(command::FETCH_DATA, 0xE000);
        assert_eq!(command::BREAK, 0x3093);
        assert_eq!(command::SOFT_RESET, 0x30A2);
        assert_eq!(command::GENERAL_CALL_RESET.to_be_bytes(), [0x00, 0x06]);
        assert_eq!(command::HEATER_ENABLE, 0x306D);
        assert_eq!(command::HEATER_DISABLE, 0x3066);
        assert_eq!(command::READ_STATUS, 0xF32D);
        assert_eq!(command::CLEAR_STATUS, 0x3041);
    }

    #[test]
    fn the_addresses_and_timings_match_the_datasheet() {
        // Table 8, Table 4 (2.4 V to 5.5 V), and the 1 ms command spacing of
        // section 4.
        assert_eq!(I2C_ADDRESS_A, 0x44);
        assert_eq!(I2C_ADDRESS_B, 0x45);
        assert_eq!(Repeatability::Low.max_measurement_micros(), 4_000);
        assert_eq!(Repeatability::Medium.max_measurement_micros(), 6_000);
        assert_eq!(Repeatability::High.max_measurement_micros(), 15_000);
        assert_eq!(Repeatability::Low.typical_measurement_micros(), 2_500);
        assert_eq!(Repeatability::Medium.typical_measurement_micros(), 4_500);
        assert_eq!(Repeatability::High.typical_measurement_micros(), 12_500);
        assert_eq!(Rate::HalfMps.interval_micros(), 2_000_000);
        assert_eq!(Rate::TenMps.interval_micros(), 100_000);
        assert_eq!(MIN_COMMAND_GAP_MICROS, 1_000);
    }

    #[test]
    fn the_status_register_default_matches_table_18() {
        // Table 18: bit 15 (alert pending) and bit 4 (reset detected) default to 1,
        // every other defined bit to 0.
        assert_eq!(Status::DEFAULT, 0x8010);
        let status = Status::from_bits(Status::DEFAULT);
        assert!(status.alert_pending());
        assert!(status.reset_detected());
        assert!(!status.heater_on());
        assert!(!status.humidity_tracking_alert());
        assert!(!status.temperature_tracking_alert());
        assert!(!status.command_failed());
        assert!(!status.write_checksum_failed());
        assert_eq!(status.to_bytes(), [0x80, 0x10, 0xE1]);
        assert_eq!(Status::parse(&[0x80, 0x10, 0xE1]), Ok(status));
    }

    #[test]
    fn each_status_bit_decodes_to_its_flag() {
        let all = Status::from_bits(
            Status::ALERT_PENDING
                | Status::HEATER_ON
                | Status::HUMIDITY_TRACKING_ALERT
                | Status::TEMPERATURE_TRACKING_ALERT
                | Status::RESET_DETECTED
                | Status::COMMAND_FAILED
                | Status::WRITE_CHECKSUM_FAILED,
        );
        assert_eq!(all.bits(), 0xAC13);
        assert!(all.alert_pending());
        assert!(all.heater_on());
        assert!(all.humidity_tracking_alert());
        assert!(all.temperature_tracking_alert());
        assert!(all.reset_detected());
        assert!(all.command_failed());
        assert!(all.write_checksum_failed());
        let heater_only = Status::from_bits(Status::HEATER_ON);
        assert_eq!(heater_only.bits(), 0x2000);
        assert!(heater_only.heater_on());
        assert!(!heater_only.alert_pending());
        assert_eq!(Status::parse(&heater_only.to_bytes()), Ok(heater_only));
    }

    #[test]
    fn a_corrupted_status_word_fails_the_crc() {
        assert_eq!(Status::parse(&[0x80, 0x11, 0xE1]), Err(SensorError::Crc));
        assert_eq!(Status::parse(&[0x80, 0x10, 0x00]), Err(SensorError::Crc));
    }
}
