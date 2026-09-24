//! Generated Node bindings for cost-aware mesh routing.
//!
//! These mirror the `pamoja-routing` Rust API: a table that learns the way to a
//! node from the traffic it already hears, and the per-packet decision of whether
//! to deliver, relay, or fall back to flooding.
//!
//! A router holds state across calls, so it is a class. Its table is sized when it
//! is built, since the Rust crate fixes its size with a const generic that cannot
//! reach JavaScript.

use crate::checked::{self, OptionalWhole};
use napi_derive::napi;
use pamoja_routing::{DynamicRouter, Forward};

/// A reasonable routing table size for a caller with no reason to choose one.
#[napi]
pub const ROUTING_DEFAULT_CAPACITY: u32 = 64;

/// What to do with a packet bound for a given node.
#[napi(string_enum)]
pub enum ForwardAction {
    /// The packet is for this node; hand it to the application.
    Deliver,
    /// A route is known; unicast the packet to the next hop reported alongside.
    Relay,
    /// No route is known; fall back to flooding the packet.
    Flood,
}

/// A routing decision, and the neighbor it names when there is one.
#[napi(object)]
pub struct ForwardDecision {
    /// What to do with the packet.
    pub action: ForwardAction,
    /// The neighbor to unicast to, or `null` unless the action is `Relay`.
    pub next_hop: Option<u32>,
}

/// A learned way to reach one node.
#[napi(object)]
pub struct Route {
    /// The node this route reaches.
    pub dst: u32,
    /// The neighbor to send a packet to on the way there.
    pub next_hop: u32,
    /// What the route costs, usually in hops.
    pub cost: u16,
}

impl From<pamoja_routing::Route> for Route {
    fn from(route: pamoja_routing::Route) -> Self {
        Route {
            dst: route.dst(),
            next_hop: route.next_hop(),
            cost: route.cost(),
        }
    }
}

/// One node routing table, learned from the traffic the node hears.
#[napi]
pub struct Router {
    inner: DynamicRouter,
}

#[napi]
impl Router {
    /// Creates an empty routing table for a node at `address`.
    ///
    /// `capacity` is how many routes to make room for, defaulting to
    /// [`ROUTING_DEFAULT_CAPACITY`]. A capacity of zero floods every unknown
    /// destination, which is the behavior with no table at all.
    #[napi(constructor)]
    pub fn new(address: checked::u32, capacity: Option<checked::u32>) -> Self {
        let capacity = capacity.get().unwrap_or(ROUTING_DEFAULT_CAPACITY);
        Self {
            inner: DynamicRouter::new(address.get(), capacity as usize),
        }
    }

    /// The address this router answers for.
    #[napi(getter)]
    pub fn address(&self) -> u32 {
        self.inner.address()
    }

    /// Learns a route from a packet that arrived.
    ///
    /// When a packet from `origin` comes in through neighbor `via` at `cost`,
    /// that neighbor is the way back. Returns whether the table changed.
    #[napi]
    pub fn observe(&mut self, origin: checked::u32, via: checked::u32, cost: checked::u16) -> bool {
        self.inner.observe(origin.get(), via.get(), cost.get())
    }

    /// Returns the neighbor to send a packet to on the way to `dst`, or `null`.
    #[napi]
    pub fn next_hop(&self, dst: checked::u32) -> Option<u32> {
        self.inner.next_hop(dst.get())
    }

    /// Returns what the known route to `dst` costs, or `null` when none is known.
    #[napi]
    pub fn cost(&self, dst: checked::u32) -> Option<u16> {
        self.inner.cost(dst.get())
    }

    /// Returns the whole route to `dst`, or `null` when none is known.
    #[napi]
    pub fn route(&self, dst: checked::u32) -> Option<Route> {
        self.inner.route(dst.get()).map(Route::from)
    }

    /// Lists the routes the table holds, each once, in the order the table holds
    /// them rather than sorted by destination or cost.
    #[napi]
    pub fn routes(&self) -> Vec<Route> {
        self.inner.routes().map(Route::from).collect()
    }

    /// Decides what to do with a packet bound for `dst`.
    #[napi]
    pub fn forward(&self, dst: checked::u32) -> ForwardDecision {
        match self.inner.forward(dst.get()) {
            Forward::Deliver => ForwardDecision {
                action: ForwardAction::Deliver,
                next_hop: None,
            },
            Forward::Relay(next_hop) => ForwardDecision {
                action: ForwardAction::Relay,
                next_hop: Some(next_hop),
            },
            Forward::Flood => ForwardDecision {
                action: ForwardAction::Flood,
                next_hop: None,
            },
        }
    }

    /// Forgets the route to `dst`, for example after it stops answering.
    #[napi]
    pub fn forget(&mut self, dst: checked::u32) {
        self.inner.forget(dst.get());
    }

    /// How many routes the table currently holds.
    #[napi(getter)]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    /// Whether the table has learned nothing yet.
    #[napi(getter)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// How many routes the table can hold.
    #[napi(getter)]
    pub fn capacity(&self) -> u32 {
        self.inner.capacity() as u32
    }
}
