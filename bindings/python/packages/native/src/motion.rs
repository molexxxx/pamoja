//! Generated Python bindings for the motion helpers: chassis kinematics, an arm, odometry,
//! waypoint guidance, the safety gate, and the conversions to servo, ESC, and encoder units.
//!
//! These bind the robotics half of `pamoja-kit`. A pose, a twist, and the other plain values
//! are small frozen classes; a pair, such as a drive's left and right speeds, is a tuple, as
//! it is in Rust. A coordinate is a `(latitude, longitude)` pair, so the kit facade's
//! `Coordinate` passes straight through. A reading that is not a finite number never moves a
//! robot: the helpers stop, hold, or ignore it, as the Rust crate documents.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_kit::{
    forward_kinematics as core_forward_kinematics, obstacle_stop as core_obstacle_stop,
    Ackermann as CoreAckermann, Coordinate, DhParameters as CoreDh, DiffDrive as CoreDiffDrive,
    EStop as CoreEStop, Elbow, Esc as CoreEsc, Limits as CoreLimits, Mecanum as CoreMecanum,
    Odometry as CoreOdometry, Pose as CorePose, Quadrature as CoreQuadrature,
    QuadratureScale as CoreScale, SafetyGate as CoreGate, ServoMap as CoreServo,
    SkidSteer as CoreSkidSteer, Transform as CoreTransform, Twist as CoreTwist,
    TwoLinkArm as CoreArm, Watchdog as CoreWatchdog, WaypointFollower as CoreFollower,
    WheelSpeeds as CoreWheels,
};

/// Where a robot is and which way it faces.
#[gen_stub_pyclass]
#[pyclass(frozen, eq, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq)]
pub struct Pose {
    /// Position along the world x axis, in meters.
    #[pyo3(get)]
    x: f32,
    /// Position along the world y axis, in meters.
    #[pyo3(get)]
    y: f32,
    /// Heading from the world x axis, in radians, in `(-pi, pi]`, positive
    /// counter-clockwise.
    #[pyo3(get)]
    theta: f32,
}

#[gen_stub_pymethods]
#[pymethods]
impl Pose {
    /// Creates a pose; the heading is wrapped into `(-pi, pi]`.
    #[new]
    #[pyo3(signature = (x=0.0, y=0.0, theta=0.0))]
    fn new(x: f32, y: f32, theta: f32) -> Self {
        CorePose::new(x, y, theta).into()
    }

    fn __repr__(&self) -> String {
        format!(
            "Pose(x={:?}, y={:?}, theta={:?})",
            self.x, self.y, self.theta
        )
    }
}

impl From<CorePose> for Pose {
    fn from(value: CorePose) -> Self {
        Self {
            x: value.x,
            y: value.y,
            theta: value.theta,
        }
    }
}

impl From<&Pose> for CorePose {
    fn from(value: &Pose) -> Self {
        CorePose::new(value.x, value.y, value.theta)
    }
}

/// How fast a robot is asked to move.
#[gen_stub_pyclass]
#[pyclass(frozen, eq, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq)]
pub struct Twist {
    /// Forward speed along the x axis.
    #[pyo3(get)]
    vx: f32,
    /// Leftward speed along the y axis; zero for drives that cannot strafe.
    #[pyo3(get)]
    vy: f32,
    /// Yaw rate about the z axis, positive counter-clockwise.
    #[pyo3(get)]
    omega: f32,
}

#[gen_stub_pymethods]
#[pymethods]
impl Twist {
    /// Creates a twist; each part is 0 unless given.
    #[new]
    #[pyo3(signature = (vx=0.0, vy=0.0, omega=0.0))]
    fn new(vx: f32, vy: f32, omega: f32) -> Self {
        Self { vx, vy, omega }
    }

    fn __repr__(&self) -> String {
        format!(
            "Twist(vx={:?}, vy={:?}, omega={:?})",
            self.vx, self.vy, self.omega
        )
    }
}

impl From<CoreTwist> for Twist {
    fn from(value: CoreTwist) -> Self {
        Self {
            vx: value.vx,
            vy: value.vy,
            omega: value.omega,
        }
    }
}

impl From<&Twist> for CoreTwist {
    fn from(value: &Twist) -> Self {
        CoreTwist::new(value.vx, value.vy, value.omega)
    }
}

/// Wheel speeds for a desired body motion, and the body motion measured wheel speeds make,
/// for a robot that steers by spinning two wheels at different speeds.
#[gen_stub_pyclass]
#[pyclass]
pub struct DiffDrive {
    inner: CoreDiffDrive,
}

#[gen_stub_pymethods]
#[pymethods]
impl DiffDrive {
    /// Creates a model for wheels `track` apart; its magnitude is used.
    #[new]
    fn new(track: f32) -> Self {
        Self {
            inner: CoreDiffDrive::new(track),
        }
    }

    /// Returns the `(left, right)` wheel speeds for a forward speed and a turn rate.
    fn wheel_speeds(&self, linear: f32, angular: f32) -> (f32, f32) {
        self.inner.wheel_speeds(linear, angular)
    }

    /// Returns the `(linear, angular)` body motion measured wheel speeds make.
    fn body_motion(&self, left: f32, right: f32) -> (f32, f32) {
        self.inner.body_motion(left, right)
    }
}

/// Car-like steering: one steered axle and a driven axle a wheelbase apart.
#[gen_stub_pyclass]
#[pyclass]
pub struct Ackermann {
    inner: CoreAckermann,
}

#[gen_stub_pymethods]
#[pymethods]
impl Ackermann {
    /// Creates a model for axles `wheelbase` apart; its magnitude is used.
    #[new]
    fn new(wheelbase: f32) -> Self {
        Self {
            inner: CoreAckermann::new(wheelbase),
        }
    }

    /// Returns the steering angle, in radians, for a forward speed and a yaw rate, or 0
    /// while stopped.
    fn steering_angle(&self, linear: f32, angular: f32) -> f32 {
        self.inner.steering_angle(linear, angular)
    }

    /// Returns the yaw rate a forward speed and a steering angle, in radians, produce.
    fn yaw_rate(&self, linear: f32, steering: f32) -> f32 {
        self.inner.yaw_rate(linear, steering)
    }

    /// Returns the turn radius of a steering angle, in meters, or `inf` with the wheels
    /// straight.
    fn turn_radius(&self, steering: f32) -> f32 {
        self.inner.turn_radius(steering)
    }

    /// Returns the path curvature of a steering angle, the reciprocal of the turn radius.
    fn curvature(&self, steering: f32) -> f32 {
        self.inner.curvature(steering)
    }
}

/// A tracked or four-wheel drive that turns by skidding, with its track widened by a slip
/// factor.
#[gen_stub_pyclass]
#[pyclass]
pub struct SkidSteer {
    inner: CoreSkidSteer,
}

#[gen_stub_pymethods]
#[pymethods]
impl SkidSteer {
    /// Creates a model for sides `track` apart with a `slip` factor; each magnitude is
    /// used, and a slip of 0 is taken as 1.
    #[new]
    #[pyo3(signature = (track, slip=1.0))]
    fn new(track: f32, slip: f32) -> Self {
        Self {
            inner: CoreSkidSteer::new(track, slip),
        }
    }

    /// Returns the `(left, right)` speeds for a forward speed and a yaw rate.
    fn wheel_speeds(&self, linear: f32, angular: f32) -> (f32, f32) {
        self.inner.wheel_speeds(linear, angular)
    }

    /// Returns the `(linear, angular)` body motion measured side speeds make.
    fn body_motion(&self, left: f32, right: f32) -> (f32, f32) {
        self.inner.body_motion(left, right)
    }
}

/// The four wheel speeds of a mecanum base.
#[gen_stub_pyclass]
#[pyclass(frozen, eq, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq)]
pub struct WheelSpeeds {
    /// The front-left wheel's speed.
    #[pyo3(get)]
    front_left: f32,
    /// The front-right wheel's speed.
    #[pyo3(get)]
    front_right: f32,
    /// The rear-left wheel's speed.
    #[pyo3(get)]
    rear_left: f32,
    /// The rear-right wheel's speed.
    #[pyo3(get)]
    rear_right: f32,
}

#[gen_stub_pymethods]
#[pymethods]
impl WheelSpeeds {
    /// Creates the four speeds.
    #[new]
    fn new(front_left: f32, front_right: f32, rear_left: f32, rear_right: f32) -> Self {
        Self {
            front_left,
            front_right,
            rear_left,
            rear_right,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "WheelSpeeds(front_left={:?}, front_right={:?}, rear_left={:?}, rear_right={:?})",
            self.front_left, self.front_right, self.rear_left, self.rear_right
        )
    }
}

/// A four-wheel mecanum base, which drives, strafes, and turns at once.
#[gen_stub_pyclass]
#[pyclass]
pub struct Mecanum {
    inner: CoreMecanum,
}

#[gen_stub_pymethods]
#[pymethods]
impl Mecanum {
    /// Creates a model from the front-to-rear `wheelbase` and side-to-side `track`; each
    /// magnitude is used.
    #[new]
    fn new(wheelbase: f32, track: f32) -> Self {
        Self {
            inner: CoreMecanum::new(wheelbase, track),
        }
    }

    /// Returns the four wheel speeds for a body twist.
    fn wheel_speeds(&self, twist: PyRef<'_, Twist>) -> WheelSpeeds {
        let wheels = self.inner.wheel_speeds(CoreTwist::from(&*twist));
        WheelSpeeds {
            front_left: wheels.front_left,
            front_right: wheels.front_right,
            rear_left: wheels.rear_left,
            rear_right: wheels.rear_right,
        }
    }

    /// Returns the body twist measured wheel speeds make.
    fn body_motion(&self, wheels: PyRef<'_, WheelSpeeds>) -> Twist {
        self.inner
            .body_motion(CoreWheels {
                front_left: wheels.front_left,
                front_right: wheels.front_right,
                rear_left: wheels.rear_left,
                rear_right: wheels.rear_right,
            })
            .into()
    }
}

/// A planar arm of two links, with a closed-form inverse.
#[gen_stub_pyclass]
#[pyclass]
pub struct TwoLinkArm {
    inner: CoreArm,
}

#[gen_stub_pymethods]
#[pymethods]
impl TwoLinkArm {
    /// Creates an arm from its shoulder and elbow link lengths; each magnitude is used.
    #[new]
    fn new(l1: f32, l2: f32) -> Self {
        Self {
            inner: CoreArm::new(l1, l2),
        }
    }

    /// The `(min, max)` the hand reaches from the shoulder.
    #[getter]
    fn reach(&self) -> (f32, f32) {
        self.inner.reach()
    }

    /// Returns the hand's `(x, y)` for a shoulder and an elbow angle, in radians.
    fn tip(&self, shoulder: f32, elbow: f32) -> (f32, f32) {
        self.inner.tip(shoulder, elbow)
    }

    /// Returns the `(shoulder, elbow)` angles that put the hand at a point, or `None` for a
    /// point out of reach, an arm with a link of no length, or a coordinate that is not a
    /// finite number. `elbow` is `"up"` or `"down"`.
    #[pyo3(signature = (x, y, elbow="up"))]
    fn joints_for(&self, x: f32, y: f32, elbow: &str) -> PyResult<Option<(f32, f32)>> {
        let branch = match elbow {
            "up" => Elbow::Up,
            "down" => Elbow::Down,
            other => {
                return Err(PyValueError::new_err(format!(
                    "elbow must be \"up\" or \"down\", not {other:?}"
                )))
            }
        };
        Ok(self.inner.joints_for(x, y, branch))
    }
}

/// One joint of a serial arm in the Denavit-Hartenberg convention.
#[gen_stub_pyclass]
#[pyclass(frozen, eq, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq)]
pub struct DhParameters {
    /// The link length along the common normal, in meters.
    #[pyo3(get)]
    a: f32,
    /// The link twist about the common normal, in radians.
    #[pyo3(get)]
    alpha: f32,
    /// The link offset along the previous z axis, in meters.
    #[pyo3(get)]
    d: f32,
    /// The joint angle about the previous z axis, in radians.
    #[pyo3(get)]
    theta: f32,
}

#[gen_stub_pymethods]
#[pymethods]
impl DhParameters {
    /// Creates a joint from its four parameters; each is 0 unless given.
    #[new]
    #[pyo3(signature = (a=0.0, alpha=0.0, d=0.0, theta=0.0))]
    fn new(a: f32, alpha: f32, d: f32, theta: f32) -> Self {
        Self { a, alpha, d, theta }
    }

    /// Returns the homogeneous transform this joint makes.
    fn transform(&self) -> Transform {
        Transform {
            inner: CoreDh::from(self).transform(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "DhParameters(a={:?}, alpha={:?}, d={:?}, theta={:?})",
            self.a, self.alpha, self.d, self.theta
        )
    }
}

impl From<&DhParameters> for CoreDh {
    fn from(value: &DhParameters) -> Self {
        CoreDh {
            a: value.a,
            alpha: value.alpha,
            d: value.d,
            theta: value.theta,
        }
    }
}

/// A 4x4 homogeneous transform: a rotation and a translation.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Transform {
    inner: CoreTransform,
}

#[gen_stub_pymethods]
#[pymethods]
impl Transform {
    /// The identity: no rotation and no translation.
    #[staticmethod]
    fn identity() -> Self {
        Self {
            inner: CoreTransform::identity(),
        }
    }

    /// Returns `self * other`, the transform that applies `other` and then this one.
    fn multiply(&self, other: PyRef<'_, Transform>) -> Transform {
        Transform {
            inner: self.inner.multiply(&other.inner),
        }
    }

    /// Where this transform places the origin, its translation, as `(x, y, z)`.
    #[getter]
    fn position(&self) -> (f32, f32, f32) {
        self.inner.position()
    }

    /// The sixteen elements, row-major.
    #[getter]
    fn elements(&self) -> Vec<f32> {
        self.inner.m.to_vec()
    }

    fn __eq__(&self, other: PyRef<'_, Transform>) -> bool {
        self.inner == other.inner
    }
}

/// Returns the transform from a serial arm's base to its tool, or the identity for an arm
/// with no joints.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn forward_kinematics(joints: Vec<PyRef<'_, DhParameters>>) -> Transform {
    let chain: Vec<CoreDh> = joints.iter().map(|joint| CoreDh::from(&**joint)).collect();
    Transform {
        inner: core_forward_kinematics(&chain),
    }
}

/// Tracks a robot's pose by adding up its motion.
#[gen_stub_pyclass]
#[pyclass]
pub struct Odometry {
    inner: CoreOdometry,
}

#[gen_stub_pymethods]
#[pymethods]
impl Odometry {
    /// Creates an estimate starting at `start`, or at the origin facing along x.
    #[new]
    #[pyo3(signature = (start=None))]
    fn new(start: Option<PyRef<'_, Pose>>) -> Self {
        let start = start
            .map(|pose| CorePose::from(&*pose))
            .unwrap_or_else(CorePose::origin);
        Self {
            inner: CoreOdometry::new(start),
        }
    }

    /// The pose so far.
    #[getter]
    fn pose(&self) -> Pose {
        self.inner.pose().into()
    }

    /// Sets the estimate to a known pose.
    fn reset(&mut self, pose: PyRef<'_, Pose>) {
        self.inner.reset(CorePose::from(&*pose));
    }

    /// Adds a forward speed and yaw rate held for `dt`, and returns the new pose.
    fn integrate(&mut self, linear: f32, angular: f32, dt: f32) -> Pose {
        self.inner.integrate(linear, angular, dt).into()
    }

    /// Adds the distances two wheels rolled, through a differential drive, and returns the
    /// new pose.
    fn integrate_wheels(&mut self, left: f32, right: f32, drive: PyRef<'_, DiffDrive>) -> Pose {
        self.inner
            .integrate_wheels(left, right, &drive.inner)
            .into()
    }

    /// Nudges the heading toward an absolute measurement by `weight`, from 0 to 1.
    fn fuse_heading(&mut self, measured: f32, weight: f32) {
        self.inner.fuse_heading(measured, weight);
    }
}

/// The command toward a waypoint, with the geometry behind it.
#[gen_stub_pyclass]
#[pyclass(frozen, eq)]
#[derive(PartialEq)]
pub struct Guidance {
    /// The body motion to drive.
    #[pyo3(get)]
    twist: Twist,
    /// The distance left to the target, in meters.
    #[pyo3(get)]
    distance_m: f64,
    /// The heading error to the target, in degrees, in `(-180, 180]`.
    #[pyo3(get)]
    heading_error_deg: f32,
    /// Whether the target is within the arrival radius.
    #[pyo3(get)]
    arrived: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl Guidance {
    fn __repr__(&self) -> String {
        format!(
            "Guidance(twist={}, distance_m={:?}, heading_error_deg={:?}, arrived={})",
            self.twist.__repr__(),
            self.distance_m,
            self.heading_error_deg,
            if self.arrived { "True" } else { "False" }
        )
    }
}

/// Steers toward a waypoint: pivot toward it, then drive, slowing as the heading error
/// grows.
#[gen_stub_pyclass]
#[pyclass]
pub struct WaypointFollower {
    inner: CoreFollower,
}

#[gen_stub_pymethods]
#[pymethods]
impl WaypointFollower {
    /// Creates a follower. `cruise` is the forward speed when pointed at the target,
    /// `arrival_m` how close counts as arrived, `heading_gain` the yaw rate per radian of
    /// heading error, and `max_angular` the largest yaw rate; each magnitude is used.
    #[new]
    fn new(cruise: f32, arrival_m: f64, heading_gain: f32, max_angular: f32) -> Self {
        Self {
            inner: CoreFollower::new(cruise, arrival_m, heading_gain, max_angular),
        }
    }

    /// Returns the command from a `(latitude, longitude)` position and a compass heading,
    /// in degrees clockwise from north, toward a target.
    fn guide(&self, here: (f64, f64), heading_deg: f32, target: (f64, f64)) -> Guidance {
        let guidance = self.inner.guide(
            Coordinate::new(here.0, here.1),
            heading_deg,
            Coordinate::new(target.0, target.1),
        );
        Guidance {
            twist: guidance.twist.into(),
            distance_m: guidance.distance_m,
            heading_error_deg: guidance.heading_error_deg,
            arrived: guidance.arrived,
        }
    }
}

/// Cuts forward and sideways motion when an obstacle is within `stop_distance_m`, keeping
/// the turn. A range that is not a number counts as an obstacle.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn obstacle_stop(twist: PyRef<'_, Twist>, range_m: f32, stop_distance_m: f32) -> Twist {
    core_obstacle_stop(CoreTwist::from(&*twist), range_m, stop_distance_m).into()
}

/// An emergency stop that latches until a person resets it.
#[gen_stub_pyclass]
#[pyclass]
pub struct EStop {
    inner: CoreEStop,
}

#[gen_stub_pymethods]
#[pymethods]
impl EStop {
    /// Creates an e-stop that is not engaged.
    #[new]
    fn new() -> Self {
        Self {
            inner: CoreEStop::new(),
        }
    }

    /// Engages the stop; it holds until reset.
    fn engage(&mut self) {
        self.inner.engage();
    }

    /// Clears the stop.
    fn reset(&mut self) {
        self.inner.reset();
    }

    /// Whether the stop is engaged.
    #[getter]
    fn is_engaged(&self) -> bool {
        self.inner.is_engaged()
    }

    /// Returns `desired` while clear, or a zero twist while engaged.
    fn gate(&self, desired: PyRef<'_, Twist>) -> Twist {
        self.inner.gate(CoreTwist::from(&*desired)).into()
    }
}

/// A deadman timer that expires unless fed often enough.
#[gen_stub_pyclass]
#[pyclass]
pub struct Watchdog {
    inner: CoreWatchdog,
}

#[gen_stub_pymethods]
#[pymethods]
impl Watchdog {
    /// Creates a watchdog that expires after `timeout` without being fed; a timeout that is
    /// not a number is taken as 0.
    #[new]
    fn new(timeout: f32) -> Self {
        Self {
            inner: CoreWatchdog::new(timeout),
        }
    }

    /// Feeds the watchdog, restarting its silence timer.
    fn feed(&mut self) {
        self.inner.feed();
    }

    /// Advances the timer by `dt` and returns whether it has expired; a `dt` that is not a
    /// finite number expires it until it is fed.
    fn update(&mut self, dt: f32) -> bool {
        self.inner.update(dt)
    }

    /// Whether the watchdog has expired.
    #[getter]
    fn is_expired(&self) -> bool {
        self.inner.is_expired()
    }
}

/// Speed and acceleration limits, with the motion they last allowed.
#[gen_stub_pyclass]
#[pyclass]
pub struct Limits {
    inner: CoreLimits,
}

#[gen_stub_pymethods]
#[pymethods]
impl Limits {
    /// Creates limits starting from rest: the largest planar speed and yaw rate, and the
    /// largest change in each per second. Each magnitude is used, and one that is not a
    /// number is taken as 0, which holds the robot still.
    #[new]
    fn new(
        max_linear: f32,
        max_angular: f32,
        max_linear_accel: f32,
        max_angular_accel: f32,
    ) -> Self {
        Self {
            inner: CoreLimits::new(max_linear, max_angular, max_linear_accel, max_angular_accel),
        }
    }

    /// Returns `desired` held to the speed limits and eased toward within the acceleration
    /// limits over `dt`.
    fn apply(&mut self, desired: PyRef<'_, Twist>, dt: f32) -> Twist {
        self.inner.apply(CoreTwist::from(&*desired), dt).into()
    }

    /// Forgets the motion last allowed, so the next command eases up from rest.
    fn reset(&mut self) {
        self.inner.reset();
    }
}

/// The gate every motion command passes through: an e-stop, a watchdog, and limits.
#[gen_stub_pyclass]
#[pyclass]
pub struct SafetyGate {
    inner: CoreGate,
}

#[gen_stub_pymethods]
#[pymethods]
impl SafetyGate {
    /// Creates a gate from limits, copied as they stand, and a watchdog timeout.
    #[new]
    fn new(limits: PyRef<'_, Limits>, watchdog_timeout: f32) -> Self {
        Self {
            inner: CoreGate::new(limits.inner, watchdog_timeout),
        }
    }

    /// Feeds the watchdog; call it whenever a fresh command arrives.
    fn feed(&mut self) {
        self.inner.feed();
    }

    /// Engages the latching e-stop.
    fn engage_estop(&mut self) {
        self.inner.engage_estop();
    }

    /// Clears the e-stop.
    fn reset_estop(&mut self) {
        self.inner.reset_estop();
    }

    /// Whether the gate is forcing a stop: its e-stop is engaged or its watchdog expired.
    #[getter]
    fn is_stopped(&self) -> bool {
        self.inner.is_stopped()
    }

    /// Returns the command that is safe to drive: a zero twist while stopped, otherwise
    /// `desired` bounded by the limits.
    fn command(&mut self, desired: PyRef<'_, Twist>, dt: f32) -> Twist {
        self.inner.command(CoreTwist::from(&*desired), dt).into()
    }
}

/// Reads a pulse width a caller passed, refusing one a `u16` cannot hold.
fn pulse_width(name: &str, value: i64) -> PyResult<u16> {
    u16::try_from(value).map_err(|_| {
        PyValueError::new_err(format!(
            "{name} must be a whole number of microseconds from 0 to 65535, not {value}"
        ))
    })
}

/// A hobby servo's pulse widths across its travel.
#[gen_stub_pyclass]
#[pyclass]
pub struct ServoMap {
    inner: CoreServo,
}

#[gen_stub_pymethods]
#[pymethods]
impl ServoMap {
    /// Creates a map from the pulse at zero degrees, the pulse at full travel, and the
    /// travel in degrees, whose magnitude is used.
    #[new]
    fn new(min_us: i64, max_us: i64, range_deg: f32) -> PyResult<Self> {
        Ok(Self {
            inner: CoreServo::new(
                pulse_width("min_us", min_us)?,
                pulse_width("max_us", max_us)?,
                range_deg,
            ),
        })
    }

    /// The standard hobby servo: 1000 to 2000 microseconds over 180 degrees.
    #[staticmethod]
    fn standard() -> Self {
        Self {
            inner: CoreServo::standard(),
        }
    }

    /// The pulse width at zero degrees, in microseconds.
    #[getter]
    fn min_us(&self) -> u16 {
        self.inner.min_us()
    }

    /// The pulse width at full travel, in microseconds.
    #[getter]
    fn max_us(&self) -> u16 {
        self.inner.max_us()
    }

    /// The full travel, in degrees.
    #[getter]
    fn range_deg(&self) -> f32 {
        self.inner.range_deg()
    }

    /// Returns the pulse width for an angle, held to the travel, or 0, no pulse, for an
    /// angle that is not a number.
    fn pulse(&self, angle_deg: f32) -> u16 {
        self.inner.pulse(angle_deg)
    }

    /// Returns the angle a pulse width sets, held to the pulse range.
    fn angle(&self, pulse_us: i64) -> PyResult<f32> {
        Ok(self.inner.angle(pulse_width("pulse_us", pulse_us)?))
    }
}

/// An electronic speed controller's pulse widths from full reverse to full forward.
#[gen_stub_pyclass]
#[pyclass]
pub struct Esc {
    inner: CoreEsc,
}

#[gen_stub_pymethods]
#[pymethods]
impl Esc {
    /// Creates a map from the full-reverse, neutral, and full-forward pulse widths.
    #[new]
    fn new(min_us: i64, neutral_us: i64, max_us: i64) -> PyResult<Self> {
        Ok(Self {
            inner: CoreEsc::new(
                pulse_width("min_us", min_us)?,
                pulse_width("neutral_us", neutral_us)?,
                pulse_width("max_us", max_us)?,
            ),
        })
    }

    /// The common reversible controller: 1000, 1500, and 2000 microseconds.
    #[staticmethod]
    fn bidirectional() -> Self {
        Self {
            inner: CoreEsc::bidirectional(),
        }
    }

    /// The pulse width at full reverse, in microseconds.
    #[getter]
    fn min_us(&self) -> u16 {
        self.inner.min_us()
    }

    /// The pulse width at rest, in microseconds.
    #[getter]
    fn neutral_us(&self) -> u16 {
        self.inner.neutral_us()
    }

    /// The pulse width at full forward, in microseconds.
    #[getter]
    fn max_us(&self) -> u16 {
        self.inner.max_us()
    }

    /// Returns the pulse width for a throttle from -1 to 1, held to that range, or the
    /// neutral one for a throttle that is not a number.
    fn pulse(&self, throttle: f32) -> u16 {
        self.inner.pulse(throttle)
    }
}

/// Counts a quadrature encoder's steps from its A and B channels.
#[gen_stub_pyclass]
#[pyclass]
pub struct Quadrature {
    inner: CoreQuadrature,
}

#[gen_stub_pymethods]
#[pymethods]
impl Quadrature {
    /// Creates a decoder seeded with the channel levels it reads now, both low unless
    /// given, so the first reading does not count a step that did not happen.
    #[new]
    #[pyo3(signature = (a=false, b=false))]
    fn new(a: bool, b: bool) -> Self {
        Self {
            inner: CoreQuadrature::starting(a, b),
        }
    }

    /// Feeds the channel levels read now, and returns 1 or -1 for a step in either
    /// direction, or 0 for no change or a jump past a step.
    fn update(&mut self, a: bool, b: bool) -> i8 {
        self.inner.update(a, b)
    }

    /// The signed count of steps so far.
    #[getter]
    fn count(&self) -> i64 {
        self.inner.count()
    }

    /// Sets the count back to 0, keeping the channel state last read.
    fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Turns encoder steps into the distance and speed of the wheel they turn with.
#[gen_stub_pyclass]
#[pyclass]
pub struct QuadratureScale {
    inner: CoreScale,
}

#[gen_stub_pymethods]
#[pymethods]
impl QuadratureScale {
    /// Creates a scale from the steps per wheel revolution and the wheel radius in meters;
    /// each magnitude is used.
    #[new]
    fn new(counts_per_rev: f32, wheel_radius: f32) -> Self {
        Self {
            inner: CoreScale::new(counts_per_rev, wheel_radius),
        }
    }

    /// Returns the distance, in meters, a wheel rolled for a step count.
    fn distance(&self, count: i64) -> f32 {
        self.inner.distance(count)
    }

    /// Returns the speed, in meters per second, from the steps counted over `dt`, or 0 when
    /// `dt` is 0.
    fn velocity(&self, delta_count: i64, dt: f32) -> f32 {
        self.inner.velocity(delta_count, dt)
    }
}
