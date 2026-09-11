//! The SX126x interrupt bits.
//!
//! The chip logs each interrupt source in a 16-bit IRQ register, laid out in Table 13-29
//! of the SX1261/2 datasheet. The same layout masks which interrupts are enabled and
//! routes each one to DIO1, DIO2, or DIO3 through SetDioIrqParams, and clears them through
//! ClearIrqStatus.

use core::ops::{BitAnd, BitOr, BitOrAssign};

/// A set of SX126x interrupts, in the layout of the IRQ register.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx126x::irq::Irq;
///
/// // GetIrqStatus answered 0x0202: a timeout and a received packet.
/// let pending = Irq::from_bytes([0x02, 0x02]);
/// assert!(pending.contains(Irq::RX_DONE | Irq::TIMEOUT));
/// assert!(!pending.contains(Irq::CRC_ERROR));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Irq(u16);

impl Irq {
    /// No interrupt.
    pub const NONE: Irq = Irq(0);
    /// Bit 0: the packet transmission completed.
    pub const TX_DONE: Irq = Irq(1 << 0);
    /// Bit 1: a packet was received.
    pub const RX_DONE: Irq = Irq(1 << 1);
    /// Bit 2: a preamble was detected.
    pub const PREAMBLE_DETECTED: Irq = Irq(1 << 2);
    /// Bit 3: a valid sync word was detected, in FSK.
    pub const SYNC_WORD_VALID: Irq = Irq(1 << 3);
    /// Bit 4: a valid LoRa header was received.
    pub const HEADER_VALID: Irq = Irq(1 << 4);
    /// Bit 5: a LoRa header failed its CRC.
    pub const HEADER_ERROR: Irq = Irq(1 << 5);
    /// Bit 6: a packet failed its CRC.
    pub const CRC_ERROR: Irq = Irq(1 << 6);
    /// Bit 7: channel activity detection finished, in LoRa.
    pub const CAD_DONE: Irq = Irq(1 << 7);
    /// Bit 8: channel activity was detected, in LoRa.
    pub const CAD_DETECTED: Irq = Irq(1 << 8);
    /// Bit 9: a receive or transmit timeout.
    pub const TIMEOUT: Irq = Irq(1 << 9);
    /// Bit 14: a hop in LR-FHSS, after the power amplifier ramped up again.
    pub const LR_FHSS_HOP: Irq = Irq(1 << 14);
    /// Every interrupt the datasheet defines: bits 0 to 9 and bit 14.
    pub const ALL: Irq = Irq(0x43FF);

    /// Creates a set from the register's bits.
    ///
    /// # Arguments
    ///
    /// * `bits` - the 16-bit register value.
    ///
    /// # Returns
    ///
    /// The set, keeping reserved bits as they came.
    pub const fn from_bits(bits: u16) -> Irq {
        Irq(bits)
    }

    /// Creates a set from the two bytes the chip answers with, most significant first.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the IrqStatus bytes of a GetIrqStatus answer.
    ///
    /// # Returns
    ///
    /// The set.
    pub const fn from_bytes(bytes: [u8; 2]) -> Irq {
        Irq(u16::from_be_bytes(bytes))
    }

    /// Returns the register's bits.
    ///
    /// # Returns
    ///
    /// The 16-bit register value.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Returns the two bytes a command carries for this set, most significant first.
    ///
    /// # Returns
    ///
    /// The mask as it goes on the wire.
    pub const fn to_bytes(self) -> [u8; 2] {
        self.0.to_be_bytes()
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
    pub const fn contains(self, other: Irq) -> bool {
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
    pub const fn intersects(self, other: Irq) -> bool {
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

impl BitOr for Irq {
    type Output = Irq;

    fn bitor(self, rhs: Irq) -> Irq {
        Irq(self.0 | rhs.0)
    }
}

impl BitOrAssign for Irq {
    fn bitor_assign(&mut self, rhs: Irq) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Irq {
    type Output = Irq;

    fn bitand(self, rhs: Irq) -> Irq {
        Irq(self.0 & rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bits_follow_table_13_29() {
        let table = [
            (Irq::TX_DONE, 0),
            (Irq::RX_DONE, 1),
            (Irq::PREAMBLE_DETECTED, 2),
            (Irq::SYNC_WORD_VALID, 3),
            (Irq::HEADER_VALID, 4),
            (Irq::HEADER_ERROR, 5),
            (Irq::CRC_ERROR, 6),
            (Irq::CAD_DONE, 7),
            (Irq::CAD_DETECTED, 8),
            (Irq::TIMEOUT, 9),
            (Irq::LR_FHSS_HOP, 14),
        ];
        let mut all = Irq::NONE;
        for (irq, bit) in table {
            assert_eq!(irq.bits(), 1 << bit);
            all |= irq;
        }
        assert_eq!(all, Irq::ALL);
    }

    #[test]
    fn the_register_travels_most_significant_byte_first() {
        let irq = Irq::TIMEOUT | Irq::TX_DONE;
        assert_eq!(irq.to_bytes(), [0x02, 0x01]);
        assert_eq!(Irq::from_bytes([0x02, 0x01]), irq);
    }

    #[test]
    fn a_set_answers_for_its_members() {
        let pending = Irq::RX_DONE | Irq::CRC_ERROR;
        assert!(pending.contains(Irq::RX_DONE));
        assert!(!pending.contains(Irq::RX_DONE | Irq::TIMEOUT));
        assert!(pending.intersects(Irq::CRC_ERROR | Irq::HEADER_ERROR));
        assert_eq!(pending & Irq::CRC_ERROR, Irq::CRC_ERROR);
        assert!(Irq::NONE.is_empty());
    }
}
