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
