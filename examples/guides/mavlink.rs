//! The MAVLink guide example; see docs/guides/mavlink.md.
//!
//! Run: `cargo run -p pamoja-examples --example mavlink`

use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

/// A ground station talking to a vehicle: announce itself and read the vehicle's heartbeat
/// off a noisy link, see commands through to their answers, prove a frame came from who it
/// claims, and move a plan across one item at a time.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_mavlink::dialect::{self, enum_named, mav_autopilot, mav_mode_flag, mav_state};
    use pamoja_mavlink::dialect::{mav_type, Heartbeat, Message};
    use pamoja_mavlink::{Frame, Header, Parser};

    const VEHICLE: u8 = 1;
    const AUTOPILOT: u8 = 1;
    const STATION: u8 = 255;
    const PLANNER: u8 = 190;

    // An enum field travels as a number, and the dialect names each number. Printing the
    // name keeps a reader from looking up what 2 or 81 means.
    let name = |enumeration: &str, value: u64| {
        enum_named(enumeration)
            .and_then(|described| described.entry(value))
            .map_or_else(|| value.to_string(), str::to_owned)
    };

    // Every node broadcasts a heartbeat to say what it is and that it is alive. The frame
    // wraps the payload in a header and a checksum seeded with the message's own value.
    let announce = Heartbeat {
        type_: mav_type::GCS,
        autopilot: mav_autopilot::INVALID,
        system_status: mav_state::ACTIVE,
        mavlink_version: 3,
        ..Default::default()
    };
    let sent = Frame::encode_message(Header::new(STATION, PLANNER, 0), &announce)?;
    println!(
        "sent      {} in {} bytes",
        Heartbeat::NAME,
        sent.as_bytes().len()
    );

    // The vehicle answers with its own heartbeat, which reaches the station behind some
    // noise and a copy with its last byte flipped in flight.
    let vehicle = Heartbeat {
        type_: mav_type::QUADROTOR,
        autopilot: mav_autopilot::ARDUPILOTMEGA,
        base_mode: mav_mode_flag::CUSTOM_MODE_ENABLED
            | mav_mode_flag::STABILIZE_ENABLED
            | mav_mode_flag::MANUAL_INPUT_ENABLED,
        system_status: mav_state::STANDBY,
        mavlink_version: 3,
        ..Default::default()
    };
    let good = Frame::encode_message(Header::new(VEHICLE, AUTOPILOT, 0), &vehicle)?;
    let mut garbled = good.as_bytes().to_vec();
    *garbled.last_mut().expect("a frame byte") ^= 0xFF;
    let delivered = [b"???".as_slice(), &garbled, good.as_bytes()].concat();

    // The parser skips whatever does not start a frame and drops a frame whose checksum
    // fails, so only the good copy comes out.
    let mut parser = Parser::new();
    let frames: Vec<Frame> = delivered
        .iter()
        .filter_map(|&byte| parser.push_byte(byte, &dialect::crc_extra))
        .collect();
    println!(
        "parsed    {} frame out of {} bytes, past the noise and the garbled copy",
        frames.len(),
        delivered.len()
    );
    let heard: Heartbeat = frames[0].decode_message()?;
    println!(
        "heard     {} on {}, in {}",
        name("MAV_TYPE", heard.type_.into()),
        name("MAV_AUTOPILOT", heard.autopilot.into()),
        name("MAV_STATE", heard.system_status.into())
    );

    // The base mode is a bitmask, so it names a set of flags rather than one value.
    let modes = enum_named("MAV_MODE_FLAG").expect("a dialect enumeration");
    let flags: Vec<&str> = modes.names(heard.base_mode.into()).collect();
    println!("flags     {}", flags.join(" | "));
    if heard.base_mode & mav_mode_flag::SAFETY_ARMED == 0 {
        println!("disarmed  MAV_MODE_FLAG_SAFETY_ARMED is not among them");
    }
    // ANCHOR_END: example

    assert_eq!(heard, vehicle);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].message_id(), Heartbeat::ID);

    // ANCHOR: command
    use pamoja_mavlink::dialect::{mav_cmd, mav_result, CommandAck, CommandLong};
    use pamoja_mavlink::protocol::{AckOutcome, CommandProtocol};

    // The vehicle's answers, each naming the command it answers.
    let answer = |command: u16, result: u8, progress: u8| {
        let ack = CommandAck {
            command,
            result,
            progress,
            ..Default::default()
        };
        Frame::encode_message(Header::new(VEHICLE, AUTOPILOT, 0), &ack)
    };

    // A command is not fired and forgotten: the vehicle has to answer, and the sender asks
    // again until it does. Each resend carries the next confirmation number, which is how
    // the vehicle tells a retry from a second, deliberate command.
    let mut arming = CommandProtocol::new(mav_cmd::COMPONENT_ARM_DISARM, 3);
    let arm = CommandLong {
        param1: 1.0,
        target_system: VEHICLE,
        target_component: AUTOPILOT,
        command: arming.command(),
        confirmation: arming.confirmation(),
        ..Default::default()
    };
    Frame::encode_message(Header::new(STATION, PLANNER, 1), &arm)?;
    println!(
        "sent      {}, confirmation {}",
        name("MAV_CMD", arm.command.into()),
        arm.confirmation
    );
    if let Some(confirmation) = arming.on_timeout() {
        println!("silence   resent with confirmation {confirmation}");
    }

    // An answer names the command it answers, so one for another command leaves this one
    // waiting.
    for (command, result) in [
        (mav_cmd::NAV_TAKEOFF, mav_result::ACCEPTED),
        (mav_cmd::COMPONENT_ARM_DISARM, mav_result::ACCEPTED),
    ] {
        match arming.on_frame(&answer(command, result, 0)?)? {
            Some(AckOutcome::Unrelated) => println!(
                "stray     an answer for {} leaves it waiting",
                name("MAV_CMD", command.into())
            ),
            Some(AckOutcome::Final(result)) => {
                println!("armed     {}", name("MAV_RESULT", result.into()))
            }
            _ => {}
        }
    }

    // A long command reports progress before its final answer, and a refused one says why.
    let calibrating = CommandProtocol::new(mav_cmd::PREFLIGHT_CALIBRATION, 3);
    if let Some(AckOutcome::InProgress(percent)) =
        calibrating.on_frame(&answer(calibrating.command(), mav_result::IN_PROGRESS, 40)?)?
    {
        println!(
            "progress  {} is {percent}% done",
            name("MAV_CMD", calibrating.command().into())
        );
    }
    let changing = CommandProtocol::new(mav_cmd::DO_SET_MODE, 3);
    if let Some(AckOutcome::Final(result)) =
        changing.on_frame(&answer(changing.command(), mav_result::DENIED, 0)?)?
    {
        println!(
            "refused   {} answered {}",
            name("MAV_CMD", changing.command().into()),
            name("MAV_RESULT", result.into())
        );
    }

    // A command nobody answers runs out of retries, and the caller stops asking.
    let mut returning = CommandProtocol::new(mav_cmd::NAV_RETURN_TO_LAUNCH, 3);
    let mut sends = 1;
    while returning.on_timeout().is_some() {
        sends += 1;
    }
    println!(
        "gave up   {} went unanswered {sends} times",
        name("MAV_CMD", returning.command().into())
    );
    // ANCHOR_END: command

    assert_eq!(arming.confirmation(), 1);
    assert_eq!(sends, 4);

    // ANCHOR: signing
    use pamoja_mavlink::signing::{timestamp_from_unix_micros, KEY_LEN};
    use pamoja_mavlink::{Signer, Verifier};

    // Both ends share a secret key; replace these filler bytes with your own. The signer
    // stamps each frame with its link id and a timestamp that only moves forward.
    let key = [7u8; KEY_LEN];
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as u64;
    let mut signer = Signer::new(key, 1, timestamp_from_unix_micros(now));
    let signed = signer.sign(
        Header::new(STATION, PLANNER, 2),
        Heartbeat::ID,
        sent.payload(),
        Heartbeat::CRC_EXTRA,
    )?;
    let signature = signed.signature().expect("a signed frame").len();
    println!(
        "signed    {} bytes: the {} of the frame, then a {signature}-byte signature",
        signed.as_bytes().len(),
        sent.as_bytes().len()
    );

    // The vehicle checks each frame against the same key, and remembers the newest
    // timestamp from each sender, so a recording played back later is refused.
    let mut verifier = Verifier::new(key);
    if verifier.verify(&signed).is_ok() {
        println!("accepted  the same key, and a timestamp it has not seen");
    }
    if let Err(refusal) = verifier.verify(&signed) {
        println!("replayed  the same frame again is refused: {refusal}");
    }
    let mut stranger = Signer::new([9u8; KEY_LEN], 1, timestamp_from_unix_micros(now));
    let forged = stranger.sign(
        Header::new(STATION, PLANNER, 3),
        Heartbeat::ID,
        sent.payload(),
        Heartbeat::CRC_EXTRA,
    )?;
    if let Err(refusal) = verifier.verify(&forged) {
        println!("forged    another key's frame is refused: {refusal}");
    }
    if let Err(refusal) = verifier.verify(&sent) {
        println!("unsigned  a frame with no signature is refused: {refusal}");
    }
    // ANCHOR_END: signing

    assert!(signed.is_signed());
    assert!(!sent.is_signed());

    // ANCHOR: mission
    use pamoja_mavlink::dialect::{
        mav_frame, mav_mission_type, MissionAck, MissionItemInt, MissionRequestInt,
    };
    use pamoja_mavlink::protocol::{MissionReceiver, MissionSender, ReceiverAction, SenderStep};

    // A plan: take off to 20 m, fly to a point 50 m up, and return to launch. Positions
    // travel as degrees times ten million.
    let (latitude, longitude) = (-33.856_78_f64, 151.215_3_f64);
    let item = |command: u16, x: i32, y: i32, z: f32| MissionItemInt {
        command,
        frame: mav_frame::GLOBAL_RELATIVE_ALT_INT,
        x,
        y,
        z,
        autocontinue: 1,
        ..Default::default()
    };
    let plan = [
        item(mav_cmd::NAV_TAKEOFF, 0, 0, 20.0),
        item(
            mav_cmd::NAV_WAYPOINT,
            (latitude * 1e7).round() as i32,
            (longitude * 1e7).round() as i32,
            50.0,
        ),
        item(mav_cmd::NAV_RETURN_TO_LAUNCH, 0, 0, 0.0),
    ];

    // The station offers the plan, and the vehicle drives the transfer: it asks for each
    // item in turn and acknowledges the last one.
    let station = Header::new(STATION, PLANNER, 0);
    let aboard = Header::new(VEHICLE, AUTOPILOT, 0);
    let upload = MissionSender::new(&plan, VEHICLE, AUTOPILOT, mav_mission_type::MISSION);
    let mut vehicle_side = MissionReceiver::new(STATION, PLANNER, mav_mission_type::MISSION);
    let mut to_vehicle = upload.count_frame(station)?;
    println!("count     the station offers {} items", upload.len());
    let finished = loop {
        let step = vehicle_side
            .on_frame(&to_vehicle, aboard)?
            .expect("a mission frame");
        if let Some(arrived) = step.accepted {
            println!(
                "arrived   item {}, {}",
                arrived.seq,
                name("MAV_CMD", arrived.command.into())
            );
        }
        if let ReceiverAction::Request(_) = step.action {
            println!(
                "request   the vehicle asks for item {}",
                vehicle_side.expected()
            );
        }
        match upload.on_frame(&step.reply, station)? {
            Some(SenderStep::Reply(next)) => to_vehicle = next,
            Some(SenderStep::Finished(result)) => break result,
            None => unreachable!("the vehicle only sends mission frames"),
        }
    };
    println!(
        "done      the vehicle answered {}",
        name("MAV_MISSION_RESULT", finished.into())
    );

    // A request past the end of the plan is answered with a refusal, not with an item.
    let past = MissionRequestInt {
        seq: 7,
        target_system: STATION,
        target_component: PLANNER,
        mission_type: mav_mission_type::MISSION,
    };
    if let Some(SenderStep::Reply(reply)) =
        upload.on_frame(&Frame::encode_message(aboard, &past)?, station)?
    {
        let refused: MissionAck = reply.decode_message()?;
        println!(
            "refused   a request for item {} is answered {}",
            past.seq,
            name("MAV_MISSION_RESULT", refused.type_.into())
        );
    }
    // ANCHOR_END: mission

    assert!(vehicle_side.is_complete());
    assert_eq!(finished, dialect::mav_mission_result::ACCEPTED);
    Ok(())
}
