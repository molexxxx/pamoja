//! The channel list a join accept can carry.
//!
//! A network that wants a device on more channels than its region's defaults says so in the
//! join accept, in sixteen optional bytes at the end called the CFList. RP002-1.0.5 section
//! 3.3.1 defines two forms, told apart by the last byte.
//!
//! - **Type 0** lists up to five frequencies, three bytes each, least significant first, in
//!   units of 100 Hz. A zero is an unused slot. The dynamic channel plans (EU868, EU433,
//!   AS923, KR920, IN865, RU864) use it.
//! - **Type 1** is six sixteen-bit channel masks, least significant byte first, where bit
//!   *n* of group *g* is channel `g * 16 + n`. The fixed plans (US915, AU915, CN470) read
//!   each group as the mask it names. A dynamic plan may take it too, reading a channel
//!   number against one of the two published lists in [`FixedChannelList`].
//!
//! A [`CfList`] keeps the sixteen bytes as they arrived and reads either form out of them, so
//! nothing a network sent is lost to a type this crate does not recognize.

use crate::error::LorawanError;

/// The number of bytes a channel list occupies.
pub const CFLIST_LEN: usize = 16;

/// How many frequencies a type 0 list carries.
pub const CFLIST_FREQUENCIES: usize = 5;

/// How many sixteen-bit masks a type 1 list carries.
pub const CFLIST_MASK_GROUPS: usize = 6;

/// Which form a channel list takes, from its last byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CfListKind {
    /// Type 0: a list of frequencies.
    Frequencies,
    /// Type 1: groups of channel mask bits.
    ChannelMasks,
    /// A type the regional parameters reserve, which a device ignores.
    Reserved(u8),
}

impl CfListKind {
    /// Returns the byte a channel list of this kind ends with.
    ///
    /// # Returns
    ///
    /// `0` for frequencies, `1` for channel masks, or the reserved byte itself.
    pub const fn to_byte(self) -> u8 {
        match self {
            CfListKind::Frequencies => 0,
            CfListKind::ChannelMasks => 1,
            CfListKind::Reserved(byte) => byte,
        }
    }

    /// Reads the kind a channel list's last byte names.
    ///
    /// # Arguments
    ///
    /// * `byte` - the CFListType byte.
    ///
    /// # Returns
    ///
    /// The kind.
    pub const fn from_byte(byte: u8) -> CfListKind {
        match byte {
            0 => CfListKind::Frequencies,
            1 => CfListKind::ChannelMasks,
            other => CfListKind::Reserved(other),
        }
    }
}

/// The optional channel list at the end of a join accept.
///
/// # Examples
///
/// A European network adding the five channels most EU868 networks run between 867.1 and
/// 867.9 MHz:
///
/// ```
/// use pamoja_lorawan::{CfList, CfListKind};
///
/// let list = CfList::frequencies([867_100_000, 867_300_000, 867_500_000, 867_700_000, 867_900_000])?;
/// assert_eq!(list.kind(), CfListKind::Frequencies);
/// assert_eq!(&list.to_bytes()[..3], &[0x18, 0x4F, 0x84], "8 671 000 hundreds of hertz, low byte first");
///
/// // A device reads the same frequencies back out.
/// let heard = CfList::from_bytes(list.to_bytes());
/// assert_eq!(heard.frequencies_hz(), Some([867_100_000, 867_300_000, 867_500_000, 867_700_000, 867_900_000]));
/// # Ok::<(), pamoja_lorawan::LorawanError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CfList {
    bytes: [u8; CFLIST_LEN],
}

impl CfList {
    /// Keeps a channel list exactly as it arrived.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the sixteen CFList bytes.
    ///
    /// # Returns
    ///
    /// The channel list, whatever its type byte says.
    pub const fn from_bytes(bytes: [u8; CFLIST_LEN]) -> CfList {
        CfList { bytes }
    }

    /// Builds a type 0 list from frequencies.
    ///
    /// # Arguments
    ///
    /// * `frequencies_hz` - five frequencies in hertz, with `0` for a slot left unused.
    ///
    /// # Returns
    ///
    /// The channel list.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::MalformedFrame`] if a frequency is not a whole number of
    /// hundreds of hertz, or does not fit the three bytes it is carried in, which end just
    /// under 1.678 GHz. Between those, RP002-1.0.5 reserves anything below 100 MHz, and that
    /// is refused too.
    pub const fn frequencies(
        frequencies_hz: [u32; CFLIST_FREQUENCIES],
    ) -> Result<CfList, LorawanError> {
        let mut bytes = [0u8; CFLIST_LEN];
        let mut slot = 0;
        while slot < CFLIST_FREQUENCIES {
            let hz = frequencies_hz[slot];
            if hz != 0 {
                if !hz.is_multiple_of(100) || hz < 100_000_000 {
                    return Err(LorawanError::MalformedFrame);
                }
                let hundreds = hz / 100;
                if hundreds > 0x00FF_FFFF {
                    return Err(LorawanError::MalformedFrame);
                }
                let at = slot * 3;
                bytes[at] = hundreds as u8;
                bytes[at + 1] = (hundreds >> 8) as u8;
                bytes[at + 2] = (hundreds >> 16) as u8;
            }
            slot += 1;
        }
        bytes[CFLIST_LEN - 1] = CfListKind::Frequencies.to_byte();
        Ok(CfList { bytes })
    }

    /// Builds a type 1 list from channel mask groups.
    ///
    /// # Arguments
    ///
    /// * `masks` - six groups, where bit *n* of group *g* enables channel `g * 16 + n`.
    ///
    /// # Returns
    ///
    /// The channel list, with its three reserved bytes zero.
    pub const fn channel_masks(masks: [u16; CFLIST_MASK_GROUPS]) -> CfList {
        let mut bytes = [0u8; CFLIST_LEN];
        let mut group = 0;
        while group < CFLIST_MASK_GROUPS {
            bytes[group * 2] = masks[group] as u8;
            bytes[group * 2 + 1] = (masks[group] >> 8) as u8;
            group += 1;
        }
        bytes[CFLIST_LEN - 1] = CfListKind::ChannelMasks.to_byte();
        CfList { bytes }
    }

    /// Returns the sixteen bytes, as a join accept carries them.
    ///
    /// # Returns
    ///
    /// The bytes.
    pub const fn to_bytes(&self) -> [u8; CFLIST_LEN] {
        self.bytes
    }

    /// Returns which form the list takes.
    ///
    /// # Returns
    ///
    /// The kind its last byte names.
    pub const fn kind(&self) -> CfListKind {
        CfListKind::from_byte(self.bytes[CFLIST_LEN - 1])
    }

    /// Reads the frequencies out of a type 0 list.
    ///
    /// # Returns
    ///
    /// Five frequencies in hertz, with `0` for an unused slot, or [`None`] for a list of any
    /// other type.
    pub const fn frequencies_hz(&self) -> Option<[u32; CFLIST_FREQUENCIES]> {
        if !matches!(self.kind(), CfListKind::Frequencies) {
            return None;
        }
        let mut out = [0u32; CFLIST_FREQUENCIES];
        let mut slot = 0;
        while slot < CFLIST_FREQUENCIES {
            let at = slot * 3;
            let hundreds = self.bytes[at] as u32
                | (self.bytes[at + 1] as u32) << 8
                | (self.bytes[at + 2] as u32) << 16;
            out[slot] = hundreds * 100;
            slot += 1;
        }
        Some(out)
    }

    /// Reads the mask groups out of a type 1 list.
    ///
    /// # Returns
    ///
    /// Six groups, or [`None`] for a list of any other type.
    pub const fn channel_mask_groups(&self) -> Option<[u16; CFLIST_MASK_GROUPS]> {
        if !matches!(self.kind(), CfListKind::ChannelMasks) {
            return None;
        }
        let mut out = [0u16; CFLIST_MASK_GROUPS];
        let mut group = 0;
        while group < CFLIST_MASK_GROUPS {
            out[group] = self.bytes[group * 2] as u16 | (self.bytes[group * 2 + 1] as u16) << 8;
            group += 1;
        }
        Some(out)
    }

    /// Reports whether a type 1 list enables a channel.
    ///
    /// # Arguments
    ///
    /// * `channel` - the channel number, `group * 16 + bit`.
    ///
    /// # Returns
    ///
    /// Whether its bit is set, or [`None`] for a list of any other type or a channel past
    /// the 96 the groups cover.
    pub const fn enables(&self, channel: u8) -> Option<bool> {
        let Some(groups) = self.channel_mask_groups() else {
            return None;
        };
        let group = channel as usize / 16;
        if group >= CFLIST_MASK_GROUPS {
            return None;
        }
        Some(groups[group] & (1 << (channel % 16)) != 0)
    }

    /// Returns the channels a type 1 list enables, lowest first.
    ///
    /// This is the order RP002-1.0.5 section 3.3.1.1 has a dynamic plan create them in, one
    /// after its last default channel for each bit set.
    ///
    /// # Returns
    ///
    /// The channel numbers, which is empty for a list of any other type.
    pub fn enabled_channels(&self) -> impl Iterator<Item = u8> + '_ {
        let groups = self.channel_mask_groups();
        (0..(CFLIST_MASK_GROUPS * 16) as u8).filter(move |channel| {
            groups.is_some_and(|groups| groups[*channel as usize / 16] & (1 << (channel % 16)) != 0)
        })
    }
}

impl From<CfList> for [u8; CFLIST_LEN] {
    fn from(list: CfList) -> [u8; CFLIST_LEN] {
        list.bytes
    }
}

/// The published channel lists a dynamic plan reads a type 1 list against.
///
/// RP002-1.0.5 section 3.3.1 fixes a frequency for every channel number, so a network can
/// enable channels with a bit each rather than spelling their frequencies out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FixedChannelList {
    /// Section 3.3.1.1: forty channels, from 863.1 MHz in 200 kHz steps to 869.9 MHz at
    /// channel 34, then five at 865.0625, 865.4025, 865.6025, 865.785 and 865.985 MHz. It
    /// covers the channels EU868, IN865 and RU864 networks run.
    Mhz800,
    /// Section 3.3.1.2: ninety-six channels, from 915.1 MHz in 100 kHz steps to 924.6 MHz.
    /// It covers the channels AS923 and KR920 networks run.
    Mhz900,
}

impl FixedChannelList {
    /// Returns the frequency a channel number stands for.
    ///
    /// # Arguments
    ///
    /// * `channel` - the channel number.
    ///
    /// # Returns
    ///
    /// The frequency in hertz, or [`None`] for a number the list does not define.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::FixedChannelList;
    ///
    /// assert_eq!(FixedChannelList::Mhz800.frequency_hz(0), Some(863_100_000));
    /// assert_eq!(FixedChannelList::Mhz800.frequency_hz(39), Some(865_985_000));
    /// assert_eq!(FixedChannelList::Mhz900.frequency_hz(95), Some(924_600_000));
    /// assert_eq!(FixedChannelList::Mhz900.frequency_hz(96), None);
    /// ```
    pub const fn frequency_hz(self, channel: u8) -> Option<u32> {
        match self {
            FixedChannelList::Mhz800 => match channel {
                0..=34 => Some(863_100_000 + channel as u32 * 200_000),
                35 => Some(865_062_500),
                36 => Some(865_402_500),
                37 => Some(865_602_500),
                38 => Some(865_785_000),
                39 => Some(865_985_000),
                _ => None,
            },
            FixedChannelList::Mhz900 => match channel {
                0..=95 => Some(915_100_000 + channel as u32 * 100_000),
                _ => None,
            },
        }
    }

    /// Returns how many channels the list defines.
    ///
    /// # Returns
    ///
    /// Forty for the 800 MHz list, ninety-six for the 900 MHz one.
    pub const fn len(self) -> u8 {
        match self {
            FixedChannelList::Mhz800 => 40,
            FixedChannelList::Mhz900 => 96,
        }
    }

    /// Reports whether the list defines no channels, which neither published list does.
    ///
    /// # Returns
    ///
    /// `false`.
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> [u8; CFLIST_LEN] {
        let bytes: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("hex"))
            .collect();
        bytes.try_into().expect("sixteen bytes")
    }

    #[test]
    fn a_captured_european_list_reads_as_the_five_usual_channels() {
        // The CFList of the EU868 join accept published at
        // https://github.com/anthonykirby/lora-packet/issues/10, which this crate's network
        // tests also rebuild byte for byte.
        let list = CfList::from_bytes(hex("184f84e85684b85e84886684586e8400"));
        assert_eq!(list.kind(), CfListKind::Frequencies);
        assert_eq!(
            list.frequencies_hz(),
            Some([
                867_100_000,
                867_300_000,
                867_500_000,
                867_700_000,
                867_900_000
            ])
        );
        assert_eq!(list.channel_mask_groups(), None);
        assert_eq!(
            CfList::frequencies(list.frequencies_hz().expect("type 0")),
            Ok(list),
            "and building those frequencies gives the same bytes"
        );
    }

    #[test]
    fn the_regional_parameters_example_enables_four_channels() {
        // RP002-1.0.5 section 3.3.1.1: CFList type 1 received as
        // 0x0100_A010_0000_0000_0000_0000 creates channels at 863.1, 867.3, 867.7 and
        // 868.7 MHz on an EU868 device.
        let list = CfList::from_bytes(hex("0100a010000000000000000000000001"));
        assert_eq!(list.kind(), CfListKind::ChannelMasks);
        assert_eq!(
            list.channel_mask_groups(),
            Some([0x0001, 0x10A0, 0, 0, 0, 0])
        );

        let enabled: Vec<u8> = list.enabled_channels().collect();
        assert_eq!(enabled, [0, 21, 23, 28]);
        let created: Vec<u32> = enabled
            .iter()
            .map(|channel| {
                FixedChannelList::Mhz800
                    .frequency_hz(*channel)
                    .expect("defined")
            })
            .collect();
        assert_eq!(
            created,
            [863_100_000, 867_300_000, 867_700_000, 868_700_000]
        );

        assert_eq!(list.enables(21), Some(true));
        assert_eq!(list.enables(22), Some(false));
        assert_eq!(list.enables(96), None);
        assert_eq!(list.frequencies_hz(), None);
        assert_eq!(
            CfList::channel_masks([0x0001, 0x10A0, 0, 0, 0, 0]),
            list,
            "building the same masks gives the same bytes"
        );
    }

    fn created(list: CfList) -> Vec<u32> {
        list.enabled_channels()
            .map(|channel| {
                FixedChannelList::Mhz800
                    .frequency_hz(channel)
                    .expect("defined")
            })
            .collect()
    }

    #[test]
    fn the_notes_under_the_800_mhz_list_name_the_channels_they_say() {
        // RP002-1.0.5 section 3.3.1.1 writes each group in the order its bytes travel, as its
        // example above does. The three EU868 default channels are ChMaskGrp1 = 0x000E,
        // which is bits 9 to 11 of the group, channels 25 to 27.
        let defaults = CfList::from_bytes(hex("0000000e000000000000000000000001"));
        assert_eq!(created(defaults), [868_100_000, 868_300_000, 868_500_000]);

        // The four RFID channels of ETSI EN 302 208 are ChMaskGrp0 = 0x0020 and
        // ChMaskGrp1 = 0x4900: 865.7, 866.3, 866.9 and 867.5 MHz.
        let rfid = CfList::from_bytes(hex("00204900000000000000000000000001"));
        assert_eq!(
            created(rfid),
            [865_700_000, 866_300_000, 866_900_000, 867_500_000]
        );
    }

    #[test]
    fn the_published_channel_lists_end_where_the_document_says() {
        assert_eq!(FixedChannelList::Mhz800.frequency_hz(34), Some(869_900_000));
        assert_eq!(FixedChannelList::Mhz800.frequency_hz(35), Some(865_062_500));
        assert_eq!(FixedChannelList::Mhz800.frequency_hz(36), Some(865_402_500));
        assert_eq!(FixedChannelList::Mhz800.frequency_hz(37), Some(865_602_500));
        assert_eq!(FixedChannelList::Mhz800.frequency_hz(38), Some(865_785_000));
        assert_eq!(FixedChannelList::Mhz800.frequency_hz(40), None);
        assert_eq!(FixedChannelList::Mhz800.len(), 40);
        assert_eq!(FixedChannelList::Mhz900.frequency_hz(0), Some(915_100_000));
        assert_eq!(FixedChannelList::Mhz900.len(), 96);
    }

    #[test]
    fn a_frequency_the_bytes_cannot_carry_is_refused() {
        assert_eq!(
            CfList::frequencies([868_100_050, 0, 0, 0, 0]),
            Err(LorawanError::MalformedFrame),
            "not a whole number of hundreds of hertz"
        );
        assert_eq!(
            CfList::frequencies([1_677_721_600, 0, 0, 0, 0]),
            Err(LorawanError::MalformedFrame),
            "one step past the three bytes"
        );
        assert_eq!(
            CfList::frequencies([99_999_900, 0, 0, 0, 0]),
            Err(LorawanError::MalformedFrame),
            "below 100 MHz is reserved"
        );
        assert!(CfList::frequencies([1_677_721_500, 0, 0, 0, 0]).is_ok());
    }

    #[test]
    fn a_reserved_type_keeps_its_bytes_and_reads_as_neither_form() {
        let list = CfList::from_bytes(hex("0102030405060708090a0b0c0d0e0f07"));
        assert_eq!(list.kind(), CfListKind::Reserved(7));
        assert_eq!(list.frequencies_hz(), None);
        assert_eq!(list.channel_mask_groups(), None);
        assert_eq!(list.enabled_channels().count(), 0);
        assert_eq!(list.to_bytes(), hex("0102030405060708090a0b0c0d0e0f07"));
    }
}
