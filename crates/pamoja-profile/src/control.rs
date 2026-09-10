//! The decision logic a profile assembles: turning a reading into a reaction.

use std::collections::BTreeMap;

use pamoja_core::{Error, Result};
use pamoja_kit::{Depletion, Surge, Thermostat};

use crate::{ControlSpec, Params};

/// An alert raised when a reading crosses a profile's safety threshold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alert {
    /// A controlled reading drifted outside its safe band.
    ///
    /// For a cold-chain fridge this is a spoilage excursion: the cooler may be
    /// running, but the contents are no longer within the safe temperature range.
    OutOfRange {
        /// The reading that triggered the alert.
        reading: f32,
    },
    /// A falling level will reach its empty mark within this many more samples.
    RunningOut {
        /// The estimated number of samples until the level reaches empty.
        samples: u32,
    },
    /// A reading is changing faster than its safe rate.
    ///
    /// For a river gauge this is a flash-flood warning: the level jumped further in
    /// one sample than the profile allows.
    ChangingFast {
        /// The change since the previous sample, as a positive number.
        rate: f32,
    },
    /// A condition a policy of your own raised, named by a code it chose.
    ///
    /// The code is a fixed identifier the policy's author picks, such as
    /// `"FrostRisk"`, and the value is the measurement behind it. The bindings carry
    /// it as they carry the built-in kinds, as the code and a number, so a custom
    /// condition reads the same wherever the node's code is written.
    Custom {
        /// The condition's name, chosen by the policy that raises it.
        code: &'static str,
        /// The measurement behind the condition.
        value: f32,
    },
}

impl Alert {
    /// Returns the name of the condition, without the measurement that triggered it.
    ///
    /// The bindings carry an alert as a kind and a value, so this is the same word in
    /// every language, which is what lets one reading be logged or compared the same way
    /// wherever the node's code is written.
    ///
    /// # Returns
    ///
    /// One of `"OutOfRange"`, `"RunningOut"`, or `"ChangingFast"`, or the code a
    /// custom alert was raised with.
    pub fn kind(self) -> &'static str {
        match self {
            Alert::OutOfRange { .. } => "OutOfRange",
            Alert::RunningOut { .. } => "RunningOut",
            Alert::ChangingFast { .. } => "ChangingFast",
            Alert::Custom { code, .. } => code,
        }
    }
}

/// The outcome of evaluating one reading against a policy.
///
/// A [`Controller`] issues an on/off command, so a reaction is `Reaction<bool>` by
/// default; a policy of your own issues whatever its actuator takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reaction<C = bool> {
    /// The actuator setting this reading calls for, if the policy drives one.
    ///
    /// For a controller, `Some(true)` switches the output on, `Some(false)` switches
    /// it off, and `None` means the profile observes without driving an output.
    pub actuator: Option<C>,
    /// An alert, if the reading crossed a threshold; `None` otherwise.
    pub alert: Option<Alert>,
}

impl<C> Default for Reaction<C> {
    fn default() -> Self {
        Self {
            actuator: None,
            alert: None,
        }
    }
}

/// The decision half of a node: what one reading calls for.
///
/// A [`Controller`] is the policy behind every built-in kind, deciding an `f32`
/// reading into an on/off command. A policy of your own decides whatever a driver
/// reads, a whole measurement struct included, into whatever an actuator takes, and
/// [`Node::with_policy`](crate::Node::with_policy) runs it in the same read, decide,
/// act, publish loop. Evaluation is synchronous and hardware-free, so a policy is
/// unit-testable with no devices and no network.
///
/// # Examples
///
/// ```
/// use pamoja_profile::{Alert, Policy, Reaction};
///
/// // Frost settles when the air is cold and damp, so this reads both at once and
/// // switches a heater, raising its own condition when the risk is on.
/// struct FrostGuard {
///     warn_below: f32,
/// }
///
/// impl Policy for FrostGuard {
///     type Reading = (f32, f32);
///     type Command = bool;
///
///     fn evaluate(&mut self, &(temperature, humidity): &(f32, f32)) -> Reaction {
///         let risk = temperature < self.warn_below && humidity > 80.0;
///         Reaction {
///             actuator: Some(risk),
///             alert: risk.then_some(Alert::Custom { code: "FrostRisk", value: temperature }),
///         }
///     }
/// }
///
/// let mut guard = FrostGuard { warn_below: 2.0 };
/// let cold = guard.evaluate(&(1.0, 92.0));
/// assert_eq!(cold.actuator, Some(true));
/// assert_eq!(cold.alert.map(Alert::kind), Some("FrostRisk"));
/// assert_eq!(guard.evaluate(&(1.0, 40.0)).alert, None);
/// ```
pub trait Policy {
    /// What the policy decides on: a number, or a driver's whole measurement.
    type Reading;
    /// What the policy issues: an on/off setting, a duty, a setpoint.
    type Command;

    /// Decides what one reading calls for.
    ///
    /// # Arguments
    ///
    /// * `reading` - the latest reading, borrowed so the node can still publish it.
    ///
    /// # Returns
    ///
    /// The command to apply, if any, and any alert the reading raised.
    fn evaluate(&mut self, reading: &Self::Reading) -> Reaction<Self::Command>;
}

impl<P: Policy + ?Sized> Policy for Box<P> {
    type Reading = P::Reading;
    type Command = P::Command;

    fn evaluate(&mut self, reading: &Self::Reading) -> Reaction<Self::Command> {
        (**self).evaluate(reading)
    }
}

impl Policy for Controller {
    type Reading = f32;
    type Command = bool;

    fn evaluate(&mut self, reading: &f32) -> Reaction {
        Controller::evaluate(self, *reading)
    }
}

/// A policy behind a trait object, as a [`PolicyRegistry`] hands one out.
pub type BoxedPolicy<R = f32, C = bool> = Box<dyn Policy<Reading = R, Command = C> + Send>;

/// The code that builds a custom kind's policy from the parameters its manifest carries.
type Factory<R, C> = Box<dyn Fn(&Params) -> Result<BoxedPolicy<R, C>> + Send + Sync>;

/// What resolves a manifest's control kind to the code that decides it.
///
/// The four built-in kinds resolve to a [`Controller`]. A kind pamoja never shipped
/// resolves to the factory registered under its name, which reads the parameters the
/// manifest carried beside the kind and builds the policy, so a fleet can share a
/// manifest for a policy of its own the same way it shares one for a setpoint. A
/// registry is generic over the reading and command its policies work in;
/// [`resolve`](PolicyRegistry::resolve) exists for the `f32` reading and `bool` command
/// the built-in kinds share, and [`custom`](PolicyRegistry::custom) for any other pair.
///
/// # Examples
///
/// ```
/// use pamoja_core::Error;
/// use pamoja_profile::{Alert, BoxedPolicy, Policy, PolicyRegistry, Profile, Reaction};
///
/// struct FrostGuard {
///     warn_below: f32,
/// }
///
/// impl Policy for FrostGuard {
///     type Reading = f32;
///     type Command = bool;
///
///     fn evaluate(&mut self, reading: &f32) -> Reaction {
///         let cold = *reading < self.warn_below;
///         Reaction {
///             actuator: Some(cold),
///             alert: cold.then_some(Alert::Custom { code: "FrostRisk", value: *reading }),
///         }
///     }
/// }
///
/// // The manifest names a kind the library has never heard of, with its parameter
/// // beside it, exactly as a built-in kind carries its own.
/// let manifest = r#"{
///     "name": "orchard-frost",
///     "topic": "orchard/air/temperature",
///     "control": { "kind": "frost_guard", "warn_below": 2.0 },
///     "power": { "active_secs": 60, "saver_secs": 300, "critical_secs": 900 }
/// }"#;
/// let profile = Profile::from_json(manifest)?;
///
/// let registry = PolicyRegistry::new().register("frost_guard", |params| {
///     let warn_below = params
///         .number("warn_below")
///         .ok_or(Error::Unsupported("frost_guard needs a `warn_below` parameter"))?;
///     Ok(Box::new(FrostGuard { warn_below: warn_below as f32 }) as BoxedPolicy)
/// });
/// let mut policy = registry.resolve(&profile.control)?;
/// assert_eq!(policy.evaluate(&1.0).alert.map(Alert::kind), Some("FrostRisk"));
/// assert_eq!(policy.evaluate(&5.0).actuator, Some(false));
///
/// // A built-in kind resolves through the same registry.
/// let mut fridge = registry.resolve(&Profile::vaccine_fridge_monitor().control)?;
/// assert_eq!(fridge.evaluate(&9.0).actuator, Some(true));
/// # Ok::<(), Error>(())
/// ```
pub struct PolicyRegistry<R = f32, C = bool> {
    factories: BTreeMap<String, Factory<R, C>>,
}

impl<R, C> PolicyRegistry<R, C> {
    /// Starts a registry with no custom kinds.
    ///
    /// # Returns
    ///
    /// A registry that resolves only what is registered on it.
    pub fn new() -> Self {
        Self {
            factories: BTreeMap::new(),
        }
    }

    /// Registers the factory for a custom kind, replacing one of the same name.
    ///
    /// # Arguments
    ///
    /// * `kind` - the kind as a manifest names it, such as `"frost_guard"`.
    /// * `factory` - builds the policy from the parameters beside the kind, or says
    ///   which parameter it needed.
    ///
    /// # Returns
    ///
    /// The registry, for chaining.
    pub fn register<F>(mut self, kind: impl Into<String>, factory: F) -> Self
    where
        F: Fn(&Params) -> Result<BoxedPolicy<R, C>> + Send + Sync + 'static,
    {
        self.factories.insert(kind.into(), Box::new(factory));
        self
    }

    /// Lists the custom kinds registered, in name order.
    ///
    /// # Returns
    ///
    /// Each kind's name.
    pub fn kinds(&self) -> impl Iterator<Item = &str> {
        self.factories.keys().map(String::as_str)
    }

    /// Builds the policy for a custom kind from its parameters.
    ///
    /// # Arguments
    ///
    /// * `kind` - the kind as a manifest names it.
    /// * `params` - the parameters the manifest carried beside it.
    ///
    /// # Returns
    ///
    /// The policy, ready to evaluate readings.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unsupported`] when no factory is registered for the kind, or
    /// whatever the factory returns when the parameters are not what it needs.
    pub fn custom(&self, kind: &str, params: &Params) -> Result<BoxedPolicy<R, C>> {
        match self.factories.get(kind) {
            Some(factory) => factory(params),
            None => Err(Error::Unsupported(
                "the manifest names a control kind no policy is registered for",
            )),
        }
    }
}

impl PolicyRegistry<f32, bool> {
    /// Resolves a manifest's control kind: a built-in kind to its [`Controller`], a
    /// custom kind to the policy its factory builds.
    ///
    /// # Arguments
    ///
    /// * `spec` - the control policy a profile carries.
    ///
    /// # Returns
    ///
    /// The policy, ready to evaluate readings.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unsupported`] when the kind is custom and no factory is
    /// registered for it, or whatever the factory returns.
    pub fn resolve(&self, spec: &ControlSpec) -> Result<BoxedPolicy<f32, bool>> {
        match spec {
            ControlSpec::Custom { kind, params } => self.custom(kind, params),
            built_in => Ok(Box::new(Controller::from_spec(built_in))),
        }
    }
}

impl<R, C> Default for PolicyRegistry<R, C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R, C> core::fmt::Debug for PolicyRegistry<R, C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PolicyRegistry")
            .field("kinds", &self.factories.keys().collect::<Vec<_>>())
            .finish()
    }
}

// The live policy behind a `Controller`. It is private so the controller's public
// surface stays its constructors and `evaluate`, not the kit helpers it wraps.
#[derive(Clone, Copy, Debug)]
enum Builtin {
    Setpoint {
        thermostat: Thermostat,
        setpoint: f32,
        safe_band: f32,
    },
    Level {
        depletion: Depletion,
        warn_within: u32,
    },
    Surge {
        surge: Surge,
    },
    Monitor,
}

/// The assembled, stateful decision logic of a profile's built-in kind.
///
/// A controller is what a [`Profile`](crate::Profile) turns its
/// [`ControlSpec`](crate::ControlSpec) into: the live loop that maps each reading to
/// a [`Reaction`]. It composes the `pamoja-kit` helpers - a
/// [`Thermostat`](pamoja_kit::Thermostat) for on/off control, a
/// [`Depletion`](pamoja_kit::Depletion) predictor for level alerts, and a
/// [`Surge`](pamoja_kit::Surge) alarm for rapid change - so the same field-tested
/// math drives every profile. The logic is synchronous and
/// hardware-free, so a profile's whole control policy is unit-testable with no
/// devices and no network. It implements [`Policy`] over an `f32` reading and a
/// `bool` command, which is what lets a policy of your own stand in its place.
///
/// # Examples
///
/// ```
/// use pamoja_profile::{Alert, Controller};
///
/// // Hold a fridge near 5 C, alerting if it strays more than 3 C from target.
/// let mut control = Controller::setpoint(5.0, 0.5, true, 3.0);
///
/// let reaction = control.evaluate(9.0); // warm and out of the safe band
/// assert_eq!(reaction.actuator, Some(true));
/// assert!(matches!(reaction.alert, Some(Alert::OutOfRange { .. })));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Controller {
    policy: Builtin,
}

impl Controller {
    /// Builds a controller that holds a reading near a setpoint.
    ///
    /// This is the policy behind "keep a temperature" and "keep the soil watered":
    /// it switches an output on and off around the setpoint and raises an
    /// [`Alert::OutOfRange`] when the reading strays beyond `safe_band`.
    ///
    /// # Arguments
    ///
    /// * `setpoint` - the target reading.
    /// * `hysteresis` - half the deadband width around the setpoint, which stops the
    ///   output chattering at the threshold.
    /// * `cooling` - `true` for an output that switches on above the band (a cooler),
    ///   `false` for one that switches on below it (a heater or an irrigation valve).
    /// * `safe_band` - how far the reading may stray from the setpoint before an
    ///   alert fires.
    ///
    /// # Returns
    ///
    /// A controller whose output starts off.
    pub fn setpoint(setpoint: f32, hysteresis: f32, cooling: bool, safe_band: f32) -> Self {
        let thermostat = if cooling {
            Thermostat::cooling(setpoint, hysteresis)
        } else {
            Thermostat::heating(setpoint, hysteresis)
        };
        Self {
            policy: Builtin::Setpoint {
                thermostat,
                setpoint,
                safe_band,
            },
        }
    }

    /// Builds a controller that warns before a falling level runs out.
    ///
    /// This is the policy behind "warn before a tank runs dry": it watches a level
    /// fall and raises an [`Alert::RunningOut`] once it is estimated to reach `empty`
    /// within `warn_within` more samples.
    ///
    /// # Arguments
    ///
    /// * `empty` - the level treated as empty, such as a dry tank.
    /// * `warn_within` - warn once empty is this many samples away or nearer.
    ///
    /// # Returns
    ///
    /// A controller awaiting its first two readings.
    pub fn level(empty: f32, warn_within: u32) -> Self {
        Self {
            policy: Builtin::Level {
                depletion: Depletion::new(empty),
                warn_within,
            },
        }
    }

    /// Builds a controller that warns when a reading changes too fast.
    ///
    /// This is the policy behind "warn me before it is too late": it watches the
    /// change between samples and raises an [`Alert::ChangingFast`] when a reading
    /// moves more than `limit` per sample in the watched direction, such as a river
    /// level rising into a flash flood.
    ///
    /// # Arguments
    ///
    /// * `rising` - watch a rapid rise (`true`) or a rapid fall (`false`).
    /// * `limit` - the largest safe change per sample.
    ///
    /// # Returns
    ///
    /// A controller awaiting its first reading.
    pub fn surge(rising: bool, limit: f32) -> Self {
        let surge = if rising {
            Surge::rising(limit)
        } else {
            Surge::falling(limit)
        };
        Self {
            policy: Builtin::Surge { surge },
        }
    }

    /// Builds a controller that reports readings without driving an output.
    ///
    /// # Returns
    ///
    /// A controller that never commands an actuator and never alerts.
    pub fn monitor() -> Self {
        Self {
            policy: Builtin::Monitor,
        }
    }

    /// Assembles the controller a manifest's control kind describes.
    ///
    /// A custom kind has no built-in controller: its policy is the code a
    /// [`PolicyRegistry`] resolves it to, so for one this returns
    /// [`monitor`](Controller::monitor), which reports readings and decides nothing.
    ///
    /// # Arguments
    ///
    /// * `spec` - the control policy a profile carries.
    ///
    /// # Returns
    ///
    /// A fresh controller with its control state reset.
    pub fn from_spec(spec: &ControlSpec) -> Self {
        match *spec {
            ControlSpec::Setpoint {
                setpoint,
                hysteresis,
                cooling,
                safe_band,
            } => Controller::setpoint(setpoint, hysteresis, cooling, safe_band),
            ControlSpec::Level { empty, warn_within } => Controller::level(empty, warn_within),
            ControlSpec::Surge { rising, limit } => Controller::surge(rising, limit),
            ControlSpec::Monitor | ControlSpec::Custom { .. } => Controller::monitor(),
        }
    }

    /// Evaluates one reading and returns the action and any alert it calls for.
    ///
    /// # Arguments
    ///
    /// * `reading` - the latest measured value, in real-world units.
    ///
    /// # Returns
    ///
    /// The [`Reaction`] for this reading: the actuator setting (if the profile drives
    /// one) and any alert the reading raised.
    pub fn evaluate(&mut self, reading: f32) -> Reaction {
        match &mut self.policy {
            Builtin::Setpoint {
                thermostat,
                setpoint,
                safe_band,
            } => {
                let on = thermostat.update(reading);
                let alert = if (reading - *setpoint).abs() > *safe_band {
                    Some(Alert::OutOfRange { reading })
                } else {
                    None
                };
                Reaction {
                    actuator: Some(on),
                    alert,
                }
            }
            Builtin::Level {
                depletion,
                warn_within,
            } => {
                let warn_within = *warn_within;
                let alert = depletion
                    .update(reading)
                    .filter(|samples| *samples <= warn_within)
                    .map(|samples| Alert::RunningOut { samples });
                Reaction {
                    actuator: None,
                    alert,
                }
            }
            Builtin::Surge { surge } => {
                let alert = surge
                    .update(reading)
                    .map(|rate| Alert::ChangingFast { rate });
                Reaction {
                    actuator: None,
                    alert,
                }
            }
            Builtin::Monitor => Reaction::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setpoint_switches_the_output_and_flags_excursions() {
        let mut control = Controller::setpoint(5.0, 0.5, true, 3.0);

        // Warm and beyond the safe band: cooler on, excursion flagged.
        let hot = control.evaluate(9.0);
        assert_eq!(hot.actuator, Some(true));
        assert_eq!(hot.alert, Some(Alert::OutOfRange { reading: 9.0 }));

        // Back in range: cooler still on (above the deadband), no alert.
        let warm = control.evaluate(6.0);
        assert_eq!(warm.actuator, Some(true));
        assert_eq!(warm.alert, None);

        // Below the deadband: cooler off, no alert.
        let cold = control.evaluate(4.0);
        assert_eq!(cold.actuator, Some(false));
        assert_eq!(cold.alert, None);
    }

    #[test]
    fn heating_setpoint_switches_on_below_the_band() {
        // An irrigation valve adds water, so it is a "heater" for soil moisture.
        let mut control = Controller::setpoint(35.0, 5.0, false, 25.0);
        assert_eq!(control.evaluate(28.0).actuator, Some(true)); // dry: valve opens
        assert_eq!(control.evaluate(42.0).actuator, Some(false)); // wet: valve closes
    }

    #[test]
    fn level_warns_only_inside_the_window() {
        let mut control = Controller::level(0.0, 3);
        assert_eq!(control.evaluate(10.0).alert, None); // first reading: no rate yet
        assert_eq!(control.evaluate(8.0).alert, None); // 4 samples out: outside window
        assert_eq!(
            control.evaluate(6.0).alert,
            Some(Alert::RunningOut { samples: 3 })
        ); // now within the window
        assert_eq!(control.evaluate(6.0).actuator, None); // never drives an output
    }

    #[test]
    fn surge_warns_on_a_rapid_rise_without_an_output() {
        let mut control = Controller::surge(true, 0.5);
        assert_eq!(control.evaluate(1.0).alert, None); // first reading: no rate yet
        assert_eq!(control.evaluate(1.25).alert, None); // a gentle rise is fine
        let flood = control.evaluate(2.0); // a 0.75 jump: too fast
        assert_eq!(flood.alert, Some(Alert::ChangingFast { rate: 0.75 }));
        assert_eq!(flood.actuator, None); // never drives an output
    }

    #[test]
    fn monitor_is_inert() {
        let mut control = Controller::monitor();
        let reaction = control.evaluate(42.0);
        assert_eq!(reaction, Reaction::default());
    }

    #[test]
    fn a_controller_is_a_policy_over_a_number() {
        let mut policy: BoxedPolicy = Box::new(Controller::setpoint(5.0, 0.5, true, 3.0));
        assert_eq!(policy.evaluate(&9.0).actuator, Some(true));
        assert_eq!(
            Alert::Custom {
                code: "FrostRisk",
                value: 1.0
            }
            .kind(),
            "FrostRisk"
        );
        assert_eq!(Reaction::<u8>::default().actuator, None);
    }

    // A policy over a whole measurement, deciding a duty rather than an on/off.
    struct Fan {
        above: f32,
    }

    impl Policy for Fan {
        type Reading = (f32, f32);
        type Command = u8;

        fn evaluate(&mut self, &(temperature, humidity): &(f32, f32)) -> Reaction<u8> {
            let hot = temperature > self.above;
            Reaction {
                actuator: Some(if hot { 100 } else { 0 }),
                alert: (hot && humidity > 90.0).then_some(Alert::Custom {
                    code: "Muggy",
                    value: humidity,
                }),
            }
        }
    }

    #[test]
    fn a_registry_resolves_built_in_and_custom_kinds() {
        let registry = PolicyRegistry::<(f32, f32), u8>::new().register("fan", |params| {
            let above = params
                .number("above")
                .ok_or(Error::Unsupported("fan needs `above`"))?;
            Ok(Box::new(Fan {
                above: above as f32,
            }) as BoxedPolicy<(f32, f32), u8>)
        });
        assert_eq!(registry.kinds().collect::<Vec<_>>(), ["fan"]);

        let params = Params::new().with("above", 30.0);
        let mut fan = registry.custom("fan", &params).expect("registered");
        assert_eq!(fan.evaluate(&(35.0, 95.0)).actuator, Some(100));
        assert_eq!(
            fan.evaluate(&(35.0, 95.0)).alert.map(Alert::kind),
            Some("Muggy")
        );
        assert!(matches!(
            registry.custom("fan", &Params::new()),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(
            registry.custom("heater", &params),
            Err(Error::Unsupported(_))
        ));

        let scalar = PolicyRegistry::new();
        let mut monitor = scalar
            .resolve(&ControlSpec::Monitor)
            .expect("a built-in kind resolves without a factory");
        assert_eq!(monitor.evaluate(&1.0), Reaction::default());
        let custom = ControlSpec::Custom {
            kind: "heater".to_owned(),
            params: Params::new(),
        };
        assert!(matches!(
            scalar.resolve(&custom),
            Err(Error::Unsupported(_))
        ));
        assert_eq!(format!("{scalar:?}"), "PolicyRegistry { kinds: [] }");
    }
}
