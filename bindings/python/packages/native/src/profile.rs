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
//! the manifest JSON, which keeps one representation of a profile across every
//! language, and crosses here as typed classes so a program can read and build it
//! without composing JSON by hand.
//!
//! Assembling a running node stays in Rust, where it is generic over its sensor,
//! actuator, transport, and codec. Nothing is lost: the controller holds the
//! decisions, and the caller drives their own hardware around it.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use pamoja_profile::{
    Alert, ControlSpec, Controller as CoreController, ElementSpec as CoreElementSpec,
    LocalizedText, PowerSchedule, Presentation as CorePresentation, Profile as CoreProfile,
    Reaction as CoreReaction, Scope, Theme as CoreTheme, Viz,
};

/// The graphics a dashboard draws with, by the name a manifest carries.
const GRAPHICS: [(&str, Viz); 15] = [
    ("spark", Viz::Spark),
    ("gauge", Viz::Gauge),
    ("dial", Viz::Dial),
    ("bar", Viz::Bar),
    ("thermometer", Viz::Thermometer),
    ("droplet", Viz::Droplet),
    ("battery", Viz::Battery),
    ("wind", Viz::Wind),
    ("sun", Viz::Sun),
    ("wave", Viz::Wave),
    ("switch", Viz::Switch),
    ("valve", Viz::Valve),
    ("chain", Viz::Chain),
    ("mesh", Viz::Mesh),
    ("count", Viz::Count),
];

fn viz_named(name: &str) -> PyResult<Viz> {
    GRAPHICS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, viz)| *viz)
        .ok_or_else(|| {
            let names: Vec<&str> = GRAPHICS.iter().map(|(name, _)| *name).collect();
            pyo3::exceptions::PyValueError::new_err(format!(
                "`{name}` is not a graphic the dashboard draws; the choices are {}",
                names.join(", ")
            ))
        })
}

fn viz_name(viz: Viz) -> String {
    GRAPHICS
        .iter()
        .find(|(_, known)| *known == viz)
        .map(|(name, _)| (*name).to_owned())
        .unwrap_or_default()
}

/// Text a profile supplies for one code: one string for every locale, or a map from
/// locale tag to text.
#[derive(FromPyObject)]
enum MessageText {
    /// One text shown in every locale.
    Plain(String),
    /// Per-locale text, keyed by locale tag.
    PerLocale(HashMap<String, String>),
}

pyo3_stub_gen::impl_stub_type!(MessageText = String | HashMap<String, String>);

/// A custom sensor or node stat a profile contributes to the dashboard.
///
/// The graphic is named as a manifest names it (`droplet`, `gauge`, ...); the
/// :class:`Viz` enum in the facade lists the choices. A band is `(low, high)` in the
/// element's unit, and `scope` names the link kinds whose groups the element is offered
/// on, or `None` for every group.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct ElementSpec {
    /// The stable, language-neutral element key, such as `water_turbidity`.
    #[pyo3(get)]
    key: String,
    /// The canonical unit name, such as `ntu`, `ph`, or `count`.
    #[pyo3(get)]
    unit: String,
    /// A human-readable fallback label, shown when no localized label applies.
    #[pyo3(get)]
    label: String,
    /// The graphic this element is drawn with, by its manifest name.
    #[pyo3(get)]
    viz: String,
    /// Per-locale labels, keyed by locale tag (`en`, `sw`, ...).
    #[pyo3(get)]
    labels: Option<HashMap<String, String>>,
    /// The safe band `(low, high)` in the element's unit.
    #[pyo3(get)]
    band: Option<(f32, f32)>,
    /// Whether this is a node or network stat rather than a measurement of the world.
    #[pyo3(get)]
    stat: bool,
    /// The link kinds whose groups this element is offered on; `None` means every group.
    #[pyo3(get)]
    scope: Option<Vec<String>>,
    /// Whether the element's tile spans two columns.
    #[pyo3(get)]
    span: bool,
    /// A starting numeric value for the add-sensor dialog.
    #[pyo3(get)]
    value: Option<f32>,
    /// A starting discrete state code, such as `state.closed`, for a non-numeric element.
    #[pyo3(get)]
    state: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ElementSpec {
    /// Declares an element drawn with the named graphic.
    ///
    /// Raises `ValueError` if `viz` is not a graphic the dashboard draws.
    #[new]
    #[pyo3(signature = (key, unit, label, viz, *, labels = None, band = None, stat = false, scope = None, span = false, value = None, state = None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        key: String,
        unit: String,
        label: String,
        viz: String,
        labels: Option<HashMap<String, String>>,
        band: Option<(f32, f32)>,
        stat: bool,
        scope: Option<Vec<String>>,
        span: bool,
        value: Option<f32>,
        state: Option<String>,
    ) -> PyResult<Self> {
        viz_named(&viz)?;
        Ok(Self {
            key,
            unit,
            label,
            viz,
            labels,
            band,
            stat,
            scope,
            span,
            value,
            state,
        })
    }
}

impl ElementSpec {
    fn from_core(element: &CoreElementSpec) -> Self {
        Self {
            key: element.key.clone(),
            unit: element.unit.clone(),
            label: element.label.clone(),
            viz: viz_name(element.viz),
            labels: element
                .labels
                .as_ref()
                .map(|labels| labels.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            band: element.band.map(|[low, high]| (low, high)),
            stat: element.stat,
            scope: match &element.scope {
                Scope::Always => None,
                Scope::Links(links) => Some(links.clone()),
            },
            span: element.span,
            value: element.value,
            state: element.state.clone(),
        }
    }

    fn to_core(&self) -> PyResult<CoreElementSpec> {
        let mut spec = CoreElementSpec::new(
            self.key.clone(),
            self.unit.clone(),
            self.label.clone(),
            viz_named(&self.viz)?,
        );
        spec.labels = self
            .labels
            .as_ref()
            .map(|labels| labels.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        if let Some((low, high)) = self.band {
            spec = spec.with_band(low, high);
        }
        spec.stat = self.stat;
        spec.scope = match &self.scope {
            Some(links) => Scope::Links(links.clone()),
            None => Scope::Always,
        };
        spec.span = self.span;
        spec.value = self.value;
        spec.state = self.state.clone();
        Ok(spec)
    }
}

/// The theme tokens a profile sets on the dashboard; each is any CSS color.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct Theme {
    /// The brand and interaction accent.
    #[pyo3(get)]
    accent: Option<String>,
    /// The healthy status color, which also tints an in-band gauge.
    #[pyo3(get)]
    ok: Option<String>,
    /// The warning status color.
    #[pyo3(get)]
    warn: Option<String>,
    /// The alarm status color.
    #[pyo3(get)]
    alarm: Option<String>,
    /// The unfilled track color behind gauges and bars.
    #[pyo3(get)]
    track: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Theme {
    /// Builds a theme from the tokens given; the rest keep the dashboard's own colors.
    #[new]
    #[pyo3(signature = (*, accent = None, ok = None, warn = None, alarm = None, track = None))]
    fn new(
        accent: Option<String>,
        ok: Option<String>,
        warn: Option<String>,
        alarm: Option<String>,
        track: Option<String>,
    ) -> Self {
        Self {
            accent,
            ok,
            warn,
            alarm,
            track,
        }
    }
}

/// How a profile presents itself on the dashboard: its custom elements, an optional
/// theme, and the words for any state or event code it introduces.
#[gen_stub_pyclass]
#[pyclass]
pub struct Presentation {
    /// The custom sensors and node stats this profile contributes.
    #[pyo3(get)]
    elements: Vec<ElementSpec>,
    /// An optional theme that tints the dashboard.
    #[pyo3(get)]
    theme: Option<Theme>,
    messages: BTreeMap<String, LocalizedText>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Presentation {
    /// Builds a presentation from its elements, an optional theme, and the messages for
    /// the codes it introduces, each one string for every locale or a `dict` from locale
    /// tag to text.
    #[new]
    #[pyo3(signature = (elements, *, theme = None, messages = None))]
    fn new(
        elements: Vec<ElementSpec>,
        theme: Option<Theme>,
        messages: Option<HashMap<String, MessageText>>,
    ) -> Self {
        Self {
            elements,
            theme,
            messages: messages
                .unwrap_or_default()
                .into_iter()
                .map(|(code, text)| {
                    let text = match text {
                        MessageText::Plain(text) => LocalizedText::Plain(text),
                        MessageText::PerLocale(map) => {
                            LocalizedText::PerLocale(map.into_iter().collect())
                        }
                    };
                    (code, text)
                })
                .collect(),
        }
    }

    /// The text for each code this profile introduces, keyed by the page's message key
    /// (`state.flushing`, `event.filter_clog`): a `str` for every locale, or a `dict`
    /// from locale tag to text.
    #[getter]
    fn messages<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for (code, text) in &self.messages {
            match text {
                LocalizedText::Plain(text) => out.set_item(code, text)?,
                LocalizedText::PerLocale(map) => {
                    let per_locale = PyDict::new(py);
                    for (locale, text) in map {
                        per_locale.set_item(locale, text)?;
                    }
                    out.set_item(code, per_locale)?;
                }
            }
        }
        Ok(out)
    }
}

impl Presentation {
    fn from_core(presentation: &CorePresentation) -> Self {
        Self {
            elements: presentation
                .elements
                .iter()
                .map(ElementSpec::from_core)
                .collect(),
            theme: presentation.theme.as_ref().map(|theme| Theme {
                accent: theme.accent.clone(),
                ok: theme.ok.clone(),
                warn: theme.warn.clone(),
                alarm: theme.alarm.clone(),
                track: theme.track.clone(),
            }),
            messages: presentation.messages.clone(),
        }
    }

    fn to_core(&self) -> PyResult<CorePresentation> {
        let mut out = CorePresentation::new();
        for element in &self.elements {
            out = out.with_element(element.to_core()?);
        }
        if let Some(theme) = &self.theme {
            out = out.with_theme(CoreTheme {
                accent: theme.accent.clone(),
                ok: theme.ok.clone(),
                warn: theme.warn.clone(),
                alarm: theme.alarm.clone(),
                track: theme.track.clone(),
            });
        }
        for (code, text) in &self.messages {
            out = out.with_message(code.clone(), text.clone());
        }
        Ok(out)
    }
}

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

    /// What the profile is for, in the words its manifest carries, or `None`.
    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    /// How the profile presents itself on the dashboard, or `None` when it declares
    /// nothing beyond the built-in set.
    #[getter]
    fn presentation(&self) -> Option<Presentation> {
        self.inner
            .presentation
            .as_ref()
            .map(Presentation::from_core)
    }

    /// A copy of this profile carrying a description of what it is for.
    fn with_description(&self, description: String) -> Self {
        Self {
            inner: self.inner.clone().with_description(description),
        }
    }

    /// A copy of this profile carrying a dashboard presentation.
    ///
    /// Raises `ValueError` if an element names a graphic the dashboard does not draw.
    fn with_presentation(&self, presentation: &Presentation) -> PyResult<Self> {
        Ok(Self {
            inner: self
                .inner
                .clone()
                .with_presentation(presentation.to_core()?),
        })
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
