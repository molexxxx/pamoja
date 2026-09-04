//! The telemetry guide example; see docs/guides/telemetry.md.

/// A node reporting over a link that gets more expensive twice over, shipping less detail
/// each time while its counters stay complete.
#[test]
fn a_costlier_link_thins_the_stream_but_not_the_counts() {
    // ANCHOR: example
    use pamoja_telemetry::{Event, Level, LinkCost, Reporter};

    // The node is willing to record everything, then finds out it is reporting over a
    // metered link, which puts the bar at Info.
    let mut reporter = Reporter::new(Level::Trace);
    reporter.adapt_to(LinkCost::Metered);
    assert_eq!(reporter.threshold(), Level::Info);

    // Routine detail stops going out. A reading and the warning that follows it still do,
    // and a shipped event comes back with the measurement that triggered it.
    assert!(reporter.record(Event::debug("loop.tick")).is_none());
    let reading = Event::info("reading.ok").with_value(4.8);
    assert!(reporter.record(reading).is_some());
    let warned = reporter
        .record(Event::warn("battery.low").with_value(0.18))
        .expect("a warning is worth a metered link");
    assert_eq!(warned.code, "battery.low");
    assert_eq!(warned.value, Some(0.18));

    // The node falls back to satellite, which raises the bar to Warn. The same reading is
    // no longer worth its bytes; a failure still is.
    reporter.adapt_to(LinkCost::Expensive);
    let reading = Event::info("reading.ok").with_value(4.9);
    assert!(reporter.record(reading).is_none());
    assert!(reporter.record(Event::error("link.lost")).is_some());

    // Only the stream was thinned, not the counts, so all five events are still accounted
    // for and the snapshot is what the node ships in place of them.
    let counts = reporter.snapshot();
    assert_eq!(counts.by_level[Level::Info as usize], 2);
    assert_eq!(counts.emitted, 3);
    assert_eq!(counts.dropped, 2);
    assert_eq!(reporter.total(), 5);

    // Offline is the last rung: a node with no link at all still keeps its failures.
    assert_eq!(LinkCost::Offline.threshold(), Level::Error);
    // ANCHOR_END: example
}
