//! The event bus guide example; see docs/guides/bus.md.

/// Parts of one node talking to each other without holding references to each other, and
/// what happens to a subscriber that falls too far behind.
#[tokio::test]
async fn every_subscriber_gets_its_own_view_of_what_was_published() {
    // ANCHOR: example
    use pamoja_bus::BroadcastBus;
    use pamoja_core::EventBus;

    // A sampler announces something and whatever cares picks it up, with neither side
    // holding a reference to the other. This is how the parts of one node are wired.
    let hub: BroadcastBus<&str> = BroadcastBus::new(8);
    let mut control = hub.subscribe();
    let mut logger = hub.subscribe();

    hub.publish("battery.low").await.expect("published");
    let to_control = control
        .next_event()
        .await
        .expect("a live bus")
        .expect("an event");
    let to_logger = logger
        .next_event()
        .await
        .expect("a live bus")
        .expect("an event");
    println!("control saw {to_control}, the logger saw {to_logger}");

    // A subscriber taken later starts from the next event, so it never sees what went out
    // before it existed.
    let mut late = hub.subscribe();
    hub.publish("link.up").await.expect("published");
    let first_seen = late
        .next_event()
        .await
        .expect("a live bus")
        .expect("an event");
    println!("the late subscriber's first event is {first_seen}");

    // The buffer is per subscriber and bounded, so one further behind than the capacity
    // drops what it missed and resumes with the most recent events. A slow reader costs
    // itself, not the publisher.
    let slow: BroadcastBus<u8> = BroadcastBus::new(2);
    let mut reader = slow.subscribe();
    for count in 0..5u8 {
        slow.publish(count).await.expect("published");
    }
    let resumed = reader
        .next_event()
        .await
        .expect("a live bus")
        .expect("an event");
    println!("after five events into a buffer of two, the reader resumes at {resumed}");
    // ANCHOR_END: example

    assert_eq!(to_control, "battery.low");
    assert_eq!(to_logger, "battery.low");
    assert_eq!(first_seen, "link.up");
    assert_eq!(
        control.next_event().await.expect("an event"),
        Some("link.up")
    );
    assert_eq!(resumed, 3);
    assert_eq!(reader.next_event().await.expect("an event"), Some(4));
}
