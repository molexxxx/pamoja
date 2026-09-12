//! The transport ladder guide example; see docs/guides/ladder.md.
//!
//! Run: `cargo run -p pamoja-examples --example ladder`

use std::error::Error;

/// A node with two links and a queue behind them: the cheap hop is tried first, a reading
/// nothing will take is buffered rather than lost, and the backlog goes out exactly once
/// when a link returns.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_core::{Receive, Transport};
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
    gateway.connect().await?;
    gateway.subscribe(topic).await?;

    // Rungs are tried in the order they are added, cheapest first. The mesh hop loses
    // every packet here; the backhaul carries one send, then drops the next two.
    let mut ladder = TransportLadder::new(MemoryStore::new())
        .rung(DegradedLink::new(LoopbackTransport::new(mesh)).drop_every(1))
        .rung(DegradedLink::new(LoopbackTransport::new(backhaul)).intermittent(1, 2));
    ladder.connect().await?;

    // The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
    // broker only that rung publishes to.
    let first = ladder.send_text(topic, "21.5").await?;
    let arrived = gateway.recv().await?.expect("a message");
    let reading = arrived.text().expect("text");
    println!("first reading: {first:?}, gateway got {reading}");

    // Now nothing will take a send, so the next reading is buffered rather than lost.
    let second = ladder.send_text(topic, "21.6").await?;
    let waiting = ladder.buffered().await.expect("a count");
    println!("second reading: {second:?}, {waiting} waiting in the queue");

    // A flush while the links are still down forwards nothing and leaves the backlog
    // intact, because a record is removed only once a rung has accepted it.
    let while_down = ladder.flush().await?;
    let still_queued = ladder.buffered().await.expect("a count");
    println!("flush while down forwarded {while_down}, queue still {still_queued}");

    // The backhaul is reachable again, so the buffered reading goes out exactly once.
    let when_up = ladder.flush().await?;
    let late = gateway.recv().await?.expect("a message");
    let buffered_reading = late.text().expect("text");
    println!("flush when up forwarded {when_up}, gateway got {buffered_reading}");

    // The ladder is a link both ways. A subscription placed on it goes onto every rung
    // that listens, and a receive takes whichever rung delivers, so a command reaches
    // the node over whatever link is up. This one comes back over the backhaul.
    ladder.subscribe("actuators/1/valve").await?;
    gateway.send_text("actuators/1/valve", "open").await?;
    let command = ladder.recv().await?.expect("a command");
    let order = command.text().expect("text");
    println!("command back over the ladder: {order}");
    // ANCHOR_END: example

    assert_eq!(first, Delivery::Sent);
    assert_eq!(arrived.payload, b"21.5");
    assert_eq!(second, Delivery::Buffered);
    assert_eq!(waiting, 1);
    assert_eq!(while_down, 0);
    assert_eq!(when_up, 1);
    assert_eq!(ladder.buffered().await.expect("a count"), 0);
    assert_eq!(late.payload, b"21.6");
    assert_eq!(command.topic, "actuators/1/valve");
    assert_eq!(command.payload, b"open");

    Ok(())
}
