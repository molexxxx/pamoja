//! The mesh-routing guide example; see docs/guides/routing.md.

/// One node filling its table from the traffic it hears, and the three answers that table
/// gives for a packet: deliver it, relay it, or fall back to flooding.
#[test]
fn a_node_learns_the_way_from_what_it_hears() {
    // ANCHOR: example
    use pamoja_routing::{Forward, Router};

    // A node learns the way to another from traffic it already hears: a packet from 0x09
    // that arrived through neighbour 0x05 proves 0x05 is the way back, at the cost the
    // packet reports.
    let mut router: Router<4> = Router::new(0x01);
    assert!(router.observe(0x09, 0x05, 2));

    // The table keeps the cheapest way it knows to each node, so the report of cost 1
    // takes over and the later cost-4 report changes nothing.
    assert!(router.observe(0x09, 0x07, 1));
    assert!(!router.observe(0x09, 0x03, 4));
    assert!(router.observe(0x0A, 0x05, 3));
    let route = router.route(0x09).expect("a route to 0x09");
    assert_eq!(route.next_hop(), 0x07);
    assert_eq!(route.cost(), 1);
    assert_eq!(router.len(), 2);

    // A packet gets one of three answers: deliver it here, relay it to the neighbour on
    // the way, or flood it because no route is known yet.
    assert_eq!(router.forward(0x01), Forward::Deliver);
    assert_eq!(router.forward(0x09), Forward::Relay(0x07));
    assert_eq!(router.forward(0x20), Forward::Flood);

    // Forgetting a node that has gone quiet returns its traffic to flooding, so routing
    // is an optimisation over flooding rather than a second thing that can fail.
    router.forget(0x09);
    assert_eq!(router.forward(0x09), Forward::Flood);
    assert_eq!(router.len(), 1);
    // ANCHOR_END: example
}
