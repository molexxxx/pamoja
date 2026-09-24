//! The device-profile guide example; see docs/guides/profile.md.
//!
//! Run: `cargo run -p pamoja-examples --example profile`

use std::error::Error;

/// A profile shipped as a file rather than as code: what the manifest says, what a node
/// decides from it as a morning's readings arrive, and how often it samples as its
/// battery drains. Then the other built-in policies on the shipped presets, and what goes
/// wrong.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_profile::{ElementSpec, Presentation, Profile, Viz};

    // A profile is plain data, so a fleet ships one as a file rather than as code. This
    // manifest names no battery thresholds, so the documented defaults apply.
    let manifest = r#"{
        "name": "brooder-heater",
        "topic": "poultry/brooder/temperature",
        "control": {
            "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
            "cooling": false, "safe_band": 4.0
        },
        "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
    }"#;
    let profile = Profile::from_json(manifest)?;
    println!("profile   {} reports on {}", profile.name, profile.topic);
    println!(
        "defaults  the file names no battery thresholds, so saver starts below {:.0}% and critical below {:.0}%",
        profile.power.saver_below * 100.0,
        profile.power.critical_below * 100.0
    );

    // The schedule becomes a power plan, which says what mode a charge puts the node in
    // and how long it waits between samples there.
    let plan = profile.power.plan();
    for charge in [0.8, 0.3, 0.1] {
        println!(
            "battery   at {:.0}% it runs {:?} and samples every {} s",
            charge * 100.0,
            plan.mode(charge),
            plan.interval(charge).as_secs()
        );
    }

    // One controller runs for the life of the node, because it remembers whether the
    // lamp is on. The lamp switches on at 31.5 C or below and off at 32.5 C or above, the
    // setpoint less and plus the hysteresis, and in between it stays as it was. A reading
    // more than 4 C from the setpoint raises an alert as well.
    let mut controller = profile.controller();
    let mut lamp = false;
    for reading in [27.5, 31.8, 32.6, 32.1, 31.4] {
        let reaction = controller.evaluate(reading);
        let on = reaction.actuator.expect("this profile drives a lamp");
        let change = match (lamp, on) {
            (false, true) => "lamp on",
            (true, false) => "lamp off",
            (true, true) => "lamp stays on",
            (false, false) => "lamp stays off",
        };
        let alert = reaction
            .alert
            .map(|alert| format!(", alert {}", alert.kind()))
            .unwrap_or_default();
        println!("{:<10}{change}{alert}", format!("{reading} C"));
        lamp = on;
    }

    // Written back out, the manifest names the thresholds the file left to their
    // defaults, so the next reader has nothing to infer, and it loads as the same profile.
    let shared = profile.to_json()?;
    if shared.contains("saver_below") && Profile::from_json(&shared)?.to_json()? == shared {
        println!("shared    written back out, it names saver_below and loads as the same profile");
    }

    // The manifest also carries how a dashboard draws the node: one element here, the
    // brooder's temperature on a thermometer with the band the chicks are safe in.
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
        "draws     {} in {} on a {}, safe from {low} to {high}",
        element.key,
        element.unit,
        element.viz.name()
    );
    // ANCHOR_END: example

    assert!(lamp, "the morning ends with the lamp on");
    assert_eq!(profile.power.saver_below, 0.5);
    drawn.check()?;

    // ANCHOR: kinds
    use pamoja_profile::Alert;

    // A level warns before a tank or a well runs dry. The shipped well profile counts
    // 0.5 m as dry and warns once the last fall puts dry six samples away or nearer.
    let mut well = Profile::well_level().controller();
    for depth in [5.0, 4.4, 3.8] {
        match well.evaluate(depth).alert {
            Some(Alert::RunningOut { samples }) => {
                println!("well      {depth} m: dry in {samples} samples at this rate, RunningOut")
            }
            _ => println!("well      {depth} m: no warning yet"),
        }
    }

    // A surge warns when a reading moves too far in one sample. The shipped flood sensor
    // warns when a river rises more than 0.3 m between two readings.
    let mut river = Profile::flood_sensor().controller();
    for gauge in [1.2, 1.35, 1.9] {
        match river.evaluate(gauge).alert {
            Some(Alert::ChangingFast { rate }) => {
                println!("river     {gauge} m: up {rate:.2} m in one sample, ChangingFast")
            }
            _ => println!("river     {gauge} m: no warning"),
        }
    }
    // ANCHOR_END: kinds

    // ANCHOR: wrong
    use pamoja_profile::ControlSpec;

    // A probe that fails reports a reading that is not a number. The controller raises
    // it rather than going quiet, and the lamp holds its state; what off means for the
    // chicks is the node's call.
    let failed = controller.evaluate(f32::NAN);
    if let Some(alert) = failed.alert {
        let holds = if failed.actuator == Some(true) {
            "on"
        } else {
            "off"
        };
        println!(
            "probe     a reading of NaN raises {}, and the lamp holds {holds}",
            alert.kind()
        );
    }

    // A controller built again for each reading forgets the lamp was on, so inside the
    // deadband it switches the lamp off.
    let first = profile.controller().evaluate(27.5).actuator;
    let then = profile.controller().evaluate(31.8).actuator;
    if first == Some(true) && then == Some(false) {
        println!(
            "fresh     built again for each reading, the controller turns the lamp off at 31.8 C"
        );
    }

    // A manifest no node could run is refused as it loads, with the reason.
    for edited in [
        manifest.replace("\"hysteresis\": 0.5", "\"hysteresis\": 0.0"),
        manifest.replace("\"saver_secs\": 600", "\"saver_secs\": 60"),
    ] {
        match Profile::from_json(&edited) {
            Ok(_) => {
                println!("a manifest no node could run was accepted, which should never happen")
            }
            Err(error) => println!("refused   {error}"),
        }
    }

    // A misspelled optional field is not an error: it names no field, so the default
    // stays. Writing the profile back out shows what the node understood.
    let misspelled = manifest.replace(
        "\"critical_secs\": 1800 }",
        "\"critical_secs\": 1800, \"saver_bellow\": 0.3 }",
    );
    let understood = Profile::from_json(&misspelled)?;
    println!(
        "typo      saver_bellow names no field, so saver still starts below {:.0}%",
        understood.power.saver_below * 100.0
    );

    // A kind the library does not ship loads with its parameters and runs as a monitor
    // until the node supplies the policy, so it drives nothing and raises nothing.
    let custom = Profile::from_json(
        &manifest.replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""),
    )?;
    if let ControlSpec::Custom { kind, params } = &custom.control {
        let reaction = custom.controller().evaluate(27.5);
        if reaction.actuator.is_none() && reaction.alert.is_none() {
            println!(
                "custom    {kind} loads with {} parameters, and with no policy behind it drives nothing",
                params.len()
            );
        }
    }
    // ANCHOR_END: wrong

    Ok(())
}
