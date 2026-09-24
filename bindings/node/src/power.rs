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
    pub fn new(active_us: f64, sleep_us: f64) -> napi::Result<Self> {
        Ok(Self {
            inner: CoreDutyCycle::new(
                duration(active_us, "activeUs")?,
                duration(sleep_us, "sleepUs")?,
            ),
        })
    }

    /// Creates a duty cycle that spends `fraction` of `periodUs` awake.
    ///
    /// The fraction is clamped to 0 through 1, and one that is not a number keeps the node
    /// asleep for the whole period. The time awake is rounded down to a whole microsecond
    /// and the rest of the period is spent asleep, so the two always add up to the period.
    #[napi(factory)]
    pub fn from_fraction(period_us: f64, fraction: f64) -> napi::Result<Self> {
        let period = duration(period_us, "periodUs")?;
        Ok(Self {
            inner: whole(CoreDutyCycle::from_fraction(period, fraction as f32)),
        })
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

impl PowerPlan {
    /// Wraps a plan another module assembled, such as a profile's.
    pub(crate) fn of(inner: CorePlan) -> Self {
        Self { inner }
    }
}

#[napi]
impl PowerPlan {
    /// Creates a plan from its three work intervals in microseconds, entering
    /// saver mode below 50% charge and critical below 20%, and leaving each lower mode
    /// once the charge is five points above the threshold that brought it on.
    #[napi(constructor)]
    pub fn new(active_us: f64, saver_us: f64, critical_us: f64) -> napi::Result<Self> {
        Ok(Self {
            inner: CorePlan::new(
                duration(active_us, "activeUs")?,
                duration(saver_us, "saverUs")?,
                duration(critical_us, "criticalUs")?,
            ),
        })
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

    /// Returns a copy of this plan with the hysteresis margin moved: how far above a
    /// threshold the charge must climb before the plan leaves the lower mode. `0` switches
    /// at the thresholds themselves; a margin below zero or not a number is taken as `0`.
    #[napi]
    pub fn with_hysteresis(&self, margin: f64) -> Self {
        Self {
            inner: self.inner.with_hysteresis(margin as f32),
        }
    }

    /// How far above a threshold the charge must climb before the plan leaves the lower
    /// mode.
    #[napi(getter)]
    pub fn hysteresis(&self) -> f64 {
        f64::from(self.inner.hysteresis())
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
    ///
    /// A charge that is not a number, such as a fuel gauge that failed to answer, is taken
    /// as critical.
    #[napi]
    pub fn mode(&self, soc: f64) -> PowerMode {
        mode(self.inner.mode(soc as f32))
    }

    /// Returns the mode, eased one step toward full duty while charging.
    #[napi]
    pub fn mode_while_charging(&self, soc: f64, charging: bool) -> PowerMode {
        mode(self.inner.mode_while_charging(soc as f32, charging))
    }

    /// Returns the mode a node in `current` moves to at a new charge. It drops to a lower
    /// mode as soon as the charge falls below that mode's threshold, and climbs back only
    /// once the charge reaches the threshold plus the hysteresis margin, so a charge
    /// wandering around a threshold keeps the node where it is.
    #[napi]
    pub fn next_mode(&self, current: PowerMode, soc: f64) -> PowerMode {
        mode(self.inner.next_mode(core_mode(current), soc as f32))
    }

    /// Returns the mode a node in `current` moves to, eased one step toward full duty while
    /// charging. `current` is the mode this returned last time.
    #[napi]
    pub fn next_mode_while_charging(
        &self,
        current: PowerMode,
        soc: f64,
        charging: bool,
    ) -> PowerMode {
        mode(
            self.inner
                .next_mode_while_charging(core_mode(current), soc as f32, charging),
        )
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

/// Reads a microsecond count as a duration, refusing one that is not a whole,
/// non-negative number a JavaScript number holds exactly.
fn duration(micros: f64, name: &str) -> napi::Result<Duration> {
    const SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    if micros.fract() != 0.0 || !(0.0..=SAFE_INTEGER).contains(&micros) {
        return Err(napi::Error::from_reason(format!(
            "{name} must be a whole number of microseconds, not {micros}"
        )));
    }
    Ok(Duration::from_micros(micros as u64))
}

/// Rounds the awake half of a split down to a whole microsecond and gives the rest of the
/// period to sleep, so the halves that cross add up to the period.
fn whole(split: CoreDutyCycle) -> CoreDutyCycle {
    let active = Duration::from_micros(split.active().as_micros() as u64);
    CoreDutyCycle::new(active, split.period() - active)
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
