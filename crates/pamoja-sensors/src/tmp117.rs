//! Texas Instruments TMP117 high-accuracy digital temperature sensor.
//!
//! The TMP117 returns a 16-bit two's-complement temperature at 7.8125 m°C per count,
//! accurate to ±0.1 °C without calibration, and carries its own alert limits, a
//! user offset, and a small EEPROM. This module decodes the temperature register,
//! builds the same-format values the limit and offset registers take, and assembles
//! or parses the configuration register field by field, following the datasheet's
//! register map, its 16-bit temperature data table, and its conversion cycle table.
//!
//! Temperatures are returned in integer nano- and micro-degrees so the conversion
//! stays in integer arithmetic; the `f32` form is exact too, since one count is
//! 1/128 °C.

#[cfg(feature = "embedded-hal")]
mod driver;

#[cfg(feature = "embedded-hal")]
pub use driver::{Tmp117, STATUS_POLLS};

/// The TMP117 register addresses, written to the pointer register.
pub mod register {
    /// Temperature result register, read-only, two's complement at 7.8125 m°C.
    pub const TEMP_RESULT: u8 = 0x00;
    /// Configuration register: mode, cycle, averaging, alert setup, and flags.
    pub const CONFIGURATION: u8 = 0x01;
    /// High limit register, same format as the temperature result.
    pub const THIGH_LIMIT: u8 = 0x02;
    /// Low limit register, same format as the temperature result.
    pub const TLOW_LIMIT: u8 = 0x03;
    /// EEPROM unlock register.
    pub const EEPROM_UL: u8 = 0x04;
    /// EEPROM1 scratch register, factory-programmed with part of the unique ID.
    pub const EEPROM1: u8 = 0x05;
    /// EEPROM2 scratch register.
    pub const EEPROM2: u8 = 0x06;
    /// Temperature offset register, added to the result after linearisation.
    pub const TEMP_OFFSET: u8 = 0x07;
    /// EEPROM3 scratch register.
    pub const EEPROM3: u8 = 0x08;
    /// Device ID register: revision in bits 15:12, device ID in bits 11:0.
    pub const DEVICE_ID: u8 = 0x0F;
}

/// The four 7-bit bus addresses, selected by where the ADD0 pin is tied.
pub mod address {
    /// ADD0 tied to ground.
    pub const ADD0_GND: u8 = 0x48;
    /// ADD0 tied to V+.
    pub const ADD0_VPLUS: u8 = 0x49;
    /// ADD0 tied to SDA.
    pub const ADD0_SDA: u8 = 0x4A;
    /// ADD0 tied to SCL.
    pub const ADD0_SCL: u8 = 0x4B;
}

/// The device ID field a TMP117 reports (bits 11:0 of the Device_ID register).
pub const DEVICE_ID: u16 = 0x0117;

/// The factory value of the configuration register (0x0220): continuous conversion,
/// a 1 s cycle, 8 averaged conversions, alert mode, ALERT active low.
pub const CONFIG_RESET: u16 = 0x0220;

/// The factory value of the high limit register (0x6000, 192 °C).
pub const HIGH_LIMIT_RESET: u16 = 0x6000;

/// The factory value of the low limit register (0x8000, -256 °C).
pub const LOW_LIMIT_RESET: u16 = 0x8000;

/// The temperature register value after a reset (0x8000, -256 °C), held until the
/// first conversion completes.
pub const TEMP_RESULT_RESET: u16 = 0x8000;

/// The command byte that, sent after the general-call address `0x00`, resets every
/// register to its power-up value.
pub const GENERAL_CALL_RESET: u8 = 0x06;

/// Writing this to the EEPROM unlock register (bit 15, EUN) makes subsequent writes
/// to the programmable registers persist in EEPROM.
pub const EEPROM_UNLOCK: u16 = 0x8000;

/// One temperature count in nanodegrees Celsius: 7.8125 m°C.
const LSB_NANO_CELSIUS: i64 = 7_812_500;

/// Decodes a temperature, limit, or offset register to nanodegrees Celsius.
///
/// One count is 7.8125 m°C, so this conversion is exact for every code.
///
/// # Arguments
///
/// * `raw` - the signed 16-bit register value.
///
/// # Returns
///
/// The temperature in nanodegrees Celsius.
pub fn nano_celsius(raw: i16) -> i64 {
    raw as i64 * LSB_NANO_CELSIUS
}

/// Decodes a temperature, limit, or offset register to microdegrees Celsius.
///
/// Odd codes end in half a microdegree, which is dropped toward zero: `0x0001` is
/// `7812` and `0xFFFF` is `-7812`. Use [`nano_celsius`] where that half matters.
///
/// # Arguments
///
/// * `raw` - the signed 16-bit register value.
///
/// # Returns
///
/// The temperature in microdegrees Celsius, truncated toward zero.
pub fn micro_celsius(raw: i16) -> i32 {
    (raw as i64 * 78_125 / 10) as i32
}

/// Decodes a temperature, limit, or offset register to degrees Celsius.
///
/// One count is exactly 1/128 °C, so the result is exact in `f32`.
///
/// # Arguments
///
/// * `raw` - the signed 16-bit register value.
///
/// # Returns
///
/// The temperature in degrees Celsius.
pub fn celsius(raw: i16) -> f32 {
    raw as f32 / 128.0
}

/// Builds the register value nearest a temperature in microdegrees Celsius.
///
/// The inverse of [`micro_celsius`], for the limit and offset registers and for
/// testing a node against what a sensor sends without one attached. Rounds to the
/// nearest count, halves away from zero, and saturates at the ±256 °C register
/// range.
///
/// # Arguments
///
/// * `micro` - the temperature in microdegrees Celsius.
///
/// # Returns
///
/// The signed 16-bit register value.
pub fn raw_from_micro_celsius(micro: i32) -> i16 {
    let scaled = micro as i64 * 4;
    let half = if micro < 0 { -15_625 } else { 15_625 };
    saturate((scaled + half) / 31_250)
}

/// Builds the register value nearest a temperature in degrees Celsius.
///
/// The inverse of [`celsius`]. Rounds to the nearest count, halves away from zero,
/// and saturates at the ±256 °C register range.
///
/// # Arguments
///
/// * `degrees` - the temperature in degrees Celsius.
///
/// # Returns
///
/// The signed 16-bit register value.
pub fn raw_from_celsius(degrees: f32) -> i16 {
    let counts = degrees * 128.0;
    if counts.is_nan() {
        return 0;
    }
    let rounded = if counts < 0.0 {
        -((-counts + 0.5) as i64)
    } else {
        (counts + 0.5) as i64
    };
    saturate(rounded)
}

fn saturate(counts: i64) -> i16 {
    counts.clamp(i16::MIN as i64, i16::MAX as i64) as i16
}

/// Splits a temperature-format register into the two bytes the bus carries.
///
/// The TMP117 sends and receives register bytes most significant byte first.
///
/// # Arguments
///
/// * `raw` - the signed 16-bit register value.
///
/// # Returns
///
/// The most significant byte then the least significant byte.
pub fn temperature_bytes(raw: i16) -> [u8; 2] {
    raw.to_be_bytes()
}

/// Joins the two bytes read from a temperature-format register.
///
/// # Arguments
///
/// * `bytes` - the most significant byte then the least significant byte.
///
/// # Returns
///
/// The signed 16-bit register value.
pub fn temperature_from_bytes(bytes: [u8; 2]) -> i16 {
    i16::from_be_bytes(bytes)
}

/// Returns the device ID field of a Device_ID register read (bits 11:0).
///
/// # Arguments
///
/// * `raw` - the Device_ID register.
///
/// # Returns
///
/// The 12-bit device ID, [`DEVICE_ID`] for a TMP117.
pub fn device_id(raw: u16) -> u16 {
    raw & 0x0FFF
}

/// Returns the revision field of a Device_ID register read (bits 15:12).
///
/// # Arguments
///
/// * `raw` - the Device_ID register.
///
/// # Returns
///
/// The 4-bit silicon revision number.
pub fn revision(raw: u16) -> u8 {
    (raw >> 12) as u8
}

/// Returns whether the configuration register's HIGH_Alert flag (bit 15) is set.
///
/// In alert mode the flag means the last result was above the high limit and reading
/// the configuration register clears it; in therm mode it stays set until a result
/// falls below the low limit.
///
/// # Arguments
///
/// * `config` - the configuration register.
///
/// # Returns
///
/// `true` if the high alert flag is set.
pub fn high_alert(config: u16) -> bool {
    config & (1 << 15) != 0
}

/// Returns whether the configuration register's LOW_Alert flag (bit 14) is set.
///
/// The flag means the last result was below the low limit; it always reads `0` in
/// therm mode.
///
/// # Arguments
///
/// * `config` - the configuration register.
///
/// # Returns
///
/// `true` if the low alert flag is set.
pub fn low_alert(config: u16) -> bool {
    config & (1 << 14) != 0
}

/// Returns whether the configuration register's Data_Ready flag (bit 13) is set.
///
/// The flag is set when a conversion completes and cleared by reading either the
/// temperature register or the configuration register.
///
/// # Arguments
///
/// * `config` - the configuration register.
///
/// # Returns
///
/// `true` if a new temperature result is waiting.
pub fn data_ready(config: u16) -> bool {
    config & (1 << 13) != 0
}

/// Returns whether the configuration register's EEPROM_Busy flag (bit 12) is set.
///
/// # Arguments
///
/// * `config` - the configuration register.
///
/// # Returns
///
/// `true` while the EEPROM is programming or loading at power-up.
pub fn eeprom_busy(config: u16) -> bool {
    config & (1 << 12) != 0
}

/// Returns whether the EEPROM unlock register's EEPROM_Busy flag (bit 14) is set.
///
/// This mirrors bit 12 of the configuration register.
///
/// # Arguments
///
/// * `unlock` - the EEPROM_UL register.
///
/// # Returns
///
/// `true` while the EEPROM is programming or loading at power-up.
pub fn eeprom_unlock_busy(unlock: u16) -> bool {
    unlock & (1 << 14) != 0
}

/// The conversion mode (configuration bits 11:10, MOD).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionMode {
    /// Convert continuously at the configured cycle (the default, code `00`).
    Continuous,
    /// Abort any conversion and power down (code `01`).
    Shutdown,
    /// Convert once, then power down (code `11`).
    OneShot,
}

impl ConversionMode {
    /// Returns the 2-bit field code for this mode.
    pub fn code(self) -> u8 {
        match self {
            ConversionMode::Continuous => 0b00,
            ConversionMode::Shutdown => 0b01,
            ConversionMode::OneShot => 0b11,
        }
    }

    /// Builds a mode from a 2-bit field code.
    ///
    /// Code `10` is documented as continuous conversion that reads back as `00`, so
    /// it maps to [`ConversionMode::Continuous`].
    pub fn from_code(code: u8) -> ConversionMode {
        match code & 0b11 {
            0b01 => ConversionMode::Shutdown,
            0b11 => ConversionMode::OneShot,
            _ => ConversionMode::Continuous,
        }
    }
}

/// The number of conversions averaged into each result (configuration bits 6:5, AVG).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Averaging {
    /// No averaging: each result is one 15.5 ms conversion.
    None,
    /// 8 averaged conversions (the default).
    X8,
    /// 32 averaged conversions.
    X32,
    /// 64 averaged conversions.
    X64,
}

impl Averaging {
    /// Returns the 2-bit field code for this averaging setting.
    pub fn code(self) -> u8 {
        match self {
            Averaging::None => 0b00,
            Averaging::X8 => 0b01,
            Averaging::X32 => 0b10,
            Averaging::X64 => 0b11,
        }
    }

    /// Builds an averaging setting from a 2-bit field code.
    pub fn from_code(code: u8) -> Averaging {
        match code & 0b11 {
            0b00 => Averaging::None,
            0b01 => Averaging::X8,
            0b10 => Averaging::X32,
            _ => Averaging::X64,
        }
    }

    /// Returns how many conversions are averaged into a result.
    pub fn conversions(self) -> u8 {
        match self {
            Averaging::None => 1,
            Averaging::X8 => 8,
            Averaging::X32 => 32,
            Averaging::X64 => 64,
        }
    }

    /// Returns the active conversion time in microseconds, which is also the shortest
    /// continuous cycle this averaging allows and the length of a one-shot conversion.
    pub fn conversion_micros(self) -> u32 {
        match self {
            Averaging::None => 15_500,
            Averaging::X8 => 125_000,
            Averaging::X32 => 500_000,
            Averaging::X64 => 1_000_000,
        }
    }
}

/// The nominal conversion cycle in continuous mode (configuration bits 9:7, CONV).
///
/// The names give the cycle with no averaging; with averaging the cycle is never
/// shorter than the conversions take, see [`cycle_micros`](ConversionCycle::cycle_micros).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionCycle {
    /// 15.5 ms (code `000`).
    Ms15_5,
    /// 125 ms (code `001`).
    Ms125,
    /// 250 ms (code `010`).
    Ms250,
    /// 500 ms (code `011`).
    Ms500,
    /// 1 s (code `100`, the default).
    S1,
    /// 4 s (code `101`).
    S4,
    /// 8 s (code `110`).
    S8,
    /// 16 s (code `111`).
    S16,
}

impl ConversionCycle {
    /// Returns the 3-bit field code for this cycle setting.
    pub fn code(self) -> u8 {
        match self {
            ConversionCycle::Ms15_5 => 0b000,
            ConversionCycle::Ms125 => 0b001,
            ConversionCycle::Ms250 => 0b010,
            ConversionCycle::Ms500 => 0b011,
            ConversionCycle::S1 => 0b100,
            ConversionCycle::S4 => 0b101,
            ConversionCycle::S8 => 0b110,
            ConversionCycle::S16 => 0b111,
        }
    }

    /// Builds a cycle setting from a 3-bit field code.
    pub fn from_code(code: u8) -> ConversionCycle {
        match code & 0b111 {
            0b000 => ConversionCycle::Ms15_5,
            0b001 => ConversionCycle::Ms125,
            0b010 => ConversionCycle::Ms250,
            0b011 => ConversionCycle::Ms500,
            0b100 => ConversionCycle::S1,
            0b101 => ConversionCycle::S4,
            0b110 => ConversionCycle::S8,
            _ => ConversionCycle::S16,
        }
    }

    /// Returns the nominal cycle in microseconds, with no averaging.
    pub fn nominal_micros(self) -> u32 {
        match self {
            ConversionCycle::Ms15_5 => 15_500,
            ConversionCycle::Ms125 => 125_000,
            ConversionCycle::Ms250 => 250_000,
            ConversionCycle::Ms500 => 500_000,
            ConversionCycle::S1 => 1_000_000,
            ConversionCycle::S4 => 4_000_000,
            ConversionCycle::S8 => 8_000_000,
            ConversionCycle::S16 => 16_000_000,
        }
    }

    /// Returns the actual continuous-mode cycle in microseconds for an averaging
    /// setting, per the datasheet's conversion cycle table.
    ///
    /// When the averaged conversions take longer than the nominal cycle there is no
    /// standby time and the cycle stretches to the conversion time.
    ///
    /// # Arguments
    ///
    /// * `averaging` - the averaging setting in force.
    ///
    /// # Returns
    ///
    /// The time between result updates, in microseconds.
    pub fn cycle_micros(self, averaging: Averaging) -> u32 {
        self.nominal_micros().max(averaging.conversion_micros())
    }
}

/// How the limits are applied (configuration bit 4, T/nA).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertMode {
    /// Alert mode (the default): a result above the high limit sets HIGH_Alert, one
    /// below the low limit sets LOW_Alert, and reading the configuration clears them.
    Alert,
    /// Therm mode: HIGH_Alert sets above the high limit and clears only once a result
    /// falls below the low limit, so the two limits act as a hysteresis band.
    Therm,
}

/// The ALERT pin's active level (configuration bit 3, POL).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertPolarity {
    /// Active low (the default).
    ActiveLow,
    /// Active high.
    ActiveHigh,
}

/// What the ALERT pin reflects (configuration bit 2, DR/Alert).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertPin {
    /// The alert flags (the default).
    AlertFlags,
    /// The data ready flag.
    DataReady,
}

/// A decoded TMP117 configuration register.
///
/// Build one, set the fields, and turn it into the 16-bit register value with
/// [`bits`](Configuration::bits); or parse a register read with
/// [`from_bits`](Configuration::from_bits). [`Configuration::default`] is the factory
/// state, `0x0220`. The four flags are read-only on the device and are ignored when
/// written; `soft_reset` triggers a 2 ms reset when written and always reads back
/// clear.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::tmp117::{Averaging, Configuration, ConversionCycle};
///
/// // Average 32 conversions every 4 s, leaving everything else at the factory default.
/// let config = Configuration {
///     cycle: ConversionCycle::S4,
///     averaging: Averaging::X32,
///     ..Configuration::default()
/// };
/// // The high byte then the low byte are written to the configuration register.
/// let [hi, lo] = config.bits().to_be_bytes();
/// assert_eq!(Configuration::from_bits(u16::from_be_bytes([hi, lo])), config);
/// assert_eq!(config.cycle.cycle_micros(config.averaging), 4_000_000);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Configuration {
    /// HIGH_Alert flag (bit 15), read-only.
    pub high_alert: bool,
    /// LOW_Alert flag (bit 14), read-only.
    pub low_alert: bool,
    /// Data_Ready flag (bit 13), read-only.
    pub data_ready: bool,
    /// EEPROM_Busy flag (bit 12), read-only.
    pub eeprom_busy: bool,
    /// The conversion mode.
    pub mode: ConversionMode,
    /// The continuous-mode conversion cycle.
    pub cycle: ConversionCycle,
    /// The number of conversions averaged per result.
    pub averaging: Averaging,
    /// Whether the limits act as alerts or as a therm hysteresis band.
    pub alert_mode: AlertMode,
    /// The ALERT pin's active level.
    pub alert_polarity: AlertPolarity,
    /// What the ALERT pin reflects.
    pub alert_pin: AlertPin,
    /// Soft_Reset (bit 1): set to trigger a software reset when written.
    pub soft_reset: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            high_alert: false,
            low_alert: false,
            data_ready: false,
            eeprom_busy: false,
            mode: ConversionMode::Continuous,
            cycle: ConversionCycle::S1,
            averaging: Averaging::X8,
            alert_mode: AlertMode::Alert,
            alert_polarity: AlertPolarity::ActiveLow,
            alert_pin: AlertPin::AlertFlags,
            soft_reset: false,
        }
    }
}

impl Configuration {
    /// Assembles the 16-bit configuration register value.
    ///
    /// # Returns
    ///
    /// The register value to write, most significant byte first on the bus.
    pub fn bits(self) -> u16 {
        let mut bits = 0u16;
        bits |= u16::from(self.high_alert) << 15;
        bits |= u16::from(self.low_alert) << 14;
        bits |= u16::from(self.data_ready) << 13;
        bits |= u16::from(self.eeprom_busy) << 12;
        bits |= u16::from(self.mode.code()) << 10;
        bits |= u16::from(self.cycle.code()) << 7;
        bits |= u16::from(self.averaging.code()) << 5;
        bits |= u16::from(matches!(self.alert_mode, AlertMode::Therm)) << 4;
        bits |= u16::from(matches!(self.alert_polarity, AlertPolarity::ActiveHigh)) << 3;
        bits |= u16::from(matches!(self.alert_pin, AlertPin::DataReady)) << 2;
        bits |= u16::from(self.soft_reset) << 1;
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
    /// The decoded configuration.
    pub fn from_bits(bits: u16) -> Configuration {
        Configuration {
            high_alert: high_alert(bits),
            low_alert: low_alert(bits),
            data_ready: data_ready(bits),
            eeprom_busy: eeprom_busy(bits),
            mode: ConversionMode::from_code((bits >> 10) as u8),
            cycle: ConversionCycle::from_code((bits >> 7) as u8),
            averaging: Averaging::from_code((bits >> 5) as u8),
            alert_mode: if bits & (1 << 4) != 0 {
                AlertMode::Therm
            } else {
                AlertMode::Alert
            },
            alert_polarity: if bits & (1 << 3) != 0 {
                AlertPolarity::ActiveHigh
            } else {
                AlertPolarity::ActiveLow
            },
            alert_pin: if bits & (1 << 2) != 0 {
                AlertPin::DataReady
            } else {
                AlertPin::AlertFlags
            },
            soft_reset: bits & (1 << 1) != 0,
        }
    }
}

/// One temperature result: the register word, with the conversions to real units.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::tmp117::Reading;
///
/// // Table 7-1: 25 C is 0C80h.
/// let reading = Reading::new(0x0C80);
/// assert_eq!(reading.celsius(), 25.0);
/// assert_eq!(reading.micro_celsius(), 25_000_000);
/// assert_eq!(reading.to_bytes(), [0x0C, 0x80]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    raw: i16,
}

impl Reading {
    /// Wraps a temperature register word.
    ///
    /// # Arguments
    ///
    /// * `raw` - the signed 16-bit register value, 7.8125 m°C per count.
    ///
    /// # Returns
    ///
    /// The reading.
    pub fn new(raw: i16) -> Reading {
        Reading { raw }
    }

    /// Returns the register word.
    pub fn raw(&self) -> i16 {
        self.raw
    }

    /// Returns the temperature in degrees Celsius.
    pub fn celsius(&self) -> f32 {
        celsius(self.raw)
    }

    /// Returns the temperature in microdegrees Celsius, exact in integer arithmetic.
    pub fn micro_celsius(&self) -> i32 {
        micro_celsius(self.raw)
    }

    /// Returns the temperature in nanodegrees Celsius, exact in integer arithmetic.
    pub fn nano_celsius(&self) -> i64 {
        nano_celsius(self.raw)
    }

    /// Returns the register bytes, most significant first, as the part sends them.
    pub fn to_bytes(&self) -> [u8; 2] {
        temperature_bytes(self.raw)
    }
}

/// The two alert flags the part sets at the end of every conversion.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::tmp117::Alerts;
///
/// // Table 7-6: HIGH_Alert is bit 15 and LOW_Alert bit 14 of the configuration register.
/// let alerts = Alerts::from_bits(0x8220);
/// assert!(alerts.high);
/// assert!(!alerts.low);
/// assert_eq!(alerts | Alerts { high: false, low: true }, Alerts { high: true, low: true });
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Alerts {
    /// A result was above the high limit.
    pub high: bool,
    /// A result was below the low limit.
    pub low: bool,
}

impl Alerts {
    /// Reads the two flags from a configuration register value.
    ///
    /// # Arguments
    ///
    /// * `config` - the word read from [`register::CONFIGURATION`].
    ///
    /// # Returns
    ///
    /// The flags.
    pub fn from_bits(config: u16) -> Alerts {
        Alerts {
            high: high_alert(config),
            low: low_alert(config),
        }
    }

    /// Reports whether either flag is set.
    pub fn any(&self) -> bool {
        self.high || self.low
    }
}

impl core::ops::BitOr for Alerts {
    type Output = Alerts;

    fn bitor(self, other: Alerts) -> Alerts {
        Alerts {
            high: self.high || other.high,
            low: self.low || other.low,
        }
    }
}

#[cfg(test)]
mod driver_support_tests {
    use super::*;

    #[test]
    fn a_reading_converts_its_word_like_the_free_functions() {
        let reading = Reading::new(0xF380_u16 as i16);
        assert_eq!(reading.raw(), -3200);
        assert_eq!(reading.celsius(), -25.0);
        assert_eq!(reading.micro_celsius(), -25_000_000);
        assert_eq!(reading.nano_celsius(), -25_000_000_000);
        assert_eq!(reading.to_bytes(), [0xF3, 0x80]);
    }

    #[test]
    fn alerts_read_bits_15_and_14_and_combine() {
        assert_eq!(
            Alerts::from_bits(1 << 15),
            Alerts {
                high: true,
                low: false
            }
        );
        assert_eq!(
            Alerts::from_bits(1 << 14),
            Alerts {
                high: false,
                low: true
            }
        );
        assert_eq!(Alerts::from_bits(0x0220), Alerts::default());
        assert!(!Alerts::default().any());
        assert!((Alerts::from_bits(1 << 15) | Alerts::default()).any());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Table 7-1, 16-Bit Temperature Data Format: every row, as (hex, °C).
    const TABLE_7_1: [(u16, f64); 11] = [
        (0x8000, -256.0),
        (0xF380, -25.0),
        (0xFFF0, -0.125),
        (0xFFFF, -0.0078125),
        (0x0000, 0.0),
        (0x0001, 0.0078125),
        (0x0010, 0.125),
        (0x0080, 1.0),
        (0x0C80, 25.0),
        (0x3200, 100.0),
        (0x7FFF, 255.9921875),
    ];

    #[test]
    fn temperature_table_rows_decode_to_the_datasheet_values() {
        // Table 7-1 prints the last row as 255.9921; the exact value at 7.8125 m°C
        // per count is 255.9921875 °C.
        for &(hex, degrees) in &TABLE_7_1 {
            let raw = hex as i16;
            assert_eq!(
                nano_celsius(raw),
                (degrees * 1e9).round() as i64,
                "{hex:04X}"
            );
            assert_eq!(celsius(raw), degrees as f32, "{hex:04X}");
        }
        assert_eq!(micro_celsius(0x0C80), 25_000_000);
        assert_eq!(micro_celsius(0x3200), 100_000_000);
        assert_eq!(micro_celsius(0xF380_u16 as i16), -25_000_000);
        assert_eq!(micro_celsius(0xFFF0_u16 as i16), -125_000);
        assert_eq!(micro_celsius(0x8000_u16 as i16), -256_000_000);
        assert_eq!(micro_celsius(0x7FFF), 255_992_187);
    }

    #[test]
    fn one_count_is_the_7_8125_millidegree_lsb() {
        // Section 6.5, temperature resolution (LSB) 7.8125 m°C; Table 7-1 rows 0001
        // and FFFF.
        assert_eq!(nano_celsius(1), 7_812_500);
        assert_eq!(nano_celsius(-1), -7_812_500);
        assert_eq!(micro_celsius(1), 7_812);
        assert_eq!(micro_celsius(-1), -7_812);
        assert_eq!(celsius(1), 0.0078125);
        assert_eq!(celsius(-1), -0.0078125);
    }

    #[test]
    fn register_resets_decode_to_the_datasheet_temperatures() {
        // Table 7-3: Temp_Result 8000h (-256 °C until the first conversion),
        // THigh_Limit 6000h, TLow_Limit 8000h. 6000h is 24576 counts, 192 °C.
        assert_eq!(celsius(TEMP_RESULT_RESET as i16), -256.0);
        assert_eq!(micro_celsius(HIGH_LIMIT_RESET as i16), 192_000_000);
        assert_eq!(micro_celsius(LOW_LIMIT_RESET as i16), -256_000_000);
    }

    #[test]
    fn builders_reproduce_the_table_rows_and_saturate_at_the_register_range() {
        for &(hex, degrees) in &TABLE_7_1 {
            assert_eq!(raw_from_celsius(degrees as f32), hex as i16, "{hex:04X}");
        }
        assert_eq!(raw_from_micro_celsius(25_000_000), 0x0C80);
        assert_eq!(raw_from_micro_celsius(-25_000_000), 0xF380_u16 as i16);
        assert_eq!(raw_from_micro_celsius(7_812), 1);
        assert_eq!(raw_from_micro_celsius(-7_812), -1);
        assert_eq!(raw_from_micro_celsius(3_906), 0);
        assert_eq!(raw_from_micro_celsius(3_907), 1);
        assert_eq!(raw_from_micro_celsius(-3_906), 0);
        assert_eq!(raw_from_micro_celsius(-3_907), -1);
        // Section 7.6.4: the range of the register is ±256 °C.
        assert_eq!(raw_from_micro_celsius(256_000_000), i16::MAX);
        assert_eq!(raw_from_micro_celsius(i32::MAX), i16::MAX);
        assert_eq!(raw_from_micro_celsius(-256_000_000), i16::MIN);
        assert_eq!(raw_from_micro_celsius(i32::MIN), i16::MIN);
        assert_eq!(raw_from_celsius(300.0), i16::MAX);
        assert_eq!(raw_from_celsius(-300.0), i16::MIN);
        assert_eq!(raw_from_celsius(f32::INFINITY), i16::MAX);
        assert_eq!(raw_from_celsius(f32::NEG_INFINITY), i16::MIN);
        assert_eq!(raw_from_celsius(f32::NAN), 0);
    }

    #[test]
    fn every_code_round_trips_and_the_integer_paths_track_the_floating_point_lsb() {
        // Section 7.3.3: two's complement, 16 bits, 7.8125 m°C resolution.
        for code in i16::MIN..=i16::MAX {
            let reference = code as f64 * 0.0078125;
            assert_eq!(nano_celsius(code) as f64, reference * 1e9, "{code}");
            assert_eq!(celsius(code) as f64, reference, "{code}");
            assert_eq!(
                micro_celsius(code) as f64,
                (reference * 1e6).trunc(),
                "{code}"
            );
            assert_eq!(raw_from_micro_celsius(micro_celsius(code)), code, "{code}");
            assert_eq!(raw_from_celsius(celsius(code)), code, "{code}");
            assert_eq!(
                temperature_from_bytes(temperature_bytes(code)),
                code,
                "{code}"
            );
        }
    }

    #[test]
    fn register_bytes_travel_most_significant_first() {
        // Section 7.5.3: register bytes are sent with the most significant byte first.
        // Table 7-1: 25 °C is 0C80h.
        assert_eq!(temperature_bytes(0x0C80), [0x0C, 0x80]);
        assert_eq!(temperature_from_bytes([0x0C, 0x80]), 0x0C80);
        assert_eq!(celsius(temperature_from_bytes([0xF3, 0x80])), -25.0);
    }

    #[test]
    fn bus_addresses_follow_the_add0_pin() {
        // Table 7-2: 1001000x ground, 1001001x V+, 1001010x SDA, 1001011x SCL.
        assert_eq!(address::ADD0_GND, 0b1001000);
        assert_eq!(address::ADD0_VPLUS, 0b1001001);
        assert_eq!(address::ADD0_SDA, 0b1001010);
        assert_eq!(address::ADD0_SCL, 0b1001011);
    }

    #[test]
    fn register_addresses_match_the_register_map() {
        // Table 7-3.
        assert_eq!(register::TEMP_RESULT, 0x00);
        assert_eq!(register::CONFIGURATION, 0x01);
        assert_eq!(register::THIGH_LIMIT, 0x02);
        assert_eq!(register::TLOW_LIMIT, 0x03);
        assert_eq!(register::EEPROM_UL, 0x04);
        assert_eq!(register::EEPROM1, 0x05);
        assert_eq!(register::EEPROM2, 0x06);
        assert_eq!(register::TEMP_OFFSET, 0x07);
        assert_eq!(register::EEPROM3, 0x08);
        assert_eq!(register::DEVICE_ID, 0x0F);
    }

    #[test]
    fn device_id_register_splits_into_revision_and_id() {
        // Table 7-3 and Table 7-15: reset 0117h, Rev[3:0] in 15:12, DID[11:0] = 117h.
        assert_eq!(device_id(0x0117), DEVICE_ID);
        assert_eq!(revision(0x0117), 0);
        assert_eq!(device_id(0x1117), DEVICE_ID);
        assert_eq!(revision(0x1117), 1);
        assert_ne!(device_id(0x0116), DEVICE_ID);
    }

    #[test]
    fn default_configuration_is_the_factory_reset_value() {
        // Table 7-3 and Table 7-6: 0220h, MOD 00, CONV 100, AVG 01, T/nA 0, POL 0,
        // DR/Alert 0.
        assert_eq!(Configuration::default().bits(), CONFIG_RESET);
        assert_eq!(
            Configuration::from_bits(CONFIG_RESET),
            Configuration::default()
        );
        let config = Configuration::default();
        assert_eq!(config.mode, ConversionMode::Continuous);
        assert_eq!(config.cycle, ConversionCycle::S1);
        assert_eq!(config.averaging, Averaging::X8);
        assert_eq!(config.alert_mode, AlertMode::Alert);
        assert_eq!(config.alert_polarity, AlertPolarity::ActiveLow);
        assert_eq!(config.alert_pin, AlertPin::AlertFlags);
    }

    #[test]
    fn configuration_fields_sit_at_the_datasheet_bit_positions() {
        // Figure 7-14 and Table 7-6.
        let config = Configuration {
            mode: ConversionMode::OneShot,
            cycle: ConversionCycle::S16,
            averaging: Averaging::X64,
            alert_mode: AlertMode::Therm,
            alert_polarity: AlertPolarity::ActiveHigh,
            alert_pin: AlertPin::DataReady,
            soft_reset: true,
            ..Configuration::default()
        };
        assert_eq!(config.bits(), 0b0000_1111_1111_1110);
        assert_eq!(Configuration::from_bits(config.bits()), config);
        let shutdown = Configuration {
            mode: ConversionMode::Shutdown,
            cycle: ConversionCycle::Ms15_5,
            averaging: Averaging::None,
            ..Configuration::default()
        };
        assert_eq!(shutdown.bits(), 0b0000_0100_0000_0000);
    }

    #[test]
    fn mode_code_10_reads_back_as_continuous() {
        // Table 7-6, MOD[1:0]: 10 is continuous conversion, same as 00.
        assert_eq!(ConversionMode::from_code(0b10), ConversionMode::Continuous);
        assert_eq!(ConversionMode::from_code(0b00), ConversionMode::Continuous);
        assert_eq!(ConversionMode::from_code(0b01), ConversionMode::Shutdown);
        assert_eq!(ConversionMode::from_code(0b11), ConversionMode::OneShot);
        assert_eq!(
            Configuration::from_bits(0b10 << 10).mode,
            ConversionMode::Continuous
        );
    }

    #[test]
    fn flag_readers_pick_the_status_bits() {
        // Table 7-6: HIGH_Alert bit 15, LOW_Alert bit 14, Data_Ready bit 13,
        // EEPROM_Busy bit 12.
        assert!(high_alert(0x8000));
        assert!(low_alert(0x4000));
        assert!(data_ready(0x2000));
        assert!(eeprom_busy(0x1000));
        assert!(!high_alert(0x7FFF));
        assert!(!low_alert(0xBFFF));
        assert!(!data_ready(0xDFFF));
        assert!(!eeprom_busy(0xEFFF));
        let read = Configuration::from_bits(0xA220);
        assert!(read.high_alert && read.data_ready && !read.low_alert);
        // Table 7-10: EUN bit 15, EEPROM_Busy bit 14 of the unlock register.
        assert_eq!(EEPROM_UNLOCK, 1 << 15);
        assert!(eeprom_unlock_busy(0x4000));
        assert!(!eeprom_unlock_busy(EEPROM_UNLOCK));
    }

    #[test]
    fn averaging_counts_match_the_datasheet() {
        // Table 7-6, AVG[1:0]: 00 none, 01 8, 10 32, 11 64.
        assert_eq!(Averaging::None.conversions(), 1);
        assert_eq!(Averaging::X8.conversions(), 8);
        assert_eq!(Averaging::X32.conversions(), 32);
        assert_eq!(Averaging::X64.conversions(), 64);
        for code in 0..4 {
            assert_eq!(Averaging::from_code(code).code(), code);
        }
    }

    #[test]
    fn conversion_cycle_matches_every_cell_of_table_7_7() {
        // Table 7-7, Conversion Cycle Time in CC Mode, rows CONV 000..111 and columns
        // AVG 00, 01, 10, 11, in microseconds.
        const TABLE_7_7: [[u32; 4]; 8] = [
            [15_500, 125_000, 500_000, 1_000_000],
            [125_000, 125_000, 500_000, 1_000_000],
            [250_000, 250_000, 500_000, 1_000_000],
            [500_000, 500_000, 500_000, 1_000_000],
            [1_000_000, 1_000_000, 1_000_000, 1_000_000],
            [4_000_000, 4_000_000, 4_000_000, 4_000_000],
            [8_000_000, 8_000_000, 8_000_000, 8_000_000],
            [16_000_000, 16_000_000, 16_000_000, 16_000_000],
        ];
        for (conv, row) in TABLE_7_7.iter().enumerate() {
            let cycle = ConversionCycle::from_code(conv as u8);
            assert_eq!(cycle.code(), conv as u8);
            assert_eq!(cycle.nominal_micros(), row[0]);
            for (avg, &micros) in row.iter().enumerate() {
                let averaging = Averaging::from_code(avg as u8);
                assert_eq!(
                    cycle.cycle_micros(averaging),
                    micros,
                    "CONV {conv:03b} AVG {avg:02b}"
                );
            }
        }
        // Section 6.5: one-shot conversion time 15.5 ms typical.
        assert_eq!(Averaging::None.conversion_micros(), 15_500);
    }

    #[test]
    fn general_call_reset_is_the_datasheet_command_byte() {
        // Section 7.5.3.1.6: a second byte of 0000 0110 after the general-call address.
        assert_eq!(GENERAL_CALL_RESET, 0b0000_0110);
    }
}
