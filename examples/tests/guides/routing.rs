//! The mesh-routing guide example; see docs/guides/routing.md.

/// One node filling its table from the traffic it hears, and the three answers that table
/// gives for a packet: deliver it, relay it, or fall back to flooding.
#[test]
fn a_node_learns_the_way_from_what_it_hears() {
    // ANCHOR: example
    use pamoja_routing::{Forward, Router};

    // The nodes on this mesh. An address is just a number; naming them is what makes the
    // table below read as a map of the site rather than a list of numbers.
    const GATEWAY: u32 = 1;
    const PUMP: u32 = 9;
    const TANK: u32 = 10;
    const NORTH_RELAY: u32 = 5;
    const EAST_RELAY: u32 = 7;
    const SOUTH_RELAY: u32 = 3;
    const SILO: u32 = 32;

    // A node learns the way to another from traffic it already hears: a packet from the
    // pump that arrived through the north relay proves that relay is a way back, at the
    // cost the packet reports.
    let mut router: Router<4> = Router::new(GATEWAY);
    router.observe(PUMP, NORTH_RELAY, 2);

    // The table keeps only the cheapest way it knows to each node, so a cost-1 report
    // through the east relay takes over and the later cost-4 report changes nothing.
    router.observe(PUMP, EAST_RELAY, 1);
    router.observe(PUMP, SOUTH_RELAY, 4);
    router.observe(TANK, NORTH_RELAY, 3);

    let route = router.route(PUMP).expect("a route to the pump");
    let (hop, cost) = (route.next_hop(), route.cost());
    println!("to the pump   via {hop} at cost {cost}");
    println!("routes held   {}", router.len());

    // Every packet gets one of three answers: deliver it here, relay it to the neighbour
    // on the way, or flood it because no route is known yet.
    for (name, address) in [("gateway", GATEWAY), ("pump", PUMP), ("silo", SILO)] {
        match router.forward(address) {
            Forward::Deliver => println!("for the {name:<8} deliver here"),
            Forward::Relay(next) => println!("for the {name:<8} relay via {next}"),
            Forward::Flood => println!("for the {name:<8} flood, no route known"),
        }
    }

    // Forgetting a node that has gone quiet returns its traffic to flooding, so routing
    // is an optimisation over flooding rather than a second thing that can fail.
    router.forget(PUMP);
    let after = router.forward(PUMP);
    let floods_again = after == Forward::Flood;
    println!("pump forgotten, so it floods again: {floods_again}");
    // ANCHOR_END: example

    assert_eq!(hop, EAST_RELAY);
    assert_eq!(cost, 1);
    assert_eq!(router.len(), 1);
    assert_eq!(after, Forward::Flood);

    let mut fresh: Router<4> = Router::new(GATEWAY);
    assert!(fresh.observe(PUMP, NORTH_RELAY, 2));
    assert!(fresh.observe(PUMP, EAST_RELAY, 1));
    assert!(!fresh.observe(PUMP, SOUTH_RELAY, 4));
    assert!(fresh.observe(TANK, NORTH_RELAY, 3));
    assert_eq!(fresh.len(), 2);
    assert_eq!(fresh.forward(GATEWAY), Forward::Deliver);
    assert_eq!(fresh.forward(PUMP), Forward::Relay(EAST_RELAY));
    assert_eq!(fresh.forward(SILO), Forward::Flood);
}
