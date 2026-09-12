//! The device-profile guide example; see docs/guides/profile.md.
//!
//! Run: `cargo run -p pamoja-examples --example profile`

use std::error::Error;

/// A profile shipped as a file rather than as code: what a fleet writes, what the device
/// decides from it, and what comes back out when the device shares it again.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_profile::{Alert, ElementSpec, Presentation, Profile, Viz};

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
    let switched_on = cold.actuator.expect("this profile drives an output");
    let raised = cold.alert.expect("a reading outside the safe band");
    println!("at 27.5 C: lamp {switched_on}, alert {}", raised.kind());

    // Back inside the deadband the lamp is left as it was, and nothing is raised.
    let settled = profile.controller().evaluate(32.2);
    let still_on = settled.actuator.expect("this profile drives an output");
    let quiet = settled.alert.map_or("none", Alert::kind);
    println!("at 32.2 C: lamp {still_on}, alert {quiet}");

    // Serializing writes the defaulted fields out in full, so a profile edited on a device
    // and shared back carries no value the next reader has to infer.
    let shared = profile.to_json().expect("a serializable profile");
    println!(
        "shared form names its defaults: {}",
        shared.contains("saver_below")
    );

    // The manifest also carries how a dashboard draws the node: one element here, the
    // brooder's temperature as a thermometer with the band the chicks are safe in.
    let drawn = profile.clone().with_presentation(
        Presentation::new().with_element(
            ElementSpec::new(
                "brooder_temperature",
                "celsius",
                "Brooder temperature",
                Viz::Thermometer,
            )
            .with_band(28.0, 36.0),
        ),
    );
    let element = &drawn
        .presentation
        .as_ref()
        .expect("a declared presentation")
        .elements[0];
    let [low, high] = element.band.expect("a band");
    println!(
        "draws {} in {} with a safe band of {low} to {high}",
        element.key, element.unit
    );
    // ANCHOR_END: example

    assert_eq!(cold.actuator, Some(true));
    assert_eq!(cold.alert, Some(Alert::OutOfRange { reading: 27.5 }));
    assert_eq!(settled.alert, None);
    assert_eq!(profile.power.active_secs, 120);
    assert_eq!(profile.power.saver_below, 0.5);
    assert!(shared.contains("\"saver_below\""));
    assert_eq!(Profile::from_json(&shared).expect("valid JSON"), profile);
    assert_eq!(element.viz, Viz::Thermometer);
    assert_eq!(element.band, Some([28.0, 36.0]));
    assert!(drawn
        .to_json()
        .unwrap()
        .contains("\"viz\": \"thermometer\""));

    Ok(())
}
