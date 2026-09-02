//! Generated Node bindings for power-aware scheduling.
//!
//! These mirror the `pamoja-power` Rust API: the split between working and
//! sleeping that a duty cycle describes, and the plan that stretches a work
//! interval as a battery falls.
//!
//! Durations cross as microseconds in a JavaScript number, which is exact well
//! past any interval a node would wait.

use core::time::Duration;

use napi_derive::napi;
use pamoja_power::{DutyCycle as CoreDutyCycle, PowerMode as CoreMode, PowerPlan as CorePlan};

/// What a node should be doing at the current state of charge.
#[napi(string_enum)]
pub enum PowerMode {
    /// Full duty, because the charge is healthy.
    Active,
    /// Reduced duty, to conserve charge.
    Saver,
    /// Minimum duty, to stay alive as long as possible.
    Critical,
}

/// The split between the time a node works and the time it sleeps.
#[napi]
pub struct DutyCycle {
    inner: CoreDutyCycle,
}

#[napi]
impl DutyCycle {
    /// Creates a duty cycle from the time awake and the time asleep, in
    /// microseconds.
    #[napi(constructor)]
    pub fn new(active_us: f64, sleep_us: f64) -> Self {
        Self {
            inner: CoreDutyCycle::new(duration(active_us), duration(sleep_us)),
        }
    }

    /// Creates a duty cycle that spends `fraction` of `periodUs` awake.
    #[napi(factory)]
    pub fn from_fraction(period_us: f64, fraction: f64) -> Self {
        Self {
            inner: CoreDutyCycle::from_fraction(duration(period_us), fraction as f32),
        }
    }

    /// How long the node stays awake each period, in microseconds.
    #[napi(getter)]
    pub fn active_us(&self) -> f64 {
        micros(self.inner.active())
    }

    /// How long it sleeps each period, in microseconds.
    #[napi(getter)]
    pub fn sleep_us(&self) -> f64 {
        micros(self.inner.sleep())
    }

    /// The whole period, awake plus asleep, in microseconds.
    #[napi(getter)]
    pub fn period_us(&self) -> f64 {
        micros(self.inner.period())
    }

    /// The share of the period spent awake, from 0 through 1.
    #[napi(getter)]
    pub fn fraction(&self) -> f64 {
        f64::from(self.inner.fraction())
    }
}

/// The work intervals a node uses in each mode, and where the modes change.
#[napi]
pub struct PowerPlan {
    inner: CorePlan,
}

#[napi]
impl PowerPlan {
    /// Creates a plan from its three work intervals in microseconds, entering
    /// saver mode below 50% charge and critical below 20%.
    #[napi(constructor)]
    pub fn new(active_us: f64, saver_us: f64, critical_us: f64) -> Self {
        Self {
            inner: CorePlan::new(
                duration(active_us),
                duration(saver_us),
                duration(critical_us),
            ),
        }
    }

    /// Returns a copy of this plan with the state-of-charge thresholds moved.
    #[napi]
    pub fn with_thresholds(&self, saver_below: f64, critical_below: f64) -> Self {
        Self {
            inner: self
                .inner
                .thresholds(saver_below as f32, critical_below as f32),
        }
    }

    /// The charge below which the plan enters saver mode.
    #[napi(getter)]
    pub fn saver_below(&self) -> f64 {
        f64::from(self.inner.saver_below())
    }

    /// The charge below which the plan enters critical mode.
    #[napi(getter)]
    pub fn critical_below(&self) -> f64 {
        f64::from(self.inner.critical_below())
    }

    /// Returns the mode this plan calls for at a state of charge.
    #[napi]
    pub fn mode(&self, soc: f64) -> PowerMode {
        mode(self.inner.mode(soc as f32))
    }

    /// Returns the mode, eased one step toward full duty while charging.
    #[napi]
    pub fn mode_while_charging(&self, soc: f64, charging: bool) -> PowerMode {
        mode(self.inner.mode_while_charging(soc as f32, charging))
    }

    /// Returns the work interval for a mode, in microseconds.
    #[napi]
    pub fn interval_for_us(&self, mode: PowerMode) -> f64 {
        micros(self.inner.interval_for(core_mode(mode)))
    }

    /// Returns the work interval at a state of charge, in microseconds.
    #[napi]
    pub fn interval_us(&self, soc: f64) -> f64 {
        micros(self.inner.interval(soc as f32))
    }
}

/// Reads a microsecond count as a duration.
fn duration(micros: f64) -> Duration {
    Duration::from_micros(micros.max(0.0) as u64)
}

/// Narrows a duration to the microseconds a JavaScript number carries.
fn micros(duration: Duration) -> f64 {
    duration.as_micros() as f64
}

/// Maps a core power mode onto the value that crosses to JavaScript.
fn mode(mode: CoreMode) -> PowerMode {
    match mode {
        CoreMode::Active => PowerMode::Active,
        CoreMode::Saver => PowerMode::Saver,
        CoreMode::Critical => PowerMode::Critical,
    }
}

/// Maps a JavaScript power mode back onto the core one.
fn core_mode(mode: PowerMode) -> CoreMode {
    match mode {
        PowerMode::Active => CoreMode::Active,
        PowerMode::Saver => CoreMode::Saver,
        PowerMode::Critical => CoreMode::Critical,
    }
}
