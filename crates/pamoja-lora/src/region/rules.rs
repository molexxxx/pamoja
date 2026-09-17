//! The rules a device follows on a plan beyond its tables: what a channel mask control does,
//! the order join channels are tried in, what a power index counts down from, and which plan
//! a join lands on where a region has several.

use super::{ChannelBlock, ChannelPlan};

/// What one value of a `LinkADRReq` command's `ChMaskCntl` field does to a device's channels.
///
/// TS001-1.0.4 section 5.3 leaves the meaning to the regional parameters, and RP002-1.0.5
/// gives each region a table of eight. A plan carries its table as
/// [`ChannelPlan::mask_controls`], indexed by the field's value.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "us915")] {
/// use pamoja_lora::region::{MaskControl, Region};
///
/// // RP002-1.0.5 table 23: in US902-928, 7 turns every 125 kHz channel off and sets the
/// // 500 kHz channels from the mask.
/// assert_eq!(
///     Region::Us915.plan().mask_controls[7],
///     MaskControl::All { on: false, then_group: Some(4) }
/// );
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaskControl {
    /// The mask sets the sixteen channels of one group, channels `16 * group` to
    /// `16 * group + 15`.
    Group(u8),
    /// Each of the mask's ten low bits switches a bank of eight channels, bank `b` being
    /// channels `8 * b` to `8 * b + 7`. Channels a bank names but the device has not defined
    /// stay off.
    Banks,
    /// Each of the mask's eight low bits switches a bank of eight channels together with the
    /// channel numbered 64 plus the bit, and bit 9 switches channels 72 to 79 where they are
    /// defined. This is the 900 MHz plans' pairing of eight 125 kHz channels with one 500 kHz
    /// channel.
    PairedBanks,
    /// Every defined channel on, or every channel off, and then, where a group is named, the
    /// mask sets that group.
    All {
        /// Whether the channels go on or off.
        on: bool,
        /// The group the mask then sets, if any.
        then_group: Option<u8>,
    },
    /// A value the region reserves, which a device refuses.
    Reserved,
}

impl MaskControl {
    /// The table every dynamic plan shares, RP002-1.0.5 table 14 for EU863-870 and its
    /// counterparts: groups 0 to 4, banks of eight, every defined channel on, and a reserved 7.
    pub const DYNAMIC: [MaskControl; 8] = [
        MaskControl::Group(0),
        MaskControl::Group(1),
        MaskControl::Group(2),
        MaskControl::Group(3),
        MaskControl::Group(4),
        MaskControl::Banks,
        MaskControl::All {
            on: true,
            then_group: None,
        },
        MaskControl::Reserved,
    ];
}

/// The order a device tries a plan's join channels in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum JoinSequence {
    /// A join channel at random for each attempt, with the data rate stepping from the
    /// fastest the join channels carry down to the slowest and round again.
    #[default]
    Random,
    /// Passes over the join channels that cover every one before any repeats, RP002-1.0.5
    /// sections 3.5.2 and 3.8.2: each pass takes one channel at random from each group of
    /// eight in the first join block, in order, and then one channel of the blocks after it.
    /// Each attempt goes out at the lowest data rate its channel's block carries.
    OctetPasses,
}

/// What a plan's transmit power indexes count down from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PowerReference {
    /// A radiated ceiling, so a device takes its antenna's gain off the power it asks of its
    /// radio.
    #[default]
    Eirp,
    /// A conducted ceiling, as RP002-1.0.5 table 22 sets for US902-928, so the power index
    /// names the radio's own output.
    Conducted {
        /// The antenna gain the ceiling already allows for, in dB. An antenna with more takes
        /// the excess off the output: RP002-1.0.5 section 3.5.2 summarizes 47 CFR 15.247(b)(4)
        /// as reducing the conducted power by every dB of gain above 6 dBi.
        gain_allowance_db: u8,
    },
}

/// A run of join channels that puts a device on one plan, for a region that picks the plan
/// by the channel a device joined on.
///
/// RP002-1.0.5 section 3.9.2 has CN470-510 devices scan twenty common join channels, each
/// tied to one of four plans. The channel a join accept answers decides the plan, where the
/// accept arrives, and where the second receive window listens afterward.
#[derive(Clone, Copy, Debug)]
pub struct JoinPlan<'a> {
    /// The join channels this run covers, with the data rates a request may use on them.
    pub channels: ChannelBlock,
    /// Where the join accept answering the first channel of the run arrives, in hertz.
    pub accept_start_hz: u32,
    /// How far the accept frequency moves for each next channel, in hertz.
    pub accept_step_hz: u32,
    /// The second receive window's frequency after joining on the first channel, in hertz.
    pub rx2_start_hz: u32,
    /// How far that frequency moves for each next channel, in hertz.
    pub rx2_step_hz: u32,
    /// The plan a device joined on these channels follows.
    pub plan: &'a ChannelPlan<'a>,
}

impl<'a> JoinPlan<'a> {
    /// Builds a run of join channels.
    ///
    /// # Arguments
    ///
    /// * `channels` - the join channels and their data rates.
    /// * `accept` - the accept frequency for the first channel and its step, in hertz.
    /// * `rx2` - the second window's frequency for the first channel and its step, in hertz.
    /// * `plan` - the plan a join on these channels puts a device on.
    ///
    /// # Returns
    ///
    /// The run.
    pub const fn new(
        channels: ChannelBlock,
        accept: (u32, u32),
        rx2: (u32, u32),
        plan: &'a ChannelPlan<'a>,
    ) -> JoinPlan<'a> {
        JoinPlan {
            channels,
            accept_start_hz: accept.0,
            accept_step_hz: accept.1,
            rx2_start_hz: rx2.0,
            rx2_step_hz: rx2.1,
            plan,
        }
    }

    /// Returns where the join accept answering one channel of the run arrives.
    ///
    /// # Arguments
    ///
    /// * `offset` - the channel's position in the run.
    ///
    /// # Returns
    ///
    /// The frequency in hertz, or `None` past the end of the run.
    pub const fn accept_hz(&self, offset: u16) -> Option<u32> {
        if offset >= self.channels.count {
            return None;
        }
        Some(self.accept_start_hz + self.accept_step_hz * offset as u32)
    }

    /// Returns where the second receive window listens after joining on one channel of the run.
    ///
    /// # Arguments
    ///
    /// * `offset` - the channel's position in the run.
    ///
    /// # Returns
    ///
    /// The frequency in hertz, or `None` past the end of the run.
    pub const fn rx2_hz(&self, offset: u16) -> Option<u32> {
        if offset >= self.channels.count {
            return None;
        }
        Some(self.rx2_start_hz + self.rx2_step_hz * offset as u32)
    }
}
