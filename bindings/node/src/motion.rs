//! Generated Node bindings for the motion helpers: chassis kinematics, an arm, odometry,
//! waypoint guidance, the safety gate, and the conversions to servo, ESC, and encoder units.
//!
//! These bind the robotics half of `pamoja-kit`. A pose, a twist, and the other plain
//! values cross as objects; the kinematic models and the stateful helpers are classes. A
//! reading that is not a finite number never moves a robot: the helpers stop, hold, or
//! ignore it, as the Rust crate documents.

use napi_derive::napi;
use pamoja_kit::{
    forward_kinematics as core_forward_kinematics, obstacle_stop as core_obstacle_stop,
    Ackermann as CoreAckermann, DhParameters as CoreDh, DiffDrive as CoreDiffDrive,
    EStop as CoreEStop, Elbow as CoreElbow, Esc as CoreEsc, Limits as CoreLimits,
    Mecanum as CoreMecanum, Odometry as CoreOdometry, Pose as CorePose,
    Quadrature as CoreQuadrature, QuadratureScale as CoreScale, SafetyGate as CoreGate,
    ServoMap as CoreServo, SkidSteer as CoreSkidSteer, Transform as CoreTransform,
    Twist as CoreTwist, TwoLinkArm as CoreArm, Watchdog as CoreWatchdog,
    WaypointFollower as CoreFollower, WheelSpeeds as CoreWheels,
};

use crate::kit::Coord;

/// Where a robot is and which way it faces.
#[napi(object)]
pub struct Pose {
    /// Position along the world x axis, in meters.
    pub x: f64,
    /// Position along the world y axis, in meters.
    pub y: f64,
    /// Heading from the world x axis, in radians, positive counter-clockwise.
    pub theta: f64,
}

impl From<Pose> for CorePose {
    fn from(value: Pose) -> Self {
        CorePose::new(value.x as f32, value.y as f32, value.theta as f32)
    }
}

impl From<CorePose> for Pose {
    fn from(value: CorePose) -> Self {
        Self {
            x: f64::from(value.x),
            y: f64::from(value.y),
            theta: f64::from(value.theta),
        }
    }
}

/// How fast a robot is asked to move.
#[napi(object)]
pub struct Twist {
    /// Forward speed along the x axis.
    pub vx: f64,
    /// Leftward speed along the y axis; zero for drives that cannot strafe.
    pub vy: f64,
    /// Yaw rate about the z axis, positive counter-clockwise.
    pub omega: f64,
}

impl From<Twist> for CoreTwist {
    fn from(value: Twist) -> Self {
        CoreTwist::new(value.vx as f32, value.vy as f32, value.omega as f32)
    }
}

impl From<CoreTwist> for Twist {
    fn from(value: CoreTwist) -> Self {
        Self {
            vx: f64::from(value.vx),
            vy: f64::from(value.vy),
            omega: f64::from(value.omega),
        }
    }
}

/// The speeds of a two-sided drive's left and right wheels or tracks.
#[napi(object)]
pub struct SideSpeeds {
    /// The left side's speed.
    pub left: f64,
    /// The right side's speed.
    pub right: f64,
}

/// A planar body motion: a forward speed and a yaw rate.
#[napi(object)]
pub struct BodyMotion {
    /// The forward speed.
    pub linear: f64,
    /// The yaw rate, positive turning left.
    pub angular: f64,
}

/// Wheel speeds for a desired body motion, and the body motion measured wheel speeds make,
/// for a robot that steers by spinning two wheels at different speeds.
#[napi]
pub struct DiffDrive {
    inner: CoreDiffDrive,
}

#[napi]
impl DiffDrive {
    /// Creates a model for wheels `track` apart; its magnitude is used.
    #[napi(constructor)]
    pub fn new(track: f64) -> Self {
        Self {
            inner: CoreDiffDrive::new(track as f32),
        }
    }

    /// Returns the left and right wheel speeds for a forward speed and a turn rate.
    #[napi]
    pub fn wheel_speeds(&self, linear: f64, angular: f64) -> SideSpeeds {
        let (left, right) = self.inner.wheel_speeds(linear as f32, angular as f32);
        SideSpeeds {
            left: f64::from(left),
            right: f64::from(right),
        }
    }

    /// Returns the forward speed and turn rate measured wheel speeds make.
    #[napi]
    pub fn body_motion(&self, left: f64, right: f64) -> BodyMotion {
        let (linear, angular) = self.inner.body_motion(left as f32, right as f32);
        BodyMotion {
            linear: f64::from(linear),
            angular: f64::from(angular),
        }
    }
}

/// Car-like steering: one steered axle and a driven axle a wheelbase apart.
#[napi]
pub struct Ackermann {
    inner: CoreAckermann,
}

#[napi]
impl Ackermann {
    /// Creates a model for axles `wheelbase` apart; its magnitude is used.
    #[napi(constructor)]
    pub fn new(wheelbase: f64) -> Self {
        Self {
            inner: CoreAckermann::new(wheelbase as f32),
        }
    }

    /// Returns the steering angle, in radians, for a forward speed and a yaw rate, or 0
    /// while stopped.
    #[napi]
    pub fn steering_angle(&self, linear: f64, angular: f64) -> f64 {
        f64::from(self.inner.steering_angle(linear as f32, angular as f32))
    }

    /// Returns the yaw rate a forward speed and a steering angle, in radians, produce.
    #[napi]
    pub fn yaw_rate(&self, linear: f64, steering: f64) -> f64 {
        f64::from(self.inner.yaw_rate(linear as f32, steering as f32))
    }

    /// Returns the turn radius of a steering angle, in meters, or `Infinity` with the wheels
    /// straight.
    #[napi]
    pub fn turn_radius(&self, steering: f64) -> f64 {
        f64::from(self.inner.turn_radius(steering as f32))
    }

    /// Returns the path curvature of a steering angle, the reciprocal of the turn radius.
    #[napi]
    pub fn curvature(&self, steering: f64) -> f64 {
        f64::from(self.inner.curvature(steering as f32))
    }
}

/// A tracked or four-wheel drive that turns by skidding, with its track widened by a slip
/// factor.
#[napi]
pub struct SkidSteer {
    inner: CoreSkidSteer,
}

#[napi]
impl SkidSteer {
    /// Creates a model for sides `track` apart with a `slip` factor, 1 unless given; each
    /// magnitude is used, and a slip of 0 is taken as 1.
    #[napi(constructor)]
    pub fn new(track: f64, slip: Option<f64>) -> Self {
        Self {
            inner: CoreSkidSteer::new(track as f32, slip.unwrap_or(1.0) as f32),
        }
    }

    /// Returns the left and right speeds for a forward speed and a yaw rate.
    #[napi]
    pub fn wheel_speeds(&self, linear: f64, angular: f64) -> SideSpeeds {
        let (left, right) = self.inner.wheel_speeds(linear as f32, angular as f32);
        SideSpeeds {
            left: f64::from(left),
            right: f64::from(right),
        }
    }

    /// Returns the forward speed and yaw rate measured side speeds make.
    #[napi]
    pub fn body_motion(&self, left: f64, right: f64) -> BodyMotion {
        let (linear, angular) = self.inner.body_motion(left as f32, right as f32);
        BodyMotion {
            linear: f64::from(linear),
            angular: f64::from(angular),
        }
    }
}

/// The four wheel speeds of a mecanum base.
#[napi(object)]
pub struct WheelSpeeds {
    /// The front-left wheel's speed.
    pub front_left: f64,
    /// The front-right wheel's speed.
    pub front_right: f64,
    /// The rear-left wheel's speed.
    pub rear_left: f64,
    /// The rear-right wheel's speed.
    pub rear_right: f64,
}

/// A four-wheel mecanum base, which drives, strafes, and turns at once.
#[napi]
pub struct Mecanum {
    inner: CoreMecanum,
}

#[napi]
impl Mecanum {
    /// Creates a model from the front-to-rear `wheelbase` and side-to-side `track`; each
    /// magnitude is used.
    #[napi(constructor)]
    pub fn new(wheelbase: f64, track: f64) -> Self {
        Self {
            inner: CoreMecanum::new(wheelbase as f32, track as f32),
        }
    }

    /// Returns the four wheel speeds for a body twist.
    #[napi]
    pub fn wheel_speeds(&self, twist: Twist) -> WheelSpeeds {
        let wheels = self.inner.wheel_speeds(twist.into());
        WheelSpeeds {
            front_left: f64::from(wheels.front_left),
            front_right: f64::from(wheels.front_right),
            rear_left: f64::from(wheels.rear_left),
            rear_right: f64::from(wheels.rear_right),
        }
    }

    /// Returns the body twist measured wheel speeds make.
    #[napi]
    pub fn body_motion(&self, wheels: WheelSpeeds) -> Twist {
        self.inner
            .body_motion(CoreWheels {
                front_left: wheels.front_left as f32,
                front_right: wheels.front_right as f32,
                rear_left: wheels.rear_left as f32,
                rear_right: wheels.rear_right as f32,
            })
            .into()
    }
}

/// Which way a two-link arm's elbow bends.
#[napi(string_enum = "lowercase")]
pub enum Elbow {
    /// The elbow angle is positive, counter-clockwise.
    Up,
    /// The elbow angle is negative, clockwise.
    Down,
}

/// The closest and farthest an arm's hand reaches from its shoulder.
#[napi(object)]
pub struct Reach {
    /// The closest reach, the difference of the link lengths.
    pub min: f64,
    /// The farthest reach, the sum of the link lengths.
    pub max: f64,
}

/// A point in a plane.
#[napi(object)]
pub struct Point {
    /// The x coordinate.
    pub x: f64,
    /// The y coordinate.
    pub y: f64,
}

/// A two-link arm's joint angles, in radians.
#[napi(object)]
pub struct Joints {
    /// The shoulder angle, from the x axis.
    pub shoulder: f64,
    /// The elbow angle, relative to the first link.
    pub elbow: f64,
}

/// A planar arm of two links, with a closed-form inverse.
#[napi]
pub struct TwoLinkArm {
    inner: CoreArm,
}

#[napi]
impl TwoLinkArm {
    /// Creates an arm from its shoulder and elbow link lengths; each magnitude is used.
    #[napi(constructor)]
    pub fn new(l1: f64, l2: f64) -> Self {
        Self {
            inner: CoreArm::new(l1 as f32, l2 as f32),
        }
    }

    /// The closest and farthest the hand reaches.
    #[napi(getter)]
    pub fn reach(&self) -> Reach {
        let (min, max) = self.inner.reach();
        Reach {
            min: f64::from(min),
            max: f64::from(max),
        }
    }

    /// Returns where the hand is for a shoulder and an elbow angle, in radians.
    #[napi]
    pub fn tip(&self, shoulder: f64, elbow: f64) -> Point {
        let (x, y) = self.inner.tip(shoulder as f32, elbow as f32);
        Point {
            x: f64::from(x),
            y: f64::from(y),
        }
    }

    /// Returns the joint angles that put the hand at a point, or `null` for a point out of
    /// reach, an arm with a link of no length, or a coordinate that is not a finite number.
    #[napi]
    pub fn joints_for(&self, x: f64, y: f64, elbow: Elbow) -> Option<Joints> {
        let branch = match elbow {
            Elbow::Up => CoreElbow::Up,
            Elbow::Down => CoreElbow::Down,
        };
        self.inner
            .joints_for(x as f32, y as f32, branch)
            .map(|(shoulder, elbow)| Joints {
                shoulder: f64::from(shoulder),
                elbow: f64::from(elbow),
            })
    }
}

/// One joint of a serial arm in the Denavit-Hartenberg convention.
#[napi(object)]
pub struct DhParameters {
    /// The link length along the common normal, in meters.
    pub a: f64,
    /// The link twist about the common normal, in radians.
    pub alpha: f64,
    /// The link offset along the previous z axis, in meters.
    pub d: f64,
    /// The joint angle about the previous z axis, in radians.
    pub theta: f64,
}

impl From<&DhParameters> for CoreDh {
    fn from(value: &DhParameters) -> Self {
        CoreDh {
            a: value.a as f32,
            alpha: value.alpha as f32,
            d: value.d as f32,
            theta: value.theta as f32,
        }
    }
}

/// A point in space.
#[napi(object)]
pub struct Position {
    /// The x coordinate.
    pub x: f64,
    /// The y coordinate.
    pub y: f64,
    /// The z coordinate.
    pub z: f64,
}

/// A 4x4 homogeneous transform: a rotation and a translation.
#[napi]
pub struct Transform {
    inner: CoreTransform,
}

#[napi]
impl Transform {
    /// The identity: no rotation and no translation.
    #[napi(factory)]
    pub fn identity() -> Self {
        Self {
            inner: CoreTransform::identity(),
        }
    }

    /// The transform one Denavit-Hartenberg joint makes.
    #[napi(factory)]
    pub fn of_joint(joint: DhParameters) -> Self {
        Self {
            inner: CoreDh::from(&joint).transform(),
        }
    }

    /// Returns `this * other`, the transform that applies `other` and then this one.
    #[napi]
    pub fn multiply(&self, other: &Transform) -> Transform {
        Transform {
            inner: self.inner.multiply(&other.inner),
        }
    }

    /// Where this transform places the origin: its translation.
    #[napi(getter)]
    pub fn position(&self) -> Position {
        let (x, y, z) = self.inner.position();
        Position {
            x: f64::from(x),
            y: f64::from(y),
            z: f64::from(z),
        }
    }

    /// The sixteen elements, row-major.
    #[napi(getter)]
    pub fn elements(&self) -> Vec<f64> {
        self.inner.m.iter().copied().map(f64::from).collect()
    }
}

/// Returns the transform from a serial arm's base to its tool, or the identity for an arm
/// with no joints.
#[napi]
pub fn forward_kinematics(joints: Vec<DhParameters>) -> Transform {
    let chain: Vec<CoreDh> = joints.iter().map(CoreDh::from).collect();
    Transform {
        inner: core_forward_kinematics(&chain),
    }
}

/// Tracks a robot's pose by adding up its motion.
#[napi]
pub struct Odometry {
    inner: CoreOdometry,
}

#[napi]
impl Odometry {
    /// Creates an estimate starting at `start`, or at the origin facing along x.
    #[napi(constructor)]
    pub fn new(start: Option<Pose>) -> Self {
        let start = start.map(CorePose::from).unwrap_or_else(CorePose::origin);
        Self {
            inner: CoreOdometry::new(start),
        }
    }

    /// The pose so far.
    #[napi(getter)]
    pub fn pose(&self) -> Pose {
        self.inner.pose().into()
    }

    /// Sets the estimate to a known pose.
    #[napi]
    pub fn reset(&mut self, pose: Pose) {
        self.inner.reset(pose.into());
    }

    /// Adds a forward speed and yaw rate held for `dt`, and returns the new pose.
    #[napi]
    pub fn integrate(&mut self, linear: f64, angular: f64, dt: f64) -> Pose {
        self.inner
            .integrate(linear as f32, angular as f32, dt as f32)
            .into()
    }

    /// Adds the distances two wheels rolled, through a differential drive, and returns the
    /// new pose.
    #[napi]
    pub fn integrate_wheels(&mut self, left: f64, right: f64, drive: &DiffDrive) -> Pose {
        self.inner
            .integrate_wheels(left as f32, right as f32, &drive.inner)
            .into()
    }

    /// Nudges the heading toward an absolute measurement by `weight`, from 0 to 1.
    #[napi]
    pub fn fuse_heading(&mut self, measured: f64, weight: f64) {
        self.inner.fuse_heading(measured as f32, weight as f32);
    }
}

/// The command toward a waypoint, with the geometry behind it.
#[napi(object)]
pub struct Guidance {
    /// The body motion to drive.
    pub twist: Twist,
    /// The distance left to the target, in meters.
    pub distance_m: f64,
    /// The heading error to the target, in degrees, in `(-180, 180]`.
    pub heading_error_deg: f64,
    /// Whether the target is within the arrival radius.
    pub arrived: bool,
}

/// Steers toward a waypoint: pivot toward it, then drive, slowing as the heading error
/// grows.
#[napi]
pub struct WaypointFollower {
    inner: CoreFollower,
}

#[napi]
impl WaypointFollower {
    /// Creates a follower. Each magnitude is used.
    ///
    /// @param cruise - the forward speed when pointed at the target.
    /// @param arrivalM - how close, in meters, counts as arrived.
    /// @param headingGain - the yaw rate commanded per radian of heading error.
    /// @param maxAngular - the largest yaw rate to command.
    #[napi(constructor)]
    pub fn new(cruise: f64, arrival_m: f64, heading_gain: f64, max_angular: f64) -> Self {
        Self {
            inner: CoreFollower::new(
                cruise as f32,
                arrival_m,
                heading_gain as f32,
                max_angular as f32,
            ),
        }
    }

    /// Returns the command from a position and a compass heading, in degrees clockwise from
    /// north, toward a target.
    #[napi]
    pub fn guide(&self, here: Coord, heading_deg: f64, target: Coord) -> Guidance {
        let guidance = self
            .inner
            .guide(here.into(), heading_deg as f32, target.into());
        Guidance {
            twist: guidance.twist.into(),
            distance_m: guidance.distance_m,
            heading_error_deg: f64::from(guidance.heading_error_deg),
            arrived: guidance.arrived,
        }
    }
}

/// Cuts forward and sideways motion when an obstacle is within `stopDistanceM`, keeping the
/// turn. A range that is not a number counts as an obstacle.
#[napi]
pub fn obstacle_stop(twist: Twist, range_m: f64, stop_distance_m: f64) -> Twist {
    core_obstacle_stop(twist.into(), range_m as f32, stop_distance_m as f32).into()
}

/// An emergency stop that latches until a person resets it.
#[napi]
pub struct EStop {
    inner: CoreEStop,
}

#[napi]
impl EStop {
    /// Creates an e-stop that is not engaged.
    #[napi(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: CoreEStop::new(),
        }
    }

    /// Engages the stop; it holds until reset.
    #[napi]
    pub fn engage(&mut self) {
        self.inner.engage();
    }

    /// Clears the stop.
    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Whether the stop is engaged.
    #[napi(getter)]
    pub fn is_engaged(&self) -> bool {
        self.inner.is_engaged()
    }

    /// Returns `desired` while clear, or a zero twist while engaged.
    #[napi]
    pub fn gate(&self, desired: Twist) -> Twist {
        self.inner.gate(desired.into()).into()
    }
}

/// A deadman timer that expires unless fed often enough.
#[napi]
pub struct Watchdog {
    inner: CoreWatchdog,
}

#[napi]
impl Watchdog {
    /// Creates a watchdog that expires after `timeout` without being fed; a timeout that is
    /// not a number is taken as 0.
    #[napi(constructor)]
    pub fn new(timeout: f64) -> Self {
        Self {
            inner: CoreWatchdog::new(timeout as f32),
        }
    }

    /// Feeds the watchdog, restarting its silence timer.
    #[napi]
    pub fn feed(&mut self) {
        self.inner.feed();
    }

    /// Advances the timer by `dt` and returns whether it has expired; a `dt` that is not a
    /// finite number expires it until it is fed.
    #[napi]
    pub fn update(&mut self, dt: f64) -> bool {
        self.inner.update(dt as f32)
    }

    /// Whether the watchdog has expired.
    #[napi(getter)]
    pub fn is_expired(&self) -> bool {
        self.inner.is_expired()
    }
}

/// Speed and acceleration limits, with the motion they last allowed.
#[napi]
pub struct Limits {
    inner: CoreLimits,
}

#[napi]
impl Limits {
    /// Creates limits starting from rest. Each magnitude is used, and one that is not a
    /// number is taken as 0, which holds the robot still.
    ///
    /// @param maxLinear - the largest planar speed.
    /// @param maxAngular - the largest yaw rate.
    /// @param maxLinearAccel - the largest change in speed per second.
    /// @param maxAngularAccel - the largest change in yaw rate per second.
    #[napi(constructor)]
    pub fn new(
        max_linear: f64,
        max_angular: f64,
        max_linear_accel: f64,
        max_angular_accel: f64,
    ) -> Self {
        Self {
            inner: CoreLimits::new(
                max_linear as f32,
                max_angular as f32,
                max_linear_accel as f32,
                max_angular_accel as f32,
            ),
        }
    }

    /// Returns `desired` held to the speed limits and eased toward within the acceleration
    /// limits over `dt`.
    #[napi]
    pub fn apply(&mut self, desired: Twist, dt: f64) -> Twist {
        self.inner.apply(desired.into(), dt as f32).into()
    }

    /// Forgets the motion last allowed, so the next command eases up from rest.
    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

/// The gate every motion command passes through: an e-stop, a watchdog, and limits.
#[napi]
pub struct SafetyGate {
    inner: CoreGate,
}

#[napi]
impl SafetyGate {
    /// Creates a gate from limits, copied as they stand, and a watchdog timeout.
    #[napi(constructor)]
    pub fn new(limits: &Limits, watchdog_timeout: f64) -> Self {
        Self {
            inner: CoreGate::new(limits.inner, watchdog_timeout as f32),
        }
    }

    /// Feeds the watchdog; call it whenever a fresh command arrives.
    #[napi]
    pub fn feed(&mut self) {
        self.inner.feed();
    }

    /// Engages the latching e-stop.
    #[napi]
    pub fn engage_estop(&mut self) {
        self.inner.engage_estop();
    }

    /// Clears the e-stop.
    #[napi]
    pub fn reset_estop(&mut self) {
        self.inner.reset_estop();
    }

    /// Whether the gate is forcing a stop: its e-stop is engaged or its watchdog expired.
    #[napi(getter)]
    pub fn is_stopped(&self) -> bool {
        self.inner.is_stopped()
    }

    /// Returns the command that is safe to drive: a zero twist while stopped, otherwise
    /// `desired` bounded by the limits.
    #[napi]
    pub fn command(&mut self, desired: Twist, dt: f64) -> Twist {
        self.inner.command(desired.into(), dt as f32).into()
    }
}

/// Reads a pulse width a caller passed, refusing one a `u16` cannot hold.
fn pulse_width(name: &str, value: f64) -> napi::Result<u16> {
    if value.fract() != 0.0 || !(0.0..=f64::from(u16::MAX)).contains(&value) {
        return Err(napi::Error::from_reason(format!(
            "{name} must be a whole number of microseconds from 0 to 65535, not {value}"
        )));
    }
    Ok(value as u16)
}

/// A hobby servo's pulse widths across its travel.
#[napi]
pub struct ServoMap {
    inner: CoreServo,
}

#[napi]
impl ServoMap {
    /// Creates a map from the pulse at zero degrees, the pulse at full travel, and the
    /// travel in degrees, whose magnitude is used.
    #[napi(constructor)]
    pub fn new(min_us: f64, max_us: f64, range_deg: f64) -> napi::Result<Self> {
        Ok(Self {
            inner: CoreServo::new(
                pulse_width("minUs", min_us)?,
                pulse_width("maxUs", max_us)?,
                range_deg as f32,
            ),
        })
    }

    /// The standard hobby servo: 1000 to 2000 microseconds over 180 degrees.
    #[napi(factory)]
    pub fn standard() -> Self {
        Self {
            inner: CoreServo::standard(),
        }
    }

    /// The pulse width at zero degrees, in microseconds.
    #[napi(getter)]
    pub fn min_us(&self) -> u32 {
        u32::from(self.inner.min_us())
    }

    /// The pulse width at full travel, in microseconds.
    #[napi(getter)]
    pub fn max_us(&self) -> u32 {
        u32::from(self.inner.max_us())
    }

    /// The full travel, in degrees.
    #[napi(getter)]
    pub fn range_deg(&self) -> f64 {
        f64::from(self.inner.range_deg())
    }

    /// Returns the pulse width for an angle, held to the travel, or 0, no pulse, for an
    /// angle that is not a number.
    #[napi]
    pub fn pulse(&self, angle_deg: f64) -> u32 {
        u32::from(self.inner.pulse(angle_deg as f32))
    }

    /// Returns the angle a pulse width sets, held to the pulse range.
    #[napi]
    pub fn angle(&self, pulse_us: f64) -> napi::Result<f64> {
        Ok(f64::from(
            self.inner.angle(pulse_width("pulseUs", pulse_us)?),
        ))
    }
}

/// An electronic speed controller's pulse widths from full reverse to full forward.
#[napi]
pub struct Esc {
    inner: CoreEsc,
}

#[napi]
impl Esc {
    /// Creates a map from the full-reverse, neutral, and full-forward pulse widths.
    #[napi(constructor)]
    pub fn new(min_us: f64, neutral_us: f64, max_us: f64) -> napi::Result<Self> {
        Ok(Self {
            inner: CoreEsc::new(
                pulse_width("minUs", min_us)?,
                pulse_width("neutralUs", neutral_us)?,
                pulse_width("maxUs", max_us)?,
            ),
        })
    }

    /// The common reversible controller: 1000, 1500, and 2000 microseconds.
    #[napi(factory)]
    pub fn bidirectional() -> Self {
        Self {
            inner: CoreEsc::bidirectional(),
        }
    }

    /// The pulse width at full reverse, in microseconds.
    #[napi(getter)]
    pub fn min_us(&self) -> u32 {
        u32::from(self.inner.min_us())
    }

    /// The pulse width at rest, in microseconds.
    #[napi(getter)]
    pub fn neutral_us(&self) -> u32 {
        u32::from(self.inner.neutral_us())
    }

    /// The pulse width at full forward, in microseconds.
    #[napi(getter)]
    pub fn max_us(&self) -> u32 {
        u32::from(self.inner.max_us())
    }

    /// Returns the pulse width for a throttle from -1 to 1, held to that range, or the
    /// neutral one for a throttle that is not a number.
    #[napi]
    pub fn pulse(&self, throttle: f64) -> u32 {
        u32::from(self.inner.pulse(throttle as f32))
    }
}

/// Reads a step count a caller passed, refusing one a JavaScript number does not hold as a
/// whole number exactly.
fn step_count(name: &str, value: f64) -> napi::Result<i64> {
    const SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    if value.fract() != 0.0 || !(-SAFE_INTEGER..=SAFE_INTEGER).contains(&value) {
        return Err(napi::Error::from_reason(format!(
            "{name} must be a whole number of steps, not {value}"
        )));
    }
    Ok(value as i64)
}

/// Counts a quadrature encoder's steps from its A and B channels.
#[napi]
pub struct Quadrature {
    inner: CoreQuadrature,
}

#[napi]
impl Quadrature {
    /// Creates a decoder seeded with the channel levels it reads now, both low unless
    /// given, so the first reading does not count a step that did not happen.
    #[napi(constructor)]
    pub fn new(a: Option<bool>, b: Option<bool>) -> Self {
        Self {
            inner: CoreQuadrature::starting(a.unwrap_or(false), b.unwrap_or(false)),
        }
    }

    /// Feeds the channel levels read now, and returns 1 or -1 for a step in either
    /// direction, or 0 for no change or a jump past a step.
    #[napi]
    pub fn update(&mut self, a: bool, b: bool) -> i32 {
        i32::from(self.inner.update(a, b))
    }

    /// The signed count of steps so far.
    #[napi(getter)]
    pub fn count(&self) -> f64 {
        self.inner.count() as f64
    }

    /// Sets the count back to 0, keeping the channel state last read.
    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Turns encoder steps into the distance and speed of the wheel they turn with.
#[napi]
pub struct QuadratureScale {
    inner: CoreScale,
}

#[napi]
impl QuadratureScale {
    /// Creates a scale from the steps per wheel revolution and the wheel radius in meters;
    /// each magnitude is used.
    #[napi(constructor)]
    pub fn new(counts_per_rev: f64, wheel_radius: f64) -> Self {
        Self {
            inner: CoreScale::new(counts_per_rev as f32, wheel_radius as f32),
        }
    }

    /// Returns the distance, in meters, a wheel rolled for a step count.
    #[napi]
    pub fn distance(&self, count: f64) -> napi::Result<f64> {
        Ok(f64::from(self.inner.distance(step_count("count", count)?)))
    }

    /// Returns the speed, in meters per second, from the steps counted over `dt`, or 0 when
    /// `dt` is 0.
    #[napi]
    pub fn velocity(&self, delta_count: f64, dt: f64) -> napi::Result<f64> {
        Ok(f64::from(self.inner.velocity(
            step_count("deltaCount", delta_count)?,
            dt as f32,
        )))
    }
}
