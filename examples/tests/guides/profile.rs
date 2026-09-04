//! The device-profile guide example; see docs/guides/profile.md.

/// A profile loaded from the manifest a fleet ships, checked field by field, run against a
/// reading, and written back out, so the manifest shape and the fields it may omit are both
/// pinned rather than round-tripped against themselves.
#[test]
fn a_profile_carried_as_a_manifest() {
    // ANCHOR: example
    use pamoja_profile::{Alert, ControlSpec, Profile};

    // A profile is plain data, so a fleet ships one as a file rather than as code. The two
    // power thresholds are optional and fall back to the documented defaults.
    let manifest = r#"{
        "name": "brooder-heater",
        "topic": "poultry/brooder/temperature",
        "control": {
            "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
            "cooling": false, "safe_band": 4.0
        },
        "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
    }"#;

    let profile = Profile::from_json(manifest).expect("a well-formed manifest");
    assert_eq!(profile.name, "brooder-heater");
    assert_eq!(profile.topic, "poultry/brooder/temperature");
    assert_eq!(
        profile.control,
        ControlSpec::Setpoint {
            setpoint: 32.0,
            hysteresis: 0.5,
            cooling: false,
            safe_band: 4.0
        }
    );
    assert_eq!(profile.power.active_secs, 120);
    assert_eq!(profile.power.saver_below, 0.5);

    // The manifest is the whole control loop. At 27.5 C the reading is below the deadband,
    // so the lamp switches on, and it is more than 4 C from target, so the chicks are cold.
    let reaction = profile.controller().evaluate(27.5);
    assert_eq!(reaction.actuator, Some(true));
    assert_eq!(reaction.alert, Some(Alert::OutOfRange { reading: 27.5 }));

    // Serializing writes the defaulted fields out in full, so a profile edited on a device
    // and shared back carries no value the next reader has to infer.
    let shared = profile.to_json().expect("a serializable profile");
    assert!(shared.contains("\"saver_below\""));
    assert_eq!(Profile::from_json(&shared).expect("valid JSON"), profile);
    // ANCHOR_END: example
}
