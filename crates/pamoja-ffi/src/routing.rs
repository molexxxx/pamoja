//! The C ABI for cost-aware mesh routing.
//!
//! These functions wrap [`pamoja_routing`] for callers that reach the SDK through
//! the flat C boundary: a table that learns the way to a node from the traffic it
//! already hears, and the per-packet decision of whether to deliver, relay, or
//! fall back to flooding.
//!
//! A router holds state across calls, so it crosses as an opaque handle. Its table
//! is sized when it is built rather than by the const generic the Rust crate uses,
//! since a const generic cannot cross a C ABI at all;
//! [`PAMOJA_ROUTING_DEFAULT_CAPACITY`] is what a caller with no reason to choose
//! should pass.

use pamoja_routing::{DynamicRouter, Forward};

/// A reasonable routing table size for a caller with no reason to choose one.
pub const PAMOJA_ROUTING_DEFAULT_CAPACITY: usize = 64;

/// An opaque handle to one node routing table.
///
/// Create it with [`pamoja_router_new`], teach it with
/// [`pamoja_router_observe`], and release it with [`pamoja_router_free`].
pub struct PamojaRouter {
    router: DynamicRouter,
}

/// A learned way to reach one node.
///
/// Every field is a scalar, so this crosses the boundary by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaRoute {
    /// The node this route reaches.
    pub dst: u32,
    /// The neighbour to send a packet to on the way there.
    pub next_hop: u32,
    /// What the route costs, usually in hops.
    pub cost: u16,
}

/// What to do with a packet bound for a given node.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaForward {
    /// The packet is for this node; hand it to the application.
    Deliver = 0,
    /// A route is known; unicast the packet to the next hop reported alongside.
    Relay = 1,
    /// No route is known; fall back to flooding the packet.
    Flood = 2,
}

/// Creates an empty routing table for a node.
///
/// # Arguments
///
/// * `address` - the address of this node, which is what
///   [`pamoja_router_forward`] recognises as a local delivery.
/// * `capacity` - how many routes to make room for; pass
///   [`PAMOJA_ROUTING_DEFAULT_CAPACITY`] when there is no reason to choose. A
///   capacity of zero is allowed and makes every unknown destination flood.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_router_free`].
#[no_mangle]
pub extern "C" fn pamoja_router_new(address: u32, capacity: usize) -> *mut PamojaRouter {
    Box::into_raw(Box::new(PamojaRouter {
        router: DynamicRouter::new(address, capacity),
    }))
}

/// Returns the address a router answers for.
///
/// # Returns
///
/// The node address, or 0 if `router` is null.
///
/// # Safety
///
/// `router` must be a live handle from [`pamoja_router_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_router_address(router: *const PamojaRouter) -> u32 {
    if router.is_null() {
        return 0;
    }
    (*router).router.address()
}

/// Learns a route from a packet that arrived.
///
/// When a packet from a distant node comes in via a neighbour, that neighbour is
/// the way back to it. The table keeps the cheapest way it knows to each node, and
/// when full gives up the most expensive route to make room for a cheaper one.
///
/// # Arguments
///
/// * `router` - the routing table.
/// * `origin` - the node the packet came from.
/// * `via` - the neighbour it arrived through.
/// * `cost` - what that path costs, usually a hop count.
///
/// # Returns
///
/// `true` if the table changed, or `false` if it already knew a route at least
/// this cheap, had no room for one this expensive, or `router` is null.
///
/// # Safety
///
/// `router` must be a live handle from [`pamoja_router_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_router_observe(
    router: *mut PamojaRouter,
    origin: u32,
    via: u32,
    cost: u16,
) -> bool {
    if router.is_null() {
        return false;
    }
    (*router).router.observe(origin, via, cost)
}

/// Returns the neighbour to send a packet to on the way to a node.
///
/// # Arguments
///
/// * `router` - the routing table.
/// * `dst` - the node to reach.
/// * `out_next_hop` - receives the neighbour address.
///
/// # Returns
///
/// `true` when a route is known, with `*out_next_hop` written, or `false`
/// otherwise.
///
/// # Safety
///
/// `router` must be a live handle from [`pamoja_router_new`], or null, and
/// `out_next_hop` must point to a writable `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_router_next_hop(
    router: *const PamojaRouter,
    dst: u32,
    out_next_hop: *mut u32,
) -> bool {
    if router.is_null() || out_next_hop.is_null() {
        return false;
    }
    match (*router).router.next_hop(dst) {
        Some(next_hop) => {
            *out_next_hop = next_hop;
            true
        }
        None => false,
    }
}

/// Returns what the known route to a node costs.
///
/// # Arguments
///
/// * `router` - the routing table.
/// * `dst` - the node to reach.
/// * `out_cost` - receives the cost.
///
/// # Returns
///
/// `true` when a route is known, with `*out_cost` written, or `false` otherwise.
///
/// # Safety
///
/// `router` must be a live handle from [`pamoja_router_new`], or null, and
/// `out_cost` must point to a writable `uint16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_router_cost(
    router: *const PamojaRouter,
    dst: u32,
    out_cost: *mut u16,
) -> bool {
    if router.is_null() || out_cost.is_null() {
        return false;
    }
    match (*router).router.cost(dst) {
        Some(cost) => {
            *out_cost = cost;
            true
        }
        None => false,
    }
}

/// Returns the whole route to a node.
///
/// # Arguments
///
/// * `router` - the routing table.
/// * `dst` - the node to reach.
/// * `out_route` - receives the route.
///
/// # Returns
///
/// `true` when a route is known, with `*out_route` filled in, or `false`
/// otherwise.
///
/// # Safety
///
/// `router` must be a live handle from [`pamoja_router_new`], or null, and
/// `out_route` must point to a writable [`PamojaRoute`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_router_route(
    router: *const PamojaRouter,
    dst: u32,
    out_route: *mut PamojaRoute,
) -> bool {
    if router.is_null() || out_route.is_null() {
        return false;
    }
    match (*router).router.route(dst) {
        Some(route) => {
            *out_route = PamojaRoute {
                dst: route.dst(),
                next_hop: route.next_hop(),
                cost: route.cost(),
            };
            true
        }
        None => false,
    }
}

/// Decides what to do with a packet bound for a node.
///
/// # Arguments
///
/// * `router` - the routing table.
/// * `dst` - the node the packet is addressed to.
/// * `out_next_hop` - receives the neighbour to unicast to, written only when the
///   answer is [`PamojaForward::Relay`].
///
/// # Returns
///
/// [`PamojaForward::Deliver`] when the packet is for this node,
/// [`PamojaForward::Relay`] when a route is known, or [`PamojaForward::Flood`]
/// when none is, which hands the packet back to the flooding layer. A null router
/// answers [`PamojaForward::Flood`], the choice that always works.
///
/// # Safety
///
/// `router` must be a live handle from [`pamoja_router_new`], or null, and
/// `out_next_hop` must point to a writable `uint32_t` or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_router_forward(
    router: *const PamojaRouter,
    dst: u32,
    out_next_hop: *mut u32,
) -> PamojaForward {
    if !out_next_hop.is_null() {
        *out_next_hop = 0;
    }
    if router.is_null() {
        return PamojaForward::Flood;
    }
    match (*router).router.forward(dst) {
        Forward::Deliver => PamojaForward::Deliver,
        Forward::Relay(next_hop) => {
            if !out_next_hop.is_null() {
                *out_next_hop = next_hop;
            }
            PamojaForward::Relay
        }
        Forward::Flood => PamojaForward::Flood,
    }
}

/// Forgets the route to a node, for example after it stops answering.
///
/// # Arguments
///
/// * `router` - the routing table.
/// * `dst` - the node to forget.
///
/// # Safety
///
/// `router` must be a live handle from [`pamoja_router_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_router_forget(router: *mut PamojaRouter, dst: u32) {
    if router.is_null() {
        return;
    }
    (*router).router.forget(dst);
}

/// Returns how many routes a table currently holds.
///
/// # Returns
///
/// The number of routes, or 0 if `router` is null.
///
/// # Safety
///
/// `router` must be a live handle from [`pamoja_router_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_router_len(router: *const PamojaRouter) -> usize {
    if router.is_null() {
        return 0;
    }
    (*router).router.len()
}

/// Returns how many routes a table can hold.
///
/// # Returns
///
/// The capacity it was created with, or 0 if `router` is null.
///
/// # Safety
///
/// `router` must be a live handle from [`pamoja_router_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_router_capacity(router: *const PamojaRouter) -> usize {
    if router.is_null() {
        return 0;
    }
    (*router).router.capacity()
}

/// Releases a routing table handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `router` must be a handle from [`pamoja_router_new`] that has not already been
/// freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_router_free(router: *mut PamojaRouter) {
    if !router.is_null() {
        drop(Box::from_raw(router));
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use super::*;

    #[test]
    fn a_cheaper_neighbour_replaces_the_route() {
        let router = pamoja_router_new(0x01, PAMOJA_ROUTING_DEFAULT_CAPACITY);
        // Safety: the handle was just created and the out-pointers are valid.
        unsafe {
            assert_eq!(pamoja_router_address(router), 0x01);
            assert!(pamoja_router_observe(router, 0x09, 0x05, 2));

            let mut next_hop = 0u32;
            assert_eq!(
                pamoja_router_forward(router, 0x09, &mut next_hop),
                PamojaForward::Relay
            );
            assert_eq!(next_hop, 0x05);

            assert!(pamoja_router_observe(router, 0x09, 0x07, 1));
            let mut route = PamojaRoute {
                dst: 0,
                next_hop: 0,
                cost: 0,
            };
            assert!(pamoja_router_route(router, 0x09, &mut route));
            assert_eq!(
                route,
                PamojaRoute {
                    dst: 0x09,
                    next_hop: 0x07,
                    cost: 1
                }
            );
            assert_eq!(pamoja_router_len(router), 1);
            pamoja_router_free(router);
        }
    }

    #[test]
    fn a_packet_for_this_node_is_delivered_and_an_unknown_one_floods() {
        let router = pamoja_router_new(0x01, PAMOJA_ROUTING_DEFAULT_CAPACITY);
        // Safety: the handle was just created and the out-pointer is valid.
        unsafe {
            let mut next_hop = 0xFFFF_FFFFu32;
            assert_eq!(
                pamoja_router_forward(router, 0x01, &mut next_hop),
                PamojaForward::Deliver
            );
            assert_eq!(next_hop, 0, "no next hop belongs to a local delivery");
            assert_eq!(
                pamoja_router_forward(router, 0x20, &mut next_hop),
                PamojaForward::Flood
            );
            pamoja_router_free(router);
        }
    }

    #[test]
    fn a_forgotten_route_floods_again() {
        let router = pamoja_router_new(0x01, PAMOJA_ROUTING_DEFAULT_CAPACITY);
        // Safety: the handle was just created and the out-pointers are valid.
        unsafe {
            pamoja_router_observe(router, 0x09, 0x05, 2);
            let mut cost = 0u16;
            assert!(pamoja_router_cost(router, 0x09, &mut cost));
            assert_eq!(cost, 2);

            pamoja_router_forget(router, 0x09);
            assert_eq!(pamoja_router_len(router), 0);
            let mut next_hop = 0u32;
            assert!(!pamoja_router_next_hop(router, 0x09, &mut next_hop));
            assert_eq!(
                pamoja_router_forward(router, 0x09, ptr::null_mut()),
                PamojaForward::Flood
            );
            pamoja_router_free(router);
        }
    }

    #[test]
    fn a_table_sized_by_the_caller_holds_what_it_was_asked_for() {
        // Safety: the handle is created and released here.
        unsafe {
            let router = pamoja_router_new(0x01, 3);
            assert_eq!(pamoja_router_capacity(router), 3);
            for node in 0..10u32 {
                pamoja_router_observe(router, node + 0x100, 0x05, 4);
            }
            assert_eq!(pamoja_router_len(router), 3);
            pamoja_router_free(router);
        }
    }

    #[test]
    fn the_table_fills_to_its_capacity() {
        let router = pamoja_router_new(0x01, PAMOJA_ROUTING_DEFAULT_CAPACITY);
        // Safety: the handle was just created.
        unsafe {
            for node in 0..PAMOJA_ROUTING_DEFAULT_CAPACITY + 8 {
                pamoja_router_observe(router, node as u32 + 0x100, 0x05, 4);
            }
            assert_eq!(pamoja_router_len(router), PAMOJA_ROUTING_DEFAULT_CAPACITY);
            assert_eq!(
                pamoja_router_capacity(router),
                PAMOJA_ROUTING_DEFAULT_CAPACITY
            );
            pamoja_router_free(router);
        }
    }

    #[test]
    fn null_handles_are_tolerated() {
        // Safety: every call below is documented to accept null.
        unsafe {
            assert_eq!(pamoja_router_address(ptr::null()), 0);
            assert!(!pamoja_router_observe(ptr::null_mut(), 1, 2, 3));
            assert_eq!(
                pamoja_router_forward(ptr::null(), 1, ptr::null_mut()),
                PamojaForward::Flood
            );
            pamoja_router_forget(ptr::null_mut(), 1);
            assert_eq!(pamoja_router_len(ptr::null()), 0);
            pamoja_router_free(ptr::null_mut());
        }
    }
}
