//! The C ABI for a LoRa radio on a Linux board.
//!
//! These functions open an SX126x or SX127x module through the kernel's spidev and GPIO
//! character devices, as [`pamoja_radios::linux`] does, and drive either family with the same
//! calls: configure, transmit, receive, listen, standby, sleep, and a register read and write.
//! A radio holds its device files open between calls, so it crosses the boundary as a handle
//! the caller releases with [`pamoja_lora_radio_free`].
//!
//! Only Linux has those devices. On every other platform the header and the functions are the
//! same, and opening a radio returns [`PamojaStatus::Unsupported`] with a message saying why.
//!
//! Each call blocks until the chip answers: a transmission for its airtime, a reception for
//! its timeout. A caller with an event loop makes these calls from a worker thread, and a
//! handle is used by one thread at a time.

use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use pamoja_radios::linux::{self, LinuxRadio, OpenError, SpiError, Wiring};
use pamoja_radios::radio::{Family, RadioConfig, RadioError, Reception, SyncWord};
use pamoja_radios::sx126x::config::{PowerAmplifier, TcxoVoltage};
use pamoja_radios::sx127x::config::PaOutput;
use pamoja_radios::{sx126x, sx127x};

use crate::lora::{settings, PamojaLoraLink};
use crate::{read_str, set_last_error, PamojaStatus};

/// The family of an SX1261, SX1262, SX1268, or LLCC68.
pub const PAMOJA_LORA_RADIO_SX126X: u8 = 0;

/// The family of an SX1276, SX1277, SX1278, or SX1279.
pub const PAMOJA_LORA_RADIO_SX127X: u8 = 1;

/// A reception outcome: a frame arrived and checked.
pub const PAMOJA_LORA_RADIO_FRAME: u8 = 0;

/// A reception outcome: no frame arrived before the timeout.
pub const PAMOJA_LORA_RADIO_TIMEOUT: u8 = 1;

/// A reception outcome: a frame arrived whose header or CRC failed its check, and was dropped.
pub const PAMOJA_LORA_RADIO_CORRUPT: u8 = 2;

/// A reception outcome: nothing has arrived since the last frame was taken.
pub const PAMOJA_LORA_RADIO_NOTHING: u8 = 3;

/// The sync word of a public network such as LoRaWAN.
pub const PAMOJA_LORA_RADIO_SYNC_WORD_PUBLIC: u8 = 0x34;

/// The sync word of a private network, and both families' reset value.
pub const PAMOJA_LORA_RADIO_SYNC_WORD_PRIVATE: u8 = 0x12;

/// The SPI clock a radio opens at when the call passes 0, in hertz.
pub const PAMOJA_LORA_RADIO_DEFAULT_SPI_HZ: u32 = 2_000_000;

const _: () = assert!(PAMOJA_LORA_RADIO_DEFAULT_SPI_HZ == linux::DEFAULT_SPI_HZ);
const _: () = assert!(PAMOJA_LORA_RADIO_SYNC_WORD_PUBLIC == SyncWord::Public.to_byte());
const _: () = assert!(PAMOJA_LORA_RADIO_SYNC_WORD_PRIVATE == SyncWord::Private.to_byte());

/// A LoRa radio opened on a Linux board, released with [`pamoja_lora_radio_free`].
pub struct PamojaLoraRadio {
    radio: LinuxRadio,
}

/// How a module wires its SX126x.
///
/// The SPI interface cannot see the parts around the chip, so these come from the module's
/// schematic or its maker's example code.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PamojaSx126xBoard {
    /// `true` for the high power amplifier of the SX1262, SX1268, and LLCC68, `false` for the
    /// SX1261's low power amplifier.
    pub high_power: bool,
    /// `true` when a TCXO powered from DIO3 clocks the chip instead of a crystal.
    pub tcxo: bool,
    /// The TCXO supply voltage as SetDIO3AsTCXOCtrl takes it: 0 for 1.6 V, 1 for 1.7 V, 2 for
    /// 1.8 V, 3 for 2.2 V, 4 for 2.4 V, 5 for 2.7 V, 6 for 3.0 V, and 7 for 3.3 V.
    pub tcxo_voltage: u8,
    /// `true` when DIO2 drives the antenna switch.
    pub dio2_rf_switch: bool,
    /// `true` when the module fits the inductor the DC-DC regulator needs.
    pub dc_dc: bool,
    /// `true` for an LLCC68, which is held to the rates it supports.
    pub llcc68: bool,
    /// How long the TCXO takes to settle, in microseconds.
    pub tcxo_settle_us: u32,
}

/// What a radio of either family sends and listens with.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraRadioConfig {
    /// The carrier frequency in hertz.
    pub frequency_hz: u32,
    /// The lower edge of the band an SX126x calibrates its receiver for, in hertz, or 0 with
    /// `band_high_hz` 0 to calibrate for the carrier alone.
    pub band_low_hz: u32,
    /// The upper edge of that band in hertz.
    pub band_high_hz: u32,
    /// The spreading factor, bandwidth, coding rate, preamble, header, and CRC.
    pub link: PamojaLoraLink,
    /// The output power asked of the amplifier, in dBm, clamped to its range.
    pub output_dbm: i8,
    /// The sync word byte: [`PAMOJA_LORA_RADIO_SYNC_WORD_PUBLIC`],
    /// [`PAMOJA_LORA_RADIO_SYNC_WORD_PRIVATE`], or another value.
    pub sync_word: u8,
    /// `true` to send frames with inverted IQ, as a LoRaWAN gateway sends downlinks.
    pub invert_iq_transmit: bool,
    /// `true` to expect frames with inverted IQ, as a LoRaWAN device hears downlinks.
    pub invert_iq_receive: bool,
}

/// How a reception ended, with the frame's signal levels when one arrived.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PamojaLoraRadioReception {
    /// [`PAMOJA_LORA_RADIO_FRAME`], [`PAMOJA_LORA_RADIO_TIMEOUT`],
    /// [`PAMOJA_LORA_RADIO_CORRUPT`], or [`PAMOJA_LORA_RADIO_NOTHING`].
    pub outcome: u8,
    /// The payload length, at the start of the buffer, for a frame; 0 otherwise.
    pub len: usize,
    /// The received signal strength averaged over the frame, in hundredths of a dBm.
    pub rssi_centi_dbm: i32,
    /// The estimated signal-to-noise ratio, in hundredths of a dB.
    pub snr_centi_db: i32,
    /// The estimated strength of the LoRa signal itself, in hundredths of a dBm.
    pub signal_rssi_centi_dbm: i32,
}

/// Returns a configuration with a private sync word, standard IQ both ways, and an SX126x
/// calibrating for the carrier alone.
///
/// # Arguments
///
/// * `frequency_hz` - the carrier frequency in hertz.
/// * `link` - the LoRa link settings.
/// * `output_dbm` - the output power asked of the amplifier, in dBm.
///
/// # Returns
///
/// The configuration.
#[no_mangle]
pub extern "C" fn pamoja_lora_radio_config_default(
    frequency_hz: u32,
    link: PamojaLoraLink,
    output_dbm: i8,
) -> PamojaLoraRadioConfig {
    PamojaLoraRadioConfig {
        frequency_hz,
        band_low_hz: 0,
        band_high_hz: 0,
        link,
        output_dbm,
        sync_word: PAMOJA_LORA_RADIO_SYNC_WORD_PRIVATE,
        invert_iq_transmit: false,
        invert_iq_receive: false,
    }
}

/// Opens an SX1261, SX1262, SX1268, or LLCC68 module on a Linux board and resets it.
///
/// # Arguments
///
/// * `spi` - the SPI device file, such as `/dev/spidev0.0`.
/// * `spi_hz` - the SPI clock in hertz, or 0 for [`PAMOJA_LORA_RADIO_DEFAULT_SPI_HZ`].
/// * `gpio_chip` - the GPIO chip the lines are on, such as `/dev/gpiochip0`.
/// * `busy_line` - the line the BUSY pin is on.
/// * `reset_line` - the line the reset pin is on.
/// * `board` - how the module wires the chip.
/// * `out_radio` - receives the radio, or null when opening fails.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the radio in `out_radio`; [`PamojaStatus::Unsupported`] on any
/// platform but Linux; [`PamojaStatus::InvalidArgument`] for a null or non-UTF-8 argument or a
/// TCXO voltage past 7; or [`PamojaStatus::Io`] when a device cannot be opened or no chip
/// answers. The last error message says which.
///
/// # Safety
///
/// `spi` and `gpio_chip` must be null-terminated strings or null, and `out_radio` a writable
/// pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_open_sx126x(
    spi: *const c_char,
    spi_hz: u32,
    gpio_chip: *const c_char,
    busy_line: u32,
    reset_line: u32,
    board: PamojaSx126xBoard,
    out_radio: *mut *mut PamojaLoraRadio,
) -> PamojaStatus {
    if !clear(out_radio) {
        return PamojaStatus::InvalidArgument;
    }
    let Some(chip) = sx126x_board(board) else {
        set_last_error(format!(
            "TCXO voltage code {} is not one of 0 to 7",
            board.tcxo_voltage
        ));
        return PamojaStatus::InvalidArgument;
    };
    let Some(wiring) = wiring(spi, spi_hz, gpio_chip, reset_line) else {
        return PamojaStatus::InvalidArgument;
    };
    open(out_radio, || {
        linux::open_sx126x(&wiring.with_busy_line(busy_line), chip)
    })
}

/// Opens an SX1276, SX1277, SX1278, or SX1279 module on a Linux board, such as an RFM95W,
/// and resets it into LoRa mode.
///
/// # Arguments
///
/// * `spi` - the SPI device file, such as `/dev/spidev0.0`.
/// * `spi_hz` - the SPI clock in hertz, or 0 for [`PAMOJA_LORA_RADIO_DEFAULT_SPI_HZ`].
/// * `gpio_chip` - the GPIO chip the reset line is on, such as `/dev/gpiochip0`.
/// * `reset_line` - the line the reset pin is on.
/// * `pa_boost` - `true` when the antenna is on the PA_BOOST output, as on the RFM95W, and
///   `false` for RFO.
/// * `tcxo` - `true` when a TCXO drives the XTA pin instead of a crystal.
/// * `out_radio` - receives the radio, or null when opening fails.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the radio in `out_radio`; [`PamojaStatus::Unsupported`] on any
/// platform but Linux; [`PamojaStatus::InvalidArgument`] for a null or non-UTF-8 argument; or
/// [`PamojaStatus::Io`] when a device cannot be opened or no chip answers. The last error
/// message says which.
///
/// # Safety
///
/// `spi` and `gpio_chip` must be null-terminated strings or null, and `out_radio` a writable
/// pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_open_sx127x(
    spi: *const c_char,
    spi_hz: u32,
    gpio_chip: *const c_char,
    reset_line: u32,
    pa_boost: bool,
    tcxo: bool,
    out_radio: *mut *mut PamojaLoraRadio,
) -> PamojaStatus {
    if !clear(out_radio) {
        return PamojaStatus::InvalidArgument;
    }
    let Some(wiring) = wiring(spi, spi_hz, gpio_chip, reset_line) else {
        return PamojaStatus::InvalidArgument;
    };
    let output = if pa_boost {
        PaOutput::PaBoost
    } else {
        PaOutput::Rfo
    };
    let mut board = sx127x::Board::new(output);
    if tcxo {
        board = board.with_tcxo();
    }
    open(out_radio, || linux::open_sx127x(&wiring, board))
}

/// Returns the family of a radio's chip.
///
/// # Arguments
///
/// * `radio` - the radio.
///
/// # Returns
///
/// [`PAMOJA_LORA_RADIO_SX126X`] or [`PAMOJA_LORA_RADIO_SX127X`], or 255 if `radio` is null.
///
/// # Safety
///
/// `radio` must be a live handle from one of the open functions, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_family(radio: *const PamojaLoraRadio) -> u8 {
    if radio.is_null() {
        return u8::MAX;
    }
    match (*radio).radio.family() {
        Family::Sx126x => PAMOJA_LORA_RADIO_SX126X,
        Family::Sx127x => PAMOJA_LORA_RADIO_SX127X,
    }
}

/// Tunes a radio to a configuration.
///
/// # Arguments
///
/// * `radio` - the radio.
/// * `config` - the configuration, such as one from [`pamoja_lora_radio_config_default`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null radio or settings the
/// chip cannot use, such as a bandwidth it lacks; or [`PamojaStatus::Io`] when the chip does
/// not answer.
///
/// # Safety
///
/// `radio` must be a live handle from one of the open functions, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_configure(
    radio: *mut PamojaLoraRadio,
    config: PamojaLoraRadioConfig,
) -> PamojaStatus {
    status(drive(radio, |radio| radio.configure(config_of(config))))
}

/// Sends one frame and waits for it to leave.
///
/// # Arguments
///
/// * `radio` - the radio.
/// * `payload` - the payload bytes, 1 to 255 of them.
/// * `len` - the payload length.
/// * `out_airtime_us` - receives the frame's airtime in microseconds, or null.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null radio, a null payload
/// with a length, a payload the chip cannot send, or a radio not yet configured; or
/// [`PamojaStatus::Io`] when the chip does not report the frame sent.
///
/// # Safety
///
/// `radio` must be a live handle or null, `payload` must point at `len` readable bytes or be
/// null, and `out_airtime_us` must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_transmit(
    radio: *mut PamojaLoraRadio,
    payload: *const u8,
    len: usize,
    out_airtime_us: *mut u64,
) -> PamojaStatus {
    let payload = if len == 0 {
        &[][..]
    } else if payload.is_null() {
        set_last_error("payload must not be null when its length is non-zero".to_owned());
        return PamojaStatus::InvalidArgument;
    } else {
        std::slice::from_raw_parts(payload, len)
    };
    match drive(radio, |radio| radio.transmit(payload)) {
        Ok(airtime_us) => {
            if !out_airtime_us.is_null() {
                *out_airtime_us = airtime_us;
            }
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Listens for one frame.
///
/// # Arguments
///
/// * `radio` - the radio.
/// * `buffer` - where the payload goes.
/// * `capacity` - the buffer's length; up to 255 bytes are accepted.
/// * `timeout_us` - how long to listen for a frame to start, in microseconds.
/// * `out_reception` - receives how the reception ended.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the outcome in `out_reception`, a timeout and a corrupt frame
/// included; [`PamojaStatus::InvalidArgument`] for a null argument, a payload longer than
/// `capacity`, or a radio not yet configured; or [`PamojaStatus::Io`] when the chip does not
/// answer.
///
/// # Safety
///
/// `radio` must be a live handle or null, `buffer` must point at `capacity` writable bytes or
/// be null, and `out_reception` must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_receive(
    radio: *mut PamojaLoraRadio,
    buffer: *mut u8,
    capacity: usize,
    timeout_us: u64,
    out_reception: *mut PamojaLoraRadioReception,
) -> PamojaStatus {
    let Some(buffer) = buffer_of(buffer, capacity) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_reception.is_null() {
        set_last_error("out_reception must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match drive(radio, |radio| radio.receive(buffer, timeout_us)) {
        Ok(reception) => {
            *out_reception = reception_of(Some(reception));
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Starts listening, frame after frame, until another call changes the mode.
///
/// # Arguments
///
/// * `radio` - the radio.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null radio or one not yet
/// configured; or [`PamojaStatus::Io`] when the chip does not answer.
///
/// # Safety
///
/// `radio` must be a live handle from one of the open functions, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_listen(radio: *mut PamojaLoraRadio) -> PamojaStatus {
    status(drive(radio, |radio| radio.listen()))
}

/// Takes the frame a listening radio has received, if one has arrived.
///
/// # Arguments
///
/// * `radio` - the radio.
/// * `buffer` - where the payload goes.
/// * `capacity` - the buffer's length.
/// * `out_reception` - receives the frame, a corrupt frame, or
///   [`PAMOJA_LORA_RADIO_NOTHING`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the outcome in `out_reception`; [`PamojaStatus::InvalidArgument`]
/// for a null argument or a payload longer than `capacity`; or [`PamojaStatus::Io`] when the
/// chip does not answer.
///
/// # Safety
///
/// `radio` must be a live handle or null, `buffer` must point at `capacity` writable bytes or
/// be null, and `out_reception` must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_take_frame(
    radio: *mut PamojaLoraRadio,
    buffer: *mut u8,
    capacity: usize,
    out_reception: *mut PamojaLoraRadioReception,
) -> PamojaStatus {
    let Some(buffer) = buffer_of(buffer, capacity) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_reception.is_null() {
        set_last_error("out_reception must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match drive(radio, |radio| radio.take_frame(buffer)) {
        Ok(taken) => {
            *out_reception = reception_of(taken);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Puts a radio in standby, which stops a transmission or a reception.
///
/// # Arguments
///
/// * `radio` - the radio.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null radio; or
/// [`PamojaStatus::Io`] when the chip does not answer.
///
/// # Safety
///
/// `radio` must be a live handle from one of the open functions, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_standby(radio: *mut PamojaLoraRadio) -> PamojaStatus {
    status(drive(radio, |radio| radio.standby()))
}

/// Puts a radio to sleep until the next call wakes it. An SX126x is configured again before
/// its next frame; an SX127x keeps its registers.
///
/// # Arguments
///
/// * `radio` - the radio.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null radio; or
/// [`PamojaStatus::Io`] when the chip does not answer.
///
/// # Safety
///
/// `radio` must be a live handle from one of the open functions, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_sleep(radio: *mut PamojaLoraRadio) -> PamojaStatus {
    status(drive(radio, |radio| radio.sleep()))
}

/// Reads one register of a radio's chip.
///
/// # Arguments
///
/// * `radio` - the radio.
/// * `address` - the register address: 16 bits on the SX126x, 0x00 to 0x7F on the SX127x.
/// * `out_value` - receives the value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null argument or an SX127x
/// address past 0x7F; or [`PamojaStatus::Io`] when the chip does not answer.
///
/// # Safety
///
/// `radio` must be a live handle or null, and `out_value` writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_read_register(
    radio: *mut PamojaLoraRadio,
    address: u16,
    out_value: *mut u8,
) -> PamojaStatus {
    if out_value.is_null() {
        set_last_error("out_value must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match drive(radio, |radio| radio.read_register(address)) {
        Ok(value) => {
            *out_value = value;
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Writes one register of a radio's chip.
///
/// # Arguments
///
/// * `radio` - the radio.
/// * `address` - the register address: 16 bits on the SX126x, 0x00 to 0x7F on the SX127x.
/// * `value` - the value to write.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null radio or an SX127x
/// address past 0x7F; or [`PamojaStatus::Io`] when the chip does not answer.
///
/// # Safety
///
/// `radio` must be a live handle from one of the open functions, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_write_register(
    radio: *mut PamojaLoraRadio,
    address: u16,
    value: u8,
) -> PamojaStatus {
    status(drive(radio, |radio| radio.write_register(address, value)))
}

/// Closes a radio's device files and releases it.
///
/// # Arguments
///
/// * `radio` - the radio, which must not be used again.
///
/// # Safety
///
/// `radio` must be a live handle from one of the open functions, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lora_radio_free(radio: *mut PamojaLoraRadio) {
    if !radio.is_null() {
        drop(Box::from_raw(radio));
    }
}

/// Nulls an out pointer before a radio is opened into it, refusing a null pointer.
unsafe fn clear(out_radio: *mut *mut PamojaLoraRadio) -> bool {
    if out_radio.is_null() {
        set_last_error("out_radio must not be null".to_owned());
        return false;
    }
    *out_radio = std::ptr::null_mut();
    true
}

/// Reads the wiring the open functions share, recording an error for a bad string.
unsafe fn wiring(
    spi: *const c_char,
    spi_hz: u32,
    gpio_chip: *const c_char,
    reset_line: u32,
) -> Option<Wiring> {
    let spi = read_str(spi, "spi")?;
    let gpio_chip = read_str(gpio_chip, "gpio_chip")?;
    let wiring = Wiring::new(spi, gpio_chip, reset_line);
    Some(if spi_hz == 0 {
        wiring
    } else {
        wiring.with_spi_hz(spi_hz)
    })
}

/// Opens a radio into an out pointer that [`clear`] has checked.
unsafe fn open(
    out_radio: *mut *mut PamojaLoraRadio,
    opening: impl FnOnce() -> Result<LinuxRadio, OpenError>,
) -> PamojaStatus {
    match catch_unwind(AssertUnwindSafe(opening)) {
        Ok(Ok(radio)) => {
            *out_radio = Box::into_raw(Box::new(PamojaLoraRadio { radio }));
            PamojaStatus::Ok
        }
        Ok(Err(error)) => {
            set_last_error(error.to_string());
            match error {
                OpenError::Unsupported => PamojaStatus::Unsupported,
                OpenError::NoBusyLine => PamojaStatus::InvalidArgument,
                OpenError::Bus { .. } => PamojaStatus::Io,
                OpenError::Radio(error) => radio_status(&error),
            }
        }
        Err(_) => panicked(),
    }
}

/// Runs a call on a live radio, turning a refusal or a panic into a status.
unsafe fn drive<T>(
    radio: *mut PamojaLoraRadio,
    call: impl FnOnce(&mut LinuxRadio) -> Result<T, RadioError<SpiError>>,
) -> Result<T, PamojaStatus> {
    if radio.is_null() {
        set_last_error("radio must not be null".to_owned());
        return Err(PamojaStatus::InvalidArgument);
    }
    let radio = &mut (*radio).radio;
    match catch_unwind(AssertUnwindSafe(|| call(radio))) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => {
            set_last_error(error.to_string());
            Err(radio_status(&error))
        }
        Err(_) => Err(panicked()),
    }
}

/// Folds a call that returns nothing into its status.
fn status(result: Result<(), PamojaStatus>) -> PamojaStatus {
    result.err().unwrap_or(PamojaStatus::Ok)
}

/// Classifies a radio error: settings or a call the chip cannot take are the caller's to fix,
/// and everything else is the bus or the chip.
fn radio_status<E>(error: &RadioError<E>) -> PamojaStatus {
    match error {
        RadioError::Address(_)
        | RadioError::Sx126x(
            sx126x::RadioError::Bandwidth(_)
            | sx126x::RadioError::Llcc68 { .. }
            | sx126x::RadioError::Amplifier
            | sx126x::RadioError::PayloadTooLong(_)
            | sx126x::RadioError::BufferTooSmall(_)
            | sx126x::RadioError::NotConfigured,
        )
        | RadioError::Sx127x(
            sx127x::RadioError::Modulation(_)
            | sx127x::RadioError::Output
            | sx127x::RadioError::PayloadLength(_)
            | sx127x::RadioError::BufferTooSmall(_)
            | sx127x::RadioError::NotConfigured,
        ) => PamojaStatus::InvalidArgument,
        _ => PamojaStatus::Io,
    }
}

/// Borrows a caller's buffer, recording an error for a null one with a capacity.
unsafe fn buffer_of<'a>(buffer: *mut u8, capacity: usize) -> Option<&'a mut [u8]> {
    if capacity == 0 {
        return Some(&mut []);
    }
    if buffer.is_null() {
        set_last_error("buffer must not be null when its capacity is non-zero".to_owned());
        return None;
    }
    Some(std::slice::from_raw_parts_mut(buffer, capacity))
}

/// Reads a board description, or `None` for a TCXO voltage code past 7.
fn sx126x_board(board: PamojaSx126xBoard) -> Option<sx126x::Board> {
    let amplifier = if board.high_power {
        PowerAmplifier::HighPower
    } else {
        PowerAmplifier::LowPower
    };
    let mut chip = sx126x::Board::new(amplifier);
    if board.tcxo {
        chip = chip.with_tcxo(tcxo_voltage(board.tcxo_voltage)?, board.tcxo_settle_us);
    }
    if board.dio2_rf_switch {
        chip = chip.with_dio2_rf_switch();
    }
    if board.dc_dc {
        chip = chip.with_dc_dc();
    }
    if board.llcc68 {
        chip = chip.with_llcc68();
    }
    Some(chip)
}

/// Names the TCXO voltage a SetDIO3AsTCXOCtrl code selects.
fn tcxo_voltage(code: u8) -> Option<TcxoVoltage> {
    Some(match code {
        0 => TcxoVoltage::V1_6,
        1 => TcxoVoltage::V1_7,
        2 => TcxoVoltage::V1_8,
        3 => TcxoVoltage::V2_2,
        4 => TcxoVoltage::V2_4,
        5 => TcxoVoltage::V2_7,
        6 => TcxoVoltage::V3_0,
        7 => TcxoVoltage::V3_3,
        _ => return None,
    })
}

/// Reads a configuration from the boundary.
fn config_of(config: PamojaLoraRadioConfig) -> RadioConfig {
    let radio_config = RadioConfig::new(
        config.frequency_hz,
        settings(config.link),
        config.output_dbm,
    )
    .with_sync_word(SyncWord::from_byte(config.sync_word))
    .with_inverted_iq(config.invert_iq_transmit, config.invert_iq_receive);
    if config.band_low_hz == 0 && config.band_high_hz == 0 {
        radio_config
    } else {
        radio_config.with_band(config.band_low_hz, config.band_high_hz)
    }
}

/// Flattens how a reception ended for the boundary.
fn reception_of(reception: Option<Reception>) -> PamojaLoraRadioReception {
    match reception {
        Some(Reception::Frame { len, levels }) => PamojaLoraRadioReception {
            outcome: PAMOJA_LORA_RADIO_FRAME,
            len,
            rssi_centi_dbm: levels.rssi_dbm.hundredths(),
            snr_centi_db: levels.snr_db.hundredths(),
            signal_rssi_centi_dbm: levels.signal_rssi_dbm.hundredths(),
        },
        Some(Reception::Timeout) => PamojaLoraRadioReception {
            outcome: PAMOJA_LORA_RADIO_TIMEOUT,
            ..PamojaLoraRadioReception::default()
        },
        Some(Reception::Corrupt) => PamojaLoraRadioReception {
            outcome: PAMOJA_LORA_RADIO_CORRUPT,
            ..PamojaLoraRadioReception::default()
        },
        None => PamojaLoraRadioReception {
            outcome: PAMOJA_LORA_RADIO_NOTHING,
            ..PamojaLoraRadioReception::default()
        },
    }
}

/// Records a caught panic and reports it as [`PamojaStatus::Panic`].
fn panicked() -> PamojaStatus {
    set_last_error("panic at the FFI boundary".to_owned());
    PamojaStatus::Panic
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use std::ptr;

    use pamoja_lora::budget::Decibels;
    use pamoja_radios::radio::SignalLevels;

    use super::*;
    use crate::lora::pamoja_lora_link_default;
    use crate::pamoja_last_error_message;

    fn last_error() -> String {
        unsafe { CStr::from_ptr(pamoja_last_error_message()) }
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn a_default_config_is_private_and_calibrates_for_the_carrier() {
        let link = pamoja_lora_link_default(9, 125_000);
        let config = pamoja_lora_radio_config_default(868_100_000, link, 14);
        assert_eq!(config.sync_word, PAMOJA_LORA_RADIO_SYNC_WORD_PRIVATE);

        let plain = config_of(config);
        assert_eq!(plain.band_hz, (868_100_000, 868_100_000));
        assert_eq!(plain.sync_word, SyncWord::Private);
        assert_eq!(plain.output_dbm, 14);

        let banded = config_of(PamojaLoraRadioConfig {
            band_low_hz: 863_000_000,
            band_high_hz: 870_000_000,
            sync_word: 0x2B,
            invert_iq_receive: true,
            ..config
        });
        assert_eq!(banded.band_hz, (863_000_000, 870_000_000));
        assert_eq!(banded.sync_word, SyncWord::Custom(0x2B));
        assert!(banded.invert_iq_receive && !banded.invert_iq_transmit);
    }

    #[test]
    fn a_board_names_its_tcxo_voltage_by_code() {
        let board = PamojaSx126xBoard {
            high_power: true,
            tcxo: true,
            tcxo_voltage: 1,
            dio2_rf_switch: true,
            dc_dc: true,
            llcc68: false,
            tcxo_settle_us: 5_000,
        };
        let chip = sx126x_board(board).expect("1.7 V is a TCXO voltage");
        assert_eq!(chip.tcxo, Some((TcxoVoltage::V1_7, 5_000)));
        assert!(chip.dio2_rf_switch && !chip.llcc68);
        assert_eq!(chip.amplifier, PowerAmplifier::HighPower);
        assert!(sx126x_board(PamojaSx126xBoard {
            tcxo_voltage: 8,
            ..board
        })
        .is_none());
    }

    #[test]
    fn a_reception_crosses_with_its_levels_in_hundredths() {
        let frame = reception_of(Some(Reception::Frame {
            len: 10,
            levels: SignalLevels {
                rssi_dbm: Decibels::from_db(-109),
                snr_db: Decibels::from_tenths(-25),
                signal_rssi_dbm: Decibels::from_tenths(-1115),
            },
        }));
        assert_eq!(frame.outcome, PAMOJA_LORA_RADIO_FRAME);
        assert_eq!(frame.len, 10);
        assert_eq!(
            (
                frame.rssi_centi_dbm,
                frame.snr_centi_db,
                frame.signal_rssi_centi_dbm
            ),
            (-10_900, -250, -11_150)
        );
        assert_eq!(
            reception_of(Some(Reception::Timeout)).outcome,
            PAMOJA_LORA_RADIO_TIMEOUT
        );
        assert_eq!(reception_of(None).outcome, PAMOJA_LORA_RADIO_NOTHING);
    }

    #[test]
    fn a_null_radio_or_argument_is_refused() {
        unsafe {
            assert_eq!(
                pamoja_lora_radio_standby(ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(pamoja_lora_radio_family(ptr::null()), u8::MAX);
            let spi = CString::new("/dev/spidev0.0").expect("no interior null");
            assert_eq!(
                pamoja_lora_radio_open_sx127x(
                    spi.as_ptr(),
                    0,
                    ptr::null(),
                    25,
                    true,
                    false,
                    &mut ptr::null_mut()
                ),
                PamojaStatus::InvalidArgument
            );
            assert!(last_error().contains("gpio_chip"), "{}", last_error());
            pamoja_lora_radio_free(ptr::null_mut());
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn opening_a_missing_device_is_an_io_error_that_names_it() {
        let spi = CString::new("/dev/spidev-pamoja-absent").expect("no interior null");
        let chip = CString::new("/dev/gpiochip0").expect("no interior null");
        let mut radio = ptr::null_mut();
        let opened = unsafe {
            pamoja_lora_radio_open_sx127x(
                spi.as_ptr(),
                0,
                chip.as_ptr(),
                25,
                true,
                false,
                &mut radio,
            )
        };
        assert_eq!(opened, PamojaStatus::Io);
        assert!(radio.is_null());
        assert!(
            last_error().starts_with("/dev/spidev-pamoja-absent: "),
            "{}",
            last_error()
        );
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn opening_off_linux_is_unsupported() {
        let spi = CString::new("/dev/spidev0.0").expect("no interior null");
        let chip = CString::new("/dev/gpiochip0").expect("no interior null");
        let board = PamojaSx126xBoard {
            high_power: true,
            ..PamojaSx126xBoard::default()
        };
        let mut radio = ptr::null_mut();
        let opened = unsafe {
            pamoja_lora_radio_open_sx126x(spi.as_ptr(), 0, chip.as_ptr(), 24, 25, board, &mut radio)
        };
        assert_eq!(opened, PamojaStatus::Unsupported);
        assert!(radio.is_null());
        assert!(last_error().contains("only Linux"), "{}", last_error());
    }
}
