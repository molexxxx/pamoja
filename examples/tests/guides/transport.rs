//! The engine surface guide example; see docs/guides/transport.md.

/// The one transport contract, exercised through a fault injector wrapping an in-process
/// link: a refused send is buffered, the reading taken after it joins the backlog rather
/// than overtaking it, and a flush forwards both in the order they were taken.
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
    let mut gateway = LoopbackTransport::new(broker.clone());
    gateway.connect().await.unwrap();
    gateway.subscribe("sensors/1/temperature").await.unwrap();

    // The fault injector is itself a transport wrapping a transport, so it composes
    // anywhere a link does. This one fails its next send and passes the rest through.
    let mut ladder = TransportLadder::new(MemoryStore::new())
        .rung(Faulty::new(LoopbackTransport::new(broker), 1));
    ladder.connect().await.unwrap();

    // The injected failure lands, so the reading is buffered rather than lost.
    let topic = "sensors/1/temperature";
    assert_eq!(
        ladder.send(topic, b"20.1").await.unwrap(),
        Delivery::Buffered
    );
    assert_eq!(ladder.buffered().await.unwrap(), 1);

    // The next reading joins the back of the queue instead of overtaking it, even though
    // the link would take it now. Order on the wire is the order the readings were taken.
    assert_eq!(
        ladder.send(topic, b"20.4").await.unwrap(),
        Delivery::Buffered
    );
    assert_eq!(ladder.buffered().await.unwrap(), 2);

    // Flushing forwards the backlog oldest first, and the subscriber sees it in order.
    assert_eq!(ladder.flush().await.unwrap(), 2);
    assert_eq!(ladder.buffered().await.unwrap(), 0);
    assert_eq!(gateway.recv().await.unwrap().unwrap().payload, b"20.1");
    assert_eq!(gateway.recv().await.unwrap().unwrap().payload, b"20.4");
    // ANCHOR_END: example
}
