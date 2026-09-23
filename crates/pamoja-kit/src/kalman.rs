//! Getting a steady value from a jittery sensor.

/// Estimates a steady value from noisy readings with a one-dimensional Kalman filter.
///
/// Where [`Smoother`](crate::Smoother) blends with a fixed weight, a Kalman filter sets the
/// blend from how much it trusts its estimate versus each reading, so it settles quickly and
/// then holds steady. It tracks an estimate and its uncertainty: each step it grows the
/// uncertainty by the process noise (how much the true value may drift between readings),
/// then pulls the estimate toward the new reading by a gain derived from that uncertainty
/// and the measurement noise (how noisy the sensor is). It suits a slowly changing quantity
/// read by a noisy sensor: a battery voltage, a tank level, a temperature.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Kalman;
///
/// // Low process noise, higher measurement noise: trust history, smooth hard.
/// let mut level = Kalman::new(0.01, 1.0, 0.0);
/// let mut value = 0.0;
/// for reading in [10.0, 9.0, 11.0, 10.0, 10.0] {
///     value = level.update(reading);
/// }
/// assert!((value - 10.0).abs() < 1.0); // settles near the true 10
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Kalman {
    estimate: f32,
    error: f32,
    process: f32,
    measurement: f32,
    started: bool,
}

impl Kalman {
    /// Creates a filter.
    ///
    /// # Arguments
    ///
    /// * `process_noise` - how much the true value may change between readings; larger
    ///   tracks faster, smaller smooths harder. Its magnitude is used, and one that is not a
    ///   finite number is taken as zero.
    /// * `measurement_noise` - how noisy each reading is; larger trusts readings less. Its
    ///   magnitude is used, and one that is not a finite number is taken as zero, which
    ///   trusts every reading.
    /// * `initial` - the starting estimate, used until the first reading replaces it.
    ///
    /// # Returns
    ///
    /// A filter awaiting its first reading.
    pub fn new(process_noise: f32, measurement_noise: f32, initial: f32) -> Self {
        Self {
            estimate: initial,
            error: 1.0,
            process: noise(process_noise),
            measurement: noise(measurement_noise),
            started: false,
        }
    }

    /// Folds in a reading and returns the updated estimate.
    ///
    /// The first reading seeds the estimate and is returned unchanged. A reading that is not
    /// a finite number, such as the NaN a failed sensor reports, is ignored and the estimate
    /// returned as it stood.
    ///
    /// # Arguments
    ///
    /// * `reading` - the latest measurement.
    ///
    /// # Returns
    ///
    /// The filtered estimate after this reading.
    pub fn update(&mut self, reading: f32) -> f32 {
        if !reading.is_finite() {
            return self.estimate;
        }
        if !self.started {
            self.estimate = reading;
            self.started = true;
            return self.estimate;
        }
        let predicted_error = self.error + self.process;
        let gain = if self.measurement == 0.0 {
            1.0
        } else {
            predicted_error / (predicted_error + self.measurement)
        };
        self.estimate += gain * (reading - self.estimate);
        self.error = (1.0 - gain) * predicted_error;
        self.estimate
    }

    /// Returns the current estimate.
    pub fn estimate(&self) -> f32 {
        self.estimate
    }
}

// The magnitude of a noise, taken by hand because `f32::abs` lives in `std`, with one that
// is not a finite number read as zero.
fn noise(value: f32) -> f32 {
    if !value.is_finite() {
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
    fn the_first_reading_seeds_the_estimate() {
        let mut kalman = Kalman::new(0.1, 0.1, 0.0);
        assert_eq!(kalman.update(42.0), 42.0);
    }

    #[test]
    fn it_tracks_toward_a_new_level() {
        let mut kalman = Kalman::new(0.1, 1.0, 0.0);
        kalman.update(0.0); // seed at zero
        let mut value = 0.0;
        for _ in 0..50 {
            value = kalman.update(10.0);
        }
        assert!((value - 10.0).abs() < 0.5);
    }

    #[test]
    fn it_smooths_a_noisy_signal_toward_the_mean() {
        let mut kalman = Kalman::new(0.01, 1.0, 0.0);
        let mut value = 0.0;
        for reading in [10.0, 8.0, 12.0, 9.0, 11.0, 10.0, 10.0, 9.5, 10.5, 10.0] {
            value = kalman.update(reading);
        }
        assert!((value - 10.0).abs() < 1.0);
    }

    #[test]
    fn zero_measurement_noise_follows_the_reading() {
        let mut kalman = Kalman::new(1.0, 0.0, 0.0);
        kalman.update(5.0); // seeds
        assert_eq!(kalman.update(8.0), 8.0); // full trust in the reading
    }

    #[test]
    fn no_noise_at_all_still_gives_a_number() {
        let mut kalman = Kalman::new(0.0, 0.0, 0.0);
        for reading in [5.0, 6.0, 7.0, 8.0] {
            assert_eq!(kalman.update(reading), reading);
        }
    }

    #[test]
    fn a_reading_that_is_not_a_number_leaves_the_estimate_where_it_was() {
        let mut kalman = Kalman::new(0.1, 1.0, 0.0);
        let settled = kalman.update(10.0);
        assert_eq!(kalman.update(f32::NAN), settled);
        assert_eq!(kalman.update(f32::NEG_INFINITY), settled);
        assert!(kalman.update(11.0).is_finite());
        assert_eq!(Kalman::new(0.1, 1.0, 3.0).update(f32::NAN), 3.0);
    }

    #[test]
    fn a_noise_that_is_not_a_number_is_taken_as_zero() {
        let mut kalman = Kalman::new(f32::NAN, f32::INFINITY, 0.0);
        kalman.update(5.0);
        assert_eq!(kalman.update(8.0), 8.0);
    }
}
