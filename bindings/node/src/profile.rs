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
//! the manifest JSON rather than as its own classes, which keeps one
//! representation of a profile across every language.
//!
//! Assembling a running node stays in Rust, where it is generic over its sensor,
//! actuator, transport, and codec. Nothing is lost: the controller holds the
//! decisions, and the caller drives their own hardware around it.

use napi_derive::napi;
use pamoja_profile::{
    Alert as CoreAlert, ControlSpec, Controller as CoreController, PowerSchedule as CoreSchedule,
    Profile as CoreProfile, Reaction as CoreReaction,
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
