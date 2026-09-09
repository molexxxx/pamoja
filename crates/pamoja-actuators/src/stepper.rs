//! Coil sequencing for four-wire stepper motors.
//!
//! A stepper turns by energizing its coils in a repeating pattern; stepping through
//! the pattern in one direction or the other advances or reverses the shaft. This
//! module holds the three standard drive patterns and a sequencer that walks them, so
//! a caller toggles the four coil lines (directly, or through a darlington array like
//! the ULN2003) without hand-maintaining the sequence. It also models the
//! step-and-direction interface of driver chips such as the A4988 and DRV8825, which
//! take a step pulse and a direction level and track position as a signed count.
//!
//! Coil patterns are four bits, the most significant being the first coil (IN1 on a
//! ULN2003 board) down to the least significant (IN4).

#[cfg(feature = "embedded-hal")]
mod driver;

#[cfg(feature = "embedded-hal")]
pub use driver::{FourWire, StepDir, DEFAULT_PULSE_MICROS, DEFAULT_STEP_MICROS};

/// Which way to step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Advance the sequence, turning the shaft one way.
    Forward,
    /// Reverse the sequence, turning the shaft the other way.
    Backward,
}

/// A stepper drive pattern.
///
/// The three patterns trade torque, smoothness, and resolution. Wave drive energizes
/// one coil at a time (least torque, least power); full-step energizes two at a time
/// (most torque); half-step alternates between them to double the resolution at the
/// cost of uneven torque.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drive {
    /// One coil energized at a time: four steps.
    Wave,
    /// Two adjacent coils energized at a time: four steps, more torque.
    FullStep,
    /// Alternating one and two coils: eight steps, double resolution.
    HalfStep,
}

// The four wave-drive coil patterns, one coil at a time.
const WAVE: [u8; 4] = [0b1000, 0b0100, 0b0010, 0b0001];
// The four full-step patterns, two adjacent coils at a time.
const FULL_STEP: [u8; 4] = [0b1100, 0b0110, 0b0011, 0b1001];
// The eight half-step patterns, interleaving wave and full-step.
const HALF_STEP: [u8; 8] = [
    0b1000, 0b1100, 0b0100, 0b0110, 0b0010, 0b0011, 0b0001, 0b1001,
];

impl Drive {
    /// Returns the coil patterns for this drive, in forward order.
    ///
    /// # Returns
    ///
    /// A slice of four-bit coil patterns: four for wave and full-step, eight for
    /// half-step.
    pub fn pattern(self) -> &'static [u8] {
        match self {
            Drive::Wave => &WAVE,
            Drive::FullStep => &FULL_STEP,
            Drive::HalfStep => &HALF_STEP,
        }
    }

    /// Returns how many steps make up one full electrical cycle of this drive.
    ///
    /// # Returns
    ///
    /// `4` for wave and full-step, `8` for half-step.
    pub fn step_count(self) -> usize {
        self.pattern().len()
    }
}

/// A position in a drive sequence, walked one step at a time.
///
/// Holding the index into the pattern, a sequencer turns each [`step`](Sequencer::step)
/// into the next coil pattern to apply, wrapping around the cycle so it can run
/// indefinitely in either direction.
///
/// # Examples
///
/// ```
/// use pamoja_actuators::stepper::{Direction, Drive, Sequencer};
///
/// let mut sequencer = Sequencer::new(Drive::HalfStep);
/// assert_eq!(sequencer.coils(), 0b1000);
/// assert_eq!(sequencer.step(Direction::Forward), 0b1100);
///
/// // One full electrical cycle returns to the start.
/// for _ in 1..Drive::HalfStep.step_count() {
///     sequencer.step(Direction::Forward);
/// }
/// assert_eq!(sequencer.coils(), 0b1000);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sequencer {
    drive: Drive,
    index: usize,
}

impl Sequencer {
    /// Creates a sequencer at the start of a drive pattern.
    ///
    /// # Arguments
    ///
    /// * `drive` - the drive pattern to walk.
    ///
    /// # Returns
    ///
    /// A sequencer whose current pattern is the first in the drive.
    pub fn new(drive: Drive) -> Sequencer {
        Sequencer { drive, index: 0 }
    }

    /// Returns the coil pattern at the current position.
    ///
    /// # Returns
    ///
    /// The four-bit coil pattern to apply now.
    pub fn coils(&self) -> u8 {
        self.drive.pattern()[self.index]
    }

    /// Advances one step in `direction` and returns the new coil pattern.
    ///
    /// The index wraps around the cycle, so stepping forever in either direction is
    /// well defined.
    ///
    /// # Arguments
    ///
    /// * `direction` - which way to step.
    ///
    /// # Returns
    ///
    /// The coil pattern to apply after the step.
    pub fn step(&mut self, direction: Direction) -> u8 {
        let count = self.drive.step_count();
        self.index = match direction {
            Direction::Forward => (self.index + 1) % count,
            Direction::Backward => (self.index + count - 1) % count,
        };
        self.coils()
    }

    /// Returns the drive pattern this sequencer walks.
    pub fn drive(&self) -> Drive {
        self.drive
    }
}

/// A signed step counter for step-and-direction driver chips.
///
/// Driver chips like the A4988 and DRV8825 move one (micro)step per pulse in the
/// direction set on a level pin, so the host just counts. This tracks that count, so
/// position is known without reading the hardware.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Position {
    steps: i32,
}

impl Position {
    /// Creates a position at zero.
    ///
    /// # Returns
    ///
    /// A position whose step count is zero.
    pub fn new() -> Position {
        Position { steps: 0 }
    }

    /// Records one step in `direction` and returns the new step count.
    ///
    /// # Arguments
    ///
    /// * `direction` - which way the pulse moved the motor.
    ///
    /// # Returns
    ///
    /// The updated step count, increasing for [`Direction::Forward`].
    pub fn step(&mut self, direction: Direction) -> i32 {
        self.steps += match direction {
            Direction::Forward => 1,
            Direction::Backward => -1,
        };
        self.steps
    }

    /// Returns the current step count.
    pub fn steps(self) -> i32 {
        self.steps
    }
}

/// Converts an angle to a whole number of steps for a given motor.
///
/// # Arguments
///
/// * `degrees` - the angle to turn; negative turns the other way.
/// * `steps_per_revolution` - the motor's steps per full turn, for example 200 for a
///   1.8-degree motor.
///
/// # Returns
///
/// The nearest whole number of steps to that angle.
pub fn steps_for_degrees(degrees: f32, steps_per_revolution: u32) -> i32 {
    // `f32::round` lives in `std`; casting to `i32` truncates toward zero, so adding a
    // signed half first rounds to nearest without pulling in a floating-point library.
    let scaled = degrees / 360.0 * steps_per_revolution as f32;
    let half = if scaled >= 0.0 { 0.5 } else { -0.5 };
    (scaled + half) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_patterns_match_the_standard_sequences() {
        assert_eq!(Drive::Wave.pattern(), &[0b1000, 0b0100, 0b0010, 0b0001]);
        assert_eq!(Drive::FullStep.pattern(), &[0b1100, 0b0110, 0b0011, 0b1001]);
        assert_eq!(
            Drive::HalfStep.pattern(),
            &[0b1000, 0b1100, 0b0100, 0b0110, 0b0010, 0b0011, 0b0001, 0b1001]
        );
    }

    #[test]
    fn half_step_interleaves_wave_and_full_step() {
        let half = Drive::HalfStep.pattern();
        for i in 0..4 {
            assert_eq!(half[2 * i], Drive::Wave.pattern()[i]);
            assert_eq!(half[2 * i + 1], Drive::FullStep.pattern()[i]);
        }
    }

    #[test]
    fn a_full_cycle_returns_to_the_start() {
        for drive in [Drive::Wave, Drive::FullStep, Drive::HalfStep] {
            let mut sequencer = Sequencer::new(drive);
            let start = sequencer.coils();
            for _ in 0..drive.step_count() {
                sequencer.step(Direction::Forward);
            }
            assert_eq!(sequencer.coils(), start);
        }
    }

    #[test]
    fn stepping_back_undoes_a_step_forward() {
        let mut sequencer = Sequencer::new(Drive::FullStep);
        let start = sequencer.coils();
        sequencer.step(Direction::Forward);
        assert_eq!(sequencer.step(Direction::Backward), start);
    }

    #[test]
    fn backward_from_the_start_wraps_to_the_last_pattern() {
        let mut sequencer = Sequencer::new(Drive::Wave);
        assert_eq!(sequencer.step(Direction::Backward), 0b0001);
    }

    #[test]
    fn position_counts_signed_steps() {
        let mut position = Position::new();
        assert_eq!(position.step(Direction::Forward), 1);
        assert_eq!(position.step(Direction::Forward), 2);
        assert_eq!(position.step(Direction::Backward), 1);
        assert_eq!(position.steps(), 1);
    }

    #[test]
    fn degrees_convert_to_steps() {
        assert_eq!(steps_for_degrees(360.0, 200), 200);
        assert_eq!(steps_for_degrees(90.0, 200), 50);
        assert_eq!(steps_for_degrees(-90.0, 200), -50);
        // A 1.8-degree motor: one step is 1.8 degrees.
        assert_eq!(steps_for_degrees(1.8, 200), 1);
    }
}
