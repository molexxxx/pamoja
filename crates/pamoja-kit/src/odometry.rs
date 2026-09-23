//! Dead-reckoning a robot's pose from its motion.

use crate::motion::{clamp, magnitude, wrap_pi, Pose};
use crate::DiffDrive;
use libm::{cosf, sinf};

/// Tracks a robot's [`Pose`] by accumulating its motion over time (odometry).
///
/// With no GPS indoors, a robot estimates where it is by adding up where it has been: each small
/// move is integrated onto the running pose. This uses the exact arc model rather than a straight-
/// line step, so a robot that drives and turns at once follows the curve it actually traces
/// instead of cutting the corner; over many steps that is markedly more accurate. Feed it either a
/// body motion (forward speed and yaw rate over a time step) or wheel-distance deltas through a
/// [`DiffDrive`] model. Dead reckoning drifts, so correct the heading from an absolute source with
/// [`fuse_heading`](Odometry::fuse_heading) when one is available.
///
/// # Examples
///
/// ```
/// use pamoja_kit::{Odometry, Pose};
///
/// // Drive a quarter circle of radius 1 m: forward 1 m/s, turning left at 1 rad/s, for pi/2 s.
/// let mut odom = Odometry::at_origin();
/// let pose = odom.integrate(1.0, 1.0, core::f32::consts::FRAC_PI_2);
///
/// // It ends about (1, 1) facing 90 degrees, the far corner of the arc.
/// assert!((pose.x - 1.0).abs() < 1e-5);
/// assert!((pose.y - 1.0).abs() < 1e-5);
/// assert!((pose.theta - core::f32::consts::FRAC_PI_2).abs() < 1e-5);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Odometry {
    pose: Pose,
}

impl Odometry {
    /// Creates an estimator starting from a known pose.
    ///
    /// # Arguments
    ///
    /// * `start` - the initial pose.
    ///
    /// # Returns
    ///
    /// The estimator.
    pub fn new(start: Pose) -> Self {
        Self { pose: start }
    }

    /// Creates an estimator starting at the origin facing along the x axis.
    ///
    /// # Returns
    ///
    /// The estimator.
    pub fn at_origin() -> Self {
        Self {
            pose: Pose::origin(),
        }
    }

    /// Returns the current pose estimate.
    ///
    /// # Returns
    ///
    /// The pose accumulated so far.
    pub fn pose(&self) -> Pose {
        self.pose
    }

    /// Resets the estimate to a known pose.
    ///
    /// # Arguments
    ///
    /// * `pose` - the pose to set.
    pub fn reset(&mut self, pose: Pose) {
        self.pose = pose;
    }

    /// Integrates a body motion over a time step and returns the new pose.
    ///
    /// A speed, rate, or time step that is not a finite number is ignored and the pose
    /// returned as it stood, so one bad sample cannot poison the pose for good.
    ///
    /// # Arguments
    ///
    /// * `linear` - the forward speed.
    /// * `angular` - the yaw rate, positive turning left.
    /// * `dt` - the length of the time step.
    ///
    /// # Returns
    ///
    /// The updated pose.
    pub fn integrate(&mut self, linear: f32, angular: f32, dt: f32) -> Pose {
        self.advance(linear * dt, angular * dt);
        self.pose
    }

    /// Integrates wheel-distance deltas through a differential-drive model.
    ///
    /// A distance that is not a finite number is ignored and the pose returned as it stood.
    ///
    /// # Arguments
    ///
    /// * `left` - the distance the left wheel rolled since the last update.
    /// * `right` - the distance the right wheel rolled since the last update.
    /// * `drive` - the [`DiffDrive`] model giving the track between the wheels.
    ///
    /// # Returns
    ///
    /// The updated pose. The wheel deltas are turned into a forward distance and a heading
    /// change by [`DiffDrive::body_motion`], then integrated as one arc.
    pub fn integrate_wheels(&mut self, left: f32, right: f32, drive: &DiffDrive) -> Pose {
        let (distance, heading_change) = drive.body_motion(left, right);
        self.advance(distance, heading_change);
        self.pose
    }

    /// Corrects the heading toward an absolute measurement, the way a compass tames gyro drift.
    ///
    /// This is the angular cousin of [`Complementary`](crate::Complementary): it nudges the
    /// estimated heading along the shortest arc toward an absolute reading (an IMU yaw, a
    /// magnetometer, a GPS course) by a blend weight, leaving the position untouched.
    ///
    /// # Arguments
    ///
    /// * `measured` - the absolute heading in the pose's own frame: radians counter-clockwise
    ///   from the world x axis. A compass course, clockwise from north, is not in that frame;
    ///   with the world x axis pointing east it converts as `FRAC_PI_2 - course`.
    /// * `weight` - how strongly to trust the measurement, clamped to `[0, 1]`; zero keeps the
    ///   dead-reckoned heading, one snaps to `measured`.
    ///
    /// A measurement that is not a finite number, or a weight that is not a number, leaves the
    /// heading as it was.
    pub fn fuse_heading(&mut self, measured: f32, weight: f32) {
        if !measured.is_finite() || weight.is_nan() {
            return;
        }
        let w = clamp(weight, 0.0, 1.0);
        let error = wrap_pi(measured - self.pose.theta);
        self.pose.theta = wrap_pi(self.pose.theta + w * error);
    }

    // Advances the pose by an arc of forward distance `distance` and heading change
    // `heading_change`, using the exact integration for a constant-curvature segment.
    fn advance(&mut self, distance: f32, heading_change: f32) {
        if !distance.is_finite() || !heading_change.is_finite() {
            return;
        }
        let theta = self.pose.theta;
        if magnitude(heading_change) < 1e-6 {
            self.pose.x += distance * cosf(theta);
            self.pose.y += distance * sinf(theta);
            self.pose.theta = wrap_pi(theta + heading_change);
        } else {
            let radius = distance / heading_change;
            let new_theta = theta + heading_change;
            self.pose.x += radius * (sinf(new_theta) - sinf(theta));
            self.pose.y += radius * (cosf(theta) - cosf(new_theta));
            self.pose.theta = wrap_pi(new_theta);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::{FRAC_PI_2, PI};

    #[test]
    fn driving_straight_moves_along_the_heading() {
        let mut odom = Odometry::new(Pose::new(0.0, 0.0, FRAC_PI_2)); // facing +y
        let pose = odom.integrate(2.0, 0.0, 1.0);
        assert!(pose.x.abs() < 1e-5);
        assert!((pose.y - 2.0).abs() < 1e-5);
        assert!((pose.theta - FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn a_quarter_circle_lands_at_the_arc_corner() {
        let mut odom = Odometry::at_origin();
        let pose = odom.integrate(1.0, 1.0, FRAC_PI_2);
        assert!((pose.x - 1.0).abs() < 1e-5);
        assert!((pose.y - 1.0).abs() < 1e-5);
        assert!((pose.theta - FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn wheel_deltas_match_a_spin_in_place() {
        let drive = DiffDrive::new(0.5);
        let mut odom = Odometry::at_origin();
        // Equal and opposite wheel deltas: turn in place, no translation.
        let pose = odom.integrate_wheels(-0.25, 0.25, &drive);
        assert!(pose.x.abs() < 1e-6 && pose.y.abs() < 1e-6);
        assert!((pose.theta - 1.0).abs() < 1e-6); // (0.25 - -0.25) / 0.5 = 1 rad
    }

    #[test]
    fn fuse_heading_blends_along_the_shortest_arc() {
        let mut odom = Odometry::new(Pose::new(0.0, 0.0, 0.1));
        odom.fuse_heading(0.5, 0.5); // halfway from 0.1 toward 0.5
        assert!((odom.pose().theta - 0.3).abs() < 1e-6);
    }

    #[test]
    fn fuse_heading_takes_the_short_way_across_pi() {
        let mut odom = Odometry::new(Pose::new(0.0, 0.0, 3.0));
        // Measured just past pi: the shortest arc wraps through pi, not the long way back.
        odom.fuse_heading(-3.0, 1.0);
        assert!((odom.pose().theta - -3.0).abs() < 1e-6);
        assert!(odom.pose().theta.abs() <= PI);
    }

    #[test]
    fn a_sample_that_is_not_a_number_leaves_the_pose_as_it_was() {
        let mut odometry = Odometry::at_origin();
        let moved = odometry.integrate(1.0, 0.0, 1.0);
        assert_eq!(odometry.integrate(f32::NAN, 0.0, 1.0), moved);
        assert_eq!(odometry.integrate(1.0, 0.0, f32::INFINITY), moved);
        assert_eq!(
            odometry.integrate_wheels(f32::NAN, 1.0, &DiffDrive::new(0.5)),
            moved
        );
        odometry.fuse_heading(f32::NAN, 1.0);
        odometry.fuse_heading(1.0, f32::NAN);
        assert_eq!(odometry.pose(), moved);
    }
}
