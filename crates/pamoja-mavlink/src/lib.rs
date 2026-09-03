#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]

//! The MAVLink wire protocol for the pamoja SDK.
//!
//! MAVLink is the language drones speak: PX4 and ArduPilot autopilots and MAVSDK ground
//! stations all exchange MAVLink frames, so talking to a vehicle means putting exactly the
//! right bytes on the wire and trusting the bytes that come back. This crate is that byte
//! layer, hand-written from the [MAVLink specification](https://mavlink.io) and pinned to
//! its reference values rather than guessed from memory:
//!
//! - [`crc16_mcrf4xx`] - the CRC-16/MCRF4XX every frame carries, the checksum that lets a
//!   receiver reject a frame mangled in transit, anchored to the catalogue check value.
//! - [`Frame`] - the v1 and v2 frame on the wire, which both [assembles](Frame::encode_v2)
//!   a frame to send and [parses](Frame::parse) one received, verifying the checksum and
//!   the per-message [`message_crc_extra`] seed so a corrupt or mismatched frame never
//!   reaches the application.
//! - [`Parser`] - a streaming parser that turns the bytes a link delivers into whole
//!   frames, resynchronizing on noise so a serial port or UDP socket just works.
//! - [`signing`] - MAVLink 2 message signing: the SHA-256 signature and the monotonic
//!   timestamp that let a ground station trust a command came from the vehicle it expects
//!   and was not replayed.
//! - [`dialect`] - a broad, typed slice of the common dialect (HEARTBEAT, the command,
//!   parameter, and mission protocols, and core telemetry), plus message shapes as data:
//!   a [`MessageDescriptor`](dialect::MessageDescriptor) gives any message's bytes named
//!   fields, and a [builder](dialect::MessageDescriptorBuilder) describes one this crate
//!   does not type, so a vendor or private dialect is usable at runtime.
//! - [`protocol`] - the mission, command, and offboard exchanges as pure, allocation-free
//!   state machines: the rules of order, matching, and retransmission that turn single
//!   messages into a real conversation with an autopilot, with no IO of their own. Each
//!   machine also takes a [`Frame`] at a time and hands back the frame to send, so a
//!   caller holding a link writes no decoding or dispatch of its own.
//!
//! The protocol core is `no_std` and allocation-free, so the same framing runs on a
//! microcontroller flight controller. The default `std` feature adds the async layer: the
//! byte-stream link seam and an in-process software-in-the-loop autopilot ([`link`]), the
//! [`vehicle`] device model that presents an autopilot as a pamoja `Device`, and the real
//! [`drivers`] (UDP, TCP, and serial behind the `serial` feature) that carry MAVLink to a real
//! or simulated autopilot.
//!
//! # Examples
//!
//! ```
//! use pamoja_mavlink::dialect::{Heartbeat, Message};
//! use pamoja_mavlink::{Frame, Header};
//!
//! // Announce this node as an onboard controller.
//! let heartbeat = Heartbeat {
//!     custom_mode: 0,
//!     type_: 18, // MAV_TYPE_ONBOARD_CONTROLLER
//!     autopilot: 0,
//!     base_mode: 0,
//!     system_status: 4, // MAV_STATE_ACTIVE
//!     mavlink_version: 3,
//! };
//!
//! // Encode it into a v2 frame, then read it back the way a peer would.
//! let mut payload = [0u8; 255];
//! let len = heartbeat.encode(&mut payload);
//! let frame = Frame::encode_v2(Header::new(1, 1, 0), Heartbeat::ID, &payload[..len], Heartbeat::CRC_EXTRA)?;
//!
//! let received = Frame::parse(frame.as_bytes(), Heartbeat::CRC_EXTRA)?;
//! let decoded = Heartbeat::decode(received.payload())?;
//! assert_eq!(decoded.system_status, 4);
//! # Ok::<(), pamoja_mavlink::MavlinkError>(())
//! ```

#[cfg(feature = "alloc")]
extern crate alloc;

mod crc;
pub mod dialect;
mod error;
mod frame;
mod parser;
pub mod protocol;
pub mod signing;

#[cfg(feature = "std")]
pub mod link;

#[cfg(feature = "std")]
pub mod vehicle;

#[cfg(feature = "std")]
pub mod drivers;

pub use crc::{accumulate, checksum, crc16_mcrf4xx, message_crc_extra};
pub use error::{MavlinkError, Result};
pub use frame::{
    Frame, Header, Version, IFLAG_SIGNED, MAGIC_V1, MAGIC_V2, MAX_FRAME, MAX_PAYLOAD, SIGNATURE_LEN,
};
pub use parser::Parser;
pub use signing::{Signer, Verifier};

#[cfg(feature = "std")]
pub use vehicle::{Report, Setpoint, Vehicle};

#[cfg(feature = "std")]
pub use drivers::{TcpLink, UdpLink};

#[cfg(feature = "serial")]
pub use drivers::SerialLink;
