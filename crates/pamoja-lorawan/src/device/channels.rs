//! The channels a device may transmit on, and how a network changes them.
//!
//! In a dynamic channel plan a device starts with the region's default channels, and a
//! network adds more, moves their downlink frequency, and enables or disables them. In a fixed
//! plan every channel is numbered in advance, each answers on a downlink channel the plan
//! names, and a network only enables and disables them. RP002-1.0.5 gives the dynamic plans up
//! to 80 channels and the fixed ones 72, and the 96-channel CN470-510 plan of the LoRaWAN 1.0.3
//! Regional Parameters, revision A, addresses 96, so that is the capacity here.

use pamoja_lora::region::{ChannelPlan, MaskControl, PlanKind};

use crate::cflist::{CfList, CfListKind};

/// How many channels a device keeps.
pub const MAX_CHANNELS: usize = 96;

/// The mask groups `LinkADRReq` addresses, sixteen channels each.
pub(crate) const MASK_GROUPS: usize = MAX_CHANNELS / 16;

/// The data rates a channel a join accept creates carries: RP002-1.0.5 section 3.3.1 makes
/// them "usable for DR0 to DR5 125 kHz LoRa modulation".
const CREATED_DATA_RATES: (u8, u8) = (0, 5);

/// One channel a device can transmit on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Channel {
    /// Where uplinks go out, in hertz.
    pub uplink_hz: u32,
    /// Where the first receive window listens, in hertz: the uplink frequency unless a
    /// network moved it with `DlChannelReq`, or the downlink channel a fixed plan answers the
    /// channel on.
    pub downlink_hz: u32,
    /// The slowest data rate the channel carries.
    pub min_data_rate: u8,
    /// The fastest data rate the channel carries.
    pub max_data_rate: u8,
}

impl Channel {
    /// A channel whose downlink is on its uplink frequency.
    ///
    /// # Arguments
    ///
    /// * `hz` - the frequency, in hertz.
    /// * `min_data_rate` - the slowest data rate it carries.
    /// * `max_data_rate` - the fastest data rate it carries.
    ///
    /// # Returns
    ///
    /// The channel.
    pub const fn new(hz: u32, min_data_rate: u8, max_data_rate: u8) -> Channel {
        Channel {
            uplink_hz: hz,
            downlink_hz: hz,
            min_data_rate,
            max_data_rate,
        }
    }

    /// Reports whether the channel carries a data rate.
    ///
    /// # Arguments
    ///
    /// * `data_rate` - the data rate.
    ///
    /// # Returns
    ///
    /// Whether it falls in the channel's range.
    pub const fn carries(&self, data_rate: u8) -> bool {
        data_rate >= self.min_data_rate && data_rate <= self.max_data_rate
    }
}

/// A device's channel table and which of them are enabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Channels {
    slots: [Option<Channel>; MAX_CHANNELS],
    enabled: [u16; MASK_GROUPS],
    defaults: usize,
}

impl Channels {
    /// The region's default channels, every one enabled, each answering where the plan says.
    pub(crate) fn defaults(plan: &ChannelPlan) -> Channels {
        let mut channels = Channels {
            slots: [None; MAX_CHANNELS],
            enabled: [0; MASK_GROUPS],
            defaults: 0,
        };
        let mut index = 0;
        for block in plan.default_channels {
            for offset in 0..block.count {
                if index >= MAX_CHANNELS {
                    break;
                }
                if let Some(hz) = block.frequency_hz(offset) {
                    let mut channel = Channel::new(hz, block.min_data_rate, block.max_data_rate);
                    channel.downlink_hz = plan.rx1_frequency_hz(index as u16, hz).unwrap_or(hz);
                    channels.slots[index] = Some(channel);
                    channels.set_bit(index, true);
                }
                index += 1;
            }
        }
        channels.defaults = index;
        channels
    }

    /// How many of the channels are the region's defaults, which lead the table.
    pub(crate) const fn default_count(&self) -> usize {
        self.defaults
    }

    pub(crate) fn get(&self, index: usize) -> Option<Channel> {
        self.slots.get(index).copied().flatten()
    }

    pub(crate) fn is_enabled(&self, index: usize) -> bool {
        index < MAX_CHANNELS && self.enabled[index / 16] & (1 << (index % 16)) != 0
    }

    /// The enabled channels, with their indexes.
    pub(crate) fn enabled(&self) -> impl Iterator<Item = (usize, Channel)> + '_ {
        (0..MAX_CHANNELS).filter_map(move |index| {
            self.get(index)
                .filter(|_| self.is_enabled(index))
                .map(|channel| (index, channel))
        })
    }

    /// The enabled-channel mask, sixteen channels a group.
    pub(crate) const fn mask(&self) -> [u16; MASK_GROUPS] {
        self.enabled
    }

    /// Reports whether any channel enabled under `mask` carries a data rate.
    pub(crate) fn carries(&self, mask: &[u16; MASK_GROUPS], data_rate: u8) -> bool {
        (0..MAX_CHANNELS).any(|index| {
            mask[index / 16] & (1 << (index % 16)) != 0
                && self
                    .get(index)
                    .is_some_and(|channel| channel.carries(data_rate))
        })
    }

    /// Re-enables every default channel and leaves the others as they are.
    ///
    /// This is what TS001-1.0.4 section 4.3.1.1 has a device do at the end of its back-off:
    /// in a dynamic plan "enable the region's default channels and make no change to the
    /// configuration of the dynamically configured channels", and in a fixed plan, whose
    /// channels are all defaults, "enable all channels".
    pub(crate) fn enable_defaults(&mut self) {
        for index in 0..self.defaults {
            if self.get(index).is_some() {
                self.set_bit(index, true);
            }
        }
    }

    /// Takes the channel list a join accept carried.
    ///
    /// In a dynamic plan, RP002-1.0.5 section 3.3.1: the list replaces every channel but the
    /// defaults; a type 0 list defines the five after them, and a type 1 list creates one
    /// after them for each bit set, at the frequency the region's numbering gives that bit. A
    /// frequency the radio cannot use, a number the numbering does not define, and a reserved
    /// list type are all ignored, as the section asks.
    ///
    /// In a fixed plan only a type 1 list applies, and its groups set which of the numbered
    /// channels are enabled, a cleared bit disabling its channel (RP002-1.0.5 sections 3.5.4,
    /// 3.8.4 and 3.9.4). Bits past the plan's channels are reserved and read as nothing.
    pub(crate) fn apply_cflist(
        &mut self,
        plan: &ChannelPlan,
        list: CfList,
        usable: impl Fn(u32) -> bool,
    ) {
        if plan.kind == PlanKind::Fixed {
            if let Some(groups) = list.channel_mask_groups() {
                for index in 0..MAX_CHANNELS {
                    let on = groups[index / 16] & (1 << (index % 16)) != 0;
                    self.set_bit(index, on && self.get(index).is_some());
                }
            }
            return;
        }

        for index in self.defaults..MAX_CHANNELS {
            self.slots[index] = None;
            self.set_bit(index, false);
        }

        let (low, high) = CREATED_DATA_RATES;
        match list.kind() {
            CfListKind::Frequencies => {
                let Some(frequencies) = list.frequencies_hz() else {
                    return;
                };
                for (slot, hz) in frequencies.into_iter().enumerate() {
                    let index = self.defaults + slot;
                    if hz != 0 && index < MAX_CHANNELS && usable(hz) {
                        self.slots[index] = Some(Channel::new(hz, low, high));
                        self.set_bit(index, true);
                    }
                }
            }
            CfListKind::ChannelMasks => {
                let PlanKind::Dynamic {
                    channel_list: Some(numbering),
                } = plan.kind
                else {
                    return;
                };
                let mut index = self.defaults;
                for number in list.enabled_channels() {
                    if index >= MAX_CHANNELS {
                        break;
                    }
                    if let Some(hz) = numbering.frequency_hz(number).filter(|hz| usable(*hz)) {
                        self.slots[index] = Some(Channel::new(hz, low, high));
                        self.set_bit(index, true);
                        index += 1;
                    }
                }
            }
            CfListKind::Reserved(_) => {}
        }
    }

    /// Works out the mask a contiguous block of `LinkADRReq` channel controls leaves.
    ///
    /// Each control means what the plan's table says, RP002-1.0.5 table 14 for the dynamic
    /// plans, tables 23 and 43 for the 900 MHz plans, and tables 52 and 53 for CN470-510.
    /// TS001-1.0.4 section 5.3 applies the block in order as one command.
    ///
    /// # Returns
    ///
    /// The mask, or `None` when the block must be refused: a reserved control, a bit that
    /// enables a channel the device has not defined, or every channel left disabled.
    pub(crate) fn mask_after(
        &self,
        block: &[(u16, u8)],
        controls: &[MaskControl; 8],
    ) -> Option<[u16; MASK_GROUPS]> {
        let mut mask = self.enabled;
        for &(bits, control) in block {
            match controls[usize::from(control & 0x07)] {
                MaskControl::Group(group) => self.set_group(&mut mask, group, bits)?,
                MaskControl::Banks => {
                    for bank in 0..10 {
                        self.switch_bank(&mut mask, bank, bits & (1 << bank) != 0);
                    }
                }
                MaskControl::PairedBanks => {
                    for bank in 0..8 {
                        let on = bits & (1 << bank) != 0;
                        self.switch_bank(&mut mask, bank, on);
                        self.switch(&mut mask, 64 + bank, on);
                    }
                    self.switch_bank(&mut mask, 9, bits & (1 << 9) != 0);
                }
                MaskControl::All { on, then_group } => {
                    for index in 0..MAX_CHANNELS {
                        self.switch(&mut mask, index, on);
                    }
                    if let Some(group) = then_group {
                        self.set_group(&mut mask, group, bits)?;
                    }
                }
                MaskControl::Reserved => return None,
            }
        }
        if mask.iter().all(|group| *group == 0) {
            return None;
        }
        Some(mask)
    }

    /// Sets one group of sixteen from a mask, refusing a bit for an undefined channel.
    fn set_group(&self, mask: &mut [u16; MASK_GROUPS], group: u8, bits: u16) -> Option<()> {
        let group = usize::from(group);
        if group >= MASK_GROUPS {
            return None;
        }
        for bit in 0..16 {
            if bits & (1 << bit) != 0 && self.get(group * 16 + bit).is_none() {
                return None;
            }
        }
        mask[group] = bits;
        Some(())
    }

    /// Switches the eight channels of a bank.
    fn switch_bank(&self, mask: &mut [u16; MASK_GROUPS], bank: usize, on: bool) {
        for index in bank * 8..bank * 8 + 8 {
            self.switch(mask, index, on);
        }
    }

    /// Switches one channel in a mask, leaving an undefined channel off.
    fn switch(&self, mask: &mut [u16; MASK_GROUPS], index: usize, on: bool) {
        if index >= MAX_CHANNELS {
            return;
        }
        let bit = 1 << (index % 16);
        if on && self.get(index).is_some() {
            mask[index / 16] |= bit;
        } else {
            mask[index / 16] &= !bit;
        }
    }

    pub(crate) fn set_mask(&mut self, mask: [u16; MASK_GROUPS]) {
        self.enabled = mask;
    }

    /// Creates or replaces a channel, enabled.
    pub(crate) fn create(&mut self, index: usize, channel: Channel) {
        if index < MAX_CHANNELS {
            self.slots[index] = Some(channel);
            self.set_bit(index, true);
        }
    }

    /// Removes a channel.
    pub(crate) fn remove(&mut self, index: usize) {
        if index < MAX_CHANNELS {
            self.slots[index] = None;
            self.set_bit(index, false);
        }
    }

    /// Moves a channel's downlink frequency.
    pub(crate) fn set_downlink(&mut self, index: usize, hz: u32) {
        if let Some(Some(channel)) = self.slots.get_mut(index) {
            channel.downlink_hz = hz;
        }
    }

    fn set_bit(&mut self, index: usize, on: bool) {
        let bit = 1 << (index % 16);
        if on {
            self.enabled[index / 16] |= bit;
        } else {
            self.enabled[index / 16] &= !bit;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pamoja_lora::region::{Cn470Plan, Region};

    fn european() -> Channels {
        Channels::defaults(Region::Eu868.plan())
    }

    fn american() -> Channels {
        Channels::defaults(Region::Us915.plan())
    }

    fn enabled_numbers(channels: &Channels) -> Vec<usize> {
        channels.enabled().map(|(index, _)| index).collect()
    }

    fn numbers(mask: [u16; MASK_GROUPS]) -> Vec<usize> {
        (0..MAX_CHANNELS)
            .filter(|index| mask[index / 16] & (1 << (index % 16)) != 0)
            .collect()
    }

    #[test]
    fn a_european_device_starts_on_its_three_default_channels() {
        let channels = european();
        let enabled: Vec<u32> = channels.enabled().map(|(_, c)| c.uplink_hz).collect();
        assert_eq!(enabled, [868_100_000, 868_300_000, 868_500_000]);
        assert_eq!(channels.default_count(), 3);
        assert_eq!(
            channels.get(0).map(|c| (c.min_data_rate, c.max_data_rate)),
            Some((0, 5))
        );
        assert_eq!(channels.get(2).map(|c| c.downlink_hz), Some(868_500_000));
    }

    #[test]
    fn an_american_device_starts_on_all_seventy_two_answered_on_eight() {
        // RP002-1.0.5 sections 3.5.2 and 3.5.7.
        let channels = american();
        assert_eq!(channels.enabled().count(), 72);
        assert_eq!(channels.default_count(), 72);
        let channel = |index| channels.get(index).expect("defined");
        assert_eq!(
            (channel(0).uplink_hz, channel(0).downlink_hz),
            (902_300_000, 923_300_000)
        );
        assert_eq!(
            (channel(63).uplink_hz, channel(63).downlink_hz),
            (914_900_000, 927_500_000)
        );
        assert_eq!(
            (channel(64).uplink_hz, channel(64).downlink_hz),
            (903_000_000, 923_300_000)
        );
        assert_eq!(
            (channel(64).min_data_rate, channel(64).max_data_rate),
            (4, 4)
        );
        assert_eq!(channels.get(72), None);
    }

    #[test]
    fn the_ninety_six_channel_plan_fills_the_table() {
        let channels = Channels::defaults(Cn470Plan::Channels96.plan());
        assert_eq!(channels.enabled().count(), 96);
        assert_eq!(
            channels.get(49).map(|c| c.downlink_hz),
            Some(500_500_000),
            "channel 49 is answered on downlink channel 1"
        );
    }

    #[test]
    fn a_frequency_list_defines_the_five_channels_after_the_defaults() {
        let mut channels = european();
        let list = CfList::frequencies([867_100_000, 867_300_000, 0, 867_700_000, 867_900_000])
            .expect("valid");
        channels.apply_cflist(Region::Eu868.plan(), list, |_| true);

        let enabled: Vec<(usize, u32)> =
            channels.enabled().map(|(i, c)| (i, c.uplink_hz)).collect();
        assert_eq!(
            enabled,
            [
                (0, 868_100_000),
                (1, 868_300_000),
                (2, 868_500_000),
                (3, 867_100_000),
                (4, 867_300_000),
                (6, 867_700_000),
                (7, 867_900_000),
            ],
            "an unused slot leaves its channel undefined"
        );
        assert_eq!(
            channels.get(3).map(|c| (c.min_data_rate, c.max_data_rate)),
            Some((0, 5))
        );
    }

    #[test]
    fn a_mask_list_creates_channels_in_order_of_their_numbers() {
        // RP002-1.0.5 section 3.3.1.1's example: an EU868 device creates ChIndex 3 at 863.1,
        // 4 at 867.3, 5 at 867.7 and 6 at 868.7 MHz.
        let mut channels = european();
        let list = CfList::channel_masks([0x0001, 0x10A0, 0, 0, 0, 0]);
        channels.apply_cflist(Region::Eu868.plan(), list, |_| true);

        let created: Vec<(usize, u32)> = channels
            .enabled()
            .skip(3)
            .map(|(i, c)| (i, c.uplink_hz))
            .collect();
        assert_eq!(
            created,
            [
                (3, 863_100_000),
                (4, 867_300_000),
                (5, 867_700_000),
                (6, 868_700_000)
            ]
        );
    }

    #[test]
    fn a_fixed_plan_takes_a_mask_list_as_which_channels_are_on() {
        // RP002-1.0.5 section 3.5.4: channels 8 to 15 and the second 500 kHz channel, and a
        // bit past channel 71, which is reserved.
        let mut channels = american();
        let list = CfList::channel_masks([0xFF00, 0, 0, 0, 0x0102, 0]);
        channels.apply_cflist(Region::Us915.plan(), list, |_| true);
        assert_eq!(
            enabled_numbers(&channels),
            [8, 9, 10, 11, 12, 13, 14, 15, 65]
        );

        let mut channels = american();
        let frequencies = CfList::frequencies([903_100_000, 0, 0, 0, 0]).expect("valid");
        channels.apply_cflist(Region::Us915.plan(), frequencies, |_| true);
        assert_eq!(
            channels.enabled().count(),
            72,
            "a fixed plan ignores a list of frequencies"
        );
    }

    #[test]
    fn a_region_with_no_numbering_ignores_a_mask_list() {
        let plan = Region::Eu433.plan();
        let mut channels = Channels::defaults(plan);
        channels.apply_cflist(plan, CfList::channel_masks([0xFFFF, 0, 0, 0, 0, 0]), |_| {
            true
        });
        assert_eq!(channels.enabled().count(), 3);
    }

    #[test]
    fn a_frequency_the_radio_cannot_use_is_left_out() {
        let mut channels = european();
        let list = CfList::frequencies([867_100_000, 902_300_000, 0, 0, 0]).expect("valid");
        channels.apply_cflist(Region::Eu868.plan(), list, |hz| hz < 870_000_000);
        assert!(channels.get(3).is_some());
        assert!(channels.get(4).is_none());
    }

    #[test]
    fn channel_controls_set_groups_banks_and_everything() {
        let mut channels = european();
        channels.apply_cflist(
            Region::Eu868.plan(),
            CfList::frequencies([
                867_100_000,
                867_300_000,
                867_500_000,
                867_700_000,
                867_900_000,
            ])
            .expect("valid"),
            |_| true,
        );
        let controls = &MaskControl::DYNAMIC;

        assert_eq!(
            channels.mask_after(&[(0b0000_0101, 0)], controls),
            Some([0b0101, 0, 0, 0, 0, 0])
        );
        assert_eq!(
            channels.mask_after(&[(0, 5)], controls),
            None,
            "switching every bank off leaves nothing enabled"
        );
        assert_eq!(
            channels.mask_after(&[(1, 5)], controls),
            Some([0xFF, 0, 0, 0, 0, 0])
        );
        assert_eq!(
            channels.mask_after(&[(0b0001, 0), (0xFFFF, 6)], controls),
            Some([0xFF, 0, 0, 0, 0, 0]),
            "6 enables every defined channel, whatever came before it in the block"
        );
        assert_eq!(
            channels.mask_after(&[(0xFFFF, 7)], controls),
            None,
            "7 is reserved"
        );
        assert_eq!(
            channels.mask_after(&[(1 << 9, 0)], controls),
            None,
            "a bit for an undefined channel refuses the block"
        );
        assert_eq!(
            channels.mask_after(&[(0, 0), (0b10, 0)], controls),
            Some([0b10, 0, 0, 0, 0, 0]),
            "the block is taken as a whole, so an empty group partway is fine"
        );
    }

    #[test]
    fn the_american_controls_narrow_a_device_to_one_sub_band() {
        // RP002-1.0.5 section 3.5.5's note: from 64-channel operation to the first eight,
        // either as 7 then 0, or as 5 alone, which also keeps the paired 500 kHz channel.
        let channels = american();
        let controls = &Region::Us915.plan().mask_controls;

        let two = channels
            .mask_after(&[(0x0000, 7), (0x00FF, 0)], controls)
            .expect("accepted");
        assert_eq!(numbers(two), [0, 1, 2, 3, 4, 5, 6, 7]);

        let one = channels
            .mask_after(&[(0x0001, 5)], controls)
            .expect("accepted");
        assert_eq!(numbers(one), [0, 1, 2, 3, 4, 5, 6, 7, 64]);

        let second = channels
            .mask_after(&[(0x0002, 5)], controls)
            .expect("accepted");
        assert_eq!(
            numbers(second),
            [8, 9, 10, 11, 12, 13, 14, 15, 65],
            "the second sub-band that many networks run"
        );

        let wide_only = channels
            .mask_after(&[(0x00F0, 7)], controls)
            .expect("accepted");
        assert_eq!(numbers(wide_only), [68, 69, 70, 71]);

        let everything = channels
            .mask_after(&[(0x0000, 7), (0x00FF, 6)], controls)
            .expect("accepted");
        assert_eq!(numbers(everything).len(), 72);

        assert_eq!(
            channels.mask_after(&[(0x0100, 6)], controls),
            None,
            "channel 72 is not defined"
        );
        assert_eq!(
            channels.mask_after(&[(0x0000, 7)], controls),
            None,
            "7 with an empty mask leaves nothing on"
        );
    }

    #[test]
    fn the_chinese_antenna_plans_switch_everything_and_reserve_the_rest() {
        // RP002-1.0.5 tables 52 and 53.
        let twenty = Cn470Plan::Antenna20MhzA.plan();
        let channels = Channels::defaults(twenty);
        let controls = &twenty.mask_controls;
        assert_eq!(
            channels.mask_after(&[(0, 7), (0x000F, 3)], controls),
            Some([0, 0, 0, 0x000F, 0, 0])
        );
        assert_eq!(channels.mask_after(&[(0xFFFF, 4)], controls), None);
        assert_eq!(channels.mask_after(&[(0, 7)], controls), None);

        let twenty_six = Cn470Plan::Antenna26MhzB.plan();
        let channels = Channels::defaults(twenty_six);
        let controls = &twenty_six.mask_controls;
        assert_eq!(
            channels.mask_after(&[(0, 4), (0x8000, 2)], controls),
            Some([0, 0, 0x8000, 0, 0, 0])
        );
        assert_eq!(
            channels.mask_after(&[(0x0001, 3)], controls),
            Some([0xFFFF, 0xFFFF, 0xFFFF, 0, 0, 0])
        );
        assert_eq!(
            channels.mask_after(&[(0x0001, 3), (0x0001, 3)], controls),
            Some([0xFFFF, 0xFFFF, 0xFFFF, 0, 0, 0])
        );
        assert_eq!(channels.mask_after(&[(0x0001, 5)], controls), None);
        assert_eq!(
            channels.mask_after(&[(0x0001, 2), (0x0001, 3)], controls),
            Some([0xFFFF, 0xFFFF, 0xFFFF, 0, 0, 0])
        );
    }

    #[test]
    fn the_ninety_six_channel_plan_reaches_its_sixth_group() {
        let plan = Cn470Plan::Channels96.plan();
        let channels = Channels::defaults(plan);
        assert_eq!(
            channels.mask_after(
                &[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0x8000, 5)],
                &plan.mask_controls
            ),
            Some([0, 0, 0, 0, 0, 0x8000])
        );
    }

    #[test]
    fn restoring_defaults_leaves_created_channels_as_they_were() {
        let mut channels = european();
        channels.create(3, Channel::new(867_100_000, 0, 5));
        channels.set_mask([0b1000, 0, 0, 0, 0, 0]);
        channels.enable_defaults();
        assert_eq!(channels.mask(), [0b1111, 0, 0, 0, 0, 0]);

        let mut channels = american();
        channels.set_mask([0, 0x0100, 0, 0, 0, 0]);
        channels.enable_defaults();
        assert_eq!(
            channels.enabled().count(),
            72,
            "a fixed plan turns every channel back on"
        );
    }

    #[test]
    fn a_channel_carries_the_rates_in_its_range() {
        let channel = Channel::new(868_100_000, 0, 5);
        assert!(channel.carries(0) && channel.carries(5));
        assert!(!channel.carries(6));
        let channels = european();
        assert!(channels.carries(&channels.mask(), 5));
        assert!(!channels.carries(&channels.mask(), 6));
    }
}
