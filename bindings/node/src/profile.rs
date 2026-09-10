//! Generated Node bindings for device profiles.
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
//! language, and crosses here as a typed object so a program can read and build
//! it without composing JSON by hand.
//!
//! Assembling a running node stays in Rust, where it is generic over its sensor,
//! actuator, transport, and codec. Nothing is lost: the controller holds the
//! decisions, and the caller drives their own hardware around it.

use std::collections::{BTreeMap, HashMap};

use napi::Either;
use napi_derive::napi;
use pamoja_profile::{
    Alert as CoreAlert, ControlSpec, Controller as CoreController, ElementSpec as CoreElementSpec,
    LocalizedText, PowerSchedule as CoreSchedule, Presentation as CorePresentation,
    Profile as CoreProfile, Reaction as CoreReaction, Scope, Theme as CoreTheme, Viz as CoreViz,
};

/// Which control policy a profile applies to each reading.
#[napi(string_enum)]
pub enum ControlKind {
    /// Hold a reading near a setpoint by switching an output on and off.
    Setpoint,
    /// Watch a falling level and warn before it reaches empty.
    Level,
    /// Warn when a reading changes faster than a limit.
    Surge,
    /// Report readings only, with no output and no alerts.
    Monitor,
}

/// A profile's control policy. Only the fields belonging to `kind` are set.
#[napi(object)]
pub struct ControlPolicy {
    /// Which policy this describes.
    pub kind: ControlKind,
    /// The target reading, for a setpoint policy.
    pub setpoint: Option<f64>,
    /// Half the deadband width, for a setpoint policy.
    pub hysteresis: Option<f64>,
    /// Whether the output cools rather than heats, for a setpoint policy.
    pub cooling: Option<bool>,
    /// How far the reading may stray before an alert, for a setpoint policy.
    pub safe_band: Option<f64>,
    /// The level treated as empty, for a level policy.
    pub empty: Option<f64>,
    /// How many samples ahead to warn, for a level policy.
    pub warn_within: Option<u32>,
    /// Whether a rise rather than a fall is watched, for a surge policy.
    pub rising: Option<bool>,
    /// The largest safe change per sample, for a surge policy.
    pub limit: Option<f64>,
}

/// How often a node samples as its battery drains, in whole seconds.
#[napi(object)]
pub struct PowerScheduleSpec {
    /// Seconds between samples at a healthy charge.
    pub active_secs: f64,
    /// Seconds between samples while conserving.
    pub saver_secs: f64,
    /// Seconds between samples when critically low.
    pub critical_secs: f64,
    /// Enter the saver cadence below this state of charge.
    pub saver_below: f64,
    /// Enter the critical cadence below this state of charge.
    pub critical_below: f64,
}

/// The graphic a dashboard draws an element with, named by the instrument rather than
/// the quantity. The values are the ones a manifest carries.
#[napi(string_enum = "snake_case")]
pub enum Viz {
    /// A rolling sparkline of recent values.
    Spark,
    /// A 270-degree arch gauge, for a fraction or percentage.
    Gauge,
    /// A half-dial with a needle, for a pressure or flow reading.
    Dial,
    /// A horizontal bar with a safe-band tick, for a level or stock.
    Bar,
    /// A thermometer, for a temperature.
    Thermometer,
    /// A liquid-filled droplet, for humidity or moisture.
    Droplet,
    /// A segmented battery cell, for a state of charge or voltage.
    Battery,
    /// An anemometer, for wind speed.
    Wind,
    /// A sun whose corona grows with the reading, for illuminance.
    Sun,
    /// An acoustic waveform, for sound level or an acoustic event.
    Wave,
    /// A labeled state chip, lit when the state reads as on.
    Switch,
    /// A pipe valve, open along the flow or closed across it.
    Valve,
    /// A row of hash-chained blocks, for a tamper-evident record count.
    Chain,
    /// A neighbor-mesh topology map, for a mesh node's peers.
    Mesh,
    /// A plain numeric counter, for a node or network stat.
    Count,
}

/// A custom sensor or node stat a profile contributes to the dashboard.
#[napi(object)]
pub struct ElementSpec {
    /// The stable, language-neutral element key, such as `water_turbidity`.
    pub key: String,
    /// The canonical unit name, such as `ntu`, `ph`, or `count`.
    pub unit: String,
    /// A human-readable fallback label, shown when no localized label applies.
    pub label: String,
    /// Per-locale labels, keyed by locale tag (`en`, `sw`, ...).
    pub labels: Option<HashMap<String, String>>,
    /// The graphic this element is drawn with.
    pub viz: Viz,
    /// The safe band as `[low, high]` in the element's unit.
    pub band: Option<Vec<f64>>,
    /// Whether this is a node or network stat rather than a measurement of the world.
    pub stat: Option<bool>,
    /// The link kinds whose groups this element is offered on, such as `["mesh"]`;
    /// absent means every group.
    pub scope: Option<Vec<String>>,
    /// Whether the element's tile spans two columns.
    pub span: Option<bool>,
    /// A starting numeric value for the add-sensor dialog.
    pub value: Option<f64>,
    /// A starting discrete state code, such as `state.closed`, for a non-numeric element.
    pub state: Option<String>,
}

/// The theme tokens a profile sets on the dashboard; each is any CSS color.
#[napi(object)]
pub struct Theme {
    /// The brand and interaction accent.
    pub accent: Option<String>,
    /// The healthy status color, which also tints an in-band gauge.
    pub ok: Option<String>,
    /// The warning status color.
    pub warn: Option<String>,
    /// The alarm status color.
    pub alarm: Option<String>,
    /// The unfilled track color behind gauges and bars.
    pub track: Option<String>,
}

/// Text for one code a profile introduces: one string for every locale, or a map from
/// locale tag to text.
type MessageText = Either<String, HashMap<String, String>>;

/// How a profile presents itself on the dashboard: its custom elements, an optional
/// theme, and the words for any state or event code it introduces.
#[napi(object)]
pub struct Presentation {
    /// The custom sensors and node stats this profile contributes.
    pub elements: Vec<ElementSpec>,
    /// An optional theme that tints the dashboard.
    pub theme: Option<Theme>,
    /// Text for the codes this profile introduces, keyed by the page's message key
    /// (`state.flushing`, `event.filter_clog`): one string for every locale, or a map
    /// from locale tag to text.
    #[napi(ts_type = "Record<string, string | Record<string, string>>")]
    pub messages: Option<HashMap<String, MessageText>>,
}

/// Which threshold a reading crossed.
#[napi(string_enum)]
pub enum AlertKind {
    /// A controlled reading drifted outside its safe band.
    OutOfRange,
    /// A falling level will reach empty within a few more samples.
    RunningOut,
    /// A reading is changing faster than its safe rate.
    ChangingFast,
}

/// An alert a reading raised. Only the field belonging to `kind` is set.
#[napi(object)]
pub struct AlertReport {
    /// Which threshold the reading crossed.
    pub kind: AlertKind,
    /// The offending reading, for an out-of-range alert.
    pub reading: Option<f64>,
    /// The estimated samples until empty, for a running-out alert.
    pub samples: Option<u32>,
    /// The change since the previous sample, for a changing-fast alert.
    pub rate: Option<f64>,
}

/// What a controller decided about one reading.
#[napi(object)]
pub struct Reaction {
    /// The setting the output should take, or `null` when the profile observes
    /// rather than controls.
    pub actuator: Option<bool>,
    /// The alert the reading raised, or `null` if it crossed nothing.
    pub alert: Option<AlertReport>,
}

/// Flattens a control policy into the object JavaScript sees.
fn policy_of(spec: ControlSpec) -> ControlPolicy {
    let mut policy = ControlPolicy {
        kind: ControlKind::Monitor,
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
            policy.kind = ControlKind::Setpoint;
            policy.setpoint = Some(f64::from(setpoint));
            policy.hysteresis = Some(f64::from(hysteresis));
            policy.cooling = Some(cooling);
            policy.safe_band = Some(f64::from(safe_band));
        }
        ControlSpec::Level { empty, warn_within } => {
            policy.kind = ControlKind::Level;
            policy.empty = Some(f64::from(empty));
            policy.warn_within = Some(warn_within);
        }
        ControlSpec::Surge { rising, limit } => {
            policy.kind = ControlKind::Surge;
            policy.rising = Some(rising);
            policy.limit = Some(f64::from(limit));
        }
        ControlSpec::Monitor => {}
    }
    policy
}

/// Flattens a schedule into the object JavaScript sees.
fn schedule_of(schedule: CoreSchedule) -> PowerScheduleSpec {
    PowerScheduleSpec {
        active_secs: schedule.active_secs as f64,
        saver_secs: schedule.saver_secs as f64,
        critical_secs: schedule.critical_secs as f64,
        saver_below: f64::from(schedule.saver_below),
        critical_below: f64::from(schedule.critical_below),
    }
}

fn viz_of(viz: CoreViz) -> Viz {
    match viz {
        CoreViz::Spark => Viz::Spark,
        CoreViz::Gauge => Viz::Gauge,
        CoreViz::Dial => Viz::Dial,
        CoreViz::Bar => Viz::Bar,
        CoreViz::Thermometer => Viz::Thermometer,
        CoreViz::Droplet => Viz::Droplet,
        CoreViz::Battery => Viz::Battery,
        CoreViz::Wind => Viz::Wind,
        CoreViz::Sun => Viz::Sun,
        CoreViz::Wave => Viz::Wave,
        CoreViz::Switch => Viz::Switch,
        CoreViz::Valve => Viz::Valve,
        CoreViz::Chain => Viz::Chain,
        CoreViz::Mesh => Viz::Mesh,
        CoreViz::Count => Viz::Count,
    }
}

fn core_viz(viz: Viz) -> CoreViz {
    match viz {
        Viz::Spark => CoreViz::Spark,
        Viz::Gauge => CoreViz::Gauge,
        Viz::Dial => CoreViz::Dial,
        Viz::Bar => CoreViz::Bar,
        Viz::Thermometer => CoreViz::Thermometer,
        Viz::Droplet => CoreViz::Droplet,
        Viz::Battery => CoreViz::Battery,
        Viz::Wind => CoreViz::Wind,
        Viz::Sun => CoreViz::Sun,
        Viz::Wave => CoreViz::Wave,
        Viz::Switch => CoreViz::Switch,
        Viz::Valve => CoreViz::Valve,
        Viz::Chain => CoreViz::Chain,
        Viz::Mesh => CoreViz::Mesh,
        Viz::Count => CoreViz::Count,
    }
}

/// Lays a presentation out as the object JavaScript sees.
fn presentation_of(presentation: &CorePresentation) -> Presentation {
    Presentation {
        elements: presentation
            .elements
            .iter()
            .map(|element| ElementSpec {
                key: element.key.clone(),
                unit: element.unit.clone(),
                label: element.label.clone(),
                labels: element
                    .labels
                    .as_ref()
                    .map(|labels| labels.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                viz: viz_of(element.viz),
                band: element
                    .band
                    .map(|[low, high]| vec![f64::from(low), f64::from(high)]),
                stat: Some(element.stat),
                scope: match &element.scope {
                    Scope::Always => None,
                    Scope::Links(links) => Some(links.clone()),
                },
                span: Some(element.span),
                value: element.value.map(f64::from),
                state: element.state.clone(),
            })
            .collect(),
        theme: presentation.theme.as_ref().map(|theme| Theme {
            accent: theme.accent.clone(),
            ok: theme.ok.clone(),
            warn: theme.warn.clone(),
            alarm: theme.alarm.clone(),
            track: theme.track.clone(),
        }),
        messages: (!presentation.messages.is_empty()).then(|| {
            presentation
                .messages
                .iter()
                .map(|(code, text)| {
                    let text = match text {
                        LocalizedText::Plain(text) => Either::A(text.clone()),
                        LocalizedText::PerLocale(map) => {
                            Either::B(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        }
                    };
                    (code.clone(), text)
                })
                .collect()
        }),
    }
}

/// Reads a presentation back from the object JavaScript built, checking what the type
/// system cannot: a band is two numbers.
fn core_presentation(presentation: Presentation) -> napi::Result<CorePresentation> {
    let mut out = CorePresentation::new();
    for element in presentation.elements {
        let mut spec = CoreElementSpec::new(
            element.key.clone(),
            element.unit,
            element.label,
            core_viz(element.viz),
        );
        if let Some(labels) = element.labels {
            spec.labels = Some(labels.into_iter().collect::<BTreeMap<_, _>>());
        }
        if let Some(band) = element.band {
            let [low, high] = band[..] else {
                return Err(napi::Error::from_reason(format!(
                    "the band of `{}` must be [low, high]",
                    element.key
                )));
            };
            spec = spec.with_band(low as f32, high as f32);
        }
        spec.stat = element.stat.unwrap_or(false);
        spec.scope = match element.scope {
            Some(links) => Scope::Links(links),
            None => Scope::Always,
        };
        spec.span = element.span.unwrap_or(false);
        spec.value = element.value.map(|value| value as f32);
        spec.state = element.state;
        out = out.with_element(spec);
    }
    if let Some(theme) = presentation.theme {
        out = out.with_theme(CoreTheme {
            accent: theme.accent,
            ok: theme.ok,
            warn: theme.warn,
            alarm: theme.alarm,
            track: theme.track,
        });
    }
    for (code, text) in presentation.messages.unwrap_or_default() {
        let text = match text {
            Either::A(text) => LocalizedText::Plain(text),
            Either::B(map) => LocalizedText::PerLocale(map.into_iter().collect()),
        };
        out = out.with_message(code, text);
    }
    Ok(out)
}

/// Flattens a reaction into the object JavaScript sees.
fn reaction_of(reaction: CoreReaction) -> Reaction {
    Reaction {
        actuator: reaction.actuator,
        alert: reaction.alert.map(|alert| match alert {
            CoreAlert::OutOfRange { reading } => AlertReport {
                kind: AlertKind::OutOfRange,
                reading: Some(f64::from(reading)),
                samples: None,
                rate: None,
            },
            CoreAlert::RunningOut { samples } => AlertReport {
                kind: AlertKind::RunningOut,
                reading: None,
                samples: Some(samples),
                rate: None,
            },
            CoreAlert::ChangingFast { rate } => AlertReport {
                kind: AlertKind::ChangingFast,
                reading: None,
                samples: None,
                rate: Some(f64::from(rate)),
            },
        }),
    }
}

/// A named, ready-to-run node assembled from pamoja capabilities.
#[napi]
pub struct Profile {
    inner: CoreProfile,
}

#[napi]
impl Profile {
    /// A cold-chain fridge monitor, which holds 5 C and flags an excursion.
    #[napi(factory)]
    pub fn vaccine_fridge_monitor() -> Self {
        Self {
            inner: CoreProfile::vaccine_fridge_monitor(),
        }
    }

    /// An irrigation node, which opens a valve as soil moisture falls.
    #[napi(factory)]
    pub fn irrigation_node() -> Self {
        Self {
            inner: CoreProfile::irrigation_node(),
        }
    }

    /// A well-level monitor, which warns before a tank runs dry.
    #[napi(factory)]
    pub fn well_level() -> Self {
        Self {
            inner: CoreProfile::well_level(),
        }
    }

    /// A flood sensor, which warns when a level rises too fast.
    #[napi(factory)]
    pub fn flood_sensor() -> Self {
        Self {
            inner: CoreProfile::flood_sensor(),
        }
    }

    /// Loads a profile from its JSON manifest.
    ///
    /// Throws if the manifest is malformed.
    #[napi(factory)]
    pub fn from_json(manifest: String) -> napi::Result<Self> {
        CoreProfile::from_json(&manifest)
            .map(|inner| Self { inner })
            .map_err(to_napi)
    }

    /// Serializes this profile to its JSON manifest.
    #[napi]
    pub fn to_json(&self) -> napi::Result<String> {
        self.inner.to_json().map_err(to_napi)
    }

    /// The profile's stable, human-readable name.
    #[napi(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// The topic each reading is published to.
    #[napi(getter)]
    pub fn topic(&self) -> String {
        self.inner.topic.clone()
    }

    /// What the profile is for, in the words its manifest carries, or `null`.
    #[napi(getter)]
    pub fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    /// How the profile presents itself on the dashboard, or `null` when it declares
    /// nothing beyond the built-in set.
    #[napi(getter)]
    pub fn presentation(&self) -> Option<Presentation> {
        self.inner.presentation.as_ref().map(presentation_of)
    }

    /// A copy of this profile carrying a description of what it is for.
    #[napi]
    pub fn with_description(&self, description: String) -> Profile {
        Profile {
            inner: self.inner.clone().with_description(description),
        }
    }

    /// A copy of this profile carrying a dashboard presentation.
    ///
    /// Throws if a band is not two numbers.
    #[napi]
    pub fn with_presentation(&self, presentation: Presentation) -> napi::Result<Profile> {
        Ok(Profile {
            inner: self
                .inner
                .clone()
                .with_presentation(core_presentation(presentation)?),
        })
    }

    /// The control policy applied to each reading.
    #[napi(getter)]
    pub fn control(&self) -> ControlPolicy {
        policy_of(self.inner.control)
    }

    /// The sampling schedule kept as the battery drains.
    #[napi(getter)]
    pub fn power(&self) -> PowerScheduleSpec {
        schedule_of(self.inner.power)
    }

    /// Builds the decision logic this profile describes.
    #[napi]
    pub fn controller(&self) -> Controller {
        Controller {
            inner: self.inner.controller(),
        }
    }
}

/// The decision logic a profile assembles.
///
/// A controller carries state between readings, because a level estimate and a
/// rate of change both need the previous sample, so evaluate readings through
/// one controller in the order they were taken.
#[napi]
pub struct Controller {
    inner: CoreController,
}

#[napi]
impl Controller {
    /// Holds a reading near a setpoint by switching an output on and off.
    ///
    /// @param setpoint - the target reading.
    /// @param hysteresis - half the deadband width, which stops the output
    ///   chattering at the threshold.
    /// @param cooling - whether the output cools rather than heats.
    /// @param safe_band - how far the reading may stray before an alert.
    #[napi(factory)]
    pub fn setpoint(setpoint: f64, hysteresis: f64, cooling: bool, safe_band: f64) -> Self {
        Self {
            inner: CoreController::setpoint(
                setpoint as f32,
                hysteresis as f32,
                cooling,
                safe_band as f32,
            ),
        }
    }

    /// Warns before a falling level reaches empty.
    ///
    /// @param empty - the level treated as empty.
    /// @param warn_within - warn once empty is this many samples away.
    #[napi(factory)]
    pub fn level(empty: f64, warn_within: u32) -> Self {
        Self {
            inner: CoreController::level(empty as f32, warn_within),
        }
    }

    /// Warns when a reading changes faster than a limit.
    ///
    /// @param rising - watch a rapid rise rather than a rapid fall.
    /// @param limit - the largest safe change per sample.
    #[napi(factory)]
    pub fn surge(rising: bool, limit: f64) -> Self {
        Self {
            inner: CoreController::surge(rising, limit as f32),
        }
    }

    /// Reports readings without judging them.
    #[napi(factory)]
    pub fn monitor() -> Self {
        Self {
            inner: CoreController::monitor(),
        }
    }

    /// Decides what one reading calls for.
    #[napi]
    pub fn evaluate(&mut self, reading: f64) -> Reaction {
        reaction_of(self.inner.evaluate(reading as f32))
    }
}

/// Maps a core error onto the one JavaScript sees.
fn to_napi(error: pamoja_core::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
