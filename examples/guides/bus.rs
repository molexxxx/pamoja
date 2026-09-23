//! The event bus guide example; see docs/guides/bus.md.
//!
//! Run: `cargo run -p pamoja-examples --example bus`

use std::error::Error;

/// The parts of one weather station talking over one bus: a power monitor, a wind
/// sampler, a heater, a logger, and a radio, none of them holding a reference to
/// another, and what happens to the part that falls behind.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use std::time::Duration;

    use pamoja_bus::EventPublisher;
    use pamoja_core::EventBus;

    // The station's wiring makes one bus and hands each part what it needs: a
    // publisher to announce, an endpoint to listen. No part holds a reference to
    // another, so any of them can be replaced without touching the rest.
    let bus: EventPublisher<String> = EventPublisher::new(2);
    let power = bus.publisher();
    let sampler = bus.publisher();
    let mut heater = bus.subscribe();
    let mut logger = bus.subscribe();

    // One announcement reaches every part that listens, and each reads its own copy.
    let reached = power.publish("battery.low".into());
    println!("power     handed battery.low to {reached} parts");
    let heater_took = heater.next_event().await?.expect("an event");
    println!("heater    took {heater_took}");
    let logger_took = logger.next_event().await?.expect("an event");
    println!("logger    took {logger_took}");

    // Publishing never waits, even while the part's own wait is open, and a part
    // hears what it publishes. The wait borrows the endpoint, so in Rust the heater
    // announces through a publisher of its own.
    let heater_says = heater.publisher();
    let (heard, _) = tokio::join!(heater.next_event(), async {
        heater_says.publish("heater.off".into())
    });
    let heard = heard?.expect("an event");
    println!("heater    heard its own {heard}, sent while it waited");

    // A part that joins late sees only what is published after it subscribes. There
    // is no history to replay.
    let mut radio = bus.subscribe();
    power.publish("battery.ok".into());
    let first = radio.next_event().await?.expect("an event");
    println!("radio     joined late, so the first event it sees is {first}");

    // Each endpoint buffers two events. The logger, busy writing to flash, falls
    // behind while the sampler publishes five readings: it loses the oldest events,
    // resumes with the newest, and counts what it lost.
    for reading in 0..5 {
        sampler.publish(format!("wind {reading}"));
    }
    let resumed = logger.next_event().await?.expect("an event");
    let missed = logger.missed();
    println!("logger    missed {missed} and resumes at {resumed}");
    let newest = logger.next_event().await?.expect("an event");
    println!("logger    then took {newest}");

    // A wait with a limit gives up without taking anything, so a part can do other
    // work between events and lose nothing by it.
    let quiet = Duration::from_millis(50);
    match tokio::time::timeout(quiet, logger.next_event()).await {
        Ok(_) => println!("logger    took an event no one published, which should never happen"),
        Err(_) => println!(
            "logger    heard nothing more within {} ms",
            quiet.as_millis()
        ),
    }
    // ANCHOR_END: example

    assert_eq!(reached, 2);
    assert_eq!(heater_took, "battery.low");
    assert_eq!(logger_took, "battery.low");
    assert_eq!(heard, "heater.off");
    assert_eq!(first, "battery.ok");
    assert_eq!(missed, 5);
    assert_eq!(resumed, "wind 3");
    assert_eq!(newest, "wind 4");

    Ok(())
}
