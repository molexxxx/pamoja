//! Tests for remote multicast setup, against the text of TS005-2.0.0 and the command bytes
//! ChirpStack's multicast setup package is tested with.

use super::multicast::{
    mc_app_s_key, mc_ke_key, mc_key, mc_nwk_s_key, mc_root_key, mc_root_key_for, wrap_mc_key,
    McCommand, SessionStatus,
};
use super::PackageVersion;
use crate::{Direction, LorawanError};

/// Reads a command and checks it writes back to the same bytes.
fn round_trip(direction: Direction, bytes: &[u8], expected: McCommand) {
    let (read, taken) = McCommand::parse(direction, bytes).expect("it reads");
    assert_eq!(read, expected, "reading {bytes:02x?}");
    assert_eq!(taken, bytes.len(), "the whole command was read");
    let mut out = [0u8; 40];
    let len = read.encode(&mut out).expect("it writes");
    assert_eq!(&out[..len], bytes, "writing {expected:?}");
}

#[test]
fn the_commands_match_the_bytes_chirpstack_tests_its_own_package_with() {
    round_trip(Direction::Downlink, &[0x00], McCommand::PackageVersionReq);
    round_trip(
        Direction::Uplink,
        &[0x00, 0x01, 0x01],
        McCommand::PackageVersionAns(PackageVersion {
            package: 1,
            version: 1,
        }),
    );
    round_trip(
        Direction::Downlink,
        &[0x01, 0x03],
        McCommand::McGroupStatusReq { req_group_mask: 3 },
    );
    round_trip(
        Direction::Downlink,
        &[
            0x02, 0x02, 0x04, 0x03, 0x02, 0x01, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x00,
        ],
        McCommand::McGroupSetupReq {
            mc_group_id: 2,
            mc_addr: 0x0102_0304,
            mc_key_encrypted: [1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8],
            min_mc_fcnt: 1024,
            max_mc_fcnt: 2048,
        },
    );
    round_trip(
        Direction::Uplink,
        &[0x02, 0x06],
        McCommand::McGroupSetupAns {
            mc_group_id: 2,
            id_error: true,
        },
    );
    round_trip(
        Direction::Downlink,
        &[0x03, 0x03],
        McCommand::McGroupDeleteReq { mc_group_id: 3 },
    );
    round_trip(
        Direction::Uplink,
        &[0x03, 0x07],
        McCommand::McGroupDeleteAns {
            mc_group_id: 3,
            group_undefined: true,
        },
    );
    round_trip(
        Direction::Downlink,
        &[
            0x04, 0x02, 0x00, 0x04, 0x00, 0x00, 0x0F, 0x28, 0x76, 0x84, 0x05,
        ],
        McCommand::McClassCSessionReq {
            mc_group_id: 2,
            session_time: 1024,
            time_out: 15,
            dl_frequency_hz: 868_100_000,
            data_rate: 5,
        },
    );
    round_trip(
        Direction::Uplink,
        &[0x04, 0x02, 0x00, 0x04, 0x00],
        McCommand::McClassCSessionAns {
            status: SessionStatus {
                mc_group_id: 2,
                ..SessionStatus::default()
            },
            time_to_start: Some(1024),
        },
    );
    round_trip(
        Direction::Uplink,
        &[0x04, 0x1E],
        McCommand::McClassCSessionAns {
            status: SessionStatus {
                mc_group_id: 2,
                dr_error: true,
                freq_error: true,
                group_undefined: true,
                start_missed: false,
            },
            time_to_start: None,
        },
    );
}

#[test]
fn a_status_answer_lists_the_groups_that_follow_it() {
    // ChirpStack's own bytes: two groups reported, two held in all.
    let message = [
        0x01, 0x23, 0x00, 0x04, 0x03, 0x02, 0x01, 0x01, 0x04, 0x03, 0x02, 0x02,
    ];
    let (answer, taken) = McCommand::parse(Direction::Uplink, &message).expect("it reads");
    assert_eq!(
        answer,
        McCommand::McGroupStatusAns {
            ans_group_mask: 0b0011,
            nb_total_groups: 2,
        }
    );

    let (first, used) = McCommand::status_item(&message[taken..]).expect("a group");
    assert_eq!(
        first,
        McCommand::McGroupStatusItem {
            mc_group_id: 0,
            mc_addr: 0x0102_0304,
        }
    );
    let (second, _) = McCommand::status_item(&message[taken + used..]).expect("a group");
    assert_eq!(
        second,
        McCommand::McGroupStatusItem {
            mc_group_id: 1,
            mc_addr: 0x0202_0304,
        }
    );

    // And the whole message writes back the way it arrived.
    let mut out = [0u8; 16];
    let mut at = answer.encode(&mut out).expect("it writes");
    at += first.encode(&mut out[at..]).expect("it writes");
    at += second.encode(&mut out[at..]).expect("it writes");
    assert_eq!(&out[..at], &message);
}

#[test]
fn a_class_b_session_carries_its_periodicity_beside_its_timeout() {
    let command = McCommand::McClassBSessionReq {
        mc_group_id: 1,
        session_time: 0x0000_8000,
        time_out: 9,
        periodicity: 3,
        dl_frequency_hz: 869_525_000,
        data_rate: 3,
    };
    let mut out = [0u8; 16];
    let len = command.encode(&mut out).expect("it writes");
    assert_eq!(out[6], 0x39, "the periodicity above the timeout");
    assert_eq!(
        &out[7..10],
        &(869_525_000u32 / 100).to_le_bytes()[..3],
        "the frequency in hundreds of hertz"
    );
    let (read, taken) = McCommand::parse(Direction::Downlink, &out[..len]).expect("it reads");
    assert_eq!(read, command);
    assert_eq!(taken, len);
}

#[test]
fn a_session_the_device_refuses_carries_no_start() {
    let refused = SessionStatus {
        mc_group_id: 1,
        start_missed: true,
        ..SessionStatus::default()
    };
    assert!(!refused.accepted());
    let mut out = [0u8; 8];
    let len = McCommand::McClassBSessionAns {
        status: refused,
        time_to_start: Some(500),
    }
    .encode(&mut out)
    .expect("it writes");
    assert_eq!(
        &out[..len],
        &[0x05, 0x21],
        "a start the device missed leaves the time out of the answer"
    );
}

#[test]
fn a_command_cut_short_is_refused() {
    assert_eq!(
        McCommand::parse(Direction::Downlink, &[0x02, 0x00]),
        Err(LorawanError::MalformedFrame)
    );
    assert_eq!(
        McCommand::parse(Direction::Downlink, &[0x07]),
        Err(LorawanError::UnknownCommand(0x07))
    );
    assert_eq!(
        McCommand::status_item(&[0x00, 0x01]),
        Err(LorawanError::MalformedFrame)
    );
}

#[test]
fn a_group_key_travels_under_a_key_the_device_never_sends() {
    let app_key = [0x2B; 16];
    let group_key = [0x77; 16];

    // The chain of section 4.3, each step a block of its own.
    let root = mc_root_key(&app_key);
    assert_ne!(root, app_key);
    let ke = mc_ke_key(&root);
    assert_ne!(ke, root);

    let wrapped = wrap_mc_key(&ke, &group_key);
    assert_ne!(wrapped, group_key, "it does not travel in the clear");
    assert_eq!(mc_key(&ke, &wrapped), group_key, "and comes back whole");

    // The 1.1 scheme starts from another constant, so the same key gives another chain.
    assert_ne!(mc_root_key_for(&app_key, true), root);
}

#[test]
fn the_group_session_keys_are_tied_to_the_group_address() {
    let group_key = [0x77; 16];
    let addr = 0x2601_1BDA;
    let app = mc_app_s_key(&group_key, addr);
    let nwk = mc_nwk_s_key(&group_key, addr);
    assert_ne!(app, nwk, "the two keys differ by their constant");
    assert_ne!(
        mc_app_s_key(&group_key, addr + 1),
        app,
        "and by the address they are for"
    );
}
