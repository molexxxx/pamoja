//! The SX127x driven over embedded-hal in LoRa mode: reset, calibrate, configure, transmit,
//! and receive.
//!
//! [`Sx127x`] owns the SPI device, the NRESET line, and a delay. It reads and writes the
//! registers of [`register`](super::register), reads RegIrqFlags to learn when a frame has
//! gone out or come in, and follows the transmit and receive sequences of the SX1276/77/78/79
//! datasheet (Rev 7), with the errata applied as Semtech's LoRaMac-node applies them.

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{self, OutputPin};
use embedded_hal::spi::{Operation, SpiDevice};
use pamoja_lora::budget::Decibels;
use pamoja_lora::LinkSettings;

use super::config::{
    self, automatic_if, frequency_bytes, high_bw_optimize, image_cal_start, invert_iq, invert_iq_2,
    preamble_bytes, spurious_reception, LoraModulation, ModulationError, PaOutput, SyncWord,
    TxPower,
};
use super::irq::IrqFlags;
use super::register::{self, fsk_op_mode, lora_op_mode, read_address, write_address, Mode};
use super::status::{rssi_dbm, ModemStatus, PacketStatus, Port};

/// How long NRESET is held low, in microseconds. The datasheet's Manual Reset section asks for
/// a hundred microseconds; this is the 1 ms Semtech's LoRaMac-node holds it for.
pub const RESET_HOLD_US: u32 = 1_000;

/// How long to wait after releasing NRESET, in microseconds. The datasheet asks for 5 ms;
/// this is the 6 ms LoRaMac-node waits.
pub const RESET_SETTLE_US: u32 = 6_000;

/// How often the interrupt flags are read while a frame goes out or comes in, in
/// microseconds.
pub const IRQ_POLL_US: u32 = 1_000;

/// How much longer than a frame's airtime a transmission may take before the driver gives
/// up on it, in microseconds.
pub const TIMEOUT_MARGIN_US: u64 = 1_000_000;

/// How often RegImageCal is read while a calibration runs, in microseconds.
pub const CALIBRATION_POLL_US: u32 = 1_000;

/// How long a calibration may run before the driver gives up, in microseconds. The datasheet
/// says it takes about 10 ms.
pub const CALIBRATION_LIMIT_US: u32 = 100_000;

/// How a module wires its SX127x: the amplifier output on its antenna and its clock.
///
/// The SPI interface cannot see which output a module uses, so it comes from the module's
/// schematic.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::config::PaOutput;
/// use pamoja_radios::sx127x::Board;
///
/// // An RFM95W: the antenna on PA_BOOST and a crystal.
/// let board = Board::new(PaOutput::PaBoost);
/// assert!(!board.tcxo);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Board {
    /// The amplifier output the antenna is on.
    pub output: PaOutput,
    /// Whether a TCXO drives the XTA pin in place of a crystal.
    pub tcxo: bool,
}

impl Board {
    /// A module with a crystal.
    ///
    /// # Arguments
    ///
    /// * `output` - the amplifier output the antenna is on.
    ///
    /// # Returns
    ///
    /// The board.
    pub const fn new(output: PaOutput) -> Board {
        Board {
            output,
            tcxo: false,
        }
    }

    /// Returns the board clocked by a TCXO on XTA.
    ///
    /// # Returns
    ///
    /// The board.
    pub const fn with_tcxo(mut self) -> Board {
        self.tcxo = true;
        self
    }
}

/// What a radio sends and listens with: the carrier, the LoRa link, the power, and the sync
/// word and IQ polarity that keep one network apart from another.
///
/// # Examples
///
/// ```
/// use pamoja_lora::LinkSettings;
/// use pamoja_radios::sx127x::config::{PaOutput, SyncWord, TxPower};
/// use pamoja_radios::sx127x::RadioConfig;
///
/// // A LoRaWAN device on 868.1 MHz at SF9 with 14 dBm on PA_BOOST.
/// let config = RadioConfig::new(
///     868_100_000,
///     LinkSettings::new(9, 125_000),
///     TxPower::for_output(PaOutput::PaBoost, 14),
/// )
/// .lorawan_device();
/// assert_eq!(config.sync_word, SyncWord::Public);
/// assert!(config.invert_iq_receive && !config.invert_iq_transmit);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadioConfig {
    /// The carrier frequency in hertz.
    pub frequency_hz: u32,
    /// The LoRa link settings frames are sent and received with.
    pub link: LinkSettings,
    /// The amplifier settings.
    pub power: TxPower,
    /// The sync word.
    pub sync_word: SyncWord,
    /// Whether transmitted frames have inverted IQ.
    pub invert_iq_transmit: bool,
    /// Whether received frames are expected with inverted IQ.
    pub invert_iq_receive: bool,
}

impl RadioConfig {
    /// A configuration with the private sync word and standard IQ both ways.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the carrier frequency in hertz.
    /// * `link` - the LoRa link settings.
    /// * `power` - the amplifier settings.
    ///
    /// # Returns
    ///
    /// The configuration.
    pub const fn new(frequency_hz: u32, link: LinkSettings, power: TxPower) -> RadioConfig {
        RadioConfig {
            frequency_hz,
            link,
            power,
            sync_word: SyncWord::Private,
            invert_iq_transmit: false,
            invert_iq_receive: false,
        }
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

    /// Returns the configuration with the IQ polarity of each direction set.
    ///
    /// # Arguments
    ///
    /// * `transmit` - whether transmitted frames have inverted IQ.
    /// * `receive` - whether received frames have inverted IQ.
    ///
    /// # Returns
    ///
    /// The configuration.
    pub const fn with_inverted_iq(mut self, transmit: bool, receive: bool) -> RadioConfig {
        self.invert_iq_transmit = transmit;
        self.invert_iq_receive = receive;
        self
    }

    /// Returns the configuration for a LoRaWAN end device: the public sync word, standard IQ
    /// on uplinks, and inverted IQ on downlinks.
    ///
    /// # Returns
    ///
    /// The configuration.
    pub const fn lorawan_device(self) -> RadioConfig {
        self.with_sync_word(SyncWord::Public)
            .with_inverted_iq(false, true)
    }
}

/// How a reception ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reception {
    /// A frame whose CRC checked, or that carried none.
    Frame {
        /// The payload length; the payload is at the start of the buffer.
        len: usize,
        /// The signal levels the frame arrived with.
        status: PacketStatus,
    },
    /// No preamble arrived before the timeout.
    Timeout,
    /// A frame arrived whose payload CRC failed, and was dropped.
    Corrupt,
}

/// What can go wrong driving an SX127x.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioError<E> {
    /// The SPI device failed.
    Spi(E),
    /// The NRESET line could not be driven.
    Pin(digital::ErrorKind),
    /// RegVersion held this instead of 0x12, so no SX1276, SX1277, SX1278, or SX1279, or a
    /// miswired one, is on the bus.
    Absent(u8),
    /// The link settings are not ones the SX127x can use at the carrier.
    Modulation(ModulationError),
    /// The power settings are for the other amplifier output than the board wires.
    Output,
    /// A payload the chip cannot send: longer than 255 bytes, or empty, with its length.
    PayloadLength(usize),
    /// A received payload of this many bytes does not fit the buffer given.
    BufferTooSmall(usize),
    /// A transmission or a reception was asked for before [`Sx127x::configure`].
    NotConfigured,
    /// The image calibration was still running after [`CALIBRATION_LIMIT_US`].
    Calibration,
    /// The expected interrupt did not arrive in the time the driver allows.
    NoInterrupt,
}

impl<E: core::fmt::Debug> core::fmt::Display for RadioError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RadioError::Spi(error) => write!(f, "SPI error: {error:?}"),
            RadioError::Pin(kind) => write!(f, "NRESET line error: {kind:?}"),
            RadioError::Absent(version) => {
                write!(f, "no SX127x answered: RegVersion read {version:#04x}")
            }
            RadioError::Modulation(ModulationError::Bandwidth(hz)) => {
                write!(
                    f,
                    "the SX127x has no {hz} Hz LoRa bandwidth at this carrier"
                )
            }
            RadioError::Modulation(ModulationError::SpreadingFactor(sf)) => {
                write!(f, "the SX127x has no SF{sf}")
            }
            RadioError::Modulation(ModulationError::ExplicitHeaderAtSf6) => {
                write!(f, "SF6 needs an implicit header")
            }
            RadioError::Output => {
                write!(f, "the power settings are for the other amplifier output")
            }
            RadioError::PayloadLength(len) => {
                write!(f, "a LoRa frame carries 1 to 255 bytes, not {len}")
            }
            RadioError::BufferTooSmall(len) => {
                write!(f, "a {len} byte payload does not fit the buffer")
            }
            RadioError::NotConfigured => write!(f, "the radio has not been configured"),
            RadioError::Calibration => write!(f, "the image calibration did not finish"),
            RadioError::NoInterrupt => write!(f, "the radio raised no interrupt in time"),
        }
    }
}

impl<E: core::fmt::Debug> core::error::Error for RadioError<E> {}

fn pin<E, P: digital::Error>(error: P) -> RadioError<E> {
    RadioError::Pin(error.kind())
}

/// A Semtech SX1276, SX1277, SX1278, or SX1279 on an SPI bus, with its NRESET line.
///
/// [`init`](Sx127x::init) resets the chip and puts it in LoRa mode,
/// [`configure`](Sx127x::configure) calibrates its receiver and tunes it to a
/// [`RadioConfig`], and [`transmit`](Sx127x::transmit) and [`receive`](Sx127x::receive)
/// send and wait for one frame each. For anything those do not cover, the register and data
/// buffer methods reach the chip directly.
///
/// # Examples
///
/// The chip's side of initialization, scripted: an RFM95W that answers RegVersion with 0x12.
///
/// ```
/// use pamoja_hal::digital::PinState;
/// use pamoja_hal::script::{DelayLog, PinScript, SpiScript, SpiStep};
/// use pamoja_radios::sx127x::config::PaOutput;
/// use pamoja_radios::sx127x::{Board, Sx127x};
///
/// let spi = SpiScript::new([
///     SpiStep::write([0x42]),
///     SpiStep::read([0x12]),
///     SpiStep::write([0x81]),
///     SpiStep::write([0x08]),
///     SpiStep::write([0x81]),
///     SpiStep::write([0x88]),
///     SpiStep::write([0x81]),
///     SpiStep::write([0x89]),
///     SpiStep::write([0x8C]),
///     SpiStep::write([0x23]),
/// ]);
/// let board = Board::new(PaOutput::PaBoost);
///
/// let mut radio = Sx127x::new(spi, PinScript::new([]), DelayLog::new(), board);
/// radio.init().expect("the scripted RFM95W answers");
///
/// let (spi, reset, _) = radio.release();
/// assert!(spi.done());
/// assert_eq!(reset.driven(), [PinState::Low, PinState::High]);
/// ```
pub struct Sx127x<SPI, RESET, D> {
    spi: SPI,
    reset: RESET,
    delay: D,
    board: Board,
    config: Option<RadioConfig>,
    tuned_hz: Option<u32>,
    calibrated_high: bool,
    calibrated_low: bool,
}

impl<SPI, RESET, D> Sx127x<SPI, RESET, D> {
    /// Wraps a chip's SPI device and NRESET line. Nothing is sent until
    /// [`init`](Sx127x::init).
    ///
    /// # Arguments
    ///
    /// * `spi` - the SPI device, with NSS as its chip select.
    /// * `reset` - the NRESET line, as an output.
    /// * `delay` - a delay for the reset pulse and the polling.
    /// * `board` - how the module wires the chip.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn new(spi: SPI, reset: RESET, delay: D, board: Board) -> Self {
        Sx127x {
            spi,
            reset,
            delay,
            board,
            config: None,
            tuned_hz: None,
            calibrated_high: false,
            calibrated_low: false,
        }
    }

    /// Returns how the module wires the chip.
    ///
    /// # Returns
    ///
    /// The board.
    pub fn board(&self) -> Board {
        self.board
    }

    /// Returns the configuration the chip was last tuned to.
    ///
    /// # Returns
    ///
    /// The configuration, or `None` before [`configure`](Sx127x::configure) or after a
    /// reset.
    pub fn config(&self) -> Option<&RadioConfig> {
        self.config.as_ref()
    }

    /// Returns the power settings for an output power on this board's amplifier output.
    ///
    /// # Arguments
    ///
    /// * `output_dbm` - the output power wanted at the antenna port.
    ///
    /// # Returns
    ///
    /// The amplifier settings.
    pub fn tx_power(&self, output_dbm: i8) -> TxPower {
        TxPower::for_output(self.board.output, output_dbm)
    }

    /// Gives back the SPI device, the line, and the delay.
    ///
    /// # Returns
    ///
    /// The SPI device, NRESET, and the delay.
    pub fn release(self) -> (SPI, RESET, D) {
        (self.spi, self.reset, self.delay)
    }
}

impl<SPI, RESET, D> Sx127x<SPI, RESET, D>
where
    SPI: SpiDevice,
    RESET: OutputPin,
    D: DelayNs,
{
    /// Resets the chip and puts it in LoRa mode.
    ///
    /// NRESET is pulsed low and RegVersion read. The chip, which comes out of reset as an
    /// FSK radio in standby, is put to sleep, clocked from a TCXO if the board has one, and
    /// switched to LoRa, which it only allows in sleep. It then goes to standby with the LNA
    /// at LoRaMac-node's setting.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Absent`] if RegVersion is not 0x12, and [`RadioError::Spi`] or
    /// [`RadioError::Pin`] if the bus or the line fails.
    pub fn init(&mut self) -> Result<(), RadioError<SPI::Error>> {
        self.config = None;
        self.tuned_hz = None;
        self.calibrated_high = false;
        self.calibrated_low = false;
        self.reset.set_low().map_err(pin)?;
        self.delay.delay_us(RESET_HOLD_US);
        self.reset.set_high().map_err(pin)?;
        self.delay.delay_us(RESET_SETTLE_US);

        let version = self.version()?;
        if version != register::VERSION_SX1276 {
            return Err(RadioError::Absent(version));
        }
        self.write_register(register::OP_MODE, fsk_op_mode(Mode::Sleep))?;
        if self.board.tcxo {
            self.write_register(register::TCXO, config::TCXO_INPUT_ON)?;
        }
        self.write_register(register::OP_MODE, lora_op_mode(Mode::Sleep))?;
        self.write_register(register::OP_MODE, lora_op_mode(Mode::Standby))?;
        self.write_register(register::LNA, config::LNA_BOOSTED)
    }

    /// Tunes the chip to a configuration.
    ///
    /// The first time a carrier on each RF port is configured, the receiver's image and RSSI
    /// calibration runs there, as the datasheet's Image and RSSI Calibration section advises,
    /// since the calibration at reset only covers the low frequency port at 434 MHz. The chip
    /// then takes the frequency, the amplifier, the modem settings, the preamble, the SF6
    /// detection settings, the 500 kHz erratum, the longest payload, and the sync word.
    ///
    /// # Arguments
    ///
    /// * `config` - the configuration.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Modulation`] if the link does not fit the chip at the carrier,
    /// [`RadioError::Output`] if the power settings are for the other output,
    /// [`RadioError::Calibration`] if the calibration does not finish, and the bus errors of
    /// [`write_register`](Sx127x::write_register).
    pub fn configure(&mut self, config: RadioConfig) -> Result<(), RadioError<SPI::Error>> {
        let modulation = LoraModulation::from_link(&config.link).map_err(RadioError::Modulation)?;
        if !modulation.bandwidth.in_band(config.frequency_hz) {
            return Err(RadioError::Modulation(ModulationError::Bandwidth(
                config.link.bandwidth_hz(),
            )));
        }
        if config.power.output() != self.board.output {
            return Err(RadioError::Output);
        }
        let calibrated = match Port::for_frequency(config.frequency_hz) {
            Port::High => self.calibrated_high,
            Port::Low => self.calibrated_low,
        };
        if calibrated {
            self.standby()?;
        } else {
            self.calibrate(config.frequency_hz)?;
        }
        self.tune(config.frequency_hz)?;
        self.write_register(register::PA_CONFIG, config.power.pa_config)?;
        self.write_register(register::PA_DAC, config.power.pa_dac)?;
        self.write_register(register::OCP, config.power.ocp)?;
        self.write_register(register::MODEM_CONFIG_1, modulation.modem_config_1())?;
        self.write_register(register::MODEM_CONFIG_2, modulation.modem_config_2(0))?;
        self.write_register(register::MODEM_CONFIG_3, modulation.modem_config_3())?;
        self.write_registers(register::PREAMBLE_MSB, &preamble_bytes(&config.link))?;
        self.update_register(register::DETECT_OPTIMIZE, |value| {
            modulation.detect_optimize(value)
        })?;
        self.write_register(
            register::DETECTION_THRESHOLD,
            modulation.detection_threshold(),
        )?;
        let (optimize_1, optimize_2) = high_bw_optimize(modulation.bandwidth, config.frequency_hz);
        self.write_register(register::HIGH_BW_OPTIMIZE_1, optimize_1)?;
        if let Some(optimize_2) = optimize_2 {
            self.write_register(register::HIGH_BW_OPTIMIZE_2, optimize_2)?;
        }
        self.write_register(register::MAX_PAYLOAD_LENGTH, 0xFF)?;
        self.write_register(register::SYNC_WORD, config.sync_word.to_byte())?;
        self.config = Some(config);
        Ok(())
    }

    /// Sends one frame and waits for it to leave.
    ///
    /// This is [`start_transmit`](Sx127x::start_transmit), a wait for the frame's airtime,
    /// and [`finish_transmit`](Sx127x::finish_transmit) read every [`IRQ_POLL_US`] until the
    /// chip reports the frame sent.
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
    /// Returns the errors of [`start_transmit`](Sx127x::start_transmit), and
    /// [`RadioError::NoInterrupt`] if TxDone does not arrive within the airtime and
    /// [`TIMEOUT_MARGIN_US`] twice over.
    pub fn transmit(&mut self, payload: &[u8]) -> Result<u64, RadioError<SPI::Error>> {
        let airtime_us = self.start_transmit(payload)?;
        let limit_us = airtime_us.saturating_add(2 * TIMEOUT_MARGIN_US);
        let mut waited_us = airtime_us;
        self.pause_us(airtime_us);
        while !self.finish_transmit()? {
            if waited_us >= limit_us {
                return Err(RadioError::NoInterrupt);
            }
            self.delay.delay_us(IRQ_POLL_US);
            waited_us = waited_us.saturating_add(u64::from(IRQ_POLL_US));
        }
        Ok(airtime_us)
    }

    /// Starts sending one frame and returns once the chip is transmitting.
    ///
    /// The steps are the datasheet's transmit sequence: standby, the IQ polarity, the payload
    /// length, the data buffer pointer at the transmit base, the payload into RegFifo, TxDone
    /// on DIO0, the interrupt flags cleared, and TX mode, after which the chip returns to
    /// standby by itself.
    ///
    /// # Arguments
    ///
    /// * `payload` - the frame's payload, 1 to 255 bytes.
    ///
    /// # Returns
    ///
    /// The frame's airtime in microseconds.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::NotConfigured`] before [`configure`](Sx127x::configure),
    /// [`RadioError::PayloadLength`] for an empty payload or one past 255 bytes, and the bus
    /// errors of [`write_register`](Sx127x::write_register).
    pub fn start_transmit(&mut self, payload: &[u8]) -> Result<u64, RadioError<SPI::Error>> {
        let settings = self.config.ok_or(RadioError::NotConfigured)?;
        let len = u8::try_from(payload.len())
            .ok()
            .filter(|len| *len > 0)
            .ok_or(RadioError::PayloadLength(payload.len()))?;
        let invert = settings.invert_iq_transmit;

        self.standby()?;
        self.write_register(register::INVERT_IQ, invert_iq(false, invert))?;
        self.write_register(register::INVERT_IQ_2, invert_iq_2(invert))?;
        self.tune(settings.frequency_hz)?;
        self.write_register(register::PAYLOAD_LENGTH, len)?;
        self.write_register(register::FIFO_TX_BASE_ADDR, 0)?;
        self.write_register(register::FIFO_ADDR_PTR, 0)?;
        self.write_registers(register::FIFO, payload)?;
        self.write_register(register::DIO_MAPPING_1, config::DIO0_TX_DONE)?;
        self.write_register(register::IRQ_FLAGS, IrqFlags::ALL.bits())?;
        self.write_register(register::OP_MODE, lora_op_mode(Mode::Tx))?;
        Ok(settings.link.airtime_us(payload.len()))
    }

    /// Reports whether the frame [`start_transmit`](Sx127x::start_transmit) began has left.
    ///
    /// RegIrqFlags is read once, and on TxDone that flag is cleared.
    ///
    /// # Returns
    ///
    /// `true` once the frame has been sent, `false` while it is still going out.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of [`write_register`](Sx127x::write_register).
    pub fn finish_transmit(&mut self) -> Result<bool, RadioError<SPI::Error>> {
        let flags = self.irq_flags()?;
        if !flags.contains(IrqFlags::TX_DONE) {
            return Ok(false);
        }
        self.write_register(register::IRQ_FLAGS, IrqFlags::TX_DONE.bits())?;
        Ok(true)
    }

    /// Listens for one frame in RXSINGLE mode.
    ///
    /// The receiver is prepared as for [`listen`](Sx127x::listen), the symbol timeout is set
    /// from `timeout_us`, and RegIrqFlags is read until RxDone or RxTimeout. The chip returns
    /// to standby by itself either way.
    ///
    /// # Arguments
    ///
    /// * `buffer` - where the payload goes.
    /// * `timeout_us` - how long to listen for a preamble, in microseconds, which the chip
    ///   counts in symbols from 4 to 1023; a longer timeout ends at 1023 symbols.
    ///
    /// # Returns
    ///
    /// The frame's length and levels, or that the timeout passed or the frame was corrupt.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::NotConfigured`] before [`configure`](Sx127x::configure),
    /// [`RadioError::BufferTooSmall`] if the payload does not fit, [`RadioError::NoInterrupt`]
    /// if the chip never answers, and the bus errors of
    /// [`write_register`](Sx127x::write_register).
    pub fn receive(
        &mut self,
        buffer: &mut [u8],
        timeout_us: u64,
    ) -> Result<Reception, RadioError<SPI::Error>> {
        let settings = self.config.ok_or(RadioError::NotConfigured)?;
        let modulation =
            LoraModulation::from_link(&settings.link).map_err(RadioError::Modulation)?;
        self.prepare_reception(&settings, &modulation)?;
        let symbols = config::symbol_timeout(&settings.link, timeout_us);
        self.write_register(register::MODEM_CONFIG_2, modulation.modem_config_2(symbols))?;
        self.write_register(register::SYMB_TIMEOUT_LSB, symbols as u8)?;
        self.write_register(register::OP_MODE, lora_op_mode(Mode::RxSingle))?;

        let window_us = u64::from(symbols) * settings.link.symbol_time_us();
        let limit_us = window_us
            .saturating_add(settings.link.airtime_us(usize::from(u8::MAX)))
            .saturating_add(TIMEOUT_MARGIN_US);
        let flags = self.wait_for(IrqFlags::RX_DONE | IrqFlags::RX_TIMEOUT, limit_us)?;
        self.write_register(register::IRQ_FLAGS, flags.bits())?;
        if !flags.contains(IrqFlags::RX_DONE) {
            return Ok(Reception::Timeout);
        }
        if flags.contains(IrqFlags::PAYLOAD_CRC_ERROR) {
            return Ok(Reception::Corrupt);
        }
        self.read_frame(buffer, &settings)
    }

    /// Starts listening in RXCONTINUOUS mode, so the chip receives frame after frame until
    /// another mode is set.
    ///
    /// The steps are the datasheet's receive sequence with erratum 2.3 applied: standby, the
    /// IQ polarity, the IF and the carrier offset of the erratum, the data buffer pointer at
    /// the receive base, RxDone on DIO0, the interrupt flags cleared, and RXCONTINUOUS. Each
    /// frame is read with [`take_frame`](Sx127x::take_frame).
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::NotConfigured`] before [`configure`](Sx127x::configure), and the
    /// bus errors of [`write_register`](Sx127x::write_register).
    pub fn listen(&mut self) -> Result<(), RadioError<SPI::Error>> {
        let settings = self.config.ok_or(RadioError::NotConfigured)?;
        let modulation =
            LoraModulation::from_link(&settings.link).map_err(RadioError::Modulation)?;
        self.prepare_reception(&settings, &modulation)?;
        self.write_register(register::OP_MODE, lora_op_mode(Mode::RxContinuous))
    }

    /// Takes the frame a [`listen`](Sx127x::listen) has received, if one has arrived.
    ///
    /// RegIrqFlags is read once. On RxDone the reception flags are cleared, and a frame whose
    /// CRC checked is copied out of the data buffer from its start address with its signal
    /// levels while the chip goes on listening.
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
    /// Returns [`RadioError::NotConfigured`] before [`configure`](Sx127x::configure),
    /// [`RadioError::BufferTooSmall`] if the payload does not fit, and the bus errors of
    /// [`write_register`](Sx127x::write_register).
    pub fn take_frame(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<Option<Reception>, RadioError<SPI::Error>> {
        let settings = self.config.ok_or(RadioError::NotConfigured)?;
        let flags = self.irq_flags()?;
        if !flags.contains(IrqFlags::RX_DONE) {
            return Ok(None);
        }
        let received = IrqFlags::RX_DONE | IrqFlags::PAYLOAD_CRC_ERROR | IrqFlags::VALID_HEADER;
        self.write_register(register::IRQ_FLAGS, (flags & received).bits())?;
        if flags.contains(IrqFlags::PAYLOAD_CRC_ERROR) {
            return Ok(Some(Reception::Corrupt));
        }
        self.read_frame(buffer, &settings).map(Some)
    }

    /// Puts the chip in standby, which stops a transmission or a reception.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of [`write_register`](Sx127x::write_register).
    pub fn standby(&mut self) -> Result<(), RadioError<SPI::Error>> {
        self.write_register(register::OP_MODE, lora_op_mode(Mode::Standby))
    }

    /// Puts the chip to sleep, where it keeps its registers but loses the data buffer.
    ///
    /// The configuration survives sleep, so the next transmission or reception wakes the chip
    /// by setting its mode.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of [`write_register`](Sx127x::write_register).
    pub fn sleep(&mut self) -> Result<(), RadioError<SPI::Error>> {
        self.write_register(register::OP_MODE, lora_op_mode(Mode::Sleep))
    }

    /// Reads RegVersion.
    ///
    /// # Returns
    ///
    /// The silicon revision, 0x12 for the SX1276 family.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Spi`] if the SPI device fails.
    pub fn version(&mut self) -> Result<u8, RadioError<SPI::Error>> {
        self.read_register(register::VERSION)
    }

    /// Reads the raised interrupts.
    ///
    /// # Returns
    ///
    /// RegIrqFlags.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Spi`] if the SPI device fails.
    pub fn irq_flags(&mut self) -> Result<IrqFlags, RadioError<SPI::Error>> {
        self.read_register(register::IRQ_FLAGS)
            .map(IrqFlags::from_bits)
    }

    /// Reads the live state of the LoRa modem.
    ///
    /// # Returns
    ///
    /// RegModemStat, decoded.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Spi`] if the SPI device fails.
    pub fn modem_status(&mut self) -> Result<ModemStatus, RadioError<SPI::Error>> {
        self.read_register(register::MODEM_STAT)
            .map(ModemStatus::from_byte)
    }

    /// Reads the signal power the receiver hears right now, while it listens.
    ///
    /// # Returns
    ///
    /// The RSSI in dBm, with the offset of the port the configured carrier uses.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::NotConfigured`] before [`configure`](Sx127x::configure), and
    /// [`RadioError::Spi`] if the SPI device fails.
    pub fn rssi(&mut self) -> Result<Decibels, RadioError<SPI::Error>> {
        let settings = self.config.ok_or(RadioError::NotConfigured)?;
        let byte = self.read_register(register::RSSI_VALUE)?;
        Ok(rssi_dbm(byte, Port::for_frequency(settings.frequency_hz)))
    }

    /// Reads one register.
    ///
    /// # Arguments
    ///
    /// * `address` - the register address.
    ///
    /// # Returns
    ///
    /// The register value.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Spi`] if the SPI device fails.
    pub fn read_register(&mut self, address: u8) -> Result<u8, RadioError<SPI::Error>> {
        let mut value = [0u8; 1];
        self.read_registers(address, &mut value)?;
        Ok(value[0])
    }

    /// Writes one register.
    ///
    /// # Arguments
    ///
    /// * `address` - the register address.
    /// * `value` - the value.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Spi`] if the SPI device fails.
    pub fn write_register(&mut self, address: u8, value: u8) -> Result<(), RadioError<SPI::Error>> {
        self.write_registers(address, &[value])
    }

    /// Reads consecutive registers in one transaction, or bytes out of the data buffer when
    /// `address` is [`register::FIFO`].
    ///
    /// # Arguments
    ///
    /// * `address` - the first register's address.
    /// * `values` - where the values go.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Spi`] if the SPI device fails.
    pub fn read_registers(
        &mut self,
        address: u8,
        values: &mut [u8],
    ) -> Result<(), RadioError<SPI::Error>> {
        self.spi
            .transaction(&mut [
                Operation::Write(&[read_address(address)]),
                Operation::Read(values),
            ])
            .map_err(RadioError::Spi)
    }

    /// Writes consecutive registers in one transaction, or bytes into the data buffer when
    /// `address` is [`register::FIFO`].
    ///
    /// # Arguments
    ///
    /// * `address` - the first register's address.
    /// * `values` - the values.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Spi`] if the SPI device fails.
    pub fn write_registers(
        &mut self,
        address: u8,
        values: &[u8],
    ) -> Result<(), RadioError<SPI::Error>> {
        self.spi
            .transaction(&mut [
                Operation::Write(&[write_address(address)]),
                Operation::Write(values),
            ])
            .map_err(RadioError::Spi)
    }

    fn calibrate(&mut self, frequency_hz: u32) -> Result<(), RadioError<SPI::Error>> {
        self.write_register(register::OP_MODE, lora_op_mode(Mode::Sleep))?;
        self.write_register(register::OP_MODE, fsk_op_mode(Mode::Sleep))?;
        self.write_register(register::OP_MODE, fsk_op_mode(Mode::Standby))?;
        self.write_register(register::PA_CONFIG, 0x00)?;
        self.write_registers(register::FRF_MSB, &frequency_bytes(frequency_hz))?;
        self.tuned_hz = Some(frequency_hz);
        self.update_register(register::IMAGE_CAL, image_cal_start)?;
        let mut waited_us = 0u32;
        while self.read_register(register::IMAGE_CAL)? & config::IMAGE_CAL_RUNNING != 0 {
            if waited_us >= CALIBRATION_LIMIT_US {
                return Err(RadioError::Calibration);
            }
            self.delay.delay_us(CALIBRATION_POLL_US);
            waited_us = waited_us.saturating_add(CALIBRATION_POLL_US);
        }
        self.write_register(register::OP_MODE, fsk_op_mode(Mode::Sleep))?;
        self.write_register(register::OP_MODE, lora_op_mode(Mode::Sleep))?;
        self.standby()?;
        match Port::for_frequency(frequency_hz) {
            Port::High => self.calibrated_high = true,
            Port::Low => self.calibrated_low = true,
        }
        Ok(())
    }

    fn tune(&mut self, frequency_hz: u32) -> Result<(), RadioError<SPI::Error>> {
        if self.tuned_hz != Some(frequency_hz) {
            self.write_registers(register::FRF_MSB, &frequency_bytes(frequency_hz))?;
            self.tuned_hz = Some(frequency_hz);
        }
        Ok(())
    }

    fn prepare_reception(
        &mut self,
        settings: &RadioConfig,
        modulation: &LoraModulation,
    ) -> Result<(), RadioError<SPI::Error>> {
        let invert = settings.invert_iq_receive;
        self.standby()?;
        self.write_register(register::INVERT_IQ, invert_iq(invert, false))?;
        self.write_register(register::INVERT_IQ_2, invert_iq_2(invert))?;
        let erratum = spurious_reception(modulation.bandwidth);
        self.update_register(register::DETECT_OPTIMIZE, |value| {
            automatic_if(value, erratum.automatic_if)
        })?;
        if let Some(if_freq_2) = erratum.if_freq_2 {
            self.write_register(register::IF_FREQ_1, 0x00)?;
            self.write_register(register::IF_FREQ_2, if_freq_2)?;
        }
        self.tune(settings.frequency_hz.saturating_add(erratum.offset_hz))?;
        self.write_register(register::FIFO_RX_BASE_ADDR, 0)?;
        self.write_register(register::FIFO_ADDR_PTR, 0)?;
        self.write_register(register::DIO_MAPPING_1, config::DIO0_RX_DONE)?;
        self.write_register(register::IRQ_FLAGS, IrqFlags::ALL.bits())
    }

    fn read_frame(
        &mut self,
        buffer: &mut [u8],
        settings: &RadioConfig,
    ) -> Result<Reception, RadioError<SPI::Error>> {
        let len = usize::from(self.read_register(register::RX_NB_BYTES)?);
        let frame = buffer
            .get_mut(..len)
            .ok_or(RadioError::BufferTooSmall(len))?;
        let start = self.read_register(register::FIFO_RX_CURRENT_ADDR)?;
        self.write_register(register::FIFO_ADDR_PTR, start)?;
        if len > 0 {
            self.read_registers(register::FIFO, frame)?;
        }
        let mut levels = [0u8; 2];
        self.read_registers(register::PKT_SNR_VALUE, &mut levels)?;
        Ok(Reception::Frame {
            len,
            status: PacketStatus::from_bytes(levels, Port::for_frequency(settings.frequency_hz)),
        })
    }

    fn update_register(
        &mut self,
        address: u8,
        change: impl FnOnce(u8) -> u8,
    ) -> Result<(), RadioError<SPI::Error>> {
        let value = self.read_register(address)?;
        self.write_register(address, change(value))
    }

    fn wait_for(
        &mut self,
        events: IrqFlags,
        limit_us: u64,
    ) -> Result<IrqFlags, RadioError<SPI::Error>> {
        let mut waited_us = 0u64;
        loop {
            let flags = self.irq_flags()?;
            if flags.intersects(events) {
                return Ok(flags);
            }
            if waited_us >= limit_us {
                return Err(RadioError::NoInterrupt);
            }
            self.delay.delay_us(IRQ_POLL_US);
            waited_us = waited_us.saturating_add(u64::from(IRQ_POLL_US));
        }
    }

    fn pause_us(&mut self, micros: u64) {
        let mut left = micros;
        while left > 0 {
            let step = u32::try_from(left).unwrap_or(u32::MAX);
            self.delay.delay_us(step);
            left -= u64::from(step);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pamoja_hal::digital::PinState;
    use pamoja_hal::script::{DelayLog, PinScript, SpiScript, SpiStep};

    type Radio = Sx127x<SpiScript, PinScript, DelayLog>;

    fn radio(steps: Vec<SpiStep>) -> Radio {
        Sx127x::new(
            SpiScript::new(steps),
            PinScript::new([]),
            DelayLog::new(),
            Board::new(PaOutput::PaBoost),
        )
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

    fn eu868() -> RadioConfig {
        RadioConfig::new(
            868_100_000,
            LinkSettings::new(7, 125_000),
            TxPower::for_output(PaOutput::PaBoost, 14),
        )
        .with_sync_word(SyncWord::Public)
    }

    fn calibration(frequency: [u8; 3]) -> Vec<SpiStep> {
        let mut steps = Vec::new();
        steps.extend(wrote(0x01, &[0x88]));
        steps.extend(wrote(0x01, &[0x08]));
        steps.extend(wrote(0x01, &[0x09]));
        steps.extend(wrote(0x09, &[0x00]));
        steps.extend(wrote(0x06, &frequency));
        steps.extend(read(0x3B, &[0x82]));
        steps.extend(wrote(0x3B, &[0x42]));
        steps.extend(read(0x3B, &[0x22]));
        steps.extend(read(0x3B, &[0x02]));
        steps.extend(wrote(0x01, &[0x08]));
        steps.extend(wrote(0x01, &[0x88]));
        steps.extend(wrote(0x01, &[0x89]));
        steps
    }

    fn configuration() -> Vec<SpiStep> {
        let mut steps = calibration([0xD9, 0x06, 0x66]);
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
        steps
    }

    fn configured(more: Vec<SpiStep>) -> Radio {
        let mut steps = configuration();
        steps.extend(more);
        let mut radio = radio(steps);
        radio.configure(eu868()).expect("configures");
        radio
    }

    fn reception_setup() -> Vec<SpiStep> {
        let mut steps = Vec::new();
        steps.extend(wrote(0x01, &[0x89]));
        steps.extend(wrote(0x33, &[0x27]));
        steps.extend(wrote(0x3B, &[0x1D]));
        steps.extend(read(0x31, &[0xC3]));
        steps.extend(wrote(0x31, &[0x43]));
        steps.extend(wrote(0x30, &[0x00]));
        steps.extend(wrote(0x2F, &[0x40]));
        steps.extend(wrote(0x0F, &[0x00]));
        steps.extend(wrote(0x0D, &[0x00]));
        steps.extend(wrote(0x40, &[0x00]));
        steps.extend(wrote(0x12, &[0xFF]));
        steps
    }

    #[test]
    fn init_resets_checks_the_version_and_enters_lora_mode_with_a_tcxo() {
        let mut steps = Vec::new();
        steps.extend(read(0x42, &[0x12]));
        steps.extend(wrote(0x01, &[0x08]));
        steps.extend(wrote(0x4B, &[0x19]));
        steps.extend(wrote(0x01, &[0x88]));
        steps.extend(wrote(0x01, &[0x89]));
        steps.extend(wrote(0x0C, &[0x23]));
        let board = Board::new(PaOutput::PaBoost).with_tcxo();
        let mut radio = Sx127x::new(
            SpiScript::new(steps),
            PinScript::new([]),
            DelayLog::new(),
            board,
        );

        radio.init().expect("initializes");

        let (spi, reset, delay) = radio.release();
        assert!(spi.done(), "{} steps left", spi.remaining());
        assert_eq!(reset.driven(), [PinState::Low, PinState::High]);
        assert_eq!(delay.waits_ns(), [1_000_000, 6_000_000]);
    }

    #[test]
    fn init_refuses_a_chip_that_is_not_an_sx1276() {
        let mut radio = radio(read(0x42, &[0x22]).to_vec());
        assert_eq!(radio.init(), Err(RadioError::Absent(0x22)));
    }

    #[test]
    fn configure_calibrates_the_port_once_then_writes_the_modem() {
        let mut steps = configuration();
        steps.extend(wrote(0x01, &[0x89]));
        steps.extend(wrote(0x06, &[0xD9, 0x13, 0x33]));
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
        let mut radio = radio(steps);

        radio.configure(eu868()).expect("configures on 868.1 MHz");
        let mut next = eu868();
        next.frequency_hz = 868_300_000;
        radio.configure(next).expect("configures on 868.3 MHz");

        let (spi, _, delay) = radio.release();
        assert!(spi.done(), "{} steps left", spi.remaining());
        assert_eq!(delay.waits_ns(), [1_000_000], "one poll while calibrating");
    }

    #[test]
    fn configure_refuses_settings_the_chip_cannot_use() {
        let mut radio = radio(Vec::new());
        let sf5 = RadioConfig::new(
            868_100_000,
            LinkSettings::new(5, 125_000),
            TxPower::for_output(PaOutput::PaBoost, 14),
        );
        assert_eq!(
            radio.configure(sf5),
            Err(RadioError::Modulation(ModulationError::SpreadingFactor(5)))
        );
        let wide_at_169 = RadioConfig::new(
            169_400_000,
            LinkSettings::new(7, 500_000),
            TxPower::for_output(PaOutput::PaBoost, 14),
        );
        assert_eq!(
            radio.configure(wide_at_169),
            Err(RadioError::Modulation(ModulationError::Bandwidth(500_000)))
        );
        let rfo = RadioConfig::new(
            868_100_000,
            LinkSettings::new(7, 125_000),
            TxPower::for_output(PaOutput::Rfo, 14),
        );
        assert_eq!(radio.configure(rfo), Err(RadioError::Output));
        let (spi, _, _) = radio.release();
        assert_eq!(spi.consumed(), 0, "nothing reaches the bus");
    }

    #[test]
    fn a_frame_goes_out_through_the_fifo_and_tx_mode() {
        let mut steps = Vec::new();
        steps.extend(wrote(0x01, &[0x89]));
        steps.extend(wrote(0x33, &[0x27]));
        steps.extend(wrote(0x3B, &[0x1D]));
        steps.extend(wrote(0x22, &[0x05]));
        steps.extend(wrote(0x0E, &[0x00]));
        steps.extend(wrote(0x0D, &[0x00]));
        steps.extend(wrote(0x00, b"level"));
        steps.extend(wrote(0x40, &[0x40]));
        steps.extend(wrote(0x12, &[0xFF]));
        steps.extend(wrote(0x01, &[0x8B]));
        steps.extend(read(0x12, &[0x00]));
        steps.extend(read(0x12, &[0x08]));
        steps.extend(wrote(0x12, &[0x08]));
        let mut radio = configured(steps);

        let airtime = radio.transmit(b"level").expect("sends");

        assert_eq!(airtime, LinkSettings::new(7, 125_000).airtime_us(5));
        let (spi, _, _) = radio.release();
        assert!(spi.done(), "{} steps left", spi.remaining());
    }

    #[test]
    fn a_transmission_needs_a_configuration_and_a_payload_that_fits() {
        let mut unconfigured = radio(Vec::new());
        assert_eq!(
            unconfigured.start_transmit(b"x"),
            Err(RadioError::NotConfigured)
        );
        let mut radio = configured(Vec::new());
        assert_eq!(radio.start_transmit(&[]), Err(RadioError::PayloadLength(0)));
        assert_eq!(
            radio.start_transmit(&[0; 256]),
            Err(RadioError::PayloadLength(256))
        );
    }

    #[test]
    fn listening_applies_the_spurious_reception_erratum_and_takes_frames() {
        let mut steps = reception_setup();
        steps.extend(wrote(0x01, &[0x8D]));
        steps.extend(read(0x12, &[0x00]));
        steps.extend(read(0x12, &[0x50]));
        steps.extend(wrote(0x12, &[0x50]));
        steps.extend(read(0x13, &[0x02]));
        steps.extend(read(0x10, &[0x00]));
        steps.extend(wrote(0x0D, &[0x00]));
        steps.extend(read(0x00, b"hi"));
        steps.extend(read(0x19, &[0xF6, 0x30]));
        steps.extend(read(0x12, &[0x60]));
        steps.extend(wrote(0x12, &[0x60]));
        let mut radio = configured(steps);
        let mut buffer = [0u8; 255];

        radio.listen().expect("listens");
        assert_eq!(radio.take_frame(&mut buffer), Ok(None));
        let frame = radio.take_frame(&mut buffer).expect("reads the frame");
        assert_eq!(
            frame,
            Some(Reception::Frame {
                len: 2,
                status: PacketStatus::from_bytes([0xF6, 0x30], Port::High),
            })
        );
        assert_eq!(&buffer[..2], b"hi");
        assert_eq!(radio.take_frame(&mut buffer), Ok(Some(Reception::Corrupt)));

        let (spi, _, _) = radio.release();
        assert!(spi.done(), "{} steps left", spi.remaining());
    }

    #[test]
    fn a_narrow_bandwidth_listens_one_bandwidth_above_the_carrier() {
        let narrow = RadioConfig::new(
            433_175_000,
            LinkSettings::new(9, 20_833),
            TxPower::for_output(PaOutput::PaBoost, 10),
        );
        let mut steps = calibration([0x6C, 0x4B, 0x33]);
        steps.extend(wrote(0x09, &[0xF8]));
        steps.extend(wrote(0x4D, &[0x84]));
        steps.extend(wrote(0x0B, &[0x2B]));
        steps.extend(wrote(0x1D, &[0x32]));
        steps.extend(wrote(0x1E, &[0x94]));
        steps.extend(wrote(0x26, &[0x0C]));
        steps.extend(wrote(0x20, &[0x00, 0x08]));
        steps.extend(read(0x31, &[0xC3]));
        steps.extend(wrote(0x31, &[0xC3]));
        steps.extend(wrote(0x37, &[0x0A]));
        steps.extend(wrote(0x36, &[0x03]));
        steps.extend(wrote(0x23, &[0xFF]));
        steps.extend(wrote(0x39, &[0x12]));
        steps.extend(wrote(0x01, &[0x89]));
        steps.extend(wrote(0x33, &[0x27]));
        steps.extend(wrote(0x3B, &[0x1D]));
        steps.extend(read(0x31, &[0xC3]));
        steps.extend(wrote(0x31, &[0x43]));
        steps.extend(wrote(0x30, &[0x00]));
        steps.extend(wrote(0x2F, &[0x44]));
        steps.extend(wrote(0x06, &frequency_bytes(433_175_000 + 20_830)));
        steps.extend(wrote(0x0F, &[0x00]));
        steps.extend(wrote(0x0D, &[0x00]));
        steps.extend(wrote(0x40, &[0x00]));
        steps.extend(wrote(0x12, &[0xFF]));
        steps.extend(wrote(0x01, &[0x8D]));
        let mut radio = radio(steps);

        radio.configure(narrow).expect("configures");
        radio.listen().expect("listens");

        let (spi, _, _) = radio.release();
        assert!(spi.done(), "{} steps left", spi.remaining());
    }

    #[test]
    fn a_single_reception_counts_its_timeout_in_symbols_and_reports_it() {
        let mut steps = reception_setup();
        steps.extend(wrote(0x1E, &[0x74]));
        steps.extend(wrote(0x1F, &[0x62]));
        steps.extend(wrote(0x01, &[0x8E]));
        steps.extend(read(0x12, &[0x80]));
        steps.extend(wrote(0x12, &[0x80]));
        let mut radio = configured(steps);
        let mut buffer = [0u8; 64];

        let outcome = radio.receive(&mut buffer, 100_000).expect("listens");

        assert_eq!(outcome, Reception::Timeout);
        let (spi, _, _) = radio.release();
        assert!(spi.done(), "{} steps left", spi.remaining());
    }

    #[test]
    fn a_frame_longer_than_the_buffer_is_refused() {
        let mut steps = reception_setup();
        steps.extend(wrote(0x01, &[0x8D]));
        steps.extend(read(0x12, &[0x40]));
        steps.extend(wrote(0x12, &[0x40]));
        steps.extend(read(0x13, &[0x10]));
        let mut radio = configured(steps);
        let mut buffer = [0u8; 8];

        radio.listen().expect("listens");
        assert_eq!(
            radio.take_frame(&mut buffer),
            Err(RadioError::BufferTooSmall(16))
        );
    }

    #[test]
    fn a_calibration_that_never_finishes_is_an_error() {
        let mut steps = Vec::new();
        steps.extend(wrote(0x01, &[0x88]));
        steps.extend(wrote(0x01, &[0x08]));
        steps.extend(wrote(0x01, &[0x09]));
        steps.extend(wrote(0x09, &[0x00]));
        steps.extend(wrote(0x06, &[0xD9, 0x06, 0x66]));
        steps.extend(read(0x3B, &[0x82]));
        steps.extend(wrote(0x3B, &[0x42]));
        for _ in 0..=CALIBRATION_LIMIT_US / CALIBRATION_POLL_US {
            steps.extend(read(0x3B, &[0x22]));
        }
        let mut radio = radio(steps);

        assert_eq!(radio.configure(eu868()), Err(RadioError::Calibration));
    }

    #[test]
    fn the_rssi_uses_the_offset_of_the_configured_port() {
        let mut radio = configured(read(0x1B, &[0x30]).to_vec());
        assert_eq!(radio.rssi(), Ok(Decibels::from_db(-109)));
    }
}
