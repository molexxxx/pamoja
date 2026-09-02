//! A worked gateway: drive the dashboard from a real pamoja profile, with no mock.
//!
//! This is the shape to copy into a real project. It restores a persisted fleet on boot,
//! assembles a profile's controller, samples a sensor on a loop, surfaces a node when it is
//! discovered, applies the control commands the dashboard queues (honouring any hardware
//! binding), persists changes, and serves the dashboard from that fleet. Swap the stand-in
//! sensor for a real `pamoja-sensors` driver, and (to also publish telemetry upstream) tick
//! the async `pamoja_profile::Node` instead of its controller directly.
//!
//! Run: `cargo run -p pamoja-dashboard --example gateway`

use std::io::IsTerminal;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use pamoja_dashboard::{
    Assets, Auth, Catalog, Command, ElementSpec, Fleet, LinkKind, Presentation, Reading, Scope,
    Sensor, Server, State, StateSource, Status, Trend, Viz,
};
use pamoja_profile::{Alert, Profile};

/// The variable a service manager provisions the pairing secret through.
const SECRET_VAR: &str = "PAMOJA_PAIRING_SECRET";

/// Returns the pairing secret to serve with, or `None` to leave control locked.
///
/// A pairing code unlocks control of the whole fleet. Showing a generated one is
/// right at a terminal, where an operator reads it once, and wrong under a
/// service manager, where stdout is captured into a log that keeps it for anyone
/// who can read logs. So a gateway that cannot show a code safely serves the
/// dashboard read-only instead, which is what a server with no pairing secret
/// already does.
fn pairing_secret() -> Option<String> {
    match std::env::var(SECRET_VAR) {
        Ok(provisioned) if !provisioned.is_empty() => return Some(provisioned),
        _ => {}
    }

    let secret = Auth::generate_secret();
    if std::io::stdout().is_terminal() {
        println!("gateway: pairing code (unlock control with this): {secret}");
        return Some(secret);
    }

    eprintln!("gateway: no terminal to show a pairing code on, so control stays locked.");
    eprintln!("gateway: set {SECRET_VAR} to run under a service manager with control enabled.");
    None
}

// The profile this gateway runs: an irrigation controller, plus a dashboard presentation that
// teaches the page two elements it would not draw by default - a turbidity gauge and a
// dropped-packet node stat. The page fetches these from `GET /catalog` and renders them.
fn profile() -> Profile {
    Profile::irrigation_node().with_presentation(
        Presentation::new()
            .with_element(
                ElementSpec::new("water_turbidity", "ntu", "Turbidity", Viz::Gauge)
                    .with_band(0.0, 5.0),
            )
            .with_element(
                ElementSpec::new("packets_dropped", "count", "Packets dropped", Viz::Count)
                    .as_stat()
                    .on(Scope::Links(vec!["lora".to_owned(), "mesh".to_owned()])),
            )
            .with_element(
                ElementSpec::new("filter_state", "state", "Water filter", Viz::Switch)
                    .with_state("state.clean"),
            )
            // Localize the custom states this profile emits, so the page shows words, not codes.
            .with_message("state.clean", "Clean")
            .with_message("state.flushing", "Flushing"),
    )
}

// Where this example persists its fleet, so provisioning and the last valve state survive a
// restart. A real gateway uses its own durable storage.
fn state_path() -> PathBuf {
    std::env::temp_dir().join("pamoja-gateway-state.json")
}

// Restores a saved fleet if one exists, otherwise builds the initial structure.
fn build_fleet() -> Fleet {
    if let Ok(text) = std::fs::read_to_string(state_path()) {
        if let Ok(state) = State::from_json(&text) {
            println!("gateway: restored fleet from {}", state_path().display());
            return Fleet::from_state(state);
        }
    }
    Fleet::builder()
        .org("farm", "Pamoja farm")
        .group("farm", "field", "Field node", LinkKind::Lora)
        .sensor(
            "field",
            Sensor::new(
                "soil",
                Reading::new("soil_moisture", 60.0, "percent").with_band(40.0, 80.0),
            ),
        )
        .sensor(
            "field",
            Sensor::new(
                "valve",
                Reading::new("drip_valve", 0.0, "state")
                    .with_state("state.closed")
                    .with_actions(["open", "closed"]),
            ),
        )
        // A custom element from the profile's presentation: the page draws it as the gauge the
        // profile chose, with its band and label, because the reading pins the visualization.
        .sensor(
            "field",
            Sensor::new(
                "turbidity",
                Reading::new("water_turbidity", 2.4, "ntu")
                    .with_band(0.0, 5.0)
                    .with_viz(Viz::Gauge),
            ),
        )
        // A discrete sensor whose state code the profile localizes (see its presentation).
        .sensor(
            "field",
            Sensor::new(
                "filter",
                Reading::new("filter_state", 0.0, "state").with_state("state.clean"),
            ),
        )
        .build()
}

fn save(fleet: &mut Fleet) {
    if let Ok(json) = fleet.snapshot().to_json() {
        let _ = std::fs::write(state_path(), json);
    }
}

fn main() -> std::process::ExitCode {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8788".to_owned());

    let prof = profile();
    let fleet = build_fleet();

    // A real device only takes the sensor types it can actually bind a driver to; an add of
    // anything else is refused and the dashboard says the device does not support it. Discovery
    // (Fleet::add_sensor) is the device's own and is never gated.
    fleet.allow_sensors([
        "soil_moisture",
        "drip_valve",
        "water_turbidity",
        "packets_dropped",
        "humidity",
        "filter_state",
        "well_level",
        "battery_voltage",
    ]);

    // The sampling loop. A real project ticks its profile here on the power schedule; this
    // drifts a stand-in soil reading, judges it with the profile's controller, surfaces a
    // discovered node, and applies any control command the dashboard queued.
    let mut worker = fleet.clone();
    let worker_profile = prof.clone();
    thread::spawn(move || {
        let mut control = worker_profile.controller();
        let mut tick = 0.0f32;
        let mut step = 0u32;
        loop {
            step += 1;
            tick += 0.4;
            let moisture = 60.0 + 25.0 * tick.sin();
            let status = match control.evaluate(moisture).alert {
                Some(Alert::OutOfRange { .. }) => Status::Alarm,
                Some(_) => Status::Warn,
                None => Status::Ok,
            };
            worker.report_reading(
                "field",
                "soil",
                Reading::new("soil_moisture", moisture, "percent")
                    .with_band(40.0, 80.0)
                    .with_status(status)
                    .with_trend(Trend::Steady),
            );

            // Drift the custom turbidity probe so its gauge moves; clear water sits low.
            let turbidity = 2.4 + 1.6 * (tick * 0.7).sin();
            worker.report_reading(
                "field",
                "turbidity",
                Reading::new("water_turbidity", turbidity, "ntu")
                    .with_band(0.0, 5.0)
                    .with_viz(Viz::Gauge)
                    .with_status(if turbidity > 4.5 {
                        Status::Warn
                    } else {
                        Status::Ok
                    }),
            );

            // Discovery: a humidity node joins after a few cycles; it appears in the dashboard.
            if step == 8 {
                println!("gateway: a humidity node joined; surfacing it");
                worker.add_sensor(
                    "field",
                    Sensor::new(
                        "humidity",
                        Reading::new("humidity", 55.0, "percent").with_band(30.0, 70.0),
                    ),
                );
                save(&mut worker);
            }

            // Apply control commands the dashboard queued, then persist the change.
            let commands = worker.take_commands();
            if !commands.is_empty() {
                for command in &commands {
                    match command {
                        Command::Actuate { target, action } if target == "field/valve" => {
                            let on = action == "open";
                            worker.report_reading(
                                "field",
                                "valve",
                                Reading::new("drip_valve", if on { 1.0 } else { 0.0 }, "state")
                                    .with_state(format!("state.{action}"))
                                    .with_actions(["open", "closed"]),
                            );
                        }
                        Command::AddSensor {
                            binding: Some(binding),
                            sensor,
                            ..
                        } => println!("gateway: bind sensor {} via {binding}", sensor.id),
                        other => println!("gateway: applying {other:?}"),
                    }
                }
                save(&mut worker);
            }
            thread::sleep(Duration::from_millis(1000));
        }
    });

    let mut server =
        Server::new(fleet, Assets::Embedded).with_catalog(Catalog::from_profiles(&[&prof]));
    if let Some(secret) = pairing_secret() {
        server = server.with_pairing_secret(secret);
    }

    println!("gateway: serving on http://{addr}");
    match server.run(&addr) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("gateway: could not serve on {addr}: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}
