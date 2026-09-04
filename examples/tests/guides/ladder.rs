//! The transport ladder guide example; see docs/guides/ladder.md.

/// A reading that falls through a dead rung onto the next one, a second reading that no
/// rung will take and so waits in the buffer, and a flush that drains it once a link is
/// reachable again.
#[tokio::test]
async fn a_reading_falls_through_a_dead_rung_and_then_waits_for_a_link() {
    // ANCHOR: example
    use pamoja_core::Transport;
    use pamoja_ladder::{Delivery, TransportLadder};
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    use pamoja_sim::DegradedLink;
    use pamoja_sync::MemoryStore;

    // Two links off the same node: a near mesh hop and a metered backhaul. Each has its
    // own broker, so which rung carried a reading is visible from its subscriber.
    let mesh = LoopbackBroker::new();
    let backhaul = LoopbackBroker::new();
    let mut gateway = LoopbackTransport::new(backhaul.clone());
    gateway.connect().await.unwrap();
    gateway.subscribe("sensors/1/temperature").await.unwrap();

    // Rungs are tried in the order they are added, cheapest first. The mesh hop loses
    // every packet here; the backhaul carries one send, then drops the next two.
    let mut ladder = TransportLadder::new(MemoryStore::new())
        .rung(DegradedLink::new(LoopbackTransport::new(mesh)).drop_every(1))
        .rung(DegradedLink::new(LoopbackTransport::new(backhaul)).intermittent(1, 2));
    ladder.connect().await.unwrap();

    // The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
    // broker only that rung publishes to.
    let topic = "sensors/1/temperature";
    assert_eq!(ladder.send(topic, b"21.5").await.unwrap(), Delivery::Sent);
    assert_eq!(gateway.recv().await.unwrap().unwrap().payload, b"21.5");

    // Now nothing will take a send, so the next reading is buffered rather than lost.
    let delivery = ladder.send(topic, b"21.6").await.unwrap();
    assert_eq!(delivery, Delivery::Buffered);
    assert_eq!(ladder.buffered().await.unwrap(), 1);

    // A flush while the links are still down forwards nothing and leaves the backlog
    // intact, because a record is removed only once a rung has accepted it.
    assert_eq!(ladder.flush().await.unwrap(), 0);
    assert_eq!(ladder.buffered().await.unwrap(), 1);

    // The backhaul is reachable again, so the buffered reading goes out exactly once.
    assert_eq!(ladder.flush().await.unwrap(), 1);
    assert_eq!(ladder.buffered().await.unwrap(), 0);
    assert_eq!(gateway.recv().await.unwrap().unwrap().payload, b"21.6");
    // ANCHOR_END: example
}
