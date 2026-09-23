//! Keeping a reading near a setpoint with on/off control.

/// A hysteresis (bang-bang) controller for a single on/off actuator.
///
/// This is the controller behind "keep a temperature". It switches a cooler or
/// heater on and off to hold a reading near a setpoint. A deadband around the
/// setpoint - the hysteresis - stops the output chattering when the reading hovers
/// at the threshold, which protects relays and compressors that have a limited
/// number of switching cycles in them.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Thermostat;
///
/// let mut fridge = Thermostat::cooling(4.0, 0.5);
/// assert!(fridge.update(5.0)); // above the deadband: the cooler runs
/// assert!(fridge.update(4.2)); // inside the deadband: it holds its state
/// assert!(!fridge.update(3.4)); // below the deadband: the cooler stops
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thermostat {
    setpoint: f32,
    hysteresis: f32,
    cools: bool,
    on: bool,
}

impl Thermostat {
    /// Creates a thermostat that drives a cooler, such as a fridge.
    ///
    /// The output turns on when the reading reaches the top of the deadband and off
    /// when it reaches the bottom.
    ///
    /// # Arguments
    ///
    /// * `setpoint` - the target reading. One that is not a number never switches.
    /// * `hysteresis` - half the deadband width; its magnitude is used, and one that is not
    ///   a number is taken as zero.
    ///
    /// # Returns
    ///
    /// A thermostat whose output starts off.
    pub fn cooling(setpoint: f32, hysteresis: f32) -> Self {
        Self {
            setpoint,
            hysteresis: magnitude(hysteresis),
            cools: true,
            on: false,
        }
    }

    /// Creates a thermostat that drives a heater.
    ///
    /// The output turns on when the reading reaches the bottom of the deadband and off
    /// when it reaches the top.
    ///
    /// # Arguments
    ///
    /// * `setpoint` - the target reading. One that is not a number never switches.
    /// * `hysteresis` - half the deadband width; its magnitude is used, and one that is not
    ///   a number is taken as zero.
    ///
    /// # Returns
    ///
    /// A thermostat whose output starts off.
    pub fn heating(setpoint: f32, hysteresis: f32) -> Self {
        Self {
            setpoint,
            hysteresis: magnitude(hysteresis),
            cools: false,
            on: false,
        }
    }

    /// Updates the controller with a reading and returns whether the output is on.
    ///
    /// A reading that is not a finite number, such as the NaN a failed probe reports, is
    /// ignored and the output stays as it was: a heater that was on stays on. A node that
    /// must fail safe checks the reading itself and decides what off means for its plant.
    ///
    /// # Arguments
    ///
    /// * `reading` - the latest measured value.
    ///
    /// # Returns
    ///
    /// `true` if the cooler or heater should be running.
    pub fn update(&mut self, reading: f32) -> bool {
        if !reading.is_finite() {
            return self.on;
        }
        let upper = self.setpoint + self.hysteresis;
        let lower = self.setpoint - self.hysteresis;
        if self.cools {
            if reading >= upper {
                self.on = true;
            } else if reading <= lower {
                self.on = false;
            }
        } else if reading <= lower {
            self.on = true;
        } else if reading >= upper {
            self.on = false;
        }
        self.on
    }

    /// Returns whether the output is currently on.
    ///
    /// # Returns
    ///
    /// `true` if the most recent [`update`](Self::update) left the output running.
    pub fn is_on(&self) -> bool {
        self.on
    }
}

// The magnitude of a hysteresis, taken by hand because `f32::abs` lives in `std`, with one
// that is not a number read as zero.
fn magnitude(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else if value < 0.0 {
        -value
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooling_switches_around_the_deadband() {
        let mut fridge = Thermostat::cooling(4.0, 0.5);
        assert!(!fridge.is_on());
        assert!(fridge.update(4.6)); // above 4.5: on
        assert!(fridge.update(4.2)); // in the deadband: holds on
        assert!(!fridge.update(3.4)); // below 3.5: off
        assert!(!fridge.update(4.2)); // in the deadband: holds off
    }

    #[test]
    fn heating_switches_the_other_way() {
        let mut heater = Thermostat::heating(20.0, 1.0);
        assert!(heater.update(18.5)); // below 19.0: on
        assert!(heater.update(19.5)); // in the deadband: holds on
        assert!(!heater.update(21.5)); // above 21.0: off
    }

    #[test]
    fn the_edge_of_the_deadband_switches() {
        let mut fridge = Thermostat::cooling(4.0, 0.5);
        assert!(fridge.update(4.5));
        assert!(!fridge.update(3.5));
    }

    #[test]
    fn a_reading_that_is_not_a_number_holds_the_output() {
        let mut heater = Thermostat::heating(20.0, 1.0);
        assert!(heater.update(18.5));
        assert!(heater.update(f32::NAN));
        assert!(heater.update(f32::INFINITY));
        assert!(!heater.update(21.5));
        assert!(!heater.update(f32::NEG_INFINITY));
    }

    #[test]
    fn a_hysteresis_that_is_not_a_number_is_taken_as_zero() {
        let mut fridge = Thermostat::cooling(4.0, f32::NAN);
        assert!(fridge.update(4.1));
        assert!(!fridge.update(3.9));
    }

    #[test]
    fn negative_hysteresis_is_treated_as_its_magnitude() {
        let mut fridge = Thermostat::cooling(4.0, -0.5);
        assert!(fridge.update(4.6));
        assert!(!fridge.update(3.4));
    }
}
