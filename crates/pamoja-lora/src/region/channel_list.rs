//! The published channel numberings a channel mask can refer to.

/// A published list that gives every channel number a frequency.
///
/// RP002-1.0.5 section 3.3.1 fixes a frequency for each number, so a network can enable
/// channels on a device with a bit each rather than spelling their frequencies out. A join
/// accept's type 1 channel list is read this way in a dynamic channel plan: each bit set
/// creates the channel its number stands for.
///
/// # Examples
///
/// ```
/// use pamoja_lora::region::FixedChannelList;
///
/// assert_eq!(FixedChannelList::Mhz800.frequency_hz(0), Some(863_100_000));
/// assert_eq!(FixedChannelList::Mhz800.frequency_hz(39), Some(865_985_000));
/// assert_eq!(FixedChannelList::Mhz900.frequency_hz(95), Some(924_600_000));
/// assert_eq!(FixedChannelList::Mhz900.frequency_hz(96), None);
/// ```
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

    #[test]
    fn the_lists_end_where_the_document_says() {
        // RP002-1.0.5 sections 3.3.1.1 and 3.3.1.2.
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
    fn the_examples_under_the_800_mhz_list_land_on_their_frequencies() {
        // Section 3.3.1.1's worked example creates channels 0, 21, 23 and 28 at 863.1,
        // 867.3, 867.7 and 868.7 MHz.
        let example: [u32; 4] = [0, 21, 23, 28].map(|channel| {
            FixedChannelList::Mhz800
                .frequency_hz(channel)
                .expect("defined")
        });
        assert_eq!(
            example,
            [863_100_000, 867_300_000, 867_700_000, 868_700_000]
        );

        // Its notes put the EU868 default channels at 25 to 27, and the four RFID channels
        // of ETSI EN 302 208 at 13, 16, 19 and 22.
        let defaults: [u32; 3] = [25, 26, 27].map(|channel| {
            FixedChannelList::Mhz800
                .frequency_hz(channel)
                .expect("defined")
        });
        assert_eq!(defaults, [868_100_000, 868_300_000, 868_500_000]);
        let rfid: [u32; 4] = [13, 16, 19, 22].map(|channel| {
            FixedChannelList::Mhz800
                .frequency_hz(channel)
                .expect("defined")
        });
        assert_eq!(rfid, [865_700_000, 866_300_000, 866_900_000, 867_500_000]);
    }
}
