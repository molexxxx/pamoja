//! Generated Python bindings for LoRaWAN regional channel plans.
//!
//! These mirror the `pamoja_lora::region` Rust API: the facts a regulator and the
//! LoRa Alliance publish about one band, and the arithmetic that costs a
//! transmission out against them. Which data rates exist, what each carries, how
//! much of the time a node may hold a frequency, what it may radiate, where it
//! listens for a downlink.
//!
//! Nothing here refuses a transmission. A deployment may hold licensed spectrum
//! or be working under emergency provisions, and only the operator knows which,
//! so the plan reports and the caller decides.
//!
//! A plan built from parts is the same kind of thing as a published region, not a
//! lesser one, so [`ChannelPlanBuilder`] produces a [`ChannelPlan`] that answers
//! every question `ChannelPlan.for_region` does.

use std::sync::Mutex;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use pamoja_lora::region::{
    Beacon as CoreBeacon, ChannelBlock as CoreBlock, ChannelPlan as CorePlan,
    ChannelPlanBuilder as CoreBuilder, DataRate as CoreDataRate, MaxPayload as CoreMaxPayload,
    Modulation, OwnedChannelPlan, PayloadTable, Region, SubBand as CoreSubBand,
};

use crate::lora::LoraLink;

/// Resolves a region name to its published plan.
///
/// # Arguments
///
/// * `name` - the band's short code, such as `EU868`, or its specification name.
///
/// # Returns
///
/// The published plan.
///
/// # Errors
///
/// Returns a `ValueError` if no published region goes by that name.
fn region_plan(name: &str) -> PyResult<&'static CorePlan<'static>> {
    match Region::from_code(name) {
        Some(region) => Ok(region.plan()),
        None => {
            let known: Vec<&str> = Region::all().iter().map(|r| r.code()).collect();
            Err(PyValueError::new_err(format!(
                "{name} is not a published region; expected one of {}",
                known.join(", ")
            )))
        }
    }
}

/// Resolves a direction name.
fn direction_is_uplink(direction: &str) -> PyResult<bool> {
    match direction.to_ascii_lowercase().as_str() {
        "uplink" | "up" => Ok(true),
        "downlink" | "down" => Ok(false),
        other => Err(PyValueError::new_err(format!(
            "{other} is not a direction; expected uplink or downlink"
        ))),
    }
}

/// Resolves a payload-table name.
fn payload_table(table: &str) -> PyResult<PayloadTable> {
    match table.to_ascii_lowercase().as_str() {
        "uplink_repeater" => Ok(PayloadTable::UplinkRepeater),
        "uplink_direct" => Ok(PayloadTable::UplinkDirect),
        "downlink_repeater" => Ok(PayloadTable::DownlinkRepeater),
        "downlink_direct" => Ok(PayloadTable::DownlinkDirect),
        "dwell_limited" => Ok(PayloadTable::DwellLimited),
        other => Err(PyValueError::new_err(format!(
            "{other} is not a payload table; expected uplink_repeater, uplink_direct, downlink_repeater, downlink_direct or dwell_limited"
        ))),
    }
}

/// Resolves a channel-set name.
fn channels_are_join(which: &str) -> PyResult<bool> {
    match which.to_ascii_lowercase().as_str() {
        "join" => Ok(true),
        "default" => Ok(false),
        other => Err(PyValueError::new_err(format!(
            "{other} is not a channel set; expected join or default"
        ))),
    }
}

/// One data rate: what a number on the wire means for the radio.
///
/// Only the attributes belonging to `kind` are set; the rest are `None`.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct LoraDataRate {
    /// How this rate is carried: `lora`, `fsk`, `lr_fhss`, or `reserved`.
    #[pyo3(get)]
    kind: String,
    /// The payload bitrate in bits per second.
    #[pyo3(get)]
    bitrate_bps: u32,
    /// The channel bandwidth in hertz, for a LoRa or LR-FHSS rate.
    #[pyo3(get)]
    bandwidth_hz: Option<u32>,
    /// The spreading factor, for a LoRa rate.
    #[pyo3(get)]
    spreading_factor: Option<u8>,
    /// The coding-rate numerator, for an LR-FHSS rate.
    #[pyo3(get)]
    coding_rate_numerator: Option<u8>,
    /// The coding-rate denominator, for an LR-FHSS rate.
    #[pyo3(get)]
    coding_rate_denominator: Option<u8>,
}

impl From<Option<CoreDataRate>> for LoraDataRate {
    fn from(rate: Option<CoreDataRate>) -> Self {
        let Some(rate) = rate else {
            return Self {
                kind: "reserved".to_owned(),
                bitrate_bps: 0,
                bandwidth_hz: None,
                spreading_factor: None,
                coding_rate_numerator: None,
                coding_rate_denominator: None,
            };
        };
        let mut out = Self {
            kind: "fsk".to_owned(),
            bitrate_bps: rate.bitrate_bps,
            bandwidth_hz: None,
            spreading_factor: None,
            coding_rate_numerator: None,
            coding_rate_denominator: None,
        };
        match rate.modulation {
            Modulation::LoRa {
                spreading_factor,
                bandwidth_hz,
            } => {
                out.kind = "lora".to_owned();
                out.spreading_factor = Some(spreading_factor);
                out.bandwidth_hz = Some(bandwidth_hz);
            }
            Modulation::Fsk { .. } => {}
            Modulation::LrFhss {
                coding_rate_numerator,
                coding_rate_denominator,
                bandwidth_hz,
            } => {
                out.kind = "lr_fhss".to_owned();
                out.coding_rate_numerator = Some(coding_rate_numerator);
                out.coding_rate_denominator = Some(coding_rate_denominator);
                out.bandwidth_hz = Some(bandwidth_hz);
            }
        }
        out
    }
}

impl LoraDataRate {
    /// Converts the data rate into the Rust type, or `None` if it is reserved.
    fn to_core(&self) -> PyResult<Option<CoreDataRate>> {
        let missing =
            |field: &str| PyValueError::new_err(format!("a {} data rate needs {field}", self.kind));
        Ok(match self.kind.as_str() {
            "lora" => Some(CoreDataRate::lora(
                self.spreading_factor
                    .ok_or_else(|| missing("spreading_factor"))?,
                self.bandwidth_hz.ok_or_else(|| missing("bandwidth_hz"))?,
                self.bitrate_bps,
            )),
            "fsk" => Some(CoreDataRate::fsk(self.bitrate_bps)),
            "lr_fhss" => Some(CoreDataRate::lr_fhss(
                self.coding_rate_numerator
                    .ok_or_else(|| missing("coding_rate_numerator"))?,
                self.coding_rate_denominator
                    .ok_or_else(|| missing("coding_rate_denominator"))?,
                self.bandwidth_hz.ok_or_else(|| missing("bandwidth_hz"))?,
                self.bitrate_bps,
            )),
            "reserved" => None,
            other => {
                return Err(PyValueError::new_err(format!(
                    "{other} is not a modulation; expected lora, fsk, lr_fhss or reserved"
                )))
            }
        })
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl LoraDataRate {
    /// Describes a rate carried by LoRa modulation.
    #[staticmethod]
    fn lora(spreading_factor: u8, bandwidth_hz: u32, bitrate_bps: u32) -> Self {
        Self::from(Some(CoreDataRate::lora(
            spreading_factor,
            bandwidth_hz,
            bitrate_bps,
        )))
    }

    /// Describes a rate carried by frequency-shift keying.
    #[staticmethod]
    fn fsk(bitrate_bps: u32) -> Self {
        Self::from(Some(CoreDataRate::fsk(bitrate_bps)))
    }

    /// Describes a rate carried by long-range frequency-hopping spread spectrum.
    #[staticmethod]
    fn lr_fhss(
        coding_rate_numerator: u8,
        coding_rate_denominator: u8,
        bandwidth_hz: u32,
        bitrate_bps: u32,
    ) -> Self {
        Self::from(Some(CoreDataRate::lr_fhss(
            coding_rate_numerator,
            coding_rate_denominator,
            bandwidth_hz,
            bitrate_bps,
        )))
    }

    /// Describes a data-rate number the region reserves, which carries nothing.
    #[staticmethod]
    fn reserved() -> Self {
        Self::from(None)
    }

    fn __repr__(&self) -> String {
        match self.kind.as_str() {
            "lora" => format!(
                "LoraDataRate.lora(spreading_factor={}, bandwidth_hz={}, bitrate_bps={})",
                self.spreading_factor.unwrap_or(0),
                self.bandwidth_hz.unwrap_or(0),
                self.bitrate_bps
            ),
            "fsk" => format!("LoraDataRate.fsk(bitrate_bps={})", self.bitrate_bps),
            "lr_fhss" => format!(
                "LoraDataRate.lr_fhss(coding_rate={}/{}, bandwidth_hz={}, bitrate_bps={})",
                self.coding_rate_numerator.unwrap_or(0),
                self.coding_rate_denominator.unwrap_or(0),
                self.bandwidth_hz.unwrap_or(0),
                self.bitrate_bps
            ),
            _ => "LoraDataRate.reserved()".to_owned(),
        }
    }
}

/// What one data rate may carry in a single frame.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, Copy)]
pub struct LoraMaxPayload {
    /// The largest MAC payload, frame options included, in bytes.
    #[pyo3(get)]
    mac_payload: u16,
    /// The largest application payload, in bytes.
    #[pyo3(get)]
    application: u16,
}

#[gen_stub_pymethods]
#[pymethods]
impl LoraMaxPayload {
    #[new]
    fn new(mac_payload: u16, application: u16) -> Self {
        Self {
            mac_payload,
            application,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "LoraMaxPayload(mac_payload={}, application={})",
            self.mac_payload, self.application
        )
    }
}

/// A run of evenly spaced channels.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, Copy)]
pub struct LoraChannelBlock {
    /// The first channel's centre frequency in hertz.
    #[pyo3(get)]
    start_hz: u32,
    /// The spacing between channels in hertz.
    #[pyo3(get)]
    step_hz: u32,
    /// How many channels the block holds.
    #[pyo3(get)]
    count: u16,
    /// The slowest data rate the block allows.
    #[pyo3(get)]
    min_data_rate: u8,
    /// The fastest data rate the block allows.
    #[pyo3(get)]
    max_data_rate: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl LoraChannelBlock {
    #[new]
    fn new(start_hz: u32, step_hz: u32, count: u16, min_data_rate: u8, max_data_rate: u8) -> Self {
        Self {
            start_hz,
            step_hz,
            count,
            min_data_rate,
            max_data_rate,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "LoraChannelBlock(start_hz={}, step_hz={}, count={})",
            self.start_hz, self.step_hz, self.count
        )
    }
}

/// A slice of a band with its own transmit limits.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, Copy)]
pub struct LoraSubBand {
    /// The first frequency in the sub-band, in hertz.
    #[pyo3(get)]
    start_hz: u32,
    /// The last frequency in the sub-band, in hertz.
    #[pyo3(get)]
    end_hz: u32,
    /// The share of time a transmitter may hold the channel, in parts per
    /// thousand, so `10` is one percent and `1000` is unrestricted.
    #[pyo3(get)]
    duty_cycle_permille: u32,
    /// The power ceiling in dBm EIRP.
    #[pyo3(get)]
    max_eirp_dbm: i8,
}

#[gen_stub_pymethods]
#[pymethods]
impl LoraSubBand {
    #[new]
    fn new(start_hz: u32, end_hz: u32, duty_cycle_permille: u32, max_eirp_dbm: i8) -> Self {
        Self {
            start_hz,
            end_hz,
            duty_cycle_permille,
            max_eirp_dbm,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "LoraSubBand(start_hz={}, end_hz={}, duty_cycle_permille={}, max_eirp_dbm={})",
            self.start_hz, self.end_hz, self.duty_cycle_permille, self.max_eirp_dbm
        )
    }
}

/// The Class B beacon settings of a plan.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, Copy)]
pub struct LoraBeacon {
    /// The frequency the beacon is broadcast on, in hertz.
    #[pyo3(get)]
    frequency_hz: u32,
    /// The default ping-slot frequency, in hertz.
    #[pyo3(get)]
    ping_slot_frequency_hz: u32,
    /// The data rate the beacon is broadcast at.
    #[pyo3(get)]
    data_rate: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl LoraBeacon {
    #[new]
    fn new(frequency_hz: u32, ping_slot_frequency_hz: u32, data_rate: u8) -> Self {
        Self {
            frequency_hz,
            ping_slot_frequency_hz,
            data_rate,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "LoraBeacon(frequency_hz={}, ping_slot_frequency_hz={}, data_rate={})",
            self.frequency_hz, self.ping_slot_frequency_hz, self.data_rate
        )
    }
}

/// The scalar facts of a plan, read in one call.
#[gen_stub_pyclass]
#[pyclass]
pub struct LoraPlanInfo {
    /// The specification's name for the band, such as `EU863-870`.
    #[pyo3(get)]
    name: String,
    /// How many uplink data-rate numbers the plan defines, reserved included.
    #[pyo3(get)]
    uplink_data_rate_count: u16,
    /// How many downlink data-rate numbers the plan defines.
    #[pyo3(get)]
    downlink_data_rate_count: u16,
    /// How many channels the plan starts a device with.
    #[pyo3(get)]
    default_channel_count: u16,
    /// How many join channel blocks the plan defines.
    #[pyo3(get)]
    join_channel_block_count: u16,
    /// How many default channel blocks the plan defines.
    #[pyo3(get)]
    default_channel_block_count: u16,
    /// How many sub-bands the plan defines.
    #[pyo3(get)]
    sub_band_count: u16,
    /// The Class B beacon settings.
    #[pyo3(get)]
    beacon: LoraBeacon,
    /// The frequency the second receive window listens on, in hertz.
    #[pyo3(get)]
    rx2_frequency_hz: u32,
    /// The data rate the second receive window listens at.
    #[pyo3(get)]
    rx2_data_rate: u8,
    /// The power ceiling assumed when no sub-band says otherwise, in dBm.
    #[pyo3(get)]
    default_max_eirp_dbm: i8,
    /// The step between transmit-power settings, in dB.
    #[pyo3(get)]
    tx_power_step_db: u8,
    /// The highest transmit-power index the plan defines.
    #[pyo3(get)]
    max_tx_power_index: u8,
    /// The highest RX1 data-rate offset the plan allows.
    #[pyo3(get)]
    max_rx1_data_rate_offset: u8,
    /// Whether the plan limits how long one transmission may hold a channel.
    #[pyo3(get)]
    has_dwell_time_limit: bool,
    /// Whether the plan publishes a payload table for a dwell-limited device.
    #[pyo3(get)]
    has_dwell_limited_payloads: bool,
    /// Whether the plan publishes a second RX1 mapping for a dwell-limited
    /// downlink.
    #[pyo3(get)]
    has_dwell_limited_rx1: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl LoraPlanInfo {
    fn __repr__(&self) -> String {
        format!(
            "LoraPlanInfo(name={:?}, uplink_data_rate_count={}, default_channel_count={})",
            self.name, self.uplink_data_rate_count, self.default_channel_count
        )
    }
}

/// A regional channel plan, published or private.
#[gen_stub_pyclass]
#[pyclass]
pub struct ChannelPlan {
    inner: OwnedChannelPlan,
}

#[gen_stub_pymethods]
#[pymethods]
impl ChannelPlan {
    /// Returns the published plan for a region.
    ///
    /// Raises `ValueError` if no published region goes by that name.
    #[staticmethod]
    fn for_region(region: &str) -> PyResult<Self> {
        Ok(Self {
            inner: OwnedChannelPlan::from_plan(region_plan(region)?),
        })
    }

    /// Returns the short code of every published region, as `for_region` takes.
    ///
    /// These are the codes a deployment is configured with. The band name the
    /// specification uses, such as `EU863-870`, is the plan's `name`.
    #[staticmethod]
    fn regions() -> Vec<String> {
        Region::all()
            .iter()
            .map(|region| region.code().to_owned())
            .collect()
    }

    /// The specification's name for the band.
    #[getter]
    fn name(&self) -> String {
        self.inner.with_plan(|plan| plan.name.to_owned())
    }

    /// Returns the scalar facts of the plan.
    fn info(&self) -> LoraPlanInfo {
        self.inner.with_plan(|plan| LoraPlanInfo {
            name: plan.name.to_owned(),
            uplink_data_rate_count: plan.uplink_data_rates.len() as u16,
            downlink_data_rate_count: plan.downlink_data_rates.len() as u16,
            default_channel_count: plan.default_channel_count(),
            join_channel_block_count: plan.join_channels.len() as u16,
            default_channel_block_count: plan.default_channels.len() as u16,
            sub_band_count: plan.sub_bands.len() as u16,
            beacon: LoraBeacon {
                frequency_hz: plan.beacon.frequency_hz,
                ping_slot_frequency_hz: plan.beacon.ping_slot_frequency_hz,
                data_rate: plan.beacon.data_rate,
            },
            rx2_frequency_hz: plan.rx2_frequency_hz,
            rx2_data_rate: plan.rx2_data_rate,
            default_max_eirp_dbm: plan.default_max_eirp_dbm,
            tx_power_step_db: plan.tx_power_step_db,
            max_tx_power_index: plan.max_tx_power_index,
            max_rx1_data_rate_offset: plan.max_rx1_data_rate_offset,
            has_dwell_time_limit: plan.has_dwell_time_limit,
            has_dwell_limited_payloads: plan.max_payload_dwell_limited.is_some(),
            has_dwell_limited_rx1: plan.rx1_data_rate_offsets_dwell_limited.is_some(),
        })
    }

    /// Returns the data rate a number selects, or `None` past the end of the
    /// plan's table.
    ///
    /// A number the region reserves is a data rate of kind `reserved`, which is
    /// different from a number the plan never defines.
    #[pyo3(signature = (data_rate, direction = "uplink"))]
    fn data_rate(&self, data_rate: u8, direction: &str) -> PyResult<Option<LoraDataRate>> {
        let uplink = direction_is_uplink(direction)?;
        Ok(self.inner.with_plan(|plan| {
            let table = if uplink {
                plan.uplink_data_rates
            } else {
                plan.downlink_data_rates
            };
            table
                .get(usize::from(data_rate))
                .map(|rate| LoraDataRate::from(*rate))
        }))
    }

    /// Returns the radio settings an uplink data rate selects, ready to hand to
    /// `airtime_us`, or `None` if the number is reserved or not carried by LoRa.
    fn link_settings(&self, data_rate: u8) -> Option<LoraLink> {
        self.inner
            .with_plan(|plan| plan.link_settings(data_rate))
            .map(LoraLink::from_settings)
    }

    /// Returns what a data rate may carry in one frame, or `None` where the plan
    /// publishes no limit for it.
    #[pyo3(signature = (data_rate, table = "uplink_direct"))]
    fn max_payload(&self, data_rate: u8, table: &str) -> PyResult<Option<LoraMaxPayload>> {
        let table = payload_table(table)?;
        Ok(self.inner.with_plan(|plan| {
            let payload = match table {
                PayloadTable::UplinkRepeater => plan.max_payload(data_rate, true),
                PayloadTable::UplinkDirect => plan.max_payload(data_rate, false),
                PayloadTable::DownlinkRepeater => plan.downlink_max_payload(data_rate, true),
                PayloadTable::DownlinkDirect => plan.downlink_max_payload(data_rate, false),
                PayloadTable::DwellLimited => plan.max_payload_dwell_limited(data_rate),
            }?;
            Some(LoraMaxPayload {
                mac_payload: payload.mac_payload,
                application: payload.application,
            })
        }))
    }

    /// Returns the share of time a transmitter may hold a frequency, in parts per
    /// thousand, or `None` if the frequency falls in no sub-band this plan
    /// describes.
    ///
    /// This reports the limit; it does not impose it. Pair it with
    /// `min_off_time_us` to turn the limit into the silence a frame costs.
    fn duty_cycle_permille(&self, frequency_hz: u32) -> Option<u32> {
        self.inner
            .with_plan(|plan| plan.duty_cycle_permille(frequency_hz))
    }

    /// Returns the power ceiling that applies at a frequency, in dBm EIRP,
    /// falling back to the plan's default where no sub-band says otherwise.
    fn max_eirp_dbm(&self, frequency_hz: u32) -> i8 {
        self.inner.with_plan(|plan| plan.max_eirp_dbm(frequency_hz))
    }

    /// Returns the radiated power a transmit-power index selects, in dBm, or
    /// `None` if the index is past the highest the plan defines.
    fn tx_power_dbm(&self, index: u8, max_eirp_dbm: i8) -> Option<i8> {
        self.inner
            .with_plan(|plan| plan.tx_power_dbm(index, max_eirp_dbm))
    }

    /// Returns the downlink data rate the first receive window listens at, or
    /// `None` if the uplink data rate or offset is outside what the plan defines.
    #[pyo3(signature = (uplink_data_rate, offset, dwell_limited = false))]
    fn rx1_data_rate(&self, uplink_data_rate: u8, offset: u8, dwell_limited: bool) -> Option<u8> {
        self.inner.with_plan(|plan| {
            if dwell_limited {
                plan.rx1_data_rate_dwell_limited(uplink_data_rate, offset)
            } else {
                plan.rx1_data_rate(uplink_data_rate, offset)
            }
        })
    }

    /// Returns where the second receive window listens, as a frequency in hertz
    /// and a data rate.
    fn rx2(&self) -> (u32, u8) {
        self.inner.with_plan(|plan| plan.rx2())
    }

    /// Returns the next lower data rate to fall back to during adaptive back-off,
    /// or `None` at the slowest rate the plan has.
    ///
    /// A device that has lost the network steps down this chain, trading airtime
    /// for range until it is heard again.
    fn next_backoff_data_rate(&self, data_rate: u8) -> Option<u8> {
        self.inner
            .with_plan(|plan| plan.next_backoff_data_rate(data_rate))
    }

    /// Returns the centre frequency of one of the plan's default channels, or
    /// `None` past the last one the plan starts a device with.
    fn channel_frequency_hz(&self, channel: u16) -> Option<u32> {
        self.inner
            .with_plan(|plan| plan.channel_frequency_hz(channel))
    }

    /// Returns the plan's channel blocks, either the join set or the default set.
    #[pyo3(signature = (which = "default"))]
    fn channel_blocks(&self, which: &str) -> PyResult<Vec<LoraChannelBlock>> {
        let join = channels_are_join(which)?;
        Ok(self.inner.with_plan(|plan| {
            let blocks = if join {
                plan.join_channels
            } else {
                plan.default_channels
            };
            blocks
                .iter()
                .map(|block| LoraChannelBlock {
                    start_hz: block.start_hz,
                    step_hz: block.step_hz,
                    count: block.count,
                    min_data_rate: block.min_data_rate,
                    max_data_rate: block.max_data_rate,
                })
                .collect()
        }))
    }

    /// Returns the plan's sub-bands and the transmit limits inside each.
    fn sub_bands(&self) -> Vec<LoraSubBand> {
        self.inner.with_plan(|plan| {
            plan.sub_bands
                .iter()
                .map(|band| LoraSubBand {
                    start_hz: band.start_hz,
                    end_hz: band.end_hz,
                    duty_cycle_permille: band.duty_cycle_permille,
                    max_eirp_dbm: band.max_eirp_dbm,
                })
                .collect()
        })
    }

    fn __repr__(&self) -> String {
        format!("ChannelPlan({:?})", self.name())
    }
}

/// A channel plan under construction.
///
/// Tables are indexed by position, so entries are added in data-rate order and a
/// number the plan does not use is added as a reserved data rate. What a region
/// would share between directions is filled in by `build`.
#[gen_stub_pyclass]
#[pyclass]
pub struct ChannelPlanBuilder {
    inner: Mutex<Option<CoreBuilder>>,
}

impl ChannelPlanBuilder {
    /// Applies one step to the held builder, which consumes itself at each step.
    fn update(&self, step: impl FnOnce(CoreBuilder) -> CoreBuilder) -> PyResult<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| PyValueError::new_err("the builder is poisoned"))?;
        let taken = guard
            .take()
            .ok_or_else(|| PyValueError::new_err("this builder has already been built"))?;
        *guard = Some(step(taken));
        Ok(())
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl ChannelPlanBuilder {
    /// Starts an empty plan.
    ///
    /// The plan begins with no data rates, channels, or sub-bands, a two-decibel
    /// power ladder, and no dwell-time limit.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Mutex::new(Some(CoreBuilder::new(name))),
        }
    }

    /// Adds the next data rate in a direction.
    ///
    /// A plan that never adds a downlink rate uses its uplink table in both
    /// directions, which is what every region but the 900 MHz plans does.
    #[pyo3(signature = (rate, direction = "uplink"))]
    fn data_rate(&self, rate: &LoraDataRate, direction: &str) -> PyResult<()> {
        let uplink = direction_is_uplink(direction)?;
        let rate = rate.to_core()?;
        self.update(|builder| {
            if uplink {
                builder.uplink_data_rate(rate)
            } else {
                builder.downlink_data_rate(rate)
            }
        })
    }

    /// Adds the next entry of one payload table.
    ///
    /// Pass no payload for a data rate that carries nothing. A downlink table
    /// left empty mirrors the matching uplink one.
    #[pyo3(signature = (payload = None, table = "uplink_direct"))]
    fn max_payload(&self, payload: Option<LoraMaxPayload>, table: &str) -> PyResult<()> {
        let table = payload_table(table)?;
        let entry = payload.map(|p| CoreMaxPayload::new(p.mac_payload, p.application));
        self.update(|builder| builder.max_payload(table, entry))
    }

    /// Adds a run of evenly spaced channels.
    #[pyo3(signature = (block, which = "default"))]
    fn channel_block(&self, block: &LoraChannelBlock, which: &str) -> PyResult<()> {
        let join = channels_are_join(which)?;
        let entry = CoreBlock::new(
            block.start_hz,
            block.step_hz,
            block.count,
            block.min_data_rate,
            block.max_data_rate,
        );
        self.update(|builder| {
            if join {
                builder.join_channel(entry)
            } else {
                builder.default_channel(entry)
            }
        })
    }

    /// Adds a sub-band and the transmit limits inside it.
    ///
    /// A deployment on licensed spectrum gives its sub-band a duty cycle of
    /// `1000`, which reports as unrestricted.
    fn sub_band(&self, band: &LoraSubBand) -> PyResult<()> {
        let entry = CoreSubBand::new(
            band.start_hz,
            band.end_hz,
            band.duty_cycle_permille,
            band.max_eirp_dbm,
        );
        self.update(|builder| builder.sub_band(entry))
    }

    /// Adds the RX1 downlink data rates for the next uplink data rate.
    ///
    /// Every row must be as wide as the plan's highest RX1 offset allows.
    #[pyo3(signature = (offsets, dwell_limited = false))]
    fn rx1_row(&self, offsets: Vec<u8>, dwell_limited: bool) -> PyResult<()> {
        self.update(|builder| {
            if dwell_limited {
                builder.rx1_row_dwell_limited(&offsets)
            } else {
                builder.rx1_row(&offsets)
            }
        })
    }

    /// Adds the next entry of the adaptive back-off chain.
    ///
    /// Pass no data rate at the slowest, which has nothing below it. A chain left
    /// empty steps down one data rate at a time.
    #[pyo3(signature = (lower = None))]
    fn backoff(&self, lower: Option<u8>) -> PyResult<()> {
        self.update(|builder| builder.backoff(lower))
    }

    /// Sets the transmit-power ladder.
    #[pyo3(signature = (default_max_eirp_dbm, step_db = 2, max_index = 7))]
    fn power(&self, default_max_eirp_dbm: i8, step_db: u8, max_index: u8) -> PyResult<()> {
        self.update(|builder| builder.power(default_max_eirp_dbm, step_db, max_index))
    }

    /// Sets the receive windows.
    ///
    /// `max_rx1_offset` fixes how wide every RX1 row must be.
    #[pyo3(signature = (rx2_frequency_hz, rx2_data_rate = 0, max_rx1_offset = 0))]
    fn rx(&self, rx2_frequency_hz: u32, rx2_data_rate: u8, max_rx1_offset: u8) -> PyResult<()> {
        self.update(|builder| builder.rx(rx2_frequency_hz, rx2_data_rate, max_rx1_offset))
    }

    /// Sets the Class B beacon and whether the plan limits dwell time.
    #[pyo3(signature = (beacon, has_dwell_time_limit = false))]
    fn beacon(&self, beacon: &LoraBeacon, has_dwell_time_limit: bool) -> PyResult<()> {
        let entry = CoreBeacon {
            data_rate: beacon.data_rate,
            frequency_hz: beacon.frequency_hz,
            ping_slot_frequency_hz: beacon.ping_slot_frequency_hz,
        };
        self.update(|builder| builder.beacon(entry).dwell_time_limit(has_dwell_time_limit))
    }

    /// Finishes the plan.
    ///
    /// Raises `ValueError` if the plan would answer a question wrongly, for
    /// example because an RX1 row is narrower than the plan's offsets allow, or
    /// because the second receive window listens at a data rate the plan does not
    /// define.
    fn build(&self) -> PyResult<ChannelPlan> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| PyValueError::new_err("the builder is poisoned"))?;
        let taken = guard
            .take()
            .ok_or_else(|| PyValueError::new_err("this builder has already been built"))?;
        match taken.build() {
            Ok(inner) => Ok(ChannelPlan { inner }),
            Err(error) => Err(PyValueError::new_err(error.to_string())),
        }
    }
}
