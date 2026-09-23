//! The robot motion guide example; see docs/guides/motion.md.
//!
//! Run: `cargo run -p pamoja-examples --example motion`

use std::error::Error;

/// A rover that inspects a solar farm at night: the same turn on four chassis, its wheel
/// encoders and odometry between GPS fixes, the drive to an inverter cabinet, the safety gate
/// every command passes through, and the small arm that presses the cabinet's reset button.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_kit::{
        forward_kinematics, obstacle_stop, Ackermann, Coordinate, DhParameters, DiffDrive, Elbow,
        Esc, Limits, Mecanum, Odometry, Quadrature, QuadratureScale, SafetyGate, ServoMap,
        SkidSteer, Twist, TwoLinkArm, WaypointFollower,
    };

    // The same turn, 0.5 m/s forward while turning left at 0.4 rad/s, on four chassis.
    let (left, right) = DiffDrive::new(0.5).wheel_speeds(0.5, 0.4);
    println!("chassis   two wheels 0.5 m apart: left {left:.2} m/s, right {right:.2} m/s");
    let (left, right) = SkidSteer::new(0.5, 1.3).wheel_speeds(0.5, 0.4);
    println!("chassis   tracks that slip 1.3 times: left {left:.2} m/s, right {right:.2} m/s");
    let car = Ackermann::new(0.8);
    let steering = car.steering_angle(0.5, 0.4);
    let radius = car.turn_radius(steering);
    println!(
        "chassis   car-like, 0.8 m wheelbase: steer {:.1} degrees, a {radius:.2} m turn radius",
        steering.to_degrees()
    );
    let base = Mecanum::new(0.4, 0.3);
    let turn = base.wheel_speeds(Twist::new(0.5, 0.0, 0.4));
    println!(
        "chassis   mecanum, front and rear, left and right: {:.2}, {:.2}, {:.2}, {:.2} m/s",
        turn.front_left, turn.front_right, turn.rear_left, turn.rear_right
    );
    let strafe = base.wheel_speeds(Twist::new(0.0, 0.3, 0.0));
    println!(
        "chassis   mecanum strafing left, which no other chassis can: {:.2}, {:.2}, {:.2}, {:.2} m/s",
        strafe.front_left, strafe.front_right, strafe.rear_left, strafe.rear_right
    );

    // Each wheel's encoder gives two channels a quarter step apart; the order they change
    // in tells the direction. These levels step forward four times, then back once.
    let mut encoder = Quadrature::new();
    let mut steps = Vec::new();
    for (a, b) in [
        (false, true),
        (true, true),
        (true, false),
        (false, false),
        (true, false),
    ] {
        steps.push(encoder.update(a, b).to_string());
    }
    println!(
        "encoder   each change counts {}: {} steps forward in all",
        steps.join(", "),
        encoder.count()
    );

    // Between GPS fixes the rover knows where it is from how far each wheel rolled. Each
    // encoder makes 360 steps a turn of a 0.1 m wheel; every half second the right wheel
    // counts a few more steps than the left, so the rover curves left.
    let wheel = QuadratureScale::new(360.0, 0.1);
    let drive = DiffDrive::new(0.5);
    let mut odometry = Odometry::at_origin();
    for (left_steps, right_steps) in [(430, 430), (425, 455), (425, 455), (430, 430)] {
        odometry.integrate_wheels(
            wheel.distance(left_steps),
            wheel.distance(right_steps),
            &drive,
        );
    }
    let pose = odometry.pose();
    println!(
        "odometry  after two seconds: {:.2} m ahead, {:.2} m left, heading {:.1} degrees",
        pose.x,
        pose.y,
        pose.theta.to_degrees()
    );

    // A positive yaw rate turns left and a negative one right, as ROS has it.
    let turning = |omega: f32| {
        let side = if omega < 0.0 { "right" } else { "left" };
        format!("turning {side} at {:.2} rad/s", omega.abs())
    };

    // The inverter cabinet is about 41 m east. Facing north, the rover pivots toward it before it
    // drives; nearly facing it, it drives and trims its heading; close enough, it stops.
    let follower = WaypointFollower::new(0.8, 2.0, 1.5, 0.8);
    let here = Coordinate::new(-23.5610, 133.8700);
    let cabinet = Coordinate::new(-23.5610, 133.8704);
    for heading in [0.0, 80.0] {
        let guidance = follower.guide(here, heading, cabinet);
        println!(
            "waypoint  heading {heading:.0}, {:.0} m to go: forward {:.2} m/s, {}",
            guidance.distance_m,
            guidance.twist.vx,
            turning(guidance.twist.omega)
        );
    }
    let at_cabinet = follower.guide(Coordinate::new(-23.5610, 133.87039), 90.0, cabinet);
    let arrived = if at_cabinet.arrived {
        "arrived"
    } else {
        "still driving"
    };
    println!(
        "waypoint  {:.1} m from the cabinet: {arrived}",
        at_cabinet.distance_m
    );

    // Every command passes through the safety gate: held to 1 m/s, eased on at 0.5 m/s^2,
    // and stopped if no fresh command arrives for 0.3 s.
    let mut gate = SafetyGate::new(Limits::new(1.0, 1.0, 0.5, 2.0), 0.3);
    let ahead = Twist::planar(0.8, 0.0);
    let mut eased = Vec::new();
    for _ in 0..3 {
        gate.feed();
        eased.push(format!("{:.2}", gate.command(ahead, 0.1).vx));
    }
    println!(
        "safety    asked for 0.80 m/s from rest, allowed {} m/s",
        eased.join(", ")
    );
    let mut silence = Vec::new();
    for _ in 0..3 {
        silence.push(format!("{:.2}", gate.command(ahead, 0.1).vx));
    }
    println!(
        "safety    then no fresh command: {} m/s, stopped once more than 0.3 s pass",
        silence.join(", ")
    );
    let veering = Twist::new(0.8, 0.0, 0.2);
    let near = obstacle_stop(veering, 0.35, 0.5);
    println!(
        "safety    something 0.35 m ahead: forward {:.2} m/s, still {}",
        near.vx,
        turning(near.omega)
    );
    let blind = obstacle_stop(veering, f32::NAN, 0.5);
    println!(
        "safety    no range reading at all: forward {:.2} m/s, still {}",
        blind.vx,
        turning(blind.omega)
    );
    gate.feed();
    gate.engage_estop();
    let stopped = gate.command(ahead, 0.1);
    println!(
        "safety    e-stop engaged: {:.2} m/s until a person resets it",
        stopped.vx
    );

    // At the cabinet, a two-link arm presses the reset button 0.35 m out and 0.20 m up.
    let arm = TwoLinkArm::new(0.30, 0.25);
    let (reach_min, reach_max) = arm.reach();
    println!("arm       links 0.30 m and 0.25 m reach from {reach_min:.2} m to {reach_max:.2} m");
    let (shoulder, elbow) = arm
        .joints_for(0.35, 0.20, Elbow::Up)
        .ok_or("the button is in reach")?;
    println!(
        "arm       elbow up: shoulder {:.1} degrees, elbow {:.1} degrees",
        shoulder.to_degrees(),
        elbow.to_degrees()
    );
    let chain = [
        DhParameters {
            a: 0.30,
            alpha: 0.0,
            d: 0.0,
            theta: shoulder,
        },
        DhParameters {
            a: 0.25,
            alpha: 0.0,
            d: 0.0,
            theta: elbow,
        },
    ];
    let (x, y, _) = forward_kinematics(&chain).position();
    println!("arm       forward kinematics puts the tip at {x:.2} m, {y:.2} m");
    let too_far = arm
        .joints_for(0.70, 0.0, Elbow::Up)
        .map_or("out of reach", |_| "in reach");
    println!("arm       a button 0.70 m out: {too_far}");

    // Hobby servos turn the joints. A joint angle of 0 is the servo's center, 90 degrees.
    let servo = ServoMap::standard();
    let shoulder_pulse = servo.pulse(90.0 + shoulder.to_degrees());
    let elbow_pulse = servo.pulse(90.0 + elbow.to_degrees());
    println!("servos    shoulder {shoulder_pulse} us, elbow {elbow_pulse} us");
    let esc = Esc::bidirectional();
    println!(
        "motors    a quarter throttle forward is {} us, a quarter back {} us",
        esc.pulse(0.25),
        esc.pulse(-0.25)
    );
    // ANCHOR_END: example

    assert_eq!(steps, ["1", "1", "1", "1", "-1"]);
    assert_eq!(encoder.count(), 3);
    assert!(at_cabinet.arrived);
    assert_eq!(silence, ["0.20", "0.25", "0.00"]);
    assert!(stopped.vx == 0.0 && near.vx == 0.0 && blind.vx == 0.0);
    assert!((x - 0.35).abs() < 1e-4 && (y - 0.20).abs() < 1e-4);
    Ok(())
}
