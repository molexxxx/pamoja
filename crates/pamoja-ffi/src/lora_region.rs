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

use pamoja_lora::region::{
    Beacon, ChannelBlock, ChannelPlan, DataRate, MaxPayload, Modulation, SubBand,
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
    /// The first channel's centre frequency in hertz.
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

/// A regional channel plan, published or private.
///
/// A handle the caller must release with [`pamoja_lora_plan_free`].
pub struct PamojaLoraPlan {
    kind: PlanKind,
}

enum PlanKind {
    Published(&'static ChannelPlan<'static>),
    Owned(Box<OwnedPlan>),
}

/// A plan whose tables are owned here, because they were built at runtime rather
/// than published as constants.
struct OwnedPlan {
    name: String,
    uplink_data_rates: Vec<Option<DataRate>>,
    downlink_data_rates: Vec<Option<DataRate>>,
    max_payload_repeater: Vec<Option<MaxPayload>>,
    max_payload_direct: Vec<Option<MaxPayload>>,
    downlink_max_payload_repeater: Vec<Option<MaxPayload>>,
    downlink_max_payload_direct: Vec<Option<MaxPayload>>,
    max_payload_dwell_limited: Option<Vec<Option<MaxPayload>>>,
    join_channels: Vec<ChannelBlock>,
    default_channels: Vec<ChannelBlock>,
    sub_bands: Vec<SubBand>,
    default_max_eirp_dbm: i8,
    tx_power_step_db: u8,
    max_tx_power_index: u8,
    rx1_rows: Vec<Vec<u8>>,
    rx1_rows_dwell_limited: Option<Vec<Vec<u8>>>,
    max_rx1_data_rate_offset: u8,
    rx2_frequency_hz: u32,
    rx2_data_rate: u8,
    data_rate_backoff: Vec<Option<u8>>,
    beacon: Beacon,
    has_dwell_time_limit: bool,
}

impl OwnedPlan {
    /// Lends the owned tables to a borrowed plan for the duration of one query.
    ///
    /// The row pointers a plan needs are assembled on the stack here, so nothing
    /// outlives the call and the storage stays owned by this handle.
    ///
    /// # Arguments
    ///
    /// * `f` - the query to run against the assembled plan.
    ///
    /// # Returns
    ///
    /// Whatever the query returned.
    fn with<R>(&self, f: impl FnOnce(&ChannelPlan<'_>) -> R) -> R {
        let rx1: Vec<&[u8]> = self.rx1_rows.iter().map(Vec::as_slice).collect();
        let dwell_rx1: Option<Vec<&[u8]>> = self
            .rx1_rows_dwell_limited
            .as_ref()
            .map(|rows| rows.iter().map(Vec::as_slice).collect());
        let plan = ChannelPlan {
            name: &self.name,
            uplink_data_rates: &self.uplink_data_rates,
            downlink_data_rates: &self.downlink_data_rates,
            max_payload_repeater: &self.max_payload_repeater,
            max_payload_direct: &self.max_payload_direct,
            downlink_max_payload_repeater: &self.downlink_max_payload_repeater,
            downlink_max_payload_direct: &self.downlink_max_payload_direct,
            max_payload_dwell_limited: self.max_payload_dwell_limited.as_deref(),
            join_channels: &self.join_channels,
            default_channels: &self.default_channels,
            sub_bands: &self.sub_bands,
            default_max_eirp_dbm: self.default_max_eirp_dbm,
            tx_power_step_db: self.tx_power_step_db,
            max_tx_power_index: self.max_tx_power_index,
            rx1_data_rate_offsets: &rx1,
            rx1_data_rate_offsets_dwell_limited: dwell_rx1.as_deref(),
            max_rx1_data_rate_offset: self.max_rx1_data_rate_offset,
            rx2_frequency_hz: self.rx2_frequency_hz,
            rx2_data_rate: self.rx2_data_rate,
            data_rate_backoff: &self.data_rate_backoff,
            beacon: self.beacon,
            has_dwell_time_limit: self.has_dwell_time_limit,
        };
        f(&plan)
    }
}

impl PamojaLoraPlan {
    /// Runs a query against the plan, whichever kind it is.
    ///
    /// # Arguments
    ///
    /// * `f` - the query to run.
    ///
    /// # Returns
    ///
    /// Whatever the query returned.
    fn with<R>(&self, f: impl FnOnce(&ChannelPlan<'_>) -> R) -> R {
        match &self.kind {
            PlanKind::Published(plan) => f(plan),
            PlanKind::Owned(owned) => owned.with(f),
        }
    }

    /// Moves a plan onto the heap and hands the caller its handle.
    ///
    /// # Arguments
    ///
    /// * `kind` - the published or owned plan to wrap.
    ///
    /// # Returns
    ///
    /// A handle the caller must release with [`pamoja_lora_plan_free`].
    fn into_handle(kind: PlanKind) -> *mut Self {
        Box::into_raw(Box::new(Self { kind }))
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
            *slot = PamojaLoraPlan::into_handle(PlanKind::Published(plan));
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
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
/// [`pamoja_lora_airtime_us`](crate::pamoja_lora_airtime_us).
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
/// [`pamoja_lora_min_off_time_us`](crate::pamoja_lora_min_off_time_us) to turn the
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

/// Returns the centre frequency of one of the plan's default channels.
///
/// # Arguments
///
/// * `plan` - the plan to read.
/// * `channel` - the channel number, counting across the default blocks in order.
/// * `out_frequency_hz` - set to the centre frequency on success.
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
/// * `which` - [`PAMOJA_LORA_CHANNELS_JOIN`] or [`PAMOJA_LORA_CHANNELS_DEFAULT`].
/// * `index` - the block's position, below the count
///   [`pamoja_lora_plan_info`] reports.
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
    if which != PAMOJA_LORA_CHANNELS_JOIN && which != PAMOJA_LORA_CHANNELS_DEFAULT {
        set_last_error(format!("{which} is not a channel set"));
        return PamojaStatus::InvalidArgument;
    }
    let found = plan.with(|plan| {
        let blocks = if which == PAMOJA_LORA_CHANNELS_JOIN {
            plan.join_channels
        } else {
            plan.default_channels
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
    plan: OwnedPlan,
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
        plan: OwnedPlan {
            name: name.to_owned(),
            uplink_data_rates: Vec::new(),
            downlink_data_rates: Vec::new(),
            max_payload_repeater: Vec::new(),
            max_payload_direct: Vec::new(),
            downlink_max_payload_repeater: Vec::new(),
            downlink_max_payload_direct: Vec::new(),
            max_payload_dwell_limited: None,
            join_channels: Vec::new(),
            default_channels: Vec::new(),
            sub_bands: Vec::new(),
            default_max_eirp_dbm: 16,
            tx_power_step_db: 2,
            max_tx_power_index: 7,
            rx1_rows: Vec::new(),
            rx1_rows_dwell_limited: None,
            max_rx1_data_rate_offset: 0,
            rx2_frequency_hz: 0,
            rx2_data_rate: 0,
            data_rate_backoff: Vec::new(),
            beacon: Beacon {
                data_rate: 0,
                frequency_hz: 0,
                ping_slot_frequency_hz: 0,
            },
            has_dwell_time_limit: false,
        },
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
/// ABI defines.
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
        PAMOJA_LORA_DIRECTION_UPLINK => builder.plan.uplink_data_rates.push(rate),
        PAMOJA_LORA_DIRECTION_DOWNLINK => builder.plan.downlink_data_rates.push(rate),
        other => {
            set_last_error(format!("{other} is not a direction"));
            return PamojaStatus::InvalidArgument;
        }
    }
    PamojaStatus::Ok
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
/// not one of the constants.
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
    let entry = (present != 0).then(|| MaxPayload::new(mac_payload, application));
    match table {
        PAMOJA_LORA_PAYLOAD_TABLE_UPLINK_REPEATER => builder.plan.max_payload_repeater.push(entry),
        PAMOJA_LORA_PAYLOAD_TABLE_UPLINK_DIRECT => builder.plan.max_payload_direct.push(entry),
        PAMOJA_LORA_PAYLOAD_TABLE_DOWNLINK_REPEATER => {
            builder.plan.downlink_max_payload_repeater.push(entry)
        }
        PAMOJA_LORA_PAYLOAD_TABLE_DOWNLINK_DIRECT => {
            builder.plan.downlink_max_payload_direct.push(entry)
        }
        PAMOJA_LORA_PAYLOAD_TABLE_DWELL_LIMITED => builder
            .plan
            .max_payload_dwell_limited
            .get_or_insert_with(Vec::new)
            .push(entry),
        other => {
            set_last_error(format!("{other} is not a payload table"));
            return PamojaStatus::InvalidArgument;
        }
    }
    PamojaStatus::Ok
}

/// Appends a run of evenly spaced channels to the plan.
///
/// # Arguments
///
/// * `builder` - the builder to extend.
/// * `which` - [`PAMOJA_LORA_CHANNELS_JOIN`] or [`PAMOJA_LORA_CHANNELS_DEFAULT`].
/// * `block` - the channel block to append.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null or `which`
/// is not one of the constants.
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
        PAMOJA_LORA_CHANNELS_JOIN => builder.plan.join_channels.push(entry),
        PAMOJA_LORA_CHANNELS_DEFAULT => builder.plan.default_channels.push(entry),
        other => {
            set_last_error(format!("{other} is not a channel set"));
            return PamojaStatus::InvalidArgument;
        }
    }
    PamojaStatus::Ok
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
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
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
    builder.plan.sub_bands.push(SubBand::new(
        band.start_hz,
        band.end_hz,
        band.duty_cycle_permille,
        band.max_eirp_dbm,
    ));
    PamojaStatus::Ok
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
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` or `offsets` is null.
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
        builder.plan.rx1_rows.push(row);
    } else {
        builder
            .plan
            .rx1_rows_dwell_limited
            .get_or_insert_with(Vec::new)
            .push(row);
    }
    PamojaStatus::Ok
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
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` is null.
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
    builder
        .plan
        .data_rate_backoff
        .push((has_lower != 0).then_some(data_rate));
    PamojaStatus::Ok
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
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` is null.
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
    builder.plan.default_max_eirp_dbm = default_max_eirp_dbm;
    builder.plan.tx_power_step_db = tx_power_step_db;
    builder.plan.max_tx_power_index = max_tx_power_index;
    PamojaStatus::Ok
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
/// Returns [`PamojaStatus::InvalidArgument`] if `builder` is null.
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
    builder.plan.rx2_frequency_hz = rx2_frequency_hz;
    builder.plan.rx2_data_rate = rx2_data_rate;
    builder.plan.max_rx1_data_rate_offset = max_rx1_data_rate_offset;
    PamojaStatus::Ok
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
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
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
    builder.plan.beacon = Beacon {
        data_rate: beacon.data_rate,
        frequency_hz: beacon.frequency_hz,
        ping_slot_frequency_hz: beacon.ping_slot_frequency_hz,
    };
    builder.plan.has_dwell_time_limit = has_dwell_time_limit != 0;
    PamojaStatus::Ok
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
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null, the plan
/// has no data rates, a payload table's length disagrees with the data-rate
/// table it indexes, an RX1 row is not as wide as the plan's highest offset
/// allows, or there is not one RX1 row per uplink data rate.
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

    let mut plan = Box::from_raw(builder).plan;

    if plan.downlink_data_rates.is_empty() {
        plan.downlink_data_rates = plan.uplink_data_rates.clone();
    }
    if plan.downlink_max_payload_repeater.is_empty() {
        plan.downlink_max_payload_repeater = plan.max_payload_repeater.clone();
    }
    if plan.downlink_max_payload_direct.is_empty() {
        plan.downlink_max_payload_direct = plan.max_payload_direct.clone();
    }
    if plan.data_rate_backoff.is_empty() {
        plan.data_rate_backoff = (0..plan.uplink_data_rates.len())
            .map(|index| index.checked_sub(1).map(|lower| lower as u8))
            .collect();
    }

    if let Err(message) = check(&plan) {
        set_last_error(message);
        return PamojaStatus::InvalidArgument;
    }

    *slot = PamojaLoraPlan::into_handle(PlanKind::Owned(Box::new(plan)));
    PamojaStatus::Ok
}

/// Checks that a built plan can answer every question asked of it.
///
/// # Arguments
///
/// * `plan` - the assembled plan.
///
/// # Returns
///
/// `Ok(())` if the plan is consistent.
///
/// # Errors
///
/// Returns the reason the plan would answer a question wrongly.
fn check(plan: &OwnedPlan) -> Result<(), String> {
    let rates = plan.uplink_data_rates.len();
    if rates == 0 {
        return Err("a plan needs at least one data rate".to_owned());
    }

    let width = usize::from(plan.max_rx1_data_rate_offset) + 1;
    if plan.rx1_rows.len() != rates {
        return Err(format!(
            "a plan needs one RX1 row per uplink data rate: {rates} data rates, {} rows",
            plan.rx1_rows.len()
        ));
    }
    for (index, row) in plan.rx1_rows.iter().enumerate() {
        if row.len() != width {
            return Err(format!(
                "RX1 row {index} has {} entries, but offsets up to {} means {width}",
                row.len(),
                plan.max_rx1_data_rate_offset
            ));
        }
    }
    if let Some(rows) = &plan.rx1_rows_dwell_limited {
        if rows.len() != rates {
            return Err(format!(
                "a plan needs one dwell-limited RX1 row per uplink data rate: {rates} data rates, {} rows",
                rows.len()
            ));
        }
        for (index, row) in rows.iter().enumerate() {
            if row.len() != width {
                return Err(format!(
                    "dwell-limited RX1 row {index} has {} entries, but offsets up to {} means {width}",
                    row.len(),
                    plan.max_rx1_data_rate_offset
                ));
            }
        }
    }

    if plan.data_rate_backoff.len() != rates {
        return Err(format!(
            "the back-off chain needs one entry per uplink data rate: {rates} data rates, {} entries",
            plan.data_rate_backoff.len()
        ));
    }

    for (name, table) in [
        ("the uplink repeater", &plan.max_payload_repeater),
        ("the uplink direct", &plan.max_payload_direct),
    ] {
        if !table.is_empty() && table.len() != rates {
            return Err(format!(
                "{name} payload table has {} entries for {rates} data rates",
                table.len()
            ));
        }
    }

    let downlink_rates = plan.downlink_data_rates.len();
    for (name, table) in [
        ("the downlink repeater", &plan.downlink_max_payload_repeater),
        ("the downlink direct", &plan.downlink_max_payload_direct),
    ] {
        if !table.is_empty() && table.len() != downlink_rates {
            return Err(format!(
                "{name} payload table has {} entries for {downlink_rates} downlink data rates",
                table.len()
            ));
        }
    }

    if let Some(table) = &plan.max_payload_dwell_limited {
        if table.len() != rates {
            return Err(format!(
                "the dwell-limited payload table has {} entries for {rates} data rates",
                table.len()
            ));
        }
    }

    if usize::from(plan.rx2_data_rate) >= downlink_rates {
        return Err(format!(
            "RX2 listens at data rate {}, which this plan does not define",
            plan.rx2_data_rate
        ));
    }

    Ok(())
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
