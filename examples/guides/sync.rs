//! The store-and-forward guide example; see docs/guides/sync.md.
//!
//! Run: `cargo run -p pamoja-examples --example sync`

use std::error::Error;

/// A hive scale in a remote apiary: weights queue on its card with no link, the
/// queue survives a reboot, a drain that loses its link part-way keeps what it did
/// not send, and a gateway that comes by later takes the rest in order.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_core::{Receive, Store, Transport};
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    use pamoja_sim::DegradedLink;
    use pamoja_sync::{drain_to, FileStore};

    // The scale logs its weight to a queue on its SD card, bounded so a long outage
    // cannot fill the card. The directory is the queue, so the scale can lose power at
    // any moment and lose nothing it logged.
    let dir = std::env::temp_dir().join(format!("pamoja-hive-{}", std::process::id()));
    let topic = "apiary/hive-3/weight";
    let mut outbox = FileStore::open_with_capacity(&dir, 3)?;
    for weight in ["41.2", "41.5", "40.9"] {
        outbox.append_text(weight).await?;
    }
    let logged = outbox.len().await?;
    println!("hive      logged {logged} weights with no link, the most its store holds");

    // A full store refuses the next weight rather than dropping one it already holds.
    let refused = outbox.append_text("41.1").await.expect_err("a full store");
    println!("hive      was refused a 4th: {refused}");

    // The scale reboots. Its queue is the directory, so it comes back whole and in
    // order.
    drop(outbox);
    let mut outbox = FileStore::open_with_capacity(&dir, 3)?;
    let held = outbox.len().await?;
    let oldest = outbox.peek_text().await?.expect("a weight");
    println!("hive      restarted and still holds {held}, oldest first: {oldest}");

    // The cellular uplink carries one weight, then drops. A weight leaves the queue
    // only once a link has taken it, so what the uplink never took stays, in order.
    let cellular = LoopbackBroker::new();
    let mut uplink = DegradedLink::new(LoopbackTransport::new(cellular)).intermittent(1, 10);
    uplink.connect().await?;
    let dropped = drain_to(&mut outbox, &mut uplink, topic)
        .await
        .expect_err("the uplink drops");
    let forwarded = held - outbox.len().await?;
    println!("uplink    forwarded {forwarded}, then failed: {dropped}");
    let left = outbox.len().await?;
    let next = outbox.peek_text().await?.expect("a weight");
    println!("hive      still holds {left}, oldest first: {next}");

    // The beekeeper's gateway comes within reach, and the scale drains the rest onto it.
    let visit = LoopbackBroker::new();
    let mut gateway = LoopbackTransport::new(visit.clone());
    gateway.connect().await?;
    gateway.subscribe(topic).await?;
    let mut to_gateway = LoopbackTransport::new(visit);
    to_gateway.connect().await?;
    drain_to(&mut outbox, &mut to_gateway, topic).await?;
    let mut took = Vec::new();
    for _ in 0..left {
        let weight = gateway.recv().await?.expect("a weight");
        took.push(weight.text()?.to_owned());
    }
    println!(
        "gateway   took {} when the beekeeper came by",
        took.join(", ")
    );
    let empty = outbox.len().await?;
    println!("hive      holds {empty} once the backlog is through");
    // ANCHOR_END: example

    std::fs::remove_dir_all(&dir)?;
    assert_eq!((logged, held, forwarded, left, empty), (3, 3, 1, 2, 0));
    assert_eq!(oldest, "41.2");
    assert_eq!(next, "41.5");
    assert_eq!(took, ["41.5", "40.9"]);

    Ok(())
}
