//! Tamper-evident telemetry: a device signs each reading, a gateway verifies it.
//!
//! A field device encodes a reading, signs the encoded bytes with its private
//! identity, and sends the signature and payload together over a loopback link. The
//! gateway, holding only the device's public identity, verifies every frame before
//! trusting it: an authentic reading is accepted and decoded, while a frame whose
//! payload was altered in flight is rejected. This is the data-integrity and audit-
//! trail foundation the cold-chain and metering deployments need. It composes
//! `pamoja-security`, `pamoja-codec`, and `pamoja-loopback`, with no hardware.
//!
//! Run with: `cargo run -p pamoja-examples --example signed_telemetry`

use pamoja_codec::{CborCodec, Codec};
use pamoja_core::{Receive, Result, Transport};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_security::{DeviceIdentity, PublicIdentity, Signature};
use serde::{Deserialize, Serialize};

/// A single temperature reading from a field sensor.
#[derive(Debug, Serialize, Deserialize)]
struct Reading {
    sensor: String,
    celsius: f32,
}

/// A frame is the 64-byte signature followed by the encoded payload.
fn frame(signature: &Signature, payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64 + payload.len());
    bytes.extend_from_slice(&signature.to_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

/// Splits a frame, verifies it against the trusted identity, and reports the result.
fn check(label: &str, trusted: &PublicIdentity, codec: &CborCodec, frame: &[u8]) {
    let (signature_bytes, body) = frame.split_at(64);
    let signature_bytes: [u8; 64] = signature_bytes.try_into().expect("64-byte signature");
    let signature = Signature::from_bytes(&signature_bytes);
    match trusted.verify(body, &signature) {
        Ok(()) => {
            let reading: Reading = codec.decode(body).expect("an authentic payload decodes");
            println!("{label}: verified {reading:?}");
        }
        Err(_) => println!("{label}: rejected, the signature did not verify"),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let codec = CborCodec;
    let topic = "cold-chain/fridge-1/temperature";

    // The device holds a private identity; the gateway knows only its public half.
    let device = DeviceIdentity::from_seed(&[42u8; 32]);
    let trusted = device.public();
    println!("gateway trusts device {}", trusted.fingerprint());

    let broker = LoopbackBroker::new();
    let mut gateway = LoopbackTransport::new(broker.clone());
    let mut node = LoopbackTransport::new(broker);
    gateway.connect().await?;
    node.connect().await?;
    gateway.subscribe("cold-chain/#").await?;

    // The device signs a genuine reading and sends the signature with it.
    let reading = Reading {
        sensor: "fridge-1".to_owned(),
        celsius: 4.8,
    };
    let payload = codec.encode(&reading)?;
    let signature = device.sign(&payload);
    node.send(topic, &frame(&signature, &payload)).await?;

    let message = gateway.recv().await?.expect("a frame");
    check("genuine reading", &trusted, &codec, &message.payload);

    // Now a tampered frame: the payload is altered after it was signed.
    let mut tampered = frame(&signature, &payload);
    *tampered.last_mut().expect("a non-empty payload") ^= 0xff;
    node.send(topic, &tampered).await?;

    let message = gateway.recv().await?.expect("a frame");
    check("tampered reading", &trusted, &codec, &message.payload);

    Ok(())
}
