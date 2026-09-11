//! Generated Python bindings for LoRa radio chips.
//!
//! These mirror the `pamoja-radios` Rust API: the bytes of each Semtech SX126x command, the
//! chip's answers decoded, the amplifier settings a regional power ceiling allows, and the
//! silence a duty-cycle limit forces after each transmission.
//!
//! For the SX1276 family, which is driven through registers, they give the register
//! addresses, the values a link and an output power put in them, and the readings decoded.
//!
//! A command is a few bytes, so it crosses as `bytes`, and the settings and the decoded
//! answers are read-only objects. The duty-cycle guard keeps state between calls, so it is a
//! class.

use std::collections::HashMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

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

use pamoja_radios::sx126x::config::llcc68_supports;
use pamoja_radios::sx127x::config::{
    self as sx127x_config, LoraBandwidth as Sx127xBandwidth, LoraModulation as Sx127xModulation,
    ModulationError, PaOutput, TxPower as Sx127xPower,
};
use pamoja_radios::sx127x::irq::IrqFlags;
use pamoja_radios::sx127x::register::{self as sx127x_register, Mode as Sx127xMode};
use pamoja_radios::sx127x::status::{
    rssi_dbm as decoded_rssi_dbm, ModemStatus as DecodedModemStatus,
    PacketStatus as DecodedPacketStatus, Port,
};

use crate::lora::{db, decibels, LinkBudget, LoraLink};
use crate::PamojaError;

/// The amplifier configuration and power setting that produce an output power.
#[gen_stub_pyclass]
#[pyclass]
pub struct Sx126xTxPower {
    /// paDutyCycle, the conduction angle of the amplifier.
    #[pyo3(get)]
    pa_duty_cycle: u8,
    /// hpMax, the size of the SX1262 amplifier; no effect on the SX1261.
    #[pyo3(get)]
    hp_max: u8,
    /// deviceSel: 0 for the SX1262 and the LLCC68, 1 for the SX1261.
    #[pyo3(get)]
    device_sel: u8,
    /// paLut, reserved and always 1.
    #[pyo3(get)]
    pa_lut: u8,
    /// The power byte of SetTxParams, in dBm.
    #[pyo3(get)]
    setting_dbm: i8,
}

/// A command the chip answers in the same SPI transaction.
#[gen_stub_pyclass]
#[pyclass]
pub struct Sx126xQuery {
    bytes: Vec<u8>,
    /// How many bytes of answer to read after the query's bytes.
    #[pyo3(get)]
    answer_len: usize,
}

#[gen_stub_pymethods]
#[pymethods]
impl Sx126xQuery {
    /// The bytes to send, ending with the NOP during which the status byte comes back.
    #[getter]
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.bytes)
    }
}

/// A decoded SX126x status byte.
#[gen_stub_pyclass]
#[pyclass]
pub struct Sx126xStatus {
    /// The mode the chip is in: `StandbyRc`, `StandbyXosc`, `Fs`, `Rx`, `Tx`, or `Other`.
    #[pyo3(get)]
    chip_mode: String,
    /// How the last command went: `DataAvailable`, `Timeout`, `ProcessingError`,
    /// `ExecutionFailure`, `TxDone`, or `Other`.
    #[pyo3(get)]
    command_status: String,
    /// Whether the last command timed out, could not be processed, or failed.
    #[pyo3(get)]
    error: bool,
}

/// The signal levels of the last LoRa packet received.
#[gen_stub_pyclass]
#[pyclass]
pub struct Sx126xPacketStatus {
    /// The RSSI averaged over the packet, in dBm.
    #[pyo3(get)]
    rssi_dbm: f64,
    /// The estimated signal-to-noise ratio, in dB.
    #[pyo3(get)]
    snr_db: f64,
    /// The estimated RSSI of the LoRa signal after despreading, in dBm.
    #[pyo3(get)]
    signal_rssi_dbm: f64,
}

/// Where a received payload sits in the data buffer.
#[gen_stub_pyclass]
#[pyclass]
pub struct Sx126xRxBufferStatus {
    /// The length of the payload in bytes.
    #[pyo3(get)]
    payload_len: u8,
    /// The buffer offset of its first byte.
    #[pyo3(get)]
    start: u8,
}

/// The silence a radio owes after its transmissions under a duty-cycle limit.
#[gen_stub_pyclass]
#[pyclass]
pub struct RadioDutyCycle {
    inner: DutyCycle,
}

#[gen_stub_pymethods]
#[pymethods]
impl RadioDutyCycle {
    /// Creates a guard for a limit in parts per thousand, ready to transmit at once.
    ///
    /// `10` is 1%; `0` forbids transmitting and `1000` or more imposes no silence.
    #[new]
    fn new(permille: u32) -> Self {
        RadioDutyCycle {
            inner: DutyCycle::new(permille),
        }
    }

    /// The limit the guard enforces, in parts per thousand.
    #[getter]
    fn permille(&self) -> u32 {
        self.inner.permille()
    }

    /// The earliest time the next transmission may start, in microseconds on the caller's
    /// clock, or `None` when the limit forbids transmitting.
    #[getter]
    fn earliest_us(&self) -> Option<u64> {
        never(self.inner.earliest_us())
    }

    /// How long the radio must still stay silent, in microseconds, or `None` when the limit
    /// forbids transmitting.
    fn wait_us(&self, now_us: u64) -> Option<u64> {
        never(self.inner.wait_us(now_us))
    }

    /// Whether a transmission may start at a time in microseconds on the caller's clock.
    fn ready(&self, now_us: u64) -> bool {
        self.inner.ready(now_us)
    }

    /// Records a transmission and the silence it owes, returning its airtime in
    /// microseconds.
    fn transmitted(
        &mut self,
        started_us: u64,
        link: PyRef<'_, LoraLink>,
        payload_len: usize,
    ) -> u64 {
        self.inner
            .transmitted(started_us, &link.settings(), payload_len)
    }
}

/// The SX126x constants: the continuous receive word, the sync words and their register,
/// and every IRQ and device error bit, by name.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_constants() -> HashMap<&'static str, u32> {
    let mut constants = HashMap::new();
    constants.insert("RX_CONTINUOUS", config::RX_CONTINUOUS);
    constants.insert(
        "SYNC_WORD_PUBLIC",
        u32::from(u16::from_be_bytes(SyncWord::Public.to_bytes())),
    );
    constants.insert(
        "SYNC_WORD_PRIVATE",
        u32::from(u16::from_be_bytes(SyncWord::Private.to_bytes())),
    );
    constants.insert(
        "REGISTER_LORA_SYNC_WORD",
        u32::from(config::register::LORA_SYNC_WORD),
    );
    for (name, irq) in [
        ("IRQ_TX_DONE", Irq::TX_DONE),
        ("IRQ_RX_DONE", Irq::RX_DONE),
        ("IRQ_PREAMBLE_DETECTED", Irq::PREAMBLE_DETECTED),
        ("IRQ_SYNC_WORD_VALID", Irq::SYNC_WORD_VALID),
        ("IRQ_HEADER_VALID", Irq::HEADER_VALID),
        ("IRQ_HEADER_ERROR", Irq::HEADER_ERROR),
        ("IRQ_CRC_ERROR", Irq::CRC_ERROR),
        ("IRQ_CAD_DONE", Irq::CAD_DONE),
        ("IRQ_CAD_DETECTED", Irq::CAD_DETECTED),
        ("IRQ_TIMEOUT", Irq::TIMEOUT),
        ("IRQ_LR_FHSS_HOP", Irq::LR_FHSS_HOP),
        ("IRQ_ALL", Irq::ALL),
    ] {
        constants.insert(name, u32::from(irq.bits()));
    }
    for (name, error) in [
        ("ERROR_RC64K_CALIBRATION", DeviceErrors::RC64K_CALIBRATION),
        ("ERROR_RC13M_CALIBRATION", DeviceErrors::RC13M_CALIBRATION),
        ("ERROR_PLL_CALIBRATION", DeviceErrors::PLL_CALIBRATION),
        ("ERROR_ADC_CALIBRATION", DeviceErrors::ADC_CALIBRATION),
        ("ERROR_IMAGE_CALIBRATION", DeviceErrors::IMAGE_CALIBRATION),
        ("ERROR_XOSC_START", DeviceErrors::XOSC_START),
        ("ERROR_PLL_LOCK", DeviceErrors::PLL_LOCK),
        ("ERROR_PA_RAMP", DeviceErrors::PA_RAMP),
    ] {
        constants.insert(name, u32::from(error.bits()));
    }
    constants
}

/// The word SetRfFrequency takes for a frequency in hertz.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_frequency_word(frequency_hz: u32) -> u32 {
    config::frequency_word(frequency_hz)
}

/// The 24-bit timeout word SetTx and SetRx take for a duration in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_timeout_steps(timeout_us: u64) -> u32 {
    config::timeout_steps(timeout_us)
}

/// The two CalibrateImage codes that cover a band given by its edges in hertz.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_image_calibration(py: Python<'_>, low_hz: u32, high_hz: u32) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &config::image_calibration(low_hz, high_hz))
}

/// The shortest amplifier ramp time the chip offers that lasts at least a duration, in
/// microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_ramp_time_us(at_least_us: u32) -> u32 {
    RampTime::at_least(at_least_us).micros()
}

/// The amplifier settings for an output power, clamped to what the amplifier allows.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_tx_power(amplifier: &str, output_dbm: i32) -> PyResult<Sx126xTxPower> {
    Ok(power_of(TxPower::for_output(
        chip_amplifier(amplifier)?,
        dbm(output_dbm),
    )))
}

/// The amplifier settings that keep a link's EIRP at or under a ceiling, rounded down to
/// whole decibels.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_tx_power_under_ceiling(
    amplifier: &str,
    budget: PyRef<'_, LinkBudget>,
    eirp_ceiling_dbm: f64,
) -> PyResult<Sx126xTxPower> {
    Ok(power_of(TxPower::under_ceiling(
        chip_amplifier(amplifier)?,
        &budget.budget(),
        decibels(eirp_ceiling_dbm),
    )))
}

/// SetStandby into STDBY_RC.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_set_standby(py: Python<'_>) -> Bound<'_, PyBytes> {
    bytes_of(py, command::set_standby(StandbyMode::Rc))
}

/// SetPacketType for LoRa.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_set_packet_type_lora(py: Python<'_>) -> Bound<'_, PyBytes> {
    bytes_of(py, command::set_packet_type(PacketType::Lora))
}

/// SetRfFrequency for a carrier frequency in hertz.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_set_rf_frequency(py: Python<'_>, frequency_hz: u32) -> Bound<'_, PyBytes> {
    bytes_of(
        py,
        command::set_rf_frequency(config::frequency_word(frequency_hz)),
    )
}

/// CalibrateImage over a band given by its edges in hertz.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_calibrate_image(py: Python<'_>, low_hz: u32, high_hz: u32) -> Bound<'_, PyBytes> {
    bytes_of(
        py,
        command::calibrate_image(config::image_calibration(low_hz, high_hz)),
    )
}

/// SetPaConfig for a power setting.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_set_pa_config<'py>(
    py: Python<'py>,
    power: PyRef<'_, Sx126xTxPower>,
) -> Bound<'py, PyBytes> {
    bytes_of(py, command::set_pa_config(tx_power(&power).pa))
}

/// SetTxParams for a power setting and the least ramp time wanted, in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_set_tx_params<'py>(
    py: Python<'py>,
    power: PyRef<'_, Sx126xTxPower>,
    ramp_us: u32,
) -> Bound<'py, PyBytes> {
    bytes_of(
        py,
        command::set_tx_params(power.setting_dbm, RampTime::at_least(ramp_us)),
    )
}

/// SetModulationParams for a LoRa link.
///
/// Raises `PamojaError` when the link's bandwidth is not one the SX126x offers.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_set_lora_modulation_params<'py>(
    py: Python<'py>,
    link: PyRef<'_, LoraLink>,
) -> PyResult<Bound<'py, PyBytes>> {
    let settings = link.settings();
    let modulation = LoraModulation::from_link(&settings).ok_or_else(|| {
        PamojaError::new_err(format!(
            "the SX126x has no {} Hz LoRa bandwidth",
            settings.bandwidth_hz()
        ))
    })?;
    Ok(bytes_of(
        py,
        command::set_lora_modulation_params(modulation),
    ))
}

/// SetPacketParams for a LoRa link, a payload length, and an IQ polarity.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_set_lora_packet_params<'py>(
    py: Python<'py>,
    link: PyRef<'_, LoraLink>,
    payload_len: u8,
    invert_iq: bool,
) -> Bound<'py, PyBytes> {
    bytes_of(
        py,
        command::set_lora_packet_params(LoraPacket::from_link(
            &link.settings(),
            payload_len,
            invert_iq,
        )),
    )
}

/// SetDioIrqParams: which interrupts are enabled, and which DIO lines raise them.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (irq, dio1, dio2 = 0, dio3 = 0))]
pub fn sx126x_set_dio_irq_params(
    py: Python<'_>,
    irq: u16,
    dio1: u16,
    dio2: u16,
    dio3: u16,
) -> Bound<'_, PyBytes> {
    bytes_of(
        py,
        command::set_dio_irq_params(
            Irq::from_bits(irq),
            Irq::from_bits(dio1),
            Irq::from_bits(dio2),
            Irq::from_bits(dio3),
        ),
    )
}

/// ClearIrqStatus for a set of interrupts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_clear_irq_status(py: Python<'_>, irq: u16) -> Bound<'_, PyBytes> {
    bytes_of(py, command::clear_irq_status(Irq::from_bits(irq)))
}

/// SetTx with a timeout in microseconds; `0` disables the timeout.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_set_tx(py: Python<'_>, timeout_us: u64) -> Bound<'_, PyBytes> {
    bytes_of(py, command::set_tx(config::timeout_steps(timeout_us)))
}

/// SetRx with a timeout in microseconds; `0` listens for one packet with no timeout.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_set_rx(py: Python<'_>, timeout_us: u64) -> Bound<'_, PyBytes> {
    bytes_of(py, command::set_rx(config::timeout_steps(timeout_us)))
}

/// SetRx in continuous mode, receiving packet after packet until another command.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_set_rx_continuous(py: Python<'_>) -> Bound<'_, PyBytes> {
    bytes_of(py, command::set_rx(config::RX_CONTINUOUS))
}

/// SetSleep without an RTC wake-up; a warm start keeps the configuration in retention.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_set_sleep(py: Python<'_>, warm_start: bool) -> Bound<'_, PyBytes> {
    bytes_of(py, command::set_sleep(warm_start, false))
}

/// A whole WriteRegister transaction: the opcode, the address, and the values.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_write_register<'py>(
    py: Python<'py>,
    address: u16,
    values: Vec<u8>,
) -> Bound<'py, PyBytes> {
    let mut bytes = command::write_register(address).as_bytes().to_vec();
    bytes.extend_from_slice(&values);
    PyBytes::new(py, &bytes)
}

/// A whole WriteBuffer transaction: the opcode, the offset, and the payload.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_write_buffer<'py>(
    py: Python<'py>,
    offset: u8,
    payload: Vec<u8>,
) -> Bound<'py, PyBytes> {
    let mut bytes = command::write_buffer(offset).as_bytes().to_vec();
    bytes.extend_from_slice(&payload);
    PyBytes::new(py, &bytes)
}

/// GetStatus, answered by the status byte.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_get_status() -> Sx126xQuery {
    query_of(command::get_status())
}

/// GetIrqStatus, answered by the two IRQ bytes.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_get_irq_status() -> Sx126xQuery {
    query_of(command::get_irq_status())
}

/// GetRxBufferStatus, answered by the payload length and its offset.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_get_rx_buffer_status() -> Sx126xQuery {
    query_of(command::get_rx_buffer_status())
}

/// GetPacketStatus, answered by the three LoRa signal level bytes.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_get_packet_status() -> Sx126xQuery {
    query_of(command::get_packet_status())
}

/// GetRssiInst, answered by the instantaneous RSSI byte.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_get_rssi_inst() -> Sx126xQuery {
    query_of(command::get_rssi_inst())
}

/// GetDeviceErrors, answered by the two device error bytes.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_get_device_errors() -> Sx126xQuery {
    query_of(command::get_device_errors())
}

/// ReadRegister for a run of consecutive registers.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_read_register(address: u16, length: u8) -> Sx126xQuery {
    query_of(command::read_register(address, usize::from(length)))
}

/// ReadBuffer for a run of the data buffer.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_read_buffer(offset: u8, length: u8) -> Sx126xQuery {
    query_of(command::read_buffer(offset, usize::from(length)))
}

/// Decodes a status byte.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_status(byte: u8) -> Sx126xStatus {
    let status = Status::from_byte(byte);
    Sx126xStatus {
        chip_mode: chip_mode(status.chip_mode).to_owned(),
        command_status: command_status(status.command_status).to_owned(),
        error: status.is_error(),
    }
}

/// Decodes a GetIrqStatus answer into its IRQ bits.
///
/// Raises `ValueError` unless the answer is two bytes.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_irq(answer: Vec<u8>) -> PyResult<u16> {
    Ok(Irq::from_bytes(exactly(&answer, "an IRQ status answer")?).bits())
}

/// Decodes a GetDeviceErrors answer into its error bits.
///
/// Raises `ValueError` unless the answer is two bytes.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_device_errors(answer: Vec<u8>) -> PyResult<u16> {
    Ok(DeviceErrors::from_bytes(exactly(&answer, "a device errors answer")?).bits())
}

/// Decodes a LoRa GetPacketStatus answer.
///
/// Raises `ValueError` unless the answer is three bytes.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_packet_status(answer: Vec<u8>) -> PyResult<Sx126xPacketStatus> {
    let status = PacketStatus::from_bytes(exactly(&answer, "a packet status answer")?);
    Ok(Sx126xPacketStatus {
        rssi_dbm: db(status.rssi_dbm),
        snr_db: db(status.snr_db),
        signal_rssi_dbm: db(status.signal_rssi_dbm),
    })
}

/// Decodes a GetRxBufferStatus answer.
///
/// Raises `ValueError` unless the answer is two bytes.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_rx_buffer_status(answer: Vec<u8>) -> PyResult<Sx126xRxBufferStatus> {
    let status = RxBufferStatus::from_bytes(exactly(&answer, "an RX buffer status answer")?);
    Ok(Sx126xRxBufferStatus {
        payload_len: status.payload_len,
        start: status.start,
    })
}

/// Decodes a GetRssiInst answer, in dBm.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_rssi_inst_dbm(byte: u8) -> f64 {
    db(rssi_inst_dbm(byte))
}

/// Reports the clock value that means never as `None`.
fn never(value: u64) -> Option<u64> {
    (value != u64::MAX).then_some(value)
}

/// Clamps a Python power to the byte SetTxParams carries.
fn dbm(value: i32) -> i8 {
    value.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8
}

/// Copies a command's bytes into a Python `bytes`.
fn bytes_of(py: Python<'_>, command: Command) -> Bound<'_, PyBytes> {
    PyBytes::new(py, command.as_bytes())
}

/// Describes a query for Python.
fn query_of(query: Query) -> Sx126xQuery {
    Sx126xQuery {
        bytes: query.command.as_bytes().to_vec(),
        answer_len: query.answer_len,
    }
}

/// Reads an answer that must be exactly `N` bytes.
fn exactly<const N: usize>(answer: &[u8], what: &str) -> PyResult<[u8; N]> {
    <[u8; N]>::try_from(answer)
        .map_err(|_| PyValueError::new_err(format!("{what} is {N} bytes, not {}", answer.len())))
}

/// Names the amplifier a Python string selects.
fn chip_amplifier(name: &str) -> PyResult<PowerAmplifier> {
    match name {
        "LowPower" => Ok(PowerAmplifier::LowPower),
        "HighPower" => Ok(PowerAmplifier::HighPower),
        other => Err(PyValueError::new_err(format!(
            "no amplifier is named {other}; use LowPower or HighPower"
        ))),
    }
}

/// Flattens power settings into the object Python sees.
fn power_of(power: TxPower) -> Sx126xTxPower {
    Sx126xTxPower {
        pa_duty_cycle: power.pa.duty_cycle,
        hp_max: power.pa.hp_max,
        device_sel: power.pa.device,
        pa_lut: power.pa.lut,
        setting_dbm: power.setting_dbm,
    }
}

/// Rebuilds power settings from the object Python passed.
fn tx_power(power: &Sx126xTxPower) -> TxPower {
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

/// Names a decoded chip mode for Python.
fn chip_mode(mode: ChipMode) -> &'static str {
    if mode == ChipMode::StandbyRc {
        "StandbyRc"
    } else if mode == ChipMode::StandbyXosc {
        "StandbyXosc"
    } else if mode == ChipMode::Fs {
        "Fs"
    } else if mode == ChipMode::Rx {
        "Rx"
    } else if mode == ChipMode::Tx {
        "Tx"
    } else {
        "Other"
    }
}

/// Names a decoded command status for Python.
fn command_status(status: CommandStatus) -> &'static str {
    if status == CommandStatus::DataAvailable {
        "DataAvailable"
    } else if status == CommandStatus::Timeout {
        "Timeout"
    } else if status == CommandStatus::ProcessingError {
        "ProcessingError"
    } else if status == CommandStatus::ExecutionFailure {
        "ExecutionFailure"
    } else if status == CommandStatus::TxDone {
        "TxDone"
    } else {
        "Other"
    }
}

/// Reports whether an LLCC68 supports a link's spreading factor at its bandwidth: up to SF9
/// at 125 kHz, SF10 at 250 kHz, and SF11 at 500 kHz.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx126x_llcc68_supports(link: PyRef<'_, LoraLink>) -> bool {
    LoraModulation::from_link(&link.settings()).is_some_and(|modulation| {
        llcc68_supports(modulation.spreading_factor, modulation.bandwidth)
    })
}

/// The amplifier settings of an SX127x.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Sx127xTxPower {
    /// RegPaConfig: PaSelect, MaxPower, and OutputPower.
    #[pyo3(get)]
    pub pa_config: u8,
    /// RegPaDac: the +20 dBm setting above +17 dBm on PA_BOOST, else its reset value.
    #[pyo3(get)]
    pub pa_dac: u8,
    /// RegOcp: the current limit.
    #[pyo3(get)]
    pub ocp: u8,
    /// The output power the settings produce, in dBm.
    #[pyo3(get)]
    pub output_dbm: i8,
}

/// The LoRa modem registers of an SX127x for a link.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Sx127xModem {
    /// RegModemConfig1: bandwidth, coding rate, and header mode.
    #[pyo3(get)]
    pub modem_config_1: u8,
    /// RegModemConfig2: spreading factor, CRC, and the top bits of the symbol timeout.
    #[pyo3(get)]
    pub modem_config_2: u8,
    /// RegModemConfig3: low data rate optimization and the AGC.
    #[pyo3(get)]
    pub modem_config_3: u8,
    /// The DetectionOptimize bits for the low three bits of RegDetectOptimize.
    #[pyo3(get)]
    pub detection_optimize: u8,
    /// RegDetectionThreshold.
    #[pyo3(get)]
    pub detection_threshold: u8,
}

/// The signal levels of a packet an SX127x received.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Sx127xPacketStatus {
    /// The RSSI averaged over the packet, in dBm.
    #[pyo3(get)]
    pub rssi_dbm: f64,
    /// The estimated signal-to-noise ratio, in dB.
    #[pyo3(get)]
    pub snr_db: f64,
    /// The strength of the packet itself, in dBm.
    #[pyo3(get)]
    pub signal_rssi_dbm: f64,
}

/// The live state of an SX127x LoRa modem.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Sx127xModemStatus {
    /// The coding rate denominator the last header announced, or `None` for a reserved value.
    #[pyo3(get)]
    pub coding_rate_denominator: Option<u8>,
    /// The modem is clear.
    #[pyo3(get)]
    pub clear: bool,
    /// The header of the packet under way is valid.
    #[pyo3(get)]
    pub header_valid: bool,
    /// A reception is under way.
    #[pyo3(get)]
    pub rx_ongoing: bool,
    /// The modem has synchronized on the end of the preamble.
    #[pyo3(get)]
    pub signal_synchronized: bool,
    /// A LoRa preamble has been detected.
    #[pyo3(get)]
    pub signal_detected: bool,
}

/// The writes of the SX127x 500 kHz sensitivity erratum.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Sx127xHighBwOptimize {
    /// The RegHighBwOptimize1 value.
    #[pyo3(get)]
    pub optimize_1: u8,
    /// The RegHighBwOptimize2 value, or `None` when it is not written.
    #[pyo3(get)]
    pub optimize_2: Option<u8>,
}

/// The receive settings of the SX127x spurious reception erratum.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Sx127xSpuriousReception {
    /// Whether AutomaticIFOn stays on.
    #[pyo3(get)]
    pub automatic_if: bool,
    /// The RegIfFreq2 value, with RegIfFreq1 cleared, or `None` when the IF stays automatic.
    #[pyo3(get)]
    pub if_freq_2: Option<u8>,
    /// How far above the carrier to receive, in hertz.
    #[pyo3(get)]
    pub offset_hz: u32,
}

/// The SX127x register addresses, by name.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_registers() -> HashMap<&'static str, u8> {
    HashMap::from([
        ("fifo", sx127x_register::FIFO),
        ("opMode", sx127x_register::OP_MODE),
        ("frfMsb", sx127x_register::FRF_MSB),
        ("frfMid", sx127x_register::FRF_MID),
        ("frfLsb", sx127x_register::FRF_LSB),
        ("paConfig", sx127x_register::PA_CONFIG),
        ("paRamp", sx127x_register::PA_RAMP),
        ("ocp", sx127x_register::OCP),
        ("lna", sx127x_register::LNA),
        ("fifoAddrPtr", sx127x_register::FIFO_ADDR_PTR),
        ("fifoTxBaseAddr", sx127x_register::FIFO_TX_BASE_ADDR),
        ("fifoRxBaseAddr", sx127x_register::FIFO_RX_BASE_ADDR),
        ("fifoRxCurrentAddr", sx127x_register::FIFO_RX_CURRENT_ADDR),
        ("irqFlagsMask", sx127x_register::IRQ_FLAGS_MASK),
        ("irqFlags", sx127x_register::IRQ_FLAGS),
        ("rxNbBytes", sx127x_register::RX_NB_BYTES),
        ("modemStat", sx127x_register::MODEM_STAT),
        ("pktSnrValue", sx127x_register::PKT_SNR_VALUE),
        ("pktRssiValue", sx127x_register::PKT_RSSI_VALUE),
        ("rssiValue", sx127x_register::RSSI_VALUE),
        ("hopChannel", sx127x_register::HOP_CHANNEL),
        ("modemConfig1", sx127x_register::MODEM_CONFIG_1),
        ("modemConfig2", sx127x_register::MODEM_CONFIG_2),
        ("symbTimeoutLsb", sx127x_register::SYMB_TIMEOUT_LSB),
        ("preambleMsb", sx127x_register::PREAMBLE_MSB),
        ("preambleLsb", sx127x_register::PREAMBLE_LSB),
        ("payloadLength", sx127x_register::PAYLOAD_LENGTH),
        ("maxPayloadLength", sx127x_register::MAX_PAYLOAD_LENGTH),
        ("modemConfig3", sx127x_register::MODEM_CONFIG_3),
        ("rssiWideband", sx127x_register::RSSI_WIDEBAND),
        ("ifFreq2", sx127x_register::IF_FREQ_2),
        ("ifFreq1", sx127x_register::IF_FREQ_1),
        ("detectOptimize", sx127x_register::DETECT_OPTIMIZE),
        ("invertIq", sx127x_register::INVERT_IQ),
        ("highBwOptimize1", sx127x_register::HIGH_BW_OPTIMIZE_1),
        ("detectionThreshold", sx127x_register::DETECTION_THRESHOLD),
        ("syncWord", sx127x_register::SYNC_WORD),
        ("highBwOptimize2", sx127x_register::HIGH_BW_OPTIMIZE_2),
        ("invertIq2", sx127x_register::INVERT_IQ_2),
        ("imageCal", sx127x_register::IMAGE_CAL),
        ("dioMapping1", sx127x_register::DIO_MAPPING_1),
        ("dioMapping2", sx127x_register::DIO_MAPPING_2),
        ("version", sx127x_register::VERSION),
        ("tcxo", sx127x_register::TCXO),
        ("paDac", sx127x_register::PA_DAC),
    ])
}

/// The SX127x register values with a name: the version, the write bit, the DIO0 mappings,
/// the amplifier and calibration bits, the sync words, and the LNA and TCXO settings.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_constants() -> HashMap<&'static str, u8> {
    HashMap::from([
        ("version", sx127x_register::VERSION_SX1276),
        ("write", sx127x_register::WRITE),
        ("dio0RxDone", sx127x_config::DIO0_RX_DONE),
        ("dio0TxDone", sx127x_config::DIO0_TX_DONE),
        ("dio0CadDone", sx127x_config::DIO0_CAD_DONE),
        ("paDacDefault", sx127x_config::PA_DAC_DEFAULT),
        ("paDacHighPower", sx127x_config::PA_DAC_HIGH_POWER),
        ("imageCalStart", sx127x_config::IMAGE_CAL_START),
        ("imageCalRunning", sx127x_config::IMAGE_CAL_RUNNING),
        ("syncWordPublic", sx127x_config::SyncWord::Public.to_byte()),
        (
            "syncWordPrivate",
            sx127x_config::SyncWord::Private.to_byte(),
        ),
        ("lnaBoosted", sx127x_config::LNA_BOOSTED),
        ("tcxoInputOn", sx127x_config::TCXO_INPUT_ON),
    ])
}

/// The SX127x LoRa interrupt flags, by name.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_irq_flags() -> HashMap<&'static str, u8> {
    HashMap::from([
        ("rxTimeout", IrqFlags::RX_TIMEOUT.bits()),
        ("rxDone", IrqFlags::RX_DONE.bits()),
        ("payloadCrcError", IrqFlags::PAYLOAD_CRC_ERROR.bits()),
        ("validHeader", IrqFlags::VALID_HEADER.bits()),
        ("txDone", IrqFlags::TX_DONE.bits()),
        ("cadDone", IrqFlags::CAD_DONE.bits()),
        ("fhssChangeChannel", IrqFlags::FHSS_CHANGE_CHANNEL.bits()),
        ("cadDetected", IrqFlags::CAD_DETECTED.bits()),
    ])
}

/// The 24-bit RegFrf word an SX127x takes for a frequency in hertz.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_frequency_word(frequency_hz: u32) -> u32 {
    sx127x_config::frequency_word(frequency_hz)
}

/// The frequency in hertz an SX127x RegFrf word selects.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_frequency_from_word(word: u32) -> u32 {
    sx127x_config::frequency_from_word(word)
}

/// The SX127x address byte that reads a register.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_read_address(address: u8) -> u8 {
    sx127x_register::read_address(address)
}

/// The SX127x address byte that writes a register.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_write_address(address: u8) -> u8 {
    sx127x_register::write_address(address)
}

/// The RegOpMode value for a LoRa operating mode, named as in `Sx127xMode`.
///
/// Raises `ValueError` for a name that is not a mode.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_lora_op_mode(mode: &str) -> PyResult<u8> {
    Ok(sx127x_register::lora_op_mode(mode_named(mode)?))
}

/// The RegOpMode value for an FSK operating mode, which image calibration needs.
///
/// Raises `ValueError` for a name that is not a mode.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_fsk_op_mode(mode: &str) -> PyResult<u8> {
    Ok(sx127x_register::fsk_op_mode(mode_named(mode)?))
}

/// The name of the operating mode a RegOpMode value holds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_mode_from_op_mode(op_mode: u8) -> &'static str {
    MODES
        .iter()
        .find(|(_, mode)| *mode == Sx127xMode::from_op_mode(op_mode))
        .map_or("Cad", |(name, _)| *name)
}

/// The LoRa modem registers for a link at a carrier, with a single reception timeout in
/// symbols.
///
/// Raises `PamojaError` when the SX127x cannot use the link at the carrier.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_modem(
    link: PyRef<'_, LoraLink>,
    frequency_hz: u32,
    symbol_timeout: u16,
) -> PyResult<Sx127xModem> {
    let settings = link.settings();
    let modulation = Sx127xModulation::from_link(&settings).map_err(modulation_error)?;
    if !modulation.bandwidth.in_band(frequency_hz) {
        return Err(modulation_error(ModulationError::Bandwidth(
            settings.bandwidth_hz(),
        )));
    }
    Ok(Sx127xModem {
        modem_config_1: modulation.modem_config_1(),
        modem_config_2: modulation.modem_config_2(symbol_timeout),
        modem_config_3: modulation.modem_config_3(),
        detection_optimize: modulation.detect_optimize(0),
        detection_threshold: modulation.detection_threshold(),
    })
}

/// The SX127x single reception timeout for a duration in microseconds, in the link's symbols.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_symbol_timeout(link: PyRef<'_, LoraLink>, timeout_us: u64) -> u16 {
    sx127x_config::symbol_timeout(&link.settings(), timeout_us)
}

/// The SX127x amplifier settings for an output power on an output named "Rfo" or "PaBoost".
///
/// Raises `ValueError` for another output name.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_tx_power(output: &str, output_dbm: i32) -> PyResult<Sx127xTxPower> {
    Ok(sx127x_power(Sx127xPower::for_output(
        output_named(output)?,
        dbm(output_dbm),
    )))
}

/// The SX127x amplifier settings that keep a link's EIRP at or under a ceiling in dBm.
///
/// Raises `ValueError` for an output name other than "Rfo" or "PaBoost".
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_tx_power_under_ceiling(
    output: &str,
    budget: PyRef<'_, LinkBudget>,
    eirp_ceiling_dbm: f64,
) -> PyResult<Sx127xTxPower> {
    Ok(sx127x_power(Sx127xPower::under_ceiling(
        output_named(output)?,
        &budget.budget(),
        decibels(eirp_ceiling_dbm),
    )))
}

/// RegOcp for a current limit in milliamps.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_ocp_register(milliamps: u16) -> u8 {
    sx127x_config::ocp_register(milliamps)
}

/// RegInvertIQ for the IQ polarity of each path.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_invert_iq(receive: bool, transmit: bool) -> u8 {
    sx127x_config::invert_iq(receive, transmit)
}

/// RegInvertIQ2 for the path in use.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_invert_iq_2(inverted: bool) -> u8 {
    sx127x_config::invert_iq_2(inverted)
}

/// The writes of the 500 kHz sensitivity erratum for a link at a carrier.
///
/// Raises `PamojaError` when the SX127x has no such bandwidth.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_high_bw_optimize(
    link: PyRef<'_, LoraLink>,
    frequency_hz: u32,
) -> PyResult<Sx127xHighBwOptimize> {
    let (optimize_1, optimize_2) =
        sx127x_config::high_bw_optimize(sx127x_bandwidth(&link)?, frequency_hz);
    Ok(Sx127xHighBwOptimize {
        optimize_1,
        optimize_2,
    })
}

/// The receive settings of the spurious reception erratum for a link.
///
/// Raises `PamojaError` when the SX127x has no such bandwidth.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_spurious_reception(link: PyRef<'_, LoraLink>) -> PyResult<Sx127xSpuriousReception> {
    let erratum = sx127x_config::spurious_reception(sx127x_bandwidth(&link)?);
    Ok(Sx127xSpuriousReception {
        automatic_if: erratum.automatic_if,
        if_freq_2: erratum.if_freq_2,
        offset_hz: erratum.offset_hz,
    })
}

/// RegImageCal with a calibration started and the automatic recalibration off.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_image_cal_start(current: u8) -> u8 {
    sx127x_config::image_cal_start(current)
}

/// RegDetectOptimize with AutomaticIFOn set or clear.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_automatic_if(current: u8, automatic_if: bool) -> u8 {
    sx127x_config::automatic_if(current, automatic_if)
}

/// Decodes RegPktSnrValue and RegPktRssiValue, read together, for a packet heard at a
/// carrier.
///
/// Raises `ValueError` when the answer is not two bytes.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_packet_status(answer: Vec<u8>, frequency_hz: u32) -> PyResult<Sx127xPacketStatus> {
    let bytes = exactly::<2>(&answer, "RegPktSnrValue and RegPktRssiValue")?;
    let status = DecodedPacketStatus::from_bytes(bytes, Port::for_frequency(frequency_hz));
    Ok(Sx127xPacketStatus {
        rssi_dbm: db(status.rssi_dbm),
        snr_db: db(status.snr_db),
        signal_rssi_dbm: db(status.signal_rssi_dbm),
    })
}

/// Decodes RegRssiValue for a receiver tuned to a carrier, in dBm.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_rssi_dbm(byte: u8, frequency_hz: u32) -> f64 {
    db(decoded_rssi_dbm(byte, Port::for_frequency(frequency_hz)))
}

/// Decodes RegModemStat.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sx127x_modem_status(byte: u8) -> Sx127xModemStatus {
    let status = DecodedModemStatus::from_byte(byte);
    Sx127xModemStatus {
        coding_rate_denominator: status.coding_rate_denominator,
        clear: status.clear,
        header_valid: status.header_valid,
        rx_ongoing: status.rx_ongoing,
        signal_synchronized: status.signal_synchronized,
        signal_detected: status.signal_detected,
    }
}

/// The operating modes by the names Python uses.
const MODES: [(&str, Sx127xMode); 8] = [
    ("Sleep", Sx127xMode::Sleep),
    ("Standby", Sx127xMode::Standby),
    ("FsTx", Sx127xMode::FsTx),
    ("Tx", Sx127xMode::Tx),
    ("FsRx", Sx127xMode::FsRx),
    ("RxContinuous", Sx127xMode::RxContinuous),
    ("RxSingle", Sx127xMode::RxSingle),
    ("Cad", Sx127xMode::Cad),
];

/// Finds the operating mode a name selects.
fn mode_named(name: &str) -> PyResult<Sx127xMode> {
    MODES
        .iter()
        .find(|(named, _)| *named == name)
        .map(|(_, mode)| *mode)
        .ok_or_else(|| PyValueError::new_err(format!("no SX127x mode is named {name}")))
}

/// Finds the amplifier output a name selects.
fn output_named(name: &str) -> PyResult<PaOutput> {
    match name {
        "Rfo" => Ok(PaOutput::Rfo),
        "PaBoost" => Ok(PaOutput::PaBoost),
        other => Err(PyValueError::new_err(format!(
            "no amplifier output is named {other}; use Rfo or PaBoost"
        ))),
    }
}

/// Flattens SX127x power settings into the object Python sees.
fn sx127x_power(power: Sx127xPower) -> Sx127xTxPower {
    Sx127xTxPower {
        pa_config: power.pa_config,
        pa_dac: power.pa_dac,
        ocp: power.ocp,
        output_dbm: power.output_dbm,
    }
}

/// Finds a link's SX127x bandwidth.
fn sx127x_bandwidth(link: &LoraLink) -> PyResult<Sx127xBandwidth> {
    let hz = link.settings().bandwidth_hz();
    Sx127xBandwidth::from_hz(hz)
        .ok_or_else(|| PamojaError::new_err(format!("the SX127x has no {hz} Hz LoRa bandwidth")))
}

/// Says why an SX127x cannot use a link.
fn modulation_error(error: ModulationError) -> PyErr {
    PamojaError::new_err(match error {
        ModulationError::Bandwidth(hz) => {
            format!("the SX127x has no {hz} Hz LoRa bandwidth at this carrier")
        }
        ModulationError::SpreadingFactor(sf) => format!("the SX127x has no SF{sf}"),
        ModulationError::ExplicitHeaderAtSf6 => "SF6 needs an implicit header".to_owned(),
    })
}
