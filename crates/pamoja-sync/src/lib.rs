//! Offline-first store-and-forward buffering for pamoja.
//!
//! Devices on intermittent links append records while offline and drain them in
//! order when a link returns, so a node that is disconnected for hours or days
//! loses nothing. This crate provides durable first-in first-out implementations
//! of the core [`Store`](pamoja_core::Store) trait:
//!
//! - [`MemoryStore`] - a fast in-memory queue, optionally capacity-bounded so a
//!   full queue becomes an explicit backpressure signal.
//! - [`FileStore`] - a crash-safe on-disk queue that survives a restart or a
//!   power cut, optionally bounded so a long outage cannot fill the disk.
//!
//! Both buffer raw bytes, so an application pairs a [`Store`](pamoja_core::Store)
//! with a [`Codec`](https://docs.rs/pamoja-codec) to persist encoded payloads and
//! a [`Transport`](pamoja_core::Transport) to forward them when a link appears.
//! [`drain_to`] is the forward half: it publishes buffered records onto a
//! transport in order, removing each only after it is sent, so a failed link
//! loses nothing.

mod file;
mod forward;
mod memory;

pub use file::FileStore;
pub use forward::drain_to;
pub use memory::MemoryStore;
