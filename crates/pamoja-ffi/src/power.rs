//! The C ABI for power-aware scheduling.
//!
//! These functions wrap [`pamoja_power`] for callers that reach the SDK through
//! the flat C boundary: the split between working and sleeping that a duty cycle
//! describes, and the plan that stretches a work interval as a battery falls.
//!
//! Everything here is arithmetic over scalars, so both types cross by value and
//! nothing allocates. Durations cross as microseconds, which covers intervals
//! from a radio burst to weeks of deep sleep in a 64-bit unsigned integer.

use core::time::Duration;

use pamoja_power::{DutyCycle, PowerMode, PowerPlan};

/// The split between the time a node works and the time it sleeps.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaDutyCycle {
    /// How long the node stays awake each period, in microseconds.
    pub active_us: u64,
    /// How long it sleeps each period, in microseconds.
    pub sleep_us: u64,
}

/// What a node should be doing at the current state of charge.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaPowerMode {
    /// Full duty, because the charge is healthy.
    Active = 0,
    /// Reduced duty, to conserve charge.
    Saver = 1,
    /// Minimum duty, to stay alive as long as possible.
    Critical = 2,
}

/// The work intervals a node uses in each mode, and where the modes change.
///
/// Build one with [`pamoja_power_plan_new`], which applies the default
/// thresholds, then move them with [`pamoja_power_plan_with_thresholds`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaPowerPlan {
    /// The interval between work at a healthy charge, in microseconds.
    pub active_us: u64,
    /// The interval used to conserve charge, in microseconds.
    pub saver_us: u64,
    /// The interval used at a critically low charge, in microseconds.
    pub critical_us: u64,
    /// Enter [`PamojaPowerMode::Saver`] below this state of charge.
    pub saver_below: f32,
    /// Enter [`PamojaPowerMode::Critical`] below this state of charge.
    pub critical_below: f32,
}

/// Creates a duty cycle from the time awake and the time asleep.
///
/// # Arguments
///
/// * `active_us` - how long the node works each period, in microseconds.
/// * `sleep_us` - how long it sleeps each period, in microseconds.
///
/// # Returns
///
/// The duty cycle.
#[no_mangle]
pub extern "C" fn pamoja_duty_cycle_new(active_us: u64, sleep_us: u64) -> PamojaDutyCycle {
    let duty = DutyCycle::new(
        Duration::from_micros(active_us),
        Duration::from_micros(sleep_us),
    );
    PamojaDutyCycle {
        active_us: micros(duty.active()),
        sleep_us: micros(duty.sleep()),
    }
}

/// Creates a duty cycle that spends a fraction of each period awake.
///
/// # Arguments
///
/// * `period_us` - the whole period, in microseconds.
/// * `fraction` - the share of the period spent awake, clamped to 0.0 through 1.0.
///
/// # Returns
///
/// The duty cycle.
#[no_mangle]
pub extern "C" fn pamoja_duty_cycle_from_fraction(
    period_us: u64,
    fraction: f32,
) -> PamojaDutyCycle {
    let duty = DutyCycle::from_fraction(Duration::from_micros(period_us), fraction);
    PamojaDutyCycle {
        active_us: micros(duty.active()),
        sleep_us: micros(duty.sleep()),
    }
}

/// Returns the whole period of a duty cycle, awake plus asleep.
///
/// # Arguments
///
/// * `duty` - the duty cycle.
///
/// # Returns
///
/// The period in microseconds.
#[no_mangle]
pub extern "C" fn pamoja_duty_cycle_period_us(duty: PamojaDutyCycle) -> u64 {
    micros(cycle(duty).period())
}

/// Returns the share of a period a duty cycle spends awake.
///
/// # Arguments
///
/// * `duty` - the duty cycle.
///
/// # Returns
///
/// The fraction from 0.0 through 1.0, or `0.0` if the period is zero.
#[no_mangle]
pub extern "C" fn pamoja_duty_cycle_fraction(duty: PamojaDutyCycle) -> f32 {
    cycle(duty).fraction()
}

/// Creates a power plan from its three work intervals, with default thresholds.
///
/// The defaults enter [`PamojaPowerMode::Saver`] below 50% charge and
/// [`PamojaPowerMode::Critical`] below 20%.
///
/// # Arguments
///
/// * `active_us` - the interval at a healthy charge, in microseconds.
/// * `saver_us` - the longer interval used to conserve, in microseconds.
/// * `critical_us` - the longest interval, in microseconds.
///
/// # Returns
///
/// The power plan.
#[no_mangle]
pub extern "C" fn pamoja_power_plan_new(
    active_us: u64,
    saver_us: u64,
    critical_us: u64,
) -> PamojaPowerPlan {
    PamojaPowerPlan {
        active_us,
        saver_us,
        critical_us,
        saver_below: 0.5,
        critical_below: 0.2,
    }
}

/// Returns a plan with the state-of-charge thresholds moved.
///
/// # Arguments
///
/// * `plan` - the plan to adjust.
/// * `saver_below` - enter [`PamojaPowerMode::Saver`] below this charge.
/// * `critical_below` - enter [`PamojaPowerMode::Critical`] below this charge.
///
/// # Returns
///
/// The adjusted plan.
#[no_mangle]
pub extern "C" fn pamoja_power_plan_with_thresholds(
    plan: PamojaPowerPlan,
    saver_below: f32,
    critical_below: f32,
) -> PamojaPowerPlan {
    PamojaPowerPlan {
        saver_below,
        critical_below,
        ..plan
    }
}

/// Returns the mode a plan calls for at a state of charge.
///
/// # Arguments
///
/// * `plan` - the power plan.
/// * `soc` - the battery state of charge, from 0.0 through 1.0.
///
/// # Returns
///
/// The mode the node should run in.
#[no_mangle]
pub extern "C" fn pamoja_power_plan_mode(plan: PamojaPowerPlan, soc: f32) -> PamojaPowerMode {
    mode(rust_plan(plan).mode(soc))
}

/// Returns the mode a plan calls for, easing off one step while charging.
///
/// A node taking charge is heading the right way, so it moves one step toward
/// full duty rather than holding at what the charge alone would call for.
///
/// # Arguments
///
/// * `plan` - the power plan.
/// * `soc` - the battery state of charge, from 0.0 through 1.0.
/// * `charging` - `1` if the node is charging, `0` if it is not.
///
/// # Returns
///
/// The mode the node should run in.
#[no_mangle]
pub extern "C" fn pamoja_power_plan_mode_while_charging(
    plan: PamojaPowerPlan,
    soc: f32,
    charging: u8,
) -> PamojaPowerMode {
    mode(rust_plan(plan).mode_while_charging(soc, charging != 0))
}

/// Returns the work interval a plan uses in a mode.
///
/// # Arguments
///
/// * `plan` - the power plan.
/// * `mode` - the mode to look up.
///
/// # Returns
///
/// The interval in microseconds.
#[no_mangle]
pub extern "C" fn pamoja_power_plan_interval_for_us(
    plan: PamojaPowerPlan,
    mode: PamojaPowerMode,
) -> u64 {
    micros(rust_plan(plan).interval_for(rust_mode(mode)))
}

/// Returns the work interval a plan calls for at a state of charge.
///
/// # Arguments
///
/// * `plan` - the power plan.
/// * `soc` - the battery state of charge, from 0.0 through 1.0.
///
/// # Returns
///
/// The interval in microseconds.
#[no_mangle]
pub extern "C" fn pamoja_power_plan_interval_us(plan: PamojaPowerPlan, soc: f32) -> u64 {
    micros(rust_plan(plan).interval(soc))
}

/// Rebuilds the Rust duty cycle from the fields that crossed the boundary.
fn cycle(duty: PamojaDutyCycle) -> DutyCycle {
    DutyCycle::new(
        Duration::from_micros(duty.active_us),
        Duration::from_micros(duty.sleep_us),
    )
}

/// Rebuilds the Rust power plan from the fields that crossed the boundary.
fn rust_plan(plan: PamojaPowerPlan) -> PowerPlan {
    PowerPlan::new(
        Duration::from_micros(plan.active_us),
        Duration::from_micros(plan.saver_us),
        Duration::from_micros(plan.critical_us),
    )
    .thresholds(plan.saver_below, plan.critical_below)
}

/// Narrows a duration to the microseconds the boundary carries.
fn micros(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

/// Maps a Rust power mode onto the value that crosses the boundary.
fn mode(mode: PowerMode) -> PamojaPowerMode {
    match mode {
        PowerMode::Active => PamojaPowerMode::Active,
        PowerMode::Saver => PamojaPowerMode::Saver,
        PowerMode::Critical => PamojaPowerMode::Critical,
    }
}

/// Maps a boundary power mode back onto the Rust one.
fn rust_mode(mode: PamojaPowerMode) -> PowerMode {
    match mode {
        PamojaPowerMode::Active => PowerMode::Active,
        PamojaPowerMode::Saver => PowerMode::Saver,
        PamojaPowerMode::Critical => PowerMode::Critical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fraction_splits_the_period() {
        let duty = pamoja_duty_cycle_from_fraction(1_000_000, 0.25);
        assert_eq!(duty.active_us, 250_000);
        assert_eq!(duty.sleep_us, 750_000);
        assert_eq!(pamoja_duty_cycle_period_us(duty), 1_000_000);
        assert!((pamoja_duty_cycle_fraction(duty) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn a_falling_charge_stretches_the_interval() {
        let plan = pamoja_power_plan_new(60_000_000, 300_000_000, 3_600_000_000);

        assert_eq!(pamoja_power_plan_mode(plan, 0.9), PamojaPowerMode::Active);
        assert_eq!(pamoja_power_plan_mode(plan, 0.3), PamojaPowerMode::Saver);
        assert_eq!(pamoja_power_plan_mode(plan, 0.1), PamojaPowerMode::Critical);
        assert_eq!(pamoja_power_plan_interval_us(plan, 0.1), 3_600_000_000);
    }

    #[test]
    fn charging_eases_a_low_node_up_one_step() {
        let plan = pamoja_power_plan_new(60_000_000, 300_000_000, 3_600_000_000);

        assert_eq!(
            pamoja_power_plan_mode_while_charging(plan, 0.1, 1),
            PamojaPowerMode::Saver
        );
        assert_eq!(
            pamoja_power_plan_mode_while_charging(plan, 0.3, 1),
            PamojaPowerMode::Active
        );
        assert_eq!(
            pamoja_power_plan_mode_while_charging(plan, 0.1, 0),
            PamojaPowerMode::Critical
        );
    }

    #[test]
    fn moved_thresholds_change_where_the_modes_meet() {
        let plan = pamoja_power_plan_with_thresholds(
            pamoja_power_plan_new(60_000_000, 300_000_000, 3_600_000_000),
            0.8,
            0.4,
        );

        assert_eq!(pamoja_power_plan_mode(plan, 0.7), PamojaPowerMode::Saver);
        assert_eq!(pamoja_power_plan_mode(plan, 0.3), PamojaPowerMode::Critical);
    }
}
