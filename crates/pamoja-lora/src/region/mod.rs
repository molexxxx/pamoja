//! Regional parameters: what a LoRaWAN radio may do, and where.
//!
//! A LoRa radio takes a spreading factor and a bandwidth. A *region* is what
//! decides which of those are legal where the device is standing, what a data
//! rate number means, how much payload fits, and which frequencies a gateway is
//! listening on. Without it a caller has to already know their own channel plan,
//! which is the difference between a stack that works on one continent and one
//! that works anywhere.
//!
//! The tables come from the LoRa Alliance [`RP002-1.0.5` Regional Parameters]
//! specification, and the tests assert the values the document prints rather
//! than round-tripping the implementation against itself.
//!
//! [`RP002-1.0.5` Regional Parameters]: https://resources.lora-alliance.org/technical-specifications/rp002-1-0-5-lorawan-regional-parameters
//!
//! # These tables report; they never enforce
//!
//! Nothing here refuses to transmit, and no call gates on a duty cycle.
//! [`ChannelPlan::duty_cycle_permille`] says what the region specifies and
//! [`LinkSettings::min_off_time_us`](crate::LinkSettings::min_off_time_us) says
//! what that costs; the decision stays with the caller.
//!
//! That is deliberate rather than squeamish. Most of what a regional plan
//! encodes is physics and coordination rather than permission: the bands differ
//! because each regulator left different spectrum unlicensed, a duty cycle is
//! what stops an unlicensed band collapsing under everyone talking at once, and
//! the plan doubles as a description of what a radio front end tuned for that
//! band can physically do. But a node in a disaster zone may be operating under
//! emergency spectrum provisions, or somewhere the question has stopped being
//! meaningful, and a library that refused to transmit there would be harmful
//! exactly where it is needed most. So the tables inform, the arithmetic costs
//! it out, and the operator decides.
//!
//! # A named region is a convenience, not the only way in
//!
//! [`Region`] is a shortcut to a [`ChannelPlan`], which is an ordinary struct of
//! `'static` tables. A private deployment holding licensed spectrum, or bespoke
//! emergency work, builds its own plan from parts and everything here still
//! applies to it.
//!
//! # Examples
//!
//! ```
//! use pamoja_lora::region::{Modulation, Region};
//!
//! let plan = Region::Eu868.plan();
//!
//! // DR5 in Europe is SF7 at 125 kHz.
//! let dr5 = plan.uplink_data_rate(5).expect("EU868 defines DR5");
//! assert_eq!(
//!     dr5.modulation,
//!     Modulation::LoRa { spreading_factor: 7, bandwidth_hz: 125_000 }
//! );
//!
//! // Talking straight to a gateway it carries 242 bytes of application payload,
//! // and 222 if it may sit behind a repeater, which costs 20 bytes to encapsulate.
//! assert_eq!(plan.max_payload(5, false).expect("DR5 carries payload").application, 242);
//! assert_eq!(plan.max_payload(5, true).expect("DR5 carries payload").application, 222);
//!
//! // The airtime math already in this crate takes it from here.
//! let settings = plan.link_settings(5).expect("DR5 is a LoRa data rate");
//! assert!(settings.airtime_us(51) > 0);
//! ```

use crate::LinkSettings;

mod plans;

#[cfg(test)]
mod tests;

pub use plans::Region;

/// How a data rate puts bits on the air.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modulation {
    /// LoRa chirp spread spectrum, the modulation the rest of this crate models.
    LoRa {
        /// The spreading factor, 5 through 12.
        spreading_factor: u8,
        /// The channel bandwidth in hertz.
        bandwidth_hz: u32,
    },
    /// Plain FSK, which one data rate in several regions uses.
    Fsk {
        /// The bit rate in bits per second.
        bitrate_bps: u32,
    },
    /// Long-range frequency hopping spread spectrum.
    LrFhss {
        /// The numerator of the coding rate.
        coding_rate_numerator: u8,
        /// The denominator of the coding rate.
        coding_rate_denominator: u8,
        /// The occupied bandwidth in hertz.
        bandwidth_hz: u32,
    },
}

/// One data rate: how it is modulated and how fast it carries bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataRate {
    /// How the data rate puts bits on the air.
    pub modulation: Modulation,
    /// The indicative physical bit rate the specification prints, in bits per
    /// second.
    pub bitrate_bps: u32,
}

impl DataRate {
    /// Builds a LoRa data rate.
    ///
    /// # Arguments
    ///
    /// * `spreading_factor` - the spreading factor.
    /// * `bandwidth_hz` - the channel bandwidth in hertz.
    /// * `bitrate_bps` - the indicative bit rate the specification prints.
    ///
    /// # Returns
    ///
    /// The data rate.
    pub const fn lora(spreading_factor: u8, bandwidth_hz: u32, bitrate_bps: u32) -> Self {
        Self {
            modulation: Modulation::LoRa {
                spreading_factor,
                bandwidth_hz,
            },
            bitrate_bps,
        }
    }

    /// Builds an FSK data rate.
    ///
    /// # Arguments
    ///
    /// * `bitrate_bps` - the bit rate in bits per second.
    ///
    /// # Returns
    ///
    /// The data rate.
    pub const fn fsk(bitrate_bps: u32) -> Self {
        Self {
            modulation: Modulation::Fsk { bitrate_bps },
            bitrate_bps,
        }
    }

    /// Builds an LR-FHSS data rate.
    ///
    /// # Arguments
    ///
    /// * `numerator` - the coding-rate numerator.
    /// * `denominator` - the coding-rate denominator.
    /// * `bandwidth_hz` - the occupied bandwidth in hertz.
    /// * `bitrate_bps` - the indicative bit rate the specification prints.
    ///
    /// # Returns
    ///
    /// The data rate.
    pub const fn lr_fhss(
        numerator: u8,
        denominator: u8,
        bandwidth_hz: u32,
        bitrate_bps: u32,
    ) -> Self {
        Self {
            modulation: Modulation::LrFhss {
                coding_rate_numerator: numerator,
                coding_rate_denominator: denominator,
                bandwidth_hz,
            },
            bitrate_bps,
        }
    }

    /// Returns the link settings this data rate describes, for the airtime math.
    ///
    /// # Returns
    ///
    /// `Some(settings)` for a LoRa data rate, or `None` for FSK and LR-FHSS,
    /// which this crate's chirp-based airtime model does not describe.
    pub fn link_settings(&self) -> Option<LinkSettings> {
        match self.modulation {
            Modulation::LoRa {
                spreading_factor,
                bandwidth_hz,
            } => Some(LinkSettings::new(spreading_factor, bandwidth_hz)),
            _ => None,
        }
    }
}

/// The largest payload a data rate carries.
///
/// `M` is the MACPayload limit the physical layer imposes. `N` is the
/// application payload that leaves room for the frame header, and shrinks
/// further if the frame carries MAC commands in its `FOpts` field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaxPayload {
    /// The largest MACPayload, in bytes.
    pub mac_payload: u16,
    /// The largest application payload with an empty `FOpts` field, in bytes.
    pub application: u16,
}

impl MaxPayload {
    /// Builds a payload limit from the pair the specification tabulates.
    ///
    /// # Arguments
    ///
    /// * `mac_payload` - the `M` column.
    /// * `application` - the `N` column.
    ///
    /// # Returns
    ///
    /// The limit.
    pub const fn new(mac_payload: u16, application: u16) -> Self {
        Self {
            mac_payload,
            application,
        }
    }
}

/// A run of evenly spaced channels, which is how the plans define them.
///
/// Every region lays its channels out as a start frequency and a fixed step, so
/// a plan carries the arithmetic rather than 72 literal frequencies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelBlock {
    /// The frequency of the first channel in the block, in hertz.
    pub start_hz: u32,
    /// The spacing between channels, in hertz.
    pub step_hz: u32,
    /// How many channels the block holds.
    pub count: u16,
    /// The lowest data rate usable on these channels.
    pub min_data_rate: u8,
    /// The highest data rate usable on these channels.
    pub max_data_rate: u8,
}

impl ChannelBlock {
    /// Builds a block of evenly spaced channels.
    ///
    /// # Arguments
    ///
    /// * `start_hz` - the first channel frequency in hertz.
    /// * `step_hz` - the spacing between channels in hertz.
    /// * `count` - how many channels the block holds.
    /// * `min_data_rate` - the lowest data rate usable on them.
    /// * `max_data_rate` - the highest data rate usable on them.
    ///
    /// # Returns
    ///
    /// The block.
    pub const fn new(
        start_hz: u32,
        step_hz: u32,
        count: u16,
        min_data_rate: u8,
        max_data_rate: u8,
    ) -> Self {
        Self {
            start_hz,
            step_hz,
            count,
            min_data_rate,
            max_data_rate,
        }
    }

    /// Returns the frequency of one channel in the block.
    ///
    /// # Arguments
    ///
    /// * `index` - the channel's position within this block.
    ///
    /// # Returns
    ///
    /// `Some(hz)`, or `None` if `index` is past the end of the block.
    pub const fn frequency_hz(&self, index: u16) -> Option<u32> {
        if index >= self.count {
            return None;
        }
        Some(self.start_hz + self.step_hz * index as u32)
    }
}

/// A stretch of spectrum with its own transmit limits.
///
/// Europe divides its band into sub-bands whose duty cycles and power ceilings
/// differ, so a plan reports them per frequency rather than once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubBand {
    /// The lowest frequency in the sub-band, in hertz, inclusive.
    pub start_hz: u32,
    /// The highest frequency in the sub-band, in hertz, inclusive.
    pub end_hz: u32,
    /// The share of time a transmitter may occupy the band, in parts per
    /// thousand.
    pub duty_cycle_permille: u32,
    /// The power ceiling in the sub-band, in dBm EIRP.
    pub max_eirp_dbm: i8,
}

impl SubBand {
    /// Builds a sub-band.
    ///
    /// # Arguments
    ///
    /// * `start_hz` - the lowest frequency, inclusive.
    /// * `end_hz` - the highest frequency, inclusive.
    /// * `duty_cycle_permille` - the duty-cycle limit in parts per thousand.
    /// * `max_eirp_dbm` - the power ceiling in dBm EIRP.
    ///
    /// # Returns
    ///
    /// The sub-band.
    pub const fn new(
        start_hz: u32,
        end_hz: u32,
        duty_cycle_permille: u32,
        max_eirp_dbm: i8,
    ) -> Self {
        Self {
            start_hz,
            end_hz,
            duty_cycle_permille,
            max_eirp_dbm,
        }
    }

    /// Reports whether a frequency falls inside this sub-band.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the frequency to test.
    ///
    /// # Returns
    ///
    /// `true` when the frequency is within the sub-band, inclusive of both ends.
    pub const fn contains(&self, frequency_hz: u32) -> bool {
        frequency_hz >= self.start_hz && frequency_hz <= self.end_hz
    }
}

/// The Class B beacon settings a region broadcasts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Beacon {
    /// The data rate the beacon is sent at.
    pub data_rate: u8,
    /// The frequency the beacon is broadcast on, in hertz.
    pub frequency_hz: u32,
    /// The default ping-slot frequency, in hertz.
    pub ping_slot_frequency_hz: u32,
}

/// A complete regional channel plan.
///
/// The named [`Region`] values are constants of this type. A deployment on
/// licensed spectrum, or one doing something the published regions do not
/// describe, builds its own from `'static` tables and every method here still
/// applies.
#[derive(Clone, Copy, Debug)]
pub struct ChannelPlan {
    /// The specification's name for the band, such as `"EU863-870"`.
    pub name: &'static str,
    /// The uplink data rates, indexed by data-rate number; `None` where the
    /// number is reserved.
    pub uplink_data_rates: &'static [Option<DataRate>],
    /// The downlink data rates, indexed by data-rate number.
    ///
    /// Most regions use one table in both directions, and carry the same slice
    /// here. The 900 MHz plans do not, which is why this is separate.
    pub downlink_data_rates: &'static [Option<DataRate>],
    /// The uplink payload limits when the device may be behind a repeater.
    pub max_payload_repeater: &'static [Option<MaxPayload>],
    /// The uplink payload limits when it will not be.
    pub max_payload_direct: &'static [Option<MaxPayload>],
    /// The downlink payload limits when the device may be behind a repeater.
    ///
    /// Most regions number their downlink data rates the same way as their
    /// uplink ones and carry the same slice here. The 900 MHz plans do not.
    pub downlink_max_payload_repeater: &'static [Option<MaxPayload>],
    /// The downlink payload limits when it will not be.
    pub downlink_max_payload_direct: &'static [Option<MaxPayload>],
    /// The payload limits under a dwell-time limit, where the region has one.
    pub max_payload_dwell_limited: Option<&'static [Option<MaxPayload>]>,
    /// The channels a device must use to send a join request.
    pub join_channels: &'static [ChannelBlock],
    /// The channels a device starts with before a network adds any.
    pub default_channels: &'static [ChannelBlock],
    /// The sub-bands and their transmit limits.
    pub sub_bands: &'static [SubBand],
    /// The power ceiling assumed when no sub-band says otherwise, in dBm.
    pub default_max_eirp_dbm: i8,
    /// The step between transmit-power settings, in dB.
    pub tx_power_step_db: u8,
    /// The highest transmit-power index the region defines.
    pub max_tx_power_index: u8,
    /// The downlink data rate for each uplink data rate and RX1 offset, as
    /// `[uplink data rate][offset]`.
    pub rx1_data_rate_offsets: &'static [&'static [u8]],
    /// The same mapping under a downlink dwell-time limit, where the region
    /// publishes a second table for it.
    pub rx1_data_rate_offsets_dwell_limited: Option<&'static [&'static [u8]]>,
    /// The highest RX1 data-rate offset the region allows.
    pub max_rx1_data_rate_offset: u8,
    /// The fixed frequency the second receive window listens on, in hertz.
    pub rx2_frequency_hz: u32,
    /// The data rate the second receive window listens at.
    pub rx2_data_rate: u8,
    /// The next lower uplink data rate during adaptive back-off, indexed by the
    /// current data rate; `None` where there is nothing lower.
    pub data_rate_backoff: &'static [Option<u8>],
    /// The Class B beacon settings.
    pub beacon: Beacon,
    /// Whether the region limits how long one transmission may occupy a channel.
    pub has_dwell_time_limit: bool,
}

impl ChannelPlan {
    /// Returns the uplink data rate a number selects.
    ///
    /// # Arguments
    ///
    /// * `data_rate` - the data-rate number.
    ///
    /// # Returns
    ///
    /// `Some(rate)`, or `None` if the number is out of range or reserved in this
    /// region.
    pub fn uplink_data_rate(&self, data_rate: u8) -> Option<DataRate> {
        *self.uplink_data_rates.get(usize::from(data_rate))?
    }

    /// Returns the downlink data rate a number selects.
    ///
    /// # Arguments
    ///
    /// * `data_rate` - the data-rate number.
    ///
    /// # Returns
    ///
    /// `Some(rate)`, or `None` if the number is out of range or reserved.
    pub fn downlink_data_rate(&self, data_rate: u8) -> Option<DataRate> {
        *self.downlink_data_rates.get(usize::from(data_rate))?
    }

    /// Returns the link settings an uplink data rate describes.
    ///
    /// This is the bridge into the airtime and duty-cycle math the rest of the
    /// crate already provides.
    ///
    /// # Arguments
    ///
    /// * `data_rate` - the uplink data-rate number.
    ///
    /// # Returns
    ///
    /// `Some(settings)` for a LoRa data rate, or `None` if the number is not
    /// defined here or names an FSK or LR-FHSS rate, which the chirp airtime
    /// model does not describe.
    pub fn link_settings(&self, data_rate: u8) -> Option<LinkSettings> {
        self.uplink_data_rate(data_rate)?.link_settings()
    }

    /// Returns the largest payload an uplink data rate carries.
    ///
    /// # Arguments
    ///
    /// * `data_rate` - the uplink data-rate number.
    /// * `behind_repeater` - whether the device may operate through a repeater,
    ///   which costs a few bytes of encapsulation at the higher data rates.
    ///
    /// # Returns
    ///
    /// `Some(limit)`, or `None` if the data rate carries no payload here.
    pub fn max_payload(&self, data_rate: u8, behind_repeater: bool) -> Option<MaxPayload> {
        let table = if behind_repeater {
            self.max_payload_repeater
        } else {
            self.max_payload_direct
        };
        *table.get(usize::from(data_rate))?
    }

    /// Returns the largest payload a downlink data rate carries.
    ///
    /// # Arguments
    ///
    /// * `data_rate` - the downlink data-rate number.
    /// * `behind_repeater` - whether the device may operate through a repeater.
    ///
    /// # Returns
    ///
    /// `Some(limit)`, or `None` if the data rate carries no payload here.
    pub fn downlink_max_payload(&self, data_rate: u8, behind_repeater: bool) -> Option<MaxPayload> {
        let table = if behind_repeater {
            self.downlink_max_payload_repeater
        } else {
            self.downlink_max_payload_direct
        };
        *table.get(usize::from(data_rate))?
    }

    /// Returns the largest payload an uplink data rate carries under a dwell-time
    /// limit.
    ///
    /// # Arguments
    ///
    /// * `data_rate` - the uplink data-rate number.
    ///
    /// # Returns
    ///
    /// `Some(limit)`, or `None` if the region has no dwell-time limit or the
    /// data rate carries nothing under one.
    pub fn max_payload_dwell_limited(&self, data_rate: u8) -> Option<MaxPayload> {
        *self
            .max_payload_dwell_limited?
            .get(usize::from(data_rate))?
    }

    /// Returns the duty-cycle limit that applies to a frequency.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the frequency to look up.
    ///
    /// # Returns
    ///
    /// `Some(permille)` for a frequency inside a sub-band this region limits, or
    /// `None` where the region publishes no duty-cycle limit for it. `None` is
    /// not permission; it means the constraint is elsewhere, typically a
    /// dwell-time limit instead.
    pub fn duty_cycle_permille(&self, frequency_hz: u32) -> Option<u32> {
        self.sub_bands
            .iter()
            .find(|band| band.contains(frequency_hz))
            .map(|band| band.duty_cycle_permille)
    }

    /// Returns the power ceiling that applies to a frequency, in dBm.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the frequency to look up.
    ///
    /// # Returns
    ///
    /// The sub-band's ceiling, or the region default where no sub-band covers
    /// the frequency.
    pub fn max_eirp_dbm(&self, frequency_hz: u32) -> i8 {
        self.sub_bands
            .iter()
            .find(|band| band.contains(frequency_hz))
            .map_or(self.default_max_eirp_dbm, |band| band.max_eirp_dbm)
    }

    /// Returns the radiated power a transmit-power index selects, in dBm.
    ///
    /// # Arguments
    ///
    /// * `index` - the `TXPower` index from a `LinkADRReq`.
    /// * `max_eirp_dbm` - the ceiling the index counts down from, usually
    ///   [`max_eirp_dbm`](Self::max_eirp_dbm) for the frequency in use.
    ///
    /// # Returns
    ///
    /// `Some(dbm)`, or `None` if the index is above what the region defines.
    pub fn tx_power_dbm(&self, index: u8, max_eirp_dbm: i8) -> Option<i8> {
        if index > self.max_tx_power_index {
            return None;
        }
        let step = i16::from(self.tx_power_step_db) * i16::from(index);
        Some((i16::from(max_eirp_dbm) - step) as i8)
    }

    /// Returns the downlink data rate the first receive window uses.
    ///
    /// # Arguments
    ///
    /// * `uplink_data_rate` - the data rate the uplink was sent at.
    /// * `offset` - the `RX1DROffset` in force.
    ///
    /// # Returns
    ///
    /// `Some(data_rate)`, or `None` if either argument is outside what the
    /// region defines.
    pub fn rx1_data_rate(&self, uplink_data_rate: u8, offset: u8) -> Option<u8> {
        if offset > self.max_rx1_data_rate_offset {
            return None;
        }
        self.rx1_data_rate_offsets
            .get(usize::from(uplink_data_rate))?
            .get(usize::from(offset))
            .copied()
    }

    /// Returns the first receive window's data rate under a dwell-time limit.
    ///
    /// # Arguments
    ///
    /// * `uplink_data_rate` - the data rate the uplink was sent at.
    /// * `offset` - the `RX1DROffset` in force.
    ///
    /// # Returns
    ///
    /// `Some(data_rate)`, or `None` if the region publishes no dwell-limited
    /// mapping or either argument is outside what it defines.
    pub fn rx1_data_rate_dwell_limited(&self, uplink_data_rate: u8, offset: u8) -> Option<u8> {
        if offset > self.max_rx1_data_rate_offset {
            return None;
        }
        self.rx1_data_rate_offsets_dwell_limited?
            .get(usize::from(uplink_data_rate))?
            .get(usize::from(offset))
            .copied()
    }

    /// Returns the frequency and data rate of the second receive window.
    ///
    /// # Returns
    ///
    /// The frequency in hertz and the data-rate number.
    pub fn rx2(&self) -> (u32, u8) {
        (self.rx2_frequency_hz, self.rx2_data_rate)
    }

    /// Returns the next data rate down during adaptive back-off.
    ///
    /// # Arguments
    ///
    /// * `data_rate` - the data rate currently in use.
    ///
    /// # Returns
    ///
    /// `Some(next)`, or `None` when the device is already at the lowest rate the
    /// region backs off to.
    pub fn next_backoff_data_rate(&self, data_rate: u8) -> Option<u8> {
        *self.data_rate_backoff.get(usize::from(data_rate))?
    }

    /// Returns the frequency of a channel by its number across the whole plan.
    ///
    /// Channel numbers run through the default blocks in order, which is the
    /// numbering `LinkADRReq` channel masks use.
    ///
    /// # Arguments
    ///
    /// * `channel` - the channel number.
    ///
    /// # Returns
    ///
    /// `Some(hz)`, or `None` if the plan defines no such channel by default.
    pub fn channel_frequency_hz(&self, channel: u16) -> Option<u32> {
        let mut remaining = channel;
        for block in self.default_channels {
            if remaining < block.count {
                return block.frequency_hz(remaining);
            }
            remaining -= block.count;
        }
        None
    }

    /// Returns how many channels the plan defines by default.
    ///
    /// # Returns
    ///
    /// The channel count.
    pub fn default_channel_count(&self) -> u16 {
        self.default_channels
            .iter()
            .map(|block| block.count)
            .sum::<u16>()
    }
}
