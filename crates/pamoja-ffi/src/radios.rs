//! The C ABI for LoRa radio chips.
//!
//! These functions wrap [`pamoja_radios`] for callers that drive a radio through the flat
//! C boundary: the bytes of each Semtech SX126x command, the chip's answers decoded, the
//! amplifier settings a regional power ceiling allows, and the silence a duty-cycle limit
//! forces after each transmission. A caller with its own SPI access sends each command in
//! one transaction, once BUSY is low, and reads a query's answer in the same transaction.
//!
//! A command is at most ten bytes, so it crosses by value as [`PamojaSx126xCommand`] and
//! costs no allocation. The duty-cycle guard keeps state between calls, so it is a handle.

use pamoja_radios::duty::DutyCycle;
use pamoja_radios::sx126x::command::{self, Command, Query};
use pamoja_radios::sx126x::config::{
    self, LoraModulation, LoraPacket, PaConfig, PacketType, PowerAmplifier, RampTime, StandbyMode,
    SyncWord, TxPower,
};
use pamoja_radios::sx126x::irq::Irq;
use pamoja_radios::sx126x::status::{
    rssi_inst_dbm, ChipMode, CommandStatus, DeviceErrors, PacketStatus, RxBufferStatus, Status,
};

use pamoja_lora::budget::Decibels;

use crate::lora::{link_budget, settings, PamojaLoraLink, PamojaLoraLinkBudget};
use crate::{set_last_error, PamojaStatus};

/// The most bytes one SX126x command takes, opcode included.
pub const PAMOJA_SX126X_COMMAND_MAX: usize = 10;

/// The receive timeout word that keeps an SX126x listening until another command stops it.
pub const PAMOJA_SX126X_RX_CONTINUOUS: u32 = 0xFF_FFFF;

/// The LoRa sync word of a public network such as LoRaWAN.
pub const PAMOJA_SX126X_SYNC_WORD_PUBLIC: u16 = 0x3444;

/// The LoRa sync word of a private network, and the chip's reset value.
pub const PAMOJA_SX126X_SYNC_WORD_PRIVATE: u16 = 0x1424;

/// The register that holds the most significant byte of the LoRa sync word.
pub const PAMOJA_SX126X_REGISTER_LORA_SYNC_WORD: u16 = 0x0740;

/// The IRQ bit raised when a packet has been sent.
pub const PAMOJA_SX126X_IRQ_TX_DONE: u16 = 1 << 0;
/// The IRQ bit raised when a packet has been received.
pub const PAMOJA_SX126X_IRQ_RX_DONE: u16 = 1 << 1;
/// The IRQ bit raised when a preamble has been detected.
pub const PAMOJA_SX126X_IRQ_PREAMBLE_DETECTED: u16 = 1 << 2;
/// The IRQ bit raised when a valid (G)FSK sync word has been detected.
pub const PAMOJA_SX126X_IRQ_SYNC_WORD_VALID: u16 = 1 << 3;
/// The IRQ bit raised when a valid LoRa header has been received.
pub const PAMOJA_SX126X_IRQ_HEADER_VALID: u16 = 1 << 4;
/// The IRQ bit raised when a LoRa header failed its CRC.
pub const PAMOJA_SX126X_IRQ_HEADER_ERROR: u16 = 1 << 5;
/// The IRQ bit raised when a packet failed its CRC.
pub const PAMOJA_SX126X_IRQ_CRC_ERROR: u16 = 1 << 6;
/// The IRQ bit raised when channel activity detection has finished.
pub const PAMOJA_SX126X_IRQ_CAD_DONE: u16 = 1 << 7;
/// The IRQ bit raised when channel activity detection heard LoRa.
pub const PAMOJA_SX126X_IRQ_CAD_DETECTED: u16 = 1 << 8;
/// The IRQ bit raised when a transmission or reception timed out.
pub const PAMOJA_SX126X_IRQ_TIMEOUT: u16 = 1 << 9;
/// The IRQ bit raised at each long-range FHSS hop.
pub const PAMOJA_SX126X_IRQ_LR_FHSS_HOP: u16 = 1 << 14;
/// Every IRQ bit the chip defines.
pub const PAMOJA_SX126X_IRQ_ALL: u16 = 0x43FF;

/// The device error bit for a failed RC64k calibration.
pub const PAMOJA_SX126X_ERROR_RC64K_CALIBRATION: u16 = 1 << 0;
/// The device error bit for a failed RC13M calibration.
pub const PAMOJA_SX126X_ERROR_RC13M_CALIBRATION: u16 = 1 << 1;
/// The device error bit for a failed PLL calibration.
pub const PAMOJA_SX126X_ERROR_PLL_CALIBRATION: u16 = 1 << 2;
/// The device error bit for a failed ADC calibration.
pub const PAMOJA_SX126X_ERROR_ADC_CALIBRATION: u16 = 1 << 3;
/// The device error bit for a failed image calibration.
pub const PAMOJA_SX126X_ERROR_IMAGE_CALIBRATION: u16 = 1 << 4;
/// The device error bit for a crystal oscillator that failed to start.
pub const PAMOJA_SX126X_ERROR_XOSC_START: u16 = 1 << 5;
/// The device error bit for a PLL that failed to lock.
pub const PAMOJA_SX126X_ERROR_PLL_LOCK: u16 = 1 << 6;
/// The device error bit for a power amplifier that failed to ramp.
pub const PAMOJA_SX126X_ERROR_PA_RAMP: u16 = 1 << 8;

// The header generator does not read the crates this one depends on, so these carry their
// value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_SX126X_COMMAND_MAX == command::MAX_LEN);
const _: () = assert!(PAMOJA_SX126X_RX_CONTINUOUS == config::RX_CONTINUOUS);
const _: () =
    assert!(PAMOJA_SX126X_SYNC_WORD_PUBLIC == u16::from_be_bytes(SyncWord::Public.to_bytes()));
const _: () =
    assert!(PAMOJA_SX126X_SYNC_WORD_PRIVATE == u16::from_be_bytes(SyncWord::Private.to_bytes()));
const _: () = assert!(PAMOJA_SX126X_REGISTER_LORA_SYNC_WORD == config::register::LORA_SYNC_WORD);
const _: () = assert!(PAMOJA_SX126X_IRQ_TX_DONE == Irq::TX_DONE.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_RX_DONE == Irq::RX_DONE.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_PREAMBLE_DETECTED == Irq::PREAMBLE_DETECTED.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_SYNC_WORD_VALID == Irq::SYNC_WORD_VALID.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_HEADER_VALID == Irq::HEADER_VALID.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_HEADER_ERROR == Irq::HEADER_ERROR.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_CRC_ERROR == Irq::CRC_ERROR.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_CAD_DONE == Irq::CAD_DONE.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_CAD_DETECTED == Irq::CAD_DETECTED.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_TIMEOUT == Irq::TIMEOUT.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_LR_FHSS_HOP == Irq::LR_FHSS_HOP.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_ALL == Irq::ALL.bits());
const _: () =
    assert!(PAMOJA_SX126X_ERROR_RC64K_CALIBRATION == DeviceErrors::RC64K_CALIBRATION.bits());
const _: () =
    assert!(PAMOJA_SX126X_ERROR_RC13M_CALIBRATION == DeviceErrors::RC13M_CALIBRATION.bits());
const _: () = assert!(PAMOJA_SX126X_ERROR_PLL_CALIBRATION == DeviceErrors::PLL_CALIBRATION.bits());
const _: () = assert!(PAMOJA_SX126X_ERROR_ADC_CALIBRATION == DeviceErrors::ADC_CALIBRATION.bits());
const _: () =
    assert!(PAMOJA_SX126X_ERROR_IMAGE_CALIBRATION == DeviceErrors::IMAGE_CALIBRATION.bits());
const _: () = assert!(PAMOJA_SX126X_ERROR_XOSC_START == DeviceErrors::XOSC_START.bits());
const _: () = assert!(PAMOJA_SX126X_ERROR_PLL_LOCK == DeviceErrors::PLL_LOCK.bits());
const _: () = assert!(PAMOJA_SX126X_ERROR_PA_RAMP == DeviceErrors::PA_RAMP.bits());

/// The bytes of one SX126x command, in the order the chip receives them.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xCommand {
    /// The opcode and its parameters; only the first `len` bytes are the command.
    pub bytes: [u8; PAMOJA_SX126X_COMMAND_MAX],
    /// How many bytes the command takes.
    pub len: u8,
}

/// A command the chip answers in the same SPI transaction.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xQuery {
    /// The bytes to send, ending with the NOP during which the status byte comes back.
    pub command: PamojaSx126xCommand,
    /// How many bytes of answer to read after them.
    pub answer_len: u32,
}

/// The amplifier configuration and power setting that produce an output power, from
/// Table 13-21 of the SX1261/2 datasheet.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xTxPower {
    /// paDutyCycle, the conduction angle of the amplifier.
    pub pa_duty_cycle: u8,
    /// hpMax, the size of the SX1262 amplifier; no effect on the SX1261.
    pub hp_max: u8,
    /// deviceSel: `0` for the SX1262 and the LLCC68, `1` for the SX1261.
    pub device_sel: u8,
    /// paLut, reserved and always `1`.
    pub pa_lut: u8,
    /// The power byte of SetTxParams, in dBm.
    pub setting_dbm: i8,
}

/// The mode an SX126x reports in its status byte.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaSx126xChipMode {
    /// A value the datasheet leaves unused.
    Other = 0,
    /// Standby on the 13 MHz RC oscillator.
    StandbyRc = 2,
    /// Standby on the 32 MHz crystal.
    StandbyXosc = 3,
    /// Frequency synthesis.
    Fs = 4,
    /// Receiving.
    Rx = 5,
    /// Transmitting.
    Tx = 6,
}

/// How the last command went, as an SX126x reports it in its status byte.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaSx126xCommandStatus {
    /// A value the datasheet leaves unused, which includes a command that went well.
    Other = 0,
    /// A packet has been received and waits in the data buffer.
    DataAvailable = 2,
    /// A command timed out.
    Timeout = 3,
    /// A command could not be processed.
    ProcessingError = 4,
    /// A command failed to execute.
    ExecutionFailure = 5,
    /// A transmission has finished.
    TxDone = 6,
}

/// A decoded SX126x status byte.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xStatus {
    /// The mode the chip is in.
    pub chip_mode: PamojaSx126xChipMode,
    /// How the last command went.
    pub command_status: PamojaSx126xCommandStatus,
    /// `true` for a timeout, a processing error, or an execution failure.
    pub error: bool,
}

/// The signal levels of the last LoRa packet received, in hundredths of a decibel.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xPacketStatus {
    /// The RSSI averaged over the packet, in hundredths of a dBm.
    pub rssi_centi_dbm: i32,
    /// The estimated signal-to-noise ratio, in hundredths of a dB.
    pub snr_centi_db: i32,
    /// The estimated RSSI of the LoRa signal after despreading, in hundredths of a dBm.
    pub signal_rssi_centi_dbm: i32,
}

/// Where a received payload sits in the data buffer.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xRxBufferStatus {
    /// The length of the payload in bytes.
    pub payload_len: u8,
    /// The buffer offset of its first byte.
    pub start: u8,
}

/// An opaque handle to a duty-cycle guard.
///
/// Record each transmission with [`pamoja_radio_duty_cycle_transmitted`], ask
/// [`pamoja_radio_duty_cycle_ready`] before the next, and release it with
/// [`pamoja_radio_duty_cycle_free`].
pub struct PamojaRadioDutyCycle {
    guard: DutyCycle,
}

/// Returns the word SetRfFrequency takes for a frequency.
///
/// # Arguments
///
/// * `frequency_hz` - the carrier frequency in hertz.
///
/// # Returns
///
/// The frequency times 2^25 over the 32 MHz crystal, rounded to the nearest step.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_frequency_word(frequency_hz: u32) -> u32 {
    config::frequency_word(frequency_hz)
}

/// Returns the 24-bit timeout word SetTx and SetRx take for a duration.
///
/// # Arguments
///
/// * `timeout_us` - the duration in microseconds.
///
/// # Returns
///
/// The number of 15.625 us steps; a nonzero duration never becomes the zero word that
/// disables the timeout, and nothing reaches [`PAMOJA_SX126X_RX_CONTINUOUS`].
#[no_mangle]
pub extern "C" fn pamoja_sx126x_timeout_steps(timeout_us: u64) -> u32 {
    config::timeout_steps(timeout_us)
}

/// Returns the two CalibrateImage codes that cover a band.
///
/// # Arguments
///
/// * `low_hz` - the lower edge of the band in hertz.
/// * `high_hz` - the upper edge of the band in hertz.
///
/// # Returns
///
/// `freq1` in the high byte and `freq2` in the low byte.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_image_calibration(low_hz: u32, high_hz: u32) -> u16 {
    u16::from_be_bytes(config::image_calibration(low_hz, high_hz))
}

/// Returns the shortest amplifier ramp time the chip offers that lasts at least a duration.
///
/// # Arguments
///
/// * `at_least_us` - the least ramp time wanted, in microseconds.
///
/// # Returns
///
/// The ramp time in microseconds, one of the eight Table 13-41 gives.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_ramp_time_us(at_least_us: u32) -> u32 {
    RampTime::at_least(at_least_us).micros()
}

/// Chooses the amplifier settings for an output power.
///
/// # Arguments
///
/// * `high_power` - `true` for the high power amplifier of the SX1262 and the LLCC68,
///   `false` for the low power amplifier of the SX1261.
/// * `output_dbm` - the output power wanted at the antenna port.
///
/// # Returns
///
/// The configuration and the power setting, clamped to what the amplifier allows.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_tx_power_for_output(
    high_power: bool,
    output_dbm: i8,
) -> PamojaSx126xTxPower {
    power_of(TxPower::for_output(amplifier(high_power), output_dbm))
}

/// Chooses the amplifier settings that keep a link's EIRP at or under a ceiling.
///
/// # Arguments
///
/// * `high_power` - `true` for the high power amplifier, `false` for the low power one.
/// * `budget` - the link budget, whose transmitting antenna and cable apply.
/// * `eirp_ceiling_centi_dbm` - the EIRP limit, in hundredths of a dBm.
///
/// # Returns
///
/// The configuration and the power setting, rounded down to whole decibels.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_tx_power_under_ceiling(
    high_power: bool,
    budget: PamojaLoraLinkBudget,
    eirp_ceiling_centi_dbm: i32,
) -> PamojaSx126xTxPower {
    power_of(TxPower::under_ceiling(
        amplifier(high_power),
        &link_budget(budget),
        Decibels::from_hundredths(eirp_ceiling_centi_dbm),
    ))
}

/// SetStandby into STDBY_RC.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_standby() -> PamojaSx126xCommand {
    command_of(command::set_standby(StandbyMode::Rc))
}

/// SetPacketType for LoRa.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_packet_type_lora() -> PamojaSx126xCommand {
    command_of(command::set_packet_type(PacketType::Lora))
}

/// SetRfFrequency for a carrier frequency.
///
/// # Arguments
///
/// * `frequency_hz` - the carrier frequency in hertz.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_rf_frequency(frequency_hz: u32) -> PamojaSx126xCommand {
    command_of(command::set_rf_frequency(config::frequency_word(
        frequency_hz,
    )))
}

/// CalibrateImage over a band.
///
/// # Arguments
///
/// * `low_hz` - the lower edge of the band in hertz.
/// * `high_hz` - the upper edge of the band in hertz.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_calibrate_image(low_hz: u32, high_hz: u32) -> PamojaSx126xCommand {
    command_of(command::calibrate_image(config::image_calibration(
        low_hz, high_hz,
    )))
}

/// SetPaConfig for a power setting.
///
/// # Arguments
///
/// * `power` - the settings from [`pamoja_sx126x_tx_power_for_output`] or
///   [`pamoja_sx126x_tx_power_under_ceiling`].
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_pa_config(power: PamojaSx126xTxPower) -> PamojaSx126xCommand {
    command_of(command::set_pa_config(tx_power(power).pa))
}

/// SetTxParams for a power setting and a ramp time.
///
/// # Arguments
///
/// * `power` - the power settings.
/// * `ramp_us` - the least amplifier ramp time wanted, in microseconds; the chip takes the
///   shortest of its eight ramp times that lasts at least this long.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_tx_params(
    power: PamojaSx126xTxPower,
    ramp_us: u32,
) -> PamojaSx126xCommand {
    command_of(command::set_tx_params(
        power.setting_dbm,
        RampTime::at_least(ramp_us),
    ))
}

/// SetModulationParams for a LoRa link.
///
/// # Arguments
///
/// * `link` - the link settings.
/// * `out_command` - receives the command.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with `*out_command` set, or [`PamojaStatus::InvalidArgument`] if
/// the link's bandwidth is not one the SX126x offers or `out_command` is null.
///
/// # Safety
///
/// `out_command` must point to a writable [`PamojaSx126xCommand`], or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sx126x_set_lora_modulation_params(
    link: PamojaLoraLink,
    out_command: *mut PamojaSx126xCommand,
) -> PamojaStatus {
    if out_command.is_null() {
        set_last_error("out_command must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match LoraModulation::from_link(&settings(link)) {
        Some(modulation) => {
            *out_command = command_of(command::set_lora_modulation_params(modulation));
            PamojaStatus::Ok
        }
        None => {
            set_last_error(format!(
                "the SX126x has no {} Hz LoRa bandwidth",
                link.bandwidth_hz
            ));
            PamojaStatus::InvalidArgument
        }
    }
}

/// SetPacketParams for a LoRa link and a payload.
///
/// # Arguments
///
/// * `link` - the link settings, whose preamble, header, and CRC the frame uses.
/// * `payload_len` - the payload length to send, or the most a receiver accepts.
/// * `invert_iq` - `true` for inverted IQ, as a LoRaWAN gateway sends downlinks.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_lora_packet_params(
    link: PamojaLoraLink,
    payload_len: u8,
    invert_iq: bool,
) -> PamojaSx126xCommand {
    command_of(command::set_lora_packet_params(LoraPacket::from_link(
        &settings(link),
        payload_len,
        invert_iq,
    )))
}

/// SetDioIrqParams: which interrupts are enabled and which DIO line each raises.
///
/// # Arguments
///
/// * `irq` - the interrupts to enable, as `PAMOJA_SX126X_IRQ_*` bits.
/// * `dio1` - the interrupts routed to DIO1.
/// * `dio2` - the interrupts routed to DIO2.
/// * `dio3` - the interrupts routed to DIO3.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_dio_irq_params(
    irq: u16,
    dio1: u16,
    dio2: u16,
    dio3: u16,
) -> PamojaSx126xCommand {
    command_of(command::set_dio_irq_params(
        Irq::from_bits(irq),
        Irq::from_bits(dio1),
        Irq::from_bits(dio2),
        Irq::from_bits(dio3),
    ))
}

/// ClearIrqStatus for a set of interrupts.
///
/// # Arguments
///
/// * `irq` - the interrupts to clear, as `PAMOJA_SX126X_IRQ_*` bits.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_clear_irq_status(irq: u16) -> PamojaSx126xCommand {
    command_of(command::clear_irq_status(Irq::from_bits(irq)))
}

/// SetTx with a timeout.
///
/// # Arguments
///
/// * `timeout_us` - how long the chip may transmit before it raises TIMEOUT, in
///   microseconds; `0` disables the timeout.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_tx(timeout_us: u64) -> PamojaSx126xCommand {
    command_of(command::set_tx(config::timeout_steps(timeout_us)))
}

/// SetRx with a timeout.
///
/// # Arguments
///
/// * `timeout_us` - how long the chip listens for a packet to start, in microseconds;
///   `0` listens for a single packet with no timeout.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_rx(timeout_us: u64) -> PamojaSx126xCommand {
    command_of(command::set_rx(config::timeout_steps(timeout_us)))
}

/// SetRx in continuous mode, receiving packet after packet until another command.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_rx_continuous() -> PamojaSx126xCommand {
    command_of(command::set_rx(config::RX_CONTINUOUS))
}

/// SetSleep, without an RTC wake-up.
///
/// # Arguments
///
/// * `warm_start` - `true` to keep the configuration in retention.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_sleep(warm_start: bool) -> PamojaSx126xCommand {
    command_of(command::set_sleep(warm_start, false))
}

/// The start of a WriteRegister transaction; the register values follow it in the same
/// transaction.
///
/// # Arguments
///
/// * `address` - the first register's address.
///
/// # Returns
///
/// The opcode and the address.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_write_register_header(address: u16) -> PamojaSx126xCommand {
    command_of(command::write_register(address))
}

/// The start of a WriteBuffer transaction; the payload follows it in the same transaction.
///
/// # Arguments
///
/// * `offset` - where in the data buffer the first byte goes.
///
/// # Returns
///
/// The opcode and the offset.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_write_buffer_header(offset: u8) -> PamojaSx126xCommand {
    command_of(command::write_buffer(offset))
}

/// GetStatus, answered by the status byte.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_status() -> PamojaSx126xQuery {
    query_of(command::get_status())
}

/// GetIrqStatus, answered by the two IRQ bytes.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_irq_status() -> PamojaSx126xQuery {
    query_of(command::get_irq_status())
}

/// GetRxBufferStatus, answered by the payload length and its offset.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_rx_buffer_status() -> PamojaSx126xQuery {
    query_of(command::get_rx_buffer_status())
}

/// GetPacketStatus, answered by the three LoRa signal level bytes.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_packet_status() -> PamojaSx126xQuery {
    query_of(command::get_packet_status())
}

/// GetRssiInst, answered by the instantaneous RSSI byte.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_rssi_inst() -> PamojaSx126xQuery {
    query_of(command::get_rssi_inst())
}

/// GetDeviceErrors, answered by the two device error bytes.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_device_errors() -> PamojaSx126xQuery {
    query_of(command::get_device_errors())
}

/// ReadRegister for consecutive registers.
///
/// # Arguments
///
/// * `address` - the first register's address.
/// * `len` - how many registers to read.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_read_register(address: u16, len: u8) -> PamojaSx126xQuery {
    query_of(command::read_register(address, usize::from(len)))
}

/// ReadBuffer for a run of the data buffer.
///
/// # Arguments
///
/// * `offset` - where in the buffer the first byte is.
/// * `len` - how many bytes to read.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_read_buffer(offset: u8, len: u8) -> PamojaSx126xQuery {
    query_of(command::read_buffer(offset, usize::from(len)))
}

/// Decodes a status byte.
///
/// # Arguments
///
/// * `byte` - the status byte.
///
/// # Returns
///
/// The chip mode, how the last command went, and whether that was an error.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_status_from_byte(byte: u8) -> PamojaSx126xStatus {
    let status = Status::from_byte(byte);
    PamojaSx126xStatus {
        chip_mode: chip_mode(status.chip_mode),
        command_status: command_status(status.command_status),
        error: status.is_error(),
    }
}

/// Decodes a GetIrqStatus answer.
///
/// # Arguments
///
/// * `high` - the first answer byte, IrqStatus(15:8).
/// * `low` - the second answer byte, IrqStatus(7:0).
///
/// # Returns
///
/// The pending interrupts, as `PAMOJA_SX126X_IRQ_*` bits.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_irq_from_bytes(high: u8, low: u8) -> u16 {
    Irq::from_bytes([high, low]).bits()
}

/// Decodes a GetDeviceErrors answer.
///
/// # Arguments
///
/// * `high` - the first answer byte.
/// * `low` - the second answer byte.
///
/// # Returns
///
/// The flagged errors, as `PAMOJA_SX126X_ERROR_*` bits.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_device_errors_from_bytes(high: u8, low: u8) -> u16 {
    DeviceErrors::from_bytes([high, low]).bits()
}

/// Decodes a LoRa GetPacketStatus answer.
///
/// # Arguments
///
/// * `rssi_pkt` - RssiPkt.
/// * `snr_pkt` - SnrPkt.
/// * `signal_rssi_pkt` - SignalRssiPkt.
///
/// # Returns
///
/// The three signal levels, exact to a hundredth of a decibel.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_packet_status_from_bytes(
    rssi_pkt: u8,
    snr_pkt: u8,
    signal_rssi_pkt: u8,
) -> PamojaSx126xPacketStatus {
    let status = PacketStatus::from_bytes([rssi_pkt, snr_pkt, signal_rssi_pkt]);
    PamojaSx126xPacketStatus {
        rssi_centi_dbm: status.rssi_dbm.hundredths(),
        snr_centi_db: status.snr_db.hundredths(),
        signal_rssi_centi_dbm: status.signal_rssi_dbm.hundredths(),
    }
}

/// Decodes a GetRxBufferStatus answer.
///
/// # Arguments
///
/// * `payload_len` - PayloadLengthRx.
/// * `start` - RxStartBufferPointer.
///
/// # Returns
///
/// The payload length and where it starts in the data buffer.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_rx_buffer_status_from_bytes(
    payload_len: u8,
    start: u8,
) -> PamojaSx126xRxBufferStatus {
    let status = RxBufferStatus::from_bytes([payload_len, start]);
    PamojaSx126xRxBufferStatus {
        payload_len: status.payload_len,
        start: status.start,
    }
}

/// Decodes a GetRssiInst answer.
///
/// # Arguments
///
/// * `byte` - RssiInst.
///
/// # Returns
///
/// The instantaneous RSSI in hundredths of a dBm.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_rssi_inst_centi_dbm(byte: u8) -> i32 {
    rssi_inst_dbm(byte).hundredths()
}

/// Creates a duty-cycle guard, ready to transmit at once.
///
/// # Arguments
///
/// * `permille` - the limit in parts per thousand, so `10` is 1%; `0` forbids
///   transmitting and `1000` or more imposes no silence.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_radio_duty_cycle_free`].
#[no_mangle]
pub extern "C" fn pamoja_radio_duty_cycle_new(permille: u32) -> *mut PamojaRadioDutyCycle {
    Box::into_raw(Box::new(PamojaRadioDutyCycle {
        guard: DutyCycle::new(permille),
    }))
}

/// Records a transmission and the silence it owes.
///
/// # Arguments
///
/// * `guard` - the duty-cycle guard.
/// * `started_us` - when the transmission started, in microseconds on the caller's clock.
/// * `link` - the settings the frame was sent with.
/// * `payload_len` - the payload length in bytes.
///
/// # Returns
///
/// The frame's time on air in microseconds, or 0 if `guard` is null.
///
/// # Safety
///
/// `guard` must be a live handle from [`pamoja_radio_duty_cycle_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_radio_duty_cycle_transmitted(
    guard: *mut PamojaRadioDutyCycle,
    started_us: u64,
    link: PamojaLoraLink,
    payload_len: usize,
) -> u64 {
    if guard.is_null() {
        return 0;
    }
    (*guard)
        .guard
        .transmitted(started_us, &settings(link), payload_len)
}

/// Returns how long the radio must still stay silent.
///
/// # Arguments
///
/// * `guard` - the duty-cycle guard.
/// * `now_us` - the current time in microseconds on the caller's clock.
///
/// # Returns
///
/// The remaining silence in microseconds, zero when a transmission may start, or
/// `UINT64_MAX` when the limit forbids transmitting or `guard` is null.
///
/// # Safety
///
/// `guard` must be a live handle from [`pamoja_radio_duty_cycle_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_radio_duty_cycle_wait_us(
    guard: *const PamojaRadioDutyCycle,
    now_us: u64,
) -> u64 {
    if guard.is_null() {
        return u64::MAX;
    }
    (*guard).guard.wait_us(now_us)
}

/// Reports whether a transmission may start now.
///
/// # Arguments
///
/// * `guard` - the duty-cycle guard.
/// * `now_us` - the current time in microseconds on the caller's clock.
///
/// # Returns
///
/// `true` once the silence the last transmission owed has passed, or `false` if `guard`
/// is null.
///
/// # Safety
///
/// `guard` must be a live handle from [`pamoja_radio_duty_cycle_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_radio_duty_cycle_ready(
    guard: *const PamojaRadioDutyCycle,
    now_us: u64,
) -> bool {
    !guard.is_null() && (*guard).guard.ready(now_us)
}

/// Returns the earliest time the next transmission may start.
///
/// # Arguments
///
/// * `guard` - the duty-cycle guard.
///
/// # Returns
///
/// A time in microseconds on the caller's clock, or `UINT64_MAX` when the limit forbids
/// transmitting or `guard` is null.
///
/// # Safety
///
/// `guard` must be a live handle from [`pamoja_radio_duty_cycle_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_radio_duty_cycle_earliest_us(
    guard: *const PamojaRadioDutyCycle,
) -> u64 {
    if guard.is_null() {
        return u64::MAX;
    }
    (*guard).guard.earliest_us()
}

/// Releases a duty-cycle guard handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `guard` must be a handle from [`pamoja_radio_duty_cycle_new`] that has not already been
/// freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_radio_duty_cycle_free(guard: *mut PamojaRadioDutyCycle) {
    if !guard.is_null() {
        drop(Box::from_raw(guard));
    }
}

/// Copies a command's bytes into the value that crosses the boundary.
fn command_of(command: Command) -> PamojaSx126xCommand {
    let bytes = command.as_bytes();
    let mut out = [0u8; PAMOJA_SX126X_COMMAND_MAX];
    out[..bytes.len()].copy_from_slice(bytes);
    PamojaSx126xCommand {
        bytes: out,
        len: bytes.len() as u8,
    }
}

/// Copies a query into the value that crosses the boundary.
fn query_of(query: Query) -> PamojaSx126xQuery {
    PamojaSx126xQuery {
        command: command_of(query.command),
        answer_len: query.answer_len as u32,
    }
}

/// Names the amplifier a flag selects.
fn amplifier(high_power: bool) -> PowerAmplifier {
    if high_power {
        PowerAmplifier::HighPower
    } else {
        PowerAmplifier::LowPower
    }
}

/// Flattens power settings for the boundary.
fn power_of(power: TxPower) -> PamojaSx126xTxPower {
    PamojaSx126xTxPower {
        pa_duty_cycle: power.pa.duty_cycle,
        hp_max: power.pa.hp_max,
        device_sel: power.pa.device,
        pa_lut: power.pa.lut,
        setting_dbm: power.setting_dbm,
    }
}

/// Rebuilds power settings from the fields that crossed the boundary.
fn tx_power(power: PamojaSx126xTxPower) -> TxPower {
    TxPower {
        pa: PaConfig {
            duty_cycle: power.pa_duty_cycle,
            hp_max: power.hp_max,
            device: power.device_sel,
            lut: power.pa_lut,
        },
        setting_dbm: power.setting_dbm,
    }
}

/// Maps a decoded chip mode to the value that crosses the boundary.
fn chip_mode(mode: ChipMode) -> PamojaSx126xChipMode {
    if mode == ChipMode::StandbyRc {
        PamojaSx126xChipMode::StandbyRc
    } else if mode == ChipMode::StandbyXosc {
        PamojaSx126xChipMode::StandbyXosc
    } else if mode == ChipMode::Fs {
        PamojaSx126xChipMode::Fs
    } else if mode == ChipMode::Rx {
        PamojaSx126xChipMode::Rx
    } else if mode == ChipMode::Tx {
        PamojaSx126xChipMode::Tx
    } else {
        PamojaSx126xChipMode::Other
    }
}

/// Maps a decoded command status to the value that crosses the boundary.
fn command_status(status: CommandStatus) -> PamojaSx126xCommandStatus {
    if status == CommandStatus::DataAvailable {
        PamojaSx126xCommandStatus::DataAvailable
    } else if status == CommandStatus::Timeout {
        PamojaSx126xCommandStatus::Timeout
    } else if status == CommandStatus::ProcessingError {
        PamojaSx126xCommandStatus::ProcessingError
    } else if status == CommandStatus::ExecutionFailure {
        PamojaSx126xCommandStatus::ExecutionFailure
    } else if status == CommandStatus::TxDone {
        PamojaSx126xCommandStatus::TxDone
    } else {
        PamojaSx126xCommandStatus::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lora::{pamoja_lora_link_budget_default, pamoja_lora_link_default};

    fn bytes(command: PamojaSx126xCommand) -> Vec<u8> {
        command.bytes[..usize::from(command.len)].to_vec()
    }

    #[test]
    fn commands_carry_the_datasheet_bytes() {
        assert_eq!(
            bytes(pamoja_sx126x_set_rf_frequency(868_100_000)),
            [0x86, 0x36, 0x41, 0x99, 0x9A]
        );
        let power = pamoja_sx126x_tx_power_for_output(true, 14);
        assert_eq!(
            bytes(pamoja_sx126x_set_pa_config(power)),
            [0x95, 0x04, 0x07, 0x00, 0x01]
        );
        assert_eq!(
            bytes(pamoja_sx126x_set_tx_params(power, 40)),
            [0x8E, 0x0E, 0x02]
        );
        assert_eq!(
            bytes(pamoja_sx126x_set_rx_continuous()),
            [0x82, 0xFF, 0xFF, 0xFF]
        );
        let query = pamoja_sx126x_get_irq_status();
        assert_eq!(bytes(query.command), [0x12, 0x00]);
        assert_eq!(query.answer_len, 2);
    }

    #[test]
    fn a_bandwidth_the_chip_lacks_is_refused() {
        let mut command = PamojaSx126xCommand {
            bytes: [0; PAMOJA_SX126X_COMMAND_MAX],
            len: 0,
        };
        let link = pamoja_lora_link_default(7, 125_000);
        let status = unsafe { pamoja_sx126x_set_lora_modulation_params(link, &mut command) };
        assert_eq!(status, PamojaStatus::Ok);
        assert_eq!(bytes(command), [0x8B, 0x07, 0x04, 0x01, 0x00]);

        let narrow = pamoja_lora_link_default(7, 203_125);
        let status = unsafe { pamoja_sx126x_set_lora_modulation_params(narrow, &mut command) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
    }

    #[test]
    fn power_under_a_ceiling_takes_off_the_antenna() {
        let whip = PamojaLoraLinkBudget {
            transmit_antenna_gain_centi_dbi: 215,
            transmit_cable_loss_centi_db: 50,
            ..pamoja_lora_link_budget_default()
        };
        let power = pamoja_sx126x_tx_power_under_ceiling(true, whip, 1_600);
        assert_eq!(power.setting_dbm, 14);
        assert_eq!(
            pamoja_sx126x_tx_power_for_output(false, 15).pa_duty_cycle,
            0x06
        );
        assert_eq!(pamoja_sx126x_ramp_time_us(100), 200);
    }

    #[test]
    fn answers_decode_as_the_datasheet_gives() {
        let status = pamoja_sx126x_status_from_byte(0x2C);
        assert_eq!(status.chip_mode, PamojaSx126xChipMode::StandbyRc);
        assert_eq!(status.command_status, PamojaSx126xCommandStatus::TxDone);
        assert!(!status.error);
        assert_eq!(
            pamoja_sx126x_irq_from_bytes(0x02, 0x62),
            PAMOJA_SX126X_IRQ_TIMEOUT
                | PAMOJA_SX126X_IRQ_CRC_ERROR
                | PAMOJA_SX126X_IRQ_HEADER_ERROR
                | PAMOJA_SX126X_IRQ_RX_DONE
        );
        let packet = pamoja_sx126x_packet_status_from_bytes(0xDB, 0xF6, 0xE0);
        assert_eq!(packet.rssi_centi_dbm, -10_950);
        assert_eq!(packet.snr_centi_db, -250);
        assert_eq!(pamoja_sx126x_rssi_inst_centi_dbm(0xDB), -10_950);
    }

    #[test]
    fn a_duty_cycle_guard_holds_the_radio_silent() {
        let guard = pamoja_radio_duty_cycle_new(10);
        let link = pamoja_lora_link_default(12, 125_000);
        unsafe {
            assert_eq!(
                pamoja_radio_duty_cycle_transmitted(guard, 0, link, 10),
                991_232
            );
            assert_eq!(pamoja_radio_duty_cycle_wait_us(guard, 0), 99_123_200);
            assert!(!pamoja_radio_duty_cycle_ready(guard, 99_000_000));
            assert!(pamoja_radio_duty_cycle_ready(guard, 99_123_200));
            pamoja_radio_duty_cycle_free(guard);
            assert!(!pamoja_radio_duty_cycle_ready(std::ptr::null(), 0));
        }
    }
}
