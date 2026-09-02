#![cfg_attr(not(test), no_std)]

//! Cost-aware mesh routing for the pamoja SDK.
//!
//! Flooding gets a packet across a mesh by having every node rebroadcast it, which always
//! works but is expensive: every node spends airtime and power on every packet. Once a
//! mesh has settled, most traffic goes to a few known places, and a node that remembers
//! the way can forward a packet to just the right neighbour instead of shouting it to the
//! whole network. That is routing, and on the cheap radios this SDK targets the saving in
//! airtime and battery is the difference between a network that lasts and one that does
//! not.
//!
//! This crate is the decision layer for that, as pure logic with no radio and no
//! allocation:
//!
//! - [`Router`] - a fixed-size table that learns the way to a node from the traffic it
//!   already hears: when a packet from a distant node arrives via a neighbour, that
//!   neighbour is the way back, at the cost the packet reports. The table keeps the
//!   cheapest way it knows to each destination and forgets the most expensive when it runs
//!   out of room.
//! - [`Router::forward`] - the per-packet decision: deliver a packet that is for this
//!   node, [relay](Forward::Relay) one toward a known destination, or [flood](Forward::Flood)
//!   when there is no route yet. That last case is where this layer hands back to the
//!   flooding in `pamoja-mesh`, so routing is an optimisation over flooding, never a
//!   single point of failure.
//!
//! Nodes are identified by the same address a [`pamoja-mesh`](https://docs.rs/pamoja-mesh)
//! frame carries, so the two compose directly: learn from a received frame's source and
//! the neighbour it came from, then ask [`forward`](Router::forward) where the next one
//! should go.
//!
//! # Examples
//!
//! ```
//! use pamoja_routing::{Forward, Router};
//!
//! let mut router: Router<16> = Router::new(0x01);
//!
//! // We hear node 0x09's traffic arrive via neighbour 0x05, two hops out.
//! router.observe(0x09, 0x05, 2);
//! assert_eq!(router.forward(0x09), Forward::Relay(0x05));
//!
//! // A cheaper way to 0x09 turns up via neighbour 0x07; the router prefers it.
//! router.observe(0x09, 0x07, 1);
//! assert_eq!(router.forward(0x09), Forward::Relay(0x07));
//!
//! // With no route to 0x20 yet, the router falls back to flooding.
//! assert_eq!(router.forward(0x20), Forward::Flood);
//! ```

// `cfg(test)` already builds this crate against std, so the runtime-sized table compiles
// for the test run whether or not the feature is on, and its tests always execute.
#[cfg(any(feature = "alloc", test))]
extern crate alloc;

mod router;

#[cfg(any(feature = "alloc", test))]
pub use router::DynamicRouter;
pub use router::{Forward, Route, Router};
