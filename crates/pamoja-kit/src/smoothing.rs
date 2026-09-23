//! Smoothing a noisy reading.

/// Smooths a noisy signal with an exponential moving average.
///
/// Cheap sensors are noisy, and a single bad sample should not trip an alarm or
/// flip an actuator. A [`Smoother`] dampens that jitter. The technique one layer
/// down is an exponential moving average: each output is a weighted blend of the
/// newest sample and the previous output, so recent readings count for more
/// without storing a history buffer.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Smoother;
///
/// let mut smoother = Smoother::new(0.5);
/// assert_eq!(smoother.update(10.0), 10.0); // the first sample seeds the average
/// assert_eq!(smoother.update(0.0), 5.0); // then each output blends halfway
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Smoother {
    weight: f32,
    value: Option<f32>,
}

impl Smoother {
    /// Creates a smoother with the given responsiveness.
    ///
    /// # Arguments
    ///
    /// * `weight` - how much the newest sample counts, clamped to `[0.0, 1.0]`.
    ///   `1.0` disables smoothing so the output follows the input; values near
    ///   `0.0` smooth heavily and react slowly. A weight that is not a number is
    ///   taken as `1.0`.
    ///
    /// # Returns
    ///
    /// A smoother awaiting its first sample.
    pub fn new(weight: f32) -> Self {
        Self {
            weight: unit_interval(weight),
            value: None,
        }
    }

    /// Folds a new sample into the average and returns the smoothed value.
    ///
    /// The first sample seeds the average and is returned unchanged. A sample that is not a
    /// finite number, such as the NaN a failed sensor reports, is ignored: the average stays
    /// where it was and is returned as it stood.
    ///
    /// # Arguments
    ///
    /// * `sample` - the latest raw reading.
    ///
    /// # Returns
    ///
    /// The smoothed value after including `sample`, or the sample itself when it is not a
    /// finite number and no real one has arrived yet.
    pub fn update(&mut self, sample: f32) -> f32 {
        if !sample.is_finite() {
            return self.value.unwrap_or(sample);
        }
        let smoothed = match self.value {
            Some(previous) => self.weight * sample + (1.0 - self.weight) * previous,
            None => sample,
        };
        self.value = Some(smoothed);
        smoothed
    }

    /// Returns the current smoothed value, or `None` before the first sample.
    ///
    /// # Returns
    ///
    /// `Some(value)` once at least one sample has been seen, otherwise `None`.
    pub fn value(&self) -> Option<f32> {
        self.value
    }

    /// Forgets the smoothed value so the next sample seeds the average afresh.
    pub fn reset(&mut self) {
        self.value = None;
    }
}

// `f32::clamp` lives in `std`, so this `no_std` crate clamps by hand.
#[allow(clippy::manual_clamp)]
fn unit_interval(value: f32) -> f32 {
    if value.is_nan() || value > 1.0 {
        1.0
    } else if value < 0.0 {
        0.0
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sample_seeds_the_average() {
        let mut smoother = Smoother::new(0.5);
        assert_eq!(smoother.update(10.0), 10.0);
        assert_eq!(smoother.value(), Some(10.0));
    }

    #[test]
    fn blends_toward_newer_samples() {
        let mut smoother = Smoother::new(0.5);
        smoother.update(0.0);
        assert!((smoother.update(10.0) - 5.0).abs() < 1e-6);
        assert!((smoother.update(10.0) - 7.5).abs() < 1e-6);
    }

    #[test]
    fn weight_is_clamped_into_the_unit_interval() {
        let mut smoother = Smoother::new(2.0); // clamps to 1.0: no smoothing
        smoother.update(1.0);
        assert!((smoother.update(9.0) - 9.0).abs() < 1e-6);
    }

    #[test]
    fn a_sample_that_is_not_a_number_leaves_the_average_where_it_was() {
        let mut smoother = Smoother::new(0.5);
        smoother.update(10.0);
        assert_eq!(smoother.update(f32::NAN), 10.0);
        assert_eq!(smoother.update(f32::INFINITY), 10.0);
        assert_eq!(smoother.update(0.0), 5.0);
        assert!(Smoother::new(0.5).update(f32::NAN).is_nan());
    }

    #[test]
    fn a_weight_that_is_not_a_number_follows_the_input() {
        let mut smoother = Smoother::new(f32::NAN);
        smoother.update(1.0);
        assert_eq!(smoother.update(9.0), 9.0);
    }

    #[test]
    fn reset_forgets_the_value() {
        let mut smoother = Smoother::new(0.5);
        smoother.update(4.0);
        smoother.reset();
        assert_eq!(smoother.value(), None);
        assert_eq!(smoother.update(8.0), 8.0);
    }
}
