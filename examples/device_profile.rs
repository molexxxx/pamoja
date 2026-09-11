//! A device profile assembled into a ready-to-run cold-chain node, over loopback.
//!
//! A builder picks the `vaccine-fridge-monitor` profile and hands it a sensor, a
//! cooler relay, an in-process link, and a codec. The assembled node does the rest:
//! each cycle it reads the probe, decides with the profile's controller, switches the
//! cooler, and publishes the reading to a gateway, which decodes it. A door left open
//! pushes the temperature into the danger zone and the node raises an excursion
//! alert; as the simulated battery sags, the node reports a longer wait before the
//! next sample. No hardware, no broker. This composes six crates - `pamoja-profile`,
//! `pamoja-kit` and `pamoja-power` (through the profile), `pamoja-codec`,
//! `pamoja-loopback`, and `pamoja-sim` for the fake probe and relay - through the
//! same shapes every binding exposes.
//!
//! Run with: `cargo run -p pamoja-examples --example device_profile`

use pamoja_codec::{CborCodec, Codec};
use pamoja_core::{Receive, Result, Transport};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_power::PowerMode;
use pamoja_profile::{Node, Profile};
use pamoja_sim::{RecordingActuator, Replay};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Pick a profile. The wiring, tuning, and control loop come with it.
    let profile = Profile::vaccine_fridge_monitor();
    println!(
        "assembled '{}', publishing to {}",
        profile.name, profile.topic
    );

    // A gateway and the node share an in-process broker.
    let broker = LoopbackBroker::new();
    let mut gateway = LoopbackTransport::new(broker.clone());
    let mut link = LoopbackTransport::new(broker);
    gateway.connect().await?;
    link.connect().await?;
    gateway.subscribe("cold-chain/#").await?;

    // (probe reading in C, battery state of charge) across the afternoon. A door is
    // left open in the middle, pushing the fridge out of the safe range, while the
    // battery sags from comfortable to critical.
    let afternoon = [
        (5.0, 0.85),
        (5.4, 0.70),
        (6.8, 0.55),
        (8.6, 0.45),
        (9.1, 0.30),
        (6.0, 0.18),
        (4.8, 0.10),
    ];

    // A fake probe replays the afternoon, and a fake relay records every switch, both
    // standing in for hardware behind the same traits a real driver will implement.
    let readings: Vec<f32> = afternoon.iter().map(|&(reading, _)| reading).collect();
    let probe = Replay::new(readings);
    let cooler = RecordingActuator::new();
    let cooler_log = cooler.log();
    let mut node = Node::new(profile, probe, cooler, link, CborCodec);

    // Each cycle the node reads, decides, switches the cooler, and publishes.
    let codec = CborCodec;
    for (step, (_, soc)) in afternoon.iter().enumerate() {
        let reaction = node.tick().await?;
        let reading: f32 = codec.decode(&gateway.recv().await?.expect("a reading").payload)?;

        let cooler = if reaction.actuator == Some(true) {
            "on "
        } else {
            "off"
        };
        let flag = if reaction.alert.is_some() {
            "  ALERT out of safe range"
        } else {
            ""
        };
        println!("step {step}: {reading:.1} C  cooler {cooler}{flag}");

        // Ask the profile how long to wait before the next sample at this charge.
        let (mode, wait) = node.schedule(*soc, false);
        let mode = match mode {
            PowerMode::Active => "active",
            PowerMode::Saver => "saver",
            PowerMode::Critical => "critical",
        };
        println!(
            "        battery {:.0}%, next sample in {}s ({mode})",
            soc * 100.0,
            wait.as_secs()
        );
    }

    // The recording relay captured every switch the controller made.
    let runs = cooler_log.commands().iter().filter(|&&on| on).count();
    println!("\ncooler ran in {runs} of {} cycles", cooler_log.len());

    // The profile that drove all of this is plain data. The same manifest is in
    // profiles/vaccine-fridge-monitor.json, and a device loads it with
    // `Profile::from_json`; nothing here is recompiled to change a threshold.
    let profile = node.profile();
    println!(
        "\nthe manifest behind it, in {} lines of JSON:",
        profile.to_json()?.lines().count()
    );
    println!("  a {} policy on {}", profile.control.kind(), profile.topic);
    println!(
        "  a schedule of {}s, {}s, and {}s as the battery drains",
        profile.power.active_secs, profile.power.saver_secs, profile.power.critical_secs
    );
    if let Some(presentation) = &profile.presentation {
        let drawn: Vec<&str> = presentation
            .elements
            .iter()
            .map(|element| element.label.as_str())
            .collect();
        println!("  and a dashboard that draws {}", drawn.join(", "));
    }

    println!("done");
    Ok(())
}
