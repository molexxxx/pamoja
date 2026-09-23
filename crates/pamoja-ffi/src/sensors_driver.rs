//! The C ABI for the sensor drivers that run over an I2C bus.
//!
//! A driver runs the datasheet's whole conversation with a part over a [`PamojaI2cBus`]:
//! reset, identify, read the calibration, configure, measure. It holds a share of the bus, so the caller may free its own bus handle
//! straight after building the driver, or keep it to look at the bus between reads. A driver
//! waits as its datasheet asks through the bus's delay, which sleeps only when real parts are
//! on the other end.
//!
//! A simulated part answers a driver with nothing plugged in. [`pamoja_bme280_sim_part`]
//! holds a real part's calibration and one measurement it took, and
//! [`pamoja_bme280_sim_reporting`] reads whatever it is asked to; either goes on a simulated
//! bus with [`pamoja_i2c_bus_attach`](crate::hal::pamoja_i2c_bus_attach).
//!
//! A driver's failure is [`PamojaStatus::Io`] when the bus failed or the part never finished,
//! and [`PamojaStatus::Codec`] when a part answered but is not the one the driver expected,
//! with the reason in the last error message either way.

use std::fmt::Debug;
use std::panic::{catch_unwind, AssertUnwindSafe};

use pamoja_hal::bus::{BusDelay, I2cBus};
use pamoja_sensors::bme280::{self, sim, Bme280};
use pamoja_sensors::driver::I2cRegisters;
use pamoja_sensors::DriverError;

use crate::hal::{PamojaI2cBus, PamojaI2cPart};
use crate::sensors::{
    PamojaBme280Measurement, PAMOJA_BME280_CALIBRATION_HUMIDITY_LEN,
    PAMOJA_BME280_CALIBRATION_TEMP_PRESS_LEN, PAMOJA_BME280_MEASUREMENT_LEN,
};
use crate::{set_last_error, PamojaStatus};

/// The status a simulated BME280 reports when it is neither measuring nor loading its
/// calibration.
pub const PAMOJA_BME280_SIM_STATUS_IDLE: u8 = 0x00;

const _: () = assert!(PAMOJA_BME280_SIM_STATUS_IDLE == sim::STATUS_IDLE);

/// How a BME280 driver measures: the oversampling of each measurement and the IIR filter.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaBme280Settings {
    /// The temperature oversampling code, `0..=5`, where `0` skips the measurement.
    pub temperature: u8,
    /// The pressure oversampling code, `0..=5`, where `0` skips the measurement.
    pub pressure: u8,
    /// The humidity oversampling code, `0..=5`, where `0` skips the measurement.
    pub humidity: u8,
    /// The IIR filter code, `0..=4`, where `0` is off.
    pub filter: u8,
}

/// A BME280 driven over an I2C bus. Opaque; release it with [`pamoja_bme280_free`].
pub struct PamojaBme280 {
    sensor: Bme280<I2cRegisters<I2cBus>, BusDelay>,
}

/// Returns the settings a BME280 driver starts with: every measurement at oversampling x1 and
/// the filter off.
///
/// # Returns
///
/// The settings.
#[no_mangle]
pub extern "C" fn pamoja_bme280_settings_default() -> PamojaBme280Settings {
    PamojaBme280Settings {
        temperature: bme280::Oversampling::X1.code(),
        pressure: bme280::Oversampling::X1.code(),
        humidity: bme280::Oversampling::X1.code(),
        filter: bme280::Filter::Off.code(),
    }
}

/// Creates a BME280 driver on a bus. Nothing is sent until [`pamoja_bme280_init`] or the first
/// [`pamoja_bme280_measure`].
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `address` - [`crate::sensors::PAMOJA_BME280_I2C_ADDRESS_PRIMARY`] with SDO low, or
///   [`crate::sensors::PAMOJA_BME280_I2C_ADDRESS_SECONDARY`] with SDO high.
/// * `settings` - the oversampling and the filter.
/// * `out_sensor` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_sensor`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_sensor` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_new(
    bus: *const PamojaI2cBus,
    address: u8,
    settings: PamojaBme280Settings,
    out_sensor: *mut *mut PamojaBme280,
) -> PamojaStatus {
    if bus.is_null() || out_sensor.is_null() {
        set_last_error("bus and out_sensor must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bus = (*bus).bus.clone();
    let delay = bus.delay();
    let sensor = Bme280::i2c(bus, address, delay)
        .with_oversampling(
            bme280::Oversampling::from_code(settings.temperature),
            bme280::Oversampling::from_code(settings.pressure),
            bme280::Oversampling::from_code(settings.humidity),
        )
        .with_filter(bme280::Filter::from_code(settings.filter));
    *out_sensor = Box::into_raw(Box::new(PamojaBme280 { sensor }));
    PamojaStatus::Ok
}

/// Resets the part, checks it is a BME280, reads its calibration, and writes the settings, in
/// the order the datasheet requires, leaving the part asleep.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null driver;
/// [`PamojaStatus::Io`] when the bus fails or the calibration never finishes loading; or
/// [`PamojaStatus::Codec`] when the part at the address is not a BME280.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_init(sensor: *mut PamojaBme280) -> PamojaStatus {
    match on_sensor(sensor, |sensor| sensor.init()) {
        Ok(()) => PamojaStatus::Ok,
        Err(status) => status,
    }
}

/// Runs one forced measurement and compensates it, initializing the part first if
/// [`pamoja_bme280_init`] has not run.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_measurement` - receives the reading.
///
/// # Returns
///
/// As [`pamoja_bme280_init`], with [`PamojaStatus::Io`] also when the part is still measuring
/// after the datasheet's time.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_measurement` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_measure(
    sensor: *mut PamojaBme280,
    out_measurement: *mut PamojaBme280Measurement,
) -> PamojaStatus {
    if out_measurement.is_null() {
        set_last_error("out_measurement must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match on_sensor(sensor, |sensor| sensor.measure()) {
        Ok(reading) => {
            *out_measurement = PamojaBme280Measurement {
                celsius: reading.celsius(),
                pascals: reading.pascals(),
                hectopascals: reading.hectopascals(),
                relative_humidity_percent: reading.relative_humidity_percent(),
            };
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_bme280_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_free(sensor: *mut PamojaBme280) {
    if !sensor.is_null() {
        drop(Box::from_raw(sensor));
    }
}

/// Creates a simulated BME280 holding a real part's calibration and one measurement it took,
/// which compensate to 20.44 C, 848.05 hPa, and 44.65 %.
///
/// # Arguments
///
/// * `address` - the address it answers to.
///
/// # Returns
///
/// The part, which the caller puts on a simulated bus and releases with
/// [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_bme280_sim_part(address: u8) -> *mut PamojaI2cPart {
    Box::into_raw(Box::new(PamojaI2cPart {
        part: sim::part(address),
    }))
}

/// Creates a simulated BME280 that reads what it is asked to.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `celsius` - the temperature it reports.
/// * `hectopascals` - the pressure it reports.
/// * `relative_humidity` - the humidity it reports, as a percentage.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`]. Its readings
/// land within a hundredth of a degree, a hundredth of a hectopascal, and a thousandth of a
/// percent of what was asked for, the nearest its converter can represent.
#[no_mangle]
pub extern "C" fn pamoja_bme280_sim_reporting(
    address: u8,
    celsius: f32,
    hectopascals: f32,
    relative_humidity: f32,
) -> *mut PamojaI2cPart {
    Box::into_raw(Box::new(PamojaI2cPart {
        part: sim::reporting(address, celsius, hectopascals, relative_humidity),
    }))
}

/// Copies the calibration a simulated BME280 holds: the two blocks a driver reads at start-up.
///
/// # Arguments
///
/// * `out_temp_press` - receives the 26-byte temperature and pressure block.
/// * `out_humidity` - receives the 7-byte humidity block.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `out_temp_press` must point to 26 writable bytes and `out_humidity` to 7.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_sim_calibration(
    out_temp_press: *mut u8,
    out_humidity: *mut u8,
) -> PamojaStatus {
    if out_temp_press.is_null() || out_humidity.is_null() {
        set_last_error("out_temp_press and out_humidity must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    std::slice::from_raw_parts_mut(out_temp_press, PAMOJA_BME280_CALIBRATION_TEMP_PRESS_LEN)
        .copy_from_slice(&sim::CALIBRATION);
    std::slice::from_raw_parts_mut(out_humidity, PAMOJA_BME280_CALIBRATION_HUMIDITY_LEN)
        .copy_from_slice(&sim::CALIBRATION_HUMIDITY);
    PamojaStatus::Ok
}

/// Copies the one measurement a simulated BME280 holds: the eight data registers a real part
/// read, which compensate to 20.44 C, 848.05 hPa, and 44.65 % against its calibration.
///
/// # Arguments
///
/// * `out_burst` - receives the eight bytes a burst read returns.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `out_burst` must point to 8 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_sim_burst(out_burst: *mut u8) -> PamojaStatus {
    if out_burst.is_null() {
        set_last_error("out_burst must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    std::slice::from_raw_parts_mut(out_burst, PAMOJA_BME280_MEASUREMENT_LEN)
        .copy_from_slice(&sim::BURST);
    PamojaStatus::Ok
}

/// Builds the eight data registers a simulated BME280 holds when it reports a reading.
///
/// # Arguments
///
/// * `celsius` - the temperature.
/// * `hectopascals` - the pressure.
/// * `relative_humidity` - the humidity, as a percentage.
/// * `out_burst` - receives the eight bytes a burst read returns.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `out_burst` must point to 8 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_sim_burst_for(
    celsius: f32,
    hectopascals: f32,
    relative_humidity: f32,
    out_burst: *mut u8,
) -> PamojaStatus {
    if out_burst.is_null() {
        set_last_error("out_burst must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    std::slice::from_raw_parts_mut(out_burst, PAMOJA_BME280_MEASUREMENT_LEN)
        .copy_from_slice(&sim::burst_for(celsius, hectopascals, relative_humidity));
    PamojaStatus::Ok
}

/// Runs a driver call, turning a failure or a panic into a status.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
unsafe fn on_sensor<T, E: Debug + std::fmt::Display>(
    sensor: *mut PamojaBme280,
    call: impl FnOnce(&mut Bme280<I2cRegisters<I2cBus>, BusDelay>) -> Result<T, DriverError<E>>,
) -> Result<T, PamojaStatus> {
    if sensor.is_null() {
        set_last_error("sensor must not be null".to_owned());
        return Err(PamojaStatus::InvalidArgument);
    }
    let sensor = &mut (*sensor).sensor;
    match catch_unwind(AssertUnwindSafe(|| call(sensor))) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(driver_failed(&error)),
        Err(_) => {
            set_last_error("panic at the FFI boundary".to_owned());
            Err(PamojaStatus::Panic)
        }
    }
}

/// Records why a driver failed, in the bus's own words when the bus was the cause, and maps
/// it onto its status.
fn driver_failed<E: Debug + std::fmt::Display>(error: &DriverError<E>) -> PamojaStatus {
    match error {
        DriverError::Bus(error) => {
            set_last_error(error.to_string());
            PamojaStatus::Io
        }
        DriverError::Timeout => {
            set_last_error(error.to_string());
            PamojaStatus::Io
        }
        DriverError::Sensor(sensor) => {
            set_last_error(sensor.to_string());
            PamojaStatus::Codec
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::{
        pamoja_i2c_bus_attach, pamoja_i2c_bus_free, pamoja_i2c_bus_part, pamoja_i2c_bus_simulated,
        pamoja_i2c_bus_waited_micros, pamoja_i2c_part_free, pamoja_i2c_part_load,
        pamoja_i2c_part_new, pamoja_i2c_part_register,
    };
    use crate::sensors::{
        pamoja_bme280_ctrl_hum_from_bits, pamoja_bme280_ctrl_meas_from_bits,
        pamoja_bme280_max_measurement_micros, PamojaBme280CtrlMeas,
        PAMOJA_BME280_I2C_ADDRESS_PRIMARY, PAMOJA_BME280_REGISTER_CHIP_ID,
        PAMOJA_BME280_REGISTER_CTRL_HUM, PAMOJA_BME280_REGISTER_CTRL_MEAS,
    };
    use std::ffi::CStr;
    use std::ptr;

    const PART: u8 = PAMOJA_BME280_I2C_ADDRESS_PRIMARY;

    fn last_error() -> String {
        let message = crate::pamoja_last_error_message();
        if message.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned()
    }

    fn measurement() -> PamojaBme280Measurement {
        PamojaBme280Measurement {
            celsius: 0.0,
            pascals: 0,
            hectopascals: 0.0,
            relative_humidity_percent: 0.0,
        }
    }

    #[test]
    fn a_driver_measures_a_simulated_part_and_leaves_it_configured() {
        unsafe {
            let bus = pamoja_i2c_bus_simulated();
            let part = pamoja_bme280_sim_part(PART);
            assert_eq!(pamoja_i2c_bus_attach(bus, part), PamojaStatus::Ok);
            pamoja_i2c_part_free(part);

            let mut sensor = ptr::null_mut();
            assert_eq!(
                pamoja_bme280_new(bus, PART, pamoja_bme280_settings_default(), &mut sensor),
                PamojaStatus::Ok
            );
            let mut reading = measurement();
            assert_eq!(
                pamoja_bme280_measure(sensor, &mut reading),
                PamojaStatus::Ok
            );
            assert_eq!(reading.celsius, 20.44);
            assert_eq!(reading.pascals, 84_805);

            let held = pamoja_i2c_bus_part(bus, PART);
            let mut ctrl = PamojaBme280CtrlMeas {
                temperature: 0,
                pressure: 0,
                mode: 0,
            };
            assert_eq!(
                pamoja_bme280_ctrl_meas_from_bits(
                    pamoja_i2c_part_register(held, PAMOJA_BME280_REGISTER_CTRL_MEAS),
                    &mut ctrl
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                ctrl.mode,
                bme280::Mode::Forced.code(),
                "the last write forced it"
            );
            assert_eq!(
                pamoja_bme280_ctrl_hum_from_bits(pamoja_i2c_part_register(
                    held,
                    PAMOJA_BME280_REGISTER_CTRL_HUM
                )),
                bme280::Oversampling::X1.code()
            );
            pamoja_i2c_part_free(held);

            assert_eq!(
                pamoja_i2c_bus_waited_micros(bus),
                u64::from(bme280::STARTUP_MICROS)
                    + u64::from(pamoja_bme280_max_measurement_micros(1, 1, 1)),
                "the reset's start-up time and one measurement's"
            );
            pamoja_bme280_free(sensor);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn a_part_asked_for_a_reading_reports_it() {
        unsafe {
            let bus = pamoja_i2c_bus_simulated();
            let part = pamoja_bme280_sim_reporting(PART, 4.0, 1013.25, 80.0);
            assert_eq!(pamoja_i2c_bus_attach(bus, part), PamojaStatus::Ok);
            pamoja_i2c_part_free(part);

            let mut sensor = ptr::null_mut();
            assert_eq!(
                pamoja_bme280_new(bus, PART, pamoja_bme280_settings_default(), &mut sensor),
                PamojaStatus::Ok
            );
            pamoja_i2c_bus_free(bus);

            let mut reading = measurement();
            assert_eq!(
                pamoja_bme280_measure(sensor, &mut reading),
                PamojaStatus::Ok
            );
            assert_eq!(reading.celsius, 4.0);
            assert!((reading.hectopascals - 1013.25).abs() < 0.01);
            assert!((reading.relative_humidity_percent - 80.0).abs() < 0.01);
            pamoja_bme280_free(sensor);
        }
    }

    #[test]
    fn a_missing_part_is_an_io_error_and_another_part_a_codec_error() {
        unsafe {
            let bus = pamoja_i2c_bus_simulated();
            let mut sensor = ptr::null_mut();
            assert_eq!(
                pamoja_bme280_new(bus, PART, pamoja_bme280_settings_default(), &mut sensor),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_bme280_init(sensor), PamojaStatus::Io);
            assert_eq!(last_error(), "nothing answered at 0x76");

            let bmp280 = pamoja_i2c_part_new(PART);
            assert_eq!(
                pamoja_i2c_part_load(bmp280, PAMOJA_BME280_REGISTER_CHIP_ID, [0x58].as_ptr(), 1),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_i2c_bus_attach(bus, bmp280), PamojaStatus::Ok);
            pamoja_i2c_part_free(bmp280);
            assert_eq!(pamoja_bme280_init(sensor), PamojaStatus::Codec);
            assert_eq!(last_error(), "sensor identification mismatch");

            pamoja_bme280_free(sensor);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn the_shipped_calibration_and_a_built_burst_compensate_to_the_reading() {
        unsafe {
            let mut temp_press = [0u8; PAMOJA_BME280_CALIBRATION_TEMP_PRESS_LEN];
            let mut humidity = [0u8; PAMOJA_BME280_CALIBRATION_HUMIDITY_LEN];
            assert_eq!(
                pamoja_bme280_sim_calibration(temp_press.as_mut_ptr(), humidity.as_mut_ptr()),
                PamojaStatus::Ok
            );
            let calibration = bme280::Calibration::from_registers(&temp_press, &humidity);

            let mut shipped = [0u8; PAMOJA_BME280_MEASUREMENT_LEN];
            assert_eq!(
                pamoja_bme280_sim_burst(shipped.as_mut_ptr()),
                PamojaStatus::Ok
            );
            let reading = calibration.compensate(&bme280::RawMeasurement::from_registers(&shipped));
            assert_eq!(reading.celsius(), 20.44);

            let mut burst = [0u8; PAMOJA_BME280_MEASUREMENT_LEN];
            assert_eq!(
                pamoja_bme280_sim_burst_for(-12.5, 990.0, 35.0, burst.as_mut_ptr()),
                PamojaStatus::Ok
            );
            let reading = calibration.compensate(&bme280::RawMeasurement::from_registers(&burst));
            assert_eq!(reading.celsius(), -12.5);
            assert!((reading.hectopascals() - 990.0).abs() < 0.01);
            assert!((reading.relative_humidity_percent() - 35.0).abs() < 0.01);
            assert_eq!(
                pamoja_bme280_sim_burst_for(0.0, 0.0, 0.0, ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_bme280_sim_burst(ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
        }
    }
}
