//! The simulators guide example; see docs/guides/sim.md.
//!
//! Run: `cargo run -p pamoja-examples --example sim`

use std::error::Error;

/// A vineyard rover checking the soil down a row with nothing built: a replayed range
/// finder, a drive that records what it is told, a pose from the kinematics, a seeded
/// soil probe, and a radio that loses every third report.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_core::{Actuator, Sensor, Transport};
    use pamoja_kit::Twist;
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    use pamoja_sim::{DegradedLink, RecordingActuator, Replay, SimRobot, SimSensor};

    // The clear distance ahead, in meters, replayed from an earlier survey of the row,
    // so every run sees the same row.
    let mut ahead = Replay::new(vec![4.0, 3.0, 1.5, 0.5]);
    // The drive keeps the commands it is given instead of turning a motor.
    let mut drive = RecordingActuator::new();
    let told = drive.log();
    // The rover's pose comes from integrating each command over half a second.
    let dt = 0.5;
    let mut rover = SimRobot::new(dt);
    // The soil probe reads around 31 percent, drying half a point a reading, with a
    // wobble drawn from a seed, so the same seed gives the same readings every run.
    let mut soil = SimSensor::new(31.0)
        .with_drift(-0.5)
        .with_noise(0.3)
        .with_seed(7);
    // The radio loses every third report on its way to the base.
    let mut radio = DegradedLink::new(LoopbackTransport::new(LoopbackBroker::new())).drop_every(3);
    radio.connect().await?;

    let mut moisture = Vec::new();
    let mut delivered = 0;
    loop {
        // A replay that has handed back every reading reports that it is closed.
        let clear = match ahead.read().await {
            Ok(clear) => clear,
            Err(error) => {
                println!(
                    "ahead     ran out after {} readings: {error}",
                    moisture.len()
                );
                break;
            }
        };
        let (speed, turn) = if clear > 1.0 { (1.0, 0.0) } else { (0.0, 1.0) };
        drive.apply(speed).await?;
        rover.apply(Twist::planar(speed, turn)).await?;
        let wet = soil.read().await?;
        let elapsed = moisture.len() as f32 * dt;
        moisture.push(wet);
        let report = format!("{wet:.1}");
        if radio
            .send_text("vineyard/row-4/soil", &report)
            .await
            .is_ok()
        {
            delivered += 1;
        }
        println!("{elapsed:.1} s     {clear:.1} m clear: drive {speed:.1}, turn {turn:.1}, soil {report}");
    }

    // The drive kept every command, which is how a test says what the loop decided
    // rather than only what it ended up doing.
    let commands: Vec<String> = told.commands().iter().map(|c| format!("{c:.1}")).collect();
    println!("drive     recorded {}", commands.join(", "));

    // Three half-second commands at 1 m/s reach 1.5 m along x. The last turns on the
    // spot at 1 rad/s for half a second, which moves the rover nowhere.
    let pose = rover.pose();
    println!(
        "rover     ended at x {:.1} m, y {:.1} m, heading {:.1} rad",
        pose.x, pose.y, pose.theta
    );

    // A second probe with the same seed reads exactly the same values.
    let mut twin = SimSensor::new(31.0)
        .with_drift(-0.5)
        .with_noise(0.3)
        .with_seed(7);
    let mut again = Vec::new();
    for _ in 0..moisture.len() {
        again.push(twin.read().await?);
    }
    let verdict = if again == moisture {
        "the same"
    } else {
        "different"
    };
    println!(
        "soil      a probe with the same seed read {verdict} {} values",
        again.len()
    );
    println!(
        "radio     delivered {delivered} of {} soil reports and lost the third",
        moisture.len()
    );
    // ANCHOR_END: example

    assert_eq!(told.commands(), [1.0, 1.0, 1.0, 0.0]);
    assert!((pose.x - 1.5).abs() < 1e-6 && pose.y.abs() < 1e-6);
    assert!((pose.theta - 0.5).abs() < 1e-6);
    assert_eq!(again, moisture);
    assert_eq!(delivered, 3);

    Ok(())
}
