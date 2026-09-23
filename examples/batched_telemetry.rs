//! Metered-link encoding: pack a batch of readings into a fraction of the bytes.
//!
//! On a long-range radio or a metered cellular link every byte costs power and money,
//! so a node batches its readings and packs them with delta-and-quantize instead of
//! sending each one as a full value. A noisy simulated probe produces a batch of
//! temperatures; packing them with a `Quantizer` is compared against the raw CBOR
//! size, and the batch round-trips back within the quantizer's precision. Composes
//! `pamoja-codec` and `pamoja-sim`, with no hardware.
//!
//! Run with: `cargo run -p pamoja-examples --example batched_telemetry`

use pamoja_codec::{CborCodec, Codec, Quantizer};
use pamoja_core::{Result, Sensor};
use pamoja_sim::SimSensor;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // A probe warms slowly with a little noise; collect a batch of readings.
    let mut probe = SimSensor::new(4.0)
        .with_drift(0.02)
        .with_noise(0.05)
        .with_seed(7);
    let mut batch = Vec::new();
    for _ in 0..30 {
        batch.push(probe.read().await?);
    }

    // Raw: each reading sent as a full CBOR float.
    let cbor = CborCodec;
    let raw = cbor.encode(&batch)?;

    // Metered: quantize to 0.01 C and delta-encode the whole batch.
    let quantizer = Quantizer::new(100.0);
    let packed = quantizer.encode(&batch)?;

    println!("{} readings", batch.len());
    println!("raw CBOR:     {} bytes", raw.len());
    println!("delta-packed: {} bytes", packed.len());
    println!("saved {}%", 100 - packed.len() * 100 / raw.len());

    // The batch round-trips within the quantizer's 0.01 C precision.
    let restored = quantizer.decode(&packed)?;
    let worst = batch
        .iter()
        .zip(&restored)
        .map(|(original, decoded)| (original - decoded).abs())
        .fold(0.0_f32, f32::max);
    println!("largest reading error: {worst:.4} C");

    Ok(())
}
