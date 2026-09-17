//! The channels a device may transmit on, and how a network changes them.
//!
//! In a dynamic channel plan a device starts with the region's default channels, and a
//! network adds more, moves their downlink frequency, and enables or disables them. RP002-1.0.5
//! gives these plans between 24 and 80 channels and `LinkADRReq` addresses up to 80, so that
//! is the capacity here.

use pamoja_lora::region::{ChannelPlan, PlanKind};

use crate::cflist::{CfList, CfListKind};

/// How many channels a device keeps.
pub const MAX_CHANNELS: usize = 80;

/// The mask groups `LinkADRReq` addresses, sixteen channels each.
const MASK_GROUPS: usize = MAX_CHANNELS / 16;

/// The data rates a channel a join accept creates carries: RP002-1.0.5 section 3.3.1 makes
/// them "usable for DR0 to DR5 125 kHz LoRa modulation".
const CREATED_DATA_RATES: (u8, u8) = (0, 5);

/// One channel a device can transmit on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Channel {
    /// Where uplinks go out, in hertz.
    pub uplink_hz: u32,
    /// Where the first receive window listens, in hertz. The uplink frequency unless a
    /// network moved it with `DlChannelReq`.
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
    /// The region's default channels, every one enabled.
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
                    channels.slots[index] =
                        Some(Channel::new(hz, block.min_data_rate, block.max_data_rate));
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
    /// This is what TS001-1.0.4 section 4.3.1.1 has a dynamic plan's device do at the end
    /// of its back-off: "enable the region's default channels and make no change to the
    /// configuration of the dynamically configured channels".
    pub(crate) fn enable_defaults(&mut self) {
        for index in 0..self.defaults {
            if self.get(index).is_some() {
                self.set_bit(index, true);
            }
        }
    }

    /// Takes the channel list a join accept carried.
    ///
    /// RP002-1.0.5 section 3.3.1: the list replaces every channel but the defaults; a type 0
    /// list defines the five after them, and a type 1 list creates one after them for each
    /// bit set, at the frequency the region's numbering gives that bit. A frequency the radio
    /// cannot use, a number the numbering does not define, and a reserved list type are all
    /// ignored, as the section asks.
    pub(crate) fn apply_cflist(
        &mut self,
        plan: &ChannelPlan,
        list: CfList,
        usable: impl Fn(u32) -> bool,
    ) {
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

    /// Works out the mask a contiguous block of `LinkADRReq` channel controls leaves, for a
    /// dynamic plan.
    ///
    /// RP002-1.0.5 gives every dynamic region the same `ChMaskCntl` table (EU868 table 14):
    /// 0 to 4 set the sixteen channels of that group, 5 enables or disables banks of eight
    /// with its ten low bits, 6 enables every defined channel whatever the mask says, and 7
    /// is reserved. TS001-1.0.4 section 5.2 applies the block in order as one command.
    ///
    /// # Returns
    ///
    /// The mask, or `None` when the block must be refused: a reserved control, a bit for a
    /// channel that is not defined, or every channel left disabled.
    pub(crate) fn mask_after(&self, block: &[(u16, u8)]) -> Option<[u16; MASK_GROUPS]> {
        let mut mask = self.enabled;
        for &(bits, control) in block {
            match control {
                0..=4 => {
                    let group = usize::from(control);
                    for bit in 0..16 {
                        if bits & (1 << bit) != 0 && self.get(group * 16 + bit).is_none() {
                            return None;
                        }
                    }
                    mask[group] = bits;
                }
                5 => {
                    for bank in 0..10 {
                        let on = bits & (1 << bank) != 0;
                        for index in bank * 8..bank * 8 + 8 {
                            let bit = 1 << (index % 16);
                            if on && self.get(index).is_some() {
                                mask[index / 16] |= bit;
                            } else if !on {
                                mask[index / 16] &= !bit;
                            }
                        }
                    }
                }
                6 => {
                    for index in 0..MAX_CHANNELS {
                        if self.get(index).is_some() {
                            mask[index / 16] |= 1 << (index % 16);
                        }
                    }
                }
                _ => return None,
            }
        }
        if mask.iter().all(|group| *group == 0) {
            return None;
        }
        Some(mask)
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
    use pamoja_lora::region::Region;

    fn european() -> Channels {
        Channels::defaults(Region::Eu868.plan())
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

        assert_eq!(
            channels.mask_after(&[(0b0000_0101, 0)]),
            Some([0b0101, 0, 0, 0, 0])
        );
        assert_eq!(
            channels.mask_after(&[(0, 5)]),
            None,
            "switching every bank off leaves nothing enabled"
        );
        assert_eq!(channels.mask_after(&[(1, 5)]), Some([0xFF, 0, 0, 0, 0]));
        assert_eq!(
            channels.mask_after(&[(0b0001, 0), (0xFFFF, 6)]),
            Some([0xFF, 0, 0, 0, 0]),
            "6 enables every defined channel, whatever came before it in the block"
        );
        assert_eq!(channels.mask_after(&[(0xFFFF, 7)]), None, "7 is reserved");
        assert_eq!(
            channels.mask_after(&[(1 << 9, 0)]),
            None,
            "a bit for an undefined channel refuses the block"
        );
        assert_eq!(
            channels.mask_after(&[(0, 0), (0b10, 0)]),
            Some([0b10, 0, 0, 0, 0]),
            "the block is taken as a whole, so an empty group partway is fine"
        );
    }

    #[test]
    fn restoring_defaults_leaves_created_channels_as_they_were() {
        let mut channels = european();
        channels.create(3, Channel::new(867_100_000, 0, 5));
        channels.set_mask([0b1000, 0, 0, 0, 0]);
        channels.enable_defaults();
        assert_eq!(channels.mask(), [0b1111, 0, 0, 0, 0]);
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
