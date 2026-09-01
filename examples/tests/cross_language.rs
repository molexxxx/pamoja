//! The Rust side of the cross-language conformance suite.
//!
//! Every binding runs these same vectors from `conformance/vectors.json`. This
//! test is the reference: it proves the committed file still matches what the
//! Rust implementation produces, so a stale or hand-edited vector is caught here
//! before a binding is blamed for disagreeing with it.

use std::fs;
use std::path::PathBuf;

use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer};
use pamoja_kit::{
    deadband, Boundary, Calibration, Coordinate, Depletion, Geofence, Pid, Smoother, Thermostat,
};
use pamoja_security::{DeviceIdentity, PublicIdentity, Signature};
use serde_json::Value;

/// Loads the committed vectors.
fn vectors() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the examples crate sits under the repository root")
        .join("conformance")
        .join("vectors.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).expect("parse the vectors")
}

/// Decodes a lowercase hex string from the vectors.
fn unhex(value: &Value) -> Vec<u8> {
    let text = value.as_str().expect("a hex string");
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).expect("hex byte"))
        .collect()
}

/// Reads an array of numbers as `f32`, the width the helpers compute in.
fn floats(value: &Value) -> Vec<f32> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| entry.as_f64().expect("a number") as f32)
        .collect()
}

/// Reads a single number as `f32`.
fn float(value: &Value) -> f32 {
    value.as_f64().expect("a number") as f32
}

#[test]
fn identity_vectors_match() {
    let vectors = vectors();
    let case = &vectors["identity"];

    let seed: [u8; 32] = unhex(&case["seed"]).try_into().expect("a 32-byte seed");
    let device = DeviceIdentity::from_seed(&seed);
    let public = device.public();

    assert_eq!(public.to_bytes().to_vec(), unhex(&case["publicKey"]));
    assert_eq!(public.fingerprint(), case["fingerprint"].as_str().unwrap());

    let payload = case["payload"].as_str().expect("a payload");
    assert_eq!(
        device.sign(payload.as_bytes()).to_bytes().to_vec(),
        unhex(&case["signature"]),
        "the signature is deterministic for this seed and payload"
    );

    let signature: [u8; 64] = unhex(&case["signature"]).try_into().expect("a signature");
    let signature = Signature::from_bytes(&signature);
    let public = PublicIdentity::from_bytes(&unhex(&case["publicKey"]).try_into().unwrap())
        .expect("a valid public key");
    assert!(public.verify(payload.as_bytes(), &signature).is_ok());

    let tampered = case["tamperedPayload"]
        .as_str()
        .expect("a tampered payload");
    assert!(public.verify(tampered.as_bytes(), &signature).is_err());
}

#[test]
fn codec_vectors_match() {
    let vectors = vectors();
    let case = &vectors["codec"];

    let json = case["json"].as_str().expect("a document");
    let cbor = unhex(&case["cbor"]);
    assert_eq!(json_to_cbor(json.as_bytes()).expect("to cbor"), cbor);
    assert_eq!(cbor_to_json(&cbor).expect("to json"), json.as_bytes());

    // The unsorted form encodes to the same bytes, because the keys are sorted on
    // the way through.
    let unsorted = case["unsortedJson"].as_str().expect("a document");
    assert_eq!(json_to_cbor(unsorted.as_bytes()).expect("to cbor"), cbor);

    let deltas = &case["deltas"];
    let samples: Vec<i64> = deltas["samples"]
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| entry.as_i64().expect("an integer"))
        .collect();
    let packed = unhex(&deltas["packed"]);
    assert_eq!(encode_deltas(&samples), packed);
    assert_eq!(decode_deltas(&packed).expect("decode"), samples);

    let quantizer = &case["quantizer"];
    let scale = float(&quantizer["scale"]);
    let readings = floats(&quantizer["readings"]);
    let packed = unhex(&quantizer["packed"]);
    assert_eq!(Quantizer::new(scale).encode(&readings), packed);

    let tolerance = float(&quantizer["tolerance"]);
    let decoded = Quantizer::new(scale).decode(&packed).expect("decode");
    for (got, want) in decoded.iter().zip(readings.iter()) {
        assert!((got - want).abs() <= tolerance);
    }
}

#[test]
fn helper_vectors_match() {
    let vectors = vectors();
    let tolerance = float(&vectors["tolerance"]);

    let case = &vectors["smoother"];
    let mut smoother = Smoother::new(float(&case["weight"]));
    for (sample, want) in floats(&case["samples"])
        .iter()
        .zip(floats(&case["outputs"]).iter())
    {
        assert!((smoother.update(*sample) - want).abs() <= tolerance);
    }

    let case = &vectors["pid"];
    let mut controller = Pid::new(float(&case["kp"]), float(&case["ki"]), float(&case["kd"]));
    let setpoint = float(&case["setpoint"]);
    let dt = float(&case["dt"]);
    for (measurement, want) in floats(&case["measurements"])
        .iter()
        .zip(floats(&case["outputs"]).iter())
    {
        assert!((controller.update(setpoint, *measurement, dt) - want).abs() <= tolerance);
    }

    let case = &vectors["thermostat"];
    let mut thermostat = Thermostat::cooling(float(&case["setpoint"]), float(&case["hysteresis"]));
    for (reading, want) in floats(&case["readings"])
        .iter()
        .zip(case["outputs"].as_array().expect("an array").iter())
    {
        assert_eq!(thermostat.update(*reading), want.as_bool().expect("a bool"));
    }

    let case = &vectors["depletion"];
    let mut depletion = Depletion::new(float(&case["threshold"]));
    for (level, want) in floats(&case["levels"])
        .iter()
        .zip(case["outputs"].as_array().expect("an array").iter())
    {
        let got = depletion.update(*level);
        match want {
            Value::Null => assert_eq!(got, None),
            other => assert_eq!(got, Some(other.as_u64().expect("a count") as u32)),
        }
    }

    let case = &vectors["calibration"];
    let calibration = Calibration::two_point(
        float(&case["rawLow"]),
        float(&case["valueLow"]),
        float(&case["rawHigh"]),
        float(&case["valueHigh"]),
    );
    for (raw, want) in floats(&case["inputs"])
        .iter()
        .zip(floats(&case["outputs"]).iter())
    {
        assert!((calibration.apply(*raw) - want).abs() <= tolerance);
    }

    let case = &vectors["deadband"];
    let center = float(&case["center"]);
    let width = float(&case["width"]);
    for (value, want) in floats(&case["inputs"])
        .iter()
        .zip(floats(&case["outputs"]).iter())
    {
        assert!((deadband(*value, center, width) - want).abs() <= tolerance);
    }
}

#[test]
fn geofence_vectors_match() {
    let vectors = vectors();
    let case = &vectors["geofence"];

    let center = Coordinate::new(
        case["center"]["latitude"].as_f64().expect("a latitude"),
        case["center"]["longitude"].as_f64().expect("a longitude"),
    );
    let mut fence = Geofence::new(center, case["radiusM"].as_f64().expect("a radius"));

    let fixes = case["fixes"].as_array().expect("an array");
    let boundaries = case["boundaries"].as_array().expect("an array");
    assert_eq!(fixes.len(), boundaries.len());

    for (fix, want) in fixes.iter().zip(boundaries.iter()) {
        let point = Coordinate::new(
            fix["latitude"].as_f64().expect("a latitude"),
            fix["longitude"].as_f64().expect("a longitude"),
        );
        let got = match fence.update(point) {
            Boundary::Inside => "Inside",
            Boundary::Outside => "Outside",
            Boundary::Exited => "Exited",
            Boundary::Entered => "Entered",
        };
        assert_eq!(got, want.as_str().expect("a boundary name"));
    }
}
