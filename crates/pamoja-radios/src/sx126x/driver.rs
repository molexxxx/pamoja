//! The SX126x driven over embedded-hal: reset, configure, transmit, and receive.
//!
//! [`Sx126x`] owns the SPI device, the BUSY and NRESET lines, and a delay. It waits for
//! BUSY to fall before each command, as section 8.3.1 requires, reads the IRQ register to
//! learn when a frame has gone out or come in, and issues the commands in the order
//! sections 14.2 and 14.3 give, with the workarounds of chapter 15 applied.

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{self, InputPin, OutputPin};
use embedded_hal::spi::{Operation, SpiDevice};
use pamoja_lora::budget::Decibels;
use pamoja_lora::LinkSettings;

use super::command::{self, Command, Query, CALIBRATE_ALL};
use super::config::{
    self, frequency_word, image_calibration, iq_polarity, llcc68_supports, register, timeout_steps,
    tx_clamp, tx_modulation, LoraModulation, LoraPacket, PacketType, PowerAmplifier, RampTime,
    RegulatorMode, StandbyMode, SyncWord, TcxoVoltage, TxPower,
};
use super::irq::Irq;
use super::status::{rssi_inst_dbm, ChipMode, DeviceErrors, PacketStatus, RxBufferStatus, Status};

/// How long NRESET is held low, in microseconds. Section 8.1 needs typically 100 us; this is
/// the 20 ms Semtech's LoRaMac-node reference holds it for.
pub const RESET_HOLD_US: u32 = 20_000;

/// How long the driver waits after releasing NRESET before it watches BUSY, in microseconds,
/// the 10 ms LoRaMac-node waits.
pub const RESET_SETTLE_US: u32 = 10_000;

/// How often BUSY is sampled while the chip is busy, in microseconds.
pub const BUSY_POLL_US: u32 = 100;

/// The longest BUSY may stay high before the driver gives up, in microseconds, on top of a
/// TCXO's settling time. The slowest transitions the datasheet gives, a cold start from
/// sleep (Table 8-2) and a full calibration (section 13.1.12), each take 3.5 ms.
pub const BUSY_LIMIT_US: u32 = 100_000;

/// How often the IRQ register is read while a transmission or a reception runs, in
/// microseconds.
pub const IRQ_POLL_US: u32 = 1_000;

/// How much longer than a frame's airtime the chip's timeout, and then the driver's own
/// wait, may run before the frame is given up on, in microseconds.
pub const TIMEOUT_MARGIN_US: u64 = 1_000_000;

/// The pause between NSS falling and the first clock edge when waking the chip from sleep,
/// in nanoseconds: t10 of Table 8-1, 100 us.
pub const WAKE_SETUP_NS: u32 = 100_000;

/// How long the driver leaves the chip alone after SetSleep, in microseconds: twice the
/// "around 500 us" section 13.1.1 cautions it is unresponsive for.
pub const SLEEP_ENTRY_US: u32 = 1_000;

/// How a module wires its SX126x: the amplifier, the clock, the antenna switch, and the
/// regulator.
///
/// The SPI interface cannot tell an SX1261 from an SX1262 or an LLCC68, and it cannot see
/// the parts around the chip, so these come from the module's schematic.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx126x::config::{PowerAmplifier, RegulatorMode, TcxoVoltage};
/// use pamoja_radios::sx126x::Board;
///
/// // An SX1262 clocked by a 1.8 V TCXO that settles in 5 ms, with DIO2 on the antenna switch.
/// let board = Board::new(PowerAmplifier::HighPower)
///     .with_tcxo(TcxoVoltage::V1_8, 5_000)
///     .with_dio2_rf_switch()
///     .with_dc_dc();
/// assert_eq!(board.regulator, RegulatorMode::DcDc);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Board {
    /// The power amplifier: [`PowerAmplifier::LowPower`] for the SX1261 and
    /// [`PowerAmplifier::HighPower`] for the SX1262 and the LLCC68.
    pub amplifier: PowerAmplifier,
    /// The voltage DIO3 supplies a TCXO with, and how long the TCXO takes to settle in
    /// microseconds, for a module clocked by a TCXO rather than a crystal.
    pub tcxo: Option<(TcxoVoltage, u32)>,
    /// Whether DIO2 drives the antenna switch, high while transmitting (section 13.3.5).
    pub dio2_rf_switch: bool,
    /// The regulator: the DC-DC converter where the module fits its inductor, else the LDO.
    pub regulator: RegulatorMode,
    /// Whether the chip is an LLCC68, which [`Sx126x::configure`] holds to the spreading
    /// factors and bandwidths [`llcc68_supports`] allows.
    pub llcc68: bool,
}

impl Board {
    /// A module with a crystal, no switch on DIO2, and the LDO, which are the chip's defaults.
    ///
    /// # Arguments
    ///
    /// * `amplifier` - the chip's power amplifier.
    ///
    /// # Returns
    ///
    /// The board.
    pub const fn new(amplifier: PowerAmplifier) -> Board {
        Board {
            amplifier,
            tcxo: None,
            dio2_rf_switch: false,
            regulator: RegulatorMode::Ldo,
            llcc68: false,
        }
    }

    /// Returns the board clocked by a TCXO that DIO3 powers.
    ///
    /// # Arguments
    ///
    /// * `voltage` - the TCXO supply voltage.
    /// * `settle_us` - how long the TCXO takes to settle, in microseconds.
    ///
    /// # Returns
    ///
    /// The board.
    pub const fn with_tcxo(mut self, voltage: TcxoVoltage, settle_us: u32) -> Board {
        self.tcxo = Some((voltage, settle_us));
        self
    }

    /// Returns the board with DIO2 driving the antenna switch.
    ///
    /// # Returns
    ///
    /// The board.
    pub const fn with_dio2_rf_switch(mut self) -> Board {
        self.dio2_rf_switch = true;
        self
    }

    /// Returns the board running on the DC-DC converter.
    ///
    /// # Returns
    ///
    /// The board.
    pub const fn with_dc_dc(mut self) -> Board {
        self.regulator = RegulatorMode::DcDc;
        self
    }

    /// Returns the board with an LLCC68, whose high power amplifier it must also name.
    ///
    /// # Returns
    ///
    /// The board.
    pub const fn with_llcc68(mut self) -> Board {
        self.llcc68 = true;
        self
    }
}

/// What a radio sends and listens with: the carrier, the LoRa link, the power, and the
/// sync word and IQ polarity that keep one network apart from another.
///
/// # Examples
///
/// ```
/// use pamoja_lora::LinkSettings;
/// use pamoja_radios::sx126x::config::{PowerAmplifier, SyncWord, TxPower};
/// use pamoja_radios::sx126x::RadioConfig;
///
/// // A LoRaWAN device on 868.1 MHz at SF9, calibrated for the whole 863 to 870 MHz band.
/// let config = RadioConfig::new(
///     868_100_000,
///     LinkSettings::new(9, 125_000),
///     TxPower::for_output(PowerAmplifier::HighPower, 14),
/// )
/// .with_band(863_000_000, 870_000_000)
/// .lorawan_device();
/// assert_eq!(config.sync_word, SyncWord::Public);
/// assert!(config.invert_iq_receive && !config.invert_iq_transmit);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadioConfig {
    /// The carrier frequency in hertz.
    pub frequency_hz: u32,
    /// The band image calibration covers, as its lower and upper edges in hertz.
    pub band_hz: (u32, u32),
    /// The spreading factor, bandwidth, coding rate, preamble, header, and CRC.
    pub link: LinkSettings,
    /// The amplifier configuration and power setting.
    pub power: TxPower,
    /// How fast the amplifier ramps up.
    pub ramp: RampTime,
    /// The sync word written to the registers at 0x0740.
    pub sync_word: SyncWord,
    /// Whether frames go out with inverted IQ, as a LoRaWAN gateway sends downlinks.
    pub invert_iq_transmit: bool,
    /// Whether the receiver expects inverted IQ, as a LoRaWAN device hears downlinks.
    pub invert_iq_receive: bool,
}

impl RadioConfig {
    /// Builds a configuration with a private sync word and standard IQ both ways.
    ///
    /// Image calibration covers just the carrier, and the amplifier ramps in 40 us, the
    /// time Semtech's LoRaMac-node reference passes to SetTxParams.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the carrier frequency in hertz.
    /// * `link` - the LoRa link settings.
    /// * `power` - the amplifier configuration and power setting.
    ///
    /// # Returns
    ///
    /// The configuration.
    pub const fn new(frequency_hz: u32, link: LinkSettings, power: TxPower) -> RadioConfig {
        RadioConfig {
            frequency_hz,
            band_hz: (frequency_hz, frequency_hz),
            link,
            power,
            ramp: RampTime::Us40,
            sync_word: SyncWord::Private,
            invert_iq_transmit: false,
            invert_iq_receive: false,
        }
    }

    /// Returns the configuration with image calibration covering a whole band, so moving
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

    /// Returns the configuration with another amplifier ramp time.
    ///
    /// # Arguments
    ///
    /// * `ramp` - the ramp time.
    ///
    /// # Returns
    ///
    /// The configuration.
    pub const fn with_ramp(mut self, ramp: RampTime) -> RadioConfig {
        self.ramp = ramp;
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

/// How a reception ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reception {
    /// A frame whose header and CRC checked.
    Frame {
        /// The payload length; the payload is at the start of the buffer.
        len: usize,
        /// The signal levels the frame arrived with.
        status: PacketStatus,
    },
    /// No frame arrived before the timeout.
    Timeout,
    /// A frame arrived whose header or CRC failed its check, and was dropped.
    Corrupt,
}

/// What can go wrong driving an SX126x.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioError<E> {
    /// The SPI device failed.
    Spi(E),
    /// The BUSY or NRESET line could not be read or driven.
    Pin(digital::ErrorKind),
    /// BUSY stayed high for longer than [`BUSY_LIMIT_US`] allows, so the chip is stuck,
    /// unpowered, or not wired to that line.
    Busy,
    /// The chip answered GetStatus with a mode it cannot be in after a reset and SetStandby,
    /// so no SX126x, or a miswired one, is on the bus.
    Absent(Status),
    /// The link settings use a bandwidth the SX126x does not offer, in hertz.
    Bandwidth(u32),
    /// The board has an LLCC68, which does not support the link's spreading factor at its
    /// bandwidth.
    Llcc68 {
        /// The link's spreading factor.
        spreading_factor: u8,
        /// The link's bandwidth in hertz.
        bandwidth_hz: u32,
    },
    /// The power settings are for the other amplifier than the board has.
    Amplifier,
    /// A payload longer than the 255 bytes a LoRa frame carries, with its length.
    PayloadTooLong(usize),
    /// An answer or a received payload of this many bytes does not fit the buffer given.
    BufferTooSmall(usize),
    /// A transmission or a reception was asked for before [`Sx126x::configure`].
    NotConfigured,
    /// The chip raised its TIMEOUT interrupt before TxDone.
    TxTimeout,
    /// Neither the expected interrupt nor TIMEOUT arrived in the time the driver allows.
    NoInterrupt,
}

impl<E: core::fmt::Debug> core::fmt::Display for RadioError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RadioError::Spi(error) => write!(f, "SPI error: {error:?}"),
            RadioError::Pin(kind) => write!(f, "BUSY or NRESET line error: {kind:?}"),
            RadioError::Busy => f.write_str("the radio held BUSY high past the time allowed"),
            RadioError::Absent(status) => write!(f, "no SX126x answered, status {status:?}"),
            RadioError::Bandwidth(hz) => write!(f, "the SX126x has no {hz} Hz LoRa bandwidth"),
            RadioError::Llcc68 {
                spreading_factor,
                bandwidth_hz,
            } => write!(
                f,
                "the LLCC68 does not support SF{spreading_factor} at {bandwidth_hz} Hz"
            ),
            RadioError::Amplifier => {
                f.write_str("the power settings are for the other power amplifier")
            }
            RadioError::PayloadTooLong(len) => write!(
                f,
                "a {len} byte payload is longer than the 255 bytes a LoRa frame carries"
            ),
            RadioError::BufferTooSmall(len) => write!(f, "{len} bytes do not fit the buffer"),
            RadioError::NotConfigured => f.write_str("the radio has not been configured"),
            RadioError::TxTimeout => f.write_str("the transmission timed out before TxDone"),
            RadioError::NoInterrupt => {
                f.write_str("the radio raised no interrupt in the time allowed")
            }
        }
    }
}

impl<E: core::fmt::Debug> core::error::Error for RadioError<E> {}

fn pin<E, P: digital::Error>(error: P) -> RadioError<E> {
    RadioError::Pin(error.kind())
}

/// A Semtech SX1261, SX1262, or LLCC68 on an SPI bus, with its BUSY and NRESET lines.
///
/// [`init`](Sx126x::init) resets the chip and sets up what the [`Board`] wires around it,
/// [`configure`](Sx126x::configure) tunes it to a [`RadioConfig`], and
/// [`transmit`](Sx126x::transmit) and [`receive`](Sx126x::receive) send and wait for one
/// frame each. For anything those do not cover, [`command`](Sx126x::command),
/// [`query`](Sx126x::query), and the register and buffer methods reach the chip directly,
/// with the BUSY handshake still done for them.
///
/// # Examples
///
/// The chip's side of initialization, scripted: an SX1262 module with a crystal, which
/// answers GetStatus in STDBY_RC.
///
/// ```
/// use pamoja_hal::digital::{OutputPin, PinState};
/// use pamoja_hal::script::{DelayLog, PinScript, SpiScript, SpiStep};
/// use pamoja_radios::sx126x::config::PowerAmplifier;
/// use pamoja_radios::sx126x::{Board, Sx126x};
///
/// let spi = SpiScript::new([
///     SpiStep::write([0x80, 0x00]),
///     SpiStep::write([0xC0]),
///     SpiStep::read([0x22]),
///     SpiStep::write([0x96, 0x00]),
///     SpiStep::write([0x8A, 0x01]),
///     SpiStep::write([0x1D, 0x08, 0xD8, 0x00]),
///     SpiStep::read([0x08]),
///     SpiStep::write([0x0D, 0x08, 0xD8]),
///     SpiStep::write([0x1E]),
///     SpiStep::write([0x8F, 0x00, 0x00]),
/// ]);
/// let mut busy = PinScript::new([]);
/// busy.set_low()?;
/// let board = Board::new(PowerAmplifier::HighPower);
///
/// let mut radio = Sx126x::new(spi, busy, PinScript::new([]), DelayLog::new(), board);
/// radio.init().expect("the scripted SX1262 answers");
///
/// let (spi, _, reset, _) = radio.release();
/// assert!(spi.done());
/// assert_eq!(reset.driven(), [PinState::Low, PinState::High]);
/// # Ok::<(), core::convert::Infallible>(())
/// ```
pub struct Sx126x<SPI, BUSY, RESET, D> {
    spi: SPI,
    busy: BUSY,
    reset: RESET,
    delay: D,
    board: Board,
    config: Option<RadioConfig>,
    image: Option<[u8; 2]>,
    asleep: bool,
}

impl<SPI, BUSY, RESET, D> Sx126x<SPI, BUSY, RESET, D> {
    /// Wraps a chip's SPI device and lines. Nothing is sent until [`init`](Sx126x::init).
    ///
    /// # Arguments
    ///
    /// * `spi` - the SPI device, with NSS as its chip select.
    /// * `busy` - the BUSY line, as an input.
    /// * `reset` - the NRESET line, as an output.
    /// * `delay` - a delay for the reset pulse and the polling.
    /// * `board` - how the module wires the chip.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn new(spi: SPI, busy: BUSY, reset: RESET, delay: D, board: Board) -> Self {
        Sx126x {
            spi,
            busy,
            reset,
            delay,
            board,
            config: None,
            image: None,
            asleep: false,
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
    /// The configuration, or `None` before [`configure`](Sx126x::configure) or after a
    /// reset or sleep.
    pub fn config(&self) -> Option<&RadioConfig> {
        self.config.as_ref()
    }

    /// Returns the power settings for an output power on this board's amplifier.
    ///
    /// # Arguments
    ///
    /// * `output_dbm` - the output power wanted at the antenna port.
    ///
    /// # Returns
    ///
    /// The amplifier configuration and power setting.
    pub fn tx_power(&self, output_dbm: i8) -> TxPower {
        TxPower::for_output(self.board.amplifier, output_dbm)
    }

    /// Gives back the SPI device, the lines, and the delay.
    ///
    /// # Returns
    ///
    /// The SPI device, BUSY, NRESET, and the delay.
    pub fn release(self) -> (SPI, BUSY, RESET, D) {
        (self.spi, self.busy, self.reset, self.delay)
    }
}

impl<SPI, BUSY, RESET, D> Sx126x<SPI, BUSY, RESET, D>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    RESET: OutputPin,
    D: DelayNs,
{
    /// Resets the chip and sets up the parts the board wires around it.
    ///
    /// NRESET is pulsed low, and the chip, which calibrates itself on the way out of reset,
    /// is put in STDBY_RC and asked for its status. A TCXO is then powered from DIO3 and
    /// every block calibrated again, since section 9.2.1 says the calibration at power up
    /// fails on a TCXO, and the XOSC start error that section 13.3.6 expects is cleared.
    /// The regulator and the DIO2 antenna switch follow, then the LoRa packet type, the
    /// antenna mismatch workaround of section 15.2 for the high power amplifier, and the
    /// data buffer base addresses.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Absent`] if the status is not STDBY_RC, [`RadioError::Busy`] if
    /// BUSY never falls, and [`RadioError::Spi`] or [`RadioError::Pin`] if the bus or a line
    /// fails.
    pub fn init(&mut self) -> Result<(), RadioError<SPI::Error>> {
        self.config = None;
        self.image = None;
        self.asleep = false;
        self.reset.set_low().map_err(pin)?;
        self.delay.delay_us(RESET_HOLD_US);
        self.reset.set_high().map_err(pin)?;
        self.delay.delay_us(RESET_SETTLE_US);

        self.command(command::set_standby(StandbyMode::Rc))?;
        let status = self.status()?;
        if !matches!(status.chip_mode, ChipMode::StandbyRc) {
            return Err(RadioError::Absent(status));
        }
        if let Some((voltage, settle_us)) = self.board.tcxo {
            let settle = timeout_steps(u64::from(settle_us));
            self.command(command::set_dio3_as_tcxo(voltage, settle))?;
            self.command(command::calibrate(CALIBRATE_ALL))?;
            self.command(command::clear_device_errors())?;
        }
        self.command(command::set_regulator_mode(self.board.regulator))?;
        if self.board.dio2_rf_switch {
            self.command(command::set_dio2_as_rf_switch(true))?;
        }
        self.command(command::set_packet_type(PacketType::Lora))?;
        if self.board.amplifier == PowerAmplifier::HighPower {
            self.update_register(register::TX_CLAMP_CONFIG, tx_clamp)?;
        }
        self.command(command::set_buffer_base_address(0, 0))
    }

    /// Tunes the chip to a configuration.
    ///
    /// The chip goes to STDBY_RC and takes the LoRa packet type, a new image calibration if
    /// the band moved, the frequency, the amplifier configuration and power, the modulation,
    /// and the sync word. The packet parameters, which carry the payload length, are sent
    /// with each transmission and reception, after the modulation as section 14.5 requires.
    ///
    /// # Arguments
    ///
    /// * `config` - the configuration.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Bandwidth`] if the link's bandwidth is not one the SX126x has,
    /// [`RadioError::Llcc68`] if the board has an LLCC68 that does not support the link,
    /// [`RadioError::Amplifier`] if the power settings are for the other amplifier, and the
    /// bus errors of [`command`](Sx126x::command).
    pub fn configure(&mut self, config: RadioConfig) -> Result<(), RadioError<SPI::Error>> {
        let modulation = LoraModulation::from_link(&config.link)
            .ok_or(RadioError::Bandwidth(config.link.bandwidth_hz()))?;
        if self.board.llcc68 && !llcc68_supports(modulation.spreading_factor, modulation.bandwidth)
        {
            return Err(RadioError::Llcc68 {
                spreading_factor: modulation.spreading_factor,
                bandwidth_hz: config.link.bandwidth_hz(),
            });
        }
        let low_power = self.board.amplifier == PowerAmplifier::LowPower;
        if (config.power.pa.device == 1) != low_power {
            return Err(RadioError::Amplifier);
        }

        self.command(command::set_standby(StandbyMode::Rc))?;
        self.command(command::set_packet_type(PacketType::Lora))?;
        let (low_hz, high_hz) = config.band_hz;
        let image = image_calibration(
            low_hz.min(config.frequency_hz),
            high_hz.max(config.frequency_hz),
        );
        if self.image != Some(image) {
            self.command(command::calibrate_image(image))?;
            self.image = Some(image);
        }
        self.command(command::set_rf_frequency(frequency_word(
            config.frequency_hz,
        )))?;
        self.command(command::set_pa_config(config.power.pa))?;
        self.command(command::set_tx_params(
            config.power.setting_dbm,
            config.ramp,
        ))?;
        self.command(command::set_lora_modulation_params(modulation))?;
        self.write_register(register::LORA_SYNC_WORD, &config.sync_word.to_bytes())?;
        self.config = Some(config);
        Ok(())
    }

    /// Sends one frame and waits for it to leave.
    ///
    /// This is [`start_transmit`](Sx126x::start_transmit), a wait for the frame's airtime,
    /// and [`finish_transmit`](Sx126x::finish_transmit) read every [`IRQ_POLL_US`] until the
    /// chip reports the frame sent or timed out.
    ///
    /// # Arguments
    ///
    /// * `payload` - the frame's payload, at most 255 bytes.
    ///
    /// # Returns
    ///
    /// The frame's airtime in microseconds, for a [`DutyCycle`](crate::duty::DutyCycle) to
    /// count.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`start_transmit`](Sx126x::start_transmit) and
    /// [`finish_transmit`](Sx126x::finish_transmit), and [`RadioError::NoInterrupt`] if the
    /// chip reports neither outcome within its own timeout and [`TIMEOUT_MARGIN_US`] more.
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
    /// The steps are those of section 14.2 after configuration: the payload into the data
    /// buffer, the packet parameters, the inverted IQ workaround of section 15.4, TxDone and
    /// TIMEOUT routed to DIO1, the 500 kHz workaround of section 15.1, and SetTx with a
    /// timeout of the frame's airtime and [`TIMEOUT_MARGIN_US`]. A caller with its own
    /// scheduler waits out the airtime and then calls
    /// [`finish_transmit`](Sx126x::finish_transmit), or watches DIO1.
    ///
    /// # Arguments
    ///
    /// * `payload` - the frame's payload, at most 255 bytes.
    ///
    /// # Returns
    ///
    /// The frame's airtime in microseconds.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::NotConfigured`] before [`configure`](Sx126x::configure),
    /// [`RadioError::PayloadTooLong`] past 255 bytes, and the bus errors of
    /// [`command`](Sx126x::command).
    pub fn start_transmit(&mut self, payload: &[u8]) -> Result<u64, RadioError<SPI::Error>> {
        let settings = self.config.ok_or(RadioError::NotConfigured)?;
        let len =
            u8::try_from(payload.len()).map_err(|_| RadioError::PayloadTooLong(payload.len()))?;
        let bandwidth = LoraModulation::from_link(&settings.link)
            .ok_or(RadioError::Bandwidth(settings.link.bandwidth_hz()))?
            .bandwidth;

        self.command(command::set_standby(StandbyMode::Rc))?;
        if !payload.is_empty() {
            self.write_buffer(0, payload)?;
        }
        let invert = settings.invert_iq_transmit;
        let packet = LoraPacket::from_link(&settings.link, len, invert);
        self.command(command::set_lora_packet_params(packet))?;
        self.update_register(register::IQ_POLARITY, |value| iq_polarity(value, invert))?;
        let events = Irq::TX_DONE | Irq::TIMEOUT;
        self.command(command::set_dio_irq_params(
            events,
            events,
            Irq::NONE,
            Irq::NONE,
        ))?;
        self.update_register(register::TX_MODULATION, |value| {
            tx_modulation(value, bandwidth)
        })?;
        self.command(command::clear_irq_status(Irq::ALL))?;

        let airtime_us = settings.link.airtime_us(payload.len());
        let timeout_us = airtime_us.saturating_add(TIMEOUT_MARGIN_US);
        self.command(command::set_tx(timeout_steps(timeout_us)))?;
        Ok(airtime_us)
    }

    /// Reports whether the frame [`start_transmit`](Sx126x::start_transmit) began has left.
    ///
    /// The IRQ register is read once, and on TxDone or TIMEOUT the interrupts are cleared.
    ///
    /// # Returns
    ///
    /// `true` once the frame has been sent, `false` while it is still going out.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::TxTimeout`] if the chip timed out, and the bus errors of
    /// [`command`](Sx126x::command).
    pub fn finish_transmit(&mut self) -> Result<bool, RadioError<SPI::Error>> {
        let irq = self.irq_status()?;
        if !irq.intersects(Irq::TX_DONE | Irq::TIMEOUT) {
            return Ok(false);
        }
        self.command(command::clear_irq_status(Irq::ALL))?;
        if irq.contains(Irq::TX_DONE) {
            Ok(true)
        } else {
            Err(RadioError::TxTimeout)
        }
    }

    /// Listens for one frame.
    ///
    /// The steps are those of section 14.3 after configuration: the packet parameters with
    /// the buffer's length as the most to accept, the inverted IQ workaround, RxDone,
    /// TIMEOUT, CrcErr, and HeaderErr routed to DIO1, and SetRx with the timeout. Once an
    /// interrupt arrives, the timer is stopped and its event cleared as section 15.3 advises
    /// after any reception with a timeout, the interrupts are cleared, and a frame that
    /// checked is copied out of the data buffer with its signal levels.
    ///
    /// # Arguments
    ///
    /// * `buffer` - where the payload goes; its length, up to 255, is the most accepted.
    /// * `timeout_us` - how long to listen for a frame to start, in microseconds.
    ///
    /// # Returns
    ///
    /// The frame's length and levels, or that the timeout passed or the frame was corrupt.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::NotConfigured`] before [`configure`](Sx126x::configure),
    /// [`RadioError::BufferTooSmall`] if the chip reports a longer payload than the buffer
    /// holds, [`RadioError::NoInterrupt`] if it never answers, and the bus errors of
    /// [`command`](Sx126x::command).
    pub fn receive(
        &mut self,
        buffer: &mut [u8],
        timeout_us: u64,
    ) -> Result<Reception, RadioError<SPI::Error>> {
        let settings = self.config.ok_or(RadioError::NotConfigured)?;
        let most = u8::try_from(buffer.len()).unwrap_or(u8::MAX);
        let events = Irq::RX_DONE | Irq::TIMEOUT | Irq::CRC_ERROR | Irq::HEADER_ERROR;
        self.prepare_reception(&settings, most, events)?;

        let timeout_us = timeout_us.max(1);
        self.command(command::set_rx(timeout_steps(timeout_us)))?;
        let frame_us = settings.link.airtime_us(usize::from(most));
        let limit_us = timeout_us
            .saturating_add(frame_us)
            .saturating_add(TIMEOUT_MARGIN_US);
        let irq = self.wait_for(events, 0, limit_us)?;
        self.write_register(register::RTC_CONTROL, &[config::RTC_STOP])?;
        self.update_register(register::EVENT_MASK, config::event_clear)?;
        self.command(command::clear_irq_status(Irq::ALL))?;

        if irq.intersects(Irq::CRC_ERROR | Irq::HEADER_ERROR) {
            return Ok(Reception::Corrupt);
        }
        if !irq.contains(Irq::RX_DONE) {
            return Ok(Reception::Timeout);
        }
        self.read_frame(buffer)
    }

    /// Starts listening with no timeout, so the chip receives frame after frame until
    /// another command stops it: the Rx Continuous mode of Table 13-9.
    ///
    /// The setup is that of [`receive`](Sx126x::receive), accepting the 255 bytes a frame
    /// may carry, with RxDone, CrcErr, and HeaderErr routed to DIO1. Each frame is read with
    /// [`take_frame`](Sx126x::take_frame).
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::NotConfigured`] before [`configure`](Sx126x::configure), and the
    /// bus errors of [`command`](Sx126x::command).
    pub fn listen(&mut self) -> Result<(), RadioError<SPI::Error>> {
        let settings = self.config.ok_or(RadioError::NotConfigured)?;
        let events = Irq::RX_DONE | Irq::CRC_ERROR | Irq::HEADER_ERROR;
        self.prepare_reception(&settings, u8::MAX, events)?;
        self.command(command::set_rx(config::RX_CONTINUOUS))
    }

    /// Takes the frame a [`listen`](Sx126x::listen) has received, if one has arrived.
    ///
    /// The IRQ register is read once. On RxDone, CrcErr, or HeaderErr those interrupts are
    /// cleared, and a frame that checked is copied out with its signal levels while the chip
    /// goes on listening.
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
    /// Returns [`RadioError::BufferTooSmall`] if the payload does not fit, and the bus errors
    /// of [`command`](Sx126x::command).
    pub fn take_frame(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<Option<Reception>, RadioError<SPI::Error>> {
        let events = Irq::RX_DONE | Irq::CRC_ERROR | Irq::HEADER_ERROR;
        let irq = self.irq_status()?;
        if !irq.intersects(events) {
            return Ok(None);
        }
        self.command(command::clear_irq_status(events))?;
        if irq.intersects(Irq::CRC_ERROR | Irq::HEADER_ERROR) {
            return Ok(Some(Reception::Corrupt));
        }
        self.read_frame(buffer).map(Some)
    }

    /// Puts the chip in STDBY_RC, which stops a transmission or a reception.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of [`command`](Sx126x::command).
    pub fn standby(&mut self) -> Result<(), RadioError<SPI::Error>> {
        self.command(command::set_standby(StandbyMode::Rc))
    }

    /// Puts the chip to sleep until the next command wakes it.
    ///
    /// A warm start keeps the chip's configuration in retention; a cold start loses it,
    /// and [`init`](Sx126x::init) must run again. Either way the driver forgets the
    /// [`RadioConfig`], so [`configure`](Sx126x::configure) runs before the next frame. The
    /// next command wakes the chip with GetStatus, pausing [`WAKE_SETUP_NS`] after NSS
    /// falls.
    ///
    /// # Arguments
    ///
    /// * `warm_start` - `true` to keep the configuration in retention.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of [`command`](Sx126x::command).
    pub fn sleep(&mut self, warm_start: bool) -> Result<(), RadioError<SPI::Error>> {
        self.command(command::set_standby(StandbyMode::Rc))?;
        self.command(command::set_sleep(warm_start, false))?;
        self.delay.delay_us(SLEEP_ENTRY_US);
        self.asleep = true;
        self.config = None;
        if !warm_start {
            self.image = None;
        }
        Ok(())
    }

    /// Reads the status byte.
    ///
    /// # Returns
    ///
    /// The chip mode and how the last command went.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of [`command`](Sx126x::command).
    pub fn status(&mut self) -> Result<Status, RadioError<SPI::Error>> {
        let mut byte = [0u8; 1];
        self.query(command::get_status(), &mut byte)?;
        Ok(Status::from_byte(byte[0]))
    }

    /// Reads the pending interrupts.
    ///
    /// # Returns
    ///
    /// The IRQ register.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of [`command`](Sx126x::command).
    pub fn irq_status(&mut self) -> Result<Irq, RadioError<SPI::Error>> {
        let mut bytes = [0u8; 2];
        self.query(command::get_irq_status(), &mut bytes)?;
        Ok(Irq::from_bytes(bytes))
    }

    /// Reads the signal power the receiver hears right now, while it listens.
    ///
    /// # Returns
    ///
    /// The instantaneous RSSI in dBm.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of [`command`](Sx126x::command).
    pub fn instantaneous_rssi(&mut self) -> Result<Decibels, RadioError<SPI::Error>> {
        let mut byte = [0u8; 1];
        self.query(command::get_rssi_inst(), &mut byte)?;
        Ok(rssi_inst_dbm(byte[0]))
    }

    /// Reads the calibration, oscillator, PLL, and amplifier errors the chip has flagged.
    ///
    /// # Returns
    ///
    /// The device errors.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of [`command`](Sx126x::command).
    pub fn device_errors(&mut self) -> Result<DeviceErrors, RadioError<SPI::Error>> {
        let mut bytes = [0u8; 2];
        self.query(command::get_device_errors(), &mut bytes)?;
        Ok(DeviceErrors::from_bytes(bytes))
    }

    /// Clears the device errors.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of [`command`](Sx126x::command).
    pub fn clear_device_errors(&mut self) -> Result<(), RadioError<SPI::Error>> {
        self.command(command::clear_device_errors())
    }

    /// Chooses between the receiver's power saving gain, the chip's default, and its boosted
    /// gain, which buys sensitivity for current (Table 9-3).
    ///
    /// # Arguments
    ///
    /// * `boosted` - `true` for boosted gain.
    ///
    /// # Errors
    ///
    /// Returns the bus errors of [`command`](Sx126x::command).
    pub fn set_rx_boosted(&mut self, boosted: bool) -> Result<(), RadioError<SPI::Error>> {
        let gain = if boosted {
            config::RX_GAIN_BOOSTED
        } else {
            config::RX_GAIN_POWER_SAVING
        };
        self.write_register(register::RX_GAIN, &[gain])
    }

    /// Sends one command once BUSY is low, waking the chip first if it sleeps.
    ///
    /// # Arguments
    ///
    /// * `command` - the command.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::Busy`] if BUSY never falls, [`RadioError::Spi`] if the SPI
    /// device fails, and [`RadioError::Pin`] if BUSY cannot be read.
    pub fn command(&mut self, command: Command) -> Result<(), RadioError<SPI::Error>> {
        self.ready()?;
        self.spi.write(command.as_bytes()).map_err(RadioError::Spi)
    }

    /// Sends a query and reads its answer in the same transaction, once BUSY is low.
    ///
    /// # Arguments
    ///
    /// * `query` - the query.
    /// * `answer` - where the answer goes; its first `query.answer_len` bytes are filled.
    ///
    /// # Errors
    ///
    /// Returns [`RadioError::BufferTooSmall`] if `answer` is shorter than the answer, and
    /// the errors of [`command`](Sx126x::command).
    pub fn query(&mut self, query: Query, answer: &mut [u8]) -> Result<(), RadioError<SPI::Error>> {
        let len = query.answer_len;
        let answer = answer
            .get_mut(..len)
            .ok_or(RadioError::BufferTooSmall(len))?;
        self.ready()?;
        self.spi
            .transaction(&mut [
                Operation::Write(query.command.as_bytes()),
                Operation::Read(answer),
            ])
            .map_err(RadioError::Spi)
    }

    /// Writes consecutive registers.
    ///
    /// # Arguments
    ///
    /// * `address` - the first register's address.
    /// * `values` - the values, one per register.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`command`](Sx126x::command).
    pub fn write_register(
        &mut self,
        address: u16,
        values: &[u8],
    ) -> Result<(), RadioError<SPI::Error>> {
        let header = command::write_register(address);
        self.ready()?;
        self.spi
            .transaction(&mut [
                Operation::Write(header.as_bytes()),
                Operation::Write(values),
            ])
            .map_err(RadioError::Spi)
    }

    /// Reads consecutive registers.
    ///
    /// # Arguments
    ///
    /// * `address` - the first register's address.
    /// * `values` - where the values go, one per register.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`command`](Sx126x::command).
    pub fn read_register(
        &mut self,
        address: u16,
        values: &mut [u8],
    ) -> Result<(), RadioError<SPI::Error>> {
        self.query(command::read_register(address, values.len()), values)
    }

    /// Writes bytes into the data buffer.
    ///
    /// # Arguments
    ///
    /// * `offset` - where in the buffer the first byte goes.
    /// * `bytes` - the bytes.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`command`](Sx126x::command).
    pub fn write_buffer(&mut self, offset: u8, bytes: &[u8]) -> Result<(), RadioError<SPI::Error>> {
        let header = command::write_buffer(offset);
        self.ready()?;
        self.spi
            .transaction(&mut [Operation::Write(header.as_bytes()), Operation::Write(bytes)])
            .map_err(RadioError::Spi)
    }

    /// Reads bytes out of the data buffer.
    ///
    /// # Arguments
    ///
    /// * `offset` - where in the buffer the first byte is.
    /// * `bytes` - where the bytes go.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`command`](Sx126x::command).
    pub fn read_buffer(
        &mut self,
        offset: u8,
        bytes: &mut [u8],
    ) -> Result<(), RadioError<SPI::Error>> {
        self.query(command::read_buffer(offset, bytes.len()), bytes)
    }

    fn prepare_reception(
        &mut self,
        settings: &RadioConfig,
        most: u8,
        events: Irq,
    ) -> Result<(), RadioError<SPI::Error>> {
        self.command(command::set_standby(StandbyMode::Rc))?;
        let invert = settings.invert_iq_receive;
        let packet = LoraPacket::from_link(&settings.link, most, invert);
        self.command(command::set_lora_packet_params(packet))?;
        self.update_register(register::IQ_POLARITY, |value| iq_polarity(value, invert))?;
        self.command(command::set_dio_irq_params(
            events,
            events,
            Irq::NONE,
            Irq::NONE,
        ))?;
        self.command(command::clear_irq_status(Irq::ALL))
    }

    fn read_frame(&mut self, buffer: &mut [u8]) -> Result<Reception, RadioError<SPI::Error>> {
        let mut position = [0u8; 2];
        self.query(command::get_rx_buffer_status(), &mut position)?;
        let position = RxBufferStatus::from_bytes(position);
        let len = usize::from(position.payload_len);
        let frame = buffer
            .get_mut(..len)
            .ok_or(RadioError::BufferTooSmall(len))?;
        if len > 0 {
            self.read_buffer(position.start, frame)?;
        }
        let mut levels = [0u8; 3];
        self.query(command::get_packet_status(), &mut levels)?;
        Ok(Reception::Frame {
            len,
            status: PacketStatus::from_bytes(levels),
        })
    }

    fn update_register(
        &mut self,
        address: u16,
        change: impl FnOnce(u8) -> u8,
    ) -> Result<(), RadioError<SPI::Error>> {
        let mut value = [0u8; 1];
        self.read_register(address, &mut value)?;
        self.write_register(address, &[change(value[0])])
    }

    fn ready(&mut self) -> Result<(), RadioError<SPI::Error>> {
        if self.asleep {
            let wake = command::get_status();
            let mut status = [0u8; 1];
            self.spi
                .transaction(&mut [
                    Operation::DelayNs(WAKE_SETUP_NS),
                    Operation::Write(wake.command.as_bytes()),
                    Operation::Read(&mut status),
                ])
                .map_err(RadioError::Spi)?;
            self.asleep = false;
        }
        self.wait_busy()
    }

    fn wait_busy(&mut self) -> Result<(), RadioError<SPI::Error>> {
        let settle_us = self.board.tcxo.map_or(0, |(_, settle_us)| settle_us);
        let limit_us = BUSY_LIMIT_US.saturating_add(settle_us);
        let mut waited_us = 0u32;
        while self.busy.is_high().map_err(pin)? {
            if waited_us >= limit_us {
                return Err(RadioError::Busy);
            }
            self.delay.delay_us(BUSY_POLL_US);
            waited_us = waited_us.saturating_add(BUSY_POLL_US);
        }
        Ok(())
    }

    fn wait_for(
        &mut self,
        events: Irq,
        first_us: u64,
        limit_us: u64,
    ) -> Result<Irq, RadioError<SPI::Error>> {
        let mut waited_us = first_us.min(limit_us);
        self.pause_us(waited_us);
        loop {
            let irq = self.irq_status()?;
            if irq.intersects(events) {
                return Ok(irq);
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
    use crate::sx126x::config::{PaConfig, TcxoVoltage};
    use pamoja_hal::digital::PinState;
    use pamoja_hal::script::{DelayLog, PinScript, SpiScript, SpiStep};

    type Radio = Sx126x<SpiScript, PinScript, PinScript, DelayLog>;

    fn idle() -> PinScript {
        let mut busy = PinScript::new([]);
        busy.set_low().unwrap();
        busy
    }

    fn radio(steps: Vec<SpiStep>, board: Board) -> Radio {
        Sx126x::new(
            SpiScript::new(steps),
            idle(),
            PinScript::new([]),
            DelayLog::new(),
            board,
        )
    }

    fn sent(command: Command) -> SpiStep {
        SpiStep::write(command.as_bytes().to_vec())
    }

    fn asked(query: Query, reply: &[u8]) -> [SpiStep; 2] {
        [
            SpiStep::write(query.command.as_bytes().to_vec()),
            SpiStep::read(reply.to_vec()),
        ]
    }

    fn register_read(address: u16, value: u8) -> [SpiStep; 2] {
        let [high, low] = address.to_be_bytes();
        [
            SpiStep::write([0x1D, high, low, 0x00]),
            SpiStep::read([value]),
        ]
    }

    fn register_write(address: u16, values: &[u8]) -> [SpiStep; 2] {
        let [high, low] = address.to_be_bytes();
        [
            SpiStep::write([0x0D, high, low]),
            SpiStep::write(values.to_vec()),
        ]
    }

    fn irq(bits: u16) -> [SpiStep; 2] {
        [
            SpiStep::write([0x12, 0x00]),
            SpiStep::read(bits.to_be_bytes()),
        ]
    }

    fn eu868() -> RadioConfig {
        RadioConfig::new(
            868_100_000,
            LinkSettings::new(7, 125_000),
            TxPower::for_output(PowerAmplifier::HighPower, 14),
        )
        .with_band(863_000_000, 870_000_000)
        .with_sync_word(SyncWord::Public)
    }

    fn configured(mut steps: Vec<SpiStep>) -> Radio {
        let mut all = vec![
            SpiStep::write([0x80, 0x00]),
            SpiStep::write([0x8A, 0x01]),
            SpiStep::write([0x98, 0xD7, 0xDA]),
            SpiStep::write([0x86, 0x36, 0x41, 0x99, 0x9A]),
            SpiStep::write([0x95, 0x04, 0x07, 0x00, 0x01]),
            SpiStep::write([0x8E, 0x0E, 0x02]),
            SpiStep::write([0x8B, 0x07, 0x04, 0x01, 0x00]),
        ];
        all.extend(register_write(0x0740, &[0x34, 0x44]));
        all.append(&mut steps);
        let mut radio = radio(all, Board::new(PowerAmplifier::HighPower));
        radio.configure(eu868()).expect("configures");
        radio
    }

    #[test]
    fn init_resets_then_sets_up_a_tcxo_the_switch_and_the_clamp() {
        let mut steps = vec![sent(command::set_standby(StandbyMode::Rc))];
        steps.extend(asked(command::get_status(), &[0x22]));
        steps.extend([
            SpiStep::write([0x97, 0x02, 0x00, 0x01, 0x40]),
            SpiStep::write([0x89, 0x7F]),
            sent(command::clear_device_errors()),
            SpiStep::write([0x96, 0x01]),
            SpiStep::write([0x9D, 0x01]),
            SpiStep::write([0x8A, 0x01]),
        ]);
        steps.extend(register_read(0x08D8, 0x08));
        steps.extend(register_write(0x08D8, &[0x1E]));
        steps.push(SpiStep::write([0x8F, 0x00, 0x00]));
        let board = Board::new(PowerAmplifier::HighPower)
            .with_tcxo(TcxoVoltage::V1_8, 5_000)
            .with_dio2_rf_switch()
            .with_dc_dc();

        let mut radio = radio(steps, board);
        radio.init().expect("initializes");

        let (spi, _, reset, delay) = radio.release();
        assert!(spi.done(), "{} steps left", spi.remaining());
        assert_eq!(reset.driven(), [PinState::Low, PinState::High]);
        assert_eq!(delay.waits_ns(), [20_000_000, 10_000_000]);
    }

    #[test]
    fn init_without_an_sx126x_on_the_bus_says_so() {
        let mut steps = vec![sent(command::set_standby(StandbyMode::Rc))];
        steps.extend(asked(command::get_status(), &[0x00]));
        let mut radio = radio(steps, Board::new(PowerAmplifier::HighPower));
        assert_eq!(
            radio.init(),
            Err(RadioError::Absent(Status::from_byte(0x00)))
        );
    }

    #[test]
    fn a_busy_line_that_never_falls_times_out() {
        let mut radio = Sx126x::new(
            SpiScript::new([]),
            PinScript::new([]),
            PinScript::new([]),
            DelayLog::new(),
            Board::new(PowerAmplifier::HighPower),
        );
        assert_eq!(radio.standby(), Err(RadioError::Busy));
        let (_, _, _, delay) = radio.release();
        assert_eq!(delay.total_micros(), u64::from(BUSY_LIMIT_US));
    }

    #[test]
    fn configure_calibrates_the_band_once_and_tunes_each_channel() {
        let mut steps = vec![
            SpiStep::write([0x80, 0x00]),
            SpiStep::write([0x8A, 0x01]),
            sent(command::set_rf_frequency(frequency_word(868_300_000))),
            SpiStep::write([0x95, 0x04, 0x07, 0x00, 0x01]),
            SpiStep::write([0x8E, 0x0E, 0x02]),
            SpiStep::write([0x8B, 0x07, 0x04, 0x01, 0x00]),
        ];
        steps.extend(register_write(0x0740, &[0x34, 0x44]));
        let mut radio = configured(steps);

        let next_channel = RadioConfig {
            frequency_hz: 868_300_000,
            ..eu868()
        };
        radio.configure(next_channel).expect("retunes");
        assert_eq!(radio.config(), Some(&next_channel));
        assert!(radio.release().0.done());
    }

    #[test]
    fn configure_refuses_a_bandwidth_or_an_amplifier_the_chip_lacks() {
        let mut radio = radio(Vec::new(), Board::new(PowerAmplifier::HighPower));
        let narrow = RadioConfig {
            link: LinkSettings::new(7, 203_125),
            ..eu868()
        };
        assert_eq!(radio.configure(narrow), Err(RadioError::Bandwidth(203_125)));

        let sx1261 = RadioConfig {
            power: TxPower::for_output(PowerAmplifier::LowPower, 14),
            ..eu868()
        };
        assert_eq!(radio.configure(sx1261), Err(RadioError::Amplifier));
        assert_eq!(radio.tx_power(14).pa, PaConfig::SX1262_22_DBM);
    }

    #[test]
    fn configure_holds_an_llcc68_to_the_rates_it_supports() {
        let board = Board::new(PowerAmplifier::HighPower).with_llcc68();
        let mut radio = radio(Vec::new(), board);
        let sf10 = RadioConfig {
            link: LinkSettings::new(10, 125_000),
            ..eu868()
        };
        assert_eq!(
            radio.configure(sf10),
            Err(RadioError::Llcc68 {
                spreading_factor: 10,
                bandwidth_hz: 125_000
            })
        );
        let (spi, _, _, _) = radio.release();
        assert_eq!(spi.consumed(), 0, "nothing reaches the bus");
    }

    #[test]
    fn transmit_follows_section_14_2_and_returns_the_airtime() {
        let link = LinkSettings::new(7, 125_000);
        let airtime_us = link.airtime_us(5);
        let mut steps = vec![
            SpiStep::write([0x80, 0x00]),
            SpiStep::write([0x0E, 0x00]),
            SpiStep::write(*b"hello"),
            SpiStep::write([0x8C, 0x00, 0x08, 0x00, 0x05, 0x01, 0x00]),
        ];
        steps.extend(register_read(0x0736, 0x09));
        steps.extend(register_write(0x0736, &[0x0D]));
        steps.push(SpiStep::write([
            0x08, 0x02, 0x01, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00,
        ]));
        steps.extend(register_read(0x0889, 0x00));
        steps.extend(register_write(0x0889, &[0x04]));
        steps.push(SpiStep::write([0x02, 0x43, 0xFF]));
        steps.push(sent(command::set_tx(timeout_steps(
            airtime_us + TIMEOUT_MARGIN_US,
        ))));
        steps.extend(irq(0x0001));
        steps.push(SpiStep::write([0x02, 0x43, 0xFF]));
        let mut radio = configured(steps);

        assert_eq!(radio.transmit(b"hello"), Ok(airtime_us));
        let (spi, _, _, delay) = radio.release();
        assert!(spi.done(), "{} steps left", spi.remaining());
        assert_eq!(delay.total_micros(), airtime_us);
    }

    #[test]
    fn a_transmission_the_chip_times_out_is_an_error() {
        let mut steps = vec![
            SpiStep::write([0x80, 0x00]),
            SpiStep::write([0x0E, 0x00]),
            SpiStep::write([0xAA]),
            SpiStep::write([0x8C, 0x00, 0x08, 0x00, 0x01, 0x01, 0x00]),
        ];
        steps.extend(register_read(0x0736, 0x0D));
        steps.extend(register_write(0x0736, &[0x0D]));
        steps.push(SpiStep::write([
            0x08, 0x02, 0x01, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00,
        ]));
        steps.extend(register_read(0x0889, 0x04));
        steps.extend(register_write(0x0889, &[0x04]));
        steps.push(SpiStep::write([0x02, 0x43, 0xFF]));
        let airtime_us = LinkSettings::new(7, 125_000).airtime_us(1);
        steps.push(sent(command::set_tx(timeout_steps(
            airtime_us + TIMEOUT_MARGIN_US,
        ))));
        steps.extend(irq(0x0000));
        steps.extend(irq(0x0200));
        steps.push(SpiStep::write([0x02, 0x43, 0xFF]));
        let mut radio = configured(steps);

        assert_eq!(radio.transmit(&[0xAA]), Err(RadioError::TxTimeout));
        let (spi, _, _, delay) = radio.release();
        assert!(spi.done());
        assert_eq!(delay.total_micros(), airtime_us + u64::from(IRQ_POLL_US));
    }

    #[test]
    fn transmit_refuses_before_configure_and_past_255_bytes() {
        let mut radio = radio(Vec::new(), Board::new(PowerAmplifier::HighPower));
        assert_eq!(radio.transmit(b"early"), Err(RadioError::NotConfigured));

        let mut radio = configured(Vec::new());
        assert_eq!(
            radio.transmit(&[0u8; 256]),
            Err(RadioError::PayloadTooLong(256))
        );
        assert!(radio.release().0.done());
    }

    fn listening(irq_bits: u16) -> Vec<SpiStep> {
        let mut steps = vec![
            SpiStep::write([0x80, 0x00]),
            SpiStep::write([0x8C, 0x00, 0x08, 0x00, 0x10, 0x01, 0x00]),
        ];
        steps.extend(register_read(0x0736, 0x0D));
        steps.extend(register_write(0x0736, &[0x0D]));
        steps.push(SpiStep::write([
            0x08, 0x02, 0x62, 0x02, 0x62, 0x00, 0x00, 0x00, 0x00,
        ]));
        steps.push(SpiStep::write([0x02, 0x43, 0xFF]));
        steps.push(SpiStep::write([0x82, 0x01, 0xF4, 0x00]));
        steps.extend(irq(irq_bits));
        steps.extend(register_write(0x0902, &[0x00]));
        steps.extend(register_read(0x0944, 0x00));
        steps.extend(register_write(0x0944, &[0x02]));
        steps.push(SpiStep::write([0x02, 0x43, 0xFF]));
        steps
    }

    #[test]
    fn receive_follows_section_14_3_and_copies_out_a_good_frame() {
        let mut steps = listening(0x0002);
        steps.extend(asked(command::get_rx_buffer_status(), &[0x03, 0x80]));
        steps.push(SpiStep::write([0x1E, 0x80, 0x00]));
        steps.push(SpiStep::read(*b"hi!"));
        steps.extend(asked(command::get_packet_status(), &[0xDB, 0xF6, 0xE0]));
        let mut radio = configured(steps);

        let mut buffer = [0u8; 16];
        let reception = radio.receive(&mut buffer, 2_000_000).expect("receives");
        assert_eq!(
            reception,
            Reception::Frame {
                len: 3,
                status: PacketStatus::from_bytes([0xDB, 0xF6, 0xE0]),
            }
        );
        assert_eq!(&buffer[..3], b"hi!");
        assert!(radio.release().0.done());
    }

    #[test]
    fn a_corrupt_frame_or_a_timeout_carries_no_payload() {
        let mut radio = configured(listening(0x0042));
        let mut buffer = [0u8; 16];
        assert_eq!(
            radio.receive(&mut buffer, 2_000_000),
            Ok(Reception::Corrupt)
        );
        assert!(radio.release().0.done());

        let mut radio = configured(listening(0x0200));
        assert_eq!(
            radio.receive(&mut buffer, 2_000_000),
            Ok(Reception::Timeout)
        );
        assert!(radio.release().0.done());
    }

    #[test]
    fn a_payload_longer_than_the_buffer_is_refused() {
        let mut steps = listening(0x0002);
        steps.extend(asked(command::get_rx_buffer_status(), &[0x20, 0x00]));
        let mut radio = configured(steps);
        let mut buffer = [0u8; 16];
        assert_eq!(
            radio.receive(&mut buffer, 2_000_000),
            Err(RadioError::BufferTooSmall(32))
        );
    }

    #[test]
    fn sleep_forgets_the_configuration_and_the_next_command_wakes_the_chip() {
        let mut steps = vec![
            SpiStep::write([0x80, 0x00]),
            SpiStep::write([0x84, 0x04]),
            SpiStep::write([0xC0]),
            SpiStep::read([0x00]),
        ];
        steps.extend(asked(command::get_status(), &[0x22]));
        let mut radio = configured(steps);

        radio.sleep(true).expect("sleeps");
        assert_eq!(radio.config(), None);
        assert_eq!(
            radio.status().expect("wakes").chip_mode,
            ChipMode::StandbyRc
        );
        assert!(radio.release().0.done());
    }

    #[test]
    fn listen_keeps_receiving_and_take_frame_reads_each_frame_as_it_lands() {
        let mut steps = vec![
            SpiStep::write([0x80, 0x00]),
            SpiStep::write([0x8C, 0x00, 0x08, 0x00, 0xFF, 0x01, 0x00]),
        ];
        steps.extend(register_read(0x0736, 0x0D));
        steps.extend(register_write(0x0736, &[0x0D]));
        steps.push(SpiStep::write([
            0x08, 0x00, 0x62, 0x00, 0x62, 0x00, 0x00, 0x00, 0x00,
        ]));
        steps.push(SpiStep::write([0x02, 0x43, 0xFF]));
        steps.push(SpiStep::write([0x82, 0xFF, 0xFF, 0xFF]));
        steps.extend(irq(0x0000));
        steps.extend(irq(0x0002));
        steps.push(SpiStep::write([0x02, 0x00, 0x62]));
        steps.extend(asked(command::get_rx_buffer_status(), &[0x02, 0x00]));
        steps.push(SpiStep::write([0x1E, 0x00, 0x00]));
        steps.push(SpiStep::read(*b"ok"));
        steps.extend(asked(command::get_packet_status(), &[0x80, 0x1C, 0x82]));
        let mut radio = configured(steps);

        radio.listen().expect("listens");
        let mut buffer = [0u8; 255];
        assert_eq!(radio.take_frame(&mut buffer), Ok(None));
        let frame = radio.take_frame(&mut buffer).expect("takes the frame");
        assert!(matches!(frame, Some(Reception::Frame { len: 2, .. })));
        assert_eq!(&buffer[..2], b"ok");
        assert!(radio.release().0.done());
    }
}
