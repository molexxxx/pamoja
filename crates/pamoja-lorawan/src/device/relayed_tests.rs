//! A device sending through a relay, against the rules TS011-1.0.1 sets for it: when its
//! wake-on-radio frame goes out, what an acknowledgment changes, what it does without one,
//! and what its network configures.
//!
//! The relay in these tests is this crate's own [`Relay`], so both halves of every exchange
//! are the ones a real pair would send.

use pamoja_lora::region::Region;

use super::*;
use crate::mac::{encode_all, MacCommand};
use crate::relay::{
    root_wor_s_key, CadPeriodicity, CadToRx, Carrier, Forward, Relay, RelayConfig, RelayHeard,
    RelaySettings, RelaySync, Wake, WorKeys, XtalAccuracy, LA_FPORT_RELAY, RXR_DELAY_US,
    WOR_ACK_DELAY_US, WOR_ACK_LEN, WOR_DATA_DELAY_US, WOR_JOIN_REQUEST_LEN, WOR_UPLINK_LEN,
};
use crate::{Device, Downlink, JoinGrant, Session};

const APP_KEY: [u8; 16] = [0x2B; 16];
const DEV_EUI: [u8; 8] = [0x70, 0xB3, 0xD5, 0x7E, 0xD0, 0x05, 0x12, 0x34];
const JOIN_EUI: [u8; 8] = [0x22; 8];
const DEV_ADDR: u32 = 0x2601_2E43;
const NWK_S_KEY: [u8; 16] = [0x5A; 16];
const APP_S_KEY: [u8; 16] = [0xA5; 16];
const RELAY_ADDR: u32 = 0x2601_0001;

/// Ten minutes, which clears any duty cycle off time a test's frames run up.
const LATER: u64 = 600_000_000;

fn session() -> Session {
    Session::new(DEV_ADDR, NWK_S_KEY, APP_S_KEY)
}

fn settings() -> Settings {
    Settings::new(2, 14).with_seed(7).with_crystal_ppm(20)
}

/// A joined device with relay mode on.
fn relaying_device() -> EndDevice<'static> {
    let mut device =
        EndDevice::personalized(Region::Eu868.plan(), session(), settings()).expect("a device");
    assert!(device.use_relay(true));
    device
}

fn keys() -> WorKeys {
    WorKeys::derive(&root_wor_s_key(&NWK_S_KEY), DEV_ADDR)
}

/// A relay that trusts the device, and its own network session.
fn relay() -> (Relay<'static>, Session) {
    let session = Session::new(RELAY_ADDR, [0x11; 16], [0x22; 16]);
    let device = EndDevice::personalized(
        Region::Eu868.plan(),
        session,
        Settings::new(2, 14)
            .with_seed(3)
            .with_tuning_range(863_000_000, 870_000_000),
    )
    .expect("a device");
    let mut relay = Relay::new(
        device,
        RelaySettings::new(XtalAccuracy::Ppm10, CadToRx::Symbols2),
    );
    relay
        .start(RelayConfig::region_default(Region::Eu868.plan()).expect("relay channels"))
        .expect("a configuration it can run");
    relay.trust(0, DEV_ADDR, &root_wor_s_key(&NWK_S_KEY), 0, 63, 0);
    (relay, session)
}

#[test]
fn an_uplink_under_a_relay_goes_out_behind_a_frame_that_wakes_it() {
    let mut device = relaying_device();
    let transmission = device
        .send(1, b"21.5", false, 1_000_000)
        .expect("an uplink");
    let relay = transmission.relay.expect("a relay exchange");

    // The frame goes out on the region's first WOR channel, with a preamble spanning the
    // second a device that has heard from no relay assumes it scans.
    let wake_up = relay.wake_up;
    assert_eq!(wake_up.carrier, Carrier::new(865_100_000, 3));
    assert_eq!(wake_up.start_us, 1_000_000);
    assert_eq!(wake_up.frame().len(), WOR_UPLINK_LEN);
    assert_eq!(
        wake_up.link.preamble_symbols(),
        259,
        "a second of SF9 symbols, plus the six to demodulate and the eight to switch",
    );
    assert_eq!(device.relay_sync(), RelaySync::Initialized);

    // It names the uplink that follows, and the counter it carries.
    let Ok(crate::relay::Wor::Uplink(sealed)) = crate::relay::Wor::parse(wake_up.frame()) else {
        panic!("a wake-on-radio uplink");
    };
    assert_eq!(sealed.dev_addr(), DEV_ADDR);
    assert_eq!(sealed.wfcnt(), 0);
    assert_eq!(
        sealed.open(&keys(), 0, wake_up.carrier).expect("verifies"),
        Carrier::new(transmission.frequency_hz, transmission.data_rate)
    );
    assert_eq!(device.wor_counter(), 1, "the next frame takes the next one");

    // The acknowledgment window opens 50 ms after the frame, on the channel's answering
    // frequency, and the uplink follows 50 ms after the acknowledgment would end.
    let ack = relay.ack.expect("an uplink is acknowledged");
    let ended = wake_up.start_us + wake_up.airtime_us;
    assert_eq!(ack.start_us, ended + u64::from(WOR_ACK_DELAY_US));
    assert_eq!(ack.carrier, Carrier::new(865_300_000, 3));
    assert_eq!(ack.airtime_us, ack.link.airtime_us(WOR_ACK_LEN));
    assert_eq!(
        relay.uplink_start_us,
        ack.start_us + ack.airtime_us + u64::from(WOR_DATA_DELAY_US)
    );

    // The relay window opens eighteen seconds after the uplink, on the WOR frequency at the
    // data rate its first window uses.
    assert_eq!(relay.rxr.delay_us, RXR_DELAY_US);
    assert_eq!(relay.rxr.frequency_hz, 865_100_000);
    assert_eq!(relay.rxr.data_rate, transmission.rx1.data_rate);
    assert!(relay.rxr.link.crc(), "a relay window carries a CRC");
}

#[test]
fn an_acknowledgment_shortens_the_next_frame_to_the_drift_since_it() {
    let (mut relay, _) = relay();
    let mut device = relaying_device();

    let scan = relay.next_scan(0).expect("running");
    let transmission = device
        .send(1, b"21.5", false, scan.start_us)
        .expect("an uplink");
    let exchange = transmission.relay.expect("a relay exchange");
    let heard_at = exchange.wake_up.start_us + exchange.wake_up.airtime_us;
    let Wake::Uplink { acknowledgment, .. } = relay
        .heard_wor(&scan, exchange.wake_up.frame(), -90, 4, heard_at)
        .expect("a trusted device")
    else {
        panic!("an uplink follows");
    };
    let ack = acknowledgment.expect("acknowledged");

    let status = device.heard_wor_ack(&ack.frame).expect("it verifies");
    assert_eq!(status.cad_periodicity, CadPeriodicity::Ms1000);
    assert_eq!(status.xtal_accuracy, XtalAccuracy::Ppm10);
    assert_eq!(status.cad_to_rx, CadToRx::Symbols2);
    assert_eq!(status.forward, Forward::Available);
    assert_eq!(device.relay_sync(), RelaySync::Synchronized);
    assert_eq!(device.relay_status(), Some(status));

    // The next frame is aimed at a scan rather than covering a whole period of them, and
    // carries only the drift the two crystals could have built up since.
    device
        .nothing_heard(exchange.uplink_start_us + 5_000_000)
        .ok();
    let next = device
        .send(1, b"21.6", false, exchange.uplink_start_us + LATER)
        .expect("an uplink")
        .relay
        .expect("a relay exchange");
    assert!(
        next.wake_up.link.preamble_symbols() < 20,
        "{} symbols",
        next.wake_up.link.preamble_symbols()
    );
    let scan = relay
        .next_scan(next.wake_up.start_us)
        .expect("running")
        .start_us;
    let ends = next.wake_up.start_us + next.wake_up.airtime_us;
    assert!(
        ends >= scan,
        "the preamble still covers the relay's scan at {scan}, ending at {ends}",
    );
}

#[test]
fn a_frame_without_an_answer_sends_the_uplink_anyway_unless_the_network_asks_otherwise() {
    let mut device = relaying_device();
    device.send(1, b"21.5", false, 0).expect("an uplink");
    assert_eq!(
        device.no_wor_ack(1_000_000).expect("a frame was waiting"),
        WorNext::Uplink,
        "with no BackOff set, every frame is followed by its uplink",
    );
    assert_eq!(
        device.no_wor_ack(2_000_000),
        Err(DeviceError::NothingPending),
    );

    // A network that sets BackOff to 3 has the device try three times first.
    let mut device = relaying_device();
    network_sets(&mut device, 3);
    let at = 10 * LATER;
    let first = device.wor_counter();
    device.send(1, b"21.5", false, at).expect("an uplink");
    for attempt in 1..3u64 {
        let WorNext::WakeUp(again) = device
            .no_wor_ack(at + attempt * 1_000_000)
            .expect("a frame was waiting")
        else {
            panic!("another frame goes out first");
        };
        assert_eq!(device.wor_counter(), first + attempt as u32 + 1);
        assert!(again.wake_up.start_us >= at + attempt * 1_000_000);
    }
    assert_eq!(
        device.no_wor_ack(at + 4_000_000).expect("waiting"),
        WorNext::Uplink,
        "the third frame is followed by the uplink",
    );
}

#[test]
fn eight_frames_without_an_answer_give_up_the_relay() {
    let mut device = relaying_device();
    network_sets(&mut device, 1);
    assert_eq!(
        device.relay_activation(),
        crate::relay::RelayActivation::Enabled
    );
    for attempt in 0..8u64 {
        let at = (attempt + 10) * LATER;
        device.send(1, b"21.5", false, at).expect("an uplink");
        device
            .no_wor_ack(at + 1_000_000)
            .expect("a frame was waiting");
        device
            .nothing_heard(at + 30_000_000)
            .expect("the windows closed");
    }
    assert_eq!(
        device.relay_sync(),
        RelaySync::Initialized,
        "what it knew of the relay is gone",
    );
    assert!(
        device.relaying(),
        "a network that asked for relay mode still has it",
    );

    // A device left to itself, which has not been told what to do, stops using a relay
    // that never answers.
    let mut device =
        EndDevice::personalized(Region::Eu868.plan(), session(), settings()).expect("a device");
    for uplink in 0..16u64 {
        device
            .send(1, b"21.5", false, uplink * LATER)
            .expect("an uplink");
        device
            .nothing_heard(uplink * LATER + 30_000_000)
            .expect("the windows closed");
    }
    assert!(device.relaying(), "sixteen uplinks went unanswered");
    for attempt in 0..8u64 {
        let at = (attempt + 16) * LATER;
        device.send(1, b"21.5", false, at).expect("an uplink");
        device
            .no_wor_ack(at + 1_000_000)
            .expect("a frame was waiting");
        device
            .nothing_heard(at + 30_000_000)
            .expect("the windows closed");
    }
    assert!(
        !device.relaying(),
        "eight frames a relay never answered put it aside",
    );
}

#[test]
fn a_join_request_wakes_the_relay_without_asking_to_be_acknowledged() {
    let mut device = EndDevice::new(
        Region::Eu868.plan(),
        Device::new(DEV_EUI, JOIN_EUI, APP_KEY),
        settings(),
    )
    .expect("a device");
    assert!(device.use_relay(true));

    let request = device.join(1, 0).expect("a join request");
    let exchange = request.relay.expect("a relay exchange");
    assert_eq!(exchange.wake_up.frame().len(), WOR_JOIN_REQUEST_LEN);
    assert_eq!(
        crate::relay::Wor::parse(exchange.wake_up.frame()),
        Ok(crate::relay::Wor::JoinRequest {
            uplink: Carrier::new(request.frequency_hz, request.data_rate)
        })
    );
    assert!(
        exchange.ack.is_none(),
        "TS011-1.0.1 section 3.3: a relay never acknowledges a join request",
    );
    let ended = exchange.wake_up.start_us + exchange.wake_up.airtime_us;
    assert_eq!(
        exchange.uplink_start_us,
        ended + u64::from(WOR_DATA_DELAY_US)
    );

    // An accept that comes back through the relay keeps relay mode on, and the counter
    // starts again.
    let accept = JoinGrant::new(0x01, 0x13, DEV_ADDR).accept(&APP_KEY, 1);
    assert_eq!(
        device
            .heard_in(ReceiveWindow::Rxr, accept.as_bytes(), 7)
            .expect("the accept verifies"),
        Heard::Joined { dev_addr: DEV_ADDR }
    );
    assert!(device.relaying());
    assert_eq!(device.wor_counter(), 0);
    assert_eq!(device.relay_sync(), RelaySync::Unsynchronized);
}

#[test]
fn an_accept_that_comes_back_straight_from_a_gateway_puts_the_relay_aside() {
    let mut device = EndDevice::new(
        Region::Eu868.plan(),
        Device::new(DEV_EUI, JOIN_EUI, APP_KEY),
        settings(),
    )
    .expect("a device");

    // The fourth attempt of a device left to itself goes through a relay.
    let mut request = device.join(1, 0).expect("a join request");
    for attempt in 1..4u64 {
        device
            .nothing_heard(attempt * LATER)
            .expect("the windows closed");
        request = device
            .join(attempt as u16 + 1, attempt * LATER + 60_000_000)
            .expect("a join request");
    }
    assert!(request.relay.is_some());

    let accept = JoinGrant::new(0x01, 0x13, DEV_ADDR).accept(&APP_KEY, 4);
    device
        .heard_in(ReceiveWindow::Rx1, accept.as_bytes(), 7)
        .expect("the accept verifies");
    assert!(
        !device.relaying(),
        "a gateway hears it, so no relay is needed"
    );
}

#[test]
fn a_device_left_to_itself_tries_a_relay_on_one_join_in_four() {
    let mut device = EndDevice::new(
        Region::Eu868.plan(),
        Device::new(DEV_EUI, JOIN_EUI, APP_KEY),
        settings(),
    )
    .expect("a device");
    let mut relayed = 0;
    for attempt in 0..8u64 {
        let request = device
            .join(attempt as u16 + 1, attempt * LATER)
            .expect("a join request");
        relayed += usize::from(request.relay.is_some());
        device
            .nothing_heard(attempt * LATER + 7_000_000)
            .expect("the windows closed");
    }
    assert_eq!(relayed, 2, "two of eight attempts went through a relay");
}

#[test]
fn sixteen_unanswered_uplinks_turn_a_relay_on_and_a_downlink_turns_it_off_again() {
    let mut device =
        EndDevice::personalized(Region::Eu868.plan(), session(), settings()).expect("a device");
    assert!(!device.relaying());

    for uplink in 0..15u64 {
        device
            .send(1, b"21.5", false, uplink * LATER)
            .expect("an uplink");
        device
            .nothing_heard(uplink * LATER + 3_000_000)
            .expect("the windows closed");
        assert!(!device.relaying(), "after {} uplinks", uplink + 1);
    }
    device
        .send(1, b"21.5", false, 15 * LATER)
        .expect("an uplink");
    device
        .nothing_heard(15 * LATER + 3_000_000)
        .expect("the windows closed");
    assert!(device.relaying(), "sixteen uplinks went unanswered");

    // A downlink through the relay puts the count back.
    let transmission = device
        .send(1, b"21.5", false, 16 * LATER)
        .expect("an uplink");
    let answer = session()
        .encode_downlink(&Downlink::new(0, 1, b"ok"))
        .expect("a downlink");
    device
        .heard_in(ReceiveWindow::Rxr, answer.as_bytes(), 7)
        .expect("the downlink verifies");
    assert!(transmission.relay.is_some());
    assert_eq!(device.relay_sync(), RelaySync::Unsynchronized);
}

#[test]
fn the_network_sets_how_a_device_uses_a_relay() {
    let mut device = relaying_device();
    let answers = network_says(
        &mut device,
        MacCommand::EndDeviceConfReq {
            relay_mode: 2,
            smart_enable_level: 1,
            back_off: 7,
            second_channel_index: 1,
            second_channel_data_rate: 5,
            second_channel_ack_offset: 1,
            second_channel_frequency_hz: 866_500_000,
        },
    );
    assert_eq!(
        answers,
        vec![MacCommand::EndDeviceConfAns {
            second_channel_ack_offset_ack: true,
            second_channel_index_ack: true,
            second_channel_data_rate_ack: true,
            second_channel_frequency_ack: true,
        }]
    );
    assert_eq!(
        device.relay_activation(),
        crate::relay::RelayActivation::Dynamic
    );

    // The second channel is where the next frame goes, and its acknowledgment comes back
    // 200 kHz above it.
    let exchange = device
        .send(1, b"21.5", false, 10 * LATER)
        .expect("an uplink")
        .relay
        .expect("a relay exchange");
    assert_eq!(exchange.wake_up.carrier, Carrier::new(866_500_000, 5));
    assert_eq!(
        exchange.ack.expect("acknowledged").carrier,
        Carrier::new(866_700_000, 5)
    );
}

#[test]
fn a_device_names_the_parts_of_a_relay_configuration_it_cannot_take() {
    let mut device = relaying_device();
    let answers = network_says(
        &mut device,
        MacCommand::EndDeviceConfReq {
            relay_mode: 1,
            smart_enable_level: 0,
            back_off: 0,
            second_channel_index: 2,
            second_channel_data_rate: 15,
            second_channel_ack_offset: 6,
            second_channel_frequency_hz: 100_000,
        },
    );
    assert_eq!(
        answers,
        vec![MacCommand::EndDeviceConfAns {
            second_channel_ack_offset_ack: true,
            second_channel_index_ack: false,
            second_channel_data_rate_ack: true,
            second_channel_frequency_ack: true,
        }],
        "a reserved index leaves the fields it would have carried unread",
    );
    assert_eq!(
        device.relay_activation(),
        crate::relay::RelayActivation::DeviceControlled,
        "nothing was taken",
    );

    // A channel that is named, and cannot be used, is refused field by field.
    let answers = network_says(
        &mut device,
        MacCommand::EndDeviceConfReq {
            relay_mode: 1,
            smart_enable_level: 0,
            back_off: 0,
            second_channel_index: 1,
            second_channel_data_rate: 15,
            second_channel_ack_offset: 6,
            second_channel_frequency_hz: 100_000,
        },
    );
    assert_eq!(
        answers,
        vec![MacCommand::EndDeviceConfAns {
            second_channel_ack_offset_ack: false,
            second_channel_index_ack: true,
            second_channel_data_rate_ack: false,
            second_channel_frequency_ack: false,
        }]
    );

    // Turning relay mode off says nothing about a channel, so the rest is not read.
    let answers = network_says(
        &mut device,
        MacCommand::EndDeviceConfReq {
            relay_mode: 0,
            smart_enable_level: 0,
            back_off: 0,
            second_channel_index: 3,
            second_channel_data_rate: 15,
            second_channel_ack_offset: 7,
            second_channel_frequency_hz: 0,
        },
    );
    assert_eq!(
        answers,
        vec![MacCommand::EndDeviceConfAns {
            second_channel_ack_offset_ack: true,
            second_channel_index_ack: true,
            second_channel_data_rate_ack: true,
            second_channel_frequency_ack: true,
        }]
    );
    assert!(!device.relaying());
    assert!(!device.use_relay(true), "the network holds the decision");
}

#[test]
fn a_relay_that_forwards_slowly_holds_the_payload_down() {
    let (mut relay, _) = relay();
    let mut device = relaying_device();

    // The relay forwards at DR0, where its own frame carries 51 bytes: 27 of them are the
    // forwarded metadata and the device's own header.
    let scan = relay.next_scan(0).expect("running");
    let exchange = device
        .send(1, b"21.5", false, scan.start_us)
        .expect("an uplink")
        .relay
        .expect("a relay exchange");
    let Wake::Uplink { acknowledgment, .. } = relay
        .heard_wor(
            &scan,
            exchange.wake_up.frame(),
            -90,
            4,
            exchange.wake_up.start_us + exchange.wake_up.airtime_us,
        )
        .expect("a trusted device")
    else {
        panic!("an uplink follows");
    };
    let status = device
        .heard_wor_ack(&acknowledgment.expect("acknowledged").frame)
        .expect("it verifies");
    assert_eq!(status.relay_data_rate, 0);

    device
        .nothing_heard(exchange.uplink_start_us + 5_000_000)
        .ok();
    assert_eq!(
        device.send(1, &[0; 40], false, 10 * LATER),
        Err(DeviceError::PayloadTooLong { max: 32 }),
        "51 bytes less the 6 of metadata and the 13 of header and code",
    );
    assert!(device.send(1, &[0; 32], false, 10 * LATER).is_ok());
}

#[test]
fn a_reading_reaches_the_network_through_a_relay_and_the_answer_comes_back() {
    let (mut relay, relay_session) = relay();
    let mut device = relaying_device();

    // The device wakes the relay, which acknowledges it and listens.
    let scan = relay.next_scan(0).expect("running");
    let transmission = device
        .send(2, b"21.5", false, scan.start_us)
        .expect("an uplink");
    let exchange = transmission.relay.expect("a relay exchange");
    let wor_ended = exchange.wake_up.start_us + exchange.wake_up.airtime_us;
    let Wake::Uplink {
        acknowledgment,
        listen,
        ..
    } = relay
        .heard_wor(&scan, exchange.wake_up.frame(), -90, 4, wor_ended)
        .expect("a trusted device")
    else {
        panic!("an uplink follows");
    };
    let acknowledgment = acknowledgment.expect("acknowledged");
    let listen = listen.expect("forwarding is available");
    assert_eq!(
        acknowledgment.start_us,
        exchange.ack.expect("a window").start_us
    );
    assert_eq!(
        listen.start_us, exchange.uplink_start_us,
        "both sides time the uplink the same way",
    );
    assert_eq!(listen.carrier.frequency_hz, transmission.frequency_hz);
    device
        .heard_wor_ack(&acknowledgment.frame)
        .expect("it verifies");

    // The uplink goes out, and the relay forwards it to the network.
    let uplink_ended = exchange.uplink_start_us + transmission.airtime_us;
    let due = relay
        .heard_uplink(transmission.frame.as_bytes(), -88, 6, uplink_ended)
        .expect("forwarded");
    let forwarded = relay.forward(due).expect("sent");
    let sent = relay_session
        .decode(forwarded.frame.as_bytes(), 0)
        .expect("the relay's own uplink decodes");
    assert_eq!(sent.fport(), Some(LA_FPORT_RELAY));

    // The network answers the device through the relay, which sends it on in the relay
    // window the device is already listening in.
    let answer = session()
        .encode_downlink(&Downlink::new(0, 2, b"ok"))
        .expect("a downlink");
    let reply = relay_session
        .encode_downlink(&Downlink::new(0, LA_FPORT_RELAY, answer.as_bytes()))
        .expect("a downlink");
    let RelayHeard::Downlink { downlink, .. } = relay
        .heard_in(ReceiveWindow::Rx1, reply.as_bytes(), 7)
        .expect("read")
    else {
        panic!("a downlink for the device");
    };
    assert_eq!(
        downlink.start_us,
        uplink_ended + u64::from(RXR_DELAY_US),
        "the relay sends it when the device opens its relay window",
    );
    assert_eq!(downlink.carrier.frequency_hz, exchange.rxr.frequency_hz);
    assert_eq!(downlink.carrier.data_rate, exchange.rxr.data_rate);

    let heard = device
        .heard_in(ReceiveWindow::Rxr, downlink.frame(), 7)
        .expect("the downlink verifies");
    let Heard::Data(delivery) = heard else {
        panic!("a downlink for the application");
    };
    assert_eq!(delivery.port(), Some(2));
    assert_eq!(delivery.payload(), b"ok");
}

#[test]
fn a_saved_device_wakes_with_its_relay_and_its_counter() {
    let mut device = relaying_device();
    network_sets(&mut device, 5);
    let at = 10 * LATER;
    device.send(1, b"21.5", false, at).expect("an uplink");
    device.no_wor_ack(at + 1_000_000).expect("waiting");
    device
        .nothing_heard(at + 10_000_000)
        .expect("the windows closed");
    let counter = device.wor_counter();
    assert!(counter >= 2);

    let saved = device.save(at + 11_000_000).expect("a saved state");
    let mut woken =
        EndDevice::personalized(Region::Eu868.plan(), session(), settings()).expect("a device");
    woken.resume(&saved, 0).expect("it resumes");
    assert!(woken.relaying());
    assert_eq!(woken.wor_counter(), counter, "no frame reuses a counter");
    assert_eq!(
        woken.relay_activation(),
        crate::relay::RelayActivation::Enabled
    );
    assert_eq!(woken.relay_sync(), RelaySync::Initialized);
}

#[test]
fn a_downlink_longer_than_the_relay_window_carries_is_not_taken() {
    let mut device = relaying_device();
    let transmission = device.send(1, b"21.5", false, 0).expect("an uplink");
    let rxr = transmission.relay.expect("a relay exchange").rxr;
    assert_eq!(rxr.data_rate, 0, "DR0, where a MACPayload holds 59 bytes");

    let long = session()
        .encode_downlink(&Downlink::new(0, 1, &[0; 60]))
        .expect("a downlink");
    assert_eq!(
        device.heard_in(ReceiveWindow::Rxr, long.as_bytes(), 7),
        Err(DeviceError::Frame(crate::LorawanError::PayloadTooLong)),
    );
    let short = session()
        .encode_downlink(&Downlink::new(0, 1, b"ok"))
        .expect("a downlink");
    assert!(device
        .heard_in(ReceiveWindow::Rxr, short.as_bytes(), 7)
        .is_ok());
}

/// Has the network turn relay mode on and set a `BackOff`.
fn network_sets(device: &mut EndDevice<'_>, back_off: u8) {
    let answers = network_says(
        device,
        MacCommand::EndDeviceConfReq {
            relay_mode: 1,
            smart_enable_level: 0,
            back_off,
            second_channel_index: 0,
            second_channel_data_rate: 0,
            second_channel_ack_offset: 0,
            second_channel_frequency_hz: 0,
        },
    );
    assert_eq!(answers.len(), 1);
    assert!(device.relaying());
}

/// Sends the device a downlink carrying one command, and reads what it answers.
fn network_says(device: &mut EndDevice<'_>, command: MacCommand) -> Vec<MacCommand> {
    let session = session();
    let fcnt_down = device.fcnt_down().map_or(0, |fcnt| fcnt + 1);
    let at = 2 * LATER * u64::from(fcnt_down + 1);
    let mut payload = [0u8; crate::mac::MAX_COMMAND];
    let len = encode_all(&[command], &mut payload).expect("the command encodes");
    let downlink = session
        .encode_downlink(&Downlink::new(fcnt_down, 0, &payload[..len]))
        .expect("a downlink");
    device.send_empty(at).expect("an uplink to answer");
    device
        .heard_in(ReceiveWindow::Rx1, downlink.as_bytes(), 7)
        .expect("the downlink verifies");

    let answer = device
        .send_empty(at + LATER)
        .expect("an uplink carrying the answers");
    let fcnt_up = device.fcnt_up().saturating_sub(1);
    device
        .nothing_heard(at + LATER + 10_000_000)
        .expect("the windows closed");
    let sent = session
        .decode(answer.frame.as_bytes(), fcnt_up)
        .expect("the uplink decodes");
    let source = if sent.fport() == Some(0) {
        sent.payload()
    } else {
        sent.fopts()
    };
    crate::mac::MacCommands::new(crate::Direction::Uplink, source)
        .filter_map(Result::ok)
        .collect()
}
