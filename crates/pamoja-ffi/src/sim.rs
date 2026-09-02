//! The C ABI for simulated devices.
//!
//! These functions wrap [`pamoja_sim`] so a caller can drive a whole node with
//! no hardware attached. That is worth more from a binding than from Rust:
//! someone writing against the SDK in Python or C# can put a sensor, an
//! actuator, and a lossy link into a unit test and find out what their code does
//! when a reading drifts or a packet vanishes, without owning the device.
//!
//! The degraded link lives with the transports, in
//! [`pamoja_transport_degraded`](crate::transport::pamoja_transport_degraded),
//! because it wraps a transport rather than standing alone.

use std::ptr;

use pamoja_core::{Actuator, Sensor};
use pamoja_kit::{Pose, Twist};
use pamoja_sim::{RecordingActuator, Replay, SimRobot, SimSensor};

use crate::{runtime, set_last_error, PamojaStatus};

/// Where a robot is and which way it faces.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaPose {
    /// Position along the world x axis, in metres.
    pub x: f32,
    /// Position along the world y axis, in metres.
    pub y: f32,
    /// Heading from the world x axis, in radians, positive counter-clockwise.
    pub theta: f32,
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

/// An opaque handle to a sensor that invents plausible readings.
pub struct PamojaSimSensor {
    inner: SimSensor,
}

/// An opaque handle to a sensor that replays a recorded series.
pub struct PamojaReplay {
    inner: Replay,
}

/// An opaque handle to an actuator that records what it was told to do.
pub struct PamojaRecordingActuator {
    inner: RecordingActuator<f32>,
}

/// An opaque handle to a robot that moves only in arithmetic.
pub struct PamojaSimRobot {
    inner: SimRobot,
}

/// Creates a sensor that reads around a baseline.
///
/// # Arguments
///
/// * `baseline` - the value it reads before drift and noise.
/// * `drift_per_read` - how much the baseline moves each read, or 0 for none.
/// * `noise` - the amplitude of the wobble around it, or 0 for none.
/// * `seed` - the seed for that wobble, so a run repeats.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_sim_sensor_free`].
#[no_mangle]
pub extern "C" fn pamoja_sim_sensor_new(
    baseline: f32,
    drift_per_read: f32,
    noise: f32,
    seed: u32,
) -> *mut PamojaSimSensor {
    let mut sensor = SimSensor::new(baseline);
    if drift_per_read != 0.0 {
        sensor = sensor.with_drift(drift_per_read);
    }
    if noise != 0.0 {
        sensor = sensor.with_noise(noise);
    }
    if seed != 0 {
        sensor = sensor.with_seed(seed);
    }
    Box::into_raw(Box::new(PamojaSimSensor { inner: sensor }))
}

/// Takes the next reading.
///
/// # Arguments
///
/// * `sensor` - the sensor.
/// * `out_reading` - receives the reading.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `sensor` must be a live handle from [`pamoja_sim_sensor_new`] and
/// `out_reading` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sim_sensor_read(
    sensor: *mut PamojaSimSensor,
    out_reading: *mut f32,
) -> PamojaStatus {
    if sensor.is_null() || out_reading.is_null() {
        set_last_error("sensor and out_reading must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match runtime().block_on((*sensor).inner.read()) {
        Ok(reading) => {
            *out_reading = reading;
            PamojaStatus::Ok
        }
        Err(error) => fail(error),
    }
}

/// Releases a simulated sensor handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_sim_sensor_new`] that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sim_sensor_free(sensor: *mut PamojaSimSensor) {
    if !sensor.is_null() {
        drop(Box::from_raw(sensor));
    }
}

/// Creates a sensor that reads back a recorded series.
///
/// This is how a caller replays a real capture, so a test asks what the code
/// does with readings that actually happened rather than ones it invented.
///
/// # Arguments
///
/// * `readings` - the series to read back.
/// * `count` - how many readings `readings` holds.
/// * `repeating` - `true` to start again at the beginning once exhausted,
///   `false` to keep returning the last one.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_replay_free`].
///
/// # Safety
///
/// `readings` must point to at least `count` readable floats, or be null when
/// `count` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_replay_new(
    readings: *const f32,
    count: usize,
    repeating: bool,
) -> *mut PamojaReplay {
    if count != 0 && readings.is_null() {
        set_last_error("readings must not be null when count is non-zero".to_owned());
        return ptr::null_mut();
    }
    let values = if count == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(readings, count).to_vec()
    };
    let inner = if repeating {
        Replay::repeating(values)
    } else {
        Replay::new(values)
    };
    Box::into_raw(Box::new(PamojaReplay { inner }))
}

/// Takes the next reading from a replay.
///
/// # Arguments
///
/// * `replay` - the replay.
/// * `out_reading` - receives the reading.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `replay` must be a live handle from [`pamoja_replay_new`] and `out_reading`
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_replay_read(
    replay: *mut PamojaReplay,
    out_reading: *mut f32,
) -> PamojaStatus {
    if replay.is_null() || out_reading.is_null() {
        set_last_error("replay and out_reading must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match runtime().block_on((*replay).inner.read()) {
        Ok(reading) => {
            *out_reading = reading;
            PamojaStatus::Ok
        }
        Err(error) => fail(error),
    }
}

/// Releases a replay handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `replay` must be a handle from [`pamoja_replay_new`] that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_replay_free(replay: *mut PamojaReplay) {
    if !replay.is_null() {
        drop(Box::from_raw(replay));
    }
}

/// Creates an actuator that records every command instead of acting on one.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_recording_actuator_free`].
#[no_mangle]
pub extern "C" fn pamoja_recording_actuator_new() -> *mut PamojaRecordingActuator {
    Box::into_raw(Box::new(PamojaRecordingActuator {
        inner: RecordingActuator::new(),
    }))
}

/// Applies a command, which the actuator records rather than acts on.
///
/// # Arguments
///
/// * `actuator` - the actuator.
/// * `command` - the value commanded.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `actuator` must be a live handle from [`pamoja_recording_actuator_new`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_recording_actuator_apply(
    actuator: *mut PamojaRecordingActuator,
    command: f32,
) -> PamojaStatus {
    if actuator.is_null() {
        set_last_error("actuator must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match runtime().block_on((*actuator).inner.apply(command)) {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => fail(error),
    }
}

/// Reports how many commands an actuator has been given.
///
/// # Arguments
///
/// * `actuator` - the actuator.
///
/// # Returns
///
/// The number of commands, or 0 if `actuator` is null.
///
/// # Safety
///
/// `actuator` must be a live handle from [`pamoja_recording_actuator_new`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_recording_actuator_len(
    actuator: *const PamojaRecordingActuator,
) -> usize {
    if actuator.is_null() {
        return 0;
    }
    (*actuator).inner.log().len()
}

/// Copies out the commands an actuator recorded, oldest first.
///
/// # Arguments
///
/// * `actuator` - the actuator.
/// * `out_commands` - receives up to `capacity` commands.
/// * `capacity` - how many floats `out_commands` can hold.
///
/// # Returns
///
/// How many commands were written, which is the smaller of `capacity` and the
/// count from [`pamoja_recording_actuator_len`].
///
/// # Safety
///
/// `actuator` must be a live handle, and `out_commands` must point to at least
/// `capacity` writable floats or be null when `capacity` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_recording_actuator_commands(
    actuator: *const PamojaRecordingActuator,
    out_commands: *mut f32,
    capacity: usize,
) -> usize {
    if actuator.is_null() || capacity == 0 || out_commands.is_null() {
        return 0;
    }
    let commands = (*actuator).inner.log().commands();
    let written = commands.len().min(capacity);
    ptr::copy_nonoverlapping(commands.as_ptr(), out_commands, written);
    written
}

/// Releases a recording actuator handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `actuator` must be a handle from [`pamoja_recording_actuator_new`] that has
/// not already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_recording_actuator_free(actuator: *mut PamojaRecordingActuator) {
    if !actuator.is_null() {
        drop(Box::from_raw(actuator));
    }
}

/// Creates a robot that moves only in arithmetic.
///
/// # Arguments
///
/// * `start` - the pose it begins at.
/// * `dt` - the seconds each command advances it; its magnitude is used.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_sim_robot_free`].
#[no_mangle]
pub extern "C" fn pamoja_sim_robot_new(start: PamojaPose, dt: f32) -> *mut PamojaSimRobot {
    Box::into_raw(Box::new(PamojaSimRobot {
        inner: SimRobot::starting_at(Pose::new(start.x, start.y, start.theta), dt),
    }))
}

/// Drives the robot for one time step.
///
/// # Arguments
///
/// * `robot` - the robot.
/// * `command` - the speeds to hold for one step.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `robot` must be a live handle from [`pamoja_sim_robot_new`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_sim_robot_apply(
    robot: *mut PamojaSimRobot,
    command: PamojaTwist,
) -> PamojaStatus {
    if robot.is_null() {
        set_last_error("robot must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let twist = Twist::new(command.vx, command.vy, command.omega);
    match runtime().block_on((*robot).inner.apply(twist)) {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => fail(error),
    }
}

/// Reads where the robot has got to.
///
/// # Arguments
///
/// * `robot` - the robot.
///
/// # Returns
///
/// The pose reached so far, or an all-zero pose if `robot` is null.
///
/// # Safety
///
/// `robot` must be a live handle from [`pamoja_sim_robot_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sim_robot_pose(robot: *const PamojaSimRobot) -> PamojaPose {
    if robot.is_null() {
        return PamojaPose {
            x: 0.0,
            y: 0.0,
            theta: 0.0,
        };
    }
    let pose = (*robot).inner.pose();
    PamojaPose {
        x: pose.x,
        y: pose.y,
        theta: pose.theta,
    }
}

/// Releases a simulated robot handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `robot` must be a handle from [`pamoja_sim_robot_new`] that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sim_robot_free(robot: *mut PamojaSimRobot) {
    if !robot.is_null() {
        drop(Box::from_raw(robot));
    }
}

/// Records an error and maps it onto a status.
fn fail(error: pamoja_core::Error) -> PamojaStatus {
    let status = PamojaStatus::from_error(&error);
    set_last_error(error.to_string());
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seeded_sensor_repeats_its_run() {
        unsafe {
            let first = pamoja_sim_sensor_new(20.0, 0.5, 1.0, 42);
            let second = pamoja_sim_sensor_new(20.0, 0.5, 1.0, 42);

            for _ in 0..5 {
                let (mut a, mut b) = (0.0, 0.0);
                assert_eq!(pamoja_sim_sensor_read(first, &mut a), PamojaStatus::Ok);
                assert_eq!(pamoja_sim_sensor_read(second, &mut b), PamojaStatus::Ok);
                assert_eq!(a, b, "the same seed gives the same readings");
            }

            pamoja_sim_sensor_free(first);
            pamoja_sim_sensor_free(second);
        }
    }

    #[test]
    fn a_replay_reads_back_what_was_recorded() {
        unsafe {
            let readings = [21.0f32, 21.5, 22.0];
            let replay = pamoja_replay_new(readings.as_ptr(), readings.len(), true);

            // Twice around, because it repeats.
            for _ in 0..2 {
                for want in readings {
                    let mut got = 0.0;
                    assert_eq!(pamoja_replay_read(replay, &mut got), PamojaStatus::Ok);
                    assert_eq!(got, want);
                }
            }

            pamoja_replay_free(replay);
        }
    }

    #[test]
    fn an_actuator_records_what_it_was_told() {
        unsafe {
            let actuator = pamoja_recording_actuator_new();
            for command in [0.0f32, 0.5, 1.0] {
                assert_eq!(
                    pamoja_recording_actuator_apply(actuator, command),
                    PamojaStatus::Ok
                );
            }
            assert_eq!(pamoja_recording_actuator_len(actuator), 3);

            let mut commands = [0.0f32; 3];
            assert_eq!(
                pamoja_recording_actuator_commands(actuator, commands.as_mut_ptr(), 3),
                3
            );
            assert_eq!(commands, [0.0, 0.5, 1.0]);

            // A short buffer takes what fits rather than overrunning.
            let mut room_for_one = [0.0f32; 1];
            assert_eq!(
                pamoja_recording_actuator_commands(actuator, room_for_one.as_mut_ptr(), 1),
                1
            );
            assert_eq!(room_for_one, [0.0]);

            pamoja_recording_actuator_free(actuator);
        }
    }

    #[test]
    fn a_robot_driven_forward_ends_up_ahead() {
        unsafe {
            let start = PamojaPose {
                x: 0.0,
                y: 0.0,
                theta: 0.0,
            };
            let robot = pamoja_sim_robot_new(start, 1.0);

            let forward = PamojaTwist {
                vx: 1.0,
                vy: 0.0,
                omega: 0.0,
            };
            assert_eq!(pamoja_sim_robot_apply(robot, forward), PamojaStatus::Ok);

            let pose = pamoja_sim_robot_pose(robot);
            assert!(
                (pose.x - 1.0).abs() < 1e-5,
                "one second at one metre a second"
            );
            assert!(pose.y.abs() < 1e-5);

            pamoja_sim_robot_free(robot);
        }
    }

    #[test]
    fn a_null_handle_is_refused_rather_than_dereferenced() {
        unsafe {
            assert_eq!(
                pamoja_sim_sensor_read(ptr::null_mut(), ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(pamoja_recording_actuator_len(ptr::null()), 0);
            assert_eq!(pamoja_sim_robot_pose(ptr::null()).x, 0.0);
            pamoja_sim_sensor_free(ptr::null_mut());
            pamoja_replay_free(ptr::null_mut());
            pamoja_recording_actuator_free(ptr::null_mut());
            pamoja_sim_robot_free(ptr::null_mut());
        }
    }
}
