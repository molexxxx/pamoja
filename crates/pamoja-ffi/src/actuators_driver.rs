//! The C ABI for the actuator drivers that run over an I2C bus.
//!
//! A [`PamojaPca9685`] drives the PCA9685's sixteen PWM channels over a [`PamojaI2cBus`]. It
//! puts the oscillator to sleep, writes the prescale for its frequency, sets the output
//! wiring, wakes the oscillator, waits the 500 us it needs, and restarts the channels; then it
//! loads a channel's four registers in one transfer. It holds a share of the bus, so the
//! caller may free its own bus handle straight after building the driver, or keep it to read
//! the part back.
//!
//! [`pamoja_pca9685_sim_part`] is a PCA9685 that is not there. It holds the part's power-on
//! registers and keeps its datasheet's rules, so what a driver wrote reads back off a
//! simulated bus the way the part would hold it.
//!
//! A failure is [`PamojaStatus::Io`] when the bus failed and
//! [`PamojaStatus::InvalidArgument`] for a command the part cannot take, such as a channel past
//! 15, with the reason in the last error message.

use std::fmt::{Debug, Display};
use std::panic::{catch_unwind, AssertUnwindSafe};

use pamoja_actuators::pca9685::{self, Outputs, Pwm};
use pamoja_actuators::DriverError;
use pamoja_hal::bus::{BusDelay, I2cBus};

use crate::actuators::PamojaPwm;
use crate::hal::{PamojaI2cBus, PamojaI2cPart};
use crate::{set_last_error, PamojaStatus};

type Pca9685Driver = pca9685::Pca9685<I2cBus, BusDelay>;

/// The frequency a PCA9685 driver runs at unless given another, in hertz: the part's own
/// power-on 200 Hz.
pub const PAMOJA_PCA9685_DEFAULT_FREQUENCY_HZ: u32 = 200;

/// How a PCA9685's sixteen outputs are wired, the MODE2 register.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaPca9685Outputs {
    /// 1 for totem-pole outputs, the power-on setting; 0 for open-drain.
    pub totem_pole: u8,
    /// 1 to invert the output logic, for a board with no external driver.
    pub inverted: u8,
    /// 1 to change the outputs on the acknowledge of each register write rather than on the
    /// stop condition.
    pub change_on_ack: u8,
}

/// How a PCA9685 driver programs the part.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaPca9685Settings {
    /// The PWM frequency every channel shares, in hertz; the prescaler reaches 24 to 1526 at
    /// 25 MHz, and 50 is what a hobby servo wants.
    pub frequency_hz: u32,
    /// The clock the prescaler divides, in hertz: the internal 25 MHz unless the board drives
    /// EXTCLK.
    pub oscillator_hz: u32,
    /// How the outputs are wired.
    pub outputs: PamojaPca9685Outputs,
}

/// A PCA9685 driver. Opaque; release it with [`pamoja_pca9685_free`].
pub struct PamojaPca9685 {
    driver: Pca9685Driver,
}

impl From<PamojaPca9685Outputs> for Outputs {
    fn from(value: PamojaPca9685Outputs) -> Self {
        Outputs {
            totem_pole: value.totem_pole != 0,
            inverted: value.inverted != 0,
            change_on_ack: value.change_on_ack != 0,
        }
    }
}

impl From<Outputs> for PamojaPca9685Outputs {
    fn from(value: Outputs) -> Self {
        PamojaPca9685Outputs {
            totem_pole: u8::from(value.totem_pole),
            inverted: u8::from(value.inverted),
            change_on_ack: u8::from(value.change_on_ack),
        }
    }
}

/// Returns the settings a PCA9685 driver takes when given none: 200 Hz on the internal
/// oscillator with totem-pole outputs, the part's own power-on state.
///
/// # Returns
///
/// The settings.
#[no_mangle]
pub extern "C" fn pamoja_pca9685_settings_default() -> PamojaPca9685Settings {
    PamojaPca9685Settings {
        frequency_hz: PAMOJA_PCA9685_DEFAULT_FREQUENCY_HZ,
        oscillator_hz: pca9685::INTERNAL_OSC_HZ,
        outputs: Outputs::default().into(),
    }
}

/// Creates a PCA9685 driver on a bus. Nothing is sent until [`pamoja_pca9685_init`] or the
/// first channel is loaded.
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `address` - the address the A5 to A0 pins select, `0x40` with all six low.
/// * `settings` - the frequency, the oscillator, and the output wiring; see
///   [`pamoja_pca9685_settings_default`].
/// * `out_driver` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_driver`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_driver` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_new(
    bus: *const PamojaI2cBus,
    address: u8,
    settings: PamojaPca9685Settings,
    out_driver: *mut *mut PamojaPca9685,
) -> PamojaStatus {
    if out_driver.is_null() {
        set_last_error("out_driver must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_driver = std::ptr::null_mut();
    let Some(bus) = bus.as_ref() else {
        set_last_error("bus must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let bus = bus.bus.clone();
    let delay = bus.delay();
    let driver = pca9685::Pca9685::new(bus, address, delay)
        .with_frequency(settings.frequency_hz)
        .with_oscillator(settings.oscillator_hz)
        .with_outputs(settings.outputs.into());
    *out_driver = Box::into_raw(Box::new(PamojaPca9685 { driver }));
    PamojaStatus::Ok
}

/// Puts the oscillator to sleep, writes the prescale and the output wiring, wakes it, waits
/// the 500 us it needs, and restarts the channels.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::Io`] when the bus failed, or
/// [`PamojaStatus::InvalidArgument`] for a null driver.
///
/// # Safety
///
/// `driver` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_init(driver: *mut PamojaPca9685) -> PamojaStatus {
    run(driver, |held| held.driver.init())
}

/// Loads one channel's on and off counts, initializing the part first if it has not been.
///
/// # Arguments
///
/// * `driver` - the driver.
/// * `channel` - the output, 0 to 15.
/// * `pwm` - the four register bytes to load, as the `pamoja_pwm_` functions build them.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a channel past 15 or a null
/// driver, or [`PamojaStatus::Io`] when the bus failed.
///
/// # Safety
///
/// `driver` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_set_channel(
    driver: *mut PamojaPca9685,
    channel: u8,
    pwm: PamojaPwm,
) -> PamojaStatus {
    run(driver, |held| held.driver.set_channel(channel, pwm_of(pwm)))
}

/// Reads one channel's counts back from the part, one register a transfer, so the read works
/// whatever MODE1 holds and changes nothing on the part.
///
/// # Arguments
///
/// * `driver` - the driver.
/// * `channel` - the output, 0 to 15.
/// * `out_pwm` - receives the four register bytes the channel holds.
///
/// # Returns
///
/// As [`pamoja_pca9685_set_channel`], and [`PamojaStatus::InvalidArgument`] for a null
/// `out_pwm`.
///
/// # Safety
///
/// `driver` must be a live handle or null, and `out_pwm` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_channel(
    driver: *mut PamojaPca9685,
    channel: u8,
    out_pwm: *mut PamojaPwm,
) -> PamojaStatus {
    let Some(out) = out_pwm.as_mut() else {
        set_last_error("out_pwm must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    run(driver, |held| {
        *out = held.driver.channel(channel)?.into();
        Ok(())
    })
}

/// Loads every channel with the same counts in one transfer, through the ALL_LED registers.
///
/// # Arguments
///
/// * `driver` - the driver.
/// * `pwm` - the four register bytes to load.
///
/// # Returns
///
/// As [`pamoja_pca9685_set_channel`].
///
/// # Safety
///
/// `driver` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_set_all(
    driver: *mut PamojaPca9685,
    pwm: PamojaPwm,
) -> PamojaStatus {
    run(driver, |held| held.driver.set_all(pwm_of(pwm)))
}

/// Stops the oscillator. The channel registers keep their values, and a channel still
/// running when the part sleeps sets its RESTART bit.
///
/// # Returns
///
/// As [`pamoja_pca9685_init`].
///
/// # Safety
///
/// `driver` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_sleep(driver: *mut PamojaPca9685) -> PamojaStatus {
    run(driver, |held| held.driver.sleep())
}

/// Wakes the oscillator, waits the 500 us it needs, and restarts every channel that was
/// running before the sleep.
///
/// # Returns
///
/// As [`pamoja_pca9685_init`].
///
/// # Safety
///
/// `driver` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_wake(driver: *mut PamojaPca9685) -> PamojaStatus {
    run(driver, |held| held.driver.wake())
}

/// Sends the general-call software reset, which returns every PCA9685 on the bus to its
/// power-on state, not only this one. It goes to address `0x00`, which nothing on a simulated
/// bus answers.
///
/// # Returns
///
/// As [`pamoja_pca9685_init`].
///
/// # Safety
///
/// `driver` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_software_reset(driver: *mut PamojaPca9685) -> PamojaStatus {
    run(driver, |held| held.driver.software_reset())
}

/// Returns the prescale value the driver writes for its frequency.
///
/// # Returns
///
/// The PRE_SCALE register value, or 0 for a null driver.
///
/// # Safety
///
/// `driver` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_prescale(driver: *const PamojaPca9685) -> u8 {
    driver.as_ref().map_or(0, |held| held.driver.prescale())
}

/// Returns the frequency the part runs at once the prescaler has rounded the one asked for.
///
/// # Returns
///
/// The frequency in hertz, or 0 for a null driver.
///
/// # Safety
///
/// `driver` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_frequency(driver: *const PamojaPca9685) -> f32 {
    driver.as_ref().map_or(0.0, |held| held.driver.frequency())
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `driver` must be a handle from [`pamoja_pca9685_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_free(driver: *mut PamojaPca9685) {
    if !driver.is_null() {
        drop(Box::from_raw(driver));
    }
}

/// Creates a simulated PCA9685 as it powers up: asleep at 200 Hz with every output off,
/// keeping its datasheet's rules for writes, reads, and its register pointer.
///
/// # Arguments
///
/// * `address` - the address it answers to.
///
/// # Returns
///
/// A part to put on a simulated bus, released with
/// [`pamoja_i2c_part_free`](crate::hal::pamoja_i2c_part_free).
#[no_mangle]
pub extern "C" fn pamoja_pca9685_sim_part(address: u8) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(pca9685::sim::part(address))
}

fn pwm_of(pwm: PamojaPwm) -> Pwm {
    Pwm::from_bytes(&[pwm.on_low, pwm.on_high, pwm.off_low, pwm.off_high])
}

/// Runs a driver call, turning a failure or a panic into a status.
///
/// # Safety
///
/// `driver` must be a live handle or null.
unsafe fn run<E: Debug + Display>(
    driver: *mut PamojaPca9685,
    call: impl FnOnce(&mut PamojaPca9685) -> Result<(), DriverError<E>>,
) -> PamojaStatus {
    let Some(held) = driver.as_mut() else {
        set_last_error("driver must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match catch_unwind(AssertUnwindSafe(|| call(held))) {
        Ok(Ok(())) => PamojaStatus::Ok,
        Ok(Err(DriverError::Command(reason))) => {
            set_last_error(reason.to_owned());
            PamojaStatus::InvalidArgument
        }
        Ok(Err(DriverError::Bus(error))) => {
            set_last_error(error.to_string());
            PamojaStatus::Io
        }
        Err(_) => {
            set_last_error("panic at the FFI boundary".to_owned());
            PamojaStatus::Panic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::{
        pamoja_i2c_bus_attach, pamoja_i2c_bus_free, pamoja_i2c_bus_part, pamoja_i2c_bus_simulated,
        pamoja_i2c_bus_waited_micros, pamoja_i2c_part_free, pamoja_i2c_part_register,
    };
    use crate::{actuators, pamoja_last_error_message};
    use std::ffi::CStr;
    use std::ptr;

    fn last_error() -> String {
        unsafe { CStr::from_ptr(pamoja_last_error_message()) }
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn a_driver_programs_a_simulated_part_as_the_datasheet_asks() {
        unsafe {
            let part = pamoja_pca9685_sim_part(0x40);
            let bus = pamoja_i2c_bus_simulated();
            assert_eq!(pamoja_i2c_bus_attach(bus, part), PamojaStatus::Ok);
            pamoja_i2c_part_free(part);

            let mut settings = pamoja_pca9685_settings_default();
            assert_eq!(settings.frequency_hz, 200);
            assert_eq!(settings.outputs.totem_pole, 1);
            settings.frequency_hz = 50;
            let mut driver = ptr::null_mut();
            assert_eq!(
                pamoja_pca9685_new(bus, 0x40, settings, &mut driver),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_pca9685_prescale(driver), 121);
            assert!((pamoja_pca9685_frequency(driver) - 50.0).abs() < 0.1);

            let servo = actuators::pamoja_pwm_servo(1_500, 50);
            assert_eq!(
                pamoja_pca9685_set_channel(driver, 0, servo),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_pca9685_set_channel(driver, 16, servo),
                PamojaStatus::InvalidArgument
            );
            assert!(
                last_error().contains("sixteen channels"),
                "{}",
                last_error()
            );
            assert_eq!(pamoja_i2c_bus_waited_micros(bus), 500);

            let held = pamoja_i2c_bus_part(bus, 0x40);
            assert_eq!(
                pamoja_i2c_part_register(held, pca9685::register::PRE_SCALE),
                121
            );
            assert_eq!(
                pamoja_i2c_part_register(held, pca9685::register::MODE1),
                0x20
            );
            let loaded = PamojaPwm {
                on_low: pamoja_i2c_part_register(held, 0x06),
                on_high: pamoja_i2c_part_register(held, 0x07),
                off_low: pamoja_i2c_part_register(held, 0x08),
                off_high: pamoja_i2c_part_register(held, 0x09),
            };
            assert_eq!(loaded, servo);
            pamoja_i2c_part_free(held);

            let mut read_back = actuators::pamoja_pwm_full_on();
            assert_eq!(
                pamoja_pca9685_channel(driver, 0, &mut read_back),
                PamojaStatus::Ok
            );
            assert_eq!(read_back, servo, "the driver reads the channel back");
            assert_eq!(
                pamoja_pca9685_channel(driver, 16, &mut read_back),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_pca9685_channel(driver, 0, ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );

            assert_eq!(
                pamoja_pca9685_set_all(driver, actuators::pamoja_pwm_full_off()),
                PamojaStatus::Ok
            );
            let held = pamoja_i2c_bus_part(bus, 0x40);
            assert_eq!(
                pamoja_i2c_part_register(held, 0x09),
                0x10,
                "channel 0 off again"
            );
            pamoja_i2c_part_free(held);

            assert_eq!(pamoja_pca9685_software_reset(driver), PamojaStatus::Io);
            assert!(last_error().contains("0x00"), "{}", last_error());

            pamoja_pca9685_free(driver);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn a_null_driver_is_refused() {
        unsafe {
            assert_eq!(
                pamoja_pca9685_init(ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(pamoja_pca9685_prescale(ptr::null()), 0);
            let mut driver = ptr::null_mut();
            assert_eq!(
                pamoja_pca9685_new(
                    ptr::null(),
                    0x40,
                    pamoja_pca9685_settings_default(),
                    &mut driver
                ),
                PamojaStatus::InvalidArgument
            );
            assert!(driver.is_null());
        }
    }
}
