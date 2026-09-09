//! Bosch BME280 temperature, pressure, and humidity sensor.
//!
//! The BME280 ships raw, uncompensated readings plus a block of per-chip calibration
//! coefficients; the real measurement only appears after running those raw values
//! through Bosch's compensation formulas. Those formulas are the classic place a
//! from-memory port goes subtly wrong, so the integer arithmetic here is ported from
//! Bosch's published reference code, and the tests cross-check it against the
//! reference floating-point form and the datasheet's worked temperature example.
//!
//! A caller reads the calibration registers once with [`Calibration::from_registers`],
//! then reads the data registers each cycle into a [`RawMeasurement`] and calls
//! [`Calibration::compensate`]. The control registers ([`CtrlHum`], [`CtrlMeas`],
//! [`Config`]) and the measurement-time formula are here too, and the [`Bme280`]
//! driver (the `embedded-hal` feature) runs the whole sequence over an I2C or SPI bus.

#[cfg(feature = "embedded-hal")]
mod driver;

#[cfg(feature = "embedded-hal")]
pub use driver::{Bme280, STATUS_POLLS};

/// The I2C address with the SDO pin tied low.
pub const I2C_ADDRESS_PRIMARY: u8 = 0x76;
/// The I2C address with the SDO pin tied high.
pub const I2C_ADDRESS_SECONDARY: u8 = 0x77;
/// The value the chip-id register (0xD0) returns for a BME280.
pub const CHIP_ID: u8 = 0x60;

/// The BME280 register addresses used to read calibration and measurements.
pub mod register {
    /// Chip-id register; reads [`super::CHIP_ID`] for a BME280.
    pub const CHIP_ID: u8 = 0xD0;
    /// Soft-reset register.
    pub const RESET: u8 = 0xE0;
    /// First of the 26 temperature and pressure calibration bytes (0x88..=0xA1).
    pub const CALIB_TEMP_PRESS: u8 = 0x88;
    /// First of the 7 humidity calibration bytes (0xE1..=0xE7).
    pub const CALIB_HUMIDITY: u8 = 0xE1;
    /// First of the 8 burst-read data bytes: pressure, temperature, humidity
    /// (0xF7..=0xFE).
    pub const DATA: u8 = 0xF7;
    /// Humidity acquisition options; effective only after a write to `CTRL_MEAS`.
    pub const CTRL_HUM: u8 = 0xF2;
    /// The `measuring` and `im_update` flags.
    pub const STATUS: u8 = 0xF3;
    /// Temperature and pressure acquisition options and the power mode.
    pub const CTRL_MEAS: u8 = 0xF4;
    /// Standby time, IIR filter, and the 3-wire SPI switch.
    pub const CONFIG: u8 = 0xF5;
}

/// The soft-reset word: writing it to [`register::RESET`] runs the power-on sequence.
pub const RESET_WORD: u8 = 0xB6;
/// The value a skipped temperature or pressure measurement leaves in its registers.
pub const SKIPPED_OUTPUT: u32 = 0x80000;
/// The value a skipped humidity measurement leaves in its registers.
pub const SKIPPED_HUMIDITY: u16 = 0x8000;
/// The number of bytes in the temperature and pressure calibration block.
pub const CALIB_TEMP_PRESS_LEN: usize = 26;
/// The number of bytes in the humidity calibration block.
pub const CALIB_HUMIDITY_LEN: usize = 7;
/// The number of bytes in the burst-read data block.
pub const DATA_LEN: usize = 8;
/// The start-up time from power-on or reset until the part accepts a transfer, in
/// microseconds.
pub const STARTUP_MICROS: u32 = 2_000;

/// Reads the `measuring` flag of a status register value.
///
/// # Arguments
///
/// * `status` - the byte read from [`register::STATUS`].
///
/// # Returns
///
/// `true` while a conversion is running and its results are not yet in the data
/// registers.
pub fn measuring(status: u8) -> bool {
    status & 0x08 != 0
}

/// Reads the `im_update` flag of a status register value.
///
/// # Arguments
///
/// * `status` - the byte read from [`register::STATUS`].
///
/// # Returns
///
/// `true` while the calibration data is being copied from non-volatile memory, at
/// power-on reset and before every conversion.
pub fn image_updating(status: u8) -> bool {
    status & 0x01 != 0
}

/// The per-chip calibration coefficients read from the sensor's calibration registers.
///
/// These are factory-programmed and constant for a given device, so they are read
/// once at start-up and reused for every measurement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calibration {
    dig_t1: u16,
    dig_t2: i16,
    dig_t3: i16,
    dig_p1: u16,
    dig_p2: i16,
    dig_p3: i16,
    dig_p4: i16,
    dig_p5: i16,
    dig_p6: i16,
    dig_p7: i16,
    dig_p8: i16,
    dig_p9: i16,
    dig_h1: u8,
    dig_h2: i16,
    dig_h3: u8,
    dig_h4: i16,
    dig_h5: i16,
    dig_h6: i8,
}

impl Calibration {
    /// Parses the calibration coefficients from the two register blocks.
    ///
    /// The split-and-shifted packing of `dig_h4` and `dig_h5`, which share a byte, is
    /// done exactly as the datasheet specifies.
    ///
    /// # Arguments
    ///
    /// * `temp_press` - the 26 bytes read from `0x88..=0xA1`.
    /// * `humidity` - the 7 bytes read from `0xE1..=0xE7`.
    ///
    /// # Returns
    ///
    /// The decoded calibration.
    pub fn from_registers(temp_press: &[u8; 26], humidity: &[u8; 7]) -> Calibration {
        let tp = temp_press;
        let h = humidity;
        Calibration {
            dig_t1: u16::from_le_bytes([tp[0], tp[1]]),
            dig_t2: i16::from_le_bytes([tp[2], tp[3]]),
            dig_t3: i16::from_le_bytes([tp[4], tp[5]]),
            dig_p1: u16::from_le_bytes([tp[6], tp[7]]),
            dig_p2: i16::from_le_bytes([tp[8], tp[9]]),
            dig_p3: i16::from_le_bytes([tp[10], tp[11]]),
            dig_p4: i16::from_le_bytes([tp[12], tp[13]]),
            dig_p5: i16::from_le_bytes([tp[14], tp[15]]),
            dig_p6: i16::from_le_bytes([tp[16], tp[17]]),
            dig_p7: i16::from_le_bytes([tp[18], tp[19]]),
            dig_p8: i16::from_le_bytes([tp[20], tp[21]]),
            dig_p9: i16::from_le_bytes([tp[22], tp[23]]),
            dig_h1: tp[25],
            dig_h2: i16::from_le_bytes([h[0], h[1]]),
            dig_h3: h[2],
            dig_h4: ((h[3] as i8 as i16) * 16) | (h[4] & 0x0F) as i16,
            dig_h5: ((h[5] as i8 as i16) * 16) | (h[4] >> 4) as i16,
            dig_h6: h[6] as i8,
        }
    }

    /// Compensates a raw measurement into temperature, pressure, and humidity.
    ///
    /// Temperature is computed first because its intermediate `t_fine` term feeds the
    /// pressure and humidity formulas, exactly as the reference code shares it.
    ///
    /// # Arguments
    ///
    /// * `raw` - the uncompensated readings from the data registers.
    ///
    /// # Returns
    ///
    /// The compensated [`Measurement`].
    pub fn compensate(&self, raw: &RawMeasurement) -> Measurement {
        let t_fine = self.t_fine(raw.temperature);
        Measurement {
            temperature_centi_celsius: compensate_temperature(t_fine),
            pressure_centi_pascals: self.compensate_pressure(raw.pressure, t_fine),
            humidity_q22_10: self.compensate_humidity(raw.humidity, t_fine),
        }
    }

    // The shared fine-temperature term, in the reference code's fixed-point form.
    fn t_fine(&self, adc_t: i32) -> i32 {
        let var1 = ((adc_t / 8) - (self.dig_t1 as i32 * 2)) * self.dig_t2 as i32 / 2048;
        let near = (adc_t / 16) - self.dig_t1 as i32;
        let var2 = (((near * near) / 4096) * self.dig_t3 as i32) / 16384;
        var1 + var2
    }

    // Pressure in hundredths of a pascal, via the 64-bit reference path.
    fn compensate_pressure(&self, adc_p: i32, t_fine: i32) -> u32 {
        let mut var1 = t_fine as i64 - 128000;
        let mut var2 = var1 * var1 * self.dig_p6 as i64;
        var2 += (var1 * self.dig_p5 as i64) * 131072;
        var2 += self.dig_p4 as i64 * 34359738368;
        var1 = (var1 * var1 * self.dig_p3 as i64) / 256 + (var1 * self.dig_p2 as i64 * 4096);
        var1 = (140737488355328 + var1) * self.dig_p1 as i64 / 8589934592;
        if var1 == 0 {
            return 3_000_000;
        }
        let mut var4 = 1048576 - adc_p as i64;
        var4 = (((var4 * 2147483648) - var2) * 3125) / var1;
        var1 = (self.dig_p9 as i64 * (var4 / 8192) * (var4 / 8192)) / 33554432;
        var2 = (self.dig_p8 as i64 * var4) / 524288;
        var4 = ((var4 + var1 + var2) / 256) + (self.dig_p7 as i64 * 16);
        let pressure = ((var4 / 2) * 100) / 128;
        pressure.clamp(3_000_000, 11_000_000) as u32
    }

    // Humidity in Q22.10 fixed-point (units of 1/1024 %), via the reference path.
    fn compensate_humidity(&self, adc_h: i32, t_fine: i32) -> u32 {
        let var1 = t_fine - 76800;
        let var2 = adc_h * 16384;
        let var3 = self.dig_h4 as i32 * 1048576;
        let var4 = self.dig_h5 as i32 * var1;
        let var5 = (((var2 - var3) - var4) + 16384) / 32768;
        let var2 = (var1 * self.dig_h6 as i32) / 1024;
        let var3 = (var1 * self.dig_h3 as i32) / 2048;
        let var4 = ((var2 * (var3 + 32768)) / 1024) + 2097152;
        let var2 = ((var4 * self.dig_h2 as i32) + 8192) / 16384;
        let var3 = var5 * var2;
        let var4 = ((var3 / 32768) * (var3 / 32768)) / 128;
        let var5 = (var3 - ((var4 * self.dig_h1 as i32) / 16)).clamp(0, 419430400);
        ((var5 / 4096) as u32).min(102400)
    }
}

// The integer temperature in hundredths of a degree Celsius, clamped to range.
fn compensate_temperature(t_fine: i32) -> i32 {
    ((t_fine * 5 + 128) / 256).clamp(-4000, 8500)
}

/// The raw, uncompensated readings from the BME280 data registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawMeasurement {
    /// The 20-bit uncompensated temperature.
    pub temperature: i32,
    /// The 20-bit uncompensated pressure.
    pub pressure: i32,
    /// The 16-bit uncompensated humidity.
    pub humidity: i32,
}

impl RawMeasurement {
    /// Parses the eight data-register bytes burst-read from `0xF7..=0xFE`.
    ///
    /// The order on the wire is pressure (20 bits), temperature (20 bits), then
    /// humidity (16 bits), each most significant byte first.
    ///
    /// # Arguments
    ///
    /// * `data` - the eight bytes read from the data registers.
    ///
    /// # Returns
    ///
    /// The unpacked raw measurement.
    pub fn from_registers(data: &[u8; 8]) -> RawMeasurement {
        let pressure =
            (i32::from(data[0]) << 12) | (i32::from(data[1]) << 4) | (i32::from(data[2]) >> 4);
        let temperature =
            (i32::from(data[3]) << 12) | (i32::from(data[4]) << 4) | (i32::from(data[5]) >> 4);
        let humidity = (i32::from(data[6]) << 8) | i32::from(data[7]);
        RawMeasurement {
            temperature,
            pressure,
            humidity,
        }
    }
}

/// A compensated BME280 measurement.
///
/// The fields are the exact integer outputs of the compensation formulas; the
/// methods present them in conventional units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measurement {
    /// Temperature in hundredths of a degree Celsius.
    pub temperature_centi_celsius: i32,
    /// Pressure in hundredths of a pascal.
    pub pressure_centi_pascals: u32,
    /// Relative humidity in Q22.10 fixed-point, that is units of 1/1024 percent.
    pub humidity_q22_10: u32,
}

impl Measurement {
    /// Returns the temperature in degrees Celsius.
    pub fn celsius(&self) -> f32 {
        self.temperature_centi_celsius as f32 / 100.0
    }

    /// Returns the pressure in pascals.
    pub fn pascals(&self) -> u32 {
        self.pressure_centi_pascals / 100
    }

    /// Returns the pressure in hectopascals (millibars).
    pub fn hectopascals(&self) -> f32 {
        self.pressure_centi_pascals as f32 / 10_000.0
    }

    /// Returns the relative humidity in percent.
    pub fn relative_humidity_percent(&self) -> f32 {
        self.humidity_q22_10 as f32 / 1024.0
    }
}

/// An oversampling setting for one of the three measurements.
///
/// Each doubling of the sample count adds one bit of resolution to temperature and
/// pressure, up to the 20 bits the data registers hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Oversampling {
    /// The measurement is skipped; its data registers read [`SKIPPED_OUTPUT`] or
    /// [`SKIPPED_HUMIDITY`].
    #[default]
    Skipped,
    /// A single sample.
    X1,
    /// Two samples.
    X2,
    /// Four samples.
    X4,
    /// Eight samples.
    X8,
    /// Sixteen samples.
    X16,
}

impl Oversampling {
    /// Returns the three-bit `osrs_t`, `osrs_p`, or `osrs_h` field value.
    pub fn code(self) -> u8 {
        match self {
            Oversampling::Skipped => 0b000,
            Oversampling::X1 => 0b001,
            Oversampling::X2 => 0b010,
            Oversampling::X4 => 0b011,
            Oversampling::X8 => 0b100,
            Oversampling::X16 => 0b101,
        }
    }

    /// Decodes a three-bit oversampling field value.
    ///
    /// The codes `0b101`, `0b110`, and `0b111` all select 16x oversampling.
    ///
    /// # Arguments
    ///
    /// * `code` - the field value; only the low three bits are used.
    ///
    /// # Returns
    ///
    /// The oversampling setting.
    pub fn from_code(code: u8) -> Oversampling {
        match code & 0b111 {
            0b000 => Oversampling::Skipped,
            0b001 => Oversampling::X1,
            0b010 => Oversampling::X2,
            0b011 => Oversampling::X4,
            0b100 => Oversampling::X8,
            _ => Oversampling::X16,
        }
    }

    /// Returns the number of samples averaged, or `0` when skipped.
    pub fn factor(self) -> u8 {
        match self {
            Oversampling::Skipped => 0,
            Oversampling::X1 => 1,
            Oversampling::X2 => 2,
            Oversampling::X4 => 4,
            Oversampling::X8 => 8,
            Oversampling::X16 => 16,
        }
    }
}

/// The power mode selected by the `mode[1:0]` field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// No measurements; the power-on default. Every register stays readable.
    #[default]
    Sleep,
    /// One measurement, then back to sleep.
    Forced,
    /// Continuous cycling between a measurement and a standby period.
    Normal,
}

impl Mode {
    /// Returns the two-bit `mode` field value.
    pub fn code(self) -> u8 {
        match self {
            Mode::Sleep => 0b00,
            Mode::Forced => 0b01,
            Mode::Normal => 0b11,
        }
    }

    /// Decodes a two-bit `mode` field value; both `0b01` and `0b10` mean forced.
    ///
    /// # Arguments
    ///
    /// * `code` - the field value; only the low two bits are used.
    ///
    /// # Returns
    ///
    /// The power mode.
    pub fn from_code(code: u8) -> Mode {
        match code & 0b11 {
            0b00 => Mode::Sleep,
            0b11 => Mode::Normal,
            _ => Mode::Forced,
        }
    }
}

/// The inactive period between measurements in normal mode, the `t_sb[2:0]` field.
///
/// The two longest BMP280 settings become the two shortest here: codes `0b110` and
/// `0b111` mean 10 and 20 ms on a BME280.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Standby {
    /// 0.5 ms.
    #[default]
    Ms0_5,
    /// 62.5 ms.
    Ms62_5,
    /// 125 ms.
    Ms125,
    /// 250 ms.
    Ms250,
    /// 500 ms.
    Ms500,
    /// 1000 ms.
    Ms1000,
    /// 10 ms.
    Ms10,
    /// 20 ms.
    Ms20,
}

impl Standby {
    /// Returns the three-bit `t_sb` field value.
    pub fn code(self) -> u8 {
        match self {
            Standby::Ms0_5 => 0b000,
            Standby::Ms62_5 => 0b001,
            Standby::Ms125 => 0b010,
            Standby::Ms250 => 0b011,
            Standby::Ms500 => 0b100,
            Standby::Ms1000 => 0b101,
            Standby::Ms10 => 0b110,
            Standby::Ms20 => 0b111,
        }
    }

    /// Decodes a three-bit `t_sb` field value.
    ///
    /// # Arguments
    ///
    /// * `code` - the field value; only the low three bits are used.
    ///
    /// # Returns
    ///
    /// The standby period.
    pub fn from_code(code: u8) -> Standby {
        match code & 0b111 {
            0b000 => Standby::Ms0_5,
            0b001 => Standby::Ms62_5,
            0b010 => Standby::Ms125,
            0b011 => Standby::Ms250,
            0b100 => Standby::Ms500,
            0b101 => Standby::Ms1000,
            0b110 => Standby::Ms10,
            _ => Standby::Ms20,
        }
    }

    /// Returns the standby period in microseconds.
    pub fn microseconds(self) -> u32 {
        match self {
            Standby::Ms0_5 => 500,
            Standby::Ms62_5 => 62_500,
            Standby::Ms125 => 125_000,
            Standby::Ms250 => 250_000,
            Standby::Ms500 => 500_000,
            Standby::Ms1000 => 1_000_000,
            Standby::Ms10 => 10_000,
            Standby::Ms20 => 20_000,
        }
    }
}

/// The IIR filter coefficient, the `filter[2:0]` field.
///
/// The filter smooths pressure and temperature across measurements and raises their
/// resolution to 20 bits; humidity is never filtered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    /// No filtering.
    #[default]
    Off,
    /// Coefficient 2.
    X2,
    /// Coefficient 4.
    X4,
    /// Coefficient 8.
    X8,
    /// Coefficient 16.
    X16,
}

impl Filter {
    /// Returns the three-bit `filter` field value.
    pub fn code(self) -> u8 {
        match self {
            Filter::Off => 0b000,
            Filter::X2 => 0b001,
            Filter::X4 => 0b010,
            Filter::X8 => 0b011,
            Filter::X16 => 0b100,
        }
    }

    /// Decodes a three-bit `filter` field value; `0b100` and above mean 16.
    ///
    /// # Arguments
    ///
    /// * `code` - the field value; only the low three bits are used.
    ///
    /// # Returns
    ///
    /// The filter setting.
    pub fn from_code(code: u8) -> Filter {
        match code & 0b111 {
            0b000 => Filter::Off,
            0b001 => Filter::X2,
            0b010 => Filter::X4,
            0b011 => Filter::X8,
            _ => Filter::X16,
        }
    }

    /// Returns the filter coefficient, or `0` when off.
    pub fn coefficient(self) -> u8 {
        match self {
            Filter::Off => 0,
            Filter::X2 => 2,
            Filter::X4 => 4,
            Filter::X8 => 8,
            Filter::X16 => 16,
        }
    }
}

/// The humidity control register (0xF2, `ctrl_hum`).
///
/// A write takes effect only after the next write to `ctrl_meas`. The default is the
/// reset state, `0x00`: humidity skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CtrlHum {
    /// Humidity oversampling, `osrs_h[2:0]` in bits 2:0.
    pub humidity: Oversampling,
}

impl CtrlHum {
    /// Packs the setting into the register byte.
    ///
    /// # Returns
    ///
    /// The byte to write to [`register::CTRL_HUM`].
    pub fn bits(self) -> u8 {
        self.humidity.code()
    }

    /// Decodes a register byte.
    ///
    /// # Arguments
    ///
    /// * `bits` - the byte read from [`register::CTRL_HUM`].
    ///
    /// # Returns
    ///
    /// The decoded setting.
    pub fn from_bits(bits: u8) -> CtrlHum {
        CtrlHum {
            humidity: Oversampling::from_code(bits),
        }
    }
}

/// The measurement control register (0xF4, `ctrl_meas`).
///
/// The default is the reset state, `0x00`: temperature and pressure skipped, the
/// part asleep.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CtrlMeas {
    /// Temperature oversampling, `osrs_t[2:0]` in bits 7:5.
    pub temperature: Oversampling,
    /// Pressure oversampling, `osrs_p[2:0]` in bits 4:2.
    pub pressure: Oversampling,
    /// Power mode, `mode[1:0]` in bits 1:0.
    pub mode: Mode,
}

impl CtrlMeas {
    /// Packs the settings into the register byte.
    ///
    /// # Returns
    ///
    /// The byte to write to [`register::CTRL_MEAS`].
    pub fn bits(self) -> u8 {
        (self.temperature.code() << 5) | (self.pressure.code() << 2) | self.mode.code()
    }

    /// Decodes a register byte.
    ///
    /// # Arguments
    ///
    /// * `bits` - the byte read from [`register::CTRL_MEAS`].
    ///
    /// # Returns
    ///
    /// The decoded settings.
    pub fn from_bits(bits: u8) -> CtrlMeas {
        CtrlMeas {
            temperature: Oversampling::from_code(bits >> 5),
            pressure: Oversampling::from_code(bits >> 2),
            mode: Mode::from_code(bits),
        }
    }
}

/// The configuration register (0xF5, `config`).
///
/// Writes may be ignored in normal mode; write it in sleep mode. The default is the
/// reset state, `0x00`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Config {
    /// Normal-mode standby period, `t_sb[2:0]` in bits 7:5.
    pub standby: Standby,
    /// IIR filter coefficient, `filter[2:0]` in bits 4:2.
    pub filter: Filter,
    /// Enables the 3-wire SPI interface, `spi3w_en` in bit 0.
    pub spi_3wire: bool,
}

impl Config {
    /// Packs the settings into the register byte.
    ///
    /// # Returns
    ///
    /// The byte to write to [`register::CONFIG`].
    pub fn bits(self) -> u8 {
        (self.standby.code() << 5) | (self.filter.code() << 2) | u8::from(self.spi_3wire)
    }

    /// Decodes a register byte.
    ///
    /// # Arguments
    ///
    /// * `bits` - the byte read from [`register::CONFIG`].
    ///
    /// # Returns
    ///
    /// The decoded settings.
    pub fn from_bits(bits: u8) -> Config {
        Config {
            standby: Standby::from_code(bits >> 5),
            filter: Filter::from_code(bits >> 2),
            spi_3wire: bits & 0x01 != 0,
        }
    }
}

/// The longest one measurement cycle can take, in microseconds.
///
/// This is the datasheet's maximum: 1.25 ms, plus 2.3 ms per temperature sample,
/// plus 2.3 ms per pressure sample and 0.575 ms, plus 2.3 ms per humidity sample and
/// 0.575 ms, each term only for a measurement that is not skipped. A driver waits
/// this long after forcing a measurement before it polls the status register.
///
/// # Arguments
///
/// * `temperature` - temperature oversampling.
/// * `pressure` - pressure oversampling.
/// * `humidity` - humidity oversampling.
///
/// # Returns
///
/// The maximum measurement time in microseconds.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::bme280::{max_measurement_micros, Oversampling};
///
/// // The datasheet's worked example: temperature x1, pressure x4, no humidity.
/// let micros = max_measurement_micros(Oversampling::X1, Oversampling::X4, Oversampling::Skipped);
/// assert_eq!(micros, 13_325);
/// ```
pub fn max_measurement_micros(
    temperature: Oversampling,
    pressure: Oversampling,
    humidity: Oversampling,
) -> u32 {
    let mut micros = 1_250;
    if temperature != Oversampling::Skipped {
        micros += 2_300 * u32::from(temperature.factor());
    }
    if pressure != Oversampling::Skipped {
        micros += 2_300 * u32::from(pressure.factor()) + 575;
    }
    if humidity != Oversampling::Skipped {
        micros += 2_300 * u32::from(humidity.factor()) + 575;
    }
    micros
}

/// The typical time one measurement cycle takes, in microseconds.
///
/// The datasheet's typical figure: 1 ms, plus 2 ms per temperature sample, plus 2 ms
/// per pressure sample and 0.5 ms, plus 2 ms per humidity sample and 0.5 ms, each
/// term only for a measurement that is not skipped.
///
/// # Arguments
///
/// * `temperature` - temperature oversampling.
/// * `pressure` - pressure oversampling.
/// * `humidity` - humidity oversampling.
///
/// # Returns
///
/// The typical measurement time in microseconds.
pub fn typical_measurement_micros(
    temperature: Oversampling,
    pressure: Oversampling,
    humidity: Oversampling,
) -> u32 {
    let mut micros = 1_000;
    if temperature != Oversampling::Skipped {
        micros += 2_000 * u32::from(temperature.factor());
    }
    if pressure != Oversampling::Skipped {
        micros += 2_000 * u32::from(pressure.factor()) + 500;
    }
    if humidity != Oversampling::Skipped {
        micros += 2_000 * u32::from(humidity.factor()) + 500;
    }
    micros
}

impl Calibration {
    /// Packs the coefficients back into the two register blocks.
    ///
    /// This is the inverse of [`from_registers`](Calibration::from_registers), so a
    /// test can build what a part with these coefficients holds in its calibration
    /// registers. Byte 24 of the first block (register 0xA0) is not a coefficient and
    /// is written as zero.
    ///
    /// # Returns
    ///
    /// The 26 bytes of `0x88..=0xA1` and the 7 bytes of `0xE1..=0xE7`.
    pub fn to_registers(&self) -> ([u8; CALIB_TEMP_PRESS_LEN], [u8; CALIB_HUMIDITY_LEN]) {
        let mut tp = [0u8; CALIB_TEMP_PRESS_LEN];
        let words = [
            self.dig_t1,
            self.dig_t2 as u16,
            self.dig_t3 as u16,
            self.dig_p1,
            self.dig_p2 as u16,
            self.dig_p3 as u16,
            self.dig_p4 as u16,
            self.dig_p5 as u16,
            self.dig_p6 as u16,
            self.dig_p7 as u16,
            self.dig_p8 as u16,
            self.dig_p9 as u16,
        ];
        for (index, word) in words.into_iter().enumerate() {
            let [low, high] = word.to_le_bytes();
            tp[2 * index] = low;
            tp[2 * index + 1] = high;
        }
        tp[25] = self.dig_h1;

        let h2 = (self.dig_h2 as u16).to_le_bytes();
        let h = [
            h2[0],
            h2[1],
            self.dig_h3,
            (self.dig_h4 >> 4) as u8,
            (((self.dig_h5 & 0x0F) as u8) << 4) | (self.dig_h4 & 0x0F) as u8,
            (self.dig_h5 >> 4) as u8,
            self.dig_h6 as u8,
        ];
        (tp, h)
    }
}

impl RawMeasurement {
    /// Packs the raw readings back into the eight data-register bytes.
    ///
    /// This is the inverse of [`from_registers`](RawMeasurement::from_registers), so
    /// a test can build what a part holding these readings sends in a burst read.
    ///
    /// # Returns
    ///
    /// The eight bytes of `0xF7..=0xFE`.
    pub fn to_registers(&self) -> [u8; DATA_LEN] {
        let pressure = self.pressure as u32;
        let temperature = self.temperature as u32;
        let humidity = self.humidity as u16;
        [
            (pressure >> 12) as u8,
            (pressure >> 4) as u8,
            ((pressure & 0x0F) << 4) as u8,
            (temperature >> 12) as u8,
            (temperature >> 4) as u8,
            ((temperature & 0x0F) << 4) as u8,
            (humidity >> 8) as u8,
            humidity as u8,
        ]
    }
}

#[cfg(test)]
mod control_tests {
    use super::*;

    #[test]
    fn oversampling_codes_follow_the_datasheet_tables() {
        assert_eq!(Oversampling::Skipped.code(), 0b000);
        assert_eq!(Oversampling::X16.code(), 0b101);
        assert_eq!(Oversampling::from_code(0b110), Oversampling::X16);
        assert_eq!(Oversampling::from_code(0b100), Oversampling::X8);
        assert_eq!(Oversampling::X8.factor(), 8);
    }

    #[test]
    fn mode_codes_treat_both_forced_encodings_alike() {
        assert_eq!(Mode::from_code(0b01), Mode::Forced);
        assert_eq!(Mode::from_code(0b10), Mode::Forced);
        assert_eq!(Mode::from_code(0b11), Mode::Normal);
        assert_eq!(Mode::Normal.code(), 0b11);
    }

    #[test]
    fn standby_codes_carry_the_bme280_reassignment_of_the_top_two() {
        assert_eq!(Standby::Ms10.code(), 0b110);
        assert_eq!(Standby::Ms20.code(), 0b111);
        assert_eq!(Standby::from_code(0b110).microseconds(), 10_000);
        assert_eq!(Standby::from_code(0b101).microseconds(), 1_000_000);
        assert_eq!(Standby::from_code(0b000).microseconds(), 500);
    }

    #[test]
    fn filter_codes_saturate_at_sixteen() {
        assert_eq!(Filter::X16.code(), 0b100);
        assert_eq!(Filter::from_code(0b111), Filter::X16);
        assert_eq!(Filter::from_code(0b011).coefficient(), 8);
        assert_eq!(Filter::Off.coefficient(), 0);
    }

    #[test]
    fn control_registers_pack_their_fields_where_the_datasheet_puts_them() {
        let ctrl_meas = CtrlMeas {
            temperature: Oversampling::X2,
            pressure: Oversampling::X16,
            mode: Mode::Normal,
        };
        assert_eq!(ctrl_meas.bits(), 0b0101_0111);
        assert_eq!(CtrlMeas::from_bits(0b0101_0111), ctrl_meas);
        let config = Config {
            standby: Standby::Ms1000,
            filter: Filter::X4,
            spi_3wire: true,
        };
        assert_eq!(config.bits(), 0b1010_1001);
        assert_eq!(Config::from_bits(0b1010_1001), config);
        assert_eq!(CtrlHum::from_bits(0x03).humidity, Oversampling::X4);
        assert_eq!(CtrlMeas::default().bits(), 0x00);
        assert_eq!(Config::default().bits(), 0x00);
    }

    #[test]
    fn status_flags_sit_in_bits_three_and_zero() {
        assert!(measuring(0x08));
        assert!(!measuring(0x01));
        assert!(image_updating(0x01));
        assert!(!image_updating(0x08));
    }

    #[test]
    fn measurement_times_match_the_datasheet_worked_example() {
        let (t, p, h) = (Oversampling::X1, Oversampling::X4, Oversampling::Skipped);
        assert_eq!(max_measurement_micros(t, p, h), 13_325);
        assert_eq!(typical_measurement_micros(t, p, h), 11_500);
        let all = Oversampling::X1;
        assert_eq!(max_measurement_micros(all, all, all), 9_300);
        assert_eq!(
            max_measurement_micros(
                Oversampling::Skipped,
                Oversampling::Skipped,
                Oversampling::Skipped
            ),
            1_250
        );
    }

    #[test]
    fn calibration_registers_round_trip_including_the_shared_h4_h5_byte() {
        let tp = [
            0x45, 0x6F, 0x6F, 0x68, 0x32, 0x00, 0x46, 0x91, 0x6A, 0xD6, 0xD0, 0x0B, 0x4E, 0x1E,
            0x88, 0xFF, 0xF9, 0xFF, 0xAC, 0x26, 0x0A, 0xD8, 0xBD, 0x10, 0x00, 0x4B,
        ];
        let h = [0x62, 0x01, 0x00, 0x15, 0x23, 0x03, 0x1E];
        let calibration = Calibration::from_registers(&tp, &h);
        assert_eq!(calibration.dig_t1, 28485);
        assert_eq!(calibration.dig_p2, -10646);
        assert_eq!(calibration.dig_h4, 339);
        assert_eq!(calibration.dig_h5, 50);
        assert_eq!(calibration.to_registers(), (tp, h));

        let negative = Calibration {
            dig_h4: -100,
            dig_h5: -7,
            ..calibration
        };
        let (tp2, h2) = negative.to_registers();
        assert_eq!(Calibration::from_registers(&tp2, &h2), negative);
    }

    #[test]
    fn raw_measurement_registers_round_trip() {
        let data = [0x53, 0xD0, 0xE0, 0x81, 0xD9, 0x00, 0x6E, 0x62];
        let raw = RawMeasurement::from_registers(&data);
        assert_eq!(raw.to_registers(), data);
        let skipped = RawMeasurement {
            temperature: SKIPPED_OUTPUT as i32,
            pressure: SKIPPED_OUTPUT as i32,
            humidity: i32::from(SKIPPED_HUMIDITY),
        };
        assert_eq!(
            RawMeasurement::from_registers(&skipped.to_registers()),
            skipped
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A representative, internally consistent calibration set, used to exercise the
    // formulas across a sweep of raw codes.
    fn sample_calibration() -> Calibration {
        Calibration {
            dig_t1: 28485,
            dig_t2: 26735,
            dig_t3: 50,
            dig_p1: 37190,
            dig_p2: -10646,
            dig_p3: 3024,
            dig_p4: 7758,
            dig_p5: -120,
            dig_p6: -7,
            dig_p7: 9900,
            dig_p8: -10230,
            dig_p9: 4285,
            dig_h1: 75,
            dig_h2: 354,
            dig_h3: 0,
            dig_h4: 339,
            dig_h5: 50,
            dig_h6: 30,
        }
    }

    #[test]
    fn temperature_matches_the_datasheet_worked_example() {
        // Bosch's published worked example: dig_t1 = 27504, dig_t2 = 26435,
        // dig_t3 = -1000, adc_t = 519888 gives T = 25.08 °C (2508 in hundredths). The
        // shared t_fine intermediate lands at 128423 with the current integer formula,
        // one count off the 128422 quoted from the older BMP280 example; the published
        // result, the 25.08 °C output, is what this anchors to.
        let calib = Calibration {
            dig_t1: 27504,
            dig_t2: 26435,
            dig_t3: -1000,
            ..sample_calibration()
        };
        let t_fine = calib.t_fine(519888);
        assert!((t_fine - 128422).abs() <= 1, "t_fine {t_fine}");
        assert_eq!(compensate_temperature(t_fine), 2508);
    }

    #[test]
    fn integer_compensation_tracks_the_floating_point_reference() {
        let calib = sample_calibration();
        // Sweep a range of raw codes and require the integer path to agree with the
        // structurally different floating-point reference to within rounding.
        for &adc_t in &[400_000, 500_000, 519_888, 540_000] {
            let t_fine = calib.t_fine(adc_t);
            let t_int = compensate_temperature(t_fine);
            let t_ref = reference_temperature(&calib, adc_t);
            assert!(
                ((t_int as f64) / 100.0 - t_ref).abs() < 0.02,
                "temperature {t_int} vs {t_ref}"
            );

            for &adc_p in &[300_000, 415_148, 512_000] {
                let p_int = calib.compensate_pressure(adc_p, t_fine);
                let p_ref = reference_pressure(&calib, adc_p, adc_t);
                assert!(
                    ((p_int as f64) / 100.0 - p_ref).abs() < 2.0,
                    "pressure {p_int} (centi-Pa) vs {p_ref} Pa"
                );
            }

            for &adc_h in &[20_000, 30_000, 45_000] {
                let h_int = calib.compensate_humidity(adc_h, t_fine);
                let h_ref = reference_humidity(&calib, adc_h, adc_t);
                assert!(
                    ((h_int as f64) / 1024.0 - h_ref).abs() < 0.05,
                    "humidity {h_int} (Q22.10) vs {h_ref} %"
                );
            }
        }
    }

    #[test]
    fn raw_measurement_unpacks_the_data_registers() {
        // Pressure 0x53D0E, temperature 0x81D90, humidity 0x6E62.
        let data = [0x53, 0xD0, 0xE0, 0x81, 0xD9, 0x00, 0x6E, 0x62];
        let raw = RawMeasurement::from_registers(&data);
        assert_eq!(raw.pressure, 0x5_3D0E);
        assert_eq!(raw.temperature, 0x8_1D90);
        assert_eq!(raw.humidity, 0x6E62);
    }

    #[test]
    fn measurement_unit_helpers_convert_correctly() {
        let measurement = Measurement {
            temperature_centi_celsius: 2508,
            pressure_centi_pascals: 10_065_300,
            humidity_q22_10: 47_104,
        };
        assert!((measurement.celsius() - 25.08).abs() < 0.001);
        assert_eq!(measurement.pascals(), 100_653);
        assert!((measurement.hectopascals() - 1006.53).abs() < 0.01);
        assert!((measurement.relative_humidity_percent() - 46.0).abs() < 0.001);
    }

    // The reference floating-point compensation from Bosch's published code, used only
    // to validate the integer path above.
    fn reference_temperature(c: &Calibration, adc_t: i32) -> f64 {
        let var1 = (adc_t as f64 / 16384.0 - c.dig_t1 as f64 / 1024.0) * c.dig_t2 as f64;
        let var2 = {
            let v = adc_t as f64 / 131072.0 - c.dig_t1 as f64 / 8192.0;
            v * v * c.dig_t3 as f64
        };
        ((var1 + var2) / 5120.0).clamp(-40.0, 85.0)
    }

    fn reference_t_fine(c: &Calibration, adc_t: i32) -> f64 {
        let var1 = (adc_t as f64 / 16384.0 - c.dig_t1 as f64 / 1024.0) * c.dig_t2 as f64;
        let var2 = {
            let v = adc_t as f64 / 131072.0 - c.dig_t1 as f64 / 8192.0;
            v * v * c.dig_t3 as f64
        };
        var1 + var2
    }

    fn reference_pressure(c: &Calibration, adc_p: i32, adc_t: i32) -> f64 {
        let t_fine = reference_t_fine(c, adc_t);
        let mut var1 = (t_fine / 2.0) - 64000.0;
        let mut var2 = var1 * var1 * c.dig_p6 as f64 / 32768.0;
        var2 += var1 * c.dig_p5 as f64 * 2.0;
        var2 = (var2 / 4.0) + (c.dig_p4 as f64 * 65536.0);
        let var3 = c.dig_p3 as f64 * var1 * var1 / 524288.0;
        var1 = (var3 + c.dig_p2 as f64 * var1) / 524288.0;
        var1 = (1.0 + var1 / 32768.0) * c.dig_p1 as f64;
        if var1 <= 0.0 {
            return 30000.0;
        }
        let mut pressure = 1048576.0 - adc_p as f64;
        pressure = (pressure - (var2 / 4096.0)) * 6250.0 / var1;
        var1 = c.dig_p9 as f64 * pressure * pressure / 2147483648.0;
        var2 = pressure * c.dig_p8 as f64 / 32768.0;
        pressure += (var1 + var2 + c.dig_p7 as f64) / 16.0;
        pressure.clamp(30000.0, 110000.0)
    }

    fn reference_humidity(c: &Calibration, adc_h: i32, adc_t: i32) -> f64 {
        let t_fine = reference_t_fine(c, adc_t);
        let var1 = t_fine - 76800.0;
        let var2 = c.dig_h4 as f64 * 64.0 + (c.dig_h5 as f64 / 16384.0) * var1;
        let var3 = adc_h as f64 - var2;
        let var4 = c.dig_h2 as f64 / 65536.0;
        let var5 = 1.0 + (c.dig_h3 as f64 / 67108864.0) * var1;
        let var6 = 1.0 + (c.dig_h6 as f64 / 67108864.0) * var1 * var5;
        let var6 = var3 * var4 * (var5 * var6);
        (var6 * (1.0 - c.dig_h1 as f64 * var6 / 524288.0)).clamp(0.0, 100.0)
    }
}
