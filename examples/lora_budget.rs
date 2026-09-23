//! LoRa airtime and duty cycle: what it costs to send a batch over a long-range link.
//!
//! A node packs a batch of readings and asks how expensive it is to send over LoRa. At
//! a high spreading factor the same payload reaches much further but spends far longer
//! on air, and a 1% duty-cycle limit then forces a long silence before the next send.
//! This is the math a long-range deployment needs to stay within regulations and
//! budget its power. Composes `pamoja-lora`, `pamoja-codec`, and `pamoja-sim`, with no
//! hardware.
//!
//! Run with: `cargo run -p pamoja-examples --example lora_budget`

use pamoja_codec::Quantizer;
use pamoja_core::{Result, Sensor};
use pamoja_lora::LinkSettings;
use pamoja_sim::SimSensor;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Collect a batch of readings and pack them for a metered radio link.
    let mut probe = SimSensor::new(4.0)
        .with_drift(0.02)
        .with_noise(0.05)
        .with_seed(7);
    let mut batch = Vec::new();
    for _ in 0..20 {
        batch.push(probe.read().await?);
    }
    let payload = Quantizer::new(100.0).encode(&batch)?;
    println!(
        "{} readings packed into {} bytes",
        batch.len(),
        payload.len()
    );

    // Compare a long-range, slow link against a shorter, fast one, both at 1% duty.
    for (label, link) in [
        ("SF12/125kHz (max range)", LinkSettings::new(12, 125_000)),
        ("SF9/125kHz  (balanced)", LinkSettings::new(9, 125_000)),
        ("SF7/125kHz  (fast)", LinkSettings::new(7, 125_000)),
    ] {
        let airtime_ms = link.airtime_us(payload.len()) as f64 / 1_000.0;
        let off_s = link.min_off_time_us(payload.len(), 10) as f64 / 1_000_000.0;
        println!("{label}: {airtime_ms:6.1} ms on air, then {off_s:6.1} s of silence");
    }

    Ok(())
}
