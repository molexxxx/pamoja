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

use std::sync::Arc;

use napi_derive::napi;
use pamoja_core::{Actuator as CoreActuator, Sensor as CoreSensor};
use pamoja_kit::Pose as CorePose;
use pamoja_sim::{RecordingActuator, Replay as CoreReplay, SimRobot as CoreRobot, SimSensor};
use tokio::sync::Mutex;

use crate::motion::{Pose, Twist};

/// A sensor that invents plausible readings.
#[napi]
pub struct SimulatedSensor {
    inner: Arc<Mutex<SimSensor>>,
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
        Self {
            inner: Arc::new(Mutex::new(sensor)),
        }
    }

    /// Takes the next reading.
    #[napi]
    pub async fn read(&self) -> napi::Result<f64> {
        self.inner
            .lock()
            .await
            .read()
            .await
            .map(f64::from)
            .map_err(to_napi)
    }
}

/// A sensor that reads back a recorded series.
///
/// This is how a caller replays a real capture, so a test asks what the code
/// does with readings that actually happened rather than ones it invented.
#[napi]
pub struct Replay {
    inner: Arc<Mutex<CoreReplay>>,
}

#[napi]
impl Replay {
    /// Creates a replay over a recorded series.
    ///
    /// @param readings - the series to read back.
    /// @param repeating - start again at the beginning once exhausted, rather
    ///   than reporting the replay closed.
    #[napi(constructor)]
    pub fn new(readings: Vec<f64>, repeating: Option<bool>) -> Self {
        let values: Vec<f32> = readings.into_iter().map(|value| value as f32).collect();
        Self {
            inner: Arc::new(Mutex::new(if repeating.unwrap_or(false) {
                CoreReplay::repeating(values)
            } else {
                CoreReplay::new(values)
            })),
        }
    }

    /// Takes the next reading.
    ///
    /// @throws `resource is closed` once a replay that does not repeat has run out.
    #[napi]
    pub async fn read(&self) -> napi::Result<f64> {
        self.inner
            .lock()
            .await
            .read()
            .await
            .map(f64::from)
            .map_err(to_napi)
    }
}

/// An actuator that records every command instead of acting on one.
#[napi]
pub struct RecordingActuatorHandle {
    inner: Arc<Mutex<RecordingActuator<f32>>>,
}

#[napi]
impl RecordingActuatorHandle {
    /// Creates an actuator with nothing recorded yet.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RecordingActuator::new())),
        }
    }

    /// Applies a command, which is recorded rather than acted on.
    #[napi]
    pub async fn apply(&self, command: f64) -> napi::Result<()> {
        self.inner
            .lock()
            .await
            .apply(command as f32)
            .await
            .map_err(to_napi)
    }

    /// The commands recorded so far, oldest first.
    #[napi]
    pub async fn commands(&self) -> Vec<f64> {
        self.inner
            .lock()
            .await
            .log()
            .commands()
            .into_iter()
            .map(f64::from)
            .collect()
    }

    /// How many commands have been recorded.
    #[napi]
    pub async fn length(&self) -> u32 {
        self.inner.lock().await.log().len() as u32
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
    inner: Arc<Mutex<CoreRobot>>,
}

#[napi]
impl SimulatedRobot {
    /// Creates a robot at a starting pose.
    ///
    /// @param dt - the seconds each command advances it.
    /// @param start - where it begins, defaulting to the origin.
    #[napi(constructor)]
    pub fn new(dt: f64, start: Option<Pose>) -> Self {
        let start = start.map(CorePose::from).unwrap_or_else(CorePose::origin);
        Self {
            inner: Arc::new(Mutex::new(CoreRobot::starting_at(start, dt as f32))),
        }
    }

    /// Drives the robot for one time step.
    #[napi]
    pub async fn apply(&self, command: Twist) -> napi::Result<()> {
        self.inner
            .lock()
            .await
            .apply(command.into())
            .await
            .map_err(to_napi)
    }

    /// Where the robot has got to.
    #[napi]
    pub async fn pose(&self) -> Pose {
        self.inner.lock().await.pose().into()
    }
}

/// Maps a core error onto the one JavaScript sees.
fn to_napi(error: pamoja_core::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
