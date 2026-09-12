//! Semtech SX1302 and SX1303 concentrators.
//!
//! A transceiver listens to one channel at a time. A concentrator listens to eight at once,
//! across every spreading factor, which is what separates a gateway from a node. That is also
//! why it is driven differently: the SX126x and SX127x take commands, while this chip is a
//! map of registers, most of them a few bits inside a byte shared with their neighbors.
//!
//! Nothing works until two microcontrollers inside the chip are given firmware. The gain
//! control one sets the front end, the arbiter shares the radios between the receivers, and
//! both come up empty, so a concentrator that has been powered but not loaded reports a
//! healthy version register and hears nothing at all.
//!
//! This module carries that as data:
//!
//! - [`spi`] - the transfers the chip answers, byte for byte, and how a long write is split.
//! - [`register`] - where the blocks are and which bits inside a byte each register owns.
//! - [`chip`] - the version that says a board is answering, and the part number that says
//!   whether it is an SX1302 or an SX1303.
//! - [`firmware`] - the order the two microcontrollers are loaded in, and what to check
//!   afterwards.
//! - [`rx`] - the packets the receive buffer holds, and what the receiver made of each.
//! - [`tx`] - where a payload is written, how far ahead of its window a send starts, and
//!   the three ways a transmission is triggered.
//! - [`lbt`] - the commands for the SX1261 beside the concentrator, which listens to a
//!   channel before the gateway is allowed to talk on it.
//!
//! None of it opens a bus, so a frame can be checked against the reference implementation,
//! or against a capture from a working gateway, with no hardware present.
//!
//! The register map, the transfer framing, and the load sequence follow Semtech's
//! `sx1302_hal`, which is BSD 3-Clause. The notice is on the
//! [notices page](https://pamoja.molex.cloud/docs/about/notices.html).
//!
//! # Examples
//!
//! Asking a board what it is, without a bus in sight.
//!
//! ```
//! use pamoja_radios::sx1302::{chip, register, spi};
//!
//! // The version register is read like anything else: target, address, two dummy bytes.
//! let asked = spi::read(spi::TARGET_CONCENTRATOR, register::COMMON_VERSION.address);
//! assert_eq!(asked, [0x00, 0x56, 0x06, 0x00, 0x00]);
//!
//! // A concentrator that is powered and wired the right way round answers with one value.
//! assert!(chip::answers(0x10));
//! assert!(!chip::answers(0x00));
//!
//! // The part number takes two transfers, because it lives in one-time programmable memory.
//! let [select, read] = chip::identify();
//! assert_eq!(select, chip::Step::Write(register::OTP_BYTE_ADDR, 0xd0));
//! assert_eq!(read, chip::Step::Read(register::OTP_RD_DATA));
//!
//! assert_eq!(chip::Model::of(0x03), chip::Model::Sx1303);
//! ```

pub mod chip;
pub mod firmware;
pub mod lbt;
pub mod register;
pub mod rx;
pub mod spi;
pub mod tx;
