#![cfg_attr(not(test), no_std)]

//! CAN bus framing for the pamoja SDK.
//!
//! CAN is the bus that connects the moving parts of a machine: motor controllers, servos,
//! battery management, and the engines, gensets, and farm equipment that speak J1939 on
//! top of it. It is how a robot or a vehicle's pieces talk to each other reliably over a
//! short, noisy two-wire link, which is why it is the SDK's path to actuators and to the
//! diesel-and-hydraulic world of rural machinery.
//!
//! This crate is the byte layer for that, with no controller and no allocation:
//!
//! - [`CanId`] - a standard 11-bit or extended 29-bit identifier, always masked to width.
//! - [`Frame`] - a classic CAN 2.0 frame, a CAN-FD frame at the discrete CAN-FD lengths,
//!   or a remote frame, with [`len_to_dlc`] and [`dlc_to_len`] for the length encoding
//!   CAN-FD uses above eight bytes.
//! - [`J1939Id`] - the priority, parameter group, and addresses J1939 packs into a 29-bit
//!   identifier, decoded from one and composed back into one.
//!
//! The controller hardware handles the wire itself (arbitration, bit timing, the frame
//! CRC); this is the identifier and payload layer above it, the part an application
//! actually reasons about. Driving a real controller arrives with the hardware-I/O layer.
//!
//! # Examples
//!
//! ```
//! use pamoja_can::{CanId, Frame, J1939Id};
//!
//! // Build a classic frame for a motor controller.
//! let frame = Frame::new(CanId::standard(0x20A), &[0x01, 0xF4]).unwrap();
//! assert_eq!(frame.dlc(), 2);
//!
//! // Decode an engine-speed broadcast from a J1939 genset.
//! let message = J1939Id::from_id(CanId::extended(0x0CF0_0400)).unwrap();
//! assert_eq!(message.pgn(), 61_444);
//! assert!(message.is_broadcast());
//! ```

mod error;
mod frame;
mod id;
mod j1939;
mod signals;

pub use error::CanError;
pub use frame::{dlc_to_len, len_to_dlc, Frame};
pub use id::CanId;
pub use j1939::{priority, J1939Id, BROADCAST_ADDRESS};
pub use signals::{Signals, NOT_AVAILABLE};
