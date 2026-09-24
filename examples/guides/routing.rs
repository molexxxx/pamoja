//! The mesh-routing guide example; see docs/guides/routing.md.
//!
//! Run: `cargo run -p pamoja-examples --example routing`

use std::error::Error;

/// A gateway's table filling from the traffic it hears and the three answers it gives a
/// packet, what a full table keeps, and a site where the tables one flood leaves behind
/// carry an answer in two sends instead of six.
fn main() -> std::result::Result<(), Box<dyn Error>> {
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
    // pump that arrived through a relay proves that relay is a way back, at the cost the
    // packet reports. The table keeps the cheapest way it has heard, and a tie keeps the
    // way in use so two equal paths do not flap. Word from the relay already in use is
    // taken even when it is worse, which is how a failing link lets a detour win.
    let mut router: Router<4> = Router::new(GATEWAY);
    for (via, cost) in [
        (NORTH_RELAY, 2),
        (EAST_RELAY, 1),
        (SOUTH_RELAY, 4),
        (NORTH_RELAY, 1),
        (EAST_RELAY, 3),
        (NORTH_RELAY, 2),
    ] {
        let changed = router.observe(PUMP, via, cost);
        let route = router.route(PUMP).expect("a route to the pump");
        let outcome = if changed {
            "so the route is"
        } else {
            "and the route stays"
        };
        println!(
            "heard     the pump via {via} at cost {cost}, {outcome} {} at cost {}",
            route.next_hop(),
            route.cost()
        );
    }

    // The table lists what it holds, one route for each node it has heard from.
    router.observe(TANK, NORTH_RELAY, 3);
    let held: Vec<String> = router
        .routes()
        .map(|route| {
            let (dst, hop, cost) = (route.dst(), route.next_hop(), route.cost());
            format!("to {dst} via {hop} at cost {cost}")
        })
        .collect();
    println!(
        "table     {} routes of {}: {}",
        router.len(),
        router.capacity(),
        held.join(", ")
    );

    // Every packet gets one of three answers: deliver it here, relay it to the neighbor
    // on the way, or flood it because no route is known yet.
    for (name, address) in [("gateway", GATEWAY), ("pump", PUMP), ("silo", SILO)] {
        match router.forward(address) {
            Forward::Deliver => println!("{name:<10}deliver here"),
            Forward::Relay(next) => println!("{name:<10}relay via {next}"),
            Forward::Flood => println!("{name:<10}flood, no route known"),
        }
    }

    // The table keeps no clock, so a route through a relay that has gone quiet stays
    // until the caller forgets it, typically when a relayed packet goes unanswered.
    // Forgetting returns the node's traffic to flooding, the answer that always works.
    router.forget(PUMP);
    if router.forward(PUMP) == Forward::Flood {
        println!(
            "forgot    the pump, so it floods again, and {} route is left",
            router.len()
        );
    }
    // ANCHOR_END: example

    assert_eq!(router.len(), 1);
    assert_eq!(router.next_hop(TANK), Some(NORTH_RELAY));

    // ANCHOR: limits
    // A table has a fixed number of slots. Once they are full, a cheaper route takes the
    // slot of the costliest one held and a costlier route is refused, so a small table
    // keeps the nodes nearest to it and floods to the rest.
    const WELL: u32 = 11;
    const GATE: u32 = 12;
    let mut small: Router<2> = Router::new(GATEWAY);
    small.observe(TANK, NORTH_RELAY, 3);
    small.observe(SILO, SOUTH_RELAY, 5);
    let held: Vec<String> = small
        .routes()
        .map(|route| {
            let (dst, hop, cost) = (route.dst(), route.next_hop(), route.cost());
            format!("to {dst} via {hop} at cost {cost}")
        })
        .collect();
    println!(
        "full      {} routes of {}: {}",
        small.len(),
        small.capacity(),
        held.join(", ")
    );
    if small.observe(WELL, EAST_RELAY, 2) && small.route(SILO).is_none() {
        println!("evicted   the well at cost 2 took the slot of the silo, the costliest held");
    }
    if !small.observe(GATE, EAST_RELAY, 6) && small.forward(GATE) == Forward::Flood {
        println!("refused   the gate at cost 6 costs more than every route held, so it floods");
    }

    // A flood echoes, and the gateway hears its own packets come back through the
    // relays. A route to the node itself is never learned, whatever it costs.
    if !router.observe(GATEWAY, EAST_RELAY, 2) {
        println!("echo      the gateway's own packet coming back teaches it nothing");
    }

    // A table with no slots is flooding with nothing remembered, which a node with no
    // memory to spare can still do. A packet for the node itself is still delivered.
    let mut none: Router<0> = Router::new(GATEWAY);
    let learned = none.observe(PUMP, EAST_RELAY, 1);
    if !learned && none.forward(PUMP) == Forward::Flood && none.forward(GATEWAY) == Forward::Deliver
    {
        println!(
            "no room   a table of 0 learns nothing: the pump floods, and the gateway still delivers"
        );
    }
    // ANCHOR_END: limits

    assert_eq!(small.next_hop(WELL), Some(EAST_RELAY));
    assert!(none.is_empty());

    // ANCHOR: site
    use std::collections::{BTreeMap, VecDeque};

    use pamoja_mesh::{Frame, MeshError, SeenCache};

    // Who hears whom on the site. The gateway hears the three relays, and each relay
    // hears the one node beyond it; the pump is out of the gateway's range.
    let site: BTreeMap<u32, Vec<u32>> = BTreeMap::from([
        (GATEWAY, vec![NORTH_RELAY, EAST_RELAY, SOUTH_RELAY]),
        (NORTH_RELAY, vec![GATEWAY, TANK]),
        (EAST_RELAY, vec![GATEWAY, PUMP]),
        (SOUTH_RELAY, vec![GATEWAY, SILO]),
        (TANK, vec![NORTH_RELAY]),
        (PUMP, vec![EAST_RELAY]),
        (SILO, vec![SOUTH_RELAY]),
    ]);

    // Sends a frame from one node and plays out what the site does with it. A node that
    // hears a frame for the first time learns the way back to its source, then asks its
    // own table what to do: deliver it, relay it to the one neighbor on the way, or flood
    // it to every neighbor in range, spending a hop each time it goes on. Every frame here
    // starts at the default hop limit, and one heard straight from its source still has
    // all of it, so the hops a frame has come are what it has spent, plus one. Returns
    // how many times a radio sent, and the nodes that took the frame.
    fn send(
        site: &BTreeMap<u32, Vec<u32>>,
        tables: &mut BTreeMap<u32, Router<8>>,
        from: u32,
        frame: Frame,
    ) -> Result<(usize, Vec<u32>), MeshError> {
        let mut seen: BTreeMap<u32, SeenCache<64>> =
            site.keys().map(|&node| (node, SeenCache::new())).collect();
        seen.entry(from).or_default().record(frame.dedup_key());
        let (mut sends, mut reached) = (0, Vec::new());
        let mut on_the_air = VecDeque::from([(from, frame)]);
        while let Some((sender, sent)) = on_the_air.pop_front() {
            let to = match tables[&sender].forward(sent.dst()) {
                Forward::Deliver => continue,
                Forward::Relay(next) => vec![next],
                Forward::Flood => site[&sender].clone(),
            };
            sends += 1;
            for node in to {
                let heard = Frame::parse(sent.as_bytes())?;
                if !seen.entry(node).or_default().record(heard.dedup_key()) {
                    continue;
                }
                reached.push(node);
                let hops = Frame::DEFAULT_HOP_LIMIT - heard.hop_limit() + 1;
                if let Some(table) = tables.get_mut(&node) {
                    table.observe(heard.src(), sender, hops.into());
                }
                if let Some(onward) = heard.relayed() {
                    on_the_air.push_back((node, onward));
                }
            }
        }
        Ok((sends, reached))
    }

    // The pump floods a reading, and every node that takes it learns the way back.
    let mut tables: BTreeMap<u32, Router<8>> =
        site.keys().map(|&node| (node, Router::new(node))).collect();
    let reading = Frame::broadcast(PUMP, 1, b"flow=12")?;
    let (sends, reached) = send(&site, &mut tables, PUMP, reading)?;
    println!(
        "flood     the pump's reading took {sends} sends to reach {} nodes, and each learned the way back",
        reached.len()
    );

    // The gateway answers the pump. Each node on the way relays to the one neighbor its
    // table names, so the answer reaches only the nodes on the path.
    let answer = Frame::new(GATEWAY, PUMP, 1, b"run=10min")?;
    let (routed, path) = send(&site, &mut tables, GATEWAY, answer)?;
    let path: Vec<String> = path.iter().map(u32::to_string).collect();
    println!(
        "routed    the gateway's answer reached only {}, in {routed} sends",
        path.join(" and ")
    );

    // The same answer on a site that has learned nothing floods, and every node relays it.
    let mut blank: BTreeMap<u32, Router<8>> =
        site.keys().map(|&node| (node, Router::new(node))).collect();
    let (flooded, everyone) = send(&site, &mut blank, GATEWAY, answer)?;
    println!(
        "flooded   with nothing learned, the same answer reached all {} other nodes in {flooded} sends",
        everyone.len()
    );
    // ANCHOR_END: site

    assert_eq!(reached.len(), site.len() - 1);
    assert_eq!(path, [EAST_RELAY.to_string(), PUMP.to_string()]);
    assert!(routed < flooded);
    for (node, hop, cost) in [
        (EAST_RELAY, PUMP, 1),
        (GATEWAY, EAST_RELAY, 2),
        (NORTH_RELAY, GATEWAY, 3),
        (SOUTH_RELAY, GATEWAY, 3),
        (TANK, NORTH_RELAY, 4),
        (SILO, SOUTH_RELAY, 4),
    ] {
        let route = tables[&node]
            .route(PUMP)
            .expect("every node learned the pump");
        assert_eq!((route.next_hop(), route.cost()), (hop, cost), "node {node}");
    }

    Ok(())
}
