//! The typed common-dialect messages.
//!
//! Each is declared with the `message!` macro from its name, id, official `CRC_EXTRA`, and
//! base fields in wire order. The set spans what a ground station and an autopilot exchange
//! to fly a mission: the heartbeat, system status, the command, parameter, and mission
//! protocols, and the core position and attitude telemetry.

use super::{Message, MessageDescriptor};

/// Every typed message's shape, in message-id order.
///
/// This is what [`descriptor`](super::descriptor) and [`descriptor_by_name`](super::descriptor_by_name) search, and what a caller
/// enumerates to discover which messages this build types.
pub const DESCRIPTORS: &[&MessageDescriptor<'static>] = &[
    <Heartbeat as Message>::DESCRIPTOR,
    <SysStatus as Message>::DESCRIPTOR,
    <SystemTime as Message>::DESCRIPTOR,
    <Ping as Message>::DESCRIPTOR,
    <SetMode as Message>::DESCRIPTOR,
    <ParamRequestRead as Message>::DESCRIPTOR,
    <ParamRequestList as Message>::DESCRIPTOR,
    <ParamValue as Message>::DESCRIPTOR,
    <ParamSet as Message>::DESCRIPTOR,
    <GpsRawInt as Message>::DESCRIPTOR,
    <Attitude as Message>::DESCRIPTOR,
    <AttitudeQuaternion as Message>::DESCRIPTOR,
    <LocalPositionNed as Message>::DESCRIPTOR,
    <GlobalPositionInt as Message>::DESCRIPTOR,
    <ServoOutputRaw as Message>::DESCRIPTOR,
    <MissionRequest as Message>::DESCRIPTOR,
    <MissionCurrent as Message>::DESCRIPTOR,
    <MissionRequestList as Message>::DESCRIPTOR,
    <MissionCount as Message>::DESCRIPTOR,
    <MissionClearAll as Message>::DESCRIPTOR,
    <MissionAck as Message>::DESCRIPTOR,
    <MissionRequestInt as Message>::DESCRIPTOR,
    <RcChannels as Message>::DESCRIPTOR,
    <ManualControl as Message>::DESCRIPTOR,
    <MissionItemInt as Message>::DESCRIPTOR,
    <VfrHud as Message>::DESCRIPTOR,
    <CommandInt as Message>::DESCRIPTOR,
    <CommandLong as Message>::DESCRIPTOR,
    <CommandAck as Message>::DESCRIPTOR,
    <SetPositionTargetLocalNed as Message>::DESCRIPTOR,
    <SetPositionTargetGlobalInt as Message>::DESCRIPTOR,
    <BatteryStatus as Message>::DESCRIPTOR,
    <AutopilotVersion as Message>::DESCRIPTOR,
    <HomePosition as Message>::DESCRIPTOR,
    <ExtendedSysState as Message>::DESCRIPTOR,
    <Statustext as Message>::DESCRIPTOR,
];

message! {
    /// `HEARTBEAT`: the periodic broadcast every MAVLink node sends to announce its type,
    /// autopilot, and status, and the frame a peer waits for before deciding a link is up.
    Heartbeat = 0, crc = 50, name = "HEARTBEAT";
    custom_mode: u32,
    type_: u8,
    autopilot: u8,
    base_mode: u8,
    system_status: u8,
    mavlink_version: u8,
}

message! {
    /// `SYS_STATUS`: the vehicle's onboard sensor health, load, and battery state.
    SysStatus = 1, crc = 124, name = "SYS_STATUS";
    onboard_control_sensors_present: u32,
    onboard_control_sensors_enabled: u32,
    onboard_control_sensors_health: u32,
    load: u16,
    voltage_battery: u16,
    current_battery: i16,
    drop_rate_comm: u16,
    errors_comm: u16,
    errors_count1: u16,
    errors_count2: u16,
    errors_count3: u16,
    errors_count4: u16,
    battery_remaining: i8,
}

message! {
    /// `SYSTEM_TIME`: the vehicle's Unix and boot time, used to align clocks across a link.
    SystemTime = 2, crc = 137, name = "SYSTEM_TIME";
    time_unix_usec: u64,
    time_boot_ms: u32,
}

message! {
    /// `PING`: a round-trip timing and reachability probe.
    Ping = 4, crc = 237, name = "PING";
    time_usec: u64,
    seq: u32,
    target_system: u8,
    target_component: u8,
}

message! {
    /// `ATTITUDE`: the vehicle's orientation and angular rates in radians.
    Attitude = 30, crc = 39, name = "ATTITUDE";
    time_boot_ms: u32,
    roll: f32,
    pitch: f32,
    yaw: f32,
    rollspeed: f32,
    pitchspeed: f32,
    yawspeed: f32,
}

message! {
    /// `GLOBAL_POSITION_INT`: the fused global position, altitude, and velocity.
    GlobalPositionInt = 33, crc = 104, name = "GLOBAL_POSITION_INT";
    time_boot_ms: u32,
    lat: i32,
    lon: i32,
    alt: i32,
    relative_alt: i32,
    vx: i16,
    vy: i16,
    vz: i16,
    hdg: u16,
}

message! {
    /// `COMMAND_LONG`: a command for the vehicle to run, with up to seven float parameters.
    CommandLong = 76, crc = 152, name = "COMMAND_LONG";
    param1: f32,
    param2: f32,
    param3: f32,
    param4: f32,
    param5: f32,
    param6: f32,
    param7: f32,
    command: u16,
    target_system: u8,
    target_component: u8,
    confirmation: u8,
}

message! {
    /// `COMMAND_ACK`: the vehicle's acknowledgement of a command, carrying the result.
    ///
    /// The extension fields report progress for a long-running command
    /// ([`mav_result::IN_PROGRESS`](crate::dialect::mav_result::IN_PROGRESS)), a
    /// command-specific second result value, and which system and component the
    /// acknowledgement is addressed to.
    CommandAck = 77, crc = 143, name = "COMMAND_ACK";
    command: u16,
    result: u8;
    ext {
        progress: u8,
        result_param2: i32,
        target_system: u8,
        target_component: u8,
    }
}

message! {
    /// `COMMAND_INT`: a command in a coordinate frame, with integer-encoded position.
    CommandInt = 75, crc = 158, name = "COMMAND_INT";
    param1: f32,
    param2: f32,
    param3: f32,
    param4: f32,
    x: i32,
    y: i32,
    z: f32,
    command: u16,
    target_system: u8,
    target_component: u8,
    frame: u8,
    current: u8,
    autocontinue: u8,
}

message! {
    /// `SET_MODE`: requests the vehicle switch base and custom flight modes.
    SetMode = 11, crc = 89, name = "SET_MODE";
    custom_mode: u32,
    target_system: u8,
    base_mode: u8,
}

message! {
    /// `MANUAL_CONTROL`: a pilot's stick and button input, normalized to a fixed range.
    ManualControl = 69, crc = 243, name = "MANUAL_CONTROL";
    x: i16,
    y: i16,
    z: i16,
    r: i16,
    buttons: u16,
    target: u8,
}

message! {
    /// `PARAM_REQUEST_READ`: asks the vehicle for one onboard parameter, by name or index.
    ParamRequestRead = 20, crc = 214, name = "PARAM_REQUEST_READ";
    param_index: i16,
    target_system: u8,
    target_component: u8,
    param_id: [char; 16],
}

message! {
    /// `PARAM_REQUEST_LIST`: asks the vehicle to stream all of its onboard parameters.
    ParamRequestList = 21, crc = 159, name = "PARAM_REQUEST_LIST";
    target_system: u8,
    target_component: u8,
}

message! {
    /// `PARAM_VALUE`: one onboard parameter's value, type, and position in the set.
    ParamValue = 22, crc = 220, name = "PARAM_VALUE";
    param_value: f32,
    param_count: u16,
    param_index: u16,
    param_id: [char; 16],
    param_type: u8,
}

message! {
    /// `PARAM_SET`: sets one onboard parameter on the vehicle.
    ParamSet = 23, crc = 168, name = "PARAM_SET";
    param_value: f32,
    target_system: u8,
    target_component: u8,
    param_id: [char; 16],
    param_type: u8,
}

message! {
    /// `GPS_RAW_INT`: the raw GPS fix: position, accuracy, speed, and satellite count.
    GpsRawInt = 24, crc = 24, name = "GPS_RAW_INT";
    time_usec: u64,
    lat: i32,
    lon: i32,
    alt: i32,
    eph: u16,
    epv: u16,
    vel: u16,
    cog: u16,
    fix_type: u8,
    satellites_visible: u8,
}

message! {
    /// `ATTITUDE_QUATERNION`: orientation as a quaternion, with angular rates.
    AttitudeQuaternion = 31, crc = 246, name = "ATTITUDE_QUATERNION";
    time_boot_ms: u32,
    q1: f32,
    q2: f32,
    q3: f32,
    q4: f32,
    rollspeed: f32,
    pitchspeed: f32,
    yawspeed: f32,
}

message! {
    /// `LOCAL_POSITION_NED`: the vehicle's local position and velocity in the NED frame.
    LocalPositionNed = 32, crc = 185, name = "LOCAL_POSITION_NED";
    time_boot_ms: u32,
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    vz: f32,
}

message! {
    /// `SERVO_OUTPUT_RAW`: the raw PWM values driving the first eight servo outputs.
    ServoOutputRaw = 36, crc = 222, name = "SERVO_OUTPUT_RAW";
    time_usec: u32,
    servo1_raw: u16,
    servo2_raw: u16,
    servo3_raw: u16,
    servo4_raw: u16,
    servo5_raw: u16,
    servo6_raw: u16,
    servo7_raw: u16,
    servo8_raw: u16,
    port: u8,
}

message! {
    /// `RC_CHANNELS`: the raw values of up to eighteen RC input channels.
    RcChannels = 65, crc = 118, name = "RC_CHANNELS";
    time_boot_ms: u32,
    chan1_raw: u16,
    chan2_raw: u16,
    chan3_raw: u16,
    chan4_raw: u16,
    chan5_raw: u16,
    chan6_raw: u16,
    chan7_raw: u16,
    chan8_raw: u16,
    chan9_raw: u16,
    chan10_raw: u16,
    chan11_raw: u16,
    chan12_raw: u16,
    chan13_raw: u16,
    chan14_raw: u16,
    chan15_raw: u16,
    chan16_raw: u16,
    chan17_raw: u16,
    chan18_raw: u16,
    chancount: u8,
    rssi: u8,
}

message! {
    /// `VFR_HUD`: the heads-up flight summary: speeds, heading, throttle, altitude, climb.
    VfrHud = 74, crc = 20, name = "VFR_HUD";
    airspeed: f32,
    groundspeed: f32,
    alt: f32,
    climb: f32,
    heading: i16,
    throttle: u16,
}

message! {
    /// `BATTERY_STATUS`: a battery's charge, current, temperature, and per-cell voltages.
    BatteryStatus = 147, crc = 154, name = "BATTERY_STATUS";
    current_consumed: i32,
    energy_consumed: i32,
    temperature: i16,
    voltages: [u16; 10],
    current_battery: i16,
    id: u8,
    battery_function: u8,
    type_: u8,
    battery_remaining: i8,
}

message! {
    /// `MISSION_REQUEST_LIST`: asks the vehicle to begin downloading a plan by reporting its
    /// [`MissionCount`].
    MissionRequestList = 43, crc = 132, name = "MISSION_REQUEST_LIST";
    target_system: u8,
    target_component: u8;
    ext { mission_type: u8 }
}

message! {
    /// `MISSION_COUNT`: announces how many items a mission transfer will contain, and which
    /// plan (mission, geofence, or rally) the transfer carries.
    MissionCount = 44, crc = 221, name = "MISSION_COUNT";
    count: u16,
    target_system: u8,
    target_component: u8;
    ext { mission_type: u8, opaque_id: u32 }
}

message! {
    /// `MISSION_REQUEST_INT`: requests one mission item by sequence number, with position
    /// returned integer-encoded.
    MissionRequestInt = 51, crc = 196, name = "MISSION_REQUEST_INT";
    seq: u16,
    target_system: u8,
    target_component: u8;
    ext { mission_type: u8 }
}

message! {
    /// `MISSION_REQUEST`: requests one mission item by sequence number, the legacy request
    /// some autopilots still send in place of [`MissionRequestInt`].
    MissionRequest = 40, crc = 230, name = "MISSION_REQUEST";
    seq: u16,
    target_system: u8,
    target_component: u8;
    ext { mission_type: u8 }
}

message! {
    /// `MISSION_ITEM_INT`: one mission item, with integer-encoded position.
    MissionItemInt = 73, crc = 38, name = "MISSION_ITEM_INT";
    param1: f32,
    param2: f32,
    param3: f32,
    param4: f32,
    x: i32,
    y: i32,
    z: f32,
    seq: u16,
    command: u16,
    target_system: u8,
    target_component: u8,
    frame: u8,
    current: u8,
    autocontinue: u8;
    ext { mission_type: u8 }
}

message! {
    /// `MISSION_CURRENT`: the sequence number of the mission item the vehicle is running.
    MissionCurrent = 42, crc = 28, name = "MISSION_CURRENT";
    seq: u16,
}

message! {
    /// `MISSION_CLEAR_ALL`: asks the vehicle to erase a stored plan.
    MissionClearAll = 45, crc = 232, name = "MISSION_CLEAR_ALL";
    target_system: u8,
    target_component: u8;
    ext { mission_type: u8 }
}

message! {
    /// `MISSION_ACK`: acknowledges the end of a mission transfer with a result.
    MissionAck = 47, crc = 153, name = "MISSION_ACK";
    target_system: u8,
    target_component: u8,
    type_: u8;
    ext { mission_type: u8, opaque_id: u32 }
}

message! {
    /// `SET_POSITION_TARGET_LOCAL_NED`: an offboard position, velocity, or thrust setpoint
    /// in the local NED frame.
    SetPositionTargetLocalNed = 84, crc = 143, name = "SET_POSITION_TARGET_LOCAL_NED";
    time_boot_ms: u32,
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    afx: f32,
    afy: f32,
    afz: f32,
    yaw: f32,
    yaw_rate: f32,
    type_mask: u16,
    target_system: u8,
    target_component: u8,
    coordinate_frame: u8,
}

message! {
    /// `SET_POSITION_TARGET_GLOBAL_INT`: an offboard setpoint in the global frame, with
    /// integer-encoded latitude and longitude.
    SetPositionTargetGlobalInt = 86, crc = 5, name = "SET_POSITION_TARGET_GLOBAL_INT";
    time_boot_ms: u32,
    lat_int: i32,
    lon_int: i32,
    alt: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    afx: f32,
    afy: f32,
    afz: f32,
    yaw: f32,
    yaw_rate: f32,
    type_mask: u16,
    target_system: u8,
    target_component: u8,
    coordinate_frame: u8,
}

message! {
    /// `HOME_POSITION`: the vehicle's home location and the approach vector to it.
    HomePosition = 242, crc = 104, name = "HOME_POSITION";
    latitude: i32,
    longitude: i32,
    altitude: i32,
    x: f32,
    y: f32,
    z: f32,
    q: [f32; 4],
    approach_x: f32,
    approach_y: f32,
    approach_z: f32,
}

message! {
    /// `AUTOPILOT_VERSION`: the autopilot's capability flags and firmware versions.
    AutopilotVersion = 148, crc = 178, name = "AUTOPILOT_VERSION";
    capabilities: u64,
    uid: u64,
    flight_sw_version: u32,
    middleware_sw_version: u32,
    os_sw_version: u32,
    board_version: u32,
    vendor_id: u16,
    product_id: u16,
    flight_custom_version: [u8; 8],
    middleware_custom_version: [u8; 8],
    os_custom_version: [u8; 8],
}

message! {
    /// `EXTENDED_SYS_STATE`: the VTOL and landed state of the vehicle.
    ExtendedSysState = 245, crc = 130, name = "EXTENDED_SYS_STATE";
    vtol_state: u8,
    landed_state: u8,
}

message! {
    /// `STATUSTEXT`: a human-readable status message with a severity.
    Statustext = 253, crc = 83, name = "STATUSTEXT";
    severity: u8,
    text: [char; 50],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::{crc_extra, Message};
    use crate::message_crc_extra;

    // Re-derives a message's CRC_EXTRA from its field definitions and checks it against the
    // official dialect value the message declares and the registry returns. A wrong field
    // type, name, or order changes the derived seed, so this catches it.
    fn verify<M: Message>() {
        assert_eq!(
            message_crc_extra(M::NAME, M::BASE_FIELDS),
            M::CRC_EXTRA,
            "derived CRC_EXTRA disagrees with the dialect for {}",
            M::NAME
        );
        assert_eq!(
            crc_extra(M::ID),
            Some(M::CRC_EXTRA),
            "registry disagrees for {}",
            M::NAME
        );
    }

    #[test]
    fn every_message_crc_extra_matches_the_dialect() {
        verify::<Heartbeat>();
        verify::<SysStatus>();
        verify::<SystemTime>();
        verify::<Ping>();
        verify::<Attitude>();
        verify::<GlobalPositionInt>();
        verify::<CommandLong>();
        verify::<CommandAck>();
        verify::<CommandInt>();
        verify::<SetMode>();
        verify::<ManualControl>();
        verify::<ParamRequestRead>();
        verify::<ParamRequestList>();
        verify::<ParamValue>();
        verify::<ParamSet>();
        verify::<GpsRawInt>();
        verify::<AttitudeQuaternion>();
        verify::<LocalPositionNed>();
        verify::<ServoOutputRaw>();
        verify::<RcChannels>();
        verify::<VfrHud>();
        verify::<BatteryStatus>();
        verify::<MissionRequestList>();
        verify::<MissionCount>();
        verify::<MissionRequestInt>();
        verify::<MissionRequest>();
        verify::<MissionItemInt>();
        verify::<MissionCurrent>();
        verify::<MissionClearAll>();
        verify::<MissionAck>();
        verify::<SetPositionTargetLocalNed>();
        verify::<SetPositionTargetGlobalInt>();
        verify::<HomePosition>();
        verify::<AutopilotVersion>();
        verify::<ExtendedSysState>();
        verify::<Statustext>();
    }

    #[test]
    fn a_heartbeat_encodes_to_the_documented_frame() {
        // A published worked example of a v1 HEARTBEAT on the wire (custom_mode 0, a
        // quadrotor running ArduPilot, base_mode 0x51, status active). Encoding the same
        // message must reproduce it byte for byte, checksum included, which anchors the
        // whole v1 frame layout to an external reference rather than a round-trip.
        let heartbeat = Heartbeat {
            custom_mode: 0,
            type_: 2,
            autopilot: 3,
            base_mode: 0x51,
            system_status: 4,
            mavlink_version: 3,
        };
        let mut payload = [0u8; 255];
        let len = heartbeat.encode(&mut payload);
        let frame = crate::Frame::encode_v1(
            crate::Header::new(1, 1, 0x4E),
            Heartbeat::ID,
            &payload[..len],
            Heartbeat::CRC_EXTRA,
        )
        .unwrap();
        let expected = [
            0xFE, 0x09, 0x4E, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x03, 0x51, 0x04,
            0x03, 0x1C, 0x7F,
        ];
        assert_eq!(frame.as_bytes(), &expected);
    }

    #[test]
    fn a_heartbeat_round_trips_through_its_payload() {
        let heartbeat = Heartbeat {
            custom_mode: 0x0A0B0C0D,
            type_: 2,
            autopilot: 12,
            base_mode: 0x81,
            system_status: 4,
            mavlink_version: 3,
        };
        let mut buf = [0u8; 255];
        let len = heartbeat.encode(&mut buf);
        assert_eq!(len, Heartbeat::WIRE_LEN);
        assert_eq!(Heartbeat::decode(&buf[..len]).unwrap(), heartbeat);
    }

    #[test]
    fn a_command_long_round_trips_through_field_reordering() {
        // COMMAND_LONG mixes 4-, 2-, and 1-byte fields, so this exercises the wire
        // reordering the declaration encodes (floats first, then the command, then the bytes).
        let command = CommandLong {
            param1: 1.0,
            param2: 0.0,
            param3: 0.0,
            param4: 0.0,
            param5: 12.34,
            param6: 56.78,
            param7: 100.0,
            command: 400, // MAV_CMD_COMPONENT_ARM_DISARM
            target_system: 1,
            target_component: 1,
            confirmation: 0,
        };
        let mut buf = [0u8; 255];
        let len = command.encode(&mut buf);
        assert_eq!(len, CommandLong::WIRE_LEN);
        assert_eq!(CommandLong::decode(&buf[..len]).unwrap(), command);
    }

    #[test]
    fn an_extension_field_round_trips_after_the_base_fields() {
        // MISSION_COUNT gained the `mission_type` and `opaque_id` extension fields; a full
        // encode carries them after the base fields and decodes them back.
        let count = MissionCount {
            count: 7,
            target_system: 1,
            target_component: 1,
            mission_type: crate::dialect::mav_mission_type::FENCE,
            opaque_id: 0xDEAD_BEEF,
        };
        let mut buf = [0u8; 255];
        let len = count.encode(&mut buf);
        assert_eq!(len, MissionCount::WIRE_LEN);
        // Base fields (count, targets) occupy four bytes; the extensions follow.
        assert_eq!(len, 4 + 1 + 4);
        assert_eq!(MissionCount::decode(&buf[..len]).unwrap(), count);
    }

    #[test]
    fn a_base_only_payload_decodes_extensions_as_zero() {
        // A peer that sends only the base fields (a MAVLink 1 sender, or truncated MAVLink 2)
        // still decodes; the extensions read as zero, which is what keeps a message that gains
        // extensions compatible with one that predates them.
        let base_only = [7u8, 0, 1, 1]; // count = 7, target_system = 1, target_component = 1
        let decoded = MissionCount::decode(&base_only).unwrap();
        assert_eq!(decoded.count, 7);
        assert_eq!(decoded.mission_type, 0);
        assert_eq!(decoded.opaque_id, 0);
    }

    #[test]
    fn command_ack_keeps_its_seed_after_gaining_extensions() {
        // The extensions are excluded from CRC_EXTRA, so COMMAND_ACK's seed is still the
        // pre-extension value a peer that only knows the base fields computes.
        assert_eq!(
            message_crc_extra("COMMAND_ACK", CommandAck::BASE_FIELDS),
            CommandAck::CRC_EXTRA
        );
        assert_eq!(CommandAck::CRC_EXTRA, 143);
        // The base fields drive the seed; the extensions are not among them.
        assert_eq!(CommandAck::BASE_FIELDS.len(), 2);
    }

    #[test]
    fn decode_zero_extends_a_truncated_payload() {
        // A HEARTBEAT whose trailing bytes were truncated to a single zero still decodes,
        // with the missing fields read as zero.
        let decoded = Heartbeat::decode(&[0]).unwrap();
        assert_eq!(decoded, Heartbeat::default_zeroed());
    }

    impl Heartbeat {
        fn default_zeroed() -> Self {
            Heartbeat {
                custom_mode: 0,
                type_: 0,
                autopilot: 0,
                base_mode: 0,
                system_status: 0,
                mavlink_version: 0,
            }
        }
    }
}
