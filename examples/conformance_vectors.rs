//! Regenerates the cross-language conformance vectors.
//!
//! The vectors are produced here, from the Rust implementation, and committed to
//! `conformance/vectors.json`. Every binding's test suite then loads that one file
//! and asserts the same results, so a facade that drifts in any language fails
//! rather than quietly disagreeing with the others.
//!
//! Run with: `cargo run -p pamoja-examples --example conformance_vectors`

// One literal holds every section, and each one added unfolds further, so the macro needs
// more room than the default allows.
#![recursion_limit = "256"]

use std::fs;
use std::path::PathBuf;

use pamoja_actuators::{pca9685, stepper};
use pamoja_audit::{AuditLog, Entry};
use pamoja_can::{dlc_to_len, len_to_dlc, CanId, Frame, J1939Id};
use pamoja_codec::{encode_deltas, json_to_cbor, Quantizer};
use pamoja_core::{Actuator as _, Sensor as _, Transport as _};
use pamoja_gpio::i2c::{Address, Direction};
use pamoja_gpio::pin::{Edge, Level, Polarity};
use pamoja_gpio::spi::Mode;
use pamoja_kit::{
    deadband, Anomaly, Boundary, Calibration, Coordinate, Depletion, Edge as TriggerEdge, Geofence,
    Median, Pid, Smoother, Thermostat, Trend, Trigger, Window,
};
use pamoja_ladder::{Delivery, TransportLadder};
use pamoja_loopback::{Faulty, LoopbackBroker, LoopbackTransport};
use pamoja_lora::budget::{self, Decibels, Fcc15247, LinkBudget};
use pamoja_lora::region::{
    ChannelPlan, ChannelPlanBuilder, DataRate, MaxPayload, Modulation, Region, SubBand,
};
use pamoja_lora::LinkSettings;
use pamoja_lorawan::{Device, Downlink, FrameHeader, JoinGrant, JoinRequest, Session, Uplink};
use pamoja_mavlink::dialect::{
    crc_extra as mavlink_crc_extra, descriptor_by_name, encode_message as mav_encode,
    DynamicMessage as MavDynamicMessage, FieldType as MavFieldType, Message as _,
    MessageDescriptorBuilder as MavDescriptorBuilder, MissionItemInt, DESCRIPTORS,
};
use pamoja_mavlink::protocol::TypeMask;
use pamoja_mavlink::{
    crc16_mcrf4xx, message_crc_extra, signing, Frame as MavFrame, Header as MavHeader,
    Signer as MavSigner,
};
use pamoja_mesh::{crc16 as mesh_crc16, DynamicSeenCache, Frame as MeshFrame};
use pamoja_modbus::{crc16, Adu, Pdu, Response};
use pamoja_power::{DutyCycle, PowerMode, PowerPlan};
use pamoja_profile::{Alert, ControlSpec, Controller, PowerSchedule, Profile, Reaction};
use pamoja_radios::duty::DutyCycle as RadioDutyCycle;
use pamoja_radios::sx126x::{
    command as sx126x_command, config as sx126x_config, irq as sx126x_irq, status as sx126x_status,
};
use pamoja_radios::sx127x::{
    config as sx127x_config, irq as sx127x_irq, register as sx127x_register,
    status as sx127x_status,
};
use pamoja_ros2::key::entity_key;
use pamoja_ros2::msg::{CdrWriter, Twist as Ros2Twist, Vector3};
use pamoja_ros2::name::{percent_mangle, EntityKind};
use pamoja_ros2::typehash::TypeHash;
use pamoja_routing::{DynamicRouter, Forward};
use pamoja_security::DeviceIdentity;
use pamoja_sensors::{
    ads1115, bme280, bmp280, ds18b20, hdc1080, ina219, ina226, opt3001, scd4x, sht3x, tmp117,
};
use pamoja_serial::{cobs, slip};
use pamoja_session::{AgreementKey, Role, Session as SecuredSession};
use pamoja_sim::{Replay, SimSensor};
use pamoja_sync::MemoryStore as BufferStore;
use pamoja_telemetry::{Event, Level as TelemetryLevel, LinkCost, Reporter};
use pamoja_update::{
    Boot, Delegation, Device as UpdateDevice, Manifest, MemoryStore, PayloadFormat, SlotState,
    SlotStore, Updater,
};
use pamoja_zenoh::keyexpr;
use serde_json::{json, Value};

/// The seed every identity vector is derived from.
const SEED: [u8; 32] = [7u8; 32];

/// The payload the signature vector covers.
const PAYLOAD: &str = "21.5";

/// The document the codec vectors transcode.
const DOCUMENT: &str = r#"{"battery":88,"c":21.5,"id":"probe-1"}"#;

/// The seed the audit chain is signed with.
const AUDIT_SEED: [u8; 32] = [0x21; 32];

/// The seeds and salt the secured-session vectors agree a key from.
const SESSION_NODE_SEED: [u8; 32] = [0x01; 32];
const SESSION_GATEWAY_SEED: [u8; 32] = [0x02; 32];
const SESSION_SALT: [u8; 16] = [0x09; 16];

/// The keys the update vectors sign and delegate with.
const PUBLISHER_SEED: [u8; 32] = [0x31; 32];
const IMPOSTOR_SEED: [u8; 32] = [0x32; 32];
const ANCHOR_SEED: [u8; 32] = [0x41; 32];
const RELEASE_SEED: [u8; 32] = [0x42; 32];

/// Who the update vectors are built for.
const VENDOR_ID: [u8; 16] = [0x0a; 16];
const CLASS_ID: [u8; 16] = [0x0b; 16];

/// The topic and payloads the ladder vectors move.
const LADDER_TOPIC: &str = "sensors/1";
const LADDER_PAYLOADS: [&str; 2] = ["21.5", "21.7"];
const LADDER_FALLTHROUGH: &str = "4.8C";

/// The run the simulated devices are asked to repeat.
const SIM_BASELINE: f32 = 20.0;
const SIM_DRIFT: f32 = 0.5;
const SIM_NOISE: f32 = 1.0;
const SIM_SEED: u32 = 42;
const SIM_READS: usize = 6;
const SIM_CAPTURE: [f32; 3] = [21.0, 21.5, 22.0];
const SIM_DT: f32 = 1.0;
const SIM_VX: f32 = 1.0;
const SIM_OMEGA: f32 = 0.25;
const SIM_STEPS: usize = 3;

fn main() {
    let vectors = json!({
        "note": "Generated by `cargo run -p pamoja-examples --example conformance_vectors`. \
                 Do not edit by hand.",
        "tolerance": 1e-6,
        "identity": identity(),
        "codec": codec(),
        "smoother": smoother(),
        "pid": pid(),
        "thermostat": thermostat(),
        "trigger": trigger(),
        "depletion": depletion(),
        "calibration": calibration(),
        "deadband": deadband_vectors(),
        "geofence": geofence(),
        "serial": serial(),
        "modbus": modbus(),
        "can": can(),
        "gpio": gpio(),
        "sensors": sensors(),
        "actuators": actuators(),
        "windows": windows(),
        "lora": lora(),
        "loraRegions": lora_regions(),
        "radios": radios(),
        "gateway": gateway(),
        "gatewayNetwork": gateway_network(),
        "station": station(),
        "mavlink": mavlink(),
        "mavlinkSchema": mavlink_schema(),
        "mavlinkProtocol": mavlink_protocol(),
        "mesh": mesh(),
        "routing": routing(),
        "lorawan": lorawan(),

        "header": header(),
        "network": network(),
        "audit": audit(),
        "session": session(),
        "update": update(),
        "power": power(),
        "telemetry": telemetry(),
        "ladder": ladder(),
        "simulation": simulation(),
        "profile": profile(),
        "ros2": ros2(),
        "zenoh": zenoh(),
    });

    let path = repo_root().join("conformance").join("vectors.json");
    fs::create_dir_all(path.parent().expect("conformance directory"))
        .expect("create the conformance directory");
    let mut rendered = serde_json::to_string_pretty(&vectors).expect("render the vectors");
    rendered.push('\n');
    fs::write(&path, rendered).expect("write the vectors");
    println!("wrote {}", path.display());
}

/// Signing and verifying, plus the labels derived from a public key.
fn identity() -> Value {
    let device = DeviceIdentity::from_seed(&SEED);
    let public = device.public();
    json!({
        "seed": hex(&SEED),
        "publicKey": hex(&public.to_bytes()),
        "fingerprint": public.fingerprint(),
        "payload": PAYLOAD,
        "signature": hex(&device.sign(PAYLOAD.as_bytes()).to_bytes()),
        "tamperedPayload": "21.6",
    })
}

/// Transcoding a document, and packing samples and readings for a metered link.
fn codec() -> Value {
    let samples: Vec<i64> = vec![10, 11, 13, 12, 900];
    let readings: Vec<f32> = vec![20.0, 20.1, 20.2, 20.3];
    let scale = 100.0f32;
    json!({
        "json": DOCUMENT,
        "cbor": hex(&json_to_cbor(DOCUMENT.as_bytes()).expect("transcode the document")),
        "unsortedJson": r#"{"id":"probe-1","c":21.5,"battery":88}"#,
        "deltas": {
            "samples": samples,
            "packed": hex(&encode_deltas(&samples)),
        },
        "quantizer": {
            "scale": scale,
            "readings": readings,
            "packed": hex(&Quantizer::new(scale).encode(&readings)),
            "tolerance": 1.0 / f64::from(scale),
        },
    })
}

/// Exponential smoothing over a step input.
fn smoother() -> Value {
    let weight = 0.5f32;
    let samples: Vec<f32> = vec![10.0, 20.0, 20.0, 20.0, 12.0];
    let mut smoother = Smoother::new(weight);
    let outputs: Vec<f32> = samples
        .iter()
        .map(|&sample| smoother.update(sample))
        .collect();
    json!({ "weight": weight, "samples": samples, "outputs": outputs })
}

/// A PID controller driven toward a fixed setpoint.
fn pid() -> Value {
    let (kp, ki, kd, setpoint, dt) = (1.0f32, 0.1f32, 0.05f32, 10.0f32, 0.1f32);
    let measurements: Vec<f32> = vec![0.0, 2.0, 5.0, 8.0, 9.5, 10.0];
    let mut controller = Pid::new(kp, ki, kd);
    let outputs: Vec<f32> = measurements
        .iter()
        .map(|&measurement| controller.update(setpoint, measurement, dt))
        .collect();
    json!({
        "kp": kp, "ki": ki, "kd": kd,
        "setpoint": setpoint, "dt": dt,
        "measurements": measurements,
        "outputs": outputs,
    })
}

/// On/off control with hysteresis, over a reading that crosses the band twice.
fn thermostat() -> Value {
    let (setpoint, hysteresis) = (8.0f32, 1.0f32);
    let readings: Vec<f32> = vec![7.0, 8.5, 9.5, 8.5, 7.5, 6.5];
    let mut thermostat = Thermostat::cooling(setpoint, hysteresis);
    let outputs: Vec<bool> = readings
        .iter()
        .map(|&reading| thermostat.update(reading))
        .collect();
    json!({
        "mode": "cooling",
        "setpoint": setpoint,
        "hysteresis": hysteresis,
        "readings": readings,
        "outputs": outputs,
    })
}

/// A falling trigger with a release band, over readings that cross, hold, and come back.
fn trigger() -> Value {
    let (threshold, hysteresis) = (30.0f32, 5.0f32);
    let readings: Vec<f32> = vec![42.0, 31.0, 28.0, 25.0, 33.0, 36.0, 36.0, 29.9];
    let mut trigger = Trigger::below(threshold, hysteresis);
    let outputs: Vec<Value> = readings
        .iter()
        .map(|&reading| match trigger.update(reading) {
            Some(TriggerEdge::Set) => json!("set"),
            Some(TriggerEdge::Cleared) => json!("cleared"),
            None => Value::Null,
        })
        .collect();
    json!({
        "mode": "below",
        "threshold": threshold,
        "hysteresis": hysteresis,
        "readings": readings,
        "outputs": outputs,
    })
}

/// Countdown to a threshold, including the readings that produce no estimate.
fn depletion() -> Value {
    let threshold = 10.0f32;
    let levels: Vec<f32> = vec![100.0, 90.0, 80.0, 80.0, 60.0, 5.0];
    let mut depletion = Depletion::new(threshold);
    let outputs: Vec<Value> = levels
        .iter()
        .map(|&level| match depletion.update(level) {
            Some(samples) => json!(samples),
            None => Value::Null,
        })
        .collect();
    json!({ "threshold": threshold, "levels": levels, "outputs": outputs })
}

/// A two-point fit mapping raw counts onto real units.
fn calibration() -> Value {
    let (raw_low, value_low, raw_high, value_high) = (0.0f32, 0.0f32, 1024.0f32, 100.0f32);
    let calibration = Calibration::two_point(raw_low, value_low, raw_high, value_high);
    let inputs: Vec<f32> = vec![0.0, 256.0, 512.0, 1024.0];
    let outputs: Vec<f32> = inputs.iter().map(|&raw| calibration.apply(raw)).collect();
    json!({
        "rawLow": raw_low, "valueLow": value_low,
        "rawHigh": raw_high, "valueHigh": value_high,
        "inputs": inputs, "outputs": outputs,
    })
}

/// Suppressing movement inside a band around a center value.
fn deadband_vectors() -> Value {
    let (center, width) = (0.0f32, 0.5f32);
    let inputs: Vec<f32> = vec![0.0, 0.2, -0.2, 1.0, -1.0];
    let outputs: Vec<f32> = inputs
        .iter()
        .map(|&value| deadband(value, center, width))
        .collect();
    json!({ "center": center, "width": width, "inputs": inputs, "outputs": outputs })
}

/// Fixes walked out of a fence and back, so every crossing state appears.
fn geofence() -> Value {
    let center = Coordinate::new(-1.2921, 36.8219);
    let radius_m = 50.0;
    let fixes = [
        (-1.2921, 36.8219),
        (-1.2930, 36.8219),
        (-1.2935, 36.8219),
        (-1.2921, 36.8219),
    ];
    let mut fence = Geofence::new(center, radius_m);
    let boundaries: Vec<&str> = fixes
        .iter()
        .map(|&(latitude, longitude)| name(fence.update(Coordinate::new(latitude, longitude))))
        .collect();
    json!({
        "center": { "latitude": center.latitude, "longitude": center.longitude },
        "radiusM": radius_m,
        "fixes": fixes
            .iter()
            .map(|&(latitude, longitude)| json!({ "latitude": latitude, "longitude": longitude }))
            .collect::<Vec<Value>>(),
        "boundaries": boundaries,
    })
}

/// Both serial framings over a payload full of bytes each one has to escape, plus
/// a stream carrying a corrupt frame between two good ones.
fn serial() -> Value {
    // Every byte here is special to one framing or the other: the SLIP delimiter
    // and escape, and the zero COBS removes.
    let payload = [0xC0u8, 0xDB, 0x00, 0x2A];
    let mut framed = [0u8; 64];

    let slip_len = slip::encode(&payload, &mut framed).expect("frame the payload");
    let slip_frame = framed[..slip_len].to_vec();
    let cobs_len = cobs::encode(&payload, &mut framed).expect("frame the payload");
    let cobs_frame = framed[..cobs_len].to_vec();

    // A good frame, an escape truncated by the delimiter, then a good frame. The
    // corrupt one must be dropped without taking its neighbors with it.
    let stream = [b'o', b'k', 0xC0, 0xDB, 0xC0, b'g', b'o', 0xC0];
    let mut decoder: slip::SlipDecoder<64> = slip::SlipDecoder::new();
    let mut frames: Vec<String> = Vec::new();
    let mut discarded = 0u32;
    for &byte in &stream {
        match decoder.push(byte) {
            Ok(Some(frame)) => frames.push(hex(frame)),
            Ok(None) => {}
            Err(_) => discarded += 1,
        }
    }

    let cobs_stream = [0x03u8, 0x11, 0x22, 0x02, 0x33, 0x00];
    let mut cobs_decoder: cobs::CobsDecoder<64> = cobs::CobsDecoder::new();
    let mut cobs_frames: Vec<String> = Vec::new();
    for &byte in &cobs_stream {
        if let Ok(Some(frame)) = cobs_decoder.push(byte) {
            cobs_frames.push(hex(frame));
        }
    }

    json!({
        "payload": hex(&payload),
        "slipFrame": hex(&slip_frame),
        "cobsFrame": hex(&cobs_frame),
        "slipMaxEncodedLen": slip::max_encoded_len(payload.len()),
        "cobsMaxEncodedLen": cobs::max_encoded_len(payload.len()),
        "corruptSlipFrame": hex(&[0xDB, 0x01, 0xC0]),
        "slipStream": {
            "bytes": hex(&stream),
            "chunk": 3,
            "frames": frames,
            "discarded": discarded,
        },
        "cobsStream": {
            "bytes": hex(&cobs_stream),
            "chunk": 4,
            "frames": cobs_frames,
        },
    })
}

/// Modbus request frames, the replies they draw, and a frame corrupted on the wire.
fn modbus() -> Value {
    let read = Pdu::read_holding_registers(0x006B, 3).to_adu(0x11);

    // The reply the specification's own worked example gives for that request.
    let reply = Adu::from_pdu(0x11, &[0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64])
        .expect("assemble the reply");
    let registers: Vec<u16> = Response::new(reply.pdu())
        .registers()
        .expect("read the registers")
        .collect();

    // A read-coils reply carrying four bits packed least-significant first.
    let bit_reply = Adu::from_pdu(0x11, &[0x01, 0x01, 0b0000_1101]).expect("assemble the reply");
    let coils: Vec<bool> = Response::new(bit_reply.pdu())
        .coils(4)
        .expect("read the coils")
        .collect();

    // Registers above 0x7FFF, which catch a binding that reads them as signed.
    let high =
        Adu::from_pdu(0x11, &[0x03, 0x04, 0xFF, 0xFF, 0x80, 0x00]).expect("assemble the reply");
    let high_registers: Vec<u16> = Response::new(high.pdu())
        .registers()
        .expect("read the registers")
        .collect();

    let refused = Adu::from_pdu(0x11, &[0x83, 0x02]).expect("assemble the reply");

    let mut corrupt = read.as_bytes().to_vec();
    corrupt[2] ^= 0xFF;
    let checked = &read.as_bytes()[..read.as_bytes().len() - 2];

    json!({
        "readHoldingRegisters": {
            "address": 0x11, "start": 0x006B, "count": 3,
            "frame": hex(read.as_bytes()),
        },
        "readCoils": {
            "address": 0x11, "start": 0x0013, "count": 37,
            "frame": hex(Pdu::read_coils(0x0013, 37).to_adu(0x11).as_bytes()),
        },
        "writeSingleRegister": {
            "address": 0x11, "register": 0x0001, "value": 0x0003,
            "frame": hex(Pdu::write_single_register(0x0001, 0x0003).to_adu(0x11).as_bytes()),
        },
        "writeMultipleRegisters": {
            "address": 0x11, "start": 0x0001, "values": [0x000A, 0x0102],
            "frame": hex(
                Pdu::write_multiple_registers(0x0001, &[0x000A, 0x0102])
                    .expect("build the request")
                    .to_adu(0x11)
                    .as_bytes(),
            ),
        },
        "writeMultipleCoils": {
            "address": 0x11, "start": 0x0013, "values": [true, false, true, true, false],
            "frame": hex(
                Pdu::write_multiple_coils(0x0013, &[true, false, true, true, false])
                    .expect("build the request")
                    .to_adu(0x11)
                    .as_bytes(),
            ),
        },
        "reply": {
            "frame": hex(reply.as_bytes()),
            "address": reply.address(),
            "functionCode": reply.function_code(),
            "pdu": hex(reply.pdu()),
            "registers": registers,
        },
        "highRegisterReply": {
            "frame": hex(high.as_bytes()),
            "registers": high_registers,
        },
        "bitReply": {
            "frame": hex(bit_reply.as_bytes()),
            "count": 4,
            "coils": coils,
        },
        "exceptionReply": {
            "frame": hex(refused.as_bytes()),
            "functionCode": refused.function_code(),
            "exception": refused.exception().map(pamoja_modbus::Exception::code),
        },
        "corruptFrame": hex(&corrupt),
        "crc": { "data": hex(checked), "value": crc16(checked) },
    })
}

/// CAN frames of each kind, the CAN-FD length table, and J1939 identifiers.
fn can() -> Value {
    let classic = Frame::new(CanId::standard(0x20A), &[0x01, 0xF4]).expect("build the frame");
    let fd = Frame::fd(CanId::extended(0x1234_5678), &[0xAB; 32]).expect("build the frame");
    let remote = Frame::remote(CanId::standard(0x20A), 4);

    let lengths: Vec<Value> = [0usize, 1, 8, 12, 16, 20, 24, 32, 48, 64]
        .iter()
        .map(|&len| json!({ "len": len, "dlc": len_to_dlc(len) }))
        .collect();
    let codes: Vec<Value> = (0u8..16)
        .map(|dlc| json!({ "dlc": dlc, "len": dlc_to_len(dlc) }))
        .collect();

    json!({
        "classic": {
            "id": 0x20A, "extended": false, "data": hex(classic.data()),
            "dlc": classic.dlc(), "len": classic.len(),
        },
        "fd": {
            "id": 0x1234_5678u32, "extended": true, "data": hex(fd.data()),
            "dlc": fd.dlc(), "len": fd.len(),
        },
        "remote": {
            "id": 0x20A, "extended": false, "requested": 4,
            "dlc": remote.dlc(), "len": remote.len(), "dataLen": remote.data().len(),
        },
        "tooLongForClassic": 9,
        "invalidFdLength": 13,
        "lengths": lengths,
        "codes": codes,
        "j1939": [
            // Electronic engine controller 1, a PDU2 broadcast every genset sends.
            j1939(0x0CF0_0400),
            // A request PGN, which is PDU1 and so names a destination.
            j1939(J1939Id::from_parts(6, 0x0EA00, 0x21, 0x0A).to_id().raw()),
        ],
        "standardIsNotJ1939": 0x123,
    })
}

/// One J1939 identifier decoded into the fields every binding reports.
fn j1939(raw: u32) -> Value {
    let message = J1939Id::from_id(CanId::extended(raw)).expect("decode the identifier");
    json!({
        "id": raw,
        "pgn": message.pgn(),
        "priority": message.priority(),
        "source": message.source(),
        "pduFormat": message.pdu_format(),
        "destination": message.destination(),
        "broadcast": message.is_broadcast(),
    })
}

/// I2C address frames, the SPI mode table, and the pin logic around them.
fn gpio() -> Value {
    let addresses: Vec<Value> = [
        (0x76u16, false),
        (0x68, false),
        (0x00, false),
        (0x07, false),
        (0x08, false),
        (0x77, false),
        (0x78, false),
        (0x2A5, true),
    ]
    .iter()
    .map(|&(value, ten_bit)| {
        let address = if ten_bit {
            Address::ten_bit(value).expect("validate the address")
        } else {
            Address::seven_bit(value as u8).expect("validate the address")
        };
        let mut write = [0u8; 2];
        let mut read = [0u8; 2];
        let written = address
            .write_frame(Direction::Write, &mut write)
            .expect("frame the address");
        let readable = address
            .write_frame(Direction::Read, &mut read)
            .expect("frame the address");
        json!({
            "address": value,
            "tenBit": ten_bit,
            "writeFrame": hex(&write[..written]),
            "readFrame": hex(&read[..readable]),
            "frameLen": address.frame_len(),
            "reserved": address.is_reserved(),
            "generalCall": address.is_general_call(),
        })
    })
    .collect();

    let modes: Vec<Value> = [Mode::Mode0, Mode::Mode1, Mode::Mode2, Mode::Mode3]
        .iter()
        .map(|&mode| {
            let (cpol, cpha) = mode.cpol_cpha();
            json!({ "mode": mode.number(), "cpol": cpol, "cpha": cpha })
        })
        .collect();

    let transitions = [
        (Level::Low, Level::High),
        (Level::High, Level::Low),
        (Level::High, Level::High),
    ];
    let edges: Vec<Value> = [Edge::Rising, Edge::Falling, Edge::Both]
        .iter()
        .flat_map(|&edge| {
            transitions.iter().map(move |&(from, to)| {
                json!({
                    "edge": edge_name(edge),
                    "from": level_name(from),
                    "to": level_name(to),
                    "triggered": edge.triggered_by(from, to),
                })
            })
        })
        .collect();

    let polarities: Vec<Value> = [Polarity::ActiveHigh, Polarity::ActiveLow]
        .iter()
        .flat_map(|&polarity| {
            [true, false].iter().map(move |&asserted| {
                let level = polarity.level(asserted);
                json!({
                    "polarity": polarity_name(polarity),
                    "asserted": asserted,
                    "level": level_name(level),
                    "isAsserted": polarity.is_asserted(level),
                })
            })
        })
        .collect();

    json!({
        "i2c": addresses,
        "outOfRangeSevenBit": 0x80,
        "outOfRangeTenBit": 0x400,
        "spi": modes,
        "invalidSpiMode": 4,
        "edges": edges,
        "polarities": polarities,
    })
}

/// Names a level, matching the spelling every binding exposes.
fn level_name(level: Level) -> &'static str {
    match level {
        Level::Low => "Low",
        Level::High => "High",
    }
}

/// Names an interrupt edge, matching the spelling every binding exposes.
fn edge_name(edge: Edge) -> &'static str {
    match edge {
        Edge::Rising => "Rising",
        Edge::Falling => "Falling",
        Edge::Both => "Both",
    }
}

/// Names a polarity, matching the spelling every binding exposes.
fn polarity_name(polarity: Polarity) -> &'static str {
    match polarity {
        Polarity::ActiveHigh => "ActiveHigh",
        Polarity::ActiveLow => "ActiveLow",
    }
}

/// The sensor decoders, over register bytes taken from the parts' own datasheets.
fn sensors() -> Value {
    // The compensation coefficients from the Bosch BME280 datasheet's worked
    // example, laid out in the register order a burst read returns them in.
    let mut temp_press = [0u8; 26];
    temp_press[0..2].copy_from_slice(&27_504u16.to_le_bytes());
    temp_press[2..4].copy_from_slice(&26_435i16.to_le_bytes());
    temp_press[4..6].copy_from_slice(&(-1_000i16).to_le_bytes());
    temp_press[6..8].copy_from_slice(&36_477u16.to_le_bytes());
    temp_press[8..10].copy_from_slice(&(-10_685i16).to_le_bytes());
    temp_press[10..12].copy_from_slice(&3_024i16.to_le_bytes());
    temp_press[12..14].copy_from_slice(&2_855i16.to_le_bytes());
    temp_press[14..16].copy_from_slice(&140i16.to_le_bytes());
    temp_press[16..18].copy_from_slice(&(-7i16).to_le_bytes());
    temp_press[18..20].copy_from_slice(&15_500i16.to_le_bytes());
    temp_press[20..22].copy_from_slice(&(-14_600i16).to_le_bytes());
    temp_press[22..24].copy_from_slice(&6_000i16.to_le_bytes());
    temp_press[25] = 75;
    let humidity = [0x64u8, 0x01, 0x00, 0x14, 0x2D, 0x03, 0x1E];

    // The data registers, carrying the datasheet's raw temperature and pressure.
    let measurement = [0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x7F, 0xFF];
    let calibration = bme280::Calibration::from_registers(&temp_press, &humidity);
    let reading = calibration.compensate(&bme280::RawMeasurement::from_registers(&measurement));

    // A DS18B20 reporting 25.0625 C at 12-bit resolution, CRC included.
    let mut scratchpad = [0x91u8, 0x01, 0x4B, 0x46, 0x7F, 0xFF, 0x0C, 0x10, 0x00];
    scratchpad[8] = ds18b20::crc8(&scratchpad[..8]);
    let mut corrupt = scratchpad;
    corrupt[0] ^= 0xFF;
    let decoded = ds18b20::Scratchpad::parse(&scratchpad).expect("parse the scratchpad");

    let resolutions: Vec<Value> = [9u8, 10, 11, 12]
        .iter()
        .map(|&bits| {
            let resolution = match bits {
                9 => ds18b20::Resolution::Bits9,
                10 => ds18b20::Resolution::Bits10,
                11 => ds18b20::Resolution::Bits11,
                _ => ds18b20::Resolution::Bits12,
            };
            json!({
                "bits": bits,
                "configByte": resolution.config_byte(),
                "stepMicroCelsius": resolution.step_micro_celsius(),
                "maxConversionMicros": resolution.max_conversion_micros(),
            })
        })
        .collect();

    // The INA219 datasheet's design example: 15 A across a 2 milliohm shunt.
    const CURRENT_LSB: u32 = 1_000;

    let gains: Vec<Value> = (0u8..8)
        .map(|code| {
            json!({
                "pga": code,
                "fullScaleMicrovolts": ads1115::Pga::from_code(code).full_scale_microvolts(),
                "nanovoltsAtFullScale": ads1115::to_nanovolts(ads1115::Pga::from_code(code), 32_767),
            })
        })
        .collect();
    let rates: Vec<Value> = (0u8..8)
        .map(|code| {
            json!({
                "dataRate": code,
                "samplesPerSecond": ads1115::DataRate::from_code(code).samples_per_second(),
            })
        })
        .collect();
    let reset = ads1115::Config::from_bits(ads1115::CONFIG_RESET);

    // The BMP280 shares the BME280's temperature and pressure compensation, so the same
    // datasheet coefficients and the same raw codes drive both, which lets a reader
    // compare the two blocks directly.
    let bmp_calibration: [u8; 24] = temp_press[..24].try_into().expect("24 coefficient bytes");
    let bmp_measurement: [u8; 6] = measurement[..6].try_into().expect("6 data bytes");
    let bmp = bmp280::Calibration::parse(&bmp_calibration);
    let bmp_reading = bmp.compensate(&bmp280::Measurement::parse(&bmp_measurement));

    // An SHT3x reporting 25 C and 60 %RH: 0x6666 and 0x9999 are exactly 0.4 and 0.6 of
    // full scale, so both conversions land on a round number.
    let sht_measurement = sht3x::Measurement {
        temperature_raw: 0x6666,
        humidity_raw: 0x9999,
    }
    .to_bytes();
    let mut sht_corrupt = sht_measurement;
    sht_corrupt[1] ^= 0x01;
    let sht_decoded = sht3x::Measurement::parse(&sht_measurement).expect("a whole frame");
    let sht_status = sht3x::Status::from_bits(sht3x::Status::DEFAULT).to_bytes();

    // The SCD4x datasheet's own read_measurement example: 500 ppm, 25 C, 37 %RH. Its
    // printed checksum for the first word is the CRC of a word from another table, so
    // the frame is built from the algorithm the same datasheet publishes a check value
    // for.
    let scd_measurement = scd4x::Measurement {
        co2_ppm: 500,
        temperature_raw: 0x6667,
        humidity_raw: 0x5EB9,
    }
    .to_bytes();
    let mut scd_corrupt = scd_measurement;
    scd_corrupt[2] ^= 0xFF;
    let scd_decoded = scd4x::Measurement::parse(&scd_measurement).expect("a whole frame");
    let scd_serial = scd4x::serial_number_frame(273_325_796_834_238);

    // A TMP117 at the rows its temperature table prints.
    let tmp_readings: Vec<Value> = [
        0x8000u16, 0xF380, 0xFFFF, 0x0000, 0x0001, 0x0C80, 0x3200, 0x7FFF,
    ]
    .iter()
    .map(|&word| {
        let raw = word as i16;
        json!({
            "register": word,
            "microCelsius": tmp117::micro_celsius(raw),
            "nanoCelsius": tmp117::nano_celsius(raw),
            "roundTrip": tmp117::raw_from_micro_celsius(tmp117::micro_celsius(raw)),
        })
    })
    .collect();

    // An HDC1080 read in sequence mode: temperature then humidity, two bytes each.
    let hdc_measurement = hdc1080::Measurement {
        temperature: 0x6000,
        humidity: 0x4000,
    }
    .to_bytes();
    let hdc_decoded = hdc1080::Measurement::parse(&hdc_measurement);

    // The OPT3001 result register rows its datasheet works through, and the full-scale
    // range table beside them.
    let lux_rows: Vec<Value> = [0x0001u16, 0x0FFF, 0x3456, 0x789A, 0x8800, 0xB001, 0xBFFF]
        .iter()
        .map(|&word| {
            json!({
                "register": word,
                "milliLux": opt3001::milli_lux(word),
                "roundTrip": opt3001::raw_from_milli_lux(opt3001::milli_lux(word)),
            })
        })
        .collect();
    let lux_ranges: Vec<Value> = (0u8..12)
        .map(|range| {
            json!({
                "range": range,
                "lsbMilliLux": opt3001::lsb_milli_lux(range).expect("a real range"),
                "fullScaleMilliLux": opt3001::full_scale_milli_lux(range).expect("a real range"),
            })
        })
        .collect();

    // The INA226 datasheet's worked example: a 10 A load across a 2 milliohm shunt.
    const INA226_LSB: u32 = 1_000;

    json!({
        "bme280": {
            "calibrationTempPress": hex(&temp_press),
            "calibrationHumidity": hex(&humidity),
            "measurement": hex(&measurement),
            "celsius": reading.celsius(),
            "pascals": reading.pascals(),
            "hectopascals": reading.hectopascals(),
            "relativeHumidityPercent": reading.relative_humidity_percent(),
        },
        "ds18b20": {
            "scratchpad": hex(&scratchpad),
            "corruptScratchpad": hex(&corrupt),
            "rawTemperature": decoded.raw_temperature(),
            "microCelsius": decoded.temperature_micro_celsius(),
            "celsius": decoded.temperature_celsius(),
            "alarmHigh": decoded.alarm_high(),
            "alarmLow": decoded.alarm_low(),
            "resolutionBits": decoded.resolution().bits(),
            "resolutions": resolutions,
            "invalidResolution": 8,
            "crcData": hex(&scratchpad[..8]),
            "crc": ds18b20::crc8(&scratchpad[..8]),
        },
        "ina219": {
            "currentLsbMicroamps": CURRENT_LSB,
            "shuntMilliohms": 2,
            "calibration": ina219::calibration(CURRENT_LSB, 2),
            "maxExpectedMicroamps": 15_000_000u32,
            "minimumCurrentLsbMicroamps": ina219::minimum_current_lsb_microamps(15_000_000),
            "rawShunt": 1_000,
            "shuntMicrovolts": ina219::shunt_microvolts(1_000),
            "rawBus": 0x1F40u16,
            "busMillivolts": ina219::bus_millivolts(0x1F40),
            "conversionReady": ina219::conversion_ready(0x0002),
            "mathOverflow": ina219::math_overflow(0x0001),
            "rawCurrent": 1_000,
            "currentMicroamps": ina219::current_microamps(1_000, CURRENT_LSB),
            "rawPower": 100u16,
            "powerMicrowatts": ina219::power_microwatts(100, CURRENT_LSB),
        },
        "ads1115": {
            "configReset": ads1115::CONFIG_RESET,
            "resetConfig": {
                "startConversion": reset.start_conversion,
                "mux": reset.mux.code(),
                "pga": reset.pga.code(),
                "singleShot": matches!(reset.mode, ads1115::Mode::SingleShot),
                "dataRate": reset.data_rate.code(),
                "windowComparator": matches!(reset.comparator_mode, ads1115::ComparatorMode::Window),
                "comparatorActiveHigh": matches!(
                    reset.comparator_polarity,
                    ads1115::ComparatorPolarity::ActiveHigh
                ),
                "comparatorLatching": matches!(
                    reset.comparator_latch,
                    ads1115::ComparatorLatch::Latching
                ),
                "comparatorQueue": reset.comparator_queue.code(),
            },
            "gains": gains,
            "rates": rates,
        },
        "bmp280": {
            "calibration": hex(&bmp_calibration),
            "measurement": hex(&bmp_measurement),
            "chipId": bmp280::CHIP_ID,
            "resetWord": bmp280::RESET_WORD,
            "celsius": bmp_reading.celsius(),
            "pascals": bmp_reading.pascals(),
            "hectopascals": bmp_reading.hectopascals(),
            "calibrationRoundTrip": hex(&bmp.to_bytes()),
            "skippedOutput": bmp280::SKIPPED_OUTPUT,
        },
        "sht3x": {
            "measurement": hex(&sht_measurement),
            "corruptMeasurement": hex(&sht_corrupt),
            "temperatureRaw": sht_decoded.temperature_raw,
            "humidityRaw": sht_decoded.humidity_raw,
            "milliCelsius": sht_decoded.temperature_milli_celsius(),
            "celsius": sht_decoded.temperature_celsius(),
            "milliFahrenheit": sht_decoded.temperature_milli_fahrenheit(),
            "milliPercent": sht_decoded.humidity_milli_percent(),
            "relativeHumidity": sht_decoded.relative_humidity(),
            "crcInput": hex(&[0xBE, 0xEF]),
            "crc": sht3x::crc(&[0xBE, 0xEF]),
            "status": hex(&sht_status),
            "statusBits": sht3x::Status::DEFAULT,
            "alertPending": sht3x::Status::from_bits(sht3x::Status::DEFAULT).alert_pending(),
            "resetDetected": sht3x::Status::from_bits(sht3x::Status::DEFAULT).reset_detected(),
            "heaterOn": sht3x::Status::from_bits(sht3x::Status::DEFAULT).heater_on(),
            "singleShotHighStretch": sht3x::single_shot(sht3x::Repeatability::High, true),
            "periodicOneMpsHigh": sht3x::periodic(sht3x::Repeatability::High, sht3x::Rate::OneMps),
        },
        "scd4x": {
            "measurement": hex(&scd_measurement),
            "corruptMeasurement": hex(&scd_corrupt),
            "co2Ppm": scd_decoded.co2_ppm,
            "temperatureRaw": scd_decoded.temperature_raw,
            "humidityRaw": scd_decoded.humidity_raw,
            "milliCelsius": scd_decoded.milli_celsius(),
            "celsius": scd_decoded.celsius(),
            "humidityMilliPercent": scd_decoded.humidity_milli_percent(),
            "relativeHumidityPercent": scd_decoded.relative_humidity_percent(),
            "crcInput": hex(&[0xBE, 0xEF]),
            "crc": scd4x::crc(&[0xBE, 0xEF]),
            "temperatureOffsetMilliCelsius": 5_400,
            "temperatureOffsetWord": scd4x::temperature_offset_word(5_400),
            "temperatureOffsetFrame": hex(&scd4x::write_frame(
                scd4x::command::SET_TEMPERATURE_OFFSET,
                scd4x::temperature_offset_word(5_400),
            )),
            "ambientPressurePascals": 98_700,
            "ambientPressureWord": scd4x::ambient_pressure_word(98_700),
            "serialFrame": hex(&scd_serial),
            "serialNumber": scd4x::serial_number(&scd_serial).expect("a whole frame"),
            "dataReadyWords": [
                json!({ "word": 0x8000, "ready": scd4x::data_ready(0x8000) }),
                json!({ "word": 0x0001, "ready": scd4x::data_ready(0x0001) }),
            ],
            "readMeasurement": scd4x::command::READ_MEASUREMENT,
            "readMeasurementDurationMs": scd4x::max_duration_ms(scd4x::command::READ_MEASUREMENT),
        },
        "tmp117": {
            "deviceId": tmp117::DEVICE_ID,
            "configReset": tmp117::CONFIG_RESET,
            "readings": tmp_readings,
            "flagConfig": 0xA220u16,
            "highAlert": tmp117::high_alert(0xA220),
            "lowAlert": tmp117::low_alert(0xA220),
            "dataReady": tmp117::data_ready(0xA220),
        },
        "hdc1080": {
            "measurement": hex(&hdc_measurement),
            "temperatureRaw": hdc_decoded.temperature,
            "humidityRaw": hdc_decoded.humidity,
            "milliCelsius": hdc_decoded.milli_celsius(),
            "celsius": hdc_decoded.celsius(),
            "milliPercent": hdc_decoded.milli_percent(),
            "relativeHumidity": hdc_decoded.relative_humidity(),
            "manufacturerId": hdc1080::MANUFACTURER_ID,
            "deviceId": hdc1080::DEVICE_ID,
            "configurationReset": hdc1080::CONFIGURATION_RESET,
            "invalidConfiguration": 0x1300u16,
            "temperatureRegisterRoundTrip": hdc1080::temperature_register(
                hdc_decoded.milli_celsius(),
            ),
            "humidityRegisterRoundTrip": hdc1080::humidity_register(hdc_decoded.milli_percent()),
        },
        "opt3001": {
            "manufacturerId": opt3001::MANUFACTURER_ID,
            "deviceId": opt3001::DEVICE_ID,
            "configurationReset": opt3001::CONFIGURATION_RESET,
            "results": lux_rows,
            "ranges": lux_ranges,
            "automaticRange": opt3001::RANGE_AUTOMATIC,
            "invalidRange": 12,
        },
        "ina226": {
            "manufacturerId": ina226::MANUFACTURER_ID,
            "deviceId": ina226::DEVICE_ID,
            "configReset": ina226::CONFIG_RESET,
            "currentLsbMicroamps": INA226_LSB,
            "shuntMilliohms": 2,
            "calibration": ina226::calibration(INA226_LSB, 2),
            "maxExpectedMicroamps": 15_000_000u32,
            "minimumCurrentLsbMicroamps": ina226::minimum_current_lsb_microamps(15_000_000),
            "rawShunt": 8_000,
            "shuntNanovolts": ina226::shunt_nanovolts(8_000),
            "rawBus": 9_584u16,
            "busMicrovolts": ina226::bus_microvolts(9_584),
            "rawCurrent": 10_000,
            "currentMicroamps": ina226::current_microamps(10_000, INA226_LSB),
            "rawPower": 4_792u16,
            "powerMicrowatts": ina226::power_microwatts(4_792, INA226_LSB),
            "currentFromShunt": ina226::current_register_from_shunt(
                8_000,
                ina226::calibration(INA226_LSB, 2),
            ),
            "powerFromCurrent": ina226::power_register_from_current(10_000, 9_584),
            "badDieId": 0x2270u16,
        },
    })
}

/// The actuator encoders, and the coil patterns a stepper walks.
fn actuators() -> Value {
    let cycle: Vec<u8> = {
        let mut sequencer = stepper::Sequencer::new(stepper::Drive::HalfStep);
        let mut patterns = vec![sequencer.coils()];
        for _ in 0..stepper::Drive::HalfStep.step_count() {
            patterns.push(sequencer.step(stepper::Direction::Forward));
        }
        patterns
    };

    json!({
        "pca9685": {
            "internalOscHz": pca9685::INTERNAL_OSC_HZ,
            "channels": pca9685::CHANNELS,
            "counts": pca9685::COUNTS,
            "channelRegisters": (0..pca9685::CHANNELS)
                .map(|channel| json!({
                    "channel": channel,
                    "register": pca9685::channel_register(channel),
                }))
                .collect::<Vec<Value>>(),
            "invalidChannel": pca9685::CHANNELS,
            "updateRateHz": 50,
            "prescale": pca9685::prescale_for_frequency(50, pca9685::INTERNAL_OSC_HZ),
            "frequencyForPrescale": pca9685::frequency_for_prescale(
                pca9685::prescale_for_frequency(50, pca9685::INTERNAL_OSC_HZ),
                pca9685::INTERNAL_OSC_HZ,
            ),
        },
        "pwm": {
            "duty": { "off": 2048, "bytes": hex(&pca9685::Pwm::duty(2048).bytes()) },
            "servoCenter": {
                "pulseMicros": 1_500,
                "updateRateHz": 50,
                "bytes": hex(&pca9685::Pwm::servo(1_500, 50).bytes()),
            },
            "fullOn": hex(&pca9685::Pwm::full_on().bytes()),
            "fullOff": hex(&pca9685::Pwm::full_off().bytes()),
        },
        "stepper": {
            "drive": "HalfStep",
            "stepCount": stepper::Drive::HalfStep.step_count(),
            "forwardCycle": cycle,
            "waveStepCount": stepper::Drive::Wave.step_count(),
            "fullStepCount": stepper::Drive::FullStep.step_count(),
            "degrees": 90.0,
            "stepsPerRevolution": 200,
            "stepsForDegrees": stepper::steps_for_degrees(90.0, 200),
        },
    })
}

/// The windowed helpers, at the fixed capacity every binding builds them with.
fn windows() -> Value {
    let readings: Vec<f32> = vec![10.0, 20.0, 30.0, 20.0, 900.0, 20.0];

    let mut window: Window<32> = Window::new();
    let mut window_states: Vec<Value> = Vec::new();
    for &reading in &readings {
        window.push(reading);
        window_states.push(json!({
            "len": window.len(),
            "mean": window.mean(),
            "min": window.min(),
            "max": window.max(),
            "range": window.range(),
        }));
    }

    let mut median: Median<32> = Median::new();
    let medians: Vec<f32> = readings
        .iter()
        .map(|&reading| median.update(reading))
        .collect();

    let rising: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
    let mut trend: Trend<32> = Trend::new();
    let slopes: Vec<Value> = rising
        .iter()
        .map(|&reading| {
            trend.push(reading);
            match trend.slope() {
                Some(slope) => json!(slope),
                None => Value::Null,
            }
        })
        .collect();

    let steady: Vec<f32> = vec![20.0, 20.1, 19.9, 20.0, 20.2, 19.8, 20.1, 20.0];
    let mut anomaly: Anomaly<32> = Anomaly::new(3.0);
    let flags: Vec<bool> = steady
        .iter()
        .chain(core::iter::once(&900.0))
        .map(|&reading| anomaly.check(reading))
        .collect();

    json!({
        "capacity": 32,
        "window": { "readings": readings, "states": window_states },
        "median": { "readings": readings, "outputs": medians },
        "trend": { "readings": rising, "slopes": slopes },
        "anomaly": {
            "sigmas": 3.0,
            "readings": steady.iter().chain(core::iter::once(&900.0)).collect::<Vec<&f32>>(),
            "flags": flags,
        },
    })
}

/// The LoRa link budget: time on air, and the silence a duty cycle then forces.
/// Describes one data rate the way every binding reports it.
///
/// # Arguments
///
/// * `rate` - the data rate, or `None` for a number the region reserves.
///
/// # Returns
///
/// The rate as a JSON object, with the fields its modulation does not use null.
fn data_rate_vector(rate: Option<DataRate>) -> Value {
    let Some(rate) = rate else {
        return json!({
            "kind": "reserved",
            "bitrateBps": 0,
            "bandwidthHz": Value::Null,
            "spreadingFactor": Value::Null,
            "codingRateNumerator": Value::Null,
            "codingRateDenominator": Value::Null,
        });
    };
    match rate.modulation {
        Modulation::LoRa {
            spreading_factor,
            bandwidth_hz,
        } => json!({
            "kind": "lora",
            "bitrateBps": rate.bitrate_bps,
            "bandwidthHz": bandwidth_hz,
            "spreadingFactor": spreading_factor,
            "codingRateNumerator": Value::Null,
            "codingRateDenominator": Value::Null,
        }),
        Modulation::Fsk { .. } => json!({
            "kind": "fsk",
            "bitrateBps": rate.bitrate_bps,
            "bandwidthHz": Value::Null,
            "spreadingFactor": Value::Null,
            "codingRateNumerator": Value::Null,
            "codingRateDenominator": Value::Null,
        }),
        Modulation::LrFhss {
            coding_rate_numerator,
            coding_rate_denominator,
            bandwidth_hz,
        } => json!({
            "kind": "lr_fhss",
            "bitrateBps": rate.bitrate_bps,
            "bandwidthHz": bandwidth_hz,
            "spreadingFactor": Value::Null,
            "codingRateNumerator": coding_rate_numerator,
            "codingRateDenominator": coding_rate_denominator,
        }),
    }
}

/// Describes a payload limit, or null where the plan publishes none.
fn payload_vector(payload: Option<MaxPayload>) -> Value {
    match payload {
        Some(payload) => json!({
            "macPayload": payload.mac_payload,
            "application": payload.application,
        }),
        None => Value::Null,
    }
}

/// Describes what every binding must report about one channel plan.
///
/// The Rust tests check each region cell by cell against the published tables;
/// this pins a slice wide enough to exercise every accessor, so the four
/// languages are held to the same answers.
fn plan_vector(plan: &ChannelPlan<'_>) -> Value {
    let uplink_count = plan.uplink_data_rates.len();
    let fastest = (uplink_count - 1) as u8;
    let probe = plan
        .channel_frequency_hz(0)
        .expect("a plan has a first channel");

    let rx1_row: Vec<Value> = (0..=plan.max_rx1_data_rate_offset)
        .map(|offset| match plan.rx1_data_rate(0, offset) {
            Some(rate) => json!(rate),
            None => Value::Null,
        })
        .collect();

    let channels: Vec<Value> = (0..plan.default_channel_count().min(8))
        .map(|channel| json!(plan.channel_frequency_hz(channel)))
        .collect();

    let sub_bands: Vec<Value> = plan
        .sub_bands
        .iter()
        .map(|band| {
            json!({
                "startHz": band.start_hz,
                "endHz": band.end_hz,
                "dutyCyclePermille": band.duty_cycle_permille,
                "maxEirpDbm": band.max_eirp_dbm,
            })
        })
        .collect();

    json!({
        "name": plan.name,
        "uplinkDataRateCount": uplink_count,
        "downlinkDataRateCount": plan.downlink_data_rates.len(),
        "defaultChannelCount": plan.default_channel_count(),
        "joinChannelBlockCount": plan.join_channels.len(),
        "defaultChannelBlockCount": plan.default_channels.len(),
        "subBandCount": plan.sub_bands.len(),
        "maxRx1DataRateOffset": plan.max_rx1_data_rate_offset,
        "defaultMaxEirpDbm": plan.default_max_eirp_dbm,
        "txPowerStepDb": plan.tx_power_step_db,
        "maxTxPowerIndex": plan.max_tx_power_index,
        "hasDwellTimeLimit": plan.has_dwell_time_limit,
        "hasDwellLimitedPayloads": plan.max_payload_dwell_limited.is_some(),
        "hasDwellLimitedRx1": plan.rx1_data_rate_offsets_dwell_limited.is_some(),
        "rx2": {
            "frequencyHz": plan.rx2_frequency_hz,
            "dataRate": plan.rx2_data_rate,
        },
        "beacon": {
            "frequencyHz": plan.beacon.frequency_hz,
            "pingSlotFrequencyHz": plan.beacon.ping_slot_frequency_hz,
            "dataRate": plan.beacon.data_rate,
        },
        // The slowest and fastest uplink rates, plus the same numbers read from
        // the downlink table, which the 900 MHz plans number differently.
        "slowestUplink": data_rate_vector(plan.uplink_data_rate(0)),
        "fastestUplink": data_rate_vector(plan.uplink_data_rate(fastest)),
        "slowestDownlink": data_rate_vector(plan.downlink_data_rate(0)),
        "payloadAtSlowest": {
            "repeater": payload_vector(plan.max_payload(0, true)),
            "direct": payload_vector(plan.max_payload(0, false)),
        },
        "payloadAtFastest": {
            "repeater": payload_vector(plan.max_payload(fastest, true)),
            "direct": payload_vector(plan.max_payload(fastest, false)),
        },
        "dwellLimitedAtSlowest": payload_vector(plan.max_payload_dwell_limited(0)),
        // Probed at the plan's own first channel, so the frequency is one the
        // band actually uses.
        "probeFrequencyHz": probe,
        "dutyCyclePermilleAtProbe": plan.duty_cycle_permille(probe),
        "maxEirpDbmAtProbe": plan.max_eirp_dbm(probe),
        "txPowerAtProbe": {
            "index0": plan.tx_power_dbm(0, plan.max_eirp_dbm(probe)),
            "index3": plan.tx_power_dbm(3, plan.max_eirp_dbm(probe)),
        },
        "rx1RowForSlowest": rx1_row,
        "backoffFromFastest": plan.next_backoff_data_rate(fastest),
        "backoffFromSlowest": plan.next_backoff_data_rate(0),
        "channelFrequencies": channels,
        "subBands": sub_bands,
    })
}

/// Vectors for the published channel plans and for one assembled at runtime.
fn lora_regions() -> Value {
    let published: Vec<Value> = Region::all()
        .iter()
        .map(|region| {
            let mut described = plan_vector(region.plan());
            described["code"] = json!(region.code());
            described
        })
        .collect();

    // A private deployment on licensed spectrum, which every binding must be able
    // to assemble and which must answer what a published region does. The empty
    // downlink and back-off tables are filled in by the builder.
    let custom = ChannelPlanBuilder::new("private-915")
        .uplink_data_rate(Some(DataRate::lora(12, 125_000, 250)))
        .uplink_data_rate(Some(DataRate::lora(7, 125_000, 5_470)))
        .max_payload(
            pamoja_lora::region::PayloadTable::UplinkRepeater,
            Some(MaxPayload::new(59, 51)),
        )
        .max_payload(
            pamoja_lora::region::PayloadTable::UplinkRepeater,
            Some(MaxPayload::new(230, 222)),
        )
        .max_payload(
            pamoja_lora::region::PayloadTable::UplinkDirect,
            Some(MaxPayload::new(59, 51)),
        )
        .max_payload(
            pamoja_lora::region::PayloadTable::UplinkDirect,
            Some(MaxPayload::new(230, 222)),
        )
        .default_channel(pamoja_lora::region::ChannelBlock::new(
            915_000_000,
            500_000,
            4,
            0,
            1,
        ))
        .sub_band(SubBand::new(915_000_000, 917_000_000, 1000, 30))
        .power(30, 2, 7)
        .rx(915_000_000, 0, 0)
        .rx1_row(&[0])
        .rx1_row(&[1])
        .build()
        .expect("a consistent private plan");

    json!({
        "published": published,
        "custom": custom.with_plan(plan_vector),
    })
}

/// Vectors for the MAVLink wire layer.
///
/// The frame bytes are pinned exactly, because a wire protocol that is
/// self-consistent but wrong is the failure this suite exists to catch: every
/// binding must put the same bytes on the wire, not merely agree with itself.
fn mavlink() -> Value {
    // HEARTBEAT announcing an onboard controller in an active state.
    let heartbeat: [u8; 9] = [0, 0, 0, 0, 18, 0, 0, 4, 3];
    let header = MavHeader::new(1, 1, 7);

    let v2 = MavFrame::encode_v2(header, 0, &heartbeat, 50).expect("a heartbeat fits");
    let v1 = MavFrame::encode_v1(header, 0, &heartbeat, 50).expect("a heartbeat fits");

    // A message no common dialect defines, with a seed derived from its own
    // definition the way the specification does.
    let private_seed = message_crc_extra("PRIVATE_STATUS", &[("uint32_t", "uptime", 0)]);
    let private_frame = MavFrame::encode_v2(
        MavHeader::new(9, 1, 0),
        50_000,
        &42u32.to_le_bytes(),
        private_seed,
    )
    .expect("a private status fits");

    // Signing is deterministic given the key, link and timestamp, so the signed
    // bytes are pinned too.
    let key = [7u8; signing::KEY_LEN];
    let mut signer = MavSigner::new(key, 1, 1_000);
    let signed = signer.sign(header, 0, &heartbeat, 50).expect("signs");

    // The published CRC_EXTRA of each typed message, re-derived from the
    // registry so a binding cannot quietly disagree about any of them.
    let seeds: Vec<Value> = [
        0u32, 1, 2, 4, 11, 20, 21, 22, 23, 24, 30, 31, 32, 33, 36, 40, 42, 43, 44, 45, 47, 51, 65,
        69, 73, 74, 75, 76, 77, 84, 86, 147, 148, 242, 245, 253,
    ]
    .iter()
    .map(|&msgid| {
        json!({
            "msgid": msgid,
            "crcExtra": mavlink_crc_extra(msgid).expect("a common-dialect id"),
        })
    })
    .collect();

    json!({
        // The catalog check value, plus the frame this suite builds.
        "crc16": [
            { "input": hex("123456789".as_bytes()), "checksum": crc16_mcrf4xx(b"123456789") },
            { "input": hex(&heartbeat), "checksum": crc16_mcrf4xx(&heartbeat) },
        ],
        "knownCrcExtra": seeds,
        "unknownCrcExtra": 9999,
        // Seeds derived from a definition rather than looked up, which is what
        // makes a dialect this build has never seen usable.
        "derivedCrcExtra": [
            {
                "name": "HEARTBEAT",
                "fields": [
                    { "type": "uint32_t", "name": "custom_mode", "arrayLen": 0 },
                    { "type": "uint8_t", "name": "type", "arrayLen": 0 },
                    { "type": "uint8_t", "name": "autopilot", "arrayLen": 0 },
                    { "type": "uint8_t", "name": "base_mode", "arrayLen": 0 },
                    { "type": "uint8_t", "name": "system_status", "arrayLen": 0 },
                    { "type": "uint8_t", "name": "mavlink_version", "arrayLen": 0 },
                ],
                "crcExtra": message_crc_extra(
                    "HEARTBEAT",
                    &[
                        ("uint32_t", "custom_mode", 0),
                        ("uint8_t", "type", 0),
                        ("uint8_t", "autopilot", 0),
                        ("uint8_t", "base_mode", 0),
                        ("uint8_t", "system_status", 0),
                        ("uint8_t", "mavlink_version", 0),
                    ],
                ),
            },
            {
                "name": "PRIVATE_STATUS",
                "fields": [{ "type": "uint32_t", "name": "uptime", "arrayLen": 0 }],
                "crcExtra": private_seed,
            },
        ],
        "header": {
            "systemId": header.system_id,
            "componentId": header.component_id,
            "sequence": header.sequence,
        },
        "payload": hex(&heartbeat),
        "frames": [
            {
                "name": "heartbeat-v2",
                "version": 2,
                "msgid": 0,
                "crcExtra": 50,
                "bytes": hex(v2.as_bytes()),
                "payload": hex(v2.payload()),
                "signed": v2.is_signed(),
                "incompatFlags": v2.incompat_flags(),
            },
            {
                "name": "heartbeat-v1",
                "version": 1,
                "msgid": 0,
                "crcExtra": 50,
                "bytes": hex(v1.as_bytes()),
                "payload": hex(v1.payload()),
                "signed": v1.is_signed(),
                "incompatFlags": v1.incompat_flags(),
            },
            {
                // A four-byte field holding 42 truncates to one byte on the
                // wire, which a decoder zero-extends.
                "name": "private-status-v2",
                "version": 2,
                "msgid": 50_000,
                "crcExtra": private_seed,
                "bytes": hex(private_frame.as_bytes()),
                "payload": hex(private_frame.payload()),
                "signed": private_frame.is_signed(),
                "incompatFlags": private_frame.incompat_flags(),
            },
        ],
        "signed": {
            "key": hex(&key),
            "linkId": 1,
            "timestamp": 1_000,
            "msgid": 0,
            "crcExtra": 50,
            "bytes": hex(signed.as_bytes()),
            "signature": hex(signed.signature().expect("just signed")),
        },
        "timestamps": [
            { "unixMicros": 0u64, "timestamp": signing::timestamp_from_unix_micros(0) },
            {
                "unixMicros": 1_700_000_000_000_000u64,
                "timestamp": signing::timestamp_from_unix_micros(1_700_000_000_000_000),
            },
        ],
    })
}

// The stable code every binding writes a field type as. Kept here rather than read from a
// helper so the vectors pin the numbering itself.
fn field_type_code(ty: MavFieldType) -> u32 {
    match ty {
        MavFieldType::U8 => 1,
        MavFieldType::I8 => 2,
        MavFieldType::Char => 3,
        MavFieldType::U16 => 4,
        MavFieldType::I16 => 5,
        MavFieldType::U32 => 6,
        MavFieldType::I32 => 7,
        MavFieldType::U64 => 8,
        MavFieldType::I64 => 9,
        MavFieldType::F32 => 10,
        MavFieldType::F64 => 11,
    }
}

fn mavlink_schema() -> Value {
    // The shapes of a few typed messages, pinned field by field. Wire order, offsets, and
    // the seed derived from them are the whole contract a peer checks against, so a binding
    // that reorders a field or mistakes a type fails here rather than against a vehicle.
    let described: Vec<Value> = [
        "HEARTBEAT",
        "SYS_STATUS",
        "GLOBAL_POSITION_INT",
        "STATUSTEXT",
    ]
    .iter()
    .map(|name| {
        let shape = descriptor_by_name(name).expect("a typed message");
        let mut offset = 0usize;
        let fields: Vec<Value> = shape
            .fields
            .iter()
            .map(|field| {
                let described = json!({
                    "name": field.name,
                    "typeName": field.ty.wire_name(),
                    "fieldType": field_type_code(field.ty),
                    "arrayLen": field.array_len,
                    "extension": field.extension,
                    "offset": offset,
                });
                offset += field.size();
                described
            })
            .collect();
        json!({
            "name": shape.name,
            "msgid": shape.id,
            "crcExtra": shape.crc_extra,
            "wireLen": shape.wire_len(),
            "baseLen": shape.base_len(),
            "fields": fields,
        })
    })
    .collect();

    // A message described the way its definition reads, in declaration order, which the
    // builder puts on the wire largest first. SYS_STATUS declares an int8 in the middle of
    // its 16-bit fields, so only a stable sort by size lands on the published seed.
    let declared = [
        ("onboard_control_sensors_present", MavFieldType::U32, 0u8),
        ("onboard_control_sensors_enabled", MavFieldType::U32, 0),
        ("onboard_control_sensors_health", MavFieldType::U32, 0),
        ("load", MavFieldType::U16, 0),
        ("voltage_battery", MavFieldType::U16, 0),
        ("current_battery", MavFieldType::I16, 0),
        ("battery_remaining", MavFieldType::I8, 0),
        ("drop_rate_comm", MavFieldType::U16, 0),
        ("errors_comm", MavFieldType::U16, 0),
        ("errors_count1", MavFieldType::U16, 0),
        ("errors_count2", MavFieldType::U16, 0),
        ("errors_count3", MavFieldType::U16, 0),
        ("errors_count4", MavFieldType::U16, 0),
    ];
    let mut builder = MavDescriptorBuilder::new(1, "SYS_STATUS");
    for (name, ty, array_len) in declared {
        builder = builder.field(name, ty, array_len);
    }
    let built = builder.build().expect("a valid shape");
    let built_order: Vec<&str> = built
        .fields()
        .iter()
        .map(|field| field.name.as_str())
        .collect();

    // A private message, to prove the same path works for a dialect this build cannot know.
    let private = MavDescriptorBuilder::new(50_001, "BATTERY_CELLS")
        .field("cell_mv", MavFieldType::U16, 6)
        .field("pack_id", MavFieldType::U8, 0)
        .field("uptime_ms", MavFieldType::U32, 0)
        .build()
        .expect("a valid shape");

    // A message filled in by name, then put on the wire. The payload and frame are pinned
    // exactly: this is where a binding that writes a field to the wrong offset shows up.
    let position = descriptor_by_name("GLOBAL_POSITION_INT").expect("a typed message");
    let mut report = MavDynamicMessage::new(position).expect("it fits a payload");
    report
        .set_uint("time_boot_ms", 0, 30_000)
        .expect("a u32 field");
    report.set_int("lat", 0, -33_856_780).expect("an i32 field");
    report.set_int("lon", 0, 151_215_300).expect("an i32 field");
    report.set_int("alt", 0, 41_000).expect("an i32 field");
    report
        .set_int("relative_alt", 0, 12_500)
        .expect("an i32 field");
    report.set_int("vz", 0, -250).expect("an i16 field");
    report.set_uint("hdg", 0, 18_000).expect("a u16 field");
    let report_frame = report
        .to_frame(MavHeader::new(1, 1, 7))
        .expect("it fits a frame");

    // A char array carries text, padded with zeros, which is how STATUSTEXT is read back.
    let status_shape = descriptor_by_name("STATUSTEXT").expect("a typed message");
    let mut status = MavDynamicMessage::new(status_shape).expect("it fits a payload");
    status.set_uint("severity", 0, 4).expect("a u8 field");
    status
        .set_text("text", "preflight checks passed")
        .expect("a char array");

    json!({
        // The field type codes, which every binding writes out rather than deriving from
        // an enum, so a value means the same thing in every build.
        "fieldTypes": [
            { "name": "uint8_t", "code": 1 },
            { "name": "int8_t", "code": 2 },
            { "name": "char", "code": 3 },
            { "name": "uint16_t", "code": 4 },
            { "name": "int16_t", "code": 5 },
            { "name": "uint32_t", "code": 6 },
            { "name": "int32_t", "code": 7 },
            { "name": "uint64_t", "code": 8 },
            { "name": "int64_t", "code": 9 },
            { "name": "float", "code": 10 },
            { "name": "double", "code": 11 },
        ],
        "messageCount": DESCRIPTORS.len(),
        "unknownMessage": { "msgid": 50_000, "name": "BATTERY_CELLS" },
        "shapes": described,
        "declared": {
            "msgid": 1,
            "name": "SYS_STATUS",
            "fields": declared
                .iter()
                .map(|(name, ty, array_len)| json!({
                    "name": name,
                    "fieldType": field_type_code(*ty),
                    "arrayLen": array_len,
                }))
                .collect::<Vec<Value>>(),
            "wireOrder": built_order,
            "crcExtra": built.crc_extra(),
        },
        "private": {
            "msgid": 50_001,
            "name": "BATTERY_CELLS",
            "fields": [
                { "name": "cell_mv", "fieldType": 4, "arrayLen": 6 },
                { "name": "pack_id", "fieldType": 1, "arrayLen": 0 },
                { "name": "uptime_ms", "fieldType": 6, "arrayLen": 0 },
            ],
            "wireOrder": private
                .fields()
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<&str>>(),
            "crcExtra": private.crc_extra(),
            "wireLen": private.with_descriptor(|shape| shape.wire_len()),
        },
        "filled": {
            "name": "GLOBAL_POSITION_INT",
            "values": [
                { "field": "time_boot_ms", "value": 30_000 },
                { "field": "lat", "value": -33_856_780 },
                { "field": "lon", "value": 151_215_300 },
                { "field": "alt", "value": 41_000 },
                { "field": "relative_alt", "value": 12_500 },
                { "field": "vz", "value": -250 },
                { "field": "hdg", "value": 18_000 },
            ],
            "payload": hex(report.payload()),
            "frame": hex(report_frame.as_bytes()),
        },
        "text": {
            "name": "STATUSTEXT",
            "field": "text",
            "value": "preflight checks passed",
            "severity": 4,
            "payload": hex(status.payload()),
        },
        // A value an integer field cannot hold exactly is refused rather than truncated,
        // which every binding must agree on or the same call succeeds in one and not another.
        "refused": [
            { "field": "hdg", "value": 1.5 },
            { "field": "hdg", "value": -1.0 },
            { "field": "hdg", "value": 65_536.0 },
        ],
    })
}

fn mavlink_protocol() -> Value {
    use pamoja_mavlink::dialect::{mav_cmd, mav_result, CommandAck, MissionRequestInt};
    use pamoja_mavlink::protocol::{
        CommandProtocol, MissionReceiver, MissionSender, ReceiverAction, SenderStep,
    };

    let vehicle = MavHeader::new(1, 1, 0);
    let station = MavHeader::new(255, 190, 0);

    // A two-item plan, given as the fields a caller sets on MISSION_ITEM_INT. The sender
    // stamps the sequence number and target ids itself, so only the content is here.
    let plan_fields = [
        vec![("command", mav_cmd::NAV_TAKEOFF as f64), ("z", 20.0)],
        vec![
            ("command", mav_cmd::NAV_WAYPOINT as f64),
            ("x", -338_567_800.0),
            ("y", 1_512_153_000.0),
            ("z", 50.0),
        ],
    ];
    let item_shape = descriptor_by_name("MISSION_ITEM_INT").expect("a typed message");
    let mut plan_payloads = Vec::new();
    let mut plan = Vec::new();
    for fields in &plan_fields {
        let mut item = MavDynamicMessage::new(item_shape).expect("it fits a payload");
        for (field, value) in fields {
            item.set_number(field, 0, *value).expect("a settable field");
        }
        plan_payloads.push(hex(item.payload()));
        plan.push(MissionItemInt::decode(item.payload()).expect("a whole item"));
    }

    // The whole upload, frame by frame, with each side's exact bytes pinned. The station
    // opens with a request list, the vehicle answers with the count, and the two then take
    // turns until the station acknowledges.
    let upload = MissionSender::new(&plan, 1, 1, 0);
    let mut download = MissionReceiver::new(255, 190, 0);

    let request_list = download.request_list_frame(station).expect("a frame");
    let Some(SenderStep::Reply(count)) = upload.on_frame(&request_list, vehicle).expect("handled")
    else {
        panic!("a request list is answered with the count");
    };

    let mut exchange = Vec::new();
    let mut from_vehicle = count;
    loop {
        let step = download
            .on_frame(&from_vehicle, station)
            .expect("handled")
            .expect("a mission frame");
        let receiver_kind = match step.action {
            ReceiverAction::Request(_) => "request",
            ReceiverAction::Ack(_) => "ack",
        };
        let sender = upload
            .on_frame(&step.reply, vehicle)
            .expect("handled")
            .expect("a mission frame");
        let (sender_kind, sender_reply, sender_result) = match sender {
            SenderStep::Reply(reply) => ("reply", Some(hex(reply.as_bytes())), None),
            SenderStep::Finished(result) => ("finished", None, Some(result)),
        };
        exchange.push(json!({
            "feed": hex(from_vehicle.as_bytes()),
            "receiverKind": receiver_kind,
            "accepted": step.accepted.is_some(),
            "acceptedSeq": step.accepted.map(|item| item.seq),
            "reply": hex(step.reply.as_bytes()),
            "senderKind": sender_kind,
            "senderReply": sender_reply,
            "senderResult": sender_result,
        }));
        match sender {
            SenderStep::Reply(reply) => from_vehicle = reply,
            SenderStep::Finished(_) => break,
        }
    }

    // A request past the end of the plan is refused with the published result.
    let overrun = MissionRequestInt {
        seq: 7,
        target_system: 255,
        target_component: 190,
        mission_type: 0,
    };
    let overrun_frame = mav_encode(vehicle, &overrun).expect("a frame");
    let Some(SenderStep::Reply(refusal)) =
        upload.on_frame(&overrun_frame, station).expect("handled")
    else {
        panic!("a request is answered");
    };

    // The command protocol: one final ack, one for another command, one still in progress.
    let arm = CommandProtocol::new(mav_cmd::COMPONENT_ARM_DISARM, 2);
    let acks = [
        (
            CommandAck {
                command: mav_cmd::COMPONENT_ARM_DISARM,
                result: mav_result::ACCEPTED,
                ..CommandAck::zeroed()
            },
            "final",
            Some(mav_result::ACCEPTED),
        ),
        (
            CommandAck {
                command: mav_cmd::NAV_TAKEOFF,
                result: mav_result::ACCEPTED,
                ..CommandAck::zeroed()
            },
            "unrelated",
            None,
        ),
        (
            CommandAck {
                command: mav_cmd::COMPONENT_ARM_DISARM,
                result: mav_result::IN_PROGRESS,
                progress: 40,
                ..CommandAck::zeroed()
            },
            "inProgress",
            Some(40),
        ),
    ];
    let ack_vectors: Vec<Value> = acks
        .iter()
        .map(|(ack, kind, value)| {
            json!({
                "frame": hex(mav_encode(vehicle, ack).expect("a frame").as_bytes()),
                "kind": kind,
                "value": value,
            })
        })
        .collect();
    let mut retried = arm;
    let timeouts: Vec<Option<u8>> = (0..3).map(|_| retried.on_timeout()).collect();

    // A frame none of the machines handle.
    let heartbeat = pamoja_mavlink::dialect::Heartbeat {
        type_: 2,
        autopilot: 3,
        system_status: 4,
        mavlink_version: 3,
        ..pamoja_mavlink::dialect::Heartbeat::zeroed()
    };
    let ignored = mav_encode(vehicle, &heartbeat).expect("a frame");

    // Setpoints, with the exact frame each constructor puts on the wire.
    let local_position = mav_encode(
        station,
        &pamoja_mavlink::dialect::SetPositionTargetLocalNed::position(
            1_000, 1, 1, 1, 10.0, 0.0, -5.0,
        ),
    )
    .expect("a frame");
    let local_velocity = mav_encode(
        station,
        &pamoja_mavlink::dialect::SetPositionTargetLocalNed::velocity(
            1_000, 1, 1, 1, 0.5, 0.0, 0.0,
        ),
    )
    .expect("a frame");
    let global_position = mav_encode(
        station,
        &pamoja_mavlink::dialect::SetPositionTargetGlobalInt::position(
            1_000,
            6,
            1,
            1,
            -338_567_800,
            1_512_153_000,
            50.0,
        ),
    )
    .expect("a frame");
    let masks: Vec<Value> = [
        (0u32, TypeMask::ignore_all()),
        (1, TypeMask::ignore_all().use_position()),
        (2 | 16, TypeMask::ignore_all().use_velocity().use_yaw_rate()),
        (4 | 32, TypeMask::ignore_all().use_acceleration().force()),
        (
            63,
            TypeMask::ignore_all()
                .use_position()
                .use_velocity()
                .use_acceleration()
                .use_yaw()
                .use_yaw_rate()
                .force(),
        ),
    ]
    .iter()
    .map(|(flags, mask)| json!({ "flags": flags, "mask": mask.bits() }))
    .collect();

    json!({
        "vehicle": { "systemId": 1, "componentId": 1, "sequence": 0 },
        "station": { "systemId": 255, "componentId": 190, "sequence": 0 },
        "plan": plan_fields
            .iter()
            .map(|fields| {
                json!(fields
                    .iter()
                    .map(|(field, value)| json!({ "field": field, "value": value }))
                    .collect::<Vec<Value>>())
            })
            .collect::<Vec<Value>>(),
        "planPayloads": plan_payloads,
        "requestList": hex(request_list.as_bytes()),
        "count": hex(count.as_bytes()),
        "exchange": exchange,
        "overrun": {
            "request": hex(overrun_frame.as_bytes()),
            "reply": hex(refusal.as_bytes()),
            "result": pamoja_mavlink::dialect::mav_mission_result::INVALID_SEQUENCE,
        },
        "command": {
            "command": mav_cmd::COMPONENT_ARM_DISARM,
            "maxRetries": 2,
            "acks": ack_vectors,
            "timeouts": timeouts,
        },
        "ignored": hex(ignored.as_bytes()),
        "offboard": {
            "typeMasks": masks,
            "localPosition": {
                "timeBootMs": 1_000, "coordinateFrame": 1, "targetSystem": 1, "targetComponent": 1,
                "x": 10.0, "y": 0.0, "z": -5.0,
                "frame": hex(local_position.as_bytes()),
            },
            "localVelocity": {
                "timeBootMs": 1_000, "coordinateFrame": 1, "targetSystem": 1, "targetComponent": 1,
                "vx": 0.5, "vy": 0.0, "vz": 0.0,
                "frame": hex(local_velocity.as_bytes()),
            },
            "globalPosition": {
                "timeBootMs": 1_000, "coordinateFrame": 6, "targetSystem": 1, "targetComponent": 1,
                "latInt": -338_567_800, "lonInt": 1_512_153_000, "alt": 50.0,
                "frame": hex(global_position.as_bytes()),
            },
        },
    })
}

fn lora() -> Value {
    // Four settings that between them exercise every field: the slowest and
    // fastest spreading factors, a wider channel, and a link with the header and
    // CRC turned off at the heaviest coding rate.
    let links = [
        (
            "sf12-125k",
            LinkSettings::new(12, 125_000),
            12u8,
            125_000u32,
            5u8,
            8u16,
            true,
            true,
        ),
        (
            "sf7-125k",
            LinkSettings::new(7, 125_000),
            7,
            125_000,
            5,
            8,
            true,
            true,
        ),
        (
            "sf9-250k-cr48",
            LinkSettings::new(9, 250_000)
                .with_coding_rate(8)
                .with_preamble(12),
            9,
            250_000,
            8,
            12,
            true,
            true,
        ),
        (
            "sf10-125k-bare",
            LinkSettings::new(10, 125_000)
                .implicit_header()
                .without_crc(),
            10,
            125_000,
            5,
            8,
            false,
            false,
        ),
    ];

    let described: Vec<Value> = links
        .iter()
        .map(|(name, link, sf, bandwidth, cr, preamble, header, crc)| {
            let airtimes: Vec<Value> = [0usize, 1, 10, 51, 222]
                .iter()
                .map(|&payload_len| {
                    json!({
                        "payloadLen": payload_len,
                        "airtimeUs": link.airtime_us(payload_len),
                    })
                })
                .collect();
            let budgets: Vec<Value> = [(20usize, 10u32), (20, 1), (51, 100)]
                .iter()
                .map(|&(payload_len, permille)| {
                    json!({
                        "payloadLen": payload_len,
                        "permille": permille,
                        "offTimeUs": link.min_off_time_us(payload_len, permille),
                    })
                })
                .collect();
            json!({
                "name": name,
                "spreadingFactor": sf,
                "bandwidthHz": bandwidth,
                "codingRateDenominator": cr,
                "preambleSymbols": preamble,
                "explicitHeader": header,
                "crc": crc,
                "symbolTimeUs": link.symbol_time_us(),
                "airtimes": airtimes,
                "budgets": budgets,
            })
        })
        .collect();

    json!({
        "links": described,
        // A spreading factor outside 5 to 12 is clamped rather than refused. The
        // floor is 5 because RP002-1.0.5 defines SF6 and SF5 data rates.
        "clamped": [
            { "asked": 2, "used": LinkSettings::new(2, 125_000).spreading_factor() },
            { "asked": 15, "used": LinkSettings::new(15, 125_000).spreading_factor() },
        ],
        // A duty cycle of zero forbids transmitting; each binding reports that in
        // its own idiom, so only the inputs are pinned here.
        "forbidden": { "link": "sf12-125k", "payloadLen": 20, "permille": 0 },
        "budget": lora_budget(),
    })
}

/// LoRa link budgets: the noise floor, the demodulator SNR, free-space loss, the first
/// Fresnel zone, budgets from radio to radio, and the 47 CFR 15.247 antenna gain rule.
///
/// Every level is pinned in hundredths of a decibel, the precision the Rust type holds,
/// so a binding that carries decibels as floating point compares after rounding.
fn lora_budget() -> Value {
    let noise_floors: Vec<Value> = [7_810u32, 10_420, 62_500, 125_000, 250_000, 500_000]
        .iter()
        .map(|&bandwidth_hz| {
            json!({
                "bandwidthHz": bandwidth_hz,
                "hundredths": budget::noise_floor_dbm(bandwidth_hz).hundredths(),
            })
        })
        .collect();
    let snrs: Vec<Value> = (4u8..=13)
        .map(|spreading_factor| {
            json!({
                "spreadingFactor": spreading_factor,
                "hundredths": budget::demodulator_snr_db(spreading_factor).hundredths(),
            })
        })
        .collect();
    let losses: Vec<Value> = [
        (0u32, 868_100_000u32),
        (1, 868_100_000),
        (1_000, 868_100_000),
        (5_000, 868_100_000),
        (15_000, 915_000_000),
        (100_000, 433_175_000),
        (u32::MAX, 2_400_000_000),
    ]
    .iter()
    .map(|&(distance_m, frequency_hz)| {
        json!({
            "distanceM": distance_m,
            "frequencyHz": frequency_hz,
            "hundredths": budget::free_space_loss_db(distance_m, frequency_hz).hundredths(),
        })
    })
    .collect();
    let radii: Vec<Value> = [
        (2_500u32, 2_500u32, 868_100_000u32),
        (1_000, 9_000, 915_000_000),
        (50_000, 50_000, 433_175_000),
        (0, 0, 868_100_000),
        (2_500, 2_500, 0),
    ]
    .iter()
    .map(|&(near_m, far_m, frequency_hz)| {
        json!({
            "nearM": near_m,
            "farM": far_m,
            "frequencyHz": frequency_hz,
            "radiusMm": budget::fresnel_radius_mm(near_m, far_m, frequency_hz),
        })
    })
    .collect();

    // A node heard by a gateway through real antennas and cable, and the default budget
    // between isotropic antennas at the fastest 125 kHz rate.
    let whip_to_gateway = LinkBudget {
        transmit_power_dbm: Decibels::from_db(14),
        transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
        transmit_cable_loss_db: Decibels::from_tenths(5),
        receive_antenna_gain_dbi: Decibels::from_db(6),
        receive_cable_loss_db: Decibels::from_tenths(15),
        noise_figure_db: budget::GATEWAY_NOISE_FIGURE_DB,
    };
    let budgets: Vec<Value> = [
        (
            "whip-to-gateway",
            whip_to_gateway,
            "sf12-125k",
            LinkSettings::new(12, 125_000),
            10_520,
            1_600,
        ),
        (
            "isotropic",
            LinkBudget::default(),
            "sf7-125k",
            LinkSettings::new(7, 125_000),
            9_122,
            1_400,
        ),
    ]
    .iter()
    .map(|&(name, described, link_name, link, path_loss, ceiling)| {
        let path = Decibels::from_hundredths(path_loss);
        let ceiling = Decibels::from_hundredths(ceiling);
        json!({
            "name": name,
            "link": link_name,
            "transmitPowerHundredths": described.transmit_power_dbm.hundredths(),
            "transmitAntennaGainHundredths": described.transmit_antenna_gain_dbi.hundredths(),
            "transmitCableLossHundredths": described.transmit_cable_loss_db.hundredths(),
            "receiveAntennaGainHundredths": described.receive_antenna_gain_dbi.hundredths(),
            "receiveCableLossHundredths": described.receive_cable_loss_db.hundredths(),
            "noiseFigureHundredths": described.noise_figure_db.hundredths(),
            "pathLossHundredths": path.hundredths(),
            "ceilingHundredths": ceiling.hundredths(),
            "eirpHundredths": described.eirp_dbm().hundredths(),
            "receivedHundredths": described.received_dbm(path).hundredths(),
            "sensitivityHundredths": described.sensitivity_dbm(link).hundredths(),
            "maxPathLossHundredths": described.max_path_loss_db(link).hundredths(),
            "marginHundredths": described.margin_db(link, path).hundredths(),
            "maxTransmitPowerHundredths": described.max_transmit_power_dbm(ceiling).hundredths(),
        })
    })
    .collect();

    // A null channel count is a digitally modulated system under paragraph (b)(3).
    let fcc: Vec<Value> = [
        (None, 215),
        (None, 900),
        (Some(64u16), 900),
        (Some(49), 0),
        (Some(25), 800),
        (Some(24), 0),
    ]
    .iter()
    .map(|&(channels, gain)| {
        let rule = match channels {
            None => Fcc15247::DigitalModulation,
            Some(channels) => Fcc15247::FrequencyHopping { channels },
        };
        json!({
            "hoppingChannels": channels,
            "antennaGainHundredths": gain,
            "maxConductedHundredths": rule
                .max_conducted_dbm(Decibels::from_hundredths(gain))
                .map(Decibels::hundredths),
        })
    })
    .collect();

    json!({
        "radioNoiseFigureHundredths": budget::RADIO_NOISE_FIGURE_DB.hundredths(),
        "gatewayNoiseFigureHundredths": budget::GATEWAY_NOISE_FIGURE_DB.hundredths(),
        "noiseFloors": noise_floors,
        "demodulatorSnrs": snrs,
        "freeSpaceLosses": losses,
        "fresnelRadii": radii,
        "budgets": budgets,
        "fcc": fcc,
    })
}

/// Mesh framing: the bytes on the air, relaying, and duplicate suppression.
fn mesh() -> Value {
    let payload = b"level=high";
    let unicast = MeshFrame::new(0x0000_0001, 0x0000_0009, 7, payload)
        .expect("build the frame")
        .with_hop_limit(5);
    let broadcast = MeshFrame::broadcast(0x1234_5678, 1, payload).expect("build the frame");

    // Relaying spends a hop; a frame with none left must not be forwarded again.
    let relayed = broadcast
        .relayed()
        .expect("a fresh frame has hops to spend");
    let exhausted = broadcast.with_hop_limit(0);

    // A payload byte flipped after framing must fail the checksum.
    let mut corrupt = broadcast.as_bytes().to_vec();
    corrupt[MeshFrame::HEADER_LEN] ^= 0xFF;

    // The cache answers "new" once and "duplicate" every time after.
    let keys = [(0x42u32, 1u16), (0x42, 1), (0x42, 2), (0x43, 1), (0x42, 1)];
    let mut seen = DynamicSeenCache::new(64);
    let answers: Vec<bool> = keys.iter().map(|&key| seen.record(key)).collect();

    json!({
        "maxFrame": MeshFrame::MAX_LEN,
        "maxPayload": MeshFrame::MAX_PAYLOAD,
        "headerLen": MeshFrame::HEADER_LEN,
        "broadcastAddress": pamoja_mesh::BROADCAST,
        "defaultHopLimit": MeshFrame::DEFAULT_HOP_LIMIT,
        "version": MeshFrame::VERSION,
        "seenCapacity": 64,
        "unicast": {
            "src": unicast.src(),
            "dst": unicast.dst(),
            "id": unicast.id(),
            "payload": hex(payload),
            "hopLimit": unicast.hop_limit(),
            "bytes": hex(unicast.as_bytes()),
        },
        "broadcast": {
            "src": broadcast.src(),
            "id": broadcast.id(),
            "payload": hex(payload),
            "hopLimit": broadcast.hop_limit(),
            "bytes": hex(broadcast.as_bytes()),
        },
        "relayed": {
            "hopLimit": relayed.hop_limit(),
            "bytes": hex(relayed.as_bytes()),
        },
        "exhausted": hex(exhausted.as_bytes()),
        "corrupt": hex(&corrupt),
        "crc": {
            // The published CRC-16/CCITT-FALSE check value, so a binding is held
            // to the standard and not only to this implementation.
            "check": hex(b"123456789"),
            "checkValue": mesh_crc16(b"123456789"),
            "data": hex(payload),
            "value": mesh_crc16(payload),
        },
        "seen": {
            "keys": keys.iter().map(|&(src, id)| json!([src, id])).collect::<Vec<Value>>(),
            "new": answers,
        },
        // A cache sized by the caller evicts at that size rather than a fixed one.
        "sizedSeen": {
            "capacity": 2,
            "keys": [[1, 1], [1, 2], [1, 3]],
            "evicted": [1, 1],
        },
    })
}

/// Cost-aware routing: what a node learns, and what it then does with a packet.
fn routing() -> Value {
    let mut router = DynamicRouter::new(0x01, 64);

    let observations = [
        (0x09u32, 0x05u32, 2u16),
        (0x09, 0x07, 1),
        (0x09, 0x03, 4),
        (0x0A, 0x05, 3),
    ];
    let changed: Vec<bool> = observations
        .iter()
        .map(|&(origin, via, cost)| router.observe(origin, via, cost))
        .collect();

    let decisions: Vec<Value> = [0x01u32, 0x09, 0x0A, 0x20]
        .iter()
        .map(|&dst| decision(&router, dst))
        .collect();

    let route = router.route(0x09).expect("a route to 0x09 was learned");
    let learned = router.len();
    router.forget(0x09);

    json!({
        "capacity": 64,
        "address": router.address(),
        "observations": observations
            .iter()
            .zip(changed.iter())
            .map(|(&(origin, via, cost), &changed)| json!({
                "origin": origin,
                "via": via,
                "cost": cost,
                "changed": changed,
            }))
            .collect::<Vec<Value>>(),
        "learned": learned,
        "route": {
            "dst": route.dst(),
            "nextHop": route.next_hop(),
            "cost": route.cost(),
        },
        "decisions": decisions,
        "afterForgetting": {
            "dst": 0x09,
            "decision": decision(&router, 0x09),
            "learned": router.len(),
        },
        // A table sized by the caller holds exactly what it was asked for.
        "sized": {
            "capacity": 3,
            "offered": 10,
            "learned": sized_table(3, 10),
        },
    })
}

/// Fills a table of `capacity` with `offered` destinations and reports what it kept.
fn sized_table(capacity: usize, offered: u32) -> usize {
    let mut router = DynamicRouter::new(0x01, capacity);
    for node in 0..offered {
        router.observe(node + 0x100, 0x05, 4);
    }
    router.len()
}

/// Names one routing decision, in the spelling every binding exposes.
fn decision(router: &DynamicRouter, dst: u32) -> Value {
    match router.forward(dst) {
        Forward::Deliver => json!({ "dst": dst, "action": "Deliver", "nextHop": Value::Null }),
        Forward::Relay(next_hop) => json!({ "dst": dst, "action": "Relay", "nextHop": next_hop }),
        Forward::Flood => json!({ "dst": dst, "action": "Flood", "nextHop": Value::Null }),
    }
}

/// LoRaWAN framing: the secured bytes on the air, and the join that activates a node.
fn lorawan() -> Value {
    const NWK_SKEY: [u8; 16] = [0x2B; 16];
    const APP_SKEY: [u8; 16] = [0x99; 16];
    const DEV_ADDR: u32 = 0x2601_1BDA;
    const DEV_EUI: [u8; 8] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
    const APP_EUI: [u8; 8] = [0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    const APP_KEY: [u8; 16] = [0xAB; 16];

    let session = Session::new(DEV_ADDR, NWK_SKEY, APP_SKEY);

    let uplink_payload = b"temp=4.8";
    let uplink = session
        .encode_uplink(&Uplink::new(42, 1, uplink_payload).confirmed().with_adr())
        .expect("encode the uplink");

    let fopts = [0x03u8, 0x50, 0x00];
    let downlink_payload = b"ack";
    let downlink = session
        .encode_downlink(
            &Downlink::new(7, 2, downlink_payload)
                .with_ack()
                .with_fpending()
                .with_fopts(&fopts),
        )
        .expect("encode the downlink");

    // The last byte is part of the MIC, so flipping it must fail verification.
    let mut forged = uplink.as_bytes().to_vec();
    let last = forged.len() - 1;
    forged[last] ^= 0xFF;

    let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
    let dev_nonce = 0x1234u16;

    json!({
        "maxFrame": pamoja_lorawan::MAX_FRAME,
        "maxPayload": pamoja_lorawan::MAX_PAYLOAD,
        "devAddr": DEV_ADDR,
        "nwkSKey": hex(&NWK_SKEY),
        "appSKey": hex(&APP_SKEY),
        "uplink": {
            "fcnt": 42,
            "fport": 1,
            "payload": hex(uplink_payload),
            "confirmed": true,
            "adr": true,
            "ack": false,
            "frame": hex(uplink.as_bytes()),
        },
        "downlink": {
            "fcnt": 7,
            "fport": 2,
            "payload": hex(downlink_payload),
            "ack": true,
            "fpending": true,
            "fopts": hex(&fopts),
            "frame": hex(downlink.as_bytes()),
        },
        "forgedUplink": hex(&forged),
        "wrongCounter": 43,
        "join": {
            "devEui": hex(&DEV_EUI),
            "appEui": hex(&APP_EUI),
            "appKey": hex(&APP_KEY),
            "devNonce": dev_nonce,
            "request": hex(device.join_request(dev_nonce).as_bytes()),
            // A join accept the network never signed: every binding must refuse it.
            "forgedAccept": hex(&[0x20u8; 17]),
        },
    })
}

/// What a frame says about itself before any key is involved.
fn header() -> Value {
    let session = Session::new(0x2601_1BDA, [0x2B; 16], [0x99; 16]);
    let uplink = session
        .encode_uplink(&Uplink::new(42, 1, b"temp=4.8").confirmed().with_adr())
        .expect("encode the uplink");
    let fopts = [0x03u8, 0x50, 0x00];
    let downlink = session
        .encode_downlink(
            &Downlink::new(7, 2, b"ack")
                .with_fpending()
                .with_fopts(&fopts),
        )
        .expect("encode the downlink");
    let device = Device::new([0x11; 8], [0x22; 8], [0xAB; 16]);
    let request = device.join_request(0x1234);
    let accept = JoinGrant::new(0x0003_0201, 0x0006_0504, 0x2601_1BDA).accept(&[0xAB; 16], 0x1234);

    json!({
        "frames": [
            described(uplink.as_bytes()),
            described(downlink.as_bytes()),
            described(request.as_bytes()),
            described(accept.as_bytes()),
        ],
        // A frame carrying a message type this crate does not read must be refused.
        "unsupported": hex(&[0xC0u8; 16]),
        "truncated": hex(&[0x40u8, 0x01, 0x02]),
    })
}

/// Reads one frame header into the shape every binding reports.
fn described(bytes: &[u8]) -> Value {
    let header = FrameHeader::parse(bytes).expect("the frame parses");
    json!({
        "frame": hex(bytes),
        "messageType": match header.message_type() {
            pamoja_lorawan::MessageType::JoinRequest => "JoinRequest",
            pamoja_lorawan::MessageType::JoinAccept => "JoinAccept",
            pamoja_lorawan::MessageType::UnconfirmedUp => "UnconfirmedUp",
            pamoja_lorawan::MessageType::ConfirmedUp => "ConfirmedUp",
            pamoja_lorawan::MessageType::UnconfirmedDown => "UnconfirmedDown",
            pamoja_lorawan::MessageType::ConfirmedDown => "ConfirmedDown",
        },
        "isData": header.message_type().is_data(),
        "devAddr": header.dev_addr(),
        "fcnt": header.fcnt(),
        "fport": header.fport(),
        "confirmed": header.confirmed(),
        "adr": header.adr(),
        "ack": header.ack(),
        "fpending": header.fpending(),
        "foptsLen": header.fopts_len(),
        "payloadLen": header.payload_len(),
    })
}

/// The network side of activation, including a join captured from a real network.
fn network() -> Value {
    const APP_KEY: [u8; 16] = [0xAB; 16];
    const DEV_EUI: [u8; 8] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
    const APP_EUI: [u8; 8] = [0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    const DEV_NONCE: u16 = 0x1234;

    let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
    let request = device.join_request(DEV_NONCE);
    let verified = JoinRequest::parse(request.as_bytes(), &APP_KEY).expect("the request verifies");

    let grant = JoinGrant::new(0x0003_0201, 0x0006_0504, 0x2601_1BDA)
        .with_dl_settings(0x00)
        .with_rx_delay(0x01);
    let accept = grant.accept(&APP_KEY, DEV_NONCE);

    // Neither side sends a key, so the proof they agree is that one reads what the
    // other wrote. Every binding replays this exchange and checks the same frame.
    let probe = grant
        .session(&APP_KEY, DEV_NONCE)
        .encode_uplink(&Uplink::new(1, 1, b"joined"))
        .expect("encode with the derived session");

    // A real EU868 join-accept, with the plaintext fields and the session keys an
    // independent implementation derived from it. Pinning a third party's numbers is
    // what stops all four bindings from agreeing on an answer that is wrong together.
    // Published at https://github.com/anthonykirby/lora-packet/issues/10
    const PUBLISHED_KEY: [u8; 16] = [
        0xB6, 0xB5, 0x3F, 0x4A, 0x16, 0x8A, 0x7A, 0x88, 0xBD, 0xF7, 0xEA, 0x13, 0x5C, 0xE9, 0xCF,
        0xCA,
    ];
    const PUBLISHED_NONCE: u16 = 0xCC85;
    let published = JoinGrant::new(0x00E5_063A, 0x0000_0013, 0x2601_2E43)
        .with_dl_settings(0x03)
        .with_rx_delay(0x01)
        .with_cflist([
            0x18, 0x4F, 0x84, 0xE8, 0x56, 0x84, 0xB8, 0x5E, 0x84, 0x88, 0x66, 0x84, 0x58, 0x6E,
            0x84, 0x00,
        ]);
    let published_probe = published
        .session(&PUBLISHED_KEY, PUBLISHED_NONCE)
        .encode_uplink(&Uplink::new(1, 1, b"real"))
        .expect("encode with the derived session");

    json!({
        "appKey": hex(&APP_KEY),
        "devNonce": DEV_NONCE,
        "joinRequest": {
            "frame": hex(request.as_bytes()),
            "devEui": hex(&verified.dev_eui()),
            "appEui": hex(&verified.app_eui()),
            "devNonce": verified.dev_nonce(),
        },
        // A request signed with a different root key must not be trusted.
        "forgedRequest": hex(Device::new(DEV_EUI, APP_EUI, [0x00; 16])
            .join_request(DEV_NONCE)
            .as_bytes()),
        "grant": {
            "appNonce": 0x0003_0201,
            "netId": 0x0006_0504,
            "devAddr": 0x2601_1BDA,
            "dlSettings": 0x00,
            "rxDelay": 0x01,
            "accept": hex(accept.as_bytes()),
            "probe": { "fcnt": 1, "fport": 1, "payload": hex(b"joined"), "frame": hex(probe.as_bytes()) },
        },
        "published": {
            "source": "https://github.com/anthonykirby/lora-packet/issues/10",
            "appKey": hex(&PUBLISHED_KEY),
            "devNonce": PUBLISHED_NONCE,
            "appNonce": 0x00E5_063A,
            "netId": 0x0000_0013,
            "devAddr": 0x2601_2E43,
            "dlSettings": 0x03,
            "rxDelay": 0x01,
            "cflist": "184f84e85684b85e84886684586e8400",
            "accept": "204dd85ae608b87fc4889970b7d2042c9e72959b0057aed6094b16003df12de145",
            "nwkSKey": "2c96f7028184bb0be8aa49275290d4fc",
            "appSKey": "f3a5c8f0232a38c144029c165865802c",
            "probe": {
                "fcnt": 1,
                "fport": 1,
                "payload": hex(b"real"),
                "frame": hex(published_probe.as_bytes()),
            },
        },
    })
}

/// Names a boundary state, matching the spelling every binding exposes.
fn name(boundary: Boundary) -> &'static str {
    match boundary {
        Boundary::Inside => "Inside",
        Boundary::Outside => "Outside",
        Boundary::Exited => "Exited",
        Boundary::Entered => "Entered",
    }
}

/// Renders bytes as lowercase hex, the form every binding can parse.
fn radios() -> Value {
    use sx126x_config::{
        LoraModulation, LoraPacket, PacketType, PowerAmplifier, RampTime, StandbyMode, SyncWord,
        TxPower,
    };
    use sx126x_irq::Irq;
    use sx126x_status::{
        ChipMode, CommandStatus, DeviceErrors, PacketStatus, RxBufferStatus, Status,
    };

    // The same four links the LoRa vectors describe, so a binding rebuilds each from there.
    let links = [
        ("sf12-125k", LinkSettings::new(12, 125_000)),
        ("sf7-125k", LinkSettings::new(7, 125_000)),
        (
            "sf9-250k-cr48",
            LinkSettings::new(9, 250_000)
                .with_coding_rate(8)
                .with_preamble(12),
        ),
        (
            "sf10-125k-bare",
            LinkSettings::new(10, 125_000)
                .implicit_header()
                .without_crc(),
        ),
    ];
    let link = |name: &str| {
        links
            .iter()
            .find(|(named, _)| *named == name)
            .map(|(_, settings)| *settings)
            .expect("a link the LoRa vectors name")
    };
    let amplifier_name = |amplifier: PowerAmplifier| match amplifier {
        PowerAmplifier::LowPower => "low",
        PowerAmplifier::HighPower => "high",
    };

    let frequency_words: Vec<Value> = [
        433_175_000u32,
        470_300_000,
        868_100_000,
        915_000_000,
        923_200_000,
    ]
    .iter()
    .map(|&frequency_hz| {
        json!({
            "frequencyHz": frequency_hz,
            "word": sx126x_config::frequency_word(frequency_hz),
        })
    })
    .collect();
    let timeouts: Vec<Value> = [0u64, 1, 15, 16, 1_000_000, 262_143_000, 300_000_000]
        .iter()
        .map(|&timeout_us| {
            json!({
                "timeoutUs": timeout_us,
                "steps": sx126x_config::timeout_steps(timeout_us),
            })
        })
        .collect();
    let calibrations: Vec<Value> = [
        (430_000_000u32, 440_000_000u32),
        (470_000_000, 510_000_000),
        (863_000_000, 870_000_000),
        (902_000_000, 928_000_000),
        (868_100_000, 868_100_000),
    ]
    .iter()
    .map(|&(low_hz, high_hz)| {
        json!({
            "lowHz": low_hz,
            "highHz": high_hz,
            "codes": hex(&sx126x_config::image_calibration(low_hz, high_hz)),
        })
    })
    .collect();

    // Table 13-21: the high power amplifier keeps its +22 dBm configuration and takes the
    // power in SetTxParams; the low power one switches configuration at +15 dBm.
    let powers: Vec<Value> = [
        (PowerAmplifier::HighPower, 30i8),
        (PowerAmplifier::HighPower, 14),
        (PowerAmplifier::HighPower, -20),
        (PowerAmplifier::LowPower, 15),
        (PowerAmplifier::LowPower, 10),
        (PowerAmplifier::LowPower, -30),
    ]
    .iter()
    .map(|&(amplifier, output_dbm)| {
        let power = TxPower::for_output(amplifier, output_dbm);
        json!({
            "amplifier": amplifier_name(amplifier),
            "outputDbm": output_dbm,
            "paConfig": hex(&power.pa.to_params()),
            "settingDbm": power.setting_dbm,
        })
    })
    .collect();
    let whip = LinkBudget {
        transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
        transmit_cable_loss_db: Decibels::from_hundredths(50),
        ..LinkBudget::default()
    };
    let yagi = LinkBudget {
        transmit_antenna_gain_dbi: Decibels::from_hundredths(900),
        ..LinkBudget::default()
    };
    let ceilings: Vec<Value> = [
        (PowerAmplifier::HighPower, &whip, 1_600),
        (PowerAmplifier::HighPower, &yagi, 3_000),
        (PowerAmplifier::LowPower, &whip, 1_600),
    ]
    .iter()
    .map(|&(amplifier, budget, ceiling)| {
        let power = TxPower::under_ceiling(amplifier, budget, Decibels::from_hundredths(ceiling));
        json!({
            "amplifier": amplifier_name(amplifier),
            "transmitAntennaGainHundredths": budget.transmit_antenna_gain_dbi.hundredths(),
            "transmitCableLossHundredths": budget.transmit_cable_loss_db.hundredths(),
            "ceilingHundredths": ceiling,
            "paConfig": hex(&power.pa.to_params()),
            "settingDbm": power.setting_dbm,
        })
    })
    .collect();

    let tx_events = Irq::TX_DONE | Irq::TIMEOUT;
    let rx_events = Irq::RX_DONE | Irq::TIMEOUT | Irq::CRC_ERROR | Irq::HEADER_ERROR;
    let high_14 = TxPower::for_output(PowerAmplifier::HighPower, 14);
    let tx_timeout_us = link("sf7-125k").airtime_us(10) + 1_000_000;
    let modulations: Vec<Value> = links
        .iter()
        .map(|(name, settings)| {
            let modulation = LoraModulation::from_link(settings).expect("an SX126x bandwidth");
            json!({
                "link": name,
                "bytes": hex(sx126x_command::set_lora_modulation_params(modulation).as_bytes()),
            })
        })
        .collect();
    let packets: Vec<Value> = [
        ("sf7-125k", 10u8, false),
        ("sf10-125k-bare", 20, true),
        ("sf9-250k-cr48", 51, false),
    ]
    .iter()
    .map(|&(name, payload_len, invert_iq)| {
        let packet = LoraPacket::from_link(&link(name), payload_len, invert_iq);
        json!({
            "link": name,
            "payloadLen": payload_len,
            "invertIq": invert_iq,
            "bytes": hex(sx126x_command::set_lora_packet_params(packet).as_bytes()),
        })
    })
    .collect();
    let written = |address: u16, values: &[u8]| {
        let mut bytes = sx126x_command::write_register(address).as_bytes().to_vec();
        bytes.extend_from_slice(values);
        hex(&bytes)
    };
    let mut buffer = sx126x_command::write_buffer(0).as_bytes().to_vec();
    buffer.extend_from_slice(b"hello");
    let irq_params = |events: Irq| {
        json!({
            "irq": events.bits(),
            "dio1": events.bits(),
            "bytes": hex(sx126x_command::set_dio_irq_params(events, events, Irq::NONE, Irq::NONE).as_bytes()),
        })
    };
    let commands = json!({
        "setStandby": hex(sx126x_command::set_standby(StandbyMode::Rc).as_bytes()),
        "setPacketTypeLora": hex(sx126x_command::set_packet_type(PacketType::Lora).as_bytes()),
        "setRfFrequency": {
            "frequencyHz": 868_100_000u32,
            "bytes": hex(sx126x_command::set_rf_frequency(sx126x_config::frequency_word(868_100_000)).as_bytes()),
        },
        "calibrateImage": {
            "lowHz": 863_000_000u32,
            "highHz": 870_000_000u32,
            "bytes": hex(sx126x_command::calibrate_image(sx126x_config::image_calibration(863_000_000, 870_000_000)).as_bytes()),
        },
        "setPaConfig": {
            "amplifier": "high",
            "outputDbm": 14,
            "bytes": hex(sx126x_command::set_pa_config(high_14.pa).as_bytes()),
        },
        "setTxParams": {
            "amplifier": "high",
            "outputDbm": 14,
            "rampUs": 40,
            "bytes": hex(sx126x_command::set_tx_params(high_14.setting_dbm, RampTime::Us40).as_bytes()),
        },
        "setLoraModulationParams": modulations,
        "setLoraPacketParams": packets,
        "setDioIrqParams": [irq_params(tx_events), irq_params(rx_events)],
        "clearIrqStatus": {
            "irq": Irq::ALL.bits(),
            "bytes": hex(sx126x_command::clear_irq_status(Irq::ALL).as_bytes()),
        },
        "setTx": {
            "timeoutUs": tx_timeout_us,
            "bytes": hex(sx126x_command::set_tx(sx126x_config::timeout_steps(tx_timeout_us)).as_bytes()),
        },
        "setRx": {
            "timeoutUs": 2_000_000u64,
            "bytes": hex(sx126x_command::set_rx(sx126x_config::timeout_steps(2_000_000)).as_bytes()),
        },
        "setRxContinuous": hex(sx126x_command::set_rx(sx126x_config::RX_CONTINUOUS).as_bytes()),
        "setSleep": {
            "warmStart": true,
            "bytes": hex(sx126x_command::set_sleep(true, false).as_bytes()),
        },
        "setSyncWord": {
            "syncWord": "public",
            "bytes": written(sx126x_config::register::LORA_SYNC_WORD, &SyncWord::Public.to_bytes()),
        },
        "writeRegister": {
            "address": sx126x_config::register::RX_GAIN,
            "values": hex(&[sx126x_config::RX_GAIN_BOOSTED]),
            "bytes": written(sx126x_config::register::RX_GAIN, &[sx126x_config::RX_GAIN_BOOSTED]),
        },
        "writeBuffer": {
            "offset": 0,
            "payload": hex(b"hello"),
            "bytes": hex(&buffer),
        },
    });

    let queried = |query: sx126x_command::Query| {
        json!({
            "bytes": hex(query.command.as_bytes()),
            "answerLen": query.answer_len,
        })
    };
    let queries = json!({
        "getStatus": queried(sx126x_command::get_status()),
        "getIrqStatus": queried(sx126x_command::get_irq_status()),
        "getRxBufferStatus": queried(sx126x_command::get_rx_buffer_status()),
        "getPacketStatus": queried(sx126x_command::get_packet_status()),
        "getRssiInst": queried(sx126x_command::get_rssi_inst()),
        "getDeviceErrors": queried(sx126x_command::get_device_errors()),
        "readRegister": {
            "address": sx126x_config::register::LORA_SYNC_WORD,
            "length": 2,
            "query": queried(sx126x_command::read_register(sx126x_config::register::LORA_SYNC_WORD, 2)),
        },
        "readBuffer": {
            "offset": 128,
            "length": 3,
            "query": queried(sx126x_command::read_buffer(128, 3)),
        },
    });

    // Table 13-29: the interrupt bits, by the name each binding gives them.
    let flags = [
        ("txDone", Irq::TX_DONE),
        ("rxDone", Irq::RX_DONE),
        ("preambleDetected", Irq::PREAMBLE_DETECTED),
        ("syncWordValid", Irq::SYNC_WORD_VALID),
        ("headerValid", Irq::HEADER_VALID),
        ("headerError", Irq::HEADER_ERROR),
        ("crcError", Irq::CRC_ERROR),
        ("cadDone", Irq::CAD_DONE),
        ("cadDetected", Irq::CAD_DETECTED),
        ("timeout", Irq::TIMEOUT),
        ("lrFhssHop", Irq::LR_FHSS_HOP),
    ];
    let irq_flags: serde_json::Map<String, Value> = flags
        .iter()
        .map(|(name, flag)| ((*name).to_owned(), json!(flag.bits())))
        .collect();
    let irqs: Vec<Value> = [0x0001u16, 0x0002, 0x0042, 0x0200, 0x0262, 0x4000, 0xFFFF]
        .iter()
        .map(|&bits| {
            let irq = Irq::from_bytes(bits.to_be_bytes());
            let set: Vec<&str> = flags
                .iter()
                .filter(|(_, flag)| irq.contains(*flag))
                .map(|(name, _)| *name)
                .collect();
            json!({
                "bytes": hex(&bits.to_be_bytes()),
                "bits": irq.bits(),
                "flags": set,
            })
        })
        .collect();

    let chip_mode = |mode: ChipMode| {
        if mode == ChipMode::StandbyRc {
            "standbyRc"
        } else if mode == ChipMode::StandbyXosc {
            "standbyXosc"
        } else if mode == ChipMode::Fs {
            "fs"
        } else if mode == ChipMode::Rx {
            "rx"
        } else if mode == ChipMode::Tx {
            "tx"
        } else {
            "other"
        }
    };
    let command_status = |status: CommandStatus| {
        if status == CommandStatus::DataAvailable {
            "dataAvailable"
        } else if status == CommandStatus::Timeout {
            "timeout"
        } else if status == CommandStatus::ProcessingError {
            "processingError"
        } else if status == CommandStatus::ExecutionFailure {
            "executionFailure"
        } else if status == CommandStatus::TxDone {
            "txDone"
        } else {
            "other"
        }
    };
    let statuses: Vec<Value> = [0x2Cu8, 0x22, 0x54, 0x6C, 0x38, 0x3A, 0x00]
        .iter()
        .map(|&byte| {
            let status = Status::from_byte(byte);
            json!({
                "byte": byte,
                "chipMode": chip_mode(status.chip_mode),
                "commandStatus": command_status(status.command_status),
                "error": status.is_error(),
            })
        })
        .collect();
    let packet_statuses: Vec<Value> =
        [[0xDBu8, 0xF6, 0xE0], [0x40, 0x1C, 0x42], [0x00, 0x80, 0x00]]
            .iter()
            .map(|&bytes| {
                let status = PacketStatus::from_bytes(bytes);
                json!({
                    "bytes": hex(&bytes),
                    "rssiHundredths": status.rssi_dbm.hundredths(),
                    "snrHundredths": status.snr_db.hundredths(),
                    "signalRssiHundredths": status.signal_rssi_dbm.hundredths(),
                })
            })
            .collect();
    let rx_buffers: Vec<Value> = [[0x03u8, 0x80], [0xFF, 0x00]]
        .iter()
        .map(|&bytes| {
            let status = RxBufferStatus::from_bytes(bytes);
            json!({
                "bytes": hex(&bytes),
                "payloadLen": status.payload_len,
                "start": status.start,
            })
        })
        .collect();
    let rssi: Vec<Value> = [0x00u8, 0x5A, 0xDB]
        .iter()
        .map(|&byte| {
            json!({
                "byte": byte,
                "hundredths": sx126x_status::rssi_inst_dbm(byte).hundredths(),
            })
        })
        .collect();
    let error_flags = [
        ("rc64kCalibration", DeviceErrors::RC64K_CALIBRATION),
        ("rc13mCalibration", DeviceErrors::RC13M_CALIBRATION),
        ("pllCalibration", DeviceErrors::PLL_CALIBRATION),
        ("adcCalibration", DeviceErrors::ADC_CALIBRATION),
        ("imageCalibration", DeviceErrors::IMAGE_CALIBRATION),
        ("xoscStart", DeviceErrors::XOSC_START),
        ("pllLock", DeviceErrors::PLL_LOCK),
        ("paRamp", DeviceErrors::PA_RAMP),
    ];
    let device_errors: Vec<Value> = [[0x00u8, 0x00], [0x00, 0x20], [0x01, 0x44], [0x01, 0x7F]]
        .iter()
        .map(|&bytes| {
            let errors = DeviceErrors::from_bytes(bytes);
            let set: Vec<&str> = error_flags
                .iter()
                .filter(|(_, flag)| errors.contains(*flag))
                .map(|(name, _)| *name)
                .collect();
            json!({
                "bytes": hex(&bytes),
                "bits": errors.bits(),
                "flags": set,
            })
        })
        .collect();

    // A ten-byte reading at SF12 under a 1% limit, and a limit of zero, which never clears.
    let started_us = 5_000_000u64;
    let mut guard = RadioDutyCycle::new(10);
    let airtime_us = guard.transmitted(started_us, &link("sf12-125k"), 10);
    let earliest_us = guard.earliest_us();
    let checks: Vec<Value> = [started_us, earliest_us - 1, earliest_us, earliest_us + 1]
        .iter()
        .map(|&now_us| {
            json!({
                "nowUs": now_us,
                "waitUs": guard.wait_us(now_us),
                "ready": guard.ready(now_us),
            })
        })
        .collect();
    let forbidden = RadioDutyCycle::new(0);
    let duty = json!({
        "permille": 10,
        "link": "sf12-125k",
        "payloadLen": 10,
        "startedUs": started_us,
        "airtimeUs": airtime_us,
        "earliestUs": earliest_us,
        "checks": checks,
        "forbidden": {
            "permille": 0,
            "readyAt": [0u64, earliest_us],
            "ready": forbidden.ready(0) || forbidden.ready(earliest_us),
        },
    });

    json!({
        "frequencyWords": frequency_words,
        "timeouts": timeouts,
        "rxContinuous": sx126x_config::RX_CONTINUOUS,
        "imageCalibrations": calibrations,
        "txPowers": powers,
        "underCeilings": ceilings,
        "syncWords": {
            "public": hex(&SyncWord::Public.to_bytes()),
            "private": hex(&SyncWord::Private.to_bytes()),
        },
        "commands": commands,
        "queries": queries,
        "irqFlags": irq_flags,
        "irqs": irqs,
        "statuses": statuses,
        "packetStatuses": packet_statuses,
        "rxBufferStatuses": rx_buffers,
        "rssiInst": rssi,
        "deviceErrors": device_errors,
        "dutyCycle": duty,
        "sx127x": sx127x_vectors(),
        "llcc68": llcc68_vectors(),
    })
}

fn sx127x_vectors() -> Value {
    use pamoja_lora::budget::{Decibels, LinkBudget};
    use serde_json::Map;
    use sx127x_config::{LoraBandwidth, LoraModulation, PaOutput, TxPower};
    use sx127x_irq::IrqFlags;
    use sx127x_register::Mode;
    use sx127x_status::{ModemStatus, PacketStatus, Port};

    // The same four links the LoRa vectors describe, so a binding rebuilds each from there.
    let links = [
        ("sf12-125k", LinkSettings::new(12, 125_000)),
        ("sf7-125k", LinkSettings::new(7, 125_000)),
        (
            "sf9-250k-cr48",
            LinkSettings::new(9, 250_000)
                .with_coding_rate(8)
                .with_preamble(12),
        ),
        (
            "sf10-125k-bare",
            LinkSettings::new(10, 125_000)
                .implicit_header()
                .without_crc(),
        ),
    ];
    let link = |name: &str| {
        links
            .iter()
            .find(|(named, _)| *named == name)
            .map(|(_, settings)| *settings)
            .expect("a link the LoRa vectors name")
    };
    let output_name = |output: PaOutput| match output {
        PaOutput::Rfo => "rfo",
        PaOutput::PaBoost => "paBoost",
    };
    let power_of = |power: TxPower| {
        json!({
            "paConfig": power.pa_config,
            "paDac": power.pa_dac,
            "ocp": power.ocp,
            "outputDbm": power.output_dbm,
        })
    };

    let registers: Map<String, Value> = [
        ("fifo", sx127x_register::FIFO),
        ("opMode", sx127x_register::OP_MODE),
        ("frfMsb", sx127x_register::FRF_MSB),
        ("frfMid", sx127x_register::FRF_MID),
        ("frfLsb", sx127x_register::FRF_LSB),
        ("paConfig", sx127x_register::PA_CONFIG),
        ("paRamp", sx127x_register::PA_RAMP),
        ("ocp", sx127x_register::OCP),
        ("lna", sx127x_register::LNA),
        ("fifoAddrPtr", sx127x_register::FIFO_ADDR_PTR),
        ("fifoTxBaseAddr", sx127x_register::FIFO_TX_BASE_ADDR),
        ("fifoRxBaseAddr", sx127x_register::FIFO_RX_BASE_ADDR),
        ("fifoRxCurrentAddr", sx127x_register::FIFO_RX_CURRENT_ADDR),
        ("irqFlagsMask", sx127x_register::IRQ_FLAGS_MASK),
        ("irqFlags", sx127x_register::IRQ_FLAGS),
        ("rxNbBytes", sx127x_register::RX_NB_BYTES),
        ("modemStat", sx127x_register::MODEM_STAT),
        ("pktSnrValue", sx127x_register::PKT_SNR_VALUE),
        ("pktRssiValue", sx127x_register::PKT_RSSI_VALUE),
        ("rssiValue", sx127x_register::RSSI_VALUE),
        ("hopChannel", sx127x_register::HOP_CHANNEL),
        ("modemConfig1", sx127x_register::MODEM_CONFIG_1),
        ("modemConfig2", sx127x_register::MODEM_CONFIG_2),
        ("symbTimeoutLsb", sx127x_register::SYMB_TIMEOUT_LSB),
        ("preambleMsb", sx127x_register::PREAMBLE_MSB),
        ("preambleLsb", sx127x_register::PREAMBLE_LSB),
        ("payloadLength", sx127x_register::PAYLOAD_LENGTH),
        ("maxPayloadLength", sx127x_register::MAX_PAYLOAD_LENGTH),
        ("modemConfig3", sx127x_register::MODEM_CONFIG_3),
        ("rssiWideband", sx127x_register::RSSI_WIDEBAND),
        ("ifFreq2", sx127x_register::IF_FREQ_2),
        ("ifFreq1", sx127x_register::IF_FREQ_1),
        ("detectOptimize", sx127x_register::DETECT_OPTIMIZE),
        ("invertIq", sx127x_register::INVERT_IQ),
        ("highBwOptimize1", sx127x_register::HIGH_BW_OPTIMIZE_1),
        ("detectionThreshold", sx127x_register::DETECTION_THRESHOLD),
        ("syncWord", sx127x_register::SYNC_WORD),
        ("highBwOptimize2", sx127x_register::HIGH_BW_OPTIMIZE_2),
        ("invertIq2", sx127x_register::INVERT_IQ_2),
        ("imageCal", sx127x_register::IMAGE_CAL),
        ("dioMapping1", sx127x_register::DIO_MAPPING_1),
        ("dioMapping2", sx127x_register::DIO_MAPPING_2),
        ("version", sx127x_register::VERSION),
        ("tcxo", sx127x_register::TCXO),
        ("paDac", sx127x_register::PA_DAC),
    ]
    .into_iter()
    .map(|(name, address)| (name.to_owned(), json!(address)))
    .collect();
    let constants: Map<String, Value> = [
        ("version", sx127x_register::VERSION_SX1276),
        ("write", sx127x_register::WRITE),
        ("dio0RxDone", sx127x_config::DIO0_RX_DONE),
        ("dio0TxDone", sx127x_config::DIO0_TX_DONE),
        ("dio0CadDone", sx127x_config::DIO0_CAD_DONE),
        ("paDacDefault", sx127x_config::PA_DAC_DEFAULT),
        ("paDacHighPower", sx127x_config::PA_DAC_HIGH_POWER),
        ("imageCalStart", sx127x_config::IMAGE_CAL_START),
        ("imageCalRunning", sx127x_config::IMAGE_CAL_RUNNING),
        ("syncWordPublic", sx127x_config::SyncWord::Public.to_byte()),
        (
            "syncWordPrivate",
            sx127x_config::SyncWord::Private.to_byte(),
        ),
        ("lnaBoosted", sx127x_config::LNA_BOOSTED),
        ("tcxoInputOn", sx127x_config::TCXO_INPUT_ON),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), json!(value)))
    .collect();
    let irq_flags: Map<String, Value> = [
        ("rxTimeout", IrqFlags::RX_TIMEOUT),
        ("rxDone", IrqFlags::RX_DONE),
        ("payloadCrcError", IrqFlags::PAYLOAD_CRC_ERROR),
        ("validHeader", IrqFlags::VALID_HEADER),
        ("txDone", IrqFlags::TX_DONE),
        ("cadDone", IrqFlags::CAD_DONE),
        ("fhssChangeChannel", IrqFlags::FHSS_CHANGE_CHANNEL),
        ("cadDetected", IrqFlags::CAD_DETECTED),
    ]
    .into_iter()
    .map(|(name, flag)| (name.to_owned(), json!(flag.bits())))
    .collect();

    let modes: Vec<Value> = [
        ("sleep", Mode::Sleep),
        ("standby", Mode::Standby),
        ("fsTx", Mode::FsTx),
        ("tx", Mode::Tx),
        ("fsRx", Mode::FsRx),
        ("rxContinuous", Mode::RxContinuous),
        ("rxSingle", Mode::RxSingle),
        ("cad", Mode::Cad),
    ]
    .iter()
    .map(|&(name, mode)| {
        json!({
            "mode": name,
            "code": mode.code(),
            "lora": sx127x_register::lora_op_mode(mode),
            "fsk": sx127x_register::fsk_op_mode(mode),
        })
    })
    .collect();
    let addresses: Vec<Value> = [0x00u8, 0x01, 0x42, 0x4D]
        .iter()
        .map(|&address| {
            json!({
                "address": address,
                "read": sx127x_register::read_address(address),
                "write": sx127x_register::write_address(address),
            })
        })
        .collect();
    let frequency_words: Vec<Value> = [
        137_000_000u32,
        433_175_000,
        868_100_000,
        915_000_000,
        1_020_000_000,
    ]
    .iter()
    .map(|&frequency_hz| {
        json!({
            "frequencyHz": frequency_hz,
            "word": sx127x_config::frequency_word(frequency_hz),
        })
    })
    .collect();

    let modems: Vec<Value> = links
        .iter()
        .map(|(name, settings)| {
            let modulation = LoraModulation::from_link(settings).expect("an SX127x link");
            let symbols = sx127x_config::symbol_timeout(settings, 100_000);
            json!({
                "link": name,
                "frequencyHz": 868_100_000u32,
                "symbolTimeout": symbols,
                "modemConfig1": modulation.modem_config_1(),
                "modemConfig2": modulation.modem_config_2(symbols),
                "modemConfig3": modulation.modem_config_3(),
                "detectionOptimize": modulation.detect_optimize(0),
                "detectionThreshold": modulation.detection_threshold(),
            })
        })
        .collect();
    // SF5 is below the SX127x's range, 500 kHz is not offered in the 169 MHz band, and
    // 203.125 kHz is no bandwidth at all.
    let refusals: Vec<Value> = [
        (5u8, 125_000u32, 868_100_000u32),
        (7, 500_000, 169_400_000),
        (7, 203_125, 868_100_000),
    ]
    .iter()
    .map(|&(spreading_factor, bandwidth_hz, frequency_hz)| {
        let refused = LoraModulation::from_link(&LinkSettings::new(spreading_factor, bandwidth_hz))
            .ok()
            .filter(|modulation| modulation.bandwidth.in_band(frequency_hz))
            .is_none();
        assert!(
            refused,
            "SF{spreading_factor} at {bandwidth_hz} Hz is refused"
        );
        json!({
            "spreadingFactor": spreading_factor,
            "bandwidthHz": bandwidth_hz,
            "frequencyHz": frequency_hz,
        })
    })
    .collect();
    let symbol_timeouts: Vec<Value> = [
        ("sf7-125k", 0u64),
        ("sf7-125k", 100_000),
        ("sf12-125k", 10_000_000),
    ]
    .iter()
    .map(|&(name, timeout_us)| {
        json!({
            "link": name,
            "timeoutUs": timeout_us,
            "symbols": sx127x_config::symbol_timeout(&link(name), timeout_us),
        })
    })
    .collect();

    let powers: Vec<Value> = [
        (PaOutput::PaBoost, 20i8),
        (PaOutput::PaBoost, 18),
        (PaOutput::PaBoost, 17),
        (PaOutput::PaBoost, 0),
        (PaOutput::Rfo, 14),
        (PaOutput::Rfo, 0),
        (PaOutput::Rfo, -9),
        (PaOutput::Rfo, 30),
    ]
    .iter()
    .map(|&(output, requested_dbm)| {
        let mut entry = power_of(TxPower::for_output(output, requested_dbm));
        entry["output"] = json!(output_name(output));
        entry["requestedDbm"] = json!(requested_dbm);
        entry
    })
    .collect();
    let ceilings: Vec<Value> = [
        (PaOutput::PaBoost, 215i32, 50i32, 1600i32),
        (PaOutput::Rfo, 900, 0, 3000),
    ]
    .iter()
    .map(|&(output, gain, loss, ceiling)| {
        let budget = LinkBudget {
            transmit_antenna_gain_dbi: Decibels::from_hundredths(gain),
            transmit_cable_loss_db: Decibels::from_hundredths(loss),
            ..LinkBudget::default()
        };
        let power = TxPower::under_ceiling(output, &budget, Decibels::from_hundredths(ceiling));
        let mut entry = power_of(power);
        entry["output"] = json!(output_name(output));
        entry["transmitAntennaGainHundredths"] = json!(gain);
        entry["transmitCableLossHundredths"] = json!(loss);
        entry["ceilingHundredths"] = json!(ceiling);
        entry
    })
    .collect();
    let ocp: Vec<Value> = [45u16, 100, 120, 130, 140, 240, 300]
        .iter()
        .map(|&milliamps| {
            json!({
                "milliamps": milliamps,
                "register": sx127x_config::ocp_register(milliamps),
            })
        })
        .collect();
    let invert_iq: Vec<Value> = [(false, false), (true, false), (false, true), (true, true)]
        .iter()
        .map(|&(receive, transmit)| {
            json!({
                "receive": receive,
                "transmit": transmit,
                "register": sx127x_config::invert_iq(receive, transmit),
            })
        })
        .collect();
    let invert_iq_2: Vec<Value> = [false, true]
        .iter()
        .map(|&inverted| {
            json!({
                "inverted": inverted,
                "register": sx127x_config::invert_iq_2(inverted),
            })
        })
        .collect();
    let high_bw: Vec<Value> = [
        (500_000u32, 915_000_000u32),
        (500_000, 433_000_000),
        (125_000, 868_100_000),
    ]
    .iter()
    .map(|&(bandwidth_hz, frequency_hz)| {
        let bandwidth = LoraBandwidth::from_hz(bandwidth_hz).expect("an SX127x bandwidth");
        let (optimize_1, optimize_2) = sx127x_config::high_bw_optimize(bandwidth, frequency_hz);
        json!({
            "bandwidthHz": bandwidth_hz,
            "frequencyHz": frequency_hz,
            "optimize1": optimize_1,
            "optimize2": optimize_2,
        })
    })
    .collect();
    let spurious: Vec<Value> = [
        7_813u32, 10_417, 15_625, 20_833, 31_250, 41_667, 62_500, 125_000, 250_000, 500_000,
    ]
    .iter()
    .map(|&bandwidth_hz| {
        let bandwidth = LoraBandwidth::from_hz(bandwidth_hz).expect("an SX127x bandwidth");
        let erratum = sx127x_config::spurious_reception(bandwidth);
        json!({
            "bandwidthHz": bandwidth_hz,
            "automaticIf": erratum.automatic_if,
            "ifFreq2": erratum.if_freq_2,
            "offsetHz": erratum.offset_hz,
        })
    })
    .collect();
    let image_cal: Vec<Value> = [0x82u8, 0x02]
        .iter()
        .map(|&current| {
            json!({
                "current": current,
                "register": sx127x_config::image_cal_start(current),
            })
        })
        .collect();
    let automatic_if: Vec<Value> = [(0xC3u8, false), (0x43, true)]
        .iter()
        .map(|&(current, on)| {
            json!({
                "current": current,
                "on": on,
                "register": sx127x_config::automatic_if(current, on),
            })
        })
        .collect();

    let packet_statuses: Vec<Value> = [
        ([0xF6u8, 0x30], 868_100_000u32),
        ([0x1C, 0x7D], 915_000_000),
        ([0x80, 0x20], 433_175_000),
    ]
    .iter()
    .map(|&(bytes, frequency_hz)| {
        let status = PacketStatus::from_bytes(bytes, Port::for_frequency(frequency_hz));
        json!({
            "bytes": hex(&bytes),
            "frequencyHz": frequency_hz,
            "rssiHundredths": status.rssi_dbm.hundredths(),
            "snrHundredths": status.snr_db.hundredths(),
            "signalRssiHundredths": status.signal_rssi_dbm.hundredths(),
        })
    })
    .collect();
    let rssi: Vec<Value> = [(0x30u8, 868_100_000u32), (0x30, 433_175_000), (0x00, 915_000_000)]
        .iter()
        .map(|&(byte, frequency_hz)| {
            json!({
                "byte": byte,
                "frequencyHz": frequency_hz,
                "hundredths": sx127x_status::rssi_dbm(byte, Port::for_frequency(frequency_hz)).hundredths(),
            })
        })
        .collect();
    let modem_statuses: Vec<Value> = [0x0Fu8, 0x30, 0x90, 0x10, 0x00]
        .iter()
        .map(|&byte| {
            let status = ModemStatus::from_byte(byte);
            json!({
                "byte": byte,
                "codingRateDenominator": status.coding_rate_denominator,
                "clear": status.clear,
                "headerValid": status.header_valid,
                "rxOngoing": status.rx_ongoing,
                "signalSynchronized": status.signal_synchronized,
                "signalDetected": status.signal_detected,
            })
        })
        .collect();

    json!({
        "registers": registers,
        "constants": constants,
        "irqFlags": irq_flags,
        "modes": modes,
        "addresses": addresses,
        "frequencyWords": frequency_words,
        "modems": modems,
        "modemRefusals": refusals,
        "symbolTimeouts": symbol_timeouts,
        "txPowers": powers,
        "underCeilings": ceilings,
        "ocp": ocp,
        "invertIq": invert_iq,
        "invertIq2": invert_iq_2,
        "highBwOptimize": high_bw,
        "spuriousReception": spurious,
        "imageCalStart": image_cal,
        "automaticIf": automatic_if,
        "packetStatuses": packet_statuses,
        "rssi": rssi,
        "modemStatuses": modem_statuses,
    })
}

fn llcc68_vectors() -> Value {
    // The pairs either side of each limit: up to SF9 at 125 kHz, SF10 at 250 kHz, SF11 at
    // 500 kHz, and no bandwidth below 125 kHz.
    let pairs = [
        (9u8, 125_000u32),
        (10, 125_000),
        (11, 125_000),
        (10, 250_000),
        (11, 250_000),
        (11, 500_000),
        (12, 500_000),
        (5, 125_000),
        (7, 62_500),
    ];
    Value::Array(
        pairs
            .iter()
            .map(|&(spreading_factor, bandwidth_hz)| {
                let supported = sx126x_config::LoraModulation::from_link(&LinkSettings::new(
                    spreading_factor,
                    bandwidth_hz,
                ))
                .is_some_and(|modulation| {
                    sx126x_config::llcc68_supports(
                        modulation.spreading_factor,
                        modulation.bandwidth,
                    )
                });
                json!({
                    "spreadingFactor": spreading_factor,
                    "bandwidthHz": bandwidth_hz,
                    "supported": supported,
                })
            })
            .collect(),
    )
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Locates the repository root from this crate's manifest directory.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the examples crate sits under the repository root")
        .to_path_buf()
}

/// A signed, hash-chained log: the records, and what breaks the chain.
fn audit() -> Value {
    let keeper = DeviceIdentity::from_seed(&AUDIT_SEED);
    let public = keeper.public();
    let payloads = ["valve=open", "valve=shut", "valve=open"];

    let mut log = AuditLog::new(keeper.clone());
    let entries: Vec<Entry> = payloads
        .iter()
        .map(|payload| log.append(payload.as_bytes()))
        .collect();

    // The last record with its final byte flipped: the chain must not accept it.
    let mut tampered = entries[2].to_bytes();
    let last = tampered.len() - 1;
    tampered[last] ^= 0xff;

    // A log resumed after a restart writes the same next record as one that never
    // stopped, which is what makes a reboot leave no gap.
    let mut resumed = AuditLog::resume(keeper, &entries[2]);
    let after_reboot = resumed.append(b"valve=shut");

    json!({
        "seed": hex(&AUDIT_SEED),
        "publicKey": hex(&public.to_bytes()),
        "entries": entries
            .iter()
            .zip(payloads)
            .map(|(entry, payload)| json!({
                "payload": payload,
                "index": entry.index(),
                "previous": hex(&entry.previous()),
                "digest": hex(&entry.digest()),
                "signature": hex(&entry.signature().to_bytes()),
                "bytes": hex(&entry.to_bytes()),
            }))
            .collect::<Vec<_>>(),
        "tampered": hex(&tampered),
        "resumed": {
            "payload": "valve=shut",
            "index": after_reboot.index(),
            "bytes": hex(&after_reboot.to_bytes()),
        },
    })
}

/// A secured channel: the agreed keys, the sealed messages, and the refusals.
fn session() -> Value {
    let node = AgreementKey::from_seed(&SESSION_NODE_SEED);
    let gateway = AgreementKey::from_seed(&SESSION_GATEWAY_SEED);
    let aad = b"pump-3";

    let mut uplink =
        SecuredSession::establish(&node, &gateway.public(), &SESSION_SALT, Role::Initiator);

    // Two messages in a row, so the counter advancing is part of the contract.
    let mut first = *b"4.8C";
    let first_header = uplink.seal(&mut first, aad);
    let mut second = *b"4.9C";
    let second_header = uplink.seal(&mut second, aad);

    let mut derived = [0u8; 40];
    pamoja_session::hkdf_sha256(b"salt", b"secret", b"pairing", &mut derived);

    json!({
        "nodeSeed": hex(&SESSION_NODE_SEED),
        "gatewaySeed": hex(&SESSION_GATEWAY_SEED),
        "nodePublicKey": hex(&node.public().to_bytes()),
        "gatewayPublicKey": hex(&gateway.public().to_bytes()),
        "salt": hex(&SESSION_SALT),
        "aad": "pump-3",
        "wrongAad": "pump-4",
        "messages": [
            {
                "plaintext": "4.8C",
                "counter": first_header.counter,
                "tag": hex(&first_header.tag),
                "ciphertext": hex(&first),
            },
            {
                "plaintext": "4.9C",
                "counter": second_header.counter,
                "tag": hex(&second_header.tag),
                "ciphertext": hex(&second),
            },
        ],
        "hmac": {
            "key": "key",
            "message": "message",
            "digest": hex(&pamoja_session::hmac_sha256(b"key", b"message")),
        },
        "hkdf": {
            "salt": "salt",
            "ikm": "secret",
            "info": "pairing",
            "length": 40,
            "output": hex(&derived),
        },
    })
}

/// A signed release: the manifest bytes, the envelope, and the slot lifecycle.
fn update() -> Value {
    let publisher = DeviceIdentity::from_seed(&PUBLISHER_SEED);
    let impostor = DeviceIdentity::from_seed(&IMPOSTOR_SEED);
    let anchor = DeviceIdentity::from_seed(&ANCHOR_SEED);
    let releases = DeviceIdentity::from_seed(&RELEASE_SEED);

    let image = vec![0xa5u8; 600];
    let manifest = Manifest {
        structure_version: pamoja_update::STRUCTURE_VERSION,
        sequence: 2,
        vendor_id: VENDOR_ID,
        class_id: CLASS_ID,
        format: PayloadFormat::Raw,
        storage: 1,
        digest: image_digest(&image),
        size: image.len() as u32,
        expires: 0,
    };

    let mut body = [0u8; pamoja_update::MANIFEST_MAX];
    let body_len = manifest.encode(&mut body).expect("encode the manifest");
    let mut envelope = [0u8; pamoja_update::ENVELOPE_MAX];
    let envelope_len = manifest
        .sign(&publisher, &mut envelope)
        .expect("sign the manifest");
    let mut forged = [0u8; pamoja_update::ENVELOPE_MAX];
    let forged_len = manifest
        .sign(&impostor, &mut forged)
        .expect("sign with the wrong key");

    let delegation = Delegation {
        epoch: 1,
        release_key: releases.public().to_bytes(),
        expires: 0,
    };
    let mut statement = [0u8; pamoja_update::DELEGATION_MAX];
    let statement_len = delegation
        .sign(&anchor, &mut statement)
        .expect("sign the delegation");

    // The whole lifecycle a device runs: stage, boot on trial, confirm.
    let device = UpdateDevice {
        vendor_id: VENDOR_ID,
        class_id: CLASS_ID,
        anchor: publisher.public(),
    };
    let mut updater = Updater::new(device, MemoryStore::new(2, 4096));
    updater
        .provision(0, 1)
        .expect("provision the running image");
    let staged = updater
        .stage(&envelope[..envelope_len], &image)
        .expect("stage the release");
    let boot = updater.on_boot().expect("decide what to run");
    let confirmed = updater.confirm().expect("confirm the release");
    let record = updater.store().record(1).expect("read the slot");

    json!({
        "vendorId": hex(&VENDOR_ID),
        "classId": hex(&CLASS_ID),
        "publisherSeed": hex(&PUBLISHER_SEED),
        "publisherPublicKey": hex(&publisher.public().to_bytes()),
        "impostorSeed": hex(&IMPOSTOR_SEED),
        "anchorSeed": hex(&ANCHOR_SEED),
        "anchorPublicKey": hex(&anchor.public().to_bytes()),
        "releaseSeed": hex(&RELEASE_SEED),
        "releasePublicKey": hex(&releases.public().to_bytes()),
        "manifest": {
            "structureVersion": manifest.structure_version,
            "sequence": manifest.sequence,
            "storage": manifest.storage,
            "digest": hex(&manifest.digest),
            "size": manifest.size,
            "expires": manifest.expires,
            "format": manifest.format as u8,
        },
        "imageByte": 0xa5,
        "imageLen": image.len(),
        "body": hex(&body[..body_len]),
        "envelope": hex(&envelope[..envelope_len]),
        "forgedEnvelope": hex(&forged[..forged_len]),
        "delegation": {
            "epoch": delegation.epoch,
            "releaseKey": hex(&delegation.release_key),
            "expires": delegation.expires,
            "envelope": hex(&statement[..statement_len]),
        },
        "lifecycle": {
            "chunk": 128,
            "staged": staged,
            "boot": boot_name(boot),
            "bootSlot": boot_slot(boot),
            "confirmed": confirmed,
            "state": slot_state_name(record.state),
            "written": record.written,
        },
    })
}

/// The SHA-256 of an image, which is what a manifest promises.
fn image_digest(image: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(image).into()
}

/// Names a boot decision for the bindings that read it back.
fn boot_name(boot: Boot) -> &'static str {
    match boot {
        Boot::Confirmed(_) => "Confirmed",
        Boot::Trying(_) => "Trying",
        Boot::Reverted { .. } => "Reverted",
    }
}

/// The slot a boot decision is about.
fn boot_slot(boot: Boot) -> u8 {
    match boot {
        Boot::Confirmed(slot) | Boot::Trying(slot) => slot,
        Boot::Reverted { failed, .. } => failed,
    }
}

/// Names a slot state for the bindings that read it back.
fn slot_state_name(state: SlotState) -> &'static str {
    match state {
        SlotState::Empty => "Empty",
        SlotState::Receiving => "Receiving",
        SlotState::Staged => "Staged",
        SlotState::Pending => "Pending",
        SlotState::Confirmed => "Confirmed",
        SlotState::Failed => "Failed",
    }
}

/// How a work interval stretches as a battery falls, and what a duty cycle costs.
fn power() -> Value {
    let plan = PowerPlan::new(
        core::time::Duration::from_micros(60_000_000),
        core::time::Duration::from_micros(300_000_000),
        core::time::Duration::from_micros(3_600_000_000),
    );
    let charges = [1.0f32, 0.6, 0.5, 0.49, 0.2, 0.19, 0.0];
    let duty = DutyCycle::from_fraction(core::time::Duration::from_micros(1_000_000), 0.25);

    json!({
        "plan": {
            "activeUs": 60_000_000u64,
            "saverUs": 300_000_000u64,
            "criticalUs": 3_600_000_000u64,
            "saverBelow": plan.saver_below(),
            "criticalBelow": plan.critical_below(),
        },
        "charges": charges,
        "modes": charges
            .iter()
            .map(|&soc| power_mode_name(plan.mode(soc)))
            .collect::<Vec<_>>(),
        "charging": charges
            .iter()
            .map(|&soc| power_mode_name(plan.mode_while_charging(soc, true)))
            .collect::<Vec<_>>(),
        "intervalsUs": charges
            .iter()
            .map(|&soc| plan.interval(soc).as_micros() as u64)
            .collect::<Vec<_>>(),
        "duty": {
            "periodUs": 1_000_000u64,
            "fraction": 0.25,
            "activeUs": duty.active().as_micros() as u64,
            "sleepUs": duty.sleep().as_micros() as u64,
        },
    })
}

/// Names a power mode for the bindings that read it back.
fn power_mode_name(mode: PowerMode) -> &'static str {
    match mode {
        PowerMode::Active => "Active",
        PowerMode::Saver => "Saver",
        PowerMode::Critical => "Critical",
    }
}

/// What a reporter ships and what it drops once the link gets expensive.
fn telemetry() -> Value {
    let levels = [
        TelemetryLevel::Trace,
        TelemetryLevel::Debug,
        TelemetryLevel::Info,
        TelemetryLevel::Warn,
        TelemetryLevel::Error,
        TelemetryLevel::Info,
        TelemetryLevel::Warn,
    ];
    let mut reporter = Reporter::new(TelemetryLevel::Trace);
    reporter.adapt_to(LinkCost::Expensive);
    let shipped: Vec<bool> = levels
        .iter()
        .map(|&level| reporter.record(Event::new(level, "vector")).is_some())
        .collect();
    let snapshot = reporter.snapshot();

    json!({
        "costs": ["Free", "Metered", "Expensive", "Offline"],
        "thresholds": [
            telemetry_level_name(LinkCost::Free.threshold()),
            telemetry_level_name(LinkCost::Metered.threshold()),
            telemetry_level_name(LinkCost::Expensive.threshold()),
            telemetry_level_name(LinkCost::Offline.threshold()),
        ],
        "adaptedTo": "Expensive",
        "levels": levels.iter().map(|&level| telemetry_level_name(level)).collect::<Vec<_>>(),
        "shipped": shipped,
        "snapshot": {
            "trace": snapshot.by_level[TelemetryLevel::Trace as usize],
            "debug": snapshot.by_level[TelemetryLevel::Debug as usize],
            "info": snapshot.by_level[TelemetryLevel::Info as usize],
            "warn": snapshot.by_level[TelemetryLevel::Warn as usize],
            "error": snapshot.by_level[TelemetryLevel::Error as usize],
            "emitted": snapshot.emitted,
            "dropped": snapshot.dropped,
        },
    })
}

/// Names a telemetry level for the bindings that read it back.
fn telemetry_level_name(level: TelemetryLevel) -> &'static str {
    match level {
        TelemetryLevel::Trace => "Trace",
        TelemetryLevel::Debug => "Debug",
        TelemetryLevel::Info => "Info",
        TelemetryLevel::Warn => "Warn",
        TelemetryLevel::Error => "Error",
    }
}

/// What a ladder does with a message as its links come and go.
///
/// The whole sequence runs on one runtime, because a ladder is asynchronous and
/// the vectors have to record what it actually did rather than what it would do.
fn ladder() -> Value {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build a runtime for the ladder vectors");

    runtime.block_on(async {
        let broker = LoopbackBroker::new();
        let mut listener = LoopbackTransport::new(broker.clone());
        listener.connect().await.expect("connect the listener");
        listener.subscribe(LADDER_TOPIC).await.expect("subscribe");

        // With no rung at all, everything is buffered rather than lost.
        let mut offline = TransportLadder::new(BufferStore::new());
        let mut outcomes = Vec::new();
        for payload in LADDER_PAYLOADS {
            let delivery = offline
                .send(LADDER_TOPIC, payload.as_bytes())
                .await
                .expect("send with no rung");
            outcomes.push(delivery_name(delivery));
        }
        let buffered = offline.buffered().await.expect("count the buffer");

        // The link comes back and the buffer drains over it.
        let mut restored = offline.rung(LoopbackTransport::new(broker.clone()));
        restored.connect().await.expect("connect the restored rung");
        let flushed = restored.flush().await.expect("flush the buffer");
        let after_flush = restored.buffered().await.expect("count the buffer");

        // A rung that refuses its next send falls through to the one after it.
        let mut ladder = TransportLadder::new(BufferStore::new())
            .rung(Faulty::new(LoopbackTransport::new(broker.clone()), 1))
            .rung(LoopbackTransport::new(broker.clone()));
        ladder.connect().await.expect("connect the rungs");
        let fell_through = ladder
            .send(LADDER_TOPIC, LADDER_FALLTHROUGH.as_bytes())
            .await
            .expect("send through the ladder");

        json!({
            "topic": LADDER_TOPIC,
            "payloads": LADDER_PAYLOADS,
            "withNoRung": {
                "deliveries": outcomes,
                "buffered": buffered,
            },
            "afterTheLinkReturns": {
                "flushed": flushed,
                "buffered": after_flush,
            },
            "fallthrough": {
                "payload": LADDER_FALLTHROUGH,
                "failuresOnFirstRung": 1,
                "delivery": delivery_name(fell_through),
            },
        })
    })
}

/// Names a delivery outcome for the bindings that read it back.
fn delivery_name(delivery: Delivery) -> &'static str {
    match delivery {
        Delivery::Sent => "Sent",
        Delivery::Buffered => "Buffered",
    }
}

/// What the simulated devices produce, so every binding invents the same run.
fn simulation() -> Value {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build a runtime for the simulation vectors");

    runtime.block_on(async {
        let mut sensor = SimSensor::new(SIM_BASELINE)
            .with_drift(SIM_DRIFT)
            .with_noise(SIM_NOISE)
            .with_seed(SIM_SEED);
        let mut readings = Vec::new();
        for _ in 0..SIM_READS {
            readings.push(sensor.read().await.expect("read the sensor"));
        }

        // Twice around a repeating replay, so the wrap is part of the contract.
        let mut replay = Replay::repeating(SIM_CAPTURE.to_vec());
        let mut replayed = Vec::new();
        for _ in 0..(SIM_CAPTURE.len() * 2) {
            replayed.push(replay.read().await.expect("read the replay"));
        }

        let mut robot = pamoja_sim::SimRobot::new(SIM_DT);
        let mut poses = Vec::new();
        for _ in 0..SIM_STEPS {
            robot
                .apply(pamoja_kit::Twist::new(SIM_VX, 0.0, SIM_OMEGA))
                .await
                .expect("drive the robot");
            let pose = robot.pose();
            poses.push(json!({ "x": pose.x, "y": pose.y, "theta": pose.theta }));
        }

        json!({
            "sensor": {
                "baseline": SIM_BASELINE,
                "driftPerRead": SIM_DRIFT,
                "noise": SIM_NOISE,
                "seed": SIM_SEED,
                "readings": readings,
            },
            "replay": {
                "capture": SIM_CAPTURE,
                "repeating": true,
                "readings": replayed,
            },
            "robot": {
                "dt": SIM_DT,
                "vx": SIM_VX,
                "omega": SIM_OMEGA,
                "poses": poses,
            },
        })
    })
}

/// The RIHS01 hash of `std_msgs/msg/String`, as the ROS 2 documentation prints it.
const CHATTER_HASH: &str =
    "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18";

/// The readings a fridge controller is walked through, in order.
const FRIDGE_READINGS: [f32; 5] = [9.0, 6.0, 5.0, 4.0, 1.0];

/// The readings a well-level controller is walked through, in order.
const WELL_READINGS: [f32; 4] = [80.0, 60.0, 40.0, 20.0];

/// What a profile decides, so every binding reaches the same conclusion.
fn profile() -> Value {
    let fridge = Profile::vaccine_fridge_monitor();
    let mut control = fridge.controller();
    let cold_chain: Vec<Value> = FRIDGE_READINGS
        .iter()
        .map(|reading| reaction_value(*reading, control.evaluate(*reading)))
        .collect();

    let well = Profile::well_level();
    let mut level = well.controller();
    let draining: Vec<Value> = WELL_READINGS
        .iter()
        .map(|reading| reaction_value(*reading, level.evaluate(*reading)))
        .collect();

    let mut observer = Controller::monitor();
    let observed = reaction_value(21.5, observer.evaluate(21.5));

    // A kind the library never shipped: every binding must load it, name it, keep its
    // parameters, and hand back a controller that observes only.
    let custom_manifest = concat!(
        "{ \"name\": \"orchard-frost\", \"topic\": \"orchard/air/temperature\", ",
        "\"control\": { \"kind\": \"frost_guard\", \"warn_below\": 2.0, \"latching\": true, \"zone\": \"north\" }, ",
        "\"power\": { \"active_secs\": 60, \"saver_secs\": 300, \"critical_secs\": 900 } }"
    );
    let custom = Profile::from_json(custom_manifest).expect("a custom kind parses");
    let mut inert = custom.controller();
    let custom_reactions: Vec<Value> = [-4.0, 12.0]
        .iter()
        .map(|reading| reaction_value(*reading, inert.evaluate(*reading)))
        .collect();

    json!({
        "coldChain": {
            "name": fridge.name,
            "topic": fridge.topic,
            "control": control_value(&fridge.control),
            "power": schedule_value(fridge.power),
            "reactions": cold_chain,
        },
        "draining": {
            "name": well.name,
            "control": control_value(&well.control),
            "reactions": draining,
        },
        "observed": observed,
        "custom": {
            "manifest": custom_manifest,
            "name": custom.name,
            "control": control_value(&custom.control),
            "reactions": custom_reactions,
        },
    })
}

/// Flattens a control policy the way each binding exposes it.
fn control_value(spec: &ControlSpec) -> Value {
    match *spec {
        ControlSpec::Setpoint {
            setpoint,
            hysteresis,
            cooling,
            safe_band,
        } => json!({
            "kind": "Setpoint",
            "setpoint": setpoint,
            "hysteresis": hysteresis,
            "cooling": cooling,
            "safeBand": safe_band,
        }),
        ControlSpec::Level { empty, warn_within } => json!({
            "kind": "Level",
            "empty": empty,
            "warnWithin": warn_within,
        }),
        ControlSpec::Surge { rising, limit } => json!({
            "kind": "Surge",
            "rising": rising,
            "limit": limit,
        }),
        ControlSpec::Monitor => json!({ "kind": "Monitor" }),
        ControlSpec::Custom {
            ref kind,
            ref params,
        } => json!({
            "kind": "Custom",
            "customKind": kind,
            "params": params,
        }),
    }
}

/// Flattens a sampling schedule the way each binding exposes it.
fn schedule_value(schedule: PowerSchedule) -> Value {
    json!({
        "activeSecs": schedule.active_secs,
        "saverSecs": schedule.saver_secs,
        "criticalSecs": schedule.critical_secs,
        "saverBelow": schedule.saver_below,
        "criticalBelow": schedule.critical_below,
    })
}

/// Flattens one decision, tagged with the reading that produced it.
fn reaction_value(reading: f32, reaction: Reaction) -> Value {
    let alert = match reaction.alert {
        None => json!({ "kind": "None" }),
        Some(Alert::OutOfRange { reading }) => json!({
            "kind": "OutOfRange",
            "reading": reading,
        }),
        Some(Alert::RunningOut { samples }) => json!({
            "kind": "RunningOut",
            "samples": samples,
        }),
        Some(Alert::ChangingFast { rate }) => json!({
            "kind": "ChangingFast",
            "rate": rate,
        }),
        Some(Alert::Custom { code, value }) => json!({
            "kind": "Custom",
            "code": code,
            "value": value,
        }),
    };
    json!({
        "reading": reading,
        "actuator": reaction.actuator,
        "alert": alert,
    })
}

/// The ROS 2 naming and encoding answers every binding must agree on.
fn ros2() -> Value {
    let hash = TypeHash::parse(CHATTER_HASH).expect("the documented chatter hash");
    let twist = Ros2Twist {
        linear: Vector3::new(1.5, 0.0, 0.0),
        angular: Vector3::new(0.0, 0.0, -0.25),
    };

    let mut writer = CdrWriter::new();
    writer.write_u32(7);
    writer.write_f64(2.5);
    writer.write_i32(-3);

    json!({
        "names": [
            { "name": "/robot1/camera_left/image_raw", "valid": true, "fullyQualified": true },
            { "name": "~/setpoint", "valid": true, "fullyQualified": false },
            { "name": "/2foo", "valid": false, "fullyQualified": false },
            { "name": "/foo/", "valid": false, "fullyQualified": false },
        ],
        "ddsTopics": [
            { "fqn": "/robot1/cmd_vel", "kind": "Topic", "topic": "rt/robot1/cmd_vel" },
            { "fqn": "/add_two_ints", "kind": "ServiceRequest", "topic": "rq/add_two_ints" },
            { "fqn": "/add_two_ints", "kind": "ServiceResponse", "topic": "rr/add_two_ints" },
        ],
        "prefixes": {
            "Topic": EntityKind::Topic.prefix(),
            "ServiceRequest": EntityKind::ServiceRequest.prefix(),
            "ServiceResponse": EntityKind::ServiceResponse.prefix(),
        },
        "mangled": {
            "name": "/robot1/cmd_vel",
            "mangled": percent_mangle("/robot1/cmd_vel"),
        },
        "typeNames": [
            { "rosType": "std_msgs/msg/String", "ddsType": "std_msgs::msg::dds_::String_" },
            { "rosType": "geometry_msgs/msg/Twist", "ddsType": "geometry_msgs::msg::dds_::Twist_" },
        ],
        "typeHash": {
            "text": CHATTER_HASH,
            "digest": hex(&hash.digest()),
        },
        "entityKey": {
            "domainId": 0,
            "fqn": "/chatter",
            "rosType": "std_msgs/msg/String",
            "key": entity_key(0, "/chatter", "std_msgs/msg/String", &hash)
                .expect("the documented entity key"),
        },
        "twist": {
            "linear": [twist.linear.x, twist.linear.y, twist.linear.z],
            "angular": [twist.angular.x, twist.angular.y, twist.angular.z],
            "cdr": hex(&twist.to_cdr()),
        },
        "mixedWidths": {
            "word": 7,
            "double": 2.5,
            "signed": -3,
            "cdr": hex(&writer.into_bytes()),
        },
    })
}

/// The key-expression answers every binding must agree on.
fn zenoh() -> Value {
    json!({
        "expressions": [
            { "key": "fleet/*/battery", "valid": true, "canon": true },
            { "key": "fleet/**/battery", "valid": true, "canon": true },
            { "key": "fleet/**/**/battery", "valid": true, "canon": false },
            { "key": "fleet//battery", "valid": false, "canon": false },
        ],
        "canonized": [
            { "key": "fleet/**/**/battery", "canonical": keyexpr::canonize("fleet/**/**/battery") },
            { "key": "fleet/*/battery", "canonical": keyexpr::canonize("fleet/*/battery") },
        ],
        "matches": [
            { "pattern": "fleet/*/battery", "key": "fleet/n7/battery", "matches": true },
            { "pattern": "fleet/*/battery", "key": "fleet/n7/rack/battery", "matches": false },
            { "pattern": "fleet/**/battery", "key": "fleet/n7/rack/battery", "matches": true },
            { "pattern": "fleet/**", "key": "fleet/n7/battery", "matches": true },
        ],
    })
}

/// The datagrams of the Semtech UDP packet forwarder protocol, each with the fields it
/// carries, so every binding builds the same bytes and reads the same values back.
/// The network side of a site: a join admitted and answered, and an uplink read.
/// The Basics Station messages a session carries, built from a frame a radio heard.
fn station() -> Value {
    use pamoja_gateway::station::{id6, Discovery, Levels, Message, Router};
    use pamoja_gateway::udp::Eui;
    use pamoja_lorawan::{Device, Session, Uplink as LorawanUplink};

    let hex = |bytes: &[u8]| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };

    let router = Eui::from_hex("b827ebfffe010203").expect("sixteen hexadecimal digits");
    let muxs = Eui::new([0; 8]);
    let dev_eui = [0x11u8; 8];
    let app_eui = [0x22u8; 8];
    let app_key = [0x33u8; 16];
    let dev_nonce = 0x0102u16;
    let dev_addr = 0x2601_0001u32;
    let data_rate = 5u8;
    let frequency_hz = 868_100_000u32;
    let levels = Levels {
        rctx: 0,
        xtime: 1_000_000,
        gpstime: None,
        rssi: -35.0,
        snr: 5.1,
    };

    // A station asks the discovery endpoint where its network server is, and is told.
    let asking = Discovery::new(router);
    let answered = Router::accepted(router, muxs, "ws://lns.example.invalid:3001/router");

    // The device asks to join, and the station splits the frame into the fields it reports.
    let device = Device::new(dev_eui, app_eui, app_key);
    let request = device.join_request(dev_nonce);
    let join = Message::heard(request.as_bytes(), data_rate, frequency_hz, levels)
        .expect("a station sends a join request up");

    // Then a reading, encrypted with a session, which stays encrypted as it passes through.
    let session = Session::new(dev_addr, [0x44u8; 16], [0x55u8; 16]);
    let frame = session
        .encode_uplink(&LorawanUplink::new(7, 2, b"21.5"))
        .expect("it fits one frame");
    let uplink = Message::heard(frame.as_bytes(), data_rate, frequency_hz, levels)
        .expect("a station sends a data frame up");

    let (join_eui_read, dev_eui_read, nonce_read, join_mic) = match &join {
        Message::JoinRequest {
            join_eui,
            dev_eui,
            dev_nonce,
            mic,
            ..
        } => (*join_eui, *dev_eui, *dev_nonce, *mic),
        _ => panic!("a join request is read as one"),
    };
    let (addr_read, fcnt_read, port_read, payload_read, uplink_mic) = match &uplink {
        Message::Uplink {
            dev_addr,
            fcnt,
            fport,
            payload,
            mic,
            ..
        } => (*dev_addr, *fcnt, *fport, payload.clone(), *mic),
        _ => panic!("a data frame is read as one"),
    };

    json!({
        "router": router.to_hex(),
        "routerId6": id6(router),
        "muxsId6": id6(muxs),
        "discovery": asking.to_json(),
        "routerAnswer": answered.to_json(),
        "dataRate": data_rate,
        "frequencyHz": frequency_hz,
        "join": {
            "frame": hex(request.as_bytes()),
            "message": join.to_json(),
            "joinEui": join_eui_read.to_hex(),
            "devEui": dev_eui_read.to_hex(),
            "devNonce": nonce_read,
            "mic": join_mic,
        },
        "uplink": {
            "frame": hex(frame.as_bytes()),
            "message": uplink.to_json(),
            "devAddr": addr_read,
            "fcnt": fcnt_read,
            "fport": port_read,
            "payload": hex(&payload_read),
            "mic": uplink_mic,
        },
    })
}

fn gateway_network() -> Value {
    use pamoja_gateway::network::{Event, Network, Registration};
    use pamoja_gateway::udp::Rxpk;
    use pamoja_lora::region::Region;
    use pamoja_lorawan::{Device, Uplink as LorawanUplink};

    let hex = |bytes: &[u8]| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };

    let dev_eui = [0x11u8; 8];
    let app_eui = [0x22u8; 8];
    let app_key = [0x33u8; 16];
    let dev_nonce = 0x0102u16;
    let dev_addr = 0x2601_0001u32;
    let net_id = 0x00_00_2Au32;
    let link = LinkSettings::new(7, 125_000);

    let mut site = Network::new(Region::Eu868.plan(), net_id).with_first_dev_addr(dev_addr);
    site.register(Registration::new(dev_eui, app_eui, app_key));

    // The device asks to join, and the site answers in the join window.
    let device = Device::new(dev_eui, app_eui, app_key);
    let request = device.join_request(dev_nonce);
    let heard =
        Rxpk::new(868_100_000, link, request.as_bytes().to_vec()).with_timestamp_us(1_000_000);
    let Event::Joined { accept, .. } = site.uplink(&heard).expect("the request verifies") else {
        panic!("a join request is admitted");
    };

    // It reads the accept, then sends a reading the site decrypts.
    let session = device
        .accept_join(&accept.payload, dev_nonce)
        .expect("the accept verifies")
        .session();
    let sent = session
        .encode_uplink(&LorawanUplink::new(0, 2, b"21.5"))
        .expect("it fits one frame");
    let carried =
        Rxpk::new(868_100_000, link, sent.as_bytes().to_vec()).with_timestamp_us(9_000_000);
    let Event::Data {
        fcnt,
        fport,
        payload,
        slot,
        ..
    } = site.uplink(&carried).expect("the frame verifies")
    else {
        panic!("a data frame is read");
    };

    let downlink = site
        .answer(dev_addr, slot, 2, b"ok")
        .expect("the session is held");

    // Each half is built on its own, because one literal holding them all is deeper than the
    // json macro unfolds.
    let join = json!({
        "heardAtUs": 1_000_000,
        "request": hex(request.as_bytes()),
        "accept": hex(&accept.payload),
        "timestampUs": accept.timestamp_us.expect("the accept is scheduled"),
        "frequencyHz": accept.frequency_hz,
        "invertPolarity": accept.invert_polarity,
    });
    let uplink = json!({
        "heardAtUs": 9_000_000,
        "frame": hex(sent.as_bytes()),
        "fcnt": fcnt,
        "fport": fport.expect("the frame carries a port"),
        "payload": String::from_utf8(payload).expect("the payload is text"),
        "slotTimestampUs": slot.timestamp_us,
        "slotFrequencyHz": slot.frequency_hz,
    });
    let answer = json!({
        "frame": hex(&downlink.payload),
        "timestampUs": downlink.timestamp_us.expect("the downlink is scheduled"),
        "invertPolarity": downlink.invert_polarity,
    });

    json!({
        "devEui": hex(&dev_eui),
        "appEui": hex(&app_eui),
        "appKey": hex(&app_key),
        "devNonce": dev_nonce,
        "netId": net_id,
        "devAddr": dev_addr,
        "frequencyHz": 868_100_000,
        "spreadingFactor": link.spreading_factor(),
        "bandwidthHz": link.bandwidth_hz(),
        "join": join,
        "uplink": uplink,
        "downlink": answer,
    })
}

fn gateway() -> Value {
    use pamoja_gateway::udp::{Eui, Packet, Rxpk, Stat, TxStatus, Txpk, Uplink};

    let hex = |bytes: &[u8]| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let identifier = "b827ebfffe010203";
    let gateway = Eui::from_hex(identifier).expect("sixteen hexadecimal digits");
    let link = LinkSettings::new(7, 125_000).with_coding_rate(6);

    // One packet heard, forwarded with the levels and timestamps a concentrator reports.
    let heard = Rxpk::new(866_349_812, link, b"TEST_PACKET_1234".to_vec())
        .with_rssi_dbm(-35)
        .with_snr_db(5.1)
        .with_timestamp_us(3_512_348_611)
        .with_received_at(1_364_746_877_528_002)
        .on_channel(2, 0);
    let report = Stat::new()
        .at(1_389_517_168)
        .at_position(46.24, 3.2523, 145)
        .with_counts(2, 2, 2)
        .with_downlinks(2, 2)
        .with_acknowledged_percent(100.0);
    let push = Packet::PushData {
        token: 0x1234,
        gateway,
        uplink: Uplink {
            packets: vec![heard],
            status: Some(report),
        },
    };

    // One packet to transmit, at the timestamp that hits a device's receive window.
    let downlink = Txpk::at(
        3_513_348_611,
        869_525_000,
        LinkSettings::new(9, 125_000),
        b"downlink".to_vec(),
    )
    .with_power_dbm(27)
    .with_inverted_polarity(true)
    .without_crc();
    let pull_resp = Packet::PullResp {
        token: 0x00AB,
        transmit: downlink,
    };

    json!({
        "gateway": identifier,
        "pushData": {
            "token": 0x1234,
            "datagram": hex(&push.to_bytes()),
            "rxpk": {
                "frequencyHz": 866_349_812,
                "spreadingFactor": 7,
                "bandwidthHz": 125_000,
                "codingRateDenominator": 6,
                "rssiDbm": -35,
                "snrDb": 5.1,
                "timestampUs": 3_512_348_611u64,
                "receivedAtUs": 1_364_746_877_528_002u64,
                "channel": 2,
                "payload": "TEST_PACKET_1234",
            },
            "stat": {
                "timeS": 1_389_517_168u64,
                "latitudeDeg": 46.24,
                "longitudeDeg": 3.2523,
                "altitudeM": 145,
                "received": 2,
                "receivedOk": 2,
                "forwarded": 2,
                "acknowledgedPercent": 100.0,
                "downlinks": 2,
                "transmitted": 2,
            },
        },
        "pushAck": hex(&Packet::PushAck { token: 0x1234 }.to_bytes()),
        "pullData": hex(&Packet::PullData { token: 0x0304, gateway }.to_bytes()),
        "pullAck": hex(&Packet::PullAck { token: 0x0304 }.to_bytes()),
        "pullResp": {
            "token": 0x00AB,
            "datagram": hex(&pull_resp.to_bytes()),
            "txpk": {
                "frequencyHz": 869_525_000,
                "spreadingFactor": 9,
                "bandwidthHz": 125_000,
                "timestampUs": 3_513_348_611u64,
                "powerDbm": 27,
                "invertPolarity": true,
                "withoutCrc": true,
                "payload": "downlink",
            },
        },
        "txAck": {
            "token": 0x00AB,
            "status": "COLLISION_PACKET",
            "datagram": hex(&Packet::TxAck {
                token: 0x00AB,
                gateway,
                status: TxStatus::CollisionPacket,
            }
            .to_bytes()),
        },
        "statuses": [
            TxStatus::None.as_str(),
            TxStatus::TooLate.as_str(),
            TxStatus::TooEarly.as_str(),
            TxStatus::CollisionPacket.as_str(),
            TxStatus::CollisionBeacon.as_str(),
            TxStatus::TxFreq.as_str(),
            TxStatus::TxPower.as_str(),
            TxStatus::GpsUnlocked.as_str(),
        ],
    })
}
