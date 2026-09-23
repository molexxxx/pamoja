//! Holding a value at a target with a PID controller.

/// Drives a measured value to a target by blending proportional, integral, and derivative
/// terms.
///
/// This is the workhorse continuous controller behind "keep it here": hold a heater at a
/// temperature, a pump at a pressure, a motor at a speed. It sums three responses to the
/// error (target minus measurement): the proportional term reacts to the error now, the
/// integral term removes the steady offset the proportional term leaves behind, and the
/// derivative term damps overshoot by reacting to how fast the error is changing. The gains
/// `kp`, `ki`, and `kd` weight them. The output is clamped to a configurable range, and the
/// integral is held back from winding up past that range while the output is saturated, the
/// standard clamping anti-windup.
///
/// For the simplest on/off case (a fridge, a tank pump) reach for
/// [`Thermostat`](crate::Thermostat) instead; a PID is for a smooth, proportional actuator.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Pid;
///
/// // Proportional-only: the command is the gain times the error.
/// let mut pid = Pid::new(2.0, 0.0, 0.0);
/// assert_eq!(pid.update(10.0, 7.0, 1.0), 6.0); // error 3 times kp 2
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Pid {
    kp: f32,
    ki: f32,
    kd: f32,
    integral: f32,
    last_error: Option<f32>,
    output: f32,
    min: f32,
    max: f32,
}

impl Pid {
    /// Creates a PID controller with the given gains and no output limit.
    ///
    /// # Arguments
    ///
    /// * `kp` - proportional gain.
    /// * `ki` - integral gain.
    /// * `kd` - derivative gain.
    ///
    /// A gain that is not a finite number is taken as zero.
    ///
    /// # Returns
    ///
    /// A controller with a cleared history and unbounded output.
    pub fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            kp: gain(kp),
            ki: gain(ki),
            kd: gain(kd),
            integral: 0.0,
            last_error: None,
            output: 0.0,
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
        }
    }

    /// Limits the output to `[min, max]`, also bounding the integral so it cannot wind up
    /// beyond the range while the output is saturated.
    ///
    /// # Arguments
    ///
    /// * `min` - the lowest output. One that is not a number leaves the low side open.
    /// * `max` - the highest output. One that is not a number leaves the high side open. If
    ///   `max` is below `min` the two are swapped.
    ///
    /// # Returns
    ///
    /// The controller, for chaining after [`new`](Pid::new).
    pub fn with_limits(mut self, min: f32, max: f32) -> Self {
        let min = if min.is_nan() { f32::NEG_INFINITY } else { min };
        let max = if max.is_nan() { f32::INFINITY } else { max };
        if min <= max {
            self.min = min;
            self.max = max;
        } else {
            self.min = max;
            self.max = min;
        }
        self
    }

    /// Computes the control output for one time step.
    ///
    /// # Arguments
    ///
    /// * `setpoint` - the target value.
    /// * `measurement` - the latest measured value.
    /// * `dt` - the time since the previous update, in the unit `ki` and `kd` assume. A
    ///   value at or below zero, or one that is not a finite number, skips the integral and
    ///   derivative updates.
    ///
    /// A setpoint or measurement that is not a finite number, such as the NaN a failed
    /// sensor reports, is ignored: the controller keeps its history and returns its previous
    /// output, where one NaN would otherwise stay in the integral for good.
    ///
    /// # Returns
    ///
    /// The control output, clamped to the configured limits.
    pub fn update(&mut self, setpoint: f32, measurement: f32, dt: f32) -> f32 {
        if !setpoint.is_finite() || !measurement.is_finite() {
            return self.output;
        }
        let stepped = dt > 0.0 && dt.is_finite();
        let error = setpoint - measurement;
        let derivative = match self.last_error {
            Some(previous) if stepped => (error - previous) / dt,
            _ => 0.0,
        };
        self.last_error = Some(error);

        if stepped && self.ki != 0.0 {
            self.integral += error * dt;
            // Anti-windup: hold the integral term inside the output range.
            let term = self.ki * self.integral;
            let bounded = clamp(term, self.min, self.max);
            self.integral = bounded / self.ki;
        }

        let output = self.kp * error + self.ki * self.integral + self.kd * derivative;
        self.output = clamp(output, self.min, self.max);
        self.output
    }

    /// Clears the integral and derivative history, and the output it last returned.
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.last_error = None;
        self.output = 0.0;
    }
}

// A gain that is not a finite number contributes nothing rather than poisoning the output.
fn gain(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

// `f32::clamp` lives in `std`, so this `no_std` crate clamps by hand.
#[allow(clippy::manual_clamp)]
fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proportional_only_scales_the_error() {
        let mut pid = Pid::new(2.0, 0.0, 0.0);
        assert_eq!(pid.update(10.0, 7.0, 1.0), 6.0);
    }

    #[test]
    fn the_integral_term_accumulates() {
        let mut pid = Pid::new(0.0, 0.5, 0.0);
        assert_eq!(pid.update(2.0, 0.0, 1.0), 1.0); // integral 2 times ki 0.5
        assert_eq!(pid.update(2.0, 0.0, 1.0), 2.0); // integral 4 times 0.5
    }

    #[test]
    fn the_derivative_term_responds_to_change() {
        let mut pid = Pid::new(0.0, 0.0, 1.0);
        assert_eq!(pid.update(0.0, 0.0, 1.0), 0.0); // first step: no history
        assert_eq!(pid.update(0.0, 5.0, 1.0), -5.0); // error fell by 5 over dt 1
    }

    #[test]
    fn integral_does_not_wind_up_while_saturated() {
        let mut pid = Pid::new(0.0, 1.0, 0.0).with_limits(-5.0, 5.0);
        for _ in 0..10 {
            pid.update(100.0, 0.0, 1.0);
        }
        // Pinned at the limit, but the integral has not wound up past it.
        assert_eq!(pid.update(100.0, 0.0, 1.0), 5.0);
        // So reversing the error leaves saturation immediately, with no wound-up
        // integral holding the output high.
        assert_eq!(pid.update(-100.0, 0.0, 1.0), -5.0);
    }

    #[test]
    fn a_reading_that_is_not_a_number_does_not_poison_the_controller() {
        let mut pid = Pid::new(2.0, 0.5, 0.1);
        let before = pid.update(10.0, 7.0, 1.0);
        assert_eq!(pid.update(10.0, f32::NAN, 1.0), before);
        assert_eq!(pid.update(10.0, f32::INFINITY, 1.0), before);
        assert_eq!(pid.update(f32::NAN, 7.0, 1.0), before);
        assert!(pid.update(10.0, 7.0, 1.0).is_finite());
        assert!(pid.update(10.0, 7.0, f32::NAN).is_finite());
        assert!(pid.update(10.0, 7.0, f32::INFINITY).is_finite());
        assert!(pid.update(10.0, 7.0, 1.0).is_finite());
    }

    #[test]
    fn a_limit_that_is_not_a_number_leaves_that_side_open() {
        let mut pid = Pid::new(1.0, 0.0, 0.0).with_limits(f32::NAN, 5.0);
        assert_eq!(pid.update(100.0, 0.0, 1.0), 5.0);
        assert_eq!(pid.update(-100.0, 0.0, 1.0), -100.0);
    }

    #[test]
    fn a_gain_that_is_not_a_number_is_taken_as_zero() {
        let mut pid = Pid::new(2.0, f32::NAN, f32::INFINITY);
        assert_eq!(pid.update(10.0, 7.0, 1.0), 6.0);
    }

    #[test]
    fn reset_clears_the_history() {
        let mut pid = Pid::new(1.0, 1.0, 0.0);
        pid.update(10.0, 0.0, 1.0);
        pid.reset();
        assert_eq!(pid.update(10.0, 0.0, 0.0), 10.0); // dt 0: proportional only
    }
}
