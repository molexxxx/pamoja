//! The decision logic a profile assembles: turning a reading into a reaction.

use pamoja_kit::{Depletion, Surge, Thermostat};

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
    /// One of `"OutOfRange"`, `"RunningOut"`, or `"ChangingFast"`.
    pub fn kind(self) -> &'static str {
        match self {
            Alert::OutOfRange { .. } => "OutOfRange",
            Alert::RunningOut { .. } => "RunningOut",
            Alert::ChangingFast { .. } => "ChangingFast",
        }
    }
}

/// The outcome of evaluating one reading against a profile's control policy.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Reaction {
    /// The actuator setting this reading calls for, if the profile drives one.
    ///
    /// `Some(true)` switches the output on, `Some(false)` switches it off, and
    /// `None` means the profile observes without driving an output.
    pub actuator: Option<bool>,
    /// An alert, if the reading crossed a profile threshold; `None` otherwise.
    pub alert: Option<Alert>,
}

// The live policy behind a `Controller`. It is private so the controller's public
// surface stays its constructors and `evaluate`, not the kit helpers it wraps.
#[derive(Clone, Copy, Debug)]
enum Policy {
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

/// The assembled, stateful decision logic of a profile.
///
/// A controller is what a [`Profile`](crate::Profile) turns its
/// [`ControlSpec`](crate::ControlSpec) into: the live loop that maps each reading to
/// a [`Reaction`]. It composes the `pamoja-kit` helpers - a
/// [`Thermostat`](pamoja_kit::Thermostat) for on/off control, a
/// [`Depletion`](pamoja_kit::Depletion) predictor for level alerts, and a
/// [`Surge`](pamoja_kit::Surge) alarm for rapid change - so the same field-tested
/// math drives every profile. The logic is synchronous and
/// hardware-free, so a profile's whole control policy is unit-testable with no
/// devices and no network.
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
    policy: Policy,
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
            policy: Policy::Setpoint {
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
            policy: Policy::Level {
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
            policy: Policy::Surge { surge },
        }
    }

    /// Builds a controller that reports readings without driving an output.
    ///
    /// # Returns
    ///
    /// A controller that never commands an actuator and never alerts.
    pub fn monitor() -> Self {
        Self {
            policy: Policy::Monitor,
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
            Policy::Setpoint {
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
            Policy::Level {
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
            Policy::Surge { surge } => {
                let alert = surge
                    .update(reading)
                    .map(|rate| Alert::ChangingFast { rate });
                Reaction {
                    actuator: None,
                    alert,
                }
            }
            Policy::Monitor => Reaction::default(),
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
}
