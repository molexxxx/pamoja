//! What an SX127x reports back in LoRa mode: the signal levels of a packet and the modem's
//! state.
//!
//! The levels follow the RSSI and SNR in LoRa Mode section of the SX1276/77/78/79
//! datasheet (Rev 7) and its descriptions of RegPktSnrValue, RegPktRssiValue, and
//! RegRssiValue. The chip reports power in whole decibels above an offset that depends on
//! which of its two RF ports is in use, and SNR in quarters of a decibel, so every level
//! comes out as a [`Decibels`] with nothing rounded away.

use pamoja_lora::budget::Decibels;

/// The frequency, in hertz, above which a radio uses its high frequency port. It is the
/// threshold Semtech's LoRaMac-node applies to the SX1276's RSSI offsets and errata.
pub const MID_BAND_HZ: u32 = 525_000_000;

/// The RF port a frequency is received on, which sets the RSSI offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Port {
    /// The high frequency port, RFI_HF, for bands above [`MID_BAND_HZ`] such as 868 and
    /// 915 MHz.
    High,
    /// The low frequency port, RFI_LF, for 433 and 169 MHz.
    Low,
}

impl Port {
    /// Returns the port that receives a frequency.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the carrier frequency in hertz.
    ///
    /// # Returns
    ///
    /// [`Port::High`] above [`MID_BAND_HZ`], else [`Port::Low`].
    pub const fn for_frequency(frequency_hz: u32) -> Port {
        if frequency_hz > MID_BAND_HZ {
            Port::High
        } else {
            Port::Low
        }
    }

    /// Returns the constant the RSSI registers are added to.
    ///
    /// # Returns
    ///
    /// -157 dBm on the high frequency port and -164 dBm on the low one.
    pub const fn rssi_offset_dbm(self) -> i32 {
        match self {
            Port::High => -157,
            Port::Low => -164,
        }
    }
}

/// Decodes RegRssiValue, the signal power the receiver hears right now.
///
/// # Arguments
///
/// * `byte` - the RegRssiValue byte.
/// * `port` - the port the radio listens on.
///
/// # Returns
///
/// The RSSI in dBm: the port's offset plus the register value.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::status::{rssi_dbm, Port};
///
/// assert_eq!(rssi_dbm(0x30, Port::High).to_string(), "-109.00");
/// assert_eq!(rssi_dbm(0x30, Port::Low).to_string(), "-116.00");
/// ```
pub const fn rssi_dbm(byte: u8, port: Port) -> Decibels {
    Decibels::from_hundredths((port.rssi_offset_dbm() + byte as i32) * 100)
}

/// The signal levels of the last LoRa packet received.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::status::{PacketStatus, Port};
///
/// // RegPktSnrValue 0xF6 and RegPktRssiValue 0x30, heard at 868.1 MHz.
/// let packet = PacketStatus::from_bytes([0xF6, 0x30], Port::High);
/// assert_eq!(packet.snr_db.to_string(), "-2.50");
/// assert_eq!(packet.rssi_dbm.to_string(), "-109.00");
/// assert_eq!(packet.signal_rssi_dbm.to_string(), "-111.50");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PacketStatus {
    /// The RSSI averaged over the packet, in dBm: the port's offset plus PacketRssi.
    pub rssi_dbm: Decibels,
    /// The estimated signal-to-noise ratio, in dB: PacketSnr, a two's complement byte,
    /// divided by 4.
    pub snr_db: Decibels,
    /// The strength of the packet itself, in dBm: the RSSI, lowered by the SNR when the
    /// packet arrived below the noise floor, as the datasheet computes it.
    pub signal_rssi_dbm: Decibels,
}

impl PacketStatus {
    /// Decodes RegPktSnrValue and RegPktRssiValue.
    ///
    /// # Arguments
    ///
    /// * `bytes` - PacketSnr and PacketRssi, the registers at 0x19 and 0x1A in that order.
    /// * `port` - the port the packet was received on.
    ///
    /// # Returns
    ///
    /// The three levels, exact to a hundredth of a decibel.
    pub const fn from_bytes(bytes: [u8; 2], port: Port) -> PacketStatus {
        let snr_quarters = bytes[0] as i8 as i32;
        let rssi = (port.rssi_offset_dbm() + bytes[1] as i32) * 100;
        let signal = if snr_quarters < 0 {
            rssi + snr_quarters * 25
        } else {
            rssi
        };
        PacketStatus {
            rssi_dbm: Decibels::from_hundredths(rssi),
            snr_db: Decibels::from_hundredths(snr_quarters * 25),
            signal_rssi_dbm: Decibels::from_hundredths(signal),
        }
    }
}

/// The live state of the LoRa modem, from RegModemStat.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::status::ModemStatus;
///
/// // 0x0F: a signal detected and synchronized, a reception under way with a valid header.
/// let modem = ModemStatus::from_byte(0x0F);
/// assert!(modem.signal_detected && modem.header_valid && modem.rx_ongoing);
/// assert!(!modem.clear);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModemStatus {
    /// The coding rate the last header announced, as the denominator of 4/5 to 4/8, or
    /// `None` for a reserved value.
    pub coding_rate_denominator: Option<u8>,
    /// Bit 4: the modem is clear.
    pub clear: bool,
    /// Bit 3: the header of the packet under way is valid.
    pub header_valid: bool,
    /// Bit 2: a reception is under way.
    pub rx_ongoing: bool,
    /// Bit 1: the modem has synchronized on the end of the preamble.
    pub signal_synchronized: bool,
    /// Bit 0: a LoRa preamble has been detected.
    pub signal_detected: bool,
}

impl ModemStatus {
    /// Decodes RegModemStat.
    ///
    /// # Arguments
    ///
    /// * `byte` - the register value.
    ///
    /// # Returns
    ///
    /// The modem's state.
    pub const fn from_byte(byte: u8) -> ModemStatus {
        let coding_rate = byte >> 5;
        ModemStatus {
            coding_rate_denominator: if coding_rate >= 1 && coding_rate <= 4 {
                Some(coding_rate + 4)
            } else {
                None
            },
            clear: byte & 0x10 != 0,
            header_valid: byte & 0x08 != 0,
            rx_ongoing: byte & 0x04 != 0,
            signal_synchronized: byte & 0x02 != 0,
            signal_detected: byte & 0x01 != 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_port_changes_at_the_mid_band_threshold() {
        assert_eq!(Port::for_frequency(433_175_000), Port::Low);
        assert_eq!(Port::for_frequency(MID_BAND_HZ), Port::Low);
        assert_eq!(Port::for_frequency(MID_BAND_HZ + 1), Port::High);
        assert_eq!(Port::for_frequency(915_000_000), Port::High);
    }

    #[test]
    fn a_packet_above_the_noise_floor_is_as_strong_as_its_rssi() {
        let packet = PacketStatus::from_bytes([0x1C, 0x7D], Port::High);
        assert_eq!(packet.snr_db, Decibels::from_hundredths(700));
        assert_eq!(packet.rssi_dbm, Decibels::from_db(-32));
        assert_eq!(packet.signal_rssi_dbm, packet.rssi_dbm);
    }

    #[test]
    fn a_packet_below_the_noise_floor_is_weaker_than_its_rssi_by_its_snr() {
        let packet = PacketStatus::from_bytes([0x80, 0x20], Port::Low);
        assert_eq!(packet.snr_db, Decibels::from_db(-32));
        assert_eq!(packet.rssi_dbm, Decibels::from_db(-132));
        assert_eq!(packet.signal_rssi_dbm, Decibels::from_db(-164));
    }

    #[test]
    fn the_modem_status_reads_its_coding_rate_from_the_top_bits() {
        assert_eq!(
            ModemStatus::from_byte(0x30).coding_rate_denominator,
            Some(5)
        );
        assert_eq!(
            ModemStatus::from_byte(0x90).coding_rate_denominator,
            Some(8)
        );
        assert_eq!(ModemStatus::from_byte(0x10).coding_rate_denominator, None);
        assert!(ModemStatus::from_byte(0x10).clear);
    }
}
