//! Fusing a fast rate sensor with a slow absolute one.

/// Blends a drifting rate measurement with a noisy absolute one into a steady estimate.
///
/// A gyroscope gives a smooth rate of turn but drifts over time; an accelerometer gives an
/// absolute tilt that is right on average but noisy. A complementary filter trusts the rate
/// over the short term and the absolute reading over the long term, so the result is both
/// smooth and drift-free. Each step it integrates the rate onto its estimate, then nudges
/// that toward the absolute reading by an amount set by `alpha`. The same filter fuses any
/// fast-rate-plus-slow-absolute pair, not only an IMU.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Complementary;
///
/// // Heavily trust the integrated rate, lightly correct toward the absolute reading.
/// let mut tilt = Complementary::new(0.98, 0.0);
/// // The rate reads +10 per second for 0.1 s while the absolute reads about 1.
/// let angle = tilt.update(10.0, 1.0, 0.1);
/// assert!((angle - 1.0).abs() < 0.05); // about 0.98 * 1 + 0.02 * 1
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Complementary {
    estimate: f32,
    alpha: f32,
}

impl Complementary {
    /// Creates a filter.
    ///
    /// # Arguments
    ///
    /// * `alpha` - the weight on the integrated rate, in `[0.0, 1.0]`; near `1.0` trusts the
    ///   rate and corrects slowly, near `0.0` follows the absolute reading. Clamped to the
    ///   unit interval, and one that is not a number is taken as `0.0`, which follows the
    ///   absolute reading since that one cannot drift.
    /// * `initial` - the starting estimate.
    ///
    /// # Returns
    ///
    /// A filter seeded with `initial`.
    pub fn new(alpha: f32, initial: f32) -> Self {
        Self {
            estimate: initial,
            alpha: unit_interval(alpha),
        }
    }

    /// Fuses a rate and an absolute reading over a time step and returns the new estimate.
    ///
    /// If the rate, the absolute reading, or the time step is not a finite number, such as the
    /// NaN a failed sensor reports, the update is ignored and the estimate returned as it stood.
    ///
    /// # Arguments
    ///
    /// * `rate` - the rate of change, such as degrees per second from a gyroscope.
    /// * `absolute` - the absolute reading, such as a tilt from an accelerometer.
    /// * `dt` - the time since the previous update.
    ///
    /// # Returns
    ///
    /// The fused estimate, `alpha * (estimate + rate * dt) + (1 - alpha) * absolute`.
    pub fn update(&mut self, rate: f32, absolute: f32, dt: f32) -> f32 {
        if !rate.is_finite() || !absolute.is_finite() || !dt.is_finite() {
            return self.estimate;
        }
        let integrated = self.estimate + rate * dt;
        self.estimate = self.alpha * integrated + (1.0 - self.alpha) * absolute;
        self.estimate
    }

    /// Returns the current fused estimate.
    pub fn estimate(&self) -> f32 {
        self.estimate
    }
}

// `f32::clamp` lives in `std`, so this `no_std` crate clamps by hand.
#[allow(clippy::manual_clamp)]
fn unit_interval(value: f32) -> f32 {
    if value.is_nan() || value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_one_integrates_the_rate_only() {
        let mut filter = Complementary::new(1.0, 0.0);
        assert!((filter.update(10.0, 999.0, 1.0) - 10.0).abs() < 1e-6); // ignores absolute
        assert!((filter.update(10.0, 999.0, 1.0) - 20.0).abs() < 1e-6);
    }

    #[test]
    fn alpha_zero_follows_the_absolute_reading() {
        let mut filter = Complementary::new(0.0, 0.0);
        assert!((filter.update(10.0, 3.0, 1.0) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn it_blends_rate_and_absolute() {
        let mut filter = Complementary::new(0.9, 0.0);
        // integrated = 0 + 2 * 1 = 2; 0.9 * 2 + 0.1 * 0 = 1.8
        assert!((filter.update(2.0, 0.0, 1.0) - 1.8).abs() < 1e-6);
    }

    #[test]
    fn alpha_is_clamped_to_the_unit_interval() {
        let mut filter = Complementary::new(5.0, 0.0); // clamps to 1.0
        assert!((filter.update(4.0, 100.0, 1.0) - 4.0).abs() < 1e-6);
    }

    #[test]
    fn an_input_that_is_not_a_number_is_ignored() {
        let mut tilt = Complementary::new(0.98, 0.0);
        let settled = tilt.update(10.0, 1.0, 0.1);
        assert_eq!(tilt.update(f32::NAN, 1.0, 0.1), settled);
        assert_eq!(tilt.update(10.0, f32::INFINITY, 0.1), settled);
        assert_eq!(tilt.update(10.0, 1.0, f32::NAN), settled);
        assert!(tilt.update(10.0, 1.0, 0.1).is_finite());
    }

    #[test]
    fn an_alpha_that_is_not_a_number_follows_the_absolute_reading() {
        let mut tilt = Complementary::new(f32::NAN, 0.0);
        assert_eq!(tilt.update(10.0, 3.0, 0.1), 3.0);
    }
}
