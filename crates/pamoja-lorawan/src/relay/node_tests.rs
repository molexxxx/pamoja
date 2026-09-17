//! A relay against the rules TS011-1.0.1 sets for it: what it scans, what it answers, what
//! it forwards, and what it refuses.

use pamoja_lora::region::{Region, RelayChannel};

use super::*;
use crate::device::{EndDevice, Heard, ReceiveWindow, Settings};
use crate::mac::MacCommand;
use crate::{Device, Downlink, JoinGrant, LorawanError, Session, Uplink};

const RELAY_ADDR: u32 = 0x2601_0001;
const SENSOR_ADDR: u32 = 0x2601_1BDA;
const SENSOR_NWK_S_KEY: [u8; 16] = [0x2B; 16];
const UPLINK: Carrier = Carrier::new(868_100_000, 5);

fn relay_session() -> Session {
    Session::new(RELAY_ADDR, [0x11; 16], [0x22; 16])
}

fn sensor() -> Session {
    Session::new(SENSOR_ADDR, SENSOR_NWK_S_KEY, [0x99; 16])
}

/// A running EU868 relay with a joined device, trusting the sensor at index 0.
fn relay() -> Relay<'static> {
    let device = EndDevice::personalized(
        Region::Eu868.plan(),
        relay_session(),
        Settings::new(2, 14)
            .with_seed(7)
            .with_tuning_range(863_000_000, 870_000_000),
    )
    .expect("a device");
    let mut relay = Relay::new(
        device,
        RelaySettings::new(XtalAccuracy::Ppm10, CadToRx::Symbols2),
    );
    relay
        .start(RelayConfig::region_default(Region::Eu868.plan()).expect("relay channels"))
        .expect("a configuration the relay can run");
    relay.trust(0, SENSOR_ADDR, &root(), 0, 63, 0);
    relay
}

/// The same relay, moved up to DR5 by its network, where its own frames are short and
/// its windows carry more than a slow end device's.
fn fast_relay() -> Relay<'static> {
    let mut relay = relay();
    network_sends(
        &mut relay,
        &[MacCommand::LinkAdrReq {
            data_rate: 5,
            tx_power: 0,
            channel_mask: 0x0007,
            mask_control: 0,
            transmissions: 1,
        }],
        0,
        APART_US,
    );
    let answers = answers_of(&mut relay, 2 * APART_US);
    assert_eq!(
        answers,
        vec![MacCommand::LinkAdrAns {
            power_ack: true,
            data_rate_ack: true,
            channel_mask_ack: true,
        }]
    );
    assert_eq!(relay.device().data_rate(), 5);
    relay
}

fn root() -> [u8; 16] {
    root_wor_s_key(&SENSOR_NWK_S_KEY)
}

fn wor(scan: &Scan, wfcnt: u32) -> [u8; WOR_UPLINK_LEN] {
    wor_uplink(
        &WorKeys::derive(&root(), SENSOR_ADDR),
        SENSOR_ADDR,
        wfcnt,
        UPLINK,
        scan.carrier,
    )
    .expect("a WOR frame")
}

/// How far apart tests put a relay's own uplinks, so its duty cycle never turns one down.
const APART_US: u64 = 500_000_000;

/// Walks one uplink through a relay: the WOR frame, the uplink, and the forward.
fn forward(relay: &mut Relay<'_>, payload: &[u8], now_us: u64) -> crate::device::Transmission {
    forward_at(relay, UPLINK, payload, 0, now_us)
}

/// The same, on a carrier and counter of the caller's choosing.
fn forward_at(
    relay: &mut Relay<'_>,
    uplink: Carrier,
    payload: &[u8],
    wfcnt: u32,
    now_us: u64,
) -> crate::device::Transmission {
    let scan = relay.next_scan(now_us).expect("running");
    let frame = wor_uplink(
        &WorKeys::derive(&root(), SENSOR_ADDR),
        SENSOR_ADDR,
        wfcnt,
        uplink,
        scan.carrier,
    )
    .expect("a WOR frame");
    let Wake::Uplink { listen, .. } = relay
        .heard_wor(&scan, &frame, -90, 4, scan.start_us + 900_000)
        .expect("a trusted device")
    else {
        panic!("an uplink follows");
    };
    let listen = listen.expect("forwarding is available");
    let frame = sensor()
        .encode_uplink(&Uplink::new(wfcnt, 1, payload))
        .expect("an uplink");
    let due = relay
        .heard_uplink(frame.as_bytes(), -88, 6, listen.start_us + 60_000)
        .expect("forwarded");
    relay.forward(due).expect("sent")
}

/// Reads back what a relay's device just transmitted.
fn sent_by(relay: &Relay<'_>, transmission: &crate::device::Transmission) -> crate::RxData {
    relay_session()
        .decode(
            transmission.frame.as_bytes(),
            relay.device().fcnt_up().saturating_sub(1),
        )
        .expect("the relay's own uplink decodes")
}

#[test]
fn the_appendix_3_filter_forwards_and_filters_the_devices_it_names() {
    // TS011-1.0.1 appendix 3: a manufacturer forwarded, one of its JoinEUIs filtered, a
    // range of DevEUIs under that JoinEUI forwarded again, one device of that range
    // filtered, and everyone else filtered.
    let join_eui = [0xAB, 0xCD, 0xEF, 0xAB, 0xCD, 0xEF, 0xAB, 0xCD];
    let range = [0x12, 0x34, 0x56, 0x78, 0x28, 0x37, 0x46];
    let mut filter = JoinFilter::new();
    filter.set_default(FilterAction::Filter);
    let rules = [
        FilterRule::new(&join_eui[..3], FilterAction::Forward),
        FilterRule::new(&join_eui, FilterAction::Filter),
        FilterRule::new(&[&join_eui[..], &range[..]].concat(), FilterAction::Forward),
        FilterRule::new(
            &[&join_eui[..], &range[..], &[0x48]].concat(),
            FilterAction::Filter,
        ),
    ];
    for (index, rule) in (1u8..).zip(rules) {
        assert!(filter.set_rule(index, rule));
    }

    let other_join_eui = [0xAB, 0xCD, 0xEF, 0xAB, 0xCD, 0xEF, 0xAB, 0xAF];
    let stranger = [0x91, 0x82, 0x73, 0x64, 0x55, 0xEF, 0xAB, 0xCD];
    let devices = [
        (
            join_eui,
            [0x12, 0x34, 0x56, 0x78, 0x28, 0x37, 0x40, 0x44],
            FilterAction::Filter,
        ),
        (
            other_join_eui,
            [0x12, 0x34, 0x56, 0x78, 0x28, 0x37, 0x46, 0x45],
            FilterAction::Forward,
        ),
        (
            join_eui,
            [0x12, 0x34, 0x56, 0x78, 0x28, 0x37, 0x46, 0x46],
            FilterAction::Forward,
        ),
        (
            join_eui,
            [0x12, 0x34, 0x56, 0x78, 0x28, 0x37, 0x46, 0x47],
            FilterAction::Forward,
        ),
        (
            join_eui,
            [0x12, 0x34, 0x56, 0x78, 0x28, 0x37, 0x46, 0x48],
            FilterAction::Filter,
        ),
        (
            stranger,
            [0x12, 0x34, 0x56, 0x78, 0x28, 0x37, 0x46, 0x49],
            FilterAction::Filter,
        ),
    ];
    for (at, (join_eui, dev_eui, decision)) in devices.into_iter().enumerate() {
        assert_eq!(filter.decide(&join_eui, &dev_eui), decision, "device {at}");
    }
}

#[test]
fn a_filter_rule_is_taken_only_when_its_parts_make_one() {
    let mut filter = JoinFilter::new();
    let mut eui = [0u8; 16];
    eui[..3].copy_from_slice(&[0xAB, 0xCD, 0xEF]);

    assert_eq!(
        filter.apply(1, 2, 3, &eui),
        MacCommand::FilterListAns {
            combined_rules_ack: true,
            eui_len_ack: true,
            action_ack: true,
        }
    );
    assert_eq!(
        filter.rule(1).expect("a rule").prefix(),
        &[0xAB, 0xCD, 0xEF]
    );

    assert_eq!(
        filter.apply(0, 2, 3, &eui),
        MacCommand::FilterListAns {
            combined_rules_ack: false,
            eui_len_ack: true,
            action_ack: true,
        },
        "rule 0 carries no prefix",
    );
    assert_eq!(filter.default_action(), FilterAction::Forward, "unchanged");

    assert_eq!(
        filter.apply(2, 3, 4, &eui),
        MacCommand::FilterListAns {
            combined_rules_ack: false,
            eui_len_ack: true,
            action_ack: false,
        },
        "action 3 is reserved",
    );
    assert_eq!(
        filter.apply(2, 1, 17, &eui),
        MacCommand::FilterListAns {
            combined_rules_ack: true,
            eui_len_ack: false,
            action_ack: true,
        },
        "seventeen bytes are two EUIs and one more",
    );
    assert!(filter.rule(2).is_none(), "nothing was taken");

    assert_eq!(
        filter.apply(1, 0, 0, &eui),
        MacCommand::FilterListAns {
            combined_rules_ack: true,
            eui_len_ack: true,
            action_ack: true,
        },
    );
    assert!(filter.rule(1).is_none(), "an empty action removes a rule");
}

#[test]
fn a_relay_scans_its_channels_on_whole_periods_and_picks_up_where_it_left_off() {
    let mut relay = relay();
    let second = RelayChannel::new(865_500_000, 865_900_000, 5);
    relay
        .start(
            RelayConfig::new(
                CadPeriodicity::Ms500,
                Region::Eu868.plan().relay_channel(0).expect("a channel"),
            )
            .with_second_channel(second),
        )
        .expect("two channels the relay can scan");

    let first = relay.next_scan(1_000_000).expect("running");
    assert_eq!(
        (first.start_us, first.channel),
        (1_000_000, WorChannel::Default)
    );
    assert_eq!(first.carrier, Carrier::new(865_100_000, 3));

    let next = relay.next_scan(first.start_us + 1).expect("running");
    assert_eq!(
        (next.start_us, next.channel),
        (1_250_000, WorChannel::Second),
        "the second channel comes half a period later",
    );
    assert_eq!(next.carrier, Carrier::new(865_500_000, 5));

    // Busy for two and a bit periods, the relay resumes on the next whole one.
    let resumed = relay.next_scan(2_100_000).expect("running");
    assert_eq!(
        (resumed.start_us, resumed.channel),
        (2_250_000, WorChannel::Second)
    );
    assert_eq!(
        relay.next_scan(2_600_000).expect("running").start_us,
        2_750_000
    );
}

#[test]
fn a_relay_scans_the_default_channel_for_the_longest_preamble_a_device_may_send() {
    let mut relay = relay();
    let scan = relay.next_scan(0).expect("running");
    // A device that has never heard from a relay assumes a second between scans, SF9 here.
    assert_eq!(scan.link.spreading_factor(), 9);
    assert_eq!(
        scan.preamble_symbols,
        unsynchronized_preamble_symbols(
            CadPeriodicity::Ms1000,
            scan.link.symbol_time_us(),
            CadToRx::Symbols2
        )
    );
}

#[test]
fn a_relay_refuses_channels_it_cannot_tune_and_scans_too_close_together() {
    let mut relay = relay();
    let elsewhere = RelayChannel::new(433_100_000, 433_300_000, 3);
    assert_eq!(
        relay.start(RelayConfig::new(CadPeriodicity::Ms1000, elsewhere)),
        Err(RelayError::Configuration),
        "a European radio does not tune 433 MHz",
    );

    // A relay that takes eight symbols to switch from detecting to receiving needs more
    // than twenty milliseconds between scans of an SF9 channel.
    let device = EndDevice::personalized(
        Region::Eu868.plan(),
        relay_session(),
        Settings::new(2, 14).with_tuning_range(863_000_000, 870_000_000),
    )
    .expect("a device");
    let mut slow = Relay::new(
        device,
        RelaySettings::new(XtalAccuracy::Ppm40, CadToRx::Symbols8),
    );
    let channel = Region::Eu868.plan().relay_channel(0).expect("a channel");
    assert_eq!(channel.data_rate, 3, "SF9, 4.096 ms a symbol");
    assert_eq!(
        slow.start(RelayConfig::new(CadPeriodicity::Ms20, channel)),
        Err(RelayError::Configuration),
    );
    assert!(slow
        .start(RelayConfig::new(CadPeriodicity::Ms50, channel))
        .is_ok());

    // Two channels are scanned half a period apart, which halves what fits.
    let second = RelayChannel::new(865_500_000, 865_900_000, 3);
    assert_eq!(
        slow.start(RelayConfig::new(CadPeriodicity::Ms50, channel).with_second_channel(second)),
        Err(RelayError::Configuration),
    );
}

#[test]
fn the_network_starts_and_stops_a_relay_with_its_own_commands() {
    let mut relay = relay();
    relay.stop();
    assert!(relay.config().is_none());

    let started = MacCommand::RelayConfReq {
        enabled: true,
        cad_periodicity: CadPeriodicity::Ms500.code(),
        default_channel_index: 1,
        second_channel_index: 0,
        second_channel_data_rate: 0,
        second_channel_ack_offset: 0,
        second_channel_frequency_hz: 0,
    };
    assert_eq!(
        network_says(&mut relay, started, 0),
        vec![MacCommand::RelayConfAns {
            cad_periodicity_ack: true,
            default_channel_index_ack: true,
            second_channel_index_ack: true,
            second_channel_data_rate_ack: true,
            second_channel_ack_offset_ack: true,
            second_channel_frequency_ack: true,
        }]
    );
    let config = relay.config().expect("running");
    assert_eq!(config.cad_periodicity, CadPeriodicity::Ms500);
    assert_eq!(
        config.default_channel,
        Region::Eu868.plan().relay_channel(1).expect("a channel"),
        "the second of the region's WOR channels",
    );

    let stopped = MacCommand::RelayConfReq {
        enabled: false,
        cad_periodicity: 7,
        default_channel_index: 9,
        second_channel_index: 3,
        second_channel_data_rate: 0,
        second_channel_ack_offset: 0,
        second_channel_frequency_hz: 0,
    };
    assert_eq!(
        network_says(&mut relay, stopped, 1),
        vec![MacCommand::RelayConfAns {
            cad_periodicity_ack: true,
            default_channel_index_ack: true,
            second_channel_index_ack: true,
            second_channel_data_rate_ack: true,
            second_channel_ack_offset_ack: true,
            second_channel_frequency_ack: true,
        }],
        "a command that stops the relay ignores its other fields",
    );
    assert!(relay.config().is_none());
    assert!(relay.next_scan(0).is_none());
}

#[test]
fn a_relay_names_the_parts_of_a_configuration_it_cannot_take() {
    let mut relay = relay();
    let refused = MacCommand::RelayConfReq {
        enabled: true,
        cad_periodicity: 6,
        default_channel_index: 0,
        second_channel_index: 2,
        second_channel_data_rate: 0,
        second_channel_ack_offset: 0,
        second_channel_frequency_hz: 0,
    };
    assert_eq!(
        network_says(&mut relay, refused, 0),
        vec![MacCommand::RelayConfAns {
            cad_periodicity_ack: false,
            default_channel_index_ack: true,
            second_channel_index_ack: false,
            second_channel_data_rate_ack: true,
            second_channel_ack_offset_ack: true,
            second_channel_frequency_ack: true,
        }],
        "a reserved scan period, and a second channel index that is reserved too",
    );

    let second = MacCommand::RelayConfReq {
        enabled: true,
        cad_periodicity: CadPeriodicity::Ms1000.code(),
        default_channel_index: 0,
        second_channel_index: 1,
        second_channel_data_rate: 15,
        second_channel_ack_offset: 6,
        second_channel_frequency_hz: 1_000_000_000,
    };
    assert_eq!(
        network_says(&mut relay, second, 1),
        vec![MacCommand::RelayConfAns {
            cad_periodicity_ack: true,
            default_channel_index_ack: true,
            second_channel_index_ack: true,
            second_channel_data_rate_ack: false,
            second_channel_ack_offset_ack: false,
            second_channel_frequency_ack: false,
        }],
        "a reserved data rate, a reserved offset, and a frequency the radio does not tune",
    );
}

#[test]
fn the_relay_reads_back_and_forgets_a_trusted_device_its_network_asks_about() {
    let mut relay = relay();
    let scan = relay.next_scan(0).expect("running");
    let frame = wor(&scan, 4);
    relay
        .heard_wor(&scan, &frame, -90, 4, scan.start_us + 900_000)
        .expect("a trusted device");

    assert_eq!(
        network_says(
            &mut relay,
            MacCommand::CtrlUplinkListReq {
                index: 0,
                action: 0
            },
            0
        ),
        vec![MacCommand::CtrlUplinkListAns {
            index_ack: true,
            wfcnt: 4,
        }],
        "the last counter it accepted",
    );
    assert_eq!(
        network_says(
            &mut relay,
            MacCommand::CtrlUplinkListReq {
                index: 5,
                action: 0
            },
            1
        ),
        vec![MacCommand::CtrlUplinkListAns {
            index_ack: false,
            wfcnt: 0,
        }],
        "an empty entry",
    );
    assert_eq!(
        network_says(
            &mut relay,
            MacCommand::CtrlUplinkListReq {
                index: 0,
                action: 1
            },
            2
        ),
        vec![MacCommand::CtrlUplinkListAns {
            index_ack: true,
            wfcnt: 4,
        }],
    );
    assert!(relay.trusted().get(0).is_none(), "the entry is gone");
}

#[test]
fn a_trusted_device_starts_at_the_counter_the_network_gave_the_relay() {
    let mut relay = relay();
    relay.trust(1, 0x2601_00FF, &root(), 9, 63, 0);
    assert_eq!(
        relay.trusted().get(1).expect("trusted").last_wfcnt(),
        8,
        "one before the counter expected next, so the network reads back what it set",
    );
    relay.trust(2, 0x2601_00FE, &root(), 0, 63, 0);
    assert_eq!(
        relay.trusted().get(2).expect("trusted").last_wfcnt(),
        u32::MAX,
        "wrapping, before a first frame at counter 0",
    );
}

#[test]
fn a_wake_on_radio_frame_only_counts_once() {
    let mut relay = relay();
    let scan = relay.next_scan(0).expect("running");
    let frame = wor(&scan, 0);
    let ended = scan.start_us + 900_000;
    relay
        .heard_wor(&scan, &frame, -90, 4, ended)
        .expect("a trusted device");
    relay.uplink_missed();

    assert_eq!(
        relay.heard_wor(&scan, &frame, -90, 4, ended + 2_000_000),
        Err(RelayError::Frame(LorawanError::MicMismatch)),
        "the same frame does not verify against the counter the relay now expects",
    );
    assert_eq!(
        relay.trusted().get(0).expect("trusted").next_wfcnt(),
        Some(1)
    );

    // The counter's low bits are all a frame carries, so one that wrapped still verifies.
    relay.trust(0, SENSOR_ADDR, &root(), 0x1_0000, 63, 0);
    let scan = relay.next_scan(3_000_000).expect("running");
    let wrapped = wor(&scan, 0x1_0000);
    assert!(relay
        .heard_wor(&scan, &wrapped, -90, 4, scan.start_us + 900_000)
        .is_ok());
}

#[test]
fn a_frame_from_a_device_the_relay_does_not_trust_notifies_the_network_once() {
    let mut relay = relay();
    relay.trusted_mut().remove(0);
    let scan = relay.next_scan(0).expect("running");
    let frame = wor(&scan, 0);

    assert_eq!(
        relay.heard_wor(&scan, &frame, -100, 5, scan.start_us + 900_000),
        Ok(Wake::Notified {
            dev_addr: SENSOR_ADDR
        })
    );
    assert_eq!(relay.limits().notify.tokens(), 3, "one notification spent");
    assert_eq!(relay.limits().overall.tokens(), 7);

    let scan = relay.next_scan(2_000_000).expect("running");
    let again = wor(&scan, 1);
    assert_eq!(
        relay.heard_wor(&scan, &again, -100, 5, scan.start_us + 900_000),
        Ok(Wake::Notified {
            dev_addr: SENSOR_ADDR
        })
    );
    assert_eq!(
        relay.limits().notify.tokens(),
        3,
        "the notice waiting to go out covers the second frame",
    );

    // The notice rides the relay's next uplink, and only then is another spent.
    let transmission = relay
        .device_mut()
        .send(1, b"hi", false, APART_US)
        .expect("sent");
    let sent = sent_by(&relay, &transmission);
    assert_eq!(
        sent.fopts()[0],
        MacCommand::NotifyNewEndDeviceReq {
            dev_addr: SENSOR_ADDR,
            rssi_dbm: -100,
            snr_db: 5,
        }
        .cid()
    );
    let scan = relay.next_scan(APART_US + 6_000_000).expect("running");
    let third = wor(&scan, 2);
    relay
        .heard_wor(&scan, &third, -100, 5, scan.start_us + 900_000)
        .expect("notified");
    assert_eq!(relay.limits().notify.tokens(), 2);
}

#[test]
fn notifications_stop_at_their_limit_until_the_hour_turns() {
    let mut relay = relay();
    relay.trusted_mut().remove(0);
    let mut at = 0;
    for spent in 0..4u32 {
        let scan = relay.next_scan(at).expect("running");
        let frame = wor(&scan, spent);
        relay
            .heard_wor(&scan, &frame, -100, 5, scan.start_us + 900_000)
            .expect("notified");
        relay
            .device_mut()
            .send(1, b"hi", false, at + 1_500_000)
            .expect("sent");
        relay.nothing_heard(at + 4_000_000).expect("windows closed");
        at += APART_US;
    }
    assert_eq!(relay.limits().notify.tokens(), 0);

    let scan = relay.next_scan(at).expect("running");
    let frame = wor(&scan, 4);
    assert_eq!(
        relay.heard_wor(&scan, &frame, -100, 5, scan.start_us + 900_000),
        Err(RelayError::Limited)
    );

    // An hour on, the bucket has earned its reload.
    let scan = relay.next_scan(3_700_000_000).expect("running");
    let frame = wor(&scan, 5);
    assert!(relay
        .heard_wor(&scan, &frame, -100, 5, scan.start_us + 900_000)
        .is_ok());
    assert_eq!(relay.limits().notify.tokens(), 3);
}

#[test]
fn a_device_at_its_limit_is_acknowledged_and_told_when_to_try_again() {
    let mut relay = relay();
    // One uplink an hour, in a bucket that holds one.
    relay.trust(0, SENSOR_ADDR, &root(), 0, 1, 0);
    forward(&mut relay, b"21.5", 0);
    relay.nothing_heard(4_000_000).expect("windows closed");

    let scan = relay.next_scan(5_000_000).expect("running");
    let frame = wor(&scan, 1);
    let Wake::Uplink {
        forward,
        acknowledgment,
        listen,
        ..
    } = relay
        .heard_wor(&scan, &frame, -90, 4, scan.start_us + 900_000)
        .expect("a trusted device")
    else {
        panic!("an uplink follows");
    };
    assert_eq!(forward, Forward::RetryIn60Minutes);
    assert!(
        listen.is_none(),
        "the relay does not listen for an uplink it cannot forward"
    );

    // The acknowledgment still goes out, so the device knows to wait.
    let ack = acknowledgment.expect("acknowledged");
    let state = open_wor_ack(
        &ack.frame,
        &WorKeys::derive(&root(), SENSOR_ADDR),
        SENSOR_ADDR,
        1,
        ack.carrier,
        UPLINK,
    )
    .expect("verifies");
    assert_eq!(state.forward, Forward::RetryIn60Minutes);

    // With nothing to earn, forwarding is off rather than worth retrying.
    relay.trust(0, SENSOR_ADDR, &root(), 2, 0, 0);
    let scan = relay.next_scan(7_000_000).expect("running");
    let frame = wor(&scan, 2);
    let Wake::Uplink { forward, .. } = relay
        .heard_wor(&scan, &frame, -90, 4, scan.start_us + 900_000)
        .expect("a trusted device")
    else {
        panic!("an uplink follows");
    };
    assert_eq!(forward, Forward::Disabled);
}

#[test]
fn the_acknowledgment_tells_the_device_when_the_relay_scanned() {
    let mut relay = relay();
    let scan = relay.next_scan(10_000_000).expect("running");
    let link = scan.link.with_preamble(400);
    let started = scan.start_us - 300_000;
    let ended = started + link.airtime_us(WOR_UPLINK_LEN);
    let frame = wor(&scan, 0);

    let Wake::Uplink { acknowledgment, .. } = relay
        .heard_wor(&scan, &frame, -90, 4, ended)
        .expect("a trusted device")
    else {
        panic!("an uplink follows");
    };
    let ack = acknowledgment.expect("acknowledged");
    let state = open_wor_ack(
        &ack.frame,
        &WorKeys::derive(&root(), SENSOR_ADDR),
        SENSOR_ADDR,
        0,
        ack.carrier,
        UPLINK,
    )
    .expect("verifies");
    assert_eq!(state.cad_periodicity, CadPeriodicity::Ms1000);
    assert_eq!(state.xtal_accuracy, XtalAccuracy::Ppm10);
    assert_eq!(state.cad_to_rx, CadToRx::Symbols2);
    assert_eq!(state.relay_data_rate, relay.device().data_rate());

    // What the device works out of it lands on the scan, never after it.
    let sync = Synchronization::from_ack(started, 400, link.symbol_time_us(), &state);
    assert!(
        sync.reference_us <= scan.start_us && scan.start_us - sync.reference_us < 1_000,
        "TREF {} against the scan at {}",
        sync.reference_us,
        scan.start_us
    );
}

#[test]
fn the_relay_forwards_what_it_heard_with_the_metadata_the_network_needs() {
    let mut relay = relay();
    let transmission = forward(&mut relay, b"21.5", 0);
    let sent = sent_by(&relay, &transmission);
    assert_eq!(sent.fport(), Some(LA_FPORT_RELAY));

    let forwarded = ForwardedUplink::parse(sent.payload()).expect("a forwarded uplink");
    assert_eq!(forwarded.frequency_hz, UPLINK.frequency_hz);
    assert_eq!(
        forwarded.metadata,
        UplinkMetadata {
            wor_channel: WorChannel::Default,
            rssi_dbm: -88,
            snr_db: 6,
            data_rate: UPLINK.data_rate,
        }
    );
    let uplink = sensor()
        .decode(forwarded.phy_payload, 0)
        .expect("the sensor's own frame, untouched");
    assert_eq!(uplink.payload(), b"21.5");
}

#[test]
fn an_uplink_the_relay_did_not_expect_is_refused() {
    let mut relay = relay();
    let stranger = Session::new(0x2601_0009, [0x33; 16], [0x44; 16]);
    let frame = stranger
        .encode_uplink(&Uplink::new(0, 1, b"hi"))
        .expect("an uplink");

    assert_eq!(
        relay.heard_uplink(frame.as_bytes(), -88, 6, 1_000_000),
        Err(RelayError::NotListening),
        "no WOR frame announced it",
    );

    let scan = relay.next_scan(0).expect("running");
    let announced = wor(&scan, 0);
    relay
        .heard_wor(&scan, &announced, -90, 4, scan.start_us + 900_000)
        .expect("a trusted device");
    assert_eq!(
        relay.heard_uplink(frame.as_bytes(), -88, 6, 2_000_000),
        Err(RelayError::Foreign),
        "another device's address",
    );

    let scan = relay.next_scan(3_000_000).expect("running");
    let announced = wor(&scan, 1);
    relay
        .heard_wor(&scan, &announced, -90, 4, scan.start_us + 900_000)
        .expect("a trusted device");
    let long = sensor()
        .encode_uplink(&Uplink::new(1, 1, &[0; 60]))
        .expect("an uplink");
    assert_eq!(
        relay.heard_uplink(long.as_bytes(), -88, 6, 4_000_000),
        Err(RelayError::Frame(LorawanError::PayloadTooLong)),
        "longer than the relay can forward at its own data rate",
    );
}

#[test]
fn the_relay_forwards_a_join_request_and_sends_its_accept_on() {
    const APP_KEY: [u8; 16] = [0x5A; 16];
    let mut relay = relay();
    let credentials = Device::new([0x11; 8], [0x22; 8], APP_KEY);
    let request = credentials.join_request(1);

    let scan = relay.next_scan(0).expect("running");
    let frame = wor_join_request(UPLINK).expect("a WOR frame");
    let ended = scan.start_us + 900_000;
    let Wake::JoinRequest { listen } = relay
        .heard_wor(&scan, &frame, -90, 4, ended)
        .expect("a join request follows")
    else {
        panic!("a join request");
    };
    assert_eq!(listen.start_us, ended + u64::from(WOR_DATA_DELAY_US));
    assert_eq!(listen.carrier, UPLINK);
    assert_eq!(listen.max_len, 23);

    let due = relay
        .heard_uplink(request.as_bytes(), -88, 6, listen.start_us + 100_000)
        .expect("forwarded");
    assert_eq!(relay.limits().join_request.tokens(), 3);
    let transmission = relay.forward(due).expect("sent");
    let sent = sent_by(&relay, &transmission);
    let forwarded = ForwardedUplink::parse(sent.payload()).expect("a forwarded uplink");
    assert_eq!(forwarded.phy_payload, request.as_bytes());

    // The network's accept comes back for the relay to send in the RXR window.
    let accept = JoinGrant::new(0x01, 0x13, 0x2601_2E43).accept(&APP_KEY, 1);
    let reply = relay_session()
        .encode_downlink(&Downlink::new(0, LA_FPORT_RELAY, accept.as_bytes()))
        .expect("a downlink");
    let RelayHeard::Downlink { downlink, .. } = relay
        .heard_in(ReceiveWindow::Rx1, reply.as_bytes(), 7)
        .expect("read")
    else {
        panic!("a downlink for the joining device");
    };
    assert_eq!(downlink.frame(), accept.as_bytes());
    assert_eq!(
        downlink.start_us,
        listen.start_us + 100_000 + u64::from(RXR_DELAY_US)
    );
    assert_eq!(
        downlink.carrier,
        Carrier::new(865_100_000, 5),
        "the WOR frequency, at the uplink's first window rate"
    );
}

#[test]
fn a_join_request_the_filter_drops_never_reaches_the_network() {
    let mut relay = relay();
    relay.join_filter_mut().set_default(FilterAction::Filter);
    let credentials = Device::new([0x11; 8], [0x22; 8], [0x5A; 16]);

    let scan = relay.next_scan(0).expect("running");
    let frame = wor_join_request(UPLINK).expect("a WOR frame");
    relay
        .heard_wor(&scan, &frame, -90, 4, scan.start_us + 900_000)
        .expect("a join request follows");
    assert_eq!(
        relay.heard_uplink(credentials.join_request(1).as_bytes(), -88, 6, 2_000_000),
        Err(RelayError::Filtered)
    );
    assert_eq!(
        relay.limits().join_request.tokens(),
        4,
        "a filtered request spends nothing",
    );
    assert_eq!(relay.forward_due(), None);
}

#[test]
fn a_downlink_for_nobody_and_one_too_long_for_the_window_are_not_sent_on() {
    let mut relay = relay();
    let payload = [0x60u8; 20];
    let reply = relay_session()
        .encode_downlink(&Downlink::new(0, LA_FPORT_RELAY, &payload))
        .expect("a downlink");
    relay.device_mut().send(1, b"hi", false, 0).expect("sent");
    let RelayHeard::Undeliverable { reason, .. } = relay
        .heard_in(ReceiveWindow::Rx1, reply.as_bytes(), 7)
        .expect("read")
    else {
        panic!("nothing waits on an answer");
    };
    assert_eq!(reason, RelayError::NothingAwaited);

    // A downlink the relay's own window carries, but the end device's does not, is
    // dropped: the relay runs at DR5 here, and the sensor's uplink came in at DR0.
    let mut relay = fast_relay();
    forward_at(
        &mut relay,
        Carrier::new(868_100_000, 0),
        b"21.5",
        0,
        3 * APART_US,
    );
    let long = [0x60u8; 100];
    let reply = relay_session()
        .encode_downlink(&Downlink::new(1, LA_FPORT_RELAY, &long))
        .expect("a downlink");
    let RelayHeard::Undeliverable { reason, .. } = relay
        .heard_in(ReceiveWindow::Rx1, reply.as_bytes(), 7)
        .expect("read")
    else {
        panic!("too long for the RXR window");
    };
    assert_eq!(reason, RelayError::Frame(LorawanError::PayloadTooLong));
}

#[test]
fn the_relay_holds_a_forward_its_own_answers_crowded_out() {
    let mut relay = relay();
    relay.trust(1, 0x2601_0002, &root(), 0, 63, 0);
    relay.trust(2, 0x2601_0003, &root(), 0, 63, 0);
    // Three six-byte read-backs outgrow the fifteen bytes of frame options an uplink
    // carries, so they take a frame of their own.
    network_sends(
        &mut relay,
        &[
            MacCommand::CtrlUplinkListReq {
                index: 0,
                action: 0,
            },
            MacCommand::CtrlUplinkListReq {
                index: 1,
                action: 0,
            },
            MacCommand::CtrlUplinkListReq {
                index: 2,
                action: 0,
            },
        ],
        0,
        APART_US,
    );

    let scan = relay.next_scan(2 * APART_US).expect("running");
    let frame = wor(&scan, 0);
    let Wake::Uplink { listen, .. } = relay
        .heard_wor(&scan, &frame, -90, 4, scan.start_us + 900_000)
        .expect("a trusted device")
    else {
        panic!("an uplink follows");
    };
    let listen = listen.expect("forwarding is available");
    let uplink = sensor()
        .encode_uplink(&Uplink::new(0, 1, b"21.5"))
        .expect("an uplink");
    let due = relay
        .heard_uplink(uplink.as_bytes(), -88, 6, listen.start_us + 60_000)
        .expect("forwarded");

    let answers = relay.forward(due).expect("sent");
    assert!(!answers.carries_payload, "the answers took the frame");
    assert_eq!(sent_by(&relay, &answers).fport(), Some(0), "answers alone");
    assert_eq!(relay.forward_due(), Some(due), "the forward still waits");

    relay
        .nothing_heard(due + 4_000_000)
        .expect("windows closed");
    let carried = relay.forward(due + APART_US).expect("sent");
    assert!(carried.carries_payload);
    assert_eq!(relay.forward_due(), None);
}

#[test]
fn a_relay_with_a_forward_waiting_does_not_take_on_another_device() {
    let mut relay = relay();
    let scan = relay.next_scan(0).expect("running");
    let frame = wor(&scan, 0);
    let Wake::Uplink { listen, .. } = relay
        .heard_wor(&scan, &frame, -90, 4, scan.start_us + 900_000)
        .expect("a trusted device")
    else {
        panic!("an uplink follows");
    };
    let listen = listen.expect("forwarding is available");
    let uplink = sensor()
        .encode_uplink(&Uplink::new(0, 1, b"21.5"))
        .expect("an uplink");
    relay
        .heard_uplink(uplink.as_bytes(), -88, 6, listen.start_us + 60_000)
        .expect("forwarded");

    let scan = relay.next_scan(3_000_000).expect("running");
    let another = wor(&scan, 1);
    assert_eq!(
        relay.heard_wor(&scan, &another, -90, 4, scan.start_us + 900_000),
        Err(RelayError::Busy)
    );
}

#[test]
fn a_stopped_relay_hears_nothing() {
    let mut relay = relay();
    let scan = relay.next_scan(0).expect("running");
    let frame = wor(&scan, 0);
    relay.stop();
    assert_eq!(
        relay.heard_wor(&scan, &frame, -90, 4, scan.start_us + 900_000),
        Err(RelayError::Stopped)
    );
}

#[test]
fn the_network_sets_every_forwarding_limit_at_once() {
    let mut relay = relay();
    let answers = network_says(
        &mut relay,
        MacCommand::ConfigureFwdLimitReq {
            reset_limit_counters: CounterReset::Full.code(),
            join_request_reload_rate: 2,
            notify_reload_rate: 0,
            global_uplink_reload_rate: 10,
            overall_reload_rate: UNLIMITED_RELAY_RATE,
            join_request_bucket_size: 1,
            notify_bucket_size: 0,
            global_uplink_bucket_size: 3,
            overall_bucket_size: 0,
        },
        0,
    );
    assert_eq!(answers, vec![MacCommand::ConfigureFwdLimitAns]);

    let limits = relay.limits();
    assert_eq!(
        (
            limits.join_request.reload_rate(),
            limits.join_request.size()
        ),
        (Some(2), 4)
    );
    assert_eq!(limits.join_request.tokens(), 4, "filled");
    assert_eq!(
        (limits.notify.reload_rate(), limits.notify.size()),
        (Some(0), 0)
    );
    assert!(limits.notify.disabled(), "nothing earned and nothing held");
    assert_eq!(
        limits.global_uplink.size(),
        120,
        "ten an hour, twelve hours of them"
    );
    assert_eq!(limits.overall.reload_rate(), None, "no limit");
    assert!(limits.overall.has_token());
}

#[test]
fn a_relay_forwards_nothing_once_its_overall_limit_is_spent() {
    let mut relay = relay();
    relay.next_scan(0).expect("running");
    network_says(
        &mut relay,
        MacCommand::ConfigureFwdLimitReq {
            reset_limit_counters: CounterReset::Empty.code(),
            join_request_reload_rate: 4,
            notify_reload_rate: 4,
            global_uplink_reload_rate: 8,
            overall_reload_rate: 8,
            join_request_bucket_size: 1,
            notify_bucket_size: 1,
            global_uplink_bucket_size: 1,
            overall_bucket_size: 1,
        },
        0,
    );
    assert_eq!(relay.limits().overall.tokens(), 0);

    let scan = relay.next_scan(3 * APART_US).expect("running");
    let frame = wor(&scan, 0);
    let Wake::Uplink {
        forward, listen, ..
    } = relay
        .heard_wor(&scan, &frame, -90, 4, scan.start_us + 900_000)
        .expect("a trusted device")
    else {
        panic!("an uplink follows");
    };
    assert_eq!(forward, Forward::RetryIn60Minutes);
    assert!(listen.is_none());

    let join = wor_join_request(UPLINK).expect("a WOR frame");
    let scan = relay.next_scan(3 * APART_US + 3_000_000).expect("running");
    assert_eq!(
        relay.heard_wor(&scan, &join, -90, 4, scan.start_us + 900_000),
        Err(RelayError::Limited)
    );
}

/// Sends the relay a downlink carrying commands, without an uplink to answer them.
fn network_sends(relay: &mut Relay<'_>, commands: &[MacCommand], fcnt_down: u32, at_us: u64) {
    let mut payload = [0u8; crate::MAX_PAYLOAD];
    let mut len = 0;
    for command in commands {
        len += command.encode(&mut payload[len..]).expect("encodes");
    }
    let downlink = relay_session()
        .encode_downlink(&Downlink::new(fcnt_down, 0, &payload[..len]))
        .expect("a downlink");
    relay
        .device_mut()
        .send_empty(at_us)
        .expect("an uplink to answer");
    let heard = relay
        .heard_in(ReceiveWindow::Rx1, downlink.as_bytes(), 7)
        .expect("read");
    assert!(matches!(heard, RelayHeard::Device(Heard::Data(_))));
}

/// The commands a relay's next uplink carries.
fn answers_of(relay: &mut Relay<'_>, at_us: u64) -> Vec<MacCommand> {
    let answer = relay
        .device_mut()
        .send_empty(at_us)
        .expect("an uplink carrying the answers");
    let sent = sent_by(relay, &answer);
    relay
        .nothing_heard(at_us + 10_000_000)
        .expect("windows closed");
    let source = if sent.fport() == Some(0) {
        sent.payload()
    } else {
        sent.fopts()
    };
    crate::mac::MacCommands::new(crate::Direction::Uplink, source)
        .filter_map(Result::ok)
        .collect()
}

/// Sends the relay one command and reads what it answers.
fn network_says(relay: &mut Relay<'_>, command: MacCommand, exchange: u32) -> Vec<MacCommand> {
    let at = APART_US * (2 * u64::from(exchange) + 1);
    network_sends(relay, &[command], exchange, at);
    answers_of(relay, at + APART_US)
}
