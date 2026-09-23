# Robot motion

A robot that moves needs the same few pieces of arithmetic whatever it is built from: the
wheel speeds that make a turn, the pose its wheels say it has reached, the command that
points it at a waypoint, a gate that stops it when something goes wrong, and the angles
that put an arm's hand where it should be. The motion helpers in `pamoja-kit` are those
pieces, in every language. Each is plain, allocation-free arithmetic with its conventions
written down, so the helpers agree with each other, and with ROS, about which way is left.

The helpers fall into five families. Chassis models turn a body motion into wheel speeds
and back: `DiffDrive`, `SkidSteer`, `Ackermann`, and `Mecanum`. Position helpers say where
the robot is: `Quadrature` and `QuadratureScale` read wheel encoders, and `Odometry` adds
up the motion. Guidance and safety helpers decide what it may do: `WaypointFollower` points
it at a GPS waypoint, `obstacle_stop` holds it short of what its range sensor sees, and
`SafetyGate` bounds every command and stops the robot on an e-stop or when commands stop
arriving. Arm helpers place a hand: `TwoLinkArm` and `forward_kinematics`. Output helpers
turn an angle or a throttle into the pulse width a hobby servo or speed controller reads:
`ServoMap` and `Esc`.

## What the example does

It runs a rover that inspects a solar farm at night, driving between panel rows to an
inverter cabinet and pressing the cabinet's reset button with a small arm.

- **The chassis** section asks for the same turn from four drives: 0.5 m/s forward while
  turning left at 0.4 rad/s. A differential drive, a tracked drive that slips, a car-like
  rover, and a mecanum base each need different wheel commands, and the mecanum base can
  also strafe sideways, which none of the others can.
- **The encoder** reads its two channels and counts steps in both directions.
- **Odometry** turns the steps each wheel counted over two seconds into a pose between GPS
  fixes.
- **The waypoint follower** points the rover at the cabinet: pivoting while it faces away,
  driving and trimming its heading once it faces it, and stopping inside the arrival radius.
- **The safety gate** eases the rover up to speed, stops it once commands stop arriving,
  cuts forward motion for an obstacle or a range sensor that sees nothing it can measure,
  and holds it stopped on an e-stop.
- **The arm** solves the joint angles for the reset button, checks them with forward
  kinematics, refuses a button out of reach, and turns the angles into servo pulses.

It proves:

- The same turn needs 0.40 and 0.60 m/s from two wheels 0.5 m apart, and 0.37 and 0.63
  m/s from tracks that slip 1.3 times, since slipping tracks need a wider split.
- A car-like rover with a 0.8 m wheelbase makes that turn by steering 32.6 degrees, on a
  1.25 m radius.
- A mecanum base makes it with 0.36 m/s on the left wheels and 0.64 m/s on the right, and
  strafes left at 0.3 m/s by spinning its diagonals against each other.
- Four encoder steps forward and one back count 3 in all.
- Wheel counts over two seconds put the rover 3.01 m ahead and 0.32 m left, turned 12
  degrees.
- Facing north with the cabinet 41 m east, the rover turns right at 0.80 rad/s without
  driving; facing 80 degrees, it drives at 0.79 m/s and turns right at 0.26 rad/s; 1.0 m
  away, it has arrived.
- From rest, a 0.80 m/s command is eased on at 0.05 m/s per tenth of a second, and once
  more than 0.3 s pass with no fresh command the gate stops the rover.
- An obstacle 0.35 m ahead, or no range reading at all, cuts forward motion and keeps the
  turn, so the rover can still turn away.
- An arm with 0.30 m and 0.25 m links reaches the button at 0.35 m out and 0.20 m up with
  its shoulder at -8.5 degrees and its elbow at 86.2, and forward kinematics puts the tip
  back on the button.
- A button 0.70 m out is out of reach, so the arm reports that rather than an angle.
- Those joint angles are 1453 and 1979 microsecond servo pulses, and a quarter throttle is
  1625 microseconds forward and 1375 back.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example motion" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example motion</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- motion" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- motion</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/motion.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/motion.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- motion" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- motion</code></div>
</div>
<!-- end -->

## Rust

In Rust, the motion helpers are in `pamoja-kit` behind the `robotics` feature, which is on
by default; `DiffDrive` needs no feature, and `WaypointFollower` and `obstacle_stop` also
need `geo`. They are `no_std` and allocation-free, so the same code steers a
microcontroller. The chassis models, the arm, the waypoint follower, and the servo, ESC,
and encoder scales hold only their parameters. The helpers that remember something,
`Odometry`, `Quadrature`, and `SafetyGate` with the `EStop`, `Watchdog`, and `Limits` it
holds, change through `&mut self`. Every one of them is a small `Copy` value, so a copy of
a stateful helper carries on apart from the original. A pair comes back as a tuple, a
joint solution that does not exist as `None`, and nothing returns a `Result`: a helper
never fails, and a parameter outside its range falls back as the parameter table below
says.

<!-- snippet: examples/guides/motion.rs#example -->
From [`examples/guides/motion.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/motion.rs):

```rust
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
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/kit` exports the motion helpers beside the other helpers. A twist,
a pose, a coordinate, a mecanum base's wheel speeds, and a joint's Denavit-Hartenberg
parameters are plain objects, such as `{ vx, vy, omega }`, with every field required, and
a pair comes back as an object with named fields, `{ left, right }` or `{ linear, angular }`.
The models and the stateful helpers are classes, and their state reads as a property:
`odometry.pose`, `gate.isStopped`, `encoder.count`. `jointsFor` returns `null` for a point
out of reach, and `Elbow.Up` and `Elbow.Down` are the strings `'up'` and `'down'`. A pulse
width must be a whole number of microseconds from 0 to 65535, and a step count a whole
number a JavaScript number holds exactly; anything else throws.

<!-- snippet: bindings/node/guides/motion.ts#example -->
From [`bindings/node/guides/motion.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/motion.ts):

```typescript
import {
  Ackermann,
  type Coord,
  DhParameters,
  DiffDrive,
  Elbow,
  Esc,
  forwardKinematics,
  Limits,
  Mecanum,
  obstacleStop,
  Odometry,
  Quadrature,
  QuadratureScale,
  SafetyGate,
  ServoMap,
  SkidSteer,
  TwoLinkArm,
  WaypointFollower,
  type WheelSpeeds,
} from '@pamoja/kit'

const degrees = (radians: number) => (radians * 180) / Math.PI

// The same turn, 0.5 m/s forward while turning left at 0.4 rad/s, on four chassis.
const pair = new DiffDrive(0.5).wheelSpeeds(0.5, 0.4)
console.log(`chassis   two wheels 0.5 m apart: left ${pair.left.toFixed(2)} m/s, right ${pair.right.toFixed(2)} m/s`)
const tracks = new SkidSteer(0.5, 1.3).wheelSpeeds(0.5, 0.4)
console.log(
  `chassis   tracks that slip 1.3 times: left ${tracks.left.toFixed(2)} m/s, right ${tracks.right.toFixed(2)} m/s`,
)
const car = new Ackermann(0.8)
const steering = car.steeringAngle(0.5, 0.4)
const radius = car.turnRadius(steering)
console.log(
  `chassis   car-like, 0.8 m wheelbase: steer ${degrees(steering).toFixed(1)} degrees, a ${radius.toFixed(2)} m turn radius`,
)
const base = new Mecanum(0.4, 0.3)
const turn = base.wheelSpeeds({ vx: 0.5, vy: 0, omega: 0.4 })
const four = (w: WheelSpeeds) =>
  [w.frontLeft, w.frontRight, w.rearLeft, w.rearRight].map((speed) => speed.toFixed(2)).join(', ')
console.log(`chassis   mecanum, front and rear, left and right: ${four(turn)} m/s`)
const strafe = base.wheelSpeeds({ vx: 0, vy: 0.3, omega: 0 })
console.log(`chassis   mecanum strafing left, which no other chassis can: ${four(strafe)} m/s`)

// Each wheel's encoder gives two channels a quarter step apart; the order they change in
// tells the direction. These levels step forward four times, then back once.
const encoder = new Quadrature()
const steps = (
  [
    [false, true],
    [true, true],
    [true, false],
    [false, false],
    [true, false],
  ] as const
).map(([a, b]) => encoder.update(a, b))
console.log(`encoder   each change counts ${steps.join(', ')}: ${encoder.count} steps forward in all`)

// Between GPS fixes the rover knows where it is from how far each wheel rolled. Each encoder
// makes 360 steps a turn of a 0.1 m wheel; every half second the right wheel counts a few more
// steps than the left, so the rover curves left.
const wheel = new QuadratureScale(360, 0.1)
const drive = new DiffDrive(0.5)
const odometry = new Odometry()
for (const [leftSteps, rightSteps] of [
  [430, 430],
  [425, 455],
  [425, 455],
  [430, 430],
]) {
  odometry.integrateWheels(wheel.distance(leftSteps), wheel.distance(rightSteps), drive)
}
const pose = odometry.pose
console.log(
  `odometry  after two seconds: ${pose.x.toFixed(2)} m ahead, ${pose.y.toFixed(2)} m left, heading ${degrees(pose.theta).toFixed(1)} degrees`,
)

// A positive yaw rate turns left and a negative one right, as ROS has it.
const turning = (omega: number) =>
  `turning ${omega < 0 ? 'right' : 'left'} at ${Math.abs(omega).toFixed(2)} rad/s`

// The inverter cabinet is about 41 m east. Facing north, the rover pivots toward it before it
// drives; nearly facing it, it drives and trims its heading; close enough, it stops.
const follower = new WaypointFollower(0.8, 2.0, 1.5, 0.8)
const here: Coord = { latitude: -23.561, longitude: 133.87 }
const cabinet: Coord = { latitude: -23.561, longitude: 133.8704 }
for (const heading of [0, 80]) {
  const guidance = follower.guide(here, heading, cabinet)
  console.log(
    `waypoint  heading ${heading.toFixed(0)}, ${guidance.distanceM.toFixed(0)} m to go: forward ${guidance.twist.vx.toFixed(2)} m/s, ${turning(guidance.twist.omega)}`,
  )
}
const atCabinet = follower.guide({ latitude: -23.561, longitude: 133.87039 }, 90, cabinet)
console.log(
  `waypoint  ${atCabinet.distanceM.toFixed(1)} m from the cabinet: ${atCabinet.arrived ? 'arrived' : 'still driving'}`,
)

// Every command passes through the safety gate: held to 1 m/s, eased on at 0.5 m/s^2, and
// stopped if no fresh command arrives for 0.3 s.
const gate = new SafetyGate(new Limits(1.0, 1.0, 0.5, 2.0), 0.3)
const ahead = { vx: 0.8, vy: 0, omega: 0 }
const eased: string[] = []
for (let i = 0; i < 3; i += 1) {
  gate.feed()
  eased.push(gate.command(ahead, 0.1).vx.toFixed(2))
}
console.log(`safety    asked for 0.80 m/s from rest, allowed ${eased.join(', ')} m/s`)
const silence: string[] = []
for (let i = 0; i < 3; i += 1) {
  silence.push(gate.command(ahead, 0.1).vx.toFixed(2))
}
console.log(`safety    then no fresh command: ${silence.join(', ')} m/s, stopped once more than 0.3 s pass`)
const veering = { vx: 0.8, vy: 0, omega: 0.2 }
const near = obstacleStop(veering, 0.35, 0.5)
console.log(`safety    something 0.35 m ahead: forward ${near.vx.toFixed(2)} m/s, still ${turning(near.omega)}`)
const blind = obstacleStop(veering, NaN, 0.5)
console.log(`safety    no range reading at all: forward ${blind.vx.toFixed(2)} m/s, still ${turning(blind.omega)}`)
gate.feed()
gate.engageEstop()
const stopped = gate.command(ahead, 0.1)
console.log(`safety    e-stop engaged: ${stopped.vx.toFixed(2)} m/s until a person resets it`)

// At the cabinet, a two-link arm presses the reset button 0.35 m out and 0.20 m up.
const arm = new TwoLinkArm(0.3, 0.25)
console.log(
  `arm       links 0.30 m and 0.25 m reach from ${arm.reach.min.toFixed(2)} m to ${arm.reach.max.toFixed(2)} m`,
)
const joints = arm.jointsFor(0.35, 0.2, Elbow.Up)
assert.ok(joints, 'the button is in reach')
console.log(
  `arm       elbow up: shoulder ${degrees(joints.shoulder).toFixed(1)} degrees, elbow ${degrees(joints.elbow).toFixed(1)} degrees`,
)
const chain: DhParameters[] = [
  { a: 0.3, alpha: 0, d: 0, theta: joints.shoulder },
  { a: 0.25, alpha: 0, d: 0, theta: joints.elbow },
]
const tip = forwardKinematics(chain).position
console.log(`arm       forward kinematics puts the tip at ${tip.x.toFixed(2)} m, ${tip.y.toFixed(2)} m`)
const tooFar = arm.jointsFor(0.7, 0, Elbow.Up) === null ? 'out of reach' : 'in reach'
console.log(`arm       a button 0.70 m out: ${tooFar}`)

// Hobby servos turn the joints. A joint angle of 0 is the servo's center, 90 degrees.
const servo = ServoMap.standard()
const shoulderPulse = servo.pulse(90 + degrees(joints.shoulder))
const elbowPulse = servo.pulse(90 + degrees(joints.elbow))
console.log(`servos    shoulder ${shoulderPulse} us, elbow ${elbowPulse} us`)
const esc = Esc.bidirectional()
console.log(`motors    a quarter throttle forward is ${esc.pulse(0.25)} us, a quarter back ${esc.pulse(-0.25)} us`)
```
<!-- end -->

## Python

In Python, `pamoja.kit` exports them too. `Pose`, `Twist`, `WheelSpeeds`, `DhParameters`,
`Transform`, and `Guidance` are frozen value classes that compare by value, and `Pose`,
`Twist`, and `DhParameters` default every field to 0, so `Twist(0.8)` drives straight
ahead. A pair comes back as a tuple, so `left, right = drive.wheel_speeds(0.5, 0.4)`
unpacks it. `joints_for` returns `None` for a point out of reach and takes the `Elbow`
enum, up unless told otherwise, and a coordinate is the `Coordinate` named tuple. State
reads as a property: `odometry.pose`, `gate.is_stopped`, `encoder.count`. A pulse width
outside 0 to 65535 raises `ValueError`. `pamoja.sim` hands back the same `Pose` class.

<!-- snippet: bindings/python/guides/motion.py#example -->
From [`bindings/python/guides/motion.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/motion.py):

```python
import math

from pamoja.kit import (
    Ackermann,
    Coordinate,
    DhParameters,
    DiffDrive,
    Elbow,
    Esc,
    Limits,
    Mecanum,
    Odometry,
    Quadrature,
    QuadratureScale,
    SafetyGate,
    ServoMap,
    SkidSteer,
    Twist,
    TwoLinkArm,
    WaypointFollower,
    WheelSpeeds,
    forward_kinematics,
    obstacle_stop,
)

# The same turn, 0.5 m/s forward while turning left at 0.4 rad/s, on four chassis.
left, right = DiffDrive(0.5).wheel_speeds(0.5, 0.4)
print(f"chassis   two wheels 0.5 m apart: left {left:.2f} m/s, right {right:.2f} m/s")
left, right = SkidSteer(0.5, 1.3).wheel_speeds(0.5, 0.4)
print(f"chassis   tracks that slip 1.3 times: left {left:.2f} m/s, right {right:.2f} m/s")
car = Ackermann(0.8)
steering = car.steering_angle(0.5, 0.4)
radius = car.turn_radius(steering)
print(
    f"chassis   car-like, 0.8 m wheelbase: steer {math.degrees(steering):.1f} degrees, "
    f"a {radius:.2f} m turn radius"
)
base = Mecanum(0.4, 0.3)


def four(wheels: WheelSpeeds) -> str:
    speeds = (wheels.front_left, wheels.front_right, wheels.rear_left, wheels.rear_right)
    return ", ".join(f"{speed:.2f}" for speed in speeds)


turn = base.wheel_speeds(Twist(0.5, 0.0, 0.4))
print(f"chassis   mecanum, front and rear, left and right: {four(turn)} m/s")
strafe = base.wheel_speeds(Twist(0.0, 0.3, 0.0))
print(f"chassis   mecanum strafing left, which no other chassis can: {four(strafe)} m/s")

# Each wheel's encoder gives two channels a quarter step apart; the order they change in
# tells the direction. These levels step forward four times, then back once.
encoder = Quadrature()
steps = [
    encoder.update(a, b)
    for a, b in ((False, True), (True, True), (True, False), (False, False), (True, False))
]
print(f"encoder   each change counts {', '.join(map(str, steps))}: {encoder.count} steps forward in all")

# Between GPS fixes the rover knows where it is from how far each wheel rolled. Each encoder
# makes 360 steps a turn of a 0.1 m wheel; every half second the right wheel counts a few more
# steps than the left, so the rover curves left.
wheel = QuadratureScale(360.0, 0.1)
drive = DiffDrive(0.5)
odometry = Odometry()
for left_steps, right_steps in ((430, 430), (425, 455), (425, 455), (430, 430)):
    odometry.integrate_wheels(wheel.distance(left_steps), wheel.distance(right_steps), drive)
pose = odometry.pose
print(
    f"odometry  after two seconds: {pose.x:.2f} m ahead, {pose.y:.2f} m left, "
    f"heading {math.degrees(pose.theta):.1f} degrees"
)

# A positive yaw rate turns left and a negative one right, as ROS has it.
def turning(omega: float) -> str:
    return f"turning {'right' if omega < 0 else 'left'} at {abs(omega):.2f} rad/s"


# The inverter cabinet is about 41 m east. Facing north, the rover pivots toward it before it
# drives; nearly facing it, it drives and trims its heading; close enough, it stops.
follower = WaypointFollower(0.8, 2.0, 1.5, 0.8)
here = Coordinate(-23.5610, 133.8700)
cabinet = Coordinate(-23.5610, 133.8704)
for heading in (0.0, 80.0):
    guidance = follower.guide(here, heading, cabinet)
    print(
        f"waypoint  heading {heading:.0f}, {guidance.distance_m:.0f} m to go: "
        f"forward {guidance.twist.vx:.2f} m/s, {turning(guidance.twist.omega)}"
    )
at_cabinet = follower.guide(Coordinate(-23.5610, 133.87039), 90.0, cabinet)
arrived = "arrived" if at_cabinet.arrived else "still driving"
print(f"waypoint  {at_cabinet.distance_m:.1f} m from the cabinet: {arrived}")

# Every command passes through the safety gate: held to 1 m/s, eased on at 0.5 m/s^2, and
# stopped if no fresh command arrives for 0.3 s.
gate = SafetyGate(Limits(1.0, 1.0, 0.5, 2.0), 0.3)
ahead = Twist(0.8)
eased = []
for _ in range(3):
    gate.feed()
    eased.append(f"{gate.command(ahead, 0.1).vx:.2f}")
print(f"safety    asked for 0.80 m/s from rest, allowed {', '.join(eased)} m/s")
silence = [f"{gate.command(ahead, 0.1).vx:.2f}" for _ in range(3)]
print(f"safety    then no fresh command: {', '.join(silence)} m/s, stopped once more than 0.3 s pass")
veering = Twist(0.8, 0.0, 0.2)
near = obstacle_stop(veering, 0.35, 0.5)
print(f"safety    something 0.35 m ahead: forward {near.vx:.2f} m/s, still {turning(near.omega)}")
blind = obstacle_stop(veering, math.nan, 0.5)
print(f"safety    no range reading at all: forward {blind.vx:.2f} m/s, still {turning(blind.omega)}")
gate.feed()
gate.engage_estop()
stopped = gate.command(ahead, 0.1)
print(f"safety    e-stop engaged: {stopped.vx:.2f} m/s until a person resets it")

# At the cabinet, a two-link arm presses the reset button 0.35 m out and 0.20 m up.
arm = TwoLinkArm(0.30, 0.25)
reach_min, reach_max = arm.reach
print(f"arm       links 0.30 m and 0.25 m reach from {reach_min:.2f} m to {reach_max:.2f} m")
joints = arm.joints_for(0.35, 0.20, Elbow.UP)
assert joints is not None, "the button is in reach"
shoulder, elbow = joints
print(
    f"arm       elbow up: shoulder {math.degrees(shoulder):.1f} degrees, "
    f"elbow {math.degrees(elbow):.1f} degrees"
)
chain = [DhParameters(a=0.30, theta=shoulder), DhParameters(a=0.25, theta=elbow)]
x, y, _ = forward_kinematics(chain).position
print(f"arm       forward kinematics puts the tip at {x:.2f} m, {y:.2f} m")
too_far = "out of reach" if arm.joints_for(0.70, 0.0, Elbow.UP) is None else "in reach"
print(f"arm       a button 0.70 m out: {too_far}")

# Hobby servos turn the joints. A joint angle of 0 is the servo's center, 90 degrees.
servo = ServoMap.standard()
shoulder_pulse = servo.pulse(90.0 + math.degrees(shoulder))
elbow_pulse = servo.pulse(90.0 + math.degrees(elbow))
print(f"servos    shoulder {shoulder_pulse} us, elbow {elbow_pulse} us")
esc = Esc.bidirectional()
print(f"motors    a quarter throttle forward is {esc.pulse(0.25)} us, a quarter back {esc.pulse(-0.25)} us")
```
<!-- end -->

## C#

In C#, `Pamoja.Kit` holds the models and plain values as `readonly record struct`s, so
`DiffDrive`, `Mecanum`, `TwoLinkArm`, `WaypointFollower`, `ServoMap`, `Pose`, `Twist`, and
the rest need no disposal and compare by value. A pair comes back as a named tuple, such as
`(float Left, float Right)`, and `JointsFor` returns a nullable one. The stateful helpers,
`Odometry`, `SafetyGate`, `Limits`, `EStop`, `Watchdog`, and `Quadrature`, hold a native
handle, belong in a `using`, and run one call at a time when several threads share one. The
free functions are static on `Kit`: `Kit.ForwardKinematics` and `Kit.ObstacleStop`. `Pose`
and `Twist` live here, and the simulated robot in `Pamoja.Sim` takes them from here.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MotionGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/MotionGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MotionGuide.cs):

```csharp
static double Degrees(float radians) => radians * 180.0 / Math.PI;

// The same turn, 0.5 m/s forward while turning left at 0.4 rad/s, on four chassis.
(float left, float right) = new DiffDrive(0.5f).WheelSpeeds(0.5f, 0.4f);
Console.WriteLine(Invariant($"chassis   two wheels 0.5 m apart: left {left:F2} m/s, right {right:F2} m/s"));
(left, right) = new SkidSteer(0.5f, 1.3f).WheelSpeeds(0.5f, 0.4f);
Console.WriteLine(Invariant($"chassis   tracks that slip 1.3 times: left {left:F2} m/s, right {right:F2} m/s"));
var car = new Ackermann(0.8f);
float steering = car.SteeringAngle(0.5f, 0.4f);
float radius = car.TurnRadius(steering);
Console.WriteLine(Invariant(
    $"chassis   car-like, 0.8 m wheelbase: steer {Degrees(steering):F1} degrees, a {radius:F2} m turn radius"));
var mecanum = new Mecanum(0.4f, 0.3f);
static string Four(WheelSpeeds w) =>
    string.Join(", ", new[] { w.FrontLeft, w.FrontRight, w.RearLeft, w.RearRight }
        .Select(speed => speed.ToString("F2", CultureInfo.InvariantCulture)));
WheelSpeeds turn = mecanum.WheelSpeeds(new Twist(0.5f, 0.0f, 0.4f));
Console.WriteLine($"chassis   mecanum, front and rear, left and right: {Four(turn)} m/s");
WheelSpeeds strafe = mecanum.WheelSpeeds(new Twist(0.0f, 0.3f, 0.0f));
Console.WriteLine($"chassis   mecanum strafing left, which no other chassis can: {Four(strafe)} m/s");

// Each wheel's encoder gives two channels a quarter step apart; the order they
// change in tells the direction. These levels step forward four times, then back
// once.
using var encoder = new Quadrature();
int[] steps = new[] { (false, true), (true, true), (true, false), (false, false), (true, false) }
    .Select(level => encoder.Update(level.Item1, level.Item2))
    .ToArray();
Console.WriteLine($"encoder   each change counts {string.Join(", ", steps)}: {encoder.Count} steps forward in all");

// Between GPS fixes the rover knows where it is from how far each wheel rolled. Each
// encoder makes 360 steps a turn of a 0.1 m wheel; every half second the right wheel
// counts a few more steps than the left, so the rover curves left.
var wheel = new QuadratureScale(360.0f, 0.1f);
var drive = new DiffDrive(0.5f);
using var odometry = new Odometry();
foreach ((long leftSteps, long rightSteps) in new[] { (430L, 430L), (425L, 455L), (425L, 455L), (430L, 430L) })
{
    odometry.IntegrateWheels(wheel.Distance(leftSteps), wheel.Distance(rightSteps), drive);
}

Pose pose = odometry.Pose;
Console.WriteLine(Invariant(
    $"odometry  after two seconds: {pose.X:F2} m ahead, {pose.Y:F2} m left, heading {Degrees(pose.Theta):F1} degrees"));

// A positive yaw rate turns left and a negative one right, as ROS has it.
static string Turning(float omega) =>
    Invariant($"turning {(omega < 0.0f ? "right" : "left")} at {Math.Abs(omega):F2} rad/s");

// The inverter cabinet is about 41 m east. Facing north, the rover pivots toward it before
// it drives; nearly facing it, it drives and trims its heading; close enough, it
// stops.
var follower = new WaypointFollower(0.8f, 2.0, 1.5f, 0.8f);
var here = new Coordinate(-23.5610, 133.8700);
var cabinet = new Coordinate(-23.5610, 133.8704);
foreach (float heading in new[] { 0.0f, 80.0f })
{
    Guidance guidance = follower.Guide(here, heading, cabinet);
    Console.WriteLine(Invariant(
        $"waypoint  heading {heading:F0}, {guidance.DistanceM:F0} m to go: forward {guidance.Twist.Vx:F2} m/s, {Turning(guidance.Twist.Omega)}"));
}

Guidance atCabinet = follower.Guide(new Coordinate(-23.5610, 133.87039), 90.0f, cabinet);
string arrived = atCabinet.Arrived ? "arrived" : "still driving";
Console.WriteLine(Invariant($"waypoint  {atCabinet.DistanceM:F1} m from the cabinet: {arrived}"));

// Every command passes through the safety gate: held to 1 m/s, eased on at 0.5
// m/s^2, and stopped if no fresh command arrives for 0.3 s.
using var limits = new Limits(1.0f, 1.0f, 0.5f, 2.0f);
using var gate = new SafetyGate(limits, 0.3f);
var ahead = new Twist(0.8f);
var eased = new List<string>();
for (int i = 0; i < 3; i++)
{
    gate.Feed();
    eased.Add(Invariant($"{gate.Command(ahead, 0.1f).Vx:F2}"));
}

Console.WriteLine($"safety    asked for 0.80 m/s from rest, allowed {string.Join(", ", eased)} m/s");
string[] silence = Enumerable.Range(0, 3).Select(_ => Invariant($"{gate.Command(ahead, 0.1f).Vx:F2}")).ToArray();
Console.WriteLine($"safety    then no fresh command: {string.Join(", ", silence)} m/s, stopped once more than 0.3 s pass");
var veering = new Twist(0.8f, 0.0f, 0.2f);
Twist near = Kit.ObstacleStop(veering, 0.35f, 0.5f);
Console.WriteLine(Invariant($"safety    something 0.35 m ahead: forward {near.Vx:F2} m/s, still {Turning(near.Omega)}"));
Twist blind = Kit.ObstacleStop(veering, float.NaN, 0.5f);
Console.WriteLine(Invariant($"safety    no range reading at all: forward {blind.Vx:F2} m/s, still {Turning(blind.Omega)}"));
gate.Feed();
gate.EngageEstop();
Twist stopped = gate.Command(ahead, 0.1f);
Console.WriteLine(Invariant($"safety    e-stop engaged: {stopped.Vx:F2} m/s until a person resets it"));

// At the cabinet, a two-link arm presses the reset button 0.35 m out and 0.20 m up.
var arm = new TwoLinkArm(0.30f, 0.25f);
(float reachMin, float reachMax) = arm.Reach;
Console.WriteLine(Invariant($"arm       links 0.30 m and 0.25 m reach from {reachMin:F2} m to {reachMax:F2} m"));
(float shoulder, float elbow) = arm.JointsFor(0.35f, 0.20f, Elbow.Up)
    ?? throw new InvalidOperationException("the button is in reach");
Console.WriteLine(Invariant(
    $"arm       elbow up: shoulder {Degrees(shoulder):F1} degrees, elbow {Degrees(elbow):F1} degrees"));
Transform tool = Kit.ForwardKinematics(new DhParameters(A: 0.30f, Theta: shoulder), new DhParameters(A: 0.25f, Theta: elbow));
(float x, float y, _) = tool.Position;
Console.WriteLine(Invariant($"arm       forward kinematics puts the tip at {x:F2} m, {y:F2} m"));
string tooFar = arm.JointsFor(0.70f, 0.0f, Elbow.Up) is null ? "out of reach" : "in reach";
Console.WriteLine($"arm       a button 0.70 m out: {tooFar}");

// Hobby servos turn the joints. A joint angle of 0 is the servo's center, 90 degrees.
ServoMap servo = ServoMap.Standard;
ushort shoulderPulse = servo.Pulse((float)(90.0 + Degrees(shoulder)));
ushort elbowPulse = servo.Pulse((float)(90.0 + Degrees(elbow)));
Console.WriteLine($"servos    shoulder {shoulderPulse} us, elbow {elbowPulse} us");
Esc esc = Esc.Bidirectional;
Console.WriteLine($"motors    a quarter throttle forward is {esc.Pulse(0.25f)} us, a quarter back {esc.Pulse(-0.25f)} us");
```
<!-- end -->

## On a board

The rover's arm on a Raspberry Pi: the shoulder and elbow servos on a
[PCA9685](../hardware.md#pca9685) breakout on the header's I2C bus. The program reaches for
each control on the cabinet's panel with the same `TwoLinkArm` and `ServoMap` as the
example, holds each for two seconds, skips the one out of reach, and parks the arm in the
pose it was fitted in.

| PCA9685 breakout | Raspberry Pi |
| --- | --- |
| VCC | a 3V3 pin |
| GND | a ground pin |
| SDA | GPIO2 |
| SCL | GPIO3 |
| V+ | a 5 V supply, its ground joined to the Pi's |
| channel 0 | the shoulder servo |
| channel 1 | the elbow servo |

Fit each servo's horn while the servo sits at its center, a 1500 µs pulse, with the arm
straight out and level. A joint angle of 0 is then the servo's 90 degrees, the offset the
program adds. The servos draw from V+, not from the header: a servo draws the most current
as it starts to move and while it holds against a load, far more than the Pi should supply,
and the [actuators guide](actuators.md#on-a-board) explains the supply in full.
[Turn the I2C interface on](../boards/raspberry-pi.md#turning-the-buses-on) and check that
`i2cdetect -y 1` shows `40` before running anything. The PCA9685 makes the pulses itself,
so once the program exits the servos go on holding the arm where it parked for as long as
the board has power.

### Rust

<!-- snippet: examples/boards/raspberry-pi/src/bin/arm.rs#example -->
From [`examples/boards/raspberry-pi/src/bin/arm.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/bin/arm.rs):

```rust
use pamoja_actuators::pca9685::{self, Pca9685, Pwm};
use pamoja_hal::bus::I2cBus;
use pamoja_kit::{Elbow, ServoMap, TwoLinkArm};

/// The PCA9685 channels the shoulder and elbow servos are plugged into.
const SHOULDER_CHANNEL: u8 = 0;
const ELBOW_CHANNEL: u8 = 1;

/// The panel's controls, each in meters out from the shoulder and up from it.
const PANEL: [(&str, f32, f32); 4] = [
    ("reset button", 0.35, 0.20),
    ("breaker", 0.45, 0.10),
    ("door latch", 0.20, 0.35),
    ("fan switch", 0.70, 0.00),
];

fn main() -> Result<(), Box<dyn Error>> {
    // The PCA9685 on the header's I2C bus, at 50 Hz for the servos.
    let bus = I2cBus::open("/dev/i2c-1")?;
    let mut controller =
        Pca9685::new(bus.clone(), pca9685::DEFAULT_I2C_ADDRESS, bus.delay()).with_frequency(50);

    // Links of 0.30 m and 0.25 m. The servos were fitted with both joints at 0, so a joint
    // angle of 0 is each servo's center, 90 degrees.
    let arm = TwoLinkArm::new(0.30, 0.25);
    let servo = ServoMap::standard();
    let pulse = |joint: f32| u32::from(servo.pulse(90.0 + joint.to_degrees()));

    // Each control the arm can reach, it holds for two seconds; one it cannot, it skips rather
    // than drive a servo into its end stop.
    for (name, x, y) in PANEL {
        let Some((shoulder, elbow)) = arm.joints_for(x, y, Elbow::Up) else {
            println!("{name:12}  out of reach, skipped");
            continue;
        };
        controller.set_channel(SHOULDER_CHANNEL, Pwm::servo(pulse(shoulder), 50))?;
        controller.set_channel(ELBOW_CHANNEL, Pwm::servo(pulse(elbow), 50))?;
        println!(
            "{name:12}  shoulder {} us, elbow {} us",
            pulse(shoulder),
            pulse(elbow)
        );
        thread::sleep(Duration::from_secs(2));
    }

    // Back to the pose the arm was fitted in. The PCA9685 goes on sending both pulses after
    // the program exits, so the servos hold the arm there.
    controller.set_channel(SHOULDER_CHANNEL, Pwm::servo(pulse(0.0), 50))?;
    controller.set_channel(ELBOW_CHANNEL, Pwm::servo(pulse(0.0), 50))?;
    println!("parked        both servos at {} us", pulse(0.0));
    Ok(())
}
```
<!-- end -->

```sh
cd examples/boards/raspberry-pi
cargo run --release --bin arm
```

### TypeScript

<!-- snippet: bindings/node/boards/raspberry-pi/arm.ts#example -->
From [`bindings/node/boards/raspberry-pi/arm.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/boards/raspberry-pi/arm.ts):

```typescript
import { setTimeout as sleep } from 'node:timers/promises'
import { Pca9685, pca9685, pwm } from '@pamoja/actuators'
import { I2cBus } from '@pamoja/hal'
import { Elbow, ServoMap, TwoLinkArm } from '@pamoja/kit'

// The PCA9685 channels the shoulder and elbow servos are plugged into.
const SHOULDER_CHANNEL = 0
const ELBOW_CHANNEL = 1

// The panel's controls, each in meters out from the shoulder and up from it.
const PANEL = [
  { name: 'reset button', x: 0.35, y: 0.2 },
  { name: 'breaker', x: 0.45, y: 0.1 },
  { name: 'door latch', x: 0.2, y: 0.35 },
  { name: 'fan switch', x: 0.7, y: 0.0 },
] as const

async function main(): Promise<void> {
  // The PCA9685 on the header's I2C bus, at 50 Hz for the servos.
  const bus = I2cBus.open('/dev/i2c-1')
  const controller = new Pca9685(bus, pca9685.defaultAddress, { frequencyHz: 50 })

  // Links of 0.30 m and 0.25 m. The servos were fitted with both joints at 0, so a joint angle
  // of 0 is each servo's center, 90 degrees.
  const arm = new TwoLinkArm(0.3, 0.25)
  const servo = ServoMap.standard()
  const pulse = (joint: number) => servo.pulse(90 + (joint * 180) / Math.PI)

  // Each control the arm can reach, it holds for two seconds; one it cannot, it skips rather
  // than drive a servo into its end stop.
  for (const { name, x, y } of PANEL) {
    const joints = arm.jointsFor(x, y, Elbow.Up)
    if (joints === null) {
      console.log(`${name.padEnd(12)}  out of reach, skipped`)
      continue
    }
    await controller.setChannel(SHOULDER_CHANNEL, pwm.servo(pulse(joints.shoulder), 50))
    await controller.setChannel(ELBOW_CHANNEL, pwm.servo(pulse(joints.elbow), 50))
    console.log(`${name.padEnd(12)}  shoulder ${pulse(joints.shoulder)} us, elbow ${pulse(joints.elbow)} us`)
    await sleep(2_000)
  }

  // Back to the pose the arm was fitted in. The PCA9685 goes on sending both pulses after the
  // program exits, so the servos hold the arm there.
  await controller.setChannel(SHOULDER_CHANNEL, pwm.servo(pulse(0), 50))
  await controller.setChannel(ELBOW_CHANNEL, pwm.servo(pulse(0), 50))
  console.log(`parked        both servos at ${pulse(0)} us`)
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
```
<!-- end -->

```sh
npm --prefix bindings/node run boards
node bindings/node/build/boards/raspberry-pi/arm.js
```

### Python

<!-- snippet: bindings/python/boards/raspberry_pi/arm.py#example -->
From [`bindings/python/boards/raspberry_pi/arm.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/boards/raspberry_pi/arm.py):

```python
import math
import time

from pamoja.actuators import Pca9685, pca9685, pwm
from pamoja.hal import I2cBus
from pamoja.kit import Elbow, ServoMap, TwoLinkArm

# The PCA9685 channels the shoulder and elbow servos are plugged into.
SHOULDER_CHANNEL = 0
ELBOW_CHANNEL = 1

# The panel's controls, each in meters out from the shoulder and up from it.
PANEL = (
    ("reset button", 0.35, 0.20),
    ("breaker", 0.45, 0.10),
    ("door latch", 0.20, 0.35),
    ("fan switch", 0.70, 0.00),
)


def main() -> None:
    # The PCA9685 on the header's I2C bus, at 50 Hz for the servos.
    bus = I2cBus.open("/dev/i2c-1")
    controller = Pca9685(bus, pca9685.DEFAULT_ADDRESS, frequency_hz=50)

    # Links of 0.30 m and 0.25 m. The servos were fitted with both joints at 0, so a joint angle
    # of 0 is each servo's center, 90 degrees.
    arm = TwoLinkArm(0.30, 0.25)
    servo = ServoMap.standard()

    def pulse(joint: float) -> int:
        return servo.pulse(90.0 + math.degrees(joint))

    # Each control the arm can reach, it holds for two seconds; one it cannot, it skips rather
    # than drive a servo into its end stop.
    for name, x, y in PANEL:
        joints = arm.joints_for(x, y, Elbow.UP)
        if joints is None:
            print(f"{name:12}  out of reach, skipped")
            continue
        shoulder, elbow = joints
        controller.set_channel(SHOULDER_CHANNEL, pwm.servo(pulse(shoulder), 50))
        controller.set_channel(ELBOW_CHANNEL, pwm.servo(pulse(elbow), 50))
        print(f"{name:12}  shoulder {pulse(shoulder)} us, elbow {pulse(elbow)} us")
        time.sleep(2)

    # Back to the pose the arm was fitted in. The PCA9685 goes on sending both pulses after the
    # program exits, so the servos hold the arm there.
    controller.set_channel(SHOULDER_CHANNEL, pwm.servo(pulse(0.0), 50))
    controller.set_channel(ELBOW_CHANNEL, pwm.servo(pulse(0.0), 50))
    print(f"parked        both servos at {pulse(0.0)} us")


if __name__ == "__main__":
    main()
```
<!-- end -->

```sh
python bindings/python/boards/raspberry_pi/arm.py
```

### C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Arm.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Arm.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Arm.cs):

```csharp
using Pamoja.Actuators;
using Pamoja.Hal;
using Pamoja.Kit;

namespace Boards.RaspberryPi;

/// <summary>
/// The inspection rover's arm: a shoulder servo and an elbow servo on a PCA9685 board on the
/// header's I2C bus, reaching for the controls on an inverter cabinet's panel. Wire the PCA9685
/// board's SDA to GPIO2, SCL to GPIO3, VCC to 3V3, and GND to ground, with the shoulder servo on
/// channel 0, the elbow servo on channel 1, and V+ from a 5 V supply whose ground is joined to
/// the Pi's.
/// </summary>
public static class Arm
{
    // The PCA9685 channels the shoulder and elbow servos are plugged into.
    private const byte ShoulderChannel = 0;
    private const byte ElbowChannel = 1;

    // The panel's controls, each in meters out from the shoulder and up from it.
    private static readonly (string Name, float X, float Y)[] Panel =
    [
        ("reset button", 0.35f, 0.20f),
        ("breaker", 0.45f, 0.10f),
        ("door latch", 0.20f, 0.35f),
        ("fan switch", 0.70f, 0.00f),
    ];

    /// <summary>Reaches for each control on the panel, then parks the arm.</summary>
    public static void Run()
    {
        // The PCA9685 on the header's I2C bus, at 50 Hz for the servos.
        using I2cBus bus = I2cBus.Open("/dev/i2c-1");
        using var controller = new Pca9685(bus, Pca9685.DefaultAddress, frequencyHz: 50);

        // Links of 0.30 m and 0.25 m. The servos were fitted with both joints at 0, so a joint
        // angle of 0 is each servo's center, 90 degrees.
        var arm = new TwoLinkArm(0.30f, 0.25f);
        ServoMap servo = ServoMap.Standard;
        ushort Pulse(float joint) => servo.Pulse((float)(90.0 + joint * 180.0 / Math.PI));

        // Each control the arm can reach, it holds for two seconds; one it cannot, it skips
        // rather than drive a servo into its end stop.
        foreach ((string name, float x, float y) in Panel)
        {
            (float Shoulder, float Elbow)? joints = arm.JointsFor(x, y, Elbow.Up);
            if (joints is null)
            {
                Console.WriteLine($"{name,-12}  out of reach, skipped");
                continue;
            }

            (float shoulder, float elbow) = joints.Value;
            controller.SetChannel(ShoulderChannel, Pwm.Servo(Pulse(shoulder), 50));
            controller.SetChannel(ElbowChannel, Pwm.Servo(Pulse(elbow), 50));
            Console.WriteLine($"{name,-12}  shoulder {Pulse(shoulder)} us, elbow {Pulse(elbow)} us");
            Thread.Sleep(TimeSpan.FromSeconds(2));
        }

        // Back to the pose the arm was fitted in. The PCA9685 goes on sending both pulses after
        // the program exits, so the servos hold the arm there.
        controller.SetChannel(ShoulderChannel, Pwm.Servo(Pulse(0.0f), 50));
        controller.SetChannel(ElbowChannel, Pwm.Servo(Pulse(0.0f), 50));
        Console.WriteLine($"parked        both servos at {Pulse(0.0f)} us");
    }
}
```
<!-- end -->

```sh
dotnet run --project bindings/dotnet/samples/Pamoja.Boards -- raspberry-pi/arm
```

## Values at a glance

**The frames and units every helper agrees on.** They are the same in every helper and
every language:

| Quantity | Frame and unit |
| --- | --- |
| a twist's `vx` | forward speed, meters per second |
| a twist's `vy` | leftward speed, meters per second; drives that cannot strafe ignore it |
| a twist's `omega` | yaw rate, radians per second, positive turning left (counter-clockwise), as ROS's [REP-103](../about/standards.md#rep-103) has it |
| a pose's `x` and `y` | meters in the world frame |
| a pose's `theta` | radians counter-clockwise from the world x axis, wrapped into `(-pi, pi]` |
| a waypoint heading | compass degrees, clockwise from north |
| a guidance heading error | compass degrees in `(-180, 180]`, positive when the target is to the robot's right |
| an arm's `x` and `y` | meters from the shoulder, in the plane the arm moves in |
| a joint angle | radians, positive counter-clockwise; the shoulder's from the x axis, the elbow's from the first link |
| a servo angle | degrees, from 0 to the servo's travel |
| a pulse width | microseconds |

**Chassis models:**

| Helper | What it does |
| --- | --- |
| differential drive | two driven wheels a track apart: wheel speeds for a forward speed and turn rate, and back |
| skid steer | tracks or four fixed wheels that skid to turn: the same, with the track widened by a slip factor |
| Ackermann | a steered axle and a driven axle a wheelbase apart: the steering angle for a turn rate, and the turn rate, radius, and curvature for a steering angle |
| mecanum | four wheels with angled rollers: the four wheel speeds for any twist, strafing included, and back |

**Position helpers:**

| Helper | What it does |
| --- | --- |
| quadrature | counts an incremental encoder's steps from its A and B channels, in either direction |
| quadrature scale | turns steps into the distance and speed of the wheel they turn with |
| odometry | adds up motion into a pose, as the exact arc the robot drove, and nudges its heading toward an absolute one |

**Guidance and safety helpers:**

| Helper | What it does |
| --- | --- |
| waypoint follower | turns toward a GPS waypoint in proportion to the heading error and scales its speed by the cosine of that error, so it pivots in place while the target is behind it |
| obstacle stop | cuts forward and sideways speed when the nearest range is at or inside the stopping distance, keeping the turn |
| e-stop | a stop that latches until a person resets it |
| watchdog | a deadman timer that expires once more than its timeout passes unfed |
| limits | holds planar speed and yaw rate to ceilings, and eases toward them within acceleration limits |
| safety gate | an e-stop, a watchdog, and limits that every command passes through |

**Arm helpers:**

| Helper | What it does |
| --- | --- |
| two-link arm | a planar arm's hand position for two joint angles, and the joint angles for a hand position, elbow up or down |
| forward kinematics | the base-to-tool transform of any serial arm described joint by joint in the Denavit-Hartenberg convention |

**Output helpers:**

| Helper | What it does |
| --- | --- |
| servo map | the pulse width for a servo angle, and the angle for a pulse width; 1000 to 2000 µs over 180 degrees unless built otherwise |
| ESC | the pulse width a reversible speed controller reads for a throttle from -1 to 1; 1000, 1500, and 2000 µs unless built otherwise |

**The calls in each language:**

### Rust

Models are built once and asked; stateful helpers change through `&mut self`:

| Helper | Make it | Use it |
| --- | --- | --- |
| twist and pose | `Twist::new(vx, vy, omega)`, `Twist::planar(vx, omega)`, `Twist::zero()`; `Pose::new(x, y, theta)`, `Pose::origin()` | fields `vx`, `vy`, `omega`; `x`, `y`, `theta` |
| differential drive | `DiffDrive::new(track)` | `wheel_speeds(linear, angular)` gives `(left, right)`; `body_motion(left, right)` gives `(linear, angular)` |
| skid steer | `SkidSteer::new(track, slip)` | the same two calls |
| Ackermann | `Ackermann::new(wheelbase)` | `steering_angle(linear, angular)`, `yaw_rate(linear, steering)`, `turn_radius(steering)`, `curvature(steering)` |
| mecanum | `Mecanum::new(wheelbase, track)` | `wheel_speeds(twist)` gives `WheelSpeeds`; `body_motion(wheels)` gives a `Twist` |
| quadrature | `Quadrature::new()` or `Quadrature::starting(a, b)` | `update(a, b)` gives -1, 0, or 1; `count()`, `reset()` |
| quadrature scale | `QuadratureScale::new(counts_per_rev, wheel_radius)` | `distance(count)`, `velocity(delta_count, dt)` |
| odometry | `Odometry::at_origin()` or `Odometry::new(pose)` | `integrate(linear, angular, dt)`, `integrate_wheels(left, right, &drive)`, `fuse_heading(measured, weight)`; `pose()`, `reset(pose)` |
| waypoint follower | `WaypointFollower::new(cruise, arrival_m, heading_gain, max_angular)` | `guide(here, heading_deg, target)` gives a `Guidance` |
| obstacle stop | `obstacle_stop(twist, range_m, stop_distance_m)` | |
| e-stop | `EStop::new()` | `engage()`, `reset()`, `is_engaged()`, `gate(desired)` |
| watchdog | `Watchdog::new(timeout)` | `feed()`, `update(dt)` gives whether it has expired, `is_expired()` |
| limits | `Limits::new(max_linear, max_angular, max_linear_accel, max_angular_accel)` | `apply(desired, dt)`, `reset()` |
| safety gate | `SafetyGate::new(limits, timeout)` | `command(desired, dt)`, `feed()`, `engage_estop()`, `reset_estop()`, `is_stopped()` |
| two-link arm | `TwoLinkArm::new(l1, l2)` | `tip(shoulder, elbow)`, `joints_for(x, y, Elbow::Up)` gives an `Option`, `reach()` |
| forward kinematics | `forward_kinematics(&[DhParameters { a, alpha, d, theta }])` | the `Transform`'s `position()` |
| a joint's transform | `joint.transform()`, `Transform::identity()` | `multiply(&other)`, `position()`, and the sixteen elements, row by row, in `m` |
| servo map | `ServoMap::standard()` or `ServoMap::new(min_us, max_us, range_deg)` | `pulse(angle_deg)`, `angle(pulse_us)`; `min_us()`, `max_us()`, `range_deg()` |
| ESC | `Esc::bidirectional()` or `Esc::new(min_us, neutral_us, max_us)` | `pulse(throttle)`; `min_us()`, `neutral_us()`, `max_us()` |

### TypeScript

Plain values are objects; models and stateful helpers are classes:

| Helper | Make it | Use it |
| --- | --- | --- |
| twist and pose | `{ vx, vy, omega }`; `{ x, y, theta }` | the same fields |
| differential drive | `new DiffDrive(track)` | `wheelSpeeds(linear, angular)` gives `{ left, right }`; `bodyMotion(left, right)` gives `{ linear, angular }` |
| skid steer | `new SkidSteer(track, slip?)` | the same two calls |
| Ackermann | `new Ackermann(wheelbase)` | `steeringAngle(linear, angular)`, `yawRate(linear, steering)`, `turnRadius(steering)`, `curvature(steering)` |
| mecanum | `new Mecanum(wheelbase, track)` | `wheelSpeeds(twist)` gives `{ frontLeft, frontRight, rearLeft, rearRight }`; `bodyMotion(wheels)` gives a twist |
| quadrature | `new Quadrature(a?, b?)` | `update(a, b)` gives -1, 0, or 1; `count`, `reset()` |
| quadrature scale | `new QuadratureScale(countsPerRev, wheelRadius)` | `distance(count)`, `velocity(deltaCount, dt)` |
| odometry | `new Odometry(start?)` | `integrate(linear, angular, dt)`, `integrateWheels(left, right, drive)`, `fuseHeading(measured, weight)`; `pose`, `reset(pose)` |
| waypoint follower | `new WaypointFollower(cruise, arrivalM, headingGain, maxAngular)` | `guide(here, headingDeg, target)` gives `{ twist, distanceM, headingErrorDeg, arrived }` |
| obstacle stop | `obstacleStop(twist, rangeM, stopDistanceM)` | |
| e-stop | `new EStop()` | `engage()`, `reset()`, `isEngaged`, `gate(desired)` |
| watchdog | `new Watchdog(timeout)` | `feed()`, `update(dt)` gives whether it has expired, `isExpired` |
| limits | `new Limits(maxLinear, maxAngular, maxLinearAccel, maxAngularAccel)` | `apply(desired, dt)`, `reset()` |
| safety gate | `new SafetyGate(limits, timeout)` | `command(desired, dt)`, `feed()`, `engageEstop()`, `resetEstop()`, `isStopped` |
| two-link arm | `new TwoLinkArm(l1, l2)` | `tip(shoulder, elbow)` gives `{ x, y }`, `jointsFor(x, y, Elbow.Up)` gives `{ shoulder, elbow }` or `null`, `reach` gives `{ min, max }` |
| forward kinematics | `forwardKinematics([{ a, alpha, d, theta }])` | the `Transform`'s `position` |
| a joint's transform | `Transform.ofJoint(joint)`, `Transform.identity()` | `multiply(other)`, `position` gives `{ x, y, z }`, `elements` gives the sixteen, row by row |
| servo map | `ServoMap.standard()` or `new ServoMap(minUs, maxUs, rangeDeg)` | `pulse(angleDeg)`, `angle(pulseUs)`; `minUs`, `maxUs`, `rangeDeg` |
| ESC | `Esc.bidirectional()` or `new Esc(minUs, neutralUs, maxUs)` | `pulse(throttle)`; `minUs`, `neutralUs`, `maxUs` |

### Python

Plain values are frozen classes, and pairs are tuples:

| Helper | Make it | Use it |
| --- | --- | --- |
| twist and pose | `Twist(vx=0.0, vy=0.0, omega=0.0)`; `Pose(x=0.0, y=0.0, theta=0.0)` | the same names, read-only |
| differential drive | `DiffDrive(track)` | `wheel_speeds(linear, angular)` gives `(left, right)`; `body_motion(left, right)` gives `(linear, angular)` |
| skid steer | `SkidSteer(track, slip=1.0)` | the same two calls |
| Ackermann | `Ackermann(wheelbase)` | `steering_angle(linear, angular)`, `yaw_rate(linear, steering)`, `turn_radius(steering)`, `curvature(steering)` |
| mecanum | `Mecanum(wheelbase, track)` | `wheel_speeds(twist)` gives `WheelSpeeds`; `body_motion(wheels)` gives a `Twist` |
| quadrature | `Quadrature(a=False, b=False)` | `update(a, b)` gives -1, 0, or 1; `count`, `reset()` |
| quadrature scale | `QuadratureScale(counts_per_rev, wheel_radius)` | `distance(count)`, `velocity(delta_count, dt)` |
| odometry | `Odometry(start=None)` | `integrate(linear, angular, dt)`, `integrate_wheels(left, right, drive)`, `fuse_heading(measured, weight)`; `pose`, `reset(pose)` |
| waypoint follower | `WaypointFollower(cruise, arrival_m, heading_gain, max_angular)` | `guide(here, heading_deg, target)` gives a `Guidance` |
| obstacle stop | `obstacle_stop(twist, range_m, stop_distance_m)` | |
| e-stop | `EStop()` | `engage()`, `reset()`, `is_engaged`, `gate(desired)` |
| watchdog | `Watchdog(timeout)` | `feed()`, `update(dt)` gives whether it has expired, `is_expired` |
| limits | `Limits(max_linear, max_angular, max_linear_accel, max_angular_accel)` | `apply(desired, dt)`, `reset()` |
| safety gate | `SafetyGate(limits, timeout)` | `command(desired, dt)`, `feed()`, `engage_estop()`, `reset_estop()`, `is_stopped` |
| two-link arm | `TwoLinkArm(l1, l2)` | `tip(shoulder, elbow)` gives `(x, y)`, `joints_for(x, y, Elbow.UP)` gives `(shoulder, elbow)` or `None`, `reach` gives `(min, max)` |
| forward kinematics | `forward_kinematics([DhParameters(a=0.0, alpha=0.0, d=0.0, theta=0.0)])` | the `Transform`'s `position` |
| a joint's transform | `joint.transform()`, `Transform.identity()` | `multiply(other)`, `position` gives `(x, y, z)`, `elements` gives the sixteen, row by row |
| servo map | `ServoMap.standard()` or `ServoMap(min_us, max_us, range_deg)` | `pulse(angle_deg)`, `angle(pulse_us)`; `min_us`, `max_us`, `range_deg` |
| ESC | `Esc.bidirectional()` or `Esc(min_us, neutral_us, max_us)` | `pulse(throttle)`; `min_us`, `neutral_us`, `max_us` |

### C#

Models and values are record structs; stateful helpers are `IDisposable`:

| Helper | Make it | Use it |
| --- | --- | --- |
| twist and pose | `new Twist(vx, vy = 0, omega = 0)`; `new Pose(x, y, theta)` | `Vx`, `Vy`, `Omega`; `X`, `Y`, `Theta` |
| differential drive | `new DiffDrive(track)` | `WheelSpeeds(linear, angular)` gives `(Left, Right)`; `BodyMotion(left, right)` gives `(Linear, Angular)` |
| skid steer | `new SkidSteer(track, slip = 1)` | the same two calls |
| Ackermann | `new Ackermann(wheelbase)` | `SteeringAngle(linear, angular)`, `YawRate(linear, steering)`, `TurnRadius(steering)`, `Curvature(steering)` |
| mecanum | `new Mecanum(wheelbase, track)` | `WheelSpeeds(twist)` gives a `WheelSpeeds`; `BodyMotion(wheels)` gives a `Twist` |
| quadrature | `new Quadrature(a = false, b = false)` | `Update(a, b)` gives -1, 0, or 1; `Count`, `Reset()` |
| quadrature scale | `new QuadratureScale(countsPerRev, wheelRadius)` | `Distance(count)`, `Velocity(deltaCount, dt)` |
| odometry | `new Odometry(start = default)` | `Integrate(linear, angular, dt)`, `IntegrateWheels(left, right, drive)`, `FuseHeading(measured, weight)`; `Pose`, `Reset(pose)` |
| waypoint follower | `new WaypointFollower(cruise, arrivalM, headingGain, maxAngular)` | `Guide(here, headingDeg, target)` gives a `Guidance` |
| obstacle stop | `Kit.ObstacleStop(twist, rangeM, stopDistanceM)` | |
| e-stop | `new EStop()` | `Engage()`, `Reset()`, `IsEngaged`, `Gate(desired)` |
| watchdog | `new Watchdog(timeout)` | `Feed()`, `Update(dt)` gives whether it has expired, `IsExpired` |
| limits | `new Limits(maxLinear, maxAngular, maxLinearAccel, maxAngularAccel)` | `Apply(desired, dt)`, `Reset()` |
| safety gate | `new SafetyGate(limits, timeout)` | `Command(desired, dt)`, `Feed()`, `EngageEstop()`, `ResetEstop()`, `IsStopped` |
| two-link arm | `new TwoLinkArm(l1, l2)` | `Tip(shoulder, elbow)` gives `(X, Y)`, `JointsFor(x, y, Elbow.Up)` gives `(Shoulder, Elbow)?`, `Reach` gives `(Min, Max)` |
| forward kinematics | `Kit.ForwardKinematics(new DhParameters(A: a, Theta: theta))` | the `Transform`'s `Position` |
| a joint's transform | `joint.Transform()`, `Transform.Identity` | `Multiply(other)`, `Position` gives `(X, Y, Z)`, `Elements` gives the sixteen, row by row |
| servo map | `ServoMap.Standard` or `new ServoMap(minUs, maxUs, rangeDeg)` | `Pulse(angleDeg)`, `Angle(pulseUs)`; `MinUs`, `MaxUs`, `RangeDeg` |
| ESC | `Esc.Bidirectional` or `new Esc(minUs, neutralUs, maxUs)` | `Pulse(throttle)`; `MinUs`, `NeutralUs`, `MaxUs` |

<!-- languages end -->

**The parameters, and what a value out of range does.** As with the other helpers, a motion
helper never refuses a parameter the core can read. It falls back to the behavior the
right column describes, and where a value could move the robot, it falls back toward
stopping:

| Parameter | Means | Out of range |
| --- | --- | --- |
| a track, wheelbase, link length, wheel radius, or encoder resolution | a size in meters, or steps a turn | its magnitude is used |
| skid-steer `slip` | how much wider the effective track is than the real one, found by driving the robot and comparing | its magnitude is used; 0 is taken as 1, no slip |
| waypoint `cruise`, `heading_gain`, `max_angular`, `arrival_m` | the forward speed, the yaw rate per radian of error, its ceiling, and how close counts as there | each magnitude is used |
| obstacle `stop_distance_m` | the range at or inside which forward motion is cut | its magnitude is used |
| limits ceilings | the largest planar speed, yaw rate, and change in each per second | each magnitude is used; NaN is 0, which holds the robot still |
| watchdog `timeout` | the silence allowed before a stop | its magnitude is used; NaN is 0, so any time passing expires it |
| odometry `weight` | how strongly to trust an absolute heading, 0 to 1 | clamped to 0 to 1; NaN leaves the heading alone |
| servo `range_deg` | the servo's full travel, in degrees | its magnitude is used |
| servo angle | degrees, 0 to the servo's travel | held to the travel; NaN gives 0, no pulse at all |
| a pulse to read back as an angle | microseconds | held to the map's range |
| ESC throttle | -1, full reverse, to 1, full forward | held to -1 to 1; NaN gives the neutral pulse |
| a pulse width in TypeScript or Python | microseconds, 0 to 65535 | refused, as the next section shows |

A servo map built with its first pulse width above its second, 2000 then 1000 µs, runs the
other way, for a servo mounted facing the other side of its joint.

**What a reading that is not a number does.** A robot must not move on a number it cannot
trust, so the helpers that decide motion turn one into a stop rather than pass it on:

| Helper | Given NaN or infinity |
| --- | --- |
| obstacle stop | a range that is NaN is an obstacle, so forward and sideways speed are cut; infinity, which some sensors report when nothing is in range, is clear |
| waypoint follower | a lost fix or heading gives a zero twist |
| watchdog and safety gate | a time step that is not finite expires the watchdog, and the gate stops until it is fed |
| limits | a command part that is not finite is taken as 0, so the robot eases to a stop; a time step that is not finite allows no change that call |
| odometry | a speed, rate, time step, or distance that is not finite is ignored, and the pose kept |
| servo map | an angle that is NaN gives 0, no pulse, so the servo stops being driven |
| ESC | a throttle that is NaN gives the neutral pulse, which stops the motor |
| two-link arm | a target that is not finite has no solution |
| chassis models, `tip`, and forward kinematics | plain arithmetic, so NaN in gives NaN out; the safety gate before them turns a NaN command into a stop, and the ESC after them turns a NaN throttle into neutral |

## When it goes wrong

What the bindings refuse:

| What happened | The message | Where |
| --- | --- | --- |
| a servo or ESC pulse width outside 0 to 65535, or with a fraction | `minUs must be a whole number of microseconds from 0 to 65535, not -1` | TypeScript; Python names `min_us` and raises `ValueError`, and a fraction there raises `TypeError` before the helper sees it |
| a step count with a fraction, or past what a JavaScript number holds exactly | `count must be a whole number of steps, not 1.5` | TypeScript; a fraction raises `TypeError` in Python |
| an elbow that is not up or down | `elbow must be "up" or "down", not "sideways"` | Python, for a plain string, as a `ValueError`; TypeScript's types refuse it before the call |

Rust and C# have none of these to show: a pulse width is a `u16` or `ushort`, a step count
an `i64` or `long`, and an elbow an enum, so a wrong one does not compile.

The mistakes that cost an afternoon:

- **The robot turns the wrong way.** Two angle conventions meet in robot code. A twist's
  yaw rate and a pose's heading turn counter-clockwise for a positive number, as ROS has
  it; a compass course, which a GPS or a magnetometer gives, turns clockwise from north.
  The waypoint follower takes a compass heading and returns a ROS twist, so a target to the
  right gives a negative yaw rate. `fuse_heading` takes the pose's own frame: with the
  world x axis pointing east, as REP-103 lays out a map, pass pi/2 minus the course in
  radians.
- **The encoder counts backward.** Which channel leads depends on how the encoder is wired
  and which way it is mounted. If driving forward counts down, swap the A and B inputs.
- **The encoder misses steps.** When both channels change between two reads, the decoder
  cannot tell the direction and counts 0 for that change. Read the channels faster than
  the fastest wheel can step them, from an edge interrupt where the board has one.
- **A tracked robot turns less than asked.** Tracks and fixed four-wheel bases skid to
  turn, so the geometric track under-predicts the turn. Measure it: command a spin, time a
  full turn, and set `slip` to the commanded rate over the measured one.
- **A car-like rover will not turn in place.** An Ackermann vehicle turns only while it
  moves; asked for a yaw rate at zero speed, it steers straight ahead.
- **A mecanum base strafes the wrong way.** These equations have the front-left and
  rear-right wheels run backward to strafe left, which holds for one of the two ways the
  wheels can be fitted. Fitted the other way, the base strafes the wrong way and turns
  wrongly too, and no sign change in software undoes it: swap the left wheels with the
  right ones.
- **The arm's tip lands somewhere else.** `forward_kinematics` uses the classic
  Denavit-Hartenberg convention, which fixes each link's frame at its far end. The modified
  convention, which some textbooks and arm makers use instead, fixes it at the near end, so
  its `a` and `alpha` belong to the link before; a table written for it gives a different
  tip here. Angles are radians and lengths meters.
- **A servo slams to one end.** A servo angle runs from 0 to its travel, so a joint angle of
  0 is usually the servo's center, 90 degrees. Add that offset, as the example does, rather
  than passing the joint angle straight through.
- **A servo turns the wrong way.** A servo mounted facing the other side of its joint turns
  opposite to the arm's convention. Build its map with the pulse widths swapped, from 2000
  down to 1000 µs, rather than negating angles through the program.
- **The robot stops for no reason.** The safety gate stops once its watchdog goes unfed for
  longer than the timeout. Feed it every time a fresh command arrives, and nowhere else, or
  the watchdog guards nothing.
- **Odometry drifts.** Dead reckoning adds every small wheel error to the pose for good.
  Correct the heading from an absolute source with `fuse_heading`, and reset the pose when
  a GPS fix arrives.

## Where next

<!-- table: next motion -->
- [Simulators](sim.md): Noisy and replay sensors, a recording actuator, a simulated robot that dead-reckons its pose, and a link that loses sends on a pattern.
- [ROS 2 rules](ros2.md): ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed.
- [Actuator drivers](actuators.md): A PCA9685 driver for servos, LEDs, and valves, and stepper drivers for four coil lines or a step and direction chip, in every language.
- Also in Profiles and robotics: [Device profiles](profile.md), [Rules](rules.md), [Zenoh keys](zenoh.md).
<!-- end -->

## Reference

<!-- table: reference motion -->
- Rust: `DiffDrive`, `Odometry`, `WaypointFollower`, and `SafetyGate` in [`pamoja-kit`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-motion)
- TypeScript: [`@pamoja/kit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-motion)
- Python: [`pamoja.kit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-motion)
- C#: [`Pamoja.Kit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-motion)
<!-- end -->
