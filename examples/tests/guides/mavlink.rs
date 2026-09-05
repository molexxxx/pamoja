//! The MAVLink guide example; see docs/guides/mavlink.md.

/// One side of a ground station talking to a vehicle: announce itself, read the vehicle's
/// heartbeat off a noisy link, then arm it and see the command through to its answer.
#[test]
fn a_ground_station_announces_itself_and_arms_a_vehicle() {
    // ANCHOR: example
    use pamoja_mavlink::dialect::{self, mav_autopilot, mav_cmd, mav_result, mav_state, mav_type};
    use pamoja_mavlink::dialect::{CommandAck, CommandLong, Heartbeat, Message};
    use pamoja_mavlink::protocol::{AckOutcome, CommandProtocol};
    use pamoja_mavlink::{Frame, Header, Parser};

    const VEHICLE: u8 = 1;
    const AUTOPILOT: u8 = 1;
    const STATION: u8 = 255;

    // Every MAVLink node broadcasts a heartbeat to say what it is and that it is alive.
    // The fields have names and the common values have names, so nothing here is a number
    // a reader has to look up.
    let announce = Heartbeat {
        type_: mav_type::GCS,
        autopilot: mav_autopilot::INVALID,
        system_status: mav_state::ACTIVE,
        mavlink_version: 3,
        ..Default::default()
    };

    // Framing adds the start marker, the lengths and flags, the sending system and
    // component, the message id, and a checksum seeded with this message's own value. The
    // message supplies its own id and seed, so there is nothing to keep in step by hand.
    let sent = Frame::encode_message(Header::new(STATION, 190, 0), &announce).expect("it fits");
    let on_the_wire = sent.as_bytes().len();
    println!("sent      {} in {on_the_wire} bytes", Heartbeat::NAME);

    // The vehicle answers with its own heartbeat. This copy arrives after some bytes that
    // were already on the wire, and after a copy with one bit flipped in flight.
    let vehicle = Heartbeat {
        type_: mav_type::QUADROTOR,
        autopilot: mav_autopilot::ARDUPILOTMEGA,
        system_status: mav_state::STANDBY,
        mavlink_version: 3,
        ..Default::default()
    };
    let good =
        Frame::encode_message(Header::new(VEHICLE, AUTOPILOT, 0), &vehicle).expect("it fits");
    let mut garbled = good.as_bytes().to_vec();
    *garbled.last_mut().expect("a frame byte") ^= 0xFF;
    let delivered = [b"???".as_slice(), &garbled, good.as_bytes()].concat();

    // The parser skips whatever does not start a frame and drops one whose checksum fails,
    // so the frame it hands back is the good copy rather than the garbled one.
    let mut parser = Parser::new();
    let received = delivered
        .iter()
        .find_map(|&byte| parser.push_byte(byte, &dialect::crc_extra))
        .expect("the good frame completes");
    let heard: Heartbeat = received.decode_message().expect("a heartbeat payload");
    let (kind, state) = (heard.type_, heard.system_status);
    println!("heard     a type-{kind} vehicle in state {state}");

    // Arming it is a command, not a message a sender fires and forgets: the vehicle has to
    // answer, and the sender keeps asking until it does. The protocol numbers each resend,
    // which is how a vehicle tells a retry from a second, deliberate command.
    let mut arming = CommandProtocol::new(mav_cmd::COMPONENT_ARM_DISARM, 3);
    let arm = CommandLong {
        param1: 1.0, // 1 arms, 0 disarms
        target_system: VEHICLE,
        target_component: AUTOPILOT,
        command: arming.command(),
        confirmation: arming.confirmation(),
        ..Default::default()
    };
    Frame::encode_message(Header::new(STATION, 190, 1), &arm).expect("a command fits");
    println!("sent      arm request, confirmation {}", arm.confirmation);

    // Nothing comes back in time, so it goes again with the next confirmation number.
    match arming.on_timeout() {
        Some(confirmation) => println!("silence, resending with confirmation {confirmation}"),
        None => println!("out of retries, the vehicle is unreachable"),
    }

    // An acknowledgement names the command it answers, so one for a different command is
    // not this exchange finishing.
    let someone_elses = CommandAck {
        command: mav_cmd::NAV_TAKEOFF,
        result: mav_result::ACCEPTED,
        ..Default::default()
    };
    let stray = arming.on_ack(&someone_elses);
    println!("an ack for another command: {stray:?}");

    let accepted = CommandAck {
        command: mav_cmd::COMPONENT_ARM_DISARM,
        result: mav_result::ACCEPTED,
        ..Default::default()
    };
    match arming.on_ack(&accepted) {
        AckOutcome::Final(mav_result::ACCEPTED) => println!("armed     the vehicle is ready"),
        AckOutcome::Final(result) => println!("refused   the vehicle answered {result}"),
        AckOutcome::InProgress(percent) => println!("arming    {percent}% done"),
        AckOutcome::Unrelated => println!("that acknowledgement was for something else"),
    }
    // ANCHOR_END: example

    assert_eq!(heard, vehicle);
    assert_eq!(received.message_id(), Heartbeat::ID);
    assert_eq!(arm.confirmation, 0);
    assert_eq!(stray, AckOutcome::Unrelated);
    assert_eq!(
        arming.on_ack(&accepted),
        AckOutcome::Final(mav_result::ACCEPTED)
    );
}
