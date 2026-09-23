#![cfg_attr(not(any(test, feature = "std")), no_std)]

//! LoRa radio drivers for the pamoja SDK.
//!
//! `pamoja-lora` works out what a link costs and how far it reaches; this crate puts a
//! radio on the air to match. Each chip family has a module in two halves. The first is
//! the chip's commands or registers and its decoders: the bytes that configure it, the
//! interrupt and status bits it answers with, the signal levels of a received packet,
//! and the frequency, modulation, packet, and power settings that a
//! [`LinkSettings`](pamoja_lora::LinkSettings) and a channel plan turn into. That half
//! needs no bus and no allocation, so a test, a language binding, or a protocol
//! analyzer can use it without a radio. The second half is an `embedded-hal` driver
//! that talks to the chip over SPI and transmits and receives frames.
//!
//! - [`sx126x`] - Semtech's SX1261, SX1262, and LLCC68, from the SX1261/2 datasheet.
//! - [`sx127x`] - Semtech's SX1276, SX1277, SX1278, and SX1279, and modules such as the
//!   RFM95W, from the SX1276/77/78/79 datasheet.
//! - [`radio`] - one radio of either family behind the same calls: configure from a
//!   carrier, a link, and an output power, then transmit, receive, and listen.
//! - [`duty`] - a guard that holds a radio silent for the off time a regional duty-cycle
//!   limit requires after each transmission.
//! - `lorawan`, with the `lorawan` feature - a LoRaWAN Class A node: an end device from
//!   `pamoja-lorawan` driving a radio, with its receive windows opened on time.
//! - `relay`, with the `lorawan` feature - a LoRaWAN relay driving a radio: it scans for
//!   wake-on-radio frames, answers them, and forwards the uplinks behind them.
//! - `linux`, with the `linux` feature - opening a radio on a Linux board over spidev and
//!   the GPIO character device, with a plain error on every other platform.
//! - `mesh`, with the `std` feature - a pamoja transport over a radio, carrying topics in
//!   pamoja-mesh frames that each node relays onward, under the duty-cycle guard.
//!
//! # Examples
//!
//! A node on EU868 works out how it may transmit before it touches a radio: the link its
//! data rate names, how hard its amplifier may drive through its antenna under the plan's
//! ceiling, and how long it must stay silent after each reading.
//!
//! ```
//! use pamoja_lora::budget::{Decibels, LinkBudget};
//! use pamoja_lora::region::Region;
//! use pamoja_radios::duty::DutyCycle;
//! use pamoja_radios::radio::{RadioConfig, SyncWord};
//! use pamoja_radios::sx126x::config::{PowerAmplifier, TxPower};
//!
//! let eu868 = Region::Eu868.plan();
//! let carrier = 868_100_000;
//! let link = eu868.link_settings(3).expect("DR3 is a LoRa data rate");
//!
//! // A 2.15 dBi whip on half a decibel of pigtail: the amplifier drives only as hard as
//! // keeps the radiated power under the plan's ceiling at that carrier.
//! let whip = LinkBudget {
//!     transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
//!     transmit_cable_loss_db: Decibels::from_tenths(5),
//!     ..LinkBudget::default()
//! };
//! let ceiling = Decibels::from_db(eu868.max_eirp_dbm(carrier).into());
//! let power = TxPower::under_ceiling(PowerAmplifier::HighPower, &whip, ceiling);
//! assert_eq!(power.setting_dbm, 14);
//!
//! // The configuration an SX126x or an SX127x takes, as a LoRaWAN device uses it.
//! let config = RadioConfig::new(carrier, link, power.setting_dbm).lorawan_device();
//! assert_eq!(config.link.spreading_factor(), 9);
//! assert_eq!(config.sync_word, SyncWord::Public);
//!
//! // Under the band's one percent duty cycle, a ten-byte reading owes ninety-nine times
//! // its airtime in silence before the next one may start.
//! let mut guard = DutyCycle::new(10);
//! let airtime = guard.transmitted(0, &link, 10);
//! assert_eq!(guard.wait_us(airtime), 99 * airtime);
//! ```

pub mod duty;
#[cfg(feature = "linux")]
pub mod linux;
#[cfg(feature = "lorawan")]
pub mod lorawan;
#[cfg(feature = "std")]
pub mod mesh;
#[cfg(feature = "embedded-hal")]
pub mod radio;
#[cfg(feature = "lorawan")]
pub mod relay;
pub mod sx126x;
pub mod sx127x;
pub mod sx1302;
