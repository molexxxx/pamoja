//! A BME280 that is not there.
//!
//! Standing a part up to run a driver against used to mean writing out its side of the
//! conversation: the chip id, two calibration blocks, a status register, and eight burst
//! bytes, as literals, in the order the datasheet lists them. That is a lot of bytes to get
//! right before a driver has been run once, and it is the same bytes every time.
//!
//! This is that part, already answering. [`part`] hands back something a [`Bme280`](super::Bme280) can be
//! pointed at, holding a real calibration and a reading. [`reporting`] builds one that reads
//! whatever you ask it for, by working out the raw values the compensation turns back into
//! that reading.
//!
//! The calibration here was read from a real BME280, and [`BURST`] is one measurement that
//! part took. Together they compensate to 20.44 C, 848.05 hPa, and 44.65 %.

use pamoja_hal::sim::I2cPart;

use super::{
    register, Calibration, RawMeasurement, CALIB_HUMIDITY_LEN, CALIB_TEMP_PRESS_LEN, CHIP_ID,
    DATA_LEN,
};

/// The temperature and pressure calibration a real BME280 holds.
pub const CALIBRATION: [u8; CALIB_TEMP_PRESS_LEN] = [
    0x45, 0x6F, 0x6F, 0x68, 0x32, 0x00, 0x46, 0x91, 0x6A, 0xD6, 0xD0, 0x0B, 0x4E, 0x1E, 0x88, 0xFF,
    0xF9, 0xFF, 0xAC, 0x26, 0x0A, 0xD8, 0xBD, 0x10, 0x00, 0x4B,
];

/// The humidity calibration that goes with it.
pub const CALIBRATION_HUMIDITY: [u8; CALIB_HUMIDITY_LEN] =
    [0x62, 0x01, 0x00, 0x15, 0x23, 0x03, 0x1E];

/// One measurement that part took, as the eight data registers hold it.
///
/// Against [`CALIBRATION`] it compensates to 20.44 C, 848.05 hPa, and 44.65 %.
pub const BURST: [u8; DATA_LEN] = [0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x75, 0x30];

/// The status a part reports when it is neither measuring nor loading its calibration.
pub const STATUS_IDLE: u8 = 0x00;

/// The calibration these registers decode to.
///
/// # Returns
///
/// The constants a measurement is compensated against.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::bme280::sim;
///
/// let calibration = sim::calibration();
/// let reading = calibration.compensate(&pamoja_sensors::bme280::RawMeasurement::from_registers(
///     &sim::BURST,
/// ));
/// assert_eq!(reading.temperature_centi_celsius, 2044);
/// ```
#[must_use]
pub fn calibration() -> Calibration {
    Calibration::from_registers(&CALIBRATION, &CALIBRATION_HUMIDITY)
}

/// A BME280 answering at an address, holding a real calibration and one measurement.
///
/// # Arguments
///
/// * `address` - the address it answers to, usually
///   [`I2C_ADDRESS_PRIMARY`](super::I2C_ADDRESS_PRIMARY).
///
/// # Returns
///
/// The part, ready for a driver to reset, identify, read, and measure.
///
/// # Examples
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_hal::script::{block_on, DelayLog};
/// use pamoja_sensors::bme280::{sim, Bme280, I2C_ADDRESS_PRIMARY};
///
/// let mut sensor = Bme280::i2c(sim::part(I2C_ADDRESS_PRIMARY), I2C_ADDRESS_PRIMARY, DelayLog::new());
/// let reading = block_on(sensor.read()).expect("the part answers");
///
/// assert_eq!(reading.celsius(), 20.44);
/// ```
#[must_use]
pub fn part(address: u8) -> I2cPart {
    I2cPart::new(address)
        .holding(register::CHIP_ID, &[CHIP_ID])
        .holding(register::STATUS, &[STATUS_IDLE])
        .holding(register::CALIB_TEMP_PRESS, &CALIBRATION)
        .holding(register::CALIB_HUMIDITY, &CALIBRATION_HUMIDITY)
        .holding(register::DATA, &BURST)
}

/// A BME280 that reads what you ask it to.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `celsius` - the temperature it should report.
/// * `hectopascals` - the pressure it should report.
/// * `relative_humidity` - the humidity it should report, as a percentage.
///
/// # Returns
///
/// The part. The readings land on the nearest ones its converter can represent, which is
/// within a hundredth of a degree, a hundredth of a hectopascal, and a thousandth of a
/// percent of what was asked for.
///
/// # Examples
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_hal::script::{block_on, DelayLog};
/// use pamoja_sensors::bme280::{sim, Bme280, I2C_ADDRESS_PRIMARY};
///
/// // A cold store at four degrees, which is the reading a driver should see.
/// let cold = sim::reporting(I2C_ADDRESS_PRIMARY, 4.0, 1013.25, 80.0);
/// let mut sensor = Bme280::i2c(cold, I2C_ADDRESS_PRIMARY, DelayLog::new());
///
/// let reading = block_on(sensor.read()).expect("the part answers");
/// assert_eq!(reading.celsius(), 4.0);
/// assert!((reading.hectopascals() - 1013.25).abs() < 0.01);
/// ```
#[must_use]
pub fn reporting(address: u8, celsius: f32, hectopascals: f32, relative_humidity: f32) -> I2cPart {
    part(address).holding(
        register::DATA,
        &burst_for(celsius, hectopascals, relative_humidity),
    )
}

/// The eight data registers that compensate to a reading.
///
/// # Arguments
///
/// * `celsius` - the temperature to land on.
/// * `hectopascals` - the pressure to land on.
/// * `relative_humidity` - the humidity to land on, as a percentage.
///
/// # Returns
///
/// The bytes, against [`CALIBRATION`]. They are the converter values the compensation turns
/// back into that reading, found by searching, because the formulas only run one way.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::bme280::{sim, RawMeasurement};
///
/// let burst = sim::burst_for(20.44, 848.05, 44.65);
/// let reading = sim::calibration().compensate(&RawMeasurement::from_registers(&burst));
/// assert_eq!(reading.temperature_centi_celsius, 2044);
/// ```
#[must_use]
pub fn burst_for(celsius: f32, hectopascals: f32, relative_humidity: f32) -> [u8; DATA_LEN] {
    const TEMPERATURE_MAX: i32 = 0x000f_ffff;
    const PRESSURE_MAX: i32 = 0x000f_ffff;
    const HUMIDITY_MAX: i32 = 0x0000_ffff;

    let calibration = calibration();
    let wanted_temperature = i64::from(rounded(celsius * 100.0));
    let wanted_pressure = i64::from(rounded(hectopascals * 10_000.0));
    let wanted_humidity = i64::from(rounded(relative_humidity * 1024.0));

    // The compensation folds temperature into the other two, so it is settled first and
    // then held while they are.
    let temperature = solve(wanted_temperature, 0, TEMPERATURE_MAX, |raw| {
        i64::from(
            calibration
                .compensate(&raw_of(raw, PRESSURE_MAX / 2, HUMIDITY_MAX / 2))
                .temperature_centi_celsius,
        )
    });
    let pressure = solve(wanted_pressure, 0, PRESSURE_MAX, |raw| {
        i64::from(
            calibration
                .compensate(&raw_of(temperature, raw, HUMIDITY_MAX / 2))
                .pressure_centi_pascals,
        )
    });
    let humidity = solve(wanted_humidity, 0, HUMIDITY_MAX, |raw| {
        i64::from(
            calibration
                .compensate(&raw_of(temperature, pressure, raw))
                .humidity_q22_10,
        )
    });

    registers_of(&raw_of(temperature, pressure, humidity))
}

/// The eight data registers a raw measurement sits in.
///
/// # Arguments
///
/// * `raw` - the converter values.
///
/// # Returns
///
/// The bytes, which is what [`RawMeasurement::from_registers`] reads back.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::bme280::{sim, RawMeasurement};
///
/// let raw = RawMeasurement::from_registers(&sim::BURST);
/// assert_eq!(sim::registers_of(&raw), sim::BURST, "what it reads is what it holds");
/// ```
#[must_use]
pub fn registers_of(raw: &RawMeasurement) -> [u8; DATA_LEN] {
    [
        (raw.pressure >> 12) as u8,
        (raw.pressure >> 4) as u8,
        ((raw.pressure << 4) & 0xf0) as u8,
        (raw.temperature >> 12) as u8,
        (raw.temperature >> 4) as u8,
        ((raw.temperature << 4) & 0xf0) as u8,
        (raw.humidity >> 8) as u8,
        raw.humidity as u8,
    ]
}

const fn raw_of(temperature: i32, pressure: i32, humidity: i32) -> RawMeasurement {
    RawMeasurement {
        temperature,
        pressure,
        humidity,
    }
}

// Rounds away from zero, since a part cannot call `f32::round` without an operating system.
pub(crate) fn rounded(value: f32) -> i32 {
    if value >= 0.0 {
        (value + 0.5) as i32
    } else {
        (value - 0.5) as i32
    }
}

// The converter value a reading came from. The compensation only runs one way and is
// monotonic, so this walks in from both ends; which way it leans is read off the ends
// rather than assumed, because pressure falls as its converter value rises.
pub(crate) fn solve(wanted: i64, low: i32, high: i32, measured: impl Fn(i32) -> i64) -> i32 {
    let rising = measured(high) >= measured(low);
    let (mut low, mut high) = (low, high);
    while low < high {
        let middle = low + (high - low) / 2;
        let under = if rising {
            measured(middle) < wanted
        } else {
            measured(middle) > wanted
        };
        if under {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    low
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bme280::{Bme280, I2C_ADDRESS_PRIMARY};
    use pamoja_core::Sensor;
    use pamoja_hal::script::{block_on, DelayLog};

    #[test]
    fn a_driver_reads_the_part_without_a_transfer_being_written_out() {
        let mut sensor = Bme280::i2c(
            part(I2C_ADDRESS_PRIMARY),
            I2C_ADDRESS_PRIMARY,
            DelayLog::new(),
        );
        sensor.init().expect("the part identifies itself");

        let reading = block_on(sensor.read()).expect("a measurement");
        assert_eq!(reading.temperature_centi_celsius, 2044);
        assert_eq!(reading.pressure_centi_pascals, 8_480_523);
        assert_eq!(reading.humidity_q22_10, 45_725);
    }

    #[test]
    fn the_configuration_a_driver_wrote_can_be_read_back_off_the_part() {
        let mut sensor = Bme280::i2c(
            part(I2C_ADDRESS_PRIMARY),
            I2C_ADDRESS_PRIMARY,
            DelayLog::new(),
        );
        sensor.init().expect("the part identifies itself");
        let (registers, _) = sensor.release();
        let part = registers.release();

        // The datasheet wants humidity set before the mode register is written, and the
        // part is left asleep until a measurement is forced.
        assert_eq!(part.register(register::CTRL_HUM), 0x01);
        assert_eq!(part.register(register::CTRL_MEAS) & 0x03, 0x00, "asleep");
        assert_eq!(part.register(register::RESET), 0xb6, "and it was reset");
    }

    #[test]
    fn a_part_reports_what_it_was_asked_to() {
        for (celsius, hectopascals, humidity) in [
            (4.0f32, 1013.25f32, 80.0f32),
            (20.44, 848.05, 44.65),
            (-10.5, 950.0, 12.5),
            (38.75, 1030.0, 95.0),
        ] {
            let mut sensor = Bme280::i2c(
                reporting(I2C_ADDRESS_PRIMARY, celsius, hectopascals, humidity),
                I2C_ADDRESS_PRIMARY,
                DelayLog::new(),
            );
            let reading = block_on(sensor.read()).expect("a measurement");

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
            assert!(
                (reading.relative_humidity_percent() - humidity).abs() < 0.01,
                "{humidity} % read back as {}",
                reading.relative_humidity_percent()
            );
        }
    }

    #[test]
    fn the_registers_a_raw_measurement_sits_in_read_back_as_that_measurement() {
        let raw = RawMeasurement::from_registers(&BURST);
        assert_eq!(registers_of(&raw), BURST);
    }

    #[test]
    fn the_shipped_burst_is_the_reading_the_documentation_claims() {
        let reading = calibration().compensate(&RawMeasurement::from_registers(&BURST));
        assert_eq!(reading.celsius(), 20.44);
        assert_eq!(reading.hectopascals(), 848.0523);
        assert_eq!(reading.relative_humidity_percent(), 44.65332);
    }
}
