//! The MAVLink service protocols as pure, allocation-free state machines.
//!
//! A [`Frame`](crate::Frame) carries one message; a real exchange with an autopilot is a
//! sequence of them with rules about order, matching, and retransmission. This module holds
//! those rules as sans-IO logic: each machine is fed a decoded incoming message and returns
//! the next message to send and a state transition, with no IO, no timers, and no allocation.
//! The timing policy (when to time out and retransmit) is left to the caller that drives a
//! machine over a link, so the same logic runs on a microcontroller and under an async host
//! runtime alike.
//!
//! - [`command`] - the command protocol: send a command, match its acknowledgement, treat an
//!   in-progress result as "keep waiting", and count retries.
//! - [`mission`] - the mission (plan) transfer protocol, as a [`MissionSender`]
//!   that answers item requests and a [`MissionReceiver`] that
//!   requests and collects items; a vehicle and a ground station play opposite roles with the
//!   same two machines.
//! - [`offboard`] - the `type_mask` builder and setpoint constructors for offboard position,
//!   velocity, and acceleration control.
//! - [`frames`] - the same machines driven by a [`Frame`](crate::Frame) at a time: the
//!   payload decoding, message-id dispatch, and reply encoding a caller with a link would
//!   otherwise write around every machine.

pub mod command;
pub mod frames;
pub mod mission;
pub mod offboard;

pub use command::{AckOutcome, CommandProtocol};
pub use frames::{ReceiverStep, SenderStep};
pub use mission::{MissionReceiver, MissionSender, ReceiverAction};
pub use offboard::TypeMask;

/// The default number of times a request is retransmitted before a transfer is abandoned, as
/// the mission protocol recommends.
pub const MAX_RETRIES: u8 = 5;
