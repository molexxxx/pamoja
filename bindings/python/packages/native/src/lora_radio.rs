//! Generated Python bindings for a LoRa radio on a Linux board, or on a simulated chip.
//!
//! These open an SX126x or SX127x module through the kernel's spidev and GPIO character
//! devices and drive either family with the same calls, as `pamoja_radios::linux` does. A
//! radio holds its device files open, so it is a class, closed with `close` or by leaving the
//! `with` block it opened in. Every call waits on the chip, a transmission for its airtime and
//! a reception for its timeout, and releases the interpreter lock while it waits, so other
//! threads run meanwhile.
//!
//! Only Linux has spidev and the GPIO character device. Everywhere else the class still
//! imports and opening a radio raises `PamojaError`.
//!
//! A `SimulatedLoraChip` stands in for a module on any platform. The radio it gives is the same
//! class with the same calls, while the program says what arrives on the air and reads back
//! what the chip was tuned to and what it sent.

use std::sync::{Mutex, PoisonError};

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use pamoja_lora::budget::Decibels;
use pamoja_radios::linux::{self, LinuxRadio, OpenError, Wiring};
use pamoja_radios::radio::{Family, RadioConfig, Reception, SignalLevels, SyncWord};
use pamoja_radios::sim::{Chip, Sent, SimRadio, Tuning};
use pamoja_radios::sx126x::config::{PowerAmplifier, TcxoVoltage};
use pamoja_radios::sx126x::DEFAULT_TCXO_SETTLE_US;
use pamoja_radios::sx127x::config::PaOutput;
use pamoja_radios::{sx126x, sx127x};

use crate::lora::LoraLink;
use crate::PamojaError;

/// The most bytes one LoRa frame carries.
const FRAME_MAX: usize = 255;

/// How a reception ended, with the frame and its signal levels when one arrived.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct LoraReception {
    /// How it ended: `"Frame"`, `"Timeout"`, or `"Corrupt"`.
    #[pyo3(get)]
    outcome: String,
    /// The payload of a frame, kept for the `payload` property.
    frame: Option<Vec<u8>>,
    /// The received signal strength averaged over the frame, in dBm, or `None` without a
    /// frame.
    #[pyo3(get)]
    rssi_dbm: Option<f64>,
    /// The estimated signal-to-noise ratio, in dB, or `None` without a frame.
    #[pyo3(get)]
    snr_db: Option<f64>,
    /// The estimated strength of the LoRa signal itself, in dBm, or `None` without a frame.
    #[pyo3(get)]
    signal_rssi_dbm: Option<f64>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LoraReception {
    /// The frame's payload, or `None` without a frame.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.frame.as_ref().map(|bytes| PyBytes::new(py, bytes))
    }
}

/// A LoRa radio opened on a Linux board or wired to a simulated chip.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct LoraRadio {
    inner: Mutex<Option<Held>>,
    family: &'static str,
}

/// The radio a `LoraRadio` drives: a module on a Linux board, or a simulated chip.
enum Held {
    Linux(LinuxRadio),
    Simulated(SimRadio),
}

/// Runs the same call on whichever radio is held, turning its error into a message.
macro_rules! on_radio {
    ($held:expr, $radio:ident => $call:expr) => {
        match $held {
            Held::Linux($radio) => $call.map_err(|error| error.to_string()),
            Held::Simulated($radio) => $call.map_err(|error| error.to_string()),
        }
    };
}

#[gen_stub_pymethods]
#[pymethods]
impl LoraRadio {
    /// Opens an SX1261, SX1262, SX1268, or LLCC68 module and resets it, which takes a few
    /// tens of milliseconds.
    ///
    /// Raises `PamojaError` when the platform is not Linux, a device cannot be opened, or no
    /// chip answers, and `ValueError` for an amplifier name or a TCXO voltage the chip has no
    /// setting for.
    #[staticmethod]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        spi,
        gpio_chip,
        busy_line,
        reset_line,
        amplifier,
        *,
        spi_hz=None,
        tcxo_volts=None,
        tcxo_settle_us=None,
        dio2_rf_switch=false,
        dc_dc=false,
        llcc68=false
    ))]
    fn open_sx126x(
        py: Python<'_>,
        spi: &str,
        gpio_chip: &str,
        busy_line: u32,
        reset_line: u32,
        amplifier: &str,
        spi_hz: Option<u32>,
        tcxo_volts: Option<f64>,
        tcxo_settle_us: Option<u32>,
        dio2_rf_switch: bool,
        dc_dc: bool,
        llcc68: bool,
    ) -> PyResult<LoraRadio> {
        let board = sx126x_board(
            amplifier,
            tcxo_volts,
            tcxo_settle_us,
            dio2_rf_switch,
            dc_dc,
            llcc68,
        )?;
        let wiring = wiring(spi, gpio_chip, reset_line, spi_hz).with_busy_line(busy_line);
        opened(py.detach(|| linux::open_sx126x(&wiring, board)))
    }

    /// Opens an SX1276, SX1277, SX1278, or SX1279 module, such as an RFM95W, and resets it
    /// into LoRa mode.
    ///
    /// Raises `PamojaError` when the platform is not Linux, a device cannot be opened, or no
    /// chip answers, and `ValueError` for an amplifier output name the chip has no setting
    /// for.
    #[staticmethod]
    #[pyo3(signature = (spi, gpio_chip, reset_line, output, *, spi_hz=None, tcxo=false))]
    fn open_sx127x(
        py: Python<'_>,
        spi: &str,
        gpio_chip: &str,
        reset_line: u32,
        output: &str,
        spi_hz: Option<u32>,
        tcxo: bool,
    ) -> PyResult<LoraRadio> {
        let board = sx127x_board(output, tcxo)?;
        let wiring = wiring(spi, gpio_chip, reset_line, spi_hz);
        opened(py.detach(|| linux::open_sx127x(&wiring, board)))
    }

    /// The family of the radio's chip: `"Sx126x"` or `"Sx127x"`.
    #[getter]
    fn family(&self) -> &'static str {
        self.family
    }

    /// Tunes the radio to a carrier, a link, and an output power.
    ///
    /// The sync word is one byte, `0x34` for a public network such as LoRaWAN and `0x12`, the
    /// default, for a private one. An SX126x calibrates its receiver for the carrier alone
    /// unless a band is given.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        frequency_hz,
        link,
        output_dbm,
        *,
        sync_word=None,
        band_low_hz=None,
        band_high_hz=None,
        invert_iq_transmit=false,
        invert_iq_receive=false
    ))]
    fn configure(
        &self,
        py: Python<'_>,
        frequency_hz: u32,
        link: PyRef<'_, LoraLink>,
        output_dbm: i32,
        sync_word: Option<u16>,
        band_low_hz: Option<u32>,
        band_high_hz: Option<u32>,
        invert_iq_transmit: bool,
        invert_iq_receive: bool,
    ) -> PyResult<()> {
        let sync_word = match sync_word {
            None => SyncWord::Private,
            Some(word) => SyncWord::from_byte(u8::try_from(word).map_err(|_| {
                PyValueError::new_err(format!("a sync word is one byte, not {word}"))
            })?),
        };
        let output_dbm = output_dbm.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8;
        let config = RadioConfig::new(frequency_hz, link.settings(), output_dbm)
            .with_sync_word(sync_word)
            .with_inverted_iq(invert_iq_transmit, invert_iq_receive);
        let config = match (band_low_hz, band_high_hz) {
            (Some(low_hz), Some(high_hz)) => config.with_band(low_hz, high_hz),
            _ => config,
        };
        self.drive(py, |held| on_radio!(held, radio => radio.configure(config)))
    }

    /// Sends one frame and returns its airtime in microseconds once it has left.
    fn transmit(&self, py: Python<'_>, payload: Vec<u8>) -> PyResult<u64> {
        self.drive(
            py,
            |held| on_radio!(held, radio => radio.transmit(&payload)),
        )
    }

    /// Listens for one frame for up to a timeout in microseconds.
    fn receive(&self, py: Python<'_>, timeout_us: u64) -> PyResult<LoraReception> {
        self.drive(py, |held| {
            let mut buffer = [0u8; FRAME_MAX];
            on_radio!(held, radio => radio.receive(&mut buffer, timeout_us))
                .map(|reception| heard(reception, &buffer))
        })
    }

    /// Starts listening, frame after frame, until another call changes the mode.
    fn listen(&self, py: Python<'_>) -> PyResult<()> {
        self.drive(py, |held| on_radio!(held, radio => radio.listen()))
    }

    /// Takes the frame a listening radio has received, or `None` when nothing has arrived.
    fn take_frame(&self, py: Python<'_>) -> PyResult<Option<LoraReception>> {
        self.drive(py, |held| {
            let mut buffer = [0u8; FRAME_MAX];
            on_radio!(held, radio => radio.take_frame(&mut buffer))
                .map(|taken| taken.map(|reception| heard(reception, &buffer)))
        })
    }

    /// Listens a few symbols for a LoRa preamble and returns whether one is there, as a relay's
    /// scan does, leaving the chip in standby. An SX126x listens over 1, 2, 4, 8, or 16 symbols,
    /// rounded down to one of them, and an SX127x over one.
    fn detect(&self, py: Python<'_>, symbols: u8) -> PyResult<bool> {
        self.drive(py, |held| on_radio!(held, radio => radio.detect(symbols)))
    }

    /// Puts the radio in standby, which stops a transmission or a reception.
    fn standby(&self, py: Python<'_>) -> PyResult<()> {
        self.drive(py, |held| on_radio!(held, radio => radio.standby()))
    }

    /// Puts the radio to sleep until the next call wakes it. An SX126x is configured again
    /// before its next frame; an SX127x keeps its registers.
    fn sleep(&self, py: Python<'_>) -> PyResult<()> {
        self.drive(py, |held| on_radio!(held, radio => radio.sleep()))
    }

    /// Draws a random number from the noise the receiver hears, leaving the chip in standby.
    ///
    /// Semtech's own drivers draw it the same way, and LoRaWAN 1.0.3 suggests this source
    /// for a join nonce on a device that has no other.
    fn random(&self, py: Python<'_>) -> PyResult<u32> {
        self.drive(py, |held| on_radio!(held, radio => radio.random()))
    }

    /// Reads one register: a 16-bit address on the SX126x, 0x00 to 0x7F on the SX127x.
    fn read_register(&self, py: Python<'_>, address: u16) -> PyResult<u8> {
        self.drive(
            py,
            |held| on_radio!(held, radio => radio.read_register(address)),
        )
    }

    /// Writes one register: a 16-bit address on the SX126x, 0x00 to 0x7F on the SX127x.
    fn write_register(&self, py: Python<'_>, address: u16, value: u8) -> PyResult<()> {
        self.drive(
            py,
            |held| on_radio!(held, radio => radio.write_register(address, value)),
        )
    }

    /// Closes the radio's device files, after any call in progress finishes. Calls after this
    /// raise `PamojaError`.
    fn close(&self) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
    }

    /// Returns the radio, so it can be opened in a `with` block.
    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Closes the radio at the end of a `with` block, letting any exception through.
    #[pyo3(signature = (exception_type=None, exception=None, traceback=None))]
    fn __exit__(
        &self,
        exception_type: Option<Py<PyAny>>,
        exception: Option<Py<PyAny>>,
        traceback: Option<Py<PyAny>>,
    ) -> bool {
        let _ = (exception_type, exception, traceback);
        self.close();
        false
    }
}

impl LoraRadio {
    /// Runs a call on the radio with the interpreter lock released.
    fn drive<T: Send>(
        &self,
        py: Python<'_>,
        call: impl FnOnce(&mut Held) -> Result<T, String> + Send,
    ) -> PyResult<T> {
        py.detach(|| {
            let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            match guard.as_mut() {
                Some(radio) => call(radio),
                None => Err("the radio is closed".to_owned()),
            }
        })
        .map_err(PamojaError::new_err)
    }
}

/// Wraps an opened radio, or turns why it did not open into an exception.
fn opened(result: Result<LinuxRadio, OpenError>) -> PyResult<LoraRadio> {
    let radio = result.map_err(|error| PamojaError::new_err(error.to_string()))?;
    Ok(LoraRadio {
        family: family_name(radio.family()),
        inner: Mutex::new(Some(Held::Linux(radio))),
    })
}

/// What a simulated chip is tuned to.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct LoraTuning {
    /// The carrier frequency in hertz, as the chip's synthesizer steps it: within a hertz of
    /// the one asked for on an SX126x, and within 61 Hz on an SX127x.
    #[pyo3(get)]
    frequency_hz: u32,
    /// The link settings, kept for the `link` property.
    settings: pamoja_lora::LinkSettings,
    /// The output power the amplifier was asked for, in dBm.
    #[pyo3(get)]
    output_dbm: i8,
    /// The sync word byte.
    #[pyo3(get)]
    sync_word: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl LoraTuning {
    /// The spreading factor, bandwidth, coding rate, preamble, header, and CRC.
    #[getter]
    fn link(&self) -> LoraLink {
        LoraLink::from_settings(self.settings)
    }
}

/// A frame a simulated chip put on the air.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct LoraSentFrame {
    /// What the chip was tuned to when the frame went out.
    #[pyo3(get)]
    tuning: Py<LoraTuning>,
    /// The payload, kept for the `payload` property.
    frame: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LoraSentFrame {
    /// The frame's payload.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.frame)
    }
}

/// A simulated SX126x or SX127x, which a `LoraRadio` drives with no radio attached.
///
/// The program says what arrives on the air with `hear`, and reads back what the chip was
/// tuned to and what it sent with `tuning` and `sent`. Nothing is timed: a transmission is
/// done as soon as it starts, and a reception with a timeout ends at once when nothing is
/// waiting.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct SimulatedLoraChip {
    chip: Chip,
}

#[gen_stub_pymethods]
#[pymethods]
impl SimulatedLoraChip {
    /// A simulated SX1261, SX1262, SX1268, or LLCC68, out of reset.
    ///
    /// Raises `ValueError` for an amplifier name or a TCXO voltage the chip has no setting
    /// for.
    #[staticmethod]
    #[pyo3(signature = (
        amplifier,
        *,
        tcxo_volts=None,
        tcxo_settle_us=None,
        dio2_rf_switch=false,
        dc_dc=false,
        llcc68=false
    ))]
    fn sx126x(
        amplifier: &str,
        tcxo_volts: Option<f64>,
        tcxo_settle_us: Option<u32>,
        dio2_rf_switch: bool,
        dc_dc: bool,
        llcc68: bool,
    ) -> PyResult<SimulatedLoraChip> {
        let board = sx126x_board(
            amplifier,
            tcxo_volts,
            tcxo_settle_us,
            dio2_rf_switch,
            dc_dc,
            llcc68,
        )?;
        Ok(SimulatedLoraChip {
            chip: Chip::sx126x(board),
        })
    }

    /// A simulated SX1276, SX1277, SX1278, or SX1279, out of reset.
    ///
    /// Raises `ValueError` for an amplifier output name the chip has no setting for.
    #[staticmethod]
    #[pyo3(signature = (output, *, tcxo=false))]
    fn sx127x(output: &str, tcxo: bool) -> PyResult<SimulatedLoraChip> {
        Ok(SimulatedLoraChip {
            chip: Chip::sx127x(sx127x_board(output, tcxo)?),
        })
    }

    /// The family of the chip: `"Sx126x"` or `"Sx127x"`.
    #[getter]
    fn family(&self) -> &'static str {
        family_name(self.chip.family())
    }

    /// Returns a radio wired to the chip, reset as opening a module resets it.
    fn radio(&self) -> PyResult<LoraRadio> {
        let mut radio = self.chip.radio();
        radio
            .init()
            .map_err(|error| PamojaError::new_err(error.to_string()))?;
        Ok(LoraRadio {
            family: family_name(radio.family()),
            inner: Mutex::new(Some(Held::Simulated(radio))),
        })
    }

    /// Puts a frame on the air for the chip to receive the next time it listens, heard at a
    /// strength in dBm and a signal-to-noise ratio in dB. A payload past 255 bytes is cut to
    /// 255.
    ///
    /// Raises `ValueError` for a level that is not a finite number.
    fn hear(&self, payload: Vec<u8>, rssi_dbm: f64, snr_db: f64) -> PyResult<()> {
        let rssi = level(rssi_dbm, "rssi_dbm")?;
        let snr = level(snr_db, "snr_db")?;
        self.chip.hear(&payload, rssi, snr);
        Ok(())
    }

    /// Puts a frame on the air whose CRC fails, which the chip reports and drops.
    ///
    /// Raises `ValueError` for a level that is not a finite number.
    fn hear_corrupt(&self, rssi_dbm: f64, snr_db: f64) -> PyResult<()> {
        let rssi = level(rssi_dbm, "rssi_dbm")?;
        let snr = level(snr_db, "snr_db")?;
        self.chip.hear_corrupt(rssi, snr);
        Ok(())
    }

    /// How many frames wait on the air for the chip to receive.
    #[getter]
    fn waiting(&self) -> usize {
        self.chip.waiting()
    }

    /// Returns what the chip is tuned to now.
    fn tuning(&self) -> LoraTuning {
        tuning_of(self.chip.tuning())
    }

    /// Returns every frame the chip has put on the air, oldest first.
    fn sent(&self, py: Python<'_>) -> PyResult<Vec<LoraSentFrame>> {
        self.chip
            .sent()
            .into_iter()
            .map(|sent: Sent| {
                Ok(LoraSentFrame {
                    tuning: Py::new(py, tuning_of(sent.tuning))?,
                    frame: sent.payload,
                })
            })
            .collect()
    }
}

/// Reads a level Python passes, refusing one that is not a finite number.
fn level(value: f64, name: &str) -> PyResult<Decibels> {
    if value.is_finite() {
        Ok(Decibels::from_hundredths((value * 100.0).round() as i32))
    } else {
        Err(PyValueError::new_err(format!(
            "{name} must be a finite number of decibels, not {value}"
        )))
    }
}

/// Describes a tuning the way Python holds it.
fn tuning_of(tuning: Tuning) -> LoraTuning {
    LoraTuning {
        frequency_hz: tuning.frequency_hz,
        settings: tuning.link,
        output_dbm: tuning.output_dbm,
        sync_word: tuning.sync_word.to_byte(),
    }
}

/// Names a chip's family for Python.
fn family_name(family: Family) -> &'static str {
    match family {
        Family::Sx126x => "Sx126x",
        Family::Sx127x => "Sx127x",
    }
}

/// Reads an SX126x board description from the arguments Python passes.
fn sx126x_board(
    amplifier: &str,
    tcxo_volts: Option<f64>,
    tcxo_settle_us: Option<u32>,
    dio2_rf_switch: bool,
    dc_dc: bool,
    llcc68: bool,
) -> PyResult<sx126x::Board> {
    let mut board = sx126x::Board::new(named_amplifier(amplifier)?);
    if let Some(volts) = tcxo_volts {
        board = board.with_tcxo(
            tcxo_voltage(volts)?,
            tcxo_settle_us.unwrap_or(DEFAULT_TCXO_SETTLE_US),
        );
    }
    if dio2_rf_switch {
        board = board.with_dio2_rf_switch();
    }
    if dc_dc {
        board = board.with_dc_dc();
    }
    if llcc68 {
        board = board.with_llcc68();
    }
    Ok(board)
}

/// Reads an SX127x board description from the arguments Python passes.
fn sx127x_board(output: &str, tcxo: bool) -> PyResult<sx127x::Board> {
    let board = sx127x::Board::new(named_output(output)?);
    Ok(if tcxo { board.with_tcxo() } else { board })
}
/// Reads the wiring Python passes.
fn wiring(spi: &str, gpio_chip: &str, reset_line: u32, spi_hz: Option<u32>) -> Wiring {
    let wiring = Wiring::new(spi, gpio_chip, reset_line);
    match spi_hz {
        Some(hz) => wiring.with_spi_hz(hz),
        None => wiring,
    }
}

/// Names the amplifier a Python string selects.
fn named_amplifier(name: &str) -> PyResult<PowerAmplifier> {
    match name {
        "LowPower" => Ok(PowerAmplifier::LowPower),
        "HighPower" => Ok(PowerAmplifier::HighPower),
        other => Err(PyValueError::new_err(format!(
            "no amplifier is named {other}; use LowPower or HighPower"
        ))),
    }
}

/// Names the amplifier output a Python string selects.
fn named_output(name: &str) -> PyResult<PaOutput> {
    match name {
        "Rfo" => Ok(PaOutput::Rfo),
        "PaBoost" => Ok(PaOutput::PaBoost),
        other => Err(PyValueError::new_err(format!(
            "no amplifier output is named {other}; use Rfo or PaBoost"
        ))),
    }
}

/// Names the TCXO voltage a number of volts selects.
fn tcxo_voltage(volts: f64) -> PyResult<TcxoVoltage> {
    let millivolts = (volts * 1000.0).round();
    let voltage = if (0.0..=f64::from(u16::MAX)).contains(&millivolts) {
        TcxoVoltage::from_millivolts(millivolts as u16)
    } else {
        None
    };
    voltage.ok_or_else(|| {
        PyValueError::new_err(format!(
            "DIO3 cannot supply a TCXO with {volts} V; use 1.6, 1.7, 1.8, 2.2, 2.4, 2.7, 3.0, or 3.3"
        ))
    })
}

/// Flattens how a reception ended, copying the payload out of the buffer.
fn heard(reception: Reception, buffer: &[u8]) -> LoraReception {
    match reception {
        Reception::Frame { len, levels } => {
            levelled("Frame", Some(buffer[..len].to_vec()), Some(levels))
        }
        Reception::Timeout => levelled("Timeout", None, None),
        Reception::Corrupt => levelled("Corrupt", None, None),
    }
}

/// Builds a reception from its outcome, payload, and levels.
fn levelled(outcome: &str, frame: Option<Vec<u8>>, levels: Option<SignalLevels>) -> LoraReception {
    LoraReception {
        outcome: outcome.to_owned(),
        frame,
        rssi_dbm: levels.map(|levels| db(levels.rssi_dbm)),
        snr_db: levels.map(|levels| db(levels.snr_db)),
        signal_rssi_dbm: levels.map(|levels| db(levels.signal_rssi_dbm)),
    }
}

/// Returns hundredths of a decibel as a number of decibels.
fn db(value: Decibels) -> f64 {
    f64::from(value.hundredths()) / 100.0
}
