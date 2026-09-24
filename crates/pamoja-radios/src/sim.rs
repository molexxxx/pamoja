//! Simulated SX126x and SX127x chips, for running a radio program with no radio attached.
//!
//! A [`Chip`] answers its driver over SPI the way the part does. It takes the commands or
//! the register writes that tune it, puts a frame on the air when it is told to transmit,
//! and hands a frame over when one arrives while it listens. The program on the other side
//! says what arrives, with [`Chip::hear`], and reads back what the driver tuned the chip to
//! and what it sent, with [`Chip::tuning`] and [`Chip::sent`], so a program's radio settings
//! are checked against what the chip was actually told rather than against the program's
//! own intent.
//!
//! Nothing is timed. A transmission is done as soon as it starts, a reception with a timeout
//! ends at once when nothing is waiting on the air, and one without a timeout listens until
//! a frame arrives. The signal levels of a frame are the ones given to [`Chip::hear`],
//! rounded to the half or whole decibel the chip reports them in.
//!
//! # Examples
//!
//! ```
//! use pamoja_lora::budget::Decibels;
//! use pamoja_lora::region::Region;
//! use pamoja_radios::radio::{RadioConfig, Reception};
//! use pamoja_radios::sim::Chip;
//! use pamoja_radios::sx126x::config::PowerAmplifier;
//! use pamoja_radios::sx126x::Board;
//!
//! // An SX1262 on the bench, and the driver wired to it.
//! let chip = Chip::sx126x(Board::new(PowerAmplifier::HighPower));
//! let mut radio = chip.radio();
//! radio.init()?;
//!
//! // Tuned from the plan's DR3 at 868.1 MHz, a reading goes out.
//! let link = Region::Eu868.plan().link_settings(3).expect("DR3 is a LoRa data rate");
//! radio.configure(RadioConfig::new(868_100_000, link, 14))?;
//! radio.transmit(b"21.5")?;
//! let sent = &chip.sent()[0];
//! assert_eq!(sent.tuning.frequency_hz, 868_100_000);
//! assert_eq!(sent.tuning.link.spreading_factor(), 9);
//! assert_eq!(sent.payload, b"21.5");
//!
//! // An answer arrives 2.5 dB under the noise.
//! chip.hear(b"ok", Decibels::from_db(-109), Decibels::from_tenths(-25));
//! let mut buffer = [0u8; 255];
//! let Reception::Frame { len, levels } = radio.receive(&mut buffer, 1_000_000)? else {
//!     panic!("a frame was waiting");
//! };
//! assert_eq!(&buffer[..len], b"ok");
//! assert_eq!(levels.snr_db, Decibels::from_tenths(-25));
//! # Ok::<(), pamoja_radios::radio::RadioError<core::convert::Infallible>>(())
//! ```

use std::collections::{BTreeMap, VecDeque};
use std::convert::Infallible;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::vec::Vec;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{self, InputPin, OutputPin};
use embedded_hal::spi::{self, Operation, SpiDevice};
use pamoja_lora::budget::{self, Decibels, RADIO_NOISE_FIGURE_DB};
use pamoja_lora::LinkSettings;

use crate::radio::{Family, Radio, SyncWord};
use crate::sx126x::command::opcode;
use crate::sx126x::config::register as sx126x_register;
use crate::sx126x::irq::Irq;
use crate::sx127x::irq::IrqFlags;
use crate::sx127x::register as sx127x_register;
use crate::sx127x::status::Port;
use crate::{sx126x, sx127x};

/// A radio driver wired to a simulated chip.
pub type SimRadio = Radio<ChipSpi, ChipPin, ChipPin, ChipDelay>;

/// What a chip is tuned to: where it sends and listens, how, and how hard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tuning {
    /// The carrier frequency in hertz, as the chip's synthesizer steps it: within a hertz of
    /// the one asked for on an SX126x, and within 61 Hz on an SX127x.
    pub frequency_hz: u32,
    /// The spreading factor, bandwidth, coding rate, preamble, header, and CRC.
    pub link: LinkSettings,
    /// The output power the amplifier was asked for, in dBm.
    pub output_dbm: i8,
    /// The sync word.
    pub sync_word: SyncWord,
}

/// A frame a simulated chip put on the air.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sent {
    /// What the chip was tuned to when the frame went out.
    pub tuning: Tuning,
    /// The frame's payload.
    pub payload: Vec<u8>,
}

/// A simulated SX126x or SX127x, shared between its driver and the program watching it.
///
/// Cloning a chip gives another handle on the same part.
#[derive(Clone, Debug)]
pub struct Chip {
    state: Arc<Mutex<State>>,
}

impl Chip {
    /// A simulated SX1261, SX1262, SX1268, or LLCC68, out of reset.
    ///
    /// # Arguments
    ///
    /// * `board` - how the module wires the chip, which the driver [`radio`](Chip::radio)
    ///   returns is given.
    ///
    /// # Returns
    ///
    /// The chip.
    pub fn sx126x(board: sx126x::Board) -> Chip {
        Chip::with(Model::Sx126x(Sx126xModel::new(board)))
    }

    /// A simulated SX1276, SX1277, SX1278, or SX1279, out of reset.
    ///
    /// # Arguments
    ///
    /// * `board` - how the module wires the chip, which the driver [`radio`](Chip::radio)
    ///   returns is given.
    ///
    /// # Returns
    ///
    /// The chip.
    pub fn sx127x(board: sx127x::Board) -> Chip {
        Chip::with(Model::Sx127x(Sx127xModel::new(board)))
    }

    fn with(model: Model) -> Chip {
        Chip {
            state: Arc::new(Mutex::new(State {
                model,
                air: VecDeque::new(),
                sent: Vec::new(),
                random: RANDOM_SEED,
            })),
        }
    }

    /// Returns which family the chip belongs to.
    ///
    /// # Returns
    ///
    /// The family.
    pub fn family(&self) -> Family {
        match self.lock().model {
            Model::Sx126x(_) => Family::Sx126x,
            Model::Sx127x(_) => Family::Sx127x,
        }
    }

    /// Returns a driver wired to the chip, as a board wires a real one.
    ///
    /// # Returns
    ///
    /// The driver, which has sent nothing yet; [`init`](Radio::init) resets the chip.
    pub fn radio(&self) -> SimRadio {
        let spi = ChipSpi {
            state: Arc::clone(&self.state),
        };
        let busy = ChipPin {
            state: Arc::clone(&self.state),
        };
        let reset = ChipPin {
            state: Arc::clone(&self.state),
        };
        match &self.lock().model {
            Model::Sx126x(model) => Radio::from(sx126x::Sx126x::new(
                spi,
                busy,
                reset,
                ChipDelay,
                model.board,
            )),
            Model::Sx127x(model) => {
                Radio::from(sx127x::Sx127x::new(spi, reset, ChipDelay, model.board))
            }
        }
    }

    /// Puts a frame on the air for the chip to receive the next time it listens.
    ///
    /// # Arguments
    ///
    /// * `payload` - the frame's payload, at most 255 bytes; any more is cut off, as no LoRa
    ///   frame carries it.
    /// * `rssi_dbm` - the strength the chip hears the frame at.
    /// * `snr_db` - the signal-to-noise ratio it hears it with, negative below the noise.
    pub fn hear(&self, payload: &[u8], rssi_dbm: Decibels, snr_db: Decibels) {
        let payload = payload[..payload.len().min(usize::from(u8::MAX))].to_vec();
        self.lock().air.push_back(Arrival {
            payload,
            rssi_dbm,
            snr_db,
            intact: true,
        });
    }

    /// Puts a frame on the air whose CRC fails, which the chip reports and drops.
    ///
    /// # Arguments
    ///
    /// * `rssi_dbm` - the strength the chip hears the frame at.
    /// * `snr_db` - the signal-to-noise ratio it hears it with.
    pub fn hear_corrupt(&self, rssi_dbm: Decibels, snr_db: Decibels) {
        self.lock().air.push_back(Arrival {
            payload: Vec::new(),
            rssi_dbm,
            snr_db,
            intact: false,
        });
    }

    /// Returns how many frames wait on the air for the chip to receive.
    ///
    /// # Returns
    ///
    /// The number of frames given to [`hear`](Chip::hear) and not yet received.
    pub fn waiting(&self) -> usize {
        self.lock().air.len()
    }

    /// Returns every frame the chip has put on the air, oldest first.
    ///
    /// # Returns
    ///
    /// The frames, each with what the chip was tuned to when it went out.
    pub fn sent(&self) -> Vec<Sent> {
        self.lock().sent.clone()
    }

    /// Returns what the chip is tuned to now.
    ///
    /// Before its driver configures it, these are the chip's own values out of reset. The
    /// SX126x has no carrier until it is given one, and reports 0 Hz.
    ///
    /// # Returns
    ///
    /// The tuning.
    pub fn tuning(&self) -> Tuning {
        self.lock().model.tuning()
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The SPI device of a simulated chip.
#[derive(Debug)]
pub struct ChipSpi {
    state: Arc<Mutex<State>>,
}

impl spi::ErrorType for ChipSpi {
    type Error = Infallible;
}

impl SpiDevice for ChipSpi {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Infallible> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.transaction(operations);
        Ok(())
    }
}

/// A line of a simulated chip: BUSY, which never holds the driver up, and NRESET, which
/// resets the chip when it is pulled low and released.
#[derive(Debug)]
pub struct ChipPin {
    state: Arc<Mutex<State>>,
}

impl digital::ErrorType for ChipPin {
    type Error = Infallible;
}

impl InputPin for ChipPin {
    fn is_high(&mut self) -> Result<bool, Infallible> {
        Ok(false)
    }

    fn is_low(&mut self) -> Result<bool, Infallible> {
        Ok(true)
    }
}

impl OutputPin for ChipPin {
    fn set_low(&mut self) -> Result<(), Infallible> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.model.hold_in_reset();
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Infallible> {
        Ok(())
    }
}

/// The delay a simulated chip's driver waits with, which passes no time.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChipDelay;

impl DelayNs for ChipDelay {
    fn delay_ns(&mut self, _ns: u32) {}
}

const RANDOM_SEED: u32 = 0x2545_F491;

#[derive(Clone, Debug)]
struct Arrival {
    payload: Vec<u8>,
    rssi_dbm: Decibels,
    snr_db: Decibels,
    intact: bool,
}

#[derive(Debug)]
struct State {
    model: Model,
    air: VecDeque<Arrival>,
    sent: Vec<Sent>,
    random: u32,
}

impl State {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) {
        let mut written = Vec::new();
        let mut answered = false;
        for operation in operations.iter_mut() {
            match operation {
                Operation::Write(bytes) => written.extend_from_slice(bytes),
                Operation::Read(answer) => {
                    self.answer(&written, answer);
                    answered = true;
                }
                Operation::Transfer(answer, bytes) => {
                    written.extend_from_slice(bytes);
                    self.answer(&written, answer);
                    answered = true;
                }
                Operation::TransferInPlace(bytes) => {
                    written.extend_from_slice(bytes);
                    self.answer(&written, bytes);
                    answered = true;
                }
                Operation::DelayNs(_) => {}
            }
        }
        if !answered {
            self.write(&written);
        }
    }

    fn answer(&mut self, written: &[u8], answer: &mut [u8]) {
        let State {
            model,
            air,
            sent: _,
            random,
        } = self;
        match model {
            Model::Sx126x(chip) => chip.answer(written, answer, air, random),
            Model::Sx127x(chip) => chip.read(written, answer, air, random),
        }
    }

    fn write(&mut self, written: &[u8]) {
        let State { model, sent, .. } = self;
        match model {
            Model::Sx126x(chip) => chip.command(written, sent),
            Model::Sx127x(chip) => chip.write(written, sent),
        }
    }
}

#[derive(Debug)]
enum Model {
    Sx126x(Sx126xModel),
    Sx127x(Sx127xModel),
}

impl Model {
    fn tuning(&self) -> Tuning {
        match self {
            Model::Sx126x(chip) => chip.tuning(),
            Model::Sx127x(chip) => chip.tuning(),
        }
    }

    fn hold_in_reset(&mut self) {
        match self {
            Model::Sx126x(chip) => *chip = Sx126xModel::new(chip.board),
            Model::Sx127x(chip) => *chip = Sx127xModel::new(chip.board),
        }
    }
}

/// Draws the next byte of the chip's fixed noise sequence.
fn noise_byte(random: &mut u32) -> u8 {
    let mut x = *random;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *random = x;
    x.to_le_bytes()[0]
}

/// The noise a receiver hears in a channel: the thermal floor raised by a radio's noise
/// figure.
fn noise_floor(bandwidth_hz: u32) -> Decibels {
    budget::noise_floor_dbm(bandwidth_hz) + RADIO_NOISE_FIGURE_DB
}

/// A level in dBm as the number of half decibels below zero the SX126x reports it in.
fn half_decibels_below(level: Decibels) -> u8 {
    let halves = (-level.hundredths() + 25).div_euclid(50);
    halves.clamp(0, i32::from(u8::MAX)) as u8
}

/// A ratio in dB as the quarter decibels, two's complement, both families report it in.
fn quarter_decibels(ratio: Decibels) -> u8 {
    let quarters = (ratio.hundredths() + 12).div_euclid(25);
    quarters.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8 as u8
}

/// The strength of the signal itself: the RSSI, lowered by the SNR below the noise.
fn signal_level(arrival: &Arrival) -> Decibels {
    if arrival.snr_db < Decibels::ZERO {
        arrival.rssi_dbm + arrival.snr_db
    } else {
        arrival.rssi_dbm
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sx126xMode {
    Sleep,
    StandbyRc,
    StandbyXosc,
    Fs,
    Rx { continuous: bool, timeout: bool },
    Cad,
}

const SX126X_STATUS_DATA_AVAILABLE: u8 = 0x2;
const SX126X_STATUS_TIMEOUT: u8 = 0x3;
const SX126X_STATUS_TX_DONE: u8 = 0x6;
const SX126X_RX_CONTINUOUS: u32 = 0x00FF_FFFF;

#[derive(Debug)]
struct Sx126xModel {
    board: sx126x::Board,
    mode: Sx126xMode,
    command_status: u8,
    packet_type: u8,
    frequency_word: u32,
    modulation: [u8; 4],
    packet: [u8; 6],
    output_dbm: i8,
    irq_mask: u16,
    irq: u16,
    tx_base: u8,
    rx_base: u8,
    buffer: [u8; 256],
    rx_buffer: [u8; 2],
    packet_status: [u8; 3],
    registers: BTreeMap<u16, u8>,
}

impl Sx126xModel {
    fn new(board: sx126x::Board) -> Sx126xModel {
        let mut registers = BTreeMap::new();
        let [high, low] = sx126x::config::SyncWord::Private.to_bytes();
        registers.insert(sx126x_register::LORA_SYNC_WORD, high);
        registers.insert(sx126x_register::LORA_SYNC_WORD + 1, low);
        Sx126xModel {
            board,
            mode: Sx126xMode::StandbyRc,
            command_status: 0,
            packet_type: sx126x::config::PacketType::Lora.code(),
            frequency_word: 0,
            modulation: [7, sx126x::config::LoraBandwidth::Khz125.code(), 0x01, 0],
            packet: [0, 8, 0, u8::MAX, 1, 0],
            output_dbm: 0,
            irq_mask: 0,
            irq: 0,
            tx_base: 0,
            rx_base: 0,
            buffer: [0; 256],
            rx_buffer: [0; 2],
            packet_status: [0; 3],
            registers,
        }
    }

    fn tuning(&self) -> Tuning {
        let [spreading_factor, bandwidth, coding_rate, _] = self.modulation;
        let bandwidth_hz = sx126x::config::LoraBandwidth::from_code(bandwidth)
            .map_or(0, sx126x::config::LoraBandwidth::hz);
        let denominator = match coding_rate {
            2 | 6 => 6,
            3 => 7,
            4 | 7 => 8,
            _ => 5,
        };
        let [preamble_high, preamble_low, header, _, crc, _] = self.packet;
        let mut link = LinkSettings::new(spreading_factor, bandwidth_hz)
            .with_coding_rate(denominator)
            .with_preamble(u16::from_be_bytes([preamble_high, preamble_low]));
        if header != 0 {
            link = link.implicit_header();
        }
        if crc == 0 {
            link = link.without_crc();
        }
        let high = self.register(sx126x_register::LORA_SYNC_WORD);
        let low = self.register(sx126x_register::LORA_SYNC_WORD + 1);
        Tuning {
            frequency_hz: sx126x::config::frequency_from_word(self.frequency_word),
            link,
            output_dbm: self.output_dbm,
            sync_word: SyncWord::from_byte((high & 0xF0) | (low >> 4)),
        }
    }

    fn register(&self, address: u16) -> u8 {
        self.registers.get(&address).copied().unwrap_or(0)
    }

    fn status(&self) -> u8 {
        let mode = match self.mode {
            Sx126xMode::Sleep | Sx126xMode::StandbyRc => 0x2,
            Sx126xMode::StandbyXosc => 0x3,
            Sx126xMode::Fs => 0x4,
            Sx126xMode::Rx { .. } | Sx126xMode::Cad => 0x5,
        };
        (mode << 4) | (self.command_status << 1)
    }

    fn raise(&mut self, events: Irq) {
        self.irq |= events.bits() & self.irq_mask;
    }

    fn command(&mut self, written: &[u8], sent: &mut Vec<Sent>) {
        if self.mode == Sx126xMode::Sleep {
            self.mode = Sx126xMode::StandbyRc;
        }
        let Some((&code, params)) = written.split_first() else {
            return;
        };
        let word = |at: usize| -> u32 {
            params.get(at..at + 3).map_or(0, |bytes| {
                u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]])
            })
        };
        let pair = |at: usize| -> u16 {
            params
                .get(at..at + 2)
                .map_or(0, |bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
        };
        match code {
            opcode::SET_SLEEP => {
                let warm = params.first().is_some_and(|config| config & 0x04 != 0);
                if !warm {
                    *self = Sx126xModel::new(self.board);
                }
                self.mode = Sx126xMode::Sleep;
            }
            opcode::SET_STANDBY => {
                self.mode = if params.first() == Some(&1) {
                    Sx126xMode::StandbyXosc
                } else {
                    Sx126xMode::StandbyRc
                };
            }
            opcode::SET_FS => self.mode = Sx126xMode::Fs,
            opcode::SET_TX => self.transmit(sent),
            opcode::SET_RX => {
                let timeout = word(0);
                self.mode = Sx126xMode::Rx {
                    continuous: timeout == SX126X_RX_CONTINUOUS,
                    timeout: timeout != 0 && timeout != SX126X_RX_CONTINUOUS,
                };
            }
            opcode::SET_CAD => self.mode = Sx126xMode::Cad,
            opcode::SET_PACKET_TYPE => {
                if let Some(&packet_type) = params.first() {
                    self.packet_type = packet_type;
                }
            }
            opcode::SET_RF_FREQUENCY => {
                if let Some(bytes) = params.get(..4) {
                    self.frequency_word =
                        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                }
            }
            opcode::SET_MODULATION_PARAMS => {
                if let Some(bytes) = params.get(..4) {
                    self.modulation.copy_from_slice(bytes);
                }
            }
            opcode::SET_PACKET_PARAMS => {
                if let Some(bytes) = params.get(..6) {
                    self.packet.copy_from_slice(bytes);
                }
            }
            opcode::SET_TX_PARAMS => {
                if let Some(&power) = params.first() {
                    self.output_dbm = power as i8;
                }
            }
            opcode::SET_DIO_IRQ_PARAMS => self.irq_mask = pair(0),
            opcode::CLEAR_IRQ_STATUS => self.irq &= !pair(0),
            opcode::SET_BUFFER_BASE_ADDRESS => {
                if let [tx_base, rx_base, ..] = params {
                    self.tx_base = *tx_base;
                    self.rx_base = *rx_base;
                }
            }
            opcode::WRITE_REGISTER => {
                let address = pair(0);
                for (offset, &value) in params.iter().skip(2).enumerate() {
                    let at = address.wrapping_add(offset as u16);
                    self.registers.insert(at, value);
                }
            }
            opcode::WRITE_BUFFER => {
                if let Some((&offset, bytes)) = params.split_first() {
                    for (index, &value) in bytes.iter().enumerate() {
                        let at = offset.wrapping_add(index as u8);
                        self.buffer[usize::from(at)] = value;
                    }
                }
            }
            _ => {}
        }
    }

    fn answer(
        &mut self,
        written: &[u8],
        answer: &mut [u8],
        air: &mut VecDeque<Arrival>,
        random: &mut u32,
    ) {
        if self.mode == Sx126xMode::Sleep {
            self.mode = Sx126xMode::StandbyRc;
        }
        answer.fill(0);
        let Some(&code) = written.first() else {
            return;
        };
        match code {
            opcode::GET_STATUS => {
                if let Some(first) = answer.first_mut() {
                    *first = self.status();
                }
            }
            opcode::GET_IRQ_STATUS => {
                self.settle(air);
                fill(answer, &self.irq.to_be_bytes());
            }
            opcode::GET_RX_BUFFER_STATUS => fill(answer, &self.rx_buffer),
            opcode::GET_PACKET_STATUS => fill(answer, &self.packet_status),
            opcode::GET_PACKET_TYPE => fill(answer, &[self.packet_type]),
            opcode::GET_RSSI_INST => {
                let bandwidth_hz = self.tuning().link.bandwidth_hz();
                fill(answer, &[half_decibels_below(noise_floor(bandwidth_hz))]);
            }
            opcode::READ_REGISTER => {
                let address = written
                    .get(1..3)
                    .map_or(0, |bytes| u16::from_be_bytes([bytes[0], bytes[1]]));
                for (offset, value) in answer.iter_mut().enumerate() {
                    let at = address.wrapping_add(offset as u16);
                    let random_number = sx126x_register::RANDOM_NUMBER;
                    *value = if (random_number..random_number + 4).contains(&at) {
                        noise_byte(random)
                    } else {
                        self.register(at)
                    };
                }
            }
            opcode::READ_BUFFER => {
                let offset = written.get(1).copied().unwrap_or(0);
                for (index, value) in answer.iter_mut().enumerate() {
                    *value = self.buffer[usize::from(offset.wrapping_add(index as u8))];
                }
            }
            _ => {}
        }
    }

    fn settle(&mut self, air: &mut VecDeque<Arrival>) {
        match self.mode {
            Sx126xMode::Rx {
                continuous,
                timeout,
            } => {
                if let Some(arrival) = air.pop_front() {
                    self.receive(&arrival);
                    if !continuous {
                        self.mode = Sx126xMode::StandbyRc;
                    }
                } else if timeout {
                    self.raise(Irq::TIMEOUT);
                    self.command_status = SX126X_STATUS_TIMEOUT;
                    self.mode = Sx126xMode::StandbyRc;
                }
            }
            Sx126xMode::Cad => {
                self.raise(Irq::CAD_DONE);
                if !air.is_empty() {
                    self.raise(Irq::CAD_DETECTED);
                }
                self.mode = Sx126xMode::StandbyRc;
            }
            _ => {}
        }
    }

    fn receive(&mut self, arrival: &Arrival) {
        for (index, &value) in arrival.payload.iter().enumerate() {
            let at = self.rx_base.wrapping_add(index as u8);
            self.buffer[usize::from(at)] = value;
        }
        self.rx_buffer = [arrival.payload.len() as u8, self.rx_base];
        self.packet_status = [
            half_decibels_below(arrival.rssi_dbm),
            quarter_decibels(arrival.snr_db),
            half_decibels_below(signal_level(arrival)),
        ];
        self.raise(Irq::PREAMBLE_DETECTED | Irq::HEADER_VALID | Irq::RX_DONE);
        if !arrival.intact {
            self.raise(Irq::CRC_ERROR);
        }
        self.command_status = SX126X_STATUS_DATA_AVAILABLE;
    }

    fn transmit(&mut self, sent: &mut Vec<Sent>) {
        let len = self.packet[3];
        let payload = (0..len)
            .map(|index| self.buffer[usize::from(self.tx_base.wrapping_add(index))])
            .collect();
        sent.push(Sent {
            tuning: self.tuning(),
            payload,
        });
        self.raise(Irq::TX_DONE);
        self.command_status = SX126X_STATUS_TX_DONE;
        self.mode = Sx126xMode::StandbyRc;
    }
}

const SX127X_MODE_MASK: u8 = 0x07;
const SX127X_MODE_STANDBY: u8 = 0x01;
const SX127X_MODE_TX: u8 = 0x03;
const SX127X_MODE_RX_CONTINUOUS: u8 = 0x05;
const SX127X_MODE_RX_SINGLE: u8 = 0x06;
const SX127X_MODE_CAD: u8 = 0x07;
const SX127X_PAGED_FIRST: usize = 0x0D;
const SX127X_PAGED_LAST: usize = 0x3F;
const SX127X_IMAGE_CAL_START: u8 = 0x40;
const SX127X_IMAGE_CAL_RUNNING: u8 = 0x20;

#[derive(Debug)]
struct Sx127xModel {
    board: sx127x::Board,
    lora: [u8; 128],
    fsk: [u8; 128],
    fifo: [u8; 256],
}

impl Sx127xModel {
    fn new(board: sx127x::Board) -> Sx127xModel {
        let mut lora = [0u8; 128];
        let mut fsk = [0u8; 128];
        for (address, value) in [
            (sx127x_register::OP_MODE, 0x09),
            (sx127x_register::FRF_MSB, 0x6C),
            (sx127x_register::FRF_MID, 0x80),
            (sx127x_register::FRF_LSB, 0x00),
            (sx127x_register::PA_CONFIG, 0x4F),
            (sx127x_register::PA_RAMP, 0x09),
            (sx127x_register::OCP, 0x2B),
            (sx127x_register::LNA, 0x20),
            (sx127x_register::VERSION, sx127x_register::VERSION_SX1276),
            (sx127x_register::TCXO, 0x09),
            (sx127x_register::PA_DAC, sx127x::config::PA_DAC_DEFAULT),
        ] {
            lora[usize::from(address)] = value;
        }
        for (address, value) in [
            (sx127x_register::FIFO_TX_BASE_ADDR, 0x80),
            (sx127x_register::MODEM_CONFIG_1, 0x72),
            (sx127x_register::MODEM_CONFIG_2, 0x70),
            (sx127x_register::SYMB_TIMEOUT_LSB, 0x64),
            (sx127x_register::PREAMBLE_LSB, 0x08),
            (sx127x_register::PAYLOAD_LENGTH, 0x01),
            (sx127x_register::MAX_PAYLOAD_LENGTH, 0xFF),
            (sx127x_register::DETECT_OPTIMIZE, 0xC3),
            (sx127x_register::INVERT_IQ, 0x27),
            (sx127x_register::DETECTION_THRESHOLD, 0x0A),
            (sx127x_register::SYNC_WORD, SyncWord::Private.to_byte()),
            (sx127x_register::INVERT_IQ_2, 0x1D),
        ] {
            lora[usize::from(address)] = value;
        }
        fsk[usize::from(sx127x_register::IMAGE_CAL)] = 0x82;
        Sx127xModel {
            board,
            lora,
            fsk,
            fifo: [0; 256],
        }
    }

    fn lora_mode(&self) -> bool {
        self.lora[usize::from(sx127x_register::OP_MODE)] & sx127x_register::LONG_RANGE_MODE != 0
    }

    fn slot(&mut self, address: u8) -> &mut u8 {
        let address = usize::from(address & 0x7F);
        if (SX127X_PAGED_FIRST..=SX127X_PAGED_LAST).contains(&address) && !self.lora_mode() {
            &mut self.fsk[address]
        } else {
            &mut self.lora[address]
        }
    }

    fn lora_register(&self, address: u8) -> u8 {
        self.lora[usize::from(address & 0x7F)]
    }

    fn mode(&self) -> u8 {
        self.lora_register(sx127x_register::OP_MODE) & SX127X_MODE_MASK
    }

    fn enter(&mut self, mode: u8) {
        let op_mode = usize::from(sx127x_register::OP_MODE);
        self.lora[op_mode] = (self.lora[op_mode] & !SX127X_MODE_MASK) | mode;
    }

    fn tuning(&self) -> Tuning {
        let frf = u32::from_be_bytes([
            0,
            self.lora_register(sx127x_register::FRF_MSB),
            self.lora_register(sx127x_register::FRF_MID),
            self.lora_register(sx127x_register::FRF_LSB),
        ]);
        let modem_1 = self.lora_register(sx127x_register::MODEM_CONFIG_1);
        let modem_2 = self.lora_register(sx127x_register::MODEM_CONFIG_2);
        let bandwidth_hz = sx127x::config::LoraBandwidth::from_code(modem_1 >> 4)
            .map_or(0, sx127x::config::LoraBandwidth::hz);
        let preamble = u16::from_be_bytes([
            self.lora_register(sx127x_register::PREAMBLE_MSB),
            self.lora_register(sx127x_register::PREAMBLE_LSB),
        ]);
        let mut link = LinkSettings::new(modem_2 >> 4, bandwidth_hz)
            .with_coding_rate(((modem_1 >> 1) & 0x07) + 4)
            .with_preamble(preamble);
        if modem_1 & 0x01 != 0 {
            link = link.implicit_header();
        }
        if modem_2 & 0x04 == 0 {
            link = link.without_crc();
        }
        let pa_config = self.lora_register(sx127x_register::PA_CONFIG);
        let output_power = (pa_config & 0x0F) as i8;
        let output_dbm = if pa_config & 0x80 != 0 {
            let high_power = self.lora_register(sx127x_register::PA_DAC) & 0x07 == 0x07;
            output_power + if high_power { 5 } else { 2 }
        } else if (pa_config >> 4) & 0x07 == 0x07 {
            output_power
        } else {
            output_power - 4
        };
        Tuning {
            frequency_hz: sx127x::config::frequency_from_word(frf),
            link,
            output_dbm,
            sync_word: SyncWord::from_byte(self.lora_register(sx127x_register::SYNC_WORD)),
        }
    }

    fn raise(&mut self, flags: IrqFlags) {
        let mask = self.lora_register(sx127x_register::IRQ_FLAGS_MASK);
        self.lora[usize::from(sx127x_register::IRQ_FLAGS)] |= flags.bits() & !mask;
    }

    fn write(&mut self, written: &[u8], sent: &mut Vec<Sent>) {
        let Some((&address, values)) = written.split_first() else {
            return;
        };
        let address = address & !sx127x_register::WRITE;
        for (offset, &value) in values.iter().enumerate() {
            let at = if address == sx127x_register::FIFO {
                address
            } else {
                address.wrapping_add(offset as u8) & 0x7F
            };
            self.write_register(at, value, sent);
        }
    }

    fn write_register(&mut self, address: u8, value: u8, sent: &mut Vec<Sent>) {
        match address {
            sx127x_register::FIFO => {
                let pointer = self.lora_register(sx127x_register::FIFO_ADDR_PTR);
                self.fifo[usize::from(pointer)] = value;
                self.lora[usize::from(sx127x_register::FIFO_ADDR_PTR)] = pointer.wrapping_add(1);
            }
            sx127x_register::OP_MODE => {
                self.lora[usize::from(address)] = value;
                if self.lora_mode() && value & SX127X_MODE_MASK == SX127X_MODE_TX {
                    self.transmit(sent);
                }
            }
            sx127x_register::IRQ_FLAGS if self.lora_mode() => {
                self.lora[usize::from(address)] &= !value;
            }
            sx127x_register::IMAGE_CAL if !self.lora_mode() => {
                self.fsk[usize::from(address)] =
                    value & !(SX127X_IMAGE_CAL_START | SX127X_IMAGE_CAL_RUNNING);
            }
            sx127x_register::VERSION => {}
            _ => *self.slot(address) = value,
        }
    }

    fn read(
        &mut self,
        written: &[u8],
        answer: &mut [u8],
        air: &mut VecDeque<Arrival>,
        random: &mut u32,
    ) {
        let address = written.first().copied().unwrap_or(0) & !sx127x_register::WRITE;
        for (offset, value) in answer.iter_mut().enumerate() {
            let at = if address == sx127x_register::FIFO {
                address
            } else {
                address.wrapping_add(offset as u8) & 0x7F
            };
            *value = self.read_register(at, air, random);
        }
    }

    fn read_register(&mut self, address: u8, air: &mut VecDeque<Arrival>, random: &mut u32) -> u8 {
        if !self.lora_mode() {
            return *self.slot(address);
        }
        match address {
            sx127x_register::FIFO => {
                let pointer = self.lora_register(sx127x_register::FIFO_ADDR_PTR);
                self.lora[usize::from(sx127x_register::FIFO_ADDR_PTR)] = pointer.wrapping_add(1);
                self.fifo[usize::from(pointer)]
            }
            sx127x_register::IRQ_FLAGS => {
                self.settle(air);
                self.lora_register(address)
            }
            sx127x_register::RSSI_WIDEBAND => noise_byte(random),
            sx127x_register::RSSI_VALUE => {
                let tuning = self.tuning();
                let port = Port::for_frequency(tuning.frequency_hz);
                let floor = noise_floor(tuning.link.bandwidth_hz());
                let above = (floor.hundredths() + 50).div_euclid(100) - port.rssi_offset_dbm();
                above.clamp(0, i32::from(u8::MAX)) as u8
            }
            _ => *self.slot(address),
        }
    }

    fn settle(&mut self, air: &mut VecDeque<Arrival>) {
        match self.mode() {
            SX127X_MODE_RX_CONTINUOUS => {
                if let Some(arrival) = air.pop_front() {
                    self.receive(&arrival);
                }
            }
            SX127X_MODE_RX_SINGLE => {
                if let Some(arrival) = air.pop_front() {
                    self.receive(&arrival);
                } else {
                    self.raise(IrqFlags::RX_TIMEOUT);
                }
                self.enter(SX127X_MODE_STANDBY);
            }
            SX127X_MODE_CAD => {
                self.raise(IrqFlags::CAD_DONE);
                if !air.is_empty() {
                    self.raise(IrqFlags::CAD_DETECTED);
                }
                self.enter(SX127X_MODE_STANDBY);
            }
            _ => {}
        }
    }

    fn receive(&mut self, arrival: &Arrival) {
        let base = self.lora_register(sx127x_register::FIFO_RX_BASE_ADDR);
        for (index, &value) in arrival.payload.iter().enumerate() {
            self.fifo[usize::from(base.wrapping_add(index as u8))] = value;
        }
        let port = Port::for_frequency(self.tuning().frequency_hz);
        let rssi = (arrival.rssi_dbm.hundredths() + 50).div_euclid(100) - port.rssi_offset_dbm();
        self.lora[usize::from(sx127x_register::FIFO_RX_CURRENT_ADDR)] = base;
        self.lora[usize::from(sx127x_register::RX_NB_BYTES)] = arrival.payload.len() as u8;
        self.lora[usize::from(sx127x_register::PKT_SNR_VALUE)] = quarter_decibels(arrival.snr_db);
        self.lora[usize::from(sx127x_register::PKT_RSSI_VALUE)] =
            rssi.clamp(0, i32::from(u8::MAX)) as u8;
        self.raise(IrqFlags::VALID_HEADER | IrqFlags::RX_DONE);
        if !arrival.intact {
            self.raise(IrqFlags::PAYLOAD_CRC_ERROR);
        }
    }

    fn transmit(&mut self, sent: &mut Vec<Sent>) {
        let base = self.lora_register(sx127x_register::FIFO_TX_BASE_ADDR);
        let len = self.lora_register(sx127x_register::PAYLOAD_LENGTH);
        let payload = (0..len)
            .map(|index| self.fifo[usize::from(base.wrapping_add(index))])
            .collect();
        sent.push(Sent {
            tuning: self.tuning(),
            payload,
        });
        self.raise(IrqFlags::TX_DONE);
        self.enter(SX127X_MODE_STANDBY);
    }
}

/// Copies an answer into the bytes a query reads, as far as both go.
fn fill(answer: &mut [u8], bytes: &[u8]) {
    for (slot, &byte) in answer.iter_mut().zip(bytes) {
        *slot = byte;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radio::{RadioConfig, Reception};
    use crate::sx126x::config::PowerAmplifier;
    use crate::sx127x::config::PaOutput;
    use pamoja_lora::region::Region;

    fn dr3() -> LinkSettings {
        Region::Eu868
            .plan()
            .link_settings(3)
            .expect("DR3 is a LoRa data rate")
    }

    fn carrier(hz: u32, asked: u32) -> bool {
        hz.abs_diff(asked) <= 61
    }

    fn chips() -> [Chip; 2] {
        [
            Chip::sx126x(sx126x::Board::new(PowerAmplifier::HighPower)),
            Chip::sx127x(sx127x::Board::new(PaOutput::PaBoost)),
        ]
    }

    #[test]
    fn the_driver_tunes_each_chip_to_what_it_was_asked() {
        for chip in chips() {
            let mut radio = chip.radio();
            radio.init().expect("the chip answers");
            let config = RadioConfig::new(868_100_000, dr3(), 14).lorawan_device();
            radio.configure(config).expect("a link the chip carries");
            let tuning = chip.tuning();
            assert!(
                carrier(tuning.frequency_hz, 868_100_000),
                "{:?}",
                chip.family()
            );
            assert_eq!(tuning.link.spreading_factor(), 9);
            assert_eq!(tuning.link.bandwidth_hz(), 125_000);
            assert_eq!(tuning.link.coding_rate_denominator(), 5);
            assert_eq!(tuning.output_dbm, 14);
            assert_eq!(tuning.sync_word, SyncWord::Public);
        }
    }

    #[test]
    fn a_transmission_puts_the_payload_on_the_air_as_tuned() {
        for chip in chips() {
            let mut radio = chip.radio();
            radio.init().expect("the chip answers");
            radio
                .configure(RadioConfig::new(868_100_000, dr3(), 14))
                .expect("a link the chip carries");
            let airtime = radio.transmit(b"21.5").expect("the frame leaves");
            assert_eq!(airtime, dr3().airtime_us(4));
            let sent = chip.sent();
            assert_eq!(sent.len(), 1);
            assert_eq!(sent[0].payload, b"21.5");
            assert!(carrier(sent[0].tuning.frequency_hz, 868_100_000));
            assert_eq!(sent[0].tuning.link.preamble_symbols(), 8);
        }
    }

    #[test]
    fn a_frame_on_the_air_arrives_with_the_levels_it_was_heard_at() {
        for chip in chips() {
            let mut radio = chip.radio();
            radio.init().expect("the chip answers");
            radio
                .configure(RadioConfig::new(868_100_000, dr3(), 14))
                .expect("a link the chip carries");
            chip.hear(b"ack", Decibels::from_db(-109), Decibels::from_tenths(-25));
            let mut buffer = [0u8; 255];
            let reception = radio
                .receive(&mut buffer, 1_000_000)
                .expect("the chip answers");
            let Reception::Frame { len, levels } = reception else {
                panic!("a frame was waiting: {reception:?}");
            };
            assert_eq!(&buffer[..len], b"ack");
            assert_eq!(levels.rssi_dbm, Decibels::from_db(-109));
            assert_eq!(levels.snr_db, Decibels::from_tenths(-25));
            assert_eq!(levels.signal_rssi_dbm, Decibels::from_hundredths(-11_150));
            assert_eq!(chip.waiting(), 0);
        }
    }

    #[test]
    fn nothing_on_the_air_times_a_reception_out_and_a_bad_crc_is_corrupt() {
        for chip in chips() {
            let mut radio = chip.radio();
            radio.init().expect("the chip answers");
            radio
                .configure(RadioConfig::new(868_100_000, dr3(), 14))
                .expect("a link the chip carries");
            let mut buffer = [0u8; 255];
            assert_eq!(
                radio.receive(&mut buffer, 1_000_000),
                Ok(Reception::Timeout)
            );
            chip.hear_corrupt(Decibels::from_db(-120), Decibels::from_db(-10));
            assert_eq!(
                radio.receive(&mut buffer, 1_000_000),
                Ok(Reception::Corrupt)
            );
        }
    }

    #[test]
    fn listening_takes_frame_after_frame_until_the_air_is_quiet() {
        for chip in chips() {
            let mut radio = chip.radio();
            radio.init().expect("the chip answers");
            radio
                .configure(RadioConfig::new(868_100_000, dr3(), 14))
                .expect("a link the chip carries");
            radio.listen().expect("the chip listens");
            let mut buffer = [0u8; 255];
            assert_eq!(radio.take_frame(&mut buffer), Ok(None));
            chip.hear(b"one", Decibels::from_db(-100), Decibels::from_db(5));
            chip.hear(b"two", Decibels::from_db(-101), Decibels::from_db(4));
            for expected in [b"one", b"two"] {
                let Ok(Some(Reception::Frame { len, .. })) = radio.take_frame(&mut buffer) else {
                    panic!("a frame was waiting");
                };
                assert_eq!(&buffer[..len], expected);
            }
            assert_eq!(radio.take_frame(&mut buffer), Ok(None));
        }
    }

    #[test]
    fn activity_is_detected_only_while_a_frame_is_on_the_air() {
        for chip in chips() {
            let mut radio = chip.radio();
            radio.init().expect("the chip answers");
            radio
                .configure(RadioConfig::new(868_100_000, dr3(), 14))
                .expect("a link the chip carries");
            assert_eq!(radio.detect(4), Ok(false));
            chip.hear(b"wake", Decibels::from_db(-110), Decibels::from_db(-5));
            assert_eq!(radio.detect(4), Ok(true));
            assert_eq!(
                chip.waiting(),
                1,
                "detection leaves the frame for the receiver"
            );
        }
    }

    #[test]
    fn the_noise_gives_a_random_number_and_leaves_the_air_alone() {
        for chip in chips() {
            let mut radio = chip.radio();
            radio.init().expect("the chip answers");
            radio
                .configure(RadioConfig::new(868_100_000, dr3(), 14))
                .expect("a link the chip carries");
            chip.hear(b"kept", Decibels::from_db(-100), Decibels::from_db(5));
            let first = radio.random().expect("noise");
            let second = radio.random().expect("noise");
            assert_ne!(first, second);
            assert_eq!(chip.waiting(), 1);
        }
    }

    #[test]
    fn a_reset_forgets_the_tuning_and_keeps_what_was_sent() {
        for chip in chips() {
            let mut radio = chip.radio();
            radio.init().expect("the chip answers");
            radio
                .configure(RadioConfig::new(868_100_000, dr3(), 14))
                .expect("a link the chip carries");
            radio.transmit(b"once").expect("the frame leaves");
            radio.init().expect("the chip answers again");
            assert_ne!(chip.tuning().frequency_hz, 868_100_000);
            assert_eq!(chip.sent().len(), 1);
        }
    }

    #[test]
    fn a_warm_sleep_keeps_the_tuning_and_the_next_command_wakes_the_chip() {
        for chip in chips() {
            let mut radio = chip.radio();
            radio.init().expect("the chip answers");
            radio
                .configure(RadioConfig::new(868_100_000, dr3(), 14))
                .expect("a link the chip carries");
            radio.sleep().expect("the chip sleeps");
            assert!(carrier(chip.tuning().frequency_hz, 868_100_000));
            radio
                .configure(RadioConfig::new(868_300_000, dr3(), 14))
                .expect("the chip wakes");
            assert!(carrier(chip.tuning().frequency_hz, 868_300_000));
        }
    }

    #[test]
    fn a_cold_sleep_forgets_the_tuning() {
        let chip = Chip::sx126x(sx126x::Board::new(PowerAmplifier::HighPower));
        let Radio::Sx126x(mut radio) = chip.radio() else {
            panic!("an SX126x");
        };
        radio.init().expect("the chip answers");
        let power = radio.tx_power(14);
        let config = sx126x::RadioConfig::new(868_100_000, dr3(), power);
        radio.configure(config).expect("a link the chip carries");
        radio.sleep(false).expect("the chip sleeps");
        assert_eq!(chip.tuning().frequency_hz, 0);
    }

    #[test]
    fn the_rfo_output_and_the_high_power_setting_report_their_own_power() {
        let rfo = Chip::sx127x(sx127x::Board::new(PaOutput::Rfo));
        let mut radio = rfo.radio();
        radio.init().expect("the chip answers");
        radio
            .configure(RadioConfig::new(
                433_175_000,
                LinkSettings::new(9, 125_000),
                10,
            ))
            .expect("a link the chip carries");
        assert_eq!(rfo.tuning().output_dbm, 10);
        assert!(carrier(rfo.tuning().frequency_hz, 433_175_000));

        let boost = Chip::sx127x(sx127x::Board::new(PaOutput::PaBoost));
        let mut radio = boost.radio();
        radio.init().expect("the chip answers");
        radio
            .configure(RadioConfig::new(
                915_000_000,
                LinkSettings::new(7, 500_000),
                20,
            ))
            .expect("a link the chip carries");
        assert_eq!(boost.tuning().output_dbm, 20);
        assert_eq!(boost.tuning().link.bandwidth_hz(), 500_000);
    }

    #[test]
    fn a_payload_longer_than_a_frame_is_cut_to_one() {
        let chip = Chip::sx126x(sx126x::Board::new(PowerAmplifier::HighPower));
        chip.hear(&[7; 300], Decibels::from_db(-100), Decibels::from_db(5));
        let mut radio = chip.radio();
        radio.init().expect("the chip answers");
        radio
            .configure(RadioConfig::new(868_100_000, dr3(), 14))
            .expect("a link the chip carries");
        let mut buffer = [0u8; 255];
        let Ok(Reception::Frame { len, .. }) = radio.receive(&mut buffer, 1_000_000) else {
            panic!("a frame was waiting");
        };
        assert_eq!(len, 255);
    }
}
