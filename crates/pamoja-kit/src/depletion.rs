//! Warning before a falling level runs out.

/// Predicts how soon a falling level will reach a threshold.
///
/// This is the primitive behind "warn before a tank runs dry". Feed it successive
/// level readings and it estimates how many more samples remain before the level
/// reaches a low mark, so an alert can fire with time to act on it. The technique
/// one layer down is a linear extrapolation of the most recent rate of fall, so it
/// reacts to noise and pairs well with a [`Smoother`](crate::Smoother) on the input.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Depletion;
///
/// let mut tank = Depletion::new(0.0);
/// assert_eq!(tank.update(10.0), None); // first reading: no rate is known yet
/// assert_eq!(tank.update(8.0), Some(4)); // falling 2 per sample, 4 until empty
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Depletion {
    threshold: f32,
    last: Option<f32>,
}

impl Depletion {
    /// Creates a predictor that warns as the level approaches `threshold`.
    ///
    /// # Arguments
    ///
    /// * `threshold` - the low level to predict reaching, such as an empty tank. One that is
    ///   not a number never reports.
    ///
    /// # Returns
    ///
    /// A predictor awaiting its first two readings.
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            last: None,
        }
    }

    /// Records a reading and estimates the samples until the threshold is reached.
    ///
    /// A level that is not a finite number, such as the NaN a failed sensor reports, is
    /// ignored and reports nothing.
    ///
    /// # Arguments
    ///
    /// * `level` - the latest measured level.
    ///
    /// # Returns
    ///
    /// `Some(0)` if the level is already at or below the threshold; `Some(n)` for
    /// the estimated number of samples until it is reached at the current rate of
    /// fall; or `None` if the level is steady or rising, or if this is the first
    /// reading and no rate is known yet.
    pub fn update(&mut self, level: f32) -> Option<u32> {
        if !level.is_finite() {
            return None;
        }
        if self.threshold.is_nan() {
            self.last = Some(level);
            return None;
        }
        let estimate = if level <= self.threshold {
            Some(0)
        } else {
            match self.last {
                Some(previous) => {
                    let rate = previous - level;
                    if rate > 0.0 {
                        Some(ceil_samples((level - self.threshold) / rate))
                    } else {
                        None
                    }
                }
                None => None,
            }
        };
        self.last = Some(level);
        estimate
    }
}

// Rounds a positive sample count up; `f32::ceil` lives in `std`. A value at or beyond
// the `u32` range (including a tiny rate that makes the estimate enormous, or infinity)
// saturates rather than overflowing.
fn ceil_samples(value: f32) -> u32 {
    if value >= u32::MAX as f32 {
        return u32::MAX;
    }
    let whole = value as u32;
    if (whole as f32) < value {
        whole + 1
    } else {
        whole
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_reading_has_no_rate() {
        let mut tank = Depletion::new(0.0);
        assert_eq!(tank.update(10.0), None);
    }

    #[test]
    fn counts_down_as_the_level_falls() {
        let mut tank = Depletion::new(2.0);
        tank.update(10.0);
        assert_eq!(tank.update(8.0), Some(3)); // (8 - 2) / 2 = 3
        assert_eq!(tank.update(6.0), Some(2)); // (6 - 2) / 2 = 2
    }

    #[test]
    fn rounds_partial_samples_up() {
        let mut tank = Depletion::new(0.0);
        tank.update(10.0);
        assert_eq!(tank.update(7.0), Some(3)); // (7 - 0) / 3 = 2.33, rounded up
    }

    #[test]
    fn a_steady_or_rising_level_does_not_warn() {
        let mut tank = Depletion::new(2.0);
        tank.update(6.0);
        assert_eq!(tank.update(6.0), None); // steady
        assert_eq!(tank.update(7.0), None); // rising
    }

    #[test]
    fn at_or_below_the_threshold_is_zero() {
        let mut tank = Depletion::new(2.0);
        assert_eq!(tank.update(2.0), Some(0));
        assert_eq!(tank.update(1.0), Some(0));
    }

    #[test]
    fn an_enormous_estimate_saturates_rather_than_overflowing() {
        // A far-off threshold with a small rate makes the estimate exceed u32; it must
        // saturate instead of overflowing the count.
        let mut tank = Depletion::new(-1e10);
        tank.update(2.0);
        assert_eq!(tank.update(1.0), Some(u32::MAX));
    }

    #[test]
    fn a_level_that_is_not_a_number_is_ignored() {
        let mut tank = Depletion::new(0.0);
        assert_eq!(tank.update(10.0), None);
        assert_eq!(tank.update(f32::NAN), None);
        assert_eq!(tank.update(f32::NEG_INFINITY), None);
        assert_eq!(tank.update(8.0), Some(4));
    }

    #[test]
    fn a_threshold_that_is_not_a_number_never_reports() {
        let mut tank = Depletion::new(f32::NAN);
        assert_eq!(tank.update(10.0), None);
        assert_eq!(tank.update(8.0), None);
    }
}
