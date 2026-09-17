//! A LoRaWAN relay, TS011-1.0.1: what an end device and a relay say to each other.
//!
//! A relay is an end device that listens on behalf of others. It sleeps, waking every
//! [`CadPeriodicity`] to scan a channel for radio activity. An end device out of a gateway's
//! reach first sends a wake-on-radio (WOR) frame with a preamble long enough to span that
//! sleep, which says where and how fast its LoRaWAN uplink will follow. The relay may answer
//! with a WOR ACK describing its own timing, so the next WOR frame can be short, then
//! listens for the uplink and forwards it to the network on port [`LA_FPORT_RELAY`] over its
//! own session. A downlink for the end device comes back the same way and goes out in the
//! device's RXR window.
//!
//! This module holds the frames and the arithmetic, sans IO like [`device`](crate::device):
//!
//! - [`WorKeys`]: the integrity and encryption keys an end device and a relay share, from
//!   the root key [`root_wor_s_key`] derives out of the network session key, section 4.
//! - [`Wor`]: the two WOR frames, [`wor_join_request`] and [`wor_uplink`], section 5.3.
//! - [`wor_ack`] and [`open_wor_ack`] with [`StateSync`]: the acknowledgment, section 6.2.
//! - [`ForwardedUplink`]: an uplink a relay forwards and its metadata, section 9.1.
//! - [`Synchronization`], [`unsynchronized_preamble_symbols`] and [`t_offset_ms`]: how long a
//!   WOR preamble must be and when it goes out, section 5.2 and appendix 1.
//!
//! The relay commands a network configures both sides with are
//! [`MacCommand`](crate::mac::MacCommand) variants, and each region's default WOR channels
//! are [`ChannelPlan::relay_channels`](pamoja_lora::region::ChannelPlan::relay_channels).
//!
//! # Examples
//!
//! An end device wakes a relay, which checks the frame and acknowledges it:
//!
//! ```
//! use pamoja_lorawan::relay::{
//!     open_wor_ack, wor_ack, wor_uplink, CadPeriodicity, CadToRx, Carrier, Forward, StateSync,
//!     Wor, XtalAccuracy,
//! };
//! use pamoja_lorawan::Session;
//!
//! let session = Session::new(0x2601_1BDA, [0x2B; 16], [0x99; 16]);
//! let keys = session.wor_keys();
//! let wor = Carrier::new(865_100_000, 3);
//! let ack = Carrier::new(865_300_000, 3);
//! let uplink = Carrier::new(868_100_000, 5);
//!
//! // The end device says its next uplink goes out on 868.1 MHz at DR5.
//! let frame = wor_uplink(&keys, session.dev_addr(), 1, uplink, wor)?;
//!
//! // The relay, holding the same keys, reads it back.
//! let Wor::Uplink(sealed) = Wor::parse(&frame)? else { unreachable!() };
//! assert_eq!(sealed.open(&keys, 1, wor)?, uplink);
//!
//! // And tells the device how it scans.
//! let state = StateSync {
//!     cad_to_rx: CadToRx::Symbols4,
//!     forward: Forward::Available,
//!     relay_data_rate: 5,
//!     xtal_accuracy: XtalAccuracy::Ppm30,
//!     cad_periodicity: CadPeriodicity::Ms500,
//!     t_offset_ms: 892,
//! };
//! let answer = wor_ack(&keys, session.dev_addr(), 1, ack, uplink, state)?;
//! assert_eq!(open_wor_ack(&answer, &keys, session.dev_addr(), 1, ack, uplink)?, state);
//! # Ok::<(), pamoja_lorawan::LorawanError>(())
//! ```

mod ack;
mod forward;
mod keys;
mod timing;
mod wor;

#[cfg(test)]
mod tests;

pub use ack::{open_wor_ack, wor_ack, CadPeriodicity, CadToRx, Forward, StateSync, XtalAccuracy};
pub use forward::{ForwardedUplink, UplinkMetadata, WorChannel, FORWARD_OVERHEAD};
pub use keys::{root_wor_s_key, WorKeys};
pub use timing::{
    t_offset_ms, unsynchronized_preamble_symbols, Synchronization, WorSlot,
    MIN_WOR_PREAMBLE_SYMBOLS,
};
pub use wor::{wor_join_request, wor_uplink, Carrier, SealedWor, Wor};

/// The port every message between a relay and its network uses, TS011-1.0.1 section 9.
pub const LA_FPORT_RELAY: u8 = 226;

/// How many end devices a relay verifies wake-on-radio frames for, RP002-1.0.5 section
/// 5.4.5.
pub const TRUSTED_ED_NUMBER: usize = 16;

/// How many WOR frames an end device sends without an acknowledgment before its uplink goes
/// out anyway, by default, RP002-1.0.5 section 5.4.5.
pub const WOR_ATTEMPTS_WO_ACK: u8 = 8;

/// The gap between a WOR frame, or its acknowledgment, and the LoRaWAN frame after it, in
/// microseconds.
pub const WOR_DATA_DELAY_US: u32 = 50_000;

/// The gap between a WOR frame and its acknowledgment, in microseconds.
pub const WOR_ACK_DELAY_US: u32 = 50_000;

/// The gap between a relay hearing an uplink and forwarding it, in microseconds.
pub const RELAY_FWD_DELAY_US: u32 = 50_000;

/// How long after an uplink an end device's RXR window opens at the latest, in
/// microseconds.
pub const RXR_DELAY_US: u32 = 18_000_000;

/// The length of a WOR frame ahead of a join request: its header and five bytes.
pub const WOR_JOIN_REQUEST_LEN: usize = 5;

/// The length of a WOR frame ahead of a Class A uplink.
pub const WOR_UPLINK_LEN: usize = 15;

/// The length of a WOR ACK.
pub const WOR_ACK_LEN: usize = 7;
