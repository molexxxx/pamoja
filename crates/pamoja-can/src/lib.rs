#![cfg_attr(not(any(test, feature = "bus")), no_std)]
// The crate doc links the bus types, which a build without the `bus` feature lacks.
#![cfg_attr(not(feature = "bus"), allow(rustdoc::broken_intra_doc_links))]

//! CAN for the pamoja SDK: the frames, J1939, and a node on a bus.
//!
//! CAN is the bus that connects the moving parts of a machine: motor controllers, servos,
//! battery management, and the engines, gensets, and farm equipment that speak J1939 on
//! top of it. It is how a robot or a vehicle's pieces talk to each other reliably over a
//! short, noisy two-wire link, which is why it is the SDK's path to actuators and to the
//! diesel-and-hydraulic world of rural machinery.
//!
//! The byte layer needs no controller and no allocation:
//!
//! - [`CanId`] - a standard 11-bit or extended 29-bit identifier, always masked to width.
//! - [`Frame`] - a classic CAN 2.0 frame, a CAN-FD frame at the discrete CAN-FD lengths,
//!   or a remote frame, with [`len_to_dlc`] and [`dlc_to_len`] for the length encoding
//!   CAN-FD uses above eight bytes.
//! - [`J1939Id`] - the priority, parameter group, and addresses J1939 packs into a 29-bit
//!   identifier, decoded from one and composed back into one.
//! - [`Signals`] - the eight data bytes of a J1939 frame, read and written by the offsets
//!   its parameter group publishes, starting every signal as not available.
//!
//! The controller hardware handles the wire itself (arbitration, bit timing, the frame
//! CRC); this is the identifier and payload layer above it, the part an application
//! actually reasons about.
//!
//! With the `bus` feature, [`bus::CanBus`] is one node's place on a bus: a bus simulated
//! inside the program, or with the `linux` feature, a kernel interface such as `can0`
//! through SocketCAN. A node hears every frame the others send and none of its own, and keeps
//! the ones its [`bus::Filter`]s pass, by identifier or by J1939 parameter group.
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

#[cfg(feature = "bus")]
pub mod bus;
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
