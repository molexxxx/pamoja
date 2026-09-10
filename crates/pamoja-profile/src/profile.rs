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

use crate::{Controller, Param, Params, Presentation};

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
/// critical cadence below 20%.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
}

impl PowerSchedule {
    fn default_saver_below() -> f32 {
        0.5
    }

    fn default_critical_below() -> f32 {
        0.2
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
    /// cadence below 20%.
    pub fn new(active_secs: u64, saver_secs: u64, critical_secs: u64) -> Self {
        Self {
            active_secs,
            saver_secs,
            critical_secs,
            saver_below: Self::default_saver_below(),
            critical_below: Self::default_critical_below(),
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

    /// Assembles the `pamoja-power` governor this schedule describes.
    ///
    /// # Returns
    ///
    /// A [`PowerPlan`](pamoja_power::PowerPlan) with this schedule's intervals and
    /// thresholds.
    pub fn plan(&self) -> PowerPlan {
        PowerPlan::new(
            Duration::from_secs(self.active_secs),
            Duration::from_secs(self.saver_secs),
            Duration::from_secs(self.critical_secs),
        )
        .thresholds(self.saver_below, self.critical_below)
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
pub struct Profile {
    /// A stable, human-readable name, such as `"vaccine-fridge-monitor"`.
    pub name: String,
    /// What the profile is for, in a sentence or two: what it watches or holds, and what
    /// it does when a reading crosses a line. A manifest shared in a catalog explains
    /// itself with it; a profile built in code may leave it `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The topic each reading is published to.
    pub topic: String,
    /// The control policy applied to each reading.
    pub control: ControlSpec,
    /// The power schedule that sets how often the node samples as the battery drains.
    pub power: PowerSchedule,
    /// How this profile presents itself on the dashboard - its custom sensors, node
    /// stats, and theme. A profile that introduces no element beyond the dashboard's
    /// built-in set leaves this `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<Presentation>,
}

impl Profile {
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
            topic: "cold-chain/fridge/temperature".to_owned(),
            control: ControlSpec::Setpoint {
                setpoint: 5.0,
                hysteresis: 0.5,
                cooling: true,
                safe_band: 3.0,
            },
            power: PowerSchedule::new(60, 300, 900),
            presentation: None,
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
            topic: "farm/irrigation/soil-moisture".to_owned(),
            control: ControlSpec::Setpoint {
                setpoint: 35.0,
                hysteresis: 5.0,
                cooling: false,
                safe_band: 25.0,
            },
            power: PowerSchedule::new(300, 1800, 3600),
            presentation: None,
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
            topic: "water/well/level".to_owned(),
            control: ControlSpec::Level {
                empty: 0.5,
                warn_within: 6,
            },
            power: PowerSchedule::new(600, 1800, 3600),
            presentation: None,
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
    /// let mut control = Profile::flood_sensor().controller();
    /// control.evaluate(1.0); // first fix establishes the level
    /// let reaction = control.evaluate(1.5); // the river jumped 0.5 m
    /// assert!(matches!(reaction.alert, Some(Alert::ChangingFast { .. })));
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
            topic: "water/river/level".to_owned(),
            control: ControlSpec::Surge {
                rising: true,
                limit: 0.3,
            },
            power: PowerSchedule::new(60, 300, 900),
            presentation: None,
        }
    }

    /// Assembles this profile's [`ControlSpec`] into a live [`Controller`].
    ///
    /// A custom kind has no built-in controller; for one this returns a monitoring
    /// controller, and a [`PolicyRegistry`](crate::PolicyRegistry) resolves the kind to
    /// the code that decides it.
    ///
    /// # Returns
    ///
    /// A fresh controller implementing the profile's policy, with its control state
    /// reset.
    pub fn controller(&self) -> Controller {
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
    /// // A water-monitoring profile that adds a turbidity gauge the dashboard would not
    /// // otherwise know how to draw.
    /// let profile = Profile::well_level().with_presentation(
    ///     Presentation::new().with_element(
    ///         ElementSpec::new("water_turbidity", "ntu", "Turbidity", Viz::Gauge)
    ///             .with_band(0.0, 5.0),
    ///     ),
    /// );
    /// let elements = &profile.presentation.unwrap().elements;
    /// assert_eq!(elements[0].viz.kind(), "radial");
    /// ```
    pub fn with_presentation(mut self, presentation: Presentation) -> Self {
        self.presentation = Some(presentation);
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
    /// JSON or does not describe a profile.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::Profile;
    ///
    /// // A well-level monitor, shared as a manifest. The power thresholds are
    /// // optional and default when omitted.
    /// let manifest = r#"{
    ///     "name": "tank-level",
    ///     "topic": "water/tank/level",
    ///     "control": { "kind": "level", "empty": 0.0, "warn_within": 5 },
    ///     "power": { "active_secs": 600, "saver_secs": 1800, "critical_secs": 3600 }
    /// }"#;
    ///
    /// let profile = Profile::from_json(manifest).expect("valid manifest");
    /// assert_eq!(profile.name, "tank-level");
    ///
    /// let mut control = profile.controller();
    /// control.evaluate(10.0); // first reading establishes a level
    /// assert!(control.evaluate(2.0).alert.is_some()); // falling fast toward empty
    /// ```
    pub fn from_json(manifest: &str) -> pamoja_core::Result<Self> {
        serde_json::from_str(manifest).map_err(|error| pamoja_core::Error::Codec(error.to_string()))
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
    use crate::{Alert, Reaction};

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
        let mut control = Profile::vaccine_fridge_monitor().controller();
        let reaction = control.evaluate(9.0);
        assert_eq!(reaction.actuator, Some(true));
        assert!(matches!(reaction.alert, Some(Alert::OutOfRange { .. })));
    }

    #[test]
    fn the_well_controller_observes_without_an_output() {
        let mut control = Profile::well_level().controller();
        control.evaluate(3.0);
        assert_eq!(control.evaluate(2.0).actuator, None);
    }

    #[test]
    fn the_flood_controller_warns_on_a_rapid_rise() {
        let mut control = Profile::flood_sensor().controller();
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

        // The built-in controller for a custom kind observes only.
        assert_eq!(profile.controller().evaluate(-5.0), Reaction::default());

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
}
