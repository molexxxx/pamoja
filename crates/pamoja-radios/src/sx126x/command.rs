//! The SX126x command set: opcodes and the bytes each command sends.
//!
//! A command is an opcode followed by its parameters, multi-byte values most significant
//! byte first, framed by NSS in a single SPI transaction, as chapter 10 of the SX1261/2
//! datasheet describes. [`Command`] holds the bytes of one command without allocating,
//! and the builders here fill it from typed parameters. A command that answers comes as
//! a [`Query`]: the bytes to send, then how many bytes to read back while NSS stays low.
//! WriteRegister and WriteBuffer send a header built here, then their data, in the same
//! transaction.

use super::config::{
    FallbackMode, LoraModulation, LoraPacket, PaConfig, PacketType, RampTime, RegulatorMode,
    StandbyMode, TcxoVoltage,
};
use super::irq::Irq;

/// The opcodes of Tables 11-1 to 11-5.
pub mod opcode {
    /// SetSleep.
    pub const SET_SLEEP: u8 = 0x84;
    /// SetStandby.
    pub const SET_STANDBY: u8 = 0x80;
    /// SetFs.
    pub const SET_FS: u8 = 0xC1;
    /// SetTx.
    pub const SET_TX: u8 = 0x83;
    /// SetRx.
    pub const SET_RX: u8 = 0x82;
    /// StopTimerOnPreamble.
    pub const STOP_TIMER_ON_PREAMBLE: u8 = 0x9F;
    /// SetRxDutyCycle.
    pub const SET_RX_DUTY_CYCLE: u8 = 0x94;
    /// SetCad.
    pub const SET_CAD: u8 = 0xC5;
    /// SetTxContinuousWave.
    pub const SET_TX_CONTINUOUS_WAVE: u8 = 0xD1;
    /// SetTxInfinitePreamble.
    pub const SET_TX_INFINITE_PREAMBLE: u8 = 0xD2;
    /// SetRegulatorMode.
    pub const SET_REGULATOR_MODE: u8 = 0x96;
    /// Calibrate.
    pub const CALIBRATE: u8 = 0x89;
    /// CalibrateImage.
    pub const CALIBRATE_IMAGE: u8 = 0x98;
    /// SetPaConfig.
    pub const SET_PA_CONFIG: u8 = 0x95;
    /// SetRxTxFallbackMode.
    pub const SET_RX_TX_FALLBACK_MODE: u8 = 0x93;
    /// WriteRegister.
    pub const WRITE_REGISTER: u8 = 0x0D;
    /// ReadRegister.
    pub const READ_REGISTER: u8 = 0x1D;
    /// WriteBuffer.
    pub const WRITE_BUFFER: u8 = 0x0E;
    /// ReadBuffer.
    pub const READ_BUFFER: u8 = 0x1E;
    /// SetDioIrqParams.
    pub const SET_DIO_IRQ_PARAMS: u8 = 0x08;
    /// GetIrqStatus.
    pub const GET_IRQ_STATUS: u8 = 0x12;
    /// ClearIrqStatus.
    pub const CLEAR_IRQ_STATUS: u8 = 0x02;
    /// SetDIO2AsRfSwitchCtrl.
    pub const SET_DIO2_AS_RF_SWITCH_CTRL: u8 = 0x9D;
    /// SetDIO3AsTcxoCtrl.
    pub const SET_DIO3_AS_TCXO_CTRL: u8 = 0x97;
    /// SetRfFrequency.
    pub const SET_RF_FREQUENCY: u8 = 0x86;
    /// SetPacketType.
    pub const SET_PACKET_TYPE: u8 = 0x8A;
    /// GetPacketType.
    pub const GET_PACKET_TYPE: u8 = 0x11;
    /// SetTxParams.
    pub const SET_TX_PARAMS: u8 = 0x8E;
    /// SetModulationParams.
    pub const SET_MODULATION_PARAMS: u8 = 0x8B;
    /// SetPacketParams.
    pub const SET_PACKET_PARAMS: u8 = 0x8C;
    /// SetCadParams.
    pub const SET_CAD_PARAMS: u8 = 0x88;
    /// SetBufferBaseAddress.
    pub const SET_BUFFER_BASE_ADDRESS: u8 = 0x8F;
    /// SetLoRaSymbNumTimeout.
    pub const SET_LORA_SYMB_NUM_TIMEOUT: u8 = 0xA0;
    /// GetStatus.
    pub const GET_STATUS: u8 = 0xC0;
    /// GetRssiInst.
    pub const GET_RSSI_INST: u8 = 0x15;
    /// GetRxBufferStatus.
    pub const GET_RX_BUFFER_STATUS: u8 = 0x13;
    /// GetPacketStatus.
    pub const GET_PACKET_STATUS: u8 = 0x14;
    /// GetDeviceErrors.
    pub const GET_DEVICE_ERRORS: u8 = 0x17;
    /// ClearDeviceErrors.
    pub const CLEAR_DEVICE_ERRORS: u8 = 0x07;
    /// GetStats.
    pub const GET_STATS: u8 = 0x10;
    /// ResetStats.
    pub const RESET_STATS: u8 = 0x00;
}

/// The byte a host sends while it clocks an answer back.
pub const NOP: u8 = 0x00;

/// The calibration parameter that recalibrates every block, from Table 13-18.
pub const CALIBRATE_ALL: u8 = 0x7F;

/// The longest command the builders produce, in bytes.
pub const MAX_LEN: usize = 10;

/// The bytes of one command.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx126x::command;
///
/// assert_eq!(command::set_rf_frequency(0x3641_999A).as_bytes(), [0x86, 0x36, 0x41, 0x99, 0x9A]);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Command {
    bytes: [u8; MAX_LEN],
    len: u8,
}

impl Command {
    /// Builds a command from its opcode and its parameter bytes.
    ///
    /// # Arguments
    ///
    /// * `opcode` - the opcode.
    /// * `params` - the parameters, at most nine bytes.
    ///
    /// # Returns
    ///
    /// The command.
    ///
    /// # Panics
    ///
    /// Panics if `params` is longer than nine bytes.
    pub const fn new(opcode: u8, params: &[u8]) -> Command {
        assert!(params.len() < MAX_LEN, "an SX126x command carries at most nine parameter bytes");
        let mut bytes = [0u8; MAX_LEN];
        bytes[0] = opcode;
        let mut index = 0;
        while index < params.len() {
            bytes[index + 1] = params[index];
            index += 1;
        }
        Command {
            bytes,
            len: params.len() as u8 + 1,
        }
    }

    /// Returns the bytes to send.
    ///
    /// # Returns
    ///
    /// The opcode followed by the parameters.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }

    /// Returns the opcode.
    ///
    /// # Returns
    ///
    /// The first byte.
    pub const fn opcode(&self) -> u8 {
        self.bytes[0]
    }
}

impl core::fmt::Debug for Command {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Command").field(&self.as_bytes()).finish()
    }
}

/// A command that the chip answers in the same transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Query {
    /// The bytes to send, including the NOP bytes that precede the answer.
    pub command: Command,
    /// How many bytes of answer to read after them.
    pub answer_len: usize,
}

const fn u24(value: u32) -> [u8; 3] {
    [(value >> 16) as u8, (value >> 8) as u8, value as u8]
}

/// SetSleep: puts the chip to sleep, from Table 13-2.
///
/// # Arguments
///
/// * `warm_start` - `true` to keep the configuration in retention memory.
/// * `rtc_wake` - `true` to wake on the RTC timeout.
///
/// # Returns
///
/// The command.
pub const fn set_sleep(warm_start: bool, rtc_wake: bool) -> Command {
    Command::new(
        opcode::SET_SLEEP,
        &[((warm_start as u8) << 2) | rtc_wake as u8],
    )
}

/// SetStandby: puts the chip in a standby mode.
///
/// # Arguments
///
/// * `mode` - the oscillator the chip stands by on.
///
/// # Returns
///
/// The command.
pub const fn set_standby(mode: StandbyMode) -> Command {
    Command::new(opcode::SET_STANDBY, &[mode.code()])
}

/// SetFs: puts the chip in frequency synthesis mode.
///
/// # Returns
///
/// The command.
pub const fn set_fs() -> Command {
    Command::new(opcode::SET_FS, &[])
}

/// SetTx: starts a transmission.
///
/// # Arguments
///
/// * `timeout` - the 24-bit timeout word; [`NO_TIMEOUT`](super::config::NO_TIMEOUT)
///   transmits once with no timeout.
///
/// # Returns
///
/// The command.
pub const fn set_tx(timeout: u32) -> Command {
    Command::new(opcode::SET_TX, &u24(timeout))
}

/// SetRx: starts receiving.
///
/// # Arguments
///
/// * `timeout` - the 24-bit timeout word; [`NO_TIMEOUT`](super::config::NO_TIMEOUT)
///   receives one packet, [`RX_CONTINUOUS`](super::config::RX_CONTINUOUS) keeps
///   listening.
///
/// # Returns
///
/// The command.
pub const fn set_rx(timeout: u32) -> Command {
    Command::new(opcode::SET_RX, &u24(timeout))
}

/// StopTimerOnPreamble: chooses what stops the receive timeout.
///
/// # Arguments
///
/// * `on_preamble` - `true` to stop the timer on a preamble, `false` on a header or
///   sync word.
///
/// # Returns
///
/// The command.
pub const fn stop_timer_on_preamble(on_preamble: bool) -> Command {
    Command::new(opcode::STOP_TIMER_ON_PREAMBLE, &[on_preamble as u8])
}

/// SetRxDutyCycle: alternates the chip between listening and sleeping.
///
/// # Arguments
///
/// * `rx_period` - the 24-bit listening period word.
/// * `sleep_period` - the 24-bit sleeping period word.
///
/// # Returns
///
/// The command.
pub const fn set_rx_duty_cycle(rx_period: u32, sleep_period: u32) -> Command {
    let rx = u24(rx_period);
    let sleep = u24(sleep_period);
    Command::new(
        opcode::SET_RX_DUTY_CYCLE,
        &[rx[0], rx[1], rx[2], sleep[0], sleep[1], sleep[2]],
    )
}

/// SetCad: starts channel activity detection.
///
/// # Returns
///
/// The command.
pub const fn set_cad() -> Command {
    Command::new(opcode::SET_CAD, &[])
}

/// SetTxContinuousWave: transmits an unmodulated carrier, a test command.
///
/// # Returns
///
/// The command.
pub const fn set_tx_continuous_wave() -> Command {
    Command::new(opcode::SET_TX_CONTINUOUS_WAVE, &[])
}

/// SetTxInfinitePreamble: transmits preamble symbols without end, a test command.
///
/// # Returns
///
/// The command.
pub const fn set_tx_infinite_preamble() -> Command {
    Command::new(opcode::SET_TX_INFINITE_PREAMBLE, &[])
}

/// SetRegulatorMode: selects the LDO or the DC-DC converter.
///
/// # Arguments
///
/// * `mode` - the regulator.
///
/// # Returns
///
/// The command.
pub const fn set_regulator_mode(mode: RegulatorMode) -> Command {
    Command::new(opcode::SET_REGULATOR_MODE, &[mode.code()])
}

/// Calibrate: recalibrates blocks of the chip, from Table 13-18.
///
/// # Arguments
///
/// * `blocks` - one bit per block, such as [`CALIBRATE_ALL`].
///
/// # Returns
///
/// The command.
pub const fn calibrate(blocks: u8) -> Command {
    Command::new(opcode::CALIBRATE, &[blocks])
}

/// CalibrateImage: calibrates image rejection over a band.
///
/// # Arguments
///
/// * `codes` - `freq1` and `freq2`, such as
///   [`image_calibration`](super::config::image_calibration) returns.
///
/// # Returns
///
/// The command.
pub const fn calibrate_image(codes: [u8; 2]) -> Command {
    Command::new(opcode::CALIBRATE_IMAGE, &codes)
}

/// SetPaConfig: configures the power amplifier.
///
/// # Arguments
///
/// * `pa` - the amplifier configuration.
///
/// # Returns
///
/// The command.
pub const fn set_pa_config(pa: PaConfig) -> Command {
    Command::new(opcode::SET_PA_CONFIG, &pa.to_params())
}

/// SetRxTxFallbackMode: chooses the mode the chip returns to after a packet.
///
/// # Arguments
///
/// * `mode` - the fallback mode.
///
/// # Returns
///
/// The command.
pub const fn set_rx_tx_fallback_mode(mode: FallbackMode) -> Command {
    Command::new(opcode::SET_RX_TX_FALLBACK_MODE, &[mode.code()])
}

/// The header of WriteRegister; the register bytes follow in the same transaction.
///
/// # Arguments
///
/// * `address` - the first register to write.
///
/// # Returns
///
/// The opcode and the address.
pub const fn write_register(address: u16) -> Command {
    let address = address.to_be_bytes();
    Command::new(opcode::WRITE_REGISTER, &address)
}

/// ReadRegister: reads consecutive registers.
///
/// # Arguments
///
/// * `address` - the first register to read.
/// * `len` - how many registers to read.
///
/// # Returns
///
/// The opcode, the address, and the NOP before the answer.
pub const fn read_register(address: u16, len: usize) -> Query {
    let address = address.to_be_bytes();
    Query {
        command: Command::new(opcode::READ_REGISTER, &[address[0], address[1], NOP]),
        answer_len: len,
    }
}

/// The header of WriteBuffer; the payload follows in the same transaction.
///
/// # Arguments
///
/// * `offset` - where in the data buffer the payload starts.
///
/// # Returns
///
/// The opcode and the offset.
pub const fn write_buffer(offset: u8) -> Command {
    Command::new(opcode::WRITE_BUFFER, &[offset])
}

/// ReadBuffer: reads the data buffer.
///
/// # Arguments
///
/// * `offset` - where in the data buffer to start.
/// * `len` - how many bytes to read.
///
/// # Returns
///
/// The opcode, the offset, and the NOP before the answer.
pub const fn read_buffer(offset: u8, len: usize) -> Query {
    Query {
        command: Command::new(opcode::READ_BUFFER, &[offset, NOP]),
        answer_len: len,
    }
}

/// SetDioIrqParams: enables interrupts and routes them to the DIO lines.
///
/// # Arguments
///
/// * `irq` - the interrupts to enable.
/// * `dio1` - the interrupts that raise DIO1.
/// * `dio2` - the interrupts that raise DIO2.
/// * `dio3` - the interrupts that raise DIO3.
///
/// # Returns
///
/// The command.
pub const fn set_dio_irq_params(irq: Irq, dio1: Irq, dio2: Irq, dio3: Irq) -> Command {
    let [a, b] = irq.to_bytes();
    let [c, d] = dio1.to_bytes();
    let [e, f] = dio2.to_bytes();
    let [g, h] = dio3.to_bytes();
    Command::new(opcode::SET_DIO_IRQ_PARAMS, &[a, b, c, d, e, f, g, h])
}

/// GetIrqStatus: reads the pending interrupts.
///
/// # Returns
///
/// The query, answered by the two IrqStatus bytes.
pub const fn get_irq_status() -> Query {
    Query {
        command: Command::new(opcode::GET_IRQ_STATUS, &[NOP]),
        answer_len: 2,
    }
}

/// ClearIrqStatus: clears interrupts.
///
/// # Arguments
///
/// * `irq` - the interrupts to clear.
///
/// # Returns
///
/// The command.
pub const fn clear_irq_status(irq: Irq) -> Command {
    Command::new(opcode::CLEAR_IRQ_STATUS, &irq.to_bytes())
}

/// SetDIO2AsRfSwitchCtrl: lets DIO2 drive an RF switch, high in TX.
///
/// # Arguments
///
/// * `enable` - `true` to drive the switch from DIO2.
///
/// # Returns
///
/// The command.
pub const fn set_dio2_as_rf_switch(enable: bool) -> Command {
    Command::new(opcode::SET_DIO2_AS_RF_SWITCH_CTRL, &[enable as u8])
}

/// SetDIO3AsTcxoCtrl: powers a TCXO from DIO3.
///
/// # Arguments
///
/// * `voltage` - the supply voltage.
/// * `delay` - the 24-bit word for how long the TCXO takes to settle.
///
/// # Returns
///
/// The command.
pub const fn set_dio3_as_tcxo(voltage: TcxoVoltage, delay: u32) -> Command {
    let delay = u24(delay);
    Command::new(
        opcode::SET_DIO3_AS_TCXO_CTRL,
        &[voltage.code(), delay[0], delay[1], delay[2]],
    )
}

/// SetRfFrequency: tunes the synthesizer.
///
/// # Arguments
///
/// * `word` - the frequency word, such as
///   [`frequency_word`](super::config::frequency_word) returns.
///
/// # Returns
///
/// The command.
pub const fn set_rf_frequency(word: u32) -> Command {
    Command::new(opcode::SET_RF_FREQUENCY, &word.to_be_bytes())
}

/// SetPacketType: selects the modem, the first command of any configuration.
///
/// # Arguments
///
/// * `packet_type` - the modem.
///
/// # Returns
///
/// The command.
pub const fn set_packet_type(packet_type: PacketType) -> Command {
    Command::new(opcode::SET_PACKET_TYPE, &[packet_type.code()])
}

/// GetPacketType: reads the selected modem.
///
/// # Returns
///
/// The query, answered by the packet type byte.
pub const fn get_packet_type() -> Query {
    Query {
        command: Command::new(opcode::GET_PACKET_TYPE, &[NOP]),
        answer_len: 1,
    }
}

/// SetTxParams: sets the output power and the ramp time.
///
/// # Arguments
///
/// * `power_dbm` - the power setting, in the range of the selected amplifier.
/// * `ramp` - the ramp time.
///
/// # Returns
///
/// The command.
pub const fn set_tx_params(power_dbm: i8, ramp: RampTime) -> Command {
    Command::new(opcode::SET_TX_PARAMS, &[power_dbm as u8, ramp.code()])
}

/// SetModulationParams, for LoRa.
///
/// # Arguments
///
/// * `modulation` - the spreading factor, bandwidth, coding rate, and low data rate
///   optimization.
///
/// # Returns
///
/// The command.
pub const fn set_lora_modulation_params(modulation: LoraModulation) -> Command {
    Command::new(opcode::SET_MODULATION_PARAMS, &modulation.to_params())
}

/// SetPacketParams, for LoRa.
///
/// # Arguments
///
/// * `packet` - the preamble, header type, payload length, CRC, and IQ polarity.
///
/// # Returns
///
/// The command.
pub const fn set_lora_packet_params(packet: LoraPacket) -> Command {
    Command::new(opcode::SET_PACKET_PARAMS, &packet.to_params())
}

/// SetCadParams: configures channel activity detection, from Table 13-71.
///
/// # Arguments
///
/// * `symbols` - the cadSymbolNum code, 0x00 to 0x04 for 1 to 16 symbols.
/// * `detect_peak` - cadDetPeak.
/// * `detect_min` - cadDetMin.
/// * `exit_mode` - cadExitMode: 0x00 to return to standby, 0x01 to receive on activity.
/// * `timeout` - the 24-bit receive timeout word after activity is detected.
///
/// # Returns
///
/// The command.
pub const fn set_cad_params(
    symbols: u8,
    detect_peak: u8,
    detect_min: u8,
    exit_mode: u8,
    timeout: u32,
) -> Command {
    let timeout = u24(timeout);
    Command::new(
        opcode::SET_CAD_PARAMS,
        &[
            symbols,
            detect_peak,
            detect_min,
            exit_mode,
            timeout[0],
            timeout[1],
            timeout[2],
        ],
    )
}

/// SetBufferBaseAddress: sets where transmit and receive data start in the buffer.
///
/// # Arguments
///
/// * `tx` - the transmit base address.
/// * `rx` - the receive base address.
///
/// # Returns
///
/// The command.
pub const fn set_buffer_base_address(tx: u8, rx: u8) -> Command {
    Command::new(opcode::SET_BUFFER_BASE_ADDRESS, &[tx, rx])
}

/// SetLoRaSymbNumTimeout: sets how many symbols confirm a detected packet.
///
/// # Arguments
///
/// * `symbols` - the symbol count: even numbers up to 64, then steps of 8 up to 248.
///
/// # Returns
///
/// The command.
pub const fn set_lora_symbol_timeout(symbols: u8) -> Command {
    Command::new(opcode::SET_LORA_SYMB_NUM_TIMEOUT, &[symbols])
}

/// GetStatus: reads the status byte.
///
/// # Returns
///
/// The query, answered by the status byte.
pub const fn get_status() -> Query {
    Query {
        command: Command::new(opcode::GET_STATUS, &[]),
        answer_len: 1,
    }
}

/// GetRssiInst: reads the instantaneous RSSI while receiving.
///
/// # Returns
///
/// The query, answered by the RssiInst byte.
pub const fn get_rssi_inst() -> Query {
    Query {
        command: Command::new(opcode::GET_RSSI_INST, &[NOP]),
        answer_len: 1,
    }
}

/// GetRxBufferStatus: reads where the last payload sits.
///
/// # Returns
///
/// The query, answered by PayloadLengthRx and RxStartBufferPointer.
pub const fn get_rx_buffer_status() -> Query {
    Query {
        command: Command::new(opcode::GET_RX_BUFFER_STATUS, &[NOP]),
        answer_len: 2,
    }
}

/// GetPacketStatus: reads the signal levels of the last packet.
///
/// # Returns
///
/// The query, answered in LoRa by RssiPkt, SnrPkt, and SignalRssiPkt.
pub const fn get_packet_status() -> Query {
    Query {
        command: Command::new(opcode::GET_PACKET_STATUS, &[NOP]),
        answer_len: 3,
    }
}

/// GetDeviceErrors: reads the recorded errors.
///
/// # Returns
///
/// The query, answered by the two OpError bytes.
pub const fn get_device_errors() -> Query {
    Query {
        command: Command::new(opcode::GET_DEVICE_ERRORS, &[NOP]),
        answer_len: 2,
    }
}

/// ClearDeviceErrors: clears every recorded error.
///
/// # Returns
///
/// The command.
pub const fn clear_device_errors() -> Command {
    Command::new(opcode::CLEAR_DEVICE_ERRORS, &[0x00, 0x00])
}

/// GetStats: reads the packet counters.
///
/// # Returns
///
/// The query, answered in LoRa by the received, CRC error, and header error counts.
pub const fn get_stats() -> Query {
    Query {
        command: Command::new(opcode::GET_STATS, &[NOP]),
        answer_len: 6,
    }
}

/// ResetStats: zeroes the packet counters, an opcode and six zero bytes.
///
/// # Returns
///
/// The command.
pub const fn reset_stats() -> Command {
    Command::new(opcode::RESET_STATS, &[0; 6])
}

#[cfg(test)]
mod tests {
    use super::super::config::{
        CodingRate, LoraBandwidth, NO_TIMEOUT, RX_CONTINUOUS,
    };
    use super::*;

    #[test]
    fn the_operating_mode_commands_follow_table_11_1() {
        assert_eq!(set_sleep(false, false).as_bytes(), [0x84, 0x00]);
        assert_eq!(set_sleep(true, true).as_bytes(), [0x84, 0x05]);
        assert_eq!(set_standby(StandbyMode::Rc).as_bytes(), [0x80, 0x00]);
        assert_eq!(set_standby(StandbyMode::Xosc).as_bytes(), [0x80, 0x01]);
        assert_eq!(set_fs().as_bytes(), [0xC1]);
        assert_eq!(set_tx(NO_TIMEOUT).as_bytes(), [0x83, 0x00, 0x00, 0x00]);
        assert_eq!(set_rx(RX_CONTINUOUS).as_bytes(), [0x82, 0xFF, 0xFF, 0xFF]);
        assert_eq!(set_rx(0x01_2345).as_bytes(), [0x82, 0x01, 0x23, 0x45]);
        assert_eq!(stop_timer_on_preamble(true).as_bytes(), [0x9F, 0x01]);
        assert_eq!(
            set_rx_duty_cycle(0x00_0100, 0x02_0000).as_bytes(),
            [0x94, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00]
        );
        assert_eq!(set_cad().as_bytes(), [0xC5]);
        assert_eq!(set_tx_continuous_wave().as_bytes(), [0xD1]);
        assert_eq!(set_tx_infinite_preamble().as_bytes(), [0xD2]);
        assert_eq!(set_regulator_mode(RegulatorMode::DcDc).as_bytes(), [0x96, 0x01]);
        assert_eq!(calibrate(CALIBRATE_ALL).as_bytes(), [0x89, 0x7F]);
        assert_eq!(calibrate_image([0xD7, 0xDA]).as_bytes(), [0x98, 0xD7, 0xDA]);
        assert_eq!(
            set_pa_config(PaConfig::SX1262_22_DBM).as_bytes(),
            [0x95, 0x04, 0x07, 0x00, 0x01]
        );
        assert_eq!(
            set_rx_tx_fallback_mode(FallbackMode::StandbyXosc).as_bytes(),
            [0x93, 0x30]
        );
    }

    #[test]
    fn the_register_and_buffer_commands_follow_tables_13_24_to_13_27() {
        assert_eq!(write_register(0x0740).as_bytes(), [0x0D, 0x07, 0x40]);
        let read = read_register(0x08D8, 1);
        assert_eq!(read.command.as_bytes(), [0x1D, 0x08, 0xD8, 0x00]);
        assert_eq!(read.answer_len, 1);
        assert_eq!(write_buffer(0x00).as_bytes(), [0x0E, 0x00]);
        let buffer = read_buffer(0x80, 12);
        assert_eq!(buffer.command.as_bytes(), [0x1E, 0x80, 0x00]);
        assert_eq!(buffer.answer_len, 12);
    }

    #[test]
    fn the_irq_and_dio_commands_follow_tables_13_28_to_13_34() {
        let done = Irq::TX_DONE | Irq::RX_DONE | Irq::TIMEOUT;
        assert_eq!(
            set_dio_irq_params(done, done, Irq::NONE, Irq::NONE).as_bytes(),
            [0x08, 0x02, 0x03, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(get_irq_status().command.as_bytes(), [0x12, 0x00]);
        assert_eq!(get_irq_status().answer_len, 2);
        assert_eq!(clear_irq_status(Irq::ALL).as_bytes(), [0x02, 0x43, 0xFF]);
        assert_eq!(set_dio2_as_rf_switch(true).as_bytes(), [0x9D, 0x01]);
        assert_eq!(
            set_dio3_as_tcxo(TcxoVoltage::V1_8, 320).as_bytes(),
            [0x97, 0x02, 0x00, 0x01, 0x40]
        );
    }

    #[test]
    fn the_rf_and_packet_commands_follow_tables_13_36_to_13_75() {
        assert_eq!(set_packet_type(PacketType::Lora).as_bytes(), [0x8A, 0x01]);
        assert_eq!(get_packet_type().command.as_bytes(), [0x11, 0x00]);
        assert_eq!(set_tx_params(22, RampTime::Us200).as_bytes(), [0x8E, 0x16, 0x04]);
        assert_eq!(set_tx_params(-9, RampTime::Us10).as_bytes(), [0x8E, 0xF7, 0x00]);
        let modulation = LoraModulation {
            spreading_factor: 9,
            bandwidth: LoraBandwidth::Khz125,
            coding_rate: CodingRate::Cr4_5,
            low_data_rate_optimization: false,
        };
        assert_eq!(
            set_lora_modulation_params(modulation).as_bytes(),
            [0x8B, 0x09, 0x04, 0x01, 0x00]
        );
        let packet = LoraPacket {
            preamble_symbols: 8,
            explicit_header: true,
            payload_len: 255,
            crc: true,
            invert_iq: false,
        };
        assert_eq!(
            set_lora_packet_params(packet).as_bytes(),
            [0x8C, 0x00, 0x08, 0x00, 0xFF, 0x01, 0x00]
        );
        assert_eq!(
            set_cad_params(0x02, 22, 10, 0x00, 0).as_bytes(),
            [0x88, 0x02, 0x16, 0x0A, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(set_buffer_base_address(0x00, 0x80).as_bytes(), [0x8F, 0x00, 0x80]);
        assert_eq!(set_lora_symbol_timeout(6).as_bytes(), [0xA0, 0x06]);
    }

    #[test]
    fn the_status_commands_follow_tables_13_77_to_13_87() {
        assert_eq!(get_status().command.as_bytes(), [0xC0]);
        assert_eq!(get_status().answer_len, 1);
        assert_eq!(get_rssi_inst().command.as_bytes(), [0x15, 0x00]);
        assert_eq!(get_rx_buffer_status().command.as_bytes(), [0x13, 0x00]);
        assert_eq!(get_rx_buffer_status().answer_len, 2);
        assert_eq!(get_packet_status().command.as_bytes(), [0x14, 0x00]);
        assert_eq!(get_packet_status().answer_len, 3);
        assert_eq!(get_device_errors().command.as_bytes(), [0x17, 0x00]);
        assert_eq!(clear_device_errors().as_bytes(), [0x07, 0x00, 0x00]);
        assert_eq!(get_stats().command.as_bytes(), [0x10, 0x00]);
        assert_eq!(get_stats().answer_len, 6);
        assert_eq!(reset_stats().as_bytes(), [0x00; 7]);
    }

    #[test]
    fn a_command_knows_its_opcode_and_length() {
        let command = set_rf_frequency(0x3930_0000);
        assert_eq!(command.opcode(), opcode::SET_RF_FREQUENCY);
        assert_eq!(command.as_bytes().len(), 5);
    }
}
