//! An HDC1080 that is not there.
//!
//! [`part`] answers the way an HDC1080 at 22.5 C and 45 % does, and [`reporting`] at any
//! temperature and humidity. It answers at the one address the part has,
//! [`I2C_ADDRESS`](super::I2C_ADDRESS), and a measurement reads temperature then humidity in
//! one four-byte read, as the part's does.

use pamoja_hal::sim::WordPart;

use super::{
    humidity_register, register, temperature_register, DEVICE_ID, I2C_ADDRESS, MANUFACTURER_ID,
};

/// The temperature [`part`] reports.
pub const CELSIUS: f32 = 22.5;

/// The relative humidity [`part`] reports, as a percentage.
pub const RELATIVE_HUMIDITY: f32 = 45.0;

/// An HDC1080 reading [`CELSIUS`] and [`RELATIVE_HUMIDITY`].
///
/// # Returns
///
/// The part.
#[must_use]
pub fn part() -> WordPart {
    reporting(CELSIUS, RELATIVE_HUMIDITY)
}

/// An HDC1080 that reads what it is asked to.
///
/// # Arguments
///
/// * `celsius` - the temperature it reports.
/// * `relative_humidity` - the humidity it reports, as a percentage.
///
/// # Returns
///
/// The part. Its readings land on the nearest steps its 14-bit converters represent, within
/// three thousandths of a degree and two thousandths of a percent.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::DelayLog;
/// use pamoja_sensors::hdc1080::{sim, Hdc1080};
///
/// let mut sensor = Hdc1080::new(sim::reporting(4.0, 91.0), DelayLog::new());
/// let reading = sensor.measure().expect("the part answers");
/// assert!((reading.celsius() - 4.0).abs() < 0.01);
/// ```
#[must_use]
pub fn reporting(celsius: f32, relative_humidity: f32) -> WordPart {
    let milli_celsius = rounded(celsius * 1000.0);
    let milli_percent = rounded(relative_humidity.max(0.0) * 1000.0) as u32;
    WordPart::new(I2C_ADDRESS)
        .holding(register::MANUFACTURER_ID, MANUFACTURER_ID)
        .holding(register::DEVICE_ID, DEVICE_ID)
        .holding(register::TEMPERATURE, temperature_register(milli_celsius))
        .holding(register::HUMIDITY, humidity_register(milli_percent))
}

// Rounds away from zero, since a part cannot call `f32::round` without an operating system.
fn rounded(value: f32) -> i32 {
    if value >= 0.0 {
        (value + 0.5) as i32
    } else {
        (value - 0.5) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hdc1080::Hdc1080;
    use pamoja_hal::script::DelayLog;

    #[test]
    fn the_driver_reads_what_the_part_holds() {
        let mut sensor = Hdc1080::new(part(), DelayLog::new());
        let reading = sensor.measure().expect("a reading");
        assert!((reading.celsius() - CELSIUS).abs() < 0.003);
        assert!((reading.relative_humidity() - RELATIVE_HUMIDITY).abs() < 0.002);

        let mut cold = Hdc1080::new(reporting(-20.0, 12.5), DelayLog::new());
        let reading = cold.measure().expect("a reading");
        assert!((reading.celsius() + 20.0).abs() < 0.003);
        assert!((reading.relative_humidity() - 12.5).abs() < 0.002);
    }
}
