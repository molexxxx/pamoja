//! Stepper motors driven through pins: four coil lines, or a step and direction pair.

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use pamoja_core::Actuator;

use super::{Direction, Drive, Position, Sequencer};
use crate::error::DriverError;

/// The pause between steps a driver starts with, in microseconds.
///
/// Two milliseconds a step is a rate every common geared hobby motor follows
/// without stalling; a motor's own datasheet gives the fastest it can take.
pub const DEFAULT_STEP_MICROS: u32 = 2_000;

/// The width of a step pulse a step/direction driver starts with, in microseconds.
///
/// Step/direction driver chips need a pulse of at least a microsecond or so in
/// each state; ten leaves room for a slow pin. The chip's datasheet gives its minimum.
pub const DEFAULT_PULSE_MICROS: u32 = 10;

fn direction_of(steps: i32) -> Direction {
    if steps < 0 {
        Direction::Backward
    } else {
        Direction::Forward
    }
}

/// A four-wire stepper driven coil by coil, through a transistor array such as a
/// ULN2003 or an H-bridge.
///
/// Each step energizes the coils the [`Drive`] pattern names, pin A for the
/// pattern's high bit down to pin D for its low bit, then waits the step interval.
/// The [`Position`] counts steps from where the motor was when the driver was
/// built. As an [`Actuator`] the command is a signed step count.
///
/// # Examples
///
/// ```
/// use pamoja_actuators::stepper::{Drive, FourWire};
/// use pamoja_core::Actuator;
/// use pamoja_hal::digital::PinState::{High, Low};
/// use pamoja_hal::script::{block_on, DelayLog, PinScript};
///
/// let pins = (PinScript::new([]), PinScript::new([]), PinScript::new([]), PinScript::new([]));
/// let mut motor = FourWire::new(pins, DelayLog::new(), Drive::FullStep);
/// block_on(motor.apply(2))?;
/// assert_eq!(motor.position(), 2);
///
/// // Each step advances to the next full-step pattern: 0110 then 0011, so coil A
/// // stays off and coil B is on for the first step only.
/// let ((a, b, _, _), _) = motor.release();
/// assert_eq!(a.driven(), [Low, Low]);
/// assert_eq!(b.driven(), [High, Low]);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct FourWire<A, B, C, Dp, D> {
    coils: (A, B, C, Dp),
    delay: D,
    sequencer: Sequencer,
    position: Position,
    step_micros: u32,
}

impl<A, B, C, Dp, D> FourWire<A, B, C, Dp, D> {
    /// Wraps the four coil lines, without driving them.
    ///
    /// # Arguments
    ///
    /// * `coils` - the lines for coils A, B, C, and D, in the motor's phase order.
    /// * `delay` - the timer that paces the steps.
    /// * `drive` - the coil pattern to step through.
    ///
    /// # Returns
    ///
    /// The driver, at [`DEFAULT_STEP_MICROS`] a step, counting from zero.
    pub fn new(coils: (A, B, C, Dp), delay: D, drive: Drive) -> Self {
        FourWire {
            coils,
            delay,
            sequencer: Sequencer::new(drive),
            position: Position::new(),
            step_micros: DEFAULT_STEP_MICROS,
        }
    }

    /// Sets the pause after each step, which sets the speed.
    ///
    /// # Arguments
    ///
    /// * `micros` - the interval in microseconds.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_step_interval(mut self, micros: u32) -> Self {
        self.step_micros = micros;
        self
    }

    /// Returns the step count since the driver was built, forward positive.
    pub fn position(&self) -> i32 {
        self.position.steps()
    }

    /// Returns the drive pattern in use.
    pub fn drive(&self) -> Drive {
        self.sequencer.drive()
    }

    /// Gives back the coil lines and the delay.
    ///
    /// # Returns
    ///
    /// The pins and delay the driver was built from.
    pub fn release(self) -> ((A, B, C, Dp), D) {
        (self.coils, self.delay)
    }
}

impl<A, B, C, Dp, D, E> FourWire<A, B, C, Dp, D>
where
    A: OutputPin<Error = E>,
    B: OutputPin<Error = E>,
    C: OutputPin<Error = E>,
    Dp: OutputPin<Error = E>,
    D: DelayNs,
{
    /// Takes one step and waits the step interval.
    ///
    /// # Arguments
    ///
    /// * `direction` - which way to step.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if a coil line cannot be driven.
    pub fn step(&mut self, direction: Direction) -> Result<(), DriverError<E>> {
        let coils = self.sequencer.step(direction);
        self.energize(coils)?;
        self.position.step(direction);
        self.delay.delay_us(self.step_micros);
        Ok(())
    }

    /// Takes `count` steps, backward when negative.
    ///
    /// # Arguments
    ///
    /// * `count` - the signed number of steps.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if a coil line cannot be driven; the steps
    /// already taken stay counted.
    pub fn steps(&mut self, count: i32) -> Result<(), DriverError<E>> {
        let direction = direction_of(count);
        for _ in 0..count.unsigned_abs() {
            self.step(direction)?;
        }
        Ok(())
    }

    /// Drops every coil, so the motor holds nothing and draws nothing.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if a coil line cannot be driven.
    pub fn idle(&mut self) -> Result<(), DriverError<E>> {
        self.energize(0)
    }

    fn energize(&mut self, coils: u8) -> Result<(), DriverError<E>> {
        let (a, b, c, d) = &mut self.coils;
        a.set_state((coils & 0b1000 != 0).into())
            .map_err(DriverError::Bus)?;
        b.set_state((coils & 0b0100 != 0).into())
            .map_err(DriverError::Bus)?;
        c.set_state((coils & 0b0010 != 0).into())
            .map_err(DriverError::Bus)?;
        d.set_state((coils & 0b0001 != 0).into())
            .map_err(DriverError::Bus)
    }
}

impl<A, B, C, Dp, D, E> Actuator for FourWire<A, B, C, Dp, D>
where
    A: OutputPin<Error = E> + Send,
    B: OutputPin<Error = E> + Send,
    C: OutputPin<Error = E> + Send,
    Dp: OutputPin<Error = E> + Send,
    D: DelayNs + Send,
    E: core::fmt::Debug,
{
    type Command = i32;

    async fn apply(&mut self, count: i32) -> pamoja_core::Result<()> {
        self.steps(count).map_err(pamoja_core::Error::from)
    }
}

/// A stepper behind a step/direction driver chip such as an A4988 or a DRV8825.
///
/// Each step sets the direction line, pulses the step line, and waits the step
/// interval. Microstepping, current limiting, and enable are the chip's own pins
/// and settings, outside this driver. As an [`Actuator`] the command is a signed
/// step count.
///
/// # Examples
///
/// ```
/// use pamoja_actuators::stepper::StepDir;
/// use pamoja_core::Actuator;
/// use pamoja_hal::digital::PinState::{High, Low};
/// use pamoja_hal::script::{block_on, DelayLog, PinScript};
///
/// let mut motor = StepDir::new(PinScript::new([]), PinScript::new([]), DelayLog::new());
/// block_on(motor.apply(-1))?;
/// assert_eq!(motor.position(), -1);
///
/// let ((step, direction), _) = motor.release();
/// assert_eq!(direction.driven(), [Low]);
/// assert_eq!(step.driven(), [High, Low]);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct StepDir<S, R, D> {
    step: S,
    direction: R,
    delay: D,
    position: Position,
    pulse_micros: u32,
    step_micros: u32,
}

impl<S, R, D> StepDir<S, R, D> {
    /// Wraps the step and direction lines, without driving them.
    ///
    /// # Arguments
    ///
    /// * `step` - the line the chip counts rising edges on.
    /// * `direction` - the line the chip reads the direction from; high is forward.
    /// * `delay` - the timer that shapes the pulses and paces the steps.
    ///
    /// # Returns
    ///
    /// The driver, with [`DEFAULT_PULSE_MICROS`] pulses at [`DEFAULT_STEP_MICROS`]
    /// a step, counting from zero.
    pub fn new(step: S, direction: R, delay: D) -> Self {
        StepDir {
            step,
            direction,
            delay,
            position: Position::new(),
            pulse_micros: DEFAULT_PULSE_MICROS,
            step_micros: DEFAULT_STEP_MICROS,
        }
    }

    /// Sets the width of each half of the step pulse.
    ///
    /// # Arguments
    ///
    /// * `micros` - the time the step line is held high, and then low, per step.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_pulse_width(mut self, micros: u32) -> Self {
        self.pulse_micros = micros;
        self
    }

    /// Sets the pause after each step, which sets the speed.
    ///
    /// # Arguments
    ///
    /// * `micros` - the interval in microseconds.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_step_interval(mut self, micros: u32) -> Self {
        self.step_micros = micros;
        self
    }

    /// Returns the step count since the driver was built, forward positive.
    pub fn position(&self) -> i32 {
        self.position.steps()
    }

    /// Gives back the lines and the delay.
    ///
    /// # Returns
    ///
    /// The step and direction pins and the delay the driver was built from.
    pub fn release(self) -> ((S, R), D) {
        ((self.step, self.direction), self.delay)
    }
}

impl<S, R, D, E> StepDir<S, R, D>
where
    S: OutputPin<Error = E>,
    R: OutputPin<Error = E>,
    D: DelayNs,
{
    /// Takes one step and waits the step interval.
    ///
    /// # Arguments
    ///
    /// * `direction` - which way to step.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if a line cannot be driven.
    pub fn step(&mut self, direction: Direction) -> Result<(), DriverError<E>> {
        self.direction
            .set_state((direction == Direction::Forward).into())
            .map_err(DriverError::Bus)?;
        self.step.set_high().map_err(DriverError::Bus)?;
        self.delay.delay_us(self.pulse_micros);
        self.step.set_low().map_err(DriverError::Bus)?;
        self.delay.delay_us(self.pulse_micros);
        self.position.step(direction);
        self.delay.delay_us(self.step_micros);
        Ok(())
    }

    /// Takes `count` steps, backward when negative.
    ///
    /// # Arguments
    ///
    /// * `count` - the signed number of steps.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if a line cannot be driven; the steps already
    /// taken stay counted.
    pub fn steps(&mut self, count: i32) -> Result<(), DriverError<E>> {
        let direction = direction_of(count);
        for _ in 0..count.unsigned_abs() {
            self.step(direction)?;
        }
        Ok(())
    }
}

impl<S, R, D, E> Actuator for StepDir<S, R, D>
where
    S: OutputPin<Error = E> + Send,
    R: OutputPin<Error = E> + Send,
    D: DelayNs + Send,
    E: core::fmt::Debug,
{
    type Command = i32;

    async fn apply(&mut self, count: i32) -> pamoja_core::Result<()> {
        self.steps(count).map_err(pamoja_core::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pamoja_hal::digital::PinState::{High, Low};
    use pamoja_hal::script::{block_on, DelayLog, PinScript};

    fn pins() -> (PinScript, PinScript, PinScript, PinScript) {
        (
            PinScript::new([]),
            PinScript::new([]),
            PinScript::new([]),
            PinScript::new([]),
        )
    }

    #[test]
    fn full_steps_energize_the_datasheet_pairs_in_order() {
        let mut motor =
            FourWire::new(pins(), DelayLog::new(), Drive::FullStep).with_step_interval(1_500);
        motor.steps(4).unwrap();
        assert_eq!(motor.position(), 4);
        assert_eq!(motor.drive(), Drive::FullStep);
        let ((a, b, c, d), delay) = motor.release();
        assert_eq!(a.driven(), [Low, Low, High, High]);
        assert_eq!(b.driven(), [High, Low, Low, High]);
        assert_eq!(c.driven(), [High, High, Low, Low]);
        assert_eq!(d.driven(), [Low, High, High, Low]);
        assert_eq!(delay.waits_ns(), [1_500_000; 4]);
    }

    #[test]
    fn backward_steps_walk_the_pattern_the_other_way_and_count_down() {
        let mut motor = FourWire::new(pins(), DelayLog::new(), Drive::Wave);
        motor.steps(-2).unwrap();
        assert_eq!(motor.position(), -2);
        motor.idle().unwrap();
        let ((a, _, _, d), _) = motor.release();
        assert_eq!(a.driven(), [Low, Low, Low]);
        assert_eq!(
            d.driven(),
            [High, Low, Low],
            "wave drive backward starts at coil D"
        );
    }

    #[test]
    fn a_four_wire_motor_is_an_actuator_taking_signed_steps() {
        let mut motor = FourWire::new(pins(), DelayLog::new(), Drive::HalfStep);
        block_on(motor.apply(3)).unwrap();
        block_on(motor.apply(-1)).unwrap();
        assert_eq!(motor.position(), 2);
    }

    #[test]
    fn step_dir_pulses_the_step_line_with_the_direction_set_first() {
        let mut motor = StepDir::new(PinScript::new([]), PinScript::new([]), DelayLog::new())
            .with_pulse_width(5)
            .with_step_interval(1_000);
        motor.steps(2).unwrap();
        motor.steps(-1).unwrap();
        assert_eq!(motor.position(), 1);
        let ((step, direction), delay) = motor.release();
        assert_eq!(direction.driven(), [High, High, Low]);
        assert_eq!(step.driven(), [High, Low, High, Low, High, Low]);
        assert_eq!(delay.waits_ns()[..3], [5_000, 5_000, 1_000_000]);
    }

    #[test]
    fn a_step_dir_motor_is_an_actuator_taking_signed_steps() {
        let mut motor = StepDir::new(PinScript::new([]), PinScript::new([]), DelayLog::new());
        block_on(motor.apply(-4)).unwrap();
        assert_eq!(motor.position(), -4);
    }
}
