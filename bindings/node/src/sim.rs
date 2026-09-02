//! Generated Node bindings for simulated devices.
//!
//! These mirror the `pamoja-sim` Rust API so a caller can drive a whole node
//! with no hardware attached. That is worth more from a binding than from Rust:
//! someone writing against the SDK in JavaScript can put a sensor, an actuator,
//! and a lossy link into a unit test and find out what their code does when a
//! reading drifts or a packet vanishes, without owning the device.
//!
//! The degraded link lives with the transports, as `Transport.degraded`, because
//! it wraps a transport rather than standing alone.

use napi_derive::napi;
use pamoja_core::{Actuator as CoreActuator, Sensor as CoreSensor};
use pamoja_kit::{Pose as CorePose, Twist as CoreTwist};
use pamoja_sim::{Replay as CoreReplay, RecordingActuator, SimRobot as CoreRobot, SimSensor};

/// Where a robot is and which way it faces.
#[napi(object)]
pub struct Pose {
    /// Position along the world x axis, in metres.
    pub x: f64,
    /// Position along the world y axis, in metres.
    pub y: f64,
    /// Heading from the world x axis, in radians, positive counter-clockwise.
    pub theta: f64,
}

/// How fast a robot is asked to move.
#[napi(object)]
pub struct Twist {
    /// Forward speed along the x axis.
    pub vx: f64,
    /// Leftward speed along the y axis; zero for drives that cannot strafe.
    pub vy: f64,
    /// Yaw rate about the z axis, positive counter-clockwise.
    pub omega: f64,
}

/// A sensor that invents plausible readings.
#[napi]
pub struct SimulatedSensor {
    inner: SimSensor,
}

#[napi]
impl SimulatedSensor {
    /// Creates a sensor that reads around a baseline.
    ///
    /// @param baseline - the value it reads before drift and noise.
    /// @param driftPerRead - how much the baseline moves each read.
    /// @param noise - the amplitude of the wobble around it.
    /// @param seed - the seed for that wobble, so a run repeats.
    #[napi(constructor)]
    pub fn new(
        baseline: f64,
        drift_per_read: Option<f64>,
        noise: Option<f64>,
        seed: Option<u32>,
    ) -> Self {
        let mut sensor = SimSensor::new(baseline as f32);
        if let Some(drift) = drift_per_read {
            sensor = sensor.with_drift(drift as f32);
        }
        if let Some(noise) = noise {
            sensor = sensor.with_noise(noise as f32);
        }
        if let Some(seed) = seed {
            sensor = sensor.with_seed(seed);
        }
        Self { inner: sensor }
    }

    /// Takes the next reading.
    #[napi]
    pub async unsafe fn read(&mut self) -> napi::Result<f64> {
        self.inner
            .read()
            .await
            .map(f64::from)
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }
}

/// A sensor that reads back a recorded series.
///
/// This is how a caller replays a real capture, so a test asks what the code
/// does with readings that actually happened rather than ones it invented.
#[napi]
pub struct Replay {
    inner: CoreReplay,
}

#[napi]
impl Replay {
    /// Creates a replay over a recorded series.
    ///
    /// @param readings - the series to read back.
    /// @param repeating - start again at the beginning once exhausted, rather
    ///   than holding the last reading.
    #[napi(constructor)]
    pub fn new(readings: Vec<f64>, repeating: Option<bool>) -> Self {
        let values: Vec<f32> = readings.into_iter().map(|value| value as f32).collect();
        Self {
            inner: if repeating.unwrap_or(false) {
                CoreReplay::repeating(values)
            } else {
                CoreReplay::new(values)
            },
        }
    }

    /// Takes the next reading.
    #[napi]
    pub async unsafe fn read(&mut self) -> napi::Result<f64> {
        self.inner
            .read()
            .await
            .map(f64::from)
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }
}

/// An actuator that records every command instead of acting on one.
#[napi]
pub struct RecordingActuatorHandle {
    inner: RecordingActuator<f32>,
}

#[napi]
impl RecordingActuatorHandle {
    /// Creates an actuator with nothing recorded yet.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RecordingActuator::new(),
        }
    }

    /// Applies a command, which is recorded rather than acted on.
    #[napi]
    pub async unsafe fn apply(&mut self, command: f64) -> napi::Result<()> {
        self.inner
            .apply(command as f32)
            .await
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// The commands recorded so far, oldest first.
    #[napi(getter)]
    pub fn commands(&self) -> Vec<f64> {
        self.inner.log().commands().into_iter().map(f64::from).collect()
    }

    /// How many commands have been recorded.
    #[napi(getter)]
    pub fn length(&self) -> u32 {
        self.inner.log().len() as u32
    }
}

impl Default for RecordingActuatorHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// A robot that moves only in arithmetic.
#[napi]
pub struct SimulatedRobot {
    inner: CoreRobot,
}

#[napi]
impl SimulatedRobot {
    /// Creates a robot at a starting pose.
    ///
    /// @param dt - the seconds each command advances it.
    /// @param start - where it begins, defaulting to the origin.
    #[napi(constructor)]
    pub fn new(dt: f64, start: Option<Pose>) -> Self {
        let pose = start.unwrap_or(Pose {
            x: 0.0,
            y: 0.0,
            theta: 0.0,
        });
        Self {
            inner: CoreRobot::starting_at(
                CorePose::new(pose.x as f32, pose.y as f32, pose.theta as f32),
                dt as f32,
            ),
        }
    }

    /// Drives the robot for one time step.
    #[napi]
    pub async unsafe fn apply(&mut self, command: Twist) -> napi::Result<()> {
        let twist = CoreTwist::new(
            command.vx as f32,
            command.vy as f32,
            command.omega as f32,
        );
        self.inner
            .apply(twist)
            .await
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// Where the robot has got to.
    #[napi(getter)]
    pub fn pose(&self) -> Pose {
        let pose = self.inner.pose();
        Pose {
            x: f64::from(pose.x),
            y: f64::from(pose.y),
            theta: f64::from(pose.theta),
        }
    }
}
