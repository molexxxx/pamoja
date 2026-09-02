//! A channel plan that owns its tables, for hosts that assemble one at runtime.
//!
//! [`ChannelPlan`] borrows its tables, which is what keeps the published plans
//! free of allocation and usable on a microcontroller. A host reading a plan out
//! of a configuration file, or building one across a language boundary, has
//! nowhere to put those tables: it needs storage that outlives the call that
//! created it. [`OwnedChannelPlan`] is that storage, and
//! [`ChannelPlanBuilder`] assembles one.
//!
//! This is the same capability the published regions have, not a lesser one. A
//! deployment holding licensed spectrum, or working somewhere no published plan
//! describes, gets every answer a named region gives.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::{Beacon, ChannelBlock, ChannelPlan, DataRate, MaxPayload, SubBand};

/// Which of a plan's payload tables an entry belongs to.
///
/// A region publishes separate limits for a device that may sit behind a
/// repeater and one that will not, in each direction, plus a fifth table where
/// a dwell-time limit applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PayloadTable {
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

/// Why a plan could not be built.
///
/// Every variant describes a plan that would answer some question wrongly, so it
/// is refused at the point it is assembled rather than at the question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The plan defines no data rates, so it can answer nothing.
    NoDataRates,
    /// The number of RX1 rows does not match the number of uplink data rates.
    Rx1RowCount {
        /// How many rows the plan carries.
        rows: usize,
        /// How many it needs, one per uplink data rate.
        expected: usize,
        /// Whether this is the dwell-limited mapping rather than the ordinary one.
        dwell_limited: bool,
    },
    /// An RX1 row is not as wide as the plan's highest offset allows.
    Rx1RowWidth {
        /// The row's position, which is the uplink data rate it maps.
        row: usize,
        /// How many entries the row carries.
        width: usize,
        /// How many it needs, one per allowed offset.
        expected: usize,
        /// Whether this is the dwell-limited mapping rather than the ordinary one.
        dwell_limited: bool,
    },
    /// A table's length does not match the data-rate table it indexes.
    TableLength {
        /// How many entries the table carries.
        length: usize,
        /// How many data rates it must cover.
        expected: usize,
    },
    /// The second receive window listens at a data rate the plan does not define.
    Rx2DataRate {
        /// The data rate RX2 was set to.
        data_rate: u8,
        /// How many downlink data rates the plan defines.
        defined: usize,
    },
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDataRates => write!(f, "a channel plan needs at least one data rate"),
            Self::Rx1RowCount {
                rows,
                expected,
                dwell_limited,
            } => {
                let which = if *dwell_limited { "dwell-limited " } else { "" };
                write!(
                    f,
                    "a plan needs one {which}RX1 row per uplink data rate: {expected} data rates, {rows} rows"
                )
            }
            Self::Rx1RowWidth {
                row,
                width,
                expected,
                dwell_limited,
            } => {
                let which = if *dwell_limited { "dwell-limited " } else { "" };
                write!(
                    f,
                    "{which}RX1 row {row} has {width} entries, but the plan's offsets need {expected}"
                )
            }
            Self::TableLength { length, expected } => {
                write!(f, "a table has {length} entries for {expected} data rates")
            }
            Self::Rx2DataRate { data_rate, defined } => write!(
                f,
                "RX2 listens at data rate {data_rate}, but the plan defines {defined}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PlanError {}

/// A channel plan that owns its tables.
///
/// Query it through [`with_plan`](Self::with_plan), which lends the tables to a
/// borrowed [`ChannelPlan`] for the duration of one call.
#[derive(Clone, Debug)]
pub struct OwnedChannelPlan {
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
    rx1_rows: Vec<Box<[u8]>>,
    rx1_rows_dwell_limited: Option<Vec<Box<[u8]>>>,
    max_rx1_data_rate_offset: u8,
    rx2_frequency_hz: u32,
    rx2_data_rate: u8,
    data_rate_backoff: Vec<Option<u8>>,
    beacon: Beacon,
    has_dwell_time_limit: bool,
}

impl OwnedChannelPlan {
    /// Copies a borrowed plan into owned storage.
    ///
    /// This is how a host takes a published region and holds onto it: the result
    /// is independent of where the original tables lived, so one type serves both
    /// a named region and a plan built here.
    ///
    /// # Arguments
    ///
    /// * `plan` - the plan to copy.
    ///
    /// # Returns
    ///
    /// An owned copy answering exactly what the original does.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "eu868")] {
    /// use pamoja_lora::region::{OwnedChannelPlan, Region};
    ///
    /// let held = OwnedChannelPlan::from_plan(Region::Eu868.plan());
    /// assert_eq!(held.with_plan(|plan| plan.rx2()), (869_525_000, 0));
    /// # }
    /// ```
    pub fn from_plan(plan: &ChannelPlan<'_>) -> Self {
        Self {
            name: plan.name.into(),
            uplink_data_rates: plan.uplink_data_rates.to_vec(),
            downlink_data_rates: plan.downlink_data_rates.to_vec(),
            max_payload_repeater: plan.max_payload_repeater.to_vec(),
            max_payload_direct: plan.max_payload_direct.to_vec(),
            downlink_max_payload_repeater: plan.downlink_max_payload_repeater.to_vec(),
            downlink_max_payload_direct: plan.downlink_max_payload_direct.to_vec(),
            max_payload_dwell_limited: plan.max_payload_dwell_limited.map(<[_]>::to_vec),
            join_channels: plan.join_channels.to_vec(),
            default_channels: plan.default_channels.to_vec(),
            sub_bands: plan.sub_bands.to_vec(),
            default_max_eirp_dbm: plan.default_max_eirp_dbm,
            tx_power_step_db: plan.tx_power_step_db,
            max_tx_power_index: plan.max_tx_power_index,
            rx1_rows: plan
                .rx1_data_rate_offsets
                .iter()
                .map(|&r| r.into())
                .collect(),
            rx1_rows_dwell_limited: plan
                .rx1_data_rate_offsets_dwell_limited
                .map(|rows| rows.iter().map(|&r| r.into()).collect()),
            max_rx1_data_rate_offset: plan.max_rx1_data_rate_offset,
            rx2_frequency_hz: plan.rx2_frequency_hz,
            rx2_data_rate: plan.rx2_data_rate,
            data_rate_backoff: plan.data_rate_backoff.to_vec(),
            beacon: plan.beacon,
            has_dwell_time_limit: plan.has_dwell_time_limit,
        }
    }

    /// Lends the owned tables to a borrowed plan for one query.
    ///
    /// The row pointers a plan needs are assembled on the stack for the call, so
    /// nothing outlives it and the storage stays here.
    ///
    /// # Arguments
    ///
    /// * `query` - what to ask the plan.
    ///
    /// # Returns
    ///
    /// Whatever the query returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "in865")] {
    /// use pamoja_lora::region::{OwnedChannelPlan, Region};
    ///
    /// let held = OwnedChannelPlan::from_plan(Region::In865.plan());
    /// let name = held.with_plan(|plan| plan.name.to_owned());
    /// assert_eq!(name, "IN865");
    /// # }
    /// ```
    pub fn with_plan<R>(&self, query: impl FnOnce(&ChannelPlan<'_>) -> R) -> R {
        let rx1: Vec<&[u8]> = self.rx1_rows.iter().map(|row| &row[..]).collect();
        let dwell_rx1: Option<Vec<&[u8]>> = self
            .rx1_rows_dwell_limited
            .as_ref()
            .map(|rows| rows.iter().map(|row| &row[..]).collect());
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
        query(&plan)
    }
}

/// Assembles a [`OwnedChannelPlan`] a table at a time.
///
/// Tables are indexed by position, so entries are pushed in data-rate order and
/// a number the plan does not use is pushed as `None`. What a region would share
/// between directions is filled in at [`build`](Self::build) rather than being
/// repeated here.
///
/// # Examples
///
/// ```
/// use pamoja_lora::region::{
///     ChannelBlock, ChannelPlanBuilder, DataRate, MaxPayload, PayloadTable, SubBand,
/// };
///
/// // A private deployment on licensed spectrum: two data rates and no duty cycle.
/// let plan = ChannelPlanBuilder::new("private-915")
///     .uplink_data_rate(Some(DataRate::lora(12, 125_000, 250)))
///     .uplink_data_rate(Some(DataRate::lora(7, 125_000, 5_470)))
///     .max_payload(PayloadTable::UplinkDirect, Some(MaxPayload::new(59, 51)))
///     .max_payload(PayloadTable::UplinkDirect, Some(MaxPayload::new(230, 222)))
///     .default_channel(ChannelBlock::new(915_000_000, 500_000, 4, 0, 1))
///     .sub_band(SubBand::new(915_000_000, 917_000_000, 1000, 30))
///     .rx(915_000_000, 0, 0)
///     .rx1_row(&[0])
///     .rx1_row(&[1])
///     .build()
///     .expect("a consistent plan");
///
/// // Licensed spectrum is reported as unrestricted, not refused.
/// assert_eq!(plan.with_plan(|p| p.duty_cycle_permille(915_500_000)), Some(1000));
/// assert_eq!(plan.with_plan(|p| p.default_channel_count()), 4);
/// ```
#[derive(Clone, Debug)]
pub struct ChannelPlanBuilder {
    plan: OwnedChannelPlan,
}

impl ChannelPlanBuilder {
    /// Starts an empty plan.
    ///
    /// The plan begins with no data rates, channels, or sub-bands, a two-decibel
    /// power ladder, and no dwell-time limit.
    ///
    /// # Arguments
    ///
    /// * `name` - what to call the plan, such as the band it covers.
    ///
    /// # Returns
    ///
    /// The builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            plan: OwnedChannelPlan {
                name: name.into(),
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
        }
    }

    /// Appends the next uplink data rate.
    ///
    /// # Arguments
    ///
    /// * `rate` - the data rate, or `None` for a number the plan reserves.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn uplink_data_rate(mut self, rate: Option<DataRate>) -> Self {
        self.plan.uplink_data_rates.push(rate);
        self
    }

    /// Appends the next downlink data rate.
    ///
    /// A plan that never calls this uses its uplink table in both directions,
    /// which is what every region but the 900 MHz plans does.
    ///
    /// # Arguments
    ///
    /// * `rate` - the data rate, or `None` for a number the plan reserves.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn downlink_data_rate(mut self, rate: Option<DataRate>) -> Self {
        self.plan.downlink_data_rates.push(rate);
        self
    }

    /// Appends the next entry of one payload table.
    ///
    /// A downlink table left empty mirrors the matching uplink one.
    ///
    /// # Arguments
    ///
    /// * `table` - which table the entry belongs to.
    /// * `payload` - the limits, or `None` where the data rate carries nothing.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn max_payload(mut self, table: PayloadTable, payload: Option<MaxPayload>) -> Self {
        match table {
            PayloadTable::UplinkRepeater => self.plan.max_payload_repeater.push(payload),
            PayloadTable::UplinkDirect => self.plan.max_payload_direct.push(payload),
            PayloadTable::DownlinkRepeater => self.plan.downlink_max_payload_repeater.push(payload),
            PayloadTable::DownlinkDirect => self.plan.downlink_max_payload_direct.push(payload),
            PayloadTable::DwellLimited => self
                .plan
                .max_payload_dwell_limited
                .get_or_insert_with(Vec::new)
                .push(payload),
        }
        self
    }

    /// Adds a run of channels a device may send a join request on.
    ///
    /// # Arguments
    ///
    /// * `block` - the channels to add.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn join_channel(mut self, block: ChannelBlock) -> Self {
        self.plan.join_channels.push(block);
        self
    }

    /// Adds a run of channels a device starts with.
    ///
    /// # Arguments
    ///
    /// * `block` - the channels to add.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn default_channel(mut self, block: ChannelBlock) -> Self {
        self.plan.default_channels.push(block);
        self
    }

    /// Adds a sub-band and the transmit limits inside it.
    ///
    /// A deployment on licensed spectrum gives its sub-band a duty cycle of
    /// `1000`, which reports as unrestricted.
    ///
    /// # Arguments
    ///
    /// * `band` - the sub-band to add.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn sub_band(mut self, band: SubBand) -> Self {
        self.plan.sub_bands.push(band);
        self
    }

    /// Appends the RX1 downlink data rates for the next uplink data rate.
    ///
    /// # Arguments
    ///
    /// * `offsets` - the downlink data rate at each RX1 offset, in order.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn rx1_row(mut self, offsets: &[u8]) -> Self {
        self.plan.rx1_rows.push(offsets.into());
        self
    }

    /// Appends the dwell-limited RX1 downlink data rates for the next uplink
    /// data rate.
    ///
    /// # Arguments
    ///
    /// * `offsets` - the downlink data rate at each RX1 offset, in order.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn rx1_row_dwell_limited(mut self, offsets: &[u8]) -> Self {
        self.plan
            .rx1_rows_dwell_limited
            .get_or_insert_with(Vec::new)
            .push(offsets.into());
        self
    }

    /// Appends the next entry of the adaptive back-off chain.
    ///
    /// A chain left empty steps down one data rate at a time.
    ///
    /// # Arguments
    ///
    /// * `lower` - the data rate to fall back to, or `None` at the slowest.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn backoff(mut self, lower: Option<u8>) -> Self {
        self.plan.data_rate_backoff.push(lower);
        self
    }

    /// Sets the transmit-power ladder.
    ///
    /// # Arguments
    ///
    /// * `default_max_eirp_dbm` - the ceiling where no sub-band says otherwise.
    /// * `step_db` - the step between power settings, in decibels.
    /// * `max_index` - the highest power index the plan defines.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn power(mut self, default_max_eirp_dbm: i8, step_db: u8, max_index: u8) -> Self {
        self.plan.default_max_eirp_dbm = default_max_eirp_dbm;
        self.plan.tx_power_step_db = step_db;
        self.plan.max_tx_power_index = max_index;
        self
    }

    /// Sets the receive windows.
    ///
    /// # Arguments
    ///
    /// * `rx2_frequency_hz` - the fixed frequency the second window listens on.
    /// * `rx2_data_rate` - the data rate the second window listens at.
    /// * `max_rx1_offset` - the highest RX1 offset the plan allows, which fixes
    ///   how wide every RX1 row must be.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn rx(mut self, rx2_frequency_hz: u32, rx2_data_rate: u8, max_rx1_offset: u8) -> Self {
        self.plan.rx2_frequency_hz = rx2_frequency_hz;
        self.plan.rx2_data_rate = rx2_data_rate;
        self.plan.max_rx1_data_rate_offset = max_rx1_offset;
        self
    }

    /// Sets the Class B beacon.
    ///
    /// # Arguments
    ///
    /// * `beacon` - the beacon settings.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn beacon(mut self, beacon: Beacon) -> Self {
        self.plan.beacon = beacon;
        self
    }

    /// Sets whether the plan limits how long one transmission may hold a channel.
    ///
    /// # Arguments
    ///
    /// * `limited` - whether a dwell-time limit applies.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn dwell_time_limit(mut self, limited: bool) -> Self {
        self.plan.has_dwell_time_limit = limited;
        self
    }

    /// Finishes the plan.
    ///
    /// Tables a region would share are filled in first: an empty downlink
    /// data-rate table mirrors the uplink one, an empty downlink payload table
    /// mirrors its uplink counterpart, and an empty back-off chain steps down one
    /// data rate at a time. What cannot be inferred is checked.
    ///
    /// # Returns
    ///
    /// The finished plan.
    ///
    /// # Errors
    ///
    /// Returns the [`PlanError`] describing the question this plan would answer
    /// wrongly.
    pub fn build(self) -> Result<OwnedChannelPlan, PlanError> {
        let mut plan = self.plan;

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

        check(&plan)?;
        Ok(plan)
    }
}

/// Checks that a plan can answer every question asked of it.
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
/// Returns the [`PlanError`] describing what is inconsistent.
fn check(plan: &OwnedChannelPlan) -> Result<(), PlanError> {
    let rates = plan.uplink_data_rates.len();
    if rates == 0 {
        return Err(PlanError::NoDataRates);
    }

    let width = usize::from(plan.max_rx1_data_rate_offset) + 1;
    for (dwell_limited, rows) in [
        (false, Some(&plan.rx1_rows)),
        (true, plan.rx1_rows_dwell_limited.as_ref()),
    ] {
        let Some(rows) = rows else {
            continue;
        };
        if rows.len() != rates {
            return Err(PlanError::Rx1RowCount {
                rows: rows.len(),
                expected: rates,
                dwell_limited,
            });
        }
        for (row, entries) in rows.iter().enumerate() {
            if entries.len() != width {
                return Err(PlanError::Rx1RowWidth {
                    row,
                    width: entries.len(),
                    expected: width,
                    dwell_limited,
                });
            }
        }
    }

    if plan.data_rate_backoff.len() != rates {
        return Err(PlanError::TableLength {
            length: plan.data_rate_backoff.len(),
            expected: rates,
        });
    }

    for table in [&plan.max_payload_repeater, &plan.max_payload_direct] {
        if !table.is_empty() && table.len() != rates {
            return Err(PlanError::TableLength {
                length: table.len(),
                expected: rates,
            });
        }
    }
    if let Some(table) = &plan.max_payload_dwell_limited {
        if table.len() != rates {
            return Err(PlanError::TableLength {
                length: table.len(),
                expected: rates,
            });
        }
    }

    let downlink = plan.downlink_data_rates.len();
    for table in [
        &plan.downlink_max_payload_repeater,
        &plan.downlink_max_payload_direct,
    ] {
        if !table.is_empty() && table.len() != downlink {
            return Err(PlanError::TableLength {
                length: table.len(),
                expected: downlink,
            });
        }
    }

    if usize::from(plan.rx2_data_rate) >= downlink {
        return Err(PlanError::Rx2DataRate {
            data_rate: plan.rx2_data_rate,
            defined: downlink,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal plan that passes every check, for tests that then break one
    /// thing about it.
    fn minimal() -> ChannelPlanBuilder {
        ChannelPlanBuilder::new("test")
            .uplink_data_rate(Some(DataRate::lora(12, 125_000, 250)))
            .uplink_data_rate(Some(DataRate::lora(7, 125_000, 5_470)))
            .rx(915_000_000, 0, 0)
            .rx1_row(&[0])
            .rx1_row(&[1])
    }

    #[test]
    fn an_empty_plan_is_refused() {
        assert_eq!(
            ChannelPlanBuilder::new("empty").build().unwrap_err(),
            PlanError::NoDataRates
        );
    }

    #[test]
    fn the_downlink_tables_mirror_the_uplink_ones_when_left_empty() {
        let plan = minimal()
            .max_payload(PayloadTable::UplinkDirect, Some(MaxPayload::new(59, 51)))
            .max_payload(PayloadTable::UplinkDirect, Some(MaxPayload::new(230, 222)))
            .build()
            .expect("consistent");

        assert_eq!(
            plan.with_plan(|plan| plan.downlink_max_payload(1, false)),
            Some(MaxPayload::new(230, 222))
        );
        assert_eq!(
            plan.with_plan(|plan| plan.downlink_data_rate(1)),
            Some(DataRate::lora(7, 125_000, 5_470))
        );
    }

    #[test]
    fn an_unset_backoff_chain_steps_down_one_rate_at_a_time() {
        let plan = minimal().build().expect("consistent");
        assert_eq!(
            plan.with_plan(|plan| plan.next_backoff_data_rate(1)),
            Some(0)
        );
        assert_eq!(plan.with_plan(|plan| plan.next_backoff_data_rate(0)), None);
    }

    #[test]
    fn a_row_narrower_than_the_offsets_allow_is_refused() {
        // Offsets up to 5 mean every row needs six entries.
        let error = minimal().rx(915_000_000, 0, 5).build().unwrap_err();
        assert_eq!(
            error,
            PlanError::Rx1RowWidth {
                row: 0,
                width: 1,
                expected: 6,
                dwell_limited: false,
            }
        );
    }

    #[test]
    fn a_missing_rx1_row_is_refused() {
        let error = ChannelPlanBuilder::new("short")
            .uplink_data_rate(Some(DataRate::lora(12, 125_000, 250)))
            .uplink_data_rate(Some(DataRate::lora(7, 125_000, 5_470)))
            .rx(915_000_000, 0, 0)
            .rx1_row(&[0])
            .build()
            .unwrap_err();
        assert_eq!(
            error,
            PlanError::Rx1RowCount {
                rows: 1,
                expected: 2,
                dwell_limited: false,
            }
        );
    }

    #[test]
    fn listening_at_a_data_rate_the_plan_lacks_is_refused() {
        let error = minimal().rx(915_000_000, 9, 0).build().unwrap_err();
        assert_eq!(
            error,
            PlanError::Rx2DataRate {
                data_rate: 9,
                defined: 2,
            }
        );
    }

    #[test]
    fn a_dwell_limited_mapping_is_checked_like_the_ordinary_one() {
        let error = minimal().rx1_row_dwell_limited(&[0]).build().unwrap_err();
        assert_eq!(
            error,
            PlanError::Rx1RowCount {
                rows: 1,
                expected: 2,
                dwell_limited: true,
            }
        );
    }

    #[test]
    #[cfg(feature = "au915")]
    fn a_published_plan_survives_the_round_trip_into_owned_storage() {
        use super::super::Region;

        // AU915 exercises the awkward parts: separate downlink data rates, a
        // dwell-limited payload table, and a wide RX1 mapping.
        let published = Region::Au915.plan();
        let owned = OwnedChannelPlan::from_plan(published);

        owned.with_plan(|copy| {
            assert_eq!(copy.name, published.name);
            assert_eq!(copy.rx2(), published.rx2());
            assert_eq!(
                copy.default_channel_count(),
                published.default_channel_count()
            );
            for data_rate in 0..16 {
                assert_eq!(
                    copy.uplink_data_rate(data_rate),
                    published.uplink_data_rate(data_rate)
                );
                assert_eq!(
                    copy.downlink_data_rate(data_rate),
                    published.downlink_data_rate(data_rate)
                );
                assert_eq!(
                    copy.max_payload(data_rate, true),
                    published.max_payload(data_rate, true)
                );
                assert_eq!(
                    copy.max_payload_dwell_limited(data_rate),
                    published.max_payload_dwell_limited(data_rate)
                );
                for offset in 0..8 {
                    assert_eq!(
                        copy.rx1_data_rate(data_rate, offset),
                        published.rx1_data_rate(data_rate, offset)
                    );
                }
            }
        });
    }
}
