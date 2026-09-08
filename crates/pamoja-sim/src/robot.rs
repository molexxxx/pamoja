//! A hardware-free mobile robot you can drive and watch move.

use pamoja_core::{Actuator, Result, Sensor};
use pamoja_kit::{Odometry, Pose, Twist};

/// A simulated differential-drive robot: drive it with a [`Twist`], read back its [`Pose`].
///
/// This stands in for a real rover in a hardware-free test or demo. It is both an
/// [`Actuator`] whose command is a body twist and a [`Sensor`] whose reading is the pose:
/// each command advances the robot one time step at the commanded velocity, integrating the
/// motion with the same exact-arc odometry a real robot would use, and a read returns where it
/// has reached. A control loop can therefore be developed and tested end to end with no robot.
///
/// # Examples
///
/// ```
/// use pamoja_core::{Actuator, Sensor};
/// use pamoja_kit::Twist;
/// use pamoja_sim::SimRobot;
///
/// # async fn demo() -> pamoja_core::Result<()> {
/// let mut robot = SimRobot::new(0.1); // 0.1 s per command
/// // Drive straight at 1 m/s for ten steps: about one meter forward.
/// for _ in 0..10 {
///     robot.apply(Twist::planar(1.0, 0.0)).await?;
/// }
/// let pose = robot.read().await?;
/// assert!((pose.x - 1.0).abs() < 1e-5 && pose.y.abs() < 1e-5);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SimRobot {
    odometry: Odometry,
    dt: f32,
}

impl SimRobot {
    /// Creates a robot at the origin that advances `dt` seconds per command.
    ///
    /// # Arguments
    ///
    /// * `dt` - the time each [`apply`](SimRobot::apply) advances the robot; its magnitude is used.
    ///
    /// # Returns
    ///
    /// The simulated robot.
    pub fn new(dt: f32) -> Self {
        Self {
            odometry: Odometry::at_origin(),
            dt: dt.abs(),
        }
    }

    /// Creates a robot starting from a known pose.
    ///
    /// # Arguments
    ///
    /// * `pose` - the starting pose.
    /// * `dt` - the time each command advances the robot; its magnitude is used.
    ///
    /// # Returns
    ///
    /// The simulated robot.
    pub fn starting_at(pose: Pose, dt: f32) -> Self {
        Self {
            odometry: Odometry::new(pose),
            dt: dt.abs(),
        }
    }

    /// Returns the robot's current pose.
    ///
    /// # Returns
    ///
    /// The pose reached so far.
    pub fn pose(&self) -> Pose {
        self.odometry.pose()
    }
}

impl Actuator for SimRobot {
    type Command = Twist;

    async fn apply(&mut self, command: Twist) -> Result<()> {
        self.odometry.integrate(command.vx, command.omega, self.dt);
        Ok(())
    }
}

impl Sensor for SimRobot {
    type Reading = Pose;

    async fn read(&mut self) -> Result<Pose> {
        Ok(self.odometry.pose())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::FRAC_PI_2;

    #[tokio::test]
    async fn driving_straight_advances_along_x() {
        let mut robot = SimRobot::new(0.1);
        for _ in 0..10 {
            robot.apply(Twist::planar(1.0, 0.0)).await.unwrap();
        }
        let pose = robot.read().await.unwrap();
        assert!((pose.x - 1.0).abs() < 1e-5);
        assert!(pose.y.abs() < 1e-5);
    }

    #[tokio::test]
    async fn turning_in_place_changes_only_the_heading() {
        let mut robot = SimRobot::new(0.5);
        // Spin at 1 rad/s for two half-second steps: about one radian, no translation.
        robot.apply(Twist::planar(0.0, 1.0)).await.unwrap();
        robot.apply(Twist::planar(0.0, 1.0)).await.unwrap();
        let pose = robot.read().await.unwrap();
        assert!(pose.x.abs() < 1e-6 && pose.y.abs() < 1e-6);
        assert!((pose.theta - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn a_quarter_circle_lands_at_the_arc_corner() {
        let mut robot = SimRobot::new(FRAC_PI_2);
        // One step of forward 1 m/s and 1 rad/s over pi/2 s traces a quarter circle of radius 1.
        robot.apply(Twist::planar(1.0, 1.0)).await.unwrap();
        let pose = robot.read().await.unwrap();
        assert!((pose.x - 1.0).abs() < 1e-5 && (pose.y - 1.0).abs() < 1e-5);
    }
}
