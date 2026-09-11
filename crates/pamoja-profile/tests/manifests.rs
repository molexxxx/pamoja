//! Every manifest in `profiles/` assembled into a running node and driven through a
//! scenario, so a shipped profile is proved to work rather than only to parse.
//!
//! `cargo xtask profiles --check` reads the same files and checks their shape. This
//! goes the other way: it loads each one with the parser a device uses, builds the
//! controller the manifest names, feeds it readings that cross the profile's own
//! lines, and asserts the node reacts the way the manifest's description promises.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use pamoja_codec::{CborCodec, Codec};
use pamoja_core::{Actuator, Receive, Result, Sensor, Transport};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_power::PowerMode;
use pamoja_profile::{Alert, ControlSpec, Node, Profile};

/// A probe that answers a fixed list of readings and then repeats the last one.
struct Replay {
    readings: Vec<f32>,
    at: usize,
}

impl Replay {
    fn new(readings: impl IntoIterator<Item = f32>) -> Self {
        Self {
            readings: readings.into_iter().collect(),
            at: 0,
        }
    }
}

impl Sensor for Replay {
    type Reading = f32;

    async fn read(&mut self) -> Result<f32> {
        let reading = self.readings[self.at.min(self.readings.len() - 1)];
        self.at += 1;
        Ok(reading)
    }
}

/// An output that remembers every command it was given.
#[derive(Default)]
struct Log(Vec<bool>);

impl Actuator for &mut Log {
    type Command = bool;

    async fn apply(&mut self, command: bool) -> Result<()> {
        self.0.push(command);
        Ok(())
    }
}

/// Returns the directory the shared manifests live in.
fn profiles_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles")
}

/// Loads every `profiles/*.json` as its file stem and the profile it parses to.
fn shipped() -> Vec<(String, Profile)> {
    let mut loaded = Vec::new();
    for entry in fs::read_dir(profiles_dir()).expect("the profiles directory is readable") {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let stem = path
            .file_stem()
            .expect("a named file")
            .to_string_lossy()
            .into_owned();
        let text = fs::read_to_string(&path).expect("a readable manifest");
        let profile = Profile::from_json(&text)
            .unwrap_or_else(|err| panic!("{}.json does not parse: {err}", stem));
        loaded.push((stem, profile));
    }
    loaded.sort_by(|left, right| left.0.cmp(&right.0));
    assert!(!loaded.is_empty(), "the repository ships manifests");
    loaded
}

/// Readings that cross the lines the control spec draws, in the order a node sees them.
///
/// A setpoint gets a run well below its band and then well above it, so the output has
/// to switch both ways and the safe band has to be broken in both directions. A level
/// gets a steady fall toward empty. A surge gets one step larger than its limit, in the
/// direction it watches. A monitor gets a flat run, since it decides nothing.
fn scenario(control: &ControlSpec) -> Vec<f32> {
    match control {
        ControlSpec::Setpoint {
            setpoint,
            safe_band,
            ..
        } => {
            let low = setpoint - safe_band - 1.0;
            let high = setpoint + safe_band + 1.0;
            vec![*setpoint, low, low, *setpoint, high, high, *setpoint]
        }
        ControlSpec::Level { empty, warn_within } => {
            let steps = *warn_within as f32 + 2.0;
            (0..8)
                .map(|step| empty + steps - step as f32 * (steps / 7.0))
                .collect()
        }
        ControlSpec::Surge { rising, limit } => {
            let step = limit * 3.0;
            let base = 10.0f32.max(limit * 10.0);
            if *rising {
                vec![base, base, base + step, base + step]
            } else {
                vec![base, base, base - step, base - step]
            }
        }
        ControlSpec::Monitor => vec![1.0, 1.0, 1.0],
        ControlSpec::Custom { .. } => vec![0.0],
    }
}

/// Runs one profile as a node and returns what the output did, what was published, and
/// every alert raised, in order.
async fn run(profile: Profile) -> (Vec<bool>, Vec<f32>, Vec<Alert>) {
    let broker = LoopbackBroker::new();
    let mut gateway = LoopbackTransport::new(broker.clone());
    let mut link = LoopbackTransport::new(broker);
    gateway.connect().await.expect("the gateway connects");
    link.connect().await.expect("the node connects");
    gateway
        .subscribe(&profile.topic)
        .await
        .expect("the gateway subscribes");

    let readings = scenario(&profile.control);
    let mut log = Log::default();
    let mut alerts = Vec::new();
    let mut published = Vec::new();
    {
        let mut node = Node::new(
            profile,
            Replay::new(readings.clone()),
            &mut log,
            link,
            CborCodec,
        );
        for _ in 0..readings.len() {
            let reaction = node.tick().await.expect("the node ticks");
            if let Some(alert) = reaction.alert {
                alerts.push(alert);
            }
            let message = gateway
                .recv()
                .await
                .expect("the gateway reads")
                .expect("a reading arrives");
            let reading: f32 = CborCodec
                .decode(&message.payload)
                .expect("the reading decodes");
            published.push(reading);
        }
    }
    (log.0, published, alerts)
}

#[tokio::test]
async fn every_shipped_manifest_assembles_into_a_node_that_runs() {
    for (stem, profile) in shipped() {
        assert_eq!(stem, profile.name, "a manifest is named after its file");
        assert!(
            profile.description.is_some(),
            "{stem} explains itself to anyone reading the catalog"
        );

        let control = profile.control.clone();
        let topic = profile.topic.clone();
        let (commands, published, alerts) = run(profile).await;

        let sent = scenario(&control);
        assert_eq!(
            published, sent,
            "{stem} publishes every reading it took to {topic}"
        );

        match control {
            ControlSpec::Setpoint { cooling, .. } => {
                assert_eq!(
                    commands.len(),
                    sent.len(),
                    "{stem} drives its output on every reading"
                );
                // Cold and hot are on opposite sides of the band, so whichever way the
                // output is wired it runs for one of them and rests for the other.
                let (cold, hot) = (commands[2], commands[5]);
                assert_ne!(cold, hot, "{stem} switches its output both ways");
                assert_eq!(cold, !cooling, "{stem} runs a heater cold, a cooler hot");
                assert!(
                    alerts
                        .iter()
                        .filter(|alert| matches!(alert, Alert::OutOfRange { .. }))
                        .count()
                        >= 2,
                    "{stem} alerts on both sides of its safe band"
                );
            }
            ControlSpec::Level { warn_within, .. } => {
                let warned = alerts.iter().find_map(|alert| match alert {
                    Alert::RunningOut { samples } => Some(*samples),
                    _ => None,
                });
                let samples =
                    warned.unwrap_or_else(|| panic!("{stem} warns before the level reaches empty"));
                assert!(
                    samples <= warn_within,
                    "{stem} warns inside its own window of {warn_within} samples"
                );
                assert!(
                    commands.is_empty() || commands.iter().all(|&on| !on),
                    "{stem} watches without driving an output"
                );
            }
            ControlSpec::Surge { limit, .. } => {
                let rate = alerts
                    .iter()
                    .find_map(|alert| match alert {
                        Alert::ChangingFast { rate } => Some(*rate),
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("{stem} warns on a step past its limit"));
                assert!(rate > limit, "{stem} reports the change it objected to");
                assert_eq!(
                    alerts.len(),
                    1,
                    "{stem} warns once for the step, not for holding there"
                );
            }
            ControlSpec::Monitor => {
                assert!(alerts.is_empty(), "{stem} raises nothing");
                assert!(commands.iter().all(|&on| !on), "{stem} drives nothing");
            }
            ControlSpec::Custom { kind, .. } => {
                panic!("{stem} names the unregistered kind `{kind}`")
            }
        }
    }
}

#[tokio::test]
async fn every_shipped_manifest_slows_down_as_its_battery_drains() {
    for (stem, profile) in shipped() {
        let power = profile.power;
        let node = Node::monitor(profile, (), (), ());

        for (soc, mode, secs) in [
            (1.0, PowerMode::Active, power.active_secs),
            (power.saver_below - 0.01, PowerMode::Saver, power.saver_secs),
            (
                power.critical_below - 0.01,
                PowerMode::Critical,
                power.critical_secs,
            ),
        ] {
            let (at, wait) = node.schedule(soc, false);
            assert_eq!(at, mode, "{stem} at {soc} charge");
            assert_eq!(wait, Duration::from_secs(secs), "{stem} at {soc} charge");
        }

        // A panel putting charge back in earns the next cadence up, so a node that is
        // low but charging samples like one that is merely conserving.
        let (charging, _) = node.schedule(power.critical_below - 0.01, true);
        assert_eq!(
            charging,
            PowerMode::Saver,
            "{stem} eases off while charging"
        );
    }
}

#[test]
fn every_shipped_manifest_round_trips_through_the_parser() {
    for (stem, profile) in shipped() {
        let written = profile.to_json().expect("a profile serializes");
        let read = Profile::from_json(&written).expect("and parses back");
        assert_eq!(read, profile, "{stem} survives a round trip");
    }
}
