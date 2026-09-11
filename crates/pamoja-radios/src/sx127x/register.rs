//! The SX127x register map, its SPI framing, and its operating modes.
//!
//! An SX1276, SX1277, SX1278, or SX1279 is driven through registers rather than commands.
//! Each access is one SPI transaction framed by NSS: an address byte whose most significant
//! bit is 1 for a write and 0 for a read, then the data bytes. The address advances with
//! each further byte, except at [`FIFO`], where every byte goes to or comes from the LoRa
//! data buffer. The addresses are those of Table 41 of the SX1276/77/78/79 datasheet
//! (Rev 7) with the LoRa register page selected, and [`Mode`] holds the Mode bits of
//! RegOpMode.

/// The bit of the address byte that makes an access a write.
pub const WRITE: u8 = 0x80;

/// RegFifo: the LoRa data buffer, read or written a byte at a time at [`FIFO_ADDR_PTR`].
pub const FIFO: u8 = 0x00;
/// RegOpMode: LoRa or FSK, the register page, and the operating mode.
pub const OP_MODE: u8 = 0x01;
/// RegFrfMsb: the most significant byte of the carrier frequency word.
pub const FRF_MSB: u8 = 0x06;
/// RegFrfMid: the middle byte of the carrier frequency word.
pub const FRF_MID: u8 = 0x07;
/// RegFrfLsb: the least significant byte of the carrier frequency word.
pub const FRF_LSB: u8 = 0x08;
/// RegPaConfig: the amplifier output pin, its maximum power, and the output power.
pub const PA_CONFIG: u8 = 0x09;
/// RegPaRamp: the amplifier ramp time.
pub const PA_RAMP: u8 = 0x0A;
/// RegOcp: the over current protection of the amplifier.
pub const OCP: u8 = 0x0B;
/// RegLna: the LNA gain and current.
pub const LNA: u8 = 0x0C;
/// RegFifoAddrPtr: where in the data buffer the next [`FIFO`] access lands.
pub const FIFO_ADDR_PTR: u8 = 0x0D;
/// RegFifoTxBaseAddr: where in the data buffer a transmitted payload starts.
pub const FIFO_TX_BASE_ADDR: u8 = 0x0E;
/// RegFifoRxBaseAddr: where in the data buffer received payloads start.
pub const FIFO_RX_BASE_ADDR: u8 = 0x0F;
/// RegFifoRxCurrentAddr: where the last received packet starts in the data buffer.
pub const FIFO_RX_CURRENT_ADDR: u8 = 0x10;
/// RegIrqFlagsMask: the interrupts masked off.
pub const IRQ_FLAGS_MASK: u8 = 0x11;
/// RegIrqFlags: the interrupts raised, each cleared by writing it back as a 1.
pub const IRQ_FLAGS: u8 = 0x12;
/// RegRxNbBytes: the payload length of the last packet received.
pub const RX_NB_BYTES: u8 = 0x13;
/// RegModemStat: the live state of the LoRa modem.
pub const MODEM_STAT: u8 = 0x18;
/// RegPktSnrValue: the SNR of the last packet, in quarters of a decibel.
pub const PKT_SNR_VALUE: u8 = 0x19;
/// RegPktRssiValue: the RSSI of the last packet.
pub const PKT_RSSI_VALUE: u8 = 0x1A;
/// RegRssiValue: the RSSI the receiver hears right now.
pub const RSSI_VALUE: u8 = 0x1B;
/// RegHopChannel: the PLL lock and the CRC the last header announced.
pub const HOP_CHANNEL: u8 = 0x1C;
/// RegModemConfig1: bandwidth, coding rate, and header mode.
pub const MODEM_CONFIG_1: u8 = 0x1D;
/// RegModemConfig2: spreading factor, CRC, and the top bits of the symbol timeout.
pub const MODEM_CONFIG_2: u8 = 0x1E;
/// RegSymbTimeoutLsb: the low byte of the single reception timeout, in symbols.
pub const SYMB_TIMEOUT_LSB: u8 = 0x1F;
/// RegPreambleMsb: the high byte of the preamble length.
pub const PREAMBLE_MSB: u8 = 0x20;
/// RegPreambleLsb: the low byte of the preamble length.
pub const PREAMBLE_LSB: u8 = 0x21;
/// RegPayloadLength: the payload length to transmit, or to expect in implicit header mode.
pub const PAYLOAD_LENGTH: u8 = 0x22;
/// RegMaxPayloadLength: the longest payload a received header may announce.
pub const MAX_PAYLOAD_LENGTH: u8 = 0x23;
/// RegModemConfig3: low data rate optimization and the automatic gain control.
pub const MODEM_CONFIG_3: u8 = 0x26;
/// RegRssiWideband: a wideband RSSI sample, noisy enough to seed a random number.
pub const RSSI_WIDEBAND: u8 = 0x2C;
/// RegIfFreq2, which the spurious reception erratum sets per bandwidth.
pub const IF_FREQ_2: u8 = 0x2F;
/// RegIfFreq1, which the spurious reception erratum clears.
pub const IF_FREQ_1: u8 = 0x30;
/// RegDetectOptimize: the automatic IF and the LoRa detection optimization.
pub const DETECT_OPTIMIZE: u8 = 0x31;
/// RegInvertIQ: the IQ polarity of the receive and transmit paths.
pub const INVERT_IQ: u8 = 0x33;
/// RegHighBwOptimize1, which the 500 kHz sensitivity erratum sets.
pub const HIGH_BW_OPTIMIZE_1: u8 = 0x36;
/// RegDetectionThreshold: the LoRa detection threshold.
pub const DETECTION_THRESHOLD: u8 = 0x37;
/// RegSyncWord: the LoRa sync word.
pub const SYNC_WORD: u8 = 0x39;
/// RegHighBwOptimize2, which the 500 kHz sensitivity erratum sets.
pub const HIGH_BW_OPTIMIZE_2: u8 = 0x3A;
/// RegInvertIQ2, which completes an IQ inversion.
pub const INVERT_IQ_2: u8 = 0x3B;
/// RegImageCal, at the same address as [`INVERT_IQ_2`] on the FSK register page.
pub const IMAGE_CAL: u8 = 0x3B;
/// RegDioMapping1: the events DIO0 to DIO3 signal.
pub const DIO_MAPPING_1: u8 = 0x40;
/// RegDioMapping2: the events DIO4 and DIO5 signal.
pub const DIO_MAPPING_2: u8 = 0x41;
/// RegVersion: the silicon revision.
pub const VERSION: u8 = 0x42;
/// RegTcxo: a crystal or a TCXO on the XTA pin.
pub const TCXO: u8 = 0x4B;
/// RegPaDac: the +20 dBm setting of the PA_BOOST amplifier.
pub const PA_DAC: u8 = 0x4D;

/// The RegVersion value of an SX1276, SX1277, SX1278, or SX1279, and of the modules built
/// on them.
pub const VERSION_SX1276: u8 = 0x12;

/// Returns the address byte that reads a register.
///
/// # Arguments
///
/// * `address` - the register address.
///
/// # Returns
///
/// The address with the write bit clear.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::register::{read_address, write_address, OP_MODE};
///
/// assert_eq!(read_address(OP_MODE), 0x01);
/// assert_eq!(write_address(OP_MODE), 0x81);
/// ```
pub const fn read_address(address: u8) -> u8 {
    address & !WRITE
}

/// Returns the address byte that writes a register.
///
/// # Arguments
///
/// * `address` - the register address.
///
/// # Returns
///
/// The address with the write bit set.
pub const fn write_address(address: u8) -> u8 {
    address | WRITE
}

/// An operating mode, the Mode bits 2 to 0 of RegOpMode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    /// 000: only the SPI interface and the registers are powered, and the data buffer is
    /// lost. The only mode in which the chip may switch between LoRa and FSK.
    Sleep,
    /// 001: the crystal oscillator and the LoRa baseband are on.
    Standby,
    /// 010: the PLL is locked on the transmit frequency.
    FsTx,
    /// 011: one packet goes out, then the chip returns to standby.
    Tx,
    /// 100: the PLL is locked on the receive frequency.
    FsRx,
    /// 101: the receiver takes packet after packet until told otherwise.
    RxContinuous,
    /// 110: the receiver waits for one packet or the symbol timeout, then returns to
    /// standby.
    RxSingle,
    /// 111: channel activity detection looks for a LoRa preamble.
    Cad,
}

impl Mode {
    /// Returns the Mode bits.
    ///
    /// # Returns
    ///
    /// The three bit code.
    pub const fn code(self) -> u8 {
        match self {
            Mode::Sleep => 0b000,
            Mode::Standby => 0b001,
            Mode::FsTx => 0b010,
            Mode::Tx => 0b011,
            Mode::FsRx => 0b100,
            Mode::RxContinuous => 0b101,
            Mode::RxSingle => 0b110,
            Mode::Cad => 0b111,
        }
    }

    /// Decodes the Mode bits of a RegOpMode value.
    ///
    /// # Arguments
    ///
    /// * `op_mode` - the RegOpMode value; only bits 2 to 0 are read.
    ///
    /// # Returns
    ///
    /// The mode.
    pub const fn from_op_mode(op_mode: u8) -> Mode {
        match op_mode & 0b111 {
            0b000 => Mode::Sleep,
            0b001 => Mode::Standby,
            0b010 => Mode::FsTx,
            0b011 => Mode::Tx,
            0b100 => Mode::FsRx,
            0b101 => Mode::RxContinuous,
            0b110 => Mode::RxSingle,
            _ => Mode::Cad,
        }
    }
}

/// RegOpMode bit 7: the LoRa modem rather than FSK, changed only in [`Mode::Sleep`].
pub const LONG_RANGE_MODE: u8 = 0x80;
/// RegOpMode bit 6: the FSK register page while in LoRa mode.
pub const ACCESS_SHARED_REG: u8 = 0x40;
/// RegOpMode bit 3: the low frequency test registers, set at reset.
pub const LOW_FREQUENCY_MODE_ON: u8 = 0x08;

/// Returns the RegOpMode value for a LoRa operating mode.
///
/// The LoRa register page is selected and LowFrequencyModeOn keeps its reset value, since
/// it only chooses which test registers are reachable.
///
/// # Arguments
///
/// * `mode` - the operating mode.
///
/// # Returns
///
/// The RegOpMode value.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::register::{lora_op_mode, Mode};
///
/// assert_eq!(lora_op_mode(Mode::Sleep), 0x88);
/// assert_eq!(lora_op_mode(Mode::Tx), 0x8B);
/// ```
pub const fn lora_op_mode(mode: Mode) -> u8 {
    LONG_RANGE_MODE | LOW_FREQUENCY_MODE_ON | mode.code()
}

/// Returns the RegOpMode value for an FSK operating mode, which image calibration needs.
///
/// # Arguments
///
/// * `mode` - the operating mode.
///
/// # Returns
///
/// The RegOpMode value, with LowFrequencyModeOn at its reset value.
pub const fn fsk_op_mode(mode: Mode) -> u8 {
    LOW_FREQUENCY_MODE_ON | mode.code()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_round_trips_through_its_bits() {
        for mode in [
            Mode::Sleep,
            Mode::Standby,
            Mode::FsTx,
            Mode::Tx,
            Mode::FsRx,
            Mode::RxContinuous,
            Mode::RxSingle,
            Mode::Cad,
        ] {
            assert_eq!(Mode::from_op_mode(lora_op_mode(mode)), mode);
            assert_eq!(Mode::from_op_mode(fsk_op_mode(mode)), mode);
        }
    }

    #[test]
    fn the_op_mode_values_match_the_reset_state_and_the_reference_driver() {
        assert_eq!(
            fsk_op_mode(Mode::Standby),
            0x09,
            "the RegOpMode reset value"
        );
        assert_eq!(lora_op_mode(Mode::Standby), 0x89);
        assert_eq!(lora_op_mode(Mode::RxContinuous) & 0x07, 0x05);
        assert_eq!(lora_op_mode(Mode::RxSingle) & 0x07, 0x06);
    }

    #[test]
    fn the_address_byte_carries_the_access_direction_in_its_top_bit() {
        assert_eq!(write_address(FIFO), 0x80);
        assert_eq!(read_address(VERSION), 0x42);
        assert_eq!(write_address(PA_DAC), 0xCD);
        assert_eq!(read_address(write_address(SYNC_WORD)), SYNC_WORD);
    }
}
