//! Bosch BMP280 digital pressure sensor.
//!
//! The BMP280 is the BME280's sibling without the humidity element: a barometric
//! pressure and temperature sensor that ships 20-bit raw ADC codes plus a block of
//! per-chip trimming coefficients, and leaves the compensation to the host. The
//! integer arithmetic here is a line for line port of the `bmp280_compensate_T_int32`
//! and `bmp280_compensate_P_int64` reference code in section 3.11.3 of the data
//! sheet (BST-BMP280-DS001), and the tests cross-check it against the data sheet's
//! floating-point form in appendix 8.1.
//!
//! A caller reads the 24 calibration bytes once with [`Calibration::parse`], then
//! burst-reads the six data registers each cycle into a [`Measurement`] and calls
//! [`Calibration::compensate`] for a [`Reading`]. [`CtrlMeas`] and [`Config`] build
//! and decode the two control registers, and every decode has a matching builder so a
//! node can be exercised with nothing wired. The measurement-time functions are here
//! too, and the [`Bmp280`] driver (the `embedded-hal` feature) runs the whole
//! sequence, reset to reading, over an I2C or SPI bus.

#[cfg(feature = "embedded-hal")]
mod driver;

#[cfg(feature = "embedded-hal")]
pub use driver::{Bmp280, STATUS_POLLS};

/// The I2C address with the SDO pin tied to ground.
pub const I2C_ADDRESS_PRIMARY: u8 = 0x76;
/// The I2C address with the SDO pin tied to VDDIO.
pub const I2C_ADDRESS_SECONDARY: u8 = 0x77;
/// The value the chip-id register (0xD0) returns for a BMP280.
pub const CHIP_ID: u8 = 0x58;
/// The word written to the reset register (0xE0) to run a full power-on reset.
pub const RESET_WORD: u8 = 0xB6;
/// The raw code a data register holds when its measurement is skipped.
pub const SKIPPED_OUTPUT: u32 = 0x80000;
/// The number of calibration bytes at [`register::CALIBRATION`].
pub const CALIBRATION_LEN: usize = 24;
/// The number of data bytes at [`register::DATA`].
pub const DATA_LEN: usize = 6;
/// The start-up time from power-on or reset until the part accepts a transfer, in
/// microseconds: Table 2's `t_startup`, 2 ms at most, which the soft reset of section
/// 4.3.2 incurs again since it runs the complete power-on-reset procedure.
pub const STARTUP_MICROS: u32 = 2_000;

/// The BMP280 register addresses.
pub mod register {
    /// First of the 24 trimming bytes, `calib00..=calib23` (0x88..=0x9F).
    pub const CALIBRATION: u8 = 0x88;
    /// Chip-id register; reads [`super::CHIP_ID`] for a BMP280.
    pub const CHIP_ID: u8 = 0xD0;
    /// Soft-reset register; write [`super::RESET_WORD`].
    pub const RESET: u8 = 0xE0;
    /// Status register: `measuring` in bit 3, `im_update` in bit 0.
    pub const STATUS: u8 = 0xF3;
    /// Measurement control register: oversampling and power mode.
    pub const CTRL_MEAS: u8 = 0xF4;
    /// Configuration register: standby time, IIR filter, and 3-wire SPI.
    pub const CONFIG: u8 = 0xF5;
    /// First of the 6 burst-read data bytes: pressure then temperature (0xF7..=0xFC).
    pub const DATA: u8 = 0xF7;
}

/// Returns whether the status register reports a conversion in progress.
///
/// # Arguments
///
/// * `status` - the byte read from [`register::STATUS`].
///
/// # Returns
///
/// `true` while a conversion runs, `false` once its results have reached the data
/// registers.
pub fn measuring(status: u8) -> bool {
    status & 0x08 != 0
}

/// Returns whether the status register reports the NVM image being copied.
///
/// # Arguments
///
/// * `status` - the byte read from [`register::STATUS`].
///
/// # Returns
///
/// `true` while the trimming data is being copied to the image registers, which
/// happens at power-on reset and before every conversion.
pub fn image_updating(status: u8) -> bool {
    status & 0x01 != 0
}

/// An oversampling setting for the pressure or temperature measurement.
///
/// Each step adds one bit of output resolution, stored in the XLSB data register.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Oversampling {
    /// The measurement is skipped and its data registers read [`SKIPPED_OUTPUT`].
    #[default]
    Skipped,
    /// A single sample, 16-bit output.
    X1,
    /// Two samples, 17-bit output.
    X2,
    /// Four samples, 18-bit output.
    X4,
    /// Eight samples, 19-bit output.
    X8,
    /// Sixteen samples, 20-bit output.
    X16,
}

impl Oversampling {
    /// Returns the three-bit `osrs_t` / `osrs_p` field value.
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

    /// Decodes a three-bit `osrs_t` / `osrs_p` field value.
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
    /// No measurements; the power-on default. Registers stay readable.
    #[default]
    Sleep,
    /// One measurement, then back to sleep.
    Forced,
    /// Continuous cycling between measurement and a standby period.
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
    /// 2000 ms.
    Ms2000,
    /// 4000 ms.
    Ms4000,
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
            Standby::Ms2000 => 0b110,
            Standby::Ms4000 => 0b111,
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
            0b110 => Standby::Ms2000,
            _ => Standby::Ms4000,
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
            Standby::Ms2000 => 2_000_000,
            Standby::Ms4000 => 4_000_000,
        }
    }
}

/// The measurement control register (0xF4, `ctrl_meas`).
///
/// The default is the register's reset state, `0x00`: both measurements skipped and
/// the device asleep.
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
/// The data sheet lists the IIR filter coefficients (off, 2, 4, 8, 16) and places the
/// field in bits 4:2, but does not publish which three-bit code selects which
/// coefficient, so the field is carried here as its raw code. Writes to this register
/// may be ignored in normal mode; write it in sleep mode.
///
/// The default is the register's reset state, `0x00`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Config {
    /// Normal-mode standby period, `t_sb[2:0]` in bits 7:5.
    pub standby: Standby,
    /// IIR filter setting, the raw `filter[2:0]` code in bits 4:2.
    pub filter: u8,
    /// Enables the 3-wire SPI interface, `spi3w_en` in bit 0.
    pub spi_3wire: bool,
}

impl Config {
    /// Packs the settings into the register byte.
    ///
    /// # Returns
    ///
    /// The byte to write to [`register::CONFIG`]. Only the low three bits of `filter`
    /// are used.
    pub fn bits(self) -> u8 {
        (self.standby.code() << 5) | ((self.filter & 0b111) << 2) | u8::from(self.spi_3wire)
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
            filter: (bits >> 2) & 0b111,
            spi_3wire: bits & 0x01 != 0,
        }
    }
}

/// The longest one measurement cycle can take, in microseconds.
///
/// 1.25 ms, plus 2.3 ms per temperature sample, plus 2.3 ms per pressure sample and
/// 0.575 ms, each term only for a measurement that is not skipped. Section 3.8.1 of
/// the data sheet publishes these maxima as Table 13, one row per recommended
/// setting, rather than as a formula; the terms are the BME280 data sheet's formula
/// without its humidity term, and they reproduce every row of the table, including
/// the minimum-rate column, which is computed from the maxima before the time column
/// rounds them (6.425 ms gives the row's 155.6 Hz where 6.4 ms would give 156.3 Hz).
/// A driver waits this long after forcing a measurement before it polls the status
/// register, which is how section 3.9 asks forced-mode readout to be timed.
///
/// # Arguments
///
/// * `temperature` - temperature oversampling.
/// * `pressure` - pressure oversampling.
///
/// # Returns
///
/// The maximum measurement time in microseconds.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::bmp280::{max_measurement_micros, Oversampling};
///
/// // Table 13's standard resolution row: pressure x4, temperature x1, 13.3 ms.
/// let micros = max_measurement_micros(Oversampling::X1, Oversampling::X4);
/// assert_eq!(micros, 13_325);
/// ```
pub fn max_measurement_micros(temperature: Oversampling, pressure: Oversampling) -> u32 {
    let mut micros = 1_250;
    if temperature != Oversampling::Skipped {
        micros += 2_300 * u32::from(temperature.factor());
    }
    if pressure != Oversampling::Skipped {
        micros += 2_300 * u32::from(pressure.factor()) + 575;
    }
    micros
}

/// The typical time one measurement cycle takes, in microseconds.
///
/// 1 ms, plus 2 ms per temperature sample, plus 2 ms per pressure sample and 0.5 ms,
/// each term only for a measurement that is not skipped: the typical column of
/// Table 13, row for row.
///
/// # Arguments
///
/// * `temperature` - temperature oversampling.
/// * `pressure` - pressure oversampling.
///
/// # Returns
///
/// The typical measurement time in microseconds.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::bmp280::{typical_measurement_micros, Oversampling};
///
/// // Table 13's ultra low power row: pressure x1, temperature x1, 5.5 ms.
/// assert_eq!(typical_measurement_micros(Oversampling::X1, Oversampling::X1), 5_500);
/// ```
pub fn typical_measurement_micros(temperature: Oversampling, pressure: Oversampling) -> u32 {
    let mut micros = 1_000;
    if temperature != Oversampling::Skipped {
        micros += 2_000 * u32::from(temperature.factor());
    }
    if pressure != Oversampling::Skipped {
        micros += 2_000 * u32::from(pressure.factor()) + 500;
    }
    micros
}

/// The per-chip trimming coefficients read from `0x88..=0x9F`.
///
/// These are programmed into non-volatile memory during production and are constant
/// for a given device, so they are read once at start-up and reused for every
/// measurement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calibration {
    /// `dig_T1`, unsigned, at 0x88/0x89.
    pub dig_t1: u16,
    /// `dig_T2`, signed, at 0x8A/0x8B.
    pub dig_t2: i16,
    /// `dig_T3`, signed, at 0x8C/0x8D.
    pub dig_t3: i16,
    /// `dig_P1`, unsigned, at 0x8E/0x8F.
    pub dig_p1: u16,
    /// `dig_P2`, signed, at 0x90/0x91.
    pub dig_p2: i16,
    /// `dig_P3`, signed, at 0x92/0x93.
    pub dig_p3: i16,
    /// `dig_P4`, signed, at 0x94/0x95.
    pub dig_p4: i16,
    /// `dig_P5`, signed, at 0x96/0x97.
    pub dig_p5: i16,
    /// `dig_P6`, signed, at 0x98/0x99.
    pub dig_p6: i16,
    /// `dig_P7`, signed, at 0x9A/0x9B.
    pub dig_p7: i16,
    /// `dig_P8`, signed, at 0x9C/0x9D.
    pub dig_p8: i16,
    /// `dig_P9`, signed, at 0x9E/0x9F.
    pub dig_p9: i16,
}

impl Calibration {
    /// Parses the coefficients from the calibration block, each word LSB first.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the 24 bytes read from `0x88..=0x9F`.
    ///
    /// # Returns
    ///
    /// The decoded calibration.
    pub fn parse(bytes: &[u8; CALIBRATION_LEN]) -> Calibration {
        let b = bytes;
        Calibration {
            dig_t1: u16::from_le_bytes([b[0], b[1]]),
            dig_t2: i16::from_le_bytes([b[2], b[3]]),
            dig_t3: i16::from_le_bytes([b[4], b[5]]),
            dig_p1: u16::from_le_bytes([b[6], b[7]]),
            dig_p2: i16::from_le_bytes([b[8], b[9]]),
            dig_p3: i16::from_le_bytes([b[10], b[11]]),
            dig_p4: i16::from_le_bytes([b[12], b[13]]),
            dig_p5: i16::from_le_bytes([b[14], b[15]]),
            dig_p6: i16::from_le_bytes([b[16], b[17]]),
            dig_p7: i16::from_le_bytes([b[18], b[19]]),
            dig_p8: i16::from_le_bytes([b[20], b[21]]),
            dig_p9: i16::from_le_bytes([b[22], b[23]]),
        }
    }

    /// Builds the calibration block a device holding these coefficients would return.
    ///
    /// # Returns
    ///
    /// The 24 bytes as they sit at `0x88..=0x9F`.
    pub fn to_bytes(&self) -> [u8; CALIBRATION_LEN] {
        let mut out = [0u8; CALIBRATION_LEN];
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
        for (chunk, word) in out.as_chunks_mut::<2>().0.iter_mut().zip(words) {
            *chunk = word.to_le_bytes();
        }
        out
    }

    /// Compensates a raw measurement into temperature and pressure.
    ///
    /// Temperature is computed first because its `t_fine` intermediate feeds the
    /// pressure formula, exactly as the reference code shares it.
    ///
    /// # Arguments
    ///
    /// * `raw` - the uncompensated 20-bit codes from the data registers.
    ///
    /// # Returns
    ///
    /// The compensated [`Reading`].
    pub fn compensate(&self, raw: &Measurement) -> Reading {
        let t_fine = self.t_fine(raw.temperature);
        Reading {
            temperature_centi_celsius: compensate_temperature(t_fine),
            pressure_q24_8: self.compensate_pressure(raw.pressure, t_fine),
        }
    }

    // bmp280_compensate_T_int32 up to t_fine. The products are formed in 64 bits so
    // no 20-bit code and coefficient pair can overflow; the sum always fits 32 bits.
    fn t_fine(&self, adc_t: u32) -> i32 {
        let adc_t = i64::from(adc_t);
        let dig_t1 = i64::from(self.dig_t1);
        let var1 = (((adc_t >> 3) - (dig_t1 << 1)) * i64::from(self.dig_t2)) >> 11;
        let near = (adc_t >> 4) - dig_t1;
        let var2 = (((near * near) >> 12) * i64::from(self.dig_t3)) >> 14;
        (var1 + var2) as i32
    }

    // bmp280_compensate_P_int64: pressure in Q24.8 pascals.
    fn compensate_pressure(&self, adc_p: u32, t_fine: i32) -> u32 {
        let mut var1 = i64::from(t_fine) - 128000;
        let mut var2 = var1 * var1 * i64::from(self.dig_p6);
        var2 += (var1 * i64::from(self.dig_p5)) << 17;
        var2 += i64::from(self.dig_p4) << 35;
        var1 =
            ((var1 * var1 * i64::from(self.dig_p3)) >> 8) + ((var1 * i64::from(self.dig_p2)) << 12);
        var1 = (((1i64 << 47) + var1) * i64::from(self.dig_p1)) >> 33;
        if var1 == 0 {
            return 0;
        }
        let mut p = 1048576 - i64::from(adc_p);
        p = (((p << 31) - var2) * 3125) / var1;
        var1 = (i64::from(self.dig_p9) * (p >> 13) * (p >> 13)) >> 25;
        var2 = (i64::from(self.dig_p8) * p) >> 19;
        p = ((p + var1 + var2) >> 8) + (i64::from(self.dig_p7) << 4);
        p as u32
    }
}

// The tail of bmp280_compensate_T_int32: hundredths of a degree Celsius.
fn compensate_temperature(t_fine: i32) -> i32 {
    ((i64::from(t_fine) * 5 + 128) >> 8) as i32
}

/// The raw, uncompensated 20-bit codes from the BMP280 data registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measurement {
    /// The 20-bit uncompensated pressure, `up[19:0]`.
    pub pressure: u32,
    /// The 20-bit uncompensated temperature, `ut[19:0]`.
    pub temperature: u32,
}

impl Measurement {
    /// Parses the six data bytes burst-read from `0xF7..=0xFC`.
    ///
    /// The order on the wire is pressure then temperature, each as MSB, LSB, and an
    /// XLSB byte whose upper nibble holds the last four bits.
    ///
    /// # Arguments
    ///
    /// * `data` - the six bytes read from the data registers.
    ///
    /// # Returns
    ///
    /// The unpacked raw measurement.
    pub fn parse(data: &[u8; DATA_LEN]) -> Measurement {
        let unpack = |msb: u8, lsb: u8, xlsb: u8| {
            (u32::from(msb) << 12) | (u32::from(lsb) << 4) | (u32::from(xlsb) >> 4)
        };
        Measurement {
            pressure: unpack(data[0], data[1], data[2]),
            temperature: unpack(data[3], data[4], data[5]),
        }
    }

    /// Builds the six data bytes a device holding these codes would return.
    ///
    /// # Returns
    ///
    /// The bytes as they sit at `0xF7..=0xFC`. Only the low 20 bits of each code are
    /// carried.
    pub fn to_bytes(&self) -> [u8; DATA_LEN] {
        let pack = |code: u32| {
            [
                ((code >> 12) & 0xFF) as u8,
                ((code >> 4) & 0xFF) as u8,
                ((code & 0x0F) << 4) as u8,
            ]
        };
        let p = pack(self.pressure);
        let t = pack(self.temperature);
        [p[0], p[1], p[2], t[0], t[1], t[2]]
    }

    /// Returns whether the pressure measurement was skipped (`osrs_p = 0`).
    pub fn pressure_skipped(&self) -> bool {
        self.pressure == SKIPPED_OUTPUT
    }

    /// Returns whether the temperature measurement was skipped (`osrs_t = 0`).
    pub fn temperature_skipped(&self) -> bool {
        self.temperature == SKIPPED_OUTPUT
    }
}

/// A compensated BMP280 reading.
///
/// The fields are the exact integer outputs of the reference compensation code; the
/// methods present them in conventional units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    /// Temperature in hundredths of a degree Celsius.
    pub temperature_centi_celsius: i32,
    /// Pressure in pascals as Q24.8 fixed point, that is units of 1/256 Pa.
    pub pressure_q24_8: u32,
}

impl Reading {
    /// Returns the temperature in degrees Celsius.
    pub fn celsius(&self) -> f32 {
        self.temperature_centi_celsius as f32 / 100.0
    }

    /// Returns the pressure in whole pascals.
    pub fn pascals(&self) -> u32 {
        self.pressure_q24_8 >> 8
    }

    /// Returns the pressure in hectopascals (millibars).
    pub fn hectopascals(&self) -> f32 {
        (f64::from(self.pressure_q24_8) / 25_600.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The temperature and pressure coefficients of the BME280 module's sample set; the
    // two parts share the dig_T and dig_P formulas verbatim, so the set exercises the
    // same arithmetic here.
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
        }
    }

    // The worked temperature example carried by the BME280 module: dig_T1 = 27504,
    // dig_T2 = 26435, dig_T3 = -1000 with adc_T = 519888 gives 25.08 degrees C. The
    // temperature formula is identical between the two parts.
    fn worked_example_calibration() -> Calibration {
        Calibration {
            dig_t1: 27504,
            dig_t2: 26435,
            dig_t3: -1000,
            ..sample_calibration()
        }
    }

    #[test]
    fn register_map_matches_table_18() {
        assert_eq!(register::CALIBRATION, 0x88);
        assert_eq!(register::CHIP_ID, 0xD0);
        assert_eq!(register::RESET, 0xE0);
        assert_eq!(register::STATUS, 0xF3);
        assert_eq!(register::CTRL_MEAS, 0xF4);
        assert_eq!(register::CONFIG, 0xF5);
        assert_eq!(register::DATA, 0xF7);
        // Table 18 reset state of the id register, and section 4.3.2's reset word.
        assert_eq!(CHIP_ID, 0x58);
        assert_eq!(RESET_WORD, 0xB6);
        // Table 17 spans calib00..calib23; section 3.9 burst-reads 0xF7 to 0xFC.
        assert_eq!(CALIBRATION_LEN, 0x9F - 0x88 + 1);
        assert_eq!(DATA_LEN, 0xFC - 0xF7 + 1);
    }

    #[test]
    fn i2c_address_follows_the_sdo_pin() {
        // Section 5.2: 1110110 with SDO to GND, 1110111 with SDO to VDDIO.
        assert_eq!(I2C_ADDRESS_PRIMARY, 0b1110110);
        assert_eq!(I2C_ADDRESS_SECONDARY, 0b1110111);
    }

    #[test]
    fn start_up_time_matches_table_2() {
        // Table 2: t_startup, time to first communication, 2 ms at most.
        assert_eq!(STARTUP_MICROS, 2_000);
    }

    #[test]
    fn measurement_times_reproduce_table_13() {
        // Table 13, one row per oversampling setting: pressure and temperature
        // oversampling, then typical and maximum measurement time in tenths of a
        // millisecond and typical and minimum rate in tenths of a hertz. The rate
        // columns pin the maxima before the time column rounds them: 6.425 ms is the
        // row's 155.6 Hz, where 6.4 ms would be 156.3 Hz.
        let rows = [
            (Oversampling::X1, Oversampling::X1, 55, 64, 1818, 1556),
            (Oversampling::X2, Oversampling::X1, 75, 87, 1333, 1146),
            (Oversampling::X4, Oversampling::X1, 115, 133, 870, 750),
            (Oversampling::X8, Oversampling::X1, 195, 225, 513, 444),
            (Oversampling::X16, Oversampling::X2, 375, 432, 267, 231),
        ];
        let tenths_of_ms = |micros: u32| (micros + 50) / 100;
        let tenths_of_hz = |micros: u32| (10_000_000 + micros / 2) / micros;
        for (pressure, temperature, typ_ms, max_ms, typ_hz, min_hz) in rows {
            let typical = typical_measurement_micros(temperature, pressure);
            let maximum = max_measurement_micros(temperature, pressure);
            assert_eq!(
                tenths_of_ms(typical),
                typ_ms,
                "{pressure:?} {temperature:?}"
            );
            assert_eq!(
                tenths_of_ms(maximum),
                max_ms,
                "{pressure:?} {temperature:?}"
            );
            assert_eq!(
                tenths_of_hz(typical),
                typ_hz,
                "{pressure:?} {temperature:?}"
            );
            assert_eq!(
                tenths_of_hz(maximum),
                min_hz,
                "{pressure:?} {temperature:?}"
            );
        }
        // The BME280 data sheet's worked example of the same terms, x1 temperature and
        // x4 pressure, is Table 13's standard resolution row.
        assert_eq!(
            typical_measurement_micros(Oversampling::X1, Oversampling::X4),
            11_500
        );
        assert_eq!(
            max_measurement_micros(Oversampling::X1, Oversampling::X4),
            13_325
        );
        let none = Oversampling::Skipped;
        assert_eq!(typical_measurement_micros(none, none), 1_000);
        assert_eq!(max_measurement_micros(none, none), 1_250);
        assert_eq!(
            max_measurement_micros(Oversampling::X1, none),
            1_250 + 2_300
        );
        assert_eq!(
            max_measurement_micros(none, Oversampling::X1),
            1_250 + 2_300 + 575
        );
    }

    #[test]
    fn status_flags_sit_in_bits_3_and_0() {
        // Table 19: measuring[0] is bit 3, im_update[0] is bit 0.
        assert!(measuring(0x08));
        assert!(!measuring(0x01));
        assert!(image_updating(0x01));
        assert!(!image_updating(0x08));
        assert!(!measuring(0x00) && !image_updating(0x00));
    }

    #[test]
    fn oversampling_codes_and_factors_match_tables_21_and_22() {
        let table = [
            (0b000, Oversampling::Skipped, 0),
            (0b001, Oversampling::X1, 1),
            (0b010, Oversampling::X2, 2),
            (0b011, Oversampling::X4, 4),
            (0b100, Oversampling::X8, 8),
            (0b101, Oversampling::X16, 16),
        ];
        for (code, setting, factor) in table {
            assert_eq!(setting.code(), code);
            assert_eq!(Oversampling::from_code(code), setting);
            assert_eq!(setting.factor(), factor);
        }
        // "101, 110, 111" (Table 22) and "101, Others" (Table 21) all mean x16.
        assert_eq!(Oversampling::from_code(0b110), Oversampling::X16);
        assert_eq!(Oversampling::from_code(0b111), Oversampling::X16);
    }

    #[test]
    fn mode_codes_match_table_10() {
        assert_eq!(Mode::Sleep.code(), 0b00);
        assert_eq!(Mode::Forced.code(), 0b01);
        assert_eq!(Mode::Normal.code(), 0b11);
        assert_eq!(Mode::from_code(0b00), Mode::Sleep);
        // "01 and 10" are both forced mode.
        assert_eq!(Mode::from_code(0b01), Mode::Forced);
        assert_eq!(Mode::from_code(0b10), Mode::Forced);
        assert_eq!(Mode::from_code(0b11), Mode::Normal);
    }

    #[test]
    fn standby_codes_and_periods_match_table_11() {
        let table = [
            (0b000, Standby::Ms0_5, 500),
            (0b001, Standby::Ms62_5, 62_500),
            (0b010, Standby::Ms125, 125_000),
            (0b011, Standby::Ms250, 250_000),
            (0b100, Standby::Ms500, 500_000),
            (0b101, Standby::Ms1000, 1_000_000),
            (0b110, Standby::Ms2000, 2_000_000),
            (0b111, Standby::Ms4000, 4_000_000),
        ];
        for (code, standby, micros) in table {
            assert_eq!(standby.code(), code);
            assert_eq!(Standby::from_code(code), standby);
            assert_eq!(standby.microseconds(), micros);
        }
    }

    #[test]
    fn ctrl_meas_packs_the_fields_of_table_20() {
        // osrs_t in bits 7:5, osrs_p in bits 4:2, mode in bits 1:0. Table 7's
        // "indoor navigation" row: x2 temperature, x16 pressure, normal mode.
        let ctrl = CtrlMeas {
            temperature: Oversampling::X2,
            pressure: Oversampling::X16,
            mode: Mode::Normal,
        };
        assert_eq!(ctrl.bits(), 0b0101_0111);
        assert_eq!(CtrlMeas::from_bits(0x57), ctrl);
        // Table 18 reset state is 0x00.
        assert_eq!(CtrlMeas::default().bits(), 0x00);
        assert_eq!(CtrlMeas::from_bits(0x00), CtrlMeas::default());
        for bits in 0..=u8::MAX {
            let decoded = CtrlMeas::from_bits(bits);
            assert_eq!(CtrlMeas::from_bits(decoded.bits()), decoded);
        }
    }

    #[test]
    fn config_packs_the_fields_of_table_23() {
        // t_sb in bits 7:5, filter in bits 4:2, spi3w_en in bit 0.
        let config = Config {
            standby: Standby::Ms62_5,
            filter: 0b100,
            spi_3wire: true,
        };
        assert_eq!(config.bits(), 0b0011_0001);
        assert_eq!(Config::from_bits(0x31), config);
        assert_eq!(Config::default().bits(), 0x00);
        // Bit 1 is reserved and is dropped on decode.
        assert_eq!(Config::from_bits(0x02), Config::default());
        for bits in 0..=u8::MAX {
            let decoded = Config::from_bits(bits);
            assert_eq!(Config::from_bits(decoded.bits()), decoded);
        }
    }

    #[test]
    fn calibration_parses_the_table_17_layout_and_round_trips() {
        // Table 17: twelve little-endian words, dig_T1..dig_T3 then dig_P1..dig_P9,
        // dig_T1 and dig_P1 unsigned and the rest signed.
        let bytes: [u8; 24] = [
            0x70, 0x6B, 0x43, 0x67, 0x18, 0xFC, 0x7D, 0x8E, 0x43, 0xD6, 0xD0, 0x0B, 0x27, 0x0B,
            0x8C, 0x00, 0xF9, 0xFF, 0x8C, 0x3C, 0xF8, 0xC6, 0x70, 0x17,
        ];
        let calib = Calibration::parse(&bytes);
        assert_eq!(
            calib,
            Calibration {
                dig_t1: 27504,
                dig_t2: 26435,
                dig_t3: -1000,
                dig_p1: 36477,
                dig_p2: -10685,
                dig_p3: 3024,
                dig_p4: 2855,
                dig_p5: 140,
                dig_p6: -7,
                dig_p7: 15500,
                dig_p8: -14600,
                dig_p9: 6000,
            }
        );
        assert_eq!(calib.to_bytes(), bytes);
        let sample = sample_calibration();
        assert_eq!(Calibration::parse(&sample.to_bytes()), sample);
    }

    #[test]
    fn measurement_parses_the_burst_read_and_round_trips() {
        // Tables 24 and 25: up[19:12], up[11:4], up[3:0] in the upper nibble, then the
        // same for ut. Pressure 0x655AC = 415148, temperature 0x7EED0 = 519888.
        let data = [0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00];
        let raw = Measurement::parse(&data);
        assert_eq!(raw.pressure, 415_148);
        assert_eq!(raw.temperature, 519_888);
        assert_eq!(raw.to_bytes(), data);
        // The low nibble of each XLSB byte is not part of the code.
        assert_eq!(
            Measurement::parse(&[0x65, 0x5A, 0xCF, 0x7E, 0xED, 0x0F]),
            raw
        );
        assert_eq!(
            Measurement::parse(&[0xFF; 6]).to_bytes(),
            [0xFF, 0xFF, 0xF0, 0xFF, 0xFF, 0xF0]
        );
    }

    #[test]
    fn skipped_measurements_read_as_0x80000() {
        // Tables 21 and 22: a skipped measurement sets its output to 0x80000, which
        // is also the Table 18 reset state of the data registers.
        let raw = Measurement::parse(&[0x80, 0x00, 0x00, 0x80, 0x00, 0x00]);
        assert_eq!(raw.pressure, SKIPPED_OUTPUT);
        assert!(raw.pressure_skipped() && raw.temperature_skipped());
        assert!(!Measurement::parse(&[0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00]).pressure_skipped());
    }

    #[test]
    fn temperature_matches_the_worked_example() {
        // 25.08 degrees C for adc_T = 519888, as in the BME280 module's anchor; the
        // shift-based integer path lands exactly on the 128422 t_fine that example
        // quotes.
        let calib = worked_example_calibration();
        let t_fine = calib.t_fine(519_888);
        assert_eq!(t_fine, 128_422);
        assert_eq!(compensate_temperature(t_fine), 2508);
        let reading = calib.compensate(&Measurement {
            pressure: 415_148,
            temperature: 519_888,
        });
        assert_eq!(reading.temperature_centi_celsius, 2508);
        assert!((reading.celsius() - 25.08).abs() < 0.001);
    }

    #[test]
    fn reference_vector_pins_the_integer_output() {
        // The exact output of the section 3.11.3 code for the Table 17 layout bytes in
        // calibration_parses_the_table_17_layout_and_round_trips and the burst read
        // 65 5A C0 7E ED 00. Both temperature functions store t_fine = 128422 for this
        // input, so the appendix 8.1 double form is a like-for-like check here: it
        // gives 100653.26 Pa, and the Q24.8 value below is within 0.01 Pa of it.
        let calib = Calibration {
            dig_t1: 27504,
            dig_t2: 26435,
            dig_t3: -1000,
            dig_p1: 36477,
            dig_p2: -10685,
            dig_p3: 3024,
            dig_p4: 2855,
            dig_p5: 140,
            dig_p6: -7,
            dig_p7: 15500,
            dig_p8: -14600,
            dig_p9: 6000,
        };
        let raw = Measurement::parse(&[0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00]);
        let reading = calib.compensate(&raw);
        assert_eq!(reading.temperature_centi_celsius, 2508);
        assert_eq!(reading.pressure_q24_8, 25_767_233);
        assert_eq!(reading.pascals(), 100_653);
        let (_, t_fine_ref) = reference_temperature(&calib, raw.temperature);
        assert_eq!(calib.t_fine(raw.temperature), 128_422);
        assert_eq!(t_fine_ref, 128_422);
        let p_ref = reference_pressure(&calib, raw.pressure, t_fine_ref);
        assert!((f64::from(reading.pressure_q24_8) / 256.0 - p_ref).abs() < 0.01);
    }

    #[test]
    fn integer_compensation_tracks_the_floating_point_reference() {
        // Sweep raw codes across several coefficient sets and require the section
        // 3.11.3 integer path to agree with the appendix 8.1 double path to within
        // rounding. The two temperature functions round t_fine differently (the int32
        // path floors three shifts, the double casts once), at most three counts for
        // these coefficients, and the pressure formula moves about 0.03 Pa per count;
        // so the pressure functions are compared on the same signed 32-bit t_fine, the
        // carry-over the appendix defines them to share.
        for calib in [sample_calibration(), worked_example_calibration()] {
            for adc_t in [400_000, 450_000, 500_000, 519_888, 540_000, 600_000] {
                let t_fine = calib.t_fine(adc_t);
                let t_int = compensate_temperature(t_fine);
                let (t_ref, t_fine_ref) = reference_temperature(&calib, adc_t);
                assert!(
                    (t_fine - t_fine_ref).abs() <= 3,
                    "t_fine {t_fine} vs {t_fine_ref}"
                );
                assert!(
                    (f64::from(t_int) / 100.0 - t_ref).abs() < 0.02,
                    "temperature {t_int} vs {t_ref}"
                );
                for adc_p in [300_000, 350_000, 415_148, 450_000, 512_000] {
                    let p_int = calib.compensate_pressure(adc_p, t_fine);
                    let p_ref = reference_pressure(&calib, adc_p, t_fine);
                    assert!(
                        (f64::from(p_int) / 256.0 - p_ref).abs() < 0.05,
                        "pressure {p_int} (Q24.8) vs {p_ref} Pa"
                    );
                }
            }
        }
    }

    #[test]
    fn any_20_bit_code_pair_compensates_without_overflow() {
        let calib = sample_calibration();
        for adc_t in (0..=0xF_FFFF).step_by(0x1_0000).chain([0xF_FFFF]) {
            for adc_p in (0..=0xF_FFFF).step_by(0x1_0000).chain([0xF_FFFF]) {
                let _ = calib.compensate(&Measurement {
                    pressure: adc_p,
                    temperature: adc_t,
                });
            }
        }
    }

    #[test]
    fn zero_dig_p1_takes_the_division_guard() {
        // Section 3.11.3 returns 0 from bmp280_compensate_P_int64 when its var1 is
        // zero, "to avoid exception caused by division by zero", and appendix 8.1 does
        // the same; a dig_P1 of 0 reaches that branch for any raw code.
        let base = sample_calibration();
        let calib = Calibration { dig_p1: 0, ..base };
        let raw = Measurement::parse(&[0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00]);
        let reading = calib.compensate(&raw);
        assert_eq!(reading.pressure_q24_8, 0);
        assert_eq!(reading.pascals(), 0);
        let expected = base.compensate(&raw).temperature_centi_celsius;
        assert_eq!(reading.temperature_centi_celsius, expected);
        let t_fine = calib.t_fine(raw.temperature);
        assert_eq!(reference_pressure(&calib, raw.pressure, t_fine), 0.0);
    }

    #[test]
    fn reading_unit_helpers_match_the_reference_code_comments() {
        // Section 3.11.3: "24674867" in Q24.8 is 96386.2 Pa = 963.862 hPa, and a
        // temperature output of "5123" equals 51.23 degrees C.
        let reading = Reading {
            temperature_centi_celsius: 5123,
            pressure_q24_8: 24_674_867,
        };
        assert!((reading.celsius() - 51.23).abs() < 0.001);
        assert_eq!(reading.pascals(), 96_386);
        assert!((reading.hectopascals() - 963.862).abs() < 0.001);
    }

    // bmp280_compensate_T_double from appendix 8.1, used only to validate the integer
    // path above. It returns the Celsius output and the signed 32-bit t_fine the
    // appendix stores for its pressure function.
    fn reference_temperature(c: &Calibration, adc_t: u32) -> (f64, i32) {
        let adc_t = f64::from(adc_t);
        let var1 = (adc_t / 16384.0 - f64::from(c.dig_t1) / 1024.0) * f64::from(c.dig_t2);
        let var2 = {
            let v = adc_t / 131072.0 - f64::from(c.dig_t1) / 8192.0;
            v * v * f64::from(c.dig_t3)
        };
        ((var1 + var2) / 5120.0, (var1 + var2) as i32)
    }

    // bmp280_compensate_P_double from appendix 8.1, reading the signed 32-bit t_fine
    // its temperature function stored.
    fn reference_pressure(c: &Calibration, adc_p: u32, t_fine: i32) -> f64 {
        let mut var1 = (f64::from(t_fine) / 2.0) - 64000.0;
        let mut var2 = var1 * var1 * f64::from(c.dig_p6) / 32768.0;
        var2 += var1 * f64::from(c.dig_p5) * 2.0;
        var2 = (var2 / 4.0) + (f64::from(c.dig_p4) * 65536.0);
        var1 =
            (f64::from(c.dig_p3) * var1 * var1 / 524288.0 + f64::from(c.dig_p2) * var1) / 524288.0;
        var1 = (1.0 + var1 / 32768.0) * f64::from(c.dig_p1);
        if var1 == 0.0 {
            return 0.0;
        }
        let mut p = 1048576.0 - f64::from(adc_p);
        p = (p - (var2 / 4096.0)) * 6250.0 / var1;
        var1 = f64::from(c.dig_p9) * p * p / 2147483648.0;
        var2 = p * f64::from(c.dig_p8) / 32768.0;
        p + (var1 + var2 + f64::from(c.dig_p7)) / 16.0
    }
}
