//! Offline-first store-and-forward, end to end, with no hardware and no broker.
//!
//! A field sensor encodes its readings with CBOR and buffers them in a durable
//! queue while it has no link. When a link appears, it forwards everything
//! buffered over an in-process loopback transport to a gateway, which decodes and
//! prints each reading. This composes four crates - `pamoja-codec`, `pamoja-sync`,
//! `pamoja-loopback`, and the core traits - through the same shapes every binding
//! exposes.
//!
//! Run with: `cargo run -p pamoja-examples --example store_and_forward`

use pamoja_codec::{CborCodec, Codec};
use pamoja_core::{Receive, Result, Store, Transport};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_sync::{drain_to, MemoryStore};
use serde::{Deserialize, Serialize};

/// A single temperature reading from a field sensor.
#[derive(Debug, Serialize, Deserialize)]
struct Reading {
    sensor: String,
    celsius: f32,
    sequence: u64,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let codec = CborCodec;
    let topic = "sensors/field-1/temperature";

    // The device is offline, so encode each reading and buffer it.
    let mut outbox = MemoryStore::new();
    for sequence in 0..3 {
        let reading = Reading {
            sensor: "field-1".to_owned(),
            celsius: 20.0 + sequence as f32,
            sequence,
        };
        outbox.append(&codec.encode(&reading)?).await?;
    }
    let buffered = outbox.len().await?;
    println!("buffered {buffered} readings while offline");

    // A link appears: connect a gateway (subscriber) and the node (publisher) to a
    // shared in-process broker.
    let broker = LoopbackBroker::new();
    let mut gateway = LoopbackTransport::new(broker.clone());
    let mut node = LoopbackTransport::new(broker);
    gateway.connect().await?;
    node.connect().await?;
    gateway.subscribe("sensors/+/temperature").await?;

    // Drain the buffer, forwarding each encoded reading over the link in order.
    let forwarded = drain_to(&mut outbox, &mut node, topic).await?;
    println!("forwarded {forwarded} readings once the link was up");

    // The gateway receives and decodes everything that was forwarded.
    for _ in 0..buffered {
        let message = gateway.recv().await?.expect("a forwarded reading");
        let reading: Reading = codec.decode(&message.payload)?;
        println!("received on {}: {reading:?}", message.topic);
    }

    println!("done");
    Ok(())
}
