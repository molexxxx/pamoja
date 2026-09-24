#![cfg_attr(not(test), no_std)]

//! Power-aware scheduling for the pamoja SDK.
//!
//! A node on a battery or a solar panel lives or dies by how much it sleeps. This
//! crate holds the scheduling math that keeps such a node alive, with no runtime
//! and no hardware assumptions, so the same decisions can be made on a
//! microcontroller and verified on a server:
//!
//! - [`DutyCycle`] - trade wakefulness for battery life with a repeating
//!   wake/sleep schedule, and read back the duty fraction as a power proxy.
//! - [`PowerPlan`] - an energy-aware governor that stretches the sampling interval
//!   as the battery drains, picking a [`PowerMode`] from the state of charge and
//!   easing off when the panel is charging.
//!
//! The state of charge fed to a [`PowerPlan`] is noisy in the field. Its
//! [`next_mode`](PowerPlan::next_mode) keeps a node from flapping between modes at a
//! threshold: it drops to a lower mode at the threshold but climbs back only once the
//! charge clears the threshold by the plan's hysteresis margin. Smoothing the charge
//! first, for example with a `Smoother` from `pamoja-kit`, steadies it further.
//!
//! The crate is `no_std` and allocation-free.
//!
//! # Examples
//!
//! ```
//! use core::time::Duration;
//! use pamoja_power::{PowerMode, PowerPlan};
//!
//! let plan = PowerPlan::new(
//!     Duration::from_secs(60), // sample each minute when healthy
//!     Duration::from_secs(600), // back off to ten minutes to conserve
//!     Duration::from_secs(3600), // once an hour when critically low
//! );
//!
//! assert_eq!(plan.mode(0.8), PowerMode::Active);
//! assert_eq!(plan.interval(0.1), Duration::from_secs(3600));
//! ```

mod duty;
mod plan;

pub use duty::DutyCycle;
pub use plan::{PowerMode, PowerPlan, DEFAULT_HYSTERESIS};
