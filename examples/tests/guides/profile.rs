//! The device-profile guide example; see docs/guides/profile.md.

/// A profile shipped as a file rather than as code: what a fleet writes, what the device
/// decides from it, and what comes back out when the device shares it again.
#[test]
fn a_profile_carried_as_a_manifest() {
    // ANCHOR: example
    use pamoja_profile::{Alert, Profile};

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
    println!("{} reports on {}", profile.name, profile.topic);
    println!(
        "wakes every {}s while the battery is healthy",
        profile.power.active_secs
    );
    println!(
        "saver mode below {:.0}% charge",
        profile.power.saver_below * 100.0
    );

    // The manifest is the whole control loop. At 27.5 C the reading is below the deadband,
    // so the lamp switches on, and it is more than 4 C from target, so the chicks are cold.
    let cold = profile.controller().evaluate(27.5);
    println!(
        "at 27.5 C: lamp {:?}, alert {:?}",
        cold.actuator, cold.alert
    );

    // Back inside the deadband the lamp is left as it was, and nothing is raised.
    let settled = profile.controller().evaluate(32.2);
    println!(
        "at 32.2 C: lamp {:?}, alert {:?}",
        settled.actuator, settled.alert
    );

    // Serializing writes the defaulted fields out in full, so a profile edited on a device
    // and shared back carries no value the next reader has to infer.
    let shared = profile.to_json().expect("a serializable profile");
    println!(
        "shared form names its defaults: {}",
        shared.contains("saver_below")
    );
    // ANCHOR_END: example

    assert_eq!(cold.actuator, Some(true));
    assert_eq!(cold.alert, Some(Alert::OutOfRange { reading: 27.5 }));
    assert_eq!(settled.alert, None);
    assert_eq!(profile.power.active_secs, 120);
    assert_eq!(profile.power.saver_below, 0.5);
    assert!(shared.contains("\"saver_below\""));
    assert_eq!(Profile::from_json(&shared).expect("valid JSON"), profile);
}
