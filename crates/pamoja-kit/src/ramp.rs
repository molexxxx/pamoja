//! Easing a value toward a target at a limited rate.

/// Moves a value toward a target by at most a fixed step each update.
///
/// Commanding an actuator straight to a new value can be harsh: a motor lurches, a valve
/// slams, a lamp jumps. A [`Ramp`] limits how fast the commanded value may change, easing it
/// toward the target by at most `max_step` per update and snapping to the target once it is
/// within a step. It is the slew-rate limiter behind a smooth start and stop.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Ramp;
///
/// // Start at 0, move at most 2 per step, aim for 5.
/// let mut ramp = Ramp::new(0.0, 2.0);
/// assert_eq!(ramp.update(5.0), 2.0);
/// assert_eq!(ramp.update(5.0), 4.0);
/// assert_eq!(ramp.update(5.0), 5.0); // within a step: snaps to the target
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Ramp {
    value: f32,
    max_step: f32,
}

impl Ramp {
    /// Creates a ramp starting at `start` that moves at most `max_step` per update.
    ///
    /// # Arguments
    ///
    /// * `start` - the initial value.
    /// * `max_step` - the largest change allowed per update; its magnitude is used. A step
    ///   that is not a number holds the ramp still, and an infinite one sets no limit.
    ///
    /// # Returns
    ///
    /// A ramp resting at `start`.
    pub fn new(start: f32, max_step: f32) -> Self {
        Self {
            value: start,
            max_step: step(max_step),
        }
    }

    /// Moves toward `target` by at most the step and returns the new value.
    ///
    /// A target that is not a finite number is ignored and the value returned as it stood.
    ///
    /// # Arguments
    ///
    /// * `target` - the value being approached.
    ///
    /// # Returns
    ///
    /// The value after one limited step, equal to `target` once within a step of it.
    pub fn update(&mut self, target: f32) -> f32 {
        let step = self.max_step;
        self.update_capped(target, step)
    }

    /// Moves toward `target` by at most `max_step` this update, overriding the fixed rate.
    ///
    /// This is the variable-rate cousin of [`update`](Ramp::update): a bounded-acceleration
    /// limiter passes `accel * dt` as the step so the change allowed each update scales with
    /// the time since the last one, rather than the constant step fixed at construction.
    ///
    /// # Arguments
    ///
    /// * `target` - the value being approached. One that is not a finite number is ignored.
    /// * `max_step` - the largest change allowed this update; its magnitude is used. A step
    ///   that is not a number holds the ramp still.
    ///
    /// # Returns
    ///
    /// The value after one limited step, equal to `target` once within `max_step` of it.
    pub fn update_capped(&mut self, target: f32, max_step: f32) -> f32 {
        if !target.is_finite() {
            return self.value;
        }
        let limit = step(max_step);
        let delta = target - self.value;
        if delta > limit {
            self.value += limit;
        } else if delta < -limit {
            self.value -= limit;
        } else {
            self.value = target;
        }
        self.value
    }

    /// Returns the current value.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Jumps directly to `value`, bypassing the rate limit.
    ///
    /// # Arguments
    ///
    /// * `value` - the new value. One that is not a finite number is ignored.
    pub fn set(&mut self, value: f32) {
        if value.is_finite() {
            self.value = value;
        }
    }
}

// The magnitude of a step, taken by hand because `f32::abs` lives in `std`, with one that is
// not a number read as zero so the ramp holds still rather than jumping.
fn step(value: f32) -> f32 {
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
    fn it_climbs_toward_a_higher_target() {
        let mut ramp = Ramp::new(0.0, 2.0);
        assert_eq!(ramp.update(5.0), 2.0);
        assert_eq!(ramp.update(5.0), 4.0);
        assert_eq!(ramp.update(5.0), 5.0); // snaps within a step
        assert_eq!(ramp.update(5.0), 5.0); // holds at the target
    }

    #[test]
    fn it_falls_toward_a_lower_target() {
        let mut ramp = Ramp::new(10.0, 3.0);
        assert_eq!(ramp.update(0.0), 7.0);
        assert_eq!(ramp.update(0.0), 4.0);
        assert_eq!(ramp.update(0.0), 1.0);
        assert_eq!(ramp.update(0.0), 0.0);
    }

    #[test]
    fn a_negative_step_is_treated_as_its_magnitude() {
        let mut ramp = Ramp::new(0.0, -2.0);
        assert_eq!(ramp.update(10.0), 2.0);
    }

    #[test]
    fn set_jumps_past_the_limit() {
        let mut ramp = Ramp::new(0.0, 1.0);
        ramp.set(100.0);
        assert_eq!(ramp.value(), 100.0);
    }

    #[test]
    fn update_capped_overrides_the_fixed_rate() {
        // The construction step is 1.0, but each call here may move up to 0.25.
        let mut ramp = Ramp::new(0.0, 1.0);
        assert_eq!(ramp.update_capped(5.0, 0.25), 0.25);
        assert_eq!(ramp.update_capped(5.0, 0.25), 0.5);
        // A negative cap is treated as its magnitude.
        assert_eq!(ramp.update_capped(5.0, -0.5), 1.0);
    }

    #[test]
    fn a_target_that_is_not_a_number_is_ignored() {
        let mut ramp = Ramp::new(0.0, 2.0);
        ramp.update(5.0);
        assert_eq!(ramp.update(f32::NAN), 2.0);
        assert_eq!(ramp.update(f32::INFINITY), 2.0);
        ramp.set(f32::NAN);
        assert_eq!(ramp.value(), 2.0);
        assert_eq!(ramp.update(5.0), 4.0);
    }

    #[test]
    fn a_step_that_is_not_a_number_holds_still_and_an_infinite_one_sets_no_limit() {
        let mut stuck = Ramp::new(0.0, f32::NAN);
        assert_eq!(stuck.update(5.0), 0.0);
        assert_eq!(stuck.update_capped(5.0, f32::NAN), 0.0);
        let mut free = Ramp::new(0.0, f32::INFINITY);
        assert_eq!(free.update(5.0), 5.0);
    }
}
