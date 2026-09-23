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
//!
//! # Examples
//!
//! A rain gauge out of range for a night keeps its readings in an outbox, and hands them
//! on in order when the link returns. Here the link is an in-process loopback, so the
//! gateway on the other end can check what arrived.
//!
//! ```
//! use pamoja_core::{Receive, Store, Transport};
//! use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
//! use pamoja_sync::{drain_to, MemoryStore};
//!
//! # tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
//! let mut outbox = MemoryStore::new();
//! outbox.append(b"0.4").await?;
//! outbox.append(b"1.2").await?;
//!
//! let broker = LoopbackBroker::new();
//! let mut gateway = LoopbackTransport::new(broker.clone());
//! let mut link = LoopbackTransport::new(broker);
//! gateway.connect().await?;
//! gateway.subscribe("gauges/3/rain").await?;
//! link.connect().await?;
//!
//! let forwarded = drain_to(&mut outbox, &mut link, "gauges/3/rain").await?;
//! assert_eq!(forwarded, 2);
//! assert_eq!(outbox.len().await?, 0, "each record goes once it is sent");
//! assert_eq!(gateway.recv().await?.expect("the first").payload, b"0.4");
//! assert_eq!(gateway.recv().await?.expect("the second").payload, b"1.2");
//! # Ok::<(), pamoja_core::Error>(())
//! # }).unwrap();
//! ```

mod file;
mod forward;
mod memory;

pub use file::FileStore;
pub use forward::drain_to;
pub use memory::MemoryStore;
