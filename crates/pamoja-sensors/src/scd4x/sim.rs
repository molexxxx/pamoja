//! An SCD40 or SCD41 that is not there.
//!
//! [`part`] answers the way an SCD4x reading 800 ppm at 22.5 C and 45 % does, and
//! [`reporting`] at any reading. It answers at the one address the part has,
//! [`I2C_ADDRESS`](super::I2C_ADDRESS), and takes Sensirion's 16-bit commands: the serial
//! number, the data-ready status, which always says a reading is waiting, and the reading
//! itself, each word with its CRC; starting, stopping, and the other commands answer with
//! nothing.

use pamoja_hal::sim::CommandPart;

use super::{
    command, command_frame, data_ready, serial_number_frame, word_frame, Measurement, I2C_ADDRESS,
};

/// The carbon dioxide concentration [`part`] reports, in parts per million.
pub const CO2_PPM: u16 = 800;

/// The temperature [`part`] reports.
pub const CELSIUS: f32 = 22.5;

/// The relative humidity [`part`] reports, as a percentage.
pub const RELATIVE_HUMIDITY: f32 = 45.0;

/// The serial number every simulated part reports.
pub const SERIAL: u64 = 0x0000_5A4D_0C1E_2B3F;

/// The data-ready word the part answers with: a reading is waiting.
const DATA_READY: u16 = 0x0001;

/// An SCD4x reading [`CO2_PPM`], [`CELSIUS`], and [`RELATIVE_HUMIDITY`].
///
/// # Returns
///
/// The part.
#[must_use]
pub fn part() -> CommandPart {
    reporting(CO2_PPM, CELSIUS, RELATIVE_HUMIDITY)
}

/// An SCD4x that reads what it is asked to.
///
/// # Arguments
///
/// * `co2_ppm` - the carbon dioxide concentration it reports, in parts per million.
/// * `celsius` - the temperature it reports.
/// * `relative_humidity` - the humidity it reports, as a percentage.
///
/// # Returns
///
/// The part. Its temperature and humidity land on the nearest steps its 16-bit words
/// represent.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::DelayLog;
/// use pamoja_sensors::scd4x::{sim, Scd4x};
///
/// let mut sensor = Scd4x::new(sim::reporting(1_450, 24.0, 55.0), DelayLog::new());
/// let reading = sensor.measure().expect("the part answers");
/// assert_eq!(reading.co2_ppm, 1_450);
/// ```
#[must_use]
pub fn reporting(co2_ppm: u16, celsius: f32, relative_humidity: f32) -> CommandPart {
    let milli_celsius = rounded(celsius * 1000.0);
    let milli_percent = rounded(relative_humidity.max(0.0) * 1000.0) as u32;
    let reading = Measurement::from_physical(co2_ppm, milli_celsius, milli_percent).to_bytes();
    CommandPart::new(I2C_ADDRESS, 2)
        .answering(
            &command_frame(command::GET_SERIAL_NUMBER),
            &serial_number_frame(SERIAL),
        )
        .answering(
            &command_frame(command::GET_DATA_READY_STATUS),
            &word_frame(DATA_READY),
        )
        .answering(&command_frame(command::READ_MEASUREMENT), &reading)
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
    use crate::scd4x::Scd4x;
    use pamoja_hal::script::DelayLog;

    #[test]
    fn the_word_says_a_reading_is_waiting() {
        assert!(data_ready(DATA_READY));
    }

    #[test]
    fn the_driver_starts_the_part_and_reads_it() {
        let mut sensor = Scd4x::new(part(), DelayLog::new());
        let reading = sensor.measure().expect("a reading");
        assert_eq!(reading.co2_ppm, CO2_PPM);
        assert!((reading.celsius() - CELSIUS).abs() < 0.003);
        assert!((reading.relative_humidity_percent() - RELATIVE_HUMIDITY).abs() < 0.002);
        assert_eq!(sensor.serial(), Some(SERIAL));

        let mut single = Scd4x::new(reporting(3_000, -5.0, 20.0), DelayLog::new());
        let reading = single.measure_single_shot().expect("a reading");
        assert_eq!(reading.co2_ppm, 3_000);
        assert!((reading.celsius() + 5.0).abs() < 0.003);
    }
}
