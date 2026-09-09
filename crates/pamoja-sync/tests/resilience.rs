//! Offline-first resilience under a degraded link.
//!
//! Store-and-forward must deliver everything in order even when the link drops
//! partway through, by retrying. This drives the real forwarder over a loopback
//! transport whose first attempts fail, and asserts nothing is lost, duplicated,
//! or reordered.

use pamoja_core::{Receive, Store, Transport};
use pamoja_loopback::{Faulty, LoopbackBroker, LoopbackTransport};
use pamoja_sync::{drain_to, MemoryStore};

#[tokio::test]
async fn store_and_forward_survives_a_flaky_link() {
    let topic = "sensors/1/data";

    // Buffer five records while offline.
    let mut outbox = MemoryStore::new();
    for value in 0..5u8 {
        outbox.append(&[value]).await.expect("append");
    }

    let broker = LoopbackBroker::new();
    let mut gateway = LoopbackTransport::new(broker.clone());
    gateway.connect().await.expect("connect");
    gateway.subscribe(topic).await.expect("subscribe");

    // The link fails its first two send attempts, then recovers.
    let mut node = Faulty::new(LoopbackTransport::new(broker), 2);
    node.connect().await.expect("connect");

    // Retry draining until the buffer empties. Each failed attempt leaves the
    // records in place, in order, so a later attempt resumes without loss.
    let mut attempts = 0;
    while !outbox.is_empty().await.expect("is_empty") {
        attempts += 1;
        assert!(attempts < 10, "forwarding made no progress");
        let _ = drain_to(&mut outbox, &mut node, topic).await;
    }

    // The gateway received all five records exactly once, in order.
    let mut received = Vec::new();
    for _ in 0..5 {
        let message = gateway.recv().await.expect("recv").expect("a message");
        received.push(message.payload);
    }
    assert_eq!(received, vec![vec![0], vec![1], vec![2], vec![3], vec![4]]);
}
