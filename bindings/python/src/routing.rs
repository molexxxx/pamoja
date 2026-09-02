//! Generated Python bindings for cost-aware mesh routing.
//!
//! These mirror the `pamoja-routing` Rust API: a table that learns the way to a
//! node from the traffic it already hears, and the per-packet decision of whether
//! to deliver, relay, or fall back to flooding.
//!
//! A router holds state across calls, so it is a class, fixed at
//! [`TABLE_CAPACITY`] routes because its Rust size is a const generic. A routing
//! decision crosses as a name and an optional next hop, so a caller reads it
//! without unpacking a tagged union.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_routing::{Forward, Router as CoreRouter};

/// The number of routes a routing table holds.
const TABLE_CAPACITY: usize = 64;

/// A learned way to reach one node.
#[gen_stub_pyclass]
#[pyclass]
pub struct Route {
    /// The node this route reaches.
    #[pyo3(get)]
    dst: u32,
    /// The neighbour to send a packet to on the way there.
    #[pyo3(get)]
    next_hop: u32,
    /// What the route costs, usually in hops.
    #[pyo3(get)]
    cost: u16,
}

/// A routing decision, and the neighbour it names when there is one.
#[gen_stub_pyclass]
#[pyclass]
pub struct ForwardDecision {
    /// What to do with the packet: `Deliver`, `Relay`, or `Flood`.
    #[pyo3(get)]
    action: String,
    /// The neighbour to unicast to, or `None` unless the action is `Relay`.
    #[pyo3(get)]
    next_hop: Option<u32>,
}

/// One node routing table, learned from the traffic the node hears.
#[gen_stub_pyclass]
#[pyclass]
pub struct Router {
    inner: CoreRouter<TABLE_CAPACITY>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Router {
    /// Creates an empty routing table for a node at `address`.
    #[new]
    fn new(address: u32) -> Self {
        Router {
            inner: CoreRouter::new(address),
        }
    }

    /// The address this router answers for.
    #[getter]
    fn address(&self) -> u32 {
        self.inner.address()
    }

    /// Learns a route from a packet that arrived, reporting whether it changed
    /// the table.
    fn observe(&mut self, origin: u32, via: u32, cost: u16) -> bool {
        self.inner.observe(origin, via, cost)
    }

    /// The neighbour to send a packet to on the way to `dst`, or `None`.
    fn next_hop(&self, dst: u32) -> Option<u32> {
        self.inner.next_hop(dst)
    }

    /// What the known route to `dst` costs, or `None` when none is known.
    fn cost(&self, dst: u32) -> Option<u16> {
        self.inner.cost(dst)
    }

    /// The whole route to `dst`, or `None` when none is known.
    fn route(&self, dst: u32) -> Option<Route> {
        self.inner.route(dst).map(|route| Route {
            dst: route.dst(),
            next_hop: route.next_hop(),
            cost: route.cost(),
        })
    }

    /// Decides what to do with a packet bound for `dst`.
    fn forward(&self, dst: u32) -> ForwardDecision {
        match self.inner.forward(dst) {
            Forward::Deliver => ForwardDecision {
                action: "Deliver".to_owned(),
                next_hop: None,
            },
            Forward::Relay(next_hop) => ForwardDecision {
                action: "Relay".to_owned(),
                next_hop: Some(next_hop),
            },
            Forward::Flood => ForwardDecision {
                action: "Flood".to_owned(),
                next_hop: None,
            },
        }
    }

    /// Forgets the route to `dst`, for example after it stops answering.
    fn forget(&mut self, dst: u32) {
        self.inner.forget(dst);
    }

    /// How many routes the table currently holds.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// How many routes the table can hold.
    #[getter]
    fn capacity(&self) -> usize {
        TABLE_CAPACITY
    }
}

/// Returns how many routes a routing table holds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn routing_table_capacity() -> usize {
    TABLE_CAPACITY
}
