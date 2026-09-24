//! The profile manifest and its named, ready-to-run presets.
//!
//! A profile is data: a [`Profile`] serializes to and from a manifest a community
//! can write by hand, store in a file, and share. The presets here are convenience
//! constructors for the same data, not a closed set - any manifest that names a
//! [`ControlSpec`] and a [`PowerSchedule`] is a valid profile.

use core::time::Duration;
use std::collections::BTreeMap;

use pamoja_power::PowerPlan;
use serde::de::{self, Deserializer};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

use crate::format;
use crate::presentation::refuse;
use crate::{Controller, ElementSpec, LocalizedText, Param, Params, Presentation, Viz};

/// What a profile reads: the quantity its control decides on, and the unit its numbers
/// are in.
///
/// A manifest names it so a reader knows what to wire before opening the numbers, and so
/// a runner can pick the right reading from a part that measures several things, such as
/// the temperature a BME280 reports beside humidity and pressure. In a manifest it is the
/// `reads` object:
///
/// ```json
/// { "quantity": "temperature", "unit": "celsius" }
/// ```
///
/// Both are lowercase words joined by underscores. The setpoint, bands, and limits of the
/// profile's control are in the unit named here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reads {
    /// The quantity, such as `"temperature"`, `"relative_humidity"`, or `"water_level"`.
    pub quantity: String,
    /// The unit the profile's numbers are in, such as `"celsius"`, `"percent"`, or
    /// `"meter"`.
    pub unit: String,
}

impl Reads {
    /// Names what a profile reads.
    ///
    /// # Arguments
    ///
    /// * `quantity` - the quantity, such as `"temperature"`.
    /// * `unit` - the unit the profile's numbers are in, such as `"celsius"`.
    ///
    /// # Returns
    ///
    /// The declaration.
    pub fn new(quantity: impl Into<String>, unit: impl Into<String>) -> Self {
        Self {
            quantity: quantity.into(),
            unit: unit.into(),
        }
    }
}

/// How a profile turns each reading into control output and alerts.
///
/// This is the policy half of a profile's manifest: the tunable rule a community can
/// publish and share, with no code to write. [`Profile::controller`] assembles it
/// into a live [`Controller`]. In a manifest it is tagged by `kind`, with the kind's
/// parameters beside it:
///
/// ```json
/// { "kind": "setpoint", "setpoint": 5.0, "hysteresis": 0.5, "cooling": true, "safe_band": 3.0 }
/// ```
///
/// A `kind` the library does not know is a [`Custom`](ControlSpec::Custom) policy: it
/// loads with every other field kept as its [`Params`], and a
/// [`PolicyRegistry`](crate::PolicyRegistry) resolves it to the code that decides it,
/// so a manifest for a policy of your own reads exactly like one for a setpoint:
///
/// ```json
/// { "kind": "frost_guard", "warn_below": 2.0 }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum ControlSpec {
    /// Hold a reading near `setpoint` by switching an output on and off.
    Setpoint {
        /// The target reading, such as 5 C for a vaccine fridge.
        setpoint: f32,
        /// Half the deadband width around the setpoint, which stops the output
        /// chattering at the threshold.
        hysteresis: f32,
        /// Whether the output cools (switches on above the band) or heats (switches
        /// on below it). An irrigation valve that adds water is a "heater".
        cooling: bool,
        /// How far the reading may stray from the setpoint before an
        /// [`Alert::OutOfRange`](crate::Alert::OutOfRange) fires.
        safe_band: f32,
    },
    /// Watch a falling level and warn before it reaches `empty`.
    Level {
        /// The level treated as empty, such as a dry tank.
        empty: f32,
        /// Warn once the level is estimated to reach `empty` within this many more
        /// samples.
        warn_within: u32,
    },
    /// Warn when a reading changes faster than `limit` per sample.
    Surge {
        /// Watch a rapid rise (`true`) or a rapid fall (`false`).
        rising: bool,
        /// The largest safe change per sample.
        limit: f32,
    },
    /// Report readings only, with no control output and no alerts.
    Monitor,
    /// A policy the library does not ship, named by the manifest and decided by code a
    /// [`PolicyRegistry`](crate::PolicyRegistry) resolves the kind to.
    Custom {
        /// The kind as the manifest names it, such as `"frost_guard"`.
        kind: String,
        /// Every other field the manifest carried beside the kind.
        params: Params,
    },
}

/// The fields each built-in kind takes, as a manifest names them.
const SETPOINT_FIELDS: &[&str] = &["kind", "setpoint", "hysteresis", "cooling", "safe_band"];
const LEVEL_FIELDS: &[&str] = &["kind", "empty", "warn_within"];
const SURGE_FIELDS: &[&str] = &["kind", "rising", "limit"];
const MONITOR_FIELDS: &[&str] = &["kind"];

impl ControlSpec {
    /// The kinds the library ships, as a manifest names them.
    pub const BUILT_IN: [&'static str; 4] = ["setpoint", "level", "surge", "monitor"];

    /// Returns the kind as a manifest names it.
    ///
    /// # Returns
    ///
    /// `"setpoint"`, `"level"`, `"surge"`, `"monitor"`, or a custom kind's own name.
    pub fn kind(&self) -> &str {
        match self {
            ControlSpec::Setpoint { .. } => "setpoint",
            ControlSpec::Level { .. } => "level",
            ControlSpec::Surge { .. } => "surge",
            ControlSpec::Monitor => "monitor",
            ControlSpec::Custom { kind, .. } => kind,
        }
    }

    /// Creates a policy of a kind the library does not ship, checked so it survives a
    /// trip through its manifest.
    ///
    /// # Arguments
    ///
    /// * `kind` - the kind as the manifest will name it, such as `"frost_guard"`.
    /// * `params` - every other field the policy needs.
    ///
    /// # Returns
    ///
    /// The custom policy.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) if `kind` is empty or names a
    /// built-in kind, which a manifest would read back as that kind, or if `params`
    /// holds a field named `kind`, which the manifest keeps for the kind itself.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::{ControlSpec, Params};
    ///
    /// let guard = ControlSpec::custom("frost_guard", Params::new().with("warn_below", 2.0))?;
    /// assert_eq!(guard.kind(), "frost_guard");
    /// assert!(ControlSpec::custom("setpoint", Params::new()).is_err());
    /// # Ok::<(), pamoja_core::Error>(())
    /// ```
    pub fn custom(kind: impl Into<String>, params: Params) -> pamoja_core::Result<Self> {
        let kind = kind.into();
        if kind.is_empty() {
            return Err(pamoja_core::Error::Codec(
                "a custom control needs a kind to be named by".to_owned(),
            ));
        }
        if matches!(kind.as_str(), "setpoint" | "level" | "surge" | "monitor") {
            return Err(pamoja_core::Error::Codec(format!(
                "{kind} is a built-in control kind, so it takes its own fields rather than parameters"
            )));
        }
        if params.get("kind").is_some() {
            return Err(pamoja_core::Error::Codec(
                "a custom control cannot have a parameter named kind, which its manifest keeps for the kind itself"
                    .to_owned(),
            ));
        }
        Ok(ControlSpec::Custom { kind, params })
    }
}

impl Serialize for ControlSpec {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ControlSpec::Setpoint {
                setpoint,
                hysteresis,
                cooling,
                safe_band,
            } => {
                let mut map = serializer.serialize_map(Some(5))?;
                map.serialize_entry("kind", "setpoint")?;
                map.serialize_entry("setpoint", setpoint)?;
                map.serialize_entry("hysteresis", hysteresis)?;
                map.serialize_entry("cooling", cooling)?;
                map.serialize_entry("safe_band", safe_band)?;
                map.end()
            }
            ControlSpec::Level { empty, warn_within } => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("kind", "level")?;
                map.serialize_entry("empty", empty)?;
                map.serialize_entry("warn_within", warn_within)?;
                map.end()
            }
            ControlSpec::Surge { rising, limit } => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("kind", "surge")?;
                map.serialize_entry("rising", rising)?;
                map.serialize_entry("limit", limit)?;
                map.end()
            }
            ControlSpec::Monitor => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("kind", "monitor")?;
                map.end()
            }
            ControlSpec::Custom { kind, params } => {
                let mut map = serializer.serialize_map(Some(1 + params.len()))?;
                map.serialize_entry("kind", kind)?;
                for (name, value) in params.iter() {
                    map.serialize_entry(name, value)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ControlSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut fields: BTreeMap<String, Param> = BTreeMap::deserialize(deserializer)?;
        let kind = match fields.remove("kind") {
            Some(Param::Text(kind)) => kind,
            Some(_) => return Err(de::Error::custom("`kind` must be a string")),
            None => return Err(de::Error::missing_field("kind")),
        };
        let number = |name: &'static str| -> Result<f32, D::Error> {
            match fields.get(name) {
                Some(Param::Number(value)) => Ok(*value as f32),
                Some(_) => Err(de::Error::custom(format!("`{name}` must be a number"))),
                None => Err(de::Error::missing_field(name)),
            }
        };
        let flag = |name: &'static str| -> Result<bool, D::Error> {
            match fields.get(name) {
                Some(Param::Flag(value)) => Ok(*value),
                Some(_) => Err(de::Error::custom(format!("`{name}` must be true or false"))),
                None => Err(de::Error::missing_field(name)),
            }
        };
        let count = |name: &'static str| -> Result<u32, D::Error> {
            match fields.get(name) {
                Some(Param::Number(value))
                    if *value >= 0.0 && value.fract() == 0.0 && *value <= f64::from(u32::MAX) =>
                {
                    Ok(*value as u32)
                }
                Some(_) => Err(de::Error::custom(format!(
                    "`{name}` must be a whole number of samples"
                ))),
                None => Err(de::Error::missing_field(name)),
            }
        };
        let allowed = match kind.as_str() {
            "setpoint" => Some(SETPOINT_FIELDS),
            "level" => Some(LEVEL_FIELDS),
            "surge" => Some(SURGE_FIELDS),
            "monitor" => Some(MONITOR_FIELDS),
            _ => None,
        };
        if let Some(allowed) = allowed {
            if let Some(extra) = fields.keys().find(|name| !allowed.contains(&name.as_str())) {
                return Err(de::Error::unknown_field(extra, allowed));
            }
        }
        Ok(match kind.as_str() {
            "setpoint" => ControlSpec::Setpoint {
                setpoint: number("setpoint")?,
                hysteresis: number("hysteresis")?,
                cooling: flag("cooling")?,
                safe_band: number("safe_band")?,
            },
            "level" => ControlSpec::Level {
                empty: number("empty")?,
                warn_within: count("warn_within")?,
            },
            "surge" => ControlSpec::Surge {
                rising: flag("rising")?,
                limit: number("limit")?,
            },
            "monitor" => ControlSpec::Monitor,
            _ => ControlSpec::Custom {
                kind,
                params: Params::from(fields),
            },
        })
    }
}

/// How often a node samples as its battery drains, in plain seconds.
///
/// This is the serializable form of a [`PowerPlan`](pamoja_power::PowerPlan): a
/// manifest carries the three work intervals as whole seconds and the two
/// state-of-charge thresholds, and [`plan`](PowerSchedule::plan) assembles the
/// `pamoja-power` governor from them. The thresholds may be omitted from a manifest,
/// in which case they default to entering the saver cadence below 50% charge and the
/// critical cadence below 20%. So may the hysteresis margin, which defaults to five
/// points: a node that fell below a threshold climbs back once the charge is 5% above
/// it, so a charge hovering at the threshold does not switch the cadence every cycle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerSchedule {
    /// Seconds between samples at a healthy charge.
    pub active_secs: u64,
    /// Seconds between samples while conserving.
    pub saver_secs: u64,
    /// Seconds between samples when critically low.
    pub critical_secs: u64,
    /// Enter the saver cadence below this state of charge.
    #[serde(default = "PowerSchedule::default_saver_below")]
    pub saver_below: f32,
    /// Enter the critical cadence below this state of charge.
    #[serde(default = "PowerSchedule::default_critical_below")]
    pub critical_below: f32,
    /// How far above a threshold the charge must climb to leave the lower cadence.
    #[serde(default = "PowerSchedule::default_hysteresis")]
    pub hysteresis: f32,
}

impl PowerSchedule {
    fn default_saver_below() -> f32 {
        0.5
    }

    fn default_critical_below() -> f32 {
        0.2
    }

    fn default_hysteresis() -> f32 {
        pamoja_power::DEFAULT_HYSTERESIS
    }

    /// Creates a schedule from its three work intervals, with default thresholds.
    ///
    /// # Arguments
    ///
    /// * `active_secs` - seconds between samples at a healthy charge.
    /// * `saver_secs` - seconds between samples while conserving.
    /// * `critical_secs` - seconds between samples when critically low.
    ///
    /// # Returns
    ///
    /// A schedule that enters the saver cadence below 50% charge and the critical
    /// cadence below 20%, and leaves each once the charge is five points above it.
    pub fn new(active_secs: u64, saver_secs: u64, critical_secs: u64) -> Self {
        Self {
            active_secs,
            saver_secs,
            critical_secs,
            saver_below: Self::default_saver_below(),
            critical_below: Self::default_critical_below(),
            hysteresis: Self::default_hysteresis(),
        }
    }

    /// Sets the state-of-charge thresholds for entering each lower cadence.
    ///
    /// # Arguments
    ///
    /// * `saver_below` - enter the saver cadence when charge is below this.
    /// * `critical_below` - enter the critical cadence when charge is below this,
    ///   normally lower than `saver_below`.
    ///
    /// # Returns
    ///
    /// The updated schedule, for chaining.
    pub fn with_thresholds(mut self, saver_below: f32, critical_below: f32) -> Self {
        self.saver_below = saver_below;
        self.critical_below = critical_below;
        self
    }

    /// Sets how far above a threshold the charge must climb to leave the lower cadence.
    ///
    /// # Arguments
    ///
    /// * `margin` - the state of charge added to each threshold on the way back up; `0.0`
    ///   switches cadence at the thresholds themselves.
    ///
    /// # Returns
    ///
    /// The updated schedule, for chaining.
    pub fn with_hysteresis(mut self, margin: f32) -> Self {
        self.hysteresis = margin;
        self
    }

    /// Assembles the `pamoja-power` governor this schedule describes.
    ///
    /// # Returns
    ///
    /// A [`PowerPlan`](pamoja_power::PowerPlan) with this schedule's intervals,
    /// thresholds, and hysteresis.
    pub fn plan(&self) -> PowerPlan {
        PowerPlan::new(
            Duration::from_secs(self.active_secs),
            Duration::from_secs(self.saver_secs),
            Duration::from_secs(self.critical_secs),
        )
        .thresholds(self.saver_below, self.critical_below)
        .with_hysteresis(self.hysteresis)
    }
}

/// A named, pre-wired bundle of control policy, publish topic, and power schedule.
///
/// A profile is the unit a builder instantiates instead of wiring pins and tuning
/// constants, and it is plain data: it serializes to and from a manifest a community
/// can write, store in a file, and share. Pick a preset such as
/// [`vaccine_fridge_monitor`](Profile::vaccine_fridge_monitor) or load one with
/// [`from_json`](Profile::from_json), hand it a sensor, an actuator, a transport, and
/// a codec, and the resulting [`Node`](crate::Node) reads, decides, drives the
/// output, and publishes on its own. Every field is public, so a deployment can
/// adjust the policy, topic, or power schedule in place.
///
/// A manifest is written in the format [`FORMAT`](crate::FORMAT) names, and may say so
/// with a `$schema` naming [`Profile::SCHEMA`], which an editor reads to check the file
/// as it is typed. A field the format does not have is refused, with the nearest one it
/// does, so a misspelling never leaves a default in place without a word.
///
/// # Examples
///
/// ```
/// use pamoja_profile::{ControlSpec, Profile};
///
/// let profile = Profile::vaccine_fridge_monitor();
/// assert_eq!(profile.name, "vaccine-fridge-monitor");
/// assert!(matches!(profile.control, ControlSpec::Setpoint { .. }));
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProfileFile", into = "ProfileFile")]
pub struct Profile {
    /// A stable, human-readable name, such as `"vaccine-fridge-monitor"`.
    pub name: String,
    /// What the profile is for, in a sentence or two: what it watches or holds, and what
    /// it does when a reading crosses a line. A manifest shared in a catalog explains
    /// itself with it; a profile built in code may leave it `None`.
    pub description: Option<String>,
    /// What the profile reads, the quantity and the unit its numbers are in. A profile
    /// whose node reads a whole measurement through a policy of its own may leave it
    /// `None`.
    pub reads: Option<Reads>,
    /// The topic each reading is published to.
    pub topic: String,
    /// The control policy applied to each reading.
    pub control: ControlSpec,
    /// The power schedule that sets how often the node samples as the battery drains.
    pub power: PowerSchedule,
    /// How this profile presents itself on the dashboard - its custom sensors, node
    /// stats, and theme. A profile that introduces no element beyond the dashboard's
    /// built-in set leaves this `None`.
    pub presentation: Option<Presentation>,
}

/// A profile as its manifest writes it, with the `$schema` that names its format.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileFile {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    schema: Option<String>,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reads: Option<Reads>,
    topic: String,
    control: ControlSpec,
    power: PowerSchedule,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    presentation: Option<Presentation>,
}

impl TryFrom<ProfileFile> for Profile {
    type Error = String;

    fn try_from(file: ProfileFile) -> Result<Self, String> {
        format::check_schema(file.schema.as_deref(), "profile", Profile::SCHEMA)?;
        Ok(Profile {
            name: file.name,
            description: file.description,
            reads: file.reads,
            topic: file.topic,
            control: file.control,
            power: file.power,
            presentation: file.presentation,
        })
    }
}

impl From<Profile> for ProfileFile {
    fn from(profile: Profile) -> Self {
        ProfileFile {
            schema: Some(Profile::SCHEMA.to_owned()),
            name: profile.name,
            description: profile.description,
            reads: profile.reads,
            topic: profile.topic,
            control: profile.control,
            power: profile.power,
            presentation: profile.presentation,
        }
    }
}

impl Profile {
    /// The address of the published JSON Schema for the manifest format this build writes,
    /// which [`to_json`](Profile::to_json) names as the manifest's `$schema`.
    pub const SCHEMA: &'static str = "https://pamoja.molex.cloud/schema/profile-1.json";

    /// Creates a profile of the caller's own from its parts, with no description, no
    /// declaration of what it reads, and no presentation.
    ///
    /// # Arguments
    ///
    /// * `name` - a stable, human-readable name, such as `"raised-bed-drip"`.
    /// * `topic` - the topic each reading is published to.
    /// * `control` - the control policy applied to each reading.
    /// * `power` - how often the node samples as the battery drains.
    ///
    /// # Returns
    ///
    /// The profile. [`with_description`](Profile::with_description),
    /// [`with_reads`](Profile::with_reads), and
    /// [`with_presentation`](Profile::with_presentation) add the rest.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::{ControlSpec, PowerSchedule, Profile};
    ///
    /// let drip = Profile::new(
    ///     "raised-bed-drip",
    ///     "garden/bed-1/moisture",
    ///     ControlSpec::Setpoint { setpoint: 37.5, hysteresis: 7.5, cooling: false, safe_band: 15.0 },
    ///     PowerSchedule::new(300, 1800, 3600),
    /// )
    /// .with_reads("soil_moisture", "percent");
    /// assert_eq!(drip.name, "raised-bed-drip");
    /// assert_eq!(Profile::from_json(&drip.to_json()?)?, drip);
    /// # Ok::<(), pamoja_core::Error>(())
    /// ```
    pub fn new(
        name: impl Into<String>,
        topic: impl Into<String>,
        control: ControlSpec,
        power: PowerSchedule,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            reads: None,
            topic: topic.into(),
            control,
            power,
            presentation: None,
        }
    }

    /// Declares what the profile reads.
    ///
    /// # Arguments
    ///
    /// * `quantity` - the quantity its control decides on, such as `"temperature"`.
    /// * `unit` - the unit its numbers are in, such as `"celsius"`.
    ///
    /// # Returns
    ///
    /// The profile, for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::Profile;
    ///
    /// let fridge = Profile::vaccine_fridge_monitor();
    /// let reads = fridge.reads.as_ref().expect("every preset says what it reads");
    /// assert_eq!((reads.quantity.as_str(), reads.unit.as_str()), ("temperature", "celsius"));
    /// ```
    pub fn with_reads(mut self, quantity: impl Into<String>, unit: impl Into<String>) -> Self {
        self.reads = Some(Reads::new(quantity, unit));
        self
    }

    /// A cold-chain fridge monitor: hold 5 C and alert on a spoilage excursion.
    ///
    /// Switches a cooler to hold the contents near 5 C and raises an
    /// [`Alert::OutOfRange`](crate::Alert::OutOfRange) the moment the temperature
    /// leaves the 2-8 C safe range. Data integrity outweighs power here, so it keeps
    /// sampling often even as the battery drains.
    ///
    /// # Returns
    ///
    /// The cold-chain monitoring profile.
    pub fn vaccine_fridge_monitor() -> Self {
        Self {
            name: "vaccine-fridge-monitor".to_owned(),
            description: Some(
                "Holds a vaccine fridge at 5 C by switching its cooler, and raises an \
                 alert the moment the temperature leaves the 2 to 8 C safe range. Keeps \
                 sampling often as the battery drains, since an unnoticed excursion \
                 costs more than a flat battery."
                    .to_owned(),
            ),
            reads: Some(Reads::new("temperature", "celsius")),
            topic: "cold-chain/fridge/temperature".to_owned(),
            control: ControlSpec::Setpoint {
                setpoint: 5.0,
                hysteresis: 0.5,
                cooling: true,
                safe_band: 3.0,
            },
            power: PowerSchedule::new(60, 300, 900),
            presentation: Some(
                Presentation::new()
                    .with_element(
                        ElementSpec::new(
                            "fridge_temp",
                            "celsius",
                            "Fridge temperature",
                            Viz::Thermometer,
                        )
                        .with_band(2.0, 8.0)
                        .with_locale_label("fr", "Température du réfrigérateur")
                        .with_locale_label("sw", "Joto la friji"),
                    )
                    .with_element(
                        ElementSpec::new("cooler", "state", "Cooler", Viz::Switch)
                            .with_state("state.cooler_off")
                            .with_locale_label("fr", "Groupe froid")
                            .with_locale_label("sw", "Kipoza"),
                    )
                    .with_element(
                        ElementSpec::new("compressor_duty", "percent", "Compressor duty", Viz::Bar)
                            .as_stat()
                            .with_band(0.0, 60.0)
                            .with_locale_label("fr", "Cycle du compresseur")
                            .with_locale_label("sw", "Utendaji wa kompresa"),
                    )
                    .with_message(
                        "state.cooler_off",
                        LocalizedText::per_locale([
                            ("en", "Off"),
                            ("fr", "Arrêt"),
                            ("sw", "Imezimwa"),
                        ]),
                    )
                    .with_message(
                        "state.cooler_on",
                        LocalizedText::per_locale([
                            ("en", "Running"),
                            ("fr", "En marche"),
                            ("sw", "Inafanya kazi"),
                        ]),
                    ),
            ),
        }
    }

    /// An irrigation node: hold soil moisture near a target by opening a valve.
    ///
    /// Treats the valve as a "heater" for soil moisture, opening it when the soil
    /// dries below the band and closing it once it is wet enough, and alerts if the
    /// soil falls critically dry. Samples less often than the fridge, since soil
    /// changes slowly and battery life matters more.
    ///
    /// # Returns
    ///
    /// The irrigation profile.
    pub fn irrigation_node() -> Self {
        Self {
            name: "irrigation-node".to_owned(),
            description: Some(
                "Opens an irrigation valve when soil moisture falls below 30 % and \
                 closes it again above 40 %, and alerts when the soil dries below 10 % \
                 or is waterlogged above 60 %. Samples slowly, since soil changes over \
                 hours and the battery has to last."
                    .to_owned(),
            ),
            reads: Some(Reads::new("soil_moisture", "percent")),
            topic: "farm/irrigation/soil-moisture".to_owned(),
            control: ControlSpec::Setpoint {
                setpoint: 35.0,
                hysteresis: 5.0,
                cooling: false,
                safe_band: 25.0,
            },
            power: PowerSchedule::new(300, 1800, 3600),
            presentation: Some(
                Presentation::new()
                    .with_element(
                        ElementSpec::new("soil_moisture", "percent", "Soil moisture", Viz::Droplet)
                            .with_band(10.0, 60.0)
                            .with_locale_label("fr", "Humidité du sol")
                            .with_locale_label("sw", "Unyevu wa udongo"),
                    )
                    .with_element(
                        ElementSpec::new("drip_valve", "state", "Drip valve", Viz::Valve)
                            .with_state("state.closed")
                            .with_locale_label("fr", "Vanne goutte-à-goutte")
                            .with_locale_label("sw", "Vali ya matone"),
                    ),
            ),
        }
    }

    /// A well-level monitor: report depth and warn before the well runs dry.
    ///
    /// Observes the water level without driving an output and raises an
    /// [`Alert::RunningOut`](crate::Alert::RunningOut) once the level is on course to
    /// reach the dry mark within a few more samples.
    ///
    /// # Returns
    ///
    /// The well-level monitoring profile.
    pub fn well_level() -> Self {
        Self {
            name: "well-level".to_owned(),
            description: Some(
                "Reports a well's water level and warns once the level is on course to \
                 reach the dry mark within six more samples, so a pump is stopped \
                 before it runs dry."
                    .to_owned(),
            ),
            reads: Some(Reads::new("water_level", "meter")),
            topic: "water/well/level".to_owned(),
            control: ControlSpec::Level {
                empty: 0.5,
                warn_within: 6,
            },
            power: PowerSchedule::new(600, 1800, 3600),
            presentation: Some(
                Presentation::new()
                    .with_element(
                        ElementSpec::new("well_level", "meter", "Well level", Viz::Bar)
                            .with_band(1.0, 6.0)
                            .with_locale_label("fr", "Niveau du puits")
                            .with_locale_label("sw", "Kina cha kisima"),
                    )
                    .with_element(
                        ElementSpec::new("battery_voltage", "volt", "Battery", Viz::Battery)
                            .as_stat()
                            .with_band(3.5, 4.3)
                            .with_locale_label("fr", "Batterie")
                            .with_locale_label("sw", "Betri"),
                    ),
            ),
        }
    }

    /// A flash-flood sensor: warn when a river level rises dangerously fast.
    ///
    /// Watches a river or stream gauge and raises an
    /// [`Alert::ChangingFast`](crate::Alert::ChangingFast) when the level rises more
    /// than 0.3 m in a single sample, the signature of a flash flood. It samples
    /// often, because a flood gives little warning.
    ///
    /// # Returns
    ///
    /// The flash-flood monitoring profile.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::{Alert, Profile};
    ///
    /// let mut control = Profile::flood_sensor().controller()?;
    /// control.evaluate(1.0); // first fix establishes the level
    /// let reaction = control.evaluate(1.5); // the river jumped 0.5 m
    /// assert!(matches!(reaction.alert, Some(Alert::ChangingFast { .. })));
    /// # Ok::<(), pamoja_core::Error>(())
    /// ```
    pub fn flood_sensor() -> Self {
        Self {
            name: "flood-sensor".to_owned(),
            description: Some(
                "Watches a river gauge and warns when the level rises more than 0.3 m \
                 in one sample, the signature of a flash flood. Samples every minute, \
                 since a flood gives little warning."
                    .to_owned(),
            ),
            reads: Some(Reads::new("water_level", "meter")),
            topic: "water/river/level".to_owned(),
            control: ControlSpec::Surge {
                rising: true,
                limit: 0.3,
            },
            power: PowerSchedule::new(60, 300, 900),
            presentation: Some(
                Presentation::new()
                    .with_element(
                        ElementSpec::new("river_level", "meter", "River level", Viz::Wave)
                            .with_band(0.2, 2.5)
                            .wide()
                            .with_locale_label("fr", "Niveau de la rivière")
                            .with_locale_label("sw", "Kina cha mto"),
                    )
                    .with_element(
                        ElementSpec::new("rainfall", "millimeter", "Rainfall", Viz::Bar)
                            .with_band(0.0, 25.0)
                            .with_locale_label("fr", "Précipitations")
                            .with_locale_label("sw", "Mvua iliyonyesha"),
                    ),
            ),
        }
    }

    /// Assembles this profile's [`ControlSpec`] into a live [`Controller`].
    ///
    /// Each call builds a new controller, so keep the one it returns for the life of the
    /// node. One built again for each reading forgets whether its output was on and what
    /// the reading before was, so a heater never holds through its deadband and a level
    /// or a surge never has a previous reading to measure against.
    ///
    /// # Returns
    ///
    /// A fresh controller implementing the profile's policy, with its control state
    /// reset.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) when the profile names a custom
    /// kind, which no built-in controller decides: a
    /// [`PolicyRegistry`](crate::PolicyRegistry) that registers the kind resolves it to
    /// the code that does. The reason names the kind, and the built-in kind it is
    /// probably a misspelling of when there is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::{ControlSpec, Params, PowerSchedule, Profile};
    ///
    /// let typo = Profile::new(
    ///     "cellar-heater",
    ///     "home/cellar/temperature",
    ///     ControlSpec::custom("setpiont", Params::new())?,
    ///     PowerSchedule::new(300, 900, 1800),
    /// );
    /// let refused = typo.controller().unwrap_err().to_string();
    /// assert!(refused.contains("did you mean `setpoint`?"), "{refused}");
    /// # Ok::<(), pamoja_core::Error>(())
    /// ```
    pub fn controller(&self) -> pamoja_core::Result<Controller> {
        Controller::from_spec(&self.control)
    }

    /// Attaches a dashboard [`Presentation`] declaring this profile's custom elements.
    ///
    /// A profile that measures something the dashboard does not draw out of the box - a
    /// turbidity probe, a custom node stat - carries the graphic, band, and label for it
    /// here, so the dashboard offers and renders it with no code.
    ///
    /// # Arguments
    ///
    /// * `presentation` - how this profile presents itself on the dashboard.
    ///
    /// # Returns
    ///
    /// The profile, for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::{ElementSpec, Presentation, Profile, Viz};
    ///
    /// // `with_presentation` sets the whole presentation, so on a shipped preset it
    /// // replaces the elements the preset came with: a well-level profile drawn this
    /// // way shows only the turbidity gauge. `with_element` adds one and keeps the rest.
    /// let replaced = Profile::well_level().with_presentation(
    ///     Presentation::new().with_element(
    ///         ElementSpec::new("water_turbidity", "ntu", "Turbidity", Viz::Gauge)
    ///             .with_band(0.0, 5.0),
    ///     ),
    /// );
    /// let elements = &replaced.presentation.unwrap().elements;
    /// assert_eq!(elements.len(), 1);
    /// assert_eq!(elements[0].viz.kind(), "radial");
    /// ```
    pub fn with_presentation(mut self, presentation: Presentation) -> Self {
        self.presentation = Some(presentation);
        self
    }

    /// Adds one element to how this profile presents itself, keeping the rest.
    ///
    /// [`with_presentation`](Profile::with_presentation) replaces a profile's whole
    /// presentation, which drops what a preset already declared. This adds to it, so a
    /// deployment can hang its own gauge off a shipped profile and keep the profile's
    /// own elements, labels, and theme.
    ///
    /// # Arguments
    ///
    /// * `element` - the custom sensor or node stat to add.
    ///
    /// # Returns
    ///
    /// The profile, for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::{ElementSpec, Profile, Viz};
    ///
    /// // The shipped irrigation profile, plus the turbidity probe this farm also fitted.
    /// let profile = Profile::irrigation_node().with_element(
    ///     ElementSpec::new("water_turbidity", "ntu", "Turbidity", Viz::Gauge).with_band(0.0, 5.0),
    /// );
    /// let keys: Vec<&str> = profile
    ///     .presentation
    ///     .as_ref()
    ///     .unwrap()
    ///     .elements
    ///     .iter()
    ///     .map(|element| element.key.as_str())
    ///     .collect();
    /// assert_eq!(keys, ["soil_moisture", "drip_valve", "water_turbidity"]);
    /// ```
    pub fn with_element(mut self, element: ElementSpec) -> Self {
        self.presentation
            .get_or_insert_with(Presentation::new)
            .elements
            .push(element);
        self
    }

    /// Adds the wording for one state or event code this profile emits.
    ///
    /// The dashboard ships no translation for a code it never knew, so a profile that
    /// raises its own supplies the words here. Like
    /// [`with_element`](Profile::with_element) this keeps whatever the profile already
    /// declared.
    ///
    /// # Arguments
    ///
    /// * `code` - the message key, a `state.` or `event.` code.
    /// * `text` - one text for every locale, or per-locale text.
    ///
    /// # Returns
    ///
    /// The profile, for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::Profile;
    ///
    /// let profile = Profile::irrigation_node()
    ///     .with_message("state.flushing", [("en", "Flushing"), ("sw", "Inasafishwa")]);
    /// assert!(profile.presentation.unwrap().messages.contains_key("state.flushing"));
    /// ```
    pub fn with_message(
        mut self,
        code: impl Into<String>,
        text: impl Into<crate::LocalizedText>,
    ) -> Self {
        self.presentation
            .get_or_insert_with(Presentation::new)
            .messages
            .insert(code.into(), text.into());
        self
    }

    /// Sets what this profile is for, in a sentence or two.
    ///
    /// A manifest shared in a catalog carries its purpose with it, so a reader knows what
    /// the profile watches or holds and what it does about it before opening the file.
    ///
    /// # Arguments
    ///
    /// * `description` - the purpose, in plain words.
    ///
    /// # Returns
    ///
    /// The profile, for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::Profile;
    ///
    /// let profile = Profile::well_level()
    ///     .with_description("Warns before the village borehole runs dry.");
    /// assert!(profile.to_json().unwrap().contains("borehole"));
    /// ```
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Checks the profile for what a manifest can say that a node or a dashboard could not
    /// make sense of.
    ///
    /// A manifest is written by hand and shared, so [`from_json`](Profile::from_json) runs
    /// this on every one it loads, and each language's constructor runs it on the parts it
    /// is given. A profile built in Rust can run it too.
    ///
    /// # Returns
    ///
    /// Nothing when the profile is usable.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) naming the first problem:
    ///
    /// - an empty name, or a topic that is empty or holds `+` or `#`, which make it a
    ///   filter rather than a place to publish;
    /// - a `reads` whose quantity or unit is not lowercase words joined by underscores;
    /// - a control value that is not a finite number, a hysteresis of zero or less, a
    ///   safe band narrower than the hysteresis, a level that warns within no samples, a
    ///   surge limit of zero or less, or a custom kind with no name;
    /// - a sampling interval of zero, intervals that shorten as the battery drains, a
    ///   saver threshold outside 0 to 1, or a critical threshold that is not between 0 and
    ///   the saver threshold;
    /// - anything [`Presentation::check`] refuses in the presentation.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::{ControlSpec, PowerSchedule, Profile};
    ///
    /// // A deadband of zero switches the output on and off on every reading near the
    /// // setpoint, which wears out a relay.
    /// let chattering = Profile::new(
    ///     "cellar-heater",
    ///     "home/cellar/temperature",
    ///     ControlSpec::Setpoint { setpoint: 8.0, hysteresis: 0.0, cooling: false, safe_band: 5.0 },
    ///     PowerSchedule::new(300, 900, 1800),
    /// );
    /// let refused = chattering.check().unwrap_err().to_string();
    /// assert!(refused.contains("the output chatters at the setpoint"));
    /// ```
    pub fn check(&self) -> pamoja_core::Result<()> {
        if self.name.trim().is_empty() {
            return refuse("`name` must not be empty".to_owned());
        }
        let topic = &self.topic;
        if topic.is_empty() {
            return refuse("the topic is empty".to_owned());
        }
        if topic.contains(['+', '#']) {
            return refuse(format!(
                "the topic `{topic}` is a filter; a profile publishes to one topic"
            ));
        }
        if let Some(reads) = &self.reads {
            for (field, word) in [("quantity", &reads.quantity), ("unit", &reads.unit)] {
                if !is_word(word) {
                    return refuse(format!(
                        "the `reads` {field} `{word}` must be lowercase words joined by underscores, such as `relative_humidity`"
                    ));
                }
            }
        }
        self.check_control()?;
        self.check_power()?;
        match &self.presentation {
            Some(presentation) => presentation.check(),
            None => Ok(()),
        }
    }

    /// Checks the control policy's values, which the deserializer reads by type alone.
    fn check_control(&self) -> pamoja_core::Result<()> {
        let finite = |name: &str, value: f32| {
            if value.is_finite() {
                Ok(())
            } else {
                refuse(format!("`{name}` must be a finite number, not {value}"))
            }
        };
        match self.control {
            ControlSpec::Setpoint {
                setpoint,
                hysteresis,
                safe_band,
                ..
            } => {
                finite("setpoint", setpoint)?;
                finite("hysteresis", hysteresis)?;
                finite("safe_band", safe_band)?;
                if hysteresis <= 0.0 {
                    return refuse(
                        "`hysteresis` must be above zero, or the output chatters at the setpoint"
                            .to_owned(),
                    );
                }
                if safe_band < hysteresis {
                    return refuse(format!(
                        "`safe_band` ({safe_band}) is narrower than `hysteresis` ({hysteresis}), so an alert would fire inside the deadband"
                    ));
                }
                Ok(())
            }
            ControlSpec::Level { empty, warn_within } => {
                finite("empty", empty)?;
                if warn_within == 0 {
                    return refuse(
                        "`warn_within` must be at least one sample, or the warning never comes"
                            .to_owned(),
                    );
                }
                Ok(())
            }
            ControlSpec::Surge { limit, .. } => {
                finite("limit", limit)?;
                if limit <= 0.0 {
                    return refuse(
                        "`limit` must be above zero, or every sample is a surge".to_owned(),
                    );
                }
                Ok(())
            }
            ControlSpec::Custom { ref kind, .. } if kind.trim().is_empty() => {
                refuse("a custom control needs a kind to be named by".to_owned())
            }
            ControlSpec::Monitor | ControlSpec::Custom { .. } => Ok(()),
        }
    }

    /// Checks that the schedule slows down as the battery drains.
    fn check_power(&self) -> pamoja_core::Result<()> {
        let power = &self.power;
        if power.active_secs == 0 {
            return refuse("`active_secs` must be at least one second".to_owned());
        }
        if !(power.active_secs <= power.saver_secs && power.saver_secs <= power.critical_secs) {
            return refuse(format!(
                "the intervals must not shorten as the battery drains: active {} s, saver {} s, critical {} s",
                power.active_secs, power.saver_secs, power.critical_secs
            ));
        }
        if !(power.saver_below > 0.0 && power.saver_below <= 1.0) {
            return refuse(format!(
                "`saver_below` must be a state of charge above 0 and at most 1, not {}",
                power.saver_below
            ));
        }
        if !(power.critical_below > 0.0 && power.critical_below < power.saver_below) {
            return refuse(format!(
                "`critical_below` must sit between 0 and `saver_below` ({}), not {}",
                power.saver_below, power.critical_below
            ));
        }
        if !(power.hysteresis >= 0.0 && power.saver_below + power.hysteresis <= 1.0) {
            return refuse(format!(
                "the power `hysteresis` must be 0 or more and leave `saver_below` ({}) plus it at most 1, not {}",
                power.saver_below, power.hysteresis
            ));
        }
        Ok(())
    }
}

/// Whether a name is lowercase words joined by single underscores, such as `soil_moisture`.
fn is_word(name: &str) -> bool {
    name.split('_').all(|part| {
        part.starts_with(|c: char| c.is_ascii_lowercase())
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
}

#[cfg(feature = "json")]
impl Profile {
    /// Loads a profile from a JSON manifest.
    ///
    /// This is how a shared profile reaches a device: a community publishes a manifest
    /// file, and the runtime loads it into a profile to assemble a node from.
    ///
    /// # Arguments
    ///
    /// * `manifest` - the JSON text of the profile.
    ///
    /// # Returns
    ///
    /// The profile described by `manifest`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) if `manifest` is not valid
    /// JSON, does not describe a profile, is written in a format this build does not read,
    /// carries a field the format does not have, or describes a profile
    /// [`check`](Profile::check) refuses. A misspelled field or value is reported with
    /// the name it was probably meant to be.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::Profile;
    ///
    /// // A well-level monitor, shared as a manifest. The power thresholds are
    /// // optional and default when omitted.
    /// let manifest = r#"{
    ///     "$schema": "https://pamoja.molex.cloud/schema/profile-1.json",
    ///     "name": "tank-level",
    ///     "reads": { "quantity": "water_level", "unit": "meter" },
    ///     "topic": "water/tank/level",
    ///     "control": { "kind": "level", "empty": 0.0, "warn_within": 5 },
    ///     "power": { "active_secs": 600, "saver_secs": 1800, "critical_secs": 3600 }
    /// }"#;
    ///
    /// let profile = Profile::from_json(manifest).expect("valid manifest");
    /// assert_eq!(profile.name, "tank-level");
    ///
    /// let mut control = profile.controller().expect("a built-in kind");
    /// control.evaluate(10.0); // first reading establishes a level
    /// assert!(control.evaluate(2.0).alert.is_some()); // falling fast toward empty
    ///
    /// // A misspelled field is refused with the one it was meant to be, rather than
    /// // leaving the default in its place.
    /// let typo = manifest.replace("\"warn_within\"", "\"warn_withn\"");
    /// let refused = Profile::from_json(&typo).unwrap_err().to_string();
    /// assert!(refused.contains("did you mean `warn_within`?"), "{refused}");
    /// ```
    pub fn from_json(manifest: &str) -> pamoja_core::Result<Self> {
        let profile: Profile = serde_json::from_str(manifest)
            .map_err(|error| pamoja_core::Error::Codec(format::explain(&error)))?;
        profile.check()?;
        Ok(profile)
    }

    /// Serializes this profile to a JSON manifest a community can share.
    ///
    /// # Returns
    ///
    /// The pretty-printed JSON text of the profile.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) if the profile cannot be
    /// serialized.
    pub fn to_json(&self) -> pamoja_core::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|error| pamoja_core::Error::Codec(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Alert;

    #[test]
    fn presets_have_stable_names_and_topics() {
        assert_eq!(
            Profile::vaccine_fridge_monitor().name,
            "vaccine-fridge-monitor"
        );
        for preset in [
            Profile::vaccine_fridge_monitor(),
            Profile::irrigation_node(),
            Profile::well_level(),
            Profile::flood_sensor(),
        ] {
            let description = preset.description.expect("every preset explains itself");
            assert!(description.ends_with('.'), "{description}");
        }
        assert_eq!(
            Profile::vaccine_fridge_monitor().topic,
            "cold-chain/fridge/temperature"
        );
        assert_eq!(Profile::irrigation_node().name, "irrigation-node");
        assert_eq!(Profile::well_level().name, "well-level");
    }

    #[test]
    fn the_fridge_controller_cools_and_flags_a_spoilage_excursion() {
        let mut control = Profile::vaccine_fridge_monitor().controller().unwrap();
        let reaction = control.evaluate(9.0);
        assert_eq!(reaction.actuator, Some(true));
        assert!(matches!(reaction.alert, Some(Alert::OutOfRange { .. })));
    }

    #[test]
    fn the_well_controller_observes_without_an_output() {
        let mut control = Profile::well_level().controller().unwrap();
        control.evaluate(3.0);
        assert_eq!(control.evaluate(2.0).actuator, None);
    }

    #[test]
    fn the_flood_controller_warns_on_a_rapid_rise() {
        let mut control = Profile::flood_sensor().controller().unwrap();
        control.evaluate(1.0);
        let reaction = control.evaluate(1.5); // a 0.5 m jump in one sample
        assert!(matches!(reaction.alert, Some(Alert::ChangingFast { .. })));
    }

    #[test]
    fn the_schedule_builds_the_documented_power_plan() {
        use pamoja_power::PowerMode;

        let plan = Profile::vaccine_fridge_monitor().power.plan();
        assert_eq!(plan.mode(0.9), PowerMode::Active);
        assert_eq!(plan.mode(0.1), PowerMode::Critical);
        assert_eq!(plan.interval(0.9), Duration::from_secs(60));
    }

    #[cfg(feature = "json")]
    #[test]
    fn a_profile_round_trips_through_json() {
        // Cover a setpoint profile and a surge profile, the two manifest shapes that
        // carry the most fields.
        for profile in [Profile::irrigation_node(), Profile::flood_sensor()] {
            let json = profile.to_json().expect("serialize");
            let restored = Profile::from_json(&json).expect("deserialize");
            assert_eq!(profile, restored);
        }
    }

    #[cfg(feature = "json")]
    #[test]
    fn a_kind_the_library_never_shipped_loads_with_its_parameters() {
        let manifest = r#"{
            "name": "orchard-frost",
            "topic": "orchard/air/temperature",
            "control": { "kind": "frost_guard", "warn_below": 2.0, "latching": true, "zone": "north" },
            "power": { "active_secs": 60, "saver_secs": 300, "critical_secs": 900 }
        }"#;
        let profile = Profile::from_json(manifest).expect("a custom kind parses");
        let ControlSpec::Custom { kind, params } = &profile.control else {
            panic!("expected a custom kind, got {:?}", profile.control);
        };
        assert_eq!(kind, "frost_guard");
        assert_eq!(profile.control.kind(), "frost_guard");
        assert_eq!(params.number("warn_below"), Some(2.0));
        assert_eq!(params.flag("latching"), Some(true));
        assert_eq!(params.text("zone"), Some("north"));
        assert_eq!(params.len(), 3);

        // No built-in controller decides a custom kind, so asking for one is refused
        // rather than handing back a monitor that would never drive the output.
        assert!(profile.controller().is_err());

        // It writes back in the same flat shape, with the kind first.
        let shared = profile.to_json().expect("serializes");
        assert!(
            shared.contains("\"kind\": \"frost_guard\",\n    \"latching\": true,\n    \"warn_below\": 2.0,\n    \"zone\": \"north\""),
            "{shared}"
        );
        assert_eq!(Profile::from_json(&shared).expect("round trip"), profile);
    }

    #[cfg(feature = "json")]
    #[test]
    fn a_built_in_kind_with_a_wrong_field_is_refused_rather_than_taken_as_custom() {
        let control = |body: &str| {
            let manifest = format!(
                r#"{{ "name": "t", "topic": "t", "control": {body}, "power": {{ "active_secs": 1, "saver_secs": 2, "critical_secs": 3 }} }}"#
            );
            Profile::from_json(&manifest).map(|profile| profile.control)
        };
        let error = control(r#"{ "kind": "setpoint", "setpoint": 5.0 }"#).unwrap_err();
        assert!(error.to_string().contains("hysteresis"), "{error}");
        let error =
            control(r#"{ "kind": "level", "empty": 0.0, "warn_within": 2.5 }"#).unwrap_err();
        assert!(error.to_string().contains("whole number"), "{error}");
        let error = control(r#"{ "kind": "surge", "rising": "up", "limit": 1.0 }"#).unwrap_err();
        assert!(error.to_string().contains("true or false"), "{error}");
        let error = control(r#"{ "setpoint": 5.0 }"#).unwrap_err();
        assert!(error.to_string().contains("kind"), "{error}");
        let error = control(r#"{ "kind": 3 }"#).unwrap_err();
        assert!(error.to_string().contains("string"), "{error}");
        assert_eq!(
            control(r#"{ "kind": "level", "empty": 1, "warn_within": 4 }"#).unwrap(),
            ControlSpec::Level {
                empty: 1.0,
                warn_within: 4
            }
        );
        assert_eq!(
            ControlSpec::BUILT_IN,
            ["setpoint", "level", "surge", "monitor"]
        );
        assert_eq!(ControlSpec::Monitor.kind(), "monitor");
    }

    #[cfg(feature = "json")]
    #[test]
    fn a_field_the_format_does_not_have_is_refused_with_the_one_it_meant() {
        let brooder = Profile::new(
            "brooder",
            "poultry/brooder/temperature",
            ControlSpec::Setpoint {
                setpoint: 32.0,
                hysteresis: 0.5,
                cooling: false,
                safe_band: 4.0,
            },
            PowerSchedule::new(120, 600, 1800),
        )
        .with_reads("temperature", "celsius")
        .with_presentation(
            Presentation::new()
                .with_element(
                    ElementSpec::new(
                        "brooder_temperature",
                        "celsius",
                        "Brooder",
                        Viz::Thermometer,
                    )
                    .with_band(28.0, 36.0),
                )
                .with_theme(crate::Theme {
                    accent: Some("#c8553d".to_owned()),
                    ..crate::Theme::default()
                }),
        );
        let shared = brooder.to_json().unwrap();
        assert!(
            shared.starts_with(
                "{\n  \"$schema\": \"https://pamoja.molex.cloud/schema/profile-1.json\",\n  \"name\": \"brooder\",\n  \"reads\": {\n    \"quantity\": \"temperature\",\n    \"unit\": \"celsius\"\n  },\n  \"topic\""
            ),
            "{shared}"
        );
        assert_eq!(Profile::from_json(&shared).unwrap(), brooder);

        let refused = |from: &str, to: &str| {
            assert!(shared.contains(from), "{from} is not in {shared}");
            match Profile::from_json(&shared.replacen(from, to, 1)) {
                Err(pamoja_core::Error::Codec(reason)) => reason,
                other => panic!("expected {to} to be refused, got {other:?}"),
            }
        };
        let starts = |from: &str, to: &str, expected: &str| {
            let reason = refused(from, to);
            assert!(reason.starts_with(expected), "{reason}");
        };
        starts(
            "\"topic\"",
            "\"topik\"",
            "unknown field `topik`, did you mean `topic`?",
        );
        starts(
            "\"critical_secs\"",
            "\"critical_sec\"",
            "unknown field `critical_sec`, did you mean `critical_secs`?",
        );
        starts(
            "\"safe_band\"",
            "\"safe-band\"",
            "unknown field `safe-band`, did you mean `safe_band`?",
        );
        starts(
            "\"unit\": \"celsius\"\n  }",
            "\"units\": \"celsius\"\n  }",
            "unknown field `units`, did you mean `unit`?",
        );
        starts(
            "\"label\"",
            "\"lable\"",
            "unknown field `lable`, did you mean `label`?",
        );
        starts(
            "\"accent\"",
            "\"acent\"",
            "unknown field `acent`, did you mean `accent`?",
        );
        starts(
            "\"thermometer\"",
            "\"thermometr\"",
            "unknown variant `thermometr`, did you mean `thermometer`?",
        );
        starts(
            "\"name\"",
            "\"colour\": 1, \"name\"",
            "unknown field `colour`, expected one of",
        );
        starts(
            "\"hysteresis\": 0.5",
            "\"hysteresis\": 0.5, \"deadband\": 1.0",
            "unknown field `deadband`, expected one of `kind`, `setpoint`, `hysteresis`, `cooling`, `safe_band`",
        );
        assert!(refused("profile-1.json", "profile-2.json").contains("profile format 2"));
        assert!(refused("\"temperature\"", "\"Temperature\"").contains("lowercase words"));
        assert!(refused("\"celsius\"\n  }", "\"deg C\"\n  }").contains("lowercase words"));

        let unmarked = shared.replacen(
            "  \"$schema\": \"https://pamoja.molex.cloud/schema/profile-1.json\",\n",
            "",
            1,
        );
        assert_eq!(Profile::from_json(&unmarked).unwrap(), brooder);
        let offline = shared.replacen(Profile::SCHEMA, "./profile-1.json", 1);
        assert_eq!(Profile::from_json(&offline).unwrap(), brooder);
    }

    #[cfg(feature = "json")]
    #[test]
    fn a_manifest_may_omit_the_power_thresholds() {
        let manifest = r#"{
            "name": "tank",
            "topic": "water/tank/level",
            "control": { "kind": "level", "empty": 0.0, "warn_within": 4 },
            "power": { "active_secs": 600, "saver_secs": 1800, "critical_secs": 3600 }
        }"#;
        let profile = Profile::from_json(manifest).expect("valid manifest");
        assert_eq!(profile.power.saver_below, 0.5);
        assert_eq!(profile.power.critical_below, 0.2);
        assert_eq!(profile.power.hysteresis, pamoja_power::DEFAULT_HYSTERESIS);
        assert!(matches!(
            profile.control,
            ControlSpec::Level { warn_within: 4, .. }
        ));
        assert_eq!(profile.description, None);
        assert!(!profile.to_json().unwrap().contains("description"));
        let described = profile.with_description("Warns before a rain tank runs dry.");
        let shared = described.to_json().unwrap();
        assert!(shared.contains("\"description\": \"Warns before a rain tank runs dry.\""));
        assert_eq!(Profile::from_json(&shared).unwrap(), described);
    }

    #[test]
    fn every_preset_passes_its_own_check() {
        for preset in [
            Profile::vaccine_fridge_monitor(),
            Profile::irrigation_node(),
            Profile::well_level(),
            Profile::flood_sensor(),
        ] {
            preset
                .check()
                .unwrap_or_else(|error| panic!("{}: {error}", preset.name));
        }
    }

    #[test]
    fn a_profile_no_node_could_run_is_refused_with_the_reason() {
        let base = || {
            Profile::new(
                "brooder",
                "poultry/brooder/temperature",
                ControlSpec::Setpoint {
                    setpoint: 32.0,
                    hysteresis: 0.5,
                    cooling: false,
                    safe_band: 4.0,
                },
                PowerSchedule::new(120, 600, 1800),
            )
        };
        base().check().expect("the base profile is usable");
        let refused = |profile: Profile| profile.check().unwrap_err().to_string();

        let mut unnamed = base();
        unnamed.name = " ".to_owned();
        assert!(refused(unnamed).contains("`name` must not be empty"));
        let mut filter = base();
        filter.topic = "poultry/+/temperature".to_owned();
        assert!(refused(filter).contains("is a filter"));

        let setpoint = |hysteresis: f32, safe_band: f32| {
            let mut profile = base();
            profile.control = ControlSpec::Setpoint {
                setpoint: 32.0,
                hysteresis,
                cooling: false,
                safe_band,
            };
            profile
        };
        assert!(refused(setpoint(0.0, 4.0)).contains("chatters"));
        assert!(refused(setpoint(2.0, 1.0)).contains("inside the deadband"));
        assert!(refused(setpoint(f32::NAN, 4.0)).contains("`hysteresis` must be a finite number"));

        let mut never = base();
        never.control = ControlSpec::Level {
            empty: 0.0,
            warn_within: 0,
        };
        assert!(refused(never).contains("never comes"));
        let mut always = base();
        always.control = ControlSpec::Surge {
            rising: true,
            limit: 0.0,
        };
        assert!(refused(always).contains("every sample is a surge"));

        let schedule = |power: PowerSchedule| {
            let mut profile = base();
            profile.power = power;
            profile
        };
        assert!(refused(schedule(PowerSchedule::new(0, 600, 1800))).contains("at least one second"));
        assert!(refused(schedule(PowerSchedule::new(120, 60, 1800))).contains("must not shorten"));
        let inverted = PowerSchedule::new(120, 600, 1800).with_thresholds(0.2, 0.5);
        assert!(refused(schedule(inverted)).contains("`critical_below` must sit between"));
        let over = PowerSchedule::new(120, 600, 1800).with_thresholds(50.0, 0.2);
        assert!(refused(schedule(over)).contains("not 50"));
        let out_of_reach = PowerSchedule::new(120, 600, 1800).with_thresholds(0.98, 0.2);
        assert!(
            refused(schedule(out_of_reach)).contains("the power `hysteresis` must be 0 or more")
        );
        let negative = PowerSchedule::new(120, 600, 1800).with_hysteresis(-0.1);
        assert!(refused(schedule(negative)).contains("not -0.1"));
        let unknown = PowerSchedule::new(120, 600, 1800).with_hysteresis(f32::NAN);
        assert!(refused(schedule(unknown)).contains("not NaN"));
        let flat = PowerSchedule::new(120, 600, 1800).with_hysteresis(0.0);
        schedule(flat).check().expect("no margin is allowed");

        let drawn = base().with_element(
            ElementSpec::new(
                "brooder_temperature",
                "celsius",
                "Brooder",
                Viz::Thermometer,
            )
            .with_band(36.0, 28.0),
        );
        assert!(refused(drawn).contains("the low end comes first"));
    }

    #[cfg(feature = "json")]
    #[test]
    fn a_manifest_is_checked_as_it_loads() {
        let manifest = r#"{
            "name": "brooder",
            "topic": "poultry/brooder/temperature",
            "control": { "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5, "cooling": false, "safe_band": -4.0 },
            "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
        }"#;
        let refused = Profile::from_json(manifest).unwrap_err().to_string();
        assert!(
            refused.contains("`safe_band` (-4) is narrower than `hysteresis` (0.5)"),
            "{refused}"
        );
    }
}
