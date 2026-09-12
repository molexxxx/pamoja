//! A part pamoja has never heard of: a soil probe and a valve of the maker's own, given the
//! two core traits, then run against a rule and published with nothing plugged in.
//!
//! Run: `cargo run -p pamoja-examples --example device`

use std::error::Error;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    a_homemade_probe_waters_a_bed_and_reports().await?;
    the_same_parts_run_under_a_profile().await?;
    Ok(())
}

// ANCHOR: parts
use pamoja_core::{Actuator, Result, Sensor};
use pamoja_kit::Calibration;

/// A capacitive soil probe on an analog-to-digital converter. Whatever reads the chip is
/// `adc`: any sensor that hands back counts, so a replay stands in for it here and the
/// converter's own driver does on the node. The probe turns counts into percent, and
/// nothing downstream needs to know there was a chip at all.
struct SoilProbe<A> {
    adc: A,
    calibration: Calibration,
}

impl<A: Sensor<Reading = f32> + Send> Sensor for SoilProbe<A> {
    type Reading = f32;

    async fn read(&mut self) -> Result<f32> {
        let counts = self.adc.read().await?;
        Ok(self.calibration.apply(counts))
    }
}

/// A solenoid valve on a relay. It keeps what it was last told and counts the changes,
/// which is what a test needs and what a real one does before it drives the pin.
#[derive(Default)]
struct Valve {
    open: bool,
    switched: u32,
}

impl Actuator for Valve {
    type Command = bool;

    async fn apply(&mut self, open: bool) -> Result<()> {
        if open != self.open {
            self.open = open;
            self.switched += 1;
        }
        Ok(())
    }
}
// ANCHOR_END: parts

/// The whole of a node's own code: read, decide, act, publish, and keep going through an
/// outage.
async fn a_homemade_probe_waters_a_bed_and_reports() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_core::{Receive, Transport};
    use pamoja_kit::Thermostat;
    use pamoja_ladder::TransportLadder;
    use pamoja_loopback::{Faulty, LoopbackBroker, LoopbackTransport};
    use pamoja_sim::Replay;
    use pamoja_sync::MemoryStore;

    let topic = "garden/bed-1/moisture";

    // Bone dry read 3200 counts on this probe and a soaked pot read 1400, measured once
    // and kept. The replay hands back the counts a bed reads as it dries and is watered.
    let counts = Replay::new(vec![2900.0, 2700.0, 2300.0, 2450.0, 2750.0, 2500.0]);
    let mut probe = SoilProbe {
        adc: counts,
        calibration: Calibration::two_point(3200.0, 0.0, 1400.0, 100.0),
    };
    let mut valve = Valve::default();

    // Water below 30% and stop above 45%. A valve that adds water is what `heating`
    // names, so the band sits at 37.5 with 7.5 either side.
    let mut rule = Thermostat::heating(37.5, 7.5);

    // The link, with its first two sends failing the way a radio does at dusk. The ladder
    // keeps what it could not send and replays it in order once a send goes through.
    let broker = LoopbackBroker::new();
    let mut gateway = LoopbackTransport::new(broker.clone());
    gateway.connect().await?;
    gateway.subscribe(topic).await.expect("the gateway listens");
    let flaky = Faulty::new(LoopbackTransport::new(broker), 2);
    let mut ladder = TransportLadder::new(MemoryStore::new()).rung(flaky);
    ladder.connect().await?;

    // The loop: read, decide, act, publish. Nothing in it knows the probe is homemade.
    for _ in 0..6 {
        let moisture = probe.read().await.expect("a reading");
        let wanted = rule.update(moisture);
        valve.apply(wanted).await.expect("the valve answers");
        let report = format!("{moisture:.1}");
        let payload = report.as_bytes();
        let delivery = ladder.send(topic, payload).await?;
        let state = if valve.open { "open" } else { "closed" };
        println!("bed at {report}%, valve {state}, {delivery:?}");
        if ladder.buffered().await.expect("a count") > 0 {
            let caught_up = ladder.flush().await?;
            if caught_up > 0 {
                println!("link back, {caught_up} readings caught up");
            }
        }
    }
    let switched = valve.switched;
    println!("the valve switched {switched} times");

    // On the gateway, in the order they were read, outage included.
    let mut got = Vec::new();
    for _ in 0..6 {
        let message = gateway.recv().await?.expect("a message");
        got.push(message.text().expect("text").to_owned());
    }
    let readings = got.join(", ");
    println!("gateway got {readings}");
    // ANCHOR_END: example

    assert_eq!(got, ["16.7", "27.8", "50.0", "41.7", "25.0", "38.9"]);
    assert_eq!(switched, 3);
    assert!(valve.open);
    assert_eq!(ladder.buffered().await.expect("a count"), 0);

    Ok(())
}

/// The same two parts handed to a profile, so the rule travels as data a fleet can share.
async fn the_same_parts_run_under_a_profile() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: node
    use pamoja_codec::CborCodec;
    use pamoja_core::Transport;
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    use pamoja_profile::{ControlSpec, Node, PowerSchedule, Profile};
    use pamoja_sim::Replay;

    // The same band as a manifest rather than a line of code, with an alert once the bed
    // is more than 15 points from target. No preset is involved: this is the maker's own
    // profile, and it saves to JSON the same as a shipped one.
    let profile = Profile {
        name: "raised-bed-drip".to_owned(),
        description: None,
        topic: "garden/bed-1/moisture".to_owned(),
        control: ControlSpec::Setpoint {
            setpoint: 37.5,
            hysteresis: 7.5,
            cooling: false,
            safe_band: 15.0,
        },
        power: PowerSchedule::new(300, 1800, 3600),
        presentation: None,
    };
    let probe = SoilProbe {
        adc: Replay::new(vec![2900.0, 2300.0]),
        calibration: Calibration::two_point(3200.0, 0.0, 1400.0, 100.0),
    };
    let mut link = LoopbackTransport::new(LoopbackBroker::new());
    link.connect().await.expect("the link connects");

    // A node reads, decides, drives the valve, and publishes on every tick. The parts are
    // the ones above; only the loop moved into the library.
    let mut node = Node::new(profile, probe, Valve::default(), link, CborCodec);
    let mut reactions = Vec::new();
    for _ in 0..2 {
        let reaction = node.tick().await.expect("a tick");
        let valve = match reaction.actuator {
            Some(true) => "open",
            Some(false) => "closed",
            None => "untouched",
        };
        let alert = reaction.alert.map_or("none", |alert| alert.kind());
        println!("under the profile: valve {valve}, alert {alert}");
        reactions.push(reaction);
    }
    // ANCHOR_END: node

    assert_eq!(reactions[0].actuator, Some(true));
    assert_eq!(
        reactions[0].alert.map(|alert| alert.kind()),
        Some("OutOfRange")
    );
    assert_eq!(reactions[1].actuator, Some(false));
    assert!(reactions[1].alert.is_none());

    Ok(())
}
