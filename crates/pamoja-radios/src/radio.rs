//! One LoRa radio of either family, behind one set of calls.
//!
//! The SX126x takes commands and the SX127x takes registers, but a program that sends a
//! reading does not care which: it tunes to a carrier, sends a frame, and listens for one.
//! [`Radio`] holds either driver and gives both the same calls, configured from one
//! [`RadioConfig`] of a carrier, a LoRa link, an output power, a sync word, and the IQ
//! polarity of each direction. A program that reads which module it has from a file keeps
//! one code path, and one that needs a chip's own features matches on the enum and reaches
//! its driver.
//!
//! # Examples
//!
//! An RFM95W behind the shared calls, answering a register read over a scripted bus:
//!
//! ```
//! use pamoja_hal::script::{DelayLog, PinScript, SpiScript, SpiStep};
//! use pamoja_radios::radio::{Family, Radio, RadioError};
//! use pamoja_radios::sx127x::config::PaOutput;
//! use pamoja_radios::sx127x::{Board, Sx127x};
//!
//! // RegVersion, at 0x42, holds 0x12 on every SX1276.
//! let spi = SpiScript::new([SpiStep::write([0x42]), SpiStep::read([0x12])]);
//! let board = Board::new(PaOutput::PaBoost);
//! let chip = Sx127x::new(spi, PinScript::new([]), DelayLog::new(), board);
//! let mut radio: Radio<_, PinScript, _, _> = Radio::from(chip);
//!
//! assert_eq!(radio.family(), Family::Sx127x);
//! assert_eq!(radio.read_register(0x42), Ok(0x12));
//! // The SX127x address byte carries seven bits, so its register map ends at 0x7F.
//! assert_eq!(radio.read_register(0x0740), Err(RadioError::Address(0x0740)));
//! ```

use core::fmt;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;
use pamoja_lora::budget::Decibels;
use pamoja_lora::LinkSettings;

use crate::sx126x::{self, Sx126x};
use crate::sx127x::{self, Sx127x};

/// The last address an SX127x register read or write can reach: the SPI address byte
/// carries seven bits after the write flag.
const SX127X_LAST_REGISTER: u16 = 0x7F;

/// Which family a radio's chip belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// The SX1261, SX1262, SX1268, and LLCC68, which take commands.
    Sx126x,
    /// The SX1276, SX1277, SX1278, and SX1279, which take registers.
    Sx127x,
}

/// The LoRa sync word, which keeps one network's frames apart from another's.
///
/// Both families carry it as one byte. The SX127x writes the byte to RegSyncWord. The SX126x
/// spreads its two nibbles over its two sync word registers with a 4 after each, so 0x34 is
/// written as 0x3444 and 0x12 as 0x1424, which are the chip's own public and private values,
/// and any other byte takes the same layout, the one RadioLib's `setSyncWord` writes.
///
/// # Examples
///
/// ```
/// use pamoja_radios::radio::SyncWord;
/// use pamoja_radios::sx126x::config::SyncWord as Sx126xSyncWord;
///
/// assert_eq!(SyncWord::from_byte(0x34), SyncWord::Public);
/// assert_eq!(SyncWord::Custom(0x2B).sx126x(), Sx126xSyncWord::Custom(0x24B4));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyncWord {
    /// 0x34, for a public network such as LoRaWAN.
    Public,
    /// 0x12, for a private network, and both families' reset value.
    Private,
    /// Another byte.
    Custom(u8),
}

impl SyncWord {
    /// Returns the byte.
    ///
    /// # Returns
    ///
    /// 0x34 for a public network, 0x12 for a private one, or the custom byte.
    pub const fn to_byte(self) -> u8 {
        match self {
            SyncWord::Public => 0x34,
            SyncWord::Private => 0x12,
            SyncWord::Custom(byte) => byte,
        }
    }

    /// Names a byte.
    ///
    /// # Arguments
    ///
    /// * `byte` - the sync word byte.
    ///
    /// # Returns
    ///
    /// [`SyncWord::Public`] for 0x34, [`SyncWord::Private`] for 0x12, and a custom word for
    /// any other byte.
    pub const fn from_byte(byte: u8) -> SyncWord {
        match byte {
            0x34 => SyncWord::Public,
            0x12 => SyncWord::Private,
            other => SyncWord::Custom(other),
        }
    }

    /// Returns the word as the SX126x takes it.
    ///
    /// # Returns
    ///
    /// The two-byte word, each nibble of the byte followed by a 4.
    pub const fn sx126x(self) -> sx126x::config::SyncWord {
        match self {
            SyncWord::Public => sx126x::config::SyncWord::Public,
            SyncWord::Private => sx126x::config::SyncWord::Private,
            SyncWord::Custom(byte) => {
                let high = (byte >> 4) as u16;
                let low = (byte & 0x0F) as u16;
                sx126x::config::SyncWord::Custom((high << 12) | 0x0400 | (low << 4) | 0x0004)
            }
        }
    }

    /// Returns the word as the SX127x takes it.
    ///
    /// # Returns
    ///
    /// The RegSyncWord value.
    pub const fn sx127x(self) -> sx127x::config::SyncWord {
        match self {
            SyncWord::Public => sx127x::config::SyncWord::Public,
            SyncWord::Private => sx127x::config::SyncWord::Private,
            SyncWord::Custom(byte) => sx127x::config::SyncWord::Custom(byte),
        }
    }
}

/// What a radio of either family sends and listens with.
///
/// The output power is what the amplifier is asked for, in whole dBm. Each driver clamps it
/// to the range of the amplifier its board names, so a regional ceiling is kept by choosing
/// the power first, with
/// [`LinkBudget::max_transmit_power_dbm`](pamoja_lora::budget::LinkBudget::max_transmit_power_dbm)
/// rounded down.
///
/// # Examples
///
/// ```
/// use pamoja_lora::LinkSettings;
/// use pamoja_radios::radio::{RadioConfig, SyncWord};
///
/// // A LoRaWAN device on 868.1 MHz at SF9 with 14 dBm, calibrated for the whole band.
/// let config = RadioConfig::new(868_100_000, LinkSettings::new(9, 125_000), 14)
///     .with_band(863_000_000, 870_000_000)
///     .lorawan_device();
/// assert_eq!(config.sync_word, SyncWord::Public);
/// assert!(config.invert_iq_receive && !config.invert_iq_transmit);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadioConfig {
    /// The carrier frequency in hertz.
    pub frequency_hz: u32,
    /// The band an SX126x calibrates its receiver image for, as its lower and upper edges in
    /// hertz. The SX127x calibrates at the carrier instead, once per RF port.
    pub band_hz: (u32, u32),
    /// The spreading factor, bandwidth, coding rate, preamble, header, and CRC.
    pub link: LinkSettings,
    /// The output power asked of the amplifier, in dBm.
    pub output_dbm: i8,
    /// The sync word.
    pub sync_word: SyncWord,
    /// Whether frames go out with inverted IQ, as a LoRaWAN gateway sends downlinks.
    pub invert_iq_transmit: bool,
    /// Whether the receiver expects inverted IQ, as a LoRaWAN device hears downlinks.
    pub invert_iq_receive: bool,
}

impl RadioConfig {
    /// Builds a configuration with a private sync word and standard IQ both ways.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the carrier frequency in hertz.
    /// * `link` - the LoRa link settings.
    /// * `output_dbm` - the output power asked of the amplifier, in dBm.
    ///
    /// # Returns
    ///
    /// The configuration, with the SX126x calibration band covering just the carrier.
    pub const fn new(frequency_hz: u32, link: LinkSettings, output_dbm: i8) -> RadioConfig {
        RadioConfig {
            frequency_hz,
            band_hz: (frequency_hz, frequency_hz),
            link,
            output_dbm,
            sync_word: SyncWord::Private,
            invert_iq_transmit: false,
            invert_iq_receive: false,
        }
    }

    /// Returns the configuration with an SX126x calibrating for a whole band, so moving
    /// between channels inside it needs no new calibration.
    ///
    /// # Arguments
    ///
    /// * `low_hz` - the lower edge of the band in hertz.
    /// * `high_hz` - the upper edge of the band in hertz.
    ///
    /// # Returns
    ///
    /// The configuration.
    pub const fn with_band(mut self, low_hz: u32, high_hz: u32) -> RadioConfig {
        self.band_hz = (low_hz, high_hz);
        self
    }

    /// Returns the configuration with another sync word.
    ///
    /// # Arguments
    ///
    /// * `sync_word` - the sync word.
    ///
    /// # Returns
    ///
    /// The configuration.
    pub const fn with_sync_word(mut self, sync_word: SyncWord) -> RadioConfig {
        self.sync_word = sync_word;
        self
    }

    /// Returns the configuration with the IQ polarity set for each direction.
    ///
    /// # Arguments
    ///
    /// * `transmit` - `true` to send with inverted IQ.
    /// * `receive` - `true` to listen for inverted IQ.
    ///
    /// # Returns
    ///
    /// The configuration.
    pub const fn with_inverted_iq(mut self, transmit: bool, receive: bool) -> RadioConfig {
        self.invert_iq_transmit = transmit;
        self.invert_iq_receive = receive;
        self
    }

    /// Returns the configuration a LoRaWAN end device uses: the public sync word, uplinks
    /// with standard IQ, and downlinks heard with inverted IQ.
    ///
    /// # Returns
    ///
    /// The configuration.
    pub const fn lorawan_device(self) -> RadioConfig {
        self.with_sync_word(SyncWord::Public)
            .with_inverted_iq(false, true)
    }
}

/// The signal levels a frame arrived with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SignalLevels {
    /// The received signal strength averaged over the frame, in dBm.
    pub rssi_dbm: Decibels,
    /// The estimated signal-to-noise ratio, in dB, negative below the noise floor.
    pub snr_db: Decibels,
    /// The estimated strength of the LoRa signal itself, in dBm.
    pub signal_rssi_dbm: Decibels,
}

impl From<sx126x::status::PacketStatus> for SignalLevels {
    fn from(status: sx126x::status::PacketStatus) -> SignalLevels {
        SignalLevels {
            rssi_dbm: status.rssi_dbm,
            snr_db: status.snr_db,
            signal_rssi_dbm: status.signal_rssi_dbm,
        }
    }
}

impl From<sx127x::status::PacketStatus> for SignalLevels {
    fn from(status: sx127x::status::PacketStatus) -> SignalLevels {
        SignalLevels {
            rssi_dbm: status.rssi_dbm,
            snr_db: status.snr_db,
            signal_rssi_dbm: status.signal_rssi_dbm,
        }
    }
}

/// How a reception ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reception {
    /// A frame that checked.
    Frame {
        /// The payload length; the payload is at the start of the buffer.
        len: usize,
        /// The signal levels the frame arrived with.
        levels: SignalLevels,
    },
    /// No frame arrived before the timeout.
    Timeout,
    /// A frame arrived whose header or CRC failed its check, and was dropped.
    Corrupt,
}

impl From<sx126x::Reception> for Reception {
    fn from(reception: sx126x::Reception) -> Reception {
        match reception {
            sx126x::Reception::Frame { len, status } => Reception::Frame {
                len,
                levels: status.into(),
            },
            sx126x::Reception::Timeout => Reception::Timeout,
            sx126x::Reception::Corrupt => Reception::Corrupt,
        }
    }
}

impl From<sx127x::Reception> for Reception {
    fn from(reception: sx127x::Reception) -> Reception {
        match reception {
            sx127x::Reception::Frame { len, status } => Reception::Frame {
                len,
                levels: status.into(),
            },
            sx127x::Reception::Timeout => Reception::Timeout,
            sx127x::Reception::Corrupt => Reception::Corrupt,
        }
    }
}

/// What can go wrong driving a radio of either family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioError<E> {
    /// The SX126x driver failed.
    Sx126x(sx126x::RadioError<E>),
    /// The SX127x driver failed.
    Sx127x(sx127x::RadioError<E>),
    /// A register address past the 0x7F an SX127x register map ends at.
    Address(u16),
}

impl<E: fmt::Debug> fmt::Display for RadioError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RadioError::Sx126x(error) => error.fmt(f),
            RadioError::Sx127x(error) => error.fmt(f),
            RadioError::Address(address) => write!(
                f,
                "the SX127x has no register at {address:#06x}; its map ends at 0x7F"
            ),
        }
    }
}

impl<E: fmt::Debug> core::error::Error for RadioError<E> {}

/// A LoRa radio of either family: an SX126x with its BUSY line, or an SX127x.
///
/// Build one from a driver with [`From`], or open one on a Linux board through
/// [`linux`](crate::linux) with the `linux` feature. [`init`](Radio::init) resets the chip,
/// [`configure`](Radio::configure) tunes it, and the calls after that are the same for both
/// families. The SX127x has no BUSY line, so its radio names a BUSY type it never uses.
pub enum Radio<SPI, BUSY, RESET, D> {
    /// An SX1261, SX1262, SX1268, or LLCC68.
    Sx126x(Sx126x<SPI, BUSY, RESET, D>),
    /// An SX1276, SX1277, SX1278, or SX1279.
    Sx127x(Sx127x<SPI, RESET, D>),
}

impl<SPI, BUSY, RESET, D> From<Sx126x<SPI, BUSY, RESET, D>> for Radio<SPI, BUSY, RESET, D> {
    fn from(radio: Sx126x<SPI, BUSY, RESET, D>) -> Self {
        Radio::Sx126x(radio)
    }
}

impl<SPI, BUSY, RESET, D> From<Sx127x<SPI, RESET, D>> for Radio<SPI, BUSY, RESET, D> {
    fn from(radio: Sx127x<SPI, RESET, D>) -> Self {
        Radio::Sx127x(radio)
    }
}

impl<SPI, BUSY, RESET, D> Radio<SPI, BUSY, RESET, D> {
    /// Returns the family of the chip.
    ///
    /// # Returns
    ///
    /// [`Family::Sx126x`] or [`Family::Sx127x`].
    pub fn family(&self) -> Family {
        match self {
            Radio::Sx126x(_) => Family::Sx126x,
            Radio::Sx127x(_) => Family::Sx127x,
        }
    }

    /// Returns the link settings the radio was last configured with.
    ///
    /// # Returns
    ///
    /// The settings, or `None` before [`configure`](Radio::configure) and after an SX126x
    /// has slept.
    pub fn link(&self) -> Option<LinkSettings> {
        match self {
            Radio::Sx126x(radio) => radio.config().map(|config| config.link),
            Radio::Sx127x(radio) => radio.config().map(|config| config.link),
        }
    }
}

impl<SPI, BUSY, RESET, D> Radio<SPI, BUSY, RESET, D>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    RESET: OutputPin,
    D: DelayNs,
{
    /// Resets the chip and sets up what its board wires around it.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Sx126x::init`] or [`Sx127x::init`]: most often that no chip
    /// of the family answered, which is a wiring or a power problem.
    pub fn init(&mut self) -> Result<(), RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => radio.init().map_err(RadioError::Sx126x),
            Radio::Sx127x(radio) => radio.init().map_err(RadioError::Sx127x),
        }
    }

    /// Tunes the chip to a configuration.
    ///
    /// The output power becomes the settings of the amplifier the board names, clamped to
    /// its range, and the sync word takes the family's own layout.
    ///
    /// # Arguments
    ///
    /// * `config` - the configuration.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Sx126x::configure`] or [`Sx127x::configure`], such as a
    /// bandwidth the chip does not have or a link an LLCC68 cannot carry.
    pub fn configure(&mut self, config: RadioConfig) -> Result<(), RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => {
                let power = radio.tx_power(config.output_dbm);
                let (low_hz, high_hz) = config.band_hz;
                let settings = sx126x::RadioConfig::new(config.frequency_hz, config.link, power)
                    .with_band(low_hz, high_hz)
                    .with_sync_word(config.sync_word.sx126x())
                    .with_inverted_iq(config.invert_iq_transmit, config.invert_iq_receive);
                radio.configure(settings).map_err(RadioError::Sx126x)
            }
            Radio::Sx127x(radio) => {
                let power = radio.tx_power(config.output_dbm);
                let settings = sx127x::RadioConfig::new(config.frequency_hz, config.link, power)
                    .with_sync_word(config.sync_word.sx127x())
                    .with_inverted_iq(config.invert_iq_transmit, config.invert_iq_receive);
                radio.configure(settings).map_err(RadioError::Sx127x)
            }
        }
    }

    /// Sends one frame and waits for it to leave.
    ///
    /// # Arguments
    ///
    /// * `payload` - the frame's payload, 1 to 255 bytes.
    ///
    /// # Returns
    ///
    /// The frame's airtime in microseconds, for a [`DutyCycle`](crate::duty::DutyCycle) to
    /// count.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Sx126x::transmit`] or [`Sx127x::transmit`].
    pub fn transmit(&mut self, payload: &[u8]) -> Result<u64, RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => radio.transmit(payload).map_err(RadioError::Sx126x),
            Radio::Sx127x(radio) => radio.transmit(payload).map_err(RadioError::Sx127x),
        }
    }

    /// Starts sending one frame and returns once the chip is transmitting.
    ///
    /// # Arguments
    ///
    /// * `payload` - the frame's payload, 1 to 255 bytes.
    ///
    /// # Returns
    ///
    /// The frame's airtime in microseconds, after which
    /// [`finish_transmit`](Radio::finish_transmit) reports it sent.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Sx126x::start_transmit`] or [`Sx127x::start_transmit`].
    pub fn start_transmit(&mut self, payload: &[u8]) -> Result<u64, RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => radio.start_transmit(payload).map_err(RadioError::Sx126x),
            Radio::Sx127x(radio) => radio.start_transmit(payload).map_err(RadioError::Sx127x),
        }
    }

    /// Reports whether the frame [`start_transmit`](Radio::start_transmit) began has left.
    ///
    /// # Returns
    ///
    /// `true` once it has been sent.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Sx126x::finish_transmit`] or [`Sx127x::finish_transmit`].
    pub fn finish_transmit(&mut self) -> Result<bool, RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => radio.finish_transmit().map_err(RadioError::Sx126x),
            Radio::Sx127x(radio) => radio.finish_transmit().map_err(RadioError::Sx127x),
        }
    }

    /// Listens for one frame.
    ///
    /// # Arguments
    ///
    /// * `buffer` - where the payload goes; up to 255 bytes are accepted.
    /// * `timeout_us` - how long to listen for a frame to start, in microseconds. The SX127x
    ///   counts it in symbols, from 4 to 1023.
    ///
    /// # Returns
    ///
    /// The frame's length and signal levels, or that the timeout passed or the frame was
    /// corrupt.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Sx126x::receive`] or [`Sx127x::receive`].
    pub fn receive(
        &mut self,
        buffer: &mut [u8],
        timeout_us: u64,
    ) -> Result<Reception, RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => radio
                .receive(buffer, timeout_us)
                .map(Reception::from)
                .map_err(RadioError::Sx126x),
            Radio::Sx127x(radio) => radio
                .receive(buffer, timeout_us)
                .map(Reception::from)
                .map_err(RadioError::Sx127x),
        }
    }

    /// Starts listening, frame after frame, until another mode is set.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Sx126x::listen`] or [`Sx127x::listen`].
    pub fn listen(&mut self) -> Result<(), RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => radio.listen().map_err(RadioError::Sx126x),
            Radio::Sx127x(radio) => radio.listen().map_err(RadioError::Sx127x),
        }
    }

    /// Takes the frame a [`listen`](Radio::listen) has received, if one has arrived.
    ///
    /// # Arguments
    ///
    /// * `buffer` - where the payload goes.
    ///
    /// # Returns
    ///
    /// The frame, a corrupt frame, or `None` when nothing has arrived.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Sx126x::take_frame`] or [`Sx127x::take_frame`].
    pub fn take_frame(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<Option<Reception>, RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => radio
                .take_frame(buffer)
                .map(|taken| taken.map(Reception::from))
                .map_err(RadioError::Sx126x),
            Radio::Sx127x(radio) => radio
                .take_frame(buffer)
                .map(|taken| taken.map(Reception::from))
                .map_err(RadioError::Sx127x),
        }
    }

    /// Puts the chip in standby, which stops a transmission or a reception.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of either driver.
    pub fn standby(&mut self) -> Result<(), RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => radio.standby().map_err(RadioError::Sx126x),
            Radio::Sx127x(radio) => radio.standby().map_err(RadioError::Sx127x),
        }
    }

    /// Puts the chip to sleep until the next call wakes it.
    ///
    /// An SX126x takes a warm start, keeping its settings in retention, and is configured
    /// again before its next frame. An SX127x keeps its registers and wakes for the next
    /// frame as it is.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of either driver.
    pub fn sleep(&mut self) -> Result<(), RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => radio.sleep(true).map_err(RadioError::Sx126x),
            Radio::Sx127x(radio) => radio.sleep().map_err(RadioError::Sx127x),
        }
    }

    /// Reads one register.
    ///
    /// # Arguments
    ///
    /// * `address` - the register address: 16 bits on the SX126x, 0x00 to 0x7F on the
    ///   SX127x.
    ///
    /// # Returns
    ///
    /// The register's value.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Address`] for an SX127x address past 0x7F, and the bus errors
    /// of either driver.
    pub fn read_register(&mut self, address: u16) -> Result<u8, RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => {
                let mut value = [0u8; 1];
                radio
                    .read_register(address, &mut value)
                    .map_err(RadioError::Sx126x)?;
                Ok(value[0])
            }
            Radio::Sx127x(radio) => radio
                .read_register(sx127x_address(address)?)
                .map_err(RadioError::Sx127x),
        }
    }

    /// Writes one register.
    ///
    /// # Arguments
    ///
    /// * `address` - the register address: 16 bits on the SX126x, 0x00 to 0x7F on the
    ///   SX127x.
    /// * `value` - the value to write.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Address`] for an SX127x address past 0x7F, and the bus errors
    /// of either driver.
    pub fn write_register(
        &mut self,
        address: u16,
        value: u8,
    ) -> Result<(), RadioError<SPI::Error>> {
        match self {
            Radio::Sx126x(radio) => radio
                .write_register(address, &[value])
                .map_err(RadioError::Sx126x),
            Radio::Sx127x(radio) => radio
                .write_register(sx127x_address(address)?, value)
                .map_err(RadioError::Sx127x),
        }
    }
}

fn sx127x_address<E>(address: u16) -> Result<u8, RadioError<E>> {
    if address > SX127X_LAST_REGISTER {
        return Err(RadioError::Address(address));
    }
    Ok(address as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pamoja_hal::script::{DelayLog, PinScript, SpiScript, SpiStep};

    type ScriptedRadio = Radio<SpiScript, PinScript, PinScript, DelayLog>;

    fn sx126x_radio(steps: Vec<SpiStep>, board: sx126x::Board) -> ScriptedRadio {
        let mut busy = PinScript::new([]);
        busy.set_low().expect("a scripted line takes any level");
        Radio::from(Sx126x::new(
            SpiScript::new(steps),
            busy,
            PinScript::new([]),
            DelayLog::new(),
            board,
        ))
    }

    fn sx127x_radio(steps: Vec<SpiStep>) -> ScriptedRadio {
        Radio::from(Sx127x::new(
            SpiScript::new(steps),
            PinScript::new([]),
            DelayLog::new(),
            sx127x::Board::new(sx127x::config::PaOutput::PaBoost),
        ))
    }

    fn done(radio: ScriptedRadio) -> bool {
        match radio {
            Radio::Sx126x(radio) => radio.release().0.done(),
            Radio::Sx127x(radio) => radio.release().0.done(),
        }
    }

    fn wrote(address: u8, values: &[u8]) -> [SpiStep; 2] {
        [
            SpiStep::write([address | 0x80]),
            SpiStep::write(values.to_vec()),
        ]
    }

    fn read(address: u8, values: &[u8]) -> [SpiStep; 2] {
        [SpiStep::write([address]), SpiStep::read(values.to_vec())]
    }

    #[test]
    fn a_sync_word_byte_takes_each_familys_layout() {
        assert_eq!(
            SyncWord::Public.sx126x().to_bytes(),
            SyncWord::Custom(0x34).sx126x().to_bytes()
        );
        assert_eq!(
            SyncWord::Private.sx126x().to_bytes(),
            SyncWord::Custom(0x12).sx126x().to_bytes()
        );
        assert_eq!(SyncWord::Custom(0x2B).sx126x().to_bytes(), [0x24, 0xB4]);
        assert_eq!(SyncWord::Custom(0x2B).sx127x().to_byte(), 0x2B);
        for byte in [0x00, 0x12, 0x2B, 0x34, 0xFF] {
            assert_eq!(SyncWord::from_byte(byte).to_byte(), byte);
        }
    }

    #[test]
    fn an_sx127x_config_reaches_the_registers_with_the_boards_output() {
        let mut steps = Vec::new();
        steps.extend(wrote(0x01, &[0x88]));
        steps.extend(wrote(0x01, &[0x08]));
        steps.extend(wrote(0x01, &[0x09]));
        steps.extend(wrote(0x09, &[0x00]));
        steps.extend(wrote(0x06, &[0xD9, 0x06, 0x66]));
        steps.extend(read(0x3B, &[0x82]));
        steps.extend(wrote(0x3B, &[0x42]));
        steps.extend(read(0x3B, &[0x22]));
        steps.extend(read(0x3B, &[0x02]));
        steps.extend(wrote(0x01, &[0x08]));
        steps.extend(wrote(0x01, &[0x88]));
        steps.extend(wrote(0x01, &[0x89]));
        steps.extend(wrote(0x09, &[0xFC]));
        steps.extend(wrote(0x4D, &[0x84]));
        steps.extend(wrote(0x0B, &[0x2B]));
        steps.extend(wrote(0x1D, &[0x72]));
        steps.extend(wrote(0x1E, &[0x74]));
        steps.extend(wrote(0x26, &[0x04]));
        steps.extend(wrote(0x20, &[0x00, 0x08]));
        steps.extend(read(0x31, &[0xC3]));
        steps.extend(wrote(0x31, &[0xC3]));
        steps.extend(wrote(0x37, &[0x0A]));
        steps.extend(wrote(0x36, &[0x03]));
        steps.extend(wrote(0x23, &[0xFF]));
        steps.extend(wrote(0x39, &[0x34]));
        let mut radio = sx127x_radio(steps);

        let config =
            RadioConfig::new(868_100_000, LinkSettings::new(7, 125_000), 14).lorawan_device();
        radio.configure(config).expect("configures");

        assert_eq!(radio.link(), Some(LinkSettings::new(7, 125_000)));
        assert!(done(radio));
    }

    #[test]
    fn an_sx126x_config_sends_the_commands_and_the_spread_sync_word() {
        use sx126x::command;
        use sx126x::config::{self as chip, LoraModulation, PacketType, RampTime, StandbyMode};

        let link = LinkSettings::new(7, 125_000);
        let board = sx126x::Board::new(chip::PowerAmplifier::HighPower);
        let power = chip::TxPower::for_output(chip::PowerAmplifier::HighPower, 14);
        let modulation = LoraModulation::from_link(&link).expect("125 kHz is an SX126x bandwidth");
        let image = chip::image_calibration(863_000_000, 870_000_000);
        let steps = vec![
            SpiStep::write(command::set_standby(StandbyMode::Rc).as_bytes().to_vec()),
            SpiStep::write(
                command::set_packet_type(PacketType::Lora)
                    .as_bytes()
                    .to_vec(),
            ),
            SpiStep::write(command::calibrate_image(image).as_bytes().to_vec()),
            SpiStep::write(
                command::set_rf_frequency(chip::frequency_word(868_100_000))
                    .as_bytes()
                    .to_vec(),
            ),
            SpiStep::write(command::set_pa_config(power.pa).as_bytes().to_vec()),
            SpiStep::write(
                command::set_tx_params(power.setting_dbm, RampTime::Us40)
                    .as_bytes()
                    .to_vec(),
            ),
            SpiStep::write(
                command::set_lora_modulation_params(modulation)
                    .as_bytes()
                    .to_vec(),
            ),
            SpiStep::write([0x0D, 0x07, 0x40]),
            SpiStep::write([0x24, 0xB4]),
        ];
        let mut radio = sx126x_radio(steps, board);

        let config = RadioConfig::new(868_100_000, link, 14)
            .with_band(863_000_000, 870_000_000)
            .with_sync_word(SyncWord::Custom(0x2B));
        radio.configure(config).expect("configures");

        assert_eq!(radio.family(), Family::Sx126x);
        assert!(done(radio));
    }

    #[test]
    fn a_chip_refusal_arrives_wrapped_in_its_family() {
        let board = sx126x::Board::new(sx126x::config::PowerAmplifier::HighPower).with_llcc68();
        let mut radio = sx126x_radio(Vec::new(), board);
        let config = RadioConfig::new(868_100_000, LinkSettings::new(10, 125_000), 14);

        let refused = radio.configure(config);

        assert_eq!(
            refused,
            Err(RadioError::Sx126x(sx126x::RadioError::Llcc68 {
                spreading_factor: 10,
                bandwidth_hz: 125_000,
            }))
        );
        assert!(refused.expect_err("refused").to_string().contains("LLCC68"));
    }

    #[test]
    fn nothing_is_sent_before_a_configuration() {
        let mut radio = sx127x_radio(Vec::new());
        assert_eq!(
            radio.transmit(b"21.5"),
            Err(RadioError::Sx127x(sx127x::RadioError::NotConfigured))
        );
        assert_eq!(radio.link(), None);
    }

    #[test]
    fn registers_reach_either_family_and_the_sx127x_map_ends_at_0x7f() {
        let mut steps = Vec::new();
        steps.extend(wrote(0x39, &[0x2B]));
        steps.extend(read(0x39, &[0x2B]));
        let mut radio = sx127x_radio(steps);
        radio.write_register(0x39, 0x2B).expect("writes");
        assert_eq!(radio.read_register(0x39), Ok(0x2B));
        assert_eq!(
            radio.write_register(0x80, 0),
            Err(RadioError::Address(0x80))
        );
        assert!(done(radio));

        let board = sx126x::Board::new(sx126x::config::PowerAmplifier::HighPower);
        let steps = vec![
            SpiStep::write([0x1D, 0x07, 0x40, 0x00]),
            SpiStep::read([0x14]),
        ];
        let mut radio = sx126x_radio(steps, board);
        assert_eq!(radio.read_register(0x0740), Ok(0x14));
        assert!(done(radio));
    }
}
