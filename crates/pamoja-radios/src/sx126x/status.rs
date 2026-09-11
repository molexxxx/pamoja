//! What an SX126x reports back: its status byte, its errors, and a received packet.
//!
//! Each decoder follows section 13.5 and 13.6 of the SX1261/2 datasheet. Signal levels
//! come out as [`Decibels`], held to a hundredth of a decibel, since the chip reports
//! power in half decibels and SNR in quarters, so nothing is rounded away.

use pamoja_lora::budget::Decibels;

/// The operating mode in bits 6 to 4 of the status byte, from Table 13-76.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChipMode {
    /// Standby on the 13 MHz RC oscillator (0x2).
    StandbyRc,
    /// Standby on the 32 MHz crystal oscillator (0x3).
    StandbyXosc,
    /// Frequency synthesis (0x4).
    Fs,
    /// Receive (0x5).
    Rx,
    /// Transmit (0x6).
    Tx,
    /// A value the datasheet leaves unused or reserved (0x0, 0x1, 0x7).
    Other(u8),
}

/// The outcome of the last command, in bits 3 to 1 of the status byte, from Table 13-76.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandStatus {
    /// A packet was received and its data can be read (0x2).
    DataAvailable,
    /// A transaction took too long and tripped the internal watchdog (0x3).
    Timeout,
    /// The opcode was invalid or the parameters were the wrong length (0x4).
    ProcessingError,
    /// The command was understood but could not be carried out (0x5).
    ExecutionFailure,
    /// The transmission of the current packet ended (0x6).
    TxDone,
    /// A value the datasheet leaves reserved (0x0, 0x1, 0x7).
    Other(u8),
}

/// The status byte an SX126x returns.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx126x::status::{ChipMode, CommandStatus, Status};
///
/// // 0x2C: standby on the RC oscillator after a transmission ended.
/// let status = Status::from_byte(0x2C);
/// assert_eq!(status.chip_mode, ChipMode::StandbyRc);
/// assert_eq!(status.command_status, CommandStatus::TxDone);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Status {
    /// The mode the chip is in.
    pub chip_mode: ChipMode,
    /// How the last command went.
    pub command_status: CommandStatus,
}

impl Status {
    /// Decodes a status byte.
    ///
    /// # Arguments
    ///
    /// * `byte` - the status byte.
    ///
    /// # Returns
    ///
    /// The chip mode and the command status it carries.
    pub const fn from_byte(byte: u8) -> Status {
        let chip_mode = match (byte >> 4) & 0x07 {
            0x2 => ChipMode::StandbyRc,
            0x3 => ChipMode::StandbyXosc,
            0x4 => ChipMode::Fs,
            0x5 => ChipMode::Rx,
            0x6 => ChipMode::Tx,
            other => ChipMode::Other(other),
        };
        let command_status = match (byte >> 1) & 0x07 {
            0x2 => CommandStatus::DataAvailable,
            0x3 => CommandStatus::Timeout,
            0x4 => CommandStatus::ProcessingError,
            0x5 => CommandStatus::ExecutionFailure,
            0x6 => CommandStatus::TxDone,
            other => CommandStatus::Other(other),
        };
        Status {
            chip_mode,
            command_status,
        }
    }

    /// Reports whether the last command failed.
    ///
    /// # Returns
    ///
    /// `true` for a watchdog timeout, a processing error, or an execution failure.
    pub const fn is_error(&self) -> bool {
        matches!(
            self.command_status,
            CommandStatus::Timeout | CommandStatus::ProcessingError | CommandStatus::ExecutionFailure
        )
    }
}

/// Where a received payload sits in the data buffer, from GetRxBufferStatus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RxBufferStatus {
    /// The length of the last received payload in bytes.
    pub payload_len: u8,
    /// The buffer offset of its first byte.
    pub start: u8,
}

impl RxBufferStatus {
    /// Decodes a GetRxBufferStatus answer.
    ///
    /// # Arguments
    ///
    /// * `bytes` - PayloadLengthRx and RxStartBufferPointer, in that order.
    ///
    /// # Returns
    ///
    /// The payload length and its offset.
    pub const fn from_bytes(bytes: [u8; 2]) -> RxBufferStatus {
        RxBufferStatus {
            payload_len: bytes[0],
            start: bytes[1],
        }
    }
}

/// The signal levels of the last LoRa packet received, from GetPacketStatus.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx126x::status::PacketStatus;
///
/// // RssiPkt 0xDB, SnrPkt 0xF6, SignalRssiPkt 0xE0.
/// let packet = PacketStatus::from_bytes([0xDB, 0xF6, 0xE0]);
/// assert_eq!(packet.rssi_dbm.to_string(), "-109.50");
/// assert_eq!(packet.snr_db.to_string(), "-2.50");
/// assert_eq!(packet.signal_rssi_dbm.to_string(), "-112.00");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PacketStatus {
    /// The RSSI averaged over the packet, in dBm: -RssiPkt/2.
    pub rssi_dbm: Decibels,
    /// The estimated signal-to-noise ratio, in dB: SnrPkt/4, a two's complement byte.
    pub snr_db: Decibels,
    /// The estimated RSSI of the LoRa signal after despreading, in dBm: -SignalRssiPkt/2.
    pub signal_rssi_dbm: Decibels,
}

impl PacketStatus {
    /// Decodes a LoRa GetPacketStatus answer.
    ///
    /// # Arguments
    ///
    /// * `bytes` - RssiPkt, SnrPkt, and SignalRssiPkt, in that order.
    ///
    /// # Returns
    ///
    /// The three levels, exact to a hundredth of a decibel.
    pub const fn from_bytes(bytes: [u8; 3]) -> PacketStatus {
        PacketStatus {
            rssi_dbm: Decibels::from_hundredths(-(bytes[0] as i32) * 50),
            snr_db: Decibels::from_hundredths((bytes[1] as i8) as i32 * 25),
            signal_rssi_dbm: Decibels::from_hundredths(-(bytes[2] as i32) * 50),
        }
    }
}

/// Decodes the instantaneous RSSI of a GetRssiInst answer, -RssiInst/2 in dBm.
///
/// # Arguments
///
/// * `byte` - the RssiInst byte.
///
/// # Returns
///
/// The received power in dBm.
pub const fn rssi_inst_dbm(byte: u8) -> Decibels {
    Decibels::from_hundredths(-(byte as i32) * 50)
}

/// The errors an SX126x has recorded, from GetDeviceErrors and Table 13-86.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DeviceErrors(u16);

impl DeviceErrors {
    /// Bit 0: the RC64k calibration failed.
    pub const RC64K_CALIBRATION: DeviceErrors = DeviceErrors(1 << 0);
    /// Bit 1: the RC13M calibration failed.
    pub const RC13M_CALIBRATION: DeviceErrors = DeviceErrors(1 << 1);
    /// Bit 2: the PLL calibration failed.
    pub const PLL_CALIBRATION: DeviceErrors = DeviceErrors(1 << 2);
    /// Bit 3: the ADC calibration failed.
    pub const ADC_CALIBRATION: DeviceErrors = DeviceErrors(1 << 3);
    /// Bit 4: the image calibration failed.
    pub const IMAGE_CALIBRATION: DeviceErrors = DeviceErrors(1 << 4);
    /// Bit 5: the crystal oscillator failed to start. Expected at power on and after a
    /// cold wake when a TCXO is fitted, and cleared with ClearDeviceErrors.
    pub const XOSC_START: DeviceErrors = DeviceErrors(1 << 5);
    /// Bit 6: the PLL failed to lock.
    pub const PLL_LOCK: DeviceErrors = DeviceErrors(1 << 6);
    /// Bit 8: the power amplifier failed to ramp.
    pub const PA_RAMP: DeviceErrors = DeviceErrors(1 << 8);

    /// Decodes a GetDeviceErrors answer.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the OpError bytes, most significant first.
    ///
    /// # Returns
    ///
    /// The recorded errors.
    pub const fn from_bytes(bytes: [u8; 2]) -> DeviceErrors {
        DeviceErrors(u16::from_be_bytes(bytes))
    }

    /// Returns the raw error bits.
    ///
    /// # Returns
    ///
    /// The OpError value.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Reports whether every error of another set was recorded.
    ///
    /// # Arguments
    ///
    /// * `other` - the errors to look for.
    ///
    /// # Returns
    ///
    /// `true` when all of them are set.
    pub const fn contains(self, other: DeviceErrors) -> bool {
        self.0 & other.0 == other.0
    }

    /// Reports whether no error was recorded.
    ///
    /// # Returns
    ///
    /// `true` when no bit is set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_chip_mode_and_command_status_decodes() {
        let modes = [
            (0x2, ChipMode::StandbyRc),
            (0x3, ChipMode::StandbyXosc),
            (0x4, ChipMode::Fs),
            (0x5, ChipMode::Rx),
            (0x6, ChipMode::Tx),
            (0x0, ChipMode::Other(0)),
        ];
        for (bits, mode) in modes {
            assert_eq!(Status::from_byte(bits << 4).chip_mode, mode);
        }
        let outcomes = [
            (0x2, CommandStatus::DataAvailable),
            (0x3, CommandStatus::Timeout),
            (0x4, CommandStatus::ProcessingError),
            (0x5, CommandStatus::ExecutionFailure),
            (0x6, CommandStatus::TxDone),
            (0x1, CommandStatus::Other(1)),
        ];
        for (bits, outcome) in outcomes {
            assert_eq!(Status::from_byte(bits << 1).command_status, outcome);
        }
    }

    #[test]
    fn the_failing_command_statuses_are_errors() {
        assert!(Status::from_byte(0x3 << 1).is_error());
        assert!(Status::from_byte(0x4 << 1).is_error());
        assert!(Status::from_byte(0x5 << 1).is_error());
        assert!(!Status::from_byte(0x6 << 1).is_error());
        assert!(!Status::from_byte(0x2 << 1).is_error());
    }

    #[test]
    fn the_packet_status_follows_the_datasheet_formulas() {
        for raw in [0u8, 1, 0x7F, 0x80, 0xFF] {
            let packet = PacketStatus::from_bytes([raw, raw, raw]);
            assert_eq!(packet.rssi_dbm.hundredths(), -(i32::from(raw)) * 100 / 2);
            assert_eq!(packet.snr_db.hundredths(), i32::from(raw as i8) * 100 / 4);
            assert_eq!(packet.signal_rssi_dbm, packet.rssi_dbm);
        }
        assert_eq!(rssi_inst_dbm(0x6F), Decibels::from_hundredths(-5_550));
    }

    #[test]
    fn the_buffer_status_and_device_errors_decode() {
        assert_eq!(
            RxBufferStatus::from_bytes([0x0A, 0x80]),
            RxBufferStatus {
                payload_len: 10,
                start: 0x80
            }
        );
        let errors = DeviceErrors::from_bytes([0x01, 0x20]);
        assert!(errors.contains(DeviceErrors::PA_RAMP));
        assert!(errors.contains(DeviceErrors::XOSC_START));
        assert!(!errors.contains(DeviceErrors::PLL_LOCK));
        assert!(DeviceErrors::default().is_empty());
    }
}
