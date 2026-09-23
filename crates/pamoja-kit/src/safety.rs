//! Keeping a moving robot safe: stop on command, stop on silence, and never lurch.
//!
//! A robot that drives itself is dangerous when something goes wrong, so safety here is a real
//! feature rather than an afterthought. Three pieces compose into a [`SafetyGate`] that every
//! motion command passes through: an [`EStop`] that latches the machine stopped until a person
//! clears it, a [`Watchdog`] that stops the machine if the commands stop arriving (a crashed
//! controller or a dropped link), and [`Limits`] that bound how fast and how abruptly the robot
//! may move. The gate fails safe: when stopped it commands zero, and it eases back up through the
//! limits rather than jumping.

use crate::motion::{cap, clamp, finite_or_zero, magnitude, Twist};
use crate::Ramp;
use libm::sqrtf;

/// An emergency stop that latches: once engaged it holds until explicitly reset.
///
/// Unlike a transient condition, an e-stop must stay tripped after the event that caused it, so a
/// person decides when it is safe to move again. While engaged, [`gate`](EStop::gate) forces any
/// command to a full stop.
///
/// # Examples
///
/// ```
/// use pamoja_kit::{EStop, Twist};
///
/// let mut estop = EStop::new();
/// let cmd = Twist::planar(1.0, 0.0);
/// assert_eq!(estop.gate(cmd), cmd); // clear: passes through
///
/// estop.engage();
/// assert_eq!(estop.gate(cmd), Twist::zero()); // latched: stopped
/// estop.reset();
/// assert_eq!(estop.gate(cmd), cmd); // cleared by a person
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct EStop {
    engaged: bool,
}

impl EStop {
    /// Creates a cleared e-stop.
    ///
    /// # Returns
    ///
    /// An e-stop that is not engaged.
    pub fn new() -> Self {
        Self { engaged: false }
    }

    /// Engages the stop; it latches until [`reset`](EStop::reset).
    pub fn engage(&mut self) {
        self.engaged = true;
    }

    /// Clears the stop, allowing motion again.
    pub fn reset(&mut self) {
        self.engaged = false;
    }

    /// Returns whether the stop is currently engaged.
    ///
    /// # Returns
    ///
    /// `true` while latched.
    pub fn is_engaged(&self) -> bool {
        self.engaged
    }

    /// Returns the command to apply: the input when clear, a full stop when engaged.
    ///
    /// # Arguments
    ///
    /// * `desired` - the command that would be applied if clear.
    ///
    /// # Returns
    ///
    /// `desired` when clear, or [`Twist::zero`] when engaged.
    pub fn gate(&self, desired: Twist) -> Twist {
        if self.engaged {
            Twist::zero()
        } else {
            desired
        }
    }
}

/// A deadman timer: it expires unless fed often enough, catching a stalled controller or link.
///
/// Autonomy assumes a stream of fresh commands. If that stream stops, because the controller hung
/// or the radio dropped, the last command must not run forever. A watchdog counts the time since it
/// was last fed and reports expiry once that exceeds its timeout; the caller feeds it each time a
/// fresh command arrives. It accumulates a supplied `dt` rather than reading a clock, so it works
/// the same on a microcontroller with no wall time.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Watchdog;
///
/// let mut dog = Watchdog::new(0.5); // expire after 0.5 s of silence
/// dog.feed();
/// assert!(!dog.update(0.3)); // 0.3 s since feeding: still alive
/// assert!(dog.update(0.3)); // 0.6 s total: expired
/// dog.feed(); // a fresh command revives it
/// assert!(!dog.is_expired());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Watchdog {
    timeout: f32,
    elapsed: f32,
}

impl Watchdog {
    /// Creates a watchdog that expires after `timeout` without being fed.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the allowed silence before expiry; its magnitude is used, and one that is
    ///   not a number is taken as zero, so the watchdog expires rather than never.
    ///
    /// # Returns
    ///
    /// A freshly fed watchdog.
    pub fn new(timeout: f32) -> Self {
        Self {
            timeout: cap(timeout),
            elapsed: 0.0,
        }
    }

    /// Feeds the watchdog, resetting the silence timer.
    pub fn feed(&mut self) {
        self.elapsed = 0.0;
    }

    /// Advances the timer by `dt` and returns whether it has expired.
    ///
    /// # Arguments
    ///
    /// * `dt` - the time since the previous update; its magnitude is used. One that is not a
    ///   finite number means the time is unknown, and the watchdog expires until it is fed,
    ///   since silence of unknown length is not safe to assume short.
    ///
    /// # Returns
    ///
    /// `true` if the watchdog is now expired.
    pub fn update(&mut self, dt: f32) -> bool {
        if dt.is_finite() {
            self.elapsed += magnitude(dt);
        } else {
            self.elapsed = f32::INFINITY;
        }
        self.is_expired()
    }

    /// Returns whether the watchdog is currently expired.
    ///
    /// # Returns
    ///
    /// `true` if the time since feeding exceeds the timeout.
    pub fn is_expired(&self) -> bool {
        self.elapsed > self.timeout
    }
}

/// Bounds a robot's speed and acceleration so commands stay within what the machine can do safely.
///
/// A raw command can be too fast or too sudden: a full-speed setpoint snaps the wheels, a hard
/// reverse strips traction. [`Limits`] caps the planar speed and yaw rate, then eases each toward
/// the capped target at a bounded acceleration, so motion is smooth and within envelope. The
/// easing reuses [`Ramp`] as its slew-rate limiter, with the step set to `accel * dt` each update.
///
/// # Examples
///
/// ```
/// use pamoja_kit::{Limits, Twist};
///
/// // Up to 1 m/s and 2 rad/s, easing on at 0.5 m/s^2 and 4 rad/s^2.
/// let mut limits = Limits::new(1.0, 2.0, 0.5, 4.0);
///
/// // Asking for full speed from rest: the first 0.1 s step is acceleration-limited.
/// let cmd = limits.apply(Twist::planar(1.0, 0.0), 0.1);
/// assert!((cmd.vx - 0.05).abs() < 1e-6); // 0.5 m/s^2 * 0.1 s
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    max_linear: f32,
    max_angular: f32,
    max_linear_accel: f32,
    max_angular_accel: f32,
    vx: Ramp,
    vy: Ramp,
    omega: Ramp,
}

impl Limits {
    /// Creates limits from the speed and acceleration ceilings.
    ///
    /// # Arguments
    ///
    /// * `max_linear` - the largest planar speed; its magnitude is used.
    /// * `max_angular` - the largest yaw rate; its magnitude is used.
    /// * `max_linear_accel` - the largest change in linear speed per second; its magnitude is used.
    /// * `max_angular_accel` - the largest change in yaw rate per second; its magnitude is used.
    ///
    /// A ceiling that is not a number is taken as zero, so a mistake holds the robot still.
    ///
    /// # Returns
    ///
    /// Limits starting from rest.
    pub fn new(
        max_linear: f32,
        max_angular: f32,
        max_linear_accel: f32,
        max_angular_accel: f32,
    ) -> Self {
        Self {
            max_linear: cap(max_linear),
            max_angular: cap(max_angular),
            max_linear_accel: cap(max_linear_accel),
            max_angular_accel: cap(max_angular_accel),
            vx: Ramp::new(0.0, 0.0),
            vy: Ramp::new(0.0, 0.0),
            omega: Ramp::new(0.0, 0.0),
        }
    }

    /// Resets the remembered motion to rest, so the next command eases up from zero.
    pub fn reset(&mut self) {
        self.vx.set(0.0);
        self.vy.set(0.0);
        self.omega.set(0.0);
    }

    /// Bounds a desired command in speed and acceleration and returns the safe command.
    ///
    /// # Arguments
    ///
    /// * `desired` - the requested body motion. A component that is not a finite number is
    ///   taken as zero, so a planner that fails eases the robot to a stop.
    /// * `dt` - the time since the previous call, setting the acceleration step. One that is
    ///   not a finite number allows no change this call.
    ///
    /// # Returns
    ///
    /// The command after capping speed and easing toward it within the acceleration limit.
    pub fn apply(&mut self, desired: Twist, dt: f32) -> Twist {
        let desired = Twist::new(
            finite_or_zero(desired.vx),
            finite_or_zero(desired.vy),
            finite_or_zero(desired.omega),
        );
        let dt = if dt.is_finite() { magnitude(dt) } else { 0.0 };
        let bounded = self.clamp_speed(desired);
        let linear_step = self.max_linear_accel * dt;
        let angular_step = self.max_angular_accel * dt;
        Twist::new(
            self.vx.update_capped(bounded.vx, linear_step),
            self.vy.update_capped(bounded.vy, linear_step),
            self.omega.update_capped(bounded.omega, angular_step),
        )
    }

    // Caps the planar speed (scaling vx and vy together so direction is kept) and the yaw rate.
    fn clamp_speed(&self, twist: Twist) -> Twist {
        let speed = sqrtf(twist.vx * twist.vx + twist.vy * twist.vy);
        let (vx, vy) = if speed > self.max_linear {
            let scale = self.max_linear / speed;
            (twist.vx * scale, twist.vy * scale)
        } else {
            (twist.vx, twist.vy)
        };
        Twist::new(
            vx,
            vy,
            clamp(twist.omega, -self.max_angular, self.max_angular),
        )
    }
}

/// The single gate every motion command passes through, composing e-stop, watchdog, and limits.
///
/// This is the one call a control loop makes to drive safely: feed it the desired motion and the
/// time step, and it returns what is actually safe to command. It stops hard (commands zero and
/// forgets its motion history, so resuming eases from rest) whenever the e-stop is engaged or the
/// watchdog has expired; otherwise it returns the desired motion bounded by the [`Limits`]. Call
/// [`feed`](SafetyGate::feed) whenever a fresh command arrives to keep the watchdog satisfied.
///
/// # Examples
///
/// ```
/// use pamoja_kit::{Limits, SafetyGate, Twist};
///
/// let limits = Limits::new(1.0, 2.0, 0.5, 4.0);
/// let mut gate = SafetyGate::new(limits, 0.2); // stop if unfed for 0.2 s
///
/// gate.feed();
/// let cmd = gate.command(Twist::planar(1.0, 0.0), 0.1);
/// assert!((cmd.vx - 0.05).abs() < 1e-6); // eased on, acceleration-limited
///
/// gate.engage_estop();
/// assert_eq!(gate.command(Twist::planar(1.0, 0.0), 0.1), Twist::zero());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SafetyGate {
    estop: EStop,
    watchdog: Watchdog,
    limits: Limits,
}

impl SafetyGate {
    /// Creates a gate from motion limits and a watchdog timeout.
    ///
    /// # Arguments
    ///
    /// * `limits` - the speed and acceleration bounds for normal motion.
    /// * `watchdog_timeout` - the allowed silence before the gate stops the robot.
    ///
    /// # Returns
    ///
    /// The gate, cleared and freshly fed.
    pub fn new(limits: Limits, watchdog_timeout: f32) -> Self {
        Self {
            estop: EStop::new(),
            watchdog: Watchdog::new(watchdog_timeout),
            limits,
        }
    }

    /// Feeds the watchdog; call this whenever a fresh command arrives.
    pub fn feed(&mut self) {
        self.watchdog.feed();
    }

    /// Engages the latching emergency stop.
    pub fn engage_estop(&mut self) {
        self.estop.engage();
    }

    /// Clears the emergency stop.
    pub fn reset_estop(&mut self) {
        self.estop.reset();
    }

    /// Returns whether the gate is currently forcing a stop.
    ///
    /// # Returns
    ///
    /// `true` if the e-stop is engaged or the watchdog has expired.
    pub fn is_stopped(&self) -> bool {
        self.estop.is_engaged() || self.watchdog.is_expired()
    }

    /// Returns the safe command for a desired motion over a time step.
    ///
    /// # Arguments
    ///
    /// * `desired` - the requested body motion.
    /// * `dt` - the time since the previous call.
    ///
    /// # Returns
    ///
    /// [`Twist::zero`] when stopped, otherwise the desired motion bounded by the limits.
    pub fn command(&mut self, desired: Twist, dt: f32) -> Twist {
        self.watchdog.update(dt);
        if self.is_stopped() {
            self.limits.reset();
            return Twist::zero();
        }
        self.limits.apply(desired, dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estop_latches_until_reset() {
        let mut estop = EStop::new();
        let cmd = Twist::planar(1.0, 0.5);
        assert_eq!(estop.gate(cmd), cmd);
        estop.engage();
        assert!(estop.is_engaged());
        assert_eq!(estop.gate(cmd), Twist::zero());
        estop.reset();
        assert_eq!(estop.gate(cmd), cmd);
    }

    #[test]
    fn watchdog_expires_on_silence_and_revives_on_feeding() {
        let mut dog = Watchdog::new(0.5);
        dog.feed();
        assert!(!dog.update(0.3));
        assert!(dog.update(0.3)); // 0.6 > 0.5
        dog.feed();
        assert!(!dog.is_expired());
    }

    #[test]
    fn limits_cap_planar_speed_by_scaling() {
        let mut limits = Limits::new(1.0, 10.0, 100.0, 100.0); // accel high so slew is not the cap
                                                               // A 3-4-5 triangle at speed 5 scales down to length 1, keeping direction.
        let cmd = limits.apply(Twist::new(3.0, 4.0, 0.0), 1.0);
        assert!((cmd.vx - 0.6).abs() < 1e-5);
        assert!((cmd.vy - 0.8).abs() < 1e-5);
    }

    #[test]
    fn limits_ease_in_at_the_acceleration_bound() {
        let mut limits = Limits::new(1.0, 2.0, 0.5, 4.0);
        assert!((limits.apply(Twist::planar(1.0, 0.0), 0.1).vx - 0.05).abs() < 1e-6);
        assert!((limits.apply(Twist::planar(1.0, 0.0), 0.1).vx - 0.10).abs() < 1e-6);
    }

    #[test]
    fn gate_stops_on_estop_and_on_watchdog_expiry() {
        let mut gate = SafetyGate::new(Limits::new(1.0, 2.0, 100.0, 100.0), 0.2);

        gate.feed();
        assert!(gate.command(Twist::planar(1.0, 0.0), 0.1).vx > 0.0);

        // Stop feeding: after enough time the watchdog trips and the gate zeroes the command.
        assert_eq!(gate.command(Twist::planar(1.0, 0.0), 0.5), Twist::zero());
        assert!(gate.is_stopped());

        // Feeding revives it.
        gate.feed();
        assert!(gate.command(Twist::planar(1.0, 0.0), 0.1).vx > 0.0);

        // The e-stop overrides even a fed watchdog.
        gate.feed();
        gate.engage_estop();
        assert_eq!(gate.command(Twist::planar(1.0, 0.0), 0.1), Twist::zero());
    }

    #[test]
    fn a_time_step_that_is_not_a_number_stops_the_gate_until_fed() {
        let mut gate = SafetyGate::new(Limits::new(1.0, 2.0, 100.0, 100.0), 0.2);
        gate.feed();
        assert!(gate.command(Twist::planar(1.0, 0.0), 0.1).vx > 0.0);
        assert_eq!(
            gate.command(Twist::planar(1.0, 0.0), f32::NAN),
            Twist::zero()
        );
        assert_eq!(gate.command(Twist::planar(1.0, 0.0), 0.01), Twist::zero());
        gate.feed();
        assert!(gate.command(Twist::planar(1.0, 0.0), 0.1).vx > 0.0);
    }

    #[test]
    fn a_watchdog_timeout_that_is_not_a_number_expires() {
        let mut dog = Watchdog::new(f32::NAN);
        assert!(dog.update(0.001));
    }

    #[test]
    fn a_command_that_is_not_a_number_eases_to_a_stop() {
        let mut limits = Limits::new(1.0, 2.0, 0.5, 4.0);
        limits.apply(Twist::planar(1.0, 0.0), 1.0);
        let easing = limits.apply(Twist::new(f32::NAN, 0.0, f32::INFINITY), 1.0);
        assert_eq!(easing, Twist::new(0.0, 0.0, 0.0));
        assert!(limits.apply(Twist::planar(1.0, 0.0), f32::NAN).vx == 0.0);
    }

    #[test]
    fn a_ceiling_that_is_not_a_number_holds_the_robot_still() {
        let mut limits = Limits::new(f32::NAN, f32::NAN, 100.0, 100.0);
        assert_eq!(limits.apply(Twist::planar(1.0, 1.0), 1.0), Twist::zero());
    }
}
