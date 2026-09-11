//! A poultry brooder, built from a shared profile and run through a cold night.
//!
//! This is a whole small deployment rather than one feature. A smallholder keeps
//! day-old chicks in a shed: they need 32 C, they die below about 28, and the shed has
//! a heat lamp on a relay, a vent fan by the door, a solar battery, and a radio uplink
//! that comes and goes. Nobody there writes a control loop. They download
//! `profiles/brooder-heater.json`, which is the same file this example loads, and the
//! toolkit does the rest.
//!
//! What each part is doing:
//!
//! - the manifest names the target, the deadband, the alert range, and how often to
//!   sample as the battery drains, and no code here repeats any of those numbers
//! - a node ties the profile to a probe, a lamp on an active-low relay, a link, and a
//!   wire format, and one `tick` reads, decides, switches, and publishes
//! - the link is a ladder, so a reading nothing can carry is kept and sent later
//!   instead of lost
//! - a rule file at the gateway watches the same topic and opens the vent fan, which is
//!   a second node the brooder knows nothing about
//! - the power schedule stretches the sampling interval as the battery sags
//!
//! Swap the two stand-in parts and this runs a real shed: the probe becomes a driver
//! from `pamoja-sensors` on the board's I2C bus, and the relay pin becomes a pin from
//! the board's GPIO library. Everything between them stays as written.
//!
//! Run with: `cargo run -p pamoja-examples --example brooder_node`

use std::path::PathBuf;

use pamoja_codec::JsonCodec;
use pamoja_core::Result;
use pamoja_gpio::pin::Polarity;
use pamoja_gpio::switch::Switch;
use pamoja_hal::script::PinScript;
use pamoja_kit::Edge;
use pamoja_ladder::TransportLadder;
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_power::PowerMode;
use pamoja_profile::{
    Action, Alert, Condition, ControlSpec, Node, Profile, Rule, RuleEngine, Rules,
};
use pamoja_sim::{DegradedLink, RecordingActuator, Replay};
use pamoja_sync::MemoryStore;

/// One hour of the night: what the probe reads, and what the battery has left.
///
/// The shed cools through the small hours, the lamp fights it, and a door left open
/// near dawn lets the heat out and then bakes the shed once the sun is up. The battery
/// starts the night charged and is nearly flat by morning.
const NIGHT: [(f32, f32); 8] = [
    (32.1, 0.95),
    (31.2, 0.88),
    (30.4, 0.74),
    (29.6, 0.61),
    (27.3, 0.47),
    (26.4, 0.33),
    (30.8, 0.21),
    (34.6, 0.14),
];

/// Returns the path of a manifest in the repository's shared `profiles/` directory.
fn shared_profile(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../profiles")
        .join(format!("{name}.json"))
}

/// The rule the gateway runs, written the way a fleet would share it.
///
/// It watches the brooder's own topic, so the vent fan reacts to a reading published by
/// a node it has no connection to. The release band is what stops the fan chattering
/// around the threshold: it opens above 34 and does not close until 33.
fn vent_rules(topic: &str) -> Rules {
    Rules::new().with(
        Rule::new(
            "vent-when-hot",
            Condition::above(topic, 34.0).with_hysteresis(1.0),
        )
        .then(Action::Drive {
            actuator: "vent-fan".to_owned(),
            on: true,
        })
        .otherwise(Action::Drive {
            actuator: "vent-fan".to_owned(),
            on: false,
        }),
    )
}

/// Renders a count so a printed line reads as a sentence rather than a field.
fn times(count: usize) -> String {
    match count {
        1 => "once".to_owned(),
        2 => "twice".to_owned(),
        other => format!("{other} times"),
    }
}

/// Names the power mode the way the printout reads it.
fn mode_name(mode: PowerMode) -> &'static str {
    match mode {
        PowerMode::Active => "active",
        PowerMode::Saver => "saver",
        PowerMode::Critical => "critical",
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // The profile is a file. Reading it here is exactly what a device does on boot, and
    // it is the same file anyone can download, edit, and share back.
    let text = std::fs::read_to_string(shared_profile("brooder-heater"))
        .expect("the shared manifest is in the repository");
    let profile = Profile::from_json(&text)?;

    println!("== the profile this shed runs");
    println!("{}", profile.description.as_deref().unwrap_or_default());
    println!("publishing to {}", profile.topic);
    println!(
        "sampling every {}s, easing to {}s and then {}s as the battery drains\n",
        profile.power.active_secs, profile.power.saver_secs, profile.power.critical_secs
    );

    // One broker stands in for the radio everything shares.
    let broker = LoopbackBroker::new();

    // The uplink is bad for stretches: two readings get through, the next three do not,
    // over and over. A node that sent and forgot would lose the cold hours entirely.
    let radio = DegradedLink::new(LoopbackTransport::new(broker.clone())).intermittent(2, 3);

    // So the node publishes through a ladder instead of straight down the radio. A
    // ladder tries each rung in turn and, when none will take the message, keeps it in a
    // store to send later. Here there is one rung and a store in memory; on a real node
    // the rungs are wifi, then LoRa, then a cellular modem, and the store is on flash.
    let mut uplink = TransportLadder::new(MemoryStore::new()).rung(radio);
    uplink.connect().await?;

    // The heat lamp. Relay boards almost always energize on a low input, so the polarity
    // is stated once and nothing below it thinks about inversion again. `PinScript`
    // stands in for the board's own pin and remembers every level it was driven to; on a
    // Raspberry Pi this is the only line that changes.
    let lamp = Switch::new(PinScript::new([]), Polarity::ActiveLow);

    // The probe. On a board this is a driver reading a real part.
    let probe = Replay::new(NIGHT.iter().map(|&(reading, _)| reading).collect());

    // That is the whole node: profile, probe, lamp, link, wire format.
    let mut node = Node::new(profile.clone(), probe, lamp, uplink, JsonCodec);

    // The gateway end. The vent fan hangs off the rule engine, not off the node, and the
    // rules are a file the fleet shares the same way it shares the profile.
    let rules = vent_rules(&profile.topic);
    println!("== the rule the gateway runs");
    println!("{}\n", rules.to_json()?);

    let fan = RecordingActuator::new();
    let fan_log = fan.log();
    let mut gateway = RuleEngine::new(rules, LoopbackTransport::new(broker), JsonCodec)
        .with_actuator("vent-fan", fan);
    gateway.connect().await?;

    // The safe range is the manifest's, not this program's, so the printout cannot claim
    // something the profile does not say.
    let safe = match profile.control {
        ControlSpec::Setpoint {
            setpoint,
            safe_band,
            ..
        } => (setpoint - safe_band, setpoint + safe_band),
        _ => (f32::NEG_INFINITY, f32::INFINITY),
    };

    println!("== the night, hour by hour");
    let mut held = 0;
    for (hour, &(reading, charge)) in NIGHT.iter().enumerate() {
        // One tick: read the probe, decide with the profile, switch the lamp, publish.
        let reaction = node.tick().await?;

        let lamp = if reaction.actuator == Some(true) {
            "lamp on "
        } else {
            "lamp off"
        };
        let alert = match reaction.alert {
            Some(Alert::OutOfRange { reading }) => {
                format!("  ALERT {reading:.1} C is outside {} to {}", safe.0, safe.1)
            }
            Some(other) => format!("  ALERT {}", other.kind()),
            None => String::new(),
        };

        // Whatever the ladder could not send is still in the store, in order.
        let waiting = node.transport_mut().buffered().await?;
        let carried = if waiting > held {
            "  radio down, reading held"
        } else {
            ""
        };
        held = waiting;

        println!("hour {hour}: {reading:.1} C  {lamp}{alert}{carried}");

        // Ask the profile, not this code, how long to wait for the next sample.
        let (mode, wait) = node.schedule(charge, false);
        println!(
            "        battery {:.0}%, next sample in {}s ({})",
            charge * 100.0,
            wait.as_secs(),
            mode_name(mode)
        );
    }

    // Morning. Draining the store is a retry, not one shot: each attempt sends until the
    // radio refuses again, and the readings that are left keep their order for the next
    // one. This is the loop a gateway runs on a timer, and it is why nothing about the
    // night is lost, the cold hours included.
    println!("\n== morning");
    let mut attempts = 0;
    let mut flushed = 0;
    while node.transport_mut().buffered().await? > 0 && attempts < 20 {
        flushed += node.transport_mut().flush().await?;
        attempts += 1;
    }
    let still_held = node.transport_mut().buffered().await?;
    println!("{flushed} held readings went out in order over {attempts} attempts");
    if still_held > 0 {
        println!("{still_held} are still waiting for the next window, in order, not lost");
    }

    // The gateway now judges every reading that reached it, in order, against its rules.
    let delivered = NIGHT.len() - still_held;
    for _ in 0..delivered {
        let Some(fired) = gateway.step().await? else {
            break;
        };
        for firing in fired {
            println!(
                "the gateway's `{}` rule {} at {:.1} C",
                firing.rule,
                if firing.edge == Edge::Set {
                    "opened the vent"
                } else {
                    "closed the vent"
                },
                firing.reading
            );
        }
    }

    // What the hardware actually did, read back off the parts themselves.
    let (_, _, _, lamp, _, _) = node.into_parts();
    let line = lamp.release();
    let switched = line
        .driven()
        .windows(2)
        .filter(|pair| pair[0] != pair[1])
        .count();
    println!(
        "\nthe relay line was driven {} and actually changed state {}",
        times(line.driven().len()),
        times(switched)
    );
    println!("the vent fan changed state {}", times(fan_log.len()));

    // And this is what a dashboard draws, straight off the same manifest.
    if let Some(presentation) = &profile.presentation {
        println!("\n== what a dashboard draws from this profile");
        for element in &presentation.elements {
            let band = match element.band {
                Some([low, high]) => format!(", safe between {low} and {high}"),
                None => String::new(),
            };
            println!(
                "{} as a {} in {}{band}",
                element.label,
                element.viz.name(),
                element.unit
            );
        }
        let swahili: Vec<&str> = presentation
            .elements
            .iter()
            .filter_map(|element| element.labels.as_ref()?.get("sw").map(String::as_str))
            .collect();
        println!("in Swahili the same two read {}", swahili.join(" and "));
    }

    Ok(())
}
