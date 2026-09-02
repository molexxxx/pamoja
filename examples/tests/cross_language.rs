//! The Rust side of the cross-language conformance suite.
//!
//! Every binding runs these same vectors from `conformance/vectors.json`. This
//! test is the reference: it proves the committed file still matches what the
//! Rust implementation produces, so a stale or hand-edited vector is caught here
//! before a binding is blamed for disagreeing with it.

use std::fs;
use std::path::PathBuf;

use pamoja_actuators::{pca9685, stepper};
use pamoja_audit::{verify_chain, AuditLog, Entry};
use pamoja_can::{dlc_to_len, len_to_dlc, CanId, Frame, J1939Id};
use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer};
use pamoja_core::{Actuator as _, Sensor as _, Transport as _};
use pamoja_gpio::i2c::{Address, Direction};
use pamoja_gpio::pin::{Edge, Level, Polarity};
use pamoja_gpio::spi::Mode;
use pamoja_kit::{
    deadband, Anomaly, Boundary, Calibration, Coordinate, Depletion, Geofence, Median, Pid,
    Smoother, Thermostat, Trend, Window,
};
use pamoja_ladder::{Delivery, TransportLadder};
use pamoja_loopback::{Faulty, LoopbackBroker, LoopbackTransport};
use pamoja_lora::region::{
    ChannelBlock, ChannelPlan, ChannelPlanBuilder, DataRate, MaxPayload, Modulation, PayloadTable,
    Region, SubBand,
};
use pamoja_lora::LinkSettings;
use pamoja_lorawan::{
    Device, Direction as LorawanDirection, Downlink, FrameHeader, JoinGrant, JoinRequest,
    MessageType, Session, Uplink,
};
use pamoja_mesh::{crc16 as mesh_crc16, DynamicSeenCache, Frame as MeshFrame};
use pamoja_modbus::Pdu;
use pamoja_modbus::{crc16, Adu};
use pamoja_power::{DutyCycle, PowerMode, PowerPlan};
use pamoja_profile::{Alert, ControlSpec, Controller, Profile};
use pamoja_ros2::key::entity_key;
use pamoja_ros2::msg::{CdrReader, Twist as Ros2Twist, Vector3};
use pamoja_ros2::name::{dds_topic, is_fully_qualified, is_valid_name, percent_mangle, EntityKind};
use pamoja_ros2::typehash::{dds_type_name, TypeHash};
use pamoja_routing::{DynamicRouter, Forward};
use pamoja_security::{DeviceIdentity, PublicIdentity, Signature};
use pamoja_sensors::{ads1115, bme280, ds18b20, ina219};
use pamoja_serial::{cobs, slip};
use pamoja_session::{AgreementKey, Role, Sealed, Session as SecuredSession, SessionError};
use pamoja_sim::{Replay, SimRobot, SimSensor};
use pamoja_sync::MemoryStore as BufferStore;
use pamoja_telemetry::{Event, Level as TelemetryLevel, LinkCost, Reporter};
use pamoja_update::{
    Boot, Delegation, Device as UpdateDevice, Envelope, Manifest, MemoryStore, Refusal, SlotState,
    SlotStore, Updater,
};
use pamoja_zenoh::keyexpr;
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

/// Renders bytes as the lowercase hex the vectors carry.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
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

    // Registers above 0x7FFF, which catch a binding that reads them as signed.
    let high = &case["highRegisterReply"];
    let parsed = Adu::parse(&unhex(&high["frame"])).expect("parse the reply");
    let registers: Vec<u64> = parsed
        .response()
        .registers()
        .expect("read the registers")
        .map(u64::from)
        .collect();
    let want: Vec<u64> = high["registers"]
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

#[test]
fn sensor_vectors_match() {
    let vectors = vectors();
    let case = &vectors["sensors"];

    let bme = &case["bme280"];
    let temp_press: [u8; 26] = unhex(&bme["calibrationTempPress"])
        .try_into()
        .expect("26 calibration bytes");
    let humidity: [u8; 7] = unhex(&bme["calibrationHumidity"])
        .try_into()
        .expect("7 calibration bytes");
    let measurement: [u8; 8] = unhex(&bme["measurement"])
        .try_into()
        .expect("8 measurement bytes");
    let reading = bme280::Calibration::from_registers(&temp_press, &humidity)
        .compensate(&bme280::RawMeasurement::from_registers(&measurement));
    assert!((reading.celsius() - float(&bme["celsius"])).abs() < 1e-3);
    assert_eq!(
        u64::from(reading.pascals()),
        bme["pascals"].as_u64().expect("a pressure")
    );
    assert!(
        (reading.relative_humidity_percent() - float(&bme["relativeHumidityPercent"])).abs() < 1e-3
    );

    let ds = &case["ds18b20"];
    let scratchpad: [u8; 9] = unhex(&ds["scratchpad"]).try_into().expect("9 bytes");
    let decoded = ds18b20::Scratchpad::parse(&scratchpad).expect("parse the scratchpad");
    assert_eq!(
        i64::from(decoded.raw_temperature()),
        ds["rawTemperature"].as_i64().expect("a register")
    );
    assert_eq!(
        i64::from(decoded.temperature_micro_celsius()),
        ds["microCelsius"].as_i64().expect("a temperature")
    );
    assert_eq!(
        u64::from(decoded.resolution().bits()),
        ds["resolutionBits"].as_u64().expect("a resolution")
    );
    assert_eq!(
        u64::from(ds18b20::crc8(&unhex(&ds["crcData"]))),
        ds["crc"].as_u64().expect("a checksum")
    );

    let corrupt: [u8; 9] = unhex(&ds["corruptScratchpad"]).try_into().expect("9 bytes");
    assert!(
        ds18b20::Scratchpad::parse(&corrupt).is_err(),
        "a read corrupted on the bus must not be trusted"
    );

    for entry in ds["resolutions"].as_array().expect("an array") {
        let bits = entry["bits"].as_u64().expect("a resolution") as u8;
        let resolution = match bits {
            9 => ds18b20::Resolution::Bits9,
            10 => ds18b20::Resolution::Bits10,
            11 => ds18b20::Resolution::Bits11,
            _ => ds18b20::Resolution::Bits12,
        };
        assert_eq!(
            u64::from(resolution.config_byte()),
            entry["configByte"].as_u64().expect("a config byte")
        );
        assert_eq!(
            u64::from(resolution.step_micro_celsius()),
            entry["stepMicroCelsius"].as_u64().expect("a step")
        );
        assert_eq!(
            ds18b20::Resolution::from_config_byte(resolution.config_byte()).bits(),
            bits,
            "the resolution round-trips through its config byte"
        );
    }

    let ina = &case["ina219"];
    let lsb = ina["currentLsbMicroamps"].as_u64().expect("a resolution") as u32;
    assert_eq!(
        u64::from(ina219::calibration(
            lsb,
            ina["shuntMilliohms"].as_u64().expect("a shunt") as u32
        )),
        ina["calibration"].as_u64().expect("a calibration")
    );
    assert_eq!(
        u64::from(ina219::minimum_current_lsb_microamps(
            ina["maxExpectedMicroamps"].as_u64().expect("a current") as u32
        )),
        ina["minimumCurrentLsbMicroamps"]
            .as_u64()
            .expect("a resolution")
    );
    assert_eq!(
        i64::from(ina219::current_microamps(
            ina["rawCurrent"].as_i64().expect("a register") as i16,
            lsb
        )),
        ina["currentMicroamps"].as_i64().expect("a current")
    );
    assert_eq!(
        u64::from(ina219::power_microwatts(
            ina["rawPower"].as_u64().expect("a register") as u16,
            lsb
        )),
        ina["powerMicrowatts"].as_u64().expect("a power")
    );

    let ads = &case["ads1115"];
    let reset = ads1115::Config::from_bits(ads["configReset"].as_u64().expect("a register") as u16);
    let want = &ads["resetConfig"];
    assert_eq!(
        u64::from(reset.pga.code()),
        want["pga"].as_u64().expect("a gain")
    );
    assert_eq!(
        u64::from(reset.data_rate.code()),
        want["dataRate"].as_u64().expect("a rate")
    );
    assert_eq!(
        u64::from(reset.bits()),
        ads["configReset"].as_u64().expect("a register"),
        "the configuration round-trips through its register"
    );

    for entry in ads["gains"].as_array().expect("an array") {
        let pga = ads1115::Pga::from_code(entry["pga"].as_u64().expect("a gain") as u8);
        assert_eq!(
            u64::from(pga.full_scale_microvolts()),
            entry["fullScaleMicrovolts"].as_u64().expect("a full scale")
        );
        assert_eq!(
            ads1115::to_nanovolts(pga, 32_767),
            entry["nanovoltsAtFullScale"].as_i64().expect("a voltage")
        );
    }
    for entry in ads["rates"].as_array().expect("an array") {
        let rate = ads1115::DataRate::from_code(entry["dataRate"].as_u64().expect("a rate") as u8);
        assert_eq!(
            u64::from(rate.samples_per_second()),
            entry["samplesPerSecond"].as_u64().expect("a rate")
        );
    }
}

#[test]
fn actuator_vectors_match() {
    let vectors = vectors();
    let case = &vectors["actuators"];

    let pca = &case["pca9685"];
    for entry in pca["channelRegisters"].as_array().expect("an array") {
        assert_eq!(
            u64::from(pca9685::channel_register(
                entry["channel"].as_u64().expect("a channel") as u8
            )),
            entry["register"].as_u64().expect("a register")
        );
    }
    let prescale = pca9685::prescale_for_frequency(
        pca["updateRateHz"].as_u64().expect("a rate") as u32,
        pca["internalOscHz"].as_u64().expect("an oscillator") as u32,
    );
    assert_eq!(
        u64::from(prescale),
        pca["prescale"].as_u64().expect("a prescale")
    );

    let pwm = &case["pwm"];
    assert_eq!(
        hex(&pca9685::Pwm::duty(pwm["duty"]["off"].as_u64().expect("a count") as u16).bytes()),
        pwm["duty"]["bytes"].as_str().expect("the bytes")
    );
    let servo = &pwm["servoCentre"];
    assert_eq!(
        hex(&pca9685::Pwm::servo(
            servo["pulseMicros"].as_u64().expect("a pulse") as u32,
            servo["updateRateHz"].as_u64().expect("a rate") as u32,
        )
        .bytes()),
        servo["bytes"].as_str().expect("the bytes")
    );
    assert_eq!(
        hex(&pca9685::Pwm::full_on().bytes()),
        pwm["fullOn"].as_str().expect("the bytes")
    );
    assert_eq!(
        hex(&pca9685::Pwm::full_off().bytes()),
        pwm["fullOff"].as_str().expect("the bytes"),
        "fully off is its own encoding, not a zero duty"
    );

    let motor = &case["stepper"];
    let mut sequencer = stepper::Sequencer::new(stepper::Drive::HalfStep);
    let mut cycle = vec![u64::from(sequencer.coils())];
    for _ in 0..stepper::Drive::HalfStep.step_count() {
        cycle.push(u64::from(sequencer.step(stepper::Direction::Forward)));
    }
    let want: Vec<u64> = motor["forwardCycle"]
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| entry.as_u64().expect("a coil pattern"))
        .collect();
    assert_eq!(
        cycle, want,
        "one electrical cycle returns to its first pattern"
    );
    assert_eq!(
        i64::from(stepper::steps_for_degrees(
            float(&motor["degrees"]),
            motor["stepsPerRevolution"].as_u64().expect("a motor") as u32
        )),
        motor["stepsForDegrees"].as_i64().expect("a step count")
    );
}

#[test]
fn windowed_helper_vectors_match() {
    let vectors = vectors();
    let case = &vectors["windows"];
    let tolerance = float(&vectors["tolerance"]);

    let readings = floats(&case["window"]["readings"]);
    let mut window: Window<32> = Window::new();
    for (reading, want) in readings
        .iter()
        .zip(case["window"]["states"].as_array().expect("an array"))
    {
        window.push(*reading);
        assert_eq!(window.len() as u64, want["len"].as_u64().expect("a count"));
        assert!((window.mean().expect("a mean") - float(&want["mean"])).abs() <= tolerance);
        assert!((window.min().expect("a minimum") - float(&want["min"])).abs() <= tolerance);
        assert!((window.max().expect("a maximum") - float(&want["max"])).abs() <= tolerance);
    }

    let mut median: Median<32> = Median::new();
    for (reading, want) in floats(&case["median"]["readings"])
        .iter()
        .zip(floats(&case["median"]["outputs"]).iter())
    {
        assert!((median.update(*reading) - want).abs() <= tolerance);
    }

    let mut trend: Trend<32> = Trend::new();
    for (reading, want) in floats(&case["trend"]["readings"])
        .iter()
        .zip(case["trend"]["slopes"].as_array().expect("an array"))
    {
        trend.push(*reading);
        match (trend.slope(), want.as_f64()) {
            (Some(slope), Some(expected)) => assert!((slope - expected as f32).abs() <= 1e-4),
            (None, None) => {}
            (got, expected) => panic!("slope disagreed: {got:?} against {expected:?}"),
        }
    }

    let mut anomaly: Anomaly<32> = Anomaly::new(float(&case["anomaly"]["sigmas"]));
    for (reading, want) in floats(&case["anomaly"]["readings"])
        .iter()
        .zip(case["anomaly"]["flags"].as_array().expect("an array"))
    {
        assert_eq!(
            anomaly.check(*reading),
            want.as_bool().expect("a flag"),
            "the detector flags the reading that stands out"
        );
    }
}

/// Checks one plan against the vector describing it.
///
/// # Arguments
///
/// * `plan` - the plan to check.
/// * `want` - the vector it must agree with.
fn assert_plan(plan: &ChannelPlan<'_>, want: &serde_json::Value) {
    let name = want["name"].as_str().expect("a name");
    assert_eq!(plan.name, name);

    let uplink_count = want["uplinkDataRateCount"].as_u64().expect("a count") as usize;
    assert_eq!(
        plan.uplink_data_rates.len(),
        uplink_count,
        "uplink rates of {name}"
    );
    assert_eq!(
        plan.downlink_data_rates.len(),
        want["downlinkDataRateCount"].as_u64().expect("a count") as usize,
        "downlink rates of {name}"
    );
    assert_eq!(
        u64::from(plan.default_channel_count()),
        want["defaultChannelCount"].as_u64().expect("a count"),
        "default channels of {name}"
    );
    assert_eq!(
        u64::from(plan.max_rx1_data_rate_offset),
        want["maxRx1DataRateOffset"].as_u64().expect("an offset"),
        "RX1 offsets of {name}"
    );
    assert_eq!(
        plan.has_dwell_time_limit,
        want["hasDwellTimeLimit"].as_bool().expect("a flag"),
        "dwell limit of {name}"
    );

    let (rx2_frequency, rx2_data_rate) = plan.rx2();
    assert_eq!(
        u64::from(rx2_frequency),
        want["rx2"]["frequencyHz"].as_u64().expect("a frequency"),
        "RX2 frequency of {name}"
    );
    assert_eq!(
        u64::from(rx2_data_rate),
        want["rx2"]["dataRate"].as_u64().expect("a data rate"),
        "RX2 data rate of {name}"
    );

    let fastest = (uplink_count - 1) as u8;
    assert_data_rate(plan.uplink_data_rate(0), &want["slowestUplink"], name);
    assert_data_rate(plan.uplink_data_rate(fastest), &want["fastestUplink"], name);
    assert_data_rate(plan.downlink_data_rate(0), &want["slowestDownlink"], name);

    assert_payload(
        plan.max_payload(0, true),
        &want["payloadAtSlowest"]["repeater"],
        name,
    );
    assert_payload(
        plan.max_payload(0, false),
        &want["payloadAtSlowest"]["direct"],
        name,
    );
    assert_payload(
        plan.max_payload_dwell_limited(0),
        &want["dwellLimitedAtSlowest"],
        name,
    );

    let probe = want["probeFrequencyHz"].as_u64().expect("a frequency") as u32;
    assert_eq!(
        plan.duty_cycle_permille(probe).map(u64::from),
        want["dutyCyclePermilleAtProbe"].as_u64(),
        "duty cycle of {name}"
    );
    assert_eq!(
        i64::from(plan.max_eirp_dbm(probe)),
        want["maxEirpDbmAtProbe"].as_i64().expect("a ceiling"),
        "EIRP ceiling of {name}"
    );

    let row = want["rx1RowForSlowest"].as_array().expect("an array");
    for (offset, entry) in row.iter().enumerate() {
        assert_eq!(
            plan.rx1_data_rate(0, offset as u8).map(u64::from),
            entry.as_u64(),
            "RX1 offset {offset} of {name}"
        );
    }

    assert_eq!(
        plan.next_backoff_data_rate(fastest).map(u64::from),
        want["backoffFromFastest"].as_u64(),
        "back-off from the fastest rate of {name}"
    );
    assert_eq!(
        plan.next_backoff_data_rate(0).map(u64::from),
        want["backoffFromSlowest"].as_u64(),
        "back-off from the slowest rate of {name}"
    );

    for (channel, entry) in want["channelFrequencies"]
        .as_array()
        .expect("an array")
        .iter()
        .enumerate()
    {
        assert_eq!(
            plan.channel_frequency_hz(channel as u16).map(u64::from),
            entry.as_u64(),
            "channel {channel} of {name}"
        );
    }

    let bands = want["subBands"].as_array().expect("an array");
    assert_eq!(plan.sub_bands.len(), bands.len(), "sub-bands of {name}");
    for (band, entry) in plan.sub_bands.iter().zip(bands) {
        assert_eq!(
            u64::from(band.start_hz),
            entry["startHz"].as_u64().expect("a start")
        );
        assert_eq!(
            u64::from(band.end_hz),
            entry["endHz"].as_u64().expect("an end")
        );
        assert_eq!(
            u64::from(band.duty_cycle_permille),
            entry["dutyCyclePermille"].as_u64().expect("a duty cycle")
        );
        assert_eq!(
            i64::from(band.max_eirp_dbm),
            entry["maxEirpDbm"].as_i64().expect("a ceiling")
        );
    }
}

/// Checks a data rate against the vector describing it.
fn assert_data_rate(rate: Option<DataRate>, want: &serde_json::Value, region: &str) {
    let kind = want["kind"].as_str().expect("a kind");
    let Some(rate) = rate else {
        assert_eq!(kind, "reserved", "a reserved rate in {region}");
        return;
    };
    assert_eq!(
        u64::from(rate.bitrate_bps),
        want["bitrateBps"].as_u64().expect("a bitrate"),
        "bitrate in {region}"
    );
    match rate.modulation {
        Modulation::LoRa {
            spreading_factor,
            bandwidth_hz,
        } => {
            assert_eq!(kind, "lora", "modulation in {region}");
            assert_eq!(
                u64::from(spreading_factor),
                want["spreadingFactor"]
                    .as_u64()
                    .expect("a spreading factor")
            );
            assert_eq!(
                u64::from(bandwidth_hz),
                want["bandwidthHz"].as_u64().expect("a bandwidth")
            );
        }
        Modulation::Fsk { .. } => assert_eq!(kind, "fsk", "modulation in {region}"),
        Modulation::LrFhss {
            coding_rate_numerator,
            coding_rate_denominator,
            bandwidth_hz,
        } => {
            assert_eq!(kind, "lr_fhss", "modulation in {region}");
            assert_eq!(
                u64::from(coding_rate_numerator),
                want["codingRateNumerator"].as_u64().expect("a numerator")
            );
            assert_eq!(
                u64::from(coding_rate_denominator),
                want["codingRateDenominator"]
                    .as_u64()
                    .expect("a denominator")
            );
            assert_eq!(
                u64::from(bandwidth_hz),
                want["bandwidthHz"].as_u64().expect("a bandwidth")
            );
        }
    }
}

/// Checks a payload limit against the vector describing it.
fn assert_payload(payload: Option<MaxPayload>, want: &serde_json::Value, region: &str) {
    match payload {
        Some(payload) => {
            assert_eq!(
                u64::from(payload.mac_payload),
                want["macPayload"].as_u64().expect("a MAC payload"),
                "MAC payload in {region}"
            );
            assert_eq!(
                u64::from(payload.application),
                want["application"]
                    .as_u64()
                    .expect("an application payload"),
                "application payload in {region}"
            );
        }
        None => assert!(want.is_null(), "an absent payload limit in {region}"),
    }
}

#[test]
fn lora_region_vectors_match() {
    let vectors = vectors();
    let case = &vectors["loraRegions"];

    let published = case["published"].as_array().expect("an array");
    assert_eq!(
        published.len(),
        Region::all().len(),
        "every published region is described"
    );
    for (region, want) in Region::all().iter().zip(published) {
        assert_eq!(region.code(), want["code"].as_str().expect("a code"));
        assert_plan(region.plan(), want);
    }

    // The same questions, answered by a plan assembled at runtime rather than
    // published: a private deployment on licensed spectrum.
    let custom = ChannelPlanBuilder::new("private-915")
        .uplink_data_rate(Some(DataRate::lora(12, 125_000, 250)))
        .uplink_data_rate(Some(DataRate::lora(7, 125_000, 5_470)))
        .max_payload(PayloadTable::UplinkRepeater, Some(MaxPayload::new(59, 51)))
        .max_payload(
            PayloadTable::UplinkRepeater,
            Some(MaxPayload::new(230, 222)),
        )
        .max_payload(PayloadTable::UplinkDirect, Some(MaxPayload::new(59, 51)))
        .max_payload(PayloadTable::UplinkDirect, Some(MaxPayload::new(230, 222)))
        .default_channel(ChannelBlock::new(915_000_000, 500_000, 4, 0, 1))
        .sub_band(SubBand::new(915_000_000, 917_000_000, 1000, 30))
        .power(30, 2, 7)
        .rx(915_000_000, 0, 0)
        .rx1_row(&[0])
        .rx1_row(&[1])
        .build()
        .expect("a consistent private plan");
    custom.with_plan(|plan| assert_plan(plan, &case["custom"]));
}

#[test]
fn lora_vectors_match() {
    let vectors = vectors();
    let case = &vectors["lora"];

    for described in case["links"].as_array().expect("an array") {
        let link = link_of(described);
        assert_eq!(
            link.symbol_time_us(),
            described["symbolTimeUs"].as_u64().expect("a symbol time"),
            "symbol time for {}",
            described["name"]
        );

        for airtime in described["airtimes"].as_array().expect("an array") {
            let payload_len = airtime["payloadLen"].as_u64().expect("a length") as usize;
            assert_eq!(
                link.airtime_us(payload_len),
                airtime["airtimeUs"].as_u64().expect("an airtime"),
                "airtime of {payload_len} bytes on {}",
                described["name"]
            );
        }

        for budget in described["budgets"].as_array().expect("an array") {
            let payload_len = budget["payloadLen"].as_u64().expect("a length") as usize;
            let permille = budget["permille"].as_u64().expect("a limit") as u32;
            assert_eq!(
                link.min_off_time_us(payload_len, permille),
                budget["offTimeUs"].as_u64().expect("an off time"),
                "off time at {permille} permille on {}",
                described["name"]
            );
        }
    }

    for clamp in case["clamped"].as_array().expect("an array") {
        let asked = clamp["asked"].as_u64().expect("a factor") as u8;
        assert_eq!(
            LinkSettings::new(asked, 125_000).spreading_factor(),
            clamp["used"].as_u64().expect("a factor") as u8,
            "a spreading factor outside 5 to 12 is clamped"
        );
    }

    // Rust reports a forbidden duty cycle as a saturated off time; the bindings
    // each surface it as their own "never" value.
    let forbidden = &case["forbidden"];
    let link = link_of(named(
        case,
        forbidden["link"].as_str().expect("a link name"),
    ));
    assert_eq!(
        link.min_off_time_us(
            forbidden["payloadLen"].as_u64().expect("a length") as usize,
            forbidden["permille"].as_u64().expect("a limit") as u32,
        ),
        u64::MAX,
        "a zero duty cycle forbids transmitting"
    );
}

/// Rebuilds the link a vector describes.
fn link_of(described: &Value) -> LinkSettings {
    let mut link = LinkSettings::new(
        described["spreadingFactor"].as_u64().expect("a factor") as u8,
        described["bandwidthHz"].as_u64().expect("a bandwidth") as u32,
    )
    .with_coding_rate(described["codingRateDenominator"].as_u64().expect("a rate") as u8)
    .with_preamble(described["preambleSymbols"].as_u64().expect("a preamble") as u16);
    if !described["explicitHeader"].as_bool().expect("a flag") {
        link = link.implicit_header();
    }
    if !described["crc"].as_bool().expect("a flag") {
        link = link.without_crc();
    }
    link
}

/// Finds the named link among the vectors.
fn named<'a>(case: &'a Value, name: &str) -> &'a Value {
    case["links"]
        .as_array()
        .expect("an array")
        .iter()
        .find(|link| link["name"] == name)
        .unwrap_or_else(|| panic!("no link named {name}"))
}

#[test]
fn mesh_vectors_match() {
    let vectors = vectors();
    let case = &vectors["mesh"];

    assert_eq!(
        MeshFrame::MAX_LEN,
        case["maxFrame"].as_u64().expect("a length") as usize
    );
    assert_eq!(
        MeshFrame::MAX_PAYLOAD,
        case["maxPayload"].as_u64().expect("a length") as usize
    );
    assert_eq!(
        pamoja_mesh::BROADCAST,
        case["broadcastAddress"].as_u64().expect("an address") as u32
    );

    let unicast = &case["unicast"];
    let built = MeshFrame::new(
        unicast["src"].as_u64().expect("an address") as u32,
        unicast["dst"].as_u64().expect("an address") as u32,
        unicast["id"].as_u64().expect("an id") as u16,
        &unhex(&unicast["payload"]),
    )
    .expect("build the frame")
    .with_hop_limit(unicast["hopLimit"].as_u64().expect("a hop limit") as u8);
    assert_eq!(built.as_bytes().to_vec(), unhex(&unicast["bytes"]));

    let broadcast = &case["broadcast"];
    let built = MeshFrame::broadcast(
        broadcast["src"].as_u64().expect("an address") as u32,
        broadcast["id"].as_u64().expect("an id") as u16,
        &unhex(&broadcast["payload"]),
    )
    .expect("build the frame");
    assert_eq!(built.as_bytes().to_vec(), unhex(&broadcast["bytes"]));
    assert!(built.is_broadcast());

    let parsed = MeshFrame::parse(&unhex(&broadcast["bytes"])).expect("parse the frame");
    assert_eq!(parsed.payload().to_vec(), unhex(&broadcast["payload"]));

    let relayed = parsed.relayed().expect("a fresh frame has hops to spend");
    assert_eq!(
        relayed.as_bytes().to_vec(),
        unhex(&case["relayed"]["bytes"])
    );
    assert_eq!(
        u64::from(relayed.hop_limit()),
        case["relayed"]["hopLimit"].as_u64().expect("a hop limit")
    );

    let exhausted = MeshFrame::parse(&unhex(&case["exhausted"])).expect("parse the frame");
    assert!(
        exhausted.relayed().is_none(),
        "a frame with no hops left must not be relayed"
    );

    assert!(
        MeshFrame::parse(&unhex(&case["corrupt"])).is_err(),
        "a frame the air mangled must be refused"
    );

    let crc = &case["crc"];
    assert_eq!(
        u64::from(mesh_crc16(&unhex(&crc["check"]))),
        crc["checkValue"].as_u64().expect("a checksum"),
        "the published CRC-16/CCITT-FALSE check value"
    );
    assert_eq!(
        u64::from(mesh_crc16(&unhex(&crc["data"]))),
        crc["value"].as_u64().expect("a checksum")
    );

    let seen_case = &case["seen"];
    let mut seen = DynamicSeenCache::new(case["seenCapacity"].as_u64().expect("a size") as usize);
    let answers: Vec<bool> = seen_case["keys"]
        .as_array()
        .expect("an array")
        .iter()
        .map(|key| {
            let key = key.as_array().expect("a pair");
            seen.record((
                key[0].as_u64().expect("an address") as u32,
                key[1].as_u64().expect("an id") as u16,
            ))
        })
        .collect();
    let want: Vec<bool> = seen_case["new"]
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| entry.as_bool().expect("a flag"))
        .collect();
    assert_eq!(answers, want, "each packet is new exactly once");

    let sized = &case["sizedSeen"];
    let mut small = DynamicSeenCache::new(sized["capacity"].as_u64().expect("a capacity") as usize);
    for key in sized["keys"].as_array().expect("an array") {
        let key = key.as_array().expect("a pair");
        small.record((
            key[0].as_u64().expect("an address") as u32,
            key[1].as_u64().expect("an id") as u16,
        ));
    }
    let evicted = sized["evicted"].as_array().expect("a pair");
    assert!(
        !small.contains((
            evicted[0].as_u64().expect("an address") as u32,
            evicted[1].as_u64().expect("an id") as u16,
        )),
        "a cache sized by the caller evicts at that size"
    );
}

#[test]
fn routing_vectors_match() {
    let vectors = vectors();
    let case = &vectors["routing"];

    let mut router = DynamicRouter::new(
        case["address"].as_u64().expect("an address") as u32,
        case["capacity"].as_u64().expect("a capacity") as usize,
    );

    for observation in case["observations"].as_array().expect("an array") {
        let changed = router.observe(
            observation["origin"].as_u64().expect("an address") as u32,
            observation["via"].as_u64().expect("an address") as u32,
            observation["cost"].as_u64().expect("a cost") as u16,
        );
        assert_eq!(
            changed,
            observation["changed"].as_bool().expect("a flag"),
            "observing {} via {} changes the table",
            observation["origin"],
            observation["via"]
        );
    }

    assert_eq!(
        router.len(),
        case["learned"].as_u64().expect("a count") as usize
    );

    let route = &case["route"];
    let learned = router
        .route(route["dst"].as_u64().expect("an address") as u32)
        .expect("the route was learned");
    assert_eq!(
        u64::from(learned.next_hop()),
        route["nextHop"].as_u64().expect("an address")
    );
    assert_eq!(
        u64::from(learned.cost()),
        route["cost"].as_u64().expect("a cost")
    );

    for want in case["decisions"].as_array().expect("an array") {
        assert_decision(&router, want);
    }

    let forgotten = &case["afterForgetting"];
    router.forget(forgotten["dst"].as_u64().expect("an address") as u32);
    assert_decision(&router, &forgotten["decision"]);
    assert_eq!(
        router.len(),
        forgotten["learned"].as_u64().expect("a count") as usize
    );

    let sized = &case["sized"];
    let capacity = sized["capacity"].as_u64().expect("a capacity") as usize;
    let mut small = DynamicRouter::new(0x01, capacity);
    for node in 0..sized["offered"].as_u64().expect("a count") as u32 {
        small.observe(node + 0x100, 0x05, 4);
    }
    assert_eq!(small.capacity(), capacity);
    assert_eq!(
        small.len(),
        sized["learned"].as_u64().expect("a count") as usize,
        "a table sized by the caller holds exactly what it was asked for"
    );
}

/// Checks one routing decision against the vector that describes it.
fn assert_decision(router: &DynamicRouter, want: &Value) {
    let dst = want["dst"].as_u64().expect("an address") as u32;
    let action = want["action"].as_str().expect("an action");
    match router.forward(dst) {
        Forward::Deliver => assert_eq!(action, "Deliver", "packet for {dst:#x}"),
        Forward::Relay(next_hop) => {
            assert_eq!(action, "Relay", "packet for {dst:#x}");
            assert_eq!(
                u64::from(next_hop),
                want["nextHop"].as_u64().expect("an address")
            );
        }
        Forward::Flood => assert_eq!(action, "Flood", "packet for {dst:#x}"),
    }
}

#[test]
fn lorawan_vectors_match() {
    let vectors = vectors();
    let case = &vectors["lorawan"];

    let session = Session::new(
        case["devAddr"].as_u64().expect("an address") as u32,
        unhex(&case["nwkSKey"]).try_into().expect("a 16-byte key"),
        unhex(&case["appSKey"]).try_into().expect("a 16-byte key"),
    );

    let uplink_case = &case["uplink"];
    let payload = unhex(&uplink_case["payload"]);
    let uplink = session
        .encode_uplink(
            &Uplink::new(
                uplink_case["fcnt"].as_u64().expect("a counter") as u32,
                uplink_case["fport"].as_u64().expect("a port") as u8,
                &payload,
            )
            .confirmed()
            .with_adr(),
        )
        .expect("encode the uplink");
    assert_eq!(uplink.as_bytes().to_vec(), unhex(&uplink_case["frame"]));

    let fcnt = uplink_case["fcnt"].as_u64().expect("a counter") as u32;
    let rx = session
        .decode(uplink.as_bytes(), fcnt)
        .expect("the frame verifies");
    assert_eq!(rx.direction(), LorawanDirection::Uplink);
    assert!(rx.confirmed());
    assert!(rx.adr());
    assert!(!rx.ack());
    assert_eq!(rx.payload().to_vec(), payload);

    let downlink_case = &case["downlink"];
    let payload = unhex(&downlink_case["payload"]);
    let fopts = unhex(&downlink_case["fopts"]);
    let downlink = session
        .encode_downlink(
            &Downlink::new(
                downlink_case["fcnt"].as_u64().expect("a counter") as u32,
                downlink_case["fport"].as_u64().expect("a port") as u8,
                &payload,
            )
            .with_ack()
            .with_fpending()
            .with_fopts(&fopts),
        )
        .expect("encode the downlink");
    assert_eq!(downlink.as_bytes().to_vec(), unhex(&downlink_case["frame"]));

    let rx = session
        .decode(
            downlink.as_bytes(),
            downlink_case["fcnt"].as_u64().expect("a counter") as u32,
        )
        .expect("the frame verifies");
    assert_eq!(rx.direction(), LorawanDirection::Downlink);
    assert!(rx.fpending());
    assert_eq!(rx.fopts().to_vec(), fopts);

    assert!(
        session.decode(&unhex(&case["forgedUplink"]), fcnt).is_err(),
        "a frame altered after signing must not verify"
    );
    assert!(
        session
            .decode(
                uplink.as_bytes(),
                case["wrongCounter"].as_u64().expect("a counter") as u32
            )
            .is_err(),
        "a frame out of its place in the counter stream must not verify"
    );

    let join = &case["join"];
    let device = Device::new(
        unhex(&join["devEui"]).try_into().expect("an 8-byte EUI"),
        unhex(&join["appEui"]).try_into().expect("an 8-byte EUI"),
        unhex(&join["appKey"]).try_into().expect("a 16-byte key"),
    );
    let dev_nonce = join["devNonce"].as_u64().expect("a nonce") as u16;
    assert_eq!(
        device.join_request(dev_nonce).as_bytes().to_vec(),
        unhex(&join["request"])
    );
    assert!(
        device
            .accept_join(&unhex(&join["forgedAccept"]), dev_nonce)
            .is_err(),
        "a join the network never signed must not activate a session"
    );
}

#[test]
fn header_vectors_match() {
    let vectors = vectors();
    let case = &vectors["header"];

    for want in case["frames"].as_array().expect("an array") {
        let bytes = unhex(&want["frame"]);
        let header = FrameHeader::parse(&bytes).expect("the frame parses");

        let name = match header.message_type() {
            MessageType::JoinRequest => "JoinRequest",
            MessageType::JoinAccept => "JoinAccept",
            MessageType::UnconfirmedUp => "UnconfirmedUp",
            MessageType::ConfirmedUp => "ConfirmedUp",
            MessageType::UnconfirmedDown => "UnconfirmedDown",
            MessageType::ConfirmedDown => "ConfirmedDown",
        };
        assert_eq!(name, want["messageType"].as_str().expect("a name"));
        assert_eq!(
            header.message_type().is_data(),
            want["isData"].as_bool().expect("a flag")
        );
        assert_eq!(
            header.dev_addr().map(u64::from),
            want["devAddr"].as_u64(),
            "the address a receiver routes by"
        );
        assert_eq!(header.fcnt().map(u64::from), want["fcnt"].as_u64());
        assert_eq!(header.fport().map(u64::from), want["fport"].as_u64());
        assert_eq!(
            header.confirmed(),
            want["confirmed"].as_bool().expect("a flag")
        );
        assert_eq!(header.adr(), want["adr"].as_bool().expect("a flag"));
        assert_eq!(header.ack(), want["ack"].as_bool().expect("a flag"));
        assert_eq!(
            header.fpending(),
            want["fpending"].as_bool().expect("a flag")
        );
        assert_eq!(
            header.fopts_len() as u64,
            want["foptsLen"].as_u64().expect("a length")
        );
        assert_eq!(
            header.payload_len() as u64,
            want["payloadLen"].as_u64().expect("a length")
        );
    }

    assert!(
        FrameHeader::parse(&unhex(&case["unsupported"])).is_err(),
        "a message type this crate does not read must be refused"
    );
    assert!(
        FrameHeader::parse(&unhex(&case["truncated"])).is_err(),
        "a frame too short to hold a header must be refused"
    );
}

#[test]
fn network_vectors_match() {
    let vectors = vectors();
    let case = &vectors["network"];
    let app_key: [u8; 16] = unhex(&case["appKey"]).try_into().expect("a 16-byte key");
    let dev_nonce = case["devNonce"].as_u64().expect("a nonce") as u16;

    let want = &case["joinRequest"];
    let request =
        JoinRequest::parse(&unhex(&want["frame"]), &app_key).expect("the request verifies");
    assert_eq!(hex(&request.dev_eui()), want["devEui"].as_str().unwrap());
    assert_eq!(hex(&request.app_eui()), want["appEui"].as_str().unwrap());
    assert_eq!(
        u64::from(request.dev_nonce()),
        want["devNonce"].as_u64().expect("a nonce")
    );

    assert!(
        JoinRequest::parse(&unhex(&case["forgedRequest"]), &app_key).is_err(),
        "a request signed with another root key must not be trusted"
    );

    assert_grant(&case["grant"], &app_key, dev_nonce);

    // The captured join: a third party's numbers, so agreement here is not just
    // this implementation agreeing with itself.
    let published = &case["published"];
    let published_key: [u8; 16] = unhex(&published["appKey"]).try_into().expect("a key");
    let published_nonce = published["devNonce"].as_u64().expect("a nonce") as u16;
    assert_grant(published, &published_key, published_nonce);

    // The device side reaches the same session from the captured bytes.
    let accepted = Device::new([0; 8], [0; 8], published_key)
        .accept_join(&unhex(&published["accept"]), published_nonce)
        .expect("the captured accept verifies");
    assert_eq!(
        u64::from(accepted.dev_addr()),
        published["devAddr"].as_u64().expect("an address")
    );
    let probe = &published["probe"];
    assert_eq!(
        hex(accepted
            .session()
            .encode_uplink(&Uplink::new(
                probe["fcnt"].as_u64().expect("a counter") as u32,
                probe["fport"].as_u64().expect("a port") as u8,
                &unhex(&probe["payload"]),
            ))
            .expect("encode with the activated session")
            .as_bytes()),
        probe["frame"].as_str().expect("a frame"),
        "the session the device derived matches the published keys"
    );
}

/// Checks that a grant builds its accept and derives the session both sides share.
fn assert_grant(case: &Value, app_key: &[u8; 16], dev_nonce: u16) {
    let mut grant = JoinGrant::new(
        case["appNonce"].as_u64().expect("a nonce") as u32,
        case["netId"].as_u64().expect("a network") as u32,
        case["devAddr"].as_u64().expect("an address") as u32,
    )
    .with_dl_settings(case["dlSettings"].as_u64().expect("settings") as u8)
    .with_rx_delay(case["rxDelay"].as_u64().expect("a delay") as u8);
    if let Some(cflist) = case.get("cflist").and_then(Value::as_str) {
        grant = grant.with_cflist(
            unhex(&Value::String(cflist.to_owned()))
                .try_into()
                .expect("a 16-byte channel list"),
        );
    }

    assert_eq!(
        hex(grant.accept(app_key, dev_nonce).as_bytes()),
        case["accept"].as_str().expect("an accept"),
        "the signed join-accept matches byte for byte"
    );

    // Neither side sent a key, so the proof they agree is that one reads what the
    // other wrote.
    let probe = &case["probe"];
    assert_eq!(
        hex(grant
            .session(app_key, dev_nonce)
            .encode_uplink(&Uplink::new(
                probe["fcnt"].as_u64().expect("a counter") as u32,
                probe["fport"].as_u64().expect("a port") as u8,
                &unhex(&probe["payload"]),
            ))
            .expect("encode with the derived session")
            .as_bytes()),
        probe["frame"].as_str().expect("a frame"),
        "the session this network derived is the one the device holds"
    );
}

#[test]
fn audit_vectors_match() {
    let vectors = vectors();
    let vector = &vectors["audit"];
    let seed = <[u8; 32]>::try_from(unhex(&vector["seed"]).as_slice()).expect("the seed");
    let keeper = DeviceIdentity::from_seed(&seed);
    assert_eq!(
        hex(&keeper.public().to_bytes()),
        vector["publicKey"].as_str().expect("the public key"),
        "the key a chain is checked against"
    );

    let mut log = AuditLog::new(keeper.clone());
    let mut entries = Vec::new();
    for want in vector["entries"].as_array().expect("the entries") {
        let payload = want["payload"].as_str().expect("the payload");
        let entry = log.append(payload.as_bytes());

        assert_eq!(entry.index(), want["index"].as_u64().expect("the index"));
        assert_eq!(
            hex(&entry.previous()),
            want["previous"].as_str().expect("the previous hash"),
            "each record carries the hash of the one before it"
        );
        assert_eq!(
            hex(&entry.digest()),
            want["digest"].as_str().expect("the digest")
        );
        assert_eq!(
            hex(&entry.signature().to_bytes()),
            want["signature"].as_str().expect("the signature")
        );
        assert_eq!(
            hex(&entry.to_bytes()),
            want["bytes"].as_str().expect("the encoded entry"),
            "a record encodes the same in every language"
        );
        entries.push(entry);
    }

    assert!(
        verify_chain(&keeper.public(), &entries).is_ok(),
        "an untouched chain verifies"
    );

    let tampered = Entry::from_bytes(&unhex(&vector["tampered"])).expect("a well-formed entry");
    let broken = vec![entries[0].clone(), entries[1].clone(), tampered];
    assert!(
        verify_chain(&keeper.public(), &broken).is_err(),
        "and an altered record breaks it"
    );

    let resumed_want = &vector["resumed"];
    let mut resumed = AuditLog::resume(keeper, &entries[2]);
    let after_reboot = resumed.append(
        resumed_want["payload"]
            .as_str()
            .expect("the payload")
            .as_bytes(),
    );
    assert_eq!(
        after_reboot.index(),
        resumed_want["index"].as_u64().expect("the index"),
        "a reboot leaves no gap in the chain"
    );
    assert_eq!(
        hex(&after_reboot.to_bytes()),
        resumed_want["bytes"].as_str().expect("the encoded entry")
    );
}

#[test]
fn session_vectors_match() {
    let vectors = vectors();
    let vector = &vectors["session"];

    let node_seed =
        <[u8; 32]>::try_from(unhex(&vector["nodeSeed"]).as_slice()).expect("the node seed");
    let gateway_seed =
        <[u8; 32]>::try_from(unhex(&vector["gatewaySeed"]).as_slice()).expect("the gateway seed");
    let node = AgreementKey::from_seed(&node_seed);
    let gateway = AgreementKey::from_seed(&gateway_seed);

    assert_eq!(
        hex(&node.public().to_bytes()),
        vector["nodePublicKey"].as_str().expect("the node key")
    );
    assert_eq!(
        hex(&gateway.public().to_bytes()),
        vector["gatewayPublicKey"]
            .as_str()
            .expect("the gateway key")
    );

    let salt = unhex(&vector["salt"]);
    let aad = vector["aad"]
        .as_str()
        .expect("the associated data")
        .as_bytes();
    let mut uplink = SecuredSession::establish(&node, &gateway.public(), &salt, Role::Initiator);
    let mut downlink = SecuredSession::establish(&gateway, &node.public(), &salt, Role::Responder);

    for want in vector["messages"].as_array().expect("the messages") {
        let plaintext = want["plaintext"].as_str().expect("the plaintext");
        let mut message = plaintext.as_bytes().to_vec();
        let header = uplink.seal(&mut message, aad);

        assert_eq!(
            header.counter,
            want["counter"].as_u64().expect("the counter")
        );
        assert_eq!(hex(&header.tag), want["tag"].as_str().expect("the tag"));
        assert_eq!(
            hex(&message),
            want["ciphertext"].as_str().expect("the ciphertext"),
            "the same key and counter produce the same bytes everywhere"
        );

        downlink
            .open(&header, &mut message, aad)
            .expect("the peer opens it");
        assert_eq!(message, plaintext.as_bytes(), "and recovers the reading");
    }

    // The first message again: the peer has already seen that counter.
    let first = &vector["messages"][0];
    let mut replayed = unhex(&first["ciphertext"]);
    let header = Sealed {
        counter: first["counter"].as_u64().expect("the counter"),
        tag: <[u8; 16]>::try_from(unhex(&first["tag"]).as_slice()).expect("the tag"),
    };
    assert_eq!(
        downlink.open(&header, &mut replayed, aad),
        Err(SessionError::Replayed),
        "a repeated counter is refused"
    );

    // A fresh peer, so the replay window is not what refuses it this time.
    let mut fresh = SecuredSession::establish(&gateway, &node.public(), &salt, Role::Responder);
    let wrong = vector["wrongAad"]
        .as_str()
        .expect("the wrong aad")
        .as_bytes();
    let mut message = unhex(&first["ciphertext"]);
    assert_eq!(
        fresh.open(&header, &mut message, wrong),
        Err(SessionError::Inauthentic),
        "and associated data that does not match fails authentication"
    );

    let hmac = &vector["hmac"];
    assert_eq!(
        hex(&pamoja_session::hmac_sha256(
            hmac["key"].as_str().expect("the key").as_bytes(),
            hmac["message"].as_str().expect("the message").as_bytes(),
        )),
        hmac["digest"].as_str().expect("the digest")
    );

    let hkdf = &vector["hkdf"];
    let mut derived = vec![0u8; hkdf["length"].as_u64().expect("the length") as usize];
    pamoja_session::hkdf_sha256(
        hkdf["salt"].as_str().expect("the salt").as_bytes(),
        hkdf["ikm"].as_str().expect("the ikm").as_bytes(),
        hkdf["info"].as_str().expect("the info").as_bytes(),
        &mut derived,
    );
    assert_eq!(hex(&derived), hkdf["output"].as_str().expect("the output"));
}

#[test]
fn update_vectors_match() {
    let vectors = vectors();
    let vector = &vectors["update"];

    let publisher = DeviceIdentity::from_seed(
        &<[u8; 32]>::try_from(unhex(&vector["publisherSeed"]).as_slice()).expect("the seed"),
    );
    assert_eq!(
        hex(&publisher.public().to_bytes()),
        vector["publisherPublicKey"].as_str().expect("the key")
    );

    let want = &vector["manifest"];
    let image = vec![
        vector["imageByte"].as_u64().expect("the image byte") as u8;
        vector["imageLen"].as_u64().expect("the image length") as usize
    ];
    let vendor = <[u8; 16]>::try_from(unhex(&vector["vendorId"]).as_slice()).expect("the vendor");
    let class = <[u8; 16]>::try_from(unhex(&vector["classId"]).as_slice()).expect("the class");
    let manifest = Manifest {
        structure_version: want["structureVersion"].as_u64().expect("the version") as u8,
        sequence: want["sequence"].as_u64().expect("the sequence"),
        vendor_id: vendor,
        class_id: class,
        format: pamoja_update::PayloadFormat::Raw,
        storage: want["storage"].as_u64().expect("the slot") as u8,
        digest: <[u8; 32]>::try_from(unhex(&want["digest"]).as_slice()).expect("the digest"),
        size: want["size"].as_u64().expect("the size") as u32,
        expires: want["expires"].as_u64().expect("the expiry"),
    };

    let mut body = [0u8; pamoja_update::MANIFEST_MAX];
    let body_len = manifest.encode(&mut body).expect("encode the manifest");
    assert_eq!(
        hex(&body[..body_len]),
        vector["body"].as_str().expect("the encoded body"),
        "a manifest encodes the same in every language"
    );

    let mut envelope = [0u8; pamoja_update::ENVELOPE_MAX];
    let envelope_len = manifest
        .sign(&publisher, &mut envelope)
        .expect("sign the manifest");
    assert_eq!(
        hex(&envelope[..envelope_len]),
        vector["envelope"].as_str().expect("the envelope")
    );

    let verified = Envelope::decode(&envelope[..envelope_len])
        .expect("a well-formed envelope")
        .verify(&publisher.public())
        .expect("the signature holds");
    assert_eq!(verified.digest, manifest.digest);

    let forged = unhex(&vector["forgedEnvelope"]);
    assert_eq!(
        Envelope::decode(&forged)
            .expect("a well-formed envelope")
            .verify(&publisher.public())
            .err(),
        Some(Refusal::Signature),
        "a release signed by another key is refused"
    );

    let delegation_want = &vector["delegation"];
    let anchor = DeviceIdentity::from_seed(
        &<[u8; 32]>::try_from(unhex(&vector["anchorSeed"]).as_slice()).expect("the seed"),
    );
    let delegation = Delegation {
        epoch: delegation_want["epoch"].as_u64().expect("the epoch"),
        release_key: <[u8; 32]>::try_from(unhex(&delegation_want["releaseKey"]).as_slice())
            .expect("the release key"),
        expires: delegation_want["expires"].as_u64().expect("the expiry"),
    };
    let mut statement = [0u8; pamoja_update::DELEGATION_MAX];
    let statement_len = delegation
        .sign(&anchor, &mut statement)
        .expect("sign the delegation");
    assert_eq!(
        hex(&statement[..statement_len]),
        delegation_want["envelope"].as_str().expect("the envelope")
    );

    let lifecycle = &vector["lifecycle"];
    let device = UpdateDevice {
        vendor_id: vendor,
        class_id: class,
        anchor: publisher.public(),
    };
    let mut updater = Updater::new(device, MemoryStore::new(2, 4096));
    updater
        .provision(0, 1)
        .expect("provision the running image");

    let chunk = lifecycle["chunk"].as_u64().expect("the chunk size") as usize;
    let opened = updater
        .begin(&envelope[..envelope_len])
        .expect("open the transfer")
        .manifest()
        .storage;
    assert_eq!(
        u64::from(opened),
        lifecycle["staged"].as_u64().expect("the slot"),
        "the release names the same slot everywhere"
    );
    for piece in image.chunks(chunk) {
        let mut staging = updater
            .resume_at(&envelope[..envelope_len], None)
            .expect("resume the transfer");
        staging.write(piece).expect("take the piece");
    }
    let staged = updater
        .resume_at(&envelope[..envelope_len], None)
        .expect("resume the transfer")
        .finish()
        .expect("settle the image");
    assert_eq!(
        u64::from(staged),
        lifecycle["staged"].as_u64().expect("the slot")
    );

    let boot = updater.on_boot().expect("decide what to run");
    assert_eq!(
        match boot {
            Boot::Confirmed(_) => "Confirmed",
            Boot::Trying(_) => "Trying",
            Boot::Reverted { .. } => "Reverted",
        },
        lifecycle["boot"].as_str().expect("the boot decision")
    );
    assert_eq!(
        u64::from(match boot {
            Boot::Confirmed(slot) | Boot::Trying(slot) => slot,
            Boot::Reverted { failed, .. } => failed,
        }),
        lifecycle["bootSlot"].as_u64().expect("the boot slot")
    );

    let confirmed = updater.confirm().expect("confirm the release");
    assert_eq!(
        u64::from(confirmed),
        lifecycle["confirmed"].as_u64().expect("the confirmed slot")
    );

    let record = updater.store().record(confirmed).expect("read the slot");
    assert_eq!(
        match record.state {
            SlotState::Empty => "Empty",
            SlotState::Receiving => "Receiving",
            SlotState::Staged => "Staged",
            SlotState::Pending => "Pending",
            SlotState::Confirmed => "Confirmed",
            SlotState::Failed => "Failed",
        },
        lifecycle["state"].as_str().expect("the slot state")
    );
    assert_eq!(
        u64::from(record.written),
        lifecycle["written"].as_u64().expect("the bytes written")
    );
}

#[test]
fn power_vectors_match() {
    let vectors = vectors();
    let vector = &vectors["power"];
    let want = &vector["plan"];

    let plan = PowerPlan::new(
        core::time::Duration::from_micros(want["activeUs"].as_u64().expect("the interval")),
        core::time::Duration::from_micros(want["saverUs"].as_u64().expect("the interval")),
        core::time::Duration::from_micros(want["criticalUs"].as_u64().expect("the interval")),
    );
    assert_eq!(plan.saver_below(), float(&want["saverBelow"]));
    assert_eq!(plan.critical_below(), float(&want["criticalBelow"]));

    let charges = floats(&vector["charges"]);
    let modes = vector["modes"].as_array().expect("the modes");
    let charging = vector["charging"].as_array().expect("the charging modes");
    let intervals = vector["intervalsUs"].as_array().expect("the intervals");

    for (at, &soc) in charges.iter().enumerate() {
        assert_eq!(
            power_mode_name(plan.mode(soc)),
            modes[at].as_str().expect("the mode"),
            "the mode at {soc}"
        );
        assert_eq!(
            power_mode_name(plan.mode_while_charging(soc, true)),
            charging[at].as_str().expect("the mode"),
            "the mode while charging at {soc}"
        );
        assert_eq!(
            plan.interval(soc).as_micros() as u64,
            intervals[at].as_u64().expect("the interval"),
            "the interval at {soc}"
        );
    }

    let duty_want = &vector["duty"];
    let duty = DutyCycle::from_fraction(
        core::time::Duration::from_micros(duty_want["periodUs"].as_u64().expect("the period")),
        float(&duty_want["fraction"]),
    );
    assert_eq!(
        duty.active().as_micros() as u64,
        duty_want["activeUs"].as_u64().expect("the awake time")
    );
    assert_eq!(
        duty.sleep().as_micros() as u64,
        duty_want["sleepUs"].as_u64().expect("the sleep time")
    );
}

/// Names a power mode the way the vectors record it.
fn power_mode_name(mode: PowerMode) -> &'static str {
    match mode {
        PowerMode::Active => "Active",
        PowerMode::Saver => "Saver",
        PowerMode::Critical => "Critical",
    }
}

#[test]
fn telemetry_vectors_match() {
    let vectors = vectors();
    let vector = &vectors["telemetry"];

    let costs = vector["costs"].as_array().expect("the link costs");
    let thresholds = vector["thresholds"].as_array().expect("the thresholds");
    for (at, cost) in costs.iter().enumerate() {
        assert_eq!(
            telemetry_level_name(link_cost(cost.as_str().expect("the cost")).threshold()),
            thresholds[at].as_str().expect("the threshold"),
            "the bar each link cost sets"
        );
    }

    let mut reporter = Reporter::new(TelemetryLevel::Trace);
    reporter.adapt_to(link_cost(
        vector["adaptedTo"].as_str().expect("the link cost"),
    ));

    let levels = vector["levels"].as_array().expect("the levels");
    let shipped = vector["shipped"].as_array().expect("the outcomes");
    for (at, level) in levels.iter().enumerate() {
        let level = telemetry_level(level.as_str().expect("the level"));
        assert_eq!(
            reporter.record(Event::new(level, "vector")).is_some(),
            shipped[at].as_bool().expect("the outcome"),
            "whether event {at} is worth its bytes"
        );
    }

    let want = &vector["snapshot"];
    let snapshot = reporter.snapshot();
    assert_eq!(
        u64::from(snapshot.by_level[TelemetryLevel::Trace as usize]),
        want["trace"].as_u64().expect("the count")
    );
    assert_eq!(
        u64::from(snapshot.by_level[TelemetryLevel::Debug as usize]),
        want["debug"].as_u64().expect("the count")
    );
    assert_eq!(
        u64::from(snapshot.by_level[TelemetryLevel::Info as usize]),
        want["info"].as_u64().expect("the count")
    );
    assert_eq!(
        u64::from(snapshot.by_level[TelemetryLevel::Warn as usize]),
        want["warn"].as_u64().expect("the count")
    );
    assert_eq!(
        u64::from(snapshot.by_level[TelemetryLevel::Error as usize]),
        want["error"].as_u64().expect("the count")
    );
    assert_eq!(
        u64::from(snapshot.emitted),
        want["emitted"].as_u64().expect("the shipped count")
    );
    assert_eq!(
        u64::from(snapshot.dropped),
        want["dropped"].as_u64().expect("the dropped count"),
        "what was dropped is still counted"
    );
}

/// Reads a link cost back from the name the vectors record.
fn link_cost(name: &str) -> LinkCost {
    match name {
        "Free" => LinkCost::Free,
        "Metered" => LinkCost::Metered,
        "Expensive" => LinkCost::Expensive,
        "Offline" => LinkCost::Offline,
        other => panic!("unknown link cost {other}"),
    }
}

/// Reads a telemetry level back from the name the vectors record.
fn telemetry_level(name: &str) -> TelemetryLevel {
    match name {
        "Trace" => TelemetryLevel::Trace,
        "Debug" => TelemetryLevel::Debug,
        "Info" => TelemetryLevel::Info,
        "Warn" => TelemetryLevel::Warn,
        "Error" => TelemetryLevel::Error,
        other => panic!("unknown level {other}"),
    }
}

/// Names a telemetry level the way the vectors record it.
fn telemetry_level_name(level: TelemetryLevel) -> &'static str {
    match level {
        TelemetryLevel::Trace => "Trace",
        TelemetryLevel::Debug => "Debug",
        TelemetryLevel::Info => "Info",
        TelemetryLevel::Warn => "Warn",
        TelemetryLevel::Error => "Error",
    }
}

#[test]
fn ladder_vectors_match() {
    let vectors = vectors();
    let vector = &vectors["ladder"];
    let topic = vector["topic"].as_str().expect("the topic");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build a runtime");

    runtime.block_on(async {
        let broker = LoopbackBroker::new();
        let mut listener = LoopbackTransport::new(broker.clone());
        listener.connect().await.expect("connect the listener");
        listener.subscribe(topic).await.expect("subscribe");

        let payloads = vector["payloads"].as_array().expect("the payloads");
        let offline_want = &vector["withNoRung"];
        let deliveries = offline_want["deliveries"].as_array().expect("the outcomes");

        let mut offline = TransportLadder::new(BufferStore::new());
        for (at, payload) in payloads.iter().enumerate() {
            let delivery = offline
                .send(topic, payload.as_str().expect("the payload").as_bytes())
                .await
                .expect("send with no rung");
            assert_eq!(
                delivery_name(delivery),
                deliveries[at].as_str().expect("the outcome"),
                "a message no rung takes is buffered rather than lost"
            );
        }
        assert_eq!(
            offline.buffered().await.expect("count") as u64,
            offline_want["buffered"].as_u64().expect("the count")
        );

        let restored_want = &vector["afterTheLinkReturns"];
        let mut restored = offline.rung(LoopbackTransport::new(broker.clone()));
        restored.connect().await.expect("connect");
        assert_eq!(
            restored.flush().await.expect("flush") as u64,
            restored_want["flushed"]
                .as_u64()
                .expect("the flushed count"),
            "the buffer replays once a link returns"
        );
        assert_eq!(
            restored.buffered().await.expect("count") as u64,
            restored_want["buffered"].as_u64().expect("the count")
        );

        let fallthrough = &vector["fallthrough"];
        let failures = fallthrough["failuresOnFirstRung"]
            .as_u64()
            .expect("the failure count") as usize;
        let mut ladder = TransportLadder::new(BufferStore::new())
            .rung(Faulty::new(
                LoopbackTransport::new(broker.clone()),
                failures,
            ))
            .rung(LoopbackTransport::new(broker.clone()));
        ladder.connect().await.expect("connect the rungs");
        let delivery = ladder
            .send(
                topic,
                fallthrough["payload"]
                    .as_str()
                    .expect("the payload")
                    .as_bytes(),
            )
            .await
            .expect("send through the ladder");
        assert_eq!(
            delivery_name(delivery),
            fallthrough["delivery"].as_str().expect("the outcome"),
            "a rung that refuses falls through to the next"
        );
    });
}

/// Names a delivery outcome the way the vectors record it.
fn delivery_name(delivery: Delivery) -> &'static str {
    match delivery {
        Delivery::Sent => "Sent",
        Delivery::Buffered => "Buffered",
    }
}

#[test]
fn simulation_vectors_match() {
    let vectors = vectors();
    let vector = &vectors["simulation"];

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build a runtime");

    runtime.block_on(async {
        let want = &vector["sensor"];
        let mut sensor = SimSensor::new(float(&want["baseline"]))
            .with_drift(float(&want["driftPerRead"]))
            .with_noise(float(&want["noise"]))
            .with_seed(want["seed"].as_u64().expect("the seed") as u32);
        for reading in want["readings"].as_array().expect("the readings") {
            assert_eq!(
                sensor.read().await.expect("read"),
                float(reading),
                "a seeded sensor invents the same run everywhere"
            );
        }

        let want = &vector["replay"];
        let mut replay = Replay::repeating(floats(&want["capture"]));
        for reading in want["readings"].as_array().expect("the readings") {
            assert_eq!(replay.read().await.expect("read"), float(reading));
        }

        let want = &vector["robot"];
        let mut robot = SimRobot::new(float(&want["dt"]));
        let twist = pamoja_kit::Twist::new(float(&want["vx"]), 0.0, float(&want["omega"]));
        for pose in want["poses"].as_array().expect("the poses") {
            robot.apply(twist).await.expect("drive");
            let reached = robot.pose();
            assert_eq!(reached.x, float(&pose["x"]), "the x it reached");
            assert_eq!(reached.y, float(&pose["y"]), "the y it reached");
            assert_eq!(reached.theta, float(&pose["theta"]), "the heading it holds");
        }
    });
}

#[test]
fn profile_vectors_match() {
    let vectors = vectors();
    let vector = &vectors["profile"];

    let cold_chain = &vector["coldChain"];
    let fridge = Profile::vaccine_fridge_monitor();
    assert_eq!(fridge.name, cold_chain["name"].as_str().expect("the name"));
    assert_eq!(
        fridge.topic,
        cold_chain["topic"].as_str().expect("the topic")
    );
    assert_control(fridge.control, &cold_chain["control"]);

    let power = &cold_chain["power"];
    assert_eq!(
        fridge.power.active_secs,
        power["activeSecs"].as_u64().expect("the active cadence")
    );
    assert_eq!(
        fridge.power.saver_below,
        float(&power["saverBelow"]),
        "the saver threshold"
    );

    let mut control = fridge.controller();
    assert_reactions(&mut control, &cold_chain["reactions"]);

    let draining = &vector["draining"];
    let well = Profile::well_level();
    assert_eq!(well.name, draining["name"].as_str().expect("the name"));
    assert_control(well.control, &draining["control"]);
    let mut level = well.controller();
    assert_reactions(&mut level, &draining["reactions"]);

    let mut observer = Controller::monitor();
    let observed = &vector["observed"];
    let reaction = observer.evaluate(float(&observed["reading"]));
    assert!(
        reaction.actuator.is_none(),
        "a monitoring profile drives no output"
    );
    assert_eq!(alert_name(reaction.alert.as_ref()), "None");
}

/// Walks a controller through a recorded run and checks every decision.
fn assert_reactions(control: &mut Controller, reactions: &Value) {
    for want in reactions.as_array().expect("the reactions") {
        let reading = float(&want["reading"]);
        let reaction = control.evaluate(reading);
        assert_eq!(
            reaction.actuator,
            want["actuator"].as_bool(),
            "the output setting at {reading}"
        );

        let alert = &want["alert"];
        assert_eq!(
            alert_name(reaction.alert.as_ref()),
            alert["kind"].as_str().expect("the alert kind"),
            "the alert raised at {reading}"
        );
        match reaction.alert {
            Some(Alert::OutOfRange { reading: offending }) => {
                assert_eq!(offending, float(&alert["reading"]));
            }
            Some(Alert::RunningOut { samples }) => {
                assert_eq!(
                    u64::from(samples),
                    alert["samples"].as_u64().expect("count")
                );
            }
            Some(Alert::ChangingFast { rate }) => {
                assert_eq!(rate, float(&alert["rate"]));
            }
            None => {}
        }
    }
}

/// Checks a control policy against the flattened form the vectors carry.
fn assert_control(spec: ControlSpec, want: &Value) {
    let kind = want["kind"].as_str().expect("the policy kind");
    match spec {
        ControlSpec::Setpoint {
            setpoint,
            hysteresis,
            cooling,
            safe_band,
        } => {
            assert_eq!(kind, "Setpoint");
            assert_eq!(setpoint, float(&want["setpoint"]));
            assert_eq!(hysteresis, float(&want["hysteresis"]));
            assert_eq!(cooling, want["cooling"].as_bool().expect("the direction"));
            assert_eq!(safe_band, float(&want["safeBand"]));
        }
        ControlSpec::Level { empty, warn_within } => {
            assert_eq!(kind, "Level");
            assert_eq!(empty, float(&want["empty"]));
            assert_eq!(
                u64::from(warn_within),
                want["warnWithin"].as_u64().expect("the warning horizon")
            );
        }
        ControlSpec::Surge { rising, limit } => {
            assert_eq!(kind, "Surge");
            assert_eq!(rising, want["rising"].as_bool().expect("the direction"));
            assert_eq!(limit, float(&want["limit"]));
        }
        ControlSpec::Monitor => assert_eq!(kind, "Monitor"),
    }
}

/// Names an alert the way the vectors record it.
fn alert_name(alert: Option<&Alert>) -> &'static str {
    match alert {
        None => "None",
        Some(Alert::OutOfRange { .. }) => "OutOfRange",
        Some(Alert::RunningOut { .. }) => "RunningOut",
        Some(Alert::ChangingFast { .. }) => "ChangingFast",
    }
}

#[test]
fn ros2_vectors_match() {
    let vectors = vectors();
    let vector = &vectors["ros2"];

    for case in vector["names"].as_array().expect("the names") {
        let name = case["name"].as_str().expect("the name");
        assert_eq!(
            is_valid_name(name),
            case["valid"].as_bool().expect("the verdict"),
            "whether `{name}` obeys the ROS 2 rules"
        );
        assert_eq!(
            is_fully_qualified(name),
            case["fullyQualified"].as_bool().expect("the verdict"),
            "whether `{name}` is fully qualified"
        );
    }

    for case in vector["ddsTopics"].as_array().expect("the topics") {
        let fqn = case["fqn"].as_str().expect("the name");
        let kind = entity_kind(case["kind"].as_str().expect("the kind"));
        assert_eq!(
            dds_topic(fqn, kind).as_deref(),
            case["topic"].as_str(),
            "the DDS topic for `{fqn}`"
        );
    }

    let prefixes = &vector["prefixes"];
    for (name, kind) in [
        ("Topic", EntityKind::Topic),
        ("ServiceRequest", EntityKind::ServiceRequest),
        ("ServiceResponse", EntityKind::ServiceResponse),
    ] {
        assert_eq!(kind.prefix(), prefixes[name].as_str().expect("the prefix"));
    }

    let mangled = &vector["mangled"];
    assert_eq!(
        percent_mangle(mangled["name"].as_str().expect("the name")),
        mangled["mangled"].as_str().expect("the mangled name")
    );

    for case in vector["typeNames"].as_array().expect("the type names") {
        let ros_type = case["rosType"].as_str().expect("the type");
        assert_eq!(
            dds_type_name(ros_type).as_deref(),
            case["ddsType"].as_str(),
            "the DDS type name for `{ros_type}`"
        );
    }

    let type_hash = &vector["typeHash"];
    let text = type_hash["text"].as_str().expect("the hash");
    let hash = TypeHash::parse(text).expect("a well-formed RIHS01 hash");
    assert_eq!(
        hex(&hash.digest()),
        type_hash["digest"].as_str().expect("the digest")
    );
    assert_eq!(hash.to_string(), text, "a hash renders back to its string");

    let key = &vector["entityKey"];
    assert_eq!(
        entity_key(
            key["domainId"].as_u64().expect("the domain") as u32,
            key["fqn"].as_str().expect("the name"),
            key["rosType"].as_str().expect("the type"),
            &hash,
        )
        .as_deref(),
        key["key"].as_str(),
        "the Zenoh key an rmw_zenoh peer publishes on"
    );

    let twist = &vector["twist"];
    let linear = floats(&twist["linear"]);
    let angular = floats(&twist["angular"]);
    let command = Ros2Twist {
        linear: Vector3::new(
            f64::from(linear[0]),
            f64::from(linear[1]),
            f64::from(linear[2]),
        ),
        angular: Vector3::new(
            f64::from(angular[0]),
            f64::from(angular[1]),
            f64::from(angular[2]),
        ),
    };
    let encoded = command.to_cdr();
    assert_eq!(
        hex(&encoded),
        twist["cdr"].as_str().expect("the encoded twist"),
        "a twist encodes to the same CDR everywhere"
    );
    assert_eq!(
        Ros2Twist::from_cdr(&encoded),
        Some(command),
        "and decodes back unchanged"
    );

    let mixed = &vector["mixedWidths"];
    let bytes = unhex(&mixed["cdr"]);
    let mut reader = CdrReader::new(&bytes).expect("a valid encapsulation header");
    assert_eq!(
        u64::from(reader.read_u32().expect("the word")),
        mixed["word"].as_u64().expect("the word")
    );
    assert_eq!(
        reader.read_f64().expect("the double"),
        mixed["double"].as_f64().expect("the double"),
        "an eight-byte field keeps its alignment"
    );
    assert_eq!(
        i64::from(reader.read_i32().expect("the signed word")),
        mixed["signed"].as_i64().expect("the signed word"),
        "and the field after it is not skewed"
    );
}

/// Maps a subsystem name onto the kind it selects.
fn entity_kind(name: &str) -> EntityKind {
    match name {
        "Topic" => EntityKind::Topic,
        "ServiceRequest" => EntityKind::ServiceRequest,
        "ServiceResponse" => EntityKind::ServiceResponse,
        other => panic!("unknown entity kind `{other}`"),
    }
}

#[test]
fn zenoh_vectors_match() {
    let vectors = vectors();
    let vector = &vectors["zenoh"];

    for case in vector["expressions"].as_array().expect("the expressions") {
        let key = case["key"].as_str().expect("the expression");
        assert_eq!(
            keyexpr::is_valid(key),
            case["valid"].as_bool().expect("the verdict"),
            "whether `{key}` is well formed"
        );
        assert_eq!(
            keyexpr::is_canon(key),
            case["canon"].as_bool().expect("the verdict"),
            "whether `{key}` is already canonical"
        );
    }

    for case in vector["canonized"].as_array().expect("the canonical forms") {
        let key = case["key"].as_str().expect("the expression");
        assert_eq!(
            keyexpr::canonize(key).as_deref(),
            case["canonical"].as_str(),
            "the canonical form of `{key}`"
        );
    }

    for case in vector["matches"].as_array().expect("the matches") {
        let pattern = case["pattern"].as_str().expect("the pattern");
        let key = case["key"].as_str().expect("the key");
        assert_eq!(
            keyexpr::matches(pattern, key),
            case["matches"].as_bool().expect("the verdict"),
            "whether `{pattern}` selects `{key}`"
        );
    }
}
