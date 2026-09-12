//! The in-process broker guide example; see docs/guides/loopback.md.
//!
//! Run: `cargo run -p pamoja-examples --example loopback`

use std::error::Error;

/// Two links off one in-process broker, showing what each topic filter catches and what a
/// disconnected link does with a reading, all without binding a port.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_core::{Receive, Transport};
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
    subscriber.subscribe("line/+/temp").await?;
    publisher.send_text("line/mixer/temp/raw", "2150").await?;
    publisher.send_text("line/mixer/temp", "21.5").await?;

    let message = subscriber.recv().await?.expect("a message");
    let reading = message.text().expect("text");
    println!("line/+/temp took {reading} from {}", message.topic);

    // A `#` covers every level that remains, so a second link takes the whole subtree,
    // including the reading the single-level filter passed over.
    let mut watcher = LoopbackTransport::new(broker);
    watcher.connect().await.expect("the watcher connects");
    watcher.subscribe("line/#").await?;
    publisher.send_text("line/mixer/temp/raw", "2150").await?;

    let deep = watcher.recv().await?.expect("a message");
    let raw = deep.text().expect("text");
    println!("line/#     took {raw} from {}", deep.topic);

    // A link that has been disconnected reports the failure instead of dropping the
    // reading, which is the case a test wants to reach without unplugging anything.
    publisher.disconnect();
    match publisher.send_text("line/mixer/temp", "21.6").await {
        Ok(_) => println!("a disconnected link took a reading, which should never happen"),
        Err(error) => println!("disconnected refused the reading: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(message.topic, "line/mixer/temp");
    assert_eq!(message.payload, b"21.5");
    assert_eq!(deep.topic, "line/mixer/temp/raw");
    assert_eq!(deep.payload, b"2150");
    assert!(publisher
        .send_text("line/mixer/temp", "21.6")
        .await
        .is_err());

    Ok(())
}
