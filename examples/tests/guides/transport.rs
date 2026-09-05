//! The engine surface guide example; see docs/guides/transport.md.

/// The one transport contract, exercised through a fault injector wrapping an in-process
/// link: a refused send is buffered, the reading after it queues behind rather than
/// overtaking, and a flush forwards both in the order they were taken.
#[tokio::test]
async fn one_transport_contract_carries_every_link() {
    // ANCHOR: example
    use pamoja_core::Transport;
    use pamoja_ladder::{Delivery, TransportLadder};
    use pamoja_loopback::{Faulty, LoopbackBroker, LoopbackTransport};
    use pamoja_sync::MemoryStore;

    // Whatever a link is underneath, MQTT, CoAP, or the in-process broker here, it reaches
    // the rest of the framework through one trait. Anything that takes a link is generic
    // over it, so a node is written once and pointed at whichever link it has.
    let broker = LoopbackBroker::new();
    let topic = "sensors/1/temperature";
    let mut gateway = LoopbackTransport::new(broker.clone());
    gateway.connect().await.expect("the gateway connects");
    gateway.subscribe(topic).await.expect("subscribe");

    // The fault injector is itself a transport wrapping a transport, so it composes
    // anywhere a link does. This one fails its next send and passes the rest through.
    let mut ladder = TransportLadder::new(MemoryStore::new())
        .rung(Faulty::new(LoopbackTransport::new(broker), 1));
    ladder.connect().await.expect("the ladder connects");

    // The injected failure lands, so the reading is buffered rather than lost.
    let first = ladder.send(topic, b"20.1").await.expect("a delivery");
    let after_first = ladder.buffered().await.expect("a count");
    println!("first reading: {first:?}, {after_first} queued");

    // The next reading joins the back of the queue instead of overtaking it, even though
    // the link would take it now. Order on the wire is the order the readings were taken.
    let second = ladder.send(topic, b"20.4").await.expect("a delivery");
    let queued = ladder.buffered().await.expect("a count");
    println!("second reading: {second:?}, {queued} queued");

    // Flushing forwards the backlog oldest first, and the subscriber sees it in order.
    let forwarded = ladder.flush().await.expect("a flush");
    let first_out = gateway.recv().await.expect("recv").expect("a message");
    let second_out = gateway.recv().await.expect("recv").expect("a message");
    let earlier = String::from_utf8_lossy(&first_out.payload);
    let later = String::from_utf8_lossy(&second_out.payload);
    println!("flush forwarded {forwarded}, gateway saw {earlier} then {later}");
    // ANCHOR_END: example

    assert_eq!(first, Delivery::Buffered);
    assert_eq!(second, Delivery::Buffered);
    assert_eq!(queued, 2);
    assert_eq!(forwarded, 2);
    assert_eq!(ladder.buffered().await.expect("a count"), 0);
    assert_eq!(first_out.payload, b"20.1");
    assert_eq!(second_out.payload, b"20.4");
}
