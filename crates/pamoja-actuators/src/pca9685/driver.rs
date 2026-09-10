//! The PCA9685 driven over I2C: set the frequency, wake the oscillator, load channels.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use pamoja_core::Actuator;

use super::{
    channel_register, frequency_for_prescale, mode1, mode2, prescale_for_frequency, register, Pwm,
    CHANNELS, INTERNAL_OSC_HZ, OSCILLATOR_STARTUP_MICROS, SOFTWARE_RESET_ADDRESS,
    SOFTWARE_RESET_BYTE,
};
use crate::error::DriverError;

/// One channel's setting, the command a [`Pca9685`] takes as an [`Actuator`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Output {
    /// The channel, 0 to 15.
    pub channel: u8,
    /// The on and off counts to load.
    pub pwm: Pwm,
}

/// How the sixteen outputs are wired, the MODE2 register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outputs {
    /// Totem-pole (push-pull) outputs, the reset default, or open-drain when
    /// `false`; the datasheet asks for open-drain when driving LEDs with built-in
    /// protection.
    pub totem_pole: bool,
    /// Invert the output logic, for boards with no external driver.
    pub inverted: bool,
    /// Change the outputs on the acknowledge of each register write rather than on
    /// the stop condition; the default synchronizes a whole update.
    pub change_on_ack: bool,
}

impl Default for Outputs {
    fn default() -> Self {
        Outputs {
            totem_pole: true,
            inverted: false,
            change_on_ack: false,
        }
    }
}

impl Outputs {
    /// Packs the wiring into the MODE2 register byte.
    ///
    /// # Returns
    ///
    /// The byte to write to [`register::MODE2`].
    pub fn bits(self) -> u8 {
        let mut bits = 0;
        if self.totem_pole {
            bits |= mode2::OUTDRV;
        }
        if self.inverted {
            bits |= mode2::INVRT;
        }
        if self.change_on_ack {
            bits |= mode2::OCH;
        }
        bits
    }
}

/// A PCA9685 on an I2C bus, driving up to sixteen PWM outputs.
///
/// [`init`](Pca9685::init) follows the datasheet's prescale procedure: the
/// oscillator is put to sleep, because the prescale register only takes a write
/// while it is, the prescale for the requested frequency is written, the output
/// wiring is set, the oscillator is woken, the 500 us it needs to stabilize pass,
/// and the restart bit is written so any channel loaded before the sleep resumes.
/// Register auto-increment stays on so a channel's four bytes load in one transfer.
/// As an [`Actuator`] the command is an [`Output`]; a servo is
/// [`Pwm::servo`] on its channel, and [`Actuator::map_command`] turns an angle or
/// a `bool` into one.
///
/// # Examples
///
/// The part's side of the conversation, scripted: initialization at the 50 Hz a
/// hobby servo wants, then one channel loaded with a centered 1.5 ms pulse.
///
/// ```
/// use pamoja_actuators::pca9685::{Output, Pca9685, Pwm, DEFAULT_I2C_ADDRESS};
/// use pamoja_core::Actuator;
/// use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
///
/// const PART: u8 = DEFAULT_I2C_ADDRESS;
/// let center = Pwm::servo(1_500, 50);
/// let [on_l, on_h, off_l, off_h] = center.bytes();
/// let bus = I2cScript::new([
///     I2cStep::write(PART, [0x00, 0x30]),
///     I2cStep::write(PART, [0xFE, 0x79]),
///     I2cStep::write(PART, [0x01, 0x04]),
///     I2cStep::write(PART, [0x00, 0x20]),
///     I2cStep::write(PART, [0x00, 0xA0]),
///     I2cStep::write(PART, [0x06, on_l, on_h, off_l, off_h]),
/// ]);
///
/// let mut board = Pca9685::new(bus, PART, DelayLog::new()).with_frequency(50);
/// block_on(board.apply(Output { channel: 0, pwm: center }))?;
/// let (bus, delay) = board.release();
/// assert!(bus.done());
/// assert_eq!(delay.total_micros(), 500);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Pca9685<I2C, D> {
    bus: I2C,
    delay: D,
    address: u8,
    frequency_hz: u32,
    oscillator_hz: u32,
    outputs: Outputs,
    initialized: bool,
}

impl<I2C, D> Pca9685<I2C, D> {
    /// Wraps an I2C bus and the part's address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `address` - the address the A5 to A0 pins select;
    ///   [`DEFAULT_I2C_ADDRESS`](super::DEFAULT_I2C_ADDRESS) with all six low.
    /// * `delay` - the timer that paces the oscillator start-up.
    ///
    /// # Returns
    ///
    /// The driver, at the part's default 200 Hz on its internal oscillator with
    /// totem-pole outputs.
    pub fn new(bus: I2C, address: u8, delay: D) -> Self {
        Pca9685 {
            bus,
            delay,
            address,
            frequency_hz: 200,
            oscillator_hz: INTERNAL_OSC_HZ,
            outputs: Outputs::default(),
            initialized: false,
        }
    }

    /// Sets the PWM frequency every channel shares, 24 to 1526 Hz.
    ///
    /// The prescaler can only approximate a frequency; [`frequency`](Pca9685::frequency)
    /// reports the one the part will run at.
    ///
    /// # Arguments
    ///
    /// * `hz` - the update rate; 50 Hz for hobby servos.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_frequency(mut self, hz: u32) -> Self {
        self.frequency_hz = hz;
        self.initialized = false;
        self
    }

    /// Sets the clock the prescaler divides, for a board that drives EXTCLK.
    ///
    /// # Arguments
    ///
    /// * `hz` - the oscillator frequency; the internal one is
    ///   [`INTERNAL_OSC_HZ`](super::INTERNAL_OSC_HZ).
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_oscillator(mut self, hz: u32) -> Self {
        self.oscillator_hz = hz;
        self.initialized = false;
        self
    }

    /// Sets how the outputs are wired.
    ///
    /// # Arguments
    ///
    /// * `outputs` - the MODE2 settings.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_outputs(mut self, outputs: Outputs) -> Self {
        self.outputs = outputs;
        self.initialized = false;
        self
    }

    /// Returns the prescale value the driver writes for its frequency.
    pub fn prescale(&self) -> u8 {
        prescale_for_frequency(self.frequency_hz, self.oscillator_hz)
    }

    /// Returns the frequency the part will run at, after the prescaler rounds.
    pub fn frequency(&self) -> f32 {
        frequency_for_prescale(self.prescale(), self.oscillator_hz)
    }

    /// Gives back the bus and the delay.
    ///
    /// # Returns
    ///
    /// The bus and delay the driver was built from.
    pub fn release(self) -> (I2C, D) {
        (self.bus, self.delay)
    }
}

impl<I2C: I2c, D: DelayNs> Pca9685<I2C, D> {
    /// Sets the prescale and output wiring and starts the oscillator.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn init(&mut self) -> Result<(), DriverError<I2C::Error>> {
        let awake = mode1::AUTO_INCREMENT;
        self.write(register::MODE1, awake | mode1::SLEEP)?;
        self.write(register::PRE_SCALE, self.prescale())?;
        self.write(register::MODE2, self.outputs.bits())?;
        self.write(register::MODE1, awake)?;
        self.delay.delay_us(OSCILLATOR_STARTUP_MICROS);
        self.write(register::MODE1, awake | mode1::RESTART)?;
        self.initialized = true;
        Ok(())
    }

    /// Loads one channel's on and off counts.
    ///
    /// Initializes the part first if [`init`](Pca9685::init) has not run.
    ///
    /// # Arguments
    ///
    /// * `channel` - the output, 0 to 15.
    /// * `pwm` - the counts to load.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Command`] for a channel the part does not have, and
    /// [`DriverError::Bus`] if the bus fails.
    pub fn set_channel(&mut self, channel: u8, pwm: Pwm) -> Result<(), DriverError<I2C::Error>> {
        if channel >= CHANNELS {
            return Err(DriverError::Command(
                "the PCA9685 has sixteen channels, 0 to 15",
            ));
        }
        if !self.initialized {
            self.init()?;
        }
        let [on_l, on_h, off_l, off_h] = pwm.bytes();
        self.bus
            .write(
                self.address,
                &[channel_register(channel), on_l, on_h, off_l, off_h],
            )
            .map_err(DriverError::Bus)
    }

    /// Loads every channel with the same on and off counts in one transfer.
    ///
    /// # Arguments
    ///
    /// * `pwm` - the counts to load.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn set_all(&mut self, pwm: Pwm) -> Result<(), DriverError<I2C::Error>> {
        if !self.initialized {
            self.init()?;
        }
        let [on_l, on_h, off_l, off_h] = pwm.bytes();
        self.bus
            .write(
                self.address,
                &[register::ALL_LED_ON_L, on_l, on_h, off_l, off_h],
            )
            .map_err(DriverError::Bus)
    }

    /// Stops the oscillator; the channel registers keep their values.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn sleep(&mut self) -> Result<(), DriverError<I2C::Error>> {
        self.write(register::MODE1, mode1::AUTO_INCREMENT | mode1::SLEEP)
    }

    /// Restarts the oscillator and every channel that was running before sleep.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn wake(&mut self) -> Result<(), DriverError<I2C::Error>> {
        self.write(register::MODE1, mode1::AUTO_INCREMENT)?;
        self.delay.delay_us(OSCILLATOR_STARTUP_MICROS);
        self.write(register::MODE1, mode1::AUTO_INCREMENT | mode1::RESTART)
    }

    /// Resets every PCA9685 on the bus to its power-up state.
    ///
    /// This is the general-call software reset the datasheet defines; it reaches
    /// every part on the bus that honors it, not only this one.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn software_reset(&mut self) -> Result<(), DriverError<I2C::Error>> {
        self.initialized = false;
        self.bus
            .write(SOFTWARE_RESET_ADDRESS, &[SOFTWARE_RESET_BYTE])
            .map_err(DriverError::Bus)
    }

    fn write(&mut self, register: u8, value: u8) -> Result<(), DriverError<I2C::Error>> {
        self.bus
            .write(self.address, &[register, value])
            .map_err(DriverError::Bus)
    }
}

impl<I2C, D> Actuator for Pca9685<I2C, D>
where
    I2C: I2c + Send,
    I2C::Error: core::fmt::Debug,
    D: DelayNs + Send,
{
    type Command = Output;

    async fn apply(&mut self, command: Output) -> pamoja_core::Result<()> {
        self.set_channel(command.channel, command.pwm)
            .map_err(pamoja_core::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pca9685::DEFAULT_I2C_ADDRESS;
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};

    fn init_steps(address: u8, prescale: u8, mode2: u8) -> Vec<I2cStep> {
        vec![
            I2cStep::write(address, [register::MODE1, 0x30]),
            I2cStep::write(address, [register::PRE_SCALE, prescale]),
            I2cStep::write(address, [register::MODE2, mode2]),
            I2cStep::write(address, [register::MODE1, 0x20]),
            I2cStep::write(address, [register::MODE1, 0xA0]),
        ]
    }

    #[test]
    fn init_sleeps_to_set_the_prescale_then_wakes_and_restarts() {
        let mut board = Pca9685::new(
            I2cScript::new(init_steps(DEFAULT_I2C_ADDRESS, 0x79, 0x04)),
            DEFAULT_I2C_ADDRESS,
            DelayLog::new(),
        )
        .with_frequency(50);
        assert_eq!(board.prescale(), 0x79, "the datasheet formula at 50 Hz");
        assert!((board.frequency() - 50.03).abs() < 0.01);
        board.init().unwrap();
        let (bus, delay) = board.release();
        assert!(bus.done());
        assert_eq!(delay.waits_ns(), [OSCILLATOR_STARTUP_MICROS * 1_000]);
    }

    #[test]
    fn the_default_frequency_is_the_datasheet_default_prescale() {
        let board = Pca9685::new(I2cScript::new([]), 0x41, DelayLog::new());
        assert_eq!(board.prescale(), super::super::PRE_SCALE_RESET);
    }

    #[test]
    fn channels_load_four_bytes_through_auto_increment() {
        let pulse = Pwm::servo(1_500, 50);
        let [on_l, on_h, off_l, off_h] = pulse.bytes();
        let mut steps = init_steps(0x41, 0x79, 0x14);
        steps.push(I2cStep::write(
            0x41,
            [0x06 + 4 * 15, on_l, on_h, off_l, off_h],
        ));
        steps.push(I2cStep::write(
            0x41,
            [register::ALL_LED_ON_L, 0x00, 0x10, 0x00, 0x00],
        ));
        let mut board = Pca9685::new(I2cScript::new(steps), 0x41, DelayLog::new())
            .with_frequency(50)
            .with_outputs(Outputs {
                inverted: true,
                ..Outputs::default()
            });
        board.set_channel(15, pulse).unwrap();
        board.set_all(Pwm::full_on()).unwrap();
        assert_eq!(
            board.set_channel(16, pulse),
            Err(DriverError::Command(
                "the PCA9685 has sixteen channels, 0 to 15"
            ))
        );
        assert!(board.release().0.done());
    }

    #[test]
    fn sleep_wake_and_software_reset_write_what_the_datasheet_names() {
        let steps = [
            I2cStep::write(0x40, [register::MODE1, 0x30]),
            I2cStep::write(0x40, [register::MODE1, 0x20]),
            I2cStep::write(0x40, [register::MODE1, 0xA0]),
            I2cStep::write(SOFTWARE_RESET_ADDRESS, [SOFTWARE_RESET_BYTE]),
        ];
        let mut board = Pca9685::new(I2cScript::new(steps), 0x40, DelayLog::new());
        board.sleep().unwrap();
        board.wake().unwrap();
        board.software_reset().unwrap();
        let (bus, delay) = board.release();
        assert!(bus.done());
        assert_eq!(delay.total_micros(), 500);
    }

    #[test]
    fn a_bus_fault_surfaces_as_a_bus_error_and_maps_to_the_core() {
        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let mut board = Pca9685::new(
            I2cScript::new([I2cStep::fault(0x40, kind)]),
            0x40,
            DelayLog::new(),
        );
        let error = board.init().unwrap_err();
        assert!(matches!(error, DriverError::Bus(_)));
        let core: pamoja_core::Error = error.into();
        assert!(matches!(core, pamoja_core::Error::Io(_)));
        let refused: pamoja_core::Error = DriverError::<ErrorKind>::Command("channel").into();
        assert!(matches!(refused, pamoja_core::Error::Codec(_)));
    }

    #[test]
    fn as_an_actuator_a_mapped_command_drives_a_servo_from_a_bool() {
        let open = Pwm::servo(2_000, 50);
        let closed = Pwm::servo(1_000, 50);
        let mut steps = init_steps(0x40, 0x79, 0x04);
        let [a, b, c, d] = open.bytes();
        steps.push(I2cStep::write(0x40, [0x06, a, b, c, d]));
        let [a, b, c, d] = closed.bytes();
        steps.push(I2cStep::write(0x40, [0x06, a, b, c, d]));
        let board = Pca9685::new(I2cScript::new(steps), 0x40, DelayLog::new()).with_frequency(50);
        let mut valve = board.map_command(|open_now: bool| Output {
            channel: 0,
            pwm: if open_now { open } else { closed },
        });
        block_on(valve.apply(true)).unwrap();
        block_on(valve.apply(false)).unwrap();
        assert!(valve.into_inner().release().0.done());
    }
}
