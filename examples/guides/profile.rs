//! The device-profile guide example; see docs/guides/profile.md.
//!
//! Run: `cargo run -p pamoja-examples --example profile`

use std::error::Error;
use std::sync::{Arc, Mutex};

use pamoja_core::{Actuator, Result, Sensor};

/// A morning of brooder temperatures, standing in for the probe; it ends when the morning
/// does.
struct Morning(Vec<f32>);

impl Sensor for Morning {
    type Reading = f32;

    async fn read(&mut self) -> Result<f32> {
        if self.0.is_empty() {
            Err(pamoja_core::Error::Closed)
        } else {
            Ok(self.0.remove(0))
        }
    }
}

/// The heat lamp, remembering whether it is on.
#[derive(Clone, Default)]
struct Lamp(Arc<Mutex<bool>>);

impl Actuator for Lamp {
    type Command = bool;

    async fn apply(&mut self, on: bool) -> Result<()> {
        *self.0.lock().expect("lamp lock") = on;
        Ok(())
    }
}

/// A profile shipped as a file, run as a node over a morning of readings; then the other
/// built-in policies, a control kind of the program's own, and what goes wrong.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_codec::{Codec, JsonCodec};
    use pamoja_core::{Receive, Transport};
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    use pamoja_profile::{Node, Profile};

    // A profile is a file. This one ships in the catalog under profiles/: it holds a
    // brooder at 32 C by switching a heat lamp, says what it reads, and says how a
    // dashboard draws it.
    let text = std::fs::read_to_string("profiles/brooder-heater.json")?;
    let profile = Profile::from_json(&text)?;
    let reads = profile
        .reads
        .clone()
        .expect("a catalog profile says what it reads");
    println!(
        "profile   {} reads {} in {} and reports on {}",
        profile.name, reads.quantity, reads.unit, profile.topic
    );
    let topic = profile.topic.clone();
    let element = profile
        .presentation
        .clone()
        .expect("a presentation")
        .elements[0]
        .clone();

    // A node is the profile and the parts that make it run: a sensor, an output, and a
    // link. A morning of readings stands in for the probe, and a dashboard listens on the
    // same broker.
    let broker = LoopbackBroker::new();
    let mut link = LoopbackTransport::new(broker.clone());
    let mut dashboard = LoopbackTransport::new(broker);
    link.connect().await?;
    dashboard.connect().await?;
    dashboard.subscribe(&topic).await?;
    let morning = Morning(vec![27.5, 31.8, 32.6, 32.1, 31.4]);
    let mut node = Node::new(profile, morning, Lamp::default(), link, JsonCodec)?;

    // Each tick reads, decides, switches the lamp, and publishes the reading. The lamp
    // comes on at 31.5 C or below and goes off at 32.5 C or above, and in between it stays
    // as it was; a reading more than 4 C from 32 raises an alert as well.
    let mut was = false;
    for _ in 0..5 {
        let tick = node.tick().await?;
        let on = tick.reaction.actuator == Some(true);
        let change = match (was, on) {
            (false, true) => "lamp on",
            (true, false) => "lamp off",
            (true, true) => "lamp stays on",
            (false, false) => "lamp stays off",
        };
        let alert = tick
            .reaction
            .alert
            .map(|alert| format!(", alert {}", alert.kind()))
            .unwrap_or_default();
        println!("{:<10}{change}{alert}", format!("{} C", tick.reading));
        was = on;
    }

    // The dashboard heard every reading the node published.
    let mut heard = Vec::new();
    for _ in 0..5 {
        let message = dashboard.recv().await?.expect("a reading");
        let reading: f32 = JsonCodec.decode(&message.payload)?;
        heard.push(reading.to_string());
    }
    println!("heard     {} on {topic}", heard.join(", "));

    // Between ticks the node waits as long as its battery allows: often on a healthy
    // charge, sparingly on a low one. run() does this until stopped, waiting each interval.
    for charge in [0.8, 0.3, 0.1] {
        let (mode, wait) = node.schedule(charge, false);
        println!(
            "battery   at {:.0}% it runs {mode:?} and waits {} s",
            charge * 100.0,
            wait.as_secs()
        );
    }

    // The same file says how a dashboard draws the node.
    let [low, high] = element.band.expect("a band");
    println!(
        "draws     {} in {} on a {}, safe from {low} to {high}",
        element.key,
        element.unit,
        element.viz.name()
    );
    // ANCHOR_END: example

    assert!(*node.actuator_mut().0.lock().expect("lamp lock"));
    assert_eq!(heard, ["27.5", "31.8", "32.6", "32.1", "31.4"]);

    // ANCHOR: kinds
    use pamoja_profile::Alert;

    // A level warns before a tank or a well runs dry. The shipped well profile counts
    // 0.5 m as dry and warns once the last fall puts dry six samples away or nearer.
    let mut well = Profile::well_level().controller()?;
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
    let mut river = Profile::flood_sensor().controller()?;
    for gauge in [1.2, 1.35, 1.9] {
        match river.evaluate(gauge).alert {
            Some(Alert::ChangingFast { rate }) => {
                println!("river     {gauge} m: up {rate:.2} m in one sample, ChangingFast")
            }
            _ => println!("river     {gauge} m: no warning"),
        }
    }
    // ANCHOR_END: kinds

    // ANCHOR: custom
    use pamoja_profile::{BoxedPolicy, Params, Policy, PolicyRegistry, Reaction};

    /// A policy of the program's own: the lamp on below the setpoint, and a condition of
    /// its own when the chicks are chilled.
    struct BrooderGuard {
        setpoint: f32,
        chilled_below: f32,
    }

    impl Policy for BrooderGuard {
        type Reading = f32;
        type Command = bool;

        fn evaluate(&mut self, reading: &f32) -> Reaction {
            let chilled = *reading < self.chilled_below;
            Reaction {
                actuator: Some(*reading < self.setpoint),
                alert: chilled.then_some(Alert::Custom {
                    code: "Chilled",
                    value: *reading,
                }),
            }
        }
    }

    // A manifest may name a control kind the library never shipped, with its parameters
    // beside it. The program registers the code that decides it under that name, and the
    // node runs whichever kind the file names.
    let guarded =
        Profile::from_json(&text.replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""))?;
    let registry = PolicyRegistry::new().register("brooder_guard", |params: &Params| {
        let setpoint = params.number("setpoint").unwrap_or(32.0) as f32;
        let band = params.number("safe_band").unwrap_or(4.0) as f32;
        Ok(Box::new(BrooderGuard {
            setpoint,
            chilled_below: setpoint - band,
        }) as BoxedPolicy)
    });
    println!(
        "custom    {} is decided by the program's own code, registered under its name",
        guarded.control.kind()
    );
    let mut link = LoopbackTransport::new(LoopbackBroker::new());
    link.connect().await?;
    let mut node = Node::resolve(
        guarded,
        &registry,
        Morning(vec![27.5]),
        Lamp::default(),
        link,
        JsonCodec,
    )?;
    let tick = node.tick().await?;
    let lamp = if tick.reaction.actuator == Some(true) {
        "on"
    } else {
        "off"
    };
    let alert = tick.reaction.alert.map(Alert::kind).unwrap_or("none");
    println!(
        "{:<10}lamp {lamp}, alert {alert}",
        format!("{} C", tick.reading)
    );
    // ANCHOR_END: custom

    // ANCHOR: network
    use pamoja_security::DeviceIdentity;

    // A profile can arrive over a link as well as from a disk: on an MQTT topic the gateway
    // publishes to, or as the body of an HTTP response. The gateway signs what it sends and
    // each node holds only the gateway's public key, so a profile is checked before it is
    // read, and one from anywhere else never runs.
    let gateway = DeviceIdentity::from_seed(&[7u8; 32]);
    let trusted = gateway.public();
    let broker = LoopbackBroker::new();
    let mut uplink = LoopbackTransport::new(broker.clone());
    let mut downlink = LoopbackTransport::new(broker);
    uplink.connect().await?;
    downlink.connect().await?;
    let fleet = "fleet/brooders/profile";
    downlink.subscribe(fleet).await?;

    uplink
        .send(fleet, &gateway.sign_message(text.as_bytes()))
        .await?;
    let message = downlink.recv().await?.expect("a profile");
    let signed = trusted.verify_message(&message.payload)?;
    let delivered = Profile::from_json(std::str::from_utf8(signed)?)?;
    println!(
        "network   {} arrived on {fleet}, signed by the gateway, and loads",
        delivered.name
    );

    let stranger = DeviceIdentity::from_seed(&[9u8; 32]);
    uplink
        .send(fleet, &stranger.sign_message(text.as_bytes()))
        .await?;
    let message = downlink.recv().await?.expect("a profile");
    if trusted.verify_message(&message.payload).is_err() {
        println!("network   one signed by any other key is refused before it is read");
    }
    // ANCHOR_END: network

    assert_eq!(delivered.to_json()?, Profile::from_json(&text)?.to_json()?);
    assert!(trusted.verify_message(&message.payload).is_err());

    // ANCHOR: wrong
    // A probe that fails reports a reading that is not a number. The controller raises
    // it rather than going quiet, and the lamp holds its state; what off means for the
    // chicks is the node's call.
    let profile = Profile::from_json(&text)?;
    let mut controller = profile.controller()?;
    controller.evaluate(27.5);
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
    let first = profile.controller()?.evaluate(27.5).actuator;
    let then = profile.controller()?.evaluate(31.8).actuator;
    if first == Some(true) && then == Some(false) {
        println!(
            "fresh     built again for each reading, the controller turns the lamp off at 31.8 C"
        );
    }

    // A manifest no node could run is refused as it loads, with the reason. So is a
    // misspelled field, with the one it was probably meant to be and where it sits,
    // rather than leaving the default in its place without a word.
    for edited in [
        text.replace("\"hysteresis\": 0.5", "\"hysteresis\": 0.0"),
        text.replace("\"saver_secs\": 600", "\"saver_secs\": 60"),
        text.replace("\"saver_below\"", "\"saver_bellow\""),
    ] {
        match Profile::from_json(&edited) {
            Ok(_) => {
                println!("a manifest no node could run was accepted, which should never happen")
            }
            Err(error) => println!("refused   {error}"),
        }
    }

    // A kind the library does not ship loads with its parameters, but no built-in
    // controller decides it, so a node without a registry that knows it is refused
    // rather than running one that never switches the lamp.
    let unknown =
        Profile::from_json(&text.replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""))?;
    match unknown.controller() {
        Ok(_) => println!("a custom kind ran without its policy, which should never happen"),
        Err(error) => println!("refused   {error}"),
    }
    // ANCHOR_END: wrong

    Ok(())
}
