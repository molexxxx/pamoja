//! Texas Instruments INA219 high-side current, voltage, and power monitor.
//!
//! The INA219 measures the voltage across a shunt resistor and the bus voltage, and,
//! once its calibration register is programmed, computes current and power on the
//! chip. This module builds the calibration value and decodes each register into a
//! physical quantity, following the datasheet's equations and its worked design
//! example, so a solar-battery or microgrid node reads amps and watts directly.
//!
//! Currents, voltages, and powers are returned in integer micro-units (microvolts,
//! microamps, microwatts) so the conversions stay exact without floating point.

#[cfg(feature = "embedded-hal")]
mod driver;

#[cfg(feature = "embedded-hal")]
pub use driver::{Ina219, STATUS_POLLS};

/// The I2C address with both address pins tied to GND; A1 and A0 select the rest.
pub const BASE_ADDRESS: u8 = 0x40;

/// The INA219 register addresses.
pub mod register {
    /// Configuration register: bus-voltage range, gain, ADC settings, and mode.
    pub const CONFIGURATION: u8 = 0x00;
    /// Shunt voltage register, signed, 10 µV per count.
    pub const SHUNT_VOLTAGE: u8 = 0x01;
    /// Bus voltage register, value in bits 15:3, 4 mV per count.
    pub const BUS_VOLTAGE: u8 = 0x02;
    /// Power register, scaled by the calibration register.
    pub const POWER: u8 = 0x03;
    /// Current register, scaled by the calibration register.
    pub const CURRENT: u8 = 0x04;
    /// Calibration register, sets the current and power scale.
    pub const CALIBRATION: u8 = 0x05;
}

/// The power-on value of the configuration register (0x399F): 32 V bus range, gain
/// /8, 12-bit ADCs, and continuous shunt-and-bus conversion.
pub const CONFIG_RESET: u16 = 0x399F;

/// Computes the calibration register value for a chosen current resolution and shunt.
///
/// This is the datasheet's calibration equation, `Cal = trunc(0.04096 / (Current_LSB
/// * R_shunt))`, expressed in integer micro-units: with the current LSB in microamps
/// and the shunt in milliohms, the fixed `0.04096` becomes `40_960_000`.
///
/// # Arguments
///
/// * `current_lsb_microamps` - the amps-per-count the current register should carry.
/// * `shunt_milliohms` - the shunt resistor value, in milliohms.
///
/// # Returns
///
/// The 16-bit value to program into the calibration register. Returns `0` if either
/// argument is `0`, which is the chip's own uncalibrated state.
pub fn calibration(current_lsb_microamps: u32, shunt_milliohms: u32) -> u16 {
    let denominator = current_lsb_microamps.saturating_mul(shunt_milliohms);
    if denominator == 0 {
        return 0;
    }
    (40_960_000 / denominator) as u16
}

/// Returns the smallest current LSB, in microamps, that still spans a full-scale
/// current.
///
/// The current register is 15 bits of magnitude, so the minimum resolution is the
/// maximum expected current divided by 32768, rounded up to the next whole microamp.
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

/// Decodes the shunt voltage register to microvolts.
///
/// # Arguments
///
/// * `raw` - the signed shunt voltage register.
///
/// # Returns
///
/// The shunt voltage in microvolts, at 10 µV per count.
pub fn shunt_microvolts(raw: i16) -> i32 {
    raw as i32 * 10
}

/// Decodes the bus voltage register to millivolts.
///
/// The voltage occupies bits 15:3, so the register is shifted right by three before
/// scaling by the 4 mV LSB; the low bits are the conversion-ready and overflow flags.
///
/// # Arguments
///
/// * `raw` - the bus voltage register.
///
/// # Returns
///
/// The bus voltage in millivolts.
pub fn bus_millivolts(raw: u16) -> u32 {
    (raw >> 3) as u32 * 4
}

/// Returns whether the bus voltage register's conversion-ready (CNVR) flag is set.
///
/// # Arguments
///
/// * `raw` - the bus voltage register.
///
/// # Returns
///
/// `true` if a conversion has completed and the data is ready to read.
pub fn conversion_ready(raw: u16) -> bool {
    raw & 0x0002 != 0
}

/// Returns whether the bus voltage register's math-overflow (OVF) flag is set.
///
/// # Arguments
///
/// * `raw` - the bus voltage register.
///
/// # Returns
///
/// `true` if the power or current calculation overflowed and the readings are invalid.
pub fn math_overflow(raw: u16) -> bool {
    raw & 0x0001 != 0
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
    raw as i32 * current_lsb_microamps as i32
}

/// Decodes the power register to microwatts for a given current LSB.
///
/// The power LSB is fixed by the datasheet at twenty times the current LSB.
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
    raw as u32 * (20 * current_lsb_microamps)
}

/// Builds the shunt voltage register a monitor reports for a shunt voltage.
///
/// The inverse of [`shunt_microvolts`], so a node can be written and tested against
/// what a monitor sends without one attached.
///
/// # Arguments
///
/// * `microvolts` - the shunt voltage in microvolts.
///
/// # Returns
///
/// The signed shunt voltage register, at 10 µV per count.
pub fn shunt_register(microvolts: i32) -> i16 {
    (microvolts / 10) as i16
}

/// Builds the bus voltage register a monitor reports for a bus voltage.
///
/// The inverse of [`bus_millivolts`], with the conversion-ready flag set and the
/// overflow flag clear, which is what a completed conversion reads as.
///
/// # Arguments
///
/// * `millivolts` - the bus voltage in millivolts.
///
/// # Returns
///
/// The bus voltage register, the voltage in bits 15:3 at 4 mV per count.
pub fn bus_register(millivolts: u32) -> u16 {
    (((millivolts / 4) as u16) << 3) | 0x0002
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
/// The inverse of [`power_microwatts`]; the power LSB is fixed by the datasheet at
/// twenty times the current LSB.
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
    (microwatts / (20 * current_lsb_microamps)) as u16
}

/// The bus voltage full-scale range, the `BRNG` bit of the Configuration register.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BusRange {
    /// 0 to 16 V.
    V16,
    /// 0 to 32 V, the reset setting.
    #[default]
    V32,
}

impl BusRange {
    /// Returns the `BRNG` bit value.
    pub fn code(self) -> u8 {
        match self {
            BusRange::V16 => 0,
            BusRange::V32 => 1,
        }
    }

    /// Decodes the `BRNG` bit value.
    ///
    /// # Arguments
    ///
    /// * `code` - the bit; only the low bit is used.
    ///
    /// # Returns
    ///
    /// The range.
    pub fn from_code(code: u8) -> BusRange {
        if code & 1 == 0 {
            BusRange::V16
        } else {
            BusRange::V32
        }
    }
}

/// The shunt voltage gain and range, the `PG` bits of the Configuration register.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Gain {
    /// Gain 1, ±40 mV.
    Div1,
    /// Gain 1/2, ±80 mV.
    Div2,
    /// Gain 1/4, ±160 mV.
    Div4,
    /// Gain 1/8, ±320 mV, the reset setting.
    #[default]
    Div8,
}

impl Gain {
    /// Returns the two-bit `PG` field value.
    pub fn code(self) -> u8 {
        match self {
            Gain::Div1 => 0b00,
            Gain::Div2 => 0b01,
            Gain::Div4 => 0b10,
            Gain::Div8 => 0b11,
        }
    }

    /// Decodes a two-bit `PG` field value.
    ///
    /// # Arguments
    ///
    /// * `code` - the field value; only the low two bits are used.
    ///
    /// # Returns
    ///
    /// The gain.
    pub fn from_code(code: u8) -> Gain {
        match code & 0b11 {
            0b00 => Gain::Div1,
            0b01 => Gain::Div2,
            0b10 => Gain::Div4,
            _ => Gain::Div8,
        }
    }

    /// Returns the shunt voltage range in millivolts, either side of zero.
    pub fn range_millivolts(self) -> u16 {
        match self {
            Gain::Div1 => 40,
            Gain::Div2 => 80,
            Gain::Div4 => 160,
            Gain::Div8 => 320,
        }
    }
}

/// An ADC resolution or averaging setting, for the `BADC` and `SADC` fields.
///
/// The four-bit code selects either a resolution, when its high bit is clear, or a
/// sample count averaged at 12 bits, when it is set. Each setting has the conversion
/// time the datasheet tabulates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Adc {
    /// 9-bit, 84 us.
    Bits9,
    /// 10-bit, 148 us.
    Bits10,
    /// 11-bit, 276 us.
    Bits11,
    /// 12-bit, 532 us, the reset setting.
    #[default]
    Bits12,
    /// 2 samples averaged, 1.06 ms.
    Samples2,
    /// 4 samples averaged, 2.13 ms.
    Samples4,
    /// 8 samples averaged, 4.26 ms.
    Samples8,
    /// 16 samples averaged, 8.51 ms.
    Samples16,
    /// 32 samples averaged, 17.02 ms.
    Samples32,
    /// 64 samples averaged, 34.05 ms.
    Samples64,
    /// 128 samples averaged, 68.10 ms.
    Samples128,
}

impl Adc {
    /// Returns the four-bit field value.
    pub fn code(self) -> u8 {
        match self {
            Adc::Bits9 => 0b0000,
            Adc::Bits10 => 0b0001,
            Adc::Bits11 => 0b0010,
            Adc::Bits12 => 0b0011,
            Adc::Samples2 => 0b1001,
            Adc::Samples4 => 0b1010,
            Adc::Samples8 => 0b1011,
            Adc::Samples16 => 0b1100,
            Adc::Samples32 => 0b1101,
            Adc::Samples64 => 0b1110,
            Adc::Samples128 => 0b1111,
        }
    }

    /// Decodes a four-bit field value.
    ///
    /// With the high bit clear the second bit is ignored and the low two select the
    /// resolution; `0b1000` is 12-bit as well.
    ///
    /// # Arguments
    ///
    /// * `code` - the field value; only the low four bits are used.
    ///
    /// # Returns
    ///
    /// The setting.
    pub fn from_code(code: u8) -> Adc {
        match code & 0b1111 {
            0b1001 => Adc::Samples2,
            0b1010 => Adc::Samples4,
            0b1011 => Adc::Samples8,
            0b1100 => Adc::Samples16,
            0b1101 => Adc::Samples32,
            0b1110 => Adc::Samples64,
            0b1111 => Adc::Samples128,
            0b1000 => Adc::Bits12,
            low => match low & 0b11 {
                0b00 => Adc::Bits9,
                0b01 => Adc::Bits10,
                0b10 => Adc::Bits11,
                _ => Adc::Bits12,
            },
        }
    }

    /// Returns the conversion time in microseconds.
    pub fn conversion_micros(self) -> u32 {
        match self {
            Adc::Bits9 => 84,
            Adc::Bits10 => 148,
            Adc::Bits11 => 276,
            Adc::Bits12 => 532,
            Adc::Samples2 => 1_060,
            Adc::Samples4 => 2_130,
            Adc::Samples8 => 4_260,
            Adc::Samples16 => 8_510,
            Adc::Samples32 => 17_020,
            Adc::Samples64 => 34_050,
            Adc::Samples128 => 68_100,
        }
    }
}

/// The operating mode, the `MODE` bits of the Configuration register.
///
/// Writing a triggered mode starts one conversion, even if that mode is already set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// No conversions, lowest power.
    PowerDown = 0,
    /// One shunt conversion.
    ShuntTriggered = 1,
    /// One bus conversion.
    BusTriggered = 2,
    /// One shunt and one bus conversion.
    ShuntAndBusTriggered = 3,
    /// The ADC disabled.
    AdcOff = 4,
    /// Shunt conversions back to back.
    ShuntContinuous = 5,
    /// Bus conversions back to back.
    BusContinuous = 6,
    /// Shunt and bus conversions back to back, the reset setting.
    #[default]
    ShuntAndBusContinuous = 7,
}

impl Mode {
    /// Returns the three-bit `MODE` field value.
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Decodes a three-bit `MODE` field value.
    ///
    /// # Arguments
    ///
    /// * `code` - the field value; only the low three bits are used.
    ///
    /// # Returns
    ///
    /// The mode.
    pub fn from_code(code: u8) -> Mode {
        match code & 0b111 {
            0 => Mode::PowerDown,
            1 => Mode::ShuntTriggered,
            2 => Mode::BusTriggered,
            3 => Mode::ShuntAndBusTriggered,
            4 => Mode::AdcOff,
            5 => Mode::ShuntContinuous,
            6 => Mode::BusContinuous,
            _ => Mode::ShuntAndBusContinuous,
        }
    }

    /// Reports whether the mode converts the shunt voltage.
    pub fn measures_shunt(self) -> bool {
        matches!(
            self,
            Mode::ShuntTriggered
                | Mode::ShuntAndBusTriggered
                | Mode::ShuntContinuous
                | Mode::ShuntAndBusContinuous
        )
    }

    /// Reports whether the mode converts the bus voltage.
    pub fn measures_bus(self) -> bool {
        matches!(
            self,
            Mode::BusTriggered
                | Mode::ShuntAndBusTriggered
                | Mode::BusContinuous
                | Mode::ShuntAndBusContinuous
        )
    }
}

/// The Configuration register (00h).
///
/// The default is the reset state, [`CONFIG_RESET`]: 32 V range, gain 1/8, 12-bit
/// conversions, shunt and bus continuous.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Configuration {
    /// Reset the part as at power-on; the bit clears itself.
    pub reset: bool,
    /// Bus voltage range, `BRNG` in bit 13.
    pub bus_range: BusRange,
    /// Shunt gain and range, `PG` in bits 12:11.
    pub gain: Gain,
    /// Bus ADC resolution or averaging, `BADC` in bits 10:7.
    pub bus_adc: Adc,
    /// Shunt ADC resolution or averaging, `SADC` in bits 6:3.
    pub shunt_adc: Adc,
    /// Operating mode, `MODE` in bits 2:0.
    pub mode: Mode,
}

impl Configuration {
    /// Packs the settings into the register word.
    ///
    /// # Returns
    ///
    /// The word to write to [`register::CONFIGURATION`].
    pub fn bits(self) -> u16 {
        u16::from(self.reset) << 15
            | u16::from(self.bus_range.code()) << 13
            | u16::from(self.gain.code()) << 11
            | u16::from(self.bus_adc.code()) << 7
            | u16::from(self.shunt_adc.code()) << 3
            | u16::from(self.mode.code())
    }

    /// Decodes a register word.
    ///
    /// # Arguments
    ///
    /// * `bits` - the word read from [`register::CONFIGURATION`].
    ///
    /// # Returns
    ///
    /// The decoded settings.
    pub fn from_bits(bits: u16) -> Configuration {
        Configuration {
            reset: bits & (1 << 15) != 0,
            bus_range: BusRange::from_code((bits >> 13) as u8),
            gain: Gain::from_code((bits >> 11) as u8),
            bus_adc: Adc::from_code((bits >> 7) as u8),
            shunt_adc: Adc::from_code((bits >> 3) as u8),
            mode: Mode::from_code(bits as u8),
        }
    }

    /// Returns how long one conversion cycle of the mode takes, in microseconds.
    ///
    /// A shunt conversion and a bus conversion run one after the other, so the time
    /// is the sum of whichever the mode measures.
    pub fn conversion_micros(self) -> u32 {
        let mut micros = 0;
        if self.mode.measures_shunt() {
            micros += self.shunt_adc.conversion_micros();
        }
        if self.mode.measures_bus() {
            micros += self.bus_adc.conversion_micros();
        }
        micros
    }
}

/// One set of results, with the current resolution they were taken at.
///
/// The four data registers are kept as read; the methods apply the datasheet's
/// scaling. A [`Reading`] whose current and power are zero means the calibration
/// register was never programmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    /// The Shunt Voltage register.
    pub shunt: i16,
    /// The Bus Voltage register, flags included.
    pub bus: u16,
    /// The Current register.
    pub current: i16,
    /// The Power register.
    pub power: u16,
    /// The programmed current resolution, in microamps per count.
    pub current_lsb_microamps: u32,
}

impl Reading {
    /// Returns the shunt voltage in microvolts.
    pub fn shunt_microvolts(&self) -> i32 {
        shunt_microvolts(self.shunt)
    }

    /// Returns the bus voltage in millivolts.
    pub fn bus_millivolts(&self) -> u32 {
        bus_millivolts(self.bus)
    }

    /// Returns the current in microamps.
    pub fn current_microamps(&self) -> i32 {
        current_microamps(self.current, self.current_lsb_microamps)
    }

    /// Returns the power in microwatts.
    pub fn power_microwatts(&self) -> u32 {
        power_microwatts(self.power, self.current_lsb_microamps)
    }

    /// Reports the math overflow flag: the current and power may be meaningless.
    pub fn math_overflow(&self) -> bool {
        math_overflow(self.bus)
    }
}

#[cfg(test)]
mod driver_support_tests {
    use super::*;

    #[test]
    fn the_default_configuration_is_the_reset_word() {
        assert_eq!(Configuration::default().bits(), CONFIG_RESET);
        assert_eq!(
            Configuration::from_bits(CONFIG_RESET),
            Configuration::default()
        );
    }

    #[test]
    fn fields_land_where_the_datasheet_puts_them() {
        let configuration = Configuration {
            reset: true,
            bus_range: BusRange::V16,
            gain: Gain::Div2,
            bus_adc: Adc::Samples16,
            shunt_adc: Adc::Bits9,
            mode: Mode::ShuntAndBusTriggered,
        };
        let bits = configuration.bits();
        assert_eq!(bits, 0x8000 | 0x0800 | (0b1100 << 7) | 0x0003);
        assert_eq!(Configuration::from_bits(bits), configuration);
    }

    #[test]
    fn adc_codes_and_times_follow_table_5() {
        assert_eq!(Adc::from_code(0b0100), Adc::Bits9, "bit 2 is a do not care");
        assert_eq!(Adc::from_code(0b1000), Adc::Bits12);
        assert_eq!(Adc::Samples128.conversion_micros(), 68_100);
        assert_eq!(Adc::Bits12.conversion_micros(), 532);
        for adc in [Adc::Bits11, Adc::Samples2, Adc::Samples64] {
            assert_eq!(Adc::from_code(adc.code()), adc);
        }
    }

    #[test]
    fn gains_and_modes_follow_tables_4_and_6() {
        assert_eq!(Gain::Div8.range_millivolts(), 320);
        assert_eq!(Gain::from_code(0b01), Gain::Div2);
        assert_eq!(Mode::from_code(0b011), Mode::ShuntAndBusTriggered);
        assert!(Mode::ShuntTriggered.measures_shunt());
        assert!(!Mode::ShuntTriggered.measures_bus());
        assert!(!Mode::AdcOff.measures_shunt());
        let triggered = Configuration {
            mode: Mode::ShuntAndBusTriggered,
            ..Configuration::default()
        };
        assert_eq!(triggered.conversion_micros(), 1_064);
    }

    #[test]
    fn a_reading_scales_its_registers_like_the_free_functions() {
        let reading = Reading {
            shunt: shunt_register(100_000),
            bus: bus_register(12_000),
            current: current_register(1_000_000, 100),
            power: power_register(12_000_000, 100),
            current_lsb_microamps: 100,
        };
        assert_eq!(reading.shunt_microvolts(), 100_000);
        assert_eq!(reading.bus_millivolts(), 12_000);
        assert_eq!(reading.current_microamps(), 1_000_000);
        assert_eq!(reading.power_microwatts(), 12_000_000);
        assert!(!reading.math_overflow());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The datasheet's worked design example (Table 8): max expected current 15 A,
    // shunt 2 mΩ, current LSB rounded to 1 mA/bit, bus range 16 V.
    const CURRENT_LSB: u32 = 1_000; // 1 mA in microamps

    #[test]
    fn every_register_survives_a_round_trip_through_its_builder() {
        // The datasheet's worked design example reads 11.98 V, 10 A, and 119.8 W, and
        // each builder reproduces the register that decodes back to it.
        assert_eq!(bus_millivolts(bus_register(11_980)), 11_980);
        assert_eq!(bus_register(11_980) >> 3, 0x5D98 >> 3);
        assert!(conversion_ready(bus_register(11_980)));
        assert!(!math_overflow(bus_register(11_980)));
        assert_eq!(current_register(10_000_000, CURRENT_LSB), 0x2710);
        assert_eq!(
            current_microamps(current_register(10_000_000, CURRENT_LSB), CURRENT_LSB),
            10_000_000
        );
        assert_eq!(power_register(119_800_000, CURRENT_LSB), 0x1766);
        assert_eq!(
            power_microwatts(power_register(119_800_000, CURRENT_LSB), CURRENT_LSB),
            119_800_000
        );
        assert_eq!(shunt_microvolts(shunt_register(20_000)), 20_000);
        assert_eq!(current_register(1, 0), 0);
        assert_eq!(power_register(1, 0), 0);
    }

    #[test]
    fn calibration_matches_the_datasheet_example() {
        // Cal = trunc(0.04096 / (0.001 A * 0.002 Ω)) = 20480 = 0x5000.
        assert_eq!(calibration(CURRENT_LSB, 2), 20_480);
        assert_eq!(calibration(CURRENT_LSB, 2), 0x5000);
    }

    #[test]
    fn minimum_current_lsb_matches_the_datasheet_example() {
        // 15 A / 32768 = 457.76 µA, computed up to the next whole microamp.
        assert_eq!(minimum_current_lsb_microamps(15_000_000), 458);
    }

    #[test]
    fn shunt_register_decodes_per_the_datasheet() {
        // Table 8: shunt register 0x07D0 = 2000 → 20 mV.
        assert_eq!(shunt_microvolts(0x07D0), 20_000);
        // Negative full scale at gain /8: -320 mV is register 0x8300 (Figure 20).
        assert_eq!(shunt_microvolts(i16::from_be_bytes([0x83, 0x00])), -320_000);
    }

    #[test]
    fn bus_register_decodes_per_the_datasheet() {
        // Table 8: bus register 0x5D98 → shifted 0x0BB3 = 2995 → 11.98 V.
        assert_eq!(bus_millivolts(0x5D98), 11_980);
        assert!(!conversion_ready(0x5D98));
        assert!(!math_overflow(0x5D98));
        // A reading with both status flags set.
        assert!(conversion_ready(0x1F43));
        assert!(math_overflow(0x1F43));
    }

    #[test]
    fn current_register_decodes_to_the_example_load() {
        // Table 8: current register 0x2710 = 10000 → 10.0 A.
        assert_eq!(current_microamps(0x2710, CURRENT_LSB), 10_000_000);
    }

    #[test]
    fn power_register_decodes_to_the_example_load() {
        // Table 8: power register 0x1766 = 5990 → 119.8 W (power LSB 20 mW).
        assert_eq!(power_microwatts(0x1766, CURRENT_LSB), 119_800_000);
    }

    #[test]
    fn an_uncalibrated_request_returns_zero() {
        assert_eq!(calibration(0, 2), 0);
        assert_eq!(calibration(CURRENT_LSB, 0), 0);
    }
}
