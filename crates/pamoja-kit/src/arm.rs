//! Serial-arm (manipulator) kinematics: where the hand is, and how to place it.
//!
//! A robot arm is a chain of joints, and two questions recur: given the joint angles, where is the
//! tool (forward kinematics), and given a target, what joint angles put the tool there (inverse
//! kinematics). [`forward_kinematics`] answers the first for any serial arm described in the
//! standard Denavit-Hartenberg convention, in full 3D. The second has no closed form for a general
//! arm, so this provides the classic solvable case, the planar [`TwoLinkArm`], with both its
//! elbow-up and elbow-down solutions; numeric inverse kinematics for longer chains can build on the
//! same forward model later.

use crate::motion::{clamp, magnitude};
use libm::{acosf, atan2f, cosf, sinf, sqrtf};

/// A 4x4 homogeneous transform: a rotation and a translation in one matrix.
///
/// Stored row-major, this is the building block of forward kinematics: each joint contributes one
/// transform, and chaining them places the tool relative to the base.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// The sixteen elements in row-major order (row 0 first).
    pub m: [f32; 16],
}

impl Transform {
    /// Returns the identity transform: no rotation, no translation.
    ///
    /// # Returns
    ///
    /// The identity.
    pub fn identity() -> Self {
        let mut m = [0.0; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m[15] = 1.0;
        Self { m }
    }

    /// Returns the product `self * other`, the transform that applies `other` then `self`.
    ///
    /// # Arguments
    ///
    /// * `other` - the transform applied first (the one further down the chain).
    ///
    /// # Returns
    ///
    /// The composed transform.
    pub fn multiply(&self, other: &Transform) -> Transform {
        let mut m = [0.0f32; 16];
        for row in 0..4 {
            for col in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.m[row * 4 + k] * other.m[k * 4 + col];
                }
                m[row * 4 + col] = sum;
            }
        }
        Transform { m }
    }

    /// Returns the translation part: the position this transform places the origin at.
    ///
    /// # Returns
    ///
    /// `(x, y, z)`, the last column of the matrix.
    pub fn position(&self) -> (f32, f32, f32) {
        (self.m[3], self.m[7], self.m[11])
    }
}

/// The four Denavit-Hartenberg parameters describing one joint-to-joint step of a serial arm.
///
/// The DH convention pins each link with four numbers, so an arm is just a list of these. For a
/// revolute joint the joint variable is `theta`; for a prismatic joint it is `d`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DhParameters {
    /// Link length: distance along the common normal, in meters.
    pub a: f32,
    /// Link twist: angle about the common normal, in radians.
    pub alpha: f32,
    /// Link offset: distance along the previous z axis, in meters.
    pub d: f32,
    /// Joint angle: rotation about the previous z axis, in radians.
    pub theta: f32,
}

impl DhParameters {
    /// Returns the homogeneous [`Transform`] for this DH step.
    ///
    /// # Returns
    ///
    /// The standard DH transform built from `(a, alpha, d, theta)`.
    pub fn transform(&self) -> Transform {
        let (ct, st) = (cosf(self.theta), sinf(self.theta));
        let (ca, sa) = (cosf(self.alpha), sinf(self.alpha));
        Transform {
            m: [
                ct,
                -st * ca,
                st * sa,
                self.a * ct,
                st,
                ct * ca,
                -ct * sa,
                self.a * st,
                0.0,
                sa,
                ca,
                self.d,
                0.0,
                0.0,
                0.0,
                1.0,
            ],
        }
    }
}

/// Returns the transform from the base to the tool for a serial arm of DH joints.
///
/// # Arguments
///
/// * `joints` - the arm's joints, base first, each as [`DhParameters`].
///
/// # Returns
///
/// The composed base-to-tool [`Transform`]; the identity for an empty arm. Take
/// [`Transform::position`] for the tool point.
///
/// # Examples
///
/// ```
/// use pamoja_kit::{forward_kinematics, DhParameters};
///
/// // A two-link planar arm written in DH form: links of 1.0, both joints at 0, flat along x.
/// let arm = [
///     DhParameters { a: 1.0, alpha: 0.0, d: 0.0, theta: 0.0 },
///     DhParameters { a: 1.0, alpha: 0.0, d: 0.0, theta: 0.0 },
/// ];
/// let (x, y, _z) = forward_kinematics(&arm).position();
/// assert!((x - 2.0).abs() < 1e-5 && y.abs() < 1e-5); // reaches straight out to x = 2
/// ```
pub fn forward_kinematics(joints: &[DhParameters]) -> Transform {
    let mut transform = Transform::identity();
    for joint in joints {
        transform = transform.multiply(&joint.transform());
    }
    transform
}

/// Which way a two-link arm's elbow bends; both reach the same point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Elbow {
    /// The elbow bends so the second joint angle is positive (counter-clockwise).
    Up,
    /// The elbow bends so the second joint angle is negative (clockwise).
    Down,
}

/// A planar two-link arm: the textbook arm with a closed-form inverse.
///
/// Two links of fixed length in a plane, with a shoulder and an elbow joint. [`tip`](TwoLinkArm::tip)
/// is forward kinematics; [`joints_for`](TwoLinkArm::joints_for) is the analytic inverse, returning
/// the shoulder and elbow angles that place the hand at a target, for the chosen [`Elbow`] branch.
///
/// # Examples
///
/// ```
/// use pamoja_kit::{Elbow, TwoLinkArm};
///
/// let arm = TwoLinkArm::new(1.0, 1.0);
/// // Place the hand, then recover the joint angles for it.
/// let (x, y) = arm.tip(0.5, 0.7);
/// let (q1, q2) = arm.joints_for(x, y, Elbow::Up).unwrap();
/// assert!((q1 - 0.5).abs() < 1e-4 && (q2 - 0.7).abs() < 1e-4);
///
/// // A target beyond the arm's reach has no solution.
/// assert!(arm.joints_for(5.0, 0.0, Elbow::Up).is_none());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct TwoLinkArm {
    l1: f32,
    l2: f32,
}

impl TwoLinkArm {
    /// Creates an arm from its two link lengths.
    ///
    /// # Arguments
    ///
    /// * `l1` - the first (shoulder) link length; its magnitude is used.
    /// * `l2` - the second (elbow) link length; its magnitude is used.
    ///
    /// # Returns
    ///
    /// The arm.
    pub fn new(l1: f32, l2: f32) -> Self {
        Self {
            l1: magnitude(l1),
            l2: magnitude(l2),
        }
    }

    /// Returns the closest and farthest distances the hand can reach from the shoulder.
    ///
    /// # Returns
    ///
    /// `(min, max)`, where `min` is `|l1 - l2|` and `max` is `l1 + l2`.
    pub fn reach(&self) -> (f32, f32) {
        (magnitude(self.l1 - self.l2), self.l1 + self.l2)
    }

    /// Returns the hand position for given joint angles (forward kinematics).
    ///
    /// # Arguments
    ///
    /// * `shoulder` - the first joint angle, in radians from the x axis.
    /// * `elbow` - the second joint angle, in radians relative to the first link.
    ///
    /// # Returns
    ///
    /// The hand `(x, y)`.
    pub fn tip(&self, shoulder: f32, elbow: f32) -> (f32, f32) {
        let x = self.l1 * cosf(shoulder) + self.l2 * cosf(shoulder + elbow);
        let y = self.l1 * sinf(shoulder) + self.l2 * sinf(shoulder + elbow);
        (x, y)
    }

    /// Returns the joint angles that place the hand at a target (inverse kinematics).
    ///
    /// # Arguments
    ///
    /// * `x` - the target x coordinate.
    /// * `y` - the target y coordinate.
    /// * `elbow` - which [`Elbow`] branch to solve for.
    ///
    /// # Returns
    ///
    /// `Some((shoulder, elbow))` for a reachable target, or `None` if the target lies outside the
    /// arm's reach.
    pub fn joints_for(&self, x: f32, y: f32, elbow: Elbow) -> Option<(f32, f32)> {
        let distance_squared = x * x + y * y;
        let distance = sqrtf(distance_squared);
        let (min, max) = self.reach();
        // A tiny tolerance keeps a target exactly on the boundary solvable despite rounding.
        let tolerance = 1e-4;
        if distance > max + tolerance || distance < min - tolerance {
            return None;
        }

        let denominator = 2.0 * self.l1 * self.l2;
        if denominator == 0.0 {
            return None;
        }
        // Clamp guards the boundary case where rounding pushes the cosine just past +/-1.
        let cos_elbow = clamp(
            (distance_squared - self.l1 * self.l1 - self.l2 * self.l2) / denominator,
            -1.0,
            1.0,
        );
        let elbow_magnitude = acosf(cos_elbow);
        let elbow_angle = match elbow {
            Elbow::Up => elbow_magnitude,
            Elbow::Down => -elbow_magnitude,
        };
        let shoulder = atan2f(y, x)
            - atan2f(
                self.l2 * sinf(elbow_angle),
                self.l1 + self.l2 * cosf(elbow_angle),
            );
        Some((shoulder, elbow_angle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::FRAC_PI_2;

    #[test]
    fn dh_forward_kinematics_matches_the_planar_arm() {
        // The same two-link arm, once in DH form and once by the planar formula, must agree.
        let (q1, q2) = (0.6_f32, -0.4_f32);
        let dh = [
            DhParameters {
                a: 1.5,
                alpha: 0.0,
                d: 0.0,
                theta: q1,
            },
            DhParameters {
                a: 1.0,
                alpha: 0.0,
                d: 0.0,
                theta: q2,
            },
        ];
        let (x, y, z) = forward_kinematics(&dh).position();
        let arm = TwoLinkArm::new(1.5, 1.0);
        let (px, py) = arm.tip(q1, q2);
        assert!((x - px).abs() < 1e-5);
        assert!((y - py).abs() < 1e-5);
        assert!(z.abs() < 1e-5);
    }

    #[test]
    fn identity_is_the_multiplicative_unit() {
        let t = DhParameters {
            a: 0.7,
            alpha: 0.3,
            d: 0.2,
            theta: 1.1,
        }
        .transform();
        let i = Transform::identity();
        assert_eq!(i.multiply(&t), t);
        assert_eq!(t.multiply(&i), t);
    }

    #[test]
    fn a_z_offset_lifts_the_tool_out_of_the_plane() {
        // A pure link offset along the base z axis puts the tool one unit straight up.
        let dh = [DhParameters {
            a: 0.0,
            alpha: 0.0,
            d: 1.0,
            theta: 0.0,
        }];
        let (x, y, z) = forward_kinematics(&dh).position();
        assert!(x.abs() < 1e-5 && y.abs() < 1e-5 && (z - 1.0).abs() < 1e-5);
    }

    #[test]
    fn a_twist_then_offset_swings_into_the_y_axis() {
        // A 90-degree twist about x turns a z offset into a -y displacement, still in the xy plane.
        let dh = [
            DhParameters {
                a: 0.0,
                alpha: FRAC_PI_2,
                d: 0.0,
                theta: 0.0,
            },
            DhParameters {
                a: 0.0,
                alpha: 0.0,
                d: 1.0,
                theta: 0.0,
            },
        ];
        let (x, y, z) = forward_kinematics(&dh).position();
        assert!(x.abs() < 1e-5 && (y + 1.0).abs() < 1e-5 && z.abs() < 1e-5);
    }

    #[test]
    fn two_link_inverse_round_trips_both_elbows() {
        let arm = TwoLinkArm::new(1.0, 1.2);
        for &(q1, q2) in &[(0.5, 0.7), (0.2, -0.9), (-0.6, 1.1)] {
            let (x, y) = arm.tip(q1, q2);
            let elbow = if q2 >= 0.0 { Elbow::Up } else { Elbow::Down };
            let (s, e) = arm.joints_for(x, y, elbow).unwrap();
            let (rx, ry) = arm.tip(s, e);
            // The recovered angles must reproduce the same hand position.
            assert!((rx - x).abs() < 1e-4 && (ry - y).abs() < 1e-4);
        }
    }

    #[test]
    fn unreachable_targets_have_no_solution() {
        let arm = TwoLinkArm::new(1.0, 1.0);
        assert!(arm.joints_for(5.0, 0.0, Elbow::Up).is_none()); // too far
        assert!(arm.joints_for(0.0, 0.0, Elbow::Up).is_some()); // folded back: reachable (min = 0)
    }

    #[test]
    fn reach_is_the_link_sum_and_difference() {
        let arm = TwoLinkArm::new(2.0, 0.5);
        assert_eq!(arm.reach(), (1.5, 2.5));
    }
}
