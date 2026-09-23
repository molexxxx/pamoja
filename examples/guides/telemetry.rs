//! The telemetry guide example; see docs/guides/telemetry.md.
//!
//! Run: `cargo run -p pamoja-examples --example telemetry_guide`

use std::error::Error;

/// A node thinning what it reports as its link gets more expensive, from a free link down to
/// none at all, without losing count of what it decided not to send.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_telemetry::{Event, Level, LinkCost, Reporter};

    // What a node does with an event the reporter hands back: on a link it sends it, and
    // with no link it keeps it for when one returns.
    let fate = |event: &Option<Event>, kept: &'static str| {
        if event.is_some() {
            kept
        } else {
            "counted only"
        }
    };

    // On the site's own network nothing is held back.
    let mut reporter = Reporter::new(Level::Trace);
    reporter.adapt_to(LinkCost::Free);
    let tick = reporter.record(Event::debug("loop.tick"));
    println!(
        "free      nothing is held back: loop.tick {}",
        fate(&tick, "sent")
    );

    // On a metered link the bar rises to Info. Routine detail stops going out; a reading
    // and a warning still do, and a warning carries the measurement that raised it.
    reporter.adapt_to(LinkCost::Metered);
    let bar = reporter.threshold();
    let tick = reporter.record(Event::debug("loop.tick"));
    let reading = reporter.record(Event::info("reading.ok").with_value(4.8));
    println!(
        "metered   nothing below {bar:?} is sent: loop.tick {}, reading.ok {}",
        fate(&tick, "sent"),
        fate(&reading, "sent")
    );
    let warned = reporter
        .record(Event::warn("battery.low").with_value(0.18))
        .expect("a warning is worth a metered link");
    let measured = warned.value.expect("the measurement that raised it");
    println!("metered   {} sent, carrying {measured:.2}", warned.code);

    // On satellite the bar is Warn: the same reading is no longer worth its bytes, and a
    // failure still is.
    reporter.adapt_to(LinkCost::Expensive);
    let bar = reporter.threshold();
    let reading = reporter.record(Event::info("reading.ok").with_value(4.9));
    let lost = reporter.record(Event::error("link.lost"));
    println!(
        "satellite nothing below {bar:?} is sent: reading.ok {}, link.lost {}",
        fate(&reading, "sent"),
        fate(&lost, "sent")
    );

    // With no link at all only errors are kept, for the link's return.
    reporter.adapt_to(LinkCost::Offline);
    let bar = reporter.threshold();
    let low = reporter.record(Event::warn("battery.low").with_value(0.17));
    let lost = reporter.record(Event::error("link.lost"));
    println!(
        "offline   nothing below {bar:?} is kept: battery.low {}, link.lost {}",
        fate(&low, "kept"),
        fate(&lost, "kept")
    );

    // Only the stream was thinned, not the counts, so every event is still accounted for,
    // and the snapshot is what the node ships in place of them.
    let snapshot = reporter.snapshot();
    println!(
        "counts    of {} events, {} passed the bar and {} were counted only",
        reporter.total(),
        snapshot.emitted,
        snapshot.dropped
    );
    let at = |level: Level| snapshot.by_level[level as usize];
    println!(
        "levels    trace {}, debug {}, info {}, warn {}, error {}",
        at(Level::Trace),
        at(Level::Debug),
        at(Level::Info),
        at(Level::Warn),
        at(Level::Error)
    );
    // ANCHOR_END: example

    assert_eq!(reporter.threshold(), Level::Error);
    assert_eq!(warned.code, "battery.low");
    assert_eq!(snapshot.emitted, 5);
    assert_eq!(snapshot.dropped, 3);
    assert_eq!(reporter.total(), 8);

    Ok(())
}
