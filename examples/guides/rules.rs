//! The rules guide example; see docs/guides/rules.md.
//!
//! Run: `cargo run -p pamoja-examples --example rules`

use std::error::Error;

use std::sync::{Arc, Mutex};

use pamoja_core::{Actuator, Result};

/// The valve on the other node, recording every switch so the example can count them.
#[derive(Clone, Default)]
struct Valve {
    switches: Arc<Mutex<Vec<bool>>>,
}

impl Actuator for Valve {
    type Command = bool;

    async fn apply(&mut self, open: bool) -> Result<()> {
        self.switches.lock().expect("valve lock").push(open);
        Ok(())
    }
}

/// A rule file that reads one node's topic and drives another node's valve, run by the
/// engine off an in-process broker; then the same file judged by an evaluator, and what
/// goes wrong.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_codec::JsonCodec;
    use pamoja_core::{Receive, Transport};
    use pamoja_kit::Edge;
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    use pamoja_profile::{Action, RuleEngine, Rules};

    // A rule is a file: the topic it watches, the line a reading crosses, the release
    // band that stops it firing over and over, and what to do on the way down and on the
    // way back. Two rules watch one bed here: one waters it when it dries past 30 and
    // stops once it is wetter than 35, and one raises an alarm when it is soaked past 60.
    let file = r#"{ "rules": [
  { "name": "water-when-dry",
    "when": { "topic": "garden/bed-1/moisture", "below": 30.0, "hysteresis": 5.0 },
    "then": [ { "drive": "bed-valve", "on": true },
              { "publish": "garden/bed-1/valve", "payload": "open" } ],
    "otherwise": [ { "drive": "bed-valve", "on": false },
                   { "publish": "garden/bed-1/valve", "payload": "closed" } ] },
  { "name": "flood-alarm",
    "when": { "topic": "garden/bed-1/moisture", "above": 60.0, "hysteresis": 5.0 },
    "then": [ { "publish": "garden/alarm", "payload": "waterlogged" } ] }
] }"#;

    // Three parties on one broker: the node that reads the bed, the engine that holds the
    // valve, and a watcher on the topics the rules publish to.
    let broker = LoopbackBroker::new();
    let mut probe = LoopbackTransport::new(broker.clone());
    let mut watcher = LoopbackTransport::new(broker.clone());
    probe.connect().await?;
    watcher.connect().await?;
    watcher.subscribe("garden/bed-1/valve").await?;
    watcher.subscribe("garden/alarm").await?;

    let valve = Valve::default();
    let mut engine = RuleEngine::new(
        Rules::from_json(file)?,
        LoopbackTransport::new(broker),
        JsonCodec,
    )
    .with_actuator("bed-valve", valve.clone());
    engine.connect().await?;
    println!(
        "watches   {}, and drives {}",
        engine.topics().join(", "),
        engine.actuators().join(", ")
    );

    // The bed dries out, is watered, and floods. A rule fires only as its condition sets
    // or clears, and the readings in between change nothing. At 65 two rules fire on one
    // reading, in the order the file lists them.
    for reading in [42.0f32, 31.0, 28.0, 33.0, 65.0, 50.0] {
        probe
            .send_text("garden/bed-1/moisture", &reading.to_string())
            .await?;
        let fired = engine.step().await?.expect("the link is up");
        if fired.is_empty() {
            println!("{reading:<10}nothing fired");
        }
        for one in &fired {
            let edge = match one.edge {
                Edge::Set => "set",
                Edge::Cleared => "cleared",
            };
            let actions: Vec<String> = one
                .actions
                .iter()
                .map(|action| match action {
                    Action::Drive { actuator, on } => {
                        format!("drive {actuator} {}", if *on { "on" } else { "off" })
                    }
                    Action::Publish { topic, payload } => format!("publish {payload} to {topic}"),
                })
                .collect();
            if actions.is_empty() {
                println!("{reading:<10}{} {edge}, with nothing to do", one.rule);
            } else {
                println!("{reading:<10}{} {edge}: {}", one.rule, actions.join(", "));
            }
        }
    }

    // The watcher heard every message the rules published, in the order they went out.
    let mut heard = Vec::new();
    for _ in 0..3 {
        let message = watcher.recv().await?.expect("a message");
        heard.push(message.text().expect("words").to_owned());
    }
    println!("heard     {}", heard.join(", "));
    let switches = valve.switches.lock().expect("valve lock").clone();
    let state = if switches.last() == Some(&true) {
        "on"
    } else {
        "off"
    };
    println!(
        "valve     switched {} times, and it is {state}",
        switches.len()
    );
    // ANCHOR_END: example

    assert_eq!(heard, ["open", "closed", "waterlogged"]);
    assert_eq!(switches, [true, false]);
    assert_eq!(engine.is_set("water-when-dry"), Some(false));
    assert_eq!(engine.is_set("flood-alarm"), Some(false));

    // ANCHOR: wrong
    use pamoja_profile::RuleEvaluator;

    // A program that moves its own messages hands each reading to an evaluator, the
    // engine's deciding half on its own, and carries out what it says.
    let mut evaluator = RuleEvaluator::new(Rules::from_json(file)?)?;

    // A reading that is not a number, such as the NaN a failed probe reports, is refused
    // on a watched topic rather than leaving every rule as it was with nothing to say why.
    match evaluator.evaluate("garden/bed-1/moisture", f32::NAN) {
        Ok(_) => println!("a reading of NaN was judged, which should never happen"),
        Err(error) => println!("refused   {error}"),
    }

    // A topic no rule watches is not judged at all, so even a NaN there says nothing.
    if evaluator
        .evaluate("garden/bed-2/moisture", f32::NAN)?
        .is_empty()
    {
        println!(
            "ignored   no rule watches garden/bed-2/moisture, so even a NaN there is not judged"
        );
    }

    // A file no engine could run is refused as it loads, with the rule and the reason.
    for edited in [
        file.replace("garden/bed-1/moisture", "garden/+/moisture"),
        file.replace("\"flood-alarm\"", "\"water-when-dry\""),
    ] {
        match Rules::from_json(&edited).and_then(RuleEvaluator::new) {
            Ok(_) => println!("a file no engine could run was accepted, which should never happen"),
            Err(error) => println!("refused   {error}"),
        }
    }

    // With no release band, readings that hover at the line set and clear the rule on
    // every sample, and each edge switches the valve. The band of 5 holds it through them.
    let fires = |text: &str| -> std::result::Result<usize, Box<dyn Error>> {
        let mut judge = RuleEvaluator::new(Rules::from_json(text)?)?;
        let mut count = 0;
        for reading in [29.9, 30.1, 29.8, 30.2] {
            count += judge.evaluate("garden/bed-1/moisture", reading)?.len();
        }
        Ok(count)
    };
    let bare = fires(&file.replace("\"hysteresis\": 5.0", "\"hysteresis\": 0.0"))?;
    let banded = fires(file)?;
    println!(
        "chatter   4 readings hovering at 30 fire the rule {bare} times with no release band, {banded} with a band of 5"
    );
    // ANCHOR_END: wrong

    Ok(())
}
