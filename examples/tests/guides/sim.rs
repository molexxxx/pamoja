//! The simulators guide example; see docs/guides/sim.md.

/// A survey rover driven through a recorded capture with nothing wired up: what it read,
/// what it decided to do, and where those commands dead-reckon it to.
#[tokio::test]
async fn a_rover_runs_a_recorded_capture_with_no_hardware() {
    // ANCHOR: example
    use pamoja_core::{Actuator, Sensor};
    use pamoja_kit::Twist;
    use pamoja_sim::{RecordingActuator, Replay, SimRobot};

    // The clear distance ahead, in metres, taken from an earlier survey run. A replay
    // hands it back one reading at a time, so the loop below sees the same input on every
    // run: the same rover code, driven by a recording rather than a range finder.
    let capture = vec![4.0, 3.0, 1.5, 0.5];
    let mut ahead = Replay::new(capture.clone());
    let mut throttle = RecordingActuator::new();
    let log = throttle.log();
    let mut rover = SimRobot::new(0.5); // each command advances the rover half a second

    let mut seen = Vec::new();
    for _ in &capture {
        let reading = ahead.read().await.expect("a reading from the capture");
        seen.push(reading);

        // Drive on while there is room ahead, otherwise stop and turn on the spot.
        let clear = reading > 1.0;
        let speed = if clear { 1.0 } else { 0.0 };
        let turn = if clear { 0.0 } else { 1.0 };
        throttle.apply(speed).await.expect("the throttle takes it");
        rover
            .apply(Twist::planar(speed, turn))
            .await
            .expect("the rover takes it");
        println!("{reading} m ahead, so drive at {speed} and turn at {turn}");
    }

    // The recording actuator kept every command, which is how a test says what the control
    // loop decided rather than only what it ended up doing.
    println!("commands  {:?}", log.commands());

    // Three half-second commands at 1 m/s reach 1.5 m along x. The last one turns on the
    // spot at 1 rad/s for half a second, which moves the rover nowhere.
    let pose = rover.pose();
    let (x, y, heading) = (pose.x, pose.y, pose.theta);
    println!("pose      x {x:.1} m, y {y:.1} m, heading {heading:.1} rad");
    // ANCHOR_END: example

    assert_eq!(seen, capture);
    assert_eq!(log.commands(), vec![1.0, 1.0, 1.0, 0.0]);
    assert!((pose.x - 1.5).abs() < 1e-6);
    assert!(pose.y.abs() < 1e-6);
    assert!((pose.theta - 0.5).abs() < 1e-6);
}
