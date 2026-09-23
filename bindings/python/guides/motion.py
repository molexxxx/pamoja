"""The robot motion guide example; see docs/guides/motion.md."""

# ANCHOR: example
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
# ANCHOR_END: example

assert steps == [1, 1, 1, 1, -1]
assert encoder.count == 3
assert at_cabinet.arrived
assert silence == ["0.20", "0.25", "0.00"]
assert stopped.vx == 0.0 and near.vx == 0.0 and blind.vx == 0.0
assert abs(x - 0.35) < 1e-4 and abs(y - 0.20) < 1e-4
