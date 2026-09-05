//! The transport ladder guide example; see docs/guides/ladder.md.

/// A node with two links and a queue behind them: the cheap hop is tried first, a reading
/// nothing will take is buffered rather than lost, and the backlog goes out exactly once
/// when a link returns.
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
    let topic = "sensors/1/temperature";
    let mut gateway = LoopbackTransport::new(backhaul.clone());
    gateway.connect().await.expect("the gateway connects");
    gateway.subscribe(topic).await.expect("subscribe");

    // Rungs are tried in the order they are added, cheapest first. The mesh hop loses
    // every packet here; the backhaul carries one send, then drops the next two.
    let mut ladder = TransportLadder::new(MemoryStore::new())
        .rung(DegradedLink::new(LoopbackTransport::new(mesh)).drop_every(1))
        .rung(DegradedLink::new(LoopbackTransport::new(backhaul)).intermittent(1, 2));
    ladder.connect().await.expect("the ladder connects");

    // The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
    // broker only that rung publishes to.
    let first = ladder.send(topic, b"21.5").await.expect("a delivery");
    let arrived = gateway.recv().await.expect("recv").expect("a message");
    let reading = String::from_utf8_lossy(&arrived.payload);
    println!("first reading: {first:?}, gateway got {reading}");

    // Now nothing will take a send, so the next reading is buffered rather than lost.
    let second = ladder.send(topic, b"21.6").await.expect("a delivery");
    let waiting = ladder.buffered().await.expect("a count");
    println!("second reading: {second:?}, {waiting} waiting in the queue");

    // A flush while the links are still down forwards nothing and leaves the backlog
    // intact, because a record is removed only once a rung has accepted it.
    let while_down = ladder.flush().await.expect("a flush");
    let still_queued = ladder.buffered().await.expect("a count");
    println!("flush while down forwarded {while_down}, queue still {still_queued}");

    // The backhaul is reachable again, so the buffered reading goes out exactly once.
    let when_up = ladder.flush().await.expect("a flush");
    let late = gateway.recv().await.expect("recv").expect("a message");
    let buffered_reading = String::from_utf8_lossy(&late.payload);
    println!("flush when up forwarded {when_up}, gateway got {buffered_reading}");
    // ANCHOR_END: example

    assert_eq!(first, Delivery::Sent);
    assert_eq!(arrived.payload, b"21.5");
    assert_eq!(second, Delivery::Buffered);
    assert_eq!(waiting, 1);
    assert_eq!(while_down, 0);
    assert_eq!(when_up, 1);
    assert_eq!(ladder.buffered().await.expect("a count"), 0);
    assert_eq!(late.payload, b"21.6");
}
