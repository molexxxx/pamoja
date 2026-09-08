//! Fake sensors that stand in for real hardware.

use pamoja_core::{Error, Result, Sensor};

/// A fake sensor that generates a signal from a baseline, drift, and noise.
///
/// This is the workhorse for hardware-free development: it implements the core
/// [`Sensor`] trait, so it drops into a `Node`, a profile, or any test exactly where
/// a real probe would, and it produces readings that look like the field rather than
/// a clean constant. A reading is the baseline plus any accumulated drift plus a
/// bounded pseudo-random wobble, so a control loop can be exercised against a signal
/// that warms, sags, or jitters the way a real one does.
///
/// The noise is deterministic for a given seed - a small xorshift generator drives
/// it, with no `rand` dependency - so a test that uses a `SimSensor` produces the
/// same sequence every run and stays reproducible in CI.
///
/// # Examples
///
/// A noisy thermometer that warms by 0.1 degrees each reading:
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_sim::SimSensor;
///
/// # async fn demo() -> pamoja_core::Result<()> {
/// let mut probe = SimSensor::new(20.0).with_drift(0.1).with_noise(0.05).with_seed(7);
/// let first = probe.read().await?;
/// assert!((first - 20.0).abs() <= 0.05); // the first reading sits near the baseline
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SimSensor {
    value: f32,
    drift: f32,
    noise: f32,
    rng: u32,
}

impl SimSensor {
    /// Creates a sensor that reads `baseline` with no drift or noise.
    ///
    /// # Arguments
    ///
    /// * `baseline` - the value the sensor reads before drift and noise are added.
    ///
    /// # Returns
    ///
    /// A steady sensor; add [`with_drift`](SimSensor::with_drift) and
    /// [`with_noise`](SimSensor::with_noise) to make it lifelike.
    pub fn new(baseline: f32) -> Self {
        Self {
            value: baseline,
            drift: 0.0,
            noise: 0.0,
            rng: 0x9E37_79B9,
        }
    }

    /// Sets how much the baseline moves each reading, modeling a slow trend.
    ///
    /// # Arguments
    ///
    /// * `per_read` - the amount added to the baseline after each reading; negative
    ///   values sag the signal downward.
    ///
    /// # Returns
    ///
    /// The updated sensor, for chaining.
    pub fn with_drift(mut self, per_read: f32) -> Self {
        self.drift = per_read;
        self
    }

    /// Sets the amplitude of the bounded noise added to each reading.
    ///
    /// # Arguments
    ///
    /// * `amplitude` - the largest magnitude the noise can reach; its magnitude is
    ///   used, and each reading wobbles within plus or minus this amount.
    ///
    /// # Returns
    ///
    /// The updated sensor, for chaining.
    pub fn with_noise(mut self, amplitude: f32) -> Self {
        self.noise = magnitude(amplitude);
        self
    }

    /// Sets the seed for the noise generator, making a run reproducible.
    ///
    /// # Arguments
    ///
    /// * `seed` - the generator seed; zero is replaced with a fixed non-zero value,
    ///   since the xorshift generator cannot start from zero.
    ///
    /// # Returns
    ///
    /// The updated sensor, for chaining.
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.rng = if seed == 0 { 1 } else { seed };
        self
    }

    // Advances the xorshift generator and maps it to bounded noise.
    fn next_noise(&mut self) -> f32 {
        if self.noise == 0.0 {
            return 0.0;
        }
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        let unit = (x as f32 / u32::MAX as f32) * 2.0 - 1.0; // [-1.0, 1.0)
        unit * self.noise
    }
}

impl Sensor for SimSensor {
    type Reading = f32;

    async fn read(&mut self) -> Result<f32> {
        let reading = self.value + self.next_noise();
        self.value += self.drift;
        Ok(reading)
    }
}

/// A fake sensor that replays a fixed sequence of readings.
///
/// Where a [`SimSensor`] generates a signal, a `Replay` plays back exact values in
/// order, which is what a deterministic test or a scripted demo wants: spell out the
/// readings that tell the story, and the sensor yields them one per
/// [`read`](Sensor::read). A one-shot replay reports [`Error::Closed`] once the
/// sequence is exhausted; a repeating one loops forever.
///
/// # Examples
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_sim::Replay;
///
/// # async fn demo() -> pamoja_core::Result<()> {
/// let mut gauge = Replay::new(vec![1.0, 1.2, 1.9]);
/// assert_eq!(gauge.read().await?, 1.0);
/// assert_eq!(gauge.read().await?, 1.2);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Replay {
    readings: Vec<f32>,
    index: usize,
    repeat: bool,
}

impl Replay {
    /// Creates a sensor that yields `readings` once, then reports closed.
    ///
    /// # Arguments
    ///
    /// * `readings` - the values to play back in order.
    ///
    /// # Returns
    ///
    /// A one-shot replay sensor.
    pub fn new(readings: Vec<f32>) -> Self {
        Self {
            readings,
            index: 0,
            repeat: false,
        }
    }

    /// Creates a sensor that yields `readings` in a loop forever.
    ///
    /// # Arguments
    ///
    /// * `readings` - the values to play back in order, repeating from the start.
    ///
    /// # Returns
    ///
    /// A repeating replay sensor.
    pub fn repeating(readings: Vec<f32>) -> Self {
        Self {
            readings,
            index: 0,
            repeat: true,
        }
    }
}

impl Sensor for Replay {
    type Reading = f32;

    async fn read(&mut self) -> Result<f32> {
        if self.index >= self.readings.len() {
            if self.repeat && !self.readings.is_empty() {
                self.index = 0;
            } else {
                return Err(Error::Closed);
            }
        }
        let reading = self.readings[self.index];
        self.index += 1;
        Ok(reading)
    }
}

// `f32::abs` lives in `std`, but a hand-rolled magnitude keeps this consistent with
// the rest of the SDK's pure-logic crates.
fn magnitude(value: f32) -> f32 {
    if value < 0.0 {
        -value
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_plain_sensor_returns_its_baseline() {
        let mut sensor = SimSensor::new(3.5);
        assert_eq!(sensor.read().await.unwrap(), 3.5);
        assert_eq!(sensor.read().await.unwrap(), 3.5);
    }

    #[tokio::test]
    async fn drift_accumulates_each_reading() {
        let mut sensor = SimSensor::new(10.0).with_drift(2.0);
        assert_eq!(sensor.read().await.unwrap(), 10.0);
        assert_eq!(sensor.read().await.unwrap(), 12.0);
        assert_eq!(sensor.read().await.unwrap(), 14.0);
    }

    #[tokio::test]
    async fn the_same_seed_replays_the_same_noise() {
        let mut a = SimSensor::new(20.0).with_noise(0.5).with_seed(7);
        let mut b = SimSensor::new(20.0).with_noise(0.5).with_seed(7);
        for _ in 0..16 {
            assert_eq!(a.read().await.unwrap(), b.read().await.unwrap());
        }
    }

    #[tokio::test]
    async fn noise_stays_within_its_amplitude() {
        let mut sensor = SimSensor::new(20.0).with_noise(0.5).with_seed(99);
        for _ in 0..1000 {
            let reading = sensor.read().await.unwrap();
            assert!((reading - 20.0).abs() <= 0.5 + f32::EPSILON);
        }
    }

    #[tokio::test]
    async fn replay_yields_readings_in_order_then_closes() {
        let mut sensor = Replay::new(vec![1.0, 2.0]);
        assert_eq!(sensor.read().await.unwrap(), 1.0);
        assert_eq!(sensor.read().await.unwrap(), 2.0);
        assert!(matches!(sensor.read().await, Err(Error::Closed)));
    }

    #[tokio::test]
    async fn a_repeating_replay_loops() {
        let mut sensor = Replay::repeating(vec![1.0, 2.0]);
        assert_eq!(sensor.read().await.unwrap(), 1.0);
        assert_eq!(sensor.read().await.unwrap(), 2.0);
        assert_eq!(sensor.read().await.unwrap(), 1.0);
    }

    #[tokio::test]
    async fn an_empty_replay_is_closed() {
        let mut sensor = Replay::repeating(vec![]);
        assert!(matches!(sensor.read().await, Err(Error::Closed)));
    }
}
