//! An SHT3x that is not there.
//!
//! [`part`] answers the way an SHT3x at 22.5 C and 45 % does, and [`reporting`] at any
//! temperature and humidity. It takes Sensirion's 16-bit commands: every single-shot
//! measurement, with or without clock stretching, and a fetch in periodic mode, answer with
//! the reading and its CRCs; the status command answers with the status a part reports after
//! a reset; a reset, the heater, and the rest answer with nothing.

use pamoja_hal::sim::CommandPart;

use super::{
    command, humidity_raw_from_relative_humidity, single_shot, temperature_raw_from_celsius,
    Measurement, Repeatability, Status,
};

/// The temperature [`part`] reports.
pub const CELSIUS: f32 = 22.5;

/// The relative humidity [`part`] reports, as a percentage.
pub const RELATIVE_HUMIDITY: f32 = 45.0;

/// An SHT3x answering at an address, reading [`CELSIUS`] and [`RELATIVE_HUMIDITY`].
///
/// # Arguments
///
/// * `address` - the address it answers to, [`I2C_ADDRESS_A`](super::I2C_ADDRESS_A) with
///   ADDR low.
///
/// # Returns
///
/// The part.
#[must_use]
pub fn part(address: u8) -> CommandPart {
    reporting(address, CELSIUS, RELATIVE_HUMIDITY)
}

/// An SHT3x that reads what it is asked to.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `celsius` - the temperature it reports.
/// * `relative_humidity` - the humidity it reports, as a percentage.
///
/// # Returns
///
/// The part. Its readings land on the nearest steps its 16-bit words represent, within
/// three thousandths of a degree and two thousandths of a percent.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::DelayLog;
/// use pamoja_sensors::sht3x::{sim, Sht3x, I2C_ADDRESS_A};
///
/// let mut sensor = Sht3x::new(sim::reporting(I2C_ADDRESS_A, 30.0, 70.0), I2C_ADDRESS_A, DelayLog::new());
/// let reading = sensor.measure().expect("the part answers");
/// assert!((reading.temperature_celsius() - 30.0).abs() < 0.01);
/// ```
#[must_use]
pub fn reporting(address: u8, celsius: f32, relative_humidity: f32) -> CommandPart {
    let reading = Measurement {
        temperature_raw: temperature_raw_from_celsius(celsius),
        humidity_raw: humidity_raw_from_relative_humidity(relative_humidity),
    }
    .to_bytes();
    let status = Status::from_bits(Status::DEFAULT).to_bytes();
    let mut part = CommandPart::new(address, 2)
        .answering(&command::READ_STATUS.to_be_bytes(), &status)
        .answering(&command::FETCH_DATA.to_be_bytes(), &reading);
    for repeatability in [Repeatability::Low, Repeatability::Medium, Repeatability::High] {
        for stretching in [false, true] {
            part.answer(
                &single_shot(repeatability, stretching).to_be_bytes(),
                &reading,
            );
        }
    }
    part
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sht3x::{Sht3x, I2C_ADDRESS_A, I2C_ADDRESS_B};
    use pamoja_hal::script::DelayLog;

    #[test]
    fn the_driver_resets_the_part_and_reads_it() {
        let mut sensor = Sht3x::new(part(I2C_ADDRESS_A), I2C_ADDRESS_A, DelayLog::new());
        let reading = sensor.measure().expect("a reading");
        assert!((reading.temperature_celsius() - CELSIUS).abs() < 0.003);
        assert!((reading.relative_humidity() - RELATIVE_HUMIDITY).abs() < 0.002);
        assert_eq!(
            sensor.status().map(|status| status.bits()),
            Some(Status::DEFAULT)
        );

        let (part, _) = sensor.release();
        assert_eq!(
            part.received()[0],
            command::SOFT_RESET.to_be_bytes(),
            "the reset went first"
        );
    }

    #[test]
    fn a_part_reports_what_it_was_asked_to() {
        let mut sensor = Sht3x::new(
            reporting(I2C_ADDRESS_B, -12.0, 88.5),
            I2C_ADDRESS_B,
            DelayLog::new(),
        );
        let reading = sensor.measure().expect("a reading");
        assert!((reading.temperature_celsius() + 12.0).abs() < 0.003);
        assert!((reading.relative_humidity() - 88.5).abs() < 0.002);
    }
}
