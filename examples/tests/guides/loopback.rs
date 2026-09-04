//! The loopback transport guide example; see docs/guides/loopback.md.

/// A publish-and-subscribe round trip with no broker process behind it: what a topic
/// filter selects, where the two wildcards differ, and what a link that has gone away
/// does with a message rather than dropping it.
#[tokio::test]
async fn a_round_trip_through_an_in_process_broker() {
    // ANCHOR: example
    use pamoja_core::Transport;
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};

    // One broker and two links off it, all in this process. Nothing binds a port and
    // nothing has to be running for the traffic below to flow.
    let broker = LoopbackBroker::new();
    let mut publisher = LoopbackTransport::new(broker.clone());
    let mut subscriber = LoopbackTransport::new(broker.clone());
    publisher.connect().await.expect("connect");
    subscriber.connect().await.expect("connect");

    // A `+` stands for exactly one level, so the deeper topic is not delivered here and
    // the first message this subscriber sees is the second publish.
    subscriber
        .subscribe("line/+/temp")
        .await
        .expect("subscribe");
    publisher
        .send("line/mixer/temp/raw", b"2150")
        .await
        .expect("send");
    publisher
        .send("line/mixer/temp", b"21.5")
        .await
        .expect("send");

    let message = subscriber.recv().await.expect("recv").expect("a message");
    assert_eq!(message.topic, "line/mixer/temp");
    assert_eq!(message.payload, b"21.5");

    // A `#` covers the levels that remain, so a second link takes the whole subtree,
    // including the reading the single-level filter passed over.
    let mut watcher = LoopbackTransport::new(broker);
    watcher.connect().await.expect("connect");
    watcher.subscribe("line/#").await.expect("subscribe");
    publisher
        .send("line/mixer/temp/raw", b"2150")
        .await
        .expect("send");

    let deep = watcher.recv().await.expect("recv").expect("a message");
    assert_eq!(deep.topic, "line/mixer/temp/raw");
    assert_eq!(deep.payload, b"2150");

    // A link that has been disconnected reports the failure instead of dropping the
    // reading, which is the case a test wants to reach without unplugging anything.
    publisher.disconnect();
    assert!(publisher.send("line/mixer/temp", b"21.6").await.is_err());
    // ANCHOR_END: example
}
