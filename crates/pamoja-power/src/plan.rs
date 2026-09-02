//! An energy-aware governor that adapts the work cadence to the battery.

use core::time::Duration;

/// How hard a node should work, chosen from its battery state of charge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerMode {
    /// Healthy charge: run at the normal cadence.
    Active,
    /// Low charge: stretch the cadence to conserve.
    Saver,
    /// Critically low charge: do the bare minimum to survive.
    Critical,
}

/// Maps a battery state of charge onto a [`PowerMode`] and a work interval.
///
/// As the battery drains, a node should do less: sample and transmit less often so
/// it survives the night or a cloudy week. A [`PowerPlan`] encodes that policy as
/// three intervals and two thresholds. Feed it a state of charge in `[0.0, 1.0]`
/// and it returns the mode to run in and how long to wait before the next cycle.
/// When the panel is charging it eases off by one mode, since incoming energy buys
/// back some headroom.
///
/// # Examples
///
/// ```
/// use core::time::Duration;
/// use pamoja_power::{PowerMode, PowerPlan};
///
/// let plan = PowerPlan::new(
///     Duration::from_secs(60),
///     Duration::from_secs(600),
///     Duration::from_secs(3600),
/// );
///
/// // Low battery means the saver cadence...
/// assert_eq!(plan.mode(0.3), PowerMode::Saver);
/// // ...unless the panel is charging, which buys back the active cadence.
/// assert_eq!(plan.mode_while_charging(0.3, true), PowerMode::Active);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerPlan {
    active_interval: Duration,
    saver_interval: Duration,
    critical_interval: Duration,
    saver_below: f32,
    critical_below: f32,
}

impl PowerPlan {
    /// Creates a plan from its three work intervals, with default thresholds.
    ///
    /// The defaults enter [`PowerMode::Saver`] below 50% charge and
    /// [`PowerMode::Critical`] below 20%.
    ///
    /// # Arguments
    ///
    /// * `active` - the interval at a healthy charge.
    /// * `saver` - the longer interval used to conserve, normally larger than
    ///   `active`.
    /// * `critical` - the longest interval, used when charge is critically low.
    ///
    /// # Returns
    ///
    /// The power plan.
    pub fn new(active: Duration, saver: Duration, critical: Duration) -> Self {
        Self {
            active_interval: active,
            saver_interval: saver,
            critical_interval: critical,
            saver_below: 0.5,
            critical_below: 0.2,
        }
    }

    /// Sets the state-of-charge thresholds for entering each lower mode.
    ///
    /// # Arguments
    ///
    /// * `saver_below` - enter [`PowerMode::Saver`] when charge is below this.
    /// * `critical_below` - enter [`PowerMode::Critical`] when charge is below this,
    ///   normally lower than `saver_below`.
    ///
    /// # Returns
    ///
    /// The updated plan, for chaining.
    pub fn thresholds(mut self, saver_below: f32, critical_below: f32) -> Self {
        self.saver_below = saver_below;
        self.critical_below = critical_below;
        self
    }

    /// Returns the charge below which the plan enters [`PowerMode::Saver`].
    ///
    /// # Returns
    ///
    /// The saver threshold as a state of charge in `[0.0, 1.0]`.
    pub fn saver_below(&self) -> f32 {
        self.saver_below
    }

    /// Returns the charge below which the plan enters [`PowerMode::Critical`].
    ///
    /// # Returns
    ///
    /// The critical threshold as a state of charge in `[0.0, 1.0]`.
    pub fn critical_below(&self) -> f32 {
        self.critical_below
    }

    /// Returns the mode for the given state of charge.
    ///
    /// # Arguments
    ///
    /// * `soc` - the battery state of charge in `[0.0, 1.0]`.
    ///
    /// # Returns
    ///
    /// The [`PowerMode`] the node should run in.
    pub fn mode(&self, soc: f32) -> PowerMode {
        if soc < self.critical_below {
            PowerMode::Critical
        } else if soc < self.saver_below {
            PowerMode::Saver
        } else {
            PowerMode::Active
        }
    }

    /// Returns the mode for the given charge, easing off by one step when charging.
    ///
    /// # Arguments
    ///
    /// * `soc` - the battery state of charge in `[0.0, 1.0]`.
    /// * `charging` - whether the panel is currently delivering charge.
    ///
    /// # Returns
    ///
    /// The [`PowerMode`], promoted one step toward [`PowerMode::Active`] while
    /// `charging` is `true`.
    pub fn mode_while_charging(&self, soc: f32, charging: bool) -> PowerMode {
        let mode = self.mode(soc);
        if charging {
            match mode {
                PowerMode::Critical => PowerMode::Saver,
                PowerMode::Saver | PowerMode::Active => PowerMode::Active,
            }
        } else {
            mode
        }
    }

    /// Returns the work interval for a given mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - the mode to look up.
    ///
    /// # Returns
    ///
    /// The interval to wait before the next work cycle in that mode.
    pub fn interval_for(&self, mode: PowerMode) -> Duration {
        match mode {
            PowerMode::Active => self.active_interval,
            PowerMode::Saver => self.saver_interval,
            PowerMode::Critical => self.critical_interval,
        }
    }

    /// Returns the work interval for the given state of charge.
    ///
    /// # Arguments
    ///
    /// * `soc` - the battery state of charge in `[0.0, 1.0]`.
    ///
    /// # Returns
    ///
    /// The interval to wait before the next work cycle.
    pub fn interval(&self, soc: f32) -> Duration {
        self.interval_for(self.mode(soc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> PowerPlan {
        PowerPlan::new(
            Duration::from_secs(60),
            Duration::from_secs(600),
            Duration::from_secs(3600),
        )
    }

    #[test]
    fn mode_steps_down_as_charge_falls() {
        let plan = plan();
        assert_eq!(plan.mode(0.9), PowerMode::Active);
        assert_eq!(plan.mode(0.4), PowerMode::Saver);
        assert_eq!(plan.mode(0.1), PowerMode::Critical);
    }

    #[test]
    fn thresholds_are_the_lower_bound_of_each_mode() {
        let plan = plan();
        // Exactly at a threshold stays in the higher mode.
        assert_eq!(plan.mode(0.5), PowerMode::Active);
        assert_eq!(plan.mode(0.2), PowerMode::Saver);
    }

    #[test]
    fn interval_follows_the_mode() {
        let plan = plan();
        assert_eq!(plan.interval(0.9), Duration::from_secs(60));
        assert_eq!(plan.interval(0.4), Duration::from_secs(600));
        assert_eq!(plan.interval(0.1), Duration::from_secs(3600));
    }

    #[test]
    fn charging_eases_off_by_one_mode() {
        let plan = plan();
        assert_eq!(plan.mode_while_charging(0.1, true), PowerMode::Saver);
        assert_eq!(plan.mode_while_charging(0.4, true), PowerMode::Active);
        assert_eq!(plan.mode_while_charging(0.9, true), PowerMode::Active);
        // Not charging is unchanged.
        assert_eq!(plan.mode_while_charging(0.1, false), PowerMode::Critical);
    }

    #[test]
    fn custom_thresholds_apply() {
        let plan = plan().thresholds(0.7, 0.3);
        assert_eq!(plan.mode(0.65), PowerMode::Saver);
        assert_eq!(plan.mode(0.25), PowerMode::Critical);
    }
}
