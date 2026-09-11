//! Generated Python bindings for LoRa radio chips.
//!
//! These mirror the `pamoja-radios` Rust API: the bytes of each Semtech SX126x command, the
//! chip's answers decoded, the amplifier settings a regional power ceiling allows, and the
//! silence a duty-cycle limit forces after each transmission.
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
