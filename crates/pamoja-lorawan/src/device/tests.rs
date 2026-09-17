//! The device against a network played by this crate's own network half.
//!
//! Every exchange here is the one the specification describes, built from the frames a real
//! network would send: a `JoinGrant` signs the accept, a `Session` holding the same keys
//! encodes each downlink and decodes each uplink, and the MAC commands are encoded the way
//! a network encodes them. The expected answers are the bytes TS001-1.0.4 chapter 5 lays
//! out, not whatever the device happens to produce.

use pamoja_lora::region::Region;

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
    .expect("a dynamic plan")
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

#[test]
fn a_fixed_channel_plan_is_refused_for_now() {
    let refused = EndDevice::new(
        Region::Us915.plan(),
        Device::new(DEV_EUI, JOIN_EUI, APP_KEY),
        settings(),
    );
    assert!(matches!(refused, Err(DeviceError::FixedChannelPlan)));
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
