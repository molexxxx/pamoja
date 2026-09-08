//! The small, exact conversions between robot actuators and sensors and real units.
//!
//! Driving a robot means turning intent into the pulse widths a servo or motor controller expects,
//! and turning encoder edges back into how far a wheel has rolled. Each conversion is pure
//! arithmetic with a classic off-by-one or sign trap, so it lives here as checked logic rather
//! than scattered inline math: a [`ServoMap`] and [`Esc`] for hobby PWM outputs, and a
//! [`Quadrature`] decoder with a [`QuadratureScale`] for incremental encoders. Clocking the pulses
//! and reading the pins arrives with the hardware-I/O layer; this is the math ahead of it.

use crate::motion::{clamp, magnitude};
use core::f32::consts::PI;

/// Maps a servo angle to its RC pulse width in microseconds, and back.
///
/// A hobby servo is positioned by the width of a pulse repeated about every 20 ms: the standard
/// range is 1000 to 2000 microseconds spanning the full travel, with 1500 at center.
/// [`ServoMap::standard`] uses those defaults over 180 degrees; [`ServoMap::new`] covers servos
/// with a different range or travel.
///
/// # Examples
///
/// ```
/// use pamoja_kit::ServoMap;
///
/// let servo = ServoMap::standard();
/// assert_eq!(servo.pulse(0.0), 1000);
/// assert_eq!(servo.pulse(90.0), 1500); // center
/// assert_eq!(servo.pulse(180.0), 2000);
/// assert!((servo.angle(1500) - 90.0).abs() < 1e-3);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ServoMap {
    min_us: u16,
    max_us: u16,
    range_deg: f32,
}

impl ServoMap {
    /// Returns the standard hobby-servo map: 1000 to 2000 microseconds over 180 degrees.
    ///
    /// # Returns
    ///
    /// The standard map.
    pub fn standard() -> Self {
        Self {
            min_us: 1000,
            max_us: 2000,
            range_deg: 180.0,
        }
    }

    /// Creates a map from explicit pulse and travel limits.
    ///
    /// # Arguments
    ///
    /// * `min_us` - the pulse width at zero degrees.
    /// * `max_us` - the pulse width at full travel.
    /// * `range_deg` - the full travel in degrees; its magnitude is used.
    ///
    /// # Returns
    ///
    /// The map.
    pub fn new(min_us: u16, max_us: u16, range_deg: f32) -> Self {
        Self {
            min_us,
            max_us,
            range_deg: magnitude(range_deg),
        }
    }

    /// Returns the pulse width for an angle.
    ///
    /// # Arguments
    ///
    /// * `angle_deg` - the desired angle in degrees, clamped to `[0, range]`.
    ///
    /// # Returns
    ///
    /// The pulse width in microseconds.
    pub fn pulse(&self, angle_deg: f32) -> u16 {
        let span_us = self.max_us as f32 - self.min_us as f32;
        let fraction = if self.range_deg == 0.0 {
            0.0
        } else {
            clamp(angle_deg, 0.0, self.range_deg) / self.range_deg
        };
        (self.min_us as f32 + fraction * span_us + 0.5) as u16
    }

    /// Returns the angle for a pulse width.
    ///
    /// # Arguments
    ///
    /// * `pulse_us` - the pulse width in microseconds, clamped to the configured range.
    ///
    /// # Returns
    ///
    /// The angle in degrees, zero when the pulse range is empty.
    pub fn angle(&self, pulse_us: u16) -> f32 {
        let span_us = self.max_us as f32 - self.min_us as f32;
        if span_us == 0.0 {
            return 0.0;
        }
        let p = clamp(pulse_us as f32, self.min_us as f32, self.max_us as f32);
        (p - self.min_us as f32) / span_us * self.range_deg
    }
}

/// Maps a normalized throttle to an electronic speed controller's RC pulse width.
///
/// An ESC reads the same RC pulse a servo does, with the pulse width setting motor output.
/// [`Esc::bidirectional`] uses the common reversible scheme: 1000 microseconds full reverse, 1500
/// neutral, 2000 full forward, so a throttle in `[-1, 1]` maps linearly across it.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Esc;
///
/// let esc = Esc::bidirectional();
/// assert_eq!(esc.pulse(0.0), 1500); // neutral
/// assert_eq!(esc.pulse(1.0), 2000); // full forward
/// assert_eq!(esc.pulse(-1.0), 1000); // full reverse
/// assert_eq!(esc.pulse(0.5), 1750);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Esc {
    min_us: u16,
    neutral_us: u16,
    max_us: u16,
}

impl Esc {
    /// Returns the standard reversible ESC map: 1000 / 1500 / 2000 microseconds.
    ///
    /// # Returns
    ///
    /// The bidirectional map.
    pub fn bidirectional() -> Self {
        Self {
            min_us: 1000,
            neutral_us: 1500,
            max_us: 2000,
        }
    }

    /// Creates a map from explicit reverse, neutral, and forward pulse widths.
    ///
    /// # Arguments
    ///
    /// * `min_us` - the pulse width at full reverse.
    /// * `neutral_us` - the pulse width at rest.
    /// * `max_us` - the pulse width at full forward.
    ///
    /// # Returns
    ///
    /// The map.
    pub fn new(min_us: u16, neutral_us: u16, max_us: u16) -> Self {
        Self {
            min_us,
            neutral_us,
            max_us,
        }
    }

    /// Returns the pulse width for a throttle.
    ///
    /// # Arguments
    ///
    /// * `throttle` - the demand in `[-1, 1]`, clamped; negative reverses, positive drives forward.
    ///
    /// # Returns
    ///
    /// The pulse width in microseconds.
    pub fn pulse(&self, throttle: f32) -> u16 {
        let t = clamp(throttle, -1.0, 1.0);
        let span = if t >= 0.0 {
            self.max_us as f32 - self.neutral_us as f32
        } else {
            self.neutral_us as f32 - self.min_us as f32
        };
        (self.neutral_us as f32 + t * span + 0.5) as u16
    }
}

// The quadrature transition table, indexed by `(previous << 2) | next` where each two-bit state
// is `(A << 1) | B`. Valid Gray-code steps give +1 or -1; no change or an illegal jump gives 0.
const QUADRATURE_TABLE: [i8; 16] = [0, 1, -1, 0, -1, 0, 0, 1, 1, 0, 0, -1, 0, -1, 1, 0];

fn encode(a: bool, b: bool) -> u8 {
    ((a as u8) << 1) | (b as u8)
}

/// Decodes a quadrature (A/B) encoder into a running tick count.
///
/// An incremental encoder reports motion as two square waves a quarter-cycle apart; their order of
/// change tells direction. Feeding successive A/B readings to [`update`](Quadrature::update) returns
/// the per-step direction and accumulates a signed count, the foundation for wheel odometry. Pair
/// it with a [`QuadratureScale`] to turn that count into meters.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Quadrature;
///
/// let mut enc = Quadrature::new();
/// // One full cycle forward: 00 -> 01 -> 11 -> 10 -> 00, one tick each.
/// for &(a, b) in &[(false, true), (true, true), (true, false), (false, false)] {
///     assert_eq!(enc.update(a, b), 1);
/// }
/// assert_eq!(enc.count(), 4);
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct Quadrature {
    state: u8,
    count: i64,
}

impl Quadrature {
    /// Creates a decoder assuming both channels start low.
    ///
    /// # Returns
    ///
    /// A decoder with a zero count.
    pub fn new() -> Self {
        Self { state: 0, count: 0 }
    }

    /// Creates a decoder seeded with the encoder's current channel levels.
    ///
    /// Seeding the initial state avoids a spurious first tick when the encoder does not happen to
    /// rest with both channels low.
    ///
    /// # Arguments
    ///
    /// * `a` - the current A channel level.
    /// * `b` - the current B channel level.
    ///
    /// # Returns
    ///
    /// A decoder with a zero count and the given starting state.
    pub fn starting(a: bool, b: bool) -> Self {
        Self {
            state: encode(a, b),
            count: 0,
        }
    }

    /// Feeds the latest channel levels and returns the tick delta.
    ///
    /// # Arguments
    ///
    /// * `a` - the latest A channel level.
    /// * `b` - the latest B channel level.
    ///
    /// # Returns
    ///
    /// `+1` or `-1` for a step in either direction, or `0` for no change or an illegal jump.
    pub fn update(&mut self, a: bool, b: bool) -> i8 {
        let next = encode(a, b);
        let delta = QUADRATURE_TABLE[((self.state << 2) | next) as usize];
        self.state = next;
        self.count += delta as i64;
        delta
    }

    /// Returns the accumulated signed tick count.
    ///
    /// # Returns
    ///
    /// The running count.
    pub fn count(&self) -> i64 {
        self.count
    }

    /// Resets the count to zero, keeping the current channel state.
    pub fn reset(&mut self) {
        self.count = 0;
    }
}

/// Converts encoder ticks into the distance and speed a wheel has traveled.
///
/// # Examples
///
/// ```
/// use pamoja_kit::QuadratureScale;
///
/// // 360 ticks per revolution on a wheel of 0.05 m radius.
/// let scale = QuadratureScale::new(360.0, 0.05);
/// // One full revolution rolls out one circumference.
/// assert!((scale.distance(360) - (2.0 * core::f32::consts::PI * 0.05)).abs() < 1e-6);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct QuadratureScale {
    counts_per_rev: f32,
    wheel_radius: f32,
}

impl QuadratureScale {
    /// Creates a scale from the encoder resolution and wheel size.
    ///
    /// # Arguments
    ///
    /// * `counts_per_rev` - ticks per wheel revolution; its magnitude is used.
    /// * `wheel_radius` - the wheel radius in meters; its magnitude is used.
    ///
    /// # Returns
    ///
    /// The scale.
    pub fn new(counts_per_rev: f32, wheel_radius: f32) -> Self {
        Self {
            counts_per_rev: magnitude(counts_per_rev),
            wheel_radius: magnitude(wheel_radius),
        }
    }

    /// Returns the distance rolled for a tick count.
    ///
    /// # Arguments
    ///
    /// * `count` - the signed tick count.
    ///
    /// # Returns
    ///
    /// The distance in meters, zero when the resolution is zero.
    pub fn distance(&self, count: i64) -> f32 {
        if self.counts_per_rev == 0.0 {
            return 0.0;
        }
        let circumference = 2.0 * PI * self.wheel_radius;
        (count as f32 / self.counts_per_rev) * circumference
    }

    /// Returns the speed for a tick count accumulated over a time step.
    ///
    /// # Arguments
    ///
    /// * `delta_count` - the ticks counted during the step.
    /// * `dt` - the length of the step.
    ///
    /// # Returns
    ///
    /// The speed in meters per second, zero when `dt` is zero.
    pub fn velocity(&self, delta_count: i64, dt: f32) -> f32 {
        if dt == 0.0 {
            0.0
        } else {
            self.distance(delta_count) / dt
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn servo_maps_ends_and_centre() {
        let servo = ServoMap::standard();
        assert_eq!(servo.pulse(0.0), 1000);
        assert_eq!(servo.pulse(90.0), 1500);
        assert_eq!(servo.pulse(180.0), 2000);
        assert_eq!(servo.pulse(999.0), 2000); // clamped
        assert!((servo.angle(1750) - 135.0).abs() < 1e-3);
    }

    #[test]
    fn esc_maps_throttle_across_the_range() {
        let esc = Esc::bidirectional();
        assert_eq!(esc.pulse(0.0), 1500);
        assert_eq!(esc.pulse(1.0), 2000);
        assert_eq!(esc.pulse(-1.0), 1000);
        assert_eq!(esc.pulse(0.5), 1750);
        assert_eq!(esc.pulse(-2.0), 1000); // clamped
    }

    #[test]
    fn quadrature_counts_forward_and_backward() {
        let mut enc = Quadrature::new();
        let forward = [(false, true), (true, true), (true, false), (false, false)];
        for &(a, b) in &forward {
            assert_eq!(enc.update(a, b), 1);
        }
        assert_eq!(enc.count(), 4);

        // Same sequence reversed steps the count back down.
        let backward = [(true, false), (true, true), (false, true), (false, false)];
        for &(a, b) in &backward {
            assert_eq!(enc.update(a, b), -1);
        }
        assert_eq!(enc.count(), 0);
    }

    #[test]
    fn quadrature_ignores_no_change_and_illegal_jumps() {
        let mut enc = Quadrature::new();
        assert_eq!(enc.update(false, false), 0); // no change from 00
        assert_eq!(enc.update(true, true), 0); // 00 -> 11 is a skipped step
    }

    #[test]
    fn scale_turns_ticks_into_distance_and_speed() {
        let scale = QuadratureScale::new(360.0, 0.05);
        let one_rev = 2.0 * PI * 0.05;
        assert!((scale.distance(360) - one_rev).abs() < 1e-6);
        assert!((scale.velocity(360, 2.0) - one_rev / 2.0).abs() < 1e-6);
        assert_eq!(scale.velocity(360, 0.0), 0.0);
    }
}
