//! The Rust side of the cross-language conformance suite.
//!
//! Every binding runs these same vectors from `conformance/vectors.json`. This
//! test is the reference: it proves the committed file still matches what the
//! Rust implementation produces, so a stale or hand-edited vector is caught here
//! before a binding is blamed for disagreeing with it.

use std::fs;
use std::path::PathBuf;

use pamoja_can::{dlc_to_len, len_to_dlc, CanId, Frame, J1939Id};
use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer};
use pamoja_gpio::i2c::{Address, Direction};
use pamoja_gpio::pin::{Edge, Level, Polarity};
use pamoja_gpio::spi::Mode;
use pamoja_kit::{
    deadband, Boundary, Calibration, Coordinate, Depletion, Geofence, Pid, Smoother, Thermostat,
};
use pamoja_modbus::Pdu;
use pamoja_modbus::{crc16, Adu};
use pamoja_security::{DeviceIdentity, PublicIdentity, Signature};
use pamoja_serial::{cobs, slip};
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

#[test]
fn serial_vectors_match() {
    let vectors = vectors();
    let case = &vectors["serial"];
    let payload = unhex(&case["payload"]);
    let mut framed = [0u8; 64];
    let mut restored = [0u8; 64];

    let written = slip::encode(&payload, &mut framed).expect("frame the payload");
    assert_eq!(framed[..written].to_vec(), unhex(&case["slipFrame"]));
    let read = slip::decode(&framed[..written], &mut restored).expect("read the frame");
    assert_eq!(restored[..read].to_vec(), payload);

    let written = cobs::encode(&payload, &mut framed).expect("frame the payload");
    assert_eq!(framed[..written].to_vec(), unhex(&case["cobsFrame"]));
    let read = cobs::decode(&framed[..written], &mut restored).expect("read the frame");
    assert_eq!(restored[..read].to_vec(), payload);

    assert_eq!(
        slip::max_encoded_len(payload.len()),
        case["slipMaxEncodedLen"].as_u64().expect("a length") as usize
    );
    assert_eq!(
        cobs::max_encoded_len(payload.len()),
        case["cobsMaxEncodedLen"].as_u64().expect("a length") as usize
    );

    assert!(
        slip::decode(&unhex(&case["corruptSlipFrame"]), &mut restored).is_err(),
        "a frame with a bad escape must be refused"
    );

    let stream = &case["slipStream"];
    let mut decoder: slip::SlipDecoder<64> = slip::SlipDecoder::new();
    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut discarded = 0u64;
    for &byte in &unhex(&stream["bytes"]) {
        match decoder.push(byte) {
            Ok(Some(frame)) => frames.push(frame.to_vec()),
            Ok(None) => {}
            Err(_) => discarded += 1,
        }
    }
    let want: Vec<Vec<u8>> = stream["frames"]
        .as_array()
        .expect("an array")
        .iter()
        .map(unhex)
        .collect();
    assert_eq!(frames, want, "the good frames survive the corrupt one");
    assert_eq!(discarded, stream["discarded"].as_u64().expect("a count"));
}

#[test]
fn modbus_vectors_match() {
    let vectors = vectors();
    let case = &vectors["modbus"];

    let read = &case["readHoldingRegisters"];
    assert_eq!(
        Pdu::read_holding_registers(
            read["start"].as_u64().expect("an address") as u16,
            read["count"].as_u64().expect("a count") as u16,
        )
        .to_adu(read["address"].as_u64().expect("an address") as u8)
        .as_bytes()
        .to_vec(),
        unhex(&read["frame"])
    );

    let crc = &case["crc"];
    assert_eq!(
        u64::from(crc16(&unhex(&crc["data"]))),
        crc["value"].as_u64().expect("a checksum")
    );

    let reply = &case["reply"];
    let parsed = Adu::parse(&unhex(&reply["frame"])).expect("parse the reply");
    assert_eq!(
        u64::from(parsed.address()),
        reply["address"].as_u64().expect("an address")
    );
    let registers: Vec<u64> = parsed
        .response()
        .registers()
        .expect("read the registers")
        .map(u64::from)
        .collect();
    let want: Vec<u64> = reply["registers"]
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| entry.as_u64().expect("a register"))
        .collect();
    assert_eq!(registers, want);

    let bits = &case["bitReply"];
    let parsed = Adu::parse(&unhex(&bits["frame"])).expect("parse the reply");
    let coils: Vec<bool> = parsed
        .response()
        .coils(bits["count"].as_u64().expect("a count") as u16)
        .expect("read the coils")
        .collect();
    let want: Vec<bool> = bits["coils"]
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| entry.as_bool().expect("a coil"))
        .collect();
    assert_eq!(coils, want);

    let refused = &case["exceptionReply"];
    let parsed = Adu::parse(&unhex(&refused["frame"])).expect("parse the reply");
    assert_eq!(
        parsed.exception().map(|code| u64::from(code.code())),
        refused["exception"].as_u64()
    );

    assert!(
        Adu::parse(&unhex(&case["corruptFrame"])).is_err(),
        "a frame mangled on the wire must not reach the application"
    );
}

#[test]
fn can_vectors_match() {
    let vectors = vectors();
    let case = &vectors["can"];

    let classic = &case["classic"];
    let frame = Frame::new(
        CanId::standard(classic["id"].as_u64().expect("an identifier") as u16),
        &unhex(&classic["data"]),
    )
    .expect("build the frame");
    assert_eq!(
        u64::from(frame.dlc()),
        classic["dlc"].as_u64().expect("a length code")
    );

    let fd = &case["fd"];
    let frame = Frame::fd(
        CanId::extended(fd["id"].as_u64().expect("an identifier") as u32),
        &unhex(&fd["data"]),
    )
    .expect("build the frame");
    assert_eq!(
        u64::from(frame.dlc()),
        fd["dlc"].as_u64().expect("a length code")
    );

    let remote = &case["remote"];
    let frame = Frame::remote(
        CanId::standard(remote["id"].as_u64().expect("an identifier") as u16),
        remote["requested"].as_u64().expect("a length") as usize,
    );
    assert_eq!(frame.data().len(), 0, "a remote frame carries no bytes");

    assert!(
        Frame::new(CanId::standard(0x100), &[0u8; 9]).is_err(),
        "a classic frame carries at most eight bytes"
    );
    assert!(
        Frame::fd(CanId::standard(0x100), &[0u8; 13]).is_err(),
        "13 bytes is not a length CAN-FD can carry"
    );

    for entry in case["lengths"].as_array().expect("an array") {
        assert_eq!(
            u64::from(len_to_dlc(entry["len"].as_u64().expect("a length") as usize)),
            entry["dlc"].as_u64().expect("a length code")
        );
    }
    for entry in case["codes"].as_array().expect("an array") {
        assert_eq!(
            dlc_to_len(entry["dlc"].as_u64().expect("a length code") as u8) as u64,
            entry["len"].as_u64().expect("a length")
        );
    }

    for entry in case["j1939"].as_array().expect("an array") {
        let raw = entry["id"].as_u64().expect("an identifier") as u32;
        let message = J1939Id::from_id(CanId::extended(raw)).expect("decode the identifier");
        assert_eq!(
            u64::from(message.pgn()),
            entry["pgn"].as_u64().expect("a parameter group")
        );
        assert_eq!(
            u64::from(message.priority()),
            entry["priority"].as_u64().expect("a priority")
        );
        assert_eq!(
            u64::from(message.source()),
            entry["source"].as_u64().expect("a source")
        );
        assert_eq!(
            message.destination().map(u64::from),
            entry["destination"].as_u64()
        );
        assert_eq!(
            message.is_broadcast(),
            entry["broadcast"].as_bool().expect("a flag")
        );
        assert_eq!(message.to_id().raw(), raw, "the identifier round-trips");
    }

    let standard = case["standardIsNotJ1939"].as_u64().expect("an identifier") as u16;
    assert!(
        J1939Id::from_id(CanId::standard(standard)).is_none(),
        "J1939 never rides an 11-bit identifier"
    );
}

#[test]
fn gpio_vectors_match() {
    let vectors = vectors();
    let case = &vectors["gpio"];

    for entry in case["i2c"].as_array().expect("an array") {
        let value = entry["address"].as_u64().expect("an address") as u16;
        let address = if entry["tenBit"].as_bool().expect("a flag") {
            Address::ten_bit(value).expect("validate the address")
        } else {
            Address::seven_bit(value as u8).expect("validate the address")
        };
        let mut frame = [0u8; 2];
        let written = address
            .write_frame(Direction::Write, &mut frame)
            .expect("frame the address");
        assert_eq!(frame[..written].to_vec(), unhex(&entry["writeFrame"]));
        let written = address
            .write_frame(Direction::Read, &mut frame)
            .expect("frame the address");
        assert_eq!(frame[..written].to_vec(), unhex(&entry["readFrame"]));
        assert_eq!(
            address.frame_len() as u64,
            entry["frameLen"].as_u64().expect("a length")
        );
        assert_eq!(
            address.is_reserved(),
            entry["reserved"].as_bool().expect("a flag")
        );
        assert_eq!(
            address.is_general_call(),
            entry["generalCall"].as_bool().expect("a flag")
        );
    }

    assert!(
        Address::seven_bit(case["outOfRangeSevenBit"].as_u64().expect("an address") as u8).is_err()
    );
    assert!(
        Address::ten_bit(case["outOfRangeTenBit"].as_u64().expect("an address") as u16).is_err()
    );

    for entry in case["spi"].as_array().expect("an array") {
        let number = entry["mode"].as_u64().expect("a mode") as u8;
        let mode = Mode::from_number(number).expect("a valid mode");
        let (cpol, cpha) = mode.cpol_cpha();
        assert_eq!(cpol, entry["cpol"].as_bool().expect("a flag"));
        assert_eq!(cpha, entry["cpha"].as_bool().expect("a flag"));
        assert_eq!(Mode::from_cpol_cpha(cpol, cpha).number(), number);
    }
    assert!(
        Mode::from_number(case["invalidSpiMode"].as_u64().expect("a mode") as u8).is_none(),
        "there are only four SPI modes"
    );

    for entry in case["edges"].as_array().expect("an array") {
        let edge = match entry["edge"].as_str().expect("an edge") {
            "Rising" => Edge::Rising,
            "Falling" => Edge::Falling,
            _ => Edge::Both,
        };
        assert_eq!(
            edge.triggered_by(level(&entry["from"]), level(&entry["to"])),
            entry["triggered"].as_bool().expect("a flag")
        );
    }

    for entry in case["polarities"].as_array().expect("an array") {
        let polarity = match entry["polarity"].as_str().expect("a polarity") {
            "ActiveHigh" => Polarity::ActiveHigh,
            _ => Polarity::ActiveLow,
        };
        let asserted = entry["asserted"].as_bool().expect("a flag");
        assert_eq!(
            level_name(polarity.level(asserted)),
            entry["level"].as_str().expect("a level")
        );
        assert_eq!(
            polarity.is_asserted(polarity.level(asserted)),
            entry["isAsserted"].as_bool().expect("a flag")
        );
    }
}

/// Reads a level back from the name the vectors use.
fn level(value: &Value) -> Level {
    match value.as_str().expect("a level") {
        "Low" => Level::Low,
        _ => Level::High,
    }
}

/// Names a level the way the vectors spell it.
fn level_name(level: Level) -> &'static str {
    match level {
        Level::Low => "Low",
        Level::High => "High",
    }
}
