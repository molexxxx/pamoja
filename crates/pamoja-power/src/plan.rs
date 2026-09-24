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

/// The margin a charge must climb past a threshold before a plan leaves the lower mode.
pub const DEFAULT_HYSTERESIS: f32 = 0.05;

/// Maps a battery state of charge onto a [`PowerMode`] and a work interval.
///
/// As the battery drains, a node should do less: sample and transmit less often so
/// it survives the night or a cloudy week. A [`PowerPlan`] encodes that policy as
/// three intervals and two thresholds. Feed it a state of charge in `[0.0, 1.0]`
/// and it returns the mode to run in and how long to wait before the next cycle.
/// When the panel is charging it eases off by one mode, since incoming energy buys
/// back some headroom.
///
/// A charge read from a fuel gauge wanders by a percent or two from one reading to the
/// next, so a node whose charge sits at a threshold would change mode on every cycle.
/// [`next_mode`](PowerPlan::next_mode) takes the mode the node is in and applies
/// hysteresis: the node drops to a lower mode as soon as the charge falls below its
/// threshold, and climbs back only once the charge reaches the threshold plus a margin,
/// [`DEFAULT_HYSTERESIS`] unless [`with_hysteresis`](PowerPlan::with_hysteresis) sets
/// another.
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
///
/// // A charge wandering around the 50% threshold settles in saver mode...
/// let mut mode = PowerMode::Active;
/// for soc in [0.49, 0.51, 0.50, 0.53, 0.48] {
///     mode = plan.next_mode(mode, soc);
///     assert_eq!(mode, PowerMode::Saver);
/// }
/// // ...until it reaches 55%.
/// assert_eq!(plan.next_mode(mode, 0.55), PowerMode::Active);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerPlan {
    active_interval: Duration,
    saver_interval: Duration,
    critical_interval: Duration,
    saver_below: f32,
    critical_below: f32,
    hysteresis: f32,
}

impl PowerPlan {
    /// Creates a plan from its three work intervals, with default thresholds.
    ///
    /// The defaults enter [`PowerMode::Saver`] below 50% charge and
    /// [`PowerMode::Critical`] below 20%, and leave each lower mode once the charge is
    /// [`DEFAULT_HYSTERESIS`] above the threshold that brought it on.
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
            hysteresis: DEFAULT_HYSTERESIS,
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

    /// Sets how far above a threshold the charge must climb before the plan leaves the
    /// lower mode.
    ///
    /// # Arguments
    ///
    /// * `margin` - the state of charge added to each threshold on the way back up. `0.0`
    ///   turns hysteresis off, so [`next_mode`](PowerPlan::next_mode) agrees with
    ///   [`mode`](PowerPlan::mode) at every charge. A margin that is negative or not a
    ///   number is taken as `0.0`.
    ///
    /// # Returns
    ///
    /// The updated plan, for chaining.
    pub fn with_hysteresis(mut self, margin: f32) -> Self {
        self.hysteresis = if margin > 0.0 { margin } else { 0.0 };
        self
    }

    /// Returns how far above a threshold the charge must climb to leave the lower mode.
    ///
    /// # Returns
    ///
    /// The margin as a state of charge.
    pub fn hysteresis(&self) -> f32 {
        self.hysteresis
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
    /// A charge that is not a number, such as a fuel gauge that failed to answer, is
    /// taken as critical: a node that cannot tell how much it has left does the least
    /// until it can.
    ///
    /// # Arguments
    ///
    /// * `soc` - the battery state of charge in `[0.0, 1.0]`.
    ///
    /// # Returns
    ///
    /// The [`PowerMode`] the node should run in.
    pub fn mode(&self, soc: f32) -> PowerMode {
        Self::mode_at(soc, self.saver_below, self.critical_below)
    }

    /// Returns the mode a node in `current` moves to at a new state of charge.
    ///
    /// The node drops to a lower mode as soon as the charge falls below that mode's
    /// threshold, as [`mode`](PowerPlan::mode) does. It climbs to a higher mode only once
    /// the charge reaches the threshold plus the plan's
    /// [`hysteresis`](PowerPlan::hysteresis), so a charge wandering around a threshold
    /// keeps the node where it is. A charge that is not a number is taken as critical.
    ///
    /// # Arguments
    ///
    /// * `current` - the mode the node is running in.
    /// * `soc` - the battery state of charge in `[0.0, 1.0]`.
    ///
    /// # Returns
    ///
    /// The [`PowerMode`] the node should run in next.
    pub fn next_mode(&self, current: PowerMode, soc: f32) -> PowerMode {
        let target = self.mode(soc);
        if rank(target) <= rank(current) {
            return target;
        }
        let reached = Self::mode_at(
            soc,
            self.saver_below + self.hysteresis,
            self.critical_below + self.hysteresis,
        );
        if rank(reached) > rank(current) {
            reached
        } else {
            current
        }
    }

    /// Returns the mode a node in `current` moves to, easing off by one step when
    /// charging.
    ///
    /// The hysteresis of [`next_mode`](PowerPlan::next_mode) applies to the mode the
    /// node runs in, so `current` is the mode this returned last time. Easing up because
    /// the panel started charging counts as a climb, so it too waits until the charge is
    /// the margin clear of the threshold below it.
    ///
    /// # Arguments
    ///
    /// * `current` - the mode the node is running in.
    /// * `soc` - the battery state of charge in `[0.0, 1.0]`.
    /// * `charging` - whether the panel is currently delivering charge.
    ///
    /// # Returns
    ///
    /// The [`PowerMode`] the node should run in next.
    pub fn next_mode_while_charging(
        &self,
        current: PowerMode,
        soc: f32,
        charging: bool,
    ) -> PowerMode {
        if !charging {
            return self.next_mode(current, soc);
        }
        let target = eased(self.mode(soc));
        if rank(target) <= rank(current) {
            return target;
        }
        let reached = eased(Self::mode_at(
            soc,
            self.saver_below + self.hysteresis,
            self.critical_below + self.hysteresis,
        ));
        if rank(reached) > rank(current) {
            reached
        } else {
            current
        }
    }

    fn mode_at(soc: f32, saver_below: f32, critical_below: f32) -> PowerMode {
        if soc.is_nan() || soc < critical_below {
            PowerMode::Critical
        } else if soc < saver_below {
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
            eased(mode)
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

fn eased(mode: PowerMode) -> PowerMode {
    match mode {
        PowerMode::Critical => PowerMode::Saver,
        PowerMode::Saver | PowerMode::Active => PowerMode::Active,
    }
}

fn rank(mode: PowerMode) -> u8 {
    match mode {
        PowerMode::Critical => 0,
        PowerMode::Saver => 1,
        PowerMode::Active => 2,
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

    #[test]
    fn a_charge_wandering_at_a_threshold_holds_the_mode() {
        let plan = plan();
        let mut mode = PowerMode::Active;
        let mut changes = 0;
        for soc in [0.52, 0.49, 0.51, 0.495, 0.505, 0.53, 0.5, 0.54] {
            let next = plan.next_mode(mode, soc);
            changes += usize::from(next != mode);
            mode = next;
        }
        assert_eq!(mode, PowerMode::Saver);
        assert_eq!(changes, 1, "one drop, and no climb back inside the margin");
        assert_eq!(plan.next_mode(mode, 0.56), PowerMode::Active);
    }

    #[test]
    fn a_fall_takes_effect_at_the_threshold() {
        let plan = plan();
        assert_eq!(plan.next_mode(PowerMode::Active, 0.49), PowerMode::Saver);
        assert_eq!(plan.next_mode(PowerMode::Active, 0.1), PowerMode::Critical);
        assert_eq!(
            plan.next_mode(PowerMode::Saver, f32::NAN),
            PowerMode::Critical
        );
    }

    #[test]
    fn a_climb_stops_at_the_highest_mode_the_margin_allows() {
        let plan = plan();
        assert_eq!(
            plan.next_mode(PowerMode::Critical, 0.22),
            PowerMode::Critical
        );
        assert_eq!(plan.next_mode(PowerMode::Critical, 0.3), PowerMode::Saver);
        assert_eq!(plan.next_mode(PowerMode::Critical, 0.52), PowerMode::Saver);
        assert_eq!(plan.next_mode(PowerMode::Critical, 0.9), PowerMode::Active);
    }

    #[test]
    fn no_margin_follows_the_plain_mode() {
        let plan = plan().with_hysteresis(0.0);
        for soc in [0.0, 0.19, 0.2, 0.3, 0.49, 0.5, 0.9, 1.0] {
            for current in [PowerMode::Active, PowerMode::Saver, PowerMode::Critical] {
                assert_eq!(plan.next_mode(current, soc), plan.mode(soc));
            }
        }
        assert_eq!(plan.with_hysteresis(-1.0).hysteresis(), 0.0);
        assert_eq!(plan.with_hysteresis(f32::NAN).hysteresis(), 0.0);
    }

    #[test]
    fn charging_eases_the_mode_the_margin_settles_on() {
        let plan = plan();
        assert_eq!(
            plan.next_mode_while_charging(PowerMode::Saver, 0.21, true),
            PowerMode::Saver,
            "a node that fell to critical and eased to saver stays there inside the margin"
        );
        assert_eq!(
            plan.next_mode_while_charging(PowerMode::Saver, 0.26, true),
            PowerMode::Active
        );
        assert_eq!(
            plan.next_mode_while_charging(PowerMode::Active, 0.19, true),
            PowerMode::Saver
        );
        assert_eq!(
            plan.next_mode_while_charging(PowerMode::Active, 0.19, false),
            PowerMode::Critical
        );
    }

    #[test]
    fn a_charge_that_is_not_a_number_is_taken_as_critical() {
        let plan = plan();
        assert_eq!(plan.mode(f32::NAN), PowerMode::Critical);
        assert_eq!(plan.interval(f32::NAN), Duration::from_secs(3600));
        assert_eq!(
            plan.mode_while_charging(f32::NAN, true),
            PowerMode::Saver,
            "a delivering panel still buys back one mode"
        );
    }
}
