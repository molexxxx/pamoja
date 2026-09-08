//! Steering a robot toward a waypoint, and stopping before an obstacle.

use crate::motion::{clamp, magnitude, Twist};
use crate::Coordinate;
use core::f32::consts::PI;
use libm::cosf;

// Wraps an angle in degrees into the half-open interval `(-180, 180]`.
fn wrap_deg_180(angle: f32) -> f32 {
    let mut a = angle % 360.0;
    if a > 180.0 {
        a -= 360.0;
    } else if a <= -180.0 {
        a += 360.0;
    }
    a
}

// `f64::abs` lives in `std`, so this `no_std` crate takes an f64 magnitude by hand.
fn magnitude_f64(value: f64) -> f64 {
    if value < 0.0 {
        -value
    } else {
        value
    }
}

/// The steering command toward a waypoint, with the geometry behind it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guidance {
    /// The body twist to drive: a forward speed and a yaw rate toward the target.
    pub twist: Twist,
    /// The remaining distance to the target, in meters.
    pub distance_m: f64,
    /// The heading error to the target, in degrees, in `(-180, 180]`.
    pub heading_error_deg: f32,
    /// Whether the target is within the arrival radius.
    pub arrived: bool,
}

/// Guides a robot from waypoint to waypoint by GPS-style coordinates (carrot following).
///
/// This is the "go to that point" primitive behind a patrol route, a return-to-base, or a field
/// pass: given where the robot is, which way it faces, and the next waypoint, it produces the
/// twist to get there. It turns toward the target in proportion to the heading error and slows
/// the forward speed as that error grows (by the cosine of the error), so the robot pivots toward
/// a target behind it before driving off, rather than swinging wide. The caller holds the list of
/// waypoints and advances to the next once [`Guidance::arrived`] is set, which keeps this
/// allocation-free.
///
/// # Examples
///
/// ```
/// use pamoja_kit::{Coordinate, WaypointFollower};
///
/// // Cruise 1.5 m/s, arrive within 3 m, turn at 1.5 rad per rad of error, cap 1 rad/s.
/// let follower = WaypointFollower::new(1.5, 3.0, 1.5, 1.0);
///
/// // At the equator/prime meridian facing east (90 deg), with the target due east.
/// let here = Coordinate::new(0.0, 0.0);
/// let target = Coordinate::new(0.0, 0.01);
/// let g = follower.guide(here, 90.0, target);
/// assert!(g.heading_error_deg.abs() < 1e-3); // already pointed at it
/// assert!((g.twist.vx - 1.5).abs() < 1e-3); // so drive at cruise
/// assert!(!g.arrived);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct WaypointFollower {
    cruise: f32,
    arrival_m: f64,
    heading_gain: f32,
    max_angular: f32,
}

impl WaypointFollower {
    /// Creates a follower with the given speeds and tolerances.
    ///
    /// # Arguments
    ///
    /// * `cruise` - the forward speed when pointed at the target; its magnitude is used.
    /// * `arrival_m` - how close, in meters, counts as arrived; its magnitude is used.
    /// * `heading_gain` - yaw rate commanded per radian of heading error; its magnitude is used.
    /// * `max_angular` - the largest yaw rate to command; its magnitude is used.
    ///
    /// # Returns
    ///
    /// The follower.
    pub fn new(cruise: f32, arrival_m: f64, heading_gain: f32, max_angular: f32) -> Self {
        Self {
            cruise: magnitude(cruise),
            arrival_m: magnitude_f64(arrival_m),
            heading_gain: magnitude(heading_gain),
            max_angular: magnitude(max_angular),
        }
    }

    /// Produces the steering command from the robot's position and heading to a target.
    ///
    /// # Arguments
    ///
    /// * `here` - the robot's current coordinate.
    /// * `heading_deg` - the robot's heading in degrees clockwise from north (a compass course).
    /// * `target` - the waypoint to head toward.
    ///
    /// # Returns
    ///
    /// The [`Guidance`]; once within the arrival radius the twist is zero and `arrived` is set.
    pub fn guide(&self, here: Coordinate, heading_deg: f32, target: Coordinate) -> Guidance {
        let distance_m = here.distance_to(target);
        let bearing_deg = here.bearing_to(target) as f32;
        let heading_error_deg = wrap_deg_180(bearing_deg - heading_deg);

        if distance_m <= self.arrival_m {
            return Guidance {
                twist: Twist::zero(),
                distance_m,
                heading_error_deg,
                arrived: true,
            };
        }

        let error_rad = heading_error_deg * (PI / 180.0);
        let angular = clamp(
            self.heading_gain * error_rad,
            -self.max_angular,
            self.max_angular,
        );
        let facing = cosf(error_rad);
        let forward = if facing > 0.0 {
            self.cruise * facing
        } else {
            0.0
        };

        Guidance {
            twist: Twist::planar(forward, angular),
            distance_m,
            heading_error_deg,
            arrived: false,
        }
    }
}

/// Stops forward motion when an obstacle is within the stopping distance, leaving turning free.
///
/// This is the simplest reliable safety reflex for a robot with a forward range sensor: hold the
/// requested rotation so the robot can still turn away, but cut `vx` and `vy` to zero once the
/// nearest reading falls inside the stop distance, so it does not drive into what it sees.
///
/// # Arguments
///
/// * `twist` - the requested body motion.
/// * `range_m` - the nearest measured range ahead, in meters.
/// * `stop_distance_m` - the range at or below which forward motion is cut; its magnitude is used.
///
/// # Returns
///
/// The original twist when the way is clear, or one with no translation (rotation preserved) when
/// an obstacle is within range.
///
/// # Examples
///
/// ```
/// use pamoja_kit::{obstacle_stop, Twist};
///
/// let driving = Twist::new(1.0, 0.0, 0.5);
/// // Clear ahead: unchanged.
/// assert_eq!(obstacle_stop(driving, 2.0, 0.5), driving);
/// // Obstacle at 0.3 m: forward cut, turn kept so it can escape.
/// assert_eq!(obstacle_stop(driving, 0.3, 0.5), Twist::new(0.0, 0.0, 0.5));
/// ```
pub fn obstacle_stop(twist: Twist, range_m: f32, stop_distance_m: f32) -> Twist {
    if range_m <= magnitude(stop_distance_m) {
        Twist::new(0.0, 0.0, twist.omega)
    } else {
        twist
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_drives_at_cruise_when_pointed_at_the_target() {
        let follower = WaypointFollower::new(1.5, 3.0, 1.5, 1.0);
        let here = Coordinate::new(0.0, 0.0);
        let target = Coordinate::new(0.0, 0.01); // due east
        let g = follower.guide(here, 90.0, target); // facing east
        assert!(g.heading_error_deg.abs() < 1e-3);
        assert!((g.twist.vx - 1.5).abs() < 1e-3);
        assert!(g.twist.omega.abs() < 1e-3);
        assert!(!g.arrived);
    }

    #[test]
    fn it_pivots_without_driving_when_the_target_is_behind() {
        let follower = WaypointFollower::new(1.5, 3.0, 1.5, 1.0);
        let here = Coordinate::new(0.0, 0.0);
        let target = Coordinate::new(0.0, 0.01); // due east (bearing 90)
        let g = follower.guide(here, 270.0, target); // facing west: 180 deg error
        assert!(g.twist.vx.abs() < 1e-6); // cosine of 180 is negative -> no forward
        assert!(g.twist.omega.abs() > 0.0); // but it turns
    }

    #[test]
    fn it_reports_arrival_inside_the_radius() {
        let follower = WaypointFollower::new(1.5, 50.0, 1.5, 1.0);
        let here = Coordinate::new(0.0, 0.0);
        let target = Coordinate::new(0.0, 0.0001); // about 11 m east, inside 50 m
        let g = follower.guide(here, 90.0, target);
        assert!(g.arrived);
        assert_eq!(g.twist, Twist::zero());
    }

    #[test]
    fn the_angular_command_is_capped() {
        let follower = WaypointFollower::new(1.0, 1.0, 10.0, 0.5); // huge gain, small cap
        let here = Coordinate::new(0.0, 0.0);
        let target = Coordinate::new(0.0001, 0.0); // due north, 90 deg off
        let g = follower.guide(here, 90.0, target); // facing east
        assert!((g.twist.omega.abs() - 0.5).abs() < 1e-6); // clamped to the cap
    }
}
