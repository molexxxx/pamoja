//! Generated Python bindings for the goal-named helper math.
//!
//! These mirror the `pamoja-kit` Rust API one-to-one. The helpers are synchronous
//! pure math, so every method here returns its value directly; the ones that
//! answer "maybe" return `None` rather than raising, because having no answer yet
//! is an ordinary state and not a failure.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_kit::{
    deadband as core_deadband, Anomaly as CoreAnomaly, Boundary, Calibration as CoreCalibration,
    Coordinate, Debounce as CoreDebounce, Depletion as CoreDepletion, Geofence as CoreGeofence,
    Kalman as CoreKalman, Median as CoreMedian, Pid as CorePid, Ramp as CoreRamp,
    Smoother as CoreSmoother, Surge as CoreSurge, Thermostat as CoreThermostat, Trend as CoreTrend,
    Window as CoreWindow,
};

/// Names the boundary state a geofence reports, as a plain string.
fn boundary_name(value: Boundary) -> &'static str {
    match value {
        Boundary::Inside => "Inside",
        Boundary::Outside => "Outside",
        Boundary::Exited => "Exited",
        Boundary::Entered => "Entered",
    }
}

/// Smooths a noisy reading by weighting each new sample against the running value.
#[gen_stub_pyclass]
#[pyclass]
pub struct Smoother {
    inner: CoreSmoother,
}

#[gen_stub_pymethods]
#[pymethods]
impl Smoother {
    /// Creates a smoother whose `weight` sets how much each new sample counts.
    #[new]
    fn new(weight: f32) -> Self {
        Self {
            inner: CoreSmoother::new(weight),
        }
    }

    /// Folds a sample in and returns the smoothed value.
    fn update(&mut self, sample: f32) -> f32 {
        self.inner.update(sample)
    }

    /// The current value, or `None` before the first sample.
    #[getter]
    fn value(&self) -> Option<f32> {
        self.inner.value()
    }

    /// Clears the smoother back to its initial state.
    fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Holds a value at a setpoint by trading off present, past, and predicted error.
#[gen_stub_pyclass]
#[pyclass]
pub struct Pid {
    inner: CorePid,
}

#[gen_stub_pymethods]
#[pymethods]
impl Pid {
    /// Creates a controller with the given gains, optionally clamping its output.
    #[new]
    #[pyo3(signature = (kp, ki, kd, *, min=None, max=None))]
    fn new(kp: f32, ki: f32, kd: f32, min: Option<f32>, max: Option<f32>) -> Self {
        let mut inner = CorePid::new(kp, ki, kd);
        if let (Some(min), Some(max)) = (min, max) {
            inner = inner.with_limits(min, max);
        }
        Self { inner }
    }

    /// Advances the controller by one step and returns the control output.
    fn update(&mut self, setpoint: f32, measurement: f32, dt: f32) -> f32 {
        self.inner.update(setpoint, measurement, dt)
    }

    /// Clears the accumulated integral and last error.
    fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Switches a load on and off around a setpoint, with hysteresis to stop chatter.
#[gen_stub_pyclass]
#[pyclass]
pub struct Thermostat {
    inner: CoreThermostat,
}

#[gen_stub_pymethods]
#[pymethods]
impl Thermostat {
    /// Creates a cooling thermostat, which switches on when the reading rises.
    #[staticmethod]
    fn cooling(setpoint: f32, hysteresis: f32) -> Self {
        Self {
            inner: CoreThermostat::cooling(setpoint, hysteresis),
        }
    }

    /// Creates a heating thermostat, which switches on when the reading falls.
    #[staticmethod]
    fn heating(setpoint: f32, hysteresis: f32) -> Self {
        Self {
            inner: CoreThermostat::heating(setpoint, hysteresis),
        }
    }

    /// Feeds a reading in and returns whether the load should be on.
    fn update(&mut self, reading: f32) -> bool {
        self.inner.update(reading)
    }

    /// Whether the load should currently be on.
    #[getter]
    fn is_on(&self) -> bool {
        self.inner.is_on()
    }
}

/// Warns before a falling level runs out, by projecting its rate of fall.
#[gen_stub_pyclass]
#[pyclass]
pub struct Depletion {
    inner: CoreDepletion,
}

#[gen_stub_pymethods]
#[pymethods]
impl Depletion {
    /// Creates an estimator that counts down to `threshold`.
    #[new]
    fn new(threshold: f32) -> Self {
        Self {
            inner: CoreDepletion::new(threshold),
        }
    }

    /// Records a level and returns the samples left before the threshold.
    ///
    /// Returns `None` while the level is steady or rising, and on the first
    /// reading, when no rate of fall is known yet.
    fn update(&mut self, level: f32) -> Option<u32> {
        self.inner.update(level)
    }
}

/// Estimates a true value from noisy readings, trusting the model and the sensor
/// in proportion to how noisy each is.
#[gen_stub_pyclass]
#[pyclass]
pub struct Kalman {
    inner: CoreKalman,
}

#[gen_stub_pymethods]
#[pymethods]
impl Kalman {
    /// Creates a filter from the process and measurement noise, and a first guess.
    #[new]
    fn new(process_noise: f32, measurement_noise: f32, initial: f32) -> Self {
        Self {
            inner: CoreKalman::new(process_noise, measurement_noise, initial),
        }
    }

    /// Folds a reading in and returns the new estimate.
    fn update(&mut self, reading: f32) -> f32 {
        self.inner.update(reading)
    }

    /// The current estimate.
    #[getter]
    fn estimate(&self) -> f32 {
        self.inner.estimate()
    }
}

/// Stops a flickering input from acting until it has settled.
#[gen_stub_pyclass]
#[pyclass]
pub struct Debounce {
    inner: CoreDebounce,
}

#[gen_stub_pymethods]
#[pymethods]
impl Debounce {
    /// Creates a debouncer needing `samples` agreeing readings to change state.
    #[new]
    fn new(samples: u16, initial: bool) -> Self {
        Self {
            inner: CoreDebounce::new(samples, initial),
        }
    }

    /// Feeds a raw reading in and returns the settled state.
    fn update(&mut self, raw: bool) -> bool {
        self.inner.update(raw)
    }

    /// The settled state.
    #[getter]
    fn state(&self) -> bool {
        self.inner.state()
    }
}

/// Limits how fast a value may change, so a load is never slammed.
#[gen_stub_pyclass]
#[pyclass]
pub struct Ramp {
    inner: CoreRamp,
}

#[gen_stub_pymethods]
#[pymethods]
impl Ramp {
    /// Creates a limiter starting at `start` and moving at most `max_step` a step.
    #[new]
    fn new(start: f32, max_step: f32) -> Self {
        Self {
            inner: CoreRamp::new(start, max_step),
        }
    }

    /// Moves one step toward `target` and returns the new value.
    fn update(&mut self, target: f32) -> f32 {
        self.inner.update(target)
    }

    /// The current value.
    #[getter]
    fn value(&self) -> f32 {
        self.inner.value()
    }

    /// Forces the value without rate limiting.
    fn set(&mut self, value: f32) {
        self.inner.set(value);
    }
}

/// Notices a step change between successive readings, such as a burst pipe.
#[gen_stub_pyclass]
#[pyclass]
pub struct Surge {
    inner: CoreSurge,
}

#[gen_stub_pymethods]
#[pymethods]
impl Surge {
    /// Creates a detector for rises of at least `limit` between readings.
    #[staticmethod]
    fn rising(limit: f32) -> Self {
        Self {
            inner: CoreSurge::rising(limit),
        }
    }

    /// Creates a detector for falls of at least `limit` between readings.
    #[staticmethod]
    fn falling(limit: f32) -> Self {
        Self {
            inner: CoreSurge::falling(limit),
        }
    }

    /// Feeds a value in and returns the size of a qualifying step, or `None`.
    fn update(&mut self, value: f32) -> Option<f32> {
        self.inner.update(value)
    }
}

/// Turns a raw sensor count into the units the reading is actually in.
#[gen_stub_pyclass]
#[pyclass]
pub struct Calibration {
    inner: CoreCalibration,
}

#[gen_stub_pymethods]
#[pymethods]
impl Calibration {
    /// Creates a calibration applying `raw * scale + offset`.
    #[staticmethod]
    fn linear(scale: f32, offset: f32) -> Self {
        Self {
            inner: CoreCalibration::linear(scale, offset),
        }
    }

    /// Creates a calibration fitted through two known reference points.
    #[staticmethod]
    fn two_point(raw_low: f32, value_low: f32, raw_high: f32, value_high: f32) -> Self {
        Self {
            inner: CoreCalibration::two_point(raw_low, value_low, raw_high, value_high),
        }
    }

    /// Converts a raw reading into calibrated units.
    fn apply(&self, raw: f32) -> f32 {
        self.inner.apply(raw)
    }
}

/// Keeps a tracked point inside an area, and notices when it leaves.
#[gen_stub_pyclass]
#[pyclass]
pub struct Geofence {
    inner: CoreGeofence,
}

#[gen_stub_pymethods]
#[pymethods]
impl Geofence {
    /// Creates a circular fence of `radius_m` metres around a centre fix.
    #[new]
    fn new(latitude: f64, longitude: f64, radius_m: f64) -> Self {
        Self {
            inner: CoreGeofence::new(Coordinate::new(latitude, longitude), radius_m),
        }
    }

    /// Feeds a fix in and names where it sits, including a single crossing.
    ///
    /// Returns one of `"Inside"`, `"Outside"`, `"Exited"`, or `"Entered"`.
    fn update(&mut self, latitude: f64, longitude: f64) -> &'static str {
        boundary_name(self.inner.update(Coordinate::new(latitude, longitude)))
    }

    /// Reports whether a fix lies inside, without recording a crossing.
    fn contains(&self, latitude: f64, longitude: f64) -> bool {
        self.inner.contains(Coordinate::new(latitude, longitude))
    }
}

/// Returns the great-circle distance between two coordinates, in metres.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn distance_between(
    from_latitude: f64,
    from_longitude: f64,
    to_latitude: f64,
    to_longitude: f64,
) -> f64 {
    Coordinate::new(from_latitude, from_longitude)
        .distance_to(Coordinate::new(to_latitude, to_longitude))
}

/// Returns the initial bearing from one coordinate to another, in degrees.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bearing_between(
    from_latitude: f64,
    from_longitude: f64,
    to_latitude: f64,
    to_longitude: f64,
) -> f64 {
    Coordinate::new(from_latitude, from_longitude)
        .bearing_to(Coordinate::new(to_latitude, to_longitude))
}

/// Suppresses movement within `width` of `center`, so noise does not act.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn deadband(value: f32, center: f32, width: f32) -> f32 {
    core_deadband(value, center, width)
}

/// The capacity every windowed helper here is built at.
const CAPACITY: usize = 32;

/// The number of readings a windowed helper keeps.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn window_capacity() -> usize {
    CAPACITY
}

/// A rolling window of the most recent readings, with the stats over them.
#[gen_stub_pyclass]
#[pyclass]
pub struct Window {
    inner: CoreWindow<CAPACITY>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Window {
    /// Creates an empty window.
    #[new]
    fn new() -> Self {
        Self {
            inner: CoreWindow::new(),
        }
    }

    /// Adds a reading, dropping the oldest once the window is full.
    fn push(&mut self, reading: f32) {
        self.inner.push(reading);
    }

    /// How many readings the window holds.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// How many readings the window holds before it starts dropping.
    #[getter]
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// The mean of the readings, or ``None`` while the window is empty.
    fn mean(&self) -> Option<f32> {
        self.inner.mean()
    }

    /// The smallest reading, or ``None`` while the window is empty.
    fn min(&self) -> Option<f32> {
        self.inner.min()
    }

    /// The largest reading, or ``None`` while the window is empty.
    fn max(&self) -> Option<f32> {
        self.inner.max()
    }

    /// The spread between the smallest and largest readings.
    fn range(&self) -> Option<f32> {
        self.inner.range()
    }

    /// The variance of the readings, or ``None`` without enough of them.
    fn variance(&self) -> Option<f32> {
        self.inner.variance()
    }
}

/// Rejects a single wild reading, where an average would let it pull the answer.
#[gen_stub_pyclass]
#[pyclass]
pub struct Median {
    inner: CoreMedian<CAPACITY>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Median {
    /// Creates an empty median filter.
    #[new]
    fn new() -> Self {
        Self {
            inner: CoreMedian::new(),
        }
    }

    /// Folds a reading in and returns the median of the window.
    fn update(&mut self, reading: f32) -> f32 {
        self.inner.update(reading)
    }

    /// The current median, or ``None`` before the first reading.
    #[getter]
    fn value(&self) -> Option<f32> {
        self.inner.median()
    }
}

/// Fits a line through recent readings, so a slow drift shows before it matters.
#[gen_stub_pyclass]
#[pyclass]
pub struct Trend {
    inner: CoreTrend<CAPACITY>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Trend {
    /// Creates an empty trend estimator.
    #[new]
    fn new() -> Self {
        Self {
            inner: CoreTrend::new(),
        }
    }

    /// Adds a reading.
    fn push(&mut self, reading: f32) {
        self.inner.push(reading);
    }

    /// The fitted slope in units per reading, or ``None`` without enough readings.
    #[getter]
    fn slope(&self) -> Option<f32> {
        self.inner.slope()
    }
}

/// Flags a reading that stands out from the ones around it.
#[gen_stub_pyclass]
#[pyclass]
pub struct Anomaly {
    inner: CoreAnomaly<CAPACITY>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Anomaly {
    /// Creates a detector that flags a reading `sigmas` deviations from the mean.
    #[new]
    fn new(sigmas: f32) -> Self {
        Self {
            inner: CoreAnomaly::new(sigmas),
        }
    }

    /// Folds a reading in and reports whether it stands out.
    fn check(&mut self, reading: f32) -> bool {
        self.inner.check(reading)
    }
}
