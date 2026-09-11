//! The SX127x LoRa interrupt flags.
//!
//! In LoRa mode the chip raises its interrupts in RegIrqFlags and masks them in
//! RegIrqFlagsMask, both laid out as the SX1276/77/78/79 datasheet (Rev 7) describes them.
//! A raised flag stays set until it is written back as a 1.

use core::ops::{BitAnd, BitOr, BitOrAssign};

/// A set of SX127x LoRa interrupts, in the layout of RegIrqFlags.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::irq::IrqFlags;
///
/// // RegIrqFlags read 0x70: a packet arrived with a valid header but a bad payload CRC.
/// let raised = IrqFlags::from_bits(0x70);
/// assert!(raised.contains(IrqFlags::RX_DONE | IrqFlags::PAYLOAD_CRC_ERROR));
/// assert!(!raised.contains(IrqFlags::TX_DONE));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct IrqFlags(u8);

impl IrqFlags {
    /// No interrupt.
    pub const NONE: IrqFlags = IrqFlags(0);
    /// Bit 0: channel activity detection heard a LoRa signal.
    pub const CAD_DETECTED: IrqFlags = IrqFlags(1 << 0);
    /// Bit 1: frequency hopping moved to the next channel.
    pub const FHSS_CHANGE_CHANNEL: IrqFlags = IrqFlags(1 << 1);
    /// Bit 2: channel activity detection finished.
    pub const CAD_DONE: IrqFlags = IrqFlags(1 << 2);
    /// Bit 3: the payload in the data buffer has been transmitted.
    pub const TX_DONE: IrqFlags = IrqFlags(1 << 3);
    /// Bit 4: a valid header was received.
    pub const VALID_HEADER: IrqFlags = IrqFlags(1 << 4);
    /// Bit 5: the payload failed its CRC.
    pub const PAYLOAD_CRC_ERROR: IrqFlags = IrqFlags(1 << 5);
    /// Bit 6: a packet has been received.
    pub const RX_DONE: IrqFlags = IrqFlags(1 << 6);
    /// Bit 7: a single reception timed out before a preamble arrived.
    pub const RX_TIMEOUT: IrqFlags = IrqFlags(1 << 7);
    /// Every interrupt, which is what writing 0xFF clears.
    pub const ALL: IrqFlags = IrqFlags(0xFF);

    /// Creates a set from the register's bits.
    ///
    /// # Arguments
    ///
    /// * `bits` - the RegIrqFlags value.
    ///
    /// # Returns
    ///
    /// The set.
    pub const fn from_bits(bits: u8) -> IrqFlags {
        IrqFlags(bits)
    }

    /// Returns the register's bits.
    ///
    /// # Returns
    ///
    /// The RegIrqFlags value, which written back clears exactly these interrupts.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Reports whether every interrupt of another set is in this one.
    ///
    /// # Arguments
    ///
    /// * `other` - the interrupts to look for.
    ///
    /// # Returns
    ///
    /// `true` when all of them are set.
    pub const fn contains(self, other: IrqFlags) -> bool {
        self.0 & other.0 == other.0
    }

    /// Reports whether any interrupt of another set is in this one.
    ///
    /// # Arguments
    ///
    /// * `other` - the interrupts to look for.
    ///
    /// # Returns
    ///
    /// `true` when at least one of them is set.
    pub const fn intersects(self, other: IrqFlags) -> bool {
        self.0 & other.0 != 0
    }

    /// Reports whether the set holds no interrupt.
    ///
    /// # Returns
    ///
    /// `true` when no bit is set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for IrqFlags {
    type Output = IrqFlags;

    fn bitor(self, rhs: IrqFlags) -> IrqFlags {
        IrqFlags(self.0 | rhs.0)
    }
}

impl BitOrAssign for IrqFlags {
    fn bitor_assign(&mut self, rhs: IrqFlags) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for IrqFlags {
    type Output = IrqFlags;

    fn bitand(self, rhs: IrqFlags) -> IrqFlags {
        IrqFlags(self.0 & rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_flag_sits_on_the_bit_the_register_description_gives() {
        assert_eq!(IrqFlags::RX_TIMEOUT.bits(), 0x80);
        assert_eq!(IrqFlags::RX_DONE.bits(), 0x40);
        assert_eq!(IrqFlags::PAYLOAD_CRC_ERROR.bits(), 0x20);
        assert_eq!(IrqFlags::VALID_HEADER.bits(), 0x10);
        assert_eq!(IrqFlags::TX_DONE.bits(), 0x08);
        assert_eq!(IrqFlags::CAD_DONE.bits(), 0x04);
        assert_eq!(IrqFlags::FHSS_CHANGE_CHANNEL.bits(), 0x02);
        assert_eq!(IrqFlags::CAD_DETECTED.bits(), 0x01);
    }

    #[test]
    fn the_flags_together_are_the_whole_register() {
        let every = IrqFlags::RX_TIMEOUT
            | IrqFlags::RX_DONE
            | IrqFlags::PAYLOAD_CRC_ERROR
            | IrqFlags::VALID_HEADER
            | IrqFlags::TX_DONE
            | IrqFlags::CAD_DONE
            | IrqFlags::FHSS_CHANGE_CHANNEL
            | IrqFlags::CAD_DETECTED;
        assert_eq!(every, IrqFlags::ALL);
    }

    #[test]
    fn intersects_finds_any_and_contains_needs_all() {
        let raised = IrqFlags::RX_DONE | IrqFlags::VALID_HEADER;
        assert!(raised.intersects(IrqFlags::RX_DONE | IrqFlags::RX_TIMEOUT));
        assert!(!raised.contains(IrqFlags::RX_DONE | IrqFlags::RX_TIMEOUT));
        assert!(IrqFlags::NONE.is_empty());
        assert_eq!(raised & IrqFlags::RX_DONE, IrqFlags::RX_DONE);
    }
}
