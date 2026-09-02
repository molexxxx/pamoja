#![cfg_attr(not(test), no_std)]

//! Mesh packet framing for the pamoja SDK.
//!
//! When the fixed infrastructure is gone or was never there, devices have to carry each
//! other's traffic. A flood-warning sensor on a riverbank, a handheld in a search team, a
//! solar node on a rooftop: each can hear only its nearest neighbours over a cheap radio,
//! yet a message has to cross the whole area. The answer is a mesh, where every node
//! relays what it hears so a packet hops node to node until it arrives. This is the
//! messaging backbone for exactly the places the SDK is built for, and it rides on the
//! cheapest radios there are, the connectionless ESP-NOW of an ESP32 swarm and the
//! pennies-per-node nRF24, neither of which gives you addressing, hops, or integrity on
//! its own.
//!
//! This crate is that missing layer, as pure logic with no radio and no allocation:
//!
//! - [`Frame`] - an addressed packet: a source and destination node, a sequence number,
//!   a hop limit, a payload, and a checksum. It [encodes](Frame::new) a packet to send
//!   and [parses](Frame::parse) one received, rejecting anything a noisy radio mangled.
//!   The checksum deliberately covers everything except the hop limit, so a packet's
//!   integrity check is end to end and survives relaying unchanged.
//! - [`Frame::relayed`] - the forwarding primitive: the same packet with one hop spent,
//!   or nothing once its hops run out, which is what stops a flood from circulating
//!   forever.
//! - [`SeenCache`] - the duplicate suppressor: a fixed-size memory of recently seen
//!   packets, so a node relays each packet once however many copies reach it across the
//!   mesh. Without it a flood multiplies without bound.
//!
//! Together these are enough to build a flood: receive a frame, drop it if it is a
//! duplicate, use it if it is for you, and relay it onward if it still has hops. Driving
//! an actual radio arrives with the hardware-I/O layer; this is the packet half ahead of
//! it.
//!
//! # Examples
//!
//! ```
//! use pamoja_mesh::{Frame, SeenCache};
//!
//! // A flood-warning sensor broadcasts a reading into the mesh.
//! let reading = Frame::broadcast(0x1234_5678, 1, b"level=high")?;
//! let on_air = reading.as_bytes();
//!
//! // A neighbour receives it, checks it has not already seen this packet, and reads it.
//! let mut seen: SeenCache<32> = SeenCache::new();
//! let received = Frame::parse(on_air)?;
//! assert!(received.is_broadcast());
//! assert_eq!(received.payload(), b"level=high");
//! assert!(seen.record(received.dedup_key())); // true: new to us
//!
//! // It forwards the packet one hop further into the mesh.
//! let forwarded = received.relayed().unwrap();
//! assert_eq!(forwarded.hop_limit(), received.hop_limit() - 1);
//!
//! // The same packet arriving again by another path is recognised and dropped.
//! assert!(!seen.record(received.dedup_key()));
//! # Ok::<(), pamoja_mesh::MeshError>(())
//! ```

// `cfg(test)` already builds this crate against std, so the runtime-sized cache compiles
// for the test run whether or not the feature is on, and its tests always execute.
#[cfg(any(feature = "alloc", test))]
extern crate alloc;

mod crc;
mod error;
mod frame;
mod seen;

pub use crc::{crc16, Crc16};
pub use error::MeshError;
pub use frame::{Frame, BROADCAST};
#[cfg(any(feature = "alloc", test))]
pub use seen::DynamicSeenCache;
pub use seen::SeenCache;
