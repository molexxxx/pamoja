//! Generated Python bindings for device profiles.
//!
//! These mirror the `pamoja-profile` Rust API. A profile is a named, pre-wired
//! bundle: a control policy, a publish topic, and a power schedule, so someone
//! who can put a sensor to good use does not also have to choose algorithms and
//! tuning constants by hand.
//!
//! Two things cross. A profile is the manifest, which loads from and saves to
//! JSON, so it ships as a file that a community can publish and a device can
//! read. A controller is the decision logic that manifest describes: hand it a
//! reading and it says what the output should do and whether the reading crossed
//! a threshold worth raising. The presentation a dashboard reads travels inside
//! the manifest JSON rather than as its own classes, which keeps one
//! representation of a profile across every language.
//!
//! Assembling a running node stays in Rust, where it is generic over its sensor,
//! actuator, transport, and codec. Nothing is lost: the controller holds the
//! decisions, and the caller drives their own hardware around it.

use std::sync::Mutex;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use pamoja_profile::{
    Alert, ControlSpec, Controller as CoreController, PowerSchedule, Profile as CoreProfile,
    Reaction as CoreReaction,
};

/// A profile's control policy.
///
/// Only the attributes belonging to `kind` are set; the rest are `None`.
#[gen_stub_pyclass]
#[pyclass]
pub struct ControlPolicy {
    /// Which policy this describes: `Setpoint`, `Level`, `Surge`, or `Monitor`.
    #[pyo3(get)]
    kind: String,
    /// The target reading, for a setpoint policy.
    #[pyo3(get)]
    setpoint: Option<f32>,
    /// Half the deadband width, for a setpoint policy.
    #[pyo3(get)]
    hysteresis: Option<f32>,
    /// Whether the output cools rather than heats, for a setpoint policy.
    #[pyo3(get)]
    cooling: Option<bool>,
    /// How far the reading may stray before an alert, for a setpoint policy.
    #[pyo3(get)]
    safe_band: Option<f32>,
    /// The level treated as empty, for a level policy.
    #[pyo3(get)]
    empty: Option<f32>,
    /// How many samples ahead to warn, for a level policy.
    #[pyo3(get)]
    warn_within: Option<u32>,
    /// Whether a rise rather than a fall is watched, for a surge policy.
    #[pyo3(get)]
    rising: Option<bool>,
    /// The largest safe change per sample, for a surge policy.
    #[pyo3(get)]
    limit: Option<f32>,
}

/// How often a node samples as its battery drains, in whole seconds.
#[gen_stub_pyclass]
#[pyclass]
pub struct PowerScheduleSpec {
    /// Seconds between samples at a healthy charge.
    #[pyo3(get)]
    active_secs: u64,
    /// Seconds between samples while conserving.
    #[pyo3(get)]
    saver_secs: u64,
    /// Seconds between samples when critically low.
    #[pyo3(get)]
    critical_secs: u64,
    /// Enter the saver cadence below this state of charge.
    #[pyo3(get)]
    saver_below: f32,
    /// Enter the critical cadence below this state of charge.
    #[pyo3(get)]
    critical_below: f32,
}

/// An alert a reading raised.
///
/// Only the attribute belonging to `kind` is set; the rest are `None`.
#[gen_stub_pyclass]
#[pyclass]
pub struct AlertReport {
    /// Which threshold the reading crossed: `OutOfRange`, `RunningOut`, or
    /// `ChangingFast`.
    #[pyo3(get)]
    kind: String,
    /// The offending reading, for an out-of-range alert.
    #[pyo3(get)]
    reading: Option<f32>,
    /// The estimated samples until empty, for a running-out alert.
    #[pyo3(get)]
    samples: Option<u32>,
    /// The change since the previous sample, for a changing-fast alert.
    #[pyo3(get)]
    rate: Option<f32>,
}

/// What a controller decided about one reading.
#[gen_stub_pyclass]
#[pyclass]
pub struct Reaction {
    /// The setting the output should take, or `None` when the profile observes
    /// rather than controls.
    #[pyo3(get)]
    actuator: Option<bool>,
    /// The alert the reading raised, or `None` if it crossed nothing.
    #[pyo3(get)]
    alert: Option<Py<AlertReport>>,
}

/// Flattens a control policy into the object Python sees.
fn policy_of(spec: ControlSpec) -> ControlPolicy {
    let mut policy = ControlPolicy {
        kind: "Monitor".to_owned(),
        setpoint: None,
        hysteresis: None,
        cooling: None,
        safe_band: None,
        empty: None,
        warn_within: None,
        rising: None,
        limit: None,
    };
    match spec {
        ControlSpec::Setpoint {
            setpoint,
            hysteresis,
            cooling,
            safe_band,
        } => {
            policy.kind = "Setpoint".to_owned();
            policy.setpoint = Some(setpoint);
            policy.hysteresis = Some(hysteresis);
            policy.cooling = Some(cooling);
            policy.safe_band = Some(safe_band);
        }
        ControlSpec::Level { empty, warn_within } => {
            policy.kind = "Level".to_owned();
            policy.empty = Some(empty);
            policy.warn_within = Some(warn_within);
        }
        ControlSpec::Surge { rising, limit } => {
            policy.kind = "Surge".to_owned();
            policy.rising = Some(rising);
            policy.limit = Some(limit);
        }
        ControlSpec::Monitor => {}
    }
    policy
}

/// Flattens a schedule into the object Python sees.
fn schedule_of(schedule: PowerSchedule) -> PowerScheduleSpec {
    PowerScheduleSpec {
        active_secs: schedule.active_secs,
        saver_secs: schedule.saver_secs,
        critical_secs: schedule.critical_secs,
        saver_below: schedule.saver_below,
        critical_below: schedule.critical_below,
    }
}

/// Flattens an alert into the object Python sees.
fn alert_of(alert: Alert) -> AlertReport {
    let mut report = AlertReport {
        kind: String::new(),
        reading: None,
        samples: None,
        rate: None,
    };
    match alert {
        Alert::OutOfRange { reading } => {
            report.kind = "OutOfRange".to_owned();
            report.reading = Some(reading);
        }
        Alert::RunningOut { samples } => {
            report.kind = "RunningOut".to_owned();
            report.samples = Some(samples);
        }
        Alert::ChangingFast { rate } => {
            report.kind = "ChangingFast".to_owned();
            report.rate = Some(rate);
        }
    }
    report
}

/// A named, ready-to-run node assembled from pamoja capabilities.
#[gen_stub_pyclass]
#[pyclass]
pub struct Profile {
    inner: CoreProfile,
}

#[gen_stub_pymethods]
#[pymethods]
impl Profile {
    /// A cold-chain fridge monitor, which holds 5 C and flags an excursion.
    #[staticmethod]
    fn vaccine_fridge_monitor() -> Self {
        Self {
            inner: CoreProfile::vaccine_fridge_monitor(),
        }
    }

    /// An irrigation node, which opens a valve as soil moisture falls.
    #[staticmethod]
    fn irrigation_node() -> Self {
        Self {
            inner: CoreProfile::irrigation_node(),
        }
    }

    /// A well-level monitor, which warns before a tank runs dry.
    #[staticmethod]
    fn well_level() -> Self {
        Self {
            inner: CoreProfile::well_level(),
        }
    }

    /// A flood sensor, which warns when a level rises too fast.
    #[staticmethod]
    fn flood_sensor() -> Self {
        Self {
            inner: CoreProfile::flood_sensor(),
        }
    }

    /// Loads a profile from its JSON manifest.
    ///
    /// Raises `ValueError` if the manifest is malformed.
    #[staticmethod]
    fn from_json(manifest: &str) -> PyResult<Self> {
        CoreProfile::from_json(manifest)
            .map(|inner| Self { inner })
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))
    }

    /// Serializes this profile to its JSON manifest.
    fn to_json(&self) -> PyResult<String> {
        self.inner
            .to_json()
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))
    }

    /// The profile's stable, human-readable name.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// The topic each reading is published to.
    #[getter]
    fn topic(&self) -> String {
        self.inner.topic.clone()
    }

    /// The control policy applied to each reading.
    #[getter]
    fn control(&self) -> ControlPolicy {
        policy_of(self.inner.control)
    }

    /// The sampling schedule kept as the battery drains.
    #[getter]
    fn power(&self) -> PowerScheduleSpec {
        schedule_of(self.inner.power)
    }

    /// Builds the decision logic this profile describes.
    fn controller(&self) -> Controller {
        Controller {
            inner: Mutex::new(self.inner.controller()),
        }
    }
}

/// The decision logic a profile assembles.
///
/// A controller carries state between readings, because a level estimate and a
/// rate of change both need the previous sample, so evaluate readings through
/// one controller in the order they were taken.
#[gen_stub_pyclass]
#[pyclass]
pub struct Controller {
    inner: Mutex<CoreController>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Controller {
    /// Holds a reading near a setpoint by switching an output on and off.
    #[staticmethod]
    fn setpoint(setpoint: f32, hysteresis: f32, cooling: bool, safe_band: f32) -> Self {
        Self {
            inner: Mutex::new(CoreController::setpoint(
                setpoint, hysteresis, cooling, safe_band,
            )),
        }
    }

    /// Warns before a falling level reaches empty.
    #[staticmethod]
    fn level(empty: f32, warn_within: u32) -> Self {
        Self {
            inner: Mutex::new(CoreController::level(empty, warn_within)),
        }
    }

    /// Warns when a reading changes faster than a limit.
    #[staticmethod]
    fn surge(rising: bool, limit: f32) -> Self {
        Self {
            inner: Mutex::new(CoreController::surge(rising, limit)),
        }
    }

    /// Reports readings without judging them.
    #[staticmethod]
    fn monitor() -> Self {
        Self {
            inner: Mutex::new(CoreController::monitor()),
        }
    }

    /// Decides what one reading calls for.
    fn evaluate(&self, py: Python<'_>, reading: f32) -> PyResult<Reaction> {
        let decided: CoreReaction = {
            let mut controller = self.inner.lock().map_err(|_| {
                pyo3::exceptions::PyRuntimeError::new_err("this controller is poisoned")
            })?;
            controller.evaluate(reading)
        };
        let alert = match decided.alert {
            Some(alert) => Some(Py::new(py, alert_of(alert))?),
            None => None,
        };
        Ok(Reaction {
            actuator: decided.actuator,
            alert,
        })
    }
}
