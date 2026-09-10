#![cfg_attr(not(test), no_std)]

//! Goal-named helper math for the pamoja SDK.
//!
//! The hardest part of building something real on a cheap sensor is rarely reading
//! the sensor; it is turning that reading into a decision that holds up in the
//! field. `pamoja-kit` is the layer that closes that gap for people who are not
//! signal-processing engineers. Each helper is named for the goal rather than the
//! technique, ships with correct defaults, and documents the real algorithm one
//! layer down, so it teaches as it abstracts.
//!
//! The first helpers cover the jobs the cookbook leans on most:
//!
//! - [`Smoother`] - smooth a noisy reading (exponential moving average).
//! - [`Kalman`] - settle to a steady value from a noisy sensor (one-dimensional Kalman).
//! - [`Complementary`] - fuse a fast rate with a slow absolute reading (complementary filter).
//! - [`Median`] - reject spikes with a rolling median (robust to a lone bad reading).
//! - [`Debounce`] - clean a chattering on/off signal into one event (counter debounce).
//! - [`Calibration`] - turn a raw reading into real units (two-point linear map).
//! - [`deadband`] - ignore small wiggle around a setpoint so an actuator does not chatter.
//! - [`Thermostat`] - keep a reading near a setpoint (on/off control with
//!   hysteresis).
//! - [`Trigger`] - fire once when a reading crosses a line, and not again until it has
//!   come back (a threshold with hysteresis that reports its edges).
//! - [`Pid`] - hold a value at a target with a smooth, proportional command (PID control).
//! - [`Ramp`] - ease a command toward a target at a limited rate (slew-rate limiter).
//! - [`Depletion`] - warn before a falling level runs out (linear extrapolation).
//! - [`Surge`] - warn when a reading changes dangerously fast (first difference).
//! - [`Trend`] - tell whether a value is rising or falling and how fast (least-squares slope).
//! - [`Anomaly`] - flag a reading that departs from its recent history (three-sigma rule).
//! - [`Window`] - keep a rolling window of recent readings and read their spread
//!   (min, max, range, mean, population variance).
//! - [`units`] - convert a reading to the unit a person reads (Celsius and Fahrenheit,
//!   pascals to hPa/kPa/psi, ratio and percent).
//! - [`DiffDrive`], [`Ackermann`], [`SkidSteer`], [`Mecanum`] - wheel kinematics for the common
//!   differential, car-like, skid-steer, and omnidirectional chassis (behind `robotics`).
//! - [`TwoLinkArm`] / [`forward_kinematics`] - manipulator kinematics: the planar two-link inverse,
//!   and Denavit-Hartenberg forward kinematics for any serial arm.
//! - [`Odometry`] - dead-reckon a [`Pose`] from a body motion or wheel deltas, exact-arc.
//! - [`WaypointFollower`] / [`obstacle_stop`] - steer toward a waypoint and stop for an obstacle
//!   (behind `robotics` and `geo`).
//! - [`SafetyGate`] - the gate every motion command passes through: emergency [`EStop`], deadman
//!   [`Watchdog`], and bounded motion ([`Limits`]).
//! - [`ServoMap`] / [`Esc`] / [`Quadrature`] / [`QuadratureScale`] - servo and ESC pulse widths
//!   and quadrature-encoder decoding.
//! - [`Twist`] / [`Pose`] - the shared body-velocity and world-pose types the robotics helpers use.
//! - [`Coordinate`] / [`Geofence`] - great-circle distance and bearing, and leaving a safe
//!   area (behind the default `geo` feature).
//! - [`imu`] - roll and pitch from a three-axis accelerometer (behind the `imu` feature).
//! - [`weather`] - the dew point from temperature and humidity (behind the `weather` feature).
//!
//! The `geo`, `imu`, `weather`, and `robotics` features each pull in `libm` for `no_std` float
//! math and can be turned off on the most constrained targets; the waypoint guidance needs both
//! `robotics` and `geo`.
//!
//! The crate is `no_std` and allocation-free, so the same helpers run on a
//! microcontroller and on a server.
//!
//! # Examples
//!
//! Compose two helpers to hold a noisy fridge probe near 4 C:
//!
//! ```
//! use pamoja_kit::{Smoother, Thermostat};
//!
//! let mut probe = Smoother::new(0.5);
//! let mut fridge = Thermostat::cooling(4.0, 0.5);
//!
//! // A warm, noisy reading is smoothed, then drives the cooler on.
//! let cooler_on = fridge.update(probe.update(7.8));
//! assert!(cooler_on);
//! ```

mod anomaly;
mod calibration;
mod complementary;
mod debounce;
mod depletion;
mod drive;
mod kalman;
mod median;
mod pid;
mod ramp;
mod shape;
mod smoothing;
mod surge;
mod thermostat;
mod trend;
mod trigger;
mod window;

pub mod units;

#[cfg(feature = "geo")]
mod geo;

#[cfg(feature = "imu")]
pub mod imu;

#[cfg(feature = "weather")]
pub mod weather;

#[cfg(feature = "robotics")]
mod arm;
#[cfg(feature = "robotics")]
mod chassis;
#[cfg(feature = "robotics")]
mod drivers;
#[cfg(feature = "robotics")]
mod motion;
#[cfg(all(feature = "robotics", feature = "geo"))]
mod navigate;
#[cfg(feature = "robotics")]
mod odometry;
#[cfg(feature = "robotics")]
mod safety;

pub use anomaly::Anomaly;
pub use calibration::Calibration;
pub use complementary::Complementary;
pub use debounce::Debounce;
pub use depletion::Depletion;
pub use drive::DiffDrive;
pub use kalman::Kalman;
pub use median::Median;
pub use pid::Pid;
pub use ramp::Ramp;
pub use shape::deadband;
pub use smoothing::Smoother;
pub use surge::Surge;
pub use thermostat::Thermostat;
pub use trend::Trend;
pub use trigger::{Edge, Trigger};
pub use window::Window;

#[cfg(feature = "geo")]
pub use geo::{Boundary, Coordinate, Geofence};

#[cfg(feature = "robotics")]
pub use arm::{forward_kinematics, DhParameters, Elbow, Transform, TwoLinkArm};
#[cfg(feature = "robotics")]
pub use chassis::{Ackermann, Mecanum, SkidSteer, WheelSpeeds};
#[cfg(feature = "robotics")]
pub use drivers::{Esc, Quadrature, QuadratureScale, ServoMap};
#[cfg(feature = "robotics")]
pub use motion::{Pose, Twist};
#[cfg(all(feature = "robotics", feature = "geo"))]
pub use navigate::{obstacle_stop, Guidance, WaypointFollower};
#[cfg(feature = "robotics")]
pub use odometry::Odometry;
#[cfg(feature = "robotics")]
pub use safety::{EStop, Limits, SafetyGate, Watchdog};
