//! The C ABI for the motion helpers: chassis kinematics, an arm, odometry, waypoint
//! guidance, the safety gate, and the conversions to servo, ESC, and encoder units.
//!
//! These wrap the robotics half of [`pamoja_kit`]. The kinematic models, the arm, the
//! waypoint follower, and the servo, ESC, and encoder scales hold only their parameters and
//! never change, so each crosses by value as a small struct the caller fills in and passes
//! to every call; the helper applies the same fallbacks the Rust constructor does, such as
//! taking a length's magnitude. The helpers that remember something between calls, the
//! odometry, the e-stop, the watchdog, the limits, the safety gate, and the quadrature
//! decoder, are opaque handles, each with its own `_free`. A constructor never returns
//! null, and each method documents what it returns for a null handle.

use std::ptr;

use pamoja_kit::{
    forward_kinematics, obstacle_stop, Ackermann, Coordinate, DhParameters, DiffDrive, EStop,
    Elbow, Esc, Limits, Mecanum, Odometry, Pose, Quadrature, QuadratureScale, SafetyGate, ServoMap,
    SkidSteer, Transform, Twist, TwoLinkArm, Watchdog, WaypointFollower, WheelSpeeds,
};

use crate::kit::PamojaCoordinate;

/// Where a robot is and which way it faces.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaPose {
    /// Position along the world x axis, in meters.
    pub x: f32,
    /// Position along the world y axis, in meters.
    pub y: f32,
    /// Heading from the world x axis, in radians, positive counter-clockwise.
    pub theta: f32,
}

impl From<PamojaPose> for Pose {
    fn from(value: PamojaPose) -> Self {
        Pose::new(value.x, value.y, value.theta)
    }
}

impl From<Pose> for PamojaPose {
    fn from(value: Pose) -> Self {
        Self {
            x: value.x,
            y: value.y,
            theta: value.theta,
        }
    }
}

/// How fast a robot is asked to move.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaTwist {
    /// Forward speed along the x axis.
    pub vx: f32,
    /// Leftward speed along the y axis; zero for drives that cannot strafe.
    pub vy: f32,
    /// Yaw rate about the z axis, positive counter-clockwise.
    pub omega: f32,
}

impl From<PamojaTwist> for Twist {
    fn from(value: PamojaTwist) -> Self {
        Twist::new(value.vx, value.vy, value.omega)
    }
}

impl From<Twist> for PamojaTwist {
    fn from(value: Twist) -> Self {
        Self {
            vx: value.vx,
            vy: value.vy,
            omega: value.omega,
        }
    }
}

/// The speeds of a two-sided drive's left and right wheels or tracks.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaSideSpeeds {
    /// The left side's speed.
    pub left: f32,
    /// The right side's speed.
    pub right: f32,
}

/// A planar body motion: a forward speed and a yaw rate.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaBodyMotion {
    /// The forward speed.
    pub linear: f32,
    /// The yaw rate, positive turning left.
    pub angular: f32,
}

/// A differential drive: two wheels `track` apart.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaDiffDrive {
    /// The distance between the left and right wheels; its magnitude is used.
    pub track: f32,
}

/// Returns the wheel speeds a differential drive needs for a body motion.
///
/// # Arguments
///
/// * `drive` - the drive.
/// * `linear` - the forward speed.
/// * `angular` - the turn rate, positive turning left.
///
/// # Returns
///
/// The left and right wheel speeds.
#[no_mangle]
pub extern "C" fn pamoja_diff_drive_wheel_speeds(
    drive: PamojaDiffDrive,
    linear: f32,
    angular: f32,
) -> PamojaSideSpeeds {
    let (left, right) = DiffDrive::new(drive.track).wheel_speeds(linear, angular);
    PamojaSideSpeeds { left, right }
}

/// Returns the body motion a differential drive makes from measured wheel speeds.
///
/// # Arguments
///
/// * `drive` - the drive.
/// * `left` - the left wheel speed.
/// * `right` - the right wheel speed.
///
/// # Returns
///
/// The forward speed and turn rate; the turn rate is 0 for a drive with no track.
#[no_mangle]
pub extern "C" fn pamoja_diff_drive_body_motion(
    drive: PamojaDiffDrive,
    left: f32,
    right: f32,
) -> PamojaBodyMotion {
    let (linear, angular) = DiffDrive::new(drive.track).body_motion(left, right);
    PamojaBodyMotion { linear, angular }
}

/// Car-like steering: one steered axle and a driven axle `wheelbase` apart.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaAckermann {
    /// The distance from the steered axle to the driven axle; its magnitude is used.
    pub wheelbase: f32,
}

/// Returns the steering angle for a forward speed and a yaw rate.
///
/// # Arguments
///
/// * `car` - the vehicle.
/// * `linear` - the forward speed.
/// * `angular` - the desired yaw rate, positive turning left.
///
/// # Returns
///
/// The steering angle in radians, or 0 while the vehicle is stopped.
#[no_mangle]
pub extern "C" fn pamoja_ackermann_steering_angle(
    car: PamojaAckermann,
    linear: f32,
    angular: f32,
) -> f32 {
    Ackermann::new(car.wheelbase).steering_angle(linear, angular)
}

/// Returns the yaw rate a forward speed and a steering angle produce.
///
/// # Arguments
///
/// * `car` - the vehicle.
/// * `linear` - the forward speed.
/// * `steering` - the steering angle in radians.
///
/// # Returns
///
/// The yaw rate, or 0 for a vehicle with no wheelbase.
#[no_mangle]
pub extern "C" fn pamoja_ackermann_yaw_rate(
    car: PamojaAckermann,
    linear: f32,
    steering: f32,
) -> f32 {
    Ackermann::new(car.wheelbase).yaw_rate(linear, steering)
}

/// Returns the turn radius of a steering angle.
///
/// # Arguments
///
/// * `car` - the vehicle.
/// * `steering` - the steering angle in radians.
///
/// # Returns
///
/// The radius in meters, or infinity with the wheels straight.
#[no_mangle]
pub extern "C" fn pamoja_ackermann_turn_radius(car: PamojaAckermann, steering: f32) -> f32 {
    Ackermann::new(car.wheelbase).turn_radius(steering)
}

/// Returns the path curvature of a steering angle, the reciprocal of the turn radius.
///
/// # Arguments
///
/// * `car` - the vehicle.
/// * `steering` - the steering angle in radians.
///
/// # Returns
///
/// The curvature, or 0 for a vehicle with no wheelbase.
#[no_mangle]
pub extern "C" fn pamoja_ackermann_curvature(car: PamojaAckermann, steering: f32) -> f32 {
    Ackermann::new(car.wheelbase).curvature(steering)
}

/// A skid-steer drive: tracks or wheels `track` apart that slip sideways to turn.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaSkidSteer {
    /// The distance between the left and right sides; its magnitude is used.
    pub track: f32,
    /// How much wider the effective track is than the geometric one; its magnitude is
    /// used, and 0 is taken as 1, no slip.
    pub slip: f32,
}

/// Returns the side speeds a skid-steer drive needs for a body motion.
///
/// # Arguments
///
/// * `drive` - the drive.
/// * `linear` - the forward speed.
/// * `angular` - the yaw rate, positive turning left.
///
/// # Returns
///
/// The left and right speeds, split across the slip-corrected track.
#[no_mangle]
pub extern "C" fn pamoja_skid_steer_wheel_speeds(
    drive: PamojaSkidSteer,
    linear: f32,
    angular: f32,
) -> PamojaSideSpeeds {
    let (left, right) = SkidSteer::new(drive.track, drive.slip).wheel_speeds(linear, angular);
    PamojaSideSpeeds { left, right }
}

/// Returns the body motion a skid-steer drive makes from measured side speeds.
///
/// # Arguments
///
/// * `drive` - the drive.
/// * `left` - the left side's speed.
/// * `right` - the right side's speed.
///
/// # Returns
///
/// The forward speed and yaw rate; the yaw rate is 0 for a drive with no track.
#[no_mangle]
pub extern "C" fn pamoja_skid_steer_body_motion(
    drive: PamojaSkidSteer,
    left: f32,
    right: f32,
) -> PamojaBodyMotion {
    let (linear, angular) = SkidSteer::new(drive.track, drive.slip).body_motion(left, right);
    PamojaBodyMotion { linear, angular }
}

/// A mecanum base: four wheels `wheelbase` front to rear and `track` side to side.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaMecanum {
    /// The front-to-rear distance between the axles; its magnitude is used.
    pub wheelbase: f32,
    /// The left-to-right distance between the wheels; its magnitude is used.
    pub track: f32,
}

/// The four wheel speeds of a mecanum base.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaWheelSpeeds {
    /// The front-left wheel's speed.
    pub front_left: f32,
    /// The front-right wheel's speed.
    pub front_right: f32,
    /// The rear-left wheel's speed.
    pub rear_left: f32,
    /// The rear-right wheel's speed.
    pub rear_right: f32,
}

impl From<PamojaWheelSpeeds> for WheelSpeeds {
    fn from(value: PamojaWheelSpeeds) -> Self {
        WheelSpeeds {
            front_left: value.front_left,
            front_right: value.front_right,
            rear_left: value.rear_left,
            rear_right: value.rear_right,
        }
    }
}

impl From<WheelSpeeds> for PamojaWheelSpeeds {
    fn from(value: WheelSpeeds) -> Self {
        Self {
            front_left: value.front_left,
            front_right: value.front_right,
            rear_left: value.rear_left,
            rear_right: value.rear_right,
        }
    }
}

/// Returns the four wheel speeds a mecanum base needs for a body twist.
///
/// # Arguments
///
/// * `base` - the base.
/// * `twist` - the body motion, using its forward, sideways, and yaw parts.
///
/// # Returns
///
/// The four wheel speeds.
#[no_mangle]
pub extern "C" fn pamoja_mecanum_wheel_speeds(
    base: PamojaMecanum,
    twist: PamojaTwist,
) -> PamojaWheelSpeeds {
    Mecanum::new(base.wheelbase, base.track)
        .wheel_speeds(twist.into())
        .into()
}

/// Returns the body twist a mecanum base makes from measured wheel speeds.
///
/// # Arguments
///
/// * `base` - the base.
/// * `wheels` - the four measured wheel speeds.
///
/// # Returns
///
/// The body twist; its yaw rate is 0 for a base with no size.
#[no_mangle]
pub extern "C" fn pamoja_mecanum_body_motion(
    base: PamojaMecanum,
    wheels: PamojaWheelSpeeds,
) -> PamojaTwist {
    Mecanum::new(base.wheelbase, base.track)
        .body_motion(wheels.into())
        .into()
}

/// A planar two-link arm: a shoulder link `l1` long and an elbow link `l2` long.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaTwoLinkArm {
    /// The shoulder link's length; its magnitude is used.
    pub l1: f32,
    /// The elbow link's length; its magnitude is used.
    pub l2: f32,
}

/// Which way a two-link arm's elbow bends.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaElbow {
    /// The elbow angle is positive, counter-clockwise.
    Up = 0,
    /// The elbow angle is negative, clockwise.
    Down = 1,
}

/// The closest and farthest a two-link arm's hand reaches from its shoulder.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaReach {
    /// The closest reach, the difference of the link lengths.
    pub min: f32,
    /// The farthest reach, the sum of the link lengths.
    pub max: f32,
}

/// A point in a plane.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaPoint {
    /// The x coordinate.
    pub x: f32,
    /// The y coordinate.
    pub y: f32,
}

/// A two-link arm's joint angles, in radians.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaJoints {
    /// The shoulder angle, from the x axis.
    pub shoulder: f32,
    /// The elbow angle, relative to the first link.
    pub elbow: f32,
}

/// Returns how close and how far a two-link arm reaches.
///
/// # Arguments
///
/// * `arm` - the arm.
///
/// # Returns
///
/// The closest and farthest reach.
#[no_mangle]
pub extern "C" fn pamoja_two_link_arm_reach(arm: PamojaTwoLinkArm) -> PamojaReach {
    let (min, max) = TwoLinkArm::new(arm.l1, arm.l2).reach();
    PamojaReach { min, max }
}

/// Returns where a two-link arm's hand is for its joint angles.
///
/// # Arguments
///
/// * `arm` - the arm.
/// * `shoulder` - the shoulder angle, in radians from the x axis.
/// * `elbow` - the elbow angle, in radians relative to the first link.
///
/// # Returns
///
/// The hand's position.
#[no_mangle]
pub extern "C" fn pamoja_two_link_arm_tip(
    arm: PamojaTwoLinkArm,
    shoulder: f32,
    elbow: f32,
) -> PamojaPoint {
    let (x, y) = TwoLinkArm::new(arm.l1, arm.l2).tip(shoulder, elbow);
    PamojaPoint { x, y }
}

/// Finds the joint angles that put a two-link arm's hand at a point.
///
/// # Arguments
///
/// * `arm` - the arm.
/// * `x` - the target's x coordinate.
/// * `y` - the target's y coordinate.
/// * `elbow` - which way the elbow bends.
/// * `out_joints` - where to write the angles.
///
/// # Returns
///
/// `true` with the angles written to `out_joints`, or `false` for a target out of reach,
/// an arm with a link of no length, or a coordinate that is not a finite number.
///
/// # Safety
///
/// `out_joints` must point to a writable `PamojaJoints`, or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_two_link_arm_joints_for(
    arm: PamojaTwoLinkArm,
    x: f32,
    y: f32,
    elbow: PamojaElbow,
    out_joints: *mut PamojaJoints,
) -> bool {
    let branch = match elbow {
        PamojaElbow::Up => Elbow::Up,
        PamojaElbow::Down => Elbow::Down,
    };
    let Some((shoulder, elbow)) = TwoLinkArm::new(arm.l1, arm.l2).joints_for(x, y, branch) else {
        return false;
    };
    if let Some(slot) = out_joints.as_mut() {
        *slot = PamojaJoints { shoulder, elbow };
    }
    true
}

/// One joint of a serial arm in the Denavit-Hartenberg convention.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaDhParameters {
    /// The link length along the common normal, in meters.
    pub a: f32,
    /// The link twist about the common normal, in radians.
    pub alpha: f32,
    /// The link offset along the previous z axis, in meters.
    pub d: f32,
    /// The joint angle about the previous z axis, in radians.
    pub theta: f32,
}

impl From<PamojaDhParameters> for DhParameters {
    fn from(value: PamojaDhParameters) -> Self {
        DhParameters {
            a: value.a,
            alpha: value.alpha,
            d: value.d,
            theta: value.theta,
        }
    }
}

/// A 4x4 homogeneous transform, a rotation and a translation, stored row-major.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaTransform {
    /// The sixteen elements, row 0 first; the translation is elements 3, 7, and 11.
    pub m: [f32; 16],
}

impl From<Transform> for PamojaTransform {
    fn from(value: Transform) -> Self {
        Self { m: value.m }
    }
}

/// Returns the transform one Denavit-Hartenberg joint makes.
///
/// # Arguments
///
/// * `joint` - the joint's four parameters.
///
/// # Returns
///
/// The joint's homogeneous transform.
#[no_mangle]
pub extern "C" fn pamoja_dh_transform(joint: PamojaDhParameters) -> PamojaTransform {
    DhParameters::from(joint).transform().into()
}

/// Returns the transform from a serial arm's base to its tool.
///
/// # Arguments
///
/// * `joints` - the arm's joints, base first.
/// * `len` - how many joints `joints` holds.
///
/// # Returns
///
/// The composed transform, or the identity for an arm with no joints or a null `joints`.
///
/// # Safety
///
/// `joints` must point to `len` readable `PamojaDhParameters`, or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_forward_kinematics(
    joints: *const PamojaDhParameters,
    len: usize,
) -> PamojaTransform {
    if joints.is_null() || len == 0 {
        return Transform::identity().into();
    }
    let chain: Vec<DhParameters> = std::slice::from_raw_parts(joints, len)
        .iter()
        .map(|&joint| joint.into())
        .collect();
    forward_kinematics(&chain).into()
}

/// Returns the transform that applies `second` and then `first`, their product.
///
/// # Arguments
///
/// * `first` - the transform on the left of the product, nearer the base.
/// * `second` - the transform on the right, further down the chain.
///
/// # Returns
///
/// `first * second`.
#[no_mangle]
pub extern "C" fn pamoja_transform_multiply(
    first: PamojaTransform,
    second: PamojaTransform,
) -> PamojaTransform {
    Transform { m: first.m }
        .multiply(&Transform { m: second.m })
        .into()
}

/// Returns the identity transform: no rotation and no translation.
///
/// # Returns
///
/// The identity.
#[no_mangle]
pub extern "C" fn pamoja_transform_identity() -> PamojaTransform {
    Transform::identity().into()
}

/// An opaque handle to a dead-reckoned pose.
pub struct PamojaOdometry {
    inner: Odometry,
}

/// Creates an odometry estimate starting from a known pose.
///
/// # Arguments
///
/// * `start` - the starting pose; its heading is wrapped into `(-pi, pi]`.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_odometry_free`].
///
/// # Safety
///
/// The returned handle must be freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn pamoja_odometry_new(start: PamojaPose) -> *mut PamojaOdometry {
    Box::into_raw(Box::new(PamojaOdometry {
        inner: Odometry::new(start.into()),
    }))
}

/// Reads an odometry estimate's pose.
///
/// # Returns
///
/// The pose, or the origin if `odometry` is null.
///
/// # Safety
///
/// `odometry` must be a live handle from [`pamoja_odometry_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_odometry_pose(odometry: *const PamojaOdometry) -> PamojaPose {
    match odometry.as_ref() {
        Some(odometry) => odometry.inner.pose().into(),
        None => Pose::origin().into(),
    }
}

/// Sets an odometry estimate to a known pose.
///
/// # Safety
///
/// `odometry` must be a live handle from [`pamoja_odometry_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_odometry_reset(odometry: *mut PamojaOdometry, pose: PamojaPose) {
    if let Some(odometry) = odometry.as_mut() {
        odometry.inner.reset(pose.into());
    }
}

/// Integrates a body motion over a time step onto an odometry estimate.
///
/// A speed, rate, or time step that is not a finite number is ignored.
///
/// # Returns
///
/// The new pose, or the origin if `odometry` is null.
///
/// # Safety
///
/// `odometry` must be a live handle from [`pamoja_odometry_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_odometry_integrate(
    odometry: *mut PamojaOdometry,
    linear: f32,
    angular: f32,
    dt: f32,
) -> PamojaPose {
    match odometry.as_mut() {
        Some(odometry) => odometry.inner.integrate(linear, angular, dt).into(),
        None => Pose::origin().into(),
    }
}

/// Integrates the distances two wheels rolled onto an odometry estimate.
///
/// A distance that is not a finite number is ignored.
///
/// # Returns
///
/// The new pose, or the origin if `odometry` is null.
///
/// # Safety
///
/// `odometry` must be a live handle from [`pamoja_odometry_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_odometry_integrate_wheels(
    odometry: *mut PamojaOdometry,
    left: f32,
    right: f32,
    drive: PamojaDiffDrive,
) -> PamojaPose {
    match odometry.as_mut() {
        Some(odometry) => odometry
            .inner
            .integrate_wheels(left, right, &DiffDrive::new(drive.track))
            .into(),
        None => Pose::origin().into(),
    }
}

/// Nudges an odometry estimate's heading toward an absolute measurement.
///
/// `weight` is how strongly to trust `measured`, from 0 to 1. A measurement that is not a
/// finite number, or a weight that is not a number, leaves the heading as it was.
///
/// # Safety
///
/// `odometry` must be a live handle from [`pamoja_odometry_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_odometry_fuse_heading(
    odometry: *mut PamojaOdometry,
    measured: f32,
    weight: f32,
) {
    if let Some(odometry) = odometry.as_mut() {
        odometry.inner.fuse_heading(measured, weight);
    }
}

/// Releases an odometry handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `odometry` must be a handle from [`pamoja_odometry_new`] that has not already been
/// freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_odometry_free(odometry: *mut PamojaOdometry) {
    if !odometry.is_null() {
        drop(Box::from_raw(odometry));
    }
}

/// Steers toward a waypoint: the speeds and gains of a carrot-following guide.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaWaypointFollower {
    /// The forward speed when pointed at the target; its magnitude is used.
    pub cruise: f32,
    /// How close, in meters, counts as arrived; its magnitude is used.
    pub arrival_m: f64,
    /// The yaw rate commanded per radian of heading error; its magnitude is used.
    pub heading_gain: f32,
    /// The largest yaw rate to command; its magnitude is used.
    pub max_angular: f32,
}

/// The command toward a waypoint, with the geometry behind it.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaGuidance {
    /// The body motion to drive.
    pub twist: PamojaTwist,
    /// The distance left to the target, in meters.
    pub distance_m: f64,
    /// The heading error to the target, in degrees, in `(-180, 180]`.
    pub heading_error_deg: f32,
    /// 1 once the target is within the arrival radius, else 0.
    pub arrived: u8,
}

/// Produces the command from a robot's position and heading toward a target.
///
/// # Arguments
///
/// * `follower` - the guide's speeds and gains.
/// * `here` - the robot's position.
/// * `heading_deg` - the robot's heading in degrees clockwise from north.
/// * `target` - the waypoint.
///
/// # Returns
///
/// The guidance. Within the arrival radius its twist is zero and `arrived` is 1; a heading
/// or position that is not a finite number also gives a zero twist.
#[no_mangle]
pub extern "C" fn pamoja_waypoint_follower_guide(
    follower: PamojaWaypointFollower,
    here: PamojaCoordinate,
    heading_deg: f32,
    target: PamojaCoordinate,
) -> PamojaGuidance {
    let guidance = WaypointFollower::new(
        follower.cruise,
        follower.arrival_m,
        follower.heading_gain,
        follower.max_angular,
    )
    .guide(
        Coordinate::from(here),
        heading_deg,
        Coordinate::from(target),
    );
    PamojaGuidance {
        twist: guidance.twist.into(),
        distance_m: guidance.distance_m,
        heading_error_deg: guidance.heading_error_deg,
        arrived: u8::from(guidance.arrived),
    }
}

/// Cuts forward and sideways motion when an obstacle is within the stopping distance.
///
/// # Arguments
///
/// * `twist` - the requested motion.
/// * `range_m` - the nearest range ahead, in meters.
/// * `stop_distance_m` - the range at or below which motion is cut; its magnitude is used.
///
/// # Returns
///
/// `twist` while the way is clear, or it with no forward or sideways speed, keeping the
/// turn. A range that is not a number counts as an obstacle.
#[no_mangle]
pub extern "C" fn pamoja_obstacle_stop(
    twist: PamojaTwist,
    range_m: f32,
    stop_distance_m: f32,
) -> PamojaTwist {
    obstacle_stop(twist.into(), range_m, stop_distance_m).into()
}

/// An opaque handle to a latching emergency stop.
pub struct PamojaEStop {
    inner: EStop,
}

/// Creates an e-stop that is not engaged.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_estop_free`].
///
/// # Safety
///
/// The returned handle must be freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn pamoja_estop_new() -> *mut PamojaEStop {
    Box::into_raw(Box::new(PamojaEStop {
        inner: EStop::new(),
    }))
}

/// Engages an e-stop; it holds until reset.
///
/// # Safety
///
/// `estop` must be a live handle from [`pamoja_estop_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_estop_engage(estop: *mut PamojaEStop) {
    if let Some(estop) = estop.as_mut() {
        estop.inner.engage();
    }
}

/// Clears an e-stop.
///
/// # Safety
///
/// `estop` must be a live handle from [`pamoja_estop_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_estop_reset(estop: *mut PamojaEStop) {
    if let Some(estop) = estop.as_mut() {
        estop.inner.reset();
    }
}

/// Reports whether an e-stop is engaged.
///
/// # Returns
///
/// `true` while it is engaged, and also if `estop` is null, so a missing stop reads as a
/// stop.
///
/// # Safety
///
/// `estop` must be a live handle from [`pamoja_estop_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_estop_is_engaged(estop: *const PamojaEStop) -> bool {
    match estop.as_ref() {
        Some(estop) => estop.inner.is_engaged(),
        None => true,
    }
}

/// Passes a command through an e-stop.
///
/// # Returns
///
/// `desired` while the stop is clear, or a zero twist while it is engaged or if `estop` is
/// null.
///
/// # Safety
///
/// `estop` must be a live handle from [`pamoja_estop_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_estop_gate(
    estop: *const PamojaEStop,
    desired: PamojaTwist,
) -> PamojaTwist {
    match estop.as_ref() {
        Some(estop) => estop.inner.gate(desired.into()).into(),
        None => Twist::zero().into(),
    }
}

/// Releases an e-stop handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `estop` must be a handle from [`pamoja_estop_new`] that has not already been freed, or
/// null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_estop_free(estop: *mut PamojaEStop) {
    if !estop.is_null() {
        drop(Box::from_raw(estop));
    }
}

/// An opaque handle to a deadman timer.
pub struct PamojaWatchdog {
    inner: Watchdog,
}

/// Creates a watchdog that expires after `timeout` without being fed.
///
/// A timeout that is not a number is taken as 0, so the watchdog expires rather than never.
///
/// # Returns
///
/// A freshly fed watchdog; the caller must release it with [`pamoja_watchdog_free`].
///
/// # Safety
///
/// The returned handle must be freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn pamoja_watchdog_new(timeout: f32) -> *mut PamojaWatchdog {
    Box::into_raw(Box::new(PamojaWatchdog {
        inner: Watchdog::new(timeout),
    }))
}

/// Feeds a watchdog, restarting its silence timer.
///
/// # Safety
///
/// `watchdog` must be a live handle from [`pamoja_watchdog_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_watchdog_feed(watchdog: *mut PamojaWatchdog) {
    if let Some(watchdog) = watchdog.as_mut() {
        watchdog.inner.feed();
    }
}

/// Advances a watchdog's timer and reports whether it has expired.
///
/// A time step that is not a finite number expires the watchdog until it is fed.
///
/// # Returns
///
/// `true` if it has expired, and also if `watchdog` is null.
///
/// # Safety
///
/// `watchdog` must be a live handle from [`pamoja_watchdog_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_watchdog_update(watchdog: *mut PamojaWatchdog, dt: f32) -> bool {
    match watchdog.as_mut() {
        Some(watchdog) => watchdog.inner.update(dt),
        None => true,
    }
}

/// Reports whether a watchdog has expired.
///
/// # Returns
///
/// `true` if the time since feeding is past the timeout, and also if `watchdog` is null.
///
/// # Safety
///
/// `watchdog` must be a live handle from [`pamoja_watchdog_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_watchdog_is_expired(watchdog: *const PamojaWatchdog) -> bool {
    match watchdog.as_ref() {
        Some(watchdog) => watchdog.inner.is_expired(),
        None => true,
    }
}

/// Releases a watchdog handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `watchdog` must be a handle from [`pamoja_watchdog_new`] that has not already been
/// freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_watchdog_free(watchdog: *mut PamojaWatchdog) {
    if !watchdog.is_null() {
        drop(Box::from_raw(watchdog));
    }
}

/// An opaque handle to speed and acceleration limits, with the motion they last allowed.
pub struct PamojaLimits {
    inner: Limits,
}

/// Creates limits on speed and acceleration, starting from rest.
///
/// Each ceiling's magnitude is used, and one that is not a number is taken as 0, which holds
/// the robot still.
///
/// # Arguments
///
/// * `max_linear` - the largest planar speed.
/// * `max_angular` - the largest yaw rate.
/// * `max_linear_accel` - the largest change in speed per second.
/// * `max_angular_accel` - the largest change in yaw rate per second.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_limits_free`].
///
/// # Safety
///
/// The returned handle must be freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn pamoja_limits_new(
    max_linear: f32,
    max_angular: f32,
    max_linear_accel: f32,
    max_angular_accel: f32,
) -> *mut PamojaLimits {
    Box::into_raw(Box::new(PamojaLimits {
        inner: Limits::new(max_linear, max_angular, max_linear_accel, max_angular_accel),
    }))
}

/// Bounds a command in speed and acceleration.
///
/// A command part that is not a finite number is taken as 0, and a time step that is not
/// finite allows no change.
///
/// # Returns
///
/// The bounded command, or a zero twist if `limits` is null.
///
/// # Safety
///
/// `limits` must be a live handle from [`pamoja_limits_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_limits_apply(
    limits: *mut PamojaLimits,
    desired: PamojaTwist,
    dt: f32,
) -> PamojaTwist {
    match limits.as_mut() {
        Some(limits) => limits.inner.apply(desired.into(), dt).into(),
        None => Twist::zero().into(),
    }
}

/// Forgets the motion limits last allowed, so the next command eases up from rest.
///
/// # Safety
///
/// `limits` must be a live handle from [`pamoja_limits_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_limits_reset(limits: *mut PamojaLimits) {
    if let Some(limits) = limits.as_mut() {
        limits.inner.reset();
    }
}

/// Releases a limits handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `limits` must be a handle from [`pamoja_limits_new`] that has not already been freed,
/// or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_limits_free(limits: *mut PamojaLimits) {
    if !limits.is_null() {
        drop(Box::from_raw(limits));
    }
}

/// An opaque handle to the gate every motion command passes through: an e-stop, a
/// watchdog, and limits.
pub struct PamojaSafetyGate {
    inner: SafetyGate,
}

/// Creates a safety gate from limits and a watchdog timeout.
///
/// # Arguments
///
/// * `limits` - the limits for normal motion, copied as they stand; the handle stays the
///   caller's.
/// * `watchdog_timeout` - the silence after which the gate stops the robot.
///
/// # Returns
///
/// A cleared, freshly fed gate the caller must release with [`pamoja_safety_gate_free`],
/// or null if `limits` is null.
///
/// # Safety
///
/// `limits` must be a live handle from [`pamoja_limits_new`], or null. A returned handle
/// must be freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn pamoja_safety_gate_new(
    limits: *const PamojaLimits,
    watchdog_timeout: f32,
) -> *mut PamojaSafetyGate {
    let Some(limits) = limits.as_ref() else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(PamojaSafetyGate {
        inner: SafetyGate::new(limits.inner, watchdog_timeout),
    }))
}

/// Feeds a safety gate's watchdog; call it whenever a fresh command arrives.
///
/// # Safety
///
/// `gate` must be a live handle from [`pamoja_safety_gate_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_safety_gate_feed(gate: *mut PamojaSafetyGate) {
    if let Some(gate) = gate.as_mut() {
        gate.inner.feed();
    }
}

/// Engages a safety gate's latching e-stop.
///
/// # Safety
///
/// `gate` must be a live handle from [`pamoja_safety_gate_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_safety_gate_engage_estop(gate: *mut PamojaSafetyGate) {
    if let Some(gate) = gate.as_mut() {
        gate.inner.engage_estop();
    }
}

/// Clears a safety gate's e-stop.
///
/// # Safety
///
/// `gate` must be a live handle from [`pamoja_safety_gate_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_safety_gate_reset_estop(gate: *mut PamojaSafetyGate) {
    if let Some(gate) = gate.as_mut() {
        gate.inner.reset_estop();
    }
}

/// Reports whether a safety gate is forcing a stop.
///
/// # Returns
///
/// `true` while its e-stop is engaged or its watchdog has expired, and also if `gate` is
/// null.
///
/// # Safety
///
/// `gate` must be a live handle from [`pamoja_safety_gate_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_safety_gate_is_stopped(gate: *const PamojaSafetyGate) -> bool {
    match gate.as_ref() {
        Some(gate) => gate.inner.is_stopped(),
        None => true,
    }
}

/// Returns the command that is safe to drive for a desired one over a time step.
///
/// # Returns
///
/// A zero twist while the gate is stopped or if `gate` is null, and otherwise `desired`
/// bounded by the limits.
///
/// # Safety
///
/// `gate` must be a live handle from [`pamoja_safety_gate_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_safety_gate_command(
    gate: *mut PamojaSafetyGate,
    desired: PamojaTwist,
    dt: f32,
) -> PamojaTwist {
    match gate.as_mut() {
        Some(gate) => gate.inner.command(desired.into(), dt).into(),
        None => Twist::zero().into(),
    }
}

/// Releases a safety gate handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `gate` must be a handle from [`pamoja_safety_gate_new`] that has not already been freed,
/// or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_safety_gate_free(gate: *mut PamojaSafetyGate) {
    if !gate.is_null() {
        drop(Box::from_raw(gate));
    }
}

/// A hobby servo's pulse range and travel.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaServoMap {
    /// The pulse width at zero degrees, in microseconds.
    pub min_us: u16,
    /// The pulse width at full travel, in microseconds.
    pub max_us: u16,
    /// The full travel in degrees; its magnitude is used.
    pub range_deg: f32,
}

/// Returns the standard hobby servo: 1000 to 2000 microseconds over 180 degrees.
///
/// # Returns
///
/// The standard servo.
#[no_mangle]
pub extern "C" fn pamoja_servo_map_standard() -> PamojaServoMap {
    let standard = ServoMap::standard();
    PamojaServoMap {
        min_us: standard.min_us(),
        max_us: standard.max_us(),
        range_deg: standard.range_deg(),
    }
}

/// Returns the pulse width that sets a servo to an angle.
///
/// # Arguments
///
/// * `servo` - the servo.
/// * `angle_deg` - the angle in degrees, held to the servo's travel.
///
/// # Returns
///
/// The pulse width in microseconds, or 0, no pulse, for an angle that is not a number.
#[no_mangle]
pub extern "C" fn pamoja_servo_map_pulse(servo: PamojaServoMap, angle_deg: f32) -> u16 {
    ServoMap::new(servo.min_us, servo.max_us, servo.range_deg).pulse(angle_deg)
}

/// Returns the angle a servo pulse width sets.
///
/// # Arguments
///
/// * `servo` - the servo.
/// * `pulse_us` - the pulse width in microseconds, held to the servo's range.
///
/// # Returns
///
/// The angle in degrees, or 0 for a servo whose pulse range is empty.
#[no_mangle]
pub extern "C" fn pamoja_servo_map_angle(servo: PamojaServoMap, pulse_us: u16) -> f32 {
    ServoMap::new(servo.min_us, servo.max_us, servo.range_deg).angle(pulse_us)
}

/// An electronic speed controller's reverse, neutral, and forward pulse widths.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaEsc {
    /// The pulse width at full reverse, in microseconds.
    pub min_us: u16,
    /// The pulse width at rest, in microseconds.
    pub neutral_us: u16,
    /// The pulse width at full forward, in microseconds.
    pub max_us: u16,
}

/// Returns the common reversible ESC: 1000, 1500, and 2000 microseconds.
///
/// # Returns
///
/// The reversible ESC.
#[no_mangle]
pub extern "C" fn pamoja_esc_bidirectional() -> PamojaEsc {
    let reversible = Esc::bidirectional();
    PamojaEsc {
        min_us: reversible.min_us(),
        neutral_us: reversible.neutral_us(),
        max_us: reversible.max_us(),
    }
}

/// Returns the pulse width for a throttle.
///
/// # Arguments
///
/// * `esc` - the controller.
/// * `throttle` - the demand from -1, full reverse, to 1, full forward, held to that range.
///
/// # Returns
///
/// The pulse width in microseconds, or the neutral one for a throttle that is not a number.
#[no_mangle]
pub extern "C" fn pamoja_esc_pulse(esc: PamojaEsc, throttle: f32) -> u16 {
    Esc::new(esc.min_us, esc.neutral_us, esc.max_us).pulse(throttle)
}

/// An opaque handle to a quadrature encoder decoder.
pub struct PamojaQuadrature {
    inner: Quadrature,
}

/// Creates a quadrature decoder seeded with the encoder's current channel levels.
///
/// # Arguments
///
/// * `a` - the A channel's level now.
/// * `b` - the B channel's level now.
///
/// # Returns
///
/// A decoder with a count of 0 the caller must release with [`pamoja_quadrature_free`].
///
/// # Safety
///
/// The returned handle must be freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn pamoja_quadrature_new(a: bool, b: bool) -> *mut PamojaQuadrature {
    Box::into_raw(Box::new(PamojaQuadrature {
        inner: Quadrature::starting(a, b),
    }))
}

/// Feeds a quadrature decoder the channel levels it reads now.
///
/// # Returns
///
/// 1 or -1 for a step in either direction, and 0 for no change, a jump past a step, or a
/// null `quadrature`.
///
/// # Safety
///
/// `quadrature` must be a live handle from [`pamoja_quadrature_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_quadrature_update(
    quadrature: *mut PamojaQuadrature,
    a: bool,
    b: bool,
) -> i8 {
    match quadrature.as_mut() {
        Some(quadrature) => quadrature.inner.update(a, b),
        None => 0,
    }
}

/// Reads a quadrature decoder's signed count of steps.
///
/// # Returns
///
/// The count, or 0 if `quadrature` is null.
///
/// # Safety
///
/// `quadrature` must be a live handle from [`pamoja_quadrature_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_quadrature_count(quadrature: *const PamojaQuadrature) -> i64 {
    match quadrature.as_ref() {
        Some(quadrature) => quadrature.inner.count(),
        None => 0,
    }
}

/// Sets a quadrature decoder's count back to 0, keeping the channel state it last read.
///
/// # Safety
///
/// `quadrature` must be a live handle from [`pamoja_quadrature_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_quadrature_reset(quadrature: *mut PamojaQuadrature) {
    if let Some(quadrature) = quadrature.as_mut() {
        quadrature.inner.reset();
    }
}

/// Releases a quadrature decoder handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `quadrature` must be a handle from [`pamoja_quadrature_new`] that has not already been
/// freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_quadrature_free(quadrature: *mut PamojaQuadrature) {
    if !quadrature.is_null() {
        drop(Box::from_raw(quadrature));
    }
}

/// An encoder's resolution and the radius of the wheel it turns with.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaQuadratureScale {
    /// Steps per wheel revolution; its magnitude is used.
    pub counts_per_rev: f32,
    /// The wheel radius in meters; its magnitude is used.
    pub wheel_radius: f32,
}

/// Returns the distance a wheel rolled for a step count.
///
/// # Returns
///
/// The distance in meters, or 0 for a scale with no resolution.
#[no_mangle]
pub extern "C" fn pamoja_quadrature_scale_distance(
    scale: PamojaQuadratureScale,
    count: i64,
) -> f32 {
    QuadratureScale::new(scale.counts_per_rev, scale.wheel_radius).distance(count)
}

/// Returns a wheel's speed from the steps counted over a time step.
///
/// # Returns
///
/// The speed in meters per second, or 0 when `dt` is 0.
#[no_mangle]
pub extern "C" fn pamoja_quadrature_scale_velocity(
    scale: PamojaQuadratureScale,
    delta_count: i64,
    dt: f32,
) -> f32 {
    QuadratureScale::new(scale.counts_per_rev, scale.wheel_radius).velocity(delta_count, dt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_differential_drive_turns_both_ways() {
        let drive = PamojaDiffDrive { track: 0.5 };
        let spin = pamoja_diff_drive_wheel_speeds(drive, 0.0, 2.0);
        assert_eq!((spin.left, spin.right), (-0.5, 0.5));
        let body = pamoja_diff_drive_body_motion(drive, -0.5, 0.5);
        assert_eq!((body.linear, body.angular), (0.0, 2.0));
    }

    #[test]
    fn a_car_goes_straight_with_its_wheels_straight() {
        let car = PamojaAckermann { wheelbase: 2.5 };
        assert_eq!(pamoja_ackermann_turn_radius(car, 0.0), f32::INFINITY);
        let omega = pamoja_ackermann_yaw_rate(car, 5.0, 0.4);
        assert!((pamoja_ackermann_steering_angle(car, 5.0, omega) - 0.4).abs() < 1e-5);
    }

    #[test]
    fn a_mecanum_base_round_trips_a_twist() {
        let base = PamojaMecanum {
            wheelbase: 0.5,
            track: 0.4,
        };
        let twist = PamojaTwist {
            vx: 0.8,
            vy: -0.3,
            omega: 0.6,
        };
        let back = pamoja_mecanum_body_motion(base, pamoja_mecanum_wheel_speeds(base, twist));
        assert!((back.vx - 0.8).abs() < 1e-6 && (back.vy + 0.3).abs() < 1e-6);
        assert!((back.omega - 0.6).abs() < 1e-6);
    }

    #[test]
    fn an_arm_finds_the_joints_for_where_its_hand_went() {
        let arm = PamojaTwoLinkArm { l1: 1.0, l2: 1.0 };
        let tip = pamoja_two_link_arm_tip(arm, 0.5, 0.7);
        let mut joints = PamojaJoints {
            shoulder: 0.0,
            elbow: 0.0,
        };
        // Safety: the out-pointer is a local.
        unsafe {
            assert!(pamoja_two_link_arm_joints_for(
                arm,
                tip.x,
                tip.y,
                PamojaElbow::Up,
                &mut joints
            ));
            assert!(!pamoja_two_link_arm_joints_for(
                arm,
                5.0,
                0.0,
                PamojaElbow::Up,
                &mut joints
            ));
        }
        assert!((joints.shoulder - 0.5).abs() < 1e-4 && (joints.elbow - 0.7).abs() < 1e-4);
        assert_eq!(
            pamoja_two_link_arm_reach(arm),
            PamojaReach { min: 0.0, max: 2.0 }
        );
    }

    #[test]
    fn forward_kinematics_reaches_out_along_a_flat_arm() {
        let link = PamojaDhParameters {
            a: 1.0,
            alpha: 0.0,
            d: 0.0,
            theta: 0.0,
        };
        // Safety: the slice outlives the call, and null is handled.
        unsafe {
            let tool = pamoja_forward_kinematics([link, link].as_ptr(), 2);
            assert!((tool.m[3] - 2.0).abs() < 1e-6 && tool.m[7].abs() < 1e-6);
            assert_eq!(
                pamoja_forward_kinematics(ptr::null(), 3),
                pamoja_transform_identity()
            );
        }
    }

    #[test]
    fn odometry_follows_a_quarter_circle() {
        // Safety: the handle is live for the whole test and freed once.
        unsafe {
            let odometry = pamoja_odometry_new(PamojaPose {
                x: 0.0,
                y: 0.0,
                theta: 0.0,
            });
            let pose = pamoja_odometry_integrate(odometry, 1.0, 1.0, core::f32::consts::FRAC_PI_2);
            assert!((pose.x - 1.0).abs() < 1e-5 && (pose.y - 1.0).abs() < 1e-5);
            assert_eq!(
                pamoja_odometry_integrate(odometry, f32::NAN, 1.0, 1.0),
                pose
            );
            pamoja_odometry_free(odometry);
            assert_eq!(
                pamoja_odometry_pose(ptr::null()),
                PamojaPose {
                    x: 0.0,
                    y: 0.0,
                    theta: 0.0
                }
            );
        }
    }

    #[test]
    fn a_follower_arrives_and_an_obstacle_stops_the_robot() {
        let follower = PamojaWaypointFollower {
            cruise: 1.5,
            arrival_m: 3.0,
            heading_gain: 1.5,
            max_angular: 1.0,
        };
        let here = PamojaCoordinate {
            latitude: 0.0,
            longitude: 0.0,
        };
        let east = PamojaCoordinate {
            latitude: 0.0,
            longitude: 0.01,
        };
        let guidance = pamoja_waypoint_follower_guide(follower, here, 90.0, east);
        assert_eq!(guidance.arrived, 0);
        assert!((guidance.twist.vx - 1.5).abs() < 1e-3);
        assert_eq!(
            pamoja_waypoint_follower_guide(follower, here, 90.0, here).arrived,
            1
        );

        let driving = PamojaTwist {
            vx: 1.0,
            vy: 0.0,
            omega: 0.5,
        };
        let stopped = pamoja_obstacle_stop(driving, f32::NAN, 0.5);
        assert_eq!((stopped.vx, stopped.omega), (0.0, 0.5));
    }

    #[test]
    fn a_safety_gate_eases_on_and_stops_on_its_estop() {
        // Safety: every handle is live until freed once, and null is handled.
        unsafe {
            let limits = pamoja_limits_new(1.0, 2.0, 0.5, 4.0);
            let gate = pamoja_safety_gate_new(limits, 0.2);
            pamoja_limits_free(limits);
            let ahead = PamojaTwist {
                vx: 1.0,
                vy: 0.0,
                omega: 0.0,
            };
            pamoja_safety_gate_feed(gate);
            assert!((pamoja_safety_gate_command(gate, ahead, 0.1).vx - 0.05).abs() < 1e-6);
            pamoja_safety_gate_engage_estop(gate);
            assert!(pamoja_safety_gate_is_stopped(gate));
            assert_eq!(pamoja_safety_gate_command(gate, ahead, 0.1).vx, 0.0);
            pamoja_safety_gate_free(gate);
            assert!(pamoja_safety_gate_new(ptr::null(), 0.2).is_null());
            assert!(pamoja_safety_gate_is_stopped(ptr::null()));

            let dog = pamoja_watchdog_new(0.5);
            assert!(!pamoja_watchdog_update(dog, 0.3));
            assert!(pamoja_watchdog_update(dog, f32::NAN));
            pamoja_watchdog_feed(dog);
            assert!(!pamoja_watchdog_is_expired(dog));
            pamoja_watchdog_free(dog);

            let estop = pamoja_estop_new();
            pamoja_estop_engage(estop);
            assert_eq!(pamoja_estop_gate(estop, ahead).vx, 0.0);
            pamoja_estop_reset(estop);
            assert_eq!(pamoja_estop_gate(estop, ahead), ahead);
            pamoja_estop_free(estop);
        }
    }

    #[test]
    fn servo_esc_and_encoder_convert_to_their_units() {
        let servo = pamoja_servo_map_standard();
        assert_eq!(pamoja_servo_map_pulse(servo, 90.0), 1500);
        assert_eq!(pamoja_servo_map_pulse(servo, f32::NAN), 0);
        assert!((pamoja_servo_map_angle(servo, 1500) - 90.0).abs() < 1e-3);
        assert_eq!(pamoja_esc_pulse(pamoja_esc_bidirectional(), 0.5), 1750);

        // Safety: the handle is live for the block and freed once.
        unsafe {
            let encoder = pamoja_quadrature_new(false, false);
            for (a, b) in [(false, true), (true, true), (true, false), (false, false)] {
                assert_eq!(pamoja_quadrature_update(encoder, a, b), 1);
            }
            assert_eq!(pamoja_quadrature_count(encoder), 4);
            pamoja_quadrature_reset(encoder);
            assert_eq!(pamoja_quadrature_count(encoder), 0);
            pamoja_quadrature_free(encoder);
        }
        let scale = PamojaQuadratureScale {
            counts_per_rev: 360.0,
            wheel_radius: 0.05,
        };
        let turn = pamoja_quadrature_scale_distance(scale, 360);
        assert!((turn - 2.0 * core::f32::consts::PI * 0.05).abs() < 1e-6);
        assert_eq!(pamoja_quadrature_scale_velocity(scale, 10, 0.0), 0.0);
    }
}
