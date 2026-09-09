//! A cold-chain fridge node and its gateway, run end to end over loopback.
//!
//! This is the SDK's conformance scenario: one run that threads a reading through every
//! layer a real cold-chain deployment uses, so the crates are proven to compose rather
//! than only to work in isolation. A simulated probe warms past its safe range; each
//! reading is judged by the device profile, encoded by the codec, signed and chained
//! into a tamper-evident log, accounted for by the metered-link telemetry, and published
//! over the in-process loopback link to a gateway that verifies and decodes it. The run
//! returns everything it observed, so a test can assert each seam held and an example can
//! narrate the story.
//!
//! It composes `pamoja-sim`, `pamoja-profile`, `pamoja-codec`, `pamoja-security`,
//! `pamoja-audit`, `pamoja-telemetry`, and `pamoja-loopback` over `pamoja-core`, with no
//! hardware and no broker.

use pamoja_audit::{verify_chain, AuditLog, Entry};
use pamoja_codec::{encode_deltas, CborCodec, Codec, Quantizer};
use pamoja_core::{Receive, Result, Sensor, Transport};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_profile::{Alert, Profile, Reaction};
use pamoja_security::{DeviceIdentity, Signature};
use pamoja_sim::SimSensor;
use pamoja_telemetry::{Event, Level, LinkCost, Reporter};
use serde::{Deserialize, Serialize};

// The fixed seed for the device identity, so the run is reproducible.
const DEVICE_SEED: [u8; 32] = [7u8; 32];
// The number of readings the afternoon run takes.
const STEPS: usize = 10;
// The quantizer scale: 100 steps per unit, so 0.01 C of precision.
const QUANT_SCALE: f32 = 100.0;
// The topic the device publishes to, matched by the gateway's subscription.
const TOPIC: &str = "cold-chain/fridge/temperature";

/// One temperature reading on the wire.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Reading {
    step: u32,
    celsius: f32,
}

/// What happened at one step of the run.
#[derive(Debug, Clone, Copy)]
pub struct StepRecord {
    /// The step number, from zero.
    pub step: usize,
    /// The reading the device sampled.
    pub sent: f32,
    /// The reading the gateway decoded from the frame it received.
    pub received: f32,
    /// Whether the gateway verified the frame's signature.
    pub verified: bool,
    /// The profile's decision for this reading.
    pub reaction: Reaction,
    /// The link cost in force when this reading was reported.
    pub link_cost: LinkCost,
    /// Whether telemetry shipped this reading's event rather than dropping it.
    pub shipped: bool,
}

/// Everything one run of the scenario observed, for a test to assert on or an example to
/// narrate.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The profile the device ran.
    pub profile_name: String,
    /// The device's public fingerprint.
    pub fingerprint: String,
    /// One record per step of the run.
    pub steps: Vec<StepRecord>,
    /// Whether the gateway rejected a frame whose payload was altered after signing.
    pub tamper_rejected: bool,
    /// Whether the genuine audit chain verified against the device's public identity.
    pub genuine_chain_ok: bool,
    /// Whether an audit chain with one edited reading still verified (it must not).
    pub edited_chain_ok: bool,
    /// The size in bytes of the batch of readings encoded as raw CBOR.
    pub raw_bytes: usize,
    /// The size in bytes of the same batch packed with the quantizer.
    pub packed_bytes: usize,
    /// The largest error any reading picked up across a quantizer round-trip.
    pub max_quant_error: f32,
    /// The quantizer's precision, the largest error it is allowed to introduce.
    pub quant_precision: f32,
    /// Whether the profile reloaded from its JSON manifest made identical decisions.
    pub reloaded_matches: bool,
    /// The profile's shareable JSON manifest.
    pub manifest: String,
    /// How many telemetry events the run produced.
    pub telemetry_total: u32,
    /// How many of those events telemetry shipped.
    pub telemetry_emitted: u32,
    /// How many of those events telemetry dropped to spare the link.
    pub telemetry_dropped: u32,
    /// How many error-level events the run produced.
    pub error_events: u32,
    /// Whether every excursion's event was shipped despite the rising link cost.
    pub excursion_shipped: bool,
    /// The size in bytes of the delta-packed telemetry snapshot.
    pub snapshot_packed_bytes: usize,
}

// A frame is the 64-byte signature followed by the encoded payload.
fn frame(signature: &Signature, payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64 + payload.len());
    bytes.extend_from_slice(&signature.to_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

// The link cost at a step: a free link early, then metered, then expensive, so the run
// exercises telemetry raising its bar as the link degrades.
fn link_cost_at(step: usize) -> LinkCost {
    match step * 3 / STEPS {
        0 => LinkCost::Free,
        1 => LinkCost::Metered,
        _ => LinkCost::Expensive,
    }
}

/// Runs the cold-chain scenario once and returns what it observed.
///
/// # Returns
///
/// The [`Outcome`] of the run: the per-step records and the results of the closing
/// integrity, metered-link, and profile-as-data checks.
///
/// # Errors
///
/// Returns an [`Error`](pamoja_core::Error) if any composed step fails, such as a codec
/// round-trip or a loopback send, which a passing run never does.
pub async fn run() -> Result<Outcome> {
    let codec = CborCodec;

    // The device holds a private identity; the gateway and auditor know only its public
    // half. A second identity from the same seed signs frames while the first records the
    // audit log, since the log takes ownership of the identity it signs with.
    let signer = DeviceIdentity::from_seed(&DEVICE_SEED);
    let public = signer.public();
    let mut audit = AuditLog::new(DeviceIdentity::from_seed(&DEVICE_SEED));

    // The profile supplies the control policy; a fresh controller tracks its state.
    let profile = Profile::vaccine_fridge_monitor();
    let mut controller = profile.controller();

    // A gateway and the node share an in-process broker.
    let broker = LoopbackBroker::new();
    let mut node = LoopbackTransport::new(broker.clone());
    let mut gateway = LoopbackTransport::new(broker);
    node.connect().await?;
    gateway.connect().await?;
    gateway.subscribe("cold-chain/#").await?;

    // A probe that warms steadily from inside the safe range up through it.
    let mut probe = SimSensor::new(5.2)
        .with_drift(0.5)
        .with_noise(0.05)
        .with_seed(42);
    let mut reporter = Reporter::new(Level::Trace);

    let mut steps = Vec::with_capacity(STEPS);
    let mut sent_readings = Vec::with_capacity(STEPS);
    let mut stored: Vec<Vec<u8>> = Vec::with_capacity(STEPS);
    let mut excursion_shipped = true;

    for step in 0..STEPS {
        let cost = link_cost_at(step);
        reporter.adapt_to(cost);

        // Sim sensor -> profile -> codec -> security -> audit.
        let reading = probe.read().await?;
        sent_readings.push(reading);
        let reaction = controller.evaluate(reading);
        let payload = codec.encode(&Reading {
            step: step as u32,
            celsius: reading,
        })?;
        let signature = signer.sign(&payload);
        let entry = audit.append(&payload);
        stored.push(entry.to_bytes());

        // Telemetry: a routine tick that the rising link cost will drop, and the reading
        // itself, raised to an error when the fridge leaves its safe range so it ships
        // even on the most expensive link.
        reporter.record(Event::debug("loop.tick"));
        let is_excursion = matches!(reaction.alert, Some(Alert::OutOfRange { .. }));
        let event = if is_excursion {
            Event::error("fridge.excursion").with_value(reading)
        } else {
            Event::info("fridge.reading").with_value(reading)
        };
        let shipped = reporter.record(event).is_some();
        if is_excursion && !shipped {
            excursion_shipped = false;
        }

        // Publish the signed frame; the gateway verifies and decodes it.
        node.send(TOPIC, &frame(&signature, &payload)).await?;
        let message = gateway.recv().await?.expect("a frame over loopback");
        let (signature_bytes, body) = message.payload.split_at(64);
        let signature_bytes: [u8; 64] = signature_bytes.try_into().expect("64-byte signature");
        let verified = public
            .verify(body, &Signature::from_bytes(&signature_bytes))
            .is_ok();
        let received: Reading = codec.decode(body)?;

        steps.push(StepRecord {
            step,
            sent: reading,
            received: received.celsius,
            verified,
            reaction,
            link_cost: cost,
            shipped,
        });
    }

    // Security: a frame whose payload was altered after signing must be rejected.
    let payload = codec.encode(&Reading {
        step: STEPS as u32,
        celsius: 4.8,
    })?;
    let mut tampered = frame(&signer.sign(&payload), &payload);
    *tampered.last_mut().expect("a non-empty payload") ^= 0xff;
    node.send(TOPIC, &tampered).await?;
    let message = gateway.recv().await?.expect("a frame over loopback");
    let (signature_bytes, body) = message.payload.split_at(64);
    let signature_bytes: [u8; 64] = signature_bytes.try_into().expect("64-byte signature");
    let tamper_rejected = public
        .verify(body, &Signature::from_bytes(&signature_bytes))
        .is_err();

    // Audit: the genuine chain verifies; editing any one stored reading breaks it.
    let entries: Vec<Entry> = stored
        .iter()
        .map(|bytes| Entry::from_bytes(bytes))
        .collect::<Result<_>>()?;
    let genuine_chain_ok = verify_chain(&public, &entries).is_ok();
    let mut edited = stored.clone();
    if let Some(byte) = edited[STEPS / 2].last_mut() {
        *byte ^= 0xff;
    }
    let edited_chain_ok = match edited
        .iter()
        .map(|bytes| Entry::from_bytes(bytes))
        .collect::<Result<Vec<Entry>>>()
    {
        Ok(entries) => verify_chain(&public, &entries).is_ok(),
        Err(_) => false,
    };

    // Codec, metered link: the batch packs smaller than raw and round-trips in precision.
    let raw = codec.encode(&sent_readings)?;
    let quantizer = Quantizer::new(QUANT_SCALE);
    let packed = quantizer.encode(&sent_readings);
    let restored = quantizer.decode(&packed)?;
    let max_quant_error = sent_readings
        .iter()
        .zip(&restored)
        .map(|(original, decoded)| (original - decoded).abs())
        .fold(0.0_f32, f32::max);

    // Profile as data: the same profile reloaded from JSON decides identically.
    let manifest = profile.to_json()?;
    let mut reloaded = Profile::from_json(&manifest)?.controller();
    let reloaded_matches = sent_readings
        .iter()
        .zip(&steps)
        .all(|(&reading, record)| reloaded.evaluate(reading) == record.reaction);

    // Telemetry: a compact snapshot of the counters in place of the raw stream.
    let snapshot = reporter.snapshot();
    let counters: Vec<i64> = snapshot
        .by_level
        .iter()
        .map(|&count| count as i64)
        .collect();
    let snapshot_packed_bytes = encode_deltas(&counters).len();

    Ok(Outcome {
        profile_name: profile.name,
        fingerprint: public.fingerprint(),
        steps,
        tamper_rejected,
        genuine_chain_ok,
        edited_chain_ok,
        raw_bytes: raw.len(),
        packed_bytes: packed.len(),
        max_quant_error,
        quant_precision: 1.0 / QUANT_SCALE,
        reloaded_matches,
        manifest,
        telemetry_total: reporter.total(),
        telemetry_emitted: reporter.emitted(),
        telemetry_dropped: reporter.dropped(),
        error_events: reporter.count(Level::Error),
        excursion_shipped,
        snapshot_packed_bytes,
    })
}
