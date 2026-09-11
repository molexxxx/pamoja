//! Generated Node bindings for LoRa radio chips.
//!
//! These mirror the `pamoja-radios` Rust API: the bytes of each Semtech SX126x command,
//! the chip's answers decoded, the amplifier settings a regional power ceiling allows, and
//! the silence a duty-cycle limit forces after each transmission.
//!
//! A command is a few bytes, so it crosses as a `Buffer`, and the settings and the decoded
//! answers are plain objects. The duty-cycle guard keeps state between calls, so it is a
//! class. Microsecond clocks cross as numbers, exact far past any deployment's uptime.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
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

use crate::lora::{db, decibels, link_budget, settings, LoraLink, LoraLinkBudget};

/// The receive timeout word that keeps an SX126x listening until another command stops it.
#[napi]
pub const SX126X_RX_CONTINUOUS: u32 = config::RX_CONTINUOUS;

/// The LoRa sync word of a public network such as LoRaWAN.
#[napi]
pub const SX126X_SYNC_WORD_PUBLIC: u32 = u16::from_be_bytes(SyncWord::Public.to_bytes()) as u32;

/// The LoRa sync word of a private network, and the chip's reset value.
#[napi]
pub const SX126X_SYNC_WORD_PRIVATE: u32 = u16::from_be_bytes(SyncWord::Private.to_bytes()) as u32;

/// The register that holds the most significant byte of the LoRa sync word.
#[napi]
pub const SX126X_REGISTER_LORA_SYNC_WORD: u32 = config::register::LORA_SYNC_WORD as u32;

/// The IRQ bit raised when a packet has been sent.
#[napi]
pub const SX126X_IRQ_TX_DONE: u32 = Irq::TX_DONE.bits() as u32;
/// The IRQ bit raised when a packet has been received.
#[napi]
pub const SX126X_IRQ_RX_DONE: u32 = Irq::RX_DONE.bits() as u32;
/// The IRQ bit raised when a preamble has been detected.
#[napi]
pub const SX126X_IRQ_PREAMBLE_DETECTED: u32 = Irq::PREAMBLE_DETECTED.bits() as u32;
/// The IRQ bit raised when a valid (G)FSK sync word has been detected.
#[napi]
pub const SX126X_IRQ_SYNC_WORD_VALID: u32 = Irq::SYNC_WORD_VALID.bits() as u32;
/// The IRQ bit raised when a valid LoRa header has been received.
#[napi]
pub const SX126X_IRQ_HEADER_VALID: u32 = Irq::HEADER_VALID.bits() as u32;
/// The IRQ bit raised when a LoRa header failed its CRC.
#[napi]
pub const SX126X_IRQ_HEADER_ERROR: u32 = Irq::HEADER_ERROR.bits() as u32;
/// The IRQ bit raised when a packet failed its CRC.
#[napi]
pub const SX126X_IRQ_CRC_ERROR: u32 = Irq::CRC_ERROR.bits() as u32;
/// The IRQ bit raised when channel activity detection has finished.
#[napi]
pub const SX126X_IRQ_CAD_DONE: u32 = Irq::CAD_DONE.bits() as u32;
/// The IRQ bit raised when channel activity detection heard LoRa.
#[napi]
pub const SX126X_IRQ_CAD_DETECTED: u32 = Irq::CAD_DETECTED.bits() as u32;
/// The IRQ bit raised when a transmission or reception timed out.
#[napi]
pub const SX126X_IRQ_TIMEOUT: u32 = Irq::TIMEOUT.bits() as u32;
/// The IRQ bit raised at each long-range FHSS hop.
#[napi]
pub const SX126X_IRQ_LR_FHSS_HOP: u32 = Irq::LR_FHSS_HOP.bits() as u32;
/// Every IRQ bit the chip defines.
#[napi]
pub const SX126X_IRQ_ALL: u32 = Irq::ALL.bits() as u32;

/// The device error bit for a failed RC64k calibration.
#[napi]
pub const SX126X_ERROR_RC64K_CALIBRATION: u32 = DeviceErrors::RC64K_CALIBRATION.bits() as u32;
/// The device error bit for a failed RC13M calibration.
#[napi]
pub const SX126X_ERROR_RC13M_CALIBRATION: u32 = DeviceErrors::RC13M_CALIBRATION.bits() as u32;
/// The device error bit for a failed PLL calibration.
#[napi]
pub const SX126X_ERROR_PLL_CALIBRATION: u32 = DeviceErrors::PLL_CALIBRATION.bits() as u32;
/// The device error bit for a failed ADC calibration.
#[napi]
pub const SX126X_ERROR_ADC_CALIBRATION: u32 = DeviceErrors::ADC_CALIBRATION.bits() as u32;
/// The device error bit for a failed image calibration.
#[napi]
pub const SX126X_ERROR_IMAGE_CALIBRATION: u32 = DeviceErrors::IMAGE_CALIBRATION.bits() as u32;
/// The device error bit for a crystal oscillator that failed to start.
#[napi]
pub const SX126X_ERROR_XOSC_START: u32 = DeviceErrors::XOSC_START.bits() as u32;
/// The device error bit for a PLL that failed to lock.
#[napi]
pub const SX126X_ERROR_PLL_LOCK: u32 = DeviceErrors::PLL_LOCK.bits() as u32;
/// The device error bit for a power amplifier that failed to ramp.
#[napi]
pub const SX126X_ERROR_PA_RAMP: u32 = DeviceErrors::PA_RAMP.bits() as u32;

/// Which power amplifier a chip has.
#[napi(string_enum, js_name = "Sx126xAmplifier")]
pub enum Sx126xAmplifier {
    /// The low power amplifier of the SX1261, up to +15 dBm.
    LowPower,
    /// The high power amplifier of the SX1262 and the LLCC68, up to +22 dBm.
    HighPower,
}

/// The mode an SX126x reports in its status byte.
#[napi(string_enum, js_name = "Sx126xChipMode")]
pub enum Sx126xChipMode {
    /// Standby on the 13 MHz RC oscillator.
    StandbyRc,
    /// Standby on the 32 MHz crystal.
    StandbyXosc,
    /// Frequency synthesis.
    Fs,
    /// Receiving.
    Rx,
    /// Transmitting.
    Tx,
    /// A value the datasheet leaves unused.
    Other,
}

/// How the last command went, as an SX126x reports it in its status byte.
#[napi(string_enum, js_name = "Sx126xCommandStatus")]
pub enum Sx126xCommandStatus {
    /// A packet has been received and waits in the data buffer.
    DataAvailable,
    /// A command timed out.
    Timeout,
    /// A command could not be processed.
    ProcessingError,
    /// A command failed to execute.
    ExecutionFailure,
    /// A transmission has finished.
    TxDone,
    /// A value the datasheet leaves unused, which includes a command that went well.
    Other,
}

/// The amplifier configuration and power setting that produce an output power.
#[napi(object, js_name = "Sx126xTxPower")]
pub struct Sx126xTxPower {
    /// paDutyCycle, the conduction angle of the amplifier.
    pub pa_duty_cycle: u8,
    /// hpMax, the size of the SX1262 amplifier; no effect on the SX1261.
    pub hp_max: u8,
    /// deviceSel: 0 for the SX1262 and the LLCC68, 1 for the SX1261.
    pub device_sel: u8,
    /// paLut, reserved and always 1.
    pub pa_lut: u8,
    /// The power byte of SetTxParams, in dBm.
    pub setting_dbm: i32,
}

/// A command the chip answers in the same SPI transaction.
#[napi(object, js_name = "Sx126xQuery")]
pub struct Sx126xQuery {
    /// The bytes to send, ending with the NOP during which the status byte comes back.
    pub bytes: Buffer,
    /// How many bytes of answer to read after them.
    pub answer_length: u32,
}

/// A decoded SX126x status byte.
#[napi(object, js_name = "Sx126xStatus")]
pub struct Sx126xStatus {
    /// The mode the chip is in.
    pub chip_mode: Sx126xChipMode,
    /// How the last command went.
    pub command_status: Sx126xCommandStatus,
    /// Whether the last command timed out, could not be processed, or failed.
    pub error: bool,
}

/// The signal levels of the last LoRa packet received.
#[napi(object, js_name = "Sx126xPacketStatus")]
pub struct Sx126xPacketStatus {
    /// The RSSI averaged over the packet, in dBm.
    pub rssi_dbm: f64,
    /// The estimated signal-to-noise ratio, in dB.
    pub snr_db: f64,
    /// The estimated RSSI of the LoRa signal after despreading, in dBm.
    pub signal_rssi_dbm: f64,
}

/// Where a received payload sits in the data buffer.
#[napi(object, js_name = "Sx126xRxBufferStatus")]
pub struct Sx126xRxBufferStatus {
    /// The length of the payload in bytes.
    pub payload_length: u8,
    /// The buffer offset of its first byte.
    pub start: u8,
}

/// Returns the word SetRfFrequency takes for a frequency.
#[napi(js_name = "sx126xFrequencyWord")]
pub fn sx126x_frequency_word(frequency_hz: u32) -> u32 {
    config::frequency_word(frequency_hz)
}

/// Returns the 24-bit timeout word SetTx and SetRx take for a duration in microseconds.
#[napi(js_name = "sx126xTimeoutSteps")]
pub fn sx126x_timeout_steps(timeout_us: f64) -> u32 {
    config::timeout_steps(micros(timeout_us))
}

/// Returns the two CalibrateImage codes that cover a band.
#[napi(js_name = "sx126xImageCalibration")]
pub fn sx126x_image_calibration(low_hz: u32, high_hz: u32) -> Buffer {
    Buffer::from(config::image_calibration(low_hz, high_hz).to_vec())
}

/// Returns the shortest amplifier ramp time the chip offers that lasts at least a
/// duration, in microseconds.
#[napi(js_name = "sx126xRampTimeUs")]
pub fn sx126x_ramp_time_us(at_least_us: u32) -> u32 {
    RampTime::at_least(at_least_us).micros()
}

/// Chooses the amplifier settings for an output power, clamped to what the amplifier
/// allows.
#[napi(js_name = "sx126xTxPower")]
pub fn sx126x_tx_power(amplifier: Sx126xAmplifier, output_dbm: i32) -> Sx126xTxPower {
    power_of(TxPower::for_output(
        chip_amplifier(amplifier),
        dbm(output_dbm),
    ))
}

/// Chooses the amplifier settings that keep a link's EIRP at or under a ceiling, rounded
/// down to whole decibels.
#[napi(js_name = "sx126xTxPowerUnderCeiling")]
pub fn sx126x_tx_power_under_ceiling(
    amplifier: Sx126xAmplifier,
    budget: LoraLinkBudget,
    eirp_ceiling_dbm: f64,
) -> Sx126xTxPower {
    power_of(TxPower::under_ceiling(
        chip_amplifier(amplifier),
        &link_budget(&budget),
        decibels(eirp_ceiling_dbm),
    ))
}

/// SetStandby into STDBY_RC.
#[napi(js_name = "sx126xSetStandby")]
pub fn sx126x_set_standby() -> Buffer {
    bytes_of(command::set_standby(StandbyMode::Rc))
}

/// SetPacketType for LoRa.
#[napi(js_name = "sx126xSetPacketTypeLora")]
pub fn sx126x_set_packet_type_lora() -> Buffer {
    bytes_of(command::set_packet_type(PacketType::Lora))
}

/// SetRfFrequency for a carrier frequency in hertz.
#[napi(js_name = "sx126xSetRfFrequency")]
pub fn sx126x_set_rf_frequency(frequency_hz: u32) -> Buffer {
    bytes_of(command::set_rf_frequency(config::frequency_word(
        frequency_hz,
    )))
}

/// CalibrateImage over a band given by its edges in hertz.
#[napi(js_name = "sx126xCalibrateImage")]
pub fn sx126x_calibrate_image(low_hz: u32, high_hz: u32) -> Buffer {
    bytes_of(command::calibrate_image(config::image_calibration(
        low_hz, high_hz,
    )))
}

/// SetPaConfig for a power setting.
#[napi(js_name = "sx126xSetPaConfig")]
pub fn sx126x_set_pa_config(power: Sx126xTxPower) -> Buffer {
    bytes_of(command::set_pa_config(tx_power(&power).pa))
}

/// SetTxParams for a power setting and the least ramp time wanted, in microseconds.
#[napi(js_name = "sx126xSetTxParams")]
pub fn sx126x_set_tx_params(power: Sx126xTxPower, ramp_us: u32) -> Buffer {
    bytes_of(command::set_tx_params(
        tx_power(&power).setting_dbm,
        RampTime::at_least(ramp_us),
    ))
}

/// SetModulationParams for a LoRa link.
///
/// Throws when the link's bandwidth is not one the SX126x offers.
#[napi(js_name = "sx126xSetLoraModulationParams")]
pub fn sx126x_set_lora_modulation_params(link: LoraLink) -> napi::Result<Buffer> {
    LoraModulation::from_link(&settings(&link))
        .map(|modulation| bytes_of(command::set_lora_modulation_params(modulation)))
        .ok_or_else(|| {
            napi::Error::from_reason(format!(
                "the SX126x has no {} Hz LoRa bandwidth",
                link.bandwidth_hz
            ))
        })
}

/// SetPacketParams for a LoRa link, a payload length, and an IQ polarity.
#[napi(js_name = "sx126xSetLoraPacketParams")]
pub fn sx126x_set_lora_packet_params(
    link: LoraLink,
    payload_length: u8,
    invert_iq: bool,
) -> Buffer {
    bytes_of(command::set_lora_packet_params(LoraPacket::from_link(
        &settings(&link),
        payload_length,
        invert_iq,
    )))
}

/// SetDioIrqParams: which interrupts are enabled, and which DIO lines raise them.
#[napi(js_name = "sx126xSetDioIrqParams")]
pub fn sx126x_set_dio_irq_params(
    irq: u32,
    dio1: u32,
    dio2: Option<u32>,
    dio3: Option<u32>,
) -> Buffer {
    bytes_of(command::set_dio_irq_params(
        irq_of(irq),
        irq_of(dio1),
        irq_of(dio2.unwrap_or(0)),
        irq_of(dio3.unwrap_or(0)),
    ))
}

/// ClearIrqStatus for a set of interrupts.
#[napi(js_name = "sx126xClearIrqStatus")]
pub fn sx126x_clear_irq_status(irq: u32) -> Buffer {
    bytes_of(command::clear_irq_status(irq_of(irq)))
}

/// SetTx with a timeout in microseconds; `0` disables the timeout.
#[napi(js_name = "sx126xSetTx")]
pub fn sx126x_set_tx(timeout_us: f64) -> Buffer {
    bytes_of(command::set_tx(config::timeout_steps(micros(timeout_us))))
}

/// SetRx with a timeout in microseconds; `0` listens for one packet with no timeout.
#[napi(js_name = "sx126xSetRx")]
pub fn sx126x_set_rx(timeout_us: f64) -> Buffer {
    bytes_of(command::set_rx(config::timeout_steps(micros(timeout_us))))
}

/// SetRx in continuous mode, receiving packet after packet until another command.
#[napi(js_name = "sx126xSetRxContinuous")]
pub fn sx126x_set_rx_continuous() -> Buffer {
    bytes_of(command::set_rx(config::RX_CONTINUOUS))
}

/// SetSleep without an RTC wake-up; a warm start keeps the configuration in retention.
#[napi(js_name = "sx126xSetSleep")]
pub fn sx126x_set_sleep(warm_start: bool) -> Buffer {
    bytes_of(command::set_sleep(warm_start, false))
}

/// A whole WriteRegister transaction: the opcode, the address, and the values.
#[napi(js_name = "sx126xWriteRegister")]
pub fn sx126x_write_register(address: u16, values: Buffer) -> Buffer {
    let mut bytes = command::write_register(address).as_bytes().to_vec();
    bytes.extend_from_slice(values.as_ref());
    Buffer::from(bytes)
}

/// A whole WriteBuffer transaction: the opcode, the offset, and the payload.
#[napi(js_name = "sx126xWriteBuffer")]
pub fn sx126x_write_buffer(offset: u8, payload: Buffer) -> Buffer {
    let mut bytes = command::write_buffer(offset).as_bytes().to_vec();
    bytes.extend_from_slice(payload.as_ref());
    Buffer::from(bytes)
}

/// GetStatus, answered by the status byte.
#[napi(js_name = "sx126xGetStatus")]
pub fn sx126x_get_status() -> Sx126xQuery {
    query_of(command::get_status())
}

/// GetIrqStatus, answered by the two IRQ bytes.
#[napi(js_name = "sx126xGetIrqStatus")]
pub fn sx126x_get_irq_status() -> Sx126xQuery {
    query_of(command::get_irq_status())
}

/// GetRxBufferStatus, answered by the payload length and its offset.
#[napi(js_name = "sx126xGetRxBufferStatus")]
pub fn sx126x_get_rx_buffer_status() -> Sx126xQuery {
    query_of(command::get_rx_buffer_status())
}

/// GetPacketStatus, answered by the three LoRa signal level bytes.
#[napi(js_name = "sx126xGetPacketStatus")]
pub fn sx126x_get_packet_status() -> Sx126xQuery {
    query_of(command::get_packet_status())
}

/// GetRssiInst, answered by the instantaneous RSSI byte.
#[napi(js_name = "sx126xGetRssiInst")]
pub fn sx126x_get_rssi_inst() -> Sx126xQuery {
    query_of(command::get_rssi_inst())
}

/// GetDeviceErrors, answered by the two device error bytes.
#[napi(js_name = "sx126xGetDeviceErrors")]
pub fn sx126x_get_device_errors() -> Sx126xQuery {
    query_of(command::get_device_errors())
}

/// ReadRegister for a run of consecutive registers.
#[napi(js_name = "sx126xReadRegister")]
pub fn sx126x_read_register(address: u16, length: u8) -> Sx126xQuery {
    query_of(command::read_register(address, usize::from(length)))
}

/// ReadBuffer for a run of the data buffer.
#[napi(js_name = "sx126xReadBuffer")]
pub fn sx126x_read_buffer(offset: u8, length: u8) -> Sx126xQuery {
    query_of(command::read_buffer(offset, usize::from(length)))
}

/// Decodes a status byte.
#[napi(js_name = "sx126xStatus")]
pub fn sx126x_status(byte: u8) -> Sx126xStatus {
    let status = Status::from_byte(byte);
    Sx126xStatus {
        chip_mode: chip_mode(status.chip_mode),
        command_status: command_status(status.command_status),
        error: status.is_error(),
    }
}

/// Decodes a GetIrqStatus answer into its IRQ bits.
///
/// Throws unless the answer is two bytes.
#[napi(js_name = "sx126xIrq")]
pub fn sx126x_irq(answer: Buffer) -> napi::Result<u32> {
    let bytes = exactly::<2>(&answer, "an IRQ status answer")?;
    Ok(u32::from(Irq::from_bytes(bytes).bits()))
}

/// Decodes a GetDeviceErrors answer into its error bits.
///
/// Throws unless the answer is two bytes.
#[napi(js_name = "sx126xDeviceErrors")]
pub fn sx126x_device_errors(answer: Buffer) -> napi::Result<u32> {
    let bytes = exactly::<2>(&answer, "a device errors answer")?;
    Ok(u32::from(DeviceErrors::from_bytes(bytes).bits()))
}

/// Decodes a LoRa GetPacketStatus answer.
///
/// Throws unless the answer is three bytes.
#[napi(js_name = "sx126xPacketStatus")]
pub fn sx126x_packet_status(answer: Buffer) -> napi::Result<Sx126xPacketStatus> {
    let status = PacketStatus::from_bytes(exactly::<3>(&answer, "a packet status answer")?);
    Ok(Sx126xPacketStatus {
        rssi_dbm: db(status.rssi_dbm),
        snr_db: db(status.snr_db),
        signal_rssi_dbm: db(status.signal_rssi_dbm),
    })
}

/// Decodes a GetRxBufferStatus answer.
///
/// Throws unless the answer is two bytes.
#[napi(js_name = "sx126xRxBufferStatus")]
pub fn sx126x_rx_buffer_status(answer: Buffer) -> napi::Result<Sx126xRxBufferStatus> {
    let status = RxBufferStatus::from_bytes(exactly::<2>(&answer, "an RX buffer status answer")?);
    Ok(Sx126xRxBufferStatus {
        payload_length: status.payload_len,
        start: status.start,
    })
}

/// Decodes a GetRssiInst answer, in dBm.
#[napi(js_name = "sx126xRssiInstDbm")]
pub fn sx126x_rssi_inst_dbm(byte: u8) -> f64 {
    db(rssi_inst_dbm(byte))
}

/// The silence a radio owes after its transmissions under a duty-cycle limit.
#[napi]
pub struct RadioDutyCycle {
    inner: DutyCycle,
}

#[napi]
impl RadioDutyCycle {
    /// Creates a guard for a limit in parts per thousand, ready to transmit at once.
    ///
    /// `10` is 1%; `0` forbids transmitting and `1000` or more imposes no silence.
    #[napi(constructor)]
    pub fn new(permille: u32) -> Self {
        Self {
            inner: DutyCycle::new(permille),
        }
    }

    /// The limit the guard enforces, in parts per thousand.
    #[napi(getter)]
    pub fn permille(&self) -> u32 {
        self.inner.permille()
    }

    /// The earliest time the next transmission may start, in microseconds on the caller's
    /// clock, or `null` when the limit forbids transmitting.
    #[napi(getter)]
    pub fn earliest_us(&self) -> Option<f64> {
        never(self.inner.earliest_us())
    }

    /// How long the radio must still stay silent, in microseconds, or `null` when the
    /// limit forbids transmitting.
    #[napi]
    pub fn wait_us(&self, now_us: f64) -> Option<f64> {
        never(self.inner.wait_us(micros(now_us)))
    }

    /// Whether a transmission may start at a time in microseconds on the caller's clock.
    #[napi]
    pub fn ready(&self, now_us: f64) -> bool {
        self.inner.ready(micros(now_us))
    }

    /// Records a transmission and the silence it owes, returning its airtime in
    /// microseconds.
    #[napi]
    pub fn transmitted(&mut self, started_us: f64, link: LoraLink, payload_length: u32) -> f64 {
        self.inner.transmitted(
            micros(started_us),
            &settings(&link),
            payload_length as usize,
        ) as f64
    }
}

/// Turns a JavaScript number of microseconds into a count, treating anything below zero
/// or not a number as zero.
fn micros(value: f64) -> u64 {
    if value.is_finite() && value > 0.0 {
        value as u64
    } else {
        0
    }
}

/// Reports the clock value that means never as `null`.
fn never(value: u64) -> Option<f64> {
    (value != u64::MAX).then_some(value as f64)
}

/// Clamps a JavaScript power to the byte SetTxParams carries.
fn dbm(value: i32) -> i8 {
    value.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8
}

/// Takes the low sixteen bits of a JavaScript IRQ mask.
fn irq_of(bits: u32) -> Irq {
    Irq::from_bits(bits as u16)
}

/// Copies a command's bytes into a buffer.
fn bytes_of(command: Command) -> Buffer {
    Buffer::from(command.as_bytes().to_vec())
}

/// Describes a query for JavaScript.
fn query_of(query: Query) -> Sx126xQuery {
    Sx126xQuery {
        bytes: bytes_of(query.command),
        answer_length: query.answer_len as u32,
    }
}

/// Reads an answer that must be exactly `N` bytes.
fn exactly<const N: usize>(answer: &Buffer, what: &str) -> napi::Result<[u8; N]> {
    <[u8; N]>::try_from(answer.as_ref())
        .map_err(|_| napi::Error::from_reason(format!("{what} is {N} bytes, not {}", answer.len())))
}

/// Names the amplifier the JavaScript enum selects.
fn chip_amplifier(amplifier: Sx126xAmplifier) -> PowerAmplifier {
    match amplifier {
        Sx126xAmplifier::LowPower => PowerAmplifier::LowPower,
        Sx126xAmplifier::HighPower => PowerAmplifier::HighPower,
    }
}

/// Flattens power settings into the object JavaScript sees.
fn power_of(power: TxPower) -> Sx126xTxPower {
    Sx126xTxPower {
        pa_duty_cycle: power.pa.duty_cycle,
        hp_max: power.pa.hp_max,
        device_sel: power.pa.device,
        pa_lut: power.pa.lut,
        setting_dbm: i32::from(power.setting_dbm),
    }
}

/// Rebuilds power settings from the object JavaScript passed.
fn tx_power(power: &Sx126xTxPower) -> TxPower {
    TxPower {
        pa: PaConfig {
            duty_cycle: power.pa_duty_cycle,
            hp_max: power.hp_max,
            device: power.device_sel,
            lut: power.pa_lut,
        },
        setting_dbm: dbm(power.setting_dbm),
    }
}

/// Names a decoded chip mode for JavaScript.
fn chip_mode(mode: ChipMode) -> Sx126xChipMode {
    if mode == ChipMode::StandbyRc {
        Sx126xChipMode::StandbyRc
    } else if mode == ChipMode::StandbyXosc {
        Sx126xChipMode::StandbyXosc
    } else if mode == ChipMode::Fs {
        Sx126xChipMode::Fs
    } else if mode == ChipMode::Rx {
        Sx126xChipMode::Rx
    } else if mode == ChipMode::Tx {
        Sx126xChipMode::Tx
    } else {
        Sx126xChipMode::Other
    }
}

/// Names a decoded command status for JavaScript.
fn command_status(status: CommandStatus) -> Sx126xCommandStatus {
    if status == CommandStatus::DataAvailable {
        Sx126xCommandStatus::DataAvailable
    } else if status == CommandStatus::Timeout {
        Sx126xCommandStatus::Timeout
    } else if status == CommandStatus::ProcessingError {
        Sx126xCommandStatus::ProcessingError
    } else if status == CommandStatus::ExecutionFailure {
        Sx126xCommandStatus::ExecutionFailure
    } else if status == CommandStatus::TxDone {
        Sx126xCommandStatus::TxDone
    } else {
        Sx126xCommandStatus::Other
    }
}
