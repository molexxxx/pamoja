//! The routing table and the per-packet forwarding decision.

/// A learned route to a destination: the neighbour to send through, and the cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Route {
    dst: u32,
    next_hop: u32,
    cost: u16,
}

impl Route {
    /// Returns the destination node this route reaches.
    ///
    /// # Returns
    ///
    /// The destination address.
    pub fn dst(&self) -> u32 {
        self.dst
    }

    /// Returns the neighbour to send through to reach the destination.
    ///
    /// # Returns
    ///
    /// The next-hop address.
    pub fn next_hop(&self) -> u32 {
        self.next_hop
    }

    /// Returns the cost of this route, in whatever metric the caller reports (hop count,
    /// summed link cost, or another).
    ///
    /// # Returns
    ///
    /// The route cost; lower is better.
    pub fn cost(&self) -> u16 {
        self.cost
    }
}

/// What to do with a packet bound for a given destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forward {
    /// The packet is for this node; hand it to the application.
    Deliver,
    /// A route is known; unicast the packet to this next hop.
    Relay(u32),
    /// No route is known; fall back to flooding the packet.
    Flood,
}

/// A fixed-size routing table for one node.
///
/// The table holds up to `N` routes, learned from the traffic the node hears. It keeps the
/// cheapest route it knows to each destination, and when full it gives up the most
/// expensive route to make room for a cheaper one, so its limited memory holds the routes
/// most worth keeping.
///
/// # Examples
///
/// ```
/// use pamoja_routing::{Forward, Router};
///
/// let mut router: Router<8> = Router::new(0x0A);
/// router.observe(0x0B, 0x0C, 3); // reach 0x0B via 0x0C, cost 3
/// assert_eq!(router.next_hop(0x0B), Some(0x0C));
/// assert_eq!(router.forward(0x0A), Forward::Deliver); // a packet for us
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Router<const N: usize> {
    me: u32,
    routes: [Option<Route>; N],
}

impl<const N: usize> Router<N> {
    /// Creates an empty router for the node at `me`.
    ///
    /// # Arguments
    ///
    /// * `me` - this node's address.
    ///
    /// # Returns
    ///
    /// A router holding no routes.
    pub const fn new(me: u32) -> Self {
        Router {
            me,
            routes: [None; N],
        }
    }

    /// Returns this node's address.
    ///
    /// # Returns
    ///
    /// The address the router was created with.
    pub fn address(&self) -> u32 {
        self.me
    }

    /// Learns the way to a node from a packet heard from it.
    ///
    /// A packet that originated at `origin` and reached this node via the neighbour `via`
    /// proves `via` is a way back to `origin` at the reported `cost`. The router adopts the
    /// route if it is cheaper than what it knows, or if it refreshes the cost of the route
    /// it is already using, and ignores a route to itself.
    ///
    /// # Arguments
    ///
    /// * `origin` - the node the packet came from, the destination this route reaches.
    /// * `via` - the neighbour the packet arrived through, the next hop for this route.
    /// * `cost` - the cost the packet reports for reaching `origin` through `via`.
    ///
    /// # Returns
    ///
    /// `true` if the table changed (a route was added, redirected, or recosted), `false`
    /// if the observation taught it nothing new.
    pub fn observe(&mut self, origin: u32, via: u32, cost: u16) -> bool {
        observe_into(&mut self.routes, self.me, origin, via, cost)
    }

    /// Returns the next hop to reach a destination, if a route is known.
    ///
    /// # Arguments
    ///
    /// * `dst` - the destination to reach.
    ///
    /// # Returns
    ///
    /// The next-hop address, or [`None`] if no route is known.
    pub fn next_hop(&self, dst: u32) -> Option<u32> {
        self.route(dst).map(|route| route.next_hop)
    }

    /// Returns the cost of the known route to a destination, if any.
    ///
    /// # Arguments
    ///
    /// * `dst` - the destination to reach.
    ///
    /// # Returns
    ///
    /// The route cost, or [`None`] if no route is known.
    pub fn cost(&self, dst: u32) -> Option<u16> {
        self.route(dst).map(|route| route.cost)
    }

    /// Returns the known route to a destination, if any.
    ///
    /// # Arguments
    ///
    /// * `dst` - the destination to reach.
    ///
    /// # Returns
    ///
    /// The [`Route`], or [`None`] if no route is known.
    pub fn route(&self, dst: u32) -> Option<Route> {
        route_in(&self.routes, dst)
    }

    /// Decides what to do with a packet bound for a destination.
    ///
    /// # Arguments
    ///
    /// * `dst` - the packet's destination.
    ///
    /// # Returns
    ///
    /// [`Forward::Deliver`] if the packet is for this node, [`Forward::Relay`] with the
    /// next hop if a route is known, or [`Forward::Flood`] otherwise.
    pub fn forward(&self, dst: u32) -> Forward {
        forward_in(&self.routes, self.me, dst)
    }

    /// Forgets the route to a destination, if one is held.
    ///
    /// # Arguments
    ///
    /// * `dst` - the destination whose route to drop.
    pub fn forget(&mut self, dst: u32) {
        forget_in(&mut self.routes, dst)
    }

    /// Returns how many routes the table currently holds.
    ///
    /// # Returns
    ///
    /// The number of routes.
    pub fn len(&self) -> usize {
        self.routes.iter().filter(|slot| slot.is_some()).count()
    }

    /// Reports whether the table holds no routes.
    ///
    /// # Returns
    ///
    /// `true` if no routes are held.
    pub fn is_empty(&self) -> bool {
        self.routes.iter().all(Option::is_none)
    }
}

// Learns a route into a slot slice. Split out so the fixed-size and runtime-sized tables
// share one implementation rather than two that can drift.
fn observe_into(routes: &mut [Option<Route>], me: u32, origin: u32, via: u32, cost: u16) -> bool {
    if origin == me {
        return false;
    }
    if let Some(index) = index_of_in(routes, origin) {
        let route = routes[index]
            .as_mut()
            .expect("index_of_in points at a route");
        if cost < route.cost || via == route.next_hop {
            let changed = route.next_hop != via || route.cost != cost;
            route.next_hop = via;
            route.cost = cost;
            return changed;
        }
        return false;
    }

    let new = Route {
        dst: origin,
        next_hop: via,
        cost,
    };
    if let Some(empty) = routes.iter().position(Option::is_none) {
        routes[empty] = Some(new);
        return true;
    }

    // The table is full; replace the costliest route if this one is cheaper. A capacity of
    // zero leaves nothing to replace, so the observation is dropped.
    if let Some((worst, worst_cost)) = routes
        .iter()
        .enumerate()
        .filter_map(|(i, slot)| slot.as_ref().map(|route| (i, route.cost)))
        .max_by_key(|&(_, cost)| cost)
    {
        if cost < worst_cost {
            routes[worst] = Some(new);
            return true;
        }
    }
    false
}

// The slot index of the route to `dst`, if one is held.
fn index_of_in(routes: &[Option<Route>], dst: u32) -> Option<usize> {
    routes
        .iter()
        .position(|slot| slot.as_ref().is_some_and(|route| route.dst == dst))
}

// The route to `dst`, if one is held.
fn route_in(routes: &[Option<Route>], dst: u32) -> Option<Route> {
    index_of_in(routes, dst).map(|index| routes[index].expect("index_of_in points at a route"))
}

// Decides what to do with a packet bound for `dst`.
fn forward_in(routes: &[Option<Route>], me: u32, dst: u32) -> Forward {
    if dst == me {
        return Forward::Deliver;
    }
    match route_in(routes, dst) {
        Some(route) => Forward::Relay(route.next_hop),
        None => Forward::Flood,
    }
}

// Drops the route to `dst`, if one is held.
fn forget_in(routes: &mut [Option<Route>], dst: u32) {
    if let Some(index) = index_of_in(routes, dst) {
        routes[index] = None;
    }
}

/// A routing table whose size is chosen when it is built, rather than at compile time.
///
/// [`Router`] fixes its capacity in the type, which suits a microcontroller that knows its
/// own limits. A gateway, or any caller reaching this through a language binding, does not
/// know the size until it runs, and a const generic cannot cross a foreign function
/// boundary at all. This is the same table with its slots on the heap, so both share one
/// implementation and answer identically.
///
/// Requires the `alloc` feature.
///
/// # Examples
///
/// ```
/// use pamoja_routing::{DynamicRouter, Forward};
///
/// // A gateway sizes its table for the mesh it is actually serving.
/// let mut router = DynamicRouter::new(0x01, 512);
/// router.observe(0x09, 0x05, 2);
/// assert_eq!(router.forward(0x09), Forward::Relay(0x05));
/// assert_eq!(router.capacity(), 512);
/// ```
#[cfg(any(feature = "alloc", test))]
#[derive(Clone, Debug)]
pub struct DynamicRouter {
    me: u32,
    routes: alloc::vec::Vec<Option<Route>>,
}

#[cfg(any(feature = "alloc", test))]
impl DynamicRouter {
    /// Creates an empty router holding up to `capacity` routes.
    ///
    /// # Arguments
    ///
    /// * `me` - this node's address.
    /// * `capacity` - how many routes to make room for. A capacity of zero is allowed and
    ///   makes every unknown destination flood, which is the behaviour with no table.
    ///
    /// # Returns
    ///
    /// A router holding no routes.
    pub fn new(me: u32, capacity: usize) -> Self {
        DynamicRouter {
            me,
            routes: alloc::vec![None; capacity],
        }
    }

    /// Returns this node's address.
    ///
    /// # Returns
    ///
    /// The address the router was created with.
    pub fn address(&self) -> u32 {
        self.me
    }

    /// Returns how many routes this table can hold.
    ///
    /// # Returns
    ///
    /// The capacity it was created with.
    pub fn capacity(&self) -> usize {
        self.routes.len()
    }

    /// Learns the way to a node from a packet heard from it.
    ///
    /// # Arguments
    ///
    /// * `origin` - the node the packet came from, the destination this route reaches.
    /// * `via` - the neighbour the packet arrived through, the next hop for this route.
    /// * `cost` - the cost the packet reports for reaching `origin` through `via`.
    ///
    /// # Returns
    ///
    /// `true` if the table changed, `false` if the observation taught it nothing new.
    pub fn observe(&mut self, origin: u32, via: u32, cost: u16) -> bool {
        observe_into(&mut self.routes, self.me, origin, via, cost)
    }

    /// Returns the next hop to reach a destination, if a route is known.
    ///
    /// # Arguments
    ///
    /// * `dst` - the destination to reach.
    ///
    /// # Returns
    ///
    /// The next-hop address, or [`None`] if no route is known.
    pub fn next_hop(&self, dst: u32) -> Option<u32> {
        route_in(&self.routes, dst).map(|route| route.next_hop)
    }

    /// Returns the cost of the known route to a destination, if any.
    ///
    /// # Arguments
    ///
    /// * `dst` - the destination to reach.
    ///
    /// # Returns
    ///
    /// The route cost, or [`None`] if no route is known.
    pub fn cost(&self, dst: u32) -> Option<u16> {
        route_in(&self.routes, dst).map(|route| route.cost)
    }

    /// Returns the known route to a destination, if any.
    ///
    /// # Arguments
    ///
    /// * `dst` - the destination to reach.
    ///
    /// # Returns
    ///
    /// The [`Route`], or [`None`] if no route is known.
    pub fn route(&self, dst: u32) -> Option<Route> {
        route_in(&self.routes, dst)
    }

    /// Decides what to do with a packet bound for a destination.
    ///
    /// # Arguments
    ///
    /// * `dst` - the packet's destination.
    ///
    /// # Returns
    ///
    /// [`Forward::Deliver`] if the packet is for this node, [`Forward::Relay`] with the
    /// next hop if a route is known, or [`Forward::Flood`] otherwise.
    pub fn forward(&self, dst: u32) -> Forward {
        forward_in(&self.routes, self.me, dst)
    }

    /// Forgets the route to a destination, if one is held.
    ///
    /// # Arguments
    ///
    /// * `dst` - the destination whose route to drop.
    pub fn forget(&mut self, dst: u32) {
        forget_in(&mut self.routes, dst)
    }

    /// Returns how many routes the table currently holds.
    ///
    /// # Returns
    ///
    /// The number of routes.
    pub fn len(&self) -> usize {
        self.routes.iter().filter(|slot| slot.is_some()).count()
    }

    /// Reports whether the table holds no routes.
    ///
    /// # Returns
    ///
    /// `true` if no routes are held.
    pub fn is_empty(&self) -> bool {
        self.routes.iter().all(Option::is_none)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_learned_route_is_used() {
        let mut router: Router<8> = Router::new(1);
        assert!(router.observe(9, 5, 2));
        assert_eq!(router.next_hop(9), Some(5));
        assert_eq!(router.cost(9), Some(2));
        assert_eq!(router.forward(9), Forward::Relay(5));
    }

    #[test]
    fn a_packet_for_this_node_is_delivered() {
        let router: Router<8> = Router::new(1);
        assert_eq!(router.forward(1), Forward::Deliver);
    }

    #[test]
    fn an_unknown_destination_floods() {
        let router: Router<8> = Router::new(1);
        assert_eq!(router.forward(42), Forward::Flood);
    }

    #[test]
    fn a_cheaper_route_replaces_a_costlier_one() {
        let mut router: Router<8> = Router::new(1);
        router.observe(9, 5, 4);
        assert!(router.observe(9, 7, 1));
        assert_eq!(router.next_hop(9), Some(7));
        assert_eq!(router.cost(9), Some(1));
    }

    #[test]
    fn a_costlier_route_is_ignored() {
        let mut router: Router<8> = Router::new(1);
        router.observe(9, 7, 1);
        assert!(!router.observe(9, 5, 4));
        assert_eq!(router.next_hop(9), Some(7));
    }

    #[test]
    fn the_current_next_hop_can_refresh_its_cost() {
        let mut router: Router<8> = Router::new(1);
        router.observe(9, 7, 1);
        // The same neighbour now reports a higher cost; we trust our current path.
        assert!(router.observe(9, 7, 3));
        assert_eq!(router.cost(9), Some(3));
    }

    #[test]
    fn we_never_route_to_ourselves() {
        let mut router: Router<8> = Router::new(1);
        assert!(!router.observe(1, 5, 1));
        assert_eq!(router.route(1), None);
    }

    #[test]
    fn a_full_table_evicts_its_costliest_route_for_a_cheaper_one() {
        let mut router: Router<2> = Router::new(1);
        router.observe(10, 2, 5);
        router.observe(11, 3, 8); // the costliest
        assert_eq!(router.len(), 2);

        // A cheaper route than the costliest evicts it.
        assert!(router.observe(12, 4, 2));
        assert_eq!(router.next_hop(11), None); // evicted
        assert_eq!(router.next_hop(10), Some(2)); // kept
        assert_eq!(router.next_hop(12), Some(4)); // added
    }

    #[test]
    fn a_full_table_keeps_its_routes_against_a_costlier_one() {
        let mut router: Router<2> = Router::new(1);
        router.observe(10, 2, 5);
        router.observe(11, 3, 8);
        // A new route costlier than everything held is not worth a slot.
        assert!(!router.observe(12, 4, 9));
        assert_eq!(router.next_hop(12), None);
        assert_eq!(router.len(), 2);
    }

    #[test]
    fn forgetting_a_route_drops_it() {
        let mut router: Router<8> = Router::new(1);
        router.observe(9, 5, 2);
        router.forget(9);
        assert_eq!(router.route(9), None);
        assert!(router.is_empty());
    }

    #[test]
    fn an_empty_router_reports_empty() {
        let router: Router<8> = Router::new(1);
        assert!(router.is_empty());
        assert_eq!(router.len(), 0);
    }

    #[test]
    fn a_zero_capacity_router_never_learns_but_does_not_panic() {
        let mut router: Router<0> = Router::new(1);
        assert!(!router.observe(9, 5, 2));
        assert_eq!(router.next_hop(9), None);
        assert_eq!(router.forward(9), Forward::Flood);
        assert!(router.is_empty());
    }

    #[test]
    fn a_runtime_sized_table_decides_the_same_way() {
        let mut fixed: Router<8> = Router::new(1);
        let mut dynamic = DynamicRouter::new(1, 8);
        for (origin, via, cost) in [(9u32, 5u32, 4u16), (9, 7, 1), (10, 5, 3), (1, 2, 1)] {
            assert_eq!(
                fixed.observe(origin, via, cost),
                dynamic.observe(origin, via, cost),
                "the two tables learn identically"
            );
        }
        for dst in [1u32, 9, 10, 42] {
            assert_eq!(fixed.forward(dst), dynamic.forward(dst));
            assert_eq!(fixed.route(dst), dynamic.route(dst));
        }
        assert_eq!(fixed.len(), dynamic.len());

        fixed.forget(9);
        dynamic.forget(9);
        assert_eq!(fixed.forward(9), dynamic.forward(9));
        assert_eq!(fixed.len(), dynamic.len());
    }

    #[test]
    fn a_runtime_sized_table_fills_to_the_size_it_was_given() {
        let mut router = DynamicRouter::new(1, 3);
        assert_eq!(router.capacity(), 3);
        assert!(router.is_empty());
        for node in 0..10u32 {
            router.observe(node + 0x100, 0x05, 4);
        }
        assert_eq!(router.len(), 3, "it holds no more than it was sized for");
    }

    #[test]
    fn a_table_with_no_room_floods_everything() {
        let mut router = DynamicRouter::new(1, 0);
        assert!(!router.observe(9, 5, 2), "there is nowhere to put a route");
        assert_eq!(router.forward(9), Forward::Flood);
        assert_eq!(
            router.forward(1),
            Forward::Deliver,
            "a local packet still arrives"
        );
    }
}
