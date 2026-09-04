//! The store-and-forward guide example; see docs/guides/sync.md.

/// Readings buffered while a node has no link, peeked without being taken, drained back in
/// the order they arrived, and a bounded queue that refuses an overflowing append rather
/// than dropping a reading.
#[tokio::test]
async fn readings_buffered_offline_drain_in_order() {
    // ANCHOR: example
    use pamoja_core::Store;
    use pamoja_sync::MemoryStore;

    // A node with nowhere to send buffers its readings. This queue is held in memory, so
    // it lasts as long as the process; FileStore::open(dir) is the same queue on disk.
    let mut outbox = MemoryStore::new();
    for reading in [b"20.1", b"20.4", b"20.2"] {
        outbox.append(reading).await.unwrap();
    }
    assert_eq!(outbox.len().await.unwrap(), 3);

    // Peek reads the oldest record without taking it, so a send that fails part-way leaves
    // the queue exactly as it was.
    assert_eq!(outbox.peek().await.unwrap(), Some(b"20.1".to_vec()));
    assert_eq!(outbox.len().await.unwrap(), 3);

    // The link returns and the queue drains oldest first, in the order the readings were
    // taken rather than the order they happen to come back off a buffer.
    let mut drained = Vec::new();
    while let Some(record) = outbox.pop().await.unwrap() {
        drained.push(record);
    }
    assert_eq!(
        drained,
        [b"20.1".to_vec(), b"20.4".to_vec(), b"20.2".to_vec()]
    );
    assert_eq!(outbox.len().await.unwrap(), 0);

    // A bounded queue refuses the append that would overflow it. A full store is
    // backpressure the caller is told about, not a reading dropped behind its back.
    let mut bounded = MemoryStore::with_capacity(2);
    bounded.append(b"20.1").await.unwrap();
    bounded.append(b"20.4").await.unwrap();
    assert!(bounded.append(b"20.2").await.is_err());
    assert_eq!(bounded.len().await.unwrap(), 2);
    // ANCHOR_END: example
}
