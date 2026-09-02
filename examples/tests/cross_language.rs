//! The Rust side of the cross-language conformance suite.
//!
//! Every binding runs these same vectors from `conformance/vectors.json`. This
//! test is the reference: it proves the committed file still matches what the
//! Rust implementation produces, so a stale or hand-edited vector is caught here
//! before a binding is blamed for disagreeing with it.

use std::fs;
use std::path::PathBuf;

use pamoja_actuators::{pca9685, stepper};
use pamoja_can::{dlc_to_len, len_to_dlc, CanId, Frame, J1939Id};
use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer};
use pamoja_gpio::i2c::{Address, Direction};
use pamoja_gpio::pin::{Edge, Level, Polarity};
use pamoja_gpio::spi::Mode;
use pamoja_kit::{
    deadband, Anomaly, Boundary, Calibration, Coordinate, Depletion, Geofence, Median, Pid,
    Smoother, Thermostat, Trend, Window,
};
use pamoja_lora::LinkSettings;
use pamoja_lorawan::{
    Device, Direction as LorawanDirection, Downlink, FrameHeader, JoinGrant, JoinRequest,
    MessageType, Session, Uplink,
};
use pamoja_mesh::{crc16 as mesh_crc16, DynamicSeenCache, Frame as MeshFrame};
use pamoja_modbus::Pdu;
use pamoja_modbus::{crc16, Adu};
use pamoja_routing::{DynamicRouter, Forward};
use pamoja_security::{DeviceIdentity, PublicIdentity, Signature};
use pamoja_sensors::{ads1115, bme280, ds18b20, ina219};
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
            "a spreading factor outside 7 to 12 is clamped"
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
