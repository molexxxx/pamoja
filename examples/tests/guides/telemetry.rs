//! The telemetry guide example; see docs/guides/telemetry.md.

/// A node thinning what it reports as its link gets more expensive, without losing count
/// of what it decided not to send.
#[test]
fn a_costlier_link_thins_the_stream_but_not_the_counts() {
    // ANCHOR: example
    use pamoja_telemetry::{Event, Level, LinkCost, Reporter};

    // The node is willing to record everything, then finds out it is reporting over a
    // metered link, which puts the bar at Info.
    let mut reporter = Reporter::new(Level::Trace);
    reporter.adapt_to(LinkCost::Metered);
    let bar = reporter.threshold();
    println!("on a metered link, nothing below {bar:?} is sent");

    // Routine detail stops going out. A reading and the warning that follows it still do,
    // and a shipped event comes back with the measurement that triggered it.
    let tick = reporter.record(Event::debug("loop.tick"));
    println!("loop.tick sent: {}", tick.is_some());
    let reading = reporter.record(Event::info("reading.ok").with_value(4.8));
    println!("reading.ok sent: {}", reading.is_some());
    let warned = reporter
        .record(Event::warn("battery.low").with_value(0.18))
        .expect("a warning is worth a metered link");
    let measured = warned.value.expect("the measurement that triggered it");
    println!("sent      {} carrying {measured}", warned.code);

    // The node falls back to satellite, which raises the bar to Warn. The same reading is
    // no longer worth its bytes; a failure still is.
    reporter.adapt_to(LinkCost::Expensive);
    let dearer = reporter.record(Event::info("reading.ok").with_value(4.9));
    let lost = reporter.record(Event::error("link.lost"));
    println!("on satellite, reading.ok sent: {}", dearer.is_some());
    println!("on satellite, link.lost sent: {}", lost.is_some());

    // Only the stream was thinned, not the counts, so every event is still accounted for
    // and the snapshot is what the node ships in place of them.
    let counts = reporter.snapshot();
    let (seen, sent, only_counted) = (reporter.total(), counts.emitted, counts.dropped);
    println!("of {seen} events, {sent} went out and {only_counted} were counted only");
    // ANCHOR_END: example

    assert_eq!(reporter.threshold(), Level::Warn);
    assert!(tick.is_none());
    assert!(reading.is_some());
    assert_eq!(warned.code, "battery.low");
    assert_eq!(warned.value, Some(0.18));
    assert!(dearer.is_none());
    assert!(lost.is_some());
    assert_eq!(counts.by_level[Level::Info as usize], 2);
    assert_eq!(counts.emitted, 3);
    assert_eq!(counts.dropped, 2);
    assert_eq!(reporter.total(), 5);

    // Offline is the last rung: a node with no link at all still keeps its failures.
    assert_eq!(LinkCost::Offline.threshold(), Level::Error);
}
