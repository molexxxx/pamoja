//! The published channel plans, transcribed from RP002-1.0.5.
//!
//! Each region is behind its own cargo feature, all on by default, because a
//! device only ever operates in one region and a microcontroller should not
//! carry the other nine in flash.

use super::{Beacon, ChannelBlock, ChannelPlan, DataRate, MaxPayload, SubBand};

/// A named regional channel plan.
///
/// This is a convenience over [`ChannelPlan`], not the only way to get one: a
/// deployment on licensed spectrum builds its own plan and every method still
/// applies. Each variant is behind its own cargo feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Region {
    /// Europe and everywhere else following ETSI EN 300 220, 863-870 MHz.
    #[cfg(feature = "eu868")]
    Eu868,
    /// The United States, Canada, and ITU Region 2, 902-928 MHz.
    #[cfg(feature = "us915")]
    Us915,
    /// The 433 MHz ISM band.
    #[cfg(feature = "eu433")]
    Eu433,
    /// Australia and the countries following it, 915-928 MHz.
    #[cfg(feature = "au915")]
    Au915,
    /// China, 470-510 MHz.
    #[cfg(feature = "cn470")]
    Cn470,
    /// Asia and the Pacific, 923 MHz.
    #[cfg(feature = "as923")]
    As923,
    /// South Korea, 920-923 MHz.
    #[cfg(feature = "kr920")]
    Kr920,
    /// India, 865-867 MHz.
    #[cfg(feature = "in865")]
    In865,
    /// Russia, 864-870 MHz.
    #[cfg(feature = "ru864")]
    Ru864,
}

impl Region {
    /// Returns the channel plan this region names.
    ///
    /// # Returns
    ///
    /// The plan, whose tables come straight from RP002-1.0.5.
    pub const fn plan(self) -> &'static ChannelPlan {
        match self {
            #[cfg(feature = "eu868")]
            Region::Eu868 => &EU868,
            #[cfg(feature = "us915")]
            Region::Us915 => &US915,
            #[cfg(feature = "eu433")]
            Region::Eu433 => &EU433,
            #[cfg(feature = "au915")]
            Region::Au915 => &AU915,
            #[cfg(feature = "cn470")]
            Region::Cn470 => &CN470,
            #[cfg(feature = "as923")]
            Region::As923 => &AS923,
            #[cfg(feature = "kr920")]
            Region::Kr920 => &KR920,
            #[cfg(feature = "in865")]
            Region::In865 => &IN865,
            #[cfg(feature = "ru864")]
            Region::Ru864 => &RU864,
        }
    }

    /// Returns the specification's name for this region's band.
    ///
    /// # Returns
    ///
    /// The band name, such as `"EU863-870"`.
    pub const fn name(self) -> &'static str {
        self.plan().name
    }

    /// Returns every region this build carries.
    ///
    /// A build that selects only its own region gets a one-element slice, which
    /// is the point of the per-region features.
    ///
    /// # Returns
    ///
    /// The compiled-in regions.
    pub const fn all() -> &'static [Region] {
        &[
            #[cfg(feature = "eu868")]
            Region::Eu868,
            #[cfg(feature = "us915")]
            Region::Us915,
            #[cfg(feature = "eu433")]
            Region::Eu433,
            #[cfg(feature = "au915")]
            Region::Au915,
            #[cfg(feature = "cn470")]
            Region::Cn470,
            #[cfg(feature = "as923")]
            Region::As923,
            #[cfg(feature = "kr920")]
            Region::Kr920,
            #[cfg(feature = "in865")]
            Region::In865,
            #[cfg(feature = "ru864")]
            Region::Ru864,
        ]
    }
}

// EU863-870, RP002-1.0.5 section 3.4.

/// RP002-1.0.5 Table 10: EU863-870 TX data rate.
#[cfg(feature = "eu868")]
static EU868_DATA_RATES: [Option<DataRate>; 14] = [
    Some(DataRate::lora(12, 125_000, 250)),
    Some(DataRate::lora(11, 125_000, 440)),
    Some(DataRate::lora(10, 125_000, 980)),
    Some(DataRate::lora(9, 125_000, 1_760)),
    Some(DataRate::lora(8, 125_000, 3_125)),
    Some(DataRate::lora(7, 125_000, 5_470)),
    Some(DataRate::lora(7, 250_000, 11_000)),
    Some(DataRate::fsk(50_000)),
    Some(DataRate::lr_fhss(1, 3, 137_000, 162)),
    Some(DataRate::lr_fhss(2, 3, 137_000, 325)),
    Some(DataRate::lr_fhss(1, 3, 336_000, 162)),
    Some(DataRate::lr_fhss(2, 3, 336_000, 325)),
    Some(DataRate::lora(6, 125_000, 9_375)),
    Some(DataRate::lora(5, 125_000, 15_625)),
];

/// RP002-1.0.5 Table 15: EU863-870 maximum payload size (repeater compatible).
#[cfg(feature = "eu868")]
static EU868_MAX_PAYLOAD_REPEATER: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(58, 50)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(58, 50)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 16: EU863-870 maximum payload size (not repeater compatible).
#[cfg(feature = "eu868")]
static EU868_MAX_PAYLOAD_DIRECT: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(58, 50)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(58, 50)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
];

/// RP002-1.0.5 Table 17: EU863-870 downlink RX1 data rate mapping.
#[cfg(feature = "eu868")]
static EU868_RX1: [&[u8]; 14] = [
    &[0, 0, 0, 0, 0, 0],
    &[1, 0, 0, 0, 0, 0],
    &[2, 1, 0, 0, 0, 0],
    &[3, 2, 1, 0, 0, 0],
    &[4, 3, 2, 1, 0, 0],
    &[5, 4, 3, 2, 1, 0],
    &[6, 5, 4, 3, 2, 1],
    &[7, 6, 5, 4, 3, 2],
    &[1, 0, 0, 0, 0, 0],
    &[2, 1, 0, 0, 0, 0],
    &[1, 0, 0, 0, 0, 0],
    &[2, 1, 0, 0, 0, 0],
    &[12, 5, 4, 3, 2, 1],
    &[13, 12, 5, 4, 3, 2],
];

/// RP002-1.0.5 Table 12: EU863-870 data rate back-off.
#[cfg(feature = "eu868")]
static EU868_BACKOFF: [Option<u8>; 14] = [
    None,
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(4),
    Some(5),
    Some(6),
    Some(0),
    Some(8),
    Some(0),
    Some(10),
    Some(5),
    Some(12),
];

/// RP002-1.0.5 Table 8 and Table 9: the three default and join channels.
#[cfg(feature = "eu868")]
static EU868_CHANNELS: [ChannelBlock; 1] = [ChannelBlock::new(868_100_000, 200_000, 3, 0, 5)];

/// The ETSI sub-bands the default channels fall in.
///
/// EN 300 220 divides the band, and the 868.0-868.6 MHz sub-band carrying the
/// three mandatory channels is limited to 1%. The RX2 downlink frequency sits in
/// the 869.4-869.65 MHz sub-band, which allows 10% at a higher ceiling.
#[cfg(feature = "eu868")]
static EU868_SUB_BANDS: [SubBand; 2] = [
    SubBand::new(868_000_000, 868_600_000, 10, 16),
    SubBand::new(869_400_000, 869_650_000, 100, 27),
];

/// The EU863-870 channel plan, RP002-1.0.5 section 3.4.
#[cfg(feature = "eu868")]
pub static EU868: ChannelPlan = ChannelPlan {
    name: "EU863-870",
    uplink_data_rates: &EU868_DATA_RATES,
    downlink_data_rates: &EU868_DATA_RATES,
    max_payload_repeater: &EU868_MAX_PAYLOAD_REPEATER,
    max_payload_direct: &EU868_MAX_PAYLOAD_DIRECT,
    downlink_max_payload_repeater: &EU868_MAX_PAYLOAD_REPEATER,
    downlink_max_payload_direct: &EU868_MAX_PAYLOAD_DIRECT,
    max_payload_dwell_limited: None,
    join_channels: &EU868_CHANNELS,
    default_channels: &EU868_CHANNELS,
    sub_bands: &EU868_SUB_BANDS,
    default_max_eirp_dbm: 16,
    tx_power_step_db: 2,
    max_tx_power_index: 7,
    rx1_data_rate_offsets: &EU868_RX1,
    rx1_data_rate_offsets_dwell_limited: None,
    max_rx1_data_rate_offset: 5,
    rx2_frequency_hz: 869_525_000,
    rx2_data_rate: 0,
    data_rate_backoff: &EU868_BACKOFF,
    beacon: Beacon {
        data_rate: 3,
        frequency_hz: 869_525_000,
        ping_slot_frequency_hz: 869_525_000,
    },
    has_dwell_time_limit: false,
};

// US902-928, RP002-1.0.5 section 3.5.

/// RP002-1.0.5 Table 19: US902-928 uplink data rates.
#[cfg(feature = "us915")]
static US915_UPLINK_DATA_RATES: [Option<DataRate>; 9] = [
    Some(DataRate::lora(10, 125_000, 980)),
    Some(DataRate::lora(9, 125_000, 1_760)),
    Some(DataRate::lora(8, 125_000, 3_125)),
    Some(DataRate::lora(7, 125_000, 5_470)),
    Some(DataRate::lora(8, 500_000, 12_500)),
    Some(DataRate::lr_fhss(1, 3, 1_523_000, 162)),
    Some(DataRate::lr_fhss(2, 3, 1_523_000, 325)),
    Some(DataRate::lora(6, 125_000, 9_375)),
    Some(DataRate::lora(5, 125_000, 15_625)),
];

/// RP002-1.0.5 Table 20: US902-928 downlink data rates.
#[cfg(feature = "us915")]
static US915_DOWNLINK_DATA_RATES: [Option<DataRate>; 15] = [
    Some(DataRate::lora(5, 500_000, 62_500)),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some(DataRate::lora(12, 500_000, 980)),
    Some(DataRate::lora(11, 500_000, 1_760)),
    Some(DataRate::lora(10, 500_000, 3_900)),
    Some(DataRate::lora(9, 500_000, 7_000)),
    Some(DataRate::lora(8, 500_000, 12_500)),
    Some(DataRate::lora(7, 500_000, 21_900)),
    Some(DataRate::lora(6, 500_000, 37_500)),
];

/// RP002-1.0.5 Table 24: US902-928 uplink maximum payload (repeater compatible).
#[cfg(feature = "us915")]
static US915_MAX_PAYLOAD_REPEATER: [Option<MaxPayload>; 9] = [
    Some(MaxPayload::new(19, 11)),
    Some(MaxPayload::new(61, 53)),
    Some(MaxPayload::new(133, 125)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(58, 50)),
    Some(MaxPayload::new(133, 125)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 25: US902-928 uplink maximum payload (not repeater compatible).
#[cfg(feature = "us915")]
static US915_MAX_PAYLOAD_DIRECT: [Option<MaxPayload>; 9] = [
    Some(MaxPayload::new(19, 11)),
    Some(MaxPayload::new(61, 53)),
    Some(MaxPayload::new(133, 125)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(58, 50)),
    Some(MaxPayload::new(133, 125)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
];

/// RP002-1.0.5 Table 24: US902-928 downlink maximum payload (repeater compatible).
#[cfg(feature = "us915")]
static US915_DOWNLINK_MAX_PAYLOAD_REPEATER: [Option<MaxPayload>; 15] = [
    Some(MaxPayload::new(230, 222)),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(61, 53)),
    Some(MaxPayload::new(137, 129)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 25: US902-928 downlink maximum payload (not repeater compatible).
#[cfg(feature = "us915")]
static US915_DOWNLINK_MAX_PAYLOAD_DIRECT: [Option<MaxPayload>; 15] = [
    Some(MaxPayload::new(250, 242)),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(61, 53)),
    Some(MaxPayload::new(137, 129)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
];

/// RP002-1.0.5 Table 26: US902-928 downlink RX1 data rate mapping.
#[cfg(feature = "us915")]
static US915_RX1: [&[u8]; 9] = [
    &[10, 9, 8, 8],
    &[11, 10, 9, 8],
    &[12, 11, 10, 9],
    &[13, 12, 11, 10],
    &[13, 13, 12, 11],
    &[10, 9, 8, 8],
    &[11, 10, 9, 8],
    &[14, 13, 12, 11],
    &[0, 14, 13, 12],
];

/// RP002-1.0.5 Table 21: US902-928 uplink data rate back-off.
#[cfg(feature = "us915")]
static US915_BACKOFF: [Option<u8>; 9] = [
    None,
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(0),
    Some(5),
    Some(3),
    Some(7),
];

/// The 64 125 kHz channels and the 8 500 kHz channels, RP002-1.0.5 section 3.5.2.
#[cfg(feature = "us915")]
static US915_CHANNELS: [ChannelBlock; 2] = [
    ChannelBlock::new(902_300_000, 200_000, 64, 0, 3),
    ChannelBlock::new(903_000_000, 1_600_000, 8, 4, 4),
];

/// The join channels: a random 125 kHz channel at DR0 and a 500 kHz one at DR4.
#[cfg(feature = "us915")]
static US915_JOIN_CHANNELS: [ChannelBlock; 2] = [
    ChannelBlock::new(902_300_000, 200_000, 64, 0, 0),
    ChannelBlock::new(903_000_000, 1_600_000, 8, 4, 4),
];

/// The US902-928 channel plan, RP002-1.0.5 section 3.5.
///
/// The FCC constrains this band by dwell time rather than duty cycle, so the
/// plan publishes no sub-band duty limit and
/// [`duty_cycle_permille`](ChannelPlan::duty_cycle_permille) reports `None`.
#[cfg(feature = "us915")]
pub static US915: ChannelPlan = ChannelPlan {
    name: "US902-928",
    uplink_data_rates: &US915_UPLINK_DATA_RATES,
    downlink_data_rates: &US915_DOWNLINK_DATA_RATES,
    max_payload_repeater: &US915_MAX_PAYLOAD_REPEATER,
    max_payload_direct: &US915_MAX_PAYLOAD_DIRECT,
    downlink_max_payload_repeater: &US915_DOWNLINK_MAX_PAYLOAD_REPEATER,
    downlink_max_payload_direct: &US915_DOWNLINK_MAX_PAYLOAD_DIRECT,
    max_payload_dwell_limited: None,
    join_channels: &US915_JOIN_CHANNELS,
    default_channels: &US915_CHANNELS,
    sub_bands: &[],
    default_max_eirp_dbm: 30,
    tx_power_step_db: 2,
    max_tx_power_index: 14,
    rx1_data_rate_offsets: &US915_RX1,
    rx1_data_rate_offsets_dwell_limited: None,
    max_rx1_data_rate_offset: 3,
    rx2_frequency_hz: 923_300_000,
    rx2_data_rate: 8,
    data_rate_backoff: &US915_BACKOFF,
    beacon: Beacon {
        data_rate: 8,
        frequency_hz: 923_300_000,
        ping_slot_frequency_hz: 923_300_000,
    },
    has_dwell_time_limit: true,
};

// EU433, RP002-1.0.5 section 3.7.

/// RP002-1.0.5 Table 30: EU433 data rate and TXPower.
#[cfg(feature = "eu433")]
static EU433_DATA_RATES: [Option<DataRate>; 14] = [
    Some(DataRate::lora(12, 125_000, 250)),
    Some(DataRate::lora(11, 125_000, 440)),
    Some(DataRate::lora(10, 125_000, 980)),
    Some(DataRate::lora(9, 125_000, 1_760)),
    Some(DataRate::lora(8, 125_000, 3_125)),
    Some(DataRate::lora(7, 125_000, 5_470)),
    Some(DataRate::lora(7, 250_000, 11_000)),
    Some(DataRate::fsk(50_000)),
    None,
    None,
    None,
    None,
    Some(DataRate::lora(6, 125_000, 9_375)),
    Some(DataRate::lora(5, 125_000, 15_625)),
];

/// RP002-1.0.5 Table 36: EU433 maximum payload size (repeater compatible).
#[cfg(feature = "eu433")]
static EU433_MAX_PAYLOAD_REPEATER: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 37: EU433 maximum payload size (not repeater compatible).
#[cfg(feature = "eu433")]
static EU433_MAX_PAYLOAD_DIRECT: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
];

/// RP002-1.0.5 Table 38: EU433 downlink RX1 data rate mapping.
///
/// DR8 through DR11 are reserved in this region, so their rows carry the DR0
/// fallback rather than a mapping the specification does not define.
#[cfg(feature = "eu433")]
static EU433_RX1: [&[u8]; 14] = [
    &[0, 0, 0, 0, 0, 0],
    &[1, 0, 0, 0, 0, 0],
    &[2, 1, 0, 0, 0, 0],
    &[3, 2, 1, 0, 0, 0],
    &[4, 3, 2, 1, 0, 0],
    &[5, 4, 3, 2, 1, 0],
    &[6, 5, 4, 3, 2, 1],
    &[7, 6, 5, 4, 3, 2],
    &[0, 0, 0, 0, 0, 0],
    &[0, 0, 0, 0, 0, 0],
    &[0, 0, 0, 0, 0, 0],
    &[0, 0, 0, 0, 0, 0],
    &[12, 5, 4, 3, 2, 1],
    &[13, 12, 5, 4, 3, 2],
];

/// RP002-1.0.5 Table 32: EU433 data rate back-off.
#[cfg(feature = "eu433")]
static EU433_BACKOFF: [Option<u8>; 14] = [
    None,
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(4),
    Some(5),
    Some(6),
    None,
    None,
    None,
    None,
    Some(5),
    Some(12),
];

/// RP002-1.0.5 Table 29: the three default and join channels.
#[cfg(feature = "eu433")]
static EU433_CHANNELS: [ChannelBlock; 1] = [ChannelBlock::new(433_175_000, 200_000, 3, 0, 5)];

/// The whole EU433 band carries one limit: below 12 dBm EIRP and under 10% duty.
#[cfg(feature = "eu433")]
static EU433_SUB_BANDS: [SubBand; 1] = [SubBand::new(433_050_000, 434_790_000, 100, 12)];

/// The EU433 channel plan, RP002-1.0.5 section 3.7.
#[cfg(feature = "eu433")]
pub static EU433: ChannelPlan = ChannelPlan {
    name: "EU433",
    uplink_data_rates: &EU433_DATA_RATES,
    downlink_data_rates: &EU433_DATA_RATES,
    max_payload_repeater: &EU433_MAX_PAYLOAD_REPEATER,
    max_payload_direct: &EU433_MAX_PAYLOAD_DIRECT,
    downlink_max_payload_repeater: &EU433_MAX_PAYLOAD_REPEATER,
    downlink_max_payload_direct: &EU433_MAX_PAYLOAD_DIRECT,
    max_payload_dwell_limited: None,
    join_channels: &EU433_CHANNELS,
    default_channels: &EU433_CHANNELS,
    sub_bands: &EU433_SUB_BANDS,
    default_max_eirp_dbm: 12,
    tx_power_step_db: 2,
    max_tx_power_index: 5,
    rx1_data_rate_offsets: &EU433_RX1,
    rx1_data_rate_offsets_dwell_limited: None,
    max_rx1_data_rate_offset: 5,
    rx2_frequency_hz: 434_665_000,
    rx2_data_rate: 0,
    data_rate_backoff: &EU433_BACKOFF,
    beacon: Beacon {
        data_rate: 3,
        frequency_hz: 434_665_000,
        ping_slot_frequency_hz: 434_665_000,
    },
    has_dwell_time_limit: false,
};

// AU915-928, RP002-1.0.5 section 3.8.

/// RP002-1.0.5 Table 39: AU915-928 uplink data rates.
#[cfg(feature = "au915")]
static AU915_UPLINK_DATA_RATES: [Option<DataRate>; 11] = [
    Some(DataRate::lora(12, 125_000, 250)),
    Some(DataRate::lora(11, 125_000, 440)),
    Some(DataRate::lora(10, 125_000, 980)),
    Some(DataRate::lora(9, 125_000, 1_760)),
    Some(DataRate::lora(8, 125_000, 3_125)),
    Some(DataRate::lora(7, 125_000, 5_470)),
    Some(DataRate::lora(8, 500_000, 12_500)),
    Some(DataRate::lr_fhss(1, 3, 1_523_000, 162)),
    None,
    Some(DataRate::lora(6, 125_000, 9_375)),
    Some(DataRate::lora(5, 125_000, 15_625)),
];

/// RP002-1.0.5 Table 40: AU915-928 downlink data rates.
#[cfg(feature = "au915")]
static AU915_DOWNLINK_DATA_RATES: [Option<DataRate>; 15] = [
    Some(DataRate::lora(5, 500_000, 62_500)),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some(DataRate::lora(12, 500_000, 980)),
    Some(DataRate::lora(11, 500_000, 1_760)),
    Some(DataRate::lora(10, 500_000, 3_900)),
    Some(DataRate::lora(9, 500_000, 7_000)),
    Some(DataRate::lora(8, 500_000, 12_500)),
    Some(DataRate::lora(7, 500_000, 21_900)),
    Some(DataRate::lora(6, 500_000, 37_500)),
];

/// RP002-1.0.5 Table 44: AU915-928 uplink maximum payload, no dwell limit.
#[cfg(feature = "au915")]
static AU915_MAX_PAYLOAD_REPEATER: [Option<MaxPayload>; 11] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(58, 50)),
    None,
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 45: AU915-928 uplink maximum payload, no dwell limit.
#[cfg(feature = "au915")]
static AU915_MAX_PAYLOAD_DIRECT: [Option<MaxPayload>; 11] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(58, 50)),
    None,
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
];

/// RP002-1.0.5 Table 44: AU915-928 uplink maximum payload under a 400 ms dwell.
///
/// The two slowest data rates carry nothing at all inside 400 ms, which is why
/// a device boots assuming the limit applies until told otherwise.
#[cfg(feature = "au915")]
static AU915_MAX_PAYLOAD_DWELL: [Option<MaxPayload>; 11] = [
    None,
    None,
    Some(MaxPayload::new(19, 11)),
    Some(MaxPayload::new(61, 53)),
    Some(MaxPayload::new(133, 125)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(58, 50)),
    None,
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 44: AU915-928 downlink maximum payload (repeater compatible).
#[cfg(feature = "au915")]
static AU915_DOWNLINK_MAX_PAYLOAD_REPEATER: [Option<MaxPayload>; 15] = [
    Some(MaxPayload::new(230, 222)),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(61, 53)),
    Some(MaxPayload::new(137, 129)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 45: AU915-928 downlink maximum payload (not repeater compatible).
#[cfg(feature = "au915")]
static AU915_DOWNLINK_MAX_PAYLOAD_DIRECT: [Option<MaxPayload>; 15] = [
    Some(MaxPayload::new(250, 242)),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(61, 53)),
    Some(MaxPayload::new(137, 129)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
];

/// RP002-1.0.5 Table 46: AU915-928 downlink RX1 data rate mapping.
#[cfg(feature = "au915")]
static AU915_RX1: [&[u8]; 11] = [
    &[8, 8, 8, 8, 8, 8],
    &[9, 8, 8, 8, 8, 8],
    &[10, 9, 8, 8, 8, 8],
    &[11, 10, 9, 8, 8, 8],
    &[12, 11, 10, 9, 8, 8],
    &[13, 12, 11, 10, 9, 8],
    &[13, 13, 12, 11, 10, 9],
    &[9, 8, 8, 8, 8, 8],
    &[8, 8, 8, 8, 8, 8],
    &[14, 13, 12, 11, 10, 9],
    &[0, 14, 13, 12, 11, 10],
];

/// RP002-1.0.5 Table 41: AU915-928 uplink data rate back-off, no dwell limit.
#[cfg(feature = "au915")]
static AU915_BACKOFF: [Option<u8>; 11] = [
    None,
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(4),
    Some(5),
    Some(0),
    None,
    Some(5),
    Some(9),
];

/// The 64 125 kHz uplink channels and the 8 500 kHz ones, section 3.8.2.
#[cfg(feature = "au915")]
static AU915_CHANNELS: [ChannelBlock; 2] = [
    ChannelBlock::new(915_200_000, 200_000, 64, 0, 5),
    ChannelBlock::new(915_900_000, 1_600_000, 8, 6, 7),
];

/// The AU915-928 channel plan, RP002-1.0.5 section 3.8.
///
/// Australia limits transmissions by dwell time rather than duty cycle, so
/// [`duty_cycle_permille`](ChannelPlan::duty_cycle_permille) reports nothing and
/// the dwell-limited payload table is the one that matters until the network
/// says otherwise.
#[cfg(feature = "au915")]
pub static AU915: ChannelPlan = ChannelPlan {
    name: "AU915-928",
    uplink_data_rates: &AU915_UPLINK_DATA_RATES,
    downlink_data_rates: &AU915_DOWNLINK_DATA_RATES,
    max_payload_repeater: &AU915_MAX_PAYLOAD_REPEATER,
    max_payload_direct: &AU915_MAX_PAYLOAD_DIRECT,
    downlink_max_payload_repeater: &AU915_DOWNLINK_MAX_PAYLOAD_REPEATER,
    downlink_max_payload_direct: &AU915_DOWNLINK_MAX_PAYLOAD_DIRECT,
    max_payload_dwell_limited: Some(&AU915_MAX_PAYLOAD_DWELL),
    join_channels: &AU915_CHANNELS,
    default_channels: &AU915_CHANNELS,
    sub_bands: &[],
    default_max_eirp_dbm: 30,
    tx_power_step_db: 2,
    max_tx_power_index: 14,
    rx1_data_rate_offsets: &AU915_RX1,
    rx1_data_rate_offsets_dwell_limited: None,
    max_rx1_data_rate_offset: 5,
    rx2_frequency_hz: 923_300_000,
    rx2_data_rate: 8,
    data_rate_backoff: &AU915_BACKOFF,
    beacon: Beacon {
        data_rate: 8,
        frequency_hz: 923_300_000,
        ping_slot_frequency_hz: 923_300_000,
    },
    has_dwell_time_limit: true,
};

// CN470-510, RP002-1.0.5 section 3.9.

/// RP002-1.0.5 Table 50: CN470-510 data rate and TXPower.
#[cfg(feature = "cn470")]
static CN470_DATA_RATES: [Option<DataRate>; 8] = [
    Some(DataRate::lora(12, 125_000, 250)),
    Some(DataRate::lora(11, 125_000, 440)),
    Some(DataRate::lora(10, 125_000, 980)),
    Some(DataRate::lora(9, 125_000, 1_760)),
    Some(DataRate::lora(8, 125_000, 3_125)),
    Some(DataRate::lora(7, 125_000, 5_470)),
    Some(DataRate::lora(7, 500_000, 21_900)),
    Some(DataRate::fsk(50_000)),
];

/// RP002-1.0.5 Table 54: CN470-510 maximum payload size (repeater compatible).
///
/// DR0 carries nothing: at SF12 the one-second transmission limit this band
/// works under leaves no room for a frame.
#[cfg(feature = "cn470")]
static CN470_MAX_PAYLOAD_REPEATER: [Option<MaxPayload>; 8] = [
    None,
    Some(MaxPayload::new(31, 23)),
    Some(MaxPayload::new(94, 86)),
    Some(MaxPayload::new(192, 184)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 55: CN470-510 maximum payload size (not repeater compatible).
#[cfg(feature = "cn470")]
static CN470_MAX_PAYLOAD_DIRECT: [Option<MaxPayload>; 8] = [
    None,
    Some(MaxPayload::new(31, 23)),
    Some(MaxPayload::new(94, 86)),
    Some(MaxPayload::new(192, 184)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
];

/// RP002-1.0.5 Table 56: CN470-510 downlink RX1 data rate mapping.
#[cfg(feature = "cn470")]
static CN470_RX1: [&[u8]; 8] = [
    &[0, 0, 0, 0, 0, 0],
    &[1, 1, 1, 1, 1, 1],
    &[2, 1, 1, 1, 1, 1],
    &[3, 2, 1, 1, 1, 1],
    &[4, 3, 2, 1, 1, 1],
    &[5, 4, 3, 2, 1, 1],
    &[6, 5, 4, 3, 2, 1],
    &[7, 6, 5, 4, 3, 2],
];

/// RP002-1.0.5 Table 51: CN470-510 data rate back-off.
#[cfg(feature = "cn470")]
static CN470_BACKOFF: [Option<u8>; 8] = [
    None,
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(4),
    Some(5),
    Some(6),
];

/// The uplink blocks of the 20 MHz antenna, channel plan A, section 3.9.2.
#[cfg(feature = "cn470")]
static CN470_CHANNELS: [ChannelBlock; 2] = [
    ChannelBlock::new(470_300_000, 200_000, 32, 0, 5),
    ChannelBlock::new(483_900_000, 200_000, 32, 0, 5),
];

/// The eight common join channels every CN470 plan shares, Table 49.
#[cfg(feature = "cn470")]
static CN470_JOIN_CHANNELS: [ChannelBlock; 2] = [
    ChannelBlock::new(470_900_000, 1_600_000, 4, 0, 5),
    ChannelBlock::new(504_100_000, 1_600_000, 4, 0, 5),
];

/// The CN470-510 channel plan, RP002-1.0.5 section 3.9.
///
/// This carries the 20 MHz antenna, channel plan A variant. RP002 defines four
/// (20 MHz and 26 MHz antennas, each with a plan A and B) whose uplink blocks,
/// RX2 frequency, and beacon frequency differ, and the RX2 frequency also
/// depends on whether the device joined over the air or was personalized. The
/// value here is the personalized default for plan A. A deployment on one of the
/// other variants builds a [`ChannelPlan`] with its own frequencies; everything
/// else in this plan is common to all four.
///
/// Transmissions in this band are limited to one second on one channel at a
/// time, with listen-before-talk rather than a duty cycle.
#[cfg(feature = "cn470")]
pub static CN470: ChannelPlan = ChannelPlan {
    name: "CN470-510",
    uplink_data_rates: &CN470_DATA_RATES,
    downlink_data_rates: &CN470_DATA_RATES,
    max_payload_repeater: &CN470_MAX_PAYLOAD_REPEATER,
    max_payload_direct: &CN470_MAX_PAYLOAD_DIRECT,
    downlink_max_payload_repeater: &CN470_MAX_PAYLOAD_REPEATER,
    downlink_max_payload_direct: &CN470_MAX_PAYLOAD_DIRECT,
    max_payload_dwell_limited: None,
    join_channels: &CN470_JOIN_CHANNELS,
    default_channels: &CN470_CHANNELS,
    sub_bands: &[],
    default_max_eirp_dbm: 19,
    tx_power_step_db: 2,
    max_tx_power_index: 7,
    rx1_data_rate_offsets: &CN470_RX1,
    rx1_data_rate_offsets_dwell_limited: None,
    max_rx1_data_rate_offset: 5,
    rx2_frequency_hz: 486_900_000,
    rx2_data_rate: 1,
    data_rate_backoff: &CN470_BACKOFF,
    beacon: Beacon {
        data_rate: 2,
        frequency_hz: 486_900_000,
        ping_slot_frequency_hz: 486_900_000,
    },
    has_dwell_time_limit: false,
};

// AS923, RP002-1.0.5 section 3.10.

/// RP002-1.0.5 Table 67: AS923 data rate.
#[cfg(feature = "as923")]
static AS923_DATA_RATES: [Option<DataRate>; 14] = [
    Some(DataRate::lora(12, 125_000, 250)),
    Some(DataRate::lora(11, 125_000, 440)),
    Some(DataRate::lora(10, 125_000, 980)),
    Some(DataRate::lora(9, 125_000, 1_760)),
    Some(DataRate::lora(8, 125_000, 3_125)),
    Some(DataRate::lora(7, 125_000, 5_470)),
    Some(DataRate::lora(7, 250_000, 11_000)),
    Some(DataRate::fsk(50_000)),
    None,
    None,
    None,
    None,
    Some(DataRate::lora(6, 125_000, 9_375)),
    Some(DataRate::lora(5, 125_000, 15_625)),
];

/// RP002-1.0.5 Table 72: AS923 maximum payload size (repeater compatible).
#[cfg(feature = "as923")]
static AS923_MAX_PAYLOAD_REPEATER: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 73: AS923 maximum payload size (not repeater compatible).
#[cfg(feature = "as923")]
static AS923_MAX_PAYLOAD_DIRECT: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
];

/// RP002-1.0.5 Table 74: AS923 RX1 mapping with no downlink dwell limit.
#[cfg(feature = "as923")]
static AS923_RX1: [&[u8]; 14] = [
    &[0, 0, 0, 0, 0, 0, 1, 2],
    &[1, 0, 0, 0, 0, 0, 2, 3],
    &[2, 1, 0, 0, 0, 0, 3, 4],
    &[3, 2, 1, 0, 0, 0, 4, 5],
    &[4, 3, 2, 1, 0, 0, 5, 6],
    &[5, 4, 3, 2, 1, 0, 6, 7],
    &[6, 5, 4, 3, 2, 1, 7, 7],
    &[7, 6, 5, 4, 3, 2, 7, 7],
    &[0, 0, 0, 0, 0, 0, 1, 2],
    &[0, 0, 0, 0, 0, 0, 1, 2],
    &[0, 0, 0, 0, 0, 0, 1, 2],
    &[0, 0, 0, 0, 0, 0, 1, 2],
    &[12, 5, 4, 3, 2, 1, 13, 13],
    &[13, 12, 5, 4, 3, 2, 13, 13],
];

/// RP002-1.0.5 Table 75: AS923 RX1 mapping under a downlink dwell limit.
#[cfg(feature = "as923")]
static AS923_RX1_DWELL: [&[u8]; 14] = [
    &[2, 2, 2, 2, 2, 2, 2, 2],
    &[2, 2, 2, 2, 2, 2, 2, 3],
    &[2, 2, 2, 2, 2, 2, 3, 4],
    &[3, 2, 2, 2, 2, 2, 4, 5],
    &[4, 3, 2, 2, 2, 2, 5, 6],
    &[5, 4, 3, 2, 2, 2, 6, 7],
    &[6, 5, 4, 3, 2, 2, 7, 7],
    &[7, 6, 5, 4, 3, 2, 7, 7],
    &[2, 2, 2, 2, 2, 2, 2, 2],
    &[2, 2, 2, 2, 2, 2, 2, 2],
    &[2, 2, 2, 2, 2, 2, 2, 2],
    &[2, 2, 2, 2, 2, 2, 2, 2],
    &[12, 5, 4, 3, 2, 2, 13, 13],
    &[13, 12, 5, 4, 3, 2, 13, 13],
];

/// RP002-1.0.5 Table 69: AS923 data rate back-off.
#[cfg(feature = "as923")]
static AS923_BACKOFF: [Option<u8>; 14] = [
    None,
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(4),
    Some(5),
    Some(6),
    None,
    None,
    None,
    None,
    Some(5),
    Some(12),
];

/// RP002-1.0.5 Table 65: the two default channels, at the AS923-1 offset of zero.
#[cfg(feature = "as923")]
static AS923_CHANNELS: [ChannelBlock; 1] = [ChannelBlock::new(923_200_000, 200_000, 2, 0, 5)];

/// The AS923 band is duty-cycle limited to 1% on its default channels.
#[cfg(feature = "as923")]
static AS923_SUB_BANDS: [SubBand; 1] = [SubBand::new(923_000_000, 923_500_000, 10, 16)];

/// The AS923 channel plan, RP002-1.0.5 section 3.10.
///
/// AS923 is four plans rather than one: every frequency here is the AS923-1
/// value, and AS923-2, AS923-3, and AS923-4 shift each of them by a fixed
/// `AS923_FREQ_OFFSET_HZ`. Everything that is not a frequency is common to all
/// four, so a deployment on another group copies this plan and adds its offset
/// to the channel, RX2, and beacon frequencies.
///
/// Japan reaches this band through ARIB STD-T108, which imposes a listen-before-
/// talk obligation this plan does not model.
#[cfg(feature = "as923")]
pub static AS923: ChannelPlan = ChannelPlan {
    name: "AS923",
    uplink_data_rates: &AS923_DATA_RATES,
    downlink_data_rates: &AS923_DATA_RATES,
    max_payload_repeater: &AS923_MAX_PAYLOAD_REPEATER,
    max_payload_direct: &AS923_MAX_PAYLOAD_DIRECT,
    downlink_max_payload_repeater: &AS923_MAX_PAYLOAD_REPEATER,
    downlink_max_payload_direct: &AS923_MAX_PAYLOAD_DIRECT,
    max_payload_dwell_limited: None,
    join_channels: &AS923_CHANNELS,
    default_channels: &AS923_CHANNELS,
    sub_bands: &AS923_SUB_BANDS,
    default_max_eirp_dbm: 16,
    tx_power_step_db: 2,
    max_tx_power_index: 7,
    rx1_data_rate_offsets: &AS923_RX1,
    rx1_data_rate_offsets_dwell_limited: Some(&AS923_RX1_DWELL),
    max_rx1_data_rate_offset: 7,
    rx2_frequency_hz: 923_200_000,
    rx2_data_rate: 2,
    data_rate_backoff: &AS923_BACKOFF,
    beacon: Beacon {
        data_rate: 3,
        frequency_hz: 923_400_000,
        ping_slot_frequency_hz: 923_400_000,
    },
    has_dwell_time_limit: true,
};

// KR920-923, RP002-1.0.5 section 3.11.

/// RP002-1.0.5 Table 80: KR920-923 TX data rate.
#[cfg(feature = "kr920")]
static KR920_DATA_RATES: [Option<DataRate>; 14] = [
    Some(DataRate::lora(12, 125_000, 250)),
    Some(DataRate::lora(11, 125_000, 440)),
    Some(DataRate::lora(10, 125_000, 980)),
    Some(DataRate::lora(9, 125_000, 1_760)),
    Some(DataRate::lora(8, 125_000, 3_125)),
    Some(DataRate::lora(7, 125_000, 5_470)),
    None,
    None,
    None,
    None,
    None,
    None,
    Some(DataRate::lora(6, 125_000, 9_375)),
    Some(DataRate::lora(5, 125_000, 15_625)),
];

/// RP002-1.0.5 Table 85: KR920-923 maximum payload size (repeater compatible).
#[cfg(feature = "kr920")]
static KR920_MAX_PAYLOAD_REPEATER: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    None,
    None,
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 86: KR920-923 maximum payload size (not repeater compatible).
#[cfg(feature = "kr920")]
static KR920_MAX_PAYLOAD_DIRECT: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    None,
    None,
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
];

/// RP002-1.0.5 Table 87: KR920-923 downlink RX1 data rate mapping.
#[cfg(feature = "kr920")]
static KR920_RX1: [&[u8]; 14] = [
    &[0, 0, 0, 0, 0, 0],
    &[1, 0, 0, 0, 0, 0],
    &[2, 1, 0, 0, 0, 0],
    &[3, 2, 1, 0, 0, 0],
    &[4, 3, 2, 1, 0, 0],
    &[5, 4, 3, 2, 1, 0],
    &[0, 0, 0, 0, 0, 0],
    &[0, 0, 0, 0, 0, 0],
    &[0, 0, 0, 0, 0, 0],
    &[0, 0, 0, 0, 0, 0],
    &[0, 0, 0, 0, 0, 0],
    &[0, 0, 0, 0, 0, 0],
    &[12, 5, 4, 3, 2, 1],
    &[13, 12, 5, 4, 3, 2],
];

/// RP002-1.0.5 Table 82: KR920-923 data rate back-off.
#[cfg(feature = "kr920")]
static KR920_BACKOFF: [Option<u8>; 14] = [
    None,
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(4),
    None,
    None,
    None,
    None,
    None,
    None,
    Some(5),
    Some(12),
];

/// RP002-1.0.5 Tables 78 and 79: the three default and join channels.
#[cfg(feature = "kr920")]
static KR920_CHANNELS: [ChannelBlock; 1] = [ChannelBlock::new(922_100_000, 200_000, 3, 0, 5)];

/// RP002-1.0.5 Table 77: the band's power ceiling steps at 922.1 MHz.
///
/// Korea publishes no duty cycle for this band; the two sub-bands differ only in
/// how much power they allow, so the limit reported here is the ceiling.
#[cfg(feature = "kr920")]
static KR920_SUB_BANDS: [SubBand; 2] = [
    SubBand::new(920_900_000, 921_900_000, 1000, 10),
    SubBand::new(922_100_000, 923_300_000, 1000, 14),
];

/// The KR920-923 channel plan, RP002-1.0.5 section 3.11.
#[cfg(feature = "kr920")]
pub static KR920: ChannelPlan = ChannelPlan {
    name: "KR920-923",
    uplink_data_rates: &KR920_DATA_RATES,
    downlink_data_rates: &KR920_DATA_RATES,
    max_payload_repeater: &KR920_MAX_PAYLOAD_REPEATER,
    max_payload_direct: &KR920_MAX_PAYLOAD_DIRECT,
    downlink_max_payload_repeater: &KR920_MAX_PAYLOAD_REPEATER,
    downlink_max_payload_direct: &KR920_MAX_PAYLOAD_DIRECT,
    max_payload_dwell_limited: None,
    join_channels: &KR920_CHANNELS,
    default_channels: &KR920_CHANNELS,
    sub_bands: &KR920_SUB_BANDS,
    default_max_eirp_dbm: 14,
    tx_power_step_db: 2,
    max_tx_power_index: 7,
    rx1_data_rate_offsets: &KR920_RX1,
    rx1_data_rate_offsets_dwell_limited: None,
    max_rx1_data_rate_offset: 5,
    rx2_frequency_hz: 921_900_000,
    rx2_data_rate: 0,
    data_rate_backoff: &KR920_BACKOFF,
    beacon: Beacon {
        data_rate: 3,
        frequency_hz: 923_100_000,
        ping_slot_frequency_hz: 923_100_000,
    },
    has_dwell_time_limit: false,
};

// IN865, RP002-1.0.5 section 3.12.

/// RP002-1.0.5 Table 91: IN865 TX data rate.
#[cfg(feature = "in865")]
static IN865_DATA_RATES: [Option<DataRate>; 14] = [
    Some(DataRate::lora(12, 125_000, 250)),
    Some(DataRate::lora(11, 125_000, 440)),
    Some(DataRate::lora(10, 125_000, 980)),
    Some(DataRate::lora(9, 125_000, 1_760)),
    Some(DataRate::lora(8, 125_000, 3_125)),
    Some(DataRate::lora(7, 125_000, 5_470)),
    None,
    Some(DataRate::fsk(50_000)),
    None,
    None,
    None,
    None,
    Some(DataRate::lora(6, 125_000, 9_375)),
    Some(DataRate::lora(5, 125_000, 15_625)),
];

/// RP002-1.0.5 Table 96: IN865 maximum payload size (repeater compatible).
#[cfg(feature = "in865")]
static IN865_MAX_PAYLOAD_REPEATER: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    None,
    Some(MaxPayload::new(230, 222)),
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 97: IN865 maximum payload size (not repeater compatible).
#[cfg(feature = "in865")]
static IN865_MAX_PAYLOAD_DIRECT: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    None,
    Some(MaxPayload::new(250, 242)),
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
];

/// RP002-1.0.5 Table 98: IN865 downlink RX1 data rate mapping.
///
/// India is one of the regions that allows all eight RX1 data-rate offsets.
#[cfg(feature = "in865")]
static IN865_RX1: [&[u8]; 14] = [
    &[0, 0, 0, 0, 0, 0, 1, 2],
    &[1, 0, 0, 0, 0, 0, 2, 3],
    &[2, 1, 0, 0, 0, 0, 3, 4],
    &[3, 2, 1, 0, 0, 0, 4, 5],
    &[4, 3, 2, 1, 0, 0, 5, 5],
    &[5, 4, 3, 2, 1, 0, 5, 7],
    &[0, 0, 0, 0, 0, 0, 1, 2],
    &[7, 5, 5, 4, 3, 2, 7, 7],
    &[0, 0, 0, 0, 0, 0, 1, 2],
    &[0, 0, 0, 0, 0, 0, 1, 2],
    &[0, 0, 0, 0, 0, 0, 1, 2],
    &[0, 0, 0, 0, 0, 0, 1, 2],
    &[12, 5, 4, 3, 2, 1, 13, 13],
    &[13, 12, 5, 4, 3, 2, 13, 13],
];

/// RP002-1.0.5 Table 93: IN865 data rate back-off.
#[cfg(feature = "in865")]
static IN865_BACKOFF: [Option<u8>; 14] = [
    None,
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(4),
    None,
    Some(5),
    None,
    None,
    None,
    None,
    Some(5),
    Some(12),
];

/// RP002-1.0.5 Tables 89 and 90: the three default and join channels.
///
/// These are not evenly spaced, so each is its own single-channel block.
#[cfg(feature = "in865")]
static IN865_CHANNELS: [ChannelBlock; 3] = [
    ChannelBlock::new(865_062_500, 0, 1, 0, 5),
    ChannelBlock::new(865_402_500, 0, 1, 0, 5),
    ChannelBlock::new(865_985_000, 0, 1, 0, 5),
];

/// The IN865 channel plan, RP002-1.0.5 section 3.12.
///
/// The published ceiling is 29.2 dBm EIRP, reported here as the whole decibel
/// below it so a caller reading this as a limit is never over the real one.
#[cfg(feature = "in865")]
pub static IN865: ChannelPlan = ChannelPlan {
    name: "IN865",
    uplink_data_rates: &IN865_DATA_RATES,
    downlink_data_rates: &IN865_DATA_RATES,
    max_payload_repeater: &IN865_MAX_PAYLOAD_REPEATER,
    max_payload_direct: &IN865_MAX_PAYLOAD_DIRECT,
    downlink_max_payload_repeater: &IN865_MAX_PAYLOAD_REPEATER,
    downlink_max_payload_direct: &IN865_MAX_PAYLOAD_DIRECT,
    max_payload_dwell_limited: None,
    join_channels: &IN865_CHANNELS,
    default_channels: &IN865_CHANNELS,
    sub_bands: &[],
    default_max_eirp_dbm: 29,
    tx_power_step_db: 2,
    max_tx_power_index: 10,
    rx1_data_rate_offsets: &IN865_RX1,
    rx1_data_rate_offsets_dwell_limited: None,
    max_rx1_data_rate_offset: 7,
    rx2_frequency_hz: 866_550_000,
    rx2_data_rate: 2,
    data_rate_backoff: &IN865_BACKOFF,
    beacon: Beacon {
        data_rate: 4,
        frequency_hz: 866_550_000,
        ping_slot_frequency_hz: 866_550_000,
    },
    has_dwell_time_limit: false,
};

// RU864-870, RP002-1.0.5 section 3.13.

/// RP002-1.0.5 Table 102: RU864-870 TX data rate.
#[cfg(feature = "ru864")]
static RU864_DATA_RATES: [Option<DataRate>; 14] = [
    Some(DataRate::lora(12, 125_000, 250)),
    Some(DataRate::lora(11, 125_000, 440)),
    Some(DataRate::lora(10, 125_000, 980)),
    Some(DataRate::lora(9, 125_000, 1_760)),
    Some(DataRate::lora(8, 125_000, 3_125)),
    Some(DataRate::lora(7, 125_000, 5_470)),
    Some(DataRate::lora(7, 250_000, 11_000)),
    Some(DataRate::fsk(50_000)),
    None,
    None,
    None,
    None,
    Some(DataRate::lora(6, 125_000, 9_375)),
    Some(DataRate::lora(5, 125_000, 15_625)),
];

/// RP002-1.0.5 Table 107: RU864-870 maximum payload size (repeater compatible).
#[cfg(feature = "ru864")]
static RU864_MAX_PAYLOAD_REPEATER: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(230, 222)),
    Some(MaxPayload::new(230, 222)),
];

/// RP002-1.0.5 Table 108: RU864-870 maximum payload size (not repeater compatible).
#[cfg(feature = "ru864")]
static RU864_MAX_PAYLOAD_DIRECT: [Option<MaxPayload>; 14] = [
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(59, 51)),
    Some(MaxPayload::new(123, 115)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
    None,
    None,
    None,
    None,
    Some(MaxPayload::new(250, 242)),
    Some(MaxPayload::new(250, 242)),
];

/// RP002-1.0.5 Table 109: RU864-870 downlink RX1 data rate mapping.
#[cfg(feature = "ru864")]
static RU864_RX1: [&[u8]; 14] = [
    &[0, 0, 0, 0, 0, 0],
    &[1, 0, 0, 0, 0, 0],
    &[2, 1, 0, 0, 0, 0],
    &[3, 2, 1, 0, 0, 0],
    &[4, 3, 2, 1, 0, 0],
    &[5, 4, 3, 2, 1, 0],
    &[6, 5, 4, 3, 2, 1],
    &[7, 6, 5, 4, 3, 2],
    &[0, 0, 0, 0, 0, 0],
    &[0, 0, 0, 0, 0, 0],
    &[0, 0, 0, 0, 0, 0],
    &[0, 0, 0, 0, 0, 0],
    &[12, 5, 4, 3, 2, 1],
    &[13, 12, 5, 4, 3, 2],
];

/// RP002-1.0.5 Table 104: RU864-870 data rate back-off.
#[cfg(feature = "ru864")]
static RU864_BACKOFF: [Option<u8>; 14] = [
    None,
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(4),
    Some(5),
    Some(6),
    None,
    None,
    None,
    None,
    Some(5),
    Some(12),
];

/// RP002-1.0.5 Tables 100 and 101: the two default and join channels.
#[cfg(feature = "ru864")]
static RU864_CHANNELS: [ChannelBlock; 1] = [ChannelBlock::new(868_900_000, 200_000, 2, 0, 5)];

/// The Russian band carries a 1% duty cycle on its default channels.
#[cfg(feature = "ru864")]
static RU864_SUB_BANDS: [SubBand; 1] = [SubBand::new(868_700_000, 869_200_000, 10, 16)];

/// The RU864-870 channel plan, RP002-1.0.5 section 3.13.
#[cfg(feature = "ru864")]
pub static RU864: ChannelPlan = ChannelPlan {
    name: "RU864-870",
    uplink_data_rates: &RU864_DATA_RATES,
    downlink_data_rates: &RU864_DATA_RATES,
    max_payload_repeater: &RU864_MAX_PAYLOAD_REPEATER,
    max_payload_direct: &RU864_MAX_PAYLOAD_DIRECT,
    downlink_max_payload_repeater: &RU864_MAX_PAYLOAD_REPEATER,
    downlink_max_payload_direct: &RU864_MAX_PAYLOAD_DIRECT,
    max_payload_dwell_limited: None,
    join_channels: &RU864_CHANNELS,
    default_channels: &RU864_CHANNELS,
    sub_bands: &RU864_SUB_BANDS,
    default_max_eirp_dbm: 16,
    tx_power_step_db: 2,
    max_tx_power_index: 7,
    rx1_data_rate_offsets: &RU864_RX1,
    rx1_data_rate_offsets_dwell_limited: None,
    max_rx1_data_rate_offset: 5,
    rx2_frequency_hz: 869_100_000,
    rx2_data_rate: 0,
    data_rate_backoff: &RU864_BACKOFF,
    beacon: Beacon {
        data_rate: 3,
        frequency_hz: 869_100_000,
        ping_slot_frequency_hz: 868_900_000,
    },
    has_dwell_time_limit: false,
};
