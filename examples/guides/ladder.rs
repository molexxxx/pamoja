//! The transport ladder guide example; see docs/guides/ladder.md.
//!
//! Run: `cargo run -p pamoja-examples --example ladder`

use std::error::Error;

/// A fishing vessel's reports going ashore over whichever of three links is in
/// reach, a backlog held through a storm, and orders from shore coming back over
/// the same ladder.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use std::time::Duration;

    use pamoja_core::{Receive, Transport};
    use pamoja_ladder::{Delivery, TransportLadder};
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    use pamoja_sync::MemoryStore;

    // Three networks a vessel can reach: the harbor's wifi, the coast's cellular
    // network, and a satellite. Each is a broker with an office ashore listening on
    // it, so which one carried a report is read off that office rather than assumed.
    let harbor = LoopbackBroker::new();
    let coast = LoopbackBroker::new();
    let sky = LoopbackBroker::new();
    let mut harbor_office = LoopbackTransport::new(harbor.clone());
    let mut coast_office = LoopbackTransport::new(coast.clone());
    let mut sky_office = LoopbackTransport::new(sky.clone());
    for office in [&mut harbor_office, &mut coast_office, &mut sky_office] {
        office.connect().await?;
        office.subscribe("vessel/7/report").await?;
    }

    // Rungs go on cheapest first. The satellite only sends, so it goes on as an
    // uplink, which the ladder never subscribes or listens on.
    let mut ladder = TransportLadder::new(MemoryStore::new())
        .rung(LoopbackTransport::new(harbor.clone()))
        .rung(LoopbackTransport::new(coast.clone()))
        .uplink(LoopbackTransport::new(sky.clone()));
    ladder.connect().await?;
    ladder.subscribe("vessel/7/orders").await?;

    // In the harbor, the cheapest link takes the report.
    let first = ladder.send_text("vessel/7/report", "report 1").await?;
    let taken = harbor_office.recv().await?.expect("a report");
    println!("harbor    carried {}", taken.text()?);

    // Past the breakwater the wifi is out of reach and the report falls through to
    // the coast, and further out to the satellite.
    harbor.set_reachable(false);
    ladder.send_text("vessel/7/report", "report 2").await?;
    let taken = coast_office.recv().await?.expect("a report");
    println!(
        "coast     carried {}, with the harbor out of reach",
        taken.text()?
    );
    coast.set_reachable(false);
    ladder.send_text("vessel/7/report", "report 3").await?;
    let taken = sky_office.recv().await?.expect("a report");
    println!(
        "sky       carried {}, with the coast out of reach too",
        taken.text()?
    );

    // In a storm nothing is in reach, and the report waits in the store rather than
    // being lost.
    sky.set_reachable(false);
    let stormy = ladder.send_text("vessel/7/report", "report 4").await?;
    let waiting = ladder.buffered().await?;
    println!("vessel    buffered report 4 with every link out of reach, {waiting} waiting");

    // A flush with every link still out of reach forwards nothing, because a record
    // leaves the store only once a link has taken it.
    let forwarded = ladder.flush().await?;
    let still = ladder.buffered().await?;
    println!(
        "vessel    flushed {forwarded} while every link was out of reach, {still} still waiting"
    );

    // Back in reach of the coast, a flush sends the backlog, oldest first.
    coast.set_reachable(true);
    let forwarded = ladder.flush().await?;
    let taken = coast_office.recv().await?.expect("a report");
    let left = ladder.buffered().await?;
    println!(
        "coast     carried {} on a flush of {forwarded}, {left} waiting",
        taken.text()?
    );

    // Orders from shore come back over whichever listening link is in reach.
    coast_office
        .send_text("vessel/7/orders", "return to port")
        .await?;
    let order = ladder.recv().await?.expect("an order");
    println!("vessel    took {} over the coast network", order.text()?);

    // A ladder does one thing at a time, so a vessel that listens and reports waits
    // for orders with a limit and reports between waits.
    let quiet = Duration::from_millis(50);
    if tokio::time::timeout(quiet, ladder.recv()).await.is_err() {
        println!(
            "vessel    heard nothing more from shore within {} ms",
            quiet.as_millis()
        );
    }
    ladder.send_text("vessel/7/report", "report 5").await?;
    let taken = coast_office.recv().await?.expect("a report");
    println!("coast     carried {} between waits", taken.text()?);
    // ANCHOR_END: example

    assert_eq!(first, Delivery::Sent);
    assert_eq!(stormy, Delivery::Buffered);
    assert_eq!((waiting, still, left), (1, 1, 0));
    assert_eq!(order.text()?, "return to port");
    assert_eq!(taken.text()?, "report 5");

    Ok(())
}
