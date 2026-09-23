//! Tests for the application layer packages, against the text of TS003-2.0.0 and TS006-1.0.0
//! and the command sizes Semtech's LoRa Basics Modem uses for the same two packages.

use super::clock::{ClockCommand, ClockCommands, ClockSync};
use super::firmware::{
    DeleteStatus, FirmwareCommand, FirmwareManager, Image, UpImageStatus, COUNTDOWN_CANCEL,
    COUNTDOWN_NOW, REBOOT_CANCEL, REBOOT_NOW,
};
use super::PackageVersion;
use crate::{Direction, LorawanError};

/// Writes a command out and reads it back in the direction it travels.
fn round_trip_clock(command: ClockCommand) -> usize {
    let mut out = [0u8; 16];
    let len = command.encode(&mut out).expect("it encodes");
    let (read, taken) =
        ClockCommand::parse(command.direction(), &out[..len]).expect("it reads back");
    assert_eq!(read, command, "the command survives the round trip");
    assert_eq!(taken, len, "and the whole command was read");
    len
}

/// Writes a firmware command out and reads it back in the direction it travels.
fn round_trip_firmware(command: FirmwareCommand) -> usize {
    let mut out = [0u8; 16];
    let len = command.encode(&mut out).expect("it encodes");
    let (read, taken) =
        FirmwareCommand::parse(command.direction(), &out[..len]).expect("it reads back");
    assert_eq!(read, command, "the command survives the round trip");
    assert_eq!(taken, len, "and the whole command was read");
    len
}

#[test]
fn every_clock_command_is_the_size_the_specification_gives_it() {
    // Sizes count the identifier, as Basics Modem's ALC_SYNC_*_SIZE constants do.
    assert_eq!(round_trip_clock(ClockCommand::PackageVersionReq), 1);
    assert_eq!(
        round_trip_clock(ClockCommand::PackageVersionAns(PackageVersion {
            package: 1,
            version: 2,
        })),
        3
    );
    assert_eq!(
        round_trip_clock(ClockCommand::AppTimeReq {
            device_time: 0x1234_5678,
            ans_required: true,
            token: 9,
        }),
        6
    );
    assert_eq!(
        round_trip_clock(ClockCommand::AppTimeAns {
            time_correction: -42,
            token: 9,
        }),
        6
    );
    assert_eq!(
        round_trip_clock(ClockCommand::DeviceAppTimePeriodicityReq { period: 5 }),
        2
    );
    assert_eq!(
        round_trip_clock(ClockCommand::DeviceAppTimePeriodicityAns {
            not_supported: true,
            device_time: 0x1234_5678,
        }),
        6
    );
    assert_eq!(
        round_trip_clock(ClockCommand::ForceDeviceResyncCmd { transmissions: 7 }),
        2
    );
}

#[test]
fn a_request_carries_its_time_little_endian_with_the_flag_above_the_token() {
    let mut out = [0u8; 8];
    let len = ClockCommand::AppTimeReq {
        device_time: 0x1234_5678,
        ans_required: true,
        token: 0x0A,
    }
    .encode(&mut out)
    .expect("it encodes");
    assert_eq!(&out[..len], &[0x01, 0x78, 0x56, 0x34, 0x12, 0x1A]);
}

#[test]
fn a_clock_correction_is_applied_once_and_its_token_moves_on() {
    let mut sync = ClockSync::new();
    let mut out = [0u8; 8];
    assert_eq!(sync.token(), 0);
    sync.app_time_req(100, false, &mut out).expect("a request");

    let mut answer = [0u8; 8];
    let len = ClockCommand::AppTimeAns {
        time_correction: 12,
        token: 0,
    }
    .encode(&mut answer)
    .expect("an answer");
    let heard = sync.heard(&answer[..len]).expect("it reads");
    assert_eq!(heard.correction, Some(12));
    assert!(!heard.more_correction);
    assert_eq!(sync.token(), 1, "the token rises with the correction");

    let again = sync.heard(&answer[..len]).expect("it reads");
    assert_eq!(
        again.correction, None,
        "the same answer again carries the old token, so it is ignored"
    );
}

#[test]
fn an_answer_to_no_request_or_with_the_wrong_token_is_ignored() {
    let mut sync = ClockSync::new();
    let mut answer = [0u8; 8];
    let len = ClockCommand::AppTimeAns {
        time_correction: 5,
        token: 0,
    }
    .encode(&mut answer)
    .expect("an answer");
    assert_eq!(
        sync.heard(&answer[..len]).expect("it reads").correction,
        None,
        "nothing was asked"
    );

    let mut out = [0u8; 8];
    sync.app_time_req(1, false, &mut out).expect("a request");
    let len = ClockCommand::AppTimeAns {
        time_correction: 5,
        token: 4,
    }
    .encode(&mut answer)
    .expect("an answer");
    assert_eq!(
        sync.heard(&answer[..len]).expect("it reads").correction,
        None,
        "the token does not match the request"
    );
    assert_eq!(sync.token(), 0, "so the device stays where it was");
}

#[test]
fn a_correction_too_large_for_one_field_is_carried_in_two() {
    // The specification's own example: a device booting with DeviceTime 0 on 2050-01-01
    // needs +2_208_643_200 seconds, which the field cannot hold, so the server sends the
    // largest value it can and the rest follows the next request.
    const NEEDED: i64 = 2_208_643_200;
    let mut sync = ClockSync::new();
    let mut out = [0u8; 8];
    sync.app_time_req(0, true, &mut out).expect("a request");

    let mut answer = [0u8; 8];
    let len = ClockCommand::AppTimeAns {
        time_correction: i32::MAX,
        token: 0,
    }
    .encode(&mut answer)
    .expect("an answer");
    let first = sync.heard(&answer[..len]).expect("it reads");
    assert_eq!(first.correction, Some(i32::MAX));
    assert!(
        first.more_correction,
        "the largest value says another correction is coming"
    );

    let rest = NEEDED - i64::from(i32::MAX);
    assert_eq!(rest, 0x03A5_3881, "the example's remainder");
    sync.app_time_req(i32::MAX as u32, true, &mut out)
        .expect("a second request");
    let len = ClockCommand::AppTimeAns {
        time_correction: rest as i32,
        token: 1,
    }
    .encode(&mut answer)
    .expect("an answer");
    let second = sync.heard(&answer[..len]).expect("it reads");
    assert_eq!(second.correction, Some(rest as i32));
    assert!(!second.more_correction, "and that is the whole of it");
    assert_eq!(sync.token(), 2);
}

#[test]
fn the_period_a_server_sets_doubles_from_two_minutes() {
    let mut sync = ClockSync::new();
    assert_eq!(sync.period_s(), 128, "the shortest period");
    let mut out = [0u8; 8];
    let len = ClockCommand::DeviceAppTimePeriodicityReq { period: 10 }
        .encode(&mut out)
        .expect("a command");
    let heard = sync.heard(&out[..len]).expect("it reads");
    assert!(heard.answer_due, "the device owes an answer");
    assert_eq!(sync.period_s(), 128 << 10);

    let mut answer = [0u8; 8];
    let written = sync.answer(0x2222_1111, &mut answer).expect("an answer");
    let (read, _) =
        ClockCommand::parse(Direction::Uplink, &answer[..written]).expect("it reads back");
    assert_eq!(
        read,
        ClockCommand::DeviceAppTimePeriodicityAns {
            not_supported: false,
            device_time: 0x2222_1111,
        }
    );
    assert!(!sync.answer_due(), "and owes nothing after writing it");
}

#[test]
fn a_device_that_manages_its_own_period_says_so_and_keeps_it() {
    let mut sync = ClockSync::self_managed();
    let mut out = [0u8; 8];
    let len = ClockCommand::DeviceAppTimePeriodicityReq { period: 3 }
        .encode(&mut out)
        .expect("a command");
    sync.heard(&out[..len]).expect("it reads");
    assert_eq!(sync.period_s(), 128, "the server's period is refused");

    let mut answer = [0u8; 8];
    let written = sync.answer(7, &mut answer).expect("an answer");
    let (read, _) =
        ClockCommand::parse(Direction::Uplink, &answer[..written]).expect("it reads back");
    assert_eq!(
        read,
        ClockCommand::DeviceAppTimePeriodicityAns {
            not_supported: true,
            device_time: 7,
        }
    );
}

#[test]
fn a_resynchronization_of_no_transmissions_is_discarded() {
    let mut sync = ClockSync::new();
    let mut out = [0u8; 8];
    let len = ClockCommand::ForceDeviceResyncCmd { transmissions: 0 }
        .encode(&mut out)
        .expect("a command");
    assert_eq!(sync.heard(&out[..len]).expect("it reads").resync, None);

    let len = ClockCommand::ForceDeviceResyncCmd { transmissions: 3 }
        .encode(&mut out)
        .expect("a command");
    assert_eq!(sync.heard(&out[..len]).expect("it reads").resync, Some(3));
}

#[test]
fn a_message_carries_several_commands_and_stops_at_one_it_does_not_know() {
    let message = [0x00, 0x03, 0x02, 0x7F, 0x11];
    let mut walk = ClockCommands::new(Direction::Downlink, &message);
    assert_eq!(
        walk.next().transpose().expect("it reads"),
        Some(ClockCommand::PackageVersionReq)
    );
    assert_eq!(
        walk.next().transpose().expect("it reads"),
        Some(ClockCommand::ForceDeviceResyncCmd { transmissions: 2 })
    );
    assert!(
        walk.next().is_none(),
        "0x7F is not a command of this package"
    );
    assert_eq!(walk.remaining(), &[0x7F, 0x11]);
}

#[test]
fn a_command_cut_short_is_refused() {
    assert_eq!(
        ClockCommand::parse(Direction::Downlink, &[0x01, 0x00, 0x00]),
        Err(LorawanError::MalformedFrame)
    );
    assert_eq!(
        FirmwareCommand::parse(Direction::Downlink, &[0x02, 0x00]),
        Err(LorawanError::MalformedFrame)
    );
}

#[test]
fn every_firmware_command_is_the_size_the_specification_gives_it() {
    assert_eq!(round_trip_firmware(FirmwareCommand::PackageVersionReq), 1);
    assert_eq!(
        round_trip_firmware(FirmwareCommand::PackageVersionAns(PackageVersion {
            package: 4,
            version: 1,
        })),
        3
    );
    assert_eq!(round_trip_firmware(FirmwareCommand::DevVersionReq), 1);
    assert_eq!(
        round_trip_firmware(FirmwareCommand::DevVersionAns {
            firmware: 0x0102_0304,
            hardware: 0x0506_0708,
        }),
        9
    );
    assert_eq!(
        round_trip_firmware(FirmwareCommand::DevRebootTimeReq {
            reboot_time: 0x1234_5678,
        }),
        5
    );
    assert_eq!(
        round_trip_firmware(FirmwareCommand::DevRebootTimeAns { reboot_time: 60 }),
        5
    );
    assert_eq!(
        round_trip_firmware(FirmwareCommand::DevRebootCountdownReq { countdown: 3_600 }),
        4
    );
    assert_eq!(
        round_trip_firmware(FirmwareCommand::DevRebootCountdownAns { countdown: 3_600 }),
        4
    );
    assert_eq!(round_trip_firmware(FirmwareCommand::DevUpgradeImageReq), 1);
    assert_eq!(
        round_trip_firmware(FirmwareCommand::DevUpgradeImageAns {
            status: UpImageStatus::Corrupt,
            next_version: None,
        }),
        2
    );
    assert_eq!(
        round_trip_firmware(FirmwareCommand::DevUpgradeImageAns {
            status: UpImageStatus::Valid,
            next_version: Some(0x0001_0204),
        }),
        6,
        "a version rides with an image that can be installed"
    );
    assert_eq!(
        round_trip_firmware(FirmwareCommand::DevDeleteImageReq {
            version: 0x0001_0204,
        }),
        5
    );
    assert_eq!(
        round_trip_firmware(FirmwareCommand::DevDeleteImageAns(DeleteStatus {
            no_valid_image: false,
            invalid_version: true,
        })),
        2
    );
}

#[test]
fn a_countdown_is_three_bytes_after_its_identifier_least_significant_first() {
    let mut out = [0u8; 8];
    let len = FirmwareCommand::DevRebootCountdownReq { countdown: 3_600 }
        .encode(&mut out)
        .expect("it encodes");
    assert_eq!(&out[..len], &[0x03, 0x10, 0x0E, 0x00]);
}

#[test]
fn the_longest_countdown_is_the_one_the_specification_works_out() {
    // The note under table 6: 0xFFFFFE is 194 days, 4 hours, 20 minutes and 14 seconds.
    let longest = COUNTDOWN_CANCEL - 1;
    assert_eq!(longest, 16_777_214);
    let days = longest / 86_400;
    let hours = (longest % 86_400) / 3_600;
    let minutes = (longest % 3_600) / 60;
    let seconds = longest % 60;
    assert_eq!((days, hours, minutes, seconds), (194, 4, 20, 14));
}

#[test]
fn a_device_reports_what_it_is_running_and_what_it_holds() {
    let mut manager = FirmwareManager::new(0x0001_0203, 0xAABB_CCDD);
    let mut out = [0u8; 32];
    let written = manager
        .heard(&[0x00, 0x01, 0x04], &mut out)
        .expect("three answers");
    let mut walk = super::firmware::FirmwareCommands::new(Direction::Uplink, &out[..written]);
    assert_eq!(
        walk.next().transpose().expect("it reads"),
        Some(FirmwareCommand::PackageVersionAns(PackageVersion {
            package: super::firmware::PACKAGE,
            version: super::firmware::VERSION,
        }))
    );
    assert_eq!(
        walk.next().transpose().expect("it reads"),
        Some(FirmwareCommand::DevVersionAns {
            firmware: 0x0001_0203,
            hardware: 0xAABB_CCDD,
        })
    );
    assert_eq!(
        walk.next().transpose().expect("it reads"),
        Some(FirmwareCommand::DevUpgradeImageAns {
            status: UpImageStatus::None,
            next_version: None,
        }),
        "it is holding nothing"
    );
}

#[test]
fn only_an_installable_image_reports_the_version_it_would_run() {
    let mut refused =
        FirmwareManager::new(1, 2).with_image(Image::refused(UpImageStatus::WrongHardware));
    let mut out = [0u8; 16];
    let written = refused.heard(&[0x04], &mut out).expect("an answer");
    assert_eq!(&out[..written], &[0x04, 0x02]);

    let mut holding = FirmwareManager::new(1, 2).with_image(Image::valid(0x0001_0204));
    let written = holding.heard(&[0x04], &mut out).expect("an answer");
    assert_eq!(&out[..written], &[0x04, 0x03, 0x04, 0x02, 0x01, 0x00]);
}

#[test]
fn a_reboot_at_a_time_the_device_cannot_keep_is_refused() {
    let mut manager = FirmwareManager::new(1, 2);
    let mut out = [0u8; 16];
    let mut request = [0u8; 8];
    let len = FirmwareCommand::DevRebootTimeReq { reboot_time: 1_000 }
        .encode(&mut request)
        .expect("a command");

    // A time in the past.
    let written = manager
        .heard_at(&request[..len], Some(2_000), &mut out)
        .expect("an answer");
    let (read, _) = FirmwareCommand::parse(Direction::Uplink, &out[..written]).expect("it reads");
    assert_eq!(
        read,
        FirmwareCommand::DevRebootTimeAns {
            reboot_time: REBOOT_NOW,
        },
        "zero is how a device says it cannot"
    );
    assert_eq!(manager.reboot_at_s(), None);

    // A device that does not know the time cannot keep any appointment.
    let written = manager.heard(&request[..len], &mut out).expect("an answer");
    let (read, _) = FirmwareCommand::parse(Direction::Uplink, &out[..written]).expect("it reads");
    assert_eq!(
        read,
        FirmwareCommand::DevRebootTimeAns {
            reboot_time: REBOOT_NOW,
        }
    );

    // A time in the future is kept, and the answer counts the seconds to it.
    let written = manager
        .heard_at(&request[..len], Some(400), &mut out)
        .expect("an answer");
    let (read, _) = FirmwareCommand::parse(Direction::Uplink, &out[..written]).expect("it reads");
    assert_eq!(read, FirmwareCommand::DevRebootTimeAns { reboot_time: 600 });
    assert_eq!(manager.reboot_at_s(), Some(1_000));
}

#[test]
fn a_reboot_at_once_goes_unanswered_and_a_cancellation_is_acknowledged() {
    let mut manager = FirmwareManager::new(1, 2);
    let mut out = [0u8; 16];
    let mut request = [0u8; 8];

    let len = FirmwareCommand::DevRebootTimeReq {
        reboot_time: REBOOT_NOW,
    }
    .encode(&mut request)
    .expect("a command");
    assert_eq!(
        manager.heard(&request[..len], &mut out).expect("nothing"),
        0,
        "the device will be gone before it could answer"
    );
    assert!(manager.reboot_now());

    let len = FirmwareCommand::DevRebootTimeReq {
        reboot_time: REBOOT_CANCEL,
    }
    .encode(&mut request)
    .expect("a command");
    let written = manager.heard(&request[..len], &mut out).expect("an answer");
    let (read, _) = FirmwareCommand::parse(Direction::Uplink, &out[..written]).expect("it reads");
    assert_eq!(
        read,
        FirmwareCommand::DevRebootTimeAns {
            reboot_time: REBOOT_CANCEL,
        }
    );
    assert!(!manager.reboot_now(), "and nothing is programmed now");
}

#[test]
fn a_countdown_replaces_whatever_was_programmed_before() {
    let mut manager = FirmwareManager::new(1, 2);
    let mut out = [0u8; 16];
    let mut request = [0u8; 8];

    let len = FirmwareCommand::DevRebootTimeReq { reboot_time: 900 }
        .encode(&mut request)
        .expect("a command");
    manager
        .heard_at(&request[..len], Some(300), &mut out)
        .expect("an answer");
    assert_eq!(manager.reboot_at_s(), Some(900));

    let len = FirmwareCommand::DevRebootCountdownReq { countdown: 120 }
        .encode(&mut request)
        .expect("a command");
    let written = manager.heard(&request[..len], &mut out).expect("an answer");
    let (read, _) = FirmwareCommand::parse(Direction::Uplink, &out[..written]).expect("it reads");
    assert_eq!(
        read,
        FirmwareCommand::DevRebootCountdownAns { countdown: 120 }
    );
    assert_eq!(manager.reboot_in_s(), Some(120));
    assert_eq!(
        manager.reboot_at_s(),
        None,
        "a device stores one programmed reboot, not two"
    );

    let len = FirmwareCommand::DevRebootCountdownReq {
        countdown: COUNTDOWN_CANCEL,
    }
    .encode(&mut request)
    .expect("a command");
    let written = manager.heard(&request[..len], &mut out).expect("an answer");
    let (read, _) = FirmwareCommand::parse(Direction::Uplink, &out[..written]).expect("it reads");
    assert_eq!(
        read,
        FirmwareCommand::DevRebootCountdownAns {
            countdown: COUNTDOWN_CANCEL,
        }
    );
    assert_eq!(manager.reboot_in_s(), None);

    let len = FirmwareCommand::DevRebootCountdownReq {
        countdown: COUNTDOWN_NOW,
    }
    .encode(&mut request)
    .expect("a command");
    assert_eq!(
        manager.heard(&request[..len], &mut out).expect("nothing"),
        0
    );
    assert!(manager.reboot_now());
}

#[test]
fn an_image_is_deleted_only_by_the_version_that_is_there() {
    let mut empty = FirmwareManager::new(1, 2);
    let mut out = [0u8; 16];
    let mut request = [0u8; 8];
    let len = FirmwareCommand::DevDeleteImageReq { version: 7 }
        .encode(&mut request)
        .expect("a command");
    let written = empty.heard(&request[..len], &mut out).expect("an answer");
    let (read, _) = FirmwareCommand::parse(Direction::Uplink, &out[..written]).expect("it reads");
    assert_eq!(
        read,
        FirmwareCommand::DevDeleteImageAns(DeleteStatus {
            no_valid_image: true,
            invalid_version: false,
        })
    );

    let mut holding = FirmwareManager::new(1, 2).with_image(Image::valid(9));
    let written = holding.heard(&request[..len], &mut out).expect("an answer");
    let (read, _) = FirmwareCommand::parse(Direction::Uplink, &out[..written]).expect("it reads");
    assert_eq!(
        read,
        FirmwareCommand::DevDeleteImageAns(DeleteStatus {
            no_valid_image: false,
            invalid_version: true,
        }),
        "the version asked for is not the one held"
    );
    assert!(holding.image().is_some(), "so the image stays");

    let len = FirmwareCommand::DevDeleteImageReq { version: 9 }
        .encode(&mut request)
        .expect("a command");
    let written = holding.heard(&request[..len], &mut out).expect("an answer");
    let (read, _) = FirmwareCommand::parse(Direction::Uplink, &out[..written]).expect("it reads");
    assert_eq!(
        read,
        FirmwareCommand::DevDeleteImageAns(DeleteStatus::default())
    );
    assert!(read_deleted(read), "which is how a device says it worked");
    assert!(holding.image().is_none());
}

/// Whether a delete answer reported success.
fn read_deleted(command: FirmwareCommand) -> bool {
    matches!(command, FirmwareCommand::DevDeleteImageAns(status) if status.deleted())
}
