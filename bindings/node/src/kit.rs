//! Generated Node bindings for the goal-named helper math.
//!
//! These bind the reading and control helpers of `pamoja-kit`; the robotics helpers are in
//! the `motion` module. The helpers are synchronous pure math, so every method here returns
//! its value directly; the ones that answer "maybe" return `null` rather than throwing,
//! because having no answer yet is an ordinary state and not a failure. A reading that is
//! not a finite number is ignored by every helper that keeps state, as the Rust crate
//! documents.

use napi_derive::napi;
use pamoja_kit::{
    deadband as core_deadband, imu, units, weather, Anomaly as CoreAnomaly, Boundary,
    Calibration as CoreCalibration, Complementary as CoreComplementary, Coordinate,
    Debounce as CoreDebounce, Depletion as CoreDepletion, Edge as CoreEdge,
    Geofence as CoreGeofence, Kalman as CoreKalman, Median as CoreMedian, Pid as CorePid,
    Ramp as CoreRamp, Smoother as CoreSmoother, Surge as CoreSurge, Thermostat as CoreThermostat,
    Trend as CoreTrend, Trigger as CoreTrigger, Window as CoreWindow,
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

    /// The current value, or `null` before the first sample.
    #[napi(getter)]
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

    /// Whether the load is on, as the last reading left it.
    #[napi(getter)]
    pub fn is_on(&self) -> bool {
        self.inner.is_on()
    }
}

/// What a trigger reports when a reading changes its state.
#[napi(string_enum = "lowercase")]
pub enum Edge {
    /// The reading just crossed the line: the condition became true.
    Set,
    /// The reading just came back past the release band: the condition stopped holding.
    Cleared,
}

/// Fires once when a reading crosses a line, and not again until it has come back
/// past the release band.
#[napi]
pub struct Trigger {
    inner: CoreTrigger,
}

#[napi]
impl Trigger {
    /// Creates a trigger that fires when a reading rises above the line and clears once
    /// it has fallen below the line by the hysteresis.
    #[napi(factory)]
    pub fn above(threshold: f64, hysteresis: f64) -> Self {
        Self {
            inner: CoreTrigger::above(threshold as f32, hysteresis as f32),
        }
    }

    /// Creates a trigger that fires when a reading falls below the line and clears once
    /// it has risen above the line by the hysteresis.
    #[napi(factory)]
    pub fn below(threshold: f64, hysteresis: f64) -> Self {
        Self {
            inner: CoreTrigger::below(threshold as f32, hysteresis as f32),
        }
    }

    /// Feeds a reading in and returns the edge it caused, or `null` while nothing changed.
    #[napi]
    pub fn update(&mut self, reading: f64) -> Option<Edge> {
        self.inner.update(reading as f32).map(|edge| match edge {
            CoreEdge::Set => Edge::Set,
            CoreEdge::Cleared => Edge::Cleared,
        })
    }

    /// Whether the condition currently holds.
    #[napi(getter)]
    pub fn is_set(&self) -> bool {
        self.inner.is_set()
    }

    /// The line the trigger watches.
    #[napi(getter)]
    pub fn threshold(&self) -> f64 {
        f64::from(self.inner.threshold())
    }

    /// The release band on the far side of the line.
    #[napi(getter)]
    pub fn hysteresis(&self) -> f64 {
        f64::from(self.inner.hysteresis())
    }

    /// Whether the trigger watches a rising reading, as `above` makes it.
    #[napi(getter)]
    pub fn watches_above(&self) -> bool {
        self.inner.watches_above()
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
    /// Returns 0 once the level is at or below the threshold, the first reading included,
    /// and `null` while the level is steady or rising, or on a first reading above it,
    /// when no rate of fall is known yet.
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

    /// The current estimate.
    #[napi(getter)]
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
    ///
    /// `samples` is a whole number from 0 to 65535; anything else is refused rather than
    /// rounded.
    #[napi(constructor)]
    pub fn new(samples: f64, initial: bool) -> napi::Result<Self> {
        if samples.fract() != 0.0 || !(0.0..=f64::from(u16::MAX)).contains(&samples) {
            return Err(napi::Error::from_reason(format!(
                "samples must be a whole number from 0 to 65535, not {samples}"
            )));
        }
        Ok(Self {
            inner: CoreDebounce::new(samples as u16, initial),
        })
    }

    /// Feeds a raw reading in and returns the settled state.
    #[napi]
    pub fn update(&mut self, raw: bool) -> bool {
        self.inner.update(raw)
    }

    /// The settled state.
    #[napi(getter)]
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

    /// The current value.
    #[napi(getter)]
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
    /// Creates a detector for rises of more than `limit` between readings.
    #[napi(factory)]
    pub fn rising(limit: f64) -> Self {
        Self {
            inner: CoreSurge::rising(limit as f32),
        }
    }

    /// Creates a detector for falls of more than `limit` between readings.
    #[napi(factory)]
    pub fn falling(limit: f64) -> Self {
        Self {
            inner: CoreSurge::falling(limit as f32),
        }
    }

    /// Feeds a value in and returns the size of a step past the limit, or `null`.
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
    /// Creates a circular fence of `radiusM` meters around `center`.
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

/// Returns the great-circle distance between two coordinates, in meters.
#[napi]
pub fn distance_between(from: Coord, to: Coord) -> f64 {
    Coordinate::from(from).distance_to(to.into())
}

/// Returns the initial bearing from one coordinate to another, in degrees.
#[napi]
pub fn bearing_between(from: Coord, to: Coord) -> f64 {
    Coordinate::from(from).bearing_to(to.into())
}

/// Holds `value` at `center` while it stays within `width` either side, and passes it
/// through unchanged once it is further out.
#[napi]
pub fn deadband(value: f64, center: f64, width: f64) -> f64 {
    f64::from(core_deadband(value as f32, center as f32, width as f32))
}

/// Fuses a drifting rate, such as a gyroscope's, with a noisy absolute reading, such as an
/// accelerometer's tilt, into one steady estimate.
#[napi]
pub struct Complementary {
    inner: CoreComplementary,
}

#[napi]
impl Complementary {
    /// Creates a filter. `alpha` is the weight on the integrated rate, held to 0 to 1: near
    /// 1 trusts the rate and corrects slowly. One that is not a number is taken as 0.
    #[napi(constructor)]
    pub fn new(alpha: f64, initial: f64) -> Self {
        Self {
            inner: CoreComplementary::new(alpha as f32, initial as f32),
        }
    }

    /// Fuses a rate and an absolute reading over `dt` and returns the estimate. If any of
    /// the three is not a finite number, the update is ignored.
    #[napi]
    pub fn update(&mut self, rate: f64, absolute: f64, dt: f64) -> f64 {
        f64::from(self.inner.update(rate as f32, absolute as f32, dt as f32))
    }

    /// The current estimate.
    #[napi(getter)]
    pub fn estimate(&self) -> f64 {
        f64::from(self.inner.estimate())
    }
}

/// Roll and pitch, in degrees.
#[napi(object)]
pub struct Tilt {
    /// Rotation about the forward axis, in degrees, from -180 to 180.
    pub roll: f64,
    /// Rotation about the right axis, in degrees, from -90 to 90.
    pub pitch: f64,
}

/// Computes roll and pitch from a three-axis accelerometer at rest; the reading's unit
/// does not matter, since only the ratios between axes set the angles.
#[napi]
pub fn tilt_from_accel(ax: f64, ay: f64, az: f64) -> Tilt {
    let tilt = imu::tilt_from_accel(ax, ay, az);
    Tilt {
        roll: tilt.roll,
        pitch: tilt.pitch,
    }
}

/// Computes the dew point, in degrees Celsius, from the air temperature in degrees
/// Celsius and the relative humidity in percent.
#[napi]
pub fn dew_point(celsius: f64, humidity_percent: f64) -> f64 {
    weather::dew_point(celsius, humidity_percent)
}

/// Converts degrees Celsius to degrees Fahrenheit.
#[napi]
pub fn celsius_to_fahrenheit(celsius: f64) -> f64 {
    f64::from(units::celsius_to_fahrenheit(celsius as f32))
}

/// Converts degrees Fahrenheit to degrees Celsius.
#[napi]
pub fn fahrenheit_to_celsius(fahrenheit: f64) -> f64 {
    f64::from(units::fahrenheit_to_celsius(fahrenheit as f32))
}

/// Converts degrees Celsius to kelvin.
#[napi]
pub fn celsius_to_kelvin(celsius: f64) -> f64 {
    f64::from(units::celsius_to_kelvin(celsius as f32))
}

/// Converts kelvin to degrees Celsius.
#[napi]
pub fn kelvin_to_celsius(kelvin: f64) -> f64 {
    f64::from(units::kelvin_to_celsius(kelvin as f32))
}

/// Converts pascals to hectopascals.
#[napi]
pub fn pascals_to_hectopascals(pascals: f64) -> f64 {
    f64::from(units::pascals_to_hectopascals(pascals as f32))
}

/// Converts hectopascals to pascals.
#[napi]
pub fn hectopascals_to_pascals(hectopascals: f64) -> f64 {
    f64::from(units::hectopascals_to_pascals(hectopascals as f32))
}

/// Converts pascals to kilopascals.
#[napi]
pub fn pascals_to_kilopascals(pascals: f64) -> f64 {
    f64::from(units::pascals_to_kilopascals(pascals as f32))
}

/// Converts kilopascals to pascals.
#[napi]
pub fn kilopascals_to_pascals(kilopascals: f64) -> f64 {
    f64::from(units::kilopascals_to_pascals(kilopascals as f32))
}

/// Converts pascals to pounds per square inch.
#[napi]
pub fn pascals_to_psi(pascals: f64) -> f64 {
    f64::from(units::pascals_to_psi(pascals as f32))
}

/// Converts pounds per square inch to pascals.
#[napi]
pub fn psi_to_pascals(psi: f64) -> f64 {
    f64::from(units::psi_to_pascals(psi as f32))
}

/// Converts a ratio from 0 to 1 to a percentage.
#[napi]
pub fn ratio_to_percent(ratio: f64) -> f64 {
    f64::from(units::ratio_to_percent(ratio as f32))
}

/// Converts a percentage to a ratio from 0 to 1.
#[napi]
pub fn percent_to_ratio(percent: f64) -> f64 {
    f64::from(units::percent_to_ratio(percent as f32))
}

/// The most readings a windowed helper keeps, and the number it keeps unless told fewer.
///
/// The Rust helpers are generic over their capacity, which has no JavaScript
/// equivalent, so these are built with room for this many and keep fewer when asked.
#[napi]
pub const WINDOW_CAPACITY: u32 = 32;

/// The storage every windowed helper here is built with.
const CAPACITY: usize = 32;

/// Reads the capacity a windowed helper was asked for, refusing one below `least`, the
/// fewest readings the helper can answer from.
fn capacity_of(capacity: Option<f64>, least: usize) -> napi::Result<usize> {
    let Some(capacity) = capacity else {
        return Ok(CAPACITY);
    };
    if capacity.fract() != 0.0 || !(least as f64..=CAPACITY as f64).contains(&capacity) {
        return Err(napi::Error::from_reason(format!(
            "capacity must be a whole number from {least} to {CAPACITY}, not {capacity}"
        )));
    }
    Ok(capacity as usize)
}

/// A rolling window of the most recent readings, with the stats over them.
#[napi]
pub struct Window {
    inner: CoreWindow<CAPACITY>,
}

#[napi]
impl Window {
    /// Creates an empty window that keeps up to `capacity` readings, 32 unless told fewer.
    #[napi(constructor)]
    pub fn new(capacity: Option<f64>) -> napi::Result<Self> {
        Ok(Self {
            inner: CoreWindow::with_capacity(capacity_of(capacity, 1)?),
        })
    }

    /// Adds a reading, dropping the oldest once the window is full.
    #[napi]
    pub fn push(&mut self, reading: f64) {
        self.inner.push(reading as f32);
    }

    /// How many readings the window holds.
    #[napi(getter)]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    /// Whether the window is still waiting for its first reading.
    #[napi(getter)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Whether the window holds as many readings as it keeps.
    #[napi(getter)]
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    /// How many readings the window holds before it starts dropping.
    #[napi(getter)]
    pub fn capacity(&self) -> u32 {
        self.inner.capacity() as u32
    }

    /// The most recent reading, or `null` while the window is empty.
    #[napi]
    pub fn latest(&self) -> Option<f64> {
        self.inner.latest().map(f64::from)
    }

    /// The oldest reading still held, or `null` while the window is empty.
    #[napi]
    pub fn oldest(&self) -> Option<f64> {
        self.inner.oldest().map(f64::from)
    }

    /// The mean of the readings, or `null` while the window is empty.
    #[napi]
    pub fn mean(&self) -> Option<f64> {
        self.inner.mean().map(f64::from)
    }

    /// The smallest reading, or `null` while the window is empty.
    #[napi]
    pub fn min(&self) -> Option<f64> {
        self.inner.min().map(f64::from)
    }

    /// The largest reading, or `null` while the window is empty.
    #[napi]
    pub fn max(&self) -> Option<f64> {
        self.inner.max().map(f64::from)
    }

    /// The spread between the smallest and largest readings, or `null` while empty.
    #[napi]
    pub fn range(&self) -> Option<f64> {
        self.inner.range().map(f64::from)
    }

    /// The population variance of the readings, 0 for one reading, or `null` while empty.
    #[napi]
    pub fn variance(&self) -> Option<f64> {
        self.inner.variance().map(f64::from)
    }
}

/// Rejects a single wild reading, where an average would let it pull the answer.
#[napi]
pub struct Median {
    inner: CoreMedian<CAPACITY>,
}

#[napi]
impl Median {
    /// Creates an empty median filter over up to `capacity` readings, 32 unless told fewer.
    ///
    /// A small odd window, such as 5, follows a real change in a few readings; a window of
    /// 32 follows it 16 readings late.
    #[napi(constructor)]
    pub fn new(capacity: Option<f64>) -> napi::Result<Self> {
        Ok(Self {
            inner: CoreMedian::with_capacity(capacity_of(capacity, 1)?),
        })
    }

    /// Folds a reading in and returns the median of the window.
    #[napi]
    pub fn update(&mut self, reading: f64) -> f64 {
        f64::from(self.inner.update(reading as f32))
    }

    /// The current median, or `null` before the first reading.
    #[napi(getter)]
    pub fn value(&self) -> Option<f64> {
        self.inner.median().map(f64::from)
    }

    /// How many readings the filter keeps.
    #[napi(getter)]
    pub fn capacity(&self) -> u32 {
        self.inner.capacity() as u32
    }
}

/// Fits a line through recent readings, so a slow drift is visible before it matters.
#[napi]
pub struct Trend {
    inner: CoreTrend<CAPACITY>,
}

#[napi]
impl Trend {
    /// Creates an empty trend estimator over up to `capacity` readings, 32 unless told fewer.
    ///
    /// A line needs two readings, so `capacity` is at least 2.
    #[napi(constructor)]
    pub fn new(capacity: Option<f64>) -> napi::Result<Self> {
        Ok(Self {
            inner: CoreTrend::with_capacity(capacity_of(capacity, 2)?),
        })
    }

    /// Adds a reading.
    #[napi]
    pub fn push(&mut self, reading: f64) {
        self.inner.push(reading as f32);
    }

    /// The fitted slope in units per reading, or `null` without two readings.
    ///
    /// A positive slope is a rising signal.
    #[napi(getter)]
    pub fn slope(&self) -> Option<f64> {
        self.inner.slope().map(f64::from)
    }

    /// How many readings the estimator keeps.
    #[napi(getter)]
    pub fn capacity(&self) -> u32 {
        self.inner.capacity() as u32
    }
}

/// Flags a reading that stands out from the ones before it.
#[napi]
pub struct Anomaly {
    inner: CoreAnomaly<CAPACITY>,
}

#[napi]
impl Anomaly {
    /// Creates a detector that flags a reading `sigmas` deviations from the mean of up to
    /// `capacity` readings before it, 32 unless told fewer.
    ///
    /// A spread needs two readings, so `capacity` is at least 2.
    #[napi(constructor)]
    pub fn new(sigmas: f64, capacity: Option<f64>) -> napi::Result<Self> {
        Ok(Self {
            inner: CoreAnomaly::with_capacity(sigmas as f32, capacity_of(capacity, 2)?),
        })
    }

    /// Folds a reading in and reports whether it stands out.
    ///
    /// Nothing is flagged before two readings are held; from the third on, a reading can
    /// be, and one that is not a finite number always is.
    #[napi]
    pub fn check(&mut self, reading: f64) -> bool {
        self.inner.check(reading as f32)
    }

    /// How many readings the baseline keeps.
    #[napi(getter)]
    pub fn capacity(&self) -> u32 {
        self.inner.capacity() as u32
    }
}
