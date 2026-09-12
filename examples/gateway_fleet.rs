//! A site gateway carrying its nodes upstream, and drawing what it carried on a console.
//!
//! This is the shape of a real deployment with the hardware taken out. Nodes publish on the
//! air, the gateway forwards each reading to the link that leaves the site, and the same pass
//! that carries a message reports what crossed, so the console shows the site without a
//! second source of truth. Swap the air side for a `pamoja_radios::mesh::MeshRadio` and the
//! upstream side for `pamoja_mqtt`, and nothing else here changes.
//!
//! A command comes back the other way, under the downlink half of the topic space, and
//! reaches the node with the site and the direction taken off.
//!
//! Run with: `cargo run -p pamoja-examples --example gateway_fleet`

use pamoja_core::{Result, Transport};
use pamoja_dashboard::{Fleet, LinkKind, Reading, Sensor, StateSource, Status};
use pamoja_gateway::bridge::{Bridge, Crossing, Direction};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};

/// What the site is called upstream, so one broker can hold many of them.
const SITE: &str = "sites/ridge";

/// The group the console draws these nodes in.
const GROUP: &str = "ridge";

/// The moisture below which a terrace needs water.
const DRY_PERCENT: f32 = 20.0;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // The console this gateway feeds: one group, on the radio the nodes report over.
    let mut fleet = Fleet::builder()
        .org("farm", "Ridge farm")
        .group("farm", GROUP, "Ridge terraces", LinkKind::Lora)
        .sensor(
            GROUP,
            Sensor::new("soil/1", Reading::new("soil_moisture", 0.0, "percent")),
        )
        .sensor(
            GROUP,
            Sensor::new("soil/2", Reading::new("soil_moisture", 0.0, "percent")),
        )
        .build();

    // Both sides of the gateway. The air is what the nodes are on; upstream is what leaves
    // the site. Only what the nodes publish under soil/ is carried, so a chatty node cannot
    // fill the backhaul with its own logging.
    let air = LoopbackTransport::new(LoopbackBroker::new());
    let upstream = LoopbackTransport::new(LoopbackBroker::new());
    let mut bridge = Bridge::new(air, upstream)
        .with_prefix(SITE)
        .forwarding("soil/#");
    bridge.connect().await?;

    // Three nodes report, and one of them is only talking to itself.
    let heard = [
        ("soil/1", "31.4"),
        ("soil/2", "17.2"),
        ("debug/log", "retrying"),
        ("soil/1", "30.8"),
    ];
    for (topic, reading) in heard {
        bridge.air_mut().send_text(topic, reading).await?;
    }

    // Carrying and reporting are one pass: what crosses is what the console draws.
    for _ in 0..3 {
        let Some(crossed) = bridge.carry_once().await? else {
            break;
        };
        report(&crossed);

        if let Some((sensor, moisture)) = measured(&crossed) {
            fleet.report_reading(
                GROUP,
                &sensor,
                Reading::new("soil_moisture", moisture, "percent")
                    .with_band(DRY_PERCENT, 40.0)
                    .with_status(if moisture < DRY_PERCENT {
                        Status::Warn
                    } else {
                        Status::Ok
                    }),
            );
        }
    }

    // The network answers the terrace that is drying out, and the node sees the plain topic.
    bridge
        .upstream_mut()
        .send_text("sites/ridge/down/valve/1/set", "open")
        .await?;
    if let Some(crossed) = bridge.carry_once().await? {
        report(&crossed);
    }

    let traffic = bridge.traffic();
    println!(
        "\ncarried   {} heard, {} forwarded, {} delivered, {} left alone",
        traffic.heard, traffic.forwarded, traffic.delivered, traffic.dropped
    );

    // The console reads the same fleet a served dashboard would.
    let state = fleet.snapshot();
    for org in &state.orgs {
        for group in &org.groups {
            println!("\nconsole   {}, {:?}", group.name, group.status);
            for sensor in &group.sensors {
                println!(
                    "          {:<8} {:>5.1} {:<8} {:?}",
                    sensor.id, sensor.reading.value, sensor.reading.unit, sensor.reading.status
                );
            }
        }
    }

    Ok(())
}

/// Prints one crossing the way a gateway log would.
///
/// # Arguments
///
/// * `crossed` - what the bridge carried.
fn report(crossed: &Crossing) {
    println!(
        "{:<5} {:<30} {} bytes",
        match crossed.direction {
            Direction::Up => "up",
            Direction::Down => "down",
        },
        crossed.topic,
        crossed.payload.len()
    );
}

/// Reads a forwarded reading back into the sensor it came from and the value it carried.
///
/// # Arguments
///
/// * `crossed` - a message the bridge carried upstream.
///
/// # Returns
///
/// The sensor and its measurement, or `None` for anything that is not one.
fn measured(crossed: &Crossing) -> Option<(String, f32)> {
    if crossed.direction != Direction::Up {
        return None;
    }
    let sensor = crossed
        .topic
        .strip_prefix(SITE)?
        .strip_prefix("/up/")
        .filter(|topic| topic.starts_with("soil/"))?;
    let measurement = core::str::from_utf8(&crossed.payload).ok()?.parse().ok()?;
    Some((sensor.to_owned(), measurement))
}
