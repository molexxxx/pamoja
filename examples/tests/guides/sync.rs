//! The store-and-forward guide example; see docs/guides/sync.md.

/// A node with nowhere to send: readings queue up, survive a send that fails part-way,
/// drain in the order they were taken, and a full queue pushes back rather than losing one.
#[tokio::test]
async fn readings_buffered_offline_drain_in_order() {
    // ANCHOR: example
    use pamoja_core::Store;
    use pamoja_sync::MemoryStore;

    // A node with nowhere to send buffers its readings. This queue is held in memory, so
    // it lasts as long as the process; FileStore::open(dir) is the same queue on disk,
    // which is what a node uses to survive a reboot with its backlog intact.
    let mut outbox = MemoryStore::new();
    for reading in [b"20.1", b"20.4", b"20.2"] {
        outbox.append(reading).await.expect("the queue takes it");
    }
    let held = outbox.len().await.expect("a count");
    println!("queued    {held} readings with no link");

    // Peek reads the oldest record without taking it, so a send that fails part-way leaves
    // the queue exactly as it was.
    let oldest = outbox.peek().await.expect("a peek").expect("a record");
    let still_held = outbox.len().await.expect("a count");
    let oldest_reading = String::from_utf8_lossy(&oldest);
    println!("oldest    {oldest_reading} and still {still_held} held");

    // The link returns and the queue drains oldest first, in the order the readings were
    // taken rather than the order they happen to come back off a buffer.
    let mut drained = Vec::new();
    while let Some(record) = outbox.pop().await.expect("a pop") {
        drained.push(String::from_utf8_lossy(&record).into_owned());
    }
    println!("drained   {}", drained.join(", "));

    // A bounded queue refuses the append that would overflow it. A full store is
    // backpressure the caller is told about, not a reading dropped behind its back.
    let mut bounded = MemoryStore::with_capacity(2);
    bounded.append(b"20.1").await.expect("room");
    bounded.append(b"20.4").await.expect("room");
    match bounded.append(b"20.2").await {
        Ok(()) => println!("a full queue took a third reading, which should never happen"),
        Err(error) => println!("full      refused the third reading: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(oldest, b"20.1".to_vec());
    assert_eq!(still_held, 3);
    assert_eq!(drained, ["20.1", "20.4", "20.2"]);
    assert_eq!(outbox.len().await.expect("a count"), 0);
    assert_eq!(bounded.len().await.expect("a count"), 2);
}
