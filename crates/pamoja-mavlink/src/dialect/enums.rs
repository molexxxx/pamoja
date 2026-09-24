//! The common-dialect enumerations, as named constants and as a table of names.
//!
//! MAVLink enum fields ride on the wire as plain integers, so the typed messages store them
//! as integers. Each module below gives an enumeration's values their Rust names, and
//! [`ENUMS`] carries the same values under the names the dialect writes, such as
//! `MAV_TYPE_QUADROTOR`, so a program looks a value up by that name, or names a value it
//! received, the same way in every language.
//!
//! Every enumeration a field of a typed message names is here in full, as the MAVLink
//! `minimal.xml` and `common.xml` define it at commit
//! `20ed9b2760cffec2ac9becb9b8287cb2456d7ee7` of `mavlink/mavlink`. A constant's name is the
//! dialect's with the enumeration's own prefix removed, and a name that would start with a
//! digit moves the digits to the end, so `MAV_SYS_STATUS_SENSOR_3D_GYRO` is
//! `mav_sys_status_sensor::GYRO_3D`. A field may carry a value no entry names; it is still
//! valid on the wire.

#![allow(missing_docs)]

/// One named value of a dialect enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnumEntry {
    /// The name the dialect gives the value, such as `"MAV_STATE_STANDBY"`.
    pub name: &'static str,
    /// The value on the wire.
    pub value: u64,
}

/// A dialect enumeration: its name, whether its values combine as bits, and every value it
/// names, in the order the dialect lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnumDescriptor {
    /// The enumeration's name, such as `"MAV_STATE"`.
    pub name: &'static str,
    /// Whether a field of this enumeration holds several of its values at once, one per bit.
    pub bitmask: bool,
    /// Every value the enumeration names.
    pub entries: &'static [EnumEntry],
}

impl EnumDescriptor {
    /// Returns the value an entry of this enumeration names.
    ///
    /// # Arguments
    ///
    /// * `entry` - the entry's name as the dialect writes it, such as `"MAV_STATE_STANDBY"`.
    ///
    /// # Returns
    ///
    /// `Some(value)`, or `None` if no entry of this enumeration has that name.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_mavlink::dialect::enum_named;
    ///
    /// let state = enum_named("MAV_STATE").expect("a dialect enumeration");
    /// assert_eq!(state.value("MAV_STATE_STANDBY"), Some(3));
    /// ```
    pub fn value(&self, entry: &str) -> Option<u64> {
        self.entries
            .iter()
            .find(|named| named.name == entry)
            .map(|named| named.value)
    }

    /// Returns the name of the entry that stands for a value.
    ///
    /// # Arguments
    ///
    /// * `value` - the value a field carried.
    ///
    /// # Returns
    ///
    /// `Some(name)` for the first entry, in dialect order, with exactly that value, or `None`
    /// if no entry names it. For a bitmask, see [`names`](Self::names).
    pub fn entry(&self, value: u64) -> Option<&'static str> {
        self.entries
            .iter()
            .find(|named| named.value == value)
            .map(|named| named.name)
    }

    /// Returns the names a value is made of.
    ///
    /// # Arguments
    ///
    /// * `value` - the value a field carried.
    ///
    /// # Returns
    ///
    /// For a bitmask, each entry whose bits are all set in `value`, in dialect order, or the
    /// entry for zero when `value` is zero. Otherwise the one entry that names `value`. Bits
    /// or values no entry names are left out.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_mavlink::dialect::{enum_named, mav_mode_flag};
    ///
    /// let flags = enum_named("MAV_MODE_FLAG").expect("a dialect enumeration");
    /// let base_mode = mav_mode_flag::SAFETY_ARMED | mav_mode_flag::CUSTOM_MODE_ENABLED;
    /// let names: Vec<&str> = flags.names(u64::from(base_mode)).collect();
    /// assert_eq!(names, ["MAV_MODE_FLAG_SAFETY_ARMED", "MAV_MODE_FLAG_CUSTOM_MODE_ENABLED"]);
    /// ```
    pub fn names(&self, value: u64) -> impl Iterator<Item = &'static str> + '_ {
        let bitmask = self.bitmask;
        let whole = if bitmask { None } else { self.entry(value) };
        whole.into_iter().chain(
            self.entries
                .iter()
                .filter(move |named| {
                    bitmask
                        && if value == 0 {
                            named.value == 0
                        } else {
                            named.value != 0 && value & named.value == named.value
                        }
                })
                .map(|named| named.name),
        )
    }
}

/// Returns a dialect enumeration by name.
///
/// # Arguments
///
/// * `name` - the enumeration's name, such as `"MAV_TYPE"`.
///
/// # Returns
///
/// The enumeration, or `None` if [`ENUMS`] holds none by that name.
pub fn enum_named(name: &str) -> Option<&'static EnumDescriptor> {
    ENUMS.iter().find(|described| described.name == name)
}

/// Returns the value a dialect entry name stands for, whichever enumeration it belongs to.
///
/// Entry names are unique across the dialect, so the name alone is enough.
///
/// # Arguments
///
/// * `entry` - the entry's name, such as `"MAV_CMD_COMPONENT_ARM_DISARM"`.
///
/// # Returns
///
/// `Some(value)`, or `None` if no entry in [`ENUMS`] has that name.
///
/// # Examples
///
/// ```
/// use pamoja_mavlink::dialect::{entry_value, mav_cmd};
///
/// assert_eq!(entry_value("MAV_CMD_COMPONENT_ARM_DISARM"), Some(u64::from(mav_cmd::COMPONENT_ARM_DISARM)));
/// assert_eq!(entry_value("MAV_CMD_ARM"), None);
/// ```
pub fn entry_value(entry: &str) -> Option<u64> {
    ENUMS.iter().find_map(|described| described.value(entry))
}

macro_rules! dialect_enums {
    ($(
        $(#[$doc:meta])*
        $module:ident $name:literal $ty:ident $($bitmask:ident)? {
            $($constant:ident = $value:literal $entry:literal,)*
        }
    )*) => {
        $(
            $(#[$doc])*
            pub mod $module {
                $(pub const $constant: $ty = $value;)*
            }
        )*

        /// Every enumeration a field of a typed message names, complete, in the order the
        /// dialect defines them.
        pub const ENUMS: &[EnumDescriptor] = &[$(
            EnumDescriptor {
                name: $name,
                bitmask: dialect_enums!(@bitmask $($bitmask)?),
                entries: &[$(EnumEntry { name: $entry, value: $value }),*],
            },
        )*];
    };
    (@bitmask bitmask) => {
        true
    };
    (@bitmask) => {
        false
    };
}

dialect_enums! {
    /// `MAV_AUTOPILOT`: which autopilot stack a [`Heartbeat`](super::Heartbeat) comes from.
    mav_autopilot "MAV_AUTOPILOT" u8 {
        GENERIC = 0 "MAV_AUTOPILOT_GENERIC",
        RESERVED = 1 "MAV_AUTOPILOT_RESERVED",
        SLUGS = 2 "MAV_AUTOPILOT_SLUGS",
        ARDUPILOTMEGA = 3 "MAV_AUTOPILOT_ARDUPILOTMEGA",
        OPENPILOT = 4 "MAV_AUTOPILOT_OPENPILOT",
        GENERIC_WAYPOINTS_ONLY = 5 "MAV_AUTOPILOT_GENERIC_WAYPOINTS_ONLY",
        GENERIC_WAYPOINTS_AND_SIMPLE_NAVIGATION_ONLY = 6 "MAV_AUTOPILOT_GENERIC_WAYPOINTS_AND_SIMPLE_NAVIGATION_ONLY",
        GENERIC_MISSION_FULL = 7 "MAV_AUTOPILOT_GENERIC_MISSION_FULL",
        INVALID = 8 "MAV_AUTOPILOT_INVALID",
        PPZ = 9 "MAV_AUTOPILOT_PPZ",
        UDB = 10 "MAV_AUTOPILOT_UDB",
        FP = 11 "MAV_AUTOPILOT_FP",
        PX4 = 12 "MAV_AUTOPILOT_PX4",
        SMACCMPILOT = 13 "MAV_AUTOPILOT_SMACCMPILOT",
        AUTOQUAD = 14 "MAV_AUTOPILOT_AUTOQUAD",
        ARMAZILA = 15 "MAV_AUTOPILOT_ARMAZILA",
        AEROB = 16 "MAV_AUTOPILOT_AEROB",
        ASLUAV = 17 "MAV_AUTOPILOT_ASLUAV",
        SMARTAP = 18 "MAV_AUTOPILOT_SMARTAP",
        AIRRAILS = 19 "MAV_AUTOPILOT_AIRRAILS",
        REFLEX = 20 "MAV_AUTOPILOT_REFLEX",
        FLIX = 21 "MAV_AUTOPILOT_FLIX",
    }

    /// `MAV_TYPE`: the kind of vehicle or component a [`Heartbeat`](super::Heartbeat) describes.
    mav_type "MAV_TYPE" u8 {
        GENERIC = 0 "MAV_TYPE_GENERIC",
        FIXED_WING = 1 "MAV_TYPE_FIXED_WING",
        QUADROTOR = 2 "MAV_TYPE_QUADROTOR",
        COAXIAL = 3 "MAV_TYPE_COAXIAL",
        HELICOPTER = 4 "MAV_TYPE_HELICOPTER",
        ANTENNA_TRACKER = 5 "MAV_TYPE_ANTENNA_TRACKER",
        GCS = 6 "MAV_TYPE_GCS",
        AIRSHIP = 7 "MAV_TYPE_AIRSHIP",
        FREE_BALLOON = 8 "MAV_TYPE_FREE_BALLOON",
        ROCKET = 9 "MAV_TYPE_ROCKET",
        GROUND_ROVER = 10 "MAV_TYPE_GROUND_ROVER",
        SURFACE_BOAT = 11 "MAV_TYPE_SURFACE_BOAT",
        SUBMARINE = 12 "MAV_TYPE_SUBMARINE",
        HEXAROTOR = 13 "MAV_TYPE_HEXAROTOR",
        OCTOROTOR = 14 "MAV_TYPE_OCTOROTOR",
        TRICOPTER = 15 "MAV_TYPE_TRICOPTER",
        FLAPPING_WING = 16 "MAV_TYPE_FLAPPING_WING",
        KITE = 17 "MAV_TYPE_KITE",
        ONBOARD_CONTROLLER = 18 "MAV_TYPE_ONBOARD_CONTROLLER",
        VTOL_TAILSITTER_DUOROTOR = 19 "MAV_TYPE_VTOL_TAILSITTER_DUOROTOR",
        VTOL_TAILSITTER_QUADROTOR = 20 "MAV_TYPE_VTOL_TAILSITTER_QUADROTOR",
        VTOL_TILTROTOR = 21 "MAV_TYPE_VTOL_TILTROTOR",
        VTOL_FIXEDROTOR = 22 "MAV_TYPE_VTOL_FIXEDROTOR",
        VTOL_TAILSITTER = 23 "MAV_TYPE_VTOL_TAILSITTER",
        VTOL_TILTWING = 24 "MAV_TYPE_VTOL_TILTWING",
        VTOL_RESERVED5 = 25 "MAV_TYPE_VTOL_RESERVED5",
        GIMBAL = 26 "MAV_TYPE_GIMBAL",
        ADSB = 27 "MAV_TYPE_ADSB",
        PARAFOIL = 28 "MAV_TYPE_PARAFOIL",
        DODECAROTOR = 29 "MAV_TYPE_DODECAROTOR",
        CAMERA = 30 "MAV_TYPE_CAMERA",
        CHARGING_STATION = 31 "MAV_TYPE_CHARGING_STATION",
        FLARM = 32 "MAV_TYPE_FLARM",
        SERVO = 33 "MAV_TYPE_SERVO",
        ODID = 34 "MAV_TYPE_ODID",
        DECAROTOR = 35 "MAV_TYPE_DECAROTOR",
        BATTERY = 36 "MAV_TYPE_BATTERY",
        PARACHUTE = 37 "MAV_TYPE_PARACHUTE",
        LOG = 38 "MAV_TYPE_LOG",
        OSD = 39 "MAV_TYPE_OSD",
        IMU = 40 "MAV_TYPE_IMU",
        GPS = 41 "MAV_TYPE_GPS",
        WINCH = 42 "MAV_TYPE_WINCH",
        GENERIC_MULTIROTOR = 43 "MAV_TYPE_GENERIC_MULTIROTOR",
        ILLUMINATOR = 44 "MAV_TYPE_ILLUMINATOR",
        SPACECRAFT_ORBITER = 45 "MAV_TYPE_SPACECRAFT_ORBITER",
        GROUND_QUADRUPED = 46 "MAV_TYPE_GROUND_QUADRUPED",
        VTOL_GYRODYNE = 47 "MAV_TYPE_VTOL_GYRODYNE",
        GRIPPER = 48 "MAV_TYPE_GRIPPER",
        RADIO = 49 "MAV_TYPE_RADIO",
    }

    /// `MAV_MODE_FLAG`: bits of the base mode field of a [`Heartbeat`](super::Heartbeat).
    mav_mode_flag "MAV_MODE_FLAG" u8 bitmask {
        SAFETY_ARMED = 128 "MAV_MODE_FLAG_SAFETY_ARMED",
        MANUAL_INPUT_ENABLED = 64 "MAV_MODE_FLAG_MANUAL_INPUT_ENABLED",
        HIL_ENABLED = 32 "MAV_MODE_FLAG_HIL_ENABLED",
        STABILIZE_ENABLED = 16 "MAV_MODE_FLAG_STABILIZE_ENABLED",
        GUIDED_ENABLED = 8 "MAV_MODE_FLAG_GUIDED_ENABLED",
        AUTO_ENABLED = 4 "MAV_MODE_FLAG_AUTO_ENABLED",
        TEST_ENABLED = 2 "MAV_MODE_FLAG_TEST_ENABLED",
        CUSTOM_MODE_ENABLED = 1 "MAV_MODE_FLAG_CUSTOM_MODE_ENABLED",
    }

    /// `MAV_STATE`: the system status carried by a [`Heartbeat`](super::Heartbeat).
    mav_state "MAV_STATE" u8 {
        UNINIT = 0 "MAV_STATE_UNINIT",
        BOOT = 1 "MAV_STATE_BOOT",
        CALIBRATING = 2 "MAV_STATE_CALIBRATING",
        STANDBY = 3 "MAV_STATE_STANDBY",
        ACTIVE = 4 "MAV_STATE_ACTIVE",
        CRITICAL = 5 "MAV_STATE_CRITICAL",
        EMERGENCY = 6 "MAV_STATE_EMERGENCY",
        POWEROFF = 7 "MAV_STATE_POWEROFF",
        FLIGHT_TERMINATION = 8 "MAV_STATE_FLIGHT_TERMINATION",
    }

    /// `MAV_PROTOCOL_CAPABILITY`: bits of the capabilities field of an
    /// [`AutopilotVersion`](super::AutopilotVersion), naming the protocol features the autopilot supports.
    mav_protocol_capability "MAV_PROTOCOL_CAPABILITY" u64 bitmask {
        MISSION_FLOAT = 1 "MAV_PROTOCOL_CAPABILITY_MISSION_FLOAT",
        PARAM_FLOAT = 2 "MAV_PROTOCOL_CAPABILITY_PARAM_FLOAT",
        MISSION_INT = 4 "MAV_PROTOCOL_CAPABILITY_MISSION_INT",
        COMMAND_INT = 8 "MAV_PROTOCOL_CAPABILITY_COMMAND_INT",
        PARAM_ENCODE_BYTEWISE = 16 "MAV_PROTOCOL_CAPABILITY_PARAM_ENCODE_BYTEWISE",
        FTP = 32 "MAV_PROTOCOL_CAPABILITY_FTP",
        SET_ATTITUDE_TARGET = 64 "MAV_PROTOCOL_CAPABILITY_SET_ATTITUDE_TARGET",
        SET_POSITION_TARGET_LOCAL_NED = 128 "MAV_PROTOCOL_CAPABILITY_SET_POSITION_TARGET_LOCAL_NED",
        SET_POSITION_TARGET_GLOBAL_INT = 256 "MAV_PROTOCOL_CAPABILITY_SET_POSITION_TARGET_GLOBAL_INT",
        TERRAIN = 512 "MAV_PROTOCOL_CAPABILITY_TERRAIN",
        RESERVED3 = 1024 "MAV_PROTOCOL_CAPABILITY_RESERVED3",
        FLIGHT_TERMINATION = 2048 "MAV_PROTOCOL_CAPABILITY_FLIGHT_TERMINATION",
        COMPASS_CALIBRATION = 4096 "MAV_PROTOCOL_CAPABILITY_COMPASS_CALIBRATION",
        MAVLINK2 = 8192 "MAV_PROTOCOL_CAPABILITY_MAVLINK2",
        MISSION_FENCE = 16384 "MAV_PROTOCOL_CAPABILITY_MISSION_FENCE",
        MISSION_RALLY = 32768 "MAV_PROTOCOL_CAPABILITY_MISSION_RALLY",
        RESERVED2 = 65536 "MAV_PROTOCOL_CAPABILITY_RESERVED2",
        PARAM_ENCODE_C_CAST = 131072 "MAV_PROTOCOL_CAPABILITY_PARAM_ENCODE_C_CAST",
        COMPONENT_IMPLEMENTS_GIMBAL_MANAGER = 262144 "MAV_PROTOCOL_CAPABILITY_COMPONENT_IMPLEMENTS_GIMBAL_MANAGER",
        COMPONENT_ACCEPTS_GCS_CONTROL = 524288 "MAV_PROTOCOL_CAPABILITY_COMPONENT_ACCEPTS_GCS_CONTROL",
        GRIPPER = 1048576 "MAV_PROTOCOL_CAPABILITY_GRIPPER",
    }

    /// `MAV_SYS_STATUS_SENSOR`: bits of the three sensor fields of a [`SysStatus`](super::SysStatus),
    /// which say what the vehicle has, what it has switched on, and what is healthy.
    ///
    /// `PREARM_CHECK` is the bit a ground station waits on before arming: ArduPilot and PX4 both
    /// set it in the health field once every pre-arm check passes.
    mav_sys_status_sensor "MAV_SYS_STATUS_SENSOR" u32 bitmask {
        GYRO_3D = 1 "MAV_SYS_STATUS_SENSOR_3D_GYRO",
        ACCEL_3D = 2 "MAV_SYS_STATUS_SENSOR_3D_ACCEL",
        MAG_3D = 4 "MAV_SYS_STATUS_SENSOR_3D_MAG",
        ABSOLUTE_PRESSURE = 8 "MAV_SYS_STATUS_SENSOR_ABSOLUTE_PRESSURE",
        DIFFERENTIAL_PRESSURE = 16 "MAV_SYS_STATUS_SENSOR_DIFFERENTIAL_PRESSURE",
        GPS = 32 "MAV_SYS_STATUS_SENSOR_GPS",
        OPTICAL_FLOW = 64 "MAV_SYS_STATUS_SENSOR_OPTICAL_FLOW",
        VISION_POSITION = 128 "MAV_SYS_STATUS_SENSOR_VISION_POSITION",
        LASER_POSITION = 256 "MAV_SYS_STATUS_SENSOR_LASER_POSITION",
        EXTERNAL_GROUND_TRUTH = 512 "MAV_SYS_STATUS_SENSOR_EXTERNAL_GROUND_TRUTH",
        ANGULAR_RATE_CONTROL = 1024 "MAV_SYS_STATUS_SENSOR_ANGULAR_RATE_CONTROL",
        ATTITUDE_STABILIZATION = 2048 "MAV_SYS_STATUS_SENSOR_ATTITUDE_STABILIZATION",
        YAW_POSITION = 4096 "MAV_SYS_STATUS_SENSOR_YAW_POSITION",
        Z_ALTITUDE_CONTROL = 8192 "MAV_SYS_STATUS_SENSOR_Z_ALTITUDE_CONTROL",
        XY_POSITION_CONTROL = 16384 "MAV_SYS_STATUS_SENSOR_XY_POSITION_CONTROL",
        MOTOR_OUTPUTS = 32768 "MAV_SYS_STATUS_SENSOR_MOTOR_OUTPUTS",
        RC_RECEIVER = 65536 "MAV_SYS_STATUS_SENSOR_RC_RECEIVER",
        GYRO2_3D = 131072 "MAV_SYS_STATUS_SENSOR_3D_GYRO2",
        ACCEL2_3D = 262144 "MAV_SYS_STATUS_SENSOR_3D_ACCEL2",
        MAG2_3D = 524288 "MAV_SYS_STATUS_SENSOR_3D_MAG2",
        GEOFENCE = 1048576 "MAV_SYS_STATUS_GEOFENCE",
        AHRS = 2097152 "MAV_SYS_STATUS_AHRS",
        TERRAIN = 4194304 "MAV_SYS_STATUS_TERRAIN",
        REVERSE_MOTOR = 8388608 "MAV_SYS_STATUS_REVERSE_MOTOR",
        LOGGING = 16777216 "MAV_SYS_STATUS_LOGGING",
        BATTERY = 33554432 "MAV_SYS_STATUS_SENSOR_BATTERY",
        PROXIMITY = 67108864 "MAV_SYS_STATUS_SENSOR_PROXIMITY",
        SATCOM = 134217728 "MAV_SYS_STATUS_SENSOR_SATCOM",
        PREARM_CHECK = 268435456 "MAV_SYS_STATUS_PREARM_CHECK",
        OBSTACLE_AVOIDANCE = 536870912 "MAV_SYS_STATUS_OBSTACLE_AVOIDANCE",
        PROPULSION = 1073741824 "MAV_SYS_STATUS_SENSOR_PROPULSION",
        EXTENSION_USED = 2147483648 "MAV_SYS_STATUS_EXTENSION_USED",
    }

    /// `MAV_SYS_STATUS_SENSOR_EXTENDED`: bits of the three extended sensor fields of a
    /// [`SysStatus`](super::SysStatus), for the sensors the first 32 bits have no room for.
    mav_sys_status_sensor_extended "MAV_SYS_STATUS_SENSOR_EXTENDED" u32 bitmask {
        RECOVERY_SYSTEM = 1 "MAV_SYS_STATUS_RECOVERY_SYSTEM",
        LEAK = 2 "MAV_SYS_STATUS_SENSOR_LEAK",
        GYRO3_3D = 4 "MAV_SYS_STATUS_SENSOR_3D_GYRO3",
        ACCEL3_3D = 8 "MAV_SYS_STATUS_SENSOR_3D_ACCEL3",
        GYRO4_3D = 16 "MAV_SYS_STATUS_SENSOR_3D_GYRO4",
        ACCEL4_3D = 32 "MAV_SYS_STATUS_SENSOR_3D_ACCEL4",
        MAG3_3D = 64 "MAV_SYS_STATUS_SENSOR_3D_MAG3",
        MAG4_3D = 128 "MAV_SYS_STATUS_SENSOR_3D_MAG4",
    }

    /// `MAV_FRAME`: the coordinate frame of a mission item or position target.
    mav_frame "MAV_FRAME" u8 {
        GLOBAL = 0 "MAV_FRAME_GLOBAL",
        LOCAL_NED = 1 "MAV_FRAME_LOCAL_NED",
        MISSION = 2 "MAV_FRAME_MISSION",
        GLOBAL_RELATIVE_ALT = 3 "MAV_FRAME_GLOBAL_RELATIVE_ALT",
        LOCAL_ENU = 4 "MAV_FRAME_LOCAL_ENU",
        GLOBAL_INT = 5 "MAV_FRAME_GLOBAL_INT",
        GLOBAL_RELATIVE_ALT_INT = 6 "MAV_FRAME_GLOBAL_RELATIVE_ALT_INT",
        LOCAL_OFFSET_NED = 7 "MAV_FRAME_LOCAL_OFFSET_NED",
        BODY_NED = 8 "MAV_FRAME_BODY_NED",
        BODY_OFFSET_NED = 9 "MAV_FRAME_BODY_OFFSET_NED",
        GLOBAL_TERRAIN_ALT = 10 "MAV_FRAME_GLOBAL_TERRAIN_ALT",
        GLOBAL_TERRAIN_ALT_INT = 11 "MAV_FRAME_GLOBAL_TERRAIN_ALT_INT",
        BODY_FRD = 12 "MAV_FRAME_BODY_FRD",
        RESERVED_13 = 13 "MAV_FRAME_RESERVED_13",
        RESERVED_14 = 14 "MAV_FRAME_RESERVED_14",
        RESERVED_15 = 15 "MAV_FRAME_RESERVED_15",
        RESERVED_16 = 16 "MAV_FRAME_RESERVED_16",
        RESERVED_17 = 17 "MAV_FRAME_RESERVED_17",
        RESERVED_18 = 18 "MAV_FRAME_RESERVED_18",
        RESERVED_19 = 19 "MAV_FRAME_RESERVED_19",
        LOCAL_FRD = 20 "MAV_FRAME_LOCAL_FRD",
        LOCAL_FLU = 21 "MAV_FRAME_LOCAL_FLU",
    }

    /// `MAV_CMD`: command ids for [`CommandLong`](super::CommandLong), [`CommandInt`](super::CommandInt),
    /// and the items of a mission, [`MissionItemInt`](super::MissionItemInt).
    mav_cmd "MAV_CMD" u16 {
        NAV_WAYPOINT = 16 "MAV_CMD_NAV_WAYPOINT",
        NAV_LOITER_UNLIM = 17 "MAV_CMD_NAV_LOITER_UNLIM",
        NAV_LOITER_TURNS = 18 "MAV_CMD_NAV_LOITER_TURNS",
        NAV_LOITER_TIME = 19 "MAV_CMD_NAV_LOITER_TIME",
        NAV_RETURN_TO_LAUNCH = 20 "MAV_CMD_NAV_RETURN_TO_LAUNCH",
        NAV_LAND = 21 "MAV_CMD_NAV_LAND",
        NAV_TAKEOFF = 22 "MAV_CMD_NAV_TAKEOFF",
        NAV_LAND_LOCAL = 23 "MAV_CMD_NAV_LAND_LOCAL",
        NAV_TAKEOFF_LOCAL = 24 "MAV_CMD_NAV_TAKEOFF_LOCAL",
        NAV_CONTINUE_AND_CHANGE_ALT = 30 "MAV_CMD_NAV_CONTINUE_AND_CHANGE_ALT",
        NAV_LOITER_TO_ALT = 31 "MAV_CMD_NAV_LOITER_TO_ALT",
        DO_FOLLOW = 32 "MAV_CMD_DO_FOLLOW",
        DO_FOLLOW_REPOSITION = 33 "MAV_CMD_DO_FOLLOW_REPOSITION",
        DO_ORBIT = 34 "MAV_CMD_DO_ORBIT",
        DO_FIGURE_EIGHT = 35 "MAV_CMD_DO_FIGURE_EIGHT",
        NAV_ARC_WAYPOINT = 36 "MAV_CMD_NAV_ARC_WAYPOINT",
        NAV_ROI = 80 "MAV_CMD_NAV_ROI",
        NAV_PATHPLANNING = 81 "MAV_CMD_NAV_PATHPLANNING",
        NAV_SPLINE_WAYPOINT = 82 "MAV_CMD_NAV_SPLINE_WAYPOINT",
        NAV_VTOL_TAKEOFF = 84 "MAV_CMD_NAV_VTOL_TAKEOFF",
        NAV_VTOL_LAND = 85 "MAV_CMD_NAV_VTOL_LAND",
        NAV_GUIDED_ENABLE = 92 "MAV_CMD_NAV_GUIDED_ENABLE",
        NAV_DELAY = 93 "MAV_CMD_NAV_DELAY",
        NAV_PAYLOAD_PLACE = 94 "MAV_CMD_NAV_PAYLOAD_PLACE",
        NAV_LAST = 95 "MAV_CMD_NAV_LAST",
        CONDITION_DELAY = 112 "MAV_CMD_CONDITION_DELAY",
        CONDITION_CHANGE_ALT = 113 "MAV_CMD_CONDITION_CHANGE_ALT",
        CONDITION_DISTANCE = 114 "MAV_CMD_CONDITION_DISTANCE",
        CONDITION_YAW = 115 "MAV_CMD_CONDITION_YAW",
        CONDITION_LAST = 159 "MAV_CMD_CONDITION_LAST",
        DO_SET_MODE = 176 "MAV_CMD_DO_SET_MODE",
        DO_JUMP = 177 "MAV_CMD_DO_JUMP",
        DO_CHANGE_SPEED = 178 "MAV_CMD_DO_CHANGE_SPEED",
        DO_SET_HOME = 179 "MAV_CMD_DO_SET_HOME",
        DO_SET_PARAMETER = 180 "MAV_CMD_DO_SET_PARAMETER",
        DO_SET_RELAY = 181 "MAV_CMD_DO_SET_RELAY",
        DO_REPEAT_RELAY = 182 "MAV_CMD_DO_REPEAT_RELAY",
        DO_SET_SERVO = 183 "MAV_CMD_DO_SET_SERVO",
        DO_REPEAT_SERVO = 184 "MAV_CMD_DO_REPEAT_SERVO",
        DO_FLIGHTTERMINATION = 185 "MAV_CMD_DO_FLIGHTTERMINATION",
        DO_CHANGE_ALTITUDE = 186 "MAV_CMD_DO_CHANGE_ALTITUDE",
        DO_SET_ACTUATOR = 187 "MAV_CMD_DO_SET_ACTUATOR",
        DO_RETURN_PATH_START = 188 "MAV_CMD_DO_RETURN_PATH_START",
        DO_LAND_START = 189 "MAV_CMD_DO_LAND_START",
        DO_RALLY_LAND = 190 "MAV_CMD_DO_RALLY_LAND",
        DO_GO_AROUND = 191 "MAV_CMD_DO_GO_AROUND",
        DO_REPOSITION = 192 "MAV_CMD_DO_REPOSITION",
        DO_PAUSE_CONTINUE = 193 "MAV_CMD_DO_PAUSE_CONTINUE",
        DO_SET_REVERSE = 194 "MAV_CMD_DO_SET_REVERSE",
        DO_SET_ROI_LOCATION = 195 "MAV_CMD_DO_SET_ROI_LOCATION",
        DO_SET_ROI_WPNEXT_OFFSET = 196 "MAV_CMD_DO_SET_ROI_WPNEXT_OFFSET",
        DO_SET_ROI_NONE = 197 "MAV_CMD_DO_SET_ROI_NONE",
        DO_SET_ROI_SYSID = 198 "MAV_CMD_DO_SET_ROI_SYSID",
        DO_CONTROL_VIDEO = 200 "MAV_CMD_DO_CONTROL_VIDEO",
        DO_SET_ROI = 201 "MAV_CMD_DO_SET_ROI",
        DO_DIGICAM_CONFIGURE = 202 "MAV_CMD_DO_DIGICAM_CONFIGURE",
        DO_DIGICAM_CONTROL = 203 "MAV_CMD_DO_DIGICAM_CONTROL",
        DO_MOUNT_CONFIGURE = 204 "MAV_CMD_DO_MOUNT_CONFIGURE",
        DO_MOUNT_CONTROL = 205 "MAV_CMD_DO_MOUNT_CONTROL",
        DO_SET_CAM_TRIGG_DIST = 206 "MAV_CMD_DO_SET_CAM_TRIGG_DIST",
        DO_FENCE_ENABLE = 207 "MAV_CMD_DO_FENCE_ENABLE",
        DO_PARACHUTE = 208 "MAV_CMD_DO_PARACHUTE",
        DO_MOTOR_TEST = 209 "MAV_CMD_DO_MOTOR_TEST",
        DO_INVERTED_FLIGHT = 210 "MAV_CMD_DO_INVERTED_FLIGHT",
        DO_GRIPPER = 211 "MAV_CMD_DO_GRIPPER",
        DO_AUTOTUNE_ENABLE = 212 "MAV_CMD_DO_AUTOTUNE_ENABLE",
        NAV_SET_YAW_SPEED = 213 "MAV_CMD_NAV_SET_YAW_SPEED",
        DO_SET_CAM_TRIGG_INTERVAL = 214 "MAV_CMD_DO_SET_CAM_TRIGG_INTERVAL",
        DO_MOUNT_CONTROL_QUAT = 220 "MAV_CMD_DO_MOUNT_CONTROL_QUAT",
        DO_GUIDED_MASTER = 221 "MAV_CMD_DO_GUIDED_MASTER",
        DO_GUIDED_LIMITS = 222 "MAV_CMD_DO_GUIDED_LIMITS",
        DO_ENGINE_CONTROL = 223 "MAV_CMD_DO_ENGINE_CONTROL",
        DO_SET_MISSION_CURRENT = 224 "MAV_CMD_DO_SET_MISSION_CURRENT",
        DO_LAST = 240 "MAV_CMD_DO_LAST",
        PREFLIGHT_CALIBRATION = 241 "MAV_CMD_PREFLIGHT_CALIBRATION",
        PREFLIGHT_SET_SENSOR_OFFSETS = 242 "MAV_CMD_PREFLIGHT_SET_SENSOR_OFFSETS",
        PREFLIGHT_UAVCAN = 243 "MAV_CMD_PREFLIGHT_UAVCAN",
        PREFLIGHT_STORAGE = 245 "MAV_CMD_PREFLIGHT_STORAGE",
        PREFLIGHT_REBOOT_SHUTDOWN = 246 "MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN",
        OVERRIDE_GOTO = 252 "MAV_CMD_OVERRIDE_GOTO",
        OBLIQUE_SURVEY = 260 "MAV_CMD_OBLIQUE_SURVEY",
        DO_SET_STANDARD_MODE = 262 "MAV_CMD_DO_SET_STANDARD_MODE",
        MISSION_START = 300 "MAV_CMD_MISSION_START",
        ACTUATOR_TEST = 310 "MAV_CMD_ACTUATOR_TEST",
        CONFIGURE_ACTUATOR = 311 "MAV_CMD_CONFIGURE_ACTUATOR",
        COMPONENT_ARM_DISARM = 400 "MAV_CMD_COMPONENT_ARM_DISARM",
        RUN_PREARM_CHECKS = 401 "MAV_CMD_RUN_PREARM_CHECKS",
        ILLUMINATOR_ON_OFF = 405 "MAV_CMD_ILLUMINATOR_ON_OFF",
        DO_ILLUMINATOR_CONFIGURE = 406 "MAV_CMD_DO_ILLUMINATOR_CONFIGURE",
        GET_HOME_POSITION = 410 "MAV_CMD_GET_HOME_POSITION",
        INJECT_FAILURE = 420 "MAV_CMD_INJECT_FAILURE",
        START_RX_PAIR = 500 "MAV_CMD_START_RX_PAIR",
        GET_MESSAGE_INTERVAL = 510 "MAV_CMD_GET_MESSAGE_INTERVAL",
        SET_MESSAGE_INTERVAL = 511 "MAV_CMD_SET_MESSAGE_INTERVAL",
        REQUEST_MESSAGE = 512 "MAV_CMD_REQUEST_MESSAGE",
        REQUEST_PROTOCOL_VERSION = 519 "MAV_CMD_REQUEST_PROTOCOL_VERSION",
        REQUEST_AUTOPILOT_CAPABILITIES = 520 "MAV_CMD_REQUEST_AUTOPILOT_CAPABILITIES",
        REQUEST_CAMERA_INFORMATION = 521 "MAV_CMD_REQUEST_CAMERA_INFORMATION",
        REQUEST_CAMERA_SETTINGS = 522 "MAV_CMD_REQUEST_CAMERA_SETTINGS",
        REQUEST_STORAGE_INFORMATION = 525 "MAV_CMD_REQUEST_STORAGE_INFORMATION",
        STORAGE_FORMAT = 526 "MAV_CMD_STORAGE_FORMAT",
        REQUEST_CAMERA_CAPTURE_STATUS = 527 "MAV_CMD_REQUEST_CAMERA_CAPTURE_STATUS",
        REQUEST_FLIGHT_INFORMATION = 528 "MAV_CMD_REQUEST_FLIGHT_INFORMATION",
        RESET_CAMERA_SETTINGS = 529 "MAV_CMD_RESET_CAMERA_SETTINGS",
        SET_CAMERA_MODE = 530 "MAV_CMD_SET_CAMERA_MODE",
        SET_CAMERA_ZOOM = 531 "MAV_CMD_SET_CAMERA_ZOOM",
        SET_CAMERA_FOCUS = 532 "MAV_CMD_SET_CAMERA_FOCUS",
        SET_STORAGE_USAGE = 533 "MAV_CMD_SET_STORAGE_USAGE",
        SET_CAMERA_SOURCE = 534 "MAV_CMD_SET_CAMERA_SOURCE",
        JUMP_TAG = 600 "MAV_CMD_JUMP_TAG",
        DO_JUMP_TAG = 601 "MAV_CMD_DO_JUMP_TAG",
        DO_SET_GLOBAL_ORIGIN = 611 "MAV_CMD_DO_SET_GLOBAL_ORIGIN",
        DO_GIMBAL_MANAGER_PITCHYAW = 1000 "MAV_CMD_DO_GIMBAL_MANAGER_PITCHYAW",
        DO_GIMBAL_MANAGER_CONFIGURE = 1001 "MAV_CMD_DO_GIMBAL_MANAGER_CONFIGURE",
        IMAGE_START_CAPTURE = 2000 "MAV_CMD_IMAGE_START_CAPTURE",
        IMAGE_STOP_CAPTURE = 2001 "MAV_CMD_IMAGE_STOP_CAPTURE",
        REQUEST_CAMERA_IMAGE_CAPTURE = 2002 "MAV_CMD_REQUEST_CAMERA_IMAGE_CAPTURE",
        DO_TRIGGER_CONTROL = 2003 "MAV_CMD_DO_TRIGGER_CONTROL",
        CAMERA_TRACK_POINT = 2004 "MAV_CMD_CAMERA_TRACK_POINT",
        CAMERA_TRACK_RECTANGLE = 2005 "MAV_CMD_CAMERA_TRACK_RECTANGLE",
        CAMERA_STOP_TRACKING = 2010 "MAV_CMD_CAMERA_STOP_TRACKING",
        VIDEO_START_CAPTURE = 2500 "MAV_CMD_VIDEO_START_CAPTURE",
        VIDEO_STOP_CAPTURE = 2501 "MAV_CMD_VIDEO_STOP_CAPTURE",
        VIDEO_START_STREAMING = 2502 "MAV_CMD_VIDEO_START_STREAMING",
        VIDEO_STOP_STREAMING = 2503 "MAV_CMD_VIDEO_STOP_STREAMING",
        REQUEST_VIDEO_STREAM_INFORMATION = 2504 "MAV_CMD_REQUEST_VIDEO_STREAM_INFORMATION",
        REQUEST_VIDEO_STREAM_STATUS = 2505 "MAV_CMD_REQUEST_VIDEO_STREAM_STATUS",
        LOGGING_START = 2510 "MAV_CMD_LOGGING_START",
        LOGGING_STOP = 2511 "MAV_CMD_LOGGING_STOP",
        AIRFRAME_CONFIGURATION = 2520 "MAV_CMD_AIRFRAME_CONFIGURATION",
        CONTROL_HIGH_LATENCY = 2600 "MAV_CMD_CONTROL_HIGH_LATENCY",
        PANORAMA_CREATE = 2800 "MAV_CMD_PANORAMA_CREATE",
        DO_VTOL_TRANSITION = 3000 "MAV_CMD_DO_VTOL_TRANSITION",
        ARM_AUTHORIZATION_REQUEST = 3001 "MAV_CMD_ARM_AUTHORIZATION_REQUEST",
        SET_GUIDED_SUBMODE_STANDARD = 4000 "MAV_CMD_SET_GUIDED_SUBMODE_STANDARD",
        SET_GUIDED_SUBMODE_CIRCLE = 4001 "MAV_CMD_SET_GUIDED_SUBMODE_CIRCLE",
        CONDITION_GATE = 4501 "MAV_CMD_CONDITION_GATE",
        NAV_FENCE_RETURN_POINT = 5000 "MAV_CMD_NAV_FENCE_RETURN_POINT",
        NAV_FENCE_POLYGON_VERTEX_INCLUSION = 5001 "MAV_CMD_NAV_FENCE_POLYGON_VERTEX_INCLUSION",
        NAV_FENCE_POLYGON_VERTEX_EXCLUSION = 5002 "MAV_CMD_NAV_FENCE_POLYGON_VERTEX_EXCLUSION",
        NAV_FENCE_CIRCLE_INCLUSION = 5003 "MAV_CMD_NAV_FENCE_CIRCLE_INCLUSION",
        NAV_FENCE_CIRCLE_EXCLUSION = 5004 "MAV_CMD_NAV_FENCE_CIRCLE_EXCLUSION",
        NAV_RALLY_POINT = 5100 "MAV_CMD_NAV_RALLY_POINT",
        UAVCAN_GET_NODE_INFO = 5200 "MAV_CMD_UAVCAN_GET_NODE_INFO",
        DO_SET_SAFETY_SWITCH_STATE = 5300 "MAV_CMD_DO_SET_SAFETY_SWITCH_STATE",
        DO_ADSB_OUT_IDENT = 10001 "MAV_CMD_DO_ADSB_OUT_IDENT",
        PAYLOAD_PREPARE_DEPLOY = 30001 "MAV_CMD_PAYLOAD_PREPARE_DEPLOY",
        PAYLOAD_CONTROL_DEPLOY = 30002 "MAV_CMD_PAYLOAD_CONTROL_DEPLOY",
        FIXED_MAG_CAL_YAW = 42006 "MAV_CMD_FIXED_MAG_CAL_YAW",
        DO_WINCH = 42600 "MAV_CMD_DO_WINCH",
        GUIDED_CHANGE_SPEED = 43000 "MAV_CMD_GUIDED_CHANGE_SPEED",
        GUIDED_CHANGE_ALTITUDE = 43001 "MAV_CMD_GUIDED_CHANGE_ALTITUDE",
        GUIDED_CHANGE_HEADING = 43002 "MAV_CMD_GUIDED_CHANGE_HEADING",
        EXTERNAL_POSITION_ESTIMATE = 43003 "MAV_CMD_EXTERNAL_POSITION_ESTIMATE",
        WAYPOINT_USER_1 = 31000 "MAV_CMD_WAYPOINT_USER_1",
        WAYPOINT_USER_2 = 31001 "MAV_CMD_WAYPOINT_USER_2",
        WAYPOINT_USER_3 = 31002 "MAV_CMD_WAYPOINT_USER_3",
        WAYPOINT_USER_4 = 31003 "MAV_CMD_WAYPOINT_USER_4",
        WAYPOINT_USER_5 = 31004 "MAV_CMD_WAYPOINT_USER_5",
        SPATIAL_USER_1 = 31005 "MAV_CMD_SPATIAL_USER_1",
        SPATIAL_USER_2 = 31006 "MAV_CMD_SPATIAL_USER_2",
        SPATIAL_USER_3 = 31007 "MAV_CMD_SPATIAL_USER_3",
        SPATIAL_USER_4 = 31008 "MAV_CMD_SPATIAL_USER_4",
        SPATIAL_USER_5 = 31009 "MAV_CMD_SPATIAL_USER_5",
        USER_1 = 31010 "MAV_CMD_USER_1",
        USER_2 = 31011 "MAV_CMD_USER_2",
        USER_3 = 31012 "MAV_CMD_USER_3",
        USER_4 = 31013 "MAV_CMD_USER_4",
        USER_5 = 31014 "MAV_CMD_USER_5",
        CAN_FORWARD = 32000 "MAV_CMD_CAN_FORWARD",
    }

    /// `MAV_PARAM_TYPE`: the type of a parameter's value, in a [`ParamValue`](super::ParamValue) or a
    /// [`ParamSet`](super::ParamSet).
    mav_param_type "MAV_PARAM_TYPE" u8 {
        UINT8 = 1 "MAV_PARAM_TYPE_UINT8",
        INT8 = 2 "MAV_PARAM_TYPE_INT8",
        UINT16 = 3 "MAV_PARAM_TYPE_UINT16",
        INT16 = 4 "MAV_PARAM_TYPE_INT16",
        UINT32 = 5 "MAV_PARAM_TYPE_UINT32",
        INT32 = 6 "MAV_PARAM_TYPE_INT32",
        UINT64 = 7 "MAV_PARAM_TYPE_UINT64",
        INT64 = 8 "MAV_PARAM_TYPE_INT64",
        REAL32 = 9 "MAV_PARAM_TYPE_REAL32",
        REAL64 = 10 "MAV_PARAM_TYPE_REAL64",
    }

    /// `MAV_RESULT`: the outcome a [`CommandAck`](super::CommandAck) reports.
    mav_result "MAV_RESULT" u8 {
        ACCEPTED = 0 "MAV_RESULT_ACCEPTED",
        TEMPORARILY_REJECTED = 1 "MAV_RESULT_TEMPORARILY_REJECTED",
        DENIED = 2 "MAV_RESULT_DENIED",
        UNSUPPORTED = 3 "MAV_RESULT_UNSUPPORTED",
        FAILED = 4 "MAV_RESULT_FAILED",
        IN_PROGRESS = 5 "MAV_RESULT_IN_PROGRESS",
        CANCELLED = 6 "MAV_RESULT_CANCELLED",
        COMMAND_LONG_ONLY = 7 "MAV_RESULT_COMMAND_LONG_ONLY",
        COMMAND_INT_ONLY = 8 "MAV_RESULT_COMMAND_INT_ONLY",
        COMMAND_UNSUPPORTED_MAV_FRAME = 9 "MAV_RESULT_COMMAND_UNSUPPORTED_MAV_FRAME",
        NOT_IN_CONTROL = 10 "MAV_RESULT_NOT_IN_CONTROL",
    }

    /// `MAV_MISSION_RESULT`: the outcome a [`MissionAck`](super::MissionAck) reports.
    mav_mission_result "MAV_MISSION_RESULT" u8 {
        ACCEPTED = 0 "MAV_MISSION_ACCEPTED",
        ERROR = 1 "MAV_MISSION_ERROR",
        UNSUPPORTED_FRAME = 2 "MAV_MISSION_UNSUPPORTED_FRAME",
        UNSUPPORTED = 3 "MAV_MISSION_UNSUPPORTED",
        NO_SPACE = 4 "MAV_MISSION_NO_SPACE",
        INVALID = 5 "MAV_MISSION_INVALID",
        INVALID_PARAM1 = 6 "MAV_MISSION_INVALID_PARAM1",
        INVALID_PARAM2 = 7 "MAV_MISSION_INVALID_PARAM2",
        INVALID_PARAM3 = 8 "MAV_MISSION_INVALID_PARAM3",
        INVALID_PARAM4 = 9 "MAV_MISSION_INVALID_PARAM4",
        INVALID_PARAM5_X = 10 "MAV_MISSION_INVALID_PARAM5_X",
        INVALID_PARAM6_Y = 11 "MAV_MISSION_INVALID_PARAM6_Y",
        INVALID_PARAM7 = 12 "MAV_MISSION_INVALID_PARAM7",
        INVALID_SEQUENCE = 13 "MAV_MISSION_INVALID_SEQUENCE",
        DENIED = 14 "MAV_MISSION_DENIED",
        OPERATION_CANCELLED = 15 "MAV_MISSION_OPERATION_CANCELLED",
    }

    /// `MAV_SEVERITY`: how urgent a [`Statustext`](super::Statustext) is, from emergency down to debug.
    mav_severity "MAV_SEVERITY" u8 {
        EMERGENCY = 0 "MAV_SEVERITY_EMERGENCY",
        ALERT = 1 "MAV_SEVERITY_ALERT",
        CRITICAL = 2 "MAV_SEVERITY_CRITICAL",
        ERROR = 3 "MAV_SEVERITY_ERROR",
        WARNING = 4 "MAV_SEVERITY_WARNING",
        NOTICE = 5 "MAV_SEVERITY_NOTICE",
        INFO = 6 "MAV_SEVERITY_INFO",
        DEBUG = 7 "MAV_SEVERITY_DEBUG",
    }

    /// `MAV_MISSION_TYPE`: which plan a mission transfer carries, in the `mission_type` field of
    /// the MISSION_* messages.
    mav_mission_type "MAV_MISSION_TYPE" u8 {
        MISSION = 0 "MAV_MISSION_TYPE_MISSION",
        FENCE = 1 "MAV_MISSION_TYPE_FENCE",
        RALLY = 2 "MAV_MISSION_TYPE_RALLY",
        ALL = 255 "MAV_MISSION_TYPE_ALL",
    }

    /// `MAV_BATTERY_TYPE`: the chemistry of the battery a [`BatteryStatus`](super::BatteryStatus)
    /// describes.
    mav_battery_type "MAV_BATTERY_TYPE" u8 {
        UNKNOWN = 0 "MAV_BATTERY_TYPE_UNKNOWN",
        LIPO = 1 "MAV_BATTERY_TYPE_LIPO",
        LIFE = 2 "MAV_BATTERY_TYPE_LIFE",
        LION = 3 "MAV_BATTERY_TYPE_LION",
        NIMH = 4 "MAV_BATTERY_TYPE_NIMH",
    }

    /// `MAV_BATTERY_FUNCTION`: what the battery a [`BatteryStatus`](super::BatteryStatus) describes
    /// powers.
    mav_battery_function "MAV_BATTERY_FUNCTION" u8 {
        UNKNOWN = 0 "MAV_BATTERY_FUNCTION_UNKNOWN",
        ALL = 1 "MAV_BATTERY_FUNCTION_ALL",
        PROPULSION = 2 "MAV_BATTERY_FUNCTION_PROPULSION",
        AVIONICS = 3 "MAV_BATTERY_FUNCTION_AVIONICS",
        PAYLOAD = 4 "MAV_BATTERY_FUNCTION_PAYLOAD",
    }

    /// `MAV_BATTERY_CHARGE_STATE`: how much charge a [`BatteryStatus`](super::BatteryStatus) says is
    /// left, as the autopilot judges it.
    mav_battery_charge_state "MAV_BATTERY_CHARGE_STATE" u8 {
        UNDEFINED = 0 "MAV_BATTERY_CHARGE_STATE_UNDEFINED",
        OK = 1 "MAV_BATTERY_CHARGE_STATE_OK",
        LOW = 2 "MAV_BATTERY_CHARGE_STATE_LOW",
        CRITICAL = 3 "MAV_BATTERY_CHARGE_STATE_CRITICAL",
        EMERGENCY = 4 "MAV_BATTERY_CHARGE_STATE_EMERGENCY",
        FAILED = 5 "MAV_BATTERY_CHARGE_STATE_FAILED",
        UNHEALTHY = 6 "MAV_BATTERY_CHARGE_STATE_UNHEALTHY",
        CHARGING = 7 "MAV_BATTERY_CHARGE_STATE_CHARGING",
    }

    /// `MAV_BATTERY_MODE`: whether the battery a [`BatteryStatus`](super::BatteryStatus) describes is in
    /// normal use, discharging for storage, or being swapped.
    mav_battery_mode "MAV_BATTERY_MODE" u8 {
        UNKNOWN = 0 "MAV_BATTERY_MODE_UNKNOWN",
        AUTO_DISCHARGING = 1 "MAV_BATTERY_MODE_AUTO_DISCHARGING",
        HOT_SWAP = 2 "MAV_BATTERY_MODE_HOT_SWAP",
    }

    /// `MAV_BATTERY_FAULT`: bits of the fault field of a [`BatteryStatus`](super::BatteryStatus).
    mav_battery_fault "MAV_BATTERY_FAULT" u32 bitmask {
        DEEP_DISCHARGE = 1 "MAV_BATTERY_FAULT_DEEP_DISCHARGE",
        SPIKES = 2 "MAV_BATTERY_FAULT_SPIKES",
        CELL_FAIL = 4 "MAV_BATTERY_FAULT_CELL_FAIL",
        OVER_CURRENT = 8 "MAV_BATTERY_FAULT_OVER_CURRENT",
        OVER_TEMPERATURE = 16 "MAV_BATTERY_FAULT_OVER_TEMPERATURE",
        UNDER_TEMPERATURE = 32 "MAV_BATTERY_FAULT_UNDER_TEMPERATURE",
        INCOMPATIBLE_VOLTAGE = 64 "MAV_BATTERY_FAULT_INCOMPATIBLE_VOLTAGE",
        INCOMPATIBLE_FIRMWARE = 128 "MAV_BATTERY_FAULT_INCOMPATIBLE_FIRMWARE",
        BATTERY_FAULT_INCOMPATIBLE_CELLS_CONFIGURATION = 256 "BATTERY_FAULT_INCOMPATIBLE_CELLS_CONFIGURATION",
    }

    /// `MAV_VTOL_STATE`: the flight phase of a VTOL vehicle, in an
    /// [`ExtendedSysState`](super::ExtendedSysState).
    mav_vtol_state "MAV_VTOL_STATE" u8 {
        UNDEFINED = 0 "MAV_VTOL_STATE_UNDEFINED",
        TRANSITION_TO_FW = 1 "MAV_VTOL_STATE_TRANSITION_TO_FW",
        TRANSITION_TO_MC = 2 "MAV_VTOL_STATE_TRANSITION_TO_MC",
        MC = 3 "MAV_VTOL_STATE_MC",
        FW = 4 "MAV_VTOL_STATE_FW",
    }

    /// `MAV_LANDED_STATE`: whether the vehicle is on the ground, taking off, flying, or landing, in an
    /// [`ExtendedSysState`](super::ExtendedSysState).
    mav_landed_state "MAV_LANDED_STATE" u8 {
        UNDEFINED = 0 "MAV_LANDED_STATE_UNDEFINED",
        ON_GROUND = 1 "MAV_LANDED_STATE_ON_GROUND",
        IN_AIR = 2 "MAV_LANDED_STATE_IN_AIR",
        TAKEOFF = 3 "MAV_LANDED_STATE_TAKEOFF",
        LANDING = 4 "MAV_LANDED_STATE_LANDING",
    }

    /// `GPS_FIX_TYPE`: the kind of fix a [`GpsRawInt`](super::GpsRawInt) reports.
    gps_fix_type "GPS_FIX_TYPE" u8 {
        NO_GPS = 0 "GPS_FIX_TYPE_NO_GPS",
        NO_FIX = 1 "GPS_FIX_TYPE_NO_FIX",
        FIX_2D = 2 "GPS_FIX_TYPE_2D_FIX",
        FIX_3D = 3 "GPS_FIX_TYPE_3D_FIX",
        DGPS = 4 "GPS_FIX_TYPE_DGPS",
        RTK_FLOAT = 5 "GPS_FIX_TYPE_RTK_FLOAT",
        RTK_FIXED = 6 "GPS_FIX_TYPE_RTK_FIXED",
        STATIC = 7 "GPS_FIX_TYPE_STATIC",
        PPP = 8 "GPS_FIX_TYPE_PPP",
    }

    /// `POSITION_TARGET_TYPEMASK`: the bits of the `type_mask` field of
    /// [`SetPositionTargetLocalNed`](super::SetPositionTargetLocalNed) and
    /// [`SetPositionTargetGlobalInt`](super::SetPositionTargetGlobalInt).
    ///
    /// A set bit tells the vehicle to ignore that dimension of the setpoint, so a position-only
    /// setpoint sets the velocity, acceleration, yaw, and yaw-rate ignore bits. `FORCE_SET`
    /// reinterprets the acceleration fields as a force setpoint.
    position_target_typemask "POSITION_TARGET_TYPEMASK" u16 bitmask {
        X_IGNORE = 1 "POSITION_TARGET_TYPEMASK_X_IGNORE",
        Y_IGNORE = 2 "POSITION_TARGET_TYPEMASK_Y_IGNORE",
        Z_IGNORE = 4 "POSITION_TARGET_TYPEMASK_Z_IGNORE",
        VX_IGNORE = 8 "POSITION_TARGET_TYPEMASK_VX_IGNORE",
        VY_IGNORE = 16 "POSITION_TARGET_TYPEMASK_VY_IGNORE",
        VZ_IGNORE = 32 "POSITION_TARGET_TYPEMASK_VZ_IGNORE",
        AX_IGNORE = 64 "POSITION_TARGET_TYPEMASK_AX_IGNORE",
        AY_IGNORE = 128 "POSITION_TARGET_TYPEMASK_AY_IGNORE",
        AZ_IGNORE = 256 "POSITION_TARGET_TYPEMASK_AZ_IGNORE",
        FORCE_SET = 512 "POSITION_TARGET_TYPEMASK_FORCE_SET",
        YAW_IGNORE = 1024 "POSITION_TARGET_TYPEMASK_YAW_IGNORE",
        YAW_RATE_IGNORE = 2048 "POSITION_TARGET_TYPEMASK_YAW_RATE_IGNORE",
    }

    /// `MISSION_STATE`: where a mission stands, in the state field of a
    /// [`MissionCurrent`](super::MissionCurrent).
    mission_state "MISSION_STATE" u8 {
        UNKNOWN = 0 "MISSION_STATE_UNKNOWN",
        NO_MISSION = 1 "MISSION_STATE_NO_MISSION",
        NOT_STARTED = 2 "MISSION_STATE_NOT_STARTED",
        ACTIVE = 3 "MISSION_STATE_ACTIVE",
        PAUSED = 4 "MISSION_STATE_PAUSED",
        COMPLETE = 5 "MISSION_STATE_COMPLETE",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_name_belongs_to_one_enumeration() {
        let names: Vec<&str> = ENUMS
            .iter()
            .flat_map(|described| described.entries.iter().map(|named| named.name))
            .collect();
        for (index, name) in names.iter().enumerate() {
            assert!(!names[index + 1..].contains(name), "{name} appears twice");
        }
    }

    #[test]
    fn the_constants_and_the_names_agree() {
        assert_eq!(
            entry_value("MAV_TYPE_QUADROTOR"),
            Some(u64::from(mav_type::QUADROTOR))
        );
        assert_eq!(
            entry_value("MAV_SYS_STATUS_SENSOR_3D_GYRO"),
            Some(u64::from(mav_sys_status_sensor::GYRO_3D))
        );
        assert_eq!(
            entry_value("MAV_SYS_STATUS_PREARM_CHECK"),
            Some(u64::from(mav_sys_status_sensor::PREARM_CHECK))
        );
        assert_eq!(
            entry_value("MAV_MISSION_ACCEPTED"),
            Some(u64::from(mav_mission_result::ACCEPTED))
        );
        assert_eq!(
            entry_value("MAV_RESULT_CANCELLED"),
            Some(u64::from(mav_result::CANCELLED))
        );
        assert_eq!(
            entry_value("GPS_FIX_TYPE_3D_FIX"),
            Some(u64::from(gps_fix_type::FIX_3D))
        );
        assert_eq!(
            entry_value("MAV_PROTOCOL_CAPABILITY_MAVLINK2"),
            Some(mav_protocol_capability::MAVLINK2)
        );
    }

    #[test]
    fn a_value_is_named_by_its_enumeration() {
        let state = enum_named("MAV_STATE").expect("MAV_STATE");
        assert!(!state.bitmask);
        assert_eq!(state.entry(3), Some("MAV_STATE_STANDBY"));
        assert_eq!(state.names(3).collect::<Vec<_>>(), ["MAV_STATE_STANDBY"]);
        assert_eq!(state.entry(200), None);
        assert_eq!(state.names(200).count(), 0);
        assert!(enum_named("MAV_STATES").is_none());
    }

    #[test]
    fn a_bitmask_is_named_bit_by_bit() {
        let sensors = enum_named("MAV_SYS_STATUS_SENSOR").expect("MAV_SYS_STATUS_SENSOR");
        assert!(sensors.bitmask);
        let health = mav_sys_status_sensor::GYRO_3D
            | mav_sys_status_sensor::GPS
            | mav_sys_status_sensor::PREARM_CHECK;
        assert_eq!(
            sensors.names(u64::from(health)).collect::<Vec<_>>(),
            [
                "MAV_SYS_STATUS_SENSOR_3D_GYRO",
                "MAV_SYS_STATUS_SENSOR_GPS",
                "MAV_SYS_STATUS_PREARM_CHECK"
            ]
        );
        assert_eq!(sensors.names(0).count(), 0);
        assert_eq!(sensors.entry(u64::from(health)), None);
    }

    #[test]
    fn the_dialect_marks_which_enumerations_are_bitmasks() {
        for name in [
            "MAV_MODE_FLAG",
            "MAV_PROTOCOL_CAPABILITY",
            "MAV_SYS_STATUS_SENSOR",
            "MAV_SYS_STATUS_SENSOR_EXTENDED",
            "MAV_BATTERY_FAULT",
            "POSITION_TARGET_TYPEMASK",
        ] {
            assert!(enum_named(name).expect(name).bitmask, "{name}");
        }
        for name in [
            "MAV_TYPE",
            "MAV_STATE",
            "MAV_CMD",
            "MAV_RESULT",
            "MAV_FRAME",
        ] {
            assert!(!enum_named(name).expect(name).bitmask, "{name}");
        }
    }
}
