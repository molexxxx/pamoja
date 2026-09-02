//! Checks the plans against the tables RP002-1.0.5 prints.
//!
//! These assert the document's own values rather than round-tripping the
//! implementation against itself, because a table that is wrong the same way in
//! both directions still puts a device on the wrong frequency.

use super::*;

#[cfg(feature = "eu868")]
mod eu868 {
    use super::*;

    /// RP002-1.0.5 Table 10.
    #[test]
    fn every_data_rate_matches_the_published_table() {
        let plan = Region::Eu868.plan();
        let want = [
            (0, 12u8, 125_000u32, 250u32),
            (1, 11, 125_000, 440),
            (2, 10, 125_000, 980),
            (3, 9, 125_000, 1_760),
            (4, 8, 125_000, 3_125),
            (5, 7, 125_000, 5_470),
            (6, 7, 250_000, 11_000),
            (12, 6, 125_000, 9_375),
            (13, 5, 125_000, 15_625),
        ];
        for (dr, sf, bw, bitrate) in want {
            let rate = plan.uplink_data_rate(dr).expect("a defined data rate");
            assert_eq!(
                rate.modulation,
                Modulation::LoRa {
                    spreading_factor: sf,
                    bandwidth_hz: bw
                },
                "DR{dr} modulation"
            );
            assert_eq!(rate.bitrate_bps, bitrate, "DR{dr} bit rate");
        }

        let fsk = plan.uplink_data_rate(7).expect("DR7 is FSK");
        assert_eq!(
            fsk.modulation,
            Modulation::Fsk {
                bitrate_bps: 50_000
            }
        );

        let lr_fhss = plan.uplink_data_rate(8).expect("DR8 is LR-FHSS");
        assert_eq!(
            lr_fhss.modulation,
            Modulation::LrFhss {
                coding_rate_numerator: 1,
                coding_rate_denominator: 3,
                bandwidth_hz: 137_000,
            }
        );
    }

    /// RP002-1.0.5 Tables 15 and 16.
    #[test]
    fn maximum_payloads_match_both_published_tables() {
        let plan = Region::Eu868.plan();
        let repeater = [
            (0, 59u16, 51u16),
            (3, 123, 115),
            (4, 230, 222),
            (8, 58, 50),
            (13, 230, 222),
        ];
        for (dr, m, n) in repeater {
            let limit = plan.max_payload(dr, true).expect("a defined limit");
            assert_eq!(limit, MaxPayload::new(m, n), "DR{dr} behind a repeater");
        }

        // The two tables differ only above DR3, where the repeater's
        // encapsulation costs 20 bytes.
        assert_eq!(
            plan.max_payload(4, false).expect("a defined limit"),
            MaxPayload::new(250, 242)
        );
        assert_eq!(
            plan.max_payload(0, false).expect("a defined limit"),
            plan.max_payload(0, true).expect("a defined limit"),
            "the lowest data rates are already below the repeater limit"
        );
    }

    /// RP002-1.0.5 Table 17, the classic place for an off-by-one.
    #[test]
    fn the_rx1_offset_matrix_matches_the_published_table() {
        let plan = Region::Eu868.plan();
        let rows: [(u8, [u8; 6]); 14] = [
            (0, [0, 0, 0, 0, 0, 0]),
            (1, [1, 0, 0, 0, 0, 0]),
            (2, [2, 1, 0, 0, 0, 0]),
            (3, [3, 2, 1, 0, 0, 0]),
            (4, [4, 3, 2, 1, 0, 0]),
            (5, [5, 4, 3, 2, 1, 0]),
            (6, [6, 5, 4, 3, 2, 1]),
            (7, [7, 6, 5, 4, 3, 2]),
            (8, [1, 0, 0, 0, 0, 0]),
            (9, [2, 1, 0, 0, 0, 0]),
            (10, [1, 0, 0, 0, 0, 0]),
            (11, [2, 1, 0, 0, 0, 0]),
            (12, [12, 5, 4, 3, 2, 1]),
            (13, [13, 12, 5, 4, 3, 2]),
        ];
        for (uplink, row) in rows {
            for (offset, want) in row.iter().enumerate() {
                assert_eq!(
                    plan.rx1_data_rate(uplink, offset as u8),
                    Some(*want),
                    "DR{uplink} at RX1DROffset {offset}"
                );
            }
        }

        assert_eq!(
            plan.rx1_data_rate(5, 6),
            None,
            "offsets 6 and 7 are reserved and must not resolve"
        );
    }

    /// RP002-1.0.5 section 3.4.7 and Table 12.
    #[test]
    fn the_receive_window_and_back_off_match_the_specification() {
        let plan = Region::Eu868.plan();
        assert_eq!(plan.rx2(), (869_525_000, 0), "RX2 is 869.525 MHz at DR0");

        assert_eq!(
            plan.next_backoff_data_rate(0),
            None,
            "DR0 is already lowest"
        );
        assert_eq!(plan.next_backoff_data_rate(5), Some(4));
        assert_eq!(
            plan.next_backoff_data_rate(8),
            Some(0),
            "LR-FHSS drops to DR0"
        );
        assert_eq!(plan.next_backoff_data_rate(12), Some(5));
        assert_eq!(plan.next_backoff_data_rate(13), Some(12));
    }

    /// RP002-1.0.5 Tables 8 and 13.
    #[test]
    fn the_default_channels_and_power_match_the_specification() {
        let plan = Region::Eu868.plan();
        assert_eq!(plan.default_channel_count(), 3);
        assert_eq!(plan.channel_frequency_hz(0), Some(868_100_000));
        assert_eq!(plan.channel_frequency_hz(1), Some(868_300_000));
        assert_eq!(plan.channel_frequency_hz(2), Some(868_500_000));
        assert_eq!(plan.channel_frequency_hz(3), None);

        assert_eq!(
            plan.default_max_eirp_dbm, 16,
            "the default ceiling is 16 dBm"
        );
        assert_eq!(plan.tx_power_dbm(0, 16), Some(16), "index 0 is the ceiling");
        assert_eq!(plan.tx_power_dbm(7, 16), Some(2), "index 7 is 14 dB down");
        assert_eq!(plan.tx_power_dbm(8, 16), None, "8 through 14 are reserved");
    }

    /// The 1% sub-band the three mandatory channels sit in.
    #[test]
    fn the_duty_cycle_is_reported_per_sub_band() {
        let plan = Region::Eu868.plan();
        assert_eq!(plan.duty_cycle_permille(868_100_000), Some(10), "1%");
        assert_eq!(plan.duty_cycle_permille(869_525_000), Some(100), "10%");
        assert_eq!(
            plan.duty_cycle_permille(867_000_000),
            None,
            "a frequency outside the tabulated sub-bands reports nothing"
        );
        assert_eq!(
            plan.max_eirp_dbm(869_525_000),
            27,
            "the RX2 sub-band allows more"
        );
    }

    /// The regional tables feed the airtime math this crate already has.
    #[test]
    fn a_data_rate_produces_link_settings_for_the_airtime_math() {
        let plan = Region::Eu868.plan();
        let settings = plan.link_settings(0).expect("DR0 is LoRa");
        assert_eq!(settings.spreading_factor(), 12);
        assert_eq!(settings.bandwidth_hz(), 125_000);
        assert_eq!(
            settings.airtime_us(10),
            991_232,
            "the published SF12 reference"
        );

        assert!(
            plan.link_settings(7).is_none(),
            "FSK has no chirp airtime model here"
        );
        assert!(plan.link_settings(8).is_none(), "and neither does LR-FHSS");
    }

    /// SF5 and SF6 arrived with RP002-1.0.5 and must survive the link settings.
    #[test]
    fn the_data_rates_added_in_this_revision_keep_their_spreading_factor() {
        let plan = Region::Eu868.plan();
        assert_eq!(
            plan.link_settings(12)
                .expect("DR12 is LoRa")
                .spreading_factor(),
            6,
            "DR12 is SF6, not clamped up to SF7"
        );
        assert_eq!(
            plan.link_settings(13)
                .expect("DR13 is LoRa")
                .spreading_factor(),
            5,
            "DR13 is SF5, not clamped up to SF7"
        );
    }
}

#[cfg(feature = "us915")]
mod us915 {
    use super::*;

    /// RP002-1.0.5 Tables 19 and 20, which number the two directions differently.
    #[test]
    fn the_two_directions_have_separate_data_rate_tables() {
        let plan = Region::Us915.plan();

        assert_eq!(
            plan.uplink_data_rate(0).expect("uplink DR0").modulation,
            Modulation::LoRa {
                spreading_factor: 10,
                bandwidth_hz: 125_000
            }
        );
        assert_eq!(
            plan.downlink_data_rate(0).expect("downlink DR0").modulation,
            Modulation::LoRa {
                spreading_factor: 5,
                bandwidth_hz: 500_000
            },
            "downlink DR0 is a different rate from uplink DR0"
        );

        // The specification notes uplink DR4 is deliberately downlink DR12.
        assert_eq!(
            plan.uplink_data_rate(4).expect("uplink DR4").modulation,
            plan.downlink_data_rate(12)
                .expect("downlink DR12")
                .modulation
        );

        assert_eq!(
            plan.downlink_data_rate(1),
            None,
            "downlink DR1 through DR7 are reserved"
        );
    }

    /// RP002-1.0.5 Tables 24 and 25.
    #[test]
    fn maximum_payloads_match_both_directions() {
        let plan = Region::Us915.plan();
        assert_eq!(
            plan.max_payload(0, true).expect("uplink DR0"),
            MaxPayload::new(19, 11),
            "the slowest uplink carries very little"
        );
        assert_eq!(
            plan.max_payload(3, false).expect("uplink DR3"),
            MaxPayload::new(250, 242)
        );
        assert_eq!(
            plan.downlink_max_payload(8, true).expect("downlink DR8"),
            MaxPayload::new(61, 53)
        );
        assert_eq!(
            plan.downlink_max_payload(9, false).expect("downlink DR9"),
            MaxPayload::new(137, 129)
        );
    }

    /// RP002-1.0.5 Table 26.
    #[test]
    fn the_rx1_offset_matrix_matches_the_published_table() {
        let plan = Region::Us915.plan();
        let rows: [(u8, [u8; 4]); 9] = [
            (0, [10, 9, 8, 8]),
            (1, [11, 10, 9, 8]),
            (2, [12, 11, 10, 9]),
            (3, [13, 12, 11, 10]),
            (4, [13, 13, 12, 11]),
            (5, [10, 9, 8, 8]),
            (6, [11, 10, 9, 8]),
            (7, [14, 13, 12, 11]),
            (8, [0, 14, 13, 12]),
        ];
        for (uplink, row) in rows {
            for (offset, want) in row.iter().enumerate() {
                assert_eq!(
                    plan.rx1_data_rate(uplink, offset as u8),
                    Some(*want),
                    "DR{uplink} at RX1DROffset {offset}"
                );
            }
        }

        assert_eq!(
            plan.rx1_data_rate(0, 4),
            None,
            "this region allows offsets 0 through 3 only"
        );
    }

    /// RP002-1.0.5 section 3.5.2: 64 channels of 125 kHz plus 8 of 500 kHz.
    #[test]
    fn the_channel_arithmetic_matches_the_published_plan() {
        let plan = Region::Us915.plan();
        assert_eq!(plan.default_channel_count(), 72);
        assert_eq!(plan.channel_frequency_hz(0), Some(902_300_000));
        assert_eq!(
            plan.channel_frequency_hz(63),
            Some(914_900_000),
            "up to 914.9"
        );
        assert_eq!(
            plan.channel_frequency_hz(64),
            Some(903_000_000),
            "then 500 kHz"
        );
        assert_eq!(
            plan.channel_frequency_hz(71),
            Some(914_200_000),
            "up to 914.2"
        );
        assert_eq!(plan.channel_frequency_hz(72), None);
    }

    /// RP002-1.0.5 Table 22 and section 3.5.7.
    #[test]
    fn the_power_and_receive_window_match_the_specification() {
        let plan = Region::Us915.plan();
        assert_eq!(plan.rx2(), (923_300_000, 8), "RX2 is 923.3 MHz at DR8");
        assert_eq!(plan.tx_power_dbm(0, 30), Some(30));
        assert_eq!(plan.tx_power_dbm(1, 30), Some(28));
        assert_eq!(plan.tx_power_dbm(14, 30), Some(2), "index 14 is 2 dBm");
        assert_eq!(plan.tx_power_dbm(15, 30), None, "15 is defined elsewhere");
    }

    /// The FCC constrains this band by dwell time, not duty cycle.
    #[test]
    fn no_duty_cycle_is_published_for_this_band() {
        let plan = Region::Us915.plan();
        assert!(plan.has_dwell_time_limit);
        assert_eq!(
            plan.duty_cycle_permille(902_300_000),
            None,
            "reporting nothing is not permission; the limit is a dwell time"
        );
    }
}

#[cfg(feature = "eu433")]
mod eu433 {
    use super::*;

    /// RP002-1.0.5 Table 30 and section 3.7.2.
    #[test]
    fn the_data_rates_and_channels_match_the_published_tables() {
        let plan = Region::Eu433.plan();
        assert_eq!(
            plan.uplink_data_rate(0).expect("DR0").modulation,
            Modulation::LoRa {
                spreading_factor: 12,
                bandwidth_hz: 125_000
            }
        );
        assert_eq!(
            plan.uplink_data_rate(13).expect("DR13").modulation,
            Modulation::LoRa {
                spreading_factor: 5,
                bandwidth_hz: 125_000
            }
        );
        assert_eq!(
            plan.uplink_data_rate(8),
            None,
            "DR8 through DR11 are reserved in this band"
        );

        assert_eq!(plan.channel_frequency_hz(0), Some(433_175_000));
        assert_eq!(plan.channel_frequency_hz(2), Some(433_575_000));
        assert_eq!(plan.rx2(), (434_665_000, 0));
    }

    /// RP002-1.0.5 section 3.7.2 and Table 33.
    #[test]
    fn the_band_limit_and_power_ceiling_match_the_specification() {
        let plan = Region::Eu433.plan();
        assert_eq!(plan.default_max_eirp_dbm, 12, "below 12 dBm EIRP");
        assert_eq!(
            plan.duty_cycle_permille(433_175_000),
            Some(100),
            "the band is limited to 10%"
        );
        assert_eq!(plan.tx_power_dbm(5, 12), Some(2), "index 5 is 10 dB down");
        assert_eq!(plan.tx_power_dbm(6, 12), None, "6 through 14 are reserved");
    }
}

/// A plan built from a caller's own tables is not a second-class citizen.
#[test]
fn a_custom_plan_answers_every_question_a_named_one_does() {
    static RATES: [Option<DataRate>; 2] = [
        Some(DataRate::lora(12, 125_000, 250)),
        Some(DataRate::lora(7, 125_000, 5_470)),
    ];
    static PAYLOADS: [Option<MaxPayload>; 2] = [
        Some(MaxPayload::new(59, 51)),
        Some(MaxPayload::new(230, 222)),
    ];
    static CHANNELS: [ChannelBlock; 1] = [ChannelBlock::new(915_000_000, 500_000, 4, 0, 1)];
    static BANDS: [SubBand; 1] = [SubBand::new(915_000_000, 917_000_000, 1000, 30)];
    static RX1: [&[u8]; 2] = [&[0], &[1]];
    static BACKOFF: [Option<u8>; 2] = [None, Some(0)];

    let plan = ChannelPlan {
        name: "private-915",
        uplink_data_rates: &RATES,
        downlink_data_rates: &RATES,
        max_payload_repeater: &PAYLOADS,
        max_payload_direct: &PAYLOADS,
        downlink_max_payload_repeater: &PAYLOADS,
        downlink_max_payload_direct: &PAYLOADS,
        max_payload_dwell_limited: None,
        join_channels: &CHANNELS,
        default_channels: &CHANNELS,
        sub_bands: &BANDS,
        default_max_eirp_dbm: 30,
        tx_power_step_db: 2,
        max_tx_power_index: 7,
        rx1_data_rate_offsets: &RX1,
        rx1_data_rate_offsets_dwell_limited: None,
        max_rx1_data_rate_offset: 0,
        rx2_frequency_hz: 915_000_000,
        rx2_data_rate: 0,
        data_rate_backoff: &BACKOFF,
        beacon: Beacon {
            data_rate: 0,
            frequency_hz: 915_000_000,
            ping_slot_frequency_hz: 915_000_000,
        },
        has_dwell_time_limit: false,
    };

    assert_eq!(plan.default_channel_count(), 4);
    assert_eq!(plan.channel_frequency_hz(3), Some(916_500_000));
    assert_eq!(
        plan.duty_cycle_permille(915_000_000),
        Some(1000),
        "a licensed deployment may hold the channel continuously"
    );
    assert_eq!(
        plan.link_settings(1)
            .expect("DR1 is LoRa")
            .spreading_factor(),
        7
    );
    assert_eq!(
        plan.max_payload(1, false).expect("DR1"),
        MaxPayload::new(230, 222)
    );
}

#[test]
fn a_plan_may_borrow_tables_that_are_not_static() {
    // A plan assembled at runtime, from storage owned by the caller rather than
    // baked into the binary. This is what a host that reads a plan from a file or
    // builds one across a language boundary does.
    let rates = vec![
        Some(DataRate::lora(10, 125_000, 980)),
        Some(DataRate::fsk(50_000)),
    ];
    let payloads = vec![
        Some(MaxPayload::new(59, 51)),
        Some(MaxPayload::new(230, 222)),
    ];
    let channels = vec![ChannelBlock::new(869_400_000, 200_000, 2, 0, 1)];
    let bands = vec![SubBand::new(869_400_000, 869_650_000, 100, 27)];
    let rx1_rows = [vec![0u8], vec![1u8]];
    let rx1: Vec<&[u8]> = rx1_rows.iter().map(Vec::as_slice).collect();
    let backoff = vec![None, Some(0)];
    let name = String::from("relief-869");

    let plan = ChannelPlan {
        name: &name,
        uplink_data_rates: &rates,
        downlink_data_rates: &rates,
        max_payload_repeater: &payloads,
        max_payload_direct: &payloads,
        downlink_max_payload_repeater: &payloads,
        downlink_max_payload_direct: &payloads,
        max_payload_dwell_limited: None,
        join_channels: &channels,
        default_channels: &channels,
        sub_bands: &bands,
        default_max_eirp_dbm: 27,
        tx_power_step_db: 2,
        max_tx_power_index: 7,
        rx1_data_rate_offsets: &rx1,
        rx1_data_rate_offsets_dwell_limited: None,
        max_rx1_data_rate_offset: 0,
        rx2_frequency_hz: 869_525_000,
        rx2_data_rate: 0,
        data_rate_backoff: &backoff,
        beacon: Beacon {
            data_rate: 0,
            frequency_hz: 869_525_000,
            ping_slot_frequency_hz: 869_525_000,
        },
        has_dwell_time_limit: false,
    };

    assert_eq!(plan.name, "relief-869");
    assert_eq!(plan.channel_frequency_hz(1), Some(869_600_000));
    assert_eq!(plan.duty_cycle_permille(869_500_000), Some(100));
    assert_eq!(plan.max_eirp_dbm(869_500_000), 27);
    assert_eq!(
        plan.link_settings(0)
            .expect("DR0 is LoRa")
            .spreading_factor(),
        10
    );
    // A data rate carried by FSK has no LoRa settings to report.
    assert!(plan.link_settings(1).is_none());
    assert_eq!(plan.rx2(), (869_525_000, 0));
}

#[cfg(feature = "au915")]
mod au915 {
    use super::*;

    /// RP002-1.0.5 Tables 39, 44 and 46.
    #[test]
    fn the_dwell_limit_shrinks_what_the_slow_data_rates_carry() {
        let plan = Region::Au915.plan();
        assert!(plan.has_dwell_time_limit);

        // With no dwell limit the two slowest rates carry a small frame.
        assert_eq!(
            plan.max_payload(0, true).expect("DR0"),
            MaxPayload::new(59, 51)
        );
        // Inside 400 ms they carry nothing at all.
        assert_eq!(
            plan.max_payload_dwell_limited(0),
            None,
            "DR0 under a dwell limit"
        );
        assert_eq!(
            plan.max_payload_dwell_limited(1),
            None,
            "DR1 under a dwell limit"
        );
        assert_eq!(
            plan.max_payload_dwell_limited(2)
                .expect("DR2 under a dwell limit"),
            MaxPayload::new(19, 11)
        );

        assert_eq!(
            plan.uplink_data_rate(8),
            None,
            "uplink DR8 is reserved here"
        );
        assert_eq!(
            plan.rx1_data_rate(0, 0),
            Some(8),
            "RX1 starts at downlink DR8"
        );
        assert_eq!(plan.rx2(), (923_300_000, 8));
        assert_eq!(plan.channel_frequency_hz(0), Some(915_200_000));
        assert_eq!(plan.channel_frequency_hz(64), Some(915_900_000));
        assert_eq!(plan.default_channel_count(), 72);
    }
}

#[cfg(feature = "cn470")]
mod cn470 {
    use super::*;

    /// RP002-1.0.5 Tables 50, 54 and 56.
    #[test]
    fn the_slowest_data_rate_carries_nothing_in_this_band() {
        let plan = Region::Cn470.plan();
        assert_eq!(
            plan.max_payload(0, true),
            None,
            "one second on air leaves no room for a frame at SF12"
        );
        assert_eq!(
            plan.max_payload(1, true).expect("DR1"),
            MaxPayload::new(31, 23)
        );
        assert_eq!(
            plan.uplink_data_rate(6).expect("DR6").modulation,
            Modulation::LoRa {
                spreading_factor: 7,
                bandwidth_hz: 500_000
            }
        );
        assert_eq!(
            plan.rx1_data_rate(1, 5),
            Some(1),
            "the DR1 row never falls to DR0"
        );
        assert_eq!(
            plan.rx2(),
            (486_900_000, 1),
            "RX2 runs at DR1 here, not DR0"
        );
        assert_eq!(plan.default_max_eirp_dbm, 19);
    }
}

#[cfg(feature = "as923")]
mod as923 {
    use super::*;

    /// RP002-1.0.5 Tables 74 and 75, the two RX1 mappings.
    #[test]
    fn a_downlink_dwell_limit_selects_a_different_rx1_mapping() {
        let plan = Region::As923.plan();
        assert_eq!(
            plan.rx1_data_rate(0, 0),
            Some(0),
            "no dwell limit reaches DR0"
        );
        assert_eq!(
            plan.rx1_data_rate_dwell_limited(0, 0),
            Some(2),
            "under a dwell limit the floor rises to DR2"
        );
        assert_eq!(
            plan.rx1_data_rate(5, 7),
            Some(7),
            "the offsets run to 7 here"
        );
        assert_eq!(plan.rx1_data_rate_dwell_limited(5, 2), Some(3));
        assert_eq!(plan.rx2(), (923_200_000, 2));
        assert_eq!(plan.channel_frequency_hz(0), Some(923_200_000));
        assert_eq!(plan.channel_frequency_hz(1), Some(923_400_000));
        assert_eq!(plan.duty_cycle_permille(923_200_000), Some(10), "1%");
    }
}

#[cfg(feature = "kr920")]
mod kr920 {
    use super::*;

    /// RP002-1.0.5 Tables 77, 80 and 87.
    #[test]
    fn the_power_ceiling_steps_across_the_band() {
        let plan = Region::Kr920.plan();
        assert_eq!(
            plan.max_eirp_dbm(921_500_000),
            10,
            "the lower sub-band allows 10 dBm"
        );
        assert_eq!(
            plan.max_eirp_dbm(922_500_000),
            14,
            "and the upper one 14 dBm"
        );
        assert_eq!(
            plan.uplink_data_rate(6),
            None,
            "DR6 through DR11 are reserved"
        );
        assert_eq!(
            plan.uplink_data_rate(13).expect("DR13").modulation,
            Modulation::LoRa {
                spreading_factor: 5,
                bandwidth_hz: 125_000
            }
        );
        assert_eq!(plan.rx2(), (921_900_000, 0));
        assert_eq!(plan.channel_frequency_hz(0), Some(922_100_000));
    }
}

#[cfg(feature = "in865")]
mod in865 {
    use super::*;

    /// RP002-1.0.5 Tables 89, 91 and 98.
    #[test]
    fn the_channels_are_not_evenly_spaced_and_the_offsets_run_to_seven() {
        let plan = Region::In865.plan();
        assert_eq!(plan.channel_frequency_hz(0), Some(865_062_500));
        assert_eq!(plan.channel_frequency_hz(1), Some(865_402_500));
        assert_eq!(plan.channel_frequency_hz(2), Some(865_985_000));
        assert_eq!(plan.default_channel_count(), 3);

        assert_eq!(
            plan.rx1_data_rate(0, 7),
            Some(2),
            "offset 7 is allowed here"
        );
        assert_eq!(plan.rx1_data_rate(0, 8), None);
        assert_eq!(plan.uplink_data_rate(6), None, "DR6 is reserved");
        assert_eq!(
            plan.uplink_data_rate(7).expect("DR7").modulation,
            Modulation::Fsk {
                bitrate_bps: 50_000
            }
        );
        assert_eq!(plan.rx2(), (866_550_000, 2));
        assert_eq!(plan.beacon.data_rate, 4, "India beacons at DR4");
    }
}

#[cfg(feature = "ru864")]
mod ru864 {
    use super::*;

    /// RP002-1.0.5 Tables 100, 102 and 109.
    #[test]
    fn the_two_default_channels_and_split_beacon_match_the_specification() {
        let plan = Region::Ru864.plan();
        assert_eq!(plan.channel_frequency_hz(0), Some(868_900_000));
        assert_eq!(plan.channel_frequency_hz(1), Some(869_100_000));
        assert_eq!(plan.default_channel_count(), 2);
        assert_eq!(plan.rx2(), (869_100_000, 0));
        assert_eq!(
            plan.beacon.frequency_hz, 869_100_000,
            "the beacon and the ping slot sit on different frequencies here"
        );
        assert_eq!(plan.beacon.ping_slot_frequency_hz, 868_900_000);
        assert_eq!(plan.duty_cycle_permille(869_100_000), Some(10), "1%");
    }
}

/// Every compiled-in region answers the questions a caller asks of any of them.
#[test]
fn every_region_is_self_consistent() {
    for region in Region::all() {
        let plan = region.plan();
        assert!(!plan.name.is_empty(), "{region:?} has a band name");
        assert!(
            plan.default_channel_count() > 0,
            "{region:?} defines channels"
        );
        assert!(
            plan.uplink_data_rate(0).is_some() || plan.name == "CN470-510",
            "{region:?} defines DR0"
        );
        assert!(
            plan.rx1_data_rate_offsets.len() <= plan.uplink_data_rates.len(),
            "{region:?} has an RX1 row for no more than its uplink data rates"
        );
        for (index, row) in plan.rx1_data_rate_offsets.iter().enumerate() {
            assert_eq!(
                row.len(),
                usize::from(plan.max_rx1_data_rate_offset) + 1,
                "{region:?} DR{index} has one entry per allowed RX1 offset"
            );
        }
        assert!(
            plan.downlink_data_rate(plan.rx2_data_rate).is_some(),
            "{region:?} RX2 names a data rate it defines"
        );
        assert_eq!(
            plan.data_rate_backoff.len(),
            plan.uplink_data_rates.len(),
            "{region:?} has a back-off entry per uplink data rate"
        );
    }
}
