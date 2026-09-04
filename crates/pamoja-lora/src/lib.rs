#![cfg_attr(not(any(test, feature = "std")), no_std)]

//! LoRa link math for the pamoja SDK.
//!
//! LoRa is the SDK's answer to reach: kilometres of range on license-free bands at
//! tiny power. That reach comes with a hard constraint, though. A LoRa transmission
//! occupies the channel for a duration fixed by the radio settings, and the regional
//! regulations a deployment lives under cap how much of the time a node may transmit,
//! typically around one percent. A node that ignores this is both illegal and, since
//! transmitting is the most expensive thing it does, wasteful of its battery.
//!
//! This crate provides the arithmetic that keeps a long-range node honest, with no
//! radio and no floating point:
//!
//! - [`LinkSettings`] - the spreading factor, bandwidth, coding rate, and frame
//!   options of a link, with [`airtime_us`](LinkSettings::airtime_us) for a payload's
//!   exact time on air and [`min_off_time_us`](LinkSettings::min_off_time_us) for the
//!   silence a duty-cycle limit then forces.
//!
//! The time-on-air calculation is the published LoRa formula, evaluated with exact
//! integer arithmetic, so the same numbers a deployment planner uses are available on
//! the node itself. It pairs naturally with the metered-link batch encoding in
//! `pamoja-codec` - pack a batch, then ask what it costs to send - and with the
//! duty-cycling in `pamoja-power`.
//!
//! This is the link-budget half of LoRa support; driving the radio arrives with the
//! hardware-I/O layer.
//!
//! # Examples
//!
//! ```
//! use pamoja_lora::LinkSettings;
//!
//! let link = LinkSettings::new(12, 125_000); // SF12, 125 kHz: maximum range
//! let airtime = link.airtime_us(20); // time on air of a 20-byte payload
//!
//! // At a 1% duty cycle, the node must then stay quiet for ninety-nine times as long.
//! assert_eq!(link.min_off_time_us(20, 10), airtime * 99);
//! ```

#[cfg(feature = "alloc")]
extern crate alloc;

mod link;

pub mod region;

pub use link::LinkSettings;
