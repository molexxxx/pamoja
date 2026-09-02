//! Generated Python bindings for simulated devices.
//!
//! These mirror the `pamoja-sim` Rust API so a caller can drive a whole node
//! with no hardware attached. That is worth more from a binding than from Rust:
//! someone writing against the SDK in Python can put a sensor, an actuator, and
//! a lossy link into a unit test and find out what their code does when a
//! reading drifts or a packet vanishes, without owning the device.
//!
//! The degraded link lives with the transports, as `Transport.degraded`, because
//! it wraps a transport rather than standing alone.

use std::sync::Arc;

use pamoja_core::{Actuator, Sensor};
use pamoja_kit::{Pose as CorePose, Twist as CoreTwist};
use pamoja_sim::{RecordingActuator, Replay as CoreReplay, SimRobot, SimSensor};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use tokio::sync::Mutex;

use crate::PamojaError;

/// Where a robot is and which way it faces.
#[gen_stub_pyclass]
#[pyclass]
pub struct Pose {
    /// Position along the world x axis, in metres.
    #[pyo3(get)]
    x: f32,
    /// Position along the world y axis, in metres.
    #[pyo3(get)]
    y: f32,
    /// Heading from the world x axis, in radians, positive counter-clockwise.
    #[pyo3(get)]
    theta: f32,
}

#[gen_stub_pymethods]
#[pymethods]
impl Pose {
    fn __repr__(&self) -> String {
        format!("Pose(x={}, y={}, theta={})", self.x, self.y, self.theta)
    }
}

/// A sensor that invents plausible readings.
#[gen_stub_pyclass]
#[pyclass]
pub struct SimulatedSensor {
    inner: Arc<Mutex<SimSensor>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl SimulatedSensor {
    /// Creates a sensor that reads around a baseline.
    #[new]
    #[pyo3(signature = (baseline, *, drift_per_read=None, noise=None, seed=None))]
    fn new(
        baseline: f32,
        drift_per_read: Option<f32>,
        noise: Option<f32>,
        seed: Option<u32>,
    ) -> Self {
        let mut sensor = SimSensor::new(baseline);
        if let Some(drift) = drift_per_read {
            sensor = sensor.with_drift(drift);
        }
        if let Some(noise) = noise {
            sensor = sensor.with_noise(noise);
        }
        if let Some(seed) = seed {
            sensor = sensor.with_seed(seed);
        }
        Self {
            inner: Arc::new(Mutex::new(sensor)),
        }
    }

    /// Takes the next reading.
    fn read<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut sensor = inner.lock().await;
            sensor.read().await.map_err(to_pyerr)
        })
    }
}

/// A sensor that reads back a recorded series.
///
/// This is how a caller replays a real capture, so a test asks what the code
/// does with readings that actually happened rather than ones it invented.
#[gen_stub_pyclass]
#[pyclass]
pub struct Replay {
    inner: Arc<Mutex<CoreReplay>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Replay {
    /// Creates a replay over a recorded series.
    #[new]
    #[pyo3(signature = (readings, *, repeating=false))]
    fn new(readings: Vec<f32>, repeating: bool) -> Self {
        Self {
            inner: Arc::new(Mutex::new(if repeating {
                CoreReplay::repeating(readings)
            } else {
                CoreReplay::new(readings)
            })),
        }
    }

    /// Takes the next reading.
    fn read<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut replay = inner.lock().await;
            replay.read().await.map_err(to_pyerr)
        })
    }
}

/// An actuator that records every command instead of acting on one.
#[gen_stub_pyclass]
#[pyclass]
pub struct RecordingActuatorHandle {
    inner: Arc<Mutex<RecordingActuator<f32>>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl RecordingActuatorHandle {
    /// Creates an actuator with nothing recorded yet.
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RecordingActuator::new())),
        }
    }

    /// Applies a command, which is recorded rather than acted on.
    fn apply<'py>(&self, py: Python<'py>, command: f32) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut actuator = inner.lock().await;
            actuator.apply(command).await.map_err(to_pyerr)
        })
    }

    /// The commands recorded so far, oldest first.
    fn commands<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let actuator = inner.lock().await;
            Ok(actuator.log().commands())
        })
    }
}

/// A robot that moves only in arithmetic.
#[gen_stub_pyclass]
#[pyclass]
pub struct SimulatedRobot {
    inner: Arc<Mutex<SimRobot>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl SimulatedRobot {
    /// Creates a robot at a starting pose.
    #[new]
    #[pyo3(signature = (dt, *, x=0.0, y=0.0, theta=0.0))]
    fn new(dt: f32, x: f32, y: f32, theta: f32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SimRobot::starting_at(
                CorePose::new(x, y, theta),
                dt,
            ))),
        }
    }

    /// Drives the robot for one time step.
    #[pyo3(signature = (*, vx = 0.0, vy = 0.0, omega = 0.0))]
    fn apply<'py>(
        &self,
        py: Python<'py>,
        vx: f32,
        vy: f32,
        omega: f32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut robot = inner.lock().await;
            robot
                .apply(CoreTwist::new(vx, vy, omega))
                .await
                .map_err(to_pyerr)
        })
    }

    /// Where the robot has got to.
    fn pose<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let robot = inner.lock().await;
            let pose = robot.pose();
            Ok(Pose {
                x: pose.x,
                y: pose.y,
                theta: pose.theta,
            })
        })
    }
}

/// Maps a core error onto the one Python sees.
fn to_pyerr(error: pamoja_core::Error) -> PyErr {
    PamojaError::new_err(error.to_string())
}
