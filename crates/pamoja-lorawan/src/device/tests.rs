//! The device against a network played by this crate's own network half.
//!
//! Every exchange here is the one the specification describes, built from the frames a real
//! network would send: a `JoinGrant` signs the accept, a `Session` holding the same keys
//! encodes each downlink and decodes each uplink, and the MAC commands are encoded the way
//! a network encodes them. The expected answers are the bytes TS001-1.0.4 chapter 5 lays
//! out, not whatever the device happens to produce.

use pamoja_lora::region::{ChannelPlan, Cn470Plan, Region};

use super::*;
use crate::mac::{encode_all, MacCommand, MacCommands};
use crate::{CfList, Direction, Downlink, JoinGrant, JoinRequest, RxData};

const APP_KEY: [u8; 16] = [0x2B; 16];
const DEV_EUI: [u8; 8] = [0x70, 0xB3, 0xD5, 0x7E, 0xD0, 0x05, 0x12, 0x34];
const JOIN_EUI: [u8; 8] = [0x22; 8];
const DEV_ADDR: u32 = 0x2601_2E43;

/// Ten minutes, which clears any duty cycle off time a test's frames run up.
const LATER: u64 = 600_000_000;

fn settings() -> Settings {
    Settings::new(2, 20).with_seed(7)
}

fn device(region: Region, settings: Settings) -> EndDevice<'static> {
    EndDevice::new(
        region.plan(),
        Device::new(DEV_EUI, JOIN_EUI, APP_KEY),
        settings,
    )
    .expect("the plan fits")
}

/// The network side of every exchange.
struct Network {
    grant: JoinGrant,
    session: Option<Session>,
    fcnt_down: u32,
}

impl Network {
    fn new(grant: JoinGrant) -> Network {
        Network {
            grant,
            session: None,
            fcnt_down: 0,
        }
    }

    fn accept(&mut self, request: &Transmission) -> Vec<u8> {
        let request = JoinRequest::parse(request.frame.as_bytes(), &APP_KEY)
            .expect("the join request verifies");
        assert_eq!(request.dev_eui(), DEV_EUI);
        self.session = Some(self.grant.session(&APP_KEY, request.dev_nonce()));
        self.grant
            .accept(&APP_KEY, request.dev_nonce())
            .as_bytes()
            .to_vec()
    }

    fn read(&self, uplink: &Transmission, fcnt: u32) -> RxData {
        self.session
            .expect("joined")
            .decode(uplink.frame.as_bytes(), fcnt)
            .expect("the uplink verifies")
    }

    /// A downlink with frame options and no port.
    fn commands(&mut self, commands: &[MacCommand]) -> Vec<u8> {
        let mut fopts = [0u8; 15];
        let len = encode_all(commands, &mut fopts).expect("the commands fit");
        self.send(Downlink::empty(self.fcnt_down).with_fopts(&fopts[..len]))
    }

    /// A downlink with its commands on port 0.
    fn commands_on_port_zero(&mut self, commands: &[MacCommand]) -> Vec<u8> {
        let mut payload = [0u8; 64];
        let len = encode_all(commands, &mut payload).expect("the commands fit");
        self.send(Downlink::new(self.fcnt_down, 0, &payload[..len]))
    }

    fn send(&mut self, downlink: Downlink) -> Vec<u8> {
        let frame = self
            .session
            .expect("joined")
            .encode_downlink(&downlink)
            .expect("the downlink encodes")
            .as_bytes()
            .to_vec();
        self.fcnt_down += 1;
        frame
    }
}

/// A device joined to a network, and the network, at a time well clear of the join.
fn joined(region: Region, settings: Settings) -> (EndDevice<'static>, Network, u64) {
    joined_with(region, settings, JoinGrant::new(0x01, 0x13, DEV_ADDR))
}

fn joined_with(
    region: Region,
    settings: Settings,
    grant: JoinGrant,
) -> (EndDevice<'static>, Network, u64) {
    let mut device = device(region, settings);
    let mut network = Network::new(grant);
    let request = device.join(1, 0).expect("the join goes out");
    let accept = network.accept(&request);
    assert_eq!(
        device.heard(&accept, 0),
        Ok(Heard::Joined { dev_addr: DEV_ADDR })
    );
    (device, network, LATER)
}

/// The frame options an uplink carried.
fn fopts(network: &Network, uplink: &Transmission, fcnt: u32) -> Vec<u8> {
    network.read(uplink, fcnt).fopts().to_vec()
}

fn data(heard: Result<Heard, DeviceError>) -> Delivery {
    match heard {
        Ok(Heard::Data(delivery)) => delivery,
        other => panic!("expected a data downlink, got {other:?}"),
    }
}

#[test]
fn a_join_request_goes_out_on_a_default_channel_with_its_windows() {
    // RP002-1.0.5 table 9 and section 3.4.7; TS001-1.0.4 section 6.2.6 and RP002-1.0.5 3.3.
    let mut device = device(Region::Eu868, settings());
    let request = device.join(0x0102, 0).expect("the join goes out");

    assert!([868_100_000, 868_300_000, 868_500_000].contains(&request.frequency_hz));
    assert_eq!(
        request.data_rate, 5,
        "the first attempt is the fastest rate"
    );
    assert_eq!(request.link.spreading_factor(), 7);
    assert!(request.link.crc(), "an uplink carries a CRC");
    assert_eq!(
        request.output_dbm, 16,
        "the region's 16 dBm with no antenna gain"
    );

    let parsed = JoinRequest::parse(request.frame.as_bytes(), &APP_KEY).expect("verifies");
    assert_eq!(parsed.dev_nonce(), 0x0102);

    assert_eq!(request.rx1.delay_us, 5_000_000);
    assert_eq!(request.rx1.frequency_hz, request.frequency_hz);
    assert_eq!(request.rx1.data_rate, 5);
    assert!(!request.rx1.link.crc(), "a downlink carries none");
    assert_eq!(request.rx2.delay_us, 6_000_000);
    assert_eq!(request.rx2.frequency_hz, 869_525_000);
    assert_eq!(request.rx2.data_rate, 0);
    assert_eq!(request.rx2.link.spreading_factor(), 12);

    assert_eq!(device.join(3, 1), Err(DeviceError::Busy));
}

#[test]
fn an_unanswered_join_waits_a_random_second_or_three_then_tries_the_next_rate_down() {
    let mut device = device(Region::Eu868, settings().without_regional_duty_cycle());
    let first = device.join(1, 0).expect("goes out");
    assert_eq!(first.data_rate, 5);

    let Ok(Next::JoinAgain { not_before_us }) = device.nothing_heard(7_000_000) else {
        panic!("a join with no answer is tried again");
    };
    assert!((8_000_000..=10_000_000).contains(&not_before_us));
    assert_eq!(
        device.join(2, 7_500_000),
        Err(DeviceError::Wait {
            until_us: not_before_us
        })
    );

    let rates: Vec<u8> = (0..7)
        .map(|attempt| {
            let now = LATER * (attempt + 1);
            let rate = device
                .join(10 + attempt as u16, now)
                .expect("goes out")
                .data_rate;
            device.nothing_heard(now + 7_000_000).expect("closes");
            rate
        })
        .collect();
    assert_eq!(
        rates,
        [4, 3, 2, 1, 0, 5, 4],
        "every rate, fastest first, and round again"
    );
}

#[test]
fn join_airtime_is_held_to_thirty_six_seconds_in_the_first_hour() {
    // TS001-1.0.4 section 7, table 56.
    let mut device = device(Region::Eu868, settings().without_regional_duty_cycle());
    let mut now = 0;
    let mut airtime = 0;
    loop {
        match device.join(now as u16, now) {
            Ok(request) => {
                airtime += request.airtime_us;
                device.nothing_heard(now + 7_000_000).expect("closes");
                now += 10_000_000;
            }
            Err(DeviceError::Wait {
                until_us: 3_600_000_000,
            }) => break,
            Err(DeviceError::Wait { until_us }) => now = until_us,
            Err(other) => panic!("{other:?}"),
        }
        assert!(now < 3_600_000_000, "the budget ran out inside the hour");
    }
    assert!(airtime <= 36_000_000, "{airtime} us of join airtime");
    assert!(airtime > 30_000_000, "and nearly all of it was used");
}

#[test]
fn a_join_accept_brings_its_receive_settings_and_channel_list() {
    let list = CfList::frequencies([
        867_100_000,
        867_300_000,
        867_500_000,
        867_700_000,
        867_900_000,
    ])
    .expect("valid");
    let grant = JoinGrant::new(0x01, 0x13, DEV_ADDR)
        .with_dl_settings((2 << 4) | 3)
        .with_rx_delay(3)
        .with_cflist(list.to_bytes());
    let (mut device, network, now) = joined_with(Region::Eu868, settings(), grant);

    assert_eq!(device.rx2(), (869_525_000, 3));
    assert_eq!(device.receive_delay_us(), 3_000_000);
    let frequencies: Vec<u32> = device.channels().map(|(_, c)| c.uplink_hz).collect();
    assert_eq!(
        frequencies,
        [
            868_100_000,
            868_300_000,
            868_500_000,
            867_100_000,
            867_300_000,
            867_500_000,
            867_700_000,
            867_900_000
        ]
    );

    let uplink = device.send(2, b"21.5", false, now).expect("goes out");
    assert_eq!(
        uplink.data_rate, 5,
        "the first uplink keeps the join's rate"
    );
    assert_eq!(uplink.rx1.delay_us, 3_000_000);
    assert_eq!(uplink.rx2.delay_us, 4_000_000);
    assert_eq!(uplink.rx1.data_rate, 3, "DR5 with an RX1 offset of 2");
    assert_eq!(uplink.rx2.data_rate, 3);

    let read = network.read(&uplink, 0);
    assert_eq!(read.fport(), Some(2));
    assert_eq!(read.payload(), b"21.5");
    assert!(read.adr());
    assert_eq!(device.fcnt_up(), 1);
}

#[test]
fn an_accept_with_a_reserved_rx1_offset_is_ignored() {
    // RP002-1.0.5 section 3.4.7: RX1DROffset 6 and 7 are reserved, and a join accept carrying
    // one "SHALL be silently ignored".
    let mut device = device(Region::Eu868, settings());
    let mut network = Network::new(JoinGrant::new(0x01, 0x13, DEV_ADDR).with_dl_settings(6 << 4));
    let request = device.join(1, 0).expect("goes out");
    let accept = network.accept(&request);
    assert_eq!(device.heard(&accept, 0), Err(DeviceError::Refused));
    assert!(!device.is_joined());
}

#[test]
fn an_unconfirmed_uplink_goes_out_once_and_is_done() {
    let (mut device, _network, now) = joined(Region::Eu868, settings());
    device.send(2, b"one", false, now).expect("goes out");
    assert_eq!(device.send(2, b"two", false, now), Err(DeviceError::Busy));
    assert_eq!(device.nothing_heard(now + 3_000_000), Ok(Next::Done));
    assert_eq!(
        device.repeat(now + 3_000_000),
        Err(DeviceError::NothingPending)
    );
    device
        .send(2, b"two", false, now + LATER)
        .expect("the next one goes out");
}

#[test]
fn a_link_adr_request_is_taken_answered_and_its_repetition_kept() {
    // TS001-1.0.4 section 5.2: DR3, TXPower 2, channels 0 to 2, NbTrans 2, all acknowledged.
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[MacCommand::LinkAdrReq {
        data_rate: 3,
        tx_power: 2,
        channel_mask: 0b111,
        mask_control: 0,
        transmissions: 2,
    }]);
    let delivery = data(device.heard(&downlink, 5));
    assert_eq!(
        delivery.port(),
        None,
        "a frame of options alone reaches no application"
    );

    assert_eq!(device.data_rate(), 3);
    assert_eq!(device.transmissions(), 2);

    let second = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(second.data_rate, 3);
    assert_eq!(second.output_dbm, 12, "16 dBm less two steps of 2 dB");
    assert_eq!(
        fopts(&network, &second, 1),
        [0x03, 0x07],
        "LinkADRAns with power, data rate and channel mask acknowledged"
    );

    let Ok(Next::Repeat { not_before_us }) = device.nothing_heard(now + LATER + 3_000_000) else {
        panic!("NbTrans 2 sends it again");
    };
    assert_eq!(
        not_before_us,
        now + LATER + 3_000_000,
        "an unconfirmed repeat need not wait"
    );
    let again = device.repeat(now + 2 * LATER).expect("the repeat goes out");
    assert_eq!(again.frame, second.frame, "the same frame, counter and all");
    assert_eq!(
        device.nothing_heard(now + 2 * LATER + 3_000_000),
        Ok(Next::Done)
    );
    assert_eq!(
        device.fcnt_up(),
        2,
        "a repetition does not move the counter"
    );
}

#[test]
fn a_mask_enabling_an_undefined_channel_refuses_the_whole_request() {
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[MacCommand::LinkAdrReq {
        data_rate: 3,
        tx_power: 2,
        channel_mask: 0b10_0111,
        mask_control: 0,
        transmissions: 1,
    }]);
    data(device.heard(&downlink, 5));

    assert_eq!(device.data_rate(), 5, "nothing was taken");
    let next = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(
        fopts(&network, &next, 1),
        [0x03, 0x06],
        "channel mask refused"
    );
    assert_eq!(next.output_dbm, 16);
}

#[test]
fn a_block_of_requests_is_answered_once_in_1_0_3_and_once_each_in_1_0_4() {
    let requests = [
        MacCommand::LinkAdrReq {
            data_rate: 4,
            tx_power: 1,
            channel_mask: 0,
            mask_control: 0,
            transmissions: 1,
        },
        MacCommand::LinkAdrReq {
            data_rate: 4,
            tx_power: 1,
            channel_mask: 0b011,
            mask_control: 0,
            transmissions: 1,
        },
    ];
    for (version, answers) in [
        (Version::V1_0_3, vec![0x03, 0x07]),
        (Version::V1_0_4, vec![0x03, 0x07, 0x03, 0x07]),
    ] {
        let (mut device, mut network, now) =
            joined(Region::Eu868, settings().with_version(version));
        device.send(2, b"a", false, now).expect("goes out");
        let downlink = network.commands(&requests);
        data(device.heard(&downlink, 5));
        let channels: Vec<usize> = device.channels().map(|(index, _)| index).collect();
        assert_eq!(
            channels,
            [0, 1],
            "{version:?}: the block's mask, applied in order"
        );

        let next = device.send(2, b"b", false, now + LATER).expect("goes out");
        assert_eq!(fopts(&network, &next, 1), answers, "{version:?}");
    }
}

#[test]
fn with_adaptive_data_rate_off_a_1_0_4_device_takes_what_it_can() {
    // TS001-1.0.4 section 4.3.1.1: with the ADR bit unset, each part is accepted or refused.
    let (mut device, mut network, now) = joined(Region::Eu868, settings().with_adr(false));
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[MacCommand::LinkAdrReq {
        data_rate: 2,
        tx_power: 3,
        channel_mask: 0b1000,
        mask_control: 0,
        transmissions: 1,
    }]);
    data(device.heard(&downlink, 5));
    assert_eq!(device.data_rate(), 2, "the rate stood on its own");
    let next = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(next.output_dbm, 10, "and so did the power");
    assert!(!network.read(&next, 1).adr());
    assert_eq!(fopts(&network, &next, 1), [0x03, 0x06]);
}

#[test]
fn a_receive_window_setting_is_answered_on_every_uplink_until_a_downlink() {
    // TS001-1.0.4 section 5.4: RXParamSetupAns goes in "all uplinks until a Class A downlink
    // is received".
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[MacCommand::RxParamSetupReq {
        rx1_offset: 1,
        rx2_data_rate: 2,
        frequency_hz: 869_525_000,
    }]);
    data(device.heard(&downlink, 5));
    assert_eq!(device.rx2(), (869_525_000, 2));

    let second = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(fopts(&network, &second, 1), [0x05, 0x07]);
    assert_eq!(second.rx1.data_rate, 4);
    device
        .nothing_heard(now + LATER + 3_000_000)
        .expect("closes");

    let third = device
        .send(2, b"c", false, now + 2 * LATER)
        .expect("goes out");
    assert_eq!(
        fopts(&network, &third, 2),
        [0x05, 0x07],
        "repeated while no downlink came"
    );
    let downlink = network.send(Downlink::empty(network.fcnt_down));
    data(device.heard(&downlink, 5));

    let fourth = device
        .send(2, b"d", false, now + 3 * LATER)
        .expect("goes out");
    assert_eq!(
        fopts(&network, &fourth, 3),
        [] as [u8; 0],
        "a downlink settled it"
    );
}

#[test]
fn an_unusable_receive_window_setting_changes_nothing() {
    let (mut device, mut network, now) = joined(
        Region::Eu868,
        settings().with_tuning_range(863_000_000, 870_000_000),
    );
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[MacCommand::RxParamSetupReq {
        rx1_offset: 6,
        rx2_data_rate: 2,
        frequency_hz: 915_000_000,
    }]);
    data(device.heard(&downlink, 5));
    assert_eq!(device.rx2(), (869_525_000, 0));
    let next = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(
        fopts(&network, &next, 1),
        [0x05, 0x02],
        "the rate was fine; the offset and the frequency were not"
    );
}

#[test]
fn a_status_request_reports_the_battery_and_how_well_it_was_heard() {
    // TS001-1.0.4 section 5.5: the margin is the SNR, six bits signed, from -32 to 31.
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.set_battery(Battery::Level(128));
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[MacCommand::DevStatusReq]);
    data(device.heard(&downlink, 40));
    let next = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(fopts(&network, &next, 1), [0x06, 128, 31]);

    let status: Vec<MacCommand> = MacCommands::new(Direction::Uplink, &fopts(&network, &next, 1))
        .map(|command| command.expect("reads"))
        .collect();
    assert_eq!(
        status,
        [MacCommand::DevStatusAns {
            battery: 128,
            margin: 31
        }]
    );
}

#[test]
fn channels_are_created_moved_and_refused_where_the_specification_says() {
    // TS001-1.0.4 section 5.6.
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    // Seventeen bytes of commands, more than frame options hold, so they go on port 0.
    let downlink = network.commands_on_port_zero(&[
        MacCommand::NewChannelReq {
            index: 3,
            frequency_hz: 867_100_000,
            max_data_rate: 5,
            min_data_rate: 0,
        },
        MacCommand::NewChannelReq {
            index: 1,
            frequency_hz: 867_300_000,
            max_data_rate: 5,
            min_data_rate: 0,
        },
        MacCommand::DlChannelReq {
            index: 3,
            frequency_hz: 869_100_000,
        },
    ]);
    data(device.heard(&downlink, 5));

    let second = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(
        fopts(&network, &second, 1),
        [0x07, 0x03, 0x07, 0x00, 0x0a, 0x03],
        "channel 3 created, default channel 1 left alone, channel 3's downlink moved"
    );
    let downlink = network.commands(&[MacCommand::LinkAdrReq {
        data_rate: 5,
        tx_power: 0,
        channel_mask: 0b1000,
        mask_control: 0,
        transmissions: 1,
    }]);
    data(device.heard(&downlink, 5));

    // Only channel 3 is left enabled, so the next uplink shows where its window moved.
    let third = device
        .send(2, b"c", false, now + 2 * LATER)
        .expect("goes out");
    assert_eq!(third.frequency_hz, 867_100_000);
    assert_eq!(third.rx1.frequency_hz, 869_100_000);
}

#[test]
fn a_new_channel_command_does_nothing_but_answer_outside_the_table() {
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[
        MacCommand::NewChannelReq {
            index: 4,
            frequency_hz: 867_300_000,
            max_data_rate: 2,
            min_data_rate: 5,
        },
        MacCommand::DlChannelReq {
            index: 9,
            frequency_hz: 869_100_000,
        },
    ]);
    data(device.heard(&downlink, 5));
    let next = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(
        fopts(&network, &next, 1),
        [0x07, 0x01, 0x0a, 0x01],
        "a backwards rate range, and a downlink for a channel that does not exist"
    );
    assert_eq!(device.channels().count(), 3);
}

#[test]
fn a_new_channel_past_the_last_a_dynamic_plan_defines_is_refused() {
    // RP002-1.0.5 section 3.4.5: EU868 supports at most 80 channels, the last that
    // ChMaskCntl 4 reaches being channel 79.
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[
        MacCommand::NewChannelReq {
            index: 79,
            frequency_hz: 867_100_000,
            max_data_rate: 5,
            min_data_rate: 0,
        },
        MacCommand::NewChannelReq {
            index: 80,
            frequency_hz: 867_300_000,
            max_data_rate: 5,
            min_data_rate: 0,
        },
    ]);
    data(device.heard(&downlink, 5));
    let next = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(fopts(&network, &next, 1), [0x07, 0x03, 0x07, 0x00]);
    let created: Vec<usize> = device.channels().map(|(index, _)| index).collect();
    assert_eq!(created, [0, 1, 2, 79]);
}

#[test]
fn a_duty_cycle_request_holds_the_device_to_its_share() {
    // TS001-1.0.4 section 5.3: an aggregated share of 1/2^MaxDutyCycle, kept by waiting
    // airtime x (2^n - 1) after each frame.
    let (mut device, mut network, now) =
        joined(Region::Eu868, settings().without_regional_duty_cycle());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[MacCommand::DutyCycleReq { max_duty_cycle: 2 }]);
    data(device.heard(&downlink, 5));

    let second = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(fopts(&network, &second, 1), [0x04]);
    device
        .nothing_heard(now + LATER + 3_000_000)
        .expect("closes");
    let free = now + LATER + second.airtime_us * 4;
    assert_eq!(
        device.send(2, b"c", false, now + LATER + second.airtime_us),
        Err(DeviceError::Wait { until_us: free })
    );
    assert!(device.send(2, b"c", false, free).is_ok());
}

#[test]
fn a_timing_request_moves_both_windows_and_is_answered_until_a_downlink() {
    // TS001-1.0.4 section 5.7: "RX2 always opens 1s after RX1".
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[MacCommand::RxTimingSetupReq { delay: 5 }]);
    data(device.heard(&downlink, 5));
    let next = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(next.rx1.delay_us, 5_000_000);
    assert_eq!(next.rx2.delay_us, 6_000_000);
    assert_eq!(fopts(&network, &next, 1), [0x08]);
}

#[test]
fn transmit_parameters_apply_where_the_region_implements_them_and_are_dropped_elsewhere() {
    let request = MacCommand::TxParamSetupReq {
        max_eirp: 3,
        uplink_dwell: false,
        downlink_dwell: false,
    };

    // RP002-1.0.5 section 3.4.3: EU868 devices do not implement it, and TS001-1.0.4 section
    // 5.8 has them neither apply nor answer it.
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[request]);
    data(device.heard(&downlink, 5));
    let next = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(fopts(&network, &next, 1), [] as [u8; 0]);
    assert_eq!(next.output_dbm, 16);

    // AS923 implements it: MaxEIRP code 3 is 13 dBm, and the dwell limit comes off.
    let (mut device, mut network, now) = joined(Region::As923, settings());
    let small = device.send(2, &[0; 11], false, now).expect("goes out");
    assert_eq!(
        device.application_room(2),
        Ok(11),
        "AS923 starts under the 400 ms limit, where DR2 carries 11 bytes"
    );
    let downlink = network.commands(&[request]);
    data(device.heard(&downlink, 5));
    assert!(small.airtime_us <= 400_000);
    let next = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(fopts(&network, &next, 1), [0x09]);
    assert_eq!(next.output_dbm, 13);
    assert_eq!(
        device.application_room(2),
        Ok(115),
        "and without it DR2 carries 115"
    );
}

#[test]
fn the_region_s_sub_band_keeps_a_device_quiet_after_a_long_frame() {
    // RP002-1.0.5 section 3.4.2: the three default channels share 868.0 to 868.6 MHz at 1%.
    let (mut device, _network, now) = joined(Region::Eu868, settings());
    let uplink = device.send(2, &[0x55; 40], false, now).expect("goes out");
    device.nothing_heard(now + 3_000_000).expect("closes");
    let free = now + uplink.airtime_us * 100;
    assert_eq!(
        device.send(2, b"x", false, now + 3_000_000),
        Err(DeviceError::Wait { until_us: free }),
        "all three channels wait out 99 airtimes"
    );

    let (mut free_device, _network, now) =
        joined(Region::Eu868, settings().without_regional_duty_cycle());
    free_device
        .send(2, &[0x55; 40], false, now)
        .expect("goes out");
    free_device.nothing_heard(now + 3_000_000).expect("closes");
    assert!(free_device.send(2, b"x", false, now + 3_000_000).is_ok());
}

#[test]
fn a_downlink_counter_only_goes_forward_and_rolls_over_sixteen_bits() {
    let session = Session::new(DEV_ADDR, [0x31; 16], [0x32; 16]);
    let mut device = EndDevice::personalized(Region::Eu868.plan(), session, settings())
        .expect("dynamic")
        .with_frame_counters(10, Some(0xFFFF));
    assert_eq!(
        device.data_rate(),
        0,
        "a personalized device starts at its slowest rate"
    );

    device.send(2, b"a", false, 0).expect("goes out");
    let replay = session
        .encode_downlink(&Downlink::empty(0xFFFF))
        .expect("encodes");
    assert_eq!(
        device.heard(replay.as_bytes(), 0),
        Err(DeviceError::Replayed)
    );

    let rolled = session
        .encode_downlink(&Downlink::empty(0x1_0000))
        .expect("encodes");
    data(device.heard(rolled.as_bytes(), 0));
    assert_eq!(device.fcnt_down(), Some(0x1_0000));
}

#[test]
fn a_1_0_3_device_refuses_a_counter_too_far_ahead() {
    // LoRaWAN 1.0.3 section 4.3.1.5 and RP002-1.0.5 section 3.3: MAX_FCNT_GAP 16384.
    let session = Session::new(DEV_ADDR, [0x31; 16], [0x32; 16]);
    for (version, expected) in [
        (Version::V1_0_3, Err(DeviceError::CounterGap)),
        (Version::V1_0_4, Ok(())),
    ] {
        let mut device = EndDevice::personalized(
            Region::Eu868.plan(),
            session,
            settings().with_version(version),
        )
        .expect("dynamic")
        .with_frame_counters(0, Some(0));
        device.send(2, b"a", false, 0).expect("goes out");
        let far = session
            .encode_downlink(&Downlink::empty(16_384))
            .expect("encodes");
        assert_eq!(
            device.heard(far.as_bytes(), 0).map(|_| ()),
            expected,
            "{version:?}"
        );
    }
}

#[test]
fn a_confirmed_uplink_is_acknowledged_or_retried_after_a_timeout() {
    // TS001-1.0.4 section 4.3.1.3: RETRANSMIT_TIMEOUT after RX2 before the next transmission.
    let (mut device, mut network, now) =
        joined(Region::Eu868, settings().without_regional_duty_cycle());
    device.send(2, b"a", true, now).expect("goes out");
    let ack = network.send(Downlink::empty(network.fcnt_down).with_ack());
    assert!(data(device.heard(&ack, 5)).acknowledged());

    let closed = now + LATER + 3_000_000;
    let unanswered = device.send(2, b"b", true, now + LATER).expect("goes out");
    assert!(network.read(&unanswered, 1).confirmed());
    assert_eq!(device.nothing_heard(closed), Ok(Next::Unacknowledged));
    let Err(DeviceError::Wait { until_us }) = device.send(2, b"c", false, closed) else {
        panic!("a new frame waits too");
    };
    assert!((closed + 1_000_000..=closed + 3_000_000).contains(&until_us));
    assert!(device.send(2, b"c", false, until_us).is_ok());
}

#[test]
fn a_confirmed_downlink_is_acknowledged_by_the_next_uplink_only() {
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let confirmed = network.send(Downlink::new(network.fcnt_down, 3, b"open").confirmed());
    let delivery = data(device.heard(&confirmed, 5));
    assert!(delivery.confirmed());
    assert_eq!(delivery.port(), Some(3));
    assert_eq!(delivery.payload(), b"open");

    let next = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert!(network.read(&next, 1).ack());
    device
        .nothing_heard(now + LATER + 3_000_000)
        .expect("closes");
    let after = device
        .send(2, b"c", false, now + 2 * LATER)
        .expect("goes out");
    assert!(!network.read(&after, 2).ack());
}

#[test]
fn a_quiet_network_brings_back_power_then_steps_the_rate_down() {
    // TS001-1.0.4 table 9, through the device: ADRACKReq from 64, default power at 96, one
    // rate down at 128.
    let (mut device, mut network, mut now) =
        joined(Region::Eu868, settings().without_regional_duty_cycle());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[MacCommand::LinkAdrReq {
        data_rate: 5,
        tx_power: 3,
        channel_mask: 0b111,
        mask_control: 0,
        transmissions: 1,
    }]);
    data(device.heard(&downlink, 5));

    let mut seen = Vec::new();
    for count in 1..=128u32 {
        now += LATER;
        let uplink = device.send(2, b"x", false, now).expect("goes out");
        let read = network.read(&uplink, count);
        if matches!(count, 63 | 64 | 95 | 96 | 127 | 128) {
            seen.push((
                count,
                read.adr_ack_req(),
                uplink.output_dbm,
                uplink.data_rate,
            ));
        }
        device.nothing_heard(now + 3_000_000).expect("closes");
    }
    assert_eq!(
        seen,
        [
            (63, false, 10, 5),
            (64, true, 10, 5),
            (95, true, 10, 5),
            (96, true, 16, 5),
            (127, true, 16, 5),
            (128, true, 16, 4),
        ]
    );
}

#[test]
fn answers_that_crowd_out_the_payload_go_first() {
    // TS001-1.0.4 table 15: MAC answers before the application payload.
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands_on_port_zero(&[MacCommand::DevStatusReq; 8]);
    data(device.heard(&downlink, 5));

    let crowded = device
        .send(2, b"reading", false, now + LATER)
        .expect("goes out");
    assert!(!crowded.carries_payload);
    let read = network.read(&crowded, 1);
    assert_eq!(
        read.fport(),
        Some(0),
        "twenty-four bytes of answers go on port 0"
    );
    assert_eq!(read.payload().len(), 24);
    device
        .nothing_heard(now + LATER + 3_000_000)
        .expect("closes");

    let clear = device
        .send(2, b"reading", false, now + 2 * LATER)
        .expect("goes out");
    assert!(clear.carries_payload);
    assert_eq!(network.read(&clear, 2).payload(), b"reading");
}

#[test]
fn answers_that_leave_no_room_for_a_full_payload_go_alone_in_the_options() {
    let session = Session::new(DEV_ADDR, [0x31; 16], [0x32; 16]);
    let mut device =
        EndDevice::personalized(Region::Eu868.plan(), session, settings()).expect("dynamic");
    device.send(2, b"a", false, 0).expect("goes out");
    let status = session
        .encode_downlink(&Downlink::empty(0).with_fopts(&[0x06]))
        .expect("encodes");
    data(device.heard(status.as_bytes(), 5));

    // DR0 carries 51 bytes; three bytes of answer and 50 of payload do not fit.
    let frame = device.send(2, &[0; 50], false, LATER).expect("goes out");
    assert!(!frame.carries_payload);
    let read = session.decode(frame.frame.as_bytes(), 1).expect("verifies");
    assert_eq!(read.fport(), None);
    assert_eq!(read.fopts().len(), 3);
}

#[test]
fn a_payload_the_data_rate_cannot_carry_is_refused() {
    let session = Session::new(DEV_ADDR, [0x31; 16], [0x32; 16]);
    let mut device =
        EndDevice::personalized(Region::Eu868.plan(), session, settings()).expect("dynamic");
    assert_eq!(
        device.send(2, &[0; 52], false, 0),
        Err(DeviceError::PayloadTooLong { max: 51 })
    );
    assert!(device.send(2, &[0; 51], false, 0).is_ok());
}

#[test]
fn another_device_s_frame_leaves_the_windows_open() {
    let (mut device, network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let stranger = Session::new(DEV_ADDR + 1, [0x31; 16], [0x32; 16])
        .encode_downlink(&Downlink::empty(0))
        .expect("encodes");
    assert_eq!(
        device.heard(stranger.as_bytes(), 5),
        Err(DeviceError::Foreign)
    );
    assert_eq!(device.nothing_heard(now + 3_000_000), Ok(Next::Done));
    let _ = network;
}

#[test]
fn a_downlink_longer_than_its_window_carries_is_discarded() {
    // TS001-1.0.4 section 4.1: a MACPayload longer than M for the data rate the frame was
    // received at is silently discarded. RP002-1.0.5 table 16 gives EU868 DR0 an M of 59
    // bytes and DR5 one of 250.
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    let uplink = device.send(2, b"a", false, now).expect("goes out");
    assert_eq!((uplink.rx1.data_rate, uplink.rx2.data_rate), (5, 0));

    // A 52-byte payload makes a MACPayload of 60: header 7, port 1, payload 52.
    let long = network.send(Downlink::new(network.fcnt_down, 3, &[0x5A; 52]));
    assert_eq!(
        device.heard_in(ReceiveWindow::Rx2, &long, 5),
        Err(DeviceError::Frame(LorawanError::PayloadTooLong))
    );
    assert_eq!(device.fcnt_down(), None, "the frame was not processed");
    let delivery = data(device.heard_in(ReceiveWindow::Rx1, &long, 5));
    assert_eq!(delivery.payload(), &[0x5A; 52]);

    // Exactly M fits.
    device.send(2, b"b", false, now + LATER).expect("goes out");
    let full = network.send(Downlink::new(network.fcnt_down, 3, &[0x5A; 51]));
    assert_eq!(
        data(device.heard_in(ReceiveWindow::Rx2, &full, 5))
            .payload()
            .len(),
        51
    );

    // Without the window, the frame is held to the longer of the two.
    device
        .send(2, b"c", false, now + 2 * LATER)
        .expect("goes out");
    let longest = network.send(Downlink::new(network.fcnt_down, 3, &[0x5A; 242]));
    assert_eq!(data(device.heard(&longest, 5)).payload().len(), 242);
}

#[test]
fn a_confirmed_uplink_answered_without_an_acknowledgment_waits_out_the_timeout() {
    // TS001-1.0.4 section 4.3.1.3: a device that asked for an acknowledgment and has not had
    // one waits RETRANSMIT_TIMEOUT after RECEIVE_DELAY2 before sending again, even though the
    // downlink ended the uplink's repetitions.
    let (mut device, mut network, now) =
        joined(Region::Eu868, settings().without_regional_duty_cycle());
    let uplink = device.send(2, b"a", true, now).expect("goes out");
    let answer = network.send(Downlink::new(network.fcnt_down, 3, b"no ack"));
    assert!(!data(device.heard(&answer, 5)).acknowledged());

    let rx2_opens = now + uplink.airtime_us + u64::from(uplink.rx2.delay_us);
    let Err(DeviceError::Wait { until_us }) = device.send(2, b"b", false, now + 1) else {
        panic!("the next uplink waits");
    };
    assert!((rx2_opens + 1_000_000..=rx2_opens + 3_000_000).contains(&until_us));
    assert!(device.send(2, b"b", false, until_us).is_ok());

    // An acknowledged one, or an unconfirmed one, leaves the air free at once.
    let answer = network.send(Downlink::empty(network.fcnt_down).with_ack());
    data(device.heard(&answer, 5));
    device.send(2, b"c", true, until_us + 1).expect("goes out");
    let answer = network.send(Downlink::empty(network.fcnt_down).with_ack());
    assert!(data(device.heard(&answer, 5)).acknowledged());
    assert!(device.send(2, b"d", false, until_us + 2).is_ok());
}

#[test]
fn a_link_check_and_the_time_are_asked_for_and_read_back() {
    let (mut device, mut network, now) = joined(Region::Eu868, settings());
    device.request_link_check();
    device.request_device_time();
    let uplink = device.send(2, b"a", false, now).expect("goes out");
    assert_eq!(fopts(&network, &uplink, 0), [0x02, 0x0d]);

    let downlink = network.commands(&[
        MacCommand::LinkCheckAns {
            margin: 20,
            gateways: 3,
        },
        MacCommand::DeviceTimeAns {
            seconds: 1_139_322_288,
            fraction: 128,
        },
    ]);
    let delivery = data(device.heard(&downlink, 5));
    assert_eq!(
        delivery.link_check(),
        Some(LinkCheck {
            margin_db: 20,
            gateways: 3
        })
    );
    assert_eq!(
        delivery.device_time(),
        Some(DeviceTime {
            gps_seconds: 1_139_322_288,
            fraction: 128
        })
    );
}

#[test]
fn an_asian_device_joins_only_at_the_rates_a_dwell_limit_allows() {
    // RP002-1.0.5 table 66: DR2 to DR5.
    let mut device = device(Region::As923, settings().without_regional_duty_cycle());
    let rates: Vec<u8> = (0..5u64)
        .map(|attempt| {
            let now = LATER * attempt;
            let request = device.join(attempt as u16, now).expect("goes out");
            assert!(
                request.airtime_us <= 400_000,
                "DR{} fits the dwell limit",
                request.data_rate
            );
            device.nothing_heard(now + 7_000_000).expect("closes");
            request.data_rate
        })
        .collect();
    assert_eq!(rates, [5, 4, 3, 2, 5]);
}

/// The number of a US902-928 uplink channel from its frequency: 0 to 63 every 200 kHz from
/// 902.3 MHz, and 64 to 71 every 1.6 MHz from 903.0 MHz, which never land on the same
/// frequency.
fn american_channel(hz: u32) -> u32 {
    if (hz - 902_300_000).is_multiple_of(200_000) {
        (hz - 902_300_000) / 200_000
    } else {
        64 + (hz - 903_000_000) / 1_600_000
    }
}

/// Joins again and again until a request goes out on one of `frequencies`, and returns it
/// with the time it went out.
fn join_on(device: &mut EndDevice<'static>, frequencies: &[u32]) -> (Transmission, u64) {
    let mut now = 0;
    for nonce in 0..400u16 {
        match device.join(nonce, now) {
            Ok(request) if frequencies.contains(&request.frequency_hz) => return (request, now),
            Ok(_) => {
                device.nothing_heard(now + 7_000_000).expect("closes");
                now += 10_000_000;
            }
            Err(DeviceError::Wait { until_us }) => now = until_us,
            Err(other) => panic!("{other:?}"),
        }
    }
    panic!("no request went out on {frequencies:?}");
}

#[test]
fn an_american_join_probes_each_group_of_eight_then_a_wide_channel_until_all_have_gone() {
    // RP002-1.0.5 section 3.5.2: "Random channel from [0-7], followed by [8-15] ... [56-63],
    // then 64", then 65 on the second pass, and 71 on the last.
    let mut device = device(Region::Us915, settings());
    let mut seen = Vec::new();
    for attempt in 0..72u32 {
        let now = u64::from(attempt) * 10_000_000;
        let request = device.join(attempt as u16, now).expect("goes out");
        let channel = american_channel(request.frequency_hz);
        if attempt % 9 < 8 {
            assert_eq!(
                channel / 8,
                attempt % 9,
                "attempt {attempt} probes its group"
            );
            assert_eq!(request.data_rate, 0);
            assert!(request.airtime_us <= 400_000, "inside the FCC dwell time");
            assert_eq!(request.rx1.data_rate, 10, "RP002-1.0.5 table 26, DR0");
        } else {
            assert_eq!(channel, 64 + attempt / 9, "the wide channels go in order");
            assert_eq!(request.data_rate, 4);
            assert_eq!(request.rx1.data_rate, 13, "table 26, DR4");
        }
        assert_eq!(
            request.rx1.frequency_hz,
            923_300_000 + 600_000 * (channel % 8),
            "section 3.5.7: the downlink channel is the uplink channel modulo 8"
        );
        assert_eq!(
            (request.rx2.frequency_hz, request.rx2.data_rate),
            (923_300_000, 8)
        );
        seen.push(channel);
        device.nothing_heard(now + 7_000_000).expect("closes");
    }
    seen.sort_unstable();
    assert_eq!(seen, (0..72).collect::<Vec<u32>>(), "every channel once");

    let again = device.join(72, 720_000_000).expect("a new cycle");
    assert_eq!(american_channel(again.frequency_hz) / 8, 0);
}

#[test]
fn an_american_accept_lists_the_sub_band_the_network_listens_on() {
    // RP002-1.0.5 sections 3.5.4 and 3.5.7: a type 1 list enabling channels 8 to 15 and 65.
    let list = CfList::channel_masks([0xFF00, 0, 0, 0, 0x0002, 0]);
    let grant = JoinGrant::new(0x01, 0x13, DEV_ADDR)
        .with_dl_settings(8)
        .with_cflist(list.to_bytes());
    let (mut device, network, now) = joined_with(Region::Us915, settings(), grant);

    let channels: Vec<usize> = device.channels().map(|(index, _)| index).collect();
    assert_eq!(channels, [8, 9, 10, 11, 12, 13, 14, 15, 65]);
    assert_eq!(device.data_rate(), 0, "the first join went out at DR0");

    for fcnt in 0..16u32 {
        let at = now + u64::from(fcnt) * LATER;
        let uplink = device.send(2, b"21.5", false, at).expect("goes out");
        let channel = american_channel(uplink.frequency_hz);
        assert!((8..16).contains(&channel), "channel {channel}");
        assert_eq!(
            uplink.rx1.frequency_hz,
            923_300_000 + 600_000 * (channel % 8)
        );
        assert_eq!(
            (uplink.rx2.frequency_hz, uplink.rx2.data_rate),
            (923_300_000, 8)
        );
        assert_eq!(network.read(&uplink, fcnt).payload(), b"21.5");
        device.nothing_heard(at + 3_000_000).expect("closes");
    }
}

#[test]
fn an_american_network_narrows_a_device_to_one_sub_band_either_way_the_note_gives() {
    // RP002-1.0.5 section 3.5.5: ChMaskCntl 7 then 0, or 5 alone.
    let (mut device, mut network, now) = joined(Region::Us915, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[
        MacCommand::LinkAdrReq {
            data_rate: 3,
            tx_power: 5,
            channel_mask: 0x0000,
            mask_control: 7,
            transmissions: 1,
        },
        MacCommand::LinkAdrReq {
            data_rate: 3,
            tx_power: 5,
            channel_mask: 0x00FF,
            mask_control: 0,
            transmissions: 1,
        },
    ]);
    data(device.heard(&downlink, 5));
    let channels: Vec<usize> = device.channels().map(|(index, _)| index).collect();
    assert_eq!(channels, [0, 1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(device.data_rate(), 3);

    let second = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(
        fopts(&network, &second, 1),
        [0x03, 0x07, 0x03, 0x07],
        "one LinkADRAns for each request of the block"
    );
    assert!(american_channel(second.frequency_hz) < 8);
    assert_eq!(second.data_rate, 3);
    assert_eq!(second.rx1.data_rate, 13, "table 26, DR3");
    assert_eq!(
        second.output_dbm, 20,
        "30 dBm conducted less five steps of 2 dB"
    );

    let downlink = network.commands(&[MacCommand::LinkAdrReq {
        data_rate: 4,
        tx_power: 5,
        channel_mask: 0x0002,
        mask_control: 5,
        transmissions: 1,
    }]);
    data(device.heard(&downlink, 5));
    let channels: Vec<usize> = device.channels().map(|(index, _)| index).collect();
    assert_eq!(
        channels,
        [8, 9, 10, 11, 12, 13, 14, 15, 65],
        "the second bank and its 500 kHz channel"
    );
    let third = device
        .send(2, b"c", false, now + 2 * LATER)
        .expect("goes out");
    assert_eq!(fopts(&network, &third, 2), [0x03, 0x07]);
    assert_eq!(
        (third.frequency_hz, third.data_rate),
        (904_600_000, 4),
        "DR4 goes out on the one 500 kHz channel left"
    );
    assert_eq!(third.rx1.frequency_hz, 923_900_000);
}

#[test]
fn a_fixed_plan_drops_channel_commands_unanswered() {
    // TS001-1.0.4 section 5.6: a fixed plan does not implement NewChannelReq or DlChannelReq.
    let (mut device, mut network, now) = joined(Region::Us915, settings());
    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[
        MacCommand::NewChannelReq {
            index: 3,
            frequency_hz: 905_000_000,
            max_data_rate: 3,
            min_data_rate: 0,
        },
        MacCommand::DlChannelReq {
            index: 3,
            frequency_hz: 925_000_000,
        },
    ]);
    data(device.heard(&downlink, 5));
    let next = device.send(2, b"b", false, now + LATER).expect("goes out");
    assert_eq!(fopts(&network, &next, 1), [] as [u8; 0]);
    assert_eq!(device.channels().count(), 72);
}

#[test]
fn american_power_is_conducted_and_only_gain_past_six_dbi_comes_off() {
    // RP002-1.0.5 table 22, and section 3.5.2 on antennas above 6 dBi.
    let radio = Settings::new(2, 30).with_seed(7);
    for (gain, want) in [(0, 30), (6, 30), (9, 27)] {
        let mut device = device(Region::Us915, radio.with_antenna_gain(gain));
        let request = device.join(1, 0).expect("goes out");
        assert_eq!(request.output_dbm, want, "a {gain} dBi antenna");
    }
    let mut european = device(Region::Eu868, radio.with_antenna_gain(3));
    assert_eq!(
        european.join(1, 0).expect("goes out").output_dbm,
        13,
        "a radiated limit takes all of the gain off"
    );
}

#[test]
fn an_australian_device_joins_at_dr2_inside_its_dwell_limit_and_at_dr6_on_a_wide_channel() {
    // RP002-1.0.5 section 3.8.2.
    let mut device = device(Region::Au915, settings());
    for attempt in 0..9u32 {
        let now = u64::from(attempt) * 10_000_000;
        let request = device.join(attempt as u16, now).expect("goes out");
        if attempt < 8 {
            let channel = (request.frequency_hz - 915_200_000) / 200_000;
            assert_eq!(channel / 8, attempt);
            assert_eq!(request.data_rate, 2);
            assert!(request.airtime_us <= 400_000);
            assert_eq!(
                request.rx1.frequency_hz,
                923_300_000 + 600_000 * (channel % 8)
            );
            assert_eq!(request.rx1.data_rate, 10, "table 46, DR2");
        } else {
            assert_eq!((request.frequency_hz, request.data_rate), (915_900_000, 6));
            assert_eq!(request.rx1.frequency_hz, 923_300_000);
            assert_eq!(request.rx1.data_rate, 13, "table 46, DR6");
        }
        device.nothing_heard(now + 7_000_000).expect("closes");
    }
}

#[test]
fn a_chinese_device_takes_the_plan_its_common_join_channel_belongs_to() {
    // RP002-1.0.5 section 3.9.2 and table 49, with the RX2 frequencies of tables 57 and 58 and
    // section 3.9.7.2: each row is the uplink, where the accept is due, and RX2 once joined.
    struct Case {
        rows: &'static [(u32, u32, u32)],
        channels: usize,
        first_uplink_hz: u32,
    }
    let cases = [
        // Common join channels 0 to 7, the 20 MHz antenna's plan A.
        Case {
            rows: &[
                (470_900_000, 484_500_000, 485_300_000),
                (472_500_000, 486_100_000, 486_900_000),
                (474_100_000, 487_700_000, 488_500_000),
                (475_700_000, 489_300_000, 490_100_000),
                (504_100_000, 490_900_000, 491_700_000),
                (505_700_000, 492_500_000, 493_300_000),
                (507_300_000, 494_100_000, 494_900_000),
                (508_900_000, 495_700_000, 496_500_000),
            ],
            channels: 64,
            first_uplink_hz: 470_300_000,
        },
        // Common join channels 8 and 9, plan B.
        Case {
            rows: &[
                (479_900_000, 479_900_000, 478_300_000),
                (499_900_000, 499_900_000, 498_300_000),
            ],
            channels: 64,
            first_uplink_hz: 476_900_000,
        },
        // Common join channels 15 to 19, the 26 MHz antenna's plan B.
        Case {
            rows: &[
                (480_300_000, 502_500_000, 502_500_000),
                (482_300_000, 502_500_000, 502_500_000),
                (484_300_000, 502_500_000, 502_500_000),
                (486_300_000, 502_500_000, 502_500_000),
                (488_300_000, 502_500_000, 502_500_000),
            ],
            channels: 48,
            first_uplink_hz: 480_300_000,
        },
    ];
    for case in cases {
        let uplinks: Vec<u32> = case.rows.iter().map(|row| row.0).collect();
        let mut device = device(Region::Cn470, settings());
        let (request, at) = join_on(&mut device, &uplinks);
        let &(_, accept_hz, rx2_hz) = case
            .rows
            .iter()
            .find(|row| row.0 == request.frequency_hz)
            .expect("one of the rows");
        assert_eq!(request.rx1.frequency_hz, accept_hz);
        assert_eq!(request.rx1.data_rate, request.data_rate);
        assert_eq!(
            (request.rx2.frequency_hz, request.rx2.data_rate),
            (accept_hz, 1),
            "both windows listen where the accept is due"
        );
        assert!(
            (1..=5).contains(&request.data_rate),
            "DR0 carries nothing here"
        );

        let mut network = Network::new(JoinGrant::new(0x01, 0x13, DEV_ADDR).with_dl_settings(1));
        let accept = network.accept(&request);
        assert_eq!(
            device.heard(&accept, 0),
            Ok(Heard::Joined { dev_addr: DEV_ADDR })
        );
        assert_eq!(device.rx2(), (rx2_hz, 1));
        assert_eq!(device.channels().count(), case.channels);
        assert_eq!(
            device.channels().next().map(|(_, c)| c.uplink_hz),
            Some(case.first_uplink_hz)
        );

        let uplink = device
            .send(2, b"21.5", false, at + LATER)
            .expect("goes out");
        let (_, channel) = device
            .channels()
            .find(|(_, c)| c.uplink_hz == uplink.frequency_hz)
            .expect("one of the plan's channels");
        assert_eq!(uplink.rx1.frequency_hz, channel.downlink_hz);
        assert_eq!(uplink.rx2.frequency_hz, rx2_hz);
        assert_eq!(network.read(&uplink, 0).payload(), b"21.5");
    }
}

#[test]
fn a_chinese_26_mhz_device_answers_modulo_24_and_reads_three_mask_groups() {
    // RP002-1.0.5 sections 3.9.4 and 3.9.7.2.
    let mut device = device(Region::Cn470, settings());
    let (request, at) = join_on(
        &mut device,
        &[
            470_300_000,
            472_300_000,
            474_300_000,
            476_300_000,
            478_300_000,
        ],
    );
    let list = CfList::channel_masks([0, 0, 0x0003, 0, 0, 0]);
    let mut network = Network::new(
        JoinGrant::new(0x01, 0x13, DEV_ADDR)
            .with_dl_settings(1)
            .with_cflist(list.to_bytes()),
    );
    let accept = network.accept(&request);
    assert!(device.heard(&accept, 0).is_ok());

    let channels: Vec<(usize, u32, u32)> = device
        .channels()
        .map(|(index, c)| (index, c.uplink_hz, c.downlink_hz))
        .collect();
    assert_eq!(
        channels,
        [
            (32, 476_700_000, 491_700_000),
            (33, 476_900_000, 491_900_000)
        ],
        "channels 32 and 33 answer on downlink channels 8 and 9"
    );
    let uplink = device.send(2, b"x", false, at + LATER).expect("goes out");
    assert_eq!(
        (uplink.rx2.frequency_hz, uplink.rx2.data_rate),
        (492_500_000, 1)
    );
}

#[test]
fn the_ninety_six_channel_plan_answers_each_channel_modulo_forty_eight() {
    // LoRaWAN 1.0.3 Regional Parameters revision A, sections 2.7.2 and 2.7.7.
    let mut device = EndDevice::new(
        Cn470Plan::Channels96.plan(),
        Device::new(DEV_EUI, JOIN_EUI, APP_KEY),
        settings(),
    )
    .expect("96 channels fit");
    let request = device.join(1, 0).expect("goes out");
    let channel = (request.frequency_hz - 470_300_000) / 200_000;
    assert!(channel < 96);
    assert_eq!(request.data_rate, 5, "DR5 first");
    assert_eq!(
        request.rx1.frequency_hz,
        500_300_000 + 200_000 * (channel % 48)
    );
    assert_eq!(
        (request.rx2.frequency_hz, request.rx2.data_rate),
        (505_300_000, 0)
    );

    let mut network = Network::new(JoinGrant::new(0x01, 0x13, DEV_ADDR));
    let accept = network.accept(&request);
    assert!(device.heard(&accept, 0).is_ok());
    assert_eq!(device.channels().count(), 96);
    for fcnt in 0..8u32 {
        let at = LATER * u64::from(fcnt + 1);
        let uplink = device.send(2, b"x", false, at).expect("goes out");
        let channel = (uplink.frequency_hz - 470_300_000) / 200_000;
        assert_eq!(
            uplink.rx1.frequency_hz,
            500_300_000 + 200_000 * (channel % 48)
        );
        device.nothing_heard(at + 3_000_000).expect("closes");
    }
}

#[test]
fn a_fixed_plan_spans_its_uplink_and_downlink_channels() {
    assert_eq!(
        device(Region::Us915, settings()).frequency_span(),
        (902_300_000, 927_500_000)
    );
    assert_eq!(
        device(Region::Cn470, settings()).frequency_span(),
        (470_300_000, 509_700_000)
    );
}

#[test]
fn port_zero_belongs_to_the_mac_layer() {
    let (mut device, _network, now) = joined(Region::Eu868, settings());
    assert_eq!(
        device.send(0, b"x", false, now),
        Err(DeviceError::Frame(LorawanError::MalformedFrame))
    );
}

#[test]
fn a_european_433_device_joins_on_its_own_band() {
    let mut device = device(Region::Eu433, settings());
    let request = device.join(1, 0).expect("goes out");
    assert!([433_175_000, 433_375_000, 433_575_000].contains(&request.frequency_hz));
    assert_eq!(request.rx2.frequency_hz, 434_665_000);
}

#[test]
fn the_frequency_span_covers_every_channel_and_the_second_window() {
    let device = device(Region::Eu868, settings());
    assert_eq!(device.frequency_span(), (868_100_000, 869_525_000));
    let device = device_for_433();
    assert_eq!(device.frequency_span(), (433_175_000, 434_665_000));
}

fn device_for_433() -> EndDevice<'static> {
    device(Region::Eu433, settings())
}

/// A device on the same plan with the same identity and settings as `device` was built with.
fn rebuilt(plan: &'static ChannelPlan<'static>, settings: Settings) -> EndDevice<'static> {
    EndDevice::new(plan, Device::new(DEV_EUI, JOIN_EUI, APP_KEY), settings).expect("the plan fits")
}

#[test]
fn a_resumed_device_carries_on_exactly_where_it_was_saved() {
    let list = CfList::frequencies([
        867_100_000,
        867_300_000,
        867_500_000,
        867_700_000,
        867_900_000,
    ])
    .expect("valid");
    let grant = JoinGrant::new(0x01, 0x13, DEV_ADDR)
        .with_dl_settings((2 << 4) | 3)
        .with_rx_delay(3)
        .with_cflist(list.to_bytes());
    let (mut device, mut network, now) = joined_with(Region::Eu868, settings(), grant);

    device.send(2, b"a", false, now).expect("goes out");
    let downlink = network.commands(&[
        MacCommand::LinkAdrReq {
            data_rate: 3,
            tx_power: 2,
            channel_mask: 0b1111_1010,
            mask_control: 0,
            transmissions: 2,
        },
        MacCommand::RxTimingSetupReq { delay: 2 },
        MacCommand::DevStatusReq,
    ]);
    data(device.heard(&downlink, 7));
    device.request_link_check();

    let saved = device.save(now + 5_000_000).expect("idle and joined");
    let stored = saved.as_bytes().to_vec();
    let mut resumed = rebuilt(Region::Eu868.plan(), settings());
    resumed
        .resume(&Saved::from_bytes(&stored).expect("intact"), 0)
        .expect("the same plan");

    assert!(resumed.is_joined());
    assert_eq!(resumed.dev_addr(), Some(DEV_ADDR));
    assert_eq!(resumed.fcnt_up(), device.fcnt_up());
    assert_eq!(resumed.fcnt_down(), device.fcnt_down());
    assert_eq!(resumed.data_rate(), 3);
    assert_eq!(resumed.transmissions(), 2);
    assert_eq!(resumed.rx2(), (869_525_000, 3));
    assert_eq!(resumed.receive_delay_us(), 2_000_000);
    assert_eq!(
        resumed.channels().collect::<Vec<_>>(),
        device.channels().collect::<Vec<_>>()
    );

    let original = device.send(2, b"b", false, LATER).expect("goes out");
    let carried_on = resumed.send(2, b"b", false, LATER).expect("goes out");
    assert_eq!(
        carried_on, original,
        "the same frame, channel, power and windows, down to the random channel choice"
    );
    assert_eq!(
        fopts(&network, &carried_on, 1),
        [0x03, 0x07, 0x08, 0x06, 0xFF, 0x07, 0x02],
        "LinkADRAns, RXTimingSetupAns, DevStatusAns and the link check request"
    );
}

#[test]
fn a_duty_cycle_wait_runs_again_on_the_new_clock() {
    // RP002-1.0.5 section 3.4.2: 1% on the default channels, so a long frame owes 99 times
    // its airtime.
    let (mut device, _network, now) = joined(Region::Eu868, settings());
    let long = device.send(2, &[0; 51], false, now).expect("goes out");
    assert_eq!(device.nothing_heard(now + 3_000_000), Ok(Next::Done));
    let saved = device.save(now + 3_000_000).expect("idle");

    let owed = now + long.airtime_us * 100 - (now + 3_000_000);
    let mut resumed = rebuilt(Region::Eu868.plan(), settings());
    resumed.resume(&saved, 1_000).expect("the same plan");
    assert_eq!(
        resumed.send(2, b"x", false, 1_000),
        Err(DeviceError::Wait {
            until_us: 1_000 + owed
        })
    );
    assert!(resumed.send(2, b"x", false, 1_000 + owed).is_ok());
}

#[test]
fn a_damaged_or_foreign_state_is_refused_and_leaves_the_device_as_it_was() {
    let (device, _network, now) = joined(Region::Eu868, settings());
    let saved = device.save(now).expect("idle");
    let bytes = saved.as_bytes();

    let mut flipped = *bytes;
    flipped[40] ^= 0x01;
    assert_eq!(Saved::from_bytes(&flipped), Err(StateError::Corrupt));
    assert_eq!(
        Saved::from_bytes(&bytes[..SAVED_LEN - 1]),
        Err(StateError::Length)
    );

    let mut indian = rebuilt(Region::In865.plan(), settings());
    assert_eq!(
        indian.resume(&saved, 0),
        Err(DeviceError::State(StateError::Plan))
    );
    assert!(!indian.is_joined(), "nothing was taken");
    assert!(indian.join(1, 0).is_ok());

    assert_eq!(format!("{saved:?}"), "Saved { .. }", "no keys in a log");
}

#[test]
fn only_a_joined_device_between_exchanges_saves() {
    let device = device(Region::Eu868, settings());
    assert_eq!(device.save(0), Err(DeviceError::NotJoined));

    let (mut device, _network, now) = joined(Region::Eu868, settings());
    device.send(2, b"a", false, now).expect("goes out");
    assert_eq!(device.save(now), Err(DeviceError::Busy));
    device.nothing_heard(now + 3_000_000).expect("closes");
    assert!(device.save(now + 3_000_000).is_ok());
}

#[test]
fn a_chinese_state_names_the_plan_its_join_chose() {
    let mut device = device(Region::Cn470, settings());
    let (request, at) = join_on(
        &mut device,
        &[
            480_300_000,
            482_300_000,
            484_300_000,
            486_300_000,
            488_300_000,
        ],
    );
    let mut network = Network::new(JoinGrant::new(0x01, 0x13, DEV_ADDR).with_dl_settings(1));
    let accept = network.accept(&request);
    assert!(device.heard(&accept, 0).is_ok());
    let saved = device.save(at + LATER).expect("idle");

    // Built from another of the four plans, the device still lands on the 26 MHz antenna's
    // plan B the join chose.
    let mut resumed = rebuilt(Cn470Plan::Antenna20MhzB.plan(), settings());
    resumed.resume(&saved, 0).expect("the plan the join chose");
    assert_eq!(resumed.channels().count(), 48);
    assert_eq!(resumed.rx2(), (502_500_000, 1));
    let uplink = resumed.send(2, b"x", false, LATER).expect("goes out");
    assert_eq!(network.read(&uplink, 0).payload(), b"x");
}
