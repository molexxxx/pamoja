//! Duty cycling: trading wakefulness for battery life.

use core::time::Duration;

/// A repeating wake/sleep schedule.
///
/// Duty cycling is the simplest way to make a battery or solar node last: stay
/// awake just long enough to do the work, then sleep through the rest of the
/// period. The duty fraction - the share of each period spent awake - is a good
/// first proxy for average power draw, so roughly halving it halves the energy the
/// cycle costs.
///
/// # Examples
///
/// ```
/// use core::time::Duration;
/// use pamoja_power::DutyCycle;
///
/// // Wake for one second every minute.
/// let cycle = DutyCycle::new(Duration::from_secs(1), Duration::from_secs(59));
/// assert_eq!(cycle.period(), Duration::from_secs(60));
/// assert!((cycle.fraction() - 1.0 / 60.0).abs() < 1e-6);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DutyCycle {
    active: Duration,
    sleep: Duration,
}

impl DutyCycle {
    /// Creates a duty cycle from its awake and asleep durations.
    ///
    /// # Arguments
    ///
    /// * `active` - how long to stay awake each period.
    /// * `sleep` - how long to sleep each period.
    ///
    /// # Returns
    ///
    /// The duty cycle.
    pub fn new(active: Duration, sleep: Duration) -> Self {
        Self { active, sleep }
    }

    /// Creates a duty cycle from a period and the fraction of it to stay awake.
    ///
    /// # Arguments
    ///
    /// * `period` - the full wake-plus-sleep period.
    /// * `fraction` - the share of the period to stay awake, clamped to
    ///   `[0.0, 1.0]`. A fraction that is not a number keeps the node asleep for the
    ///   whole period, since a budget worked out from a failed reading is no budget.
    ///
    /// # Returns
    ///
    /// A duty cycle whose awake time is `fraction` of `period`.
    pub fn from_fraction(period: Duration, fraction: f32) -> Self {
        let active = if fraction >= 1.0 {
            period
        } else if fraction > 0.0 {
            period.mul_f64(f64::from(fraction))
        } else {
            Duration::ZERO
        };
        Self {
            active,
            sleep: period - active,
        }
    }

    /// Returns the awake portion of each period.
    pub fn active(&self) -> Duration {
        self.active
    }

    /// Returns the asleep portion of each period.
    pub fn sleep(&self) -> Duration {
        self.sleep
    }

    /// Returns the full period, awake plus asleep, held at [`Duration::MAX`] rather
    /// than overflowing.
    pub fn period(&self) -> Duration {
        self.active.saturating_add(self.sleep)
    }

    /// Returns the share of each period spent awake, in `[0.0, 1.0]`.
    ///
    /// # Returns
    ///
    /// The duty fraction, or `0.0` for a zero-length period.
    pub fn fraction(&self) -> f32 {
        let period = self.period();
        if period.is_zero() {
            0.0
        } else {
            self.active.as_secs_f32() / period.as_secs_f32()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_is_awake_plus_asleep() {
        let cycle = DutyCycle::new(Duration::from_secs(2), Duration::from_secs(8));
        assert_eq!(cycle.period(), Duration::from_secs(10));
    }

    #[test]
    fn fraction_is_the_awake_share() {
        let cycle = DutyCycle::new(Duration::from_secs(2), Duration::from_secs(8));
        assert!((cycle.fraction() - 0.2).abs() < 1e-6);
    }

    #[test]
    fn a_zero_period_has_zero_fraction() {
        let cycle = DutyCycle::new(Duration::ZERO, Duration::ZERO);
        assert_eq!(cycle.fraction(), 0.0);
    }

    #[test]
    fn from_fraction_splits_the_period() {
        let cycle = DutyCycle::from_fraction(Duration::from_secs(10), 0.25);
        assert!((cycle.active().as_secs_f32() - 2.5).abs() < 1e-3);
        assert!((cycle.fraction() - 0.25).abs() < 1e-6);
    }

    #[test]
    fn from_fraction_clamps_out_of_range_values() {
        let all_awake = DutyCycle::from_fraction(Duration::from_secs(10), 5.0);
        assert_eq!(all_awake.active(), Duration::from_secs(10));
        assert_eq!(all_awake.sleep(), Duration::ZERO);

        let all_asleep = DutyCycle::from_fraction(Duration::from_secs(10), -1.0);
        assert_eq!(all_asleep.active(), Duration::ZERO);
    }

    #[test]
    fn a_fraction_that_is_not_a_number_keeps_the_node_asleep() {
        let cycle = DutyCycle::from_fraction(Duration::from_secs(10), f32::NAN);
        assert_eq!(cycle.active(), Duration::ZERO);
        assert_eq!(cycle.sleep(), Duration::from_secs(10));
    }

    #[test]
    fn the_longest_period_splits_without_overflowing() {
        let whole = DutyCycle::from_fraction(Duration::MAX, 1.0);
        assert_eq!(whole.active(), Duration::MAX);
        let nearly = DutyCycle::from_fraction(Duration::MAX, 1.0 - f32::EPSILON);
        assert!(nearly.active() < Duration::MAX);
    }

    #[test]
    fn the_longest_halves_report_their_period_without_overflowing() {
        let cycle = DutyCycle::new(Duration::MAX, Duration::from_secs(1));
        assert_eq!(cycle.period(), Duration::MAX);
        assert!((cycle.fraction() - 1.0).abs() < 1e-6);
    }
}
