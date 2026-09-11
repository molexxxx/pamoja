//! Generated Node bindings for a LoRa radio on a Linux board.
//!
//! These open an SX126x or SX127x module through the kernel's spidev and GPIO character
//! devices and drive either family with the same calls, as `pamoja_radios::linux` does. A
//! radio holds its device files open, so it is a class, closed with `close`. Every call after
//! opening waits on the chip, a transmission for its airtime and a reception for its timeout,
//! so each runs on a blocking worker thread and resolves a promise, leaving the event loop
//! free. Opening resets the chip, which takes a few tens of milliseconds, and returns at once.
//!
//! Only Linux has spidev and the GPIO character device. On every other platform the class
//! still loads, and opening a radio throws an error saying so.

use std::sync::{Arc, Mutex, PoisonError};

use napi::bindgen_prelude::{spawn_blocking, Buffer};
use napi_derive::napi;
use pamoja_radios::linux::{self, LinuxRadio, OpenError, Wiring};
use pamoja_radios::radio::{Family, RadioConfig, Reception, SignalLevels, SyncWord};
use pamoja_radios::sx126x::config::{PowerAmplifier, TcxoVoltage};
use pamoja_radios::sx126x::DEFAULT_TCXO_SETTLE_US;
use pamoja_radios::sx127x::config::PaOutput;
use pamoja_radios::{sx126x, sx127x};

use crate::lora::{db, settings, LoraLink};
use crate::radios::{Sx126xAmplifier, Sx127xPaOutput};

/// The most bytes one LoRa frame carries.
const FRAME_MAX: usize = 255;

/// Which family a radio's chip belongs to.
#[napi(string_enum, js_name = "LoraRadioFamily")]
pub enum LoraRadioFamily {
    /// The SX1261, SX1262, SX1268, and LLCC68.
    Sx126x,
    /// The SX1276, SX1277, SX1278, and SX1279.
    Sx127x,
}

/// How a reception ended.
#[napi(string_enum, js_name = "LoraReceptionOutcome")]
pub enum LoraReceptionOutcome {
    /// A frame arrived and checked.
    Frame,
    /// No frame arrived before the timeout.
    Timeout,
    /// A frame arrived whose header or CRC failed its check, and was dropped.
    Corrupt,
}

/// Where a radio module is wired on a Linux board.
#[napi(object, js_name = "LoraRadioWiring")]
pub struct LoraRadioWiring {
    /// The SPI device file, one per chip select, such as `/dev/spidev0.0`.
    pub spi: String,
    /// The GPIO chip the lines are on, `/dev/gpiochip0` for a Raspberry Pi's header.
    pub gpio_chip: String,
    /// The line the module's reset pin is on, the BCM GPIO number on a Raspberry Pi.
    pub reset_line: u32,
    /// The line an SX126x's BUSY pin is on. The SX127x has no BUSY pin.
    pub busy_line: Option<u32>,
    /// The SPI clock in hertz, 2 MHz when omitted.
    pub spi_hz: Option<u32>,
}

/// How a module wires its SX126x, from its schematic or its maker's example code.
#[napi(object, js_name = "Sx126xBoard")]
pub struct Sx126xBoard {
    /// The chip's power amplifier: `HighPower` for the SX1262, SX1268, and LLCC68.
    pub amplifier: Sx126xAmplifier,
    /// The voltage DIO3 supplies a TCXO with, such as `1.7`, when a TCXO clocks the chip.
    pub tcxo_volts: Option<f64>,
    /// How long the TCXO takes to settle, in microseconds, 5 ms when omitted.
    pub tcxo_settle_us: Option<u32>,
    /// Whether DIO2 drives the antenna switch.
    pub dio2_rf_switch: Option<bool>,
    /// Whether the module fits the inductor the DC-DC regulator needs.
    pub dc_dc: Option<bool>,
    /// Whether the chip is an LLCC68, which is held to the rates it supports.
    pub llcc68: Option<bool>,
}

/// How a module wires its SX127x.
#[napi(object, js_name = "Sx127xBoard")]
pub struct Sx127xBoard {
    /// The amplifier output the antenna is on: `PaBoost` on an RFM95W.
    pub output: Sx127xPaOutput,
    /// Whether a TCXO drives the XTA pin instead of a crystal.
    pub tcxo: Option<bool>,
}

/// What a radio sends and listens with.
#[napi(object, js_name = "LoraRadioConfig")]
pub struct LoraRadioConfig {
    /// The carrier frequency in hertz.
    pub frequency_hz: u32,
    /// The spreading factor, bandwidth, coding rate, preamble, header, and CRC.
    pub link: LoraLink,
    /// The output power asked of the amplifier, in dBm, clamped to its range.
    pub output_dbm: i32,
    /// The sync word byte: `0x34` for a public network such as LoRaWAN, and `0x12`, the
    /// default, for a private one.
    pub sync_word: Option<u32>,
    /// The lower edge of the band an SX126x calibrates its receiver for, in hertz, with
    /// `bandHighHz`; the carrier alone when omitted.
    pub band_low_hz: Option<u32>,
    /// The upper edge of that band in hertz.
    pub band_high_hz: Option<u32>,
    /// Whether frames go out with inverted IQ, as a LoRaWAN gateway sends downlinks.
    pub invert_iq_transmit: Option<bool>,
    /// Whether frames are expected with inverted IQ, as a LoRaWAN device hears downlinks.
    pub invert_iq_receive: Option<bool>,
}

/// How a reception ended, with the frame and its signal levels when one arrived.
#[napi(object, js_name = "LoraReception")]
pub struct LoraReception {
    /// How the reception ended.
    pub outcome: LoraReceptionOutcome,
    /// The frame's payload, or `null` without a frame.
    pub payload: Option<Buffer>,
    /// The received signal strength averaged over the frame, in dBm.
    pub rssi_dbm: Option<f64>,
    /// The estimated signal-to-noise ratio, in dB.
    pub snr_db: Option<f64>,
    /// The estimated strength of the LoRa signal itself, in dBm.
    pub signal_rssi_dbm: Option<f64>,
}

/// A LoRa radio opened on a Linux board.
#[napi(js_name = "LoraRadio")]
pub struct LoraRadio {
    inner: Arc<Mutex<Option<LinuxRadio>>>,
    family: Family,
}

#[napi]
impl LoraRadio {
    /// Opens an SX1261, SX1262, SX1268, or LLCC68 module and resets it.
    ///
    /// Throws when the platform is not Linux, a device cannot be opened, the wiring names no
    /// BUSY line, or no chip answers.
    #[napi(factory, js_name = "openSx126x")]
    pub fn open_sx126x(wiring: LoraRadioWiring, board: Sx126xBoard) -> napi::Result<Self> {
        let chip = sx126x_board(&board)?;
        opened(linux::open_sx126x(&wiring_of(&wiring), chip))
    }

    /// Opens an SX1276, SX1277, SX1278, or SX1279 module, such as an RFM95W, and resets it
    /// into LoRa mode.
    ///
    /// Throws when the platform is not Linux, a device cannot be opened, or no chip answers.
    #[napi(factory, js_name = "openSx127x")]
    pub fn open_sx127x(wiring: LoraRadioWiring, board: Sx127xBoard) -> napi::Result<Self> {
        let output = match board.output {
            Sx127xPaOutput::Rfo => PaOutput::Rfo,
            Sx127xPaOutput::PaBoost => PaOutput::PaBoost,
        };
        let mut chip = sx127x::Board::new(output);
        if board.tcxo.unwrap_or(false) {
            chip = chip.with_tcxo();
        }
        opened(linux::open_sx127x(&wiring_of(&wiring), chip))
    }

    /// The family of the radio's chip.
    #[napi(getter)]
    pub fn family(&self) -> LoraRadioFamily {
        match self.family {
            Family::Sx126x => LoraRadioFamily::Sx126x,
            Family::Sx127x => LoraRadioFamily::Sx127x,
        }
    }

    /// Tunes the radio to a configuration.
    #[napi]
    pub async fn configure(&self, config: LoraRadioConfig) -> napi::Result<()> {
        let radio_config = config_of(&config)?;
        drive(&self.inner, move |radio| {
            radio
                .configure(radio_config)
                .map_err(|error| error.to_string())
        })
        .await
    }

    /// Sends one frame and resolves with its airtime in microseconds once it has left.
    #[napi]
    pub async fn transmit(&self, payload: Buffer) -> napi::Result<f64> {
        let payload = payload.to_vec();
        let airtime_us = drive(&self.inner, move |radio| {
            radio.transmit(&payload).map_err(|error| error.to_string())
        })
        .await?;
        Ok(airtime_us as f64)
    }

    /// Listens for one frame for up to a timeout in microseconds.
    #[napi]
    pub async fn receive(&self, timeout_us: f64) -> napi::Result<LoraReception> {
        let timeout_us = micros(timeout_us)?;
        let heard = drive(&self.inner, move |radio| {
            let mut buffer = [0u8; FRAME_MAX];
            radio
                .receive(&mut buffer, timeout_us)
                .map(|reception| heard(reception, &buffer))
                .map_err(|error| error.to_string())
        })
        .await?;
        Ok(heard.into())
    }

    /// Starts listening, frame after frame, until another call changes the mode.
    #[napi]
    pub async fn listen(&self) -> napi::Result<()> {
        drive(&self.inner, |radio| {
            radio.listen().map_err(|error| error.to_string())
        })
        .await
    }

    /// Takes the frame a listening radio has received, or resolves `null` when nothing has
    /// arrived.
    #[napi(js_name = "takeFrame")]
    pub async fn take_frame(&self) -> napi::Result<Option<LoraReception>> {
        let taken = drive(&self.inner, |radio| {
            let mut buffer = [0u8; FRAME_MAX];
            radio
                .take_frame(&mut buffer)
                .map(|taken| taken.map(|reception| heard(reception, &buffer)))
                .map_err(|error| error.to_string())
        })
        .await?;
        Ok(taken.map(LoraReception::from))
    }

    /// Puts the radio in standby, which stops a transmission or a reception.
    #[napi]
    pub async fn standby(&self) -> napi::Result<()> {
        drive(&self.inner, |radio| {
            radio.standby().map_err(|error| error.to_string())
        })
        .await
    }

    /// Puts the radio to sleep until the next call wakes it. An SX126x is configured again
    /// before its next frame; an SX127x keeps its registers.
    #[napi]
    pub async fn sleep(&self) -> napi::Result<()> {
        drive(&self.inner, |radio| {
            radio.sleep().map_err(|error| error.to_string())
        })
        .await
    }

    /// Reads one register: a 16-bit address on the SX126x, 0x00 to 0x7F on the SX127x.
    #[napi(js_name = "readRegister")]
    pub async fn read_register(&self, address: u32) -> napi::Result<u32> {
        let address = register_address(address)?;
        let value = drive(&self.inner, move |radio| {
            radio
                .read_register(address)
                .map_err(|error| error.to_string())
        })
        .await?;
        Ok(u32::from(value))
    }

    /// Writes one register: a 16-bit address on the SX126x, 0x00 to 0x7F on the SX127x.
    #[napi(js_name = "writeRegister")]
    pub async fn write_register(&self, address: u32, value: u32) -> napi::Result<()> {
        let address = register_address(address)?;
        let value = u8::try_from(value).map_err(|_| {
            napi::Error::from_reason(format!("a register holds one byte, not {value}"))
        })?;
        drive(&self.inner, move |radio| {
            radio
                .write_register(address, value)
                .map_err(|error| error.to_string())
        })
        .await
    }

    /// Closes the radio's device files, after any call in progress finishes. Calls after this
    /// reject.
    #[napi]
    pub fn close(&self) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
    }
}

/// What a reception produced, carried back from the worker thread.
struct Heard {
    outcome: LoraReceptionOutcome,
    payload: Option<Vec<u8>>,
    levels: Option<SignalLevels>,
}

impl From<Heard> for LoraReception {
    fn from(heard: Heard) -> LoraReception {
        LoraReception {
            outcome: heard.outcome,
            payload: heard.payload.map(Buffer::from),
            rssi_dbm: heard.levels.map(|levels| db(levels.rssi_dbm)),
            snr_db: heard.levels.map(|levels| db(levels.snr_db)),
            signal_rssi_dbm: heard.levels.map(|levels| db(levels.signal_rssi_dbm)),
        }
    }
}

/// Wraps an opened radio, or turns why it did not open into an error.
fn opened(result: Result<LinuxRadio, OpenError>) -> napi::Result<LoraRadio> {
    let radio = result.map_err(|error| napi::Error::from_reason(error.to_string()))?;
    Ok(LoraRadio {
        family: radio.family(),
        inner: Arc::new(Mutex::new(Some(radio))),
    })
}

/// Runs a call on the radio on a blocking worker thread.
async fn drive<T: Send + 'static>(
    inner: &Arc<Mutex<Option<LinuxRadio>>>,
    call: impl FnOnce(&mut LinuxRadio) -> Result<T, String> + Send + 'static,
) -> napi::Result<T> {
    let inner = Arc::clone(inner);
    let outcome = spawn_blocking(move || {
        let mut guard = inner.lock().unwrap_or_else(PoisonError::into_inner);
        match guard.as_mut() {
            Some(radio) => call(radio),
            None => Err("the radio is closed".to_owned()),
        }
    })
    .await
    .map_err(|error| napi::Error::from_reason(format!("the radio call did not finish: {error}")))?;
    outcome.map_err(napi::Error::from_reason)
}

/// Reads the wiring JavaScript passes.
fn wiring_of(wiring: &LoraRadioWiring) -> Wiring {
    let mut radio_wiring = Wiring::new(&wiring.spi, &wiring.gpio_chip, wiring.reset_line);
    if let Some(line) = wiring.busy_line {
        radio_wiring = radio_wiring.with_busy_line(line);
    }
    if let Some(hz) = wiring.spi_hz {
        radio_wiring = radio_wiring.with_spi_hz(hz);
    }
    radio_wiring
}

/// Reads an SX126x board description, refusing a TCXO voltage DIO3 cannot supply.
fn sx126x_board(board: &Sx126xBoard) -> napi::Result<sx126x::Board> {
    let amplifier = match board.amplifier {
        Sx126xAmplifier::LowPower => PowerAmplifier::LowPower,
        Sx126xAmplifier::HighPower => PowerAmplifier::HighPower,
    };
    let mut chip = sx126x::Board::new(amplifier);
    if let Some(volts) = board.tcxo_volts {
        let voltage = tcxo_voltage(volts).ok_or_else(|| {
            napi::Error::from_reason(format!(
                "DIO3 cannot supply a TCXO with {volts} V; use 1.6, 1.7, 1.8, 2.2, 2.4, 2.7, 3.0, or 3.3"
            ))
        })?;
        chip = chip.with_tcxo(
            voltage,
            board.tcxo_settle_us.unwrap_or(DEFAULT_TCXO_SETTLE_US),
        );
    }
    if board.dio2_rf_switch.unwrap_or(false) {
        chip = chip.with_dio2_rf_switch();
    }
    if board.dc_dc.unwrap_or(false) {
        chip = chip.with_dc_dc();
    }
    if board.llcc68.unwrap_or(false) {
        chip = chip.with_llcc68();
    }
    Ok(chip)
}

/// Names the TCXO voltage a number of volts selects.
fn tcxo_voltage(volts: f64) -> Option<TcxoVoltage> {
    let millivolts = (volts * 1000.0).round();
    if (0.0..=f64::from(u16::MAX)).contains(&millivolts) {
        TcxoVoltage::from_millivolts(millivolts as u16)
    } else {
        None
    }
}

/// Reads a configuration JavaScript passes.
fn config_of(config: &LoraRadioConfig) -> napi::Result<RadioConfig> {
    let sync_word = match config.sync_word {
        None => SyncWord::Private,
        Some(byte) => SyncWord::from_byte(u8::try_from(byte).map_err(|_| {
            napi::Error::from_reason(format!("a sync word is one byte, not {byte}"))
        })?),
    };
    let output_dbm = config
        .output_dbm
        .clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8;
    let radio_config = RadioConfig::new(config.frequency_hz, settings(&config.link), output_dbm)
        .with_sync_word(sync_word)
        .with_inverted_iq(
            config.invert_iq_transmit.unwrap_or(false),
            config.invert_iq_receive.unwrap_or(false),
        );
    Ok(match (config.band_low_hz, config.band_high_hz) {
        (Some(low_hz), Some(high_hz)) => radio_config.with_band(low_hz, high_hz),
        _ => radio_config,
    })
}

/// Flattens how a reception ended, copying the payload out of the buffer.
fn heard(reception: Reception, buffer: &[u8]) -> Heard {
    match reception {
        Reception::Frame { len, levels } => Heard {
            outcome: LoraReceptionOutcome::Frame,
            payload: Some(buffer[..len].to_vec()),
            levels: Some(levels),
        },
        Reception::Timeout => Heard {
            outcome: LoraReceptionOutcome::Timeout,
            payload: None,
            levels: None,
        },
        Reception::Corrupt => Heard {
            outcome: LoraReceptionOutcome::Corrupt,
            payload: None,
            levels: None,
        },
    }
}

/// Reads a register address, which is at most 16 bits.
fn register_address(address: u32) -> napi::Result<u16> {
    u16::try_from(address).map_err(|_| {
        napi::Error::from_reason(format!("a register address is 16 bits, not {address}"))
    })
}

/// Reads a microsecond count JavaScript passes as a number.
fn micros(value: f64) -> napi::Result<u64> {
    if value.is_finite() && value >= 0.0 {
        Ok(value as u64)
    } else {
        Err(napi::Error::from_reason(format!(
            "a timeout is a non-negative number of microseconds, not {value}"
        )))
    }
}
