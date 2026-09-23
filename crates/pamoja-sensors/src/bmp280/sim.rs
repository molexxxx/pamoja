//! A BMP280 that is not there.
//!
//! A BMP280 is a BME280 without humidity: the same temperature and pressure registers and the
//! same calibration layout. So [`part`] holds the temperature and pressure half of the real
//! part the [BME280 simulation](crate::bme280::sim) was read from, and reads 20.44 C and
//! 848.05 hPa. [`reporting`] reads whatever it is asked to, against the BMP280's own
//! compensation.

use pamoja_hal::sim::I2cPart;

use super::{register, Calibration, Measurement, CALIBRATION_LEN, CHIP_ID, DATA_LEN};
use crate::bme280::sim::{self as bme280, rounded, solve};

/// The status a part reports when it is neither measuring nor loading its calibration.
pub const STATUS_IDLE: u8 = 0x00;

/// The 24 trimming bytes [`part`] holds from 0x88: the temperature and pressure block of the
/// real part the BME280 simulation was read from.
pub const CALIBRATION: [u8; CALIBRATION_LEN] = leading(&bme280::CALIBRATION);

/// The six data registers [`part`] holds from 0xF7: the pressure and temperature of the one
/// measurement that part took.
pub const BURST: [u8; DATA_LEN] = leading(&bme280::BURST);

/// The calibration [`part`] holds.
///
/// # Returns
///
/// The constants a measurement is compensated against.
#[must_use]
pub fn calibration() -> Calibration {
    Calibration::parse(&CALIBRATION)
}

// The first N bytes of a longer block, in a constant.
const fn leading<const N: usize, const M: usize>(block: &[u8; M]) -> [u8; N] {
    let mut out = [0u8; N];
    let mut index = 0;
    while index < N {
        out[index] = block[index];
        index += 1;
    }
    out
}

/// A BMP280 answering at an address, holding a real calibration and one measurement.
///
/// # Arguments
///
/// * `address` - the address it answers to, usually
///   [`I2C_ADDRESS_PRIMARY`](super::I2C_ADDRESS_PRIMARY).
///
/// # Returns
///
/// The part, which compensates to 20.44 C and 848.05 hPa.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::DelayLog;
/// use pamoja_sensors::bmp280::{sim, Bmp280, I2C_ADDRESS_PRIMARY};
///
/// let mut sensor = Bmp280::i2c(sim::part(I2C_ADDRESS_PRIMARY), I2C_ADDRESS_PRIMARY, DelayLog::new());
/// let reading = sensor.measure().expect("the part answers");
/// assert_eq!(reading.celsius(), 20.44);
/// ```
#[must_use]
pub fn part(address: u8) -> I2cPart {
    I2cPart::new(address)
        .holding(register::CHIP_ID, &[CHIP_ID])
        .holding(register::STATUS, &[STATUS_IDLE])
        .holding(register::CALIBRATION, &CALIBRATION)
        .holding(register::DATA, &BURST)
}

/// A BMP280 that reads what it is asked to.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `celsius` - the temperature it reports.
/// * `hectopascals` - the pressure it reports.
///
/// # Returns
///
/// The part. Its readings land within a hundredth of a degree and a hundredth of a
/// hectopascal of what was asked for.
#[must_use]
pub fn reporting(address: u8, celsius: f32, hectopascals: f32) -> I2cPart {
    part(address).holding(register::DATA, &burst_for(celsius, hectopascals))
}

/// The six data registers that compensate to a reading against [`calibration`].
///
/// # Arguments
///
/// * `celsius` - the temperature to land on.
/// * `hectopascals` - the pressure to land on.
///
/// # Returns
///
/// The bytes a burst read returns, found by searching, since the compensation only runs one
/// way. The search reads the compensated pressure as signed: at the far end of the
/// converter's range the reference arithmetic goes below zero and wraps when it is stored
/// unsigned.
#[must_use]
pub fn burst_for(celsius: f32, hectopascals: f32) -> [u8; DATA_LEN] {
    const RAW_MAX: i32 = 0x000f_ffff;
    let calibration = calibration();
    let raw_of = |temperature: i32, pressure: i32| Measurement {
        pressure: pressure as u32,
        temperature: temperature as u32,
    };
    let wanted_temperature = i64::from(rounded(celsius * 100.0));
    let wanted_pressure = i64::from(rounded(hectopascals * 25_600.0));

    let temperature = solve(wanted_temperature, 0, RAW_MAX, |raw| {
        i64::from(
            calibration
                .compensate(&raw_of(raw, RAW_MAX / 2))
                .temperature_centi_celsius,
        )
    });
    let pressure = solve(wanted_pressure, 0, RAW_MAX, |raw| {
        i64::from(
            calibration
                .compensate(&raw_of(temperature, raw))
                .pressure_q24_8 as i32,
        )
    });
    raw_of(temperature, pressure).to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bmp280::{Bmp280, I2C_ADDRESS_SECONDARY};
    use pamoja_hal::script::DelayLog;

    #[test]
    fn the_driver_reads_the_shared_half_of_the_real_part() {
        let mut sensor = Bmp280::i2c(
            part(I2C_ADDRESS_SECONDARY),
            I2C_ADDRESS_SECONDARY,
            DelayLog::new(),
        );
        let reading = sensor.measure().expect("a reading");
        assert_eq!(reading.celsius(), 20.44);
        assert!((reading.hectopascals() - 848.05).abs() < 0.01);
    }

    #[test]
    fn a_part_reports_what_it_was_asked_to() {
        for (celsius, hectopascals) in [(-7.5f32, 1003.0f32), (31.25, 940.5), (-38.2, 1080.0)] {
            let mut sensor = Bmp280::i2c(
                reporting(I2C_ADDRESS_SECONDARY, celsius, hectopascals),
                I2C_ADDRESS_SECONDARY,
                DelayLog::new(),
            );
            let reading = sensor.measure().expect("a reading");
            assert!(
                (reading.celsius() - celsius).abs() < 0.01,
                "{celsius} C read back as {}",
                reading.celsius()
            );
            assert!(
                (reading.hectopascals() - hectopascals).abs() < 0.01,
                "{hectopascals} hPa read back as {}",
                reading.hectopascals()
            );
        }
    }
}
