//! Node bindings for the actuator drivers that run over an I2C bus.
//!
//! A `Pca9685` drives the part's sixteen PWM channels over an `I2cBus`: it programs the
//! prescale for its frequency with the oscillator asleep, wakes it, waits the 500 us it
//! needs, and loads a channel's four registers in one transfer. Its calls run on a worker
//! thread and resolve once the bus has answered. `pca9685SimPart` is a PCA9685 that is not
//! there, keeping its datasheet's rules, so what a driver wrote reads back off a simulated
//! bus.

use std::sync::{Arc, Mutex, PoisonError};

use napi::bindgen_prelude::{spawn_blocking, Buffer};
use napi_derive::napi;
use pamoja_actuators::pca9685::{self, Outputs, Pwm};
use pamoja_actuators::DriverError;
use pamoja_hal::bus::{BusDelay, BusError, I2cBus as Bus};

use crate::hal::{I2cBus, I2cPart};

type Pca9685Driver = pca9685::Pca9685<Bus, BusDelay>;

/// The address a PCA9685 answers at with its six address pins low.
#[napi]
pub const PCA9685_DEFAULT_ADDRESS: u8 = pca9685::DEFAULT_I2C_ADDRESS;

/// How long the oscillator takes to run once woken, in microseconds.
#[napi]
pub const PCA9685_OSCILLATOR_STARTUP_MICROS: u32 = pca9685::OSCILLATOR_STARTUP_MICROS;

/// Mode register 1.
#[napi]
pub const PCA9685_REGISTER_MODE1: u8 = pca9685::register::MODE1;

/// Mode register 2.
#[napi]
pub const PCA9685_REGISTER_MODE2: u8 = pca9685::register::MODE2;

/// The first of channel 0's four registers.
#[napi]
pub const PCA9685_REGISTER_LED0_ON_L: u8 = pca9685::register::LED0_ON_L;

/// The first of the four registers that load every channel at once.
#[napi]
pub const PCA9685_REGISTER_ALL_LED_ON_L: u8 = pca9685::register::ALL_LED_ON_L;

/// The prescaler, writable only while the part sleeps.
#[napi]
pub const PCA9685_REGISTER_PRE_SCALE: u8 = pca9685::register::PRE_SCALE;

/// MODE1's RESTART bit.
#[napi]
pub const PCA9685_MODE1_RESTART: u8 = pca9685::mode1::RESTART;

/// MODE1's EXTCLK bit.
#[napi]
pub const PCA9685_MODE1_EXTCLK: u8 = pca9685::mode1::EXTCLK;

/// MODE1's auto-increment bit.
#[napi]
pub const PCA9685_MODE1_AUTO_INCREMENT: u8 = pca9685::mode1::AUTO_INCREMENT;

/// MODE1's SLEEP bit.
#[napi]
pub const PCA9685_MODE1_SLEEP: u8 = pca9685::mode1::SLEEP;

/// The power-on value of MODE1.
#[napi]
pub const PCA9685_MODE1_RESET: u8 = pca9685::MODE1_RESET;

/// The power-on value of MODE2.
#[napi]
pub const PCA9685_MODE2_RESET: u8 = pca9685::MODE2_RESET;

/// The power-on value of PRE_SCALE, 200 Hz on the internal oscillator.
#[napi]
pub const PCA9685_PRE_SCALE_RESET: u8 = pca9685::PRE_SCALE_RESET;

/// The smallest value the part loads into PRE_SCALE.
#[napi]
pub const PCA9685_PRE_SCALE_MIN: u8 = pca9685::PRE_SCALE_MIN;

/// How a PCA9685 driver programs the part. A field left out keeps the part's own power-on
/// setting: 200 Hz on the internal oscillator with totem-pole outputs.
#[napi(object)]
pub struct Pca9685Settings {
    /// The PWM frequency every channel shares, in hertz; 50 for hobby servos.
    pub frequency_hz: Option<u32>,
    /// The clock the prescaler divides, in hertz, for a board that drives EXTCLK.
    pub oscillator_hz: Option<u32>,
    /// Totem-pole outputs when true, open-drain when false.
    pub totem_pole: Option<bool>,
    /// Invert the output logic, for a board with no external driver.
    pub inverted: Option<bool>,
    /// Change the outputs on each register write's acknowledge rather than on the stop.
    pub change_on_ack: Option<bool>,
}

/// An NXP PCA9685 on an I2C bus, driving sixteen PWM outputs.
#[napi]
pub struct Pca9685 {
    inner: Arc<Mutex<Pca9685Driver>>,
}

#[napi]
impl Pca9685 {
    /// A driver for the part at `address` on `bus`. Nothing is sent until `init` or the first
    /// channel is loaded.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus, address: u8, settings: Option<Pca9685Settings>) -> Self {
        let wiring = Outputs::default();
        let (frequency, oscillator, outputs) = match settings {
            Some(settings) => (
                settings.frequency_hz.unwrap_or(200),
                settings.oscillator_hz.unwrap_or(pca9685::INTERNAL_OSC_HZ),
                Outputs {
                    totem_pole: settings.totem_pole.unwrap_or(wiring.totem_pole),
                    inverted: settings.inverted.unwrap_or(wiring.inverted),
                    change_on_ack: settings.change_on_ack.unwrap_or(wiring.change_on_ack),
                },
            ),
            None => (200, pca9685::INTERNAL_OSC_HZ, wiring),
        };
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = pca9685::Pca9685::new(bus, address, delay)
            .with_frequency(frequency)
            .with_oscillator(oscillator)
            .with_outputs(outputs);
        Pca9685 {
            inner: Arc::new(Mutex::new(driver)),
        }
    }

    /// Programs the prescale and the output wiring with the oscillator asleep, wakes it, waits
    /// the 500 us it needs, and restarts the channels.
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.init()).await
    }

    /// Loads one channel with the four register bytes the `pwm` builders make, initializing
    /// the part first if `init` has not run. Rejects a channel past 15.
    #[napi]
    pub async fn set_channel(&self, channel: u8, pwm: Buffer) -> napi::Result<()> {
        let pwm = pwm_of(&pwm)?;
        drive(&self.inner, move |driver| driver.set_channel(channel, pwm)).await
    }

    /// Loads every channel with the same four bytes in one transfer.
    #[napi]
    pub async fn set_all(&self, pwm: Buffer) -> napi::Result<()> {
        let pwm = pwm_of(&pwm)?;
        drive(&self.inner, move |driver| driver.set_all(pwm)).await
    }

    /// Stops the oscillator; the channels keep their settings.
    #[napi]
    pub async fn sleep(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.sleep()).await
    }

    /// Wakes the oscillator, waits the 500 us it needs, and restarts the channels that were
    /// running before the sleep.
    #[napi]
    pub async fn wake(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.wake()).await
    }

    /// Sends the general-call software reset, which returns every PCA9685 on the bus to its
    /// power-on state. It goes to address 0x00, which nothing on a simulated bus answers.
    #[napi]
    pub async fn software_reset(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.software_reset()).await
    }

    /// The prescale value the driver writes for its frequency.
    #[napi(getter)]
    pub fn prescale(&self) -> u8 {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .prescale()
    }

    /// The frequency the part runs at once the prescaler has rounded the one asked for, in
    /// hertz.
    #[napi(getter)]
    pub fn frequency(&self) -> f64 {
        f64::from(
            self.inner
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .frequency(),
        )
    }
}

/// A simulated PCA9685 as it powers up: asleep at 200 Hz with every output off, keeping its
/// datasheet's rules for writes, reads, and its register pointer.
#[napi]
pub fn pca9685_sim_part(address: u8) -> I2cPart {
    I2cPart {
        inner: pca9685::sim::part(address),
    }
}

fn pwm_of(bytes: &Buffer) -> napi::Result<Pwm> {
    let registers: [u8; 4] = bytes
        .as_ref()
        .try_into()
        .map_err(|_| napi::Error::from_reason("pwm must be exactly 4 bytes"))?;
    Ok(Pwm::from_bytes(&registers))
}

/// Runs a driver call on a blocking worker thread.
async fn drive(
    inner: &Arc<Mutex<Pca9685Driver>>,
    call: impl FnOnce(&mut Pca9685Driver) -> Result<(), DriverError<BusError>> + Send + 'static,
) -> napi::Result<()> {
    let inner = Arc::clone(inner);
    let outcome = spawn_blocking(move || {
        let mut driver = inner.lock().unwrap_or_else(PoisonError::into_inner);
        call(&mut driver)
    })
    .await
    .map_err(|error| {
        napi::Error::from_reason(format!("the driver call did not finish: {error}"))
    })?;
    outcome.map_err(|error| match error {
        DriverError::Bus(error) => napi::Error::from_reason(error.to_string()),
        DriverError::Command(reason) => napi::Error::from_reason(reason),
    })
}
