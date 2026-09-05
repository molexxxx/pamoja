//! The in-process broker guide example; see docs/guides/loopback.md.

/// Two links off one in-process broker, showing what each topic filter catches and what a
/// disconnected link does with a reading, all without binding a port.
#[tokio::test]
async fn a_round_trip_through_an_in_process_broker() {
    // ANCHOR: example
    use pamoja_core::Transport;
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};

    // One broker and two links off it, all in this process. Nothing binds a port and
    // nothing has to be running for the traffic below to flow, which is what makes this
    // the link to develop a node against before it has a real one.
    let broker = LoopbackBroker::new();
    let mut publisher = LoopbackTransport::new(broker.clone());
    let mut subscriber = LoopbackTransport::new(broker.clone());
    publisher.connect().await.expect("the publisher connects");
    subscriber.connect().await.expect("the subscriber connects");

    // A `+` stands for exactly one level, so this takes the mixer's temperature but not
    // the raw reading a level below it.
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
    let reading = String::from_utf8_lossy(&message.payload);
    println!("line/+/temp took {reading} from {}", message.topic);

    // A `#` covers every level that remains, so a second link takes the whole subtree,
    // including the reading the single-level filter passed over.
    let mut watcher = LoopbackTransport::new(broker);
    watcher.connect().await.expect("the watcher connects");
    watcher.subscribe("line/#").await.expect("subscribe");
    publisher
        .send("line/mixer/temp/raw", b"2150")
        .await
        .expect("send");

    let deep = watcher.recv().await.expect("recv").expect("a message");
    let raw = String::from_utf8_lossy(&deep.payload);
    println!("line/#     took {raw} from {}", deep.topic);

    // A link that has been disconnected reports the failure instead of dropping the
    // reading, which is the case a test wants to reach without unplugging anything.
    publisher.disconnect();
    match publisher.send("line/mixer/temp", b"21.6").await {
        Ok(_) => println!("a disconnected link took a reading, which should never happen"),
        Err(error) => println!("disconnected refused the reading: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(message.topic, "line/mixer/temp");
    assert_eq!(message.payload, b"21.5");
    assert_eq!(deep.topic, "line/mixer/temp/raw");
    assert_eq!(deep.payload, b"2150");
    assert!(publisher.send("line/mixer/temp", b"21.6").await.is_err());
}
