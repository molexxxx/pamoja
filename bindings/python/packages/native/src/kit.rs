//! Generated Python bindings for the goal-named helper math.
//!
//! These bind the reading and control helpers of `pamoja-kit`; the robotics helpers are
//! Rust only. The helpers are synchronous pure math, so every method here returns its value
//! directly; the ones that answer "maybe" return `None` rather than raising, because having
//! no answer yet is an ordinary state and not a failure. A reading that is not a finite
//! number is ignored by every helper that keeps state, as the Rust crate documents.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_kit::{
    deadband as core_deadband, Anomaly as CoreAnomaly, Boundary, Calibration as CoreCalibration,
    Coordinate, Debounce as CoreDebounce, Depletion as CoreDepletion, Edge,
    Geofence as CoreGeofence, Kalman as CoreKalman, Median as CoreMedian, Pid as CorePid,
    Ramp as CoreRamp, Smoother as CoreSmoother, Surge as CoreSurge, Thermostat as CoreThermostat,
    Trend as CoreTrend, Trigger as CoreTrigger, Window as CoreWindow,
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
    ///
    /// Either limit may be given alone; the side left out is open.
    #[new]
    #[pyo3(signature = (kp, ki, kd, *, min=None, max=None))]
    fn new(kp: f32, ki: f32, kd: f32, min: Option<f32>, max: Option<f32>) -> Self {
        let inner = CorePid::new(kp, ki, kd);
        let inner = if min.is_some() || max.is_some() {
            inner.with_limits(
                min.unwrap_or(f32::NEG_INFINITY),
                max.unwrap_or(f32::INFINITY),
            )
        } else {
            inner
        };
        Self { inner }
    }

    /// Advances the controller by one step and returns the control output.
    fn update(&mut self, setpoint: f32, measurement: f32, dt: f32) -> f32 {
        self.inner.update(setpoint, measurement, dt)
    }

    /// Clears the accumulated integral, the last error, and the last output.
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

    /// Whether the load is on, as the last reading left it.
    #[getter]
    fn is_on(&self) -> bool {
        self.inner.is_on()
    }
}

/// Fires once when a reading crosses a line, and not again until it has come back past
/// the release band.
///
/// `update` answers `"set"` the moment the reading crosses the line, `"cleared"` the
/// moment it comes back past the band, and `None` while nothing changed; the facade
/// wraps the two in its `Edge` enum.
#[gen_stub_pyclass]
#[pyclass]
pub struct Trigger {
    inner: CoreTrigger,
}

#[gen_stub_pymethods]
#[pymethods]
impl Trigger {
    /// Creates a trigger that fires when a reading rises above the line and clears once
    /// it has fallen below the line by the hysteresis.
    #[staticmethod]
    fn above(threshold: f32, hysteresis: f32) -> Self {
        Self {
            inner: CoreTrigger::above(threshold, hysteresis),
        }
    }

    /// Creates a trigger that fires when a reading falls below the line and clears once
    /// it has risen above the line by the hysteresis.
    #[staticmethod]
    fn below(threshold: f32, hysteresis: f32) -> Self {
        Self {
            inner: CoreTrigger::below(threshold, hysteresis),
        }
    }

    /// Feeds a reading in and returns the edge it caused, or `None` while nothing changed.
    fn update(&mut self, reading: f32) -> Option<&'static str> {
        self.inner.update(reading).map(|edge| match edge {
            Edge::Set => "set",
            Edge::Cleared => "cleared",
        })
    }

    /// Whether the condition currently holds.
    #[getter]
    fn is_set(&self) -> bool {
        self.inner.is_set()
    }

    /// The line the trigger watches.
    #[getter]
    fn threshold(&self) -> f32 {
        self.inner.threshold()
    }

    /// The release band on the far side of the line.
    #[getter]
    fn hysteresis(&self) -> f32 {
        self.inner.hysteresis()
    }

    /// Whether the trigger watches a rising reading, as `above` makes it.
    #[getter]
    fn watches_above(&self) -> bool {
        self.inner.watches_above()
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
    /// Returns 0 once the level is at or below the threshold, the first reading included,
    /// and `None` while the level is steady or rising, or on a first reading above it,
    /// when no rate of fall is known yet.
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
    ///
    /// `samples` is a whole number from 0 to 65535; anything else raises `ValueError`.
    #[new]
    fn new(samples: i64, initial: bool) -> PyResult<Self> {
        let samples = u16::try_from(samples).map_err(|_| {
            PyValueError::new_err(format!(
                "samples must be a whole number from 0 to 65535, not {samples}"
            ))
        })?;
        Ok(Self {
            inner: CoreDebounce::new(samples, initial),
        })
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
    /// Creates a detector for rises of more than `limit` between readings.
    #[staticmethod]
    fn rising(limit: f32) -> Self {
        Self {
            inner: CoreSurge::rising(limit),
        }
    }

    /// Creates a detector for falls of more than `limit` between readings.
    #[staticmethod]
    fn falling(limit: f32) -> Self {
        Self {
            inner: CoreSurge::falling(limit),
        }
    }

    /// Feeds a value in and returns the size of a step past the limit, or `None`.
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
    /// Creates a circular fence of `radius_m` meters around a center fix.
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

/// Returns the great-circle distance between two coordinates, in meters.
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

/// Holds `value` at `center` while it stays within `width` either side, and passes it
/// through unchanged once it is further out.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn deadband(value: f32, center: f32, width: f32) -> f32 {
    core_deadband(value, center, width)
}

/// The storage every windowed helper here is built with.
const CAPACITY: usize = 32;

/// The most readings a windowed helper keeps, and the number it keeps unless told fewer.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn window_capacity() -> usize {
    CAPACITY
}

/// Reads the capacity a windowed helper was asked for, refusing one below `least`, the
/// fewest readings the helper can answer from.
fn capacity_of(capacity: Option<i64>, least: usize) -> PyResult<usize> {
    let Some(capacity) = capacity else {
        return Ok(CAPACITY);
    };
    match usize::try_from(capacity) {
        Ok(fits) if (least..=CAPACITY).contains(&fits) => Ok(fits),
        _ => Err(PyValueError::new_err(format!(
            "capacity must be a whole number from {least} to {CAPACITY}, not {capacity}"
        ))),
    }
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
    /// Creates an empty window that keeps up to `capacity` readings, 32 unless told fewer.
    #[new]
    #[pyo3(signature = (capacity=None))]
    fn new(capacity: Option<i64>) -> PyResult<Self> {
        Ok(Self {
            inner: CoreWindow::with_capacity(capacity_of(capacity, 1)?),
        })
    }

    /// Adds a reading, dropping the oldest once the window is full.
    fn push(&mut self, reading: f32) {
        self.inner.push(reading);
    }

    /// How many readings the window holds.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Whether the window holds as many readings as it keeps.
    #[getter]
    fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    /// How many readings the window holds before it starts dropping.
    #[getter]
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// The most recent reading, or ``None`` while the window is empty.
    fn latest(&self) -> Option<f32> {
        self.inner.latest()
    }

    /// The oldest reading still held, or ``None`` while the window is empty.
    fn oldest(&self) -> Option<f32> {
        self.inner.oldest()
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

    /// The spread between the smallest and largest readings, or ``None`` while empty.
    fn range(&self) -> Option<f32> {
        self.inner.range()
    }

    /// The population variance of the readings, 0 for one reading, or ``None`` while empty.
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
    /// Creates an empty median filter over up to `capacity` readings, 32 unless told fewer.
    ///
    /// A small odd window, such as 5, follows a real change in a few readings; a window of
    /// 32 follows it 16 readings late.
    #[new]
    #[pyo3(signature = (capacity=None))]
    fn new(capacity: Option<i64>) -> PyResult<Self> {
        Ok(Self {
            inner: CoreMedian::with_capacity(capacity_of(capacity, 1)?),
        })
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

    /// How many readings the filter keeps.
    #[getter]
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }
}

/// Fits a line through recent readings, so a slow drift is visible before it matters.
#[gen_stub_pyclass]
#[pyclass]
pub struct Trend {
    inner: CoreTrend<CAPACITY>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Trend {
    /// Creates an empty trend estimator over up to `capacity` readings, 32 unless told fewer.
    ///
    /// A line needs two readings, so `capacity` is at least 2.
    #[new]
    #[pyo3(signature = (capacity=None))]
    fn new(capacity: Option<i64>) -> PyResult<Self> {
        Ok(Self {
            inner: CoreTrend::with_capacity(capacity_of(capacity, 2)?),
        })
    }

    /// Adds a reading.
    fn push(&mut self, reading: f32) {
        self.inner.push(reading);
    }

    /// The fitted slope in units per reading, or ``None`` without two readings.
    ///
    /// A positive slope is a rising signal.
    #[getter]
    fn slope(&self) -> Option<f32> {
        self.inner.slope()
    }

    /// How many readings the estimator keeps.
    #[getter]
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }
}

/// Flags a reading that stands out from the ones before it.
#[gen_stub_pyclass]
#[pyclass]
pub struct Anomaly {
    inner: CoreAnomaly<CAPACITY>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Anomaly {
    /// Creates a detector that flags a reading `sigmas` deviations from the mean of up to
    /// `capacity` readings before it, 32 unless told fewer.
    ///
    /// A spread needs two readings, so `capacity` is at least 2.
    #[new]
    #[pyo3(signature = (sigmas, capacity=None))]
    fn new(sigmas: f32, capacity: Option<i64>) -> PyResult<Self> {
        Ok(Self {
            inner: CoreAnomaly::with_capacity(sigmas, capacity_of(capacity, 2)?),
        })
    }

    /// Folds a reading in and reports whether it stands out.
    ///
    /// Nothing is flagged before two readings are held; from the third on, a reading can
    /// be, and one that is not a finite number always is.
    fn check(&mut self, reading: f32) -> bool {
        self.inner.check(reading)
    }

    /// How many readings the baseline keeps.
    #[getter]
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }
}
