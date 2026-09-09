//! Offline-first store-and-forward over a flaky link, run end to end.
//!
//! This is the resilience conformance scenario: a field node buffers its readings in a
//! durable queue and keeps trying to forward the backlog over a link that is reachable
//! only part of the time. A degraded send leaves the record and the rest queued, in order,
//! so the backlog rises while the link is down and falls when it returns, and once the
//! link has cycled enough every reading has reached the gateway in order with nothing lost.
//! The run returns what it observed, so a test can assert offline-first held and an example
//! can narrate the backlog rising and falling.
//!
//! It composes `pamoja-sim`'s degraded link with `pamoja-sync` store-and-forward,
//! `pamoja-codec`, and `pamoja-loopback`, with no hardware and no broker.

use pamoja_codec::{CborCodec, Codec};
use pamoja_core::{Receive, Result, Store, Transport};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_sim::DegradedLink;
use pamoja_sync::{drain_to, MemoryStore};
use serde::{Deserialize, Serialize};

// The number of readings the run buffers and forwards.
const READINGS: u64 = 8;
// The topic the node forwards to and the gateway subscribes to.
const TOPIC: &str = "sensors/field-1/temperature";

/// One reading on the wire, tagged with its place in the sequence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Reading {
    sequence: u64,
    celsius: f32,
}

/// What one run of the scenario observed.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// How many readings the node produced.
    pub total: u64,
    /// The sequence numbers the gateway received, in the order they arrived.
    pub received: Vec<u64>,
    /// The backlog still waiting on the link after each produce-and-drain cycle.
    pub backlog_per_cycle: Vec<usize>,
    /// How many times the run attempted to drain the queue.
    pub drain_attempts: usize,
}

impl Outcome {
    /// Returns the largest backlog that ever waited on the link.
    ///
    /// # Returns
    ///
    /// The peak backlog, which is positive when the link genuinely went down and forced
    /// the node to buffer.
    pub fn peak_backlog(&self) -> usize {
        self.backlog_per_cycle.iter().copied().max().unwrap_or(0)
    }
}

/// Runs the degraded-link scenario once and returns what it observed.
///
/// # Returns
///
/// The [`Outcome`]: the sequence numbers the gateway received in order, the backlog after
/// each cycle, and how many drain attempts it took.
///
/// # Errors
///
/// Returns an [`Error`](pamoja_core::Error) if a composed step fails, such as a codec
/// round-trip, which a passing run never does. A degraded send is expected and is not an
/// error here; it leaves the records queued to retry.
pub async fn run() -> Result<Outcome> {
    let codec = CborCodec;

    // A gateway listens on the broker; the node reaches it over a link that is up two
    // sends, then down two, repeating.
    let broker = LoopbackBroker::new();
    let mut gateway = LoopbackTransport::new(broker.clone());
    gateway.connect().await?;
    gateway.subscribe("sensors/+/temperature").await?;

    let inner = LoopbackTransport::new(broker);
    let mut link = DegradedLink::new(inner).intermittent(2, 2);
    link.connect().await?;

    let mut outbox = MemoryStore::new();
    let mut backlog_per_cycle = Vec::with_capacity(READINGS as usize);
    let mut drain_attempts = 0;

    // Each cycle: take a reading, buffer it, and try to push the backlog out.
    for sequence in 0..READINGS {
        let reading = Reading {
            sequence,
            celsius: 20.0 + sequence as f32,
        };
        outbox.append(&codec.encode(&reading)?).await?;
        let _ = drain_to(&mut outbox, &mut link, TOPIC).await;
        drain_attempts += 1;
        backlog_per_cycle.push(outbox.len().await?);
    }

    // Keep retrying until the link has carried the whole backlog.
    while !outbox.is_empty().await? {
        let _ = drain_to(&mut outbox, &mut link, TOPIC).await;
        drain_attempts += 1;
    }

    // Collect everything the gateway received, in the order it arrived.
    let mut received = Vec::with_capacity(READINGS as usize);
    while (received.len() as u64) < READINGS {
        let message = gateway.recv().await?.expect("a forwarded reading");
        let reading: Reading = codec.decode(&message.payload)?;
        received.push(reading.sequence);
    }

    Ok(Outcome {
        total: READINGS,
        received,
        backlog_per_cycle,
        drain_attempts,
    })
}
