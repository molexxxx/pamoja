//! Generated Node bindings for LoRaWAN regional channel plans.
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
//! lesser one, so [`LoraPlanBuilder`] produces a [`LoraChannelPlan`] that answers
//! every question `LoraChannelPlan.forRegion` does.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use pamoja_lora::region::{
    Beacon as CoreBeacon, ChannelBlock as CoreBlock, ChannelPlan, ChannelPlanBuilder,
    DataRate as CoreDataRate, MaxPayload as CoreMaxPayload, Modulation, OwnedChannelPlan,
    PayloadTable as CorePayloadTable, Region, SubBand as CoreSubBand,
};

use crate::lora::LoraLink;

/// A band with a published channel plan.
#[napi(string_enum)]
pub enum LoraRegion {
    /// Europe, 863-870 MHz.
    Eu868,
    /// North America, 902-928 MHz.
    Us915,
    /// Europe, 433 MHz.
    Eu433,
    /// Australia, 915-928 MHz.
    Au915,
    /// China, 470-510 MHz.
    Cn470,
    /// Asia, 923 MHz.
    As923,
    /// South Korea, 920-923 MHz.
    Kr920,
    /// India, 865-867 MHz.
    In865,
    /// Russia, 864-870 MHz.
    Ru864,
}

impl LoraRegion {
    /// Returns the published plan this region names.
    fn plan(self) -> &'static ChannelPlan<'static> {
        match self {
            Self::Eu868 => Region::Eu868.plan(),
            Self::Us915 => Region::Us915.plan(),
            Self::Eu433 => Region::Eu433.plan(),
            Self::Au915 => Region::Au915.plan(),
            Self::Cn470 => Region::Cn470.plan(),
            Self::As923 => Region::As923.plan(),
            Self::Kr920 => Region::Kr920.plan(),
            Self::In865 => Region::In865.plan(),
            Self::Ru864 => Region::Ru864.plan(),
        }
    }
}

/// Which direction a data-rate table describes.
///
/// Most regions number their data rates the same way in both directions and carry
/// one table; the 900 MHz plans do not.
#[napi(string_enum)]
pub enum LoraDirection {
    /// From the device to the network.
    Uplink,
    /// From the network to the device.
    Downlink,
}

/// Which of a plan's payload tables to read.
#[napi(string_enum)]
pub enum LoraPayloadTable {
    /// Uplink, for a device that may sit behind a repeater.
    UplinkRepeater,
    /// Uplink, for a device that will not.
    UplinkDirect,
    /// Downlink, for a device that may sit behind a repeater.
    DownlinkRepeater,
    /// Downlink, for a device that will not.
    DownlinkDirect,
    /// The limits that apply under a dwell-time limit.
    DwellLimited,
}

impl From<LoraPayloadTable> for CorePayloadTable {
    fn from(table: LoraPayloadTable) -> Self {
        match table {
            LoraPayloadTable::UplinkRepeater => Self::UplinkRepeater,
            LoraPayloadTable::UplinkDirect => Self::UplinkDirect,
            LoraPayloadTable::DownlinkRepeater => Self::DownlinkRepeater,
            LoraPayloadTable::DownlinkDirect => Self::DownlinkDirect,
            LoraPayloadTable::DwellLimited => Self::DwellLimited,
        }
    }
}

/// Which channels of a plan to read.
#[napi(string_enum)]
pub enum LoraChannelSet {
    /// The channels a device must use to send a join request.
    Join,
    /// The channels a device starts with before a network adds any.
    Default,
}

/// How a data rate is carried on the air.
#[napi(string_enum)]
pub enum LoraModulation {
    /// LoRa modulation, described by a spreading factor and bandwidth.
    Lora,
    /// Frequency-shift keying, described by its bitrate alone.
    Fsk,
    /// Long-range frequency-hopping spread spectrum.
    LrFhss,
    /// A data-rate number the region reserves, which carries nothing.
    Reserved,
}

/// One data rate: what a number on the wire means for the radio.
///
/// Only the fields belonging to `kind` are set.
#[napi(object)]
pub struct LoraDataRate {
    /// How this rate is carried.
    pub kind: LoraModulation,
    /// The payload bitrate in bits per second.
    pub bitrate_bps: u32,
    /// The channel bandwidth in hertz, for a LoRa or LR-FHSS rate.
    pub bandwidth_hz: Option<u32>,
    /// The spreading factor, for a LoRa rate.
    pub spreading_factor: Option<u8>,
    /// The coding-rate numerator, for an LR-FHSS rate.
    pub coding_rate_numerator: Option<u8>,
    /// The coding-rate denominator, for an LR-FHSS rate.
    pub coding_rate_denominator: Option<u8>,
}

impl From<CoreDataRate> for LoraDataRate {
    fn from(rate: CoreDataRate) -> Self {
        let mut out = Self {
            kind: LoraModulation::Fsk,
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
                out.kind = LoraModulation::Lora;
                out.spreading_factor = Some(spreading_factor);
                out.bandwidth_hz = Some(bandwidth_hz);
            }
            Modulation::Fsk { .. } => {}
            Modulation::LrFhss {
                coding_rate_numerator,
                coding_rate_denominator,
                bandwidth_hz,
            } => {
                out.kind = LoraModulation::LrFhss;
                out.coding_rate_numerator = Some(coding_rate_numerator);
                out.coding_rate_denominator = Some(coding_rate_denominator);
                out.bandwidth_hz = Some(bandwidth_hz);
            }
        }
        out
    }
}

impl LoraDataRate {
    /// Converts a data rate the caller supplied into the Rust type.
    ///
    /// Returns `Ok(None)` for a reserved number, which is a data-rate slot the
    /// plan defines but does not use.
    fn to_core(&self) -> Result<Option<CoreDataRate>> {
        let kind = match self.kind {
            LoraModulation::Lora => "LoRa",
            LoraModulation::Fsk => "FSK",
            LoraModulation::LrFhss => "LR-FHSS",
            LoraModulation::Reserved => "reserved",
        };
        let missing = |field: &str| {
            Error::new(
                Status::InvalidArg,
                format!("a {kind} data rate needs {field}"),
            )
        };
        Ok(match self.kind {
            LoraModulation::Lora => Some(CoreDataRate::lora(
                self.spreading_factor
                    .ok_or_else(|| missing("spreadingFactor"))?,
                self.bandwidth_hz.ok_or_else(|| missing("bandwidthHz"))?,
                self.bitrate_bps,
            )),
            LoraModulation::Fsk => Some(CoreDataRate::fsk(self.bitrate_bps)),
            LoraModulation::LrFhss => Some(CoreDataRate::lr_fhss(
                self.coding_rate_numerator
                    .ok_or_else(|| missing("codingRateNumerator"))?,
                self.coding_rate_denominator
                    .ok_or_else(|| missing("codingRateDenominator"))?,
                self.bandwidth_hz.ok_or_else(|| missing("bandwidthHz"))?,
                self.bitrate_bps,
            )),
            LoraModulation::Reserved => None,
        })
    }
}

/// What one data rate may carry in a single frame.
#[napi(object)]
pub struct LoraMaxPayload {
    /// The largest MAC payload, frame options included, in bytes.
    pub mac_payload: u16,
    /// The largest application payload, in bytes.
    pub application: u16,
}

/// A run of evenly spaced channels.
#[napi(object)]
pub struct LoraChannelBlock {
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
#[napi(object)]
pub struct LoraSubBand {
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
#[napi(object)]
pub struct LoraBeacon {
    /// The frequency the beacon is broadcast on, in hertz.
    pub frequency_hz: u32,
    /// The default ping-slot frequency, in hertz.
    pub ping_slot_frequency_hz: u32,
    /// The data rate the beacon is broadcast at.
    pub data_rate: u8,
}

/// Where the second receive window listens.
#[napi(object)]
pub struct LoraRx2 {
    /// The fixed frequency, in hertz.
    pub frequency_hz: u32,
    /// The data rate.
    pub data_rate: u8,
}

/// The scalar facts of a plan, read in one call.
#[napi(object)]
pub struct LoraPlanInfo {
    /// The specification's name for the band, such as `EU863-870`.
    pub name: String,
    /// How many uplink data-rate numbers the plan defines, reserved included.
    pub uplink_data_rate_count: u16,
    /// How many downlink data-rate numbers the plan defines.
    pub downlink_data_rate_count: u16,
    /// How many channels the plan starts a device with.
    pub default_channel_count: u16,
    /// How many join channel blocks the plan defines.
    pub join_channel_block_count: u16,
    /// How many default channel blocks the plan defines.
    pub default_channel_block_count: u16,
    /// How many sub-bands the plan defines.
    pub sub_band_count: u16,
    /// The Class B beacon settings.
    pub beacon: LoraBeacon,
    /// Where the second receive window listens.
    pub rx2: LoraRx2,
    /// The power ceiling assumed when no sub-band says otherwise, in dBm.
    pub default_max_eirp_dbm: i8,
    /// The step between transmit-power settings, in dB.
    pub tx_power_step_db: u8,
    /// The highest transmit-power index the plan defines.
    pub max_tx_power_index: u8,
    /// The highest RX1 data-rate offset the plan allows.
    pub max_rx1_data_rate_offset: u8,
    /// Whether the plan limits how long one transmission may hold a channel.
    pub has_dwell_time_limit: bool,
    /// Whether the plan publishes a payload table for a dwell-limited device.
    pub has_dwell_limited_payloads: bool,
    /// Whether the plan publishes a second RX1 mapping for a dwell-limited
    /// downlink.
    pub has_dwell_limited_rx1: bool,
}

/// A regional channel plan, published or private.
#[napi]
pub struct LoraChannelPlan {
    inner: OwnedChannelPlan,
}

#[napi]
impl LoraChannelPlan {
    /// Returns the published plan for a region.
    #[napi(factory)]
    pub fn for_region(region: LoraRegion) -> Self {
        Self {
            inner: OwnedChannelPlan::from_plan(region.plan()),
        }
    }

    /// Returns the scalar facts of the plan.
    #[napi]
    pub fn info(&self) -> LoraPlanInfo {
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
            rx2: LoraRx2 {
                frequency_hz: plan.rx2_frequency_hz,
                data_rate: plan.rx2_data_rate,
            },
            default_max_eirp_dbm: plan.default_max_eirp_dbm,
            tx_power_step_db: plan.tx_power_step_db,
            max_tx_power_index: plan.max_tx_power_index,
            max_rx1_data_rate_offset: plan.max_rx1_data_rate_offset,
            has_dwell_time_limit: plan.has_dwell_time_limit,
            has_dwell_limited_payloads: plan.max_payload_dwell_limited.is_some(),
            has_dwell_limited_rx1: plan.rx1_data_rate_offsets_dwell_limited.is_some(),
        })
    }

    /// Returns the specification's name for the band.
    #[napi(getter)]
    pub fn name(&self) -> String {
        self.inner.with_plan(|plan| plan.name.to_owned())
    }

    /// Returns the data rate a number selects, or null if the number is past the
    /// end of the plan's table.
    ///
    /// A number the region reserves is a data rate of kind `Reserved`, which is
    /// different from a number the plan never defines.
    #[napi]
    pub fn data_rate(&self, direction: LoraDirection, data_rate: u8) -> Option<LoraDataRate> {
        self.inner.with_plan(|plan| {
            let table = match direction {
                LoraDirection::Uplink => plan.uplink_data_rates,
                LoraDirection::Downlink => plan.downlink_data_rates,
            };
            let slot = *table.get(usize::from(data_rate))?;
            Some(match slot {
                Some(rate) => LoraDataRate::from(rate),
                None => LoraDataRate {
                    kind: LoraModulation::Reserved,
                    bitrate_bps: 0,
                    bandwidth_hz: None,
                    spreading_factor: None,
                    coding_rate_numerator: None,
                    coding_rate_denominator: None,
                },
            })
        })
    }

    /// Returns the radio settings an uplink data rate selects, ready to hand to
    /// `loraAirtimeUs`, or null if the number is reserved or not carried by LoRa.
    #[napi]
    pub fn link_settings(&self, data_rate: u8) -> Option<LoraLink> {
        self.inner.with_plan(|plan| {
            let settings = plan.link_settings(data_rate)?;
            Some(LoraLink {
                spreading_factor: settings.spreading_factor(),
                bandwidth_hz: settings.bandwidth_hz(),
                coding_rate_denominator: 5,
                preamble_symbols: 8,
                explicit_header: true,
                crc: true,
            })
        })
    }

    /// Returns what a data rate may carry in one frame, or null where the plan
    /// publishes no limit for it.
    #[napi]
    pub fn max_payload(&self, table: LoraPayloadTable, data_rate: u8) -> Option<LoraMaxPayload> {
        self.inner.with_plan(|plan| {
            let payload = match table {
                LoraPayloadTable::UplinkRepeater => plan.max_payload(data_rate, true),
                LoraPayloadTable::UplinkDirect => plan.max_payload(data_rate, false),
                LoraPayloadTable::DownlinkRepeater => plan.downlink_max_payload(data_rate, true),
                LoraPayloadTable::DownlinkDirect => plan.downlink_max_payload(data_rate, false),
                LoraPayloadTable::DwellLimited => plan.max_payload_dwell_limited(data_rate),
            }?;
            Some(LoraMaxPayload {
                mac_payload: payload.mac_payload,
                application: payload.application,
            })
        })
    }

    /// Returns the share of time a transmitter may hold a frequency, in parts per
    /// thousand, or null if the frequency falls in no sub-band this plan
    /// describes.
    ///
    /// This reports the limit; it does not impose it. Pair it with
    /// `loraMinOffTimeUs` to turn the limit into the silence a frame costs.
    #[napi]
    pub fn duty_cycle_permille(&self, frequency_hz: u32) -> Option<u32> {
        self.inner
            .with_plan(|plan| plan.duty_cycle_permille(frequency_hz))
    }

    /// Returns the power ceiling that applies at a frequency, in dBm EIRP,
    /// falling back to the plan's default where no sub-band says otherwise.
    #[napi]
    pub fn max_eirp_dbm(&self, frequency_hz: u32) -> i8 {
        self.inner.with_plan(|plan| plan.max_eirp_dbm(frequency_hz))
    }

    /// Returns the radiated power a transmit-power index selects, in dBm, or null
    /// if the index is past the highest the plan defines.
    #[napi]
    pub fn tx_power_dbm(&self, index: u8, max_eirp_dbm: i8) -> Option<i8> {
        self.inner
            .with_plan(|plan| plan.tx_power_dbm(index, max_eirp_dbm))
    }

    /// Returns the downlink data rate the first receive window listens at, or
    /// null if the uplink data rate or offset is outside what the plan defines.
    #[napi]
    pub fn rx1_data_rate(
        &self,
        uplink_data_rate: u8,
        offset: u8,
        dwell_limited: Option<bool>,
    ) -> Option<u8> {
        self.inner.with_plan(|plan| {
            if dwell_limited.unwrap_or(false) {
                plan.rx1_data_rate_dwell_limited(uplink_data_rate, offset)
            } else {
                plan.rx1_data_rate(uplink_data_rate, offset)
            }
        })
    }

    /// Returns where the second receive window listens.
    #[napi]
    pub fn rx2(&self) -> LoraRx2 {
        let (frequency_hz, data_rate) = self.inner.with_plan(|plan| plan.rx2());
        LoraRx2 {
            frequency_hz,
            data_rate,
        }
    }

    /// Returns the next lower data rate to fall back to during adaptive back-off,
    /// or null at the slowest rate the plan has.
    ///
    /// A device that has lost the network steps down this chain, trading airtime
    /// for range until it is heard again.
    #[napi]
    pub fn next_backoff_data_rate(&self, data_rate: u8) -> Option<u8> {
        self.inner
            .with_plan(|plan| plan.next_backoff_data_rate(data_rate))
    }

    /// Returns the centre frequency of one of the plan's default channels, or
    /// null past the last one the plan starts a device with.
    #[napi]
    pub fn channel_frequency_hz(&self, channel: u16) -> Option<u32> {
        self.inner
            .with_plan(|plan| plan.channel_frequency_hz(channel))
    }

    /// Returns one of the plan's channel blocks, or null past the end.
    #[napi]
    pub fn channel_block(&self, which: LoraChannelSet, index: u16) -> Option<LoraChannelBlock> {
        self.inner.with_plan(|plan| {
            let blocks = match which {
                LoraChannelSet::Join => plan.join_channels,
                LoraChannelSet::Default => plan.default_channels,
            };
            let block = blocks.get(usize::from(index))?;
            Some(LoraChannelBlock {
                start_hz: block.start_hz,
                step_hz: block.step_hz,
                count: block.count,
                min_data_rate: block.min_data_rate,
                max_data_rate: block.max_data_rate,
            })
        })
    }

    /// Returns one of the plan's sub-bands, or null past the end.
    #[napi]
    pub fn sub_band(&self, index: u16) -> Option<LoraSubBand> {
        self.inner.with_plan(|plan| {
            let band = plan.sub_bands.get(usize::from(index))?;
            Some(LoraSubBand {
                start_hz: band.start_hz,
                end_hz: band.end_hz,
                duty_cycle_permille: band.duty_cycle_permille,
                max_eirp_dbm: band.max_eirp_dbm,
            })
        })
    }
}

/// A channel plan under construction.
///
/// Tables are indexed by position, so entries are pushed in data-rate order and a
/// number the plan does not use is pushed as a `Reserved` data rate. What a region
/// would share between directions is filled in by `build`.
#[napi]
pub struct LoraPlanBuilder {
    inner: Option<ChannelPlanBuilder>,
}

impl LoraPlanBuilder {
    /// Applies one step to the held builder, which consumes itself at each step.
    fn update(
        &mut self,
        step: impl FnOnce(ChannelPlanBuilder) -> ChannelPlanBuilder,
    ) -> Result<()> {
        let taken = self.inner.take().ok_or_else(|| {
            Error::new(
                Status::InvalidArg,
                "this builder has already been built".to_owned(),
            )
        })?;
        self.inner = Some(step(taken));
        Ok(())
    }
}

#[napi]
impl LoraPlanBuilder {
    /// Starts an empty plan.
    ///
    /// The plan begins with no data rates, channels, or sub-bands, a two-decibel
    /// power ladder, and no dwell-time limit.
    #[napi(constructor)]
    pub fn new(name: String) -> Self {
        Self {
            inner: Some(ChannelPlanBuilder::new(name)),
        }
    }

    /// Appends the next data rate in a direction.
    ///
    /// A plan that never appends a downlink rate uses its uplink table in both
    /// directions, which is what every region but the 900 MHz plans does.
    #[napi]
    pub fn data_rate(&mut self, direction: LoraDirection, rate: LoraDataRate) -> Result<()> {
        let rate = rate.to_core()?;
        self.update(|builder| match direction {
            LoraDirection::Uplink => builder.uplink_data_rate(rate),
            LoraDirection::Downlink => builder.downlink_data_rate(rate),
        })
    }

    /// Appends the next entry of one payload table.
    ///
    /// Pass no payload for a data rate that carries nothing. A downlink table
    /// left empty mirrors the matching uplink one.
    #[napi]
    pub fn max_payload(
        &mut self,
        table: LoraPayloadTable,
        payload: Option<LoraMaxPayload>,
    ) -> Result<()> {
        let entry = payload.map(|p| CoreMaxPayload::new(p.mac_payload, p.application));
        self.update(|builder| builder.max_payload(table.into(), entry))
    }

    /// Adds a run of evenly spaced channels.
    #[napi]
    pub fn channel_block(&mut self, which: LoraChannelSet, block: LoraChannelBlock) -> Result<()> {
        let entry = CoreBlock::new(
            block.start_hz,
            block.step_hz,
            block.count,
            block.min_data_rate,
            block.max_data_rate,
        );
        self.update(|builder| match which {
            LoraChannelSet::Join => builder.join_channel(entry),
            LoraChannelSet::Default => builder.default_channel(entry),
        })
    }

    /// Adds a sub-band and the transmit limits inside it.
    ///
    /// A deployment on licensed spectrum gives its sub-band a duty cycle of
    /// `1000`, which reports as unrestricted.
    #[napi]
    pub fn sub_band(&mut self, band: LoraSubBand) -> Result<()> {
        let entry = CoreSubBand::new(
            band.start_hz,
            band.end_hz,
            band.duty_cycle_permille,
            band.max_eirp_dbm,
        );
        self.update(|builder| builder.sub_band(entry))
    }

    /// Appends the RX1 downlink data rates for the next uplink data rate.
    ///
    /// Every row must be as wide as the plan's highest RX1 offset allows.
    #[napi]
    pub fn rx1_row(&mut self, offsets: Vec<u8>, dwell_limited: Option<bool>) -> Result<()> {
        self.update(|builder| {
            if dwell_limited.unwrap_or(false) {
                builder.rx1_row_dwell_limited(&offsets)
            } else {
                builder.rx1_row(&offsets)
            }
        })
    }

    /// Appends the next entry of the adaptive back-off chain.
    ///
    /// Pass no data rate at the slowest, which has nothing below it. A chain left
    /// empty steps down one data rate at a time.
    #[napi]
    pub fn backoff(&mut self, lower: Option<u8>) -> Result<()> {
        self.update(|builder| builder.backoff(lower))
    }

    /// Sets the transmit-power ladder.
    #[napi]
    pub fn power(&mut self, default_max_eirp_dbm: i8, step_db: u8, max_index: u8) -> Result<()> {
        self.update(|builder| builder.power(default_max_eirp_dbm, step_db, max_index))
    }

    /// Sets the receive windows.
    ///
    /// `maxRx1Offset` fixes how wide every RX1 row must be.
    #[napi]
    pub fn rx(
        &mut self,
        rx2_frequency_hz: u32,
        rx2_data_rate: u8,
        max_rx1_offset: u8,
    ) -> Result<()> {
        self.update(|builder| builder.rx(rx2_frequency_hz, rx2_data_rate, max_rx1_offset))
    }

    /// Sets the Class B beacon and whether the plan limits dwell time.
    #[napi]
    pub fn beacon(&mut self, beacon: LoraBeacon, has_dwell_time_limit: Option<bool>) -> Result<()> {
        let entry = CoreBeacon {
            data_rate: beacon.data_rate,
            frequency_hz: beacon.frequency_hz,
            ping_slot_frequency_hz: beacon.ping_slot_frequency_hz,
        };
        self.update(|builder| {
            builder
                .beacon(entry)
                .dwell_time_limit(has_dwell_time_limit.unwrap_or(false))
        })
    }

    /// Finishes the plan.
    ///
    /// Throws if the plan would answer a question wrongly, for example because an
    /// RX1 row is narrower than the plan's offsets allow, or because the second
    /// receive window listens at a data rate the plan does not define.
    #[napi]
    pub fn build(&mut self) -> Result<LoraChannelPlan> {
        let taken = self.inner.take().ok_or_else(|| {
            Error::new(
                Status::InvalidArg,
                "this builder has already been built".to_owned(),
            )
        })?;
        match taken.build() {
            Ok(inner) => Ok(LoraChannelPlan { inner }),
            Err(error) => Err(Error::new(Status::InvalidArg, error.to_string())),
        }
    }
}
