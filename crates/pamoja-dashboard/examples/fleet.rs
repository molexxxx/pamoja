//! Every shipped profile running at once, on one dashboard.
//!
//! `gateway.rs` next door is one node done properly. This is the other question: what a
//! district looks like when eight unrelated deployments report into a single console. It
//! reads every manifest in `profiles/`, turns each one into a group with the sensors the
//! profile itself declares, runs each profile's own controller over its own signal, and
//! serves the result. No node is described twice: the group's name, its graphics, its
//! safe bands, its French and Swahili labels, and the wording for the state an actuator
//! sits in all come out of the manifest.
//!
//! The point is that the eight have nothing in common. A vaccine fridge alerts in
//! minutes, a grain store in hours. One holds a setpoint, one watches a level fall, one
//! watches for a step change, one only reports. They are on different radios in
//! different places, and the console still draws them side by side, because a profile
//! carries everything the console needs to know about it.
//!
//! A real district gateway differs in one place: where this drifts a signal to have
//! something to draw, it ticks a `pamoja_profile::Node` per site and reports what came
//! back off the link.
//!
//! Run: `cargo run -p pamoja-dashboard --example fleet`

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use pamoja_dashboard::{
    Assets, Catalog, Command, Fleet, LinkKind, Reading, Sensor, Server, Status,
};
use pamoja_profile::{Alert, ControlSpec, Controller, Profile};

/// The variable a service manager provisions the pairing secret through.
const SECRET_VAR: &str = "PAMOJA_PAIRING_SECRET";

/// Where each profile is deployed: its group name, the radio it reports over, and a
/// position on the map.
///
/// This is the only thing a manifest cannot say, because it is about a place rather than
/// about a kind of node. A district that runs two brooders gives them two entries here
/// and one profile between them.
struct Site {
    /// The manifest name, which is also the file name.
    profile: &'static str,
    /// What to call this deployment on the console.
    name: &'static str,
    /// The radio this site reports over.
    link: LinkKind,
    /// Where it is, so the map can place it.
    at: (f64, f64),
}

/// The eight sites this district runs, one per shipped profile.
const SITES: [Site; 8] = [
    Site {
        profile: "vaccine-fridge-monitor",
        name: "Clinic cold room",
        link: LinkKind::NbIot,
        at: (-1.2921, 36.8219),
    },
    Site {
        profile: "brooder-heater",
        name: "Poultry shed",
        link: LinkKind::Lora,
        at: (-1.3105, 36.8010),
    },
    Site {
        profile: "grain-store-humidity",
        name: "Grain store",
        link: LinkKind::Lora,
        at: (-1.3320, 36.7740),
    },
    Site {
        profile: "irrigation-node",
        name: "North field",
        link: LinkKind::Mesh,
        at: (-1.2700, 36.8500),
    },
    Site {
        profile: "soil-moisture-valve",
        name: "Market garden bed",
        link: LinkKind::Wifi,
        at: (-1.2860, 36.8330),
    },
    Site {
        profile: "well-level",
        name: "Village well",
        link: LinkKind::Lora,
        at: (-1.3480, 36.8120),
    },
    Site {
        profile: "pipeline-pressure-drop",
        name: "Water main",
        link: LinkKind::Cellular,
        at: (-1.3010, 36.8250),
    },
    Site {
        profile: "flood-sensor",
        name: "River gauge",
        link: LinkKind::Satellite,
        at: (-1.3650, 36.7900),
    },
];

/// Reads every manifest in the repository's shared `profiles/` directory.
///
/// A district gateway reads its profiles the same way: from files it was given, with the
/// parser a device uses, so a manifest that would not run on a node does not get drawn
/// on the console either.
fn load_profiles() -> BTreeMap<String, Profile> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let mut loaded = BTreeMap::new();
    for site in &SITES {
        let path = dir.join(format!("{}.json", site.profile));
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
        let profile = Profile::from_json(&text)
            .unwrap_or_else(|err| panic!("{}.json does not parse: {err}", site.profile));
        loaded.insert(site.profile.to_owned(), profile);
    }
    loaded
}

/// Builds the fleet: one group per site, holding the sensors its profile declares.
///
/// The elements come straight off the profile, so adding a ninth profile to `profiles/`
/// and a ninth entry to `SITES` is the whole change; nothing here names a reading.
fn build_fleet(profiles: &BTreeMap<String, Profile>) -> Fleet {
    let mut builder = Fleet::builder().org("district", "Kiambu district");
    for site in &SITES {
        let profile = &profiles[site.profile];
        builder = builder.group("district", site.profile, site.name, site.link);
        for element in profile.presentation.iter().flat_map(|p| &p.elements) {
            let mut reading = Reading::from_element(element);
            // A controllable output gets its two commands, so the console can drive it.
            if let Some(state) = &element.state {
                reading = reading.with_actions(actions_for(state));
            }
            builder = builder.sensor(
                site.profile,
                Sensor::new(&element.key, reading).at(site.at.0, site.at.1),
            );
        }
    }
    builder.build()
}

/// A signal for one site: a plausible walk for whatever its profile measures.
///
/// The shape comes from the profile's own policy and the safe band it drew, so nothing
/// here has to know what a grain store or a water main reads. A setpoint profile wanders
/// around its target and occasionally breaks out; a level falls steadily and refills; a
/// surge profile sits inside its band and then steps out of it; a monitor drifts gently.
fn signal(control: &ControlSpec, band: Option<[f32; 2]>, step: u32) -> f32 {
    let phase = f32::from((step % 3600) as u16) * 0.35;
    let [low, high] = band.unwrap_or([0.0, 100.0]);
    let middle = (low + high) / 2.0;
    match *control {
        ControlSpec::Setpoint {
            setpoint,
            safe_band,
            ..
        } => setpoint + safe_band * 1.15 * phase.sin(),
        ControlSpec::Level { empty, .. } => {
            // Drawn down over twenty samples and then refilled, so the profile's
            // running-out warning gets a real fall to estimate against.
            let fall = (high - empty) / 20.0;
            high - f32::from((step % 20) as u16) * fall
        }
        ControlSpec::Surge { rising, limit } => {
            let jump = if step.is_multiple_of(11) {
                limit * 3.0
            } else {
                0.0
            };
            middle + phase.sin() * limit * 0.2 + if rising { jump } else { -jump }
        }
        ControlSpec::Monitor => middle + (high - low) * 0.35 * phase.sin(),
        ControlSpec::Custom { .. } => middle,
    }
}

/// Turns a reaction's alert into the health the console draws.
fn status_of(alert: Option<Alert>) -> Status {
    match alert {
        Some(Alert::OutOfRange { .. }) => Status::Alarm,
        Some(_) => Status::Warn,
        None => Status::Ok,
    }
}

/// Reports one profile's sample into its group.
///
/// The first non-stat element the profile declares is the one its policy judges. Every
/// other element is reported as it stands, so a store's second thermometer and a well's
/// battery keep their place on the console without the policy knowing about them.
fn report(fleet: &Fleet, site: &Site, profile: &Profile, control: &mut Controller, step: u32) {
    let Some(presentation) = &profile.presentation else {
        return;
    };
    // The judged element's own band shapes the signal, so a pressure reads in bar and a
    // river in meters without this loop knowing either.
    let judged_band = presentation
        .elements
        .iter()
        .find(|element| element.state.is_none() && !element.stat)
        .and_then(|element| element.band);
    let measured = signal(&profile.control, judged_band, step);
    let reaction = control.evaluate(measured);
    let status = status_of(reaction.alert);

    let mut judged = false;
    for element in &presentation.elements {
        // A discrete element is an actuator: it shows the setting the policy just chose.
        if element.state.is_some() {
            let Some(on) = reaction.actuator else {
                continue;
            };
            let state = element
                .state
                .as_deref()
                .map(|start| swapped(start, on))
                .unwrap_or_default();
            let reading = Reading::new(&element.key, if on { 1.0 } else { 0.0 }, &element.unit)
                .with_viz(element.viz)
                .with_actions(actions_for(&state))
                .with_state(state);
            fleet.report_reading(site.profile, &element.key, reading);
            continue;
        }

        // The first measurement is the one the profile's policy judged.
        let value = if judged {
            secondary(element.band, step)
        } else {
            judged = true;
            measured
        };
        let mut reading = Reading::from_element(element);
        reading.value = value;
        // Only the judged reading carries the policy's verdict; a second thermometer in
        // the same store is reported as it stands.
        reading = reading.with_status(if value == measured {
            status
        } else {
            Status::Ok
        });
        fleet.report_reading(site.profile, &element.key, reading);
    }
}

/// The state code an actuator moves to, derived from the one its profile starts in.
///
/// A profile names one state, such as `"state.cooler_off"`, and localizes both halves.
/// Switching it swaps that last word, which keeps the wording the profile chose rather
/// than inventing a code the console has no words for. A valve names the pair the
/// dashboard already ships. Anything else is left alone, since a code with no opposite
/// would only render as a raw key.
fn swapped(start: &str, on: bool) -> String {
    match start {
        "state.open" | "state.closed" => if on { "state.open" } else { "state.closed" }.to_owned(),
        other => match other.rsplit_once('_') {
            Some((head, "on" | "off")) => {
                format!("{head}_{}", if on { "on" } else { "off" })
            }
            _ => other.to_owned(),
        },
    }
}

/// The two words an actuator in this state can be commanded to.
///
/// A valve opens and closes; everything else switches on and off.
fn actions_for(state: &str) -> [&'static str; 2] {
    match state {
        "state.open" | "state.closed" => ["open", "closed"],
        _ => ["on", "off"],
    }
}

/// A gentle walk inside a secondary element's band, for the readings no policy judges.
fn secondary(band: Option<[f32; 2]>, step: u32) -> f32 {
    let [low, high] = band.unwrap_or([0.0, 100.0]);
    let middle = (low + high) / 2.0;
    middle + (high - low) * 0.3 * f32::from((step % 17) as u16).cos()
}

/// Returns the pairing secret to serve with, or `None` to leave control locked.
///
/// A pairing code unlocks control of every node on the console, so this one is only ever
/// given to the process, never generated by it and printed. A district console is read
/// until somebody provisions that secret, which is the right default for a page whose job
/// is to draw a fleet: nothing is written to stdout that would unlock it if the output
/// were captured into a log. `gateway.rs` next door is the other case, one node an
/// operator pairs at a terminal, and it shows a generated code once for exactly that.
fn pairing_secret() -> Option<String> {
    match std::env::var(SECRET_VAR) {
        Ok(provisioned) if !provisioned.is_empty() => Some(provisioned),
        _ => {
            eprintln!("fleet: serving read-only; set {SECRET_VAR} to unlock control.");
            None
        }
    }
}

fn main() -> std::process::ExitCode {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8789".to_owned());

    let profiles = load_profiles();
    let fleet = build_fleet(&profiles);

    // Every element any profile declares is a sensor this district can bind a driver to.
    // Anything else is refused, and the console says the device does not support it.
    let allowed: Vec<String> = profiles
        .values()
        .flat_map(|profile| profile.presentation.iter())
        .flat_map(|presentation| &presentation.elements)
        .map(|element| element.key.clone())
        .collect();
    fleet.allow_sensors(allowed);

    println!("fleet: running {} profiles", profiles.len());
    for site in &SITES {
        let profile = &profiles[site.profile];
        let drawn = profile
            .presentation
            .as_ref()
            .map_or(0, |presentation| presentation.elements.len());
        println!(
            "  {:<24} {:<22} {} on {:?}, {drawn} elements",
            site.profile,
            site.name,
            profile.control.kind(),
            site.link
        );
    }

    // One sampling loop for the whole district. Each site keeps its own controller, so
    // the hysteresis and the running estimates belong to that site and not to the loop.
    let worker = fleet.clone();
    let sampled = profiles.clone();
    thread::spawn(move || {
        let mut controls: BTreeMap<&str, Controller> = SITES
            .iter()
            .map(|site| (site.profile, sampled[site.profile].controller()))
            .collect();
        let mut step = 0u32;
        loop {
            step += 1;
            for site in &SITES {
                let control = controls
                    .get_mut(site.profile)
                    .expect("a controller per site");
                report(&worker, site, &sampled[site.profile], control, step);
            }

            // Whatever the console queued gets applied and reported back, so a switch
            // thrown on the page shows the new state rather than snapping back.
            for command in worker.take_commands() {
                match command {
                    Command::Actuate { target, action } => {
                        println!("fleet: {target} -> {action}");
                    }
                    other => println!("fleet: applying {other:?}"),
                }
            }
            thread::sleep(Duration::from_millis(1200));
        }
    });

    // The catalog is the union of what all eight profiles declare: every element, every
    // localized label, and the wording for every state code they emit. The page fetches
    // it once from `GET /catalog` and can then draw a node it has never seen.
    let all: Vec<&Profile> = profiles.values().collect();
    let mut server =
        Server::new(fleet, Assets::Embedded).with_catalog(Catalog::from_profiles(&all));
    if let Some(secret) = pairing_secret() {
        server = server.with_pairing_secret(secret);
    }

    println!("fleet: serving on http://{addr}");
    match server.run(&addr) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("fleet: could not serve on {addr}: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}
