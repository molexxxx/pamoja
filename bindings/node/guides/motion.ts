// The robot motion guide example; see docs/guides/motion.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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
// ANCHOR_END: example

assert.deepEqual(steps, [1, 1, 1, 1, -1])
assert.equal(encoder.count, 3)
assert.equal(atCabinet.arrived, true)
assert.deepEqual(silence, ['0.20', '0.25', '0.00'])
assert.ok(stopped.vx === 0 && near.vx === 0 && blind.vx === 0)
assert.ok(Math.abs(tip.x - 0.35) < 1e-4 && Math.abs(tip.y - 0.2) < 1e-4)
