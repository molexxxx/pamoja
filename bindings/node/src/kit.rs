//! Generated Node bindings for the goal-named helper math.
//!
//! These mirror the `pamoja-kit` Rust API one-to-one. The helpers are synchronous
//! pure math, so every method here returns its value directly; the ones that
//! answer "maybe" return `null` rather than throwing, because having no answer yet
//! is an ordinary state and not a failure.

use napi_derive::napi;
use pamoja_kit::{
    deadband as core_deadband, Boundary, Calibration as CoreCalibration, Coordinate,
    Debounce as CoreDebounce, Depletion as CoreDepletion, Geofence as CoreGeofence,
    Kalman as CoreKalman, Pid as CorePid, Ramp as CoreRamp, Smoother as CoreSmoother,
    Surge as CoreSurge, Thermostat as CoreThermostat,
};

/// A latitude and longitude in degrees.
#[napi(object)]
pub struct Coord {
    /// Degrees north of the equator, negative for south.
    pub latitude: f64,
    /// Degrees east of the prime meridian, negative for west.
    pub longitude: f64,
}

impl From<Coord> for Coordinate {
    fn from(value: Coord) -> Self {
        Coordinate::new(value.latitude, value.longitude)
    }
}

/// Where a fix sits relative to a geofence, including the moment it crosses.
#[napi(string_enum)]
pub enum BoundaryState {
    /// The fix is inside the fence and was inside before, or is the first fix inside.
    Inside,
    /// The fix is outside the fence and was outside before, or is the first fix outside.
    Outside,
    /// The fix just crossed from inside to outside: the moment to raise a breach alert.
    Exited,
    /// The fix just crossed from outside back inside.
    Entered,
}

impl From<Boundary> for BoundaryState {
    fn from(value: Boundary) -> Self {
        match value {
            Boundary::Inside => Self::Inside,
            Boundary::Outside => Self::Outside,
            Boundary::Exited => Self::Exited,
            Boundary::Entered => Self::Entered,
        }
    }
}

/// Smooths a noisy reading by weighting each new sample against the running value.
#[napi]
pub struct Smoother {
    inner: CoreSmoother,
}

#[napi]
impl Smoother {
    /// Creates a smoother whose `weight` sets how much each new sample counts.
    #[napi(constructor)]
    pub fn new(weight: f64) -> Self {
        Self {
            inner: CoreSmoother::new(weight as f32),
        }
    }

    /// Folds a sample in and returns the smoothed value.
    #[napi]
    pub fn update(&mut self, sample: f64) -> f64 {
        f64::from(self.inner.update(sample as f32))
    }

    /// Returns the current value, or `null` before the first sample.
    #[napi]
    pub fn value(&self) -> Option<f64> {
        self.inner.value().map(f64::from)
    }

    /// Clears the smoother back to its initial state.
    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Holds a value at a setpoint by trading off present, past, and predicted error.
#[napi]
pub struct Pid {
    inner: CorePid,
}

#[napi]
impl Pid {
    /// Creates a controller with the given proportional, integral, derivative gains.
    #[napi(constructor)]
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            inner: CorePid::new(kp as f32, ki as f32, kd as f32),
        }
    }

    /// Creates a controller whose output is clamped to `[min, max]`.
    #[napi(factory)]
    pub fn with_limits(kp: f64, ki: f64, kd: f64, min: f64, max: f64) -> Self {
        Self {
            inner: CorePid::new(kp as f32, ki as f32, kd as f32)
                .with_limits(min as f32, max as f32),
        }
    }

    /// Advances the controller by one step and returns the control output.
    #[napi]
    pub fn update(&mut self, setpoint: f64, measurement: f64, dt: f64) -> f64 {
        f64::from(
            self.inner
                .update(setpoint as f32, measurement as f32, dt as f32),
        )
    }

    /// Clears the accumulated integral and last error.
    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Switches a load on and off around a setpoint, with hysteresis to stop chatter.
#[napi]
pub struct Thermostat {
    inner: CoreThermostat,
}

#[napi]
impl Thermostat {
    /// Creates a cooling thermostat, which switches on when the reading rises.
    #[napi(factory)]
    pub fn cooling(setpoint: f64, hysteresis: f64) -> Self {
        Self {
            inner: CoreThermostat::cooling(setpoint as f32, hysteresis as f32),
        }
    }

    /// Creates a heating thermostat, which switches on when the reading falls.
    #[napi(factory)]
    pub fn heating(setpoint: f64, hysteresis: f64) -> Self {
        Self {
            inner: CoreThermostat::heating(setpoint as f32, hysteresis as f32),
        }
    }

    /// Feeds a reading in and returns whether the load should be on.
    #[napi]
    pub fn update(&mut self, reading: f64) -> bool {
        self.inner.update(reading as f32)
    }

    /// Reports the current output without feeding in a reading.
    #[napi]
    pub fn is_on(&self) -> bool {
        self.inner.is_on()
    }
}

/// Warns before a falling level runs out, by projecting its rate of fall.
#[napi]
pub struct Depletion {
    inner: CoreDepletion,
}

#[napi]
impl Depletion {
    /// Creates an estimator that counts down to `threshold`.
    #[napi(constructor)]
    pub fn new(threshold: f64) -> Self {
        Self {
            inner: CoreDepletion::new(threshold as f32),
        }
    }

    /// Records a level and returns the samples left before the threshold.
    ///
    /// Returns `null` while the level is steady or rising, and on the first
    /// reading, when no rate of fall is known yet.
    #[napi]
    pub fn update(&mut self, level: f64) -> Option<u32> {
        self.inner.update(level as f32)
    }
}

/// Estimates a true value from noisy readings, trusting the model and the sensor
/// in proportion to how noisy each is.
#[napi]
pub struct Kalman {
    inner: CoreKalman,
}

#[napi]
impl Kalman {
    /// Creates a filter from the process and measurement noise, and a first guess.
    #[napi(constructor)]
    pub fn new(process_noise: f64, measurement_noise: f64, initial: f64) -> Self {
        Self {
            inner: CoreKalman::new(
                process_noise as f32,
                measurement_noise as f32,
                initial as f32,
            ),
        }
    }

    /// Folds a reading in and returns the new estimate.
    #[napi]
    pub fn update(&mut self, reading: f64) -> f64 {
        f64::from(self.inner.update(reading as f32))
    }

    /// Returns the current estimate without folding in a reading.
    #[napi]
    pub fn estimate(&self) -> f64 {
        f64::from(self.inner.estimate())
    }
}

/// Stops a flickering input from acting until it has settled.
#[napi]
pub struct Debounce {
    inner: CoreDebounce,
}

#[napi]
impl Debounce {
    /// Creates a debouncer needing `samples` agreeing readings to change state.
    #[napi(constructor)]
    pub fn new(samples: u16, initial: bool) -> Self {
        Self {
            inner: CoreDebounce::new(samples, initial),
        }
    }

    /// Feeds a raw reading in and returns the settled state.
    #[napi]
    pub fn update(&mut self, raw: bool) -> bool {
        self.inner.update(raw)
    }

    /// Returns the settled state without feeding in a reading.
    #[napi]
    pub fn state(&self) -> bool {
        self.inner.state()
    }
}

/// Limits how fast a value may change, so a load is never slammed.
#[napi]
pub struct Ramp {
    inner: CoreRamp,
}

#[napi]
impl Ramp {
    /// Creates a limiter starting at `start` and moving at most `maxStep` a step.
    #[napi(constructor)]
    pub fn new(start: f64, max_step: f64) -> Self {
        Self {
            inner: CoreRamp::new(start as f32, max_step as f32),
        }
    }

    /// Moves one step toward `target` and returns the new value.
    #[napi]
    pub fn update(&mut self, target: f64) -> f64 {
        f64::from(self.inner.update(target as f32))
    }

    /// Returns the current value.
    #[napi]
    pub fn value(&self) -> f64 {
        f64::from(self.inner.value())
    }

    /// Forces the value without rate limiting.
    #[napi]
    pub fn set(&mut self, value: f64) {
        self.inner.set(value as f32);
    }
}

/// Notices a step change between successive readings, such as a burst pipe.
#[napi]
pub struct Surge {
    inner: CoreSurge,
}

#[napi]
impl Surge {
    /// Creates a detector for rises of at least `limit` between readings.
    #[napi(factory)]
    pub fn rising(limit: f64) -> Self {
        Self {
            inner: CoreSurge::rising(limit as f32),
        }
    }

    /// Creates a detector for falls of at least `limit` between readings.
    #[napi(factory)]
    pub fn falling(limit: f64) -> Self {
        Self {
            inner: CoreSurge::falling(limit as f32),
        }
    }

    /// Feeds a value in and returns the size of a qualifying step, or `null`.
    #[napi]
    pub fn update(&mut self, value: f64) -> Option<f64> {
        self.inner.update(value as f32).map(f64::from)
    }
}

/// Turns a raw sensor count into the units the reading is actually in.
#[napi]
pub struct Calibration {
    inner: CoreCalibration,
}

#[napi]
impl Calibration {
    /// Creates a calibration applying `raw * scale + offset`.
    #[napi(factory)]
    pub fn linear(scale: f64, offset: f64) -> Self {
        Self {
            inner: CoreCalibration::linear(scale as f32, offset as f32),
        }
    }

    /// Creates a calibration fitted through two known reference points.
    #[napi(factory)]
    pub fn two_point(raw_low: f64, value_low: f64, raw_high: f64, value_high: f64) -> Self {
        Self {
            inner: CoreCalibration::two_point(
                raw_low as f32,
                value_low as f32,
                raw_high as f32,
                value_high as f32,
            ),
        }
    }

    /// Converts a raw reading into calibrated units.
    #[napi]
    pub fn apply(&self, raw: f64) -> f64 {
        f64::from(self.inner.apply(raw as f32))
    }
}

/// Keeps a tracked point inside an area, and notices when it leaves.
#[napi]
pub struct Geofence {
    inner: CoreGeofence,
}

#[napi]
impl Geofence {
    /// Creates a circular fence of `radiusM` metres around `center`.
    #[napi(constructor)]
    pub fn new(center: Coord, radius_m: f64) -> Self {
        Self {
            inner: CoreGeofence::new(center.into(), radius_m),
        }
    }

    /// Feeds a fix in and reports where it sits, including a single crossing.
    #[napi]
    pub fn update(&mut self, point: Coord) -> BoundaryState {
        self.inner.update(point.into()).into()
    }

    /// Reports whether a fix lies inside, without recording a crossing.
    #[napi]
    pub fn contains(&self, point: Coord) -> bool {
        self.inner.contains(point.into())
    }
}

/// Returns the great-circle distance between two coordinates, in metres.
#[napi]
pub fn distance_between(from: Coord, to: Coord) -> f64 {
    Coordinate::from(from).distance_to(to.into())
}

/// Returns the initial bearing from one coordinate to another, in degrees.
#[napi]
pub fn bearing_between(from: Coord, to: Coord) -> f64 {
    Coordinate::from(from).bearing_to(to.into())
}

/// Suppresses movement within `width` of `center`, so noise does not act.
#[napi]
pub fn deadband(value: f64, center: f64, width: f64) -> f64 {
    f64::from(core_deadband(value as f32, center as f32, width as f32))
}
