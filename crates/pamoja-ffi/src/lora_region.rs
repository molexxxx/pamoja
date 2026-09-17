//! The C ABI for LoRaWAN regional channel plans.
//!
//! A channel plan is the set of facts a regulator and the LoRa Alliance publish
//! about one band: which data rates exist, how much they carry, what a device may
//! radiate, where it listens for a downlink. This module hands those facts across
//! the boundary and costs nothing out of them. It never refuses a transmission,
//! because a deployment may hold licensed spectrum or be operating under emergency
//! provisions, and only the operator knows which.
//!
//! A plan crosses as an opaque handle rather than by value, because it is a set of
//! tables rather than a few scalars. A handle comes either from a published region
//! or from [`pamoja_lora_plan_builder_build`], and the query functions cannot tell
//! the difference: a private plan on licensed spectrum answers every question a
//! published one does.
//!
//! Region codes are assigned here and are stable. They are deliberately not the
//! discriminants of the Rust enum, whose variants are individually feature-gated,
//! so a build carrying one region would otherwise number it differently from a
//! build carrying all of them.

#[cfg(feature = "cn470")]
use pamoja_lora::region::Cn470Plan;
use pamoja_lora::region::{
    Beacon, ChannelBlock, ChannelPlan, ChannelPlanBuilder, DataRate, FixedChannelList,
    JoinSequence, MaskControl, MaxPayload, Modulation, OwnedChannelPlan, PayloadTable, PlanKind,
    PowerReference, SubBand,
};
// A build that carries no region still offers the builder, and then names no
// published plan at all.
#[cfg(any(
    feature = "eu868",
    feature = "us915",
    feature = "eu433",
    feature = "au915",
    feature = "cn470",
    feature = "as923",
    feature = "kr920",
    feature = "in865",
    feature = "ru864"
))]
use pamoja_lora::region::Region;

use crate::lora::PamojaLoraLink;
use crate::{set_last_error, PamojaStatus, PamojaString};

/// The EU863-870 band.
pub const PAMOJA_LORA_REGION_EU868: u32 = 1;
/// The US902-928 band.
pub const PAMOJA_LORA_REGION_US915: u32 = 2;
/// The EU433 band.
pub const PAMOJA_LORA_REGION_EU433: u32 = 3;
/// The AU915-928 band.
pub const PAMOJA_LORA_REGION_AU915: u32 = 4;
/// The CN470-510 band.
pub const PAMOJA_LORA_REGION_CN470: u32 = 5;
/// The AS923 band.
pub const PAMOJA_LORA_REGION_AS923: u32 = 6;
/// The KR920-923 band.
pub const PAMOJA_LORA_REGION_KR920: u32 = 7;
/// The IN865-867 band.
pub const PAMOJA_LORA_REGION_IN865: u32 = 8;
/// The RU864-870 band.
pub const PAMOJA_LORA_REGION_RU864: u32 = 9;

/// A data rate carried by LoRa modulation.
pub const PAMOJA_LORA_MODULATION_LORA: u8 = 0;
/// A data rate carried by FSK modulation.
pub const PAMOJA_LORA_MODULATION_FSK: u8 = 1;
/// A data rate carried by long-range frequency-hopping spread spectrum.
pub const PAMOJA_LORA_MODULATION_LR_FHSS: u8 = 2;
/// A data-rate number the region reserves, which carries nothing.
pub const PAMOJA_LORA_MODULATION_RESERVED: u8 = 3;

/// The uplink payload limits for a device that may sit behind a repeater.
pub const PAMOJA_LORA_PAYLOAD_TABLE_UPLINK_REPEATER: u32 = 0;
/// The uplink payload limits for a device that will not sit behind a repeater.
pub const PAMOJA_LORA_PAYLOAD_TABLE_UPLINK_DIRECT: u32 = 1;
/// The downlink payload limits for a device that may sit behind a repeater.
pub const PAMOJA_LORA_PAYLOAD_TABLE_DOWNLINK_REPEATER: u32 = 2;
/// The downlink payload limits for a device that will not sit behind a repeater.
pub const PAMOJA_LORA_PAYLOAD_TABLE_DOWNLINK_DIRECT: u32 = 3;
/// The payload limits that apply under a dwell-time limit.
pub const PAMOJA_LORA_PAYLOAD_TABLE_DWELL_LIMITED: u32 = 4;

/// The channels a device must use to send a join request.
pub const PAMOJA_LORA_CHANNELS_JOIN: u32 = 0;
/// The channels a device starts with before a network adds any.
pub const PAMOJA_LORA_CHANNELS_DEFAULT: u32 = 1;
/// The numbered downlink channels a fixed plan answers the first receive window on.
pub const PAMOJA_LORA_CHANNELS_DOWNLINK: u32 = 2;

/// A plan whose network creates channels and moves them.
pub const PAMOJA_LORA_PLAN_KIND_DYNAMIC: u8 = 0;
/// A plan whose channels are numbered in advance and only enabled or disabled.
pub const PAMOJA_LORA_PLAN_KIND_FIXED: u8 = 1;

/// A plan that reads a type 1 channel list against no numbering.
pub const PAMOJA_LORA_CHANNEL_LIST_NONE: u8 = 0;
/// The 800 MHz numbering of RP002-1.0.5 section 3.3.1.1.
pub const PAMOJA_LORA_CHANNEL_LIST_MHZ800: u8 = 1;
/// The 900 MHz numbering of RP002-1.0.5 section 3.3.1.2.
pub const PAMOJA_LORA_CHANNEL_LIST_MHZ900: u8 = 2;

/// A join channel at random, stepping the data rate down across attempts.
pub const PAMOJA_LORA_JOIN_RANDOM: u8 = 0;
/// The octet passes of RP002-1.0.5 section 3.5.2, eight 125 kHz channels from successive
/// groups and then a 500 kHz one.
pub const PAMOJA_LORA_JOIN_OCTET_PASSES: u8 = 1;

/// Power indexes that count down from a radiated ceiling.
pub const PAMOJA_LORA_POWER_EIRP: u8 = 0;
/// Power indexes that count down from a conducted ceiling.
pub const PAMOJA_LORA_POWER_CONDUCTED: u8 = 1;

/// A channel mask control that sets one group of sixteen channels.
pub const PAMOJA_LORA_MASK_GROUP: u8 = 0;
/// A channel mask control whose ten low bits switch banks of eight.
pub const PAMOJA_LORA_MASK_BANKS: u8 = 1;
/// A channel mask control whose eight low bits switch banks of eight with their 500 kHz
/// channel.
pub const PAMOJA_LORA_MASK_PAIRED_BANKS: u8 = 2;
/// A channel mask control that turns every channel on or off, then sets a group.
pub const PAMOJA_LORA_MASK_ALL: u8 = 3;
/// A channel mask control the region reserves.
pub const PAMOJA_LORA_MASK_RESERVED: u8 = 4;

/// No CN470-510 plan, for a join plan that points at a plan built elsewhere.
pub const PAMOJA_LORA_CN470_NONE: u32 = 0;
/// The CN470-510 plan for a 20 MHz antenna, type A.
pub const PAMOJA_LORA_CN470_ANTENNA_20MHZ_A: u32 = 1;
/// The CN470-510 plan for a 20 MHz antenna, type B.
pub const PAMOJA_LORA_CN470_ANTENNA_20MHZ_B: u32 = 2;
/// The CN470-510 plan for a 26 MHz antenna, type A.
pub const PAMOJA_LORA_CN470_ANTENNA_26MHZ_A: u32 = 3;
/// The CN470-510 plan for a 26 MHz antenna, type B.
pub const PAMOJA_LORA_CN470_ANTENNA_26MHZ_B: u32 = 4;
/// The 96-channel CN470-510 plan of the LoRaWAN 1.0.3 Regional Parameters revision A.
pub const PAMOJA_LORA_CN470_CHANNELS_96: u32 = 5;

/// The uplink direction, for a table that differs between the two.
pub const PAMOJA_LORA_DIRECTION_UPLINK: u32 = 0;
/// The downlink direction, for a table that differs between the two.
pub const PAMOJA_LORA_DIRECTION_DOWNLINK: u32 = 1;

/// One data rate: how a number on the wire maps onto radio settings.
///
/// `kind` selects which fields carry meaning. A LoRa rate uses
/// `spreading_factor` and `bandwidth_hz`; an LR-FHSS rate uses the coding-rate
/// pair and `bandwidth_hz`; an FSK rate uses `bitrate_bps` alone. A reserved
/// number leaves every field zero.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraDataRate {
    /// The payload bitrate in bits per second.
    pub bitrate_bps: u32,
    /// The channel bandwidth in hertz, or zero for FSK.
    pub bandwidth_hz: u32,
    /// One of the `PAMOJA_LORA_MODULATION_*` constants.
    pub kind: u8,
    /// The spreading factor, for a LoRa rate.
    pub spreading_factor: u8,
    /// The coding-rate numerator, for an LR-FHSS rate.
    pub coding_rate_numerator: u8,
    /// The coding-rate denominator, for an LR-FHSS rate.
    pub coding_rate_denominator: u8,
}

/// What one data rate may carry in a single frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraMaxPayload {
    /// The largest MAC payload, frame options included, in bytes.
    pub mac_payload: u16,
    /// The largest application payload, in bytes.
    pub application: u16,
}

/// A run of evenly spaced channels.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraChannelBlock {
    /// The first channel's center frequency in hertz.
    pub start_hz: u32,
    /// The spacing between channels in hertz.
    pub step_hz: u32,
    /// How many channels the block holds.
    pub count: u16,
    /// The slowest data rate the block allows.
    pub min_data_rate: u8,
    /// The fastest data rate the block allows.
    pub max_data_rate: u8,
}

/// A slice of a band with its own transmit limits.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraSubBand {
    /// The first frequency in the sub-band, in hertz.
    pub start_hz: u32,
    /// The last frequency in the sub-band, in hertz.
    pub end_hz: u32,
    /// The share of time a transmitter may hold the channel, in parts per
    /// thousand, so `10` is one percent and `1000` is unrestricted.
    pub duty_cycle_permille: u32,
    /// The power ceiling in dBm EIRP.
    pub max_eirp_dbm: i8,
}

/// The Class B beacon settings of a plan.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraBeacon {
    /// The frequency the beacon is broadcast on, in hertz.
    pub frequency_hz: u32,
    /// The default ping-slot frequency, in hertz.
    pub ping_slot_frequency_hz: u32,
    /// The data rate the beacon is broadcast at.
    pub data_rate: u8,
}

/// The scalar facts of a plan, gathered so a caller reads them in one call.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraPlanInfo {
    /// The fixed frequency the second receive window listens on, in hertz.
    pub rx2_frequency_hz: u32,
    /// How many uplink data-rate numbers the plan defines, reserved included.
    pub uplink_data_rate_count: u16,
    /// How many downlink data-rate numbers the plan defines.
    pub downlink_data_rate_count: u16,
    /// How many channels the plan starts a device with.
    pub default_channel_count: u16,
    /// How many join channels the plan defines.
    pub join_channel_block_count: u16,
    /// How many default channel blocks the plan defines.
    pub default_channel_block_count: u16,
    /// How many sub-bands the plan defines.
    pub sub_band_count: u16,
    /// The Class B beacon settings.
    pub beacon: PamojaLoraBeacon,
    /// The data rate the second receive window listens at.
    pub rx2_data_rate: u8,
    /// The power ceiling assumed when no sub-band says otherwise, in dBm.
    pub default_max_eirp_dbm: i8,
    /// The step between transmit-power settings, in dB.
    pub tx_power_step_db: u8,
    /// The highest transmit-power index the plan defines.
    pub max_tx_power_index: u8,
    /// The highest RX1 data-rate offset the plan allows.
    pub max_rx1_data_rate_offset: u8,
    /// `1` if the plan limits how long one transmission may hold a channel.
    pub has_dwell_time_limit: u8,
    /// `1` if the plan publishes a payload table for a dwell-limited device.
    pub has_dwell_limited_payloads: u8,
    /// `1` if the plan publishes a second RX1 mapping for a dwell-limited
    /// downlink.
    pub has_dwell_limited_rx1: u8,
}

/// How a plan's channels are defined and used, read in one call.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraPlanRules {
    /// [`PAMOJA_LORA_PLAN_KIND_DYNAMIC`] or [`PAMOJA_LORA_PLAN_KIND_FIXED`].
    pub kind: u8,
    /// For a dynamic plan, one of the `PAMOJA_LORA_CHANNEL_LIST_*` constants.
    pub channel_list: u8,
    /// `1` if devices on the plan answer `TXParamSetupReq`.
    pub tx_param_setup: u8,
    /// One of the `PAMOJA_LORA_JOIN_*` constants.
    pub join_sequence: u8,
    /// One of the `PAMOJA_LORA_POWER_*` constants.
    pub power_reference: u8,
    /// For a conducted ceiling, the antenna gain it already allows for, in dB.
    pub gain_allowance_db: u8,
    /// How many downlink channel blocks the plan defines.
    pub downlink_channel_block_count: u16,
    /// How many runs of join channels select a plan, which only the published CN470-510
    /// plans carry.
    pub join_plan_count: u16,
}

/// What one `ChMaskCntl` value of a `LinkADRReq` does.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraMaskControl {
    /// One of the `PAMOJA_LORA_MASK_*` constants.
    pub kind: u8,
    /// For [`PAMOJA_LORA_MASK_GROUP`], the group the mask sets.
    pub group: u8,
    /// For [`PAMOJA_LORA_MASK_ALL`], `1` to turn every channel on and `0` to turn it off.
    pub on: u8,
    /// For [`PAMOJA_LORA_MASK_ALL`], `1` if the mask then sets `then_group`.
    pub has_then_group: u8,
    /// The group the mask then sets.
    pub then_group: u8,
}

/// A run of join channels that puts a device on a plan.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraJoinPlan {
    /// The join channels and the data rates a request may use on them.
    pub channels: PamojaLoraChannelBlock,
    /// Where the accept answering the first channel arrives, in hertz.
    pub accept_start_hz: u32,
    /// How far the accept frequency moves for each next channel, in hertz.
    pub accept_step_hz: u32,
    /// The second receive window's frequency after joining on the first channel, in hertz.
    pub rx2_start_hz: u32,
    /// How far that frequency moves for each next channel, in hertz.
    pub rx2_step_hz: u32,
    /// The plan a join on these channels selects, one of the `PAMOJA_LORA_CN470_*`
    /// constants.
    pub cn470_plan: u32,
}

/// A regional channel plan, published or private.
///
/// The handle always owns its tables, so a published region and one assembled
/// here are the same type and answer the same queries. A published plan also keeps
/// the plans a join selects between, which point at other published plans and so
/// cannot be owned.
///
/// A handle the caller must release with [`pamoja_lora_plan_free`].
pub struct PamojaLoraPlan {
    plan: OwnedChannelPlan,
    published: Option<&'static ChannelPlan<'static>>,
}

impl PamojaLoraPlan {
    /// Moves a plan onto the heap and hands the caller its handle.
    ///
    /// # Arguments
    ///
    /// * `plan` - the plan to wrap.
    ///
    /// # Returns
    ///
    /// A handle the caller must release with [`pamoja_lora_plan_free`].
    fn into_handle(plan: OwnedChannelPlan) -> *mut Self {
        Box::into_raw(Box::new(Self {
            plan,
            published: None,
        }))
    }

    /// Wraps a published plan.
    ///
    /// # Arguments
    ///
    /// * `plan` - the published plan.
    ///
    /// # Returns
    ///
    /// A handle the caller must release with [`pamoja_lora_plan_free`].
    #[cfg_attr(
        not(any(
            feature = "eu868",
            feature = "us915",
            feature = "eu433",
            feature = "au915",
            feature = "cn470",
            feature = "as923",
            feature = "kr920",
            feature = "in865",
            feature = "ru864"
        )),
        allow(dead_code)
    )]
    fn published_handle(plan: &'static ChannelPlan<'static>) -> *mut Self {
        Box::into_raw(Box::new(Self {
            plan: OwnedChannelPlan::from_plan(plan),
            published: Some(plan),
        }))
    }

    /// Runs a query against the plan.
    ///
    /// # Arguments
    ///
    /// * `f` - the query to run.
    ///
    /// # Returns
    ///
    /// Whatever the query returned.
    pub(crate) fn with<R>(&self, f: impl FnOnce(&ChannelPlan<'_>) -> R) -> R {
        self.plan.with_plan(f)
    }
}

/// Converts a data rate into the shape that crosses the boundary.
///
/// # Arguments
///
/// * `rate` - the data rate to convert, or `None` for a reserved number.
///
/// # Returns
///
/// The equivalent C struct.
fn data_rate_out(rate: Option<DataRate>) -> PamojaLoraDataRate {
    let Some(rate) = rate else {
        return PamojaLoraDataRate {
            bitrate_bps: 0,
            bandwidth_hz: 0,
            kind: PAMOJA_LORA_MODULATION_RESERVED,
            spreading_factor: 0,
            coding_rate_numerator: 0,
            coding_rate_denominator: 0,
        };
    };
    let mut out = PamojaLoraDataRate {
        bitrate_bps: rate.bitrate_bps,
        bandwidth_hz: 0,
        kind: PAMOJA_LORA_MODULATION_FSK,
        spreading_factor: 0,
        coding_rate_numerator: 0,
        coding_rate_denominator: 0,
    };
    match rate.modulation {
        Modulation::LoRa {
            spreading_factor,
            bandwidth_hz,
        } => {
            out.kind = PAMOJA_LORA_MODULATION_LORA;
            out.spreading_factor = spreading_factor;
            out.bandwidth_hz = bandwidth_hz;
        }
        Modulation::Fsk { .. } => {}
        Modulation::LrFhss {
            coding_rate_numerator,
            coding_rate_denominator,
            bandwidth_hz,
        } => {
            out.kind = PAMOJA_LORA_MODULATION_LR_FHSS;
            out.coding_rate_numerator = coding_rate_numerator;
            out.coding_rate_denominator = coding_rate_denominator;
            out.bandwidth_hz = bandwidth_hz;
        }
    }
    out
}

/// Converts a data rate that crossed the boundary into the Rust type.
///
/// # Arguments
///
/// * `rate` - the data rate as the caller supplied it.
///
/// # Returns
///
/// `Ok(Some(rate))`, `Ok(None)` for a reserved number, or a status if the kind is
/// not one this ABI defines.
fn data_rate_in(rate: &PamojaLoraDataRate) -> Result<Option<DataRate>, PamojaStatus> {
    match rate.kind {
        PAMOJA_LORA_MODULATION_LORA => Ok(Some(DataRate::lora(
            rate.spreading_factor,
            rate.bandwidth_hz,
            rate.bitrate_bps,
        ))),
        PAMOJA_LORA_MODULATION_FSK => Ok(Some(DataRate::fsk(rate.bitrate_bps))),
        PAMOJA_LORA_MODULATION_LR_FHSS => Ok(Some(DataRate::lr_fhss(
            rate.coding_rate_numerator,
            rate.coding_rate_denominator,
            rate.bandwidth_hz,
            rate.bitrate_bps,
        ))),
        PAMOJA_LORA_MODULATION_RESERVED => Ok(None),
        other => {
            set_last_error(format!("{other} is not a modulation this build defines"));
            Err(PamojaStatus::InvalidArgument)
        }
    }
}

/// Reports whether a code names a region at all, whatever this build carries.
///
/// The codes are contiguous, so this stays a range check as regions are added.
///
/// # Arguments
///
/// * `region` - the code to check.
///
/// # Returns
///
/// `true` if the code names one of the published regions.
fn is_region_code(region: u32) -> bool {
    (PAMOJA_LORA_REGION_EU868..=PAMOJA_LORA_REGION_RU864).contains(&region)
}

/// Resolves a region code to its published plan.
///
/// An unknown code and a region left out of this build are told apart: the first
/// is an invalid argument, the second is unsupported. A host that offers a choice
/// of regions needs the difference, because one is a bug and the other is a build
/// that was trimmed to fit a device.
///
/// # Arguments
///
/// * `region` - one of the `PAMOJA_LORA_REGION_*` constants.
///
/// # Returns
///
/// The published plan, or a status explaining why there is none.
fn published(region: u32) -> Result<&'static ChannelPlan<'static>, PamojaStatus> {
    let plan: Option<&'static ChannelPlan<'static>> = match region {
        #[cfg(feature = "eu868")]
        PAMOJA_LORA_REGION_EU868 => Some(Region::Eu868.plan()),
        #[cfg(feature = "us915")]
        PAMOJA_LORA_REGION_US915 => Some(Region::Us915.plan()),
        #[cfg(feature = "eu433")]
        PAMOJA_LORA_REGION_EU433 => Some(Region::Eu433.plan()),
        #[cfg(feature = "au915")]
        PAMOJA_LORA_REGION_AU915 => Some(Region::Au915.plan()),
        #[cfg(feature = "cn470")]
        PAMOJA_LORA_REGION_CN470 => Some(Region::Cn470.plan()),
        #[cfg(feature = "as923")]
        PAMOJA_LORA_REGION_AS923 => Some(Region::As923.plan()),
        #[cfg(feature = "kr920")]
        PAMOJA_LORA_REGION_KR920 => Some(Region::Kr920.plan()),
        #[cfg(feature = "in865")]
        PAMOJA_LORA_REGION_IN865 => Some(Region::In865.plan()),
        #[cfg(feature = "ru864")]
        PAMOJA_LORA_REGION_RU864 => Some(Region::Ru864.plan()),
        _ => None,
    };
    match plan {
        Some(plan) => Ok(plan),
        None if is_region_code(region) => {
            set_last_error(format!(
                "region {region} is not compiled into this build of pamoja-lora"
            ));
            Err(PamojaStatus::Unsupported)
        }
        None => {
            set_last_error(format!("{region} is not a region code"));
            Err(PamojaStatus::InvalidArgument)
        }
    }
}

/// Returns the published channel plan for a region.
///
/// # Arguments
///
/// * `region` - one of the `PAMOJA_LORA_REGION_*` constants.
/// * `out_plan` - set to the plan handle on success, and to null otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_plan` is null or `region` is
/// not a region code, and [`PamojaStatus::Unsupported`] if the region is real but
/// was not compiled into this build.
///
/// # Safety
///
/// `out_plan` must point at writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_for_region(
    region: u32,
    out_plan: *mut *mut PamojaLoraPlan,
) -> PamojaStatus {
    if out_plan.is_null() {
        set_last_error("out_plan must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_plan;
    *slot = std::ptr::null_mut();

    match published(region) {
        Ok(plan) => {
            *slot = PamojaLoraPlan::published_handle(plan);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Returns one of the five CN470-510 channel plans.
///
/// # Arguments
///
/// * `which` - one of the `PAMOJA_LORA_CN470_*` constants other than
///   [`PAMOJA_LORA_CN470_NONE`].
/// * `out_plan` - set to the plan handle on success, and to null otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_plan` is null or `which` names no
/// plan, and [`PamojaStatus::Unsupported`] if CN470-510 was not compiled into this build.
///
/// # Safety
///
/// `out_plan` must point at writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_for_cn470(
    which: u32,
    out_plan: *mut *mut PamojaLoraPlan,
) -> PamojaStatus {
    if out_plan.is_null() {
        set_last_error("out_plan must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_plan;
    *slot = std::ptr::null_mut();
    if !(PAMOJA_LORA_CN470_ANTENNA_20MHZ_A..=PAMOJA_LORA_CN470_CHANNELS_96).contains(&which) {
        set_last_error(format!("{which} is not a CN470 plan"));
        return PamojaStatus::InvalidArgument;
    }

    #[cfg(feature = "cn470")]
    {
        let plan = Cn470Plan::all()[(which - 1) as usize].plan();
        *slot = PamojaLoraPlan::published_handle(plan);
        PamojaStatus::Ok
    }
    #[cfg(not(feature = "cn470"))]
    {
        set_last_error("CN470-510 is not compiled into this build of pamoja-lora".to_owned());
        PamojaStatus::Unsupported
    }
}

/// The code a join plan's target crosses as.
fn cn470_code(target: &ChannelPlan<'_>) -> u32 {
    #[cfg(feature = "cn470")]
    {
        if let Some(position) = Cn470Plan::all()
            .iter()
            .position(|plan| std::ptr::eq(plan.plan(), target))
        {
            return position as u32 + 1;
        }
    }
    let _ = target;
    PAMOJA_LORA_CN470_NONE
}

/// Reads how a plan defines and uses its channels.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `out_rules` - set to the rules on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_rules` must point at writable storage for
/// one [`PamojaLoraPlanRules`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_rules(
    plan: *const PamojaLoraPlan,
    out_rules: *mut PamojaLoraPlanRules,
) -> PamojaStatus {
    let (Some(handle), false) = (plan.as_ref(), out_rules.is_null()) else {
        set_last_error("plan and out_rules must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let join_plan_count = handle
        .published
        .map_or(0, |plan| plan.join_plans.len() as u16);
    *out_rules = handle.with(|plan| {
        let (kind, channel_list) = match plan.kind {
            PlanKind::Dynamic { channel_list } => (
                PAMOJA_LORA_PLAN_KIND_DYNAMIC,
                match channel_list {
                    None => PAMOJA_LORA_CHANNEL_LIST_NONE,
                    Some(FixedChannelList::Mhz800) => PAMOJA_LORA_CHANNEL_LIST_MHZ800,
                    Some(FixedChannelList::Mhz900) => PAMOJA_LORA_CHANNEL_LIST_MHZ900,
                },
            ),
            PlanKind::Fixed => (PAMOJA_LORA_PLAN_KIND_FIXED, PAMOJA_LORA_CHANNEL_LIST_NONE),
        };
        let (power_reference, gain_allowance_db) = match plan.power_reference {
            PowerReference::Eirp => (PAMOJA_LORA_POWER_EIRP, 0),
            PowerReference::Conducted { gain_allowance_db } => {
                (PAMOJA_LORA_POWER_CONDUCTED, gain_allowance_db)
            }
        };
        PamojaLoraPlanRules {
            kind,
            channel_list,
            tx_param_setup: u8::from(plan.tx_param_setup),
            join_sequence: match plan.join_sequence {
                JoinSequence::Random => PAMOJA_LORA_JOIN_RANDOM,
                JoinSequence::OctetPasses => PAMOJA_LORA_JOIN_OCTET_PASSES,
            },
            power_reference,
            gain_allowance_db,
            downlink_channel_block_count: plan.downlink_channels.len() as u16,
            join_plan_count,
        }
    });
    PamojaStatus::Ok
}

/// Converts a channel mask control into the shape that crosses the boundary.
fn mask_control_out(control: MaskControl) -> PamojaLoraMaskControl {
    let mut out = PamojaLoraMaskControl {
        kind: PAMOJA_LORA_MASK_RESERVED,
        group: 0,
        on: 0,
        has_then_group: 0,
        then_group: 0,
    };
    match control {
        MaskControl::Group(group) => {
            out.kind = PAMOJA_LORA_MASK_GROUP;
            out.group = group;
        }
        MaskControl::Banks => out.kind = PAMOJA_LORA_MASK_BANKS,
        MaskControl::PairedBanks => out.kind = PAMOJA_LORA_MASK_PAIRED_BANKS,
        MaskControl::All { on, then_group } => {
            out.kind = PAMOJA_LORA_MASK_ALL;
            out.on = u8::from(on);
            out.has_then_group = u8::from(then_group.is_some());
            out.then_group = then_group.unwrap_or(0);
        }
        MaskControl::Reserved => {}
    }
    out
}

/// Converts a channel mask control that crossed the boundary.
fn mask_control_in(control: &PamojaLoraMaskControl) -> Result<MaskControl, PamojaStatus> {
    match control.kind {
        PAMOJA_LORA_MASK_GROUP => Ok(MaskControl::Group(control.group)),
        PAMOJA_LORA_MASK_BANKS => Ok(MaskControl::Banks),
        PAMOJA_LORA_MASK_PAIRED_BANKS => Ok(MaskControl::PairedBanks),
        PAMOJA_LORA_MASK_ALL => Ok(MaskControl::All {
            on: control.on != 0,
            then_group: (control.has_then_group != 0).then_some(control.then_group),
        }),
        PAMOJA_LORA_MASK_RESERVED => Ok(MaskControl::Reserved),
        other => {
            set_last_error(format!("{other} is not a channel mask control"));
            Err(PamojaStatus::InvalidArgument)
        }
    }
}

/// Reads what one `ChMaskCntl` value does on a plan.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `value` - the `ChMaskCntl` value, 0 to 7.
/// * `out_control` - set to the control on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or `value` is past 7.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_control` must point at writable storage for
/// one [`PamojaLoraMaskControl`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_mask_control(
    plan: *const PamojaLoraPlan,
    value: u8,
    out_control: *mut PamojaLoraMaskControl,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_control.is_null()) else {
        set_last_error("plan and out_control must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if value > 7 {
        set_last_error(format!(
            "{value} is not a ChMaskCntl value, which is three bits"
        ));
        return PamojaStatus::InvalidArgument;
    }
    *out_control = plan.with(|plan| mask_control_out(plan.mask_controls[usize::from(value)]));
    PamojaStatus::Ok
}

/// Returns where the first receive window listens after an uplink.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `uplink_channel` - the channel number the uplink went out on.
/// * `uplink_hz` - the frequency it went out on.
/// * `out_frequency_hz` - set to the window's frequency on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success: the uplink's own frequency on a plan with no numbered
/// downlink channels, and otherwise the downlink channel the uplink channel maps to.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or the plan's
/// downlink channels leave the window undefined.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_frequency_hz` must point at writable storage
/// for one `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_rx1_frequency_hz(
    plan: *const PamojaLoraPlan,
    uplink_channel: u16,
    uplink_hz: u32,
    out_frequency_hz: *mut u32,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_frequency_hz.is_null()) else {
        set_last_error("plan and out_frequency_hz must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(frequency) = plan.with(|plan| plan.rx1_frequency_hz(uplink_channel, uplink_hz)) else {
        set_last_error(format!(
            "this plan names no downlink for uplink channel {uplink_channel}"
        ));
        return PamojaStatus::InvalidArgument;
    };
    *out_frequency_hz = frequency;
    PamojaStatus::Ok
}

/// Returns the frequency of one of the plan's numbered downlink channels.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `channel` - the downlink channel number.
/// * `out_frequency_hz` - set to its frequency on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or the plan numbers
/// no such downlink channel.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_frequency_hz` must point at writable storage
/// for one `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_downlink_channel_frequency_hz(
    plan: *const PamojaLoraPlan,
    channel: u16,
    out_frequency_hz: *mut u32,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_frequency_hz.is_null()) else {
        set_last_error("plan and out_frequency_hz must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(frequency) = plan.with(|plan| plan.downlink_channel_frequency_hz(channel)) else {
        set_last_error(format!("this plan has no downlink channel {channel}"));
        return PamojaStatus::InvalidArgument;
    };
    *out_frequency_hz = frequency;
    PamojaStatus::Ok
}

/// Returns one run of join channels that selects a plan.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `index` - the run's position, below the count [`pamoja_lora_plan_rules`] reports.
/// * `out_join_plan` - set to the run on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or the index is past
/// the end, which it always is for a plan that was built rather than published.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_join_plan` must point at writable storage for
/// one [`PamojaLoraJoinPlan`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_join_plan(
    plan: *const PamojaLoraPlan,
    index: u16,
    out_join_plan: *mut PamojaLoraJoinPlan,
) -> PamojaStatus {
    let (Some(handle), false) = (plan.as_ref(), out_join_plan.is_null()) else {
        set_last_error("plan and out_join_plan must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(run) = handle
        .published
        .and_then(|plan| plan.join_plans.get(usize::from(index)))
    else {
        set_last_error(format!("this plan has no join plan {index}"));
        return PamojaStatus::InvalidArgument;
    };
    *out_join_plan = PamojaLoraJoinPlan {
        channels: PamojaLoraChannelBlock {
            start_hz: run.channels.start_hz,
            step_hz: run.channels.step_hz,
            count: run.channels.count,
            min_data_rate: run.channels.min_data_rate,
            max_data_rate: run.channels.max_data_rate,
        },
        accept_start_hz: run.accept_start_hz,
        accept_step_hz: run.accept_step_hz,
        rx2_start_hz: run.rx2_start_hz,
        rx2_step_hz: run.rx2_step_hz,
        cn470_plan: cn470_code(run.plan),
    };
    PamojaStatus::Ok
}

/// Finds the run of join channels a join channel belongs to.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `join_channel` - the join channel, counted through the runs in order.
/// * `out_index` - set to the run's position on success.
/// * `out_offset` - set to the channel's place within the run on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. The run's accept frequency for the channel is
/// `accept_start_hz + offset * accept_step_hz`, and its second window
/// `rx2_start_hz + offset * rx2_step_hz`.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null or no run holds the
/// channel.
///
/// # Safety
///
/// `plan` must be a live plan handle, and `out_index` and `out_offset` must each point at
/// writable storage for one `uint16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_join_plan_for_channel(
    plan: *const PamojaLoraPlan,
    join_channel: u16,
    out_index: *mut u16,
    out_offset: *mut u16,
) -> PamojaStatus {
    let (Some(handle), false, false) = (plan.as_ref(), out_index.is_null(), out_offset.is_null())
    else {
        set_last_error("plan, out_index and out_offset must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let mut remaining = join_channel;
    for (index, run) in handle
        .published
        .map_or(&[][..], |plan| plan.join_plans)
        .iter()
        .enumerate()
    {
        if remaining < run.channels.count {
            *out_index = index as u16;
            *out_offset = remaining;
            return PamojaStatus::Ok;
        }
        remaining -= run.channels.count;
    }
    set_last_error(format!("no join plan holds join channel {join_channel}"));
    PamojaStatus::InvalidArgument
}

/// Reports whether a region is compiled into this build.
///
/// A slim build carries only the regions its device operates in, so a host that
/// offers a choice asks this before offering one.
///
/// # Arguments
///
/// * `region` - one of the `PAMOJA_LORA_REGION_*` constants.
///
/// # Returns
///
/// `1` if the region is available, `0` if it is a known region left out of this
/// build or is not a region code at all.
#[no_mangle]
pub extern "C" fn pamoja_lora_region_is_available(region: u32) -> u8 {
    u8::from(published(region).is_ok())
}

/// Releases a channel plan.
///
/// # Arguments
///
/// * `plan` - the handle to release; null is ignored.
///
/// # Safety
///
/// `plan` must have come from [`pamoja_lora_plan_for_region`] or
/// [`pamoja_lora_plan_builder_build`] and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_free(plan: *mut PamojaLoraPlan) {
    if !plan.is_null() {
        drop(Box::from_raw(plan));
    }
}

/// Returns the plan's name, such as `EU863-870`.
///
/// # Arguments
///
/// * `plan` - the plan to read.
///
/// # Returns
///
/// A string the caller must release with
/// [`pamoja_string_free`](crate::pamoja_string_free), or null if `plan` is null.
///
/// # Safety
///
/// `plan` must be a live plan handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_name(plan: *const PamojaLoraPlan) -> *mut PamojaString {
    let Some(plan) = plan.as_ref() else {
        set_last_error("plan must not be null".to_owned());
        return std::ptr::null_mut();
    };
    plan.with(|plan| PamojaString::into_raw(plan.name.to_owned()))
}

/// Reads the scalar facts of a plan in one call.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `out_info` - set to the plan's scalars on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_info` must point at writable
/// storage for one [`PamojaLoraPlanInfo`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_info(
    plan: *const PamojaLoraPlan,
    out_info: *mut PamojaLoraPlanInfo,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_info.is_null()) else {
        set_last_error("plan and out_info must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_info = plan.with(|plan| PamojaLoraPlanInfo {
        rx2_frequency_hz: plan.rx2_frequency_hz,
        uplink_data_rate_count: plan.uplink_data_rates.len() as u16,
        downlink_data_rate_count: plan.downlink_data_rates.len() as u16,
        default_channel_count: plan.default_channel_count(),
        join_channel_block_count: plan.join_channels.len() as u16,
        default_channel_block_count: plan.default_channels.len() as u16,
        sub_band_count: plan.sub_bands.len() as u16,
        beacon: PamojaLoraBeacon {
            frequency_hz: plan.beacon.frequency_hz,
            ping_slot_frequency_hz: plan.beacon.ping_slot_frequency_hz,
            data_rate: plan.beacon.data_rate,
        },
        rx2_data_rate: plan.rx2_data_rate,
        default_max_eirp_dbm: plan.default_max_eirp_dbm,
        tx_power_step_db: plan.tx_power_step_db,
        max_tx_power_index: plan.max_tx_power_index,
        max_rx1_data_rate_offset: plan.max_rx1_data_rate_offset,
        has_dwell_time_limit: u8::from(plan.has_dwell_time_limit),
        has_dwell_limited_payloads: u8::from(plan.max_payload_dwell_limited.is_some()),
        has_dwell_limited_rx1: u8::from(plan.rx1_data_rate_offsets_dwell_limited.is_some()),
    });
    PamojaStatus::Ok
}

/// Returns the data rate a number selects.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `direction` - [`PAMOJA_LORA_DIRECTION_UPLINK`] or
///   [`PAMOJA_LORA_DIRECTION_DOWNLINK`], which differ in the 900 MHz plans.
/// * `data_rate` - the data-rate number.
/// * `out_rate` - set to the data rate on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. A reserved number succeeds and reports
/// [`PAMOJA_LORA_MODULATION_RESERVED`].
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, the
/// direction is not one of the two constants, or the number is past the end of
/// the plan's table.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_rate` must point at writable
/// storage for one [`PamojaLoraDataRate`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_data_rate(
    plan: *const PamojaLoraPlan,
    direction: u32,
    data_rate: u8,
    out_rate: *mut PamojaLoraDataRate,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_rate.is_null()) else {
        set_last_error("plan and out_rate must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let found = plan.with(|plan| match direction {
        PAMOJA_LORA_DIRECTION_UPLINK => Ok(plan
            .uplink_data_rates
            .get(usize::from(data_rate))
            .copied()
            .map(data_rate_out)),
        PAMOJA_LORA_DIRECTION_DOWNLINK => Ok(plan
            .downlink_data_rates
            .get(usize::from(data_rate))
            .copied()
            .map(data_rate_out)),
        other => Err(other),
    });
    match found {
        Ok(Some(rate)) => {
            *out_rate = rate;
            PamojaStatus::Ok
        }
        Ok(None) => {
            set_last_error(format!("this plan defines no data rate {data_rate}"));
            PamojaStatus::InvalidArgument
        }
        Err(other) => {
            set_last_error(format!("{other} is not a direction"));
            PamojaStatus::InvalidArgument
        }
    }
}

/// Returns the radio settings an uplink data rate selects.
///
/// This is what turns a data-rate number into something a radio can be told: the
/// spreading factor and bandwidth to transmit at, ready for
/// `pamoja_lora_airtime_us`.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `data_rate` - the uplink data-rate number.
/// * `out_link` - set to the radio settings on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, and
/// [`PamojaStatus::Unsupported`] if the number is reserved or names a rate that
/// is not LoRa, which has no spreading factor to report.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_link` must point at writable
/// storage for one [`PamojaLoraLink`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_link_settings(
    plan: *const PamojaLoraPlan,
    data_rate: u8,
    out_link: *mut PamojaLoraLink,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_link.is_null()) else {
        set_last_error("plan and out_link must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(settings) = plan.with(|plan| plan.link_settings(data_rate)) else {
        set_last_error(format!(
            "data rate {data_rate} is reserved or is not carried by LoRa in this plan"
        ));
        return PamojaStatus::Unsupported;
    };
    *out_link = PamojaLoraLink {
        bandwidth_hz: settings.bandwidth_hz(),
        preamble_symbols: 8,
        spreading_factor: settings.spreading_factor(),
        coding_rate_denominator: 5,
        explicit_header: 1,
        crc: 1,
    };
    PamojaStatus::Ok
}

/// Returns what a data rate may carry in one frame.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `table` - one of the `PAMOJA_LORA_PAYLOAD_TABLE_*` constants.
/// * `data_rate` - the data-rate number.
/// * `out_payload` - set to the limits on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, the table
/// is not one of the constants, or the plan publishes no limit for that number.
/// Returns [`PamojaStatus::Unsupported`] if the dwell-limited table was asked for
/// and this plan has no dwell-time limit.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_payload` must point at writable
/// storage for one [`PamojaLoraMaxPayload`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_max_payload(
    plan: *const PamojaLoraPlan,
    table: u32,
    data_rate: u8,
    out_payload: *mut PamojaLoraMaxPayload,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_payload.is_null()) else {
        set_last_error("plan and out_payload must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let found = plan.with(|plan| match table {
        PAMOJA_LORA_PAYLOAD_TABLE_UPLINK_REPEATER => Ok(plan.max_payload(data_rate, true)),
        PAMOJA_LORA_PAYLOAD_TABLE_UPLINK_DIRECT => Ok(plan.max_payload(data_rate, false)),
        PAMOJA_LORA_PAYLOAD_TABLE_DOWNLINK_REPEATER => {
            Ok(plan.downlink_max_payload(data_rate, true))
        }
        PAMOJA_LORA_PAYLOAD_TABLE_DOWNLINK_DIRECT => {
            Ok(plan.downlink_max_payload(data_rate, false))
        }
        PAMOJA_LORA_PAYLOAD_TABLE_DWELL_LIMITED => {
            if plan.max_payload_dwell_limited.is_none() {
                Err(PamojaStatus::Unsupported)
            } else {
                Ok(plan.max_payload_dwell_limited(data_rate))
            }
        }
        _ => Err(PamojaStatus::InvalidArgument),
    });
    match found {
        Ok(Some(payload)) => {
            *out_payload = PamojaLoraMaxPayload {
                mac_payload: payload.mac_payload,
                application: payload.application,
            };
            PamojaStatus::Ok
        }
        Ok(None) => {
            set_last_error(format!(
                "this plan publishes no payload limit for data rate {data_rate}"
            ));
            PamojaStatus::InvalidArgument
        }
        Err(PamojaStatus::Unsupported) => {
            set_last_error("this plan has no dwell-time limit".to_owned());
            PamojaStatus::Unsupported
        }
        Err(_) => {
            set_last_error(format!("{table} is not a payload table"));
            PamojaStatus::InvalidArgument
        }
    }
}

/// Returns the share of time a transmitter may hold a frequency.
///
/// This reports the limit; it does not impose it. Pair it with
/// `pamoja_lora_min_off_time_us` to turn the
/// limit into the silence a given frame costs.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `frequency_hz` - the frequency in hertz.
/// * `out_permille` - set to the limit in parts per thousand on success, where
///   `1000` means the sub-band is unrestricted.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, and
/// [`PamojaStatus::Unsupported`] if the frequency falls in no sub-band this plan
/// describes.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_permille` must point at writable
/// storage for one `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_duty_cycle_permille(
    plan: *const PamojaLoraPlan,
    frequency_hz: u32,
    out_permille: *mut u32,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_permille.is_null()) else {
        set_last_error("plan and out_permille must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(permille) = plan.with(|plan| plan.duty_cycle_permille(frequency_hz)) else {
        set_last_error(format!(
            "{frequency_hz} Hz falls in no sub-band this plan describes"
        ));
        return PamojaStatus::Unsupported;
    };
    *out_permille = permille;
    PamojaStatus::Ok
}

/// Returns the power ceiling that applies at a frequency, in dBm EIRP.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `frequency_hz` - the frequency in hertz.
/// * `out_dbm` - set to the ceiling on success, falling back to the plan's default
///   where no sub-band says otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_dbm` must point at writable storage
/// for one `int8_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_max_eirp_dbm(
    plan: *const PamojaLoraPlan,
    frequency_hz: u32,
    out_dbm: *mut i8,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_dbm.is_null()) else {
        set_last_error("plan and out_dbm must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_dbm = plan.with(|plan| plan.max_eirp_dbm(frequency_hz));
    PamojaStatus::Ok
}

/// Returns the radiated power a transmit-power index selects, in dBm.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `index` - the transmit-power index, where zero is the ceiling.
/// * `max_eirp_dbm` - the ceiling the index steps down from, usually from
///   [`pamoja_lora_plan_max_eirp_dbm`].
/// * `out_dbm` - set to the radiated power on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or the
/// index is past the highest the plan defines.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_dbm` must point at writable storage
/// for one `int8_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_tx_power_dbm(
    plan: *const PamojaLoraPlan,
    index: u8,
    max_eirp_dbm: i8,
    out_dbm: *mut i8,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_dbm.is_null()) else {
        set_last_error("plan and out_dbm must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(dbm) = plan.with(|plan| plan.tx_power_dbm(index, max_eirp_dbm)) else {
        set_last_error(format!("this plan defines no transmit-power index {index}"));
        return PamojaStatus::InvalidArgument;
    };
    *out_dbm = dbm;
    PamojaStatus::Ok
}

/// Returns the downlink data rate the first receive window listens at.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `uplink_data_rate` - the data rate the uplink was sent at.
/// * `offset` - the RX1 data-rate offset the network assigned.
/// * `dwell_limited` - `1` to use the mapping for a dwell-limited downlink, `0`
///   for the ordinary one.
/// * `out_data_rate` - set to the downlink data rate on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, or the
/// uplink data rate or offset is outside what the plan defines. Returns
/// [`PamojaStatus::Unsupported`] if a dwell-limited mapping was asked for and this
/// plan publishes none.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_data_rate` must point at writable
/// storage for one `uint8_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_rx1_data_rate(
    plan: *const PamojaLoraPlan,
    uplink_data_rate: u8,
    offset: u8,
    dwell_limited: u8,
    out_data_rate: *mut u8,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_data_rate.is_null()) else {
        set_last_error("plan and out_data_rate must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let wants_dwell = dwell_limited != 0;
    if wants_dwell && plan.with(|plan| plan.rx1_data_rate_offsets_dwell_limited.is_none()) {
        set_last_error("this plan publishes no dwell-limited RX1 mapping".to_owned());
        return PamojaStatus::Unsupported;
    }
    let found = plan.with(|plan| {
        if wants_dwell {
            plan.rx1_data_rate_dwell_limited(uplink_data_rate, offset)
        } else {
            plan.rx1_data_rate(uplink_data_rate, offset)
        }
    });
    let Some(data_rate) = found else {
        set_last_error(format!(
            "this plan maps no RX1 downlink for uplink data rate {uplink_data_rate} at offset {offset}"
        ));
        return PamojaStatus::InvalidArgument;
    };
    *out_data_rate = data_rate;
    PamojaStatus::Ok
}

/// Returns the next lower data rate to fall back to during adaptive back-off.
///
/// A device that has lost the network steps down this chain, trading airtime for
/// range until it is heard again.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `data_rate` - the data rate currently in use.
/// * `out_data_rate` - set to the next lower data rate on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or the
/// number is outside the plan's table, and [`PamojaStatus::Unsupported`] if there
/// is nothing lower to fall back to.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_data_rate` must point at writable
/// storage for one `uint8_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_next_backoff_data_rate(
    plan: *const PamojaLoraPlan,
    data_rate: u8,
    out_data_rate: *mut u8,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_data_rate.is_null()) else {
        set_last_error("plan and out_data_rate must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let known = plan.with(|plan| usize::from(data_rate) < plan.data_rate_backoff.len());
    if !known {
        set_last_error(format!("this plan defines no data rate {data_rate}"));
        return PamojaStatus::InvalidArgument;
    }
    let Some(lower) = plan.with(|plan| plan.next_backoff_data_rate(data_rate)) else {
        set_last_error(format!(
            "data rate {data_rate} is the slowest this plan has"
        ));
        return PamojaStatus::Unsupported;
    };
    *out_data_rate = lower;
    PamojaStatus::Ok
}

/// Returns the center frequency of one of the plan's default channels.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `channel` - the channel number, counting across the default blocks in order.
/// * `out_frequency_hz` - set to the center frequency on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or the
/// channel is past the last one the plan starts a device with.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_frequency_hz` must point at
/// writable storage for one `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_channel_frequency_hz(
    plan: *const PamojaLoraPlan,
    channel: u16,
    out_frequency_hz: *mut u32,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_frequency_hz.is_null()) else {
        set_last_error("plan and out_frequency_hz must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(frequency) = plan.with(|plan| plan.channel_frequency_hz(channel)) else {
        set_last_error(format!("this plan has no default channel {channel}"));
        return PamojaStatus::InvalidArgument;
    };
    *out_frequency_hz = frequency;
    PamojaStatus::Ok
}

/// Returns one of the plan's channel blocks.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `which` - [`PAMOJA_LORA_CHANNELS_JOIN`], [`PAMOJA_LORA_CHANNELS_DEFAULT`] or
///   [`PAMOJA_LORA_CHANNELS_DOWNLINK`].
/// * `index` - the block's position, below the count [`pamoja_lora_plan_info`] or
///   [`pamoja_lora_plan_rules`] reports.
/// * `out_block` - set to the block on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, `which`
/// is not one of the constants, or the index is past the end.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_block` must point at writable
/// storage for one [`PamojaLoraChannelBlock`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_channel_block(
    plan: *const PamojaLoraPlan,
    which: u32,
    index: u16,
    out_block: *mut PamojaLoraChannelBlock,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_block.is_null()) else {
        set_last_error("plan and out_block must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if !(PAMOJA_LORA_CHANNELS_JOIN..=PAMOJA_LORA_CHANNELS_DOWNLINK).contains(&which) {
        set_last_error(format!("{which} is not a channel set"));
        return PamojaStatus::InvalidArgument;
    }
    let found = plan.with(|plan| {
        let blocks = match which {
            PAMOJA_LORA_CHANNELS_JOIN => plan.join_channels,
            PAMOJA_LORA_CHANNELS_DEFAULT => plan.default_channels,
            _ => plan.downlink_channels,
        };
        blocks.get(usize::from(index)).copied()
    });
    let Some(block) = found else {
        set_last_error(format!("this plan has no channel block {index}"));
        return PamojaStatus::InvalidArgument;
    };
    *out_block = PamojaLoraChannelBlock {
        start_hz: block.start_hz,
        step_hz: block.step_hz,
        count: block.count,
        min_data_rate: block.min_data_rate,
        max_data_rate: block.max_data_rate,
    };
    PamojaStatus::Ok
}

/// Returns one of the plan's sub-bands.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `index` - the sub-band's position, below the count
///   [`pamoja_lora_plan_info`] reports.
/// * `out_band` - set to the sub-band on success.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or the
/// index is past the end.
///
/// # Safety
///
/// `plan` must be a live plan handle and `out_band` must point at writable
/// storage for one [`PamojaLoraSubBand`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_sub_band(
    plan: *const PamojaLoraPlan,
    index: u16,
    out_band: *mut PamojaLoraSubBand,
) -> PamojaStatus {
    let (Some(plan), false) = (plan.as_ref(), out_band.is_null()) else {
        set_last_error("plan and out_band must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let found = plan.with(|plan| plan.sub_bands.get(usize::from(index)).copied());
    let Some(band) = found else {
        set_last_error(format!("this plan has no sub-band {index}"));
        return PamojaStatus::InvalidArgument;
    };
    *out_band = PamojaLoraSubBand {
        start_hz: band.start_hz,
        end_hz: band.end_hz,
        duty_cycle_permille: band.duty_cycle_permille,
        max_eirp_dbm: band.max_eirp_dbm,
    };
    PamojaStatus::Ok
}

/// A channel plan under construction.
///
/// A handle the caller must release with [`pamoja_lora_plan_builder_free`], or
/// hand to [`pamoja_lora_plan_builder_build`], which consumes it.
pub struct PamojaLoraPlanBuilder {
    builder: Option<ChannelPlanBuilder>,
}

/// Applies one step to a builder held behind a handle.
///
/// The Rust builder consumes itself at each step, so the handle lends it out and
/// takes it back.
///
/// # Arguments
///
/// * `builder` - the handle to update.
/// * `step` - the step to apply.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::Closed`] if the builder was already built.
fn update(
    builder: &mut PamojaLoraPlanBuilder,
    step: impl FnOnce(ChannelPlanBuilder) -> ChannelPlanBuilder,
) -> PamojaStatus {
    let Some(inner) = builder.builder.take() else {
        set_last_error("this builder has already been built".to_owned());
        return PamojaStatus::Closed;
    };
    builder.builder = Some(step(inner));
    PamojaStatus::Ok
}

/// Maps a payload-table code onto the table it names.
///
/// # Arguments
///
/// * `table` - one of the `PAMOJA_LORA_PAYLOAD_TABLE_*` constants.
///
/// # Returns
///
/// The table, or `None` if the code names none.
fn payload_table(table: u32) -> Option<PayloadTable> {
    match table {
        PAMOJA_LORA_PAYLOAD_TABLE_UPLINK_REPEATER => Some(PayloadTable::UplinkRepeater),
        PAMOJA_LORA_PAYLOAD_TABLE_UPLINK_DIRECT => Some(PayloadTable::UplinkDirect),
        PAMOJA_LORA_PAYLOAD_TABLE_DOWNLINK_REPEATER => Some(PayloadTable::DownlinkRepeater),
        PAMOJA_LORA_PAYLOAD_TABLE_DOWNLINK_DIRECT => Some(PayloadTable::DownlinkDirect),
        PAMOJA_LORA_PAYLOAD_TABLE_DWELL_LIMITED => Some(PayloadTable::DwellLimited),
        _ => None,
    }
}

/// Creates an empty plan builder.
///
/// The builder starts with no data rates, channels, or sub-bands, and with a
/// permissive power ceiling; push the tables the deployment uses, then build.
///
/// # Arguments
///
/// * `name` - a null-terminated name for the plan, such as the band it covers.
/// * `out_builder` - set to the builder handle on success, and to null otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or `name`
/// is not valid UTF-8.
///
/// # Safety
///
/// `name` must be a valid null-terminated string and `out_builder` must point at
/// writable storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_new(
    name: *const std::os::raw::c_char,
    out_builder: *mut *mut PamojaLoraPlanBuilder,
) -> PamojaStatus {
    if out_builder.is_null() {
        set_last_error("out_builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_builder;
    *slot = std::ptr::null_mut();

    let Some(name) = crate::read_str(name, "name") else {
        return PamojaStatus::InvalidArgument;
    };

    *slot = Box::into_raw(Box::new(PamojaLoraPlanBuilder {
        builder: Some(ChannelPlanBuilder::new(name)),
    }));
    PamojaStatus::Ok
}

/// Releases a plan builder that will not be built.
///
/// # Arguments
///
/// * `builder` - the handle to release; null is ignored.
///
/// # Safety
///
/// `builder` must have come from [`pamoja_lora_plan_builder_new`], must not have
/// been passed to [`pamoja_lora_plan_builder_build`], and must not be used
/// afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_free(builder: *mut PamojaLoraPlanBuilder) {
    if !builder.is_null() {
        drop(Box::from_raw(builder));
    }
}

/// Appends a data rate to the end of a direction's table.
///
/// Data rates are numbered by their position, so push them in order and use a
/// [`PAMOJA_LORA_MODULATION_RESERVED`] entry for a number the plan does not use.
/// A plan that leaves its downlink table empty reuses its uplink table, which is
/// what most regions do.
///
/// # Arguments
///
/// * `builder` - the builder to extend.
/// * `direction` - [`PAMOJA_LORA_DIRECTION_UPLINK`] or
///   [`PAMOJA_LORA_DIRECTION_DOWNLINK`].
/// * `rate` - the data rate to append.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, the
/// direction is not one of the constants, or the modulation kind is not one this
/// ABI defines, and [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle and `rate` must point at one readable
/// [`PamojaLoraDataRate`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_push_data_rate(
    builder: *mut PamojaLoraPlanBuilder,
    direction: u32,
    rate: *const PamojaLoraDataRate,
) -> PamojaStatus {
    let (Some(builder), Some(rate)) = (builder.as_mut(), rate.as_ref()) else {
        set_last_error("builder and rate must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let rate = match data_rate_in(rate) {
        Ok(rate) => rate,
        Err(status) => return status,
    };
    match direction {
        PAMOJA_LORA_DIRECTION_UPLINK => update(builder, |b| b.uplink_data_rate(rate)),
        PAMOJA_LORA_DIRECTION_DOWNLINK => update(builder, |b| b.downlink_data_rate(rate)),
        other => {
            set_last_error(format!("{other} is not a direction"));
            PamojaStatus::InvalidArgument
        }
    }
}

/// Appends a payload limit to the end of one of the plan's tables.
///
/// Limits are numbered by their position, matching the data rates. A plan that
/// leaves a downlink table empty reuses the matching uplink one.
///
/// # Arguments
///
/// * `builder` - the builder to extend.
/// * `table` - one of the `PAMOJA_LORA_PAYLOAD_TABLE_*` constants.
/// * `present` - `0` to append a reserved entry, for a data rate the plan does
///   not define; the two lengths are then ignored.
/// * `mac_payload` - the largest MAC payload in bytes.
/// * `application` - the largest application payload in bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` is null or `table` is
/// not one of the constants, and [`PamojaStatus::Closed`] if the builder was
/// already built.
///
/// # Safety
///
/// `builder` must be a live builder handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_push_max_payload(
    builder: *mut PamojaLoraPlanBuilder,
    table: u32,
    present: u8,
    mac_payload: u16,
    application: u16,
) -> PamojaStatus {
    let Some(builder) = builder.as_mut() else {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(table) = payload_table(table) else {
        set_last_error(format!("{table} is not a payload table"));
        return PamojaStatus::InvalidArgument;
    };
    let entry = (present != 0).then(|| MaxPayload::new(mac_payload, application));
    update(builder, |b| b.max_payload(table, entry))
}

/// Appends a run of evenly spaced channels to the plan.
///
/// # Arguments
///
/// * `builder` - the builder to extend.
/// * `which` - [`PAMOJA_LORA_CHANNELS_JOIN`], [`PAMOJA_LORA_CHANNELS_DEFAULT`] or
///   [`PAMOJA_LORA_CHANNELS_DOWNLINK`].
/// * `block` - the channel block to append.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or `which`
/// is not one of the constants, and [`PamojaStatus::Closed`] if the builder was
/// already built.
///
/// # Safety
///
/// `builder` must be a live builder handle and `block` must point at one readable
/// [`PamojaLoraChannelBlock`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_push_channel_block(
    builder: *mut PamojaLoraPlanBuilder,
    which: u32,
    block: *const PamojaLoraChannelBlock,
) -> PamojaStatus {
    let (Some(builder), Some(block)) = (builder.as_mut(), block.as_ref()) else {
        set_last_error("builder and block must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let entry = ChannelBlock::new(
        block.start_hz,
        block.step_hz,
        block.count,
        block.min_data_rate,
        block.max_data_rate,
    );
    match which {
        PAMOJA_LORA_CHANNELS_JOIN => update(builder, |b| b.join_channel(entry)),
        PAMOJA_LORA_CHANNELS_DEFAULT => update(builder, |b| b.default_channel(entry)),
        PAMOJA_LORA_CHANNELS_DOWNLINK => update(builder, |b| b.downlink_channel(entry)),
        other => {
            set_last_error(format!("{other} is not a channel set"));
            PamojaStatus::InvalidArgument
        }
    }
}

/// Appends a sub-band and its transmit limits to the plan.
///
/// A deployment on licensed spectrum gives its sub-band a duty cycle of `1000`,
/// which reports as unrestricted.
///
/// # Arguments
///
/// * `builder` - the builder to extend.
/// * `band` - the sub-band to append.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, and
/// [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle and `band` must point at one readable
/// [`PamojaLoraSubBand`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_push_sub_band(
    builder: *mut PamojaLoraPlanBuilder,
    band: *const PamojaLoraSubBand,
) -> PamojaStatus {
    let (Some(builder), Some(band)) = (builder.as_mut(), band.as_ref()) else {
        set_last_error("builder and band must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let entry = SubBand::new(
        band.start_hz,
        band.end_hz,
        band.duty_cycle_permille,
        band.max_eirp_dbm,
    );
    update(builder, |b| b.sub_band(entry))
}

/// Appends one uplink data rate's row of RX1 downlink data rates.
///
/// Rows are numbered by their position, matching the uplink data rates, and every
/// row must be as wide as the plan's highest RX1 offset allows.
///
/// # Arguments
///
/// * `builder` - the builder to extend.
/// * `dwell_limited` - `1` to append to the mapping used under a dwell-time
///   limit, `0` for the ordinary one.
/// * `offsets` - the downlink data rate for each offset, in order.
/// * `offsets_len` - how many offsets `offsets` holds.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` or `offsets` is null,
/// and [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle and `offsets` must point at
/// `offsets_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_push_rx1_row(
    builder: *mut PamojaLoraPlanBuilder,
    dwell_limited: u8,
    offsets: *const u8,
    offsets_len: usize,
) -> PamojaStatus {
    let Some(builder) = builder.as_mut() else {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let row = match crate::read_bytes(offsets, offsets_len) {
        Ok(row) => row,
        Err(status) => return status,
    };
    if dwell_limited == 0 {
        update(builder, |b| b.rx1_row(&row))
    } else {
        update(builder, |b| b.rx1_row_dwell_limited(&row))
    }
}

/// Appends the next entry in the adaptive back-off chain.
///
/// Entries are numbered by their position, matching the uplink data rates.
///
/// # Arguments
///
/// * `builder` - the builder to extend.
/// * `has_lower` - `0` if this data rate is the slowest, with nothing below it;
///   `data_rate` is then ignored.
/// * `data_rate` - the data rate to fall back to.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` is null, and
/// [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_push_backoff(
    builder: *mut PamojaLoraPlanBuilder,
    has_lower: u8,
    data_rate: u8,
) -> PamojaStatus {
    let Some(builder) = builder.as_mut() else {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let lower = (has_lower != 0).then_some(data_rate);
    update(builder, |b| b.backoff(lower))
}

/// Sets the plan's transmit-power ladder.
///
/// # Arguments
///
/// * `builder` - the builder to set.
/// * `default_max_eirp_dbm` - the ceiling assumed where no sub-band says
///   otherwise.
/// * `tx_power_step_db` - the step between transmit-power settings, in dB.
/// * `max_tx_power_index` - the highest transmit-power index the plan defines.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` is null, and
/// [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_set_power(
    builder: *mut PamojaLoraPlanBuilder,
    default_max_eirp_dbm: i8,
    tx_power_step_db: u8,
    max_tx_power_index: u8,
) -> PamojaStatus {
    let Some(builder) = builder.as_mut() else {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    update(builder, |b| {
        b.power(default_max_eirp_dbm, tx_power_step_db, max_tx_power_index)
    })
}

/// Sets the plan's receive windows.
///
/// # Arguments
///
/// * `builder` - the builder to set.
/// * `rx2_frequency_hz` - the fixed frequency the second window listens on.
/// * `rx2_data_rate` - the data rate the second window listens at.
/// * `max_rx1_data_rate_offset` - the highest RX1 offset the plan allows, which
///   fixes how wide every RX1 row must be.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` is null, and
/// [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_set_rx(
    builder: *mut PamojaLoraPlanBuilder,
    rx2_frequency_hz: u32,
    rx2_data_rate: u8,
    max_rx1_data_rate_offset: u8,
) -> PamojaStatus {
    let Some(builder) = builder.as_mut() else {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    update(builder, |b| {
        b.rx(rx2_frequency_hz, rx2_data_rate, max_rx1_data_rate_offset)
    })
}

/// Sets the plan's Class B beacon and whether it limits dwell time.
///
/// # Arguments
///
/// * `builder` - the builder to set.
/// * `beacon` - the beacon settings.
/// * `has_dwell_time_limit` - `1` if the plan caps how long one transmission may
///   hold a channel.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, and
/// [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle and `beacon` must point at one
/// readable [`PamojaLoraBeacon`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_set_beacon(
    builder: *mut PamojaLoraPlanBuilder,
    beacon: *const PamojaLoraBeacon,
    has_dwell_time_limit: u8,
) -> PamojaStatus {
    let (Some(builder), Some(beacon)) = (builder.as_mut(), beacon.as_ref()) else {
        set_last_error("builder and beacon must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let entry = Beacon {
        data_rate: beacon.data_rate,
        frequency_hz: beacon.frequency_hz,
        ping_slot_frequency_hz: beacon.ping_slot_frequency_hz,
    };
    update(builder, |b| {
        b.beacon(entry).dwell_time_limit(has_dwell_time_limit != 0)
    })
}

/// Sets whether the plan's network creates channels, and the numbering a dynamic plan reads
/// a type 1 channel list against.
///
/// # Arguments
///
/// * `builder` - the builder to update.
/// * `kind` - [`PAMOJA_LORA_PLAN_KIND_DYNAMIC`] or [`PAMOJA_LORA_PLAN_KIND_FIXED`].
/// * `channel_list` - for a dynamic plan, one of the `PAMOJA_LORA_CHANNEL_LIST_*` constants;
///   ignored for a fixed one.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` is null or a code names nothing,
/// and [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_set_kind(
    builder: *mut PamojaLoraPlanBuilder,
    kind: u8,
    channel_list: u8,
) -> PamojaStatus {
    let Some(builder) = builder.as_mut() else {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let kind = match (kind, channel_list) {
        (PAMOJA_LORA_PLAN_KIND_FIXED, _) => PlanKind::Fixed,
        (PAMOJA_LORA_PLAN_KIND_DYNAMIC, PAMOJA_LORA_CHANNEL_LIST_NONE) => {
            PlanKind::Dynamic { channel_list: None }
        }
        (PAMOJA_LORA_PLAN_KIND_DYNAMIC, PAMOJA_LORA_CHANNEL_LIST_MHZ800) => PlanKind::Dynamic {
            channel_list: Some(FixedChannelList::Mhz800),
        },
        (PAMOJA_LORA_PLAN_KIND_DYNAMIC, PAMOJA_LORA_CHANNEL_LIST_MHZ900) => PlanKind::Dynamic {
            channel_list: Some(FixedChannelList::Mhz900),
        },
        _ => {
            set_last_error(format!(
                "{kind} with channel list {channel_list} is not a plan kind"
            ));
            return PamojaStatus::InvalidArgument;
        }
    };
    update(builder, |b| b.kind(kind))
}

/// Sets whether devices on the plan answer `TXParamSetupReq`.
///
/// # Arguments
///
/// * `builder` - the builder to update.
/// * `answered` - `1` if the command applies.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` is null, and
/// [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_set_tx_param_setup(
    builder: *mut PamojaLoraPlanBuilder,
    answered: u8,
) -> PamojaStatus {
    let Some(builder) = builder.as_mut() else {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    update(builder, |b| b.tx_param_setup(answered != 0))
}

/// Sets what each `ChMaskCntl` value does.
///
/// # Arguments
///
/// * `builder` - the builder to update.
/// * `controls` - the eight controls, indexed by value.
/// * `len` - how many `controls` points at, which must be 8.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, `len` is not 8, or a
/// control's kind names nothing, and [`PamojaStatus::Closed`] if the builder was already
/// built.
///
/// # Safety
///
/// `builder` must be a live builder handle and `controls` must point at `len` readable
/// [`PamojaLoraMaskControl`] values.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_set_mask_controls(
    builder: *mut PamojaLoraPlanBuilder,
    controls: *const PamojaLoraMaskControl,
    len: usize,
) -> PamojaStatus {
    let Some(builder) = builder.as_mut() else {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if controls.is_null() || len != 8 {
        set_last_error("controls must point at exactly eight controls".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let crossed = std::slice::from_raw_parts(controls, len);
    let mut table = [MaskControl::Reserved; 8];
    for (slot, control) in table.iter_mut().zip(crossed) {
        *slot = match mask_control_in(control) {
            Ok(control) => control,
            Err(status) => return status,
        };
    }
    update(builder, |b| b.mask_controls(table))
}

/// Sets the order a device tries the join channels in.
///
/// # Arguments
///
/// * `builder` - the builder to update.
/// * `sequence` - one of the `PAMOJA_LORA_JOIN_*` constants.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` is null or `sequence` names
/// nothing, and [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_set_join_sequence(
    builder: *mut PamojaLoraPlanBuilder,
    sequence: u8,
) -> PamojaStatus {
    let Some(builder) = builder.as_mut() else {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let sequence = match sequence {
        PAMOJA_LORA_JOIN_RANDOM => JoinSequence::Random,
        PAMOJA_LORA_JOIN_OCTET_PASSES => JoinSequence::OctetPasses,
        other => {
            set_last_error(format!("{other} is not a join sequence"));
            return PamojaStatus::InvalidArgument;
        }
    };
    update(builder, |b| b.join_sequence(sequence))
}

/// Sets what the plan's transmit power indexes count down from.
///
/// # Arguments
///
/// * `builder` - the builder to update.
/// * `reference` - [`PAMOJA_LORA_POWER_EIRP`] or [`PAMOJA_LORA_POWER_CONDUCTED`].
/// * `gain_allowance_db` - for a conducted ceiling, the antenna gain it allows for.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` is null or `reference` names
/// nothing, and [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_set_power_reference(
    builder: *mut PamojaLoraPlanBuilder,
    reference: u8,
    gain_allowance_db: u8,
) -> PamojaStatus {
    let Some(builder) = builder.as_mut() else {
        set_last_error("builder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let reference = match reference {
        PAMOJA_LORA_POWER_EIRP => PowerReference::Eirp,
        PAMOJA_LORA_POWER_CONDUCTED => PowerReference::Conducted { gain_allowance_db },
        other => {
            set_last_error(format!("{other} is not a power reference"));
            return PamojaStatus::InvalidArgument;
        }
    };
    update(builder, |b| b.power_reference(reference))
}

/// Finishes a plan and hands back a handle the query functions accept.
///
/// The builder is consumed whether the plan is accepted or rejected, so the
/// caller must not free or reuse it afterwards.
///
/// Tables left empty are filled in where a region would share them: an empty
/// downlink data-rate table reuses the uplink one, an empty downlink payload
/// table reuses the matching uplink one, and an empty back-off chain steps down
/// one data rate at a time. What cannot be guessed is checked instead, so a plan
/// that would answer a question wrongly is refused here rather than at the
/// question.
///
/// # Arguments
///
/// * `builder` - the builder to finish, which this call consumes.
/// * `out_plan` - set to the plan handle on success, and to null otherwise.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or the
/// plan is inconsistent, with the reason available from
/// `pamoja_last_error_message`, and
/// [`PamojaStatus::Closed`] if the builder was already built.
///
/// # Safety
///
/// `builder` must be a live builder handle from
/// [`pamoja_lora_plan_builder_new`], and `out_plan` must point at writable
/// storage for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_plan_builder_build(
    builder: *mut PamojaLoraPlanBuilder,
    out_plan: *mut *mut PamojaLoraPlan,
) -> PamojaStatus {
    if builder.is_null() || out_plan.is_null() {
        set_last_error("builder and out_plan must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_plan;
    *slot = std::ptr::null_mut();

    let Some(inner) = Box::from_raw(builder).builder else {
        set_last_error("this builder has already been built".to_owned());
        return PamojaStatus::Closed;
    };

    match inner.build() {
        Ok(plan) => {
            *slot = PamojaLoraPlan::into_handle(plan);
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::InvalidArgument
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;

    /// Builds a small but complete two-rate plan, the way a private deployment on
    /// licensed spectrum would.
    unsafe fn private_plan() -> *mut PamojaLoraPlan {
        let name = CString::new("private-915").expect("name");
        let mut builder = ptr::null_mut();
        assert_eq!(
            pamoja_lora_plan_builder_new(name.as_ptr(), &mut builder),
            PamojaStatus::Ok
        );

        for rate in [
            PamojaLoraDataRate {
                bitrate_bps: 250,
                bandwidth_hz: 125_000,
                kind: PAMOJA_LORA_MODULATION_LORA,
                spreading_factor: 12,
                coding_rate_numerator: 0,
                coding_rate_denominator: 0,
            },
            PamojaLoraDataRate {
                bitrate_bps: 5_470,
                bandwidth_hz: 125_000,
                kind: PAMOJA_LORA_MODULATION_LORA,
                spreading_factor: 7,
                coding_rate_numerator: 0,
                coding_rate_denominator: 0,
            },
        ] {
            assert_eq!(
                pamoja_lora_plan_builder_push_data_rate(
                    builder,
                    PAMOJA_LORA_DIRECTION_UPLINK,
                    &rate
                ),
                PamojaStatus::Ok
            );
        }

        for (mac, app) in [(59u16, 51u16), (230, 222)] {
            for table in [
                PAMOJA_LORA_PAYLOAD_TABLE_UPLINK_REPEATER,
                PAMOJA_LORA_PAYLOAD_TABLE_UPLINK_DIRECT,
            ] {
                assert_eq!(
                    pamoja_lora_plan_builder_push_max_payload(builder, table, 1, mac, app),
                    PamojaStatus::Ok
                );
            }
        }

        let block = PamojaLoraChannelBlock {
            start_hz: 915_000_000,
            step_hz: 500_000,
            count: 4,
            min_data_rate: 0,
            max_data_rate: 1,
        };
        assert_eq!(
            pamoja_lora_plan_builder_push_channel_block(
                builder,
                PAMOJA_LORA_CHANNELS_DEFAULT,
                &block
            ),
            PamojaStatus::Ok
        );
        assert_eq!(
            pamoja_lora_plan_builder_push_channel_block(builder, PAMOJA_LORA_CHANNELS_JOIN, &block),
            PamojaStatus::Ok
        );

        // Licensed spectrum: the holder may occupy the channel continuously.
        let band = PamojaLoraSubBand {
            start_hz: 915_000_000,
            end_hz: 917_000_000,
            duty_cycle_permille: 1000,
            max_eirp_dbm: 30,
        };
        assert_eq!(
            pamoja_lora_plan_builder_push_sub_band(builder, &band),
            PamojaStatus::Ok
        );

        assert_eq!(
            pamoja_lora_plan_builder_set_rx(builder, 915_000_000, 0, 0),
            PamojaStatus::Ok
        );
        for row in [[0u8], [1u8]] {
            assert_eq!(
                pamoja_lora_plan_builder_push_rx1_row(builder, 0, row.as_ptr(), row.len()),
                PamojaStatus::Ok
            );
        }
        assert_eq!(
            pamoja_lora_plan_builder_set_power(builder, 30, 2, 7),
            PamojaStatus::Ok
        );

        let mut plan = ptr::null_mut();
        assert_eq!(
            pamoja_lora_plan_builder_build(builder, &mut plan),
            PamojaStatus::Ok
        );
        assert!(!plan.is_null());
        plan
    }

    #[test]
    #[cfg(feature = "eu868")]
    fn a_published_region_reports_its_own_tables() {
        unsafe {
            let mut plan = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_for_region(PAMOJA_LORA_REGION_EU868, &mut plan),
                PamojaStatus::Ok
            );

            let name = pamoja_lora_plan_name(plan);
            let text = std::ffi::CStr::from_ptr(crate::pamoja_string_data(name));
            assert_eq!(text.to_str().expect("utf-8"), "EU863-870");
            crate::pamoja_string_free(name);

            let mut link = PamojaLoraLink {
                bandwidth_hz: 0,
                preamble_symbols: 0,
                spreading_factor: 0,
                coding_rate_denominator: 0,
                explicit_header: 0,
                crc: 0,
            };
            assert_eq!(
                pamoja_lora_plan_link_settings(plan, 0, &mut link),
                PamojaStatus::Ok
            );
            assert_eq!(link.spreading_factor, 12);
            assert_eq!(link.bandwidth_hz, 125_000);

            let mut permille = 0;
            assert_eq!(
                pamoja_lora_plan_duty_cycle_permille(plan, 868_100_000, &mut permille),
                PamojaStatus::Ok
            );
            assert_eq!(permille, 10, "the 868.1 MHz sub-band is limited to 1%");

            pamoja_lora_plan_free(plan);
        }
    }

    #[test]
    fn an_unknown_region_code_is_told_from_one_left_out_of_the_build() {
        unsafe {
            let mut plan = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_for_region(4242, &mut plan),
                PamojaStatus::InvalidArgument
            );
            assert!(plan.is_null());
        }

        // Every code in the range is a real region, so it is never an invalid
        // argument, whatever this build happens to carry.
        for region in PAMOJA_LORA_REGION_EU868..=PAMOJA_LORA_REGION_RU864 {
            let mut plan = ptr::null_mut();
            let status = unsafe { pamoja_lora_plan_for_region(region, &mut plan) };
            assert!(
                status == PamojaStatus::Ok || status == PamojaStatus::Unsupported,
                "region {region} reported {status:?}"
            );
            assert_eq!(
                status == PamojaStatus::Ok,
                pamoja_lora_region_is_available(region) == 1
            );
            unsafe { pamoja_lora_plan_free(plan) };
        }
    }

    #[test]
    fn a_private_plan_answers_the_same_questions_a_published_one_does() {
        unsafe {
            let plan = private_plan();

            let name = pamoja_lora_plan_name(plan);
            let text = std::ffi::CStr::from_ptr(crate::pamoja_string_data(name));
            assert_eq!(text.to_str().expect("utf-8"), "private-915");
            crate::pamoja_string_free(name);

            let mut info = PamojaLoraPlanInfo {
                rx2_frequency_hz: 0,
                uplink_data_rate_count: 0,
                downlink_data_rate_count: 0,
                default_channel_count: 0,
                join_channel_block_count: 0,
                default_channel_block_count: 0,
                sub_band_count: 0,
                beacon: PamojaLoraBeacon {
                    frequency_hz: 0,
                    ping_slot_frequency_hz: 0,
                    data_rate: 0,
                },
                rx2_data_rate: 0,
                default_max_eirp_dbm: 0,
                tx_power_step_db: 0,
                max_tx_power_index: 0,
                max_rx1_data_rate_offset: 0,
                has_dwell_time_limit: 1,
                has_dwell_limited_payloads: 1,
                has_dwell_limited_rx1: 1,
            };
            assert_eq!(pamoja_lora_plan_info(plan, &mut info), PamojaStatus::Ok);
            assert_eq!(info.default_channel_count, 4);
            assert_eq!(info.uplink_data_rate_count, 2);
            // An empty downlink table falls back to the uplink one.
            assert_eq!(info.downlink_data_rate_count, 2);
            assert_eq!(info.has_dwell_time_limit, 0);
            assert_eq!(info.has_dwell_limited_payloads, 0);
            assert_eq!(info.has_dwell_limited_rx1, 0);

            let mut frequency = 0;
            assert_eq!(
                pamoja_lora_plan_channel_frequency_hz(plan, 3, &mut frequency),
                PamojaStatus::Ok
            );
            assert_eq!(frequency, 916_500_000);

            let mut permille = 0;
            assert_eq!(
                pamoja_lora_plan_duty_cycle_permille(plan, 915_000_000, &mut permille),
                PamojaStatus::Ok
            );
            assert_eq!(permille, 1000, "licensed spectrum is unrestricted");

            let mut payload = PamojaLoraMaxPayload {
                mac_payload: 0,
                application: 0,
            };
            assert_eq!(
                pamoja_lora_plan_max_payload(
                    plan,
                    PAMOJA_LORA_PAYLOAD_TABLE_DOWNLINK_DIRECT,
                    1,
                    &mut payload
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                payload.application, 222,
                "the downlink table mirrors uplink"
            );

            let mut dbm = 0;
            assert_eq!(
                pamoja_lora_plan_max_eirp_dbm(plan, 915_000_000, &mut dbm),
                PamojaStatus::Ok
            );
            assert_eq!(dbm, 30);

            let mut lower = 0;
            assert_eq!(
                pamoja_lora_plan_next_backoff_data_rate(plan, 1, &mut lower),
                PamojaStatus::Ok
            );
            assert_eq!(lower, 0, "an unset chain steps down one rate at a time");
            assert_eq!(
                pamoja_lora_plan_next_backoff_data_rate(plan, 0, &mut lower),
                PamojaStatus::Unsupported,
                "the slowest rate has nothing below it"
            );

            pamoja_lora_plan_free(plan);
        }
    }

    #[test]
    fn a_plan_whose_rx1_rows_are_too_narrow_is_refused() {
        unsafe {
            let name = CString::new("too-narrow").expect("name");
            let mut builder = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_builder_new(name.as_ptr(), &mut builder),
                PamojaStatus::Ok
            );
            let rate = PamojaLoraDataRate {
                bitrate_bps: 250,
                bandwidth_hz: 125_000,
                kind: PAMOJA_LORA_MODULATION_LORA,
                spreading_factor: 12,
                coding_rate_numerator: 0,
                coding_rate_denominator: 0,
            };
            assert_eq!(
                pamoja_lora_plan_builder_push_data_rate(
                    builder,
                    PAMOJA_LORA_DIRECTION_UPLINK,
                    &rate
                ),
                PamojaStatus::Ok
            );
            // Offsets up to 5 need six entries in every row; this row has one.
            assert_eq!(
                pamoja_lora_plan_builder_set_rx(builder, 915_000_000, 0, 5),
                PamojaStatus::Ok
            );
            let row = [0u8];
            assert_eq!(
                pamoja_lora_plan_builder_push_rx1_row(builder, 0, row.as_ptr(), row.len()),
                PamojaStatus::Ok
            );

            let mut plan = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_builder_build(builder, &mut plan),
                PamojaStatus::InvalidArgument
            );
            assert!(plan.is_null());
        }
    }

    #[test]
    fn a_plan_that_listens_at_a_data_rate_it_lacks_is_refused() {
        unsafe {
            let name = CString::new("bad-rx2").expect("name");
            let mut builder = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_builder_new(name.as_ptr(), &mut builder),
                PamojaStatus::Ok
            );
            let rate = PamojaLoraDataRate {
                bitrate_bps: 250,
                bandwidth_hz: 125_000,
                kind: PAMOJA_LORA_MODULATION_LORA,
                spreading_factor: 12,
                coding_rate_numerator: 0,
                coding_rate_denominator: 0,
            };
            assert_eq!(
                pamoja_lora_plan_builder_push_data_rate(
                    builder,
                    PAMOJA_LORA_DIRECTION_UPLINK,
                    &rate
                ),
                PamojaStatus::Ok
            );
            // The plan defines DR0 alone, so listening at DR3 could never work.
            assert_eq!(
                pamoja_lora_plan_builder_set_rx(builder, 915_000_000, 3, 0),
                PamojaStatus::Ok
            );
            let row = [0u8];
            assert_eq!(
                pamoja_lora_plan_builder_push_rx1_row(builder, 0, row.as_ptr(), row.len()),
                PamojaStatus::Ok
            );

            let mut plan = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_builder_build(builder, &mut plan),
                PamojaStatus::InvalidArgument
            );
            assert!(plan.is_null());
        }
    }

    #[test]
    #[cfg(all(feature = "us915", feature = "cn470", feature = "eu868"))]
    fn the_rules_a_published_plan_follows_cross_the_boundary() {
        unsafe {
            let empty_rules = PamojaLoraPlanRules {
                kind: 9,
                channel_list: 9,
                tx_param_setup: 9,
                join_sequence: 9,
                power_reference: 9,
                gain_allowance_db: 9,
                downlink_channel_block_count: 9,
                join_plan_count: 9,
            };

            let mut plan = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_for_region(PAMOJA_LORA_REGION_US915, &mut plan),
                PamojaStatus::Ok
            );
            let mut rules = empty_rules;
            assert_eq!(pamoja_lora_plan_rules(plan, &mut rules), PamojaStatus::Ok);
            assert_eq!(rules.kind, PAMOJA_LORA_PLAN_KIND_FIXED);
            assert_eq!(rules.join_sequence, PAMOJA_LORA_JOIN_OCTET_PASSES);
            assert_eq!(
                (rules.power_reference, rules.gain_allowance_db),
                (PAMOJA_LORA_POWER_CONDUCTED, 6)
            );
            assert_eq!(
                (rules.downlink_channel_block_count, rules.join_plan_count),
                (1, 0)
            );

            let mut control = mask_control_out(MaskControl::Reserved);
            assert_eq!(
                pamoja_lora_plan_mask_control(plan, 5, &mut control),
                PamojaStatus::Ok
            );
            assert_eq!(control.kind, PAMOJA_LORA_MASK_PAIRED_BANKS);
            assert_eq!(
                pamoja_lora_plan_mask_control(plan, 7, &mut control),
                PamojaStatus::Ok
            );
            assert_eq!(
                (
                    control.kind,
                    control.on,
                    control.has_then_group,
                    control.then_group
                ),
                (PAMOJA_LORA_MASK_ALL, 0, 1, 4)
            );
            assert_eq!(
                pamoja_lora_plan_mask_control(plan, 8, &mut control),
                PamojaStatus::InvalidArgument
            );

            let mut hz = 0;
            assert_eq!(
                pamoja_lora_plan_rx1_frequency_hz(plan, 65, 904_600_000, &mut hz),
                PamojaStatus::Ok
            );
            assert_eq!(hz, 923_900_000);
            assert_eq!(
                pamoja_lora_plan_downlink_channel_frequency_hz(plan, 7, &mut hz),
                PamojaStatus::Ok
            );
            assert_eq!(hz, 927_500_000);
            pamoja_lora_plan_free(plan);

            let mut plan = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_for_region(PAMOJA_LORA_REGION_EU868, &mut plan),
                PamojaStatus::Ok
            );
            let mut rules = empty_rules;
            assert_eq!(pamoja_lora_plan_rules(plan, &mut rules), PamojaStatus::Ok);
            assert_eq!(
                (rules.kind, rules.channel_list),
                (
                    PAMOJA_LORA_PLAN_KIND_DYNAMIC,
                    PAMOJA_LORA_CHANNEL_LIST_MHZ800
                )
            );
            assert_eq!(
                pamoja_lora_plan_rx1_frequency_hz(plan, 2, 868_500_000, &mut hz),
                PamojaStatus::Ok
            );
            assert_eq!(hz, 868_500_000, "a dynamic plan answers where it sent");
            pamoja_lora_plan_free(plan);

            // RP002-1.0.5 table 49: common join channels 8 and 9 select the 20 MHz
            // antenna's plan B, answered on their own frequency.
            let mut plan = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_for_cn470(PAMOJA_LORA_CN470_ANTENNA_26MHZ_A, &mut plan),
                PamojaStatus::Ok
            );
            let mut rules = empty_rules;
            assert_eq!(pamoja_lora_plan_rules(plan, &mut rules), PamojaStatus::Ok);
            assert_eq!(rules.join_plan_count, 5);
            let mut run = PamojaLoraJoinPlan {
                channels: PamojaLoraChannelBlock {
                    start_hz: 0,
                    step_hz: 0,
                    count: 0,
                    min_data_rate: 0,
                    max_data_rate: 0,
                },
                accept_start_hz: 0,
                accept_step_hz: 0,
                rx2_start_hz: 0,
                rx2_step_hz: 0,
                cn470_plan: 0,
            };
            assert_eq!(
                pamoja_lora_plan_join_plan(plan, 2, &mut run),
                PamojaStatus::Ok
            );
            assert_eq!(
                (
                    run.channels.start_hz,
                    run.channels.step_hz,
                    run.channels.count
                ),
                (479_900_000, 20_000_000, 2)
            );
            assert_eq!(
                (run.accept_start_hz, run.rx2_start_hz, run.rx2_step_hz),
                (479_900_000, 478_300_000, 20_000_000)
            );
            assert_eq!(run.cn470_plan, PAMOJA_LORA_CN470_ANTENNA_20MHZ_B);
            assert_eq!(
                pamoja_lora_plan_join_plan(plan, 5, &mut run),
                PamojaStatus::InvalidArgument
            );
            let (mut index, mut offset) = (0, 0);
            assert_eq!(
                pamoja_lora_plan_join_plan_for_channel(plan, 9, &mut index, &mut offset),
                PamojaStatus::Ok
            );
            assert_eq!((index, offset), (2, 1));
            assert_eq!(
                pamoja_lora_plan_join_plan_for_channel(plan, 20, &mut index, &mut offset),
                PamojaStatus::InvalidArgument
            );
            pamoja_lora_plan_free(plan);

            assert_eq!(
                pamoja_lora_plan_for_cn470(PAMOJA_LORA_CN470_NONE, &mut plan),
                PamojaStatus::InvalidArgument
            );
        }
    }

    #[test]
    fn a_built_plan_takes_every_rule_the_builder_is_given() {
        unsafe {
            let name = CString::new("fixed-private").expect("name");
            let mut builder = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_builder_new(name.as_ptr(), &mut builder),
                PamojaStatus::Ok
            );
            let rate = PamojaLoraDataRate {
                bitrate_bps: 5_470,
                bandwidth_hz: 125_000,
                kind: PAMOJA_LORA_MODULATION_LORA,
                spreading_factor: 7,
                coding_rate_numerator: 0,
                coding_rate_denominator: 0,
            };
            assert_eq!(
                pamoja_lora_plan_builder_push_data_rate(
                    builder,
                    PAMOJA_LORA_DIRECTION_UPLINK,
                    &rate
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lora_plan_builder_set_rx(builder, 923_300_000, 0, 0),
                PamojaStatus::Ok
            );
            let row = [0u8];
            assert_eq!(
                pamoja_lora_plan_builder_push_rx1_row(builder, 0, row.as_ptr(), row.len()),
                PamojaStatus::Ok
            );
            for (which, start) in [
                (PAMOJA_LORA_CHANNELS_DEFAULT, 902_300_000),
                (PAMOJA_LORA_CHANNELS_DOWNLINK, 923_300_000),
            ] {
                let block = PamojaLoraChannelBlock {
                    start_hz: start,
                    step_hz: 600_000,
                    count: 4,
                    min_data_rate: 0,
                    max_data_rate: 0,
                };
                assert_eq!(
                    pamoja_lora_plan_builder_push_channel_block(builder, which, &block),
                    PamojaStatus::Ok
                );
            }
            assert_eq!(
                pamoja_lora_plan_builder_set_kind(builder, PAMOJA_LORA_PLAN_KIND_FIXED, 0),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lora_plan_builder_set_tx_param_setup(builder, 1),
                PamojaStatus::Ok
            );
            let controls = [mask_control_out(MaskControl::All {
                on: true,
                then_group: Some(1),
            }); 8];
            assert_eq!(
                pamoja_lora_plan_builder_set_mask_controls(builder, controls.as_ptr(), 8),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lora_plan_builder_set_mask_controls(builder, controls.as_ptr(), 7),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_lora_plan_builder_set_join_sequence(builder, PAMOJA_LORA_JOIN_OCTET_PASSES),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lora_plan_builder_set_power_reference(
                    builder,
                    PAMOJA_LORA_POWER_CONDUCTED,
                    3
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lora_plan_builder_set_kind(builder, PAMOJA_LORA_PLAN_KIND_DYNAMIC, 9),
                PamojaStatus::InvalidArgument
            );

            let mut plan = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_builder_build(builder, &mut plan),
                PamojaStatus::Ok
            );
            let mut rules = PamojaLoraPlanRules {
                kind: 0,
                channel_list: 0,
                tx_param_setup: 0,
                join_sequence: 0,
                power_reference: 0,
                gain_allowance_db: 0,
                downlink_channel_block_count: 0,
                join_plan_count: 9,
            };
            assert_eq!(pamoja_lora_plan_rules(plan, &mut rules), PamojaStatus::Ok);
            assert_eq!(
                rules,
                PamojaLoraPlanRules {
                    kind: PAMOJA_LORA_PLAN_KIND_FIXED,
                    channel_list: PAMOJA_LORA_CHANNEL_LIST_NONE,
                    tx_param_setup: 1,
                    join_sequence: PAMOJA_LORA_JOIN_OCTET_PASSES,
                    power_reference: PAMOJA_LORA_POWER_CONDUCTED,
                    gain_allowance_db: 3,
                    downlink_channel_block_count: 1,
                    join_plan_count: 0,
                }
            );
            let mut hz = 0;
            assert_eq!(
                pamoja_lora_plan_rx1_frequency_hz(plan, 5, 905_300_000, &mut hz),
                PamojaStatus::Ok
            );
            assert_eq!(
                hz, 923_900_000,
                "uplink channel 5 answers on downlink channel 1"
            );
            let mut control = mask_control_out(MaskControl::Reserved);
            assert_eq!(
                pamoja_lora_plan_mask_control(plan, 3, &mut control),
                PamojaStatus::Ok
            );
            assert_eq!(
                (
                    control.kind,
                    control.on,
                    control.has_then_group,
                    control.then_group
                ),
                (PAMOJA_LORA_MASK_ALL, 1, 1, 1)
            );
            pamoja_lora_plan_free(plan);
        }
    }

    #[test]
    fn a_reserved_data_rate_crosses_and_comes_back_reserved() {
        let reserved = PamojaLoraDataRate {
            bitrate_bps: 0,
            bandwidth_hz: 0,
            kind: PAMOJA_LORA_MODULATION_RESERVED,
            spreading_factor: 0,
            coding_rate_numerator: 0,
            coding_rate_denominator: 0,
        };
        assert_eq!(data_rate_in(&reserved).expect("reserved"), None);
        assert_eq!(data_rate_out(None), reserved);
    }

    #[test]
    fn every_modulation_survives_the_round_trip() {
        for rate in [
            DataRate::lora(9, 125_000, 1_760),
            DataRate::fsk(50_000),
            DataRate::lr_fhss(1, 3, 137_000, 162),
        ] {
            let crossed = data_rate_out(Some(rate));
            let back = data_rate_in(&crossed).expect("valid").expect("present");
            assert_eq!(back, rate);
        }
    }
}
