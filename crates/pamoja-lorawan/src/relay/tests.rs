//! The relay frames against an implementation written by someone else, and the timing
//! against the worked example TS011-1.0.1 prints.
//!
//! The expected frames come from Semtech LoRa Basics Modem's `wake_on_radio.c`, rendered
//! line for line in Python with the `cryptography` package's AES and CMAC. That rendering
//! also reproduces The Things Stack's `RootWorSKey` test vector, so the two independent
//! implementations and this one agree.

use pamoja_lora::region::Region;
use pamoja_lora::LinkSettings;

use super::*;
use crate::{LorawanError, Session};

const NWK_S_KEY: [u8; 16] = [0x2B; 16];
const DEV_ADDR: u32 = 0x2601_1BDA;
const WOR: Carrier = Carrier::new(865_100_000, 3);
const ACK: Carrier = Carrier::new(865_300_000, 3);
const UPLINK: Carrier = Carrier::new(868_100_000, 5);

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("hex"))
        .collect()
}

fn keys() -> WorKeys {
    Session::new(DEV_ADDR, NWK_S_KEY, [0x99; 16]).wor_keys()
}

fn state() -> StateSync {
    StateSync {
        cad_to_rx: CadToRx::Symbols4,
        forward: Forward::Available,
        relay_data_rate: 5,
        xtal_accuracy: XtalAccuracy::Ppm30,
        cad_periodicity: CadPeriodicity::Ms500,
        t_offset_ms: 892,
    }
}

#[test]
fn the_keys_come_from_the_network_key_and_the_address() {
    let session = Session::new(DEV_ADDR, NWK_S_KEY, [0x99; 16]);
    assert_eq!(
        session.root_wor_s_key().to_vec(),
        hex("6c9b474428d7fdb18ffbd0280ab687b3")
    );
    let keys = session.wor_keys();
    assert_eq!(
        keys.integrity().to_vec(),
        hex("624b9e55ad4a5abf457ed57d5c4ec12a")
    );
    assert_eq!(
        keys.encryption().to_vec(),
        hex("1b49233a8f7216045c5193442bf7f88b")
    );
    assert_eq!(
        format!("{keys:?}"),
        "WorKeys { .. }",
        "keys are not printed"
    );
}

#[test]
fn a_wor_uplink_matches_basics_modem_and_opens_again() {
    for (wfcnt, want) in [
        (1u32, "01da1b0126ee1c12440100c02218aa"),
        (0x0001_2345, "01da1b0126b221933b452304b720b7"),
    ] {
        let frame = wor_uplink(&keys(), DEV_ADDR, wfcnt, UPLINK, WOR).expect("it encodes");
        assert_eq!(frame.to_vec(), hex(want), "WFCnt {wfcnt}");

        let Ok(Wor::Uplink(sealed)) = Wor::parse(&frame) else {
            panic!("an uplink WOR");
        };
        assert_eq!(sealed.dev_addr(), DEV_ADDR);
        assert_eq!(sealed.wfcnt(), wfcnt as u16, "only the low bits travel");
        assert_eq!(sealed.open(&keys(), wfcnt, WOR), Ok(UPLINK));
    }
}

#[test]
fn a_wor_that_does_not_verify_is_refused() {
    let frame = wor_uplink(&keys(), DEV_ADDR, 0x0001_2345, UPLINK, WOR).expect("it encodes");
    let Ok(Wor::Uplink(sealed)) = Wor::parse(&frame) else {
        panic!("an uplink WOR");
    };
    assert_eq!(
        sealed.open(&keys(), 0x0000_2345, WOR),
        Err(LorawanError::MicMismatch),
        "the upper bits of the counter are part of the check"
    );
    assert_eq!(
        sealed.open(&keys(), 0x0001_2346, WOR),
        Err(LorawanError::FcntMismatch)
    );
    let stranger = Session::new(DEV_ADDR, [0x2C; 16], [0x99; 16]).wor_keys();
    assert_eq!(
        sealed.open(&stranger, 0x0001_2345, WOR),
        Err(LorawanError::MicMismatch)
    );

    // Heard on another channel, the payload decrypts to something else, but the check does
    // not fold the channel in and still passes; the carrier is what changes.
    let elsewhere = sealed
        .open(&keys(), 0x0001_2345, Carrier::new(865_500_000, 5))
        .expect("the integrity code does not cover the WOR carrier");
    assert_ne!(elsewhere, UPLINK);
}

#[test]
fn a_wor_ack_matches_basics_modem_and_opens_again() {
    for (wfcnt, want) in [(1u32, "c091ac431978ef"), (0x0001_2345, "6c9f249eb2e6ec")] {
        let frame = wor_ack(&keys(), DEV_ADDR, wfcnt, ACK, UPLINK, state()).expect("it encodes");
        assert_eq!(frame.to_vec(), hex(want), "WFCnt {wfcnt}");
        assert_eq!(
            open_wor_ack(&frame, &keys(), DEV_ADDR, wfcnt, ACK, UPLINK),
            Ok(state())
        );
    }

    let frame = wor_ack(&keys(), DEV_ADDR, 1, ACK, UPLINK, state()).expect("it encodes");
    assert_eq!(
        open_wor_ack(
            &frame,
            &keys(),
            DEV_ADDR,
            1,
            ACK,
            Carrier::new(868_300_000, 5)
        ),
        Err(LorawanError::MicMismatch),
        "an acknowledgment of another uplink carrier"
    );
    assert_eq!(
        open_wor_ack(&frame[..6], &keys(), DEV_ADDR, 1, ACK, UPLINK),
        Err(LorawanError::MalformedFrame)
    );
}

#[test]
fn the_state_sync_fields_sit_where_table_14_puts_them() {
    let bits = StateSync {
        cad_to_rx: CadToRx::Symbols8,
        forward: Forward::Disabled,
        relay_data_rate: 0x0f,
        xtal_accuracy: XtalAccuracy::Ppm40,
        cad_periodicity: CadPeriodicity::Ms20,
        t_offset_ms: 0x07ff,
    }
    .to_bits()
    .expect("it fits");
    assert_eq!(bits, 0x00ff_ffff & !(0b010 << 11));

    assert_eq!(
        StateSync::from_bits(0b110 << 11),
        Err(LorawanError::MalformedFrame),
        "CADPeriodicity 6 is reserved"
    );
    let mut long = state();
    long.t_offset_ms = 2048;
    assert_eq!(long.to_bits(), Err(LorawanError::MalformedFrame));

    assert_eq!(CadToRx::Symbols2.symbols(), 2);
    assert_eq!(CadToRx::Symbols8.symbols(), 8);
    assert_eq!(XtalAccuracy::Ppm10.ppm(), 10);
    assert_eq!(CadPeriodicity::from_code(3), Some(CadPeriodicity::Ms100));
    assert_eq!(CadPeriodicity::Ms20.period_us(), 20_000);
}

#[test]
fn a_join_request_wor_carries_only_its_carrier() {
    let frame = wor_join_request(Carrier::new(916_800_000, 0)).expect("it encodes");
    assert_eq!(frame, [0x00, 0x00, 0x80, 0xe4, 0x8b]);
    assert_eq!(
        Wor::parse(&[0xf0, 0x00, 0x80, 0xe4, 0x8b]),
        Ok(Wor::JoinRequest {
            uplink: Carrier::new(916_800_000, 0)
        }),
        "the reserved header bits are ignored"
    );
    assert_eq!(
        wor_join_request(Carrier::new(868_100_050, 0)),
        Err(LorawanError::MalformedFrame)
    );
    assert_eq!(
        wor_join_request(Carrier::new(868_100_000, 16)),
        Err(LorawanError::MalformedFrame)
    );
}

#[test]
fn reserved_proprietary_and_misshapen_wor_frames_are_discarded() {
    for header in [2u8, 14, 15] {
        let mut frame = [0u8; WOR_UPLINK_LEN];
        frame[0] = header;
        assert_eq!(
            Wor::parse(&frame),
            Err(LorawanError::MalformedFrame),
            "WORType {header}"
        );
    }
    assert_eq!(
        Wor::parse(&[0x00, 0x05, 0x28, 0x76]),
        Err(LorawanError::MalformedFrame)
    );
    assert_eq!(
        Wor::parse(&[0x01; WOR_UPLINK_LEN + 1]),
        Err(LorawanError::MalformedFrame)
    );
    assert_eq!(Wor::parse(&[]), Err(LorawanError::FrameTooShort));
}

#[test]
fn a_forwarded_uplink_clamps_what_its_fields_cannot_carry() {
    let forwarded = ForwardedUplink {
        metadata: UplinkMetadata {
            wor_channel: WorChannel::Default,
            rssi_dbm: -10,
            snr_db: -30,
            data_rate: 3,
        },
        frequency_hz: 916_800_000,
        phy_payload: &[0x80; 12],
    };
    let mut out = [0u8; 32];
    let len = forwarded.encode(&mut out).expect("it encodes");
    assert_eq!(len, FORWARD_OVERHEAD + 12);
    let read = ForwardedUplink::parse(&out[..len]).expect("it parses");
    assert_eq!(read.metadata.rssi_dbm, -15);
    assert_eq!(read.metadata.snr_db, -20);
    assert_eq!(read.phy_payload, &[0x80; 12]);

    assert_eq!(
        forwarded.encode(&mut [0u8; 17]),
        Err(LorawanError::PayloadTooLong)
    );
    assert_eq!(
        ForwardedUplink::parse(&[0x00, 0x00, 0x02, 0, 0, 0]),
        Err(LorawanError::MalformedFrame),
        "WORChannel 2 is reserved"
    );
    assert_eq!(
        ForwardedUplink::parse(&[0; 5]),
        Err(LorawanError::FrameTooShort)
    );
}

#[test]
fn the_appendix_example_slips_a_slot_then_loses_synchronization() {
    let sync = Synchronization::from_ack(1_234_000, 133, 8_192, &state());

    // An hour on, the drifted start of the next slot is already past, so the one after it
    // is used: 3 600 931 ms, started 90 ms early with a 32-symbol preamble.
    let slot = sync
        .next_wor(3_600_400_000, 20, 8_192, false)
        .expect("synchronized");
    assert_eq!(slot.start_us, 3_600_841_549);
    assert_eq!(slot.preamble_symbols, 32);

    // The relay's other channel is scanned half a period later.
    let other = sync
        .next_wor(61_000_000, 20, 8_192, true)
        .expect("synchronized");
    assert_eq!(other.start_us, 61_180_049);

    // Unsynchronized, the preamble spans the whole period.
    assert_eq!(
        unsynchronized_preamble_symbols(CadPeriodicity::Ms1000, 8_192, CadToRx::Symbols8),
        137
    );
    // The appendix's own example, with the end of the preamble worked out from the frame:
    // detected at 87 654 ms, ended at 88 734 ms, 264.192 ms of sync word and payload.
    let sync_and_payload = LinkSettings::new(10, 125_000)
        .with_preamble(0)
        .airtime_us(WOR_UPLINK_LEN);
    assert_eq!(
        t_offset_ms(87_654_000, 88_734_000 - sync_and_payload),
        Some(816)
    );
    assert_eq!(t_offset_ms(1_000, 2_000), Some(1));
    assert_eq!(t_offset_ms(1_000_000, 900_000), None, "before the scan");
    assert_eq!(
        t_offset_ms(0, 3_000_000),
        None,
        "past the eleven bits a WOR ACK carries"
    );
}

#[test]
fn a_wor_ack_takes_the_time_on_air_rp002_table_128_gives() {
    // Seven bytes, an eight-symbol preamble, explicit header, CRC, coding rate 4/5.
    let want = [
        (7u8, 125_000u32, 36_000u64),
        (8, 125_000, 72_100),
        (9, 125_000, 123_900),
        (10, 125_000, 247_800),
        (11, 125_000, 495_600),
        (12, 125_000, 991_200),
        (7, 250_000, 18_000),
        (8, 250_000, 36_000),
        (9, 250_000, 61_900),
        (10, 250_000, 123_900),
        (11, 250_000, 247_800),
        (12, 250_000, 495_600),
        (7, 500_000, 9_000),
        (8, 500_000, 18_000),
        (9, 500_000, 30_900),
        (10, 500_000, 61_900),
        (11, 500_000, 123_900),
        (12, 500_000, 247_800),
    ];
    for (sf, bw, micros) in want {
        let airtime = LinkSettings::new(sf, bw).airtime_us(WOR_ACK_LEN);
        assert!(
            airtime.abs_diff(micros) <= 100,
            "SF{sf} at {bw} Hz: {airtime} us against {micros} us"
        );
    }
}

#[test]
fn every_region_names_a_lora_relay_channel_an_end_device_can_send_a_wor_on() {
    for region in [Region::Eu868, Region::Us915, Region::As923] {
        let plan = region.plan();
        let channel = plan.relay_channel(0).expect("a default channel");
        let wor = Carrier::new(channel.wor_frequency_hz, channel.data_rate);
        assert!(wor_uplink(&keys(), DEV_ADDR, 1, UPLINK, wor).is_ok());
    }
}
