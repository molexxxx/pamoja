//! Flagging a reading that departs from its recent history.

use crate::Window;

/// Flags a reading that lies far from the recent norm.
///
/// "Tell me when something is off" needs a baseline: a reading is suspicious only relative to
/// what is usual. An [`Anomaly`] keeps a rolling window of recent readings and flags one that
/// sits more than a chosen number of standard deviations from their mean - the three-sigma
/// rule, the standard z-score test, with the threshold left to the caller (`3.0` is the
/// usual choice). It is dependency-free: instead of taking a square root it compares the
/// squared deviation against the squared threshold, which is the same test.
///
/// With a perfectly flat baseline the spread is zero, so any change at all reads as
/// anomalous; real sensor noise gives a non-zero baseline, where this is not an issue.
///
/// A reading that is not a finite number, such as the NaN a failed sensor reports, is always
/// flagged, and is kept out of the baseline so it cannot blind the detector to the readings
/// after it.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Anomaly;
///
/// let mut watch = Anomaly::<8>::new(3.0);
/// // Establish a steady baseline.
/// for reading in [10.0, 10.2, 9.8, 10.1, 9.9, 10.0, 10.2, 9.8] {
///     watch.check(reading);
/// }
/// assert!(!watch.check(10.1)); // close to the norm: fine
/// assert!(watch.check(20.0)); // a far jump: flagged
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Anomaly<const N: usize> {
    window: Window<N>,
    sigmas: f32,
}

impl<const N: usize> Anomaly<N> {
    /// Creates a detector that flags readings beyond `sigmas` standard deviations.
    ///
    /// # Arguments
    ///
    /// * `sigmas` - the threshold in standard deviations; `3.0` is the common three-sigma
    ///   rule. Its magnitude is used, and one that is not a number is taken as zero, so the
    ///   mistake shows as alarms rather than as silence.
    ///
    /// # Returns
    ///
    /// A detector with an empty history.
    pub fn new(sigmas: f32) -> Self {
        Self::with_capacity(sigmas, N)
    }

    /// Creates a detector whose baseline keeps at most `capacity` readings.
    ///
    /// # Arguments
    ///
    /// * `sigmas` - the threshold in standard deviations, as for [`new`](Anomaly::new).
    /// * `capacity` - the most readings the baseline keeps. One above `N` is taken as `N`.
    ///
    /// # Returns
    ///
    /// A detector with an empty history.
    pub fn with_capacity(sigmas: f32, capacity: usize) -> Self {
        let sigmas = if sigmas.is_nan() {
            0.0
        } else if sigmas < 0.0 {
            -sigmas
        } else {
            sigmas
        };
        Self {
            window: Window::with_capacity(capacity),
            sigmas,
        }
    }

    /// Tests a reading against the recent norm, then folds it into the history.
    ///
    /// The reading is judged against the window of earlier readings, so the value being
    /// tested does not inflate its own baseline. Until at least two readings have been seen
    /// there is no spread to judge against, so nothing is flagged: from the third reading
    /// on, a reading can be. A reading that is not a finite number is always flagged and is
    /// not kept.
    ///
    /// # Arguments
    ///
    /// * `reading` - the latest reading.
    ///
    /// # Returns
    ///
    /// `true` if `reading` lies more than the configured standard deviations from the mean
    /// of the recent window.
    pub fn check(&mut self, reading: f32) -> bool {
        if !reading.is_finite() {
            return true;
        }
        let anomalous = match self.window.mean() {
            Some(mean) if self.window.len() >= 2 => {
                let variance = self.window.variance().unwrap_or(0.0);
                let deviation = reading - mean;
                deviation * deviation > self.sigmas * self.sigmas * variance
            }
            _ => false,
        };
        self.window.push(reading);
        anomalous
    }

    /// Returns the number of readings in the baseline window so far.
    pub fn len(&self) -> usize {
        self.window.len()
    }

    /// Returns `true` if no readings have been recorded yet.
    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }

    /// Returns the most readings the baseline keeps: `N`, or less when made with
    /// [`with_capacity`](Anomaly::with_capacity).
    pub fn capacity(&self) -> usize {
        self.window.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_flagged_until_there_is_a_baseline() {
        let mut watch = Anomaly::<5>::new(3.0);
        assert!(!watch.check(100.0)); // first reading: no baseline
        assert!(!watch.check(0.0)); // only one prior reading: still none
        assert_eq!(watch.len(), 2);
    }

    #[test]
    fn flags_beyond_three_sigma_but_not_within() {
        // A window of {1, -1, 1, -1} has mean 0 and population variance 1, so sigma = 1.
        let baseline = [1.0, -1.0, 1.0, -1.0];

        let mut inside = Anomaly::<4>::new(3.0);
        for reading in baseline {
            inside.check(reading);
        }
        assert!(!inside.check(2.9)); // 2.9 sigma: within

        let mut outside = Anomaly::<4>::new(3.0);
        for reading in baseline {
            outside.check(reading);
        }
        assert!(outside.check(3.1)); // 3.1 sigma: beyond
    }

    #[test]
    fn a_clear_outlier_is_flagged_after_a_noisy_baseline() {
        let mut watch = Anomaly::<6>::new(3.0);
        for reading in [50.0, 51.0, 49.0, 50.5, 49.5, 50.0] {
            watch.check(reading);
        }
        assert!(!watch.check(50.5)); // an ordinary reading
        assert!(watch.check(80.0)); // a far outlier
    }

    #[test]
    fn a_reading_that_is_not_a_number_is_flagged_and_kept_out_of_the_baseline() {
        let mut watch = Anomaly::<6>::new(3.0);
        for reading in [50.0, 51.0, 49.0, 50.5, 49.5] {
            watch.check(reading);
        }
        assert!(watch.check(f32::NAN));
        assert!(watch.check(f32::INFINITY));
        assert_eq!(watch.len(), 5);
        assert!(!watch.check(50.5));
        assert!(watch.check(80.0));
    }

    #[test]
    fn a_smaller_capacity_forgets_sooner() {
        let mut watch = Anomaly::<32>::with_capacity(3.0, 3);
        for reading in [1.0, -1.0, 1.0, -1.0, 100.0, 101.0, 100.0] {
            watch.check(reading);
        }
        assert_eq!(watch.capacity(), 3);
        assert!(!watch.check(100.5));
    }

    #[test]
    fn any_change_from_a_flat_baseline_is_flagged() {
        let mut watch = Anomaly::<4>::new(3.0);
        watch.check(5.0);
        watch.check(5.0); // the baseline has zero spread
        assert!(watch.check(6.0)); // so anything different stands out
    }
}
