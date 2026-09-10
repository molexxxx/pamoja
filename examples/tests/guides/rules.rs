//! The rules guide example; see docs/guides/rules.md.

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
/// engine off an in-process broker.
#[tokio::test]
async fn a_rule_file_reads_one_node_and_drives_another() {
    // ANCHOR: example
    use pamoja_codec::JsonCodec;
    use pamoja_core::{Receive, Transport};
    use pamoja_kit::Edge;
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    use pamoja_profile::{Compare, RuleEngine, Rules};

    // A rule is a file: the topic it watches, the line a reading crosses, the release
    // band that stops it firing over and over, and what to do on the way down and on
    // the way back. The same file runs in every language.
    let rules = Rules::from_json(
        r#"{ "rules": [ {
            "name": "water-when-dry",
            "when": { "topic": "garden/bed-1/moisture", "compare": "below",
                      "threshold": 30.0, "hysteresis": 5.0 },
            "then": [ { "do": "drive", "actuator": "bed-valve", "on": true },
                      { "do": "publish", "topic": "garden/bed-1/valve", "payload": "open" } ],
            "otherwise": [ { "do": "drive", "actuator": "bed-valve", "on": false },
                           { "do": "publish", "topic": "garden/bed-1/valve", "payload": "closed" } ]
        } ] }"#,
    )
    .expect("a well-formed rule file");
    let when = &rules.rules[0].when;
    let side = match when.compare {
        Compare::Below => "below",
        Compare::Above => "above",
    };
    let clears = when.threshold + when.hysteresis;
    println!(
        "the rule watches {} {side} {}, clearing above {clears}",
        when.topic, when.threshold
    );

    // Three parties on one broker: the node that reads the bed, the engine that holds
    // the valve, and a watcher on the topic the rule publishes to.
    let broker = LoopbackBroker::new();
    let mut probe = LoopbackTransport::new(broker.clone());
    let mut watcher = LoopbackTransport::new(broker.clone());
    probe.connect().await.expect("the probe connects");
    watcher.connect().await.expect("the watcher connects");
    watcher
        .subscribe("garden/bed-1/valve")
        .await
        .expect("a subscription");

    let valve = Valve::default();
    let mut engine = RuleEngine::new(rules, LoopbackTransport::new(broker), JsonCodec)
        .with_actuator("bed-valve", valve.clone());
    engine.connect().await.expect("the engine connects");

    // The bed dries out and is watered back: the rule fires once on the way down and
    // once on the way back, and holds its state for the readings in between.
    for reading in [42.0f32, 31.0, 28.0, 33.0, 36.0] {
        probe
            .send_text("garden/bed-1/moisture", &reading.to_string())
            .await
            .expect("the probe publishes");
        let fired = engine
            .step()
            .await
            .expect("a step")
            .expect("the link is up");
        let edge = match fired.first().map(|fired| fired.edge) {
            Some(Edge::Set) => "set",
            Some(Edge::Cleared) => "cleared",
            None => "no edge",
        };
        let open = valve.switches.lock().expect("valve lock").last().copied();
        let state = if open == Some(true) { "on" } else { "off" };
        println!("{reading}: {edge}, valve {state}");
    }

    // The watcher on the other topic heard each edge as the rule published it.
    let mut heard = Vec::new();
    for _ in 0..2 {
        let message = watcher.recv().await.expect("recv").expect("a message");
        heard.push(message.text().expect("words").to_owned());
    }
    println!("the watcher heard {}", heard.join(", "));
    let switched = valve.switches.lock().expect("valve lock").len();
    println!("the valve switched {switched} times");
    // ANCHOR_END: example

    assert_eq!(heard, ["open", "closed"]);
    assert_eq!(*valve.switches.lock().unwrap(), [true, false]);
    assert_eq!(engine.is_set("water-when-dry"), Some(false));
}
