//! Offboard control setpoints: the `type_mask` builder and setpoint constructors.
//!
//! An offboard setpoint carries position, velocity, and acceleration fields all at once, and a
//! `type_mask` says which of them the vehicle should act on and which to ignore. Getting the
//! mask wrong is the classic offboard bug: a velocity setpoint with the position bits left
//! active makes the vehicle chase a zero position. [`TypeMask`] builds the mask from the named
//! [`POSITION_TARGET_TYPEMASK`](crate::dialect::position_target_typemask) bits, starting from
//! "ignore everything" and enabling only the dimensions a setpoint sets, and the constructors
//! on [`SetPositionTargetLocalNed`] and [`SetPositionTargetGlobalInt`] use it to produce
//! ready-to-send position and velocity setpoints.

use crate::dialect::position_target_typemask as bits;
use crate::dialect::{SetPositionTargetGlobalInt, SetPositionTargetLocalNed};

/// Builds a `type_mask` for a `SET_POSITION_TARGET_*` message.
///
/// A set bit tells the vehicle to ignore that dimension, so the builder starts from
/// [`ignore_all`](TypeMask::ignore_all) and each `use_*` method clears the ignore bits for the
/// dimensions the setpoint provides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeMask(u16);

impl TypeMask {
    // Every ignorable dimension, i.e. every bit except FORCE_SET, which is a mode selector.
    const ALL_IGNORE: u16 = bits::X_IGNORE
        | bits::Y_IGNORE
        | bits::Z_IGNORE
        | bits::VX_IGNORE
        | bits::VY_IGNORE
        | bits::VZ_IGNORE
        | bits::AX_IGNORE
        | bits::AY_IGNORE
        | bits::AZ_IGNORE
        | bits::YAW_IGNORE
        | bits::YAW_RATE_IGNORE;

    /// Starts from a mask that ignores every dimension.
    ///
    /// # Returns
    ///
    /// A mask with every ignore bit set.
    pub fn ignore_all() -> Self {
        TypeMask(Self::ALL_IGNORE)
    }

    /// Enables the position fields (`x`, `y`, `z`).
    ///
    /// # Returns
    ///
    /// The mask, for chaining.
    pub fn use_position(mut self) -> Self {
        self.0 &= !(bits::X_IGNORE | bits::Y_IGNORE | bits::Z_IGNORE);
        self
    }

    /// Enables the velocity fields (`vx`, `vy`, `vz`).
    ///
    /// # Returns
    ///
    /// The mask, for chaining.
    pub fn use_velocity(mut self) -> Self {
        self.0 &= !(bits::VX_IGNORE | bits::VY_IGNORE | bits::VZ_IGNORE);
        self
    }

    /// Enables the acceleration fields (`afx`, `afy`, `afz`).
    ///
    /// # Returns
    ///
    /// The mask, for chaining.
    pub fn use_acceleration(mut self) -> Self {
        self.0 &= !(bits::AX_IGNORE | bits::AY_IGNORE | bits::AZ_IGNORE);
        self
    }

    /// Enables the `yaw` field.
    ///
    /// # Returns
    ///
    /// The mask, for chaining.
    pub fn use_yaw(mut self) -> Self {
        self.0 &= !bits::YAW_IGNORE;
        self
    }

    /// Enables the `yaw_rate` field.
    ///
    /// # Returns
    ///
    /// The mask, for chaining.
    pub fn use_yaw_rate(mut self) -> Self {
        self.0 &= !bits::YAW_RATE_IGNORE;
        self
    }

    /// Sets the force flag, so the acceleration fields are interpreted as a force.
    ///
    /// # Returns
    ///
    /// The mask, for chaining.
    pub fn force(mut self) -> Self {
        self.0 |= bits::FORCE_SET;
        self
    }

    /// Returns the assembled mask value.
    ///
    /// # Returns
    ///
    /// The `type_mask` bits.
    pub fn bits(self) -> u16 {
        self.0
    }
}

impl SetPositionTargetLocalNed {
    /// Builds a local-frame position setpoint, ignoring velocity, acceleration, and yaw.
    ///
    /// # Arguments
    ///
    /// * `time_boot_ms` - the sender's boot timestamp, in milliseconds.
    /// * `coordinate_frame` - the [`MAV_FRAME`](crate::dialect::mav_frame) of the setpoint.
    /// * `target_system` - the target system id.
    /// * `target_component` - the target component id.
    /// * `x`, `y`, `z` - the position, in metres in the chosen frame.
    ///
    /// # Returns
    ///
    /// The setpoint, with only the position fields active in its `type_mask`.
    pub fn position(
        time_boot_ms: u32,
        coordinate_frame: u8,
        target_system: u8,
        target_component: u8,
        x: f32,
        y: f32,
        z: f32,
    ) -> Self {
        SetPositionTargetLocalNed {
            time_boot_ms,
            x,
            y,
            z,
            type_mask: TypeMask::ignore_all().use_position().bits(),
            target_system,
            target_component,
            coordinate_frame,
            ..Self::zeroed()
        }
    }

    /// Builds a local-frame velocity setpoint, ignoring position, acceleration, and yaw.
    ///
    /// # Arguments
    ///
    /// * `time_boot_ms` - the sender's boot timestamp, in milliseconds.
    /// * `coordinate_frame` - the [`MAV_FRAME`](crate::dialect::mav_frame) of the setpoint.
    /// * `target_system` - the target system id.
    /// * `target_component` - the target component id.
    /// * `vx`, `vy`, `vz` - the velocity, in metres per second in the chosen frame.
    ///
    /// # Returns
    ///
    /// The setpoint, with only the velocity fields active in its `type_mask`.
    pub fn velocity(
        time_boot_ms: u32,
        coordinate_frame: u8,
        target_system: u8,
        target_component: u8,
        vx: f32,
        vy: f32,
        vz: f32,
    ) -> Self {
        SetPositionTargetLocalNed {
            time_boot_ms,
            vx,
            vy,
            vz,
            type_mask: TypeMask::ignore_all().use_velocity().bits(),
            target_system,
            target_component,
            coordinate_frame,
            ..Self::zeroed()
        }
    }
}

impl SetPositionTargetGlobalInt {
    /// Builds a global-frame position setpoint, ignoring velocity, acceleration, and yaw.
    ///
    /// # Arguments
    ///
    /// * `time_boot_ms` - the sender's boot timestamp, in milliseconds.
    /// * `coordinate_frame` - the [`MAV_FRAME`](crate::dialect::mav_frame) of the setpoint.
    /// * `target_system` - the target system id.
    /// * `target_component` - the target component id.
    /// * `lat_int`, `lon_int` - latitude and longitude, in degrees times 1e7.
    /// * `alt` - the altitude, in metres in the chosen frame.
    ///
    /// # Returns
    ///
    /// The setpoint, with only the position fields active in its `type_mask`.
    pub fn position(
        time_boot_ms: u32,
        coordinate_frame: u8,
        target_system: u8,
        target_component: u8,
        lat_int: i32,
        lon_int: i32,
        alt: f32,
    ) -> Self {
        SetPositionTargetGlobalInt {
            time_boot_ms,
            lat_int,
            lon_int,
            alt,
            type_mask: TypeMask::ignore_all().use_position().bits(),
            target_system,
            target_component,
            coordinate_frame,
            ..Self::zeroed()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::mav_frame;

    #[test]
    fn ignore_all_sets_every_dimension_but_the_force_flag() {
        let mask = TypeMask::ignore_all().bits();
        // Every ignore bit is set.
        assert_eq!(mask & bits::X_IGNORE, bits::X_IGNORE);
        assert_eq!(mask & bits::VZ_IGNORE, bits::VZ_IGNORE);
        assert_eq!(mask & bits::YAW_RATE_IGNORE, bits::YAW_RATE_IGNORE);
        // FORCE_SET is a mode selector, not an ignore bit, so it stays clear.
        assert_eq!(mask & bits::FORCE_SET, 0);
    }

    #[test]
    fn a_position_mask_enables_only_the_position_bits() {
        let mask = TypeMask::ignore_all().use_position().bits();
        // Position is acted on: its ignore bits are clear.
        assert_eq!(mask & (bits::X_IGNORE | bits::Y_IGNORE | bits::Z_IGNORE), 0);
        // Everything else is still ignored.
        assert_eq!(mask & bits::VX_IGNORE, bits::VX_IGNORE);
        assert_eq!(mask & bits::AX_IGNORE, bits::AX_IGNORE);
        assert_eq!(mask & bits::YAW_IGNORE, bits::YAW_IGNORE);
        // The exact value, so a regression in the bit layout is caught.
        assert_eq!(mask, TypeMask::ALL_IGNORE & !(1 | 2 | 4));
    }

    #[test]
    fn a_velocity_setpoint_ignores_position() {
        let setpoint =
            SetPositionTargetLocalNed::velocity(1000, mav_frame::LOCAL_NED, 1, 1, 0.5, 0.0, -0.2);
        assert_eq!(setpoint.vx, 0.5);
        assert_eq!(setpoint.vz, -0.2);
        // Position is ignored, velocity is not.
        assert_eq!(setpoint.type_mask & bits::X_IGNORE, bits::X_IGNORE);
        assert_eq!(setpoint.type_mask & bits::VX_IGNORE, 0);
    }

    #[test]
    fn a_global_position_setpoint_carries_scaled_coordinates() {
        let setpoint = SetPositionTargetGlobalInt::position(
            2000,
            mav_frame::GLOBAL_RELATIVE_ALT_INT,
            1,
            1,
            473_977_418,
            85_455_939,
            10.0,
        );
        assert_eq!(setpoint.lat_int, 473_977_418);
        assert_eq!(setpoint.alt, 10.0);
        assert_eq!(
            setpoint.type_mask & (bits::X_IGNORE | bits::Y_IGNORE | bits::Z_IGNORE),
            0
        );
    }
}
