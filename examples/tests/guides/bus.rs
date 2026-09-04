//! The event bus guide example; see docs/guides/bus.md.

/// Fan-out to independent subscribers, a subscriber taken later starting from the next
/// event, and a subscriber further behind than the buffer resuming at the most recent
/// events rather than blocking the publisher.
#[tokio::test]
async fn every_subscriber_gets_its_own_view_of_what_was_published() {
    // ANCHOR: example
    use pamoja_bus::BroadcastBus;
    use pamoja_core::EventBus;

    // A sampler announces a reading and whatever cares about readings picks it up, with
    // neither side holding a reference to the other.
    let hub: BroadcastBus<&str> = BroadcastBus::new(8);
    let mut sampler = hub.subscribe();
    let mut logger = hub.subscribe();

    hub.publish("battery.low").await.unwrap();
    assert_eq!(sampler.next_event().await.unwrap(), Some("battery.low"));
    assert_eq!(logger.next_event().await.unwrap(), Some("battery.low"));

    // A subscriber taken later starts from the next event, so it never sees what went out
    // before it existed.
    let mut late = hub.subscribe();
    hub.publish("link.up").await.unwrap();
    assert_eq!(late.next_event().await.unwrap(), Some("link.up"));
    assert_eq!(sampler.next_event().await.unwrap(), Some("link.up"));

    // The buffer is per subscriber and bounded, so one further behind than the capacity
    // drops what it missed and resumes with the most recent events. A slow reader costs
    // itself, not the publisher.
    let slow: BroadcastBus<u8> = BroadcastBus::new(2);
    let mut reader = slow.subscribe();
    for count in 0..5u8 {
        slow.publish(count).await.unwrap();
    }
    assert_eq!(reader.next_event().await.unwrap(), Some(3));
    assert_eq!(reader.next_event().await.unwrap(), Some(4));
    // ANCHOR_END: example
}
