//! Texas Instruments INA226 high-side or low-side current, voltage, and power monitor.
//!
//! The INA226 measures the voltage across a shunt resistor and the bus voltage on
//! common-mode voltages from 0 V to 36 V and, once its calibration register is
//! programmed, computes current and power on the chip. It adds programmable
//! conversion times and averaging, and an alert pin driven by a single limit
//! register. This module builds the calibration and configuration values and decodes
//! each register into a physical quantity, following the datasheet's equations and
//! its worked example, so a battery-balancer or rack-power node reads amps and watts
//! directly.
//!
//! Shunt voltages are returned in nanovolts, bus voltages in microvolts, and current
//! and power in microamps and microwatts, so the 2.5 µV and 1.25 mV register LSBs
//! convert exactly without floating point. `f32` conveniences sit beside them.

use crate::SensorError;

#[cfg(feature = "embedded-hal")]
mod driver;

#[cfg(feature = "embedded-hal")]
pub use driver::{Ina226, STATUS_POLLS};

/// The INA226 register pointer addresses.
pub mod register {
    /// Configuration register: reset, averaging, conversion times, and operating mode.
    pub const CONFIGURATION: u8 = 0x00;
    /// Shunt voltage register, signed, 2.5 µV per count.
    pub const SHUNT_VOLTAGE: u8 = 0x01;
    /// Bus voltage register, positive only, 1.25 mV per count.
    pub const BUS_VOLTAGE: u8 = 0x02;
    /// Power register, scaled by the calibration register.
    pub const POWER: u8 = 0x03;
    /// Current register, signed, scaled by the calibration register.
    pub const CURRENT: u8 = 0x04;
    /// Calibration register, sets the current and power scale.
    pub const CALIBRATION: u8 = 0x05;
    /// Mask/Enable register: alert function selection and status flags.
    pub const MASK_ENABLE: u8 = 0x06;
    /// Alert limit register, compared against the selected alert function.
    pub const ALERT_LIMIT: u8 = 0x07;
    /// Manufacturer ID register, reads [`MANUFACTURER_ID`](super::MANUFACTURER_ID).
    pub const MANUFACTURER_ID: u8 = 0xFE;
    /// Die ID register: the device ID in bits 15:4 and the die revision in bits 3:0.
    pub const DIE_ID: u8 = 0xFF;
}

/// The value the manufacturer ID register always reads (0x5449, "TI").
pub const MANUFACTURER_ID: u16 = 0x5449;

/// The 12-bit device ID carried in bits 15:4 of the die ID register.
pub const DEVICE_ID: u16 = 0x226;

/// The power-on value of the configuration register (0x4127): averaging off, 1.1 ms
/// conversion time for both bus and shunt, and continuous shunt-and-bus conversion.
pub const CONFIG_RESET: u16 = 0x4127;

/// The shunt voltage register LSB, 2.5 µV, in nanovolts.
pub const SHUNT_LSB_NANOVOLTS: i32 = 2_500;

/// The bus voltage register LSB, 1.25 mV, in microvolts.
pub const BUS_LSB_MICROVOLTS: u32 = 1_250;

/// The fixed ratio between the power LSB and the programmed current LSB.
pub const POWER_LSB_RATIO: u32 = 25;

/// The base I2C address, selected when both address pins are tied to ground.
pub const BASE_ADDRESS: u8 = 0x40;

/// What an address pin is tied to; each of A1 and A0 takes one of four levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AddressPin {
    /// Tied to GND.
    Ground = 0,
    /// Tied to VS.
    Supply = 1,
    /// Tied to SDA.
    Sda = 2,
    /// Tied to SCL.
    Scl = 3,
}

/// Returns the 7-bit I2C address selected by the A1 and A0 pins.
///
/// The address is `1 0 0 A1 A0` where each pin contributes two bits in the order
/// GND, VS, SDA, SCL, giving the sixteen addresses 0x40 through 0x4F.
///
/// # Arguments
///
/// * `a1` - what the A1 pin is tied to.
/// * `a0` - what the A0 pin is tied to.
///
/// # Returns
///
/// The 7-bit target address, before the read/write bit is appended.
pub fn address(a1: AddressPin, a0: AddressPin) -> u8 {
    BASE_ADDRESS | ((a1 as u8) << 2) | a0 as u8
}

/// The number of samples the ADC averages before updating the result registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Averaging {
    /// No averaging, every conversion is reported.
    Samples1 = 0,
    /// Average of 4 samples.
    Samples4 = 1,
    /// Average of 16 samples.
    Samples16 = 2,
    /// Average of 64 samples.
    Samples64 = 3,
    /// Average of 128 samples.
    Samples128 = 4,
    /// Average of 256 samples.
    Samples256 = 5,
    /// Average of 512 samples.
    Samples512 = 6,
    /// Average of 1024 samples.
    Samples1024 = 7,
}

impl Averaging {
    fn from_bits(bits: u16) -> Self {
        match bits & 0x07 {
            0 => Self::Samples1,
            1 => Self::Samples4,
            2 => Self::Samples16,
            3 => Self::Samples64,
            4 => Self::Samples128,
            5 => Self::Samples256,
            6 => Self::Samples512,
            _ => Self::Samples1024,
        }
    }

    /// Returns the number of samples this setting averages.
    ///
    /// # Returns
    ///
    /// The sample count, from 1 to 1024.
    pub fn samples(self) -> u16 {
        match self {
            Self::Samples1 => 1,
            Self::Samples4 => 4,
            Self::Samples16 => 16,
            Self::Samples64 => 64,
            Self::Samples128 => 128,
            Self::Samples256 => 256,
            Self::Samples512 => 512,
            Self::Samples1024 => 1024,
        }
    }
}

/// The ADC conversion time for one shunt or bus voltage measurement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ConversionTime {
    /// 140 µs.
    Us140 = 0,
    /// 204 µs.
    Us204 = 1,
    /// 332 µs.
    Us332 = 2,
    /// 588 µs.
    Us588 = 3,
    /// 1.1 ms.
    Us1100 = 4,
    /// 2.116 ms.
    Us2116 = 5,
    /// 4.156 ms.
    Us4156 = 6,
    /// 8.244 ms.
    Us8244 = 7,
}

impl ConversionTime {
    fn from_bits(bits: u16) -> Self {
        match bits & 0x07 {
            0 => Self::Us140,
            1 => Self::Us204,
            2 => Self::Us332,
            3 => Self::Us588,
            4 => Self::Us1100,
            5 => Self::Us2116,
            6 => Self::Us4156,
            _ => Self::Us8244,
        }
    }

    /// Returns the typical conversion time this setting selects.
    ///
    /// # Returns
    ///
    /// The conversion time in microseconds.
    pub fn microseconds(self) -> u32 {
        match self {
            Self::Us140 => 140,
            Self::Us204 => 204,
            Self::Us332 => 332,
            Self::Us588 => 588,
            Self::Us1100 => 1_100,
            Self::Us2116 => 2_116,
            Self::Us4156 => 4_156,
            Self::Us8244 => 8_244,
        }
    }
}

/// The operating mode: which inputs are converted, and whether once or continuously.
///
/// Both `000` and `100` select power-down; this type reads either as
/// [`Mode::PowerDown`] and writes it back as `000`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    /// Power-down (shutdown); registers stay readable and writable.
    PowerDown = 0,
    /// One shunt voltage conversion, then idle.
    ShuntTriggered = 1,
    /// One bus voltage conversion, then idle.
    BusTriggered = 2,
    /// One shunt and one bus voltage conversion, then idle.
    ShuntAndBusTriggered = 3,
    /// Shunt voltage conversions back to back.
    ShuntContinuous = 5,
    /// Bus voltage conversions back to back.
    BusContinuous = 6,
    /// Shunt and bus voltage conversions back to back, the power-on mode.
    ShuntAndBusContinuous = 7,
}

impl Mode {
    fn from_bits(bits: u16) -> Self {
        match bits & 0x07 {
            1 => Self::ShuntTriggered,
            2 => Self::BusTriggered,
            3 => Self::ShuntAndBusTriggered,
            5 => Self::ShuntContinuous,
            6 => Self::BusContinuous,
            7 => Self::ShuntAndBusContinuous,
            _ => Self::PowerDown,
        }
    }

    /// Returns whether this mode converts the shunt voltage.
    ///
    /// # Returns
    ///
    /// `true` for the shunt-only and shunt-and-bus modes.
    pub fn measures_shunt(self) -> bool {
        self as u8 & 0x01 != 0
    }

    /// Returns whether this mode converts the bus voltage.
    ///
    /// # Returns
    ///
    /// `true` for the bus-only and shunt-and-bus modes.
    pub fn measures_bus(self) -> bool {
        self as u8 & 0x02 != 0
    }

    /// Returns whether this mode keeps converting after the first result.
    ///
    /// # Returns
    ///
    /// `true` for the three continuous modes.
    pub fn is_continuous(self) -> bool {
        matches!(
            self,
            Self::ShuntContinuous | Self::BusContinuous | Self::ShuntAndBusContinuous
        )
    }
}

/// The configuration register (0x00), decoded into its fields.
///
/// Bit 14 is reserved and reads as `1` after reset; [`Configuration::to_register`]
/// writes it that way so the reset value round-trips unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Configuration {
    /// Set to trigger a system reset equivalent to power-on; the bit self-clears.
    pub reset: bool,
    /// How many samples are averaged before the result registers update.
    pub averaging: Averaging,
    /// The conversion time for each bus voltage measurement.
    pub bus_conversion_time: ConversionTime,
    /// The conversion time for each shunt voltage measurement.
    pub shunt_conversion_time: ConversionTime,
    /// Which inputs are converted, and whether once or continuously.
    pub mode: Mode,
}

impl Configuration {
    /// The power-on configuration, the decoded form of [`CONFIG_RESET`].
    pub const RESET: Self = Self {
        reset: false,
        averaging: Averaging::Samples1,
        bus_conversion_time: ConversionTime::Us1100,
        shunt_conversion_time: ConversionTime::Us1100,
        mode: Mode::ShuntAndBusContinuous,
    };

    /// Decodes a configuration register value.
    ///
    /// # Arguments
    ///
    /// * `raw` - the 16-bit configuration register.
    ///
    /// # Returns
    ///
    /// The decoded fields; reserved bits are ignored.
    pub fn from_register(raw: u16) -> Self {
        Self {
            reset: raw & 0x8000 != 0,
            averaging: Averaging::from_bits(raw >> 9),
            bus_conversion_time: ConversionTime::from_bits(raw >> 6),
            shunt_conversion_time: ConversionTime::from_bits(raw >> 3),
            mode: Mode::from_bits(raw),
        }
    }

    /// Encodes the fields as a configuration register value.
    ///
    /// # Returns
    ///
    /// The 16-bit register to write, with reserved bit 14 set as at reset.
    pub fn to_register(self) -> u16 {
        (if self.reset { 0x8000 } else { 0 })
            | 0x4000
            | ((self.averaging as u16) << 9)
            | ((self.bus_conversion_time as u16) << 6)
            | ((self.shunt_conversion_time as u16) << 3)
            | self.mode as u16
    }

    /// Returns how often the result registers update with these settings.
    ///
    /// One update takes the averaged sample count times the sum of the conversion
    /// times of the inputs the mode measures, as the datasheet's timing examples
    /// work it.
    ///
    /// # Returns
    ///
    /// The update interval in microseconds, or `0` in power-down.
    pub fn update_microseconds(self) -> u32 {
        let shunt = if self.mode.measures_shunt() {
            self.shunt_conversion_time.microseconds()
        } else {
            0
        };
        let bus = if self.mode.measures_bus() {
            self.bus_conversion_time.microseconds()
        } else {
            0
        };
        u32::from(self.averaging.samples()) * (shunt + bus)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self::RESET
    }
}

/// The five limit comparisons the alert pin can be assigned to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertFunction {
    /// Shunt voltage above the alert limit (SOL, bit 15).
    ShuntOverLimit,
    /// Shunt voltage below the alert limit (SUL, bit 14).
    ShuntUnderLimit,
    /// Bus voltage above the alert limit (BOL, bit 13).
    BusOverLimit,
    /// Bus voltage below the alert limit (BUL, bit 12).
    BusUnderLimit,
    /// Power above the alert limit (POL, bit 11).
    PowerOverLimit,
}

/// The Mask/Enable register (0x06): which alert function drives the pin, how the
/// pin behaves, and the status flags the chip reports back.
///
/// The alert limit register is compared in the units of the selected function, so
/// build it with [`shunt_register`], [`bus_register`], or [`power_register`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MaskEnable {
    /// Assert the alert pin when the shunt voltage exceeds the limit (SOL).
    pub shunt_over_limit: bool,
    /// Assert the alert pin when the shunt voltage drops below the limit (SUL).
    pub shunt_under_limit: bool,
    /// Assert the alert pin when the bus voltage exceeds the limit (BOL).
    pub bus_over_limit: bool,
    /// Assert the alert pin when the bus voltage drops below the limit (BUL).
    pub bus_under_limit: bool,
    /// Assert the alert pin when the power exceeds the limit (POL).
    pub power_over_limit: bool,
    /// Also assert the alert pin when a conversion completes (CNVR).
    pub conversion_ready: bool,
    /// Status: the selected alert function, not conversion-ready, caused the last
    /// alert (AFF).
    pub alert_function_flag: bool,
    /// Status: all conversions, averaging, and multiplications are complete (CVRF).
    pub conversion_ready_flag: bool,
    /// Status: an arithmetic overflow occurred and current and power may be invalid
    /// (OVF).
    pub math_overflow: bool,
    /// Alert pin polarity: `true` for inverted (active-high), `false` for the default
    /// active-low (APOL).
    pub alert_active_high: bool,
    /// Latch the alert pin and flag until this register is read, rather than clearing
    /// when the fault clears (LEN).
    pub alert_latch: bool,
}

impl MaskEnable {
    /// Decodes a Mask/Enable register value.
    ///
    /// # Arguments
    ///
    /// * `raw` - the 16-bit Mask/Enable register.
    ///
    /// # Returns
    ///
    /// The decoded enables and flags; reserved bits are ignored.
    pub fn from_register(raw: u16) -> Self {
        Self {
            shunt_over_limit: raw & 0x8000 != 0,
            shunt_under_limit: raw & 0x4000 != 0,
            bus_over_limit: raw & 0x2000 != 0,
            bus_under_limit: raw & 0x1000 != 0,
            power_over_limit: raw & 0x0800 != 0,
            conversion_ready: raw & 0x0400 != 0,
            alert_function_flag: raw & 0x0010 != 0,
            conversion_ready_flag: raw & 0x0008 != 0,
            math_overflow: raw & 0x0004 != 0,
            alert_active_high: raw & 0x0002 != 0,
            alert_latch: raw & 0x0001 != 0,
        }
    }

    /// Encodes the enables and flags as a Mask/Enable register value.
    ///
    /// # Returns
    ///
    /// The 16-bit register to write.
    pub fn to_register(self) -> u16 {
        let mut raw = 0;
        for (set, bit) in [
            (self.shunt_over_limit, 0x8000),
            (self.shunt_under_limit, 0x4000),
            (self.bus_over_limit, 0x2000),
            (self.bus_under_limit, 0x1000),
            (self.power_over_limit, 0x0800),
            (self.conversion_ready, 0x0400),
            (self.alert_function_flag, 0x0010),
            (self.conversion_ready_flag, 0x0008),
            (self.math_overflow, 0x0004),
            (self.alert_active_high, 0x0002),
            (self.alert_latch, 0x0001),
        ] {
            if set {
                raw |= bit;
            }
        }
        raw
    }

    /// Returns the alert function the pin actually responds to.
    ///
    /// Only one limit function can drive the pin at a time; when several are
    /// enabled the chip honors the most significant bit.
    ///
    /// # Returns
    ///
    /// The highest-priority enabled function, or `None` if no limit function is
    /// enabled.
    pub fn active_alert_function(self) -> Option<AlertFunction> {
        if self.shunt_over_limit {
            Some(AlertFunction::ShuntOverLimit)
        } else if self.shunt_under_limit {
            Some(AlertFunction::ShuntUnderLimit)
        } else if self.bus_over_limit {
            Some(AlertFunction::BusOverLimit)
        } else if self.bus_under_limit {
            Some(AlertFunction::BusUnderLimit)
        } else if self.power_over_limit {
            Some(AlertFunction::PowerOverLimit)
        } else {
            None
        }
    }
}

/// The die ID register (0xFF), split into its device and revision fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DieId {
    /// The 12-bit device identifier, [`DEVICE_ID`] for an INA226.
    pub device: u16,
    /// The 4-bit die revision.
    pub revision: u8,
}

impl DieId {
    /// Splits a die ID register value into its fields.
    ///
    /// # Arguments
    ///
    /// * `raw` - the 16-bit die ID register.
    ///
    /// # Returns
    ///
    /// The device ID from bits 15:4 and the revision from bits 3:0.
    pub fn from_register(raw: u16) -> Self {
        Self {
            device: raw >> 4,
            revision: (raw & 0x0F) as u8,
        }
    }
}

/// Checks that the identification registers belong to an INA226.
///
/// A node reads both registers at start-up so a wrong part on the address, or a bus
/// that reads all ones, is caught before its readings are trusted.
///
/// # Arguments
///
/// * `manufacturer_id` - the manufacturer ID register (0xFE).
/// * `die_id` - the die ID register (0xFF).
///
/// # Returns
///
/// The decoded die ID when both registers match.
///
/// # Errors
///
/// [`SensorError::Identity`] if the manufacturer ID is not [`MANUFACTURER_ID`] or
/// the device field of the die ID is not [`DEVICE_ID`].
pub fn identify(manufacturer_id: u16, die_id: u16) -> Result<DieId, SensorError> {
    let die = DieId::from_register(die_id);
    if manufacturer_id != MANUFACTURER_ID || die.device != DEVICE_ID {
        return Err(SensorError::Identity);
    }
    Ok(die)
}

/// Computes the calibration register value for a chosen current resolution and shunt.
///
/// This is the datasheet's calibration equation, `CAL = 0.00512 / (Current_LSB *
/// R_SHUNT)`, expressed in integer micro-units: with the current LSB in microamps
/// and the shunt in milliohms, the fixed `0.00512` becomes `5_120_000`.
///
/// # Arguments
///
/// * `current_lsb_microamps` - the amps-per-count the current register should carry.
/// * `shunt_milliohms` - the shunt resistor value, in milliohms.
///
/// # Returns
///
/// The value to program into the calibration register, truncated as the datasheet
/// does and capped at the register's 15-bit width. Returns `0` if either argument
/// is `0`, which is the chip's own uncalibrated state.
pub fn calibration(current_lsb_microamps: u32, shunt_milliohms: u32) -> u16 {
    let denominator = current_lsb_microamps.saturating_mul(shunt_milliohms);
    if denominator == 0 {
        return 0;
    }
    (5_120_000 / denominator).min(0x7FFF) as u16
}

/// Returns the smallest current LSB, in microamps, that still spans a full-scale
/// current.
///
/// The current register is 15 bits of magnitude, so the minimum resolution is the
/// maximum expected current divided by 2^15, rounded up to the next whole microamp.
/// The datasheet then rounds this up further to a convenient round number.
///
/// # Arguments
///
/// * `max_expected_microamps` - the largest current the application will measure.
///
/// # Returns
///
/// The minimum current LSB in microamps.
pub fn minimum_current_lsb_microamps(max_expected_microamps: u32) -> u32 {
    max_expected_microamps.div_ceil(32_768)
}

/// Decodes the shunt voltage register to nanovolts.
///
/// # Arguments
///
/// * `raw` - the signed shunt voltage register.
///
/// # Returns
///
/// The shunt voltage in nanovolts, at 2.5 µV per count.
pub fn shunt_nanovolts(raw: i16) -> i32 {
    i32::from(raw) * SHUNT_LSB_NANOVOLTS
}

/// Decodes the shunt voltage register to millivolts as a float.
///
/// # Arguments
///
/// * `raw` - the signed shunt voltage register.
///
/// # Returns
///
/// The shunt voltage in millivolts.
pub fn shunt_millivolts_f32(raw: i16) -> f32 {
    f32::from(raw) * 0.0025
}

/// Decodes the bus voltage register to microvolts.
///
/// Bit 15 is always zero on the chip, since the bus voltage is positive only, and
/// is ignored here.
///
/// # Arguments
///
/// * `raw` - the bus voltage register.
///
/// # Returns
///
/// The bus voltage in microvolts, at 1.25 mV per count.
pub fn bus_microvolts(raw: u16) -> u32 {
    u32::from(raw & 0x7FFF) * BUS_LSB_MICROVOLTS
}

/// Decodes the bus voltage register to volts as a float.
///
/// # Arguments
///
/// * `raw` - the bus voltage register.
///
/// # Returns
///
/// The bus voltage in volts.
pub fn bus_volts_f32(raw: u16) -> f32 {
    f32::from(raw & 0x7FFF) * 0.00125
}

/// Decodes the current register to microamps for a given current LSB.
///
/// # Arguments
///
/// * `raw` - the signed current register.
/// * `current_lsb_microamps` - the current LSB the calibration register was set for.
///
/// # Returns
///
/// The current in microamps.
pub fn current_microamps(raw: i16, current_lsb_microamps: u32) -> i32 {
    i32::from(raw) * current_lsb_microamps as i32
}

/// Decodes the current register to amps as a float.
///
/// # Arguments
///
/// * `raw` - the signed current register.
/// * `current_lsb_microamps` - the current LSB the calibration register was set for.
///
/// # Returns
///
/// The current in amps.
pub fn current_amps_f32(raw: i16, current_lsb_microamps: u32) -> f32 {
    current_microamps(raw, current_lsb_microamps) as f32 * 1e-6
}

/// Decodes the power register to microwatts for a given current LSB.
///
/// The power LSB is fixed by the datasheet at twenty-five times the current LSB.
///
/// # Arguments
///
/// * `raw` - the power register.
/// * `current_lsb_microamps` - the current LSB the calibration register was set for.
///
/// # Returns
///
/// The power in microwatts.
pub fn power_microwatts(raw: u16, current_lsb_microamps: u32) -> u32 {
    u32::from(raw) * (POWER_LSB_RATIO * current_lsb_microamps)
}

/// Decodes the power register to watts as a float.
///
/// # Arguments
///
/// * `raw` - the power register.
/// * `current_lsb_microamps` - the current LSB the calibration register was set for.
///
/// # Returns
///
/// The power in watts.
pub fn power_watts_f32(raw: u16, current_lsb_microamps: u32) -> f32 {
    power_microwatts(raw, current_lsb_microamps) as f32 * 1e-6
}

/// Builds the shunt voltage register a monitor reports for a shunt voltage.
///
/// The inverse of [`shunt_nanovolts`], so a node can be written and tested against
/// what a monitor sends without one attached. The same value serves as the alert
/// limit for the shunt over- and under-limit functions.
///
/// # Arguments
///
/// * `nanovolts` - the shunt voltage in nanovolts.
///
/// # Returns
///
/// The signed shunt voltage register, at 2.5 µV per count.
pub fn shunt_register(nanovolts: i32) -> i16 {
    (nanovolts / SHUNT_LSB_NANOVOLTS) as i16
}

/// Builds the bus voltage register a monitor reports for a bus voltage.
///
/// The inverse of [`bus_microvolts`]. The same value serves as the alert limit for
/// the bus over- and under-limit functions.
///
/// # Arguments
///
/// * `microvolts` - the bus voltage in microvolts.
///
/// # Returns
///
/// The bus voltage register, at 1.25 mV per count.
pub fn bus_register(microvolts: u32) -> u16 {
    (microvolts / BUS_LSB_MICROVOLTS) as u16
}

/// Builds the current register a monitor reports for a current.
///
/// The inverse of [`current_microamps`].
///
/// # Arguments
///
/// * `microamps` - the current in microamps.
/// * `current_lsb_microamps` - the current LSB the calibration register was set for.
///
/// # Returns
///
/// The signed current register, or zero if `current_lsb_microamps` is zero.
pub fn current_register(microamps: i32, current_lsb_microamps: u32) -> i16 {
    if current_lsb_microamps == 0 {
        return 0;
    }
    (microamps / current_lsb_microamps as i32) as i16
}

/// Builds the power register a monitor reports for a power.
///
/// The inverse of [`power_microwatts`]. The same value serves as the alert limit for
/// the power over-limit function.
///
/// # Arguments
///
/// * `microwatts` - the power in microwatts.
/// * `current_lsb_microamps` - the current LSB the calibration register was set for.
///
/// # Returns
///
/// The power register, or zero if `current_lsb_microamps` is zero.
pub fn power_register(microwatts: u32, current_lsb_microamps: u32) -> u16 {
    if current_lsb_microamps == 0 {
        return 0;
    }
    (microwatts / (POWER_LSB_RATIO * current_lsb_microamps)) as u16
}

/// Reproduces the chip's current calculation from a shunt reading and calibration.
///
/// This is the datasheet's `Current = ShuntVoltage * CalibrationRegister / 2048`,
/// so a simulator can fill the current register the way the chip would.
///
/// # Arguments
///
/// * `shunt` - the signed shunt voltage register.
/// * `calibration` - the programmed calibration register.
///
/// # Returns
///
/// The signed current register the chip would hold.
pub fn current_register_from_shunt(shunt: i16, calibration: u16) -> i16 {
    (i32::from(shunt) * i32::from(calibration) / 2048) as i16
}

/// Reproduces the chip's power calculation from the current and bus registers.
///
/// This is the datasheet's `Power = Current * BusVoltage / 20000`. The power
/// register is unsigned, so the magnitude of the current is used.
///
/// # Arguments
///
/// * `current` - the signed current register.
/// * `bus` - the bus voltage register.
///
/// # Returns
///
/// The power register the chip would hold.
pub fn power_register_from_current(current: i16, bus: u16) -> u16 {
    (u32::from(current.unsigned_abs()) * u32::from(bus & 0x7FFF) / 20_000) as u16
}

/// One set of results, with the current resolution they were taken at.
///
/// The four data registers are kept as read; the methods apply the datasheet's
/// scaling. The overflow flag is the one the Mask/Enable register carried when the
/// conversion finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    /// The Shunt Voltage register.
    pub shunt: i16,
    /// The Bus Voltage register.
    pub bus: u16,
    /// The Current register.
    pub current: i16,
    /// The Power register.
    pub power: u16,
    /// The programmed current resolution, in microamps per count.
    pub current_lsb_microamps: u32,
    /// The math overflow flag: the current and power may be invalid.
    pub math_overflow: bool,
}

impl Reading {
    /// Returns the shunt voltage in nanovolts.
    pub fn shunt_nanovolts(&self) -> i32 {
        shunt_nanovolts(self.shunt)
    }

    /// Returns the shunt voltage in millivolts.
    pub fn shunt_millivolts(&self) -> f32 {
        shunt_millivolts_f32(self.shunt)
    }

    /// Returns the bus voltage in microvolts.
    pub fn bus_microvolts(&self) -> u32 {
        bus_microvolts(self.bus)
    }

    /// Returns the bus voltage in volts.
    pub fn bus_volts(&self) -> f32 {
        bus_volts_f32(self.bus)
    }

    /// Returns the current in microamps.
    pub fn current_microamps(&self) -> i32 {
        current_microamps(self.current, self.current_lsb_microamps)
    }

    /// Returns the current in amps.
    pub fn current_amps(&self) -> f32 {
        current_amps_f32(self.current, self.current_lsb_microamps)
    }

    /// Returns the power in microwatts.
    pub fn power_microwatts(&self) -> u32 {
        power_microwatts(self.power, self.current_lsb_microamps)
    }

    /// Returns the power in watts.
    pub fn power_watts(&self) -> f32 {
        power_watts_f32(self.power, self.current_lsb_microamps)
    }
}

#[cfg(test)]
mod driver_support_tests {
    use super::*;

    #[test]
    fn a_reading_scales_its_registers_like_the_free_functions() {
        let reading = Reading {
            shunt: shunt_register(50_000_000),
            bus: bus_register(12_000_000),
            current: current_register(1_000_000, 100),
            power: power_register(12_000_000, 100),
            current_lsb_microamps: 100,
            math_overflow: false,
        };
        assert_eq!(reading.shunt_nanovolts(), 50_000_000);
        assert!((reading.shunt_millivolts() - 50.0).abs() < 1e-3);
        assert_eq!(reading.bus_microvolts(), 12_000_000);
        assert!((reading.bus_volts() - 12.0).abs() < 1e-3);
        assert_eq!(reading.current_microamps(), 1_000_000);
        assert!((reading.current_amps() - 1.0).abs() < 1e-3);
        assert_eq!(reading.power_microwatts(), 12_000_000);
        assert!((reading.power_watts() - 12.0).abs() < 1e-3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The datasheet's worked example (section 6.5.1, Table 6-1): a 10 A load across
    // a 2 mΩ shunt, 12 V common mode, current LSB rounded to 1 mA/bit.
    const CURRENT_LSB: u32 = 1_000; // 1 mA in microamps

    #[test]
    fn every_register_survives_a_round_trip_through_its_builder() {
        // Table 6-1: 20 mV shunt, 11.98 V bus, 10 A, 119.8 W.
        assert_eq!(shunt_register(20_000_000), 0x1F40);
        assert_eq!(shunt_nanovolts(shunt_register(20_000_000)), 20_000_000);
        assert_eq!(shunt_nanovolts(shunt_register(-80_000_000)), -80_000_000);
        assert_eq!(bus_register(11_980_000), 0x2570);
        assert_eq!(bus_microvolts(bus_register(11_980_000)), 11_980_000);
        assert_eq!(current_register(10_000_000, CURRENT_LSB), 0x2710);
        assert_eq!(
            current_microamps(current_register(10_000_000, CURRENT_LSB), CURRENT_LSB),
            10_000_000
        );
        assert_eq!(current_register(-2_500_000, CURRENT_LSB), -2_500);
        assert_eq!(power_register(119_800_000, CURRENT_LSB), 0x12B8);
        assert_eq!(
            power_microwatts(power_register(119_800_000, CURRENT_LSB), CURRENT_LSB),
            119_800_000
        );
        assert_eq!(current_register(1, 0), 0);
        assert_eq!(power_register(1, 0), 0);
    }

    #[test]
    fn calibration_matches_the_datasheet_example() {
        // Equation 1 with Current_LSB = 1 mA/bit and R_SHUNT = 2 mΩ: 2560 = A00h.
        assert_eq!(calibration(CURRENT_LSB, 2), 2_560);
        assert_eq!(calibration(CURRENT_LSB, 2), 0xA00);
    }

    #[test]
    fn calibration_is_capped_at_the_register_width() {
        // Table 7-11: the calibration value occupies FS14..FS0.
        assert_eq!(calibration(1, 1), 0x7FFF);
    }

    #[test]
    fn minimum_current_lsb_matches_the_datasheet_example() {
        // Equation 2, section 6.5.1: 15 A / 2^15 = 457.7 µA/bit, up to the next
        // whole microamp.
        assert_eq!(minimum_current_lsb_microamps(15_000_000), 458);
    }

    #[test]
    fn shunt_register_decodes_per_the_datasheet() {
        // Table 6-1: 1F40h = 8000 at 2.5 µV per count is 20 mV.
        assert_eq!(shunt_nanovolts(0x1F40), 20_000_000);
        // Section 7.1.2 worked two's complement: -80 mV is 8300h.
        assert_eq!(
            shunt_nanovolts(i16::from_be_bytes([0x83, 0x00])),
            -80_000_000
        );
        // Section 7.1.2: full scale 7FFFh is 81.92 mV (81.9175 mV exactly, per the
        // electrical characteristics input range).
        assert_eq!(shunt_nanovolts(0x7FFF), 81_917_500);
        assert!((shunt_millivolts_f32(0x1F40) - 20.0).abs() < 1e-4);
    }

    #[test]
    fn bus_register_decodes_per_the_datasheet() {
        // Table 6-1: 2570h = 9584 at 1.25 mV per count is 11.98 V.
        assert_eq!(bus_microvolts(0x2570), 11_980_000);
        // Section 7.1.3: full scale 7FFFh is 40.96 V.
        assert_eq!(bus_microvolts(0x7FFF), 40_958_750);
        // Table 7-8: D15 is always zero, so it carries no value if it ever reads set.
        assert_eq!(bus_microvolts(0xA570), bus_microvolts(0x2570));
        assert!((bus_volts_f32(0x2570) - 11.98).abs() < 1e-4);
    }

    #[test]
    fn current_register_decodes_to_the_example_load() {
        // Table 6-1: 2710h = 10000 at 1 mA per count is 10 A.
        assert_eq!(current_microamps(0x2710, CURRENT_LSB), 10_000_000);
        assert!((current_amps_f32(0x2710, CURRENT_LSB) - 10.0).abs() < 1e-5);
    }

    #[test]
    fn power_register_decodes_to_the_example_load() {
        // Table 6-1: 12B8h = 4792 at 25 mW per count (25 times the 1 mA current
        // LSB) is 119.8 W; the table prints the figure as 119.82 W.
        assert_eq!(power_microwatts(0x12B8, CURRENT_LSB), 119_800_000);
        assert!((power_watts_f32(0x12B8, CURRENT_LSB) - 119.8).abs() < 1e-3);
    }

    #[test]
    fn chip_arithmetic_reproduces_the_datasheet_example() {
        // Equation 3: 8000 * 2560 / 2048 = 10000 = 2710h.
        assert_eq!(current_register_from_shunt(0x1F40, 0xA00), 0x2710);
        // Equation 4: 10000 * 9584 / 20000 = 4792 = 12B8h.
        assert_eq!(power_register_from_current(0x2710, 0x2570), 0x12B8);
        // A reversed current gives the same power magnitude.
        assert_eq!(current_register_from_shunt(-0x1F40, 0xA00), -0x2710);
        assert_eq!(power_register_from_current(-0x2710, 0x2570), 0x12B8);
        // Nothing is computed until the calibration register is programmed
        // (Table 7-1, note 2).
        assert_eq!(current_register_from_shunt(0x1F40, 0), 0);
    }

    #[test]
    fn configuration_reset_value_matches_the_datasheet() {
        // Table 7-1 / Table 7-2: power-on reset 4127h, averaging 1, 1.1 ms for both
        // conversions, shunt and bus continuous. Table 6-1 step 1 writes the same.
        let config = Configuration::from_register(CONFIG_RESET);
        assert_eq!(config, Configuration::RESET);
        assert_eq!(config, Configuration::default());
        assert!(!config.reset);
        assert_eq!(config.averaging, Averaging::Samples1);
        assert_eq!(config.bus_conversion_time, ConversionTime::Us1100);
        assert_eq!(config.shunt_conversion_time, ConversionTime::Us1100);
        assert_eq!(config.mode, Mode::ShuntAndBusContinuous);
        assert_eq!(config.to_register(), 0x4127);
    }

    #[test]
    fn configuration_fields_land_on_their_datasheet_bits() {
        // Table 7-2: RST in D15, AVG in D11:9, VBUSCT in D8:6, VSHCT in D5:3, MODE
        // in D2:0.
        let config = Configuration {
            reset: true,
            averaging: Averaging::Samples1024,
            bus_conversion_time: ConversionTime::Us8244,
            shunt_conversion_time: ConversionTime::Us8244,
            mode: Mode::ShuntAndBusContinuous,
        };
        assert_eq!(
            config.to_register(),
            0x8000 | 0x4000 | 0x0E00 | 0x01C0 | 0x0038 | 0x0007
        );
        assert_eq!(Configuration::from_register(config.to_register()), config);

        let config = Configuration {
            reset: false,
            averaging: Averaging::Samples16,
            bus_conversion_time: ConversionTime::Us140,
            shunt_conversion_time: ConversionTime::Us588,
            mode: Mode::ShuntTriggered,
        };
        assert_eq!(config.to_register(), 0x4000 | 0x0400 | 0x0018 | 0x0001);
        assert_eq!(Configuration::from_register(config.to_register()), config);

        // Table 7-6: both 000 and 100 are power-down.
        assert_eq!(Configuration::from_register(0x4120).mode, Mode::PowerDown);
        assert_eq!(Configuration::from_register(0x4124).mode, Mode::PowerDown);
    }

    #[test]
    fn averaging_table_decodes_per_the_datasheet() {
        // Table 7-3.
        let table = [1, 4, 16, 64, 128, 256, 512, 1024];
        for (bits, samples) in table.into_iter().enumerate() {
            let averaging = Averaging::from_bits(bits as u16);
            assert_eq!(averaging as u16, bits as u16);
            assert_eq!(averaging.samples(), samples, "AVG = {bits:03b}");
        }
    }

    #[test]
    fn conversion_time_table_decodes_per_the_datasheet() {
        // Tables 7-4 and 7-5, the same eight settings for bus and shunt.
        let table = [140, 204, 332, 588, 1_100, 2_116, 4_156, 8_244];
        for (bits, microseconds) in table.into_iter().enumerate() {
            let time = ConversionTime::from_bits(bits as u16);
            assert_eq!(time as u16, bits as u16);
            assert_eq!(time.microseconds(), microseconds, "CT = {bits:03b}");
        }
    }

    #[test]
    fn mode_table_decodes_per_the_datasheet() {
        // Table 7-6.
        assert_eq!(Mode::from_bits(0b000), Mode::PowerDown);
        assert_eq!(Mode::from_bits(0b001), Mode::ShuntTriggered);
        assert_eq!(Mode::from_bits(0b010), Mode::BusTriggered);
        assert_eq!(Mode::from_bits(0b011), Mode::ShuntAndBusTriggered);
        assert_eq!(Mode::from_bits(0b100), Mode::PowerDown);
        assert_eq!(Mode::from_bits(0b101), Mode::ShuntContinuous);
        assert_eq!(Mode::from_bits(0b110), Mode::BusContinuous);
        assert_eq!(Mode::from_bits(0b111), Mode::ShuntAndBusContinuous);
        assert!(Mode::ShuntContinuous.measures_shunt());
        assert!(!Mode::ShuntContinuous.measures_bus());
        assert!(Mode::BusTriggered.measures_bus());
        assert!(!Mode::BusTriggered.is_continuous());
        assert!(Mode::ShuntAndBusContinuous.is_continuous());
        assert!(!Mode::PowerDown.measures_shunt());
        assert!(!Mode::PowerDown.measures_bus());
    }

    #[test]
    fn update_interval_matches_the_datasheet_timing_examples() {
        // Section 6.4.1: 588 µs for both conversions averaged over 4 samples updates
        // about every 4.7 ms, as does 4.156 ms shunt with 588 µs bus and no averaging.
        let config = Configuration {
            averaging: Averaging::Samples4,
            bus_conversion_time: ConversionTime::Us588,
            shunt_conversion_time: ConversionTime::Us588,
            ..Configuration::RESET
        };
        assert_eq!(config.update_microseconds(), 4_704);
        let config = Configuration {
            averaging: Averaging::Samples1,
            bus_conversion_time: ConversionTime::Us588,
            shunt_conversion_time: ConversionTime::Us4156,
            ..Configuration::RESET
        };
        assert_eq!(config.update_microseconds(), 4_744);
        assert_eq!(Configuration::RESET.update_microseconds(), 2_200);
        let config = Configuration {
            mode: Mode::ShuntContinuous,
            ..config
        };
        assert_eq!(config.update_microseconds(), 4_156);
        let config = Configuration {
            mode: Mode::PowerDown,
            ..config
        };
        assert_eq!(config.update_microseconds(), 0);
    }

    #[test]
    fn address_table_matches_the_datasheet() {
        // Table 6-2, in its row order GND, VS, SDA, SCL for each pin.
        use AddressPin::*;
        let pins = [Ground, Supply, Sda, Scl];
        let mut expected = 0x40;
        for a1 in pins {
            for a0 in pins {
                assert_eq!(address(a1, a0), expected, "A1 = {a1:?}, A0 = {a0:?}");
                expected += 1;
            }
        }
        assert_eq!(address(Ground, Ground), 0b1000000);
        assert_eq!(address(Supply, Ground), 0b1000100);
        assert_eq!(address(Sda, Scl), 0b1001011);
        assert_eq!(address(Scl, Scl), 0b1001111);
    }

    #[test]
    fn identification_registers_match_the_datasheet() {
        // Table 7-1: manufacturer ID 5449h; die ID 2260h or 2261h (note 3).
        assert_eq!(MANUFACTURER_ID, 0x5449);
        assert_eq!(
            identify(0x5449, 0x2260),
            Ok(DieId {
                device: 0x226,
                revision: 0
            })
        );
        assert_eq!(
            identify(0x5449, 0x2261),
            Ok(DieId {
                device: 0x226,
                revision: 1
            })
        );
        // An unpowered or absent part reads all ones.
        assert_eq!(identify(0xFFFF, 0xFFFF), Err(SensorError::Identity));
        // Another TI part on the same address.
        assert_eq!(identify(0x5449, 0x2280), Err(SensorError::Identity));
        assert_eq!(identify(0x0000, 0x2260), Err(SensorError::Identity));
    }

    #[test]
    fn mask_enable_decodes_the_alert_bits() {
        // Table 7-12: SOL D15, SUL D14, BOL D13, BUL D12, POL D11, CNVR D10, AFF D4,
        // CVRF D3, OVF D2, APOL D1, LEN D0.
        let mask = MaskEnable::from_register(0x8001);
        assert!(mask.shunt_over_limit);
        assert!(mask.alert_latch);
        assert_eq!(
            mask.active_alert_function(),
            Some(AlertFunction::ShuntOverLimit)
        );
        assert_eq!(mask.to_register(), 0x8001);

        let mask = MaskEnable::from_register(0x0418);
        assert!(mask.conversion_ready);
        assert!(mask.alert_function_flag);
        assert!(mask.conversion_ready_flag);
        assert!(!mask.math_overflow);
        assert_eq!(mask.active_alert_function(), None);
        assert_eq!(mask.to_register(), 0x0418);

        assert!(MaskEnable::from_register(0x0004).math_overflow);
        assert!(MaskEnable::from_register(0x0002).alert_active_high);
        assert_eq!(MaskEnable::from_register(0x0000), MaskEnable::default());
        assert_eq!(MaskEnable::default().to_register(), 0x0000);

        for bit in 0..16 {
            let raw = 1u16 << bit;
            let reserved = (5..=9).contains(&bit);
            assert_eq!(
                MaskEnable::from_register(raw).to_register(),
                if reserved { 0 } else { raw },
                "bit {bit}"
            );
        }
    }

    #[test]
    fn the_highest_alert_function_bit_takes_priority() {
        // Section 7.1.7: with several functions enabled, the most significant wins.
        assert_eq!(
            MaskEnable::from_register(0xF800).active_alert_function(),
            Some(AlertFunction::ShuntOverLimit)
        );
        assert_eq!(
            MaskEnable::from_register(0x7800).active_alert_function(),
            Some(AlertFunction::ShuntUnderLimit)
        );
        assert_eq!(
            MaskEnable::from_register(0x3800).active_alert_function(),
            Some(AlertFunction::BusOverLimit)
        );
        assert_eq!(
            MaskEnable::from_register(0x1800).active_alert_function(),
            Some(AlertFunction::BusUnderLimit)
        );
        assert_eq!(
            MaskEnable::from_register(0x0800).active_alert_function(),
            Some(AlertFunction::PowerOverLimit)
        );
        assert_eq!(
            MaskEnable::from_register(0x0400).active_alert_function(),
            None
        );
    }

    #[test]
    fn integer_conversions_track_the_floating_point_reference() {
        // Equation 1 in its floating-point form, CAL = 0.00512 / (Current_LSB *
        // R_SHUNT), swept over current LSBs and shunts a design would choose.
        for &lsb in &[50u32, 100, 250, 458, 500, 1_000, 2_000, 5_000, 10_000] {
            for &shunt in &[1u32, 2, 5, 10, 25, 100, 500, 1_000] {
                let reference = 0.00512 / (f64::from(lsb) * 1e-6 * f64::from(shunt) * 1e-3);
                let expected = reference.trunc().min(32_767.0);
                let actual = f64::from(calibration(lsb, shunt));
                assert!(
                    (actual - expected).abs() <= 1.0,
                    "lsb {lsb} µA, shunt {shunt} mΩ: {actual} vs {expected}"
                );
            }
        }

        // The 2.5 µV, 1.25 mV, and 25 x Current_LSB scalings against their float
        // forms across the register range.
        for raw in (-32_768..=32_767).step_by(997) {
            let shunt = f64::from(shunt_nanovolts(raw as i16)) * 1e-9;
            assert!((shunt - f64::from(raw) * 2.5e-6).abs() < 1e-12);
            let current = f64::from(current_microamps(raw as i16, 458)) * 1e-6;
            assert!((current - f64::from(raw) * 458e-6).abs() < 1e-9);
        }
        for raw in (0..=0x7FFF).step_by(499) {
            let bus = f64::from(bus_microvolts(raw as u16)) * 1e-6;
            assert!((bus - f64::from(raw) * 1.25e-3).abs() < 1e-9);
            let power = f64::from(power_microwatts(raw as u16, 458)) * 1e-6;
            assert!((power - f64::from(raw) * 25.0 * 458e-6).abs() < 1e-9);
        }
    }

    #[test]
    fn an_uncalibrated_request_returns_zero() {
        assert_eq!(calibration(0, 2), 0);
        assert_eq!(calibration(CURRENT_LSB, 0), 0);
    }
}
