//! Shared planar motion types: a body twist and a world pose.

use core::f32::consts::PI;

/// A planar body velocity: the command a wheeled robot is driven with.
///
/// The frame follows the robotics convention (ROS REP-103): x points forward, y points to
/// the robot's left, and a positive `omega` turns counter-clockwise (toward the left).
/// Nonholonomic drives (differential, Ackermann, skid-steer) cannot move sideways and ignore
/// `vy`; holonomic drives (mecanum, omni) use it.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Twist;
///
/// let forward = Twist::planar(1.0, 0.0); // 1 m/s ahead, no turn
/// assert_eq!(forward.vy, 0.0);
/// assert_eq!(Twist::zero(), Twist::new(0.0, 0.0, 0.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Twist {
    /// Forward speed along the x axis.
    pub vx: f32,
    /// Leftward speed along the y axis; zero for drives that cannot strafe.
    pub vy: f32,
    /// Yaw rate about the z axis, positive counter-clockwise.
    pub omega: f32,
}

impl Twist {
    /// Creates a twist from its three components.
    ///
    /// # Arguments
    ///
    /// * `vx` - forward speed.
    /// * `vy` - leftward speed.
    /// * `omega` - yaw rate, positive counter-clockwise.
    ///
    /// # Returns
    ///
    /// The twist.
    pub fn new(vx: f32, vy: f32, omega: f32) -> Self {
        Self { vx, vy, omega }
    }

    /// Creates a planar twist with no sideways motion (`vy = 0`).
    ///
    /// # Arguments
    ///
    /// * `vx` - forward speed.
    /// * `omega` - yaw rate, positive counter-clockwise.
    ///
    /// # Returns
    ///
    /// The twist with `vy` zero.
    pub fn planar(vx: f32, omega: f32) -> Self {
        Self { vx, vy: 0.0, omega }
    }

    /// Returns the zero twist: stopped.
    ///
    /// # Returns
    ///
    /// A twist whose every component is zero.
    pub fn zero() -> Self {
        Self {
            vx: 0.0,
            vy: 0.0,
            omega: 0.0,
        }
    }
}

/// A planar pose in the world frame: position and heading.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Pose;
///
/// let start = Pose::origin();
/// assert_eq!((start.x, start.y, start.theta), (0.0, 0.0, 0.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// Position along the world x axis, in meters.
    pub x: f32,
    /// Position along the world y axis, in meters.
    pub y: f32,
    /// Heading from the world x axis, in radians, in `(-pi, pi]`, positive counter-clockwise.
    pub theta: f32,
}

impl Pose {
    /// Creates a pose; the heading is wrapped into `(-pi, pi]`.
    ///
    /// # Arguments
    ///
    /// * `x` - position along the world x axis.
    /// * `y` - position along the world y axis.
    /// * `theta` - heading in radians, wrapped to `(-pi, pi]`.
    ///
    /// # Returns
    ///
    /// The pose.
    pub fn new(x: f32, y: f32, theta: f32) -> Self {
        Self {
            x,
            y,
            theta: wrap_pi(theta),
        }
    }

    /// Returns the origin pose: at `(0, 0)` facing along the x axis.
    ///
    /// # Returns
    ///
    /// The origin pose.
    pub fn origin() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            theta: 0.0,
        }
    }
}

// Wraps an angle in radians into the half-open interval `(-pi, pi]`.
pub(crate) fn wrap_pi(angle: f32) -> f32 {
    let two_pi = 2.0 * PI;
    let mut a = angle % two_pi;
    if a > PI {
        a -= two_pi;
    } else if a <= -PI {
        a += two_pi;
    }
    a
}

// `f32::abs` lives in `std`, so this `no_std` crate takes the magnitude by hand.
pub(crate) fn magnitude(value: f32) -> f32 {
    if value < 0.0 {
        -value
    } else {
        value
    }
}

// `f32::clamp` lives in `std`; clamp by hand. Callers pass `low <= high`.
pub(crate) fn clamp(value: f32, low: f32, high: f32) -> f32 {
    if value < low {
        low
    } else if value > high {
        high
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_pi_folds_into_the_canonical_interval() {
        // Interior values pass through; a value past 2pi folds to its coterminal angle.
        assert!(wrap_pi(0.0).abs() < 1e-6);
        assert!((wrap_pi(1.0) - 1.0).abs() < 1e-6);
        assert!((wrap_pi(2.0 * PI + 0.3) - 0.3).abs() < 1e-5);
        // Whatever the input, the result lands within (-pi, pi].
        for k in -10..=10 {
            let w = wrap_pi(k as f32 * 1.3);
            assert!(w > -PI - 1e-4 && w <= PI + 1e-4);
        }
    }

    #[test]
    fn clamp_and_magnitude_behave() {
        assert_eq!(clamp(5.0, 0.0, 1.0), 1.0);
        assert_eq!(clamp(-5.0, 0.0, 1.0), 0.0);
        assert_eq!(magnitude(-3.0), 3.0);
    }
}
