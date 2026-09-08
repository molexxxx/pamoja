//! Catching a value that moves dangerously fast.

/// Warns when a reading changes faster than a safe rate.
///
/// This is the primitive behind "warn me before it is too late": a river level
/// rising fast enough to mean a flash flood, a gas reading spiking toward an
/// explosive level, or a tank pressure collapsing. Feed it successive readings and
/// it reports the rate whenever the change since the previous sample, in the
/// direction being watched, exceeds a limit. The technique one layer down is a
/// first difference between consecutive samples, so a noisy signal pairs well with a
/// [`Smoother`](crate::Smoother) on the input to avoid false alarms.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Surge;
///
/// // A river gauge in meters, sampled each minute: alarm if it rises faster than
/// // 0.5 m per sample.
/// let mut flood = Surge::rising(0.5);
/// assert_eq!(flood.update(1.0), None); // first reading: no rate yet
/// assert_eq!(flood.update(1.25), None); // a gentle rise is fine
/// assert_eq!(flood.update(2.0), Some(0.75)); // a 0.75 m jump: a flash flood
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surge {
    limit: f32,
    rising: bool,
    last: Option<f32>,
}

impl Surge {
    /// Creates an alarm for a value rising too fast.
    ///
    /// # Arguments
    ///
    /// * `limit` - the largest safe increase per sample; its magnitude is used.
    ///
    /// # Returns
    ///
    /// An alarm awaiting its first reading.
    pub fn rising(limit: f32) -> Self {
        Self {
            limit: magnitude(limit),
            rising: true,
            last: None,
        }
    }

    /// Creates an alarm for a value falling too fast.
    ///
    /// # Arguments
    ///
    /// * `limit` - the largest safe decrease per sample; its magnitude is used.
    ///
    /// # Returns
    ///
    /// An alarm awaiting its first reading.
    pub fn falling(limit: f32) -> Self {
        Self {
            limit: magnitude(limit),
            rising: false,
            last: None,
        }
    }

    /// Records a reading and reports the rate if it changed too fast.
    ///
    /// # Arguments
    ///
    /// * `value` - the latest reading.
    ///
    /// # Returns
    ///
    /// `Some(rate)` for the change since the previous sample when it exceeds the limit
    /// in the watched direction, where `rate` is that change as a positive number;
    /// `None` if the change is within the limit, is in the other direction, or this is
    /// the first reading.
    pub fn update(&mut self, value: f32) -> Option<f32> {
        let exceeded = match self.last {
            Some(previous) => {
                let change = value - previous;
                let watched = if self.rising { change } else { -change };
                if watched > self.limit {
                    Some(watched)
                } else {
                    None
                }
            }
            None => None,
        };
        self.last = Some(value);
        exceeded
    }
}

// `f32::abs` lives in `std`, so this `no_std` crate takes the magnitude by hand.
fn magnitude(value: f32) -> f32 {
    if value < 0.0 {
        -value
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_reading_has_no_rate() {
        let mut surge = Surge::rising(1.0);
        assert_eq!(surge.update(5.0), None);
    }

    #[test]
    fn a_rapid_rise_reports_its_rate() {
        let mut surge = Surge::rising(0.5);
        surge.update(1.0);
        assert_eq!(surge.update(1.25), None); // within the limit
        assert_eq!(surge.update(2.0), Some(0.75)); // over the limit
    }

    #[test]
    fn a_rising_alarm_ignores_a_fall() {
        let mut surge = Surge::rising(0.5);
        surge.update(5.0);
        assert_eq!(surge.update(1.0), None); // a big drop is not a rise
    }

    #[test]
    fn a_falling_alarm_reports_a_rapid_drop() {
        let mut surge = Surge::falling(0.5);
        surge.update(3.0);
        assert_eq!(surge.update(2.75), None); // a small drop is fine
        assert_eq!(surge.update(1.0), Some(1.75)); // a steep drop
    }

    #[test]
    fn a_negative_limit_is_treated_as_its_magnitude() {
        let mut surge = Surge::rising(-0.5);
        surge.update(1.0);
        assert_eq!(surge.update(2.0), Some(1.0));
    }
}
