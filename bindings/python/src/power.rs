//! Generated Python bindings for power-aware scheduling.
//!
//! These mirror the `pamoja-power` Rust API: the split between working and
//! sleeping that a duty cycle describes, and the plan that stretches a work
//! interval as a battery falls.
//!
//! Durations cross as microseconds, and a mode crosses as its name so a caller
//! reads it without unpacking an enum.

use core::time::Duration;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use pamoja_power::{DutyCycle as CoreDutyCycle, PowerMode, PowerPlan as CorePlan};

/// The split between the time a node works and the time it sleeps.
#[gen_stub_pyclass]
#[pyclass]
pub struct DutyCycle {
    inner: CoreDutyCycle,
}

#[gen_stub_pymethods]
#[pymethods]
impl DutyCycle {
    /// Creates a duty cycle from the time awake and the time asleep, in
    /// microseconds.
    #[new]
    fn new(active_us: u64, sleep_us: u64) -> Self {
        DutyCycle {
            inner: CoreDutyCycle::new(
                Duration::from_micros(active_us),
                Duration::from_micros(sleep_us),
            ),
        }
    }

    /// Creates a duty cycle that spends `fraction` of `period_us` awake.
    #[staticmethod]
    fn from_fraction(period_us: u64, fraction: f32) -> Self {
        DutyCycle {
            inner: CoreDutyCycle::from_fraction(Duration::from_micros(period_us), fraction),
        }
    }

    /// How long the node stays awake each period, in microseconds.
    #[getter]
    fn active_us(&self) -> u64 {
        micros(self.inner.active())
    }

    /// How long it sleeps each period, in microseconds.
    #[getter]
    fn sleep_us(&self) -> u64 {
        micros(self.inner.sleep())
    }

    /// The whole period, awake plus asleep, in microseconds.
    #[getter]
    fn period_us(&self) -> u64 {
        micros(self.inner.period())
    }

    /// The share of the period spent awake, from 0 through 1.
    #[getter]
    fn fraction(&self) -> f32 {
        self.inner.fraction()
    }
}

/// The work intervals a node uses in each mode, and where the modes change.
#[gen_stub_pyclass]
#[pyclass]
pub struct PowerPlan {
    inner: CorePlan,
}

#[gen_stub_pymethods]
#[pymethods]
impl PowerPlan {
    /// Creates a plan from its three work intervals in microseconds, entering
    /// saver mode below 50% charge and critical below 20%.
    #[new]
    fn new(active_us: u64, saver_us: u64, critical_us: u64) -> Self {
        PowerPlan {
            inner: CorePlan::new(
                Duration::from_micros(active_us),
                Duration::from_micros(saver_us),
                Duration::from_micros(critical_us),
            ),
        }
    }

    /// Returns a copy of this plan with the state-of-charge thresholds moved.
    fn with_thresholds(&self, saver_below: f32, critical_below: f32) -> Self {
        PowerPlan {
            inner: self.inner.thresholds(saver_below, critical_below),
        }
    }

    /// The charge below which the plan enters saver mode.
    #[getter]
    fn saver_below(&self) -> f32 {
        self.inner.saver_below()
    }

    /// The charge below which the plan enters critical mode.
    #[getter]
    fn critical_below(&self) -> f32 {
        self.inner.critical_below()
    }

    /// Returns the mode this plan calls for at a state of charge, by name.
    fn mode(&self, soc: f32) -> String {
        name(self.inner.mode(soc))
    }

    /// Returns the mode, eased one step toward full duty while charging.
    fn mode_while_charging(&self, soc: f32, charging: bool) -> String {
        name(self.inner.mode_while_charging(soc, charging))
    }

    /// Returns the work interval for a named mode, in microseconds.
    fn interval_for_us(&self, mode: &str) -> PyResult<u64> {
        Ok(micros(self.inner.interval_for(core_mode(mode)?)))
    }

    /// Returns the work interval at a state of charge, in microseconds.
    fn interval_us(&self, soc: f32) -> u64 {
        micros(self.inner.interval(soc))
    }
}

/// Narrows a duration to the microseconds the boundary carries.
fn micros(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

/// Names a power mode for Python.
fn name(mode: PowerMode) -> String {
    match mode {
        PowerMode::Active => "Active",
        PowerMode::Saver => "Saver",
        PowerMode::Critical => "Critical",
    }
    .to_owned()
}

/// Reads a mode back from its name, refusing one that is not a mode.
fn core_mode(mode: &str) -> PyResult<PowerMode> {
    match mode {
        "Active" => Ok(PowerMode::Active),
        "Saver" => Ok(PowerMode::Saver),
        "Critical" => Ok(PowerMode::Critical),
        other => Err(crate::PamojaError::new_err(format!(
            "unknown power mode {other}"
        ))),
    }
}
