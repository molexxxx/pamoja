//! Texas Instruments HDC1080 low-power humidity and temperature sensor.
//!
//! The HDC1080 returns each measurement as a plain 16-bit fraction of full scale: the
//! temperature register spans -40 to +125 °C and the humidity register 0 to 100 %RH,
//! both linearly over the 16-bit code. There is no per-chip calibration to apply and
//! no checksum on the frame, so the decode is the datasheet's two equations, done
//! here in integer arithmetic so a node without floating point reads exact
//! millidegrees and milli-percent.
//!
//! A caller writes a [`Configuration`] to the configuration register once, triggers
//! an acquisition by writing the temperature pointer, waits the resolution's
//! conversion time, and reads the four-byte frame into a [`Measurement`].

use crate::SensorError;

/// The fixed 7-bit I2C address, `1000000`.
pub const I2C_ADDRESS: u8 = 0x40;
/// The value the manufacturer-id register (0xFE) returns: Texas Instruments.
pub const MANUFACTURER_ID: u16 = 0x5449;
/// The value the device-id register (0xFF) returns for an HDC1080.
pub const DEVICE_ID: u16 = 0x1050;
/// The power-on value of the configuration register: acquisition mode set to
/// temperature then humidity, 14-bit resolution on both, heater off.
pub const CONFIGURATION_RESET: u16 = 0x1000;

/// The HDC1080 register addresses. Every register is 16 bits, sent MSB first.
pub mod register {
    /// Temperature result; writing this pointer also triggers an acquisition.
    pub const TEMPERATURE: u8 = 0x00;
    /// Humidity result; writing this pointer triggers a humidity-only acquisition.
    pub const HUMIDITY: u8 = 0x01;
    /// Configuration and status register.
    pub const CONFIGURATION: u8 = 0x02;
    /// Serial-id bits 40:25.
    pub const SERIAL_ID_HIGH: u8 = 0xFB;
    /// Serial-id bits 24:9.
    pub const SERIAL_ID_MID: u8 = 0xFC;
    /// Serial-id bits 8:0, in bits 15:7 of the register.
    pub const SERIAL_ID_LOW: u8 = 0xFD;
    /// Manufacturer id; reads [`super::MANUFACTURER_ID`].
    pub const MANUFACTURER_ID: u8 = 0xFE;
    /// Device id; reads [`super::DEVICE_ID`] for an HDC1080.
    pub const DEVICE_ID: u8 = 0xFF;
}

const TEMPERATURE_SPAN_MILLI: i64 = 165_000;
const TEMPERATURE_OFFSET_MILLI: i32 = 40_000;
const HUMIDITY_SPAN_MILLI: u64 = 100_000;

/// Decodes the temperature register to millidegrees Celsius.
///
/// The datasheet's `T = raw / 2^16 * 165 - 40`, computed in integers and rounded to
/// the nearest millidegree.
///
/// # Arguments
///
/// * `raw` - the 16-bit temperature register.
///
/// # Returns
///
/// The temperature in millidegrees Celsius, from -40 000 to 124 997.
pub fn milli_celsius(raw: u16) -> i32 {
    let scaled = (raw as i64 * TEMPERATURE_SPAN_MILLI + (1 << 15)) >> 16;
    scaled as i32 - TEMPERATURE_OFFSET_MILLI
}

/// Decodes the temperature register to degrees Celsius.
///
/// The floating-point convenience beside [`milli_celsius`].
///
/// # Arguments
///
/// * `raw` - the 16-bit temperature register.
///
/// # Returns
///
/// The temperature in degrees Celsius.
pub fn celsius(raw: u16) -> f32 {
    raw as f32 / 65_536.0 * 165.0 - 40.0
}

/// Decodes the humidity register to thousandths of a percent relative humidity.
///
/// The datasheet's `RH = raw / 2^16 * 100`, computed in integers and rounded to the
/// nearest milli-percent.
///
/// # Arguments
///
/// * `raw` - the 16-bit humidity register.
///
/// # Returns
///
/// The relative humidity in milli-percent, from 0 to 99 998.
pub fn milli_percent(raw: u16) -> u32 {
    ((raw as u64 * HUMIDITY_SPAN_MILLI + (1 << 15)) >> 16) as u32
}

/// Decodes the humidity register to percent relative humidity.
///
/// The floating-point convenience beside [`milli_percent`].
///
/// # Arguments
///
/// * `raw` - the 16-bit humidity register.
///
/// # Returns
///
/// The relative humidity in percent.
pub fn relative_humidity(raw: u16) -> f32 {
    raw as f32 / 65_536.0 * 100.0
}

/// Builds the temperature register the sensor reports for a temperature.
///
/// The inverse of [`milli_celsius`]. The result is a 14-bit code in bits 15:2 with
/// the two reserved low bits clear, which is what the part sends, so a decode of the
/// built register lands within one 14-bit step (about 0.01 °C) of the input. Inputs
/// outside -40 to +125 °C clamp to the ends of the scale.
///
/// # Arguments
///
/// * `milli_celsius` - the temperature in millidegrees Celsius.
///
/// # Returns
///
/// The 16-bit temperature register.
pub fn temperature_register(milli_celsius: i32) -> u16 {
    let offset = (milli_celsius as i64 + TEMPERATURE_OFFSET_MILLI as i64).max(0);
    let code = (offset * 16_384 + TEMPERATURE_SPAN_MILLI / 2) / TEMPERATURE_SPAN_MILLI;
    (code.min(16_383) as u16) << 2
}

/// Builds the humidity register the sensor reports for a relative humidity.
///
/// The inverse of [`milli_percent`], with the same 14-bit alignment as
/// [`temperature_register`]. Inputs above 100 %RH clamp to full scale.
///
/// # Arguments
///
/// * `milli_percent` - the relative humidity in thousandths of a percent.
///
/// # Returns
///
/// The 16-bit humidity register.
pub fn humidity_register(milli_percent: u32) -> u16 {
    let code = (milli_percent as u64 * 16_384 + HUMIDITY_SPAN_MILLI / 2) / HUMIDITY_SPAN_MILLI;
    (code.min(16_383) as u16) << 2
}

/// Assembles the serial number from its three registers.
///
/// The datasheet numbers the serial bits 40 down to 0, spread as bits 40:25 in
/// register 0xFB, 24:9 in 0xFC, and 8:0 in bits 15:7 of 0xFD.
///
/// # Arguments
///
/// * `high` - the 0xFB register.
/// * `mid` - the 0xFC register.
/// * `low` - the 0xFD register.
///
/// # Returns
///
/// The serial number, in the low 41 bits.
pub fn serial_id(high: u16, mid: u16, low: u16) -> u64 {
    ((high as u64) << 25) | ((mid as u64) << 9) | ((low >> 7) as u64)
}

/// Splits a serial number into the three registers a sensor reports it in.
///
/// The inverse of [`serial_id`].
///
/// # Arguments
///
/// * `serial` - the serial number; bits above 40 are ignored.
///
/// # Returns
///
/// The 0xFB, 0xFC, and 0xFD registers in that order.
pub fn serial_id_registers(serial: u64) -> [u16; 3] {
    [
        (serial >> 25) as u16,
        (serial >> 9) as u16,
        ((serial & 0x1FF) as u16) << 7,
    ]
}

/// The MODE field: what one trigger acquires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionMode {
    /// Temperature or humidity alone, chosen by the pointer written to trigger it.
    Single,
    /// Temperature and humidity in sequence, temperature first, read as one frame.
    TemperatureThenHumidity,
}

/// The TRES field: temperature measurement resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemperatureResolution {
    /// 14 bits, 6.35 ms conversion.
    Bits14,
    /// 11 bits, 3.65 ms conversion.
    Bits11,
}

impl TemperatureResolution {
    /// Returns the conversion time for this resolution.
    ///
    /// # Returns
    ///
    /// The datasheet's typical temperature conversion time in microseconds.
    pub fn conversion_time_micros(self) -> u32 {
        match self {
            TemperatureResolution::Bits14 => 6_350,
            TemperatureResolution::Bits11 => 3_650,
        }
    }
}

/// The HRES field: humidity measurement resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HumidityResolution {
    /// 14 bits, 6.50 ms conversion.
    Bits14,
    /// 11 bits, 3.85 ms conversion.
    Bits11,
    /// 8 bits, 2.50 ms conversion.
    Bits8,
}

impl HumidityResolution {
    /// Returns the conversion time for this resolution.
    ///
    /// # Returns
    ///
    /// The datasheet's typical humidity conversion time in microseconds.
    pub fn conversion_time_micros(self) -> u32 {
        match self {
            HumidityResolution::Bits14 => 6_500,
            HumidityResolution::Bits11 => 3_850,
            HumidityResolution::Bits8 => 2_500,
        }
    }
}

/// The configuration register (0x02), field by field.
///
/// [`Configuration::default`] is the power-on state, [`CONFIGURATION_RESET`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Configuration {
    /// RST, bit 15: write `true` to reset the part; the bit clears itself.
    pub software_reset: bool,
    /// HEAT, bit 13: run the on-die heater during measurements.
    pub heater: bool,
    /// MODE, bit 12: what one trigger acquires.
    pub mode: AcquisitionMode,
    /// BTST, bit 11, read only: `true` when the supply is below 2.8 V, refreshed
    /// after power-on and after every measurement request.
    pub battery_low: bool,
    /// TRES, bit 10.
    pub temperature_resolution: TemperatureResolution,
    /// HRES, bits 9:8.
    pub humidity_resolution: HumidityResolution,
}

impl Default for Configuration {
    fn default() -> Configuration {
        Configuration {
            software_reset: false,
            heater: false,
            mode: AcquisitionMode::TemperatureThenHumidity,
            battery_low: false,
            temperature_resolution: TemperatureResolution::Bits14,
            humidity_resolution: HumidityResolution::Bits14,
        }
    }
}

impl Configuration {
    /// Decodes the configuration register.
    ///
    /// Reserved bits (14 and 7:0) are ignored; the part reads them as zero.
    ///
    /// # Arguments
    ///
    /// * `raw` - the 16-bit configuration register.
    ///
    /// # Returns
    ///
    /// The decoded configuration.
    ///
    /// # Errors
    ///
    /// [`SensorError::Invalid`] if HRES holds `11`, the one code the datasheet
    /// leaves undefined.
    pub fn from_register(raw: u16) -> Result<Configuration, SensorError> {
        let humidity_resolution = match (raw >> 8) & 0b11 {
            0b00 => HumidityResolution::Bits14,
            0b01 => HumidityResolution::Bits11,
            0b10 => HumidityResolution::Bits8,
            _ => return Err(SensorError::Invalid),
        };
        Ok(Configuration {
            software_reset: raw & (1 << 15) != 0,
            heater: raw & (1 << 13) != 0,
            mode: if raw & (1 << 12) != 0 {
                AcquisitionMode::TemperatureThenHumidity
            } else {
                AcquisitionMode::Single
            },
            battery_low: raw & (1 << 11) != 0,
            temperature_resolution: if raw & (1 << 10) != 0 {
                TemperatureResolution::Bits11
            } else {
                TemperatureResolution::Bits14
            },
            humidity_resolution,
        })
    }

    /// Encodes the configuration register.
    ///
    /// # Returns
    ///
    /// The 16-bit register value, reserved bits clear.
    pub fn to_register(&self) -> u16 {
        let mut raw = 0;
        if self.software_reset {
            raw |= 1 << 15;
        }
        if self.heater {
            raw |= 1 << 13;
        }
        if self.mode == AcquisitionMode::TemperatureThenHumidity {
            raw |= 1 << 12;
        }
        if self.battery_low {
            raw |= 1 << 11;
        }
        if self.temperature_resolution == TemperatureResolution::Bits11 {
            raw |= 1 << 10;
        }
        raw |= match self.humidity_resolution {
            HumidityResolution::Bits14 => 0b00,
            HumidityResolution::Bits11 => 0b01,
            HumidityResolution::Bits8 => 0b10,
        } << 8;
        raw
    }

    /// Returns how long to wait after a trigger before the result is readable.
    ///
    /// # Returns
    ///
    /// In [`AcquisitionMode::TemperatureThenHumidity`], the temperature and humidity
    /// conversion times added together. In [`AcquisitionMode::Single`], the longer of
    /// the two, which covers whichever pointer the caller triggered.
    pub fn conversion_time_micros(&self) -> u32 {
        let temperature = self.temperature_resolution.conversion_time_micros();
        let humidity = self.humidity_resolution.conversion_time_micros();
        match self.mode {
            AcquisitionMode::TemperatureThenHumidity => temperature + humidity,
            AcquisitionMode::Single => temperature.max(humidity),
        }
    }
}

/// One combined temperature and humidity result, as read in a single transaction
/// after a trigger in [`AcquisitionMode::TemperatureThenHumidity`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measurement {
    /// The temperature register.
    pub temperature: u16,
    /// The humidity register.
    pub humidity: u16,
}

impl Measurement {
    /// Parses the four-byte frame: temperature MSB, LSB, humidity MSB, LSB.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the four bytes read from the temperature pointer.
    ///
    /// # Returns
    ///
    /// The two raw registers.
    pub fn parse(bytes: &[u8; 4]) -> Measurement {
        Measurement {
            temperature: u16::from_be_bytes([bytes[0], bytes[1]]),
            humidity: u16::from_be_bytes([bytes[2], bytes[3]]),
        }
    }

    /// Builds the four-byte frame the sensor sends for this measurement.
    ///
    /// The inverse of [`Measurement::parse`].
    ///
    /// # Returns
    ///
    /// Temperature MSB, LSB, humidity MSB, LSB.
    pub fn to_bytes(&self) -> [u8; 4] {
        let [t_hi, t_lo] = self.temperature.to_be_bytes();
        let [h_hi, h_lo] = self.humidity.to_be_bytes();
        [t_hi, t_lo, h_hi, h_lo]
    }

    /// Builds the measurement a sensor reports for a temperature and humidity.
    ///
    /// # Arguments
    ///
    /// * `milli_celsius` - the temperature in millidegrees Celsius.
    /// * `milli_percent` - the relative humidity in thousandths of a percent.
    ///
    /// # Returns
    ///
    /// The measurement, built with [`temperature_register`] and
    /// [`humidity_register`].
    pub fn from_physical(milli_celsius: i32, milli_percent: u32) -> Measurement {
        Measurement {
            temperature: temperature_register(milli_celsius),
            humidity: humidity_register(milli_percent),
        }
    }

    /// Returns the temperature in millidegrees Celsius.
    ///
    /// # Returns
    ///
    /// [`milli_celsius`] of the temperature register.
    pub fn milli_celsius(&self) -> i32 {
        milli_celsius(self.temperature)
    }

    /// Returns the temperature in degrees Celsius.
    ///
    /// # Returns
    ///
    /// [`celsius`] of the temperature register.
    pub fn celsius(&self) -> f32 {
        celsius(self.temperature)
    }

    /// Returns the relative humidity in thousandths of a percent.
    ///
    /// # Returns
    ///
    /// [`milli_percent`] of the humidity register.
    pub fn milli_percent(&self) -> u32 {
        milli_percent(self.humidity)
    }

    /// Returns the relative humidity in percent.
    ///
    /// # Returns
    ///
    /// [`relative_humidity`] of the humidity register.
    pub fn relative_humidity(&self) -> f32 {
        relative_humidity(self.humidity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Datasheet section 8.6.1, Equation 1: T(°C) = TEMPERATURE[15:0] / 2^16 * 165 - 40.
    fn reference_celsius(raw: u16) -> f64 {
        raw as f64 / 65_536.0 * 165.0 - 40.0
    }

    // Datasheet section 8.6.2, Equation 2: RH(%) = HUMIDITY[15:0] / 2^16 * 100.
    fn reference_humidity(raw: u16) -> f64 {
        raw as f64 / 65_536.0 * 100.0
    }

    #[test]
    fn temperature_formula_holds_at_the_ends_and_mid_scale() {
        // Equation 1: code 0 is the -40 °C floor, mid-scale is 165 / 2 - 40, and the
        // top code sits one 16-bit step below +125 °C.
        assert_eq!(milli_celsius(0x0000), -40_000);
        assert_eq!(milli_celsius(0x8000), 42_500);
        assert_eq!(milli_celsius(0xFFFF), 124_997);
        assert_eq!(milli_celsius(0x6000), 21_875);
        assert!((celsius(0x0000) + 40.0).abs() < 1e-5);
        assert!((celsius(0x8000) - 42.5).abs() < 1e-5);
        assert!((celsius(0xFFFF) - 124.9975).abs() < 1e-3);
    }

    #[test]
    fn humidity_formula_holds_at_the_ends_and_mid_scale() {
        // Equation 2: 0 %RH at code 0, 50 %RH at mid-scale, and the top code one
        // 16-bit step below 100 %RH.
        assert_eq!(milli_percent(0x0000), 0);
        assert_eq!(milli_percent(0x8000), 50_000);
        assert_eq!(milli_percent(0xFFFF), 99_998);
        assert_eq!(milli_percent(0x4000), 25_000);
        assert!(relative_humidity(0x0000).abs() < 1e-5);
        assert!((relative_humidity(0x8000) - 50.0).abs() < 1e-5);
        assert!((relative_humidity(0xFFFF) - 99.9985).abs() < 1e-3);
    }

    #[test]
    fn integer_decode_tracks_the_floating_point_reference() {
        // Every code in a stride across the register, plus the ends, must agree with
        // the datasheet formula to within the half-unit that rounding allows.
        let codes = (0..=0xFFFF_u32).step_by(97).chain([0xFFFF]);
        for raw in codes.map(|code| code as u16) {
            let t_ref = reference_celsius(raw) * 1_000.0;
            let t_int = milli_celsius(raw) as f64;
            assert!(
                (t_int - t_ref).abs() <= 0.5,
                "temperature {raw:#06x}: {t_int} vs {t_ref}"
            );

            let h_ref = reference_humidity(raw) * 1_000.0;
            let h_int = milli_percent(raw) as f64;
            assert!(
                (h_int - h_ref).abs() <= 0.5,
                "humidity {raw:#06x}: {h_int} vs {h_ref}"
            );
        }
    }

    #[test]
    fn register_builders_invert_the_decoders_to_within_a_14_bit_step() {
        // Tables 2 and 3: bits 1:0 of both result registers are always zero, so a
        // built register is 14-bit aligned and a decode lands within one 14-bit LSB
        // (165 000 / 16 384 millidegrees, 100 000 / 16 384 milli-percent).
        assert_eq!(temperature_register(42_500), 0x8000);
        assert_eq!(temperature_register(-40_000), 0x0000);
        assert_eq!(temperature_register(21_875), 0x6000);
        assert_eq!(humidity_register(50_000), 0x8000);
        assert_eq!(humidity_register(0), 0x0000);
        assert_eq!(humidity_register(25_000), 0x4000);
        for milli_celsius_in in (-40_000..=125_000).step_by(1_234) {
            let raw = temperature_register(milli_celsius_in);
            assert_eq!(raw & 0b11, 0);
            let back = milli_celsius(raw);
            assert!(
                (back - milli_celsius_in.min(124_997)).abs() <= 6,
                "{milli_celsius_in} -> {raw:#06x} -> {back}"
            );
        }
        for milli_percent_in in (0..=100_000).step_by(789) {
            let raw = humidity_register(milli_percent_in);
            assert_eq!(raw & 0b11, 0);
            let back = milli_percent(raw);
            assert!(
                (back as i64 - milli_percent_in.min(99_994) as i64).abs() <= 4,
                "{milli_percent_in} -> {raw:#06x} -> {back}"
            );
        }
        assert_eq!(temperature_register(-100_000), 0x0000);
        assert_eq!(temperature_register(200_000), 0xFFFC);
        assert_eq!(humidity_register(150_000), 0xFFFC);
    }

    #[test]
    fn identification_registers_read_the_datasheet_values() {
        // Table 1: 0xFE reads 0x5449 (Texas Instruments), 0xFF reads 0x1050.
        assert_eq!(MANUFACTURER_ID, 0x5449);
        assert_eq!(DEVICE_ID, 0x1050);
        assert_eq!(I2C_ADDRESS, 0b100_0000);
        assert_eq!(register::MANUFACTURER_ID, 0xFE);
        assert_eq!(register::DEVICE_ID, 0xFF);
    }

    #[test]
    fn serial_id_reassembles_from_its_three_registers() {
        // Tables 5 to 7: 0xFB carries bits 40:25, 0xFC bits 24:9, 0xFD bits 8:0 in
        // its top nine bits with 6:0 reserved.
        assert_eq!(serial_id(0xFFFF, 0xFFFF, 0xFF80), (1 << 41) - 1);
        assert_eq!(serial_id(0x0001, 0x0000, 0x0000), 1 << 25);
        assert_eq!(serial_id(0x0000, 0x0001, 0x0000), 1 << 9);
        assert_eq!(serial_id(0x0000, 0x0000, 0x0080), 1);
        let serial = 0x0123_4567_89AB;
        let regs = serial_id_registers(serial);
        assert_eq!(regs[2] & 0x7F, 0);
        assert_eq!(serial_id(regs[0], regs[1], regs[2]), serial);
    }

    #[test]
    fn configuration_reset_value_matches_the_register_map() {
        // Table 1: the configuration register resets to 0x1000, which Table 4 reads
        // as MODE = 1 with every other field zero.
        assert_eq!(CONFIGURATION_RESET, 0x1000);
        assert_eq!(Configuration::default().to_register(), CONFIGURATION_RESET);
        assert_eq!(
            Configuration::from_register(CONFIGURATION_RESET),
            Ok(Configuration::default())
        );
        let reset = Configuration::default();
        assert_eq!(reset.mode, AcquisitionMode::TemperatureThenHumidity);
        assert_eq!(reset.temperature_resolution, TemperatureResolution::Bits14);
        assert_eq!(reset.humidity_resolution, HumidityResolution::Bits14);
        assert!(!reset.heater && !reset.software_reset && !reset.battery_low);
    }

    #[test]
    fn configuration_fields_sit_at_the_bits_table_4_gives_them() {
        // Table 4: RST bit 15, HEAT bit 13, MODE bit 12, BTST bit 11, TRES bit 10,
        // HRES bits 9:8 (00 = 14 bit, 01 = 11 bit, 10 = 8 bit).
        let all = Configuration {
            software_reset: true,
            heater: true,
            mode: AcquisitionMode::TemperatureThenHumidity,
            battery_low: true,
            temperature_resolution: TemperatureResolution::Bits11,
            humidity_resolution: HumidityResolution::Bits8,
        };
        assert_eq!(all.to_register(), 0xBE00);
        assert_eq!(Configuration::from_register(0xBE00), Ok(all));

        let eleven = Configuration {
            humidity_resolution: HumidityResolution::Bits11,
            ..Configuration::default()
        };
        assert_eq!(eleven.to_register(), 0x1100);
        assert_eq!(Configuration::from_register(0x1100), Ok(eleven));

        let single = Configuration {
            mode: AcquisitionMode::Single,
            ..Configuration::default()
        };
        assert_eq!(single.to_register(), 0x0000);

        // Reserved bits are ignored on the way in and never set on the way out.
        assert_eq!(
            Configuration::from_register(0x50FF).map(|c| c.to_register()),
            Ok(0x1000)
        );
    }

    #[test]
    fn an_undefined_humidity_resolution_code_is_rejected() {
        // Table 4 defines HRES codes 00, 01, and 10 only.
        assert_eq!(
            Configuration::from_register(0x1300),
            Err(SensorError::Invalid)
        );
    }

    #[test]
    fn conversion_times_follow_the_electrical_characteristics() {
        // Electrical Characteristics: RHCT 2.50 / 3.85 / 6.50 ms for 8 / 11 / 14 bit,
        // TEMPCT 3.65 / 6.35 ms for 11 / 14 bit.
        assert_eq!(HumidityResolution::Bits8.conversion_time_micros(), 2_500);
        assert_eq!(HumidityResolution::Bits11.conversion_time_micros(), 3_850);
        assert_eq!(HumidityResolution::Bits14.conversion_time_micros(), 6_500);
        assert_eq!(
            TemperatureResolution::Bits11.conversion_time_micros(),
            3_650
        );
        assert_eq!(
            TemperatureResolution::Bits14.conversion_time_micros(),
            6_350
        );
        assert_eq!(Configuration::default().conversion_time_micros(), 12_850);
        let single = Configuration {
            mode: AcquisitionMode::Single,
            humidity_resolution: HumidityResolution::Bits8,
            ..Configuration::default()
        };
        assert_eq!(single.conversion_time_micros(), 6_350);
    }

    #[test]
    fn the_combined_frame_is_temperature_then_humidity_msb_first() {
        // Figure 14: a read from pointer 0x00 returns the temperature MSB and LSB
        // followed by the humidity MSB and LSB.
        let frame = [0x60, 0x00, 0x40, 0x00];
        let m = Measurement::parse(&frame);
        assert_eq!(m.temperature, 0x6000);
        assert_eq!(m.humidity, 0x4000);
        assert_eq!(m.milli_celsius(), 21_875);
        assert_eq!(m.milli_percent(), 25_000);
        assert!((m.celsius() - 21.875).abs() < 1e-5);
        assert!((m.relative_humidity() - 25.0).abs() < 1e-5);
        assert_eq!(m.to_bytes(), frame);

        let top = Measurement::parse(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(
            (top.milli_celsius(), top.milli_percent()),
            (124_997, 99_998)
        );

        let built = Measurement::from_physical(42_500, 50_000);
        assert_eq!(built.to_bytes(), [0x80, 0x00, 0x80, 0x00]);
        assert_eq!(Measurement::parse(&built.to_bytes()), built);
    }
}
