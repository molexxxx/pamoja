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
//! - `linux`, with the `linux` feature - opening a radio on a Linux board over spidev and
//!   the GPIO character device, with a plain error on every other platform.
//! - `mesh`, with the `std` feature - a pamoja transport over a radio, carrying topics in
//!   pamoja-mesh frames that each node relays onward, under the duty-cycle guard.
//!
//! # Examples
//!
//! ```
//! use pamoja_lora::LinkSettings;
//! use pamoja_radios::sx126x::{command, config};
//!
//! // The commands that tune an SX1262 to 868.1 MHz for SF9 at 125 kHz.
//! let link = LinkSettings::new(9, 125_000);
//! let tune = command::set_rf_frequency(config::frequency_word(868_100_000));
//! let modulation = config::LoraModulation::from_link(&link).expect("125 kHz is an SX126x bandwidth");
//! let modulate = command::set_lora_modulation_params(modulation);
//!
//! assert_eq!(tune.as_bytes(), [0x86, 0x36, 0x41, 0x99, 0x9A]);
//! assert_eq!(modulate.as_bytes(), [0x8B, 0x09, 0x04, 0x01, 0x00]);
//! ```

pub mod duty;
#[cfg(feature = "linux")]
pub mod linux;
#[cfg(feature = "std")]
pub mod mesh;
#[cfg(feature = "embedded-hal")]
pub mod radio;
pub mod sx126x;
pub mod sx127x;
