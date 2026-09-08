//! Sensirion SCD40 and SCD41 CO2, temperature, and humidity sensor.
//!
//! The SCD4x is a photoacoustic NDIR CO2 sensor with a built-in humidity and
//! temperature sensor, driven over I2C by 16-bit command words. Every data word it
//! sends or receives is followed by a CRC-8, and its measurement is three such words:
//! CO2 in parts per million, then a temperature and a humidity ratio scaled over the
//! 16-bit range. This module carries the command table, the checksum, the frame
//! builders and parsers, and the datasheet's conversion formulas, each anchored to
//! the worked examples the datasheet prints beside its command descriptions.
//!
//! A caller sends [`command::START_PERIODIC_MEASUREMENT`] once, polls
//! [`data_ready`] on the [`command::GET_DATA_READY_STATUS`] word, and reads the nine
//! bytes of [`command::READ_MEASUREMENT`] into [`Measurement::parse`]. Temperatures
//! and humidities are returned in integer milli-units, with `f32` conveniences
//! beside them.

use crate::SensorError;

/// The I2C address (Table 8); the part has no address pins.
pub const I2C_ADDRESS: u8 = 0x62;

/// The largest CO2 concentration the sensor reports, in ppm (Table 1: output range
/// 0 to 40'000 ppm).
pub const CO2_MAX_PPM: u16 = 40_000;

/// The factory temperature offset, in milli-degrees Celsius (section 3.6.1: "per
/// default, the temperature offset is set to 4 °C").
pub const DEFAULT_TEMPERATURE_OFFSET_MILLI_CELSIUS: u32 = 4_000;

/// The signal update interval of periodic measurement, in milliseconds (section 3.5).
pub const PERIODIC_MEASUREMENT_INTERVAL_MS: u32 = 5_000;

/// The approximate signal update interval of low power periodic measurement, in
/// milliseconds (section 3.8).
pub const LOW_POWER_PERIODIC_MEASUREMENT_INTERVAL_MS: u32 = 30_000;

/// The time the sensor needs after power-up or `reinit` before it accepts a command,
/// in milliseconds (Table 7).
pub const POWER_UP_TIME_MS: u32 = 1_000;

/// The `perform_forced_recalibration` response that reports a failed recalibration
/// (Table 18).
pub const FORCED_RECALIBRATION_FAILED: u16 = 0xffff;

/// The command words (Table 9), 16 bits each, most significant byte first, with no
/// CRC after the command itself.
pub mod command {
    /// Start periodic measurement, one reading every 5 seconds.
    pub const START_PERIODIC_MEASUREMENT: u16 = 0x21b1;
    /// Read the CO2, temperature, and humidity words of the latest measurement.
    pub const READ_MEASUREMENT: u16 = 0xec05;
    /// Stop periodic measurement and return to idle.
    pub const STOP_PERIODIC_MEASUREMENT: u16 = 0x3f86;
    /// Write the temperature offset word; see [`super::temperature_offset_word`].
    pub const SET_TEMPERATURE_OFFSET: u16 = 0x241d;
    /// Read the temperature offset word; see [`super::temperature_offset_milli_celsius`].
    pub const GET_TEMPERATURE_OFFSET: u16 = 0x2318;
    /// Write the sensor altitude, in meters above sea level.
    pub const SET_SENSOR_ALTITUDE: u16 = 0x2427;
    /// Read the sensor altitude, in meters above sea level.
    pub const GET_SENSOR_ALTITUDE: u16 = 0x2322;
    /// Write the ambient pressure word; see [`super::ambient_pressure_word`].
    pub const SET_AMBIENT_PRESSURE: u16 = 0xe000;
    /// Recalibrate against a reference CO2 concentration and fetch the correction.
    pub const PERFORM_FORCED_RECALIBRATION: u16 = 0x362f;
    /// Enable (1) or disable (0) automatic self-calibration.
    pub const SET_AUTOMATIC_SELF_CALIBRATION_ENABLED: u16 = 0x2416;
    /// Read whether automatic self-calibration is enabled.
    pub const GET_AUTOMATIC_SELF_CALIBRATION_ENABLED: u16 = 0x2313;
    /// Start low power periodic measurement, one reading about every 30 seconds.
    pub const START_LOW_POWER_PERIODIC_MEASUREMENT: u16 = 0x21ac;
    /// Read the data ready word; see [`super::data_ready`].
    pub const GET_DATA_READY_STATUS: u16 = 0xe4b8;
    /// Store the current configuration in EEPROM.
    pub const PERSIST_SETTINGS: u16 = 0x3615;
    /// Read the 48-bit serial number; see [`super::serial_number`].
    pub const GET_SERIAL_NUMBER: u16 = 0x3682;
    /// Run the end-of-line self-test; see [`super::self_test_passed`].
    pub const PERFORM_SELF_TEST: u16 = 0x3639;
    /// Reset the EEPROM configuration and erase the calibration history.
    pub const PERFORM_FACTORY_RESET: u16 = 0x3632;
    /// Reload user settings from EEPROM.
    pub const REINIT: u16 = 0x3646;
    /// Take one CO2, humidity, and temperature measurement (SCD41 only).
    pub const MEASURE_SINGLE_SHOT: u16 = 0x219d;
    /// Take one humidity and temperature measurement, CO2 reads 0 (SCD41 only).
    pub const MEASURE_SINGLE_SHOT_RHT_ONLY: u16 = 0x2196;
    /// Put the sensor from idle into sleep (SCD41 only).
    pub const POWER_DOWN: u16 = 0x36e0;
    /// Wake the sensor from sleep into idle; not acknowledged (SCD41 only).
    pub const WAKE_UP: u16 = 0x36f6;
}

/// Returns the maximum command duration of a command word, in milliseconds.
///
/// This is the execution time column of Table 9, the time a caller waits after
/// sending the command before issuing the read header or the next command.
///
/// # Arguments
///
/// * `command` - one of the [`command`] words.
///
/// # Returns
///
/// The maximum duration in milliseconds, or `None` for the two start commands,
/// whose duration the datasheet lists as not applicable, and for any word that is
/// not a command.
pub fn max_duration_ms(command: u16) -> Option<u16> {
    let ms = match command {
        command::READ_MEASUREMENT => 1,
        command::STOP_PERIODIC_MEASUREMENT => 500,
        command::SET_TEMPERATURE_OFFSET => 1,
        command::GET_TEMPERATURE_OFFSET => 1,
        command::SET_SENSOR_ALTITUDE => 1,
        command::GET_SENSOR_ALTITUDE => 1,
        command::SET_AMBIENT_PRESSURE => 1,
        command::PERFORM_FORCED_RECALIBRATION => 400,
        command::SET_AUTOMATIC_SELF_CALIBRATION_ENABLED => 1,
        command::GET_AUTOMATIC_SELF_CALIBRATION_ENABLED => 1,
        command::GET_DATA_READY_STATUS => 1,
        command::PERSIST_SETTINGS => 800,
        command::GET_SERIAL_NUMBER => 1,
        command::PERFORM_SELF_TEST => 10_000,
        command::PERFORM_FACTORY_RESET => 1_200,
        command::REINIT => 20,
        command::MEASURE_SINGLE_SHOT => 5_000,
        command::MEASURE_SINGLE_SHOT_RHT_ONLY => 50,
        command::POWER_DOWN => 1,
        command::WAKE_UP => 20,
        _ => return None,
    };
    Some(ms)
}

/// Returns whether a command may be sent while a periodic measurement is running.
///
/// This is the "during measurement" column of Table 9: only `read_measurement`,
/// `stop_periodic_measurement`, `set_ambient_pressure`, and `get_data_ready_status`
/// are allowed; every other command needs the sensor in idle mode.
///
/// # Arguments
///
/// * `command` - one of the [`command`] words.
///
/// # Returns
///
/// `true` if the command is accepted during a periodic measurement.
pub fn allowed_during_measurement(command: u16) -> bool {
    matches!(
        command,
        command::READ_MEASUREMENT
            | command::STOP_PERIODIC_MEASUREMENT
            | command::SET_AMBIENT_PRESSURE
            | command::GET_DATA_READY_STATUS
    )
}

/// Computes the CRC-8 that follows every data word.
///
/// The parameters are those of Table 32: polynomial 0x31, initial value 0xFF, no
/// input or output reflection, no final XOR. The datasheet's own check value is
/// `crc(&[0xBE, 0xEF]) == 0x92`.
///
/// # Arguments
///
/// * `bytes` - the bytes the checksum covers, normally one 16-bit word.
///
/// # Returns
///
/// The 8-bit checksum.
pub fn crc(bytes: &[u8]) -> u8 {
    let mut crc = 0xFF;
    for &byte in bytes {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x31
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Decodes one data word from its three-byte frame, checking the CRC.
///
/// # Arguments
///
/// * `frame` - the word's two bytes, most significant first, and its CRC.
///
/// # Returns
///
/// The 16-bit word.
///
/// # Errors
///
/// Returns [`SensorError::Crc`] if the CRC byte does not match the two data bytes.
pub fn word(frame: &[u8; 3]) -> Result<u16, SensorError> {
    if crc(&frame[..2]) != frame[2] {
        return Err(SensorError::Crc);
    }
    Ok(u16::from_be_bytes([frame[0], frame[1]]))
}

/// Builds the three-byte frame of one data word: the word, most significant byte
/// first, followed by its CRC.
///
/// The inverse of [`word`], and the payload every write command carries.
///
/// # Arguments
///
/// * `value` - the 16-bit word.
///
/// # Returns
///
/// The word's two bytes and its CRC.
pub fn word_frame(value: u16) -> [u8; 3] {
    let [high, low] = value.to_be_bytes();
    [high, low, crc(&[high, low])]
}

/// Builds the two bytes of a command that carries no data word.
///
/// # Arguments
///
/// * `command` - one of the [`command`] words.
///
/// # Returns
///
/// The command word, most significant byte first; command words carry no CRC.
pub fn command_frame(command: u16) -> [u8; 2] {
    command.to_be_bytes()
}

/// Builds the five bytes of a write command: the command word followed by one
/// data word and its CRC.
///
/// # Arguments
///
/// * `command` - one of the [`command`] write words.
/// * `value` - the data word the command carries.
///
/// # Returns
///
/// The command's two bytes, the word's two bytes, and the word's CRC.
pub fn write_frame(command: u16, value: u16) -> [u8; 5] {
    let [ch, cl] = command.to_be_bytes();
    let [vh, vl, vc] = word_frame(value);
    [ch, cl, vh, vl, vc]
}

/// Decodes the raw temperature word to milli-degrees Celsius.
///
/// This is Table 11's `T = -45 + 175 * word / (2^16 - 1)`, in thousandths, rounded
/// to the nearest.
///
/// # Arguments
///
/// * `raw` - the 16-bit temperature word of a measurement.
///
/// # Returns
///
/// The temperature in milli-degrees Celsius, from -45'000 to 130'000.
pub fn milli_celsius(raw: u16) -> i32 {
    let scaled = (raw as i64 * 175_000 + 65_535 / 2) / 65_535;
    scaled as i32 - 45_000
}

/// Decodes the raw temperature word to degrees Celsius.
///
/// # Arguments
///
/// * `raw` - the 16-bit temperature word of a measurement.
///
/// # Returns
///
/// The temperature in degrees Celsius.
pub fn celsius(raw: u16) -> f32 {
    -45.0 + 175.0 * raw as f32 / 65_535.0
}

/// Builds the raw temperature word the sensor reports for a temperature.
///
/// The inverse of [`milli_celsius`], so a node can be tested against what a sensor
/// sends without one attached.
///
/// # Arguments
///
/// * `milli_celsius` - the temperature in milli-degrees Celsius.
///
/// # Returns
///
/// The 16-bit temperature word, clamped to the sensor's -45 to 130 °C word range.
pub fn temperature_raw(milli_celsius: i32) -> u16 {
    let offset = (milli_celsius as i64 + 45_000).clamp(0, 175_000);
    ((offset * 65_535 + 175_000 / 2) / 175_000) as u16
}

/// Decodes the raw humidity word to thousandths of a percent relative humidity.
///
/// This is Table 11's `RH = 100 * word / (2^16 - 1)`, in thousandths, rounded to
/// the nearest.
///
/// # Arguments
///
/// * `raw` - the 16-bit humidity word of a measurement.
///
/// # Returns
///
/// The relative humidity in milli-percent, from 0 to 100'000.
pub fn humidity_milli_percent(raw: u16) -> u32 {
    ((raw as u64 * 100_000 + 65_535 / 2) / 65_535) as u32
}

/// Decodes the raw humidity word to percent relative humidity.
///
/// # Arguments
///
/// * `raw` - the 16-bit humidity word of a measurement.
///
/// # Returns
///
/// The relative humidity in percent.
pub fn relative_humidity_percent(raw: u16) -> f32 {
    100.0 * raw as f32 / 65_535.0
}

/// Builds the raw humidity word the sensor reports for a relative humidity.
///
/// The inverse of [`humidity_milli_percent`].
///
/// # Arguments
///
/// * `milli_percent` - the relative humidity in milli-percent.
///
/// # Returns
///
/// The 16-bit humidity word, clamped to 100 %.
pub fn humidity_raw(milli_percent: u32) -> u16 {
    let bounded = milli_percent.min(100_000) as u64;
    ((bounded * 65_535 + 100_000 / 2) / 100_000) as u16
}

/// Returns whether the data ready word reports a measurement waiting to be read.
///
/// Table 22: if the least significant 11 bits of the word are zero, data is not
/// ready; otherwise a measurement is ready for read-out.
///
/// # Arguments
///
/// * `word` - the word read after [`command::GET_DATA_READY_STATUS`].
///
/// # Returns
///
/// `true` if a measurement can be read.
pub fn data_ready(word: u16) -> bool {
    word & 0x07ff != 0
}

/// Encodes a temperature offset as the `set_temperature_offset` word.
///
/// Table 13: `word = T_offset [°C] * 2^16 / 175`, rounded to the nearest. The word
/// scale is 2^16, unlike the measurement's 2^16 - 1.
///
/// # Arguments
///
/// * `milli_celsius` - the offset in milli-degrees Celsius; the offset is never
///   negative.
///
/// # Returns
///
/// The 16-bit offset word.
pub fn temperature_offset_word(milli_celsius: u32) -> u16 {
    ((milli_celsius as u64 * 65_536 + 175_000 / 2) / 175_000).min(0xffff) as u16
}

/// Decodes the `get_temperature_offset` word to milli-degrees Celsius.
///
/// Table 14: `T_offset [°C] = 175 * word / 2^16`, rounded to the nearest
/// thousandth.
///
/// # Arguments
///
/// * `word` - the word read after [`command::GET_TEMPERATURE_OFFSET`].
///
/// # Returns
///
/// The offset in milli-degrees Celsius.
pub fn temperature_offset_milli_celsius(word: u16) -> u32 {
    ((word as u64 * 175_000 + 65_536 / 2) / 65_536) as u32
}

/// Encodes an ambient pressure as the `set_ambient_pressure` word.
///
/// Table 17: `word = ambient P [Pa] / 100`, so the word is the pressure in
/// hectopascals.
///
/// # Arguments
///
/// * `pascals` - the ambient pressure in pascals.
///
/// # Returns
///
/// The 16-bit pressure word, truncated to whole hectopascals.
pub fn ambient_pressure_word(pascals: u32) -> u16 {
    (pascals / 100).min(0xffff) as u16
}

/// Decodes an ambient pressure word back to pascals.
///
/// The inverse of [`ambient_pressure_word`].
///
/// # Arguments
///
/// * `word` - the pressure word, in hectopascals.
///
/// # Returns
///
/// The pressure in pascals.
pub fn ambient_pressure_pascals(word: u16) -> u32 {
    word as u32 * 100
}

/// Decodes the `perform_forced_recalibration` response to the correction applied.
///
/// Table 18: `FRC correction [ppm] = word - 0x8000`, and a word of `0xffff` means
/// the recalibration failed.
///
/// # Arguments
///
/// * `word` - the word fetched after [`command::PERFORM_FORCED_RECALIBRATION`].
///
/// # Returns
///
/// The signed correction in ppm, or `None` if the recalibration failed.
pub fn forced_recalibration_correction_ppm(word: u16) -> Option<i32> {
    if word == FORCED_RECALIBRATION_FAILED {
        return None;
    }
    Some(word as i32 - 0x8000)
}

/// Builds the `perform_forced_recalibration` response word for a correction.
///
/// The inverse of [`forced_recalibration_correction_ppm`].
///
/// # Arguments
///
/// * `correction_ppm` - the signed correction in ppm, or `None` for a failed
///   recalibration.
///
/// # Returns
///
/// The response word, `correction + 0x8000`, or [`FORCED_RECALIBRATION_FAILED`].
pub fn forced_recalibration_word(correction_ppm: Option<i32>) -> u16 {
    match correction_ppm {
        Some(ppm) => (ppm + 0x8000).clamp(0, 0xfffe) as u16,
        None => FORCED_RECALIBRATION_FAILED,
    }
}

/// Returns whether an automatic self-calibration word reports ASC enabled.
///
/// Tables 19 and 20: `1` means enabled, `0` means disabled.
///
/// # Arguments
///
/// * `word` - the word read after
///   [`command::GET_AUTOMATIC_SELF_CALIBRATION_ENABLED`].
///
/// # Returns
///
/// `true` if automatic self-calibration is enabled.
pub fn automatic_self_calibration_enabled(word: u16) -> bool {
    word == 1
}

/// Builds the automatic self-calibration word for an enabled state.
///
/// The inverse of [`automatic_self_calibration_enabled`], and the word
/// [`command::SET_AUTOMATIC_SELF_CALIBRATION_ENABLED`] carries.
///
/// # Arguments
///
/// * `enabled` - whether automatic self-calibration should be on.
///
/// # Returns
///
/// `1` for enabled, `0` for disabled.
pub fn automatic_self_calibration_word(enabled: bool) -> u16 {
    enabled as u16
}

/// Returns whether the self-test word reports no malfunction.
///
/// Table 25: `0` means no malfunction detected, any other value means a
/// malfunction.
///
/// # Arguments
///
/// * `word` - the word read after [`command::PERFORM_SELF_TEST`].
///
/// # Returns
///
/// `true` if the sensor detected no malfunction.
pub fn self_test_passed(word: u16) -> bool {
    word == 0
}

/// Decodes the nine bytes of `get_serial_number` to the 48-bit serial number.
///
/// Table 24: `serial = word[0] << 32 | word[1] << 16 | word[2]`, each word followed
/// by its CRC.
///
/// # Arguments
///
/// * `frame` - the three word frames read after [`command::GET_SERIAL_NUMBER`].
///
/// # Returns
///
/// The serial number.
///
/// # Errors
///
/// Returns [`SensorError::Crc`] if any of the three CRC bytes does not match its
/// word.
pub fn serial_number(frame: &[u8; 9]) -> Result<u64, SensorError> {
    let w0 = word(&[frame[0], frame[1], frame[2]])? as u64;
    let w1 = word(&[frame[3], frame[4], frame[5]])? as u64;
    let w2 = word(&[frame[6], frame[7], frame[8]])? as u64;
    Ok(w0 << 32 | w1 << 16 | w2)
}

/// Builds the nine bytes a sensor sends for a serial number.
///
/// The inverse of [`serial_number`].
///
/// # Arguments
///
/// * `serial` - the 48-bit serial number; higher bits are discarded.
///
/// # Returns
///
/// The three word frames, each word followed by its CRC.
pub fn serial_number_frame(serial: u64) -> [u8; 9] {
    let w0 = word_frame((serial >> 32) as u16);
    let w1 = word_frame((serial >> 16) as u16);
    let w2 = word_frame(serial as u16);
    [
        w0[0], w0[1], w0[2], w1[0], w1[1], w1[2], w2[0], w2[1], w2[2],
    ]
}

/// One SCD4x measurement, as the three words of `read_measurement`.
///
/// The temperature and humidity words are kept raw so the decode stays exact; the
/// methods convert them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measurement {
    /// The CO2 concentration in parts per million (`word[0]`).
    pub co2_ppm: u16,
    /// The 16-bit temperature word (`word[1]`).
    pub temperature_raw: u16,
    /// The 16-bit humidity word (`word[2]`).
    pub humidity_raw: u16,
}

impl Measurement {
    /// Parses the nine bytes read after [`command::READ_MEASUREMENT`].
    ///
    /// Table 11: CO2, temperature, and humidity words in that order, each most
    /// significant byte first and followed by its CRC.
    ///
    /// # Arguments
    ///
    /// * `frame` - the nine response bytes.
    ///
    /// # Returns
    ///
    /// The measurement.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::Crc`] if any of the three CRC bytes does not match
    /// its word.
    pub fn parse(frame: &[u8; 9]) -> Result<Measurement, SensorError> {
        Ok(Measurement {
            co2_ppm: word(&[frame[0], frame[1], frame[2]])?,
            temperature_raw: word(&[frame[3], frame[4], frame[5]])?,
            humidity_raw: word(&[frame[6], frame[7], frame[8]])?,
        })
    }

    /// Builds a measurement from physical values.
    ///
    /// The inverse of the decode methods, so a node can be fed the frame a sensor
    /// would send for a chosen reading.
    ///
    /// # Arguments
    ///
    /// * `co2_ppm` - the CO2 concentration in ppm.
    /// * `milli_celsius` - the temperature in milli-degrees Celsius.
    /// * `humidity_milli_percent` - the relative humidity in milli-percent.
    ///
    /// # Returns
    ///
    /// The measurement with its raw words built by [`temperature_raw`] and
    /// [`humidity_raw`].
    pub fn from_physical(
        co2_ppm: u16,
        milli_celsius: i32,
        humidity_milli_percent: u32,
    ) -> Measurement {
        Measurement {
            co2_ppm,
            temperature_raw: temperature_raw(milli_celsius),
            humidity_raw: humidity_raw(humidity_milli_percent),
        }
    }

    /// Builds the nine bytes a sensor sends for this measurement.
    ///
    /// The inverse of [`Measurement::parse`].
    ///
    /// # Returns
    ///
    /// The three word frames, each word followed by its CRC.
    pub fn to_bytes(&self) -> [u8; 9] {
        let c = word_frame(self.co2_ppm);
        let t = word_frame(self.temperature_raw);
        let h = word_frame(self.humidity_raw);
        [c[0], c[1], c[2], t[0], t[1], t[2], h[0], h[1], h[2]]
    }

    /// Returns the temperature in milli-degrees Celsius.
    pub fn milli_celsius(&self) -> i32 {
        milli_celsius(self.temperature_raw)
    }

    /// Returns the temperature in degrees Celsius.
    pub fn celsius(&self) -> f32 {
        celsius(self.temperature_raw)
    }

    /// Returns the relative humidity in milli-percent.
    pub fn humidity_milli_percent(&self) -> u32 {
        humidity_milli_percent(self.humidity_raw)
    }

    /// Returns the relative humidity in percent.
    pub fn relative_humidity_percent(&self) -> f32 {
        relative_humidity_percent(self.humidity_raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Table 11's example response: 500 ppm, 25 °C, 37 % RH. The datasheet prints the
    // CO2 word's CRC as 0x7b, which is the CRC of Table 18's 0x7fce; its own algorithm
    // (Table 32) and every other printed example give 0x33 for 0x01f4, so the frame
    // here carries 0x33.
    const EXAMPLE_MEASUREMENT: [u8; 9] = [0x01, 0xf4, 0x33, 0x66, 0x67, 0xa2, 0x5e, 0xb9, 0x3c];

    #[test]
    fn crc_matches_the_datasheet_check_value() {
        // Table 32: CRC(0xBEEF) = 0x92.
        assert_eq!(crc(&[0xBE, 0xEF]), 0x92);
    }

    #[test]
    fn crc_matches_every_worked_example_in_the_datasheet() {
        // Each command example prints the CRC of the word it carries.
        for (word, expected) in [
            (0x6667, 0xa2), // Table 11, 25 °C
            (0x5eb9, 0x3c), // Table 11, 37 % RH
            (0x07e6, 0x48), // Table 13, offset 5.4 °C
            (0x0912, 0x63), // Table 14, offset 6.2 °C
            (0x079e, 0x09), // Table 15, altitude 1950 m
            (0x044c, 0x42), // Table 16, altitude 1100 m
            (0x03db, 0x42), // Table 17, 98700 Pa
            (0x01e0, 0xb4), // Table 18, target 480 ppm
            (0x7fce, 0x7b), // Table 18, correction -50 ppm
            (0x0001, 0xb0), // Table 19, ASC enabled
            (0x0000, 0x81), // Table 20, ASC disabled
            (0x8000, 0xa2), // Table 22, data not ready
            (0xf896, 0x31), // Table 24, serial word[0]
            (0x9f07, 0xc2), // Table 24, serial word[1]
            (0x3bbe, 0x89), // Table 24, serial word[2]
        ] {
            assert_eq!(crc(&u16::to_be_bytes(word)), expected, "crc of {word:#06x}");
            assert_eq!(word_frame(word)[2], expected);
        }
    }

    #[test]
    fn a_word_frame_with_a_corrupted_crc_is_rejected() {
        assert_eq!(word(&[0x01, 0xf4, 0x33]), Ok(0x01f4));
        assert_eq!(word(&[0x01, 0xf4, 0x7b]), Err(SensorError::Crc));
        assert_eq!(word(&[0x01, 0xf5, 0x7b]), Err(SensorError::Crc));
        let mut corrupted = EXAMPLE_MEASUREMENT;
        corrupted[4] ^= 0x01;
        assert_eq!(Measurement::parse(&corrupted), Err(SensorError::Crc));
        corrupted = EXAMPLE_MEASUREMENT;
        corrupted[8] = 0x00;
        assert_eq!(Measurement::parse(&corrupted), Err(SensorError::Crc));
    }

    #[test]
    fn the_datasheet_example_measurement_decodes_to_its_stated_values() {
        // Table 11: 0x01f4 0x6667 0xa2 0x5eb9 0x3c is 500 ppm, 25 °C, 37 % RH; the
        // words decode to 25.003 °C and 37.002 %, which the datasheet rounds.
        let m = Measurement::parse(&EXAMPLE_MEASUREMENT).unwrap();
        assert_eq!(m.co2_ppm, 500);
        assert_eq!(m.temperature_raw, 0x6667);
        assert_eq!(m.humidity_raw, 0x5eb9);
        assert_eq!(m.milli_celsius(), 25_003);
        assert_eq!(m.humidity_milli_percent(), 37_002);
        assert!((m.celsius() - 25.0).abs() < 0.005);
        assert!((m.relative_humidity_percent() - 37.0).abs() < 0.005);
        assert_eq!(m.to_bytes(), EXAMPLE_MEASUREMENT);
    }

    #[test]
    fn a_measurement_built_from_physical_values_reads_back_as_the_example() {
        let m = Measurement::from_physical(500, 25_000, 37_000);
        assert_eq!(m.co2_ppm, 500);
        assert_eq!(m.milli_celsius(), 25_000);
        assert_eq!(m.humidity_milli_percent(), 37_000);
        let parsed = Measurement::parse(&m.to_bytes()).unwrap();
        assert_eq!(parsed, m);
        assert_eq!(parsed.temperature_raw, 0x6666);
    }

    #[test]
    fn temperature_word_endpoints_match_the_formula() {
        // T = -45 + 175 * word / 65535: 0 is -45 °C, 0xffff is 130 °C.
        assert_eq!(milli_celsius(0), -45_000);
        assert_eq!(milli_celsius(0xffff), 130_000);
        assert_eq!(temperature_raw(-45_000), 0);
        assert_eq!(temperature_raw(130_000), 0xffff);
        assert_eq!(temperature_raw(-60_000), 0);
        assert_eq!(temperature_raw(200_000), 0xffff);
    }

    #[test]
    fn humidity_word_endpoints_match_the_formula() {
        // RH = 100 * word / 65535: 0 is 0 %, 0xffff is 100 %.
        assert_eq!(humidity_milli_percent(0), 0);
        assert_eq!(humidity_milli_percent(0xffff), 100_000);
        assert_eq!(humidity_raw(0), 0);
        assert_eq!(humidity_raw(100_000), 0xffff);
        assert_eq!(humidity_raw(150_000), 0xffff);
    }

    #[test]
    fn integer_conversion_tracks_the_floating_point_reference() {
        // Sweep the whole word range against a direct transcription of Table 11's
        // formulas; the milli-unit path must agree to within half a thousandth, and
        // every word must survive its round trip through the raw builder.
        for raw in (0..=0xffffu32).map(|w| w as u16) {
            let t_ref = -45.0 + 175.0 * raw as f64 / 65_535.0;
            let t_int = milli_celsius(raw);
            assert!(
                (t_int as f64 / 1000.0 - t_ref).abs() < 0.00051,
                "temperature {t_int} vs {t_ref} at {raw:#06x}"
            );
            assert!((celsius(raw) as f64 - t_ref).abs() < 0.001);
            assert_eq!(temperature_raw(t_int), raw, "temperature word {raw:#06x}");

            let h_ref = 100.0 * raw as f64 / 65_535.0;
            let h_int = humidity_milli_percent(raw);
            assert!(
                (h_int as f64 / 1000.0 - h_ref).abs() < 0.00051,
                "humidity {h_int} vs {h_ref} at {raw:#06x}"
            );
            assert!((relative_humidity_percent(raw) as f64 - h_ref).abs() < 0.001);
            assert_eq!(humidity_raw(h_int), raw, "humidity word {raw:#06x}");
        }
    }

    #[test]
    fn data_ready_reads_the_low_eleven_bits() {
        // Table 22: 0x8000 has its least significant 11 bits clear, data not ready.
        assert!(!data_ready(0x8000));
        assert!(!data_ready(0xf800));
        assert!(data_ready(0x0001));
        assert!(data_ready(0x8006));
        assert!(data_ready(0x0400));
    }

    #[test]
    fn temperature_offset_matches_the_datasheet_examples() {
        // Table 13: 5.4 °C is written as 0x07e6; Table 14: 0x0912 reads as 6.2 °C.
        assert_eq!(temperature_offset_word(5_400), 0x07e6);
        assert_eq!(temperature_offset_milli_celsius(0x0912), 6_200);
        assert_eq!(
            write_frame(command::SET_TEMPERATURE_OFFSET, 0x07e6),
            [0x24, 0x1d, 0x07, 0xe6, 0x48]
        );
        assert_eq!(
            temperature_offset_word(DEFAULT_TEMPERATURE_OFFSET_MILLI_CELSIUS),
            1_498
        );
        assert_eq!(temperature_offset_milli_celsius(1_498), 4_000);
        // The word is coarser than a thousandth, so raw words round-trip exactly.
        for word in (0..=0xffffu32).map(|w| w as u16) {
            let reference = 175.0 * word as f64 / 65_536.0;
            let milli = temperature_offset_milli_celsius(word);
            assert!((milli as f64 / 1000.0 - reference).abs() < 0.00051);
            assert_eq!(
                temperature_offset_word(milli),
                word,
                "offset word {word:#06x}"
            );
        }
    }

    #[test]
    fn altitude_and_pressure_frames_match_the_datasheet_examples() {
        // Table 15: 1950 m is written as 0x079e 0x09; Table 16: 0x044c reads as 1100 m.
        assert_eq!(
            write_frame(command::SET_SENSOR_ALTITUDE, 1_950),
            [0x24, 0x27, 0x07, 0x9e, 0x09]
        );
        assert_eq!(word(&[0x04, 0x4c, 0x42]), Ok(1_100));
        // Table 17: 98'700 Pa is written as 0x03db 0x42.
        assert_eq!(ambient_pressure_word(98_700), 0x03db);
        assert_eq!(
            write_frame(command::SET_AMBIENT_PRESSURE, ambient_pressure_word(98_700)),
            [0xe0, 0x00, 0x03, 0xdb, 0x42]
        );
        assert_eq!(ambient_pressure_pascals(0x03db), 98_700);
        assert_eq!(ambient_pressure_word(u32::MAX), 0xffff);
    }

    #[test]
    fn forced_recalibration_matches_the_datasheet_example() {
        // Table 18: target 480 ppm is written as 0x01e0 0xb4, and the response
        // 0x7fce is a correction of -50 ppm; 0xffff reports a failure.
        assert_eq!(
            write_frame(command::PERFORM_FORCED_RECALIBRATION, 480),
            [0x36, 0x2f, 0x01, 0xe0, 0xb4]
        );
        assert_eq!(forced_recalibration_correction_ppm(0x7fce), Some(-50));
        assert_eq!(forced_recalibration_correction_ppm(0x8000), Some(0));
        assert_eq!(forced_recalibration_correction_ppm(0xffff), None);
        assert_eq!(forced_recalibration_word(Some(-50)), 0x7fce);
        assert_eq!(forced_recalibration_word(None), FORCED_RECALIBRATION_FAILED);
        assert_eq!(forced_recalibration_word(Some(40_000)), 0xfffe);
    }

    #[test]
    fn automatic_self_calibration_words_match_the_datasheet_examples() {
        // Table 19: enabled is written as 0x0001 0xB0; Table 20: 0x0000 0x81 reads as
        // disabled.
        assert_eq!(
            write_frame(
                command::SET_AUTOMATIC_SELF_CALIBRATION_ENABLED,
                automatic_self_calibration_word(true)
            ),
            [0x24, 0x16, 0x00, 0x01, 0xb0]
        );
        assert_eq!(automatic_self_calibration_word(false), 0);
        assert!(automatic_self_calibration_enabled(
            word(&[0x00, 0x01, 0xb0]).unwrap()
        ));
        assert!(!automatic_self_calibration_enabled(
            word(&[0x00, 0x00, 0x81]).unwrap()
        ));
    }

    #[test]
    fn self_test_passes_only_on_zero() {
        // Table 25: 0x0000 0x81 is no malfunction detected.
        assert!(self_test_passed(word(&[0x00, 0x00, 0x81]).unwrap()));
        assert!(!self_test_passed(0x0001));
    }

    #[test]
    fn serial_number_matches_the_datasheet_example() {
        // Table 24: 0xf896 0x31 0x9f07 0xc2 0x3bbe 0x89 is serial 273'325'796'834'238.
        let frame = [0xf8, 0x96, 0x31, 0x9f, 0x07, 0xc2, 0x3b, 0xbe, 0x89];
        assert_eq!(serial_number(&frame), Ok(273_325_796_834_238));
        assert_eq!(serial_number_frame(273_325_796_834_238), frame);
        let mut corrupted = frame;
        corrupted[5] ^= 0x80;
        assert_eq!(serial_number(&corrupted), Err(SensorError::Crc));
    }

    #[test]
    fn command_words_and_durations_match_table_9() {
        assert_eq!(
            command_frame(command::START_PERIODIC_MEASUREMENT),
            [0x21, 0xb1]
        );
        assert_eq!(
            command_frame(command::STOP_PERIODIC_MEASUREMENT),
            [0x3f, 0x86]
        );
        assert_eq!(max_duration_ms(command::START_PERIODIC_MEASUREMENT), None);
        assert_eq!(
            max_duration_ms(command::START_LOW_POWER_PERIODIC_MEASUREMENT),
            None
        );
        assert_eq!(max_duration_ms(command::READ_MEASUREMENT), Some(1));
        assert_eq!(
            max_duration_ms(command::STOP_PERIODIC_MEASUREMENT),
            Some(500)
        );
        assert_eq!(
            max_duration_ms(command::PERFORM_FORCED_RECALIBRATION),
            Some(400)
        );
        assert_eq!(max_duration_ms(command::PERSIST_SETTINGS), Some(800));
        assert_eq!(max_duration_ms(command::PERFORM_SELF_TEST), Some(10_000));
        assert_eq!(max_duration_ms(command::PERFORM_FACTORY_RESET), Some(1_200));
        assert_eq!(max_duration_ms(command::REINIT), Some(20));
        assert_eq!(max_duration_ms(command::MEASURE_SINGLE_SHOT), Some(5_000));
        assert_eq!(
            max_duration_ms(command::MEASURE_SINGLE_SHOT_RHT_ONLY),
            Some(50)
        );
        assert_eq!(max_duration_ms(command::POWER_DOWN), Some(1));
        assert_eq!(max_duration_ms(command::WAKE_UP), Some(20));
        assert_eq!(max_duration_ms(0x0000), None);
    }

    #[test]
    fn only_four_commands_are_allowed_during_a_measurement() {
        assert!(allowed_during_measurement(command::READ_MEASUREMENT));
        assert!(allowed_during_measurement(
            command::STOP_PERIODIC_MEASUREMENT
        ));
        assert!(allowed_during_measurement(command::SET_AMBIENT_PRESSURE));
        assert!(allowed_during_measurement(command::GET_DATA_READY_STATUS));
        assert!(!allowed_during_measurement(command::SET_TEMPERATURE_OFFSET));
        assert!(!allowed_during_measurement(
            command::PERFORM_FORCED_RECALIBRATION
        ));
        assert!(!allowed_during_measurement(
            command::START_PERIODIC_MEASUREMENT
        ));
    }
}
