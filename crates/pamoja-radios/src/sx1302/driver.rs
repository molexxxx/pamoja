//! Driving a concentrator over a real bus.
//!
//! The modules beside this one are data: the transfers the chip answers, the registers it
//! keeps, the order its microcontrollers are loaded in. This walks that data against an SPI
//! device and a reset line, which is all a concentrator needs to be brought up.
//!
//! Bringing one up is four things, in order. The board is reset, which the host does rather
//! than the chip. The version register is read, which says whether anything is answering at
//! all. The front ends are reset through the concentrator itself. Then each microcontroller
//! is given its firmware, without which the chip reports a healthy version and hears nothing.
//!
//! The timings come from the reference implementation rather than from taste, and each one
//! says where it came from.

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{self, OutputPin};
use embedded_hal::spi::{Operation, SpiDevice};

use super::channel;
use super::chip::{self, Model};
use super::firmware::{self, LoadError, Mcu};
use super::mcu;
use super::register::{self, Register};
use super::spi as frame;
use super::sx1250;
use super::timestamp;
use super::tx::{self, Chain, FrontEnd, Trigger, TxStatus};

/// How long the reset line is held, in microseconds.
///
/// The reference platform script waits a tenth of a second between every edge it drives, and
/// this is that wait.
pub const RESET_HOLD_US: u32 = 100_000;

/// How long a front end is held in reset, in microseconds, as the reference holds it.
pub const RADIO_RESET_HOLD_US: u32 = 500_000;

/// How long a front end is given to come out of reset, in microseconds.
pub const RADIO_RESET_SETTLE_US: u32 = 10_000;

/// How long to wait between asking a microcontroller what it is doing, in microseconds.
pub const MCU_POLL_US: u32 = 1_000;

/// How many times to ask before giving up on it.
///
/// The reference waits forever. A gateway that hangs on a board which never came up is worse
/// than one that says so, which is why this ends.
pub const MCU_ATTEMPTS: u32 = 1_000;

/// How long an SX1250 is given to finish calibrating itself, in microseconds.
pub const RADIO_CALIBRATE_US: u32 = 10_000;

/// Why the concentrator could not be driven.
#[derive(Debug)]
pub enum ConcentratorError<E> {
    /// The SPI device failed.
    Spi(E),
    /// The reset line could not be driven.
    Pin(digital::ErrorKind),
    /// The version register answered with something else, so nothing is on the bus, or the
    /// board is unpowered, or it is wired to the wrong device.
    NotAnswering {
        /// What the version register read.
        version: u8,
    },
    /// The part number is one this crate does not drive.
    UnknownModel {
        /// What the one-time programmable memory answered with.
        byte: u8,
    },
    /// The firmware did not load.
    Firmware(LoadError),
    /// The receive buffer holds more than the caller left room for.
    Overrun {
        /// How many bytes are waiting.
        waiting: u16,
    },
    /// A front end did not settle into the mode it was put in.
    FrontEndMode {
        /// The mode it was asked for.
        wanted: u8,
        /// The mode its status reported.
        read: u8,
    },
    /// A front end refused its own image calibration.
    ImageCalibration,
    /// A carrier outside every band a front end calibrates over.
    UnsupportedBand {
        /// The carrier asked for, in hertz.
        hertz: u32,
    },
    /// A channel too far from the carrier for the radio to hear, however the register that
    /// carries the offset holds it.
    ChannelOutOfReach {
        /// The offset asked for, in hertz.
        offset_hz: i32,
    },
    /// A rate the fixed-rate receiver cannot be set to.
    ServiceRate {
        /// The spreading factor asked for.
        spreading_factor: u8,
        /// The bandwidth asked for, in hertz.
        bandwidth_hz: u32,
    },
    /// A packet the transmit chain cannot be told to send.
    Transmit(tx::TransmitError),
    /// A microcontroller never reached the state it was waited for.
    Stalled {
        /// The state it was waited for.
        wanted: u8,
        /// The state it was last reporting.
        reading: u8,
    },
    /// A microcontroller is running firmware this crate does not drive.
    WrongFirmware {
        /// The version expected.
        wanted: u8,
        /// The version it reported.
        running: u8,
    },
}

impl<E: core::fmt::Debug> core::error::Error for ConcentratorError<E> {}

impl<E: core::fmt::Debug> core::fmt::Display for ConcentratorError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConcentratorError::Spi(error) => write!(f, "the SPI device failed: {error:?}"),
            ConcentratorError::Pin(kind) => write!(f, "the reset line failed: {kind:?}"),
            ConcentratorError::NotAnswering { version } => write!(
                f,
                "the version register read {version:#04x} rather than {:#04x}, so no concentrator is answering",
                chip::EXPECTED_VERSION
            ),
            ConcentratorError::UnknownModel { byte } => {
                write!(f, "part number {byte:#04x} is not one this crate drives")
            }
            ConcentratorError::Firmware(error) => error.fmt(f),
            ConcentratorError::Overrun { waiting } => write!(
                f,
                "the receive buffer holds {waiting} bytes, which is more than there is room for"
            ),
            ConcentratorError::FrontEndMode { wanted, read } => write!(
                f,
                "the front end reports mode {read} rather than {wanted}, so it never settled"
            ),
            ConcentratorError::ImageCalibration => {
                f.write_str("the front end refused its own image calibration")
            }
            ConcentratorError::UnsupportedBand { hertz } => write!(
                f,
                "{hertz} Hz is outside every band a front end calibrates over"
            ),
            ConcentratorError::ChannelOutOfReach { offset_hz } => write!(
                f,
                "a channel {offset_hz} Hz from the carrier is outside the {} Hz a radio hears",
                channel::RX_BANDWIDTH_HZ
            ),
            ConcentratorError::ServiceRate {
                spreading_factor,
                bandwidth_hz,
            } => write!(
                f,
                "the service channel cannot run spreading factor {spreading_factor} at {bandwidth_hz} Hz"
            ),
            ConcentratorError::Transmit(error) => error.fmt(f),
            ConcentratorError::Stalled { wanted, reading } => write!(
                f,
                "a microcontroller stopped at {reading} rather than reaching {wanted}"
            ),
            ConcentratorError::WrongFirmware { wanted, running } => write!(
                f,
                "a microcontroller is running firmware {running} rather than {wanted}"
            ),
        }
    }
}

/// Turns a pin failure into an error, keeping only what the trait promises.
fn pin<P: digital::Error, E>(error: P) -> ConcentratorError<E> {
    ConcentratorError::Pin(error.kind())
}

/// A concentrator on an SPI device, with its reset line.
pub struct Sx1302<SPI, RESET, D> {
    spi: SPI,
    reset: RESET,
    delay: D,
}

impl<SPI, RESET, D> Sx1302<SPI, RESET, D> {
    /// Wraps a concentrator's SPI device and reset line. Nothing is sent until it is used.
    ///
    /// # Arguments
    ///
    /// * `spi` - the SPI device the concentrator answers on.
    /// * `reset` - the line its reset pin is on, which the host drives.
    /// * `delay` - how the driver waits.
    ///
    /// # Returns
    ///
    /// The concentrator.
    pub const fn new(spi: SPI, reset: RESET, delay: D) -> Sx1302<SPI, RESET, D> {
        Sx1302 { spi, reset, delay }
    }

    /// Gives the SPI device and the lines back.
    ///
    /// # Returns
    ///
    /// What was handed over, so a caller can drive something else with them.
    pub fn release(self) -> (SPI, RESET, D) {
        (self.spi, self.reset, self.delay)
    }
}

impl<SPI, RESET, D> Sx1302<SPI, RESET, D>
where
    SPI: SpiDevice,
    RESET: OutputPin,
    D: DelayNs,
{
    /// Pulses the reset line.
    ///
    /// The concentrator does not reset itself. On a gateway board the line is a host pin, and
    /// this drives it the way the reference platform script does.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the chip has been released and given time to come up.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Pin`] if the line cannot be driven.
    pub fn reset(&mut self) -> Result<(), ConcentratorError<SPI::Error>> {
        self.reset.set_high().map_err(pin)?;
        self.delay.delay_us(RESET_HOLD_US);
        self.reset.set_low().map_err(pin)?;
        self.delay.delay_us(RESET_HOLD_US);
        Ok(())
    }

    /// Reads the version register.
    ///
    /// # Returns
    ///
    /// What it answered with, which is [`EXPECTED_VERSION`](chip::EXPECTED_VERSION) on a
    /// concentrator that is powered and wired the right way round.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if the transfer fails.
    pub fn version(&mut self) -> Result<u8, ConcentratorError<SPI::Error>> {
        self.read_register(register::COMMON_VERSION)
    }

    /// Checks that a concentrator is answering at all.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the version register reads what it should.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::NotAnswering`] carrying what it read instead, which
    /// separates an unpowered board (zero) from a bus nothing is driving (all ones).
    pub fn check(&mut self) -> Result<(), ConcentratorError<SPI::Error>> {
        let version = self.version()?;
        if chip::answers(version) {
            Ok(())
        } else {
            Err(ConcentratorError::NotAnswering { version })
        }
    }

    /// Reads the part number out of the one-time programmable memory.
    ///
    /// # Returns
    ///
    /// Which concentrator this is.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn identify(&mut self) -> Result<Model, ConcentratorError<SPI::Error>> {
        let mut answer = 0u8;
        for step in chip::identify() {
            match step {
                chip::Step::Write(register, value) => self.write_register(register, value)?,
                chip::Step::Read(register) => answer = self.read_register(register)?,
            }
        }
        Ok(Model::of(answer))
    }

    /// Resets the front end on a chain, through the concentrator.
    ///
    /// The concentrator is clocked from the host while it does this, so it is still listening
    /// on the bus while the radio it drives is held down.
    ///
    /// # Arguments
    ///
    /// * `chain` - which chain's front end to reset.
    /// * `front_end` - which front end it is, since the SX1250 calibrates itself afterwards.
    ///
    /// # Returns
    ///
    /// `Ok(())` once it is out of reset and settled.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn reset_front_end(
        &mut self,
        chain: Chain,
        front_end: FrontEnd,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        let (enable, held) = match chain {
            Chain::A => (register::RF_EN_A_RADIO_EN, register::RF_EN_A_RADIO_RST),
            Chain::B => (register::RF_EN_B_RADIO_EN, register::RF_EN_B_RADIO_RST),
        };

        self.write_register(register::COMMON_CLK32_RIF_CTRL, 0)?;
        self.write_register(enable, 1)?;

        self.write_register(held, 1)?;
        self.delay.delay_us(RADIO_RESET_HOLD_US);
        self.write_register(held, 0)?;
        self.delay.delay_us(RADIO_RESET_SETTLE_US);

        // The SX1250 calibrates itself on the way out, and wants holding again while it does.
        if matches!(front_end, FrontEnd::Sx1250) {
            self.write_register(held, 1)?;
            self.delay.delay_us(RADIO_CALIBRATE_US);
        }

        Ok(())
    }

    /// Gives a microcontroller its firmware, and checks that it took.
    ///
    /// The image is written, read back, and compared before the microcontroller is released,
    /// and the chip is asked afterwards whether it accepted it. Both checks matter, and they
    /// fail differently: a read-back mismatch means the bus is not carrying the bytes, while
    /// a parity failure means they arrived and the image itself is wrong.
    ///
    /// # Arguments
    ///
    /// * `mcu` - which microcontroller to load.
    /// * `image` - the firmware, which is [`FIRMWARE_LEN`](firmware::FIRMWARE_LEN) bytes.
    ///
    /// # Returns
    ///
    /// `Ok(())` once it is loaded and the chip has accepted it.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Firmware`] for an image of the wrong size, a read-back
    /// that differs, or a parity check the chip refused.
    pub fn load_firmware(
        &mut self,
        mcu: Mcu,
        image: &[u8],
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        firmware::check_size(image).map_err(ConcentratorError::Firmware)?;

        for step in firmware::steps(mcu) {
            match step {
                firmware::Load::Write(register, value) => self.write_register(register, value)?,
                firmware::Load::WriteFirmware(address) => self.write_memory(address, image)?,
                firmware::Load::ReadBack(address) => self.verify_memory(address, image)?,
                firmware::Load::CheckParity(register) => {
                    if self.read_register(register)? != 0 {
                        return Err(ConcentratorError::Firmware(LoadError::ParityFailed));
                    }
                }
            }
        }

        Ok(())
    }

    /// Reads one register.
    ///
    /// # Arguments
    ///
    /// * `register` - which register.
    ///
    /// # Returns
    ///
    /// Its value, taken out of the byte it shares with its neighbors.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if the transfer fails.
    pub fn read_register(
        &mut self,
        register: Register,
    ) -> Result<u8, ConcentratorError<SPI::Error>> {
        let byte = self.read_byte(register.address)?;
        Ok(register.decode(byte))
    }

    /// Writes one register, leaving the rest of the byte it shares as it was.
    ///
    /// # Arguments
    ///
    /// * `register` - which register.
    /// * `value` - what to put there.
    ///
    /// # Returns
    ///
    /// `Ok(())` once written.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails. A register narrower than a
    /// byte is read first, so this can cost two transfers.
    pub fn write_register(
        &mut self,
        register: Register,
        value: u8,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        let byte = if register.is_whole_byte() {
            value
        } else {
            let held = self.read_byte(register.address)?;
            register.encode(held, value)
        };
        self.write_byte(register.address, byte)
    }

    /// Writes a run of bytes into the chip's memory.
    ///
    /// # Arguments
    ///
    /// * `address` - where the first byte lands.
    /// * `data` - the bytes.
    ///
    /// # Returns
    ///
    /// `Ok(())` once every chunk has gone out.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn write_memory(
        &mut self,
        address: u16,
        data: &[u8],
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        let mut at = address;
        let mut sent = 0usize;
        while sent < data.len() {
            let take = core::cmp::min(data.len() - sent, frame::BURST_CHUNK);
            let header = frame::burst_write_header(frame::TARGET_CONCENTRATOR, at);
            self.spi
                .transaction(&mut [
                    Operation::Write(&header),
                    Operation::Write(&data[sent..sent + take]),
                ])
                .map_err(ConcentratorError::Spi)?;
            at = at.wrapping_add(take as u16);
            sent += take;
        }
        Ok(())
    }

    /// Reads a run of bytes out of the chip's memory.
    ///
    /// # Arguments
    ///
    /// * `address` - where to start.
    /// * `into` - where to put them.
    ///
    /// # Returns
    ///
    /// `Ok(())` once it is filled.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn read_memory(
        &mut self,
        address: u16,
        into: &mut [u8],
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        self.burst_read(address, into, true)
    }

    /// Reads a port rather than a window, so every chunk comes from the same address.
    ///
    /// The receive buffer is a port: reading it moves the chip pointer along by itself. A
    /// reader that moves the address as well walks past the packets it came for, which only
    /// shows once there is more waiting than one chunk holds.
    ///
    /// # Arguments
    ///
    /// * `address` - the port to read.
    /// * `into` - where the bytes go, filled to its length.
    ///
    /// # Returns
    ///
    /// `Ok(())` once it is filled.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn read_fifo(
        &mut self,
        address: u16,
        into: &mut [u8],
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        self.burst_read(address, into, false)
    }

    /// How many bytes the receive buffer is holding.
    ///
    /// The count is read twice and the larger kept. A read of the pair can answer with less
    /// than is really there, and the reference guards against it the same way.
    ///
    /// # Returns
    ///
    /// The number of bytes waiting, which is zero when nothing has been heard.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn waiting(&mut self) -> Result<u16, ConcentratorError<SPI::Error>> {
        let first = self.buffer_count()?;
        let second = self.buffer_count()?;
        Ok(if second > first { second } else { first })
    }

    /// Takes everything the receive buffer is holding.
    ///
    /// What comes back is packets back to back, each wrapped in the metadata the chip writes
    /// around it, which [`packets`](super::rx::packets) walks.
    ///
    /// # Arguments
    ///
    /// * `into` - where the bytes go. [`BUFFER_LEN`](super::rx::BUFFER_LEN) always fits.
    ///
    /// # Returns
    ///
    /// What was read, which is empty when nothing has been heard.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Overrun`] if more is waiting than there is room for,
    /// having read nothing, so a larger buffer still gets it. Returns
    /// [`ConcentratorError::Spi`] if a transfer fails.
    pub fn receive<'a>(
        &mut self,
        into: &'a mut [u8],
    ) -> Result<&'a [u8], ConcentratorError<SPI::Error>> {
        let waiting = self.waiting()?;
        let wanted = usize::from(waiting);
        if wanted == 0 {
            return Ok(&[]);
        }
        if wanted > into.len() {
            return Err(ConcentratorError::Overrun { waiting });
        }
        self.read_fifo(register::RX_BUFFER_BASE, &mut into[..wanted])?;
        Ok(&into[..wanted])
    }

    /// Loads a packet and arms the trigger that sends it.
    ///
    /// The chip is not handed the packet and the moment together. The payload goes into the
    /// buffer of the chain, the delay it starts early by is programmed, and only then is the
    /// trigger armed. [`steps`](super::tx::steps) holds that order and this walks it.
    ///
    /// # Arguments
    ///
    /// * `chain` - which transmit chain sends it.
    /// * `transmit` - the packet and everything the modulator has to be told about it. A
    ///   chain given a payload without this sends it on whatever it was last set to.
    /// * `trigger` - what the send waits for.
    /// * `start_delay` - what [`start_delay`](super::tx::start_delay) worked out.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the trigger is armed. The packet leaves when the trigger says.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn transmit(
        &mut self,
        chain: Chain,
        transmit: &tx::Transmit<'_>,
        trigger: Trigger,
        start_delay: u16,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        // The modulator is configured and started before the payload is loaded, which is
        // the order the reference uses and the only one that describes the packet.
        for (register, value) in
            tx::settings(chain, transmit).map_err(ConcentratorError::Transmit)?
        {
            self.write_register(register, value)?;
        }

        self.send(chain, transmit.payload, trigger, start_delay)
    }

    /// Loads a payload into a chain that is already configured, and arms its trigger.
    ///
    /// [`transmit`](Self::transmit) describes a packet and then calls this. A caller answering
    /// on settings the chain already holds can call it directly, which is every downlink
    /// after the first on one channel.
    ///
    /// # Arguments
    ///
    /// * `chain` - which transmit chain sends it.
    /// * `payload` - the bytes the chain sends. For frequency shift keying the length byte
    ///   goes first, as the chip reads it out of the buffer.
    /// * `trigger` - what the send waits for.
    /// * `start_delay` - what [`start_delay`](super::tx::start_delay) worked out.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the trigger is armed. The packet leaves when the trigger says.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn send(
        &mut self,
        chain: Chain,
        payload: &[u8],
        trigger: Trigger,
        start_delay: u16,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        for step in tx::steps(chain, trigger, start_delay) {
            match step {
                tx::Send::SetStartDelay(delay) => {
                    let bytes = tx::start_delay_bytes(delay);
                    self.write_register(chain.start_delay_msb(), bytes[0])?;
                    self.write_register(chain.start_delay_lsb(), bytes[1])?;
                }
                tx::Send::OpenBuffer(register) => self.write_register(register, 1)?,
                tx::Send::WritePayload(address) => self.write_memory(address, payload)?,
                tx::Send::CloseBuffer(register) => self.write_register(register, 0)?,
                tx::Send::SetTimerTrigger(value) => {
                    for (index, byte) in tx::trigger_bytes(value).into_iter().enumerate() {
                        self.write_register(chain.timer_byte(index as u8), byte)?;
                    }
                }
                tx::Send::ResetTrigger(register) => self.write_register(register, 0)?,
                tx::Send::ArmTrigger(register) => self.write_register(register, 1)?,
            }
        }
        Ok(())
    }

    /// What a transmit chain is doing.
    ///
    /// # Arguments
    ///
    /// * `chain` - the chain to ask.
    ///
    /// # Returns
    ///
    /// Its state, which is [`TxStatus::Free`] when it will take another packet.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn tx_status(&mut self, chain: Chain) -> Result<TxStatus, ConcentratorError<SPI::Error>> {
        let value = self.read_register(chain.status())?;
        Ok(TxStatus::of(value))
    }

    /// Points a concentrator at the channels it is to listen on.
    ///
    /// A concentrator that has been reset and given firmware still hears nothing until this
    /// has run. Its receivers have been given no frequencies, no radio to take samples from,
    /// no spreading factors to look for, and no permission to run at all.
    /// [`steps`](super::channel::steps) holds that order and this walks it.
    ///
    /// # Arguments
    ///
    /// * `plan` - what to listen for.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the receivers are running.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn configure_channels(
        &mut self,
        plan: &channel::Plan,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        // The offset register reaches far past what the radio hears, so a channel outside
        // the window is refused here rather than configured into silence.
        for listener in &plan.channels {
            if listener.enabled && !channel::reachable(listener.offset_hz) {
                return Err(ConcentratorError::ChannelOutOfReach {
                    offset_hz: listener.offset_hz,
                });
            }
        }

        // The fixed-rate receiver takes a spreading factor and a bandwidth of its own, and
        // silently keeps its last ones for a pair it cannot be set to.
        if let Some(service) = plan.service {
            if !channel::reachable(service.offset_hz) {
                return Err(ConcentratorError::ChannelOutOfReach {
                    offset_hz: service.offset_hz,
                });
            }
            if !service.supported() {
                return Err(ConcentratorError::ServiceRate {
                    spreading_factor: service.spreading_factor,
                    bandwidth_hz: service.bandwidth_hz,
                });
            }
        }

        for channel::Step::Write(register, value) in channel::steps(plan) {
            self.write_register(register, value)?;
        }
        Ok(())
    }

    /// Takes the concentrator clock from the front end on a chain.
    ///
    /// Until this runs the chip is clocked by the bus, which is enough to answer registers
    /// and nothing else. The front end it takes the clock from has to be in receive, which
    /// is where [`setup_front_end`](Self::setup_front_end) leaves it.
    ///
    /// # Arguments
    ///
    /// * `chain` - the chain whose front end carries the clock.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the chip is running on the radio clock.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn select_clock(&mut self, chain: Chain) -> Result<(), ConcentratorError<SPI::Error>> {
        let (a, b) = match chain {
            Chain::A => (1, 0),
            Chain::B => (0, 1),
        };
        self.write_register(register::CLOCK_RADIO_A_SELECT, a)?;
        self.write_register(register::CLOCK_RADIO_B_SELECT, b)?;
        self.write_register(register::CLOCK_DIVIDER_ENABLE, 1)?;
        self.write_register(register::COMMON_CLK32_RIF_CTRL, 1)
    }

    /// Hands the front ends back to the gain control.
    ///
    /// The host drives them while they are being reset and tuned. From here on the gain
    /// control owns them, which is what lets it move gains while packets are arriving.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the host has let go.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn release_front_ends(&mut self) -> Result<(), ConcentratorError<SPI::Error>> {
        self.write_register(register::COMMON_HOST_RADIO_CTRL, 0)
    }

    /// Configures the gain control microcontroller and lets it run.
    ///
    /// Loading its firmware is only half of bringing it up: it comes out of reset waiting to
    /// be told what gains to use, one group of settings at a time, and a concentrator whose
    /// gain control was never configured hears very little.
    /// [`start_gain_control`](super::mcu::start_gain_control) holds that exchange and this
    /// walks it against the mailboxes.
    ///
    /// # Arguments
    ///
    /// * `front_end` - which front end the board carries, which decides the gains and the
    ///   firmware version expected.
    /// * `listen_before_talk` - whether the SX1261 beside the concentrator checks a channel
    ///   before the gateway talks on it.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the microcontroller has taken every group and been told there is no
    /// more to come.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::WrongFirmware`] if it is running an image this crate does
    /// not drive, [`ConcentratorError::Stalled`] if it stops acknowledging, and
    /// [`ConcentratorError::Spi`] if a transfer fails.
    pub fn start_gain_control(
        &mut self,
        front_end: FrontEnd,
        listen_before_talk: bool,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        for step in mcu::start_gain_control(front_end, listen_before_talk) {
            self.take(step, register::AGC_MCU_STATUS, true)?;
        }
        Ok(())
    }

    /// Configures the arbiter microcontroller and lets it run.
    ///
    /// The arbiter shares the receivers between the channels. It comes up halted and stays
    /// halted until it is told to resume.
    ///
    /// # Arguments
    ///
    /// * `dual_demodulation` - which spreading factors are demodulated twice over, one bit
    ///   each counting from SF5. Demodulating twice buys a finer timestamp and costs
    ///   capacity, so a gateway that does not need one passes zero.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the arbiter reports it is running again.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::WrongFirmware`] if it is running an image this crate does
    /// not drive, [`ConcentratorError::Stalled`] if it never resumes, and
    /// [`ConcentratorError::Spi`] if a transfer fails.
    pub fn start_arbiter(
        &mut self,
        dual_demodulation: u8,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        for step in mcu::start_arbiter(dual_demodulation) {
            self.take(step, register::ARB_MCU_STATUS, false)?;
        }
        Ok(())
    }

    /// Walks one step of a microcontroller exchange.
    ///
    /// The gain control answers through mailboxes and the arbiter through its own registers,
    /// which is the only difference between the two exchanges.
    fn take(
        &mut self,
        step: mcu::Step,
        status: Register,
        mailboxes: bool,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        match step {
            mcu::Step::Await(wanted) => self.settle(status, wanted),
            mcu::Step::Version(wanted) => {
                let running = if mailboxes {
                    self.read_register(register::agc_mailbox_read(0))?
                } else {
                    self.read_register(register::arbiter_status(0))?
                };
                if running == wanted {
                    Ok(())
                } else {
                    Err(ConcentratorError::WrongFirmware { wanted, running })
                }
            }
            mcu::Step::Write(index, value) => {
                let held = if mailboxes {
                    register::agc_mailbox_write(index)
                } else {
                    register::arbiter_config(index)
                };
                self.write_register(held, value)
            }
            // Only the gain control announces a group and is read back; the arbiter takes
            // its settings directly and reports through its own registers.
            mcu::Step::Notify(code) => self.write_register(register::agc_mailbox_write(3), code),
            mcu::Step::Verify(mailbox, wanted) => {
                let read = self.read_register(register::agc_mailbox_read(mailbox))?;
                if read == wanted {
                    Ok(())
                } else {
                    Err(ConcentratorError::Stalled {
                        wanted,
                        reading: read,
                    })
                }
            }
        }
    }

    /// Waits for a microcontroller to reach a state, and gives up rather than hanging.
    ///
    /// The reference spins here with no way out. A board that is unpowered, half loaded or
    /// wired to the wrong device would stop a gateway dead, so this bounds the wait and
    /// reports the state it was stuck on.
    fn settle(
        &mut self,
        status: Register,
        wanted: u8,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        let mut reading = 0;
        for _ in 0..MCU_ATTEMPTS {
            reading = self.read_register(status)?;
            if reading == wanted {
                return Ok(());
            }
            self.delay.delay_us(MCU_POLL_US);
        }
        Err(ConcentratorError::Stalled { wanted, reading })
    }

    /// Reads the counter packets are stamped against.
    ///
    /// Both counters come back in one burst, and the burst is read twice, because a read can
    /// catch a counter mid-step. When the two agree the first is kept; when a high byte has
    /// moved between them a third read settles it. That is what the reference does, and it
    /// is a different guard from the one on the receive byte count, where the larger of two
    /// readings is kept instead.
    ///
    /// # Arguments
    ///
    /// * `counter` - the state carried between readings, which is what widens a counter of
    ///   27 bits past its rollover.
    ///
    /// # Returns
    ///
    /// The free running counter and then the pulse one, in microseconds.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if a transfer fails.
    pub fn counter(
        &mut self,
        counter: &mut timestamp::Counter,
    ) -> Result<(u32, u32), ConcentratorError<SPI::Error>> {
        let mut bytes = [0u8; timestamp::COUNTERS_LEN];
        self.read_memory(timestamp::COUNTERS, &mut bytes)?;

        let mut again = [0u8; timestamp::COUNTERS_LEN];
        self.read_memory(timestamp::COUNTERS, &mut again)?;

        if timestamp::disagrees(&bytes, &again) {
            self.read_memory(timestamp::COUNTERS, &mut bytes)?;
        }
        Ok(counter.read(&bytes))
    }

    /// Sends a command to the front end on a chain.
    ///
    /// A front end answers on the same bus as the concentrator, with the first byte of the
    /// frame naming it rather than the concentrator. It gives the host no line to watch, so
    /// this waits [`WAIT_BUSY_US`](super::sx1250::WAIT_BUSY_US) first, as the reference does.
    ///
    /// # Arguments
    ///
    /// * `chain` - the chain the front end is on.
    /// * `opcode` - the command.
    /// * `data` - what the command carries.
    ///
    /// # Returns
    ///
    /// `Ok(())` once it has been sent.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if the transfer fails.
    pub fn radio_command(
        &mut self,
        chain: Chain,
        opcode: u8,
        data: &[u8],
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        self.delay.delay_us(sx1250::WAIT_BUSY_US);
        let header = sx1250::header(chain, opcode);
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Write(data)])
            .map_err(ConcentratorError::Spi)
    }

    /// Asks the front end on a chain for an answer.
    ///
    /// # Arguments
    ///
    /// * `chain` - the chain the front end is on.
    /// * `opcode` - the command.
    /// * `into` - where the answer goes, filled to its length.
    ///
    /// # Returns
    ///
    /// `Ok(())` once it is filled.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::Spi`] if the transfer fails.
    pub fn radio_query(
        &mut self,
        chain: Chain,
        opcode: u8,
        into: &mut [u8],
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        self.delay.delay_us(sx1250::WAIT_BUSY_US);
        let header = sx1250::header(chain, opcode);
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Read(into)])
            .map_err(ConcentratorError::Spi)
    }

    /// Brings the front end on a chain up and leaves it listening.
    ///
    /// A concentrator hears nothing until this has run, because the carrier is set here
    /// rather than on the concentrator. It also matters beyond receiving: the concentrator
    /// takes its clock from a front end that is in receive, so one left idle stops the chip
    /// beside it. [`setup`](super::sx1250::setup) holds the order and this walks it.
    ///
    /// # Arguments
    ///
    /// * `chain` - the chain the front end is on.
    /// * `hertz` - the carrier to tune to.
    /// * `single_input` - whether the board wires the front end single ended rather than
    ///   differential.
    ///
    /// # Returns
    ///
    /// `Ok(())` once it is tuned and listening.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::FrontEndMode`] if it does not settle into a standby it
    /// was put in, which is what an unpowered or unclocked front end looks like, and
    /// [`ConcentratorError::Spi`] if a transfer fails.
    pub fn setup_front_end(
        &mut self,
        chain: Chain,
        hertz: u32,
        single_input: bool,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        for step in sx1250::setup(hertz, single_input) {
            match step {
                sx1250::Step::Standby(standby) => {
                    self.radio_command(chain, sx1250::OP_SET_STANDBY, &[standby.value()])?;
                }
                sx1250::Step::Wait(micros) => self.delay.delay_us(micros),
                sx1250::Step::ExpectMode(wanted) => {
                    let mut status = [0u8; 1];
                    self.radio_query(chain, sx1250::OP_GET_STATUS, &mut status)?;
                    let read = sx1250::mode(status[0]);
                    if read != wanted {
                        return Err(ConcentratorError::FrontEndMode { wanted, read });
                    }
                }
                sx1250::Step::Calibrate(mask) => {
                    self.radio_command(chain, sx1250::OP_CALIBRATE, &[mask])?;
                }
                sx1250::Step::Register(address, value) => {
                    let [high, low] = address.to_be_bytes();
                    self.radio_command(chain, sx1250::OP_WRITE_REGISTER, &[high, low, value])?;
                }
                sx1250::Step::ClearOffset(address) => {
                    let [high, low] = address.to_be_bytes();
                    self.radio_command(chain, sx1250::OP_WRITE_REGISTER, &[high, low, 0, 0, 0])?;
                }
                sx1250::Step::Tune(carrier) => {
                    let bytes = sx1250::frequency_bytes(carrier);
                    self.radio_command(chain, sx1250::OP_SET_RF_FREQUENCY, &bytes)?;
                }
                sx1250::Step::ListenContinuously => {
                    self.radio_command(chain, sx1250::OP_SET_RX, &sx1250::RX_CONTINUOUS)?;
                }
            }
        }
        Ok(())
    }

    /// Calibrates the front end on a chain over the band a carrier falls in.
    ///
    /// The chip calibrates a band rather than a frequency, and it knows five of them. This
    /// is the SX1250 half of radio calibration: it runs once the front ends have been reset
    /// and while the host still drives them, before the receivers are given their channels.
    /// The front ends that came before calibrate through a firmware image instead.
    ///
    /// # Arguments
    ///
    /// * `chain` - the chain the front end is on.
    /// * `hertz` - the carrier it will be tuned to.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the chip reports no fault.
    ///
    /// # Errors
    ///
    /// Returns [`ConcentratorError::UnsupportedBand`] for a carrier outside every band,
    /// without sending anything, [`ConcentratorError::ImageCalibration`] if the chip reports
    /// the calibration failed, and [`ConcentratorError::Spi`] if a transfer fails.
    pub fn calibrate_front_end(
        &mut self,
        chain: Chain,
        hertz: u32,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        let band = sx1250::image_band(hertz).ok_or(ConcentratorError::UnsupportedBand { hertz })?;
        self.radio_command(chain, sx1250::OP_CALIBRATE_IMAGE, &band)?;
        self.delay.delay_us(sx1250::IMAGE_CALIBRATE_US);

        let mut errors = [0u8; 3];
        self.radio_query(chain, sx1250::OP_GET_DEVICE_ERRORS, &mut errors)?;
        if sx1250::image_failed(errors[2]) {
            return Err(ConcentratorError::ImageCalibration);
        }
        Ok(())
    }

    /// Reads the byte count once, high bits first.
    fn buffer_count(&mut self) -> Result<u16, ConcentratorError<SPI::Error>> {
        let high = self.read_register(register::RX_BUFFER_NB_BYTES_MSB)?;
        let low = self.read_register(register::RX_BUFFER_NB_BYTES_LSB)?;
        Ok((u16::from(high) << 8) | u16::from(low))
    }

    /// Reads a run of bytes a chunk at a time, moving the address on only for a window.
    fn burst_read(
        &mut self,
        address: u16,
        into: &mut [u8],
        advance: bool,
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        let mut at = address;
        let mut read = 0usize;
        while read < into.len() {
            let take = core::cmp::min(into.len() - read, frame::BURST_CHUNK);
            let header = frame::burst_read_header(frame::TARGET_CONCENTRATOR, at);
            self.spi
                .transaction(&mut [
                    Operation::Write(&header),
                    Operation::Read(&mut into[read..read + take]),
                ])
                .map_err(ConcentratorError::Spi)?;
            if advance {
                at = at.wrapping_add(take as u16);
            }
            read += take;
        }
        Ok(())
    }

    /// Reads memory back a chunk at a time and compares it with what was written.
    ///
    /// A whole firmware image is eight kilobytes, which is a great deal of stack to ask for
    /// on a small host, so this compares as it goes rather than holding a second copy.
    fn verify_memory(
        &mut self,
        address: u16,
        written: &[u8],
    ) -> Result<(), ConcentratorError<SPI::Error>> {
        let mut chunk = [0u8; frame::BURST_CHUNK];
        let mut at = address;
        let mut done = 0usize;

        while done < written.len() {
            let take = core::cmp::min(written.len() - done, frame::BURST_CHUNK);
            self.read_memory(at, &mut chunk[..take])?;

            if let Err(LoadError::ReadBackDiffers { at: differs }) =
                firmware::compare(&written[done..done + take], &chunk[..take])
            {
                return Err(ConcentratorError::Firmware(LoadError::ReadBackDiffers {
                    at: done + differs,
                }));
            }

            at = at.wrapping_add(take as u16);
            done += take;
        }

        Ok(())
    }

    /// Reads the byte a register lives in.
    fn read_byte(&mut self, address: u16) -> Result<u8, ConcentratorError<SPI::Error>> {
        let out = frame::read(frame::TARGET_CONCENTRATOR, address);
        let mut back = [0u8; frame::READ_LEN];
        self.spi
            .transfer(&mut back, &out)
            .map_err(ConcentratorError::Spi)?;
        Ok(frame::read_value(&back).unwrap_or(0))
    }

    /// Writes the byte a register lives in.
    fn write_byte(&mut self, address: u16, value: u8) -> Result<(), ConcentratorError<SPI::Error>> {
        self.spi
            .write(&frame::write(frame::TARGET_CONCENTRATOR, address, value))
            .map_err(ConcentratorError::Spi)
    }
}

#[cfg(test)]
mod tests {
    use pamoja_hal::script::{DelayLog, PinScript, SpiScript, SpiStep};

    use super::*;

    /// A concentrator whose bus answers with exactly these steps.
    fn driven(steps: Vec<SpiStep>) -> Sx1302<SpiScript, PinScript, DelayLog> {
        Sx1302::new(SpiScript::new(steps), PinScript::new([]), DelayLog::new())
    }

    #[test]
    fn a_channel_the_radio_cannot_hear_is_refused_before_the_bus_is_touched() {
        // Nothing is scripted, so any transfer at all would fail the script. The refusal
        // comes first, which is the point: the chip would take this offset and then hear
        // nothing on that channel.
        let mut chip = driven(Vec::new());
        let plan = channel::Plan::new(867_800_000, &[1_000_000]);

        let refused = chip
            .configure_channels(&plan)
            .expect_err("a megahertz is past the radio window");
        assert!(matches!(
            refused,
            ConcentratorError::ChannelOutOfReach {
                offset_hz: 1_000_000
            }
        ));
    }

    /// The transfer that reads one byte, and the answer it brings back.
    fn reads(address: u16, value: u8) -> SpiStep {
        let mut reply = [0u8; frame::READ_LEN];
        reply[frame::READ_LEN - 1] = value;
        SpiStep::transfer(
            frame::read(frame::TARGET_CONCENTRATOR, address).to_vec(),
            reply.to_vec(),
        )
    }

    /// The transfer that writes one byte.
    fn writes(address: u16, value: u8) -> SpiStep {
        SpiStep::write(frame::write(frame::TARGET_CONCENTRATOR, address, value).to_vec())
    }

    /// The two transfers that read a run of bytes out of one address.
    fn fetches(address: u16, reply: Vec<u8>) -> Vec<SpiStep> {
        vec![
            SpiStep::write(frame::burst_read_header(frame::TARGET_CONCENTRATOR, address).to_vec()),
            SpiStep::read(reply),
        ]
    }

    /// The two transfers that write a run of bytes to one address.
    fn sends(address: u16, payload: Vec<u8>) -> Vec<SpiStep> {
        vec![
            SpiStep::write(frame::burst_write_header(frame::TARGET_CONCENTRATOR, address).to_vec()),
            SpiStep::write(payload),
        ]
    }

    #[test]
    fn pointing_a_concentrator_at_its_channels_ends_by_switching_them_on() {
        let plan = channel::Plan::new(867_500_000, &[-400_000, -200_000, 0]);

        // A register that owns its byte is one write. One that shares a byte is read first,
        // so the neighbors beside it survive, which is two transfers rather than one.
        let mut steps = Vec::new();
        for channel::Step::Write(held, value) in channel::steps(&plan) {
            if held.is_whole_byte() {
                steps.push(writes(held.address, value));
            } else {
                steps.push(reads(held.address, 0));
                steps.push(writes(held.address, held.encode(0, value)));
            }
        }

        // The bus refuses anything the driver issues out of order or in the wrong shape.
        let mut chip = driven(steps);
        chip.configure_channels(&plan).expect("the bus answers");

        let channel::Step::Write(last, value) =
            channel::steps(&plan).last().expect("there are steps");
        assert_eq!(last, register::COMMON_GLOBAL_ENABLE);
        assert_eq!(value, 0x01);
    }

    #[test]
    fn the_counter_is_read_twice_and_kept_from_the_first() {
        // The counter runs at 32 MHz, so 64 ticks is two microseconds and 32 is one.
        let mut steps = fetches(0x6101, vec![0, 0, 0, 32, 0, 0, 0, 64]);
        steps.extend(fetches(0x6101, vec![0, 0, 0, 32, 0, 0, 0, 64]));
        let mut chip = driven(steps);

        let mut counter = timestamp::Counter::new();
        let (free, pulse) = chip.counter(&mut counter).expect("the bus answers");
        assert_eq!((free, pulse), (2, 1));
    }

    #[test]
    fn a_counter_that_stepped_between_readings_is_read_again() {
        // The second reading carries a different high byte, so neither is trusted and a
        // third is taken.
        let mut steps = fetches(0x6101, vec![0, 0, 0, 32, 0, 0, 0, 64]);
        steps.extend(fetches(0x6101, vec![0, 0, 0, 32, 1, 0, 0, 0]));
        steps.extend(fetches(0x6101, vec![0, 0, 0, 32, 0, 0, 1, 0]));
        let mut chip = driven(steps);

        let mut counter = timestamp::Counter::new();
        let (free, _) = chip.counter(&mut counter).expect("the bus answers");
        assert_eq!(free, 8);
    }

    /// The transfers that send one command to a front end.
    fn a_front_end_is_told(chain: Chain, opcode: u8, data: Vec<u8>) -> Vec<SpiStep> {
        vec![
            SpiStep::write(sx1250::header(chain, opcode).to_vec()),
            SpiStep::write(data),
        ]
    }

    /// The transfers that ask a front end for an answer.
    fn a_front_end_answers(chain: Chain, opcode: u8, reply: Vec<u8>) -> Vec<SpiStep> {
        vec![
            SpiStep::write(sx1250::header(chain, opcode).to_vec()),
            SpiStep::read(reply),
        ]
    }

    #[test]
    fn the_clock_moves_to_the_radio_before_anything_is_configured() {
        // Three bits share the byte at the clock block, so each is read before it is
        // written, and the last write moves the internal clock over to the radio.
        let mut chip = driven(vec![
            reads(0x57c0, 0),
            writes(0x57c0, 0b0000_0001),
            reads(0x57c0, 0b0000_0001),
            writes(0x57c0, 0b0000_0001),
            reads(0x57c0, 0b0000_0001),
            writes(0x57c0, 0b0000_0101),
            reads(0x5601, 0),
            writes(0x5601, 0b0001_0000),
        ]);

        chip.select_clock(Chain::A).expect("the bus answers");
    }

    #[test]
    fn the_second_radio_carries_the_clock_when_it_is_asked_to() {
        let mut chip = driven(vec![
            reads(0x57c0, 0b0000_0001),
            writes(0x57c0, 0b0000_0000),
            reads(0x57c0, 0),
            writes(0x57c0, 0b0000_0010),
            reads(0x57c0, 0b0000_0010),
            writes(0x57c0, 0b0000_0110),
            reads(0x5601, 0),
            writes(0x5601, 0b0001_0000),
        ]);

        chip.select_clock(Chain::B).expect("the bus answers");
    }

    #[test]
    fn a_microcontroller_that_never_answers_is_given_up_on() {
        // The reference waits here forever. A board that is unpowered or half loaded would
        // stop a gateway dead, so this ends and says what it was stuck on.
        let steps = vec![reads(0x5781, 0x00); MCU_ATTEMPTS as usize];
        let mut chip = driven(steps);

        match chip.start_gain_control(FrontEnd::Sx1250, false) {
            Err(ConcentratorError::Stalled { wanted, reading }) => {
                assert_eq!((wanted, reading), (mcu::STATUS_RUNNING, 0x00));
            }
            other => panic!("a silent microcontroller is reported: {other:?}"),
        }
    }

    #[test]
    fn firmware_the_crate_does_not_drive_is_named() {
        // It came up and answered, but it is not the image this crate knows how to talk to.
        let mut chip = driven(vec![reads(0x5781, mcu::STATUS_RUNNING), reads(0x5790, 3)]);

        match chip.start_gain_control(FrontEnd::Sx1250, false) {
            Err(ConcentratorError::WrongFirmware { wanted, running }) => {
                assert_eq!((wanted, running), (mcu::AGC_VERSION_SX1250, 3));
            }
            other => panic!("the version it is running is reported: {other:?}"),
        }
    }

    #[test]
    fn the_arbiter_reads_its_version_from_its_own_register() {
        // The arbiter answers through debug registers rather than mailboxes, which is the
        // only difference between the two exchanges.
        let mut chip = driven(vec![reads(0x6081, mcu::STATUS_RUNNING), reads(0x608d, 1)]);

        match chip.start_arbiter(0x00) {
            Err(ConcentratorError::WrongFirmware { wanted, running }) => {
                assert_eq!((wanted, running), (mcu::ARB_VERSION, 1));
            }
            other => panic!("the arbiter version is read from 0x608d: {other:?}"),
        }
    }

    #[test]
    fn bringing_a_front_end_up_tunes_it_and_leaves_it_listening() {
        // Every transfer a bring-up issues, in order. The status reads answer with the mode
        // in the top bits, which is what the chip does once it has settled.
        let mut steps = a_front_end_is_told(Chain::A, sx1250::OP_SET_STANDBY, vec![0x00]);
        steps.extend(a_front_end_answers(
            Chain::A,
            sx1250::OP_GET_STATUS,
            vec![0x20],
        ));
        steps.extend(a_front_end_is_told(
            Chain::A,
            sx1250::OP_CALIBRATE,
            vec![0x7f],
        ));
        steps.extend(a_front_end_is_told(
            Chain::A,
            sx1250::OP_SET_STANDBY,
            vec![0x01],
        ));
        steps.extend(a_front_end_answers(
            Chain::A,
            sx1250::OP_GET_STATUS,
            vec![0x30],
        ));
        for (high, low, value) in [
            (0x06, 0xa1, 0x01),
            (0x06, 0xa2, 0x00),
            (0x06, 0xa3, 0x00),
            (0x05, 0x82, 0x00),
            (0x05, 0x83, 0x00),
            (0x05, 0x84, 0x00),
            (0x05, 0x85, 0x00),
            (0x05, 0x80, 0x00),
            (0x08, 0xb6, 0x2a),
        ] {
            steps.extend(a_front_end_is_told(
                Chain::A,
                sx1250::OP_WRITE_REGISTER,
                vec![high, low, value],
            ));
        }
        steps.extend(a_front_end_is_told(
            Chain::A,
            sx1250::OP_SET_RF_FREQUENCY,
            vec![0x36, 0x41, 0x99, 0x99],
        ));
        steps.extend(a_front_end_is_told(
            Chain::A,
            sx1250::OP_WRITE_REGISTER,
            vec![0x08, 0x8f, 0x00, 0x00, 0x00],
        ));
        steps.extend(a_front_end_is_told(
            Chain::A,
            sx1250::OP_SET_RX,
            vec![0xff, 0xff, 0xff],
        ));
        steps.extend(a_front_end_is_told(
            Chain::A,
            sx1250::OP_WRITE_REGISTER,
            vec![0x05, 0x87, 0x0b],
        ));

        let mut chip = driven(steps);
        chip.setup_front_end(Chain::A, 868_100_000, false)
            .expect("the bus answers");
    }

    #[test]
    fn a_front_end_that_never_settles_says_what_it_read() {
        // An unpowered or unclocked front end answers with a mode it was not put in, and
        // going on to tune it would be tuning nothing.
        let mut steps = a_front_end_is_told(Chain::B, sx1250::OP_SET_STANDBY, vec![0x00]);
        steps.extend(a_front_end_answers(
            Chain::B,
            sx1250::OP_GET_STATUS,
            vec![0x00],
        ));

        let mut chip = driven(steps);
        match chip.setup_front_end(Chain::B, 868_100_000, false) {
            Err(ConcentratorError::FrontEndMode { wanted, read }) => {
                assert_eq!((wanted, read), (0x02, 0x00));
            }
            other => panic!("a front end that never settles is reported: {other:?}"),
        }
    }

    #[test]
    fn a_carrier_outside_every_band_is_refused_before_the_bus() {
        // Nothing is sent, so a configuration mistake is not mistaken for a bus fault.
        let mut chip = driven(vec![]);
        match chip.calibrate_front_end(Chain::A, 880_000_000) {
            Err(ConcentratorError::UnsupportedBand { hertz }) => assert_eq!(hertz, 880_000_000),
            other => panic!("a carrier between the bands is refused: {other:?}"),
        }
    }

    #[test]
    fn a_front_end_that_refuses_its_calibration_is_reported() {
        let mut steps = a_front_end_is_told(Chain::A, sx1250::OP_CALIBRATE_IMAGE, vec![0xd7, 0xdb]);
        steps.extend(a_front_end_answers(
            Chain::A,
            sx1250::OP_GET_DEVICE_ERRORS,
            vec![0x00, 0x00, 0b0001_0000],
        ));

        let mut chip = driven(steps);
        assert!(matches!(
            chip.calibrate_front_end(Chain::A, 868_100_000),
            Err(ConcentratorError::ImageCalibration)
        ));
    }

    #[test]
    fn a_calibrated_front_end_reports_nothing() {
        let mut steps = a_front_end_is_told(Chain::A, sx1250::OP_CALIBRATE_IMAGE, vec![0xe1, 0xe9]);
        steps.extend(a_front_end_answers(
            Chain::A,
            sx1250::OP_GET_DEVICE_ERRORS,
            vec![0x00, 0x00, 0x00],
        ));

        let mut chip = driven(steps);
        chip.calibrate_front_end(Chain::A, 915_000_000)
            .expect("the bus answers");
    }

    #[test]
    fn a_quiet_concentrator_hands_back_nothing() {
        let mut chip = driven(vec![
            reads(0x58c8, 0),
            reads(0x58c9, 0),
            reads(0x58c8, 0),
            reads(0x58c9, 0),
        ]);

        let mut buffer = [0u8; 32];
        assert!(chip
            .receive(&mut buffer)
            .expect("the bus answers")
            .is_empty());
    }

    #[test]
    fn the_byte_count_is_read_twice_and_the_larger_kept() {
        // A read of the pair can answer with less than is really there, so two are taken and
        // the larger wins. Here the second read sees three more bytes, and all five arrive.
        let mut steps = vec![
            reads(0x58c8, 0),
            reads(0x58c9, 2),
            reads(0x58c8, 0),
            reads(0x58c9, 5),
        ];
        steps.extend(fetches(0x4000, vec![1, 2, 3, 4, 5]));
        let mut chip = driven(steps);

        let mut buffer = [0u8; 32];
        let heard = chip.receive(&mut buffer).expect("the bus answers");
        assert_eq!(heard, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn more_than_fits_is_refused_before_a_byte_is_read() {
        // Nothing is taken out of the buffer, so the same packets are still there for a
        // caller that comes back with room for them.
        let mut chip = driven(vec![
            reads(0x58c8, 0),
            reads(0x58c9, 16),
            reads(0x58c8, 0),
            reads(0x58c9, 16),
        ]);

        let mut buffer = [0u8; 8];
        match chip.receive(&mut buffer) {
            Err(ConcentratorError::Overrun { waiting }) => assert_eq!(waiting, 16),
            other => panic!("a buffer too small is refused, not filled: {other:?}"),
        }
    }

    #[test]
    fn a_fetch_larger_than_one_chunk_reads_the_same_port_again() {
        // The receive buffer is a port, not a window. Reading it moves the chip pointer
        // along, so the second chunk comes from the same address. A reader that advanced
        // would ask for the address a chunk higher and get the wrong bytes.
        let mut steps = fetches(0x4000, vec![0xa5; frame::BURST_CHUNK]);
        steps.extend(fetches(0x4000, vec![0xc0; 100]));
        let mut chip = driven(steps);

        let mut buffer = [0u8; frame::BURST_CHUNK + 100];
        chip.read_fifo(0x4000, &mut buffer)
            .expect("the bus answers");
        assert_eq!(buffer[0], 0xa5);
        assert_eq!(buffer[frame::BURST_CHUNK], 0xc0);
    }

    #[test]
    fn a_packet_the_chain_cannot_describe_never_reaches_the_bus() {
        // Nothing is scripted, so any transfer at all would fail the script. The refusal
        // comes first, which matters more here than it looks: a chain left half configured
        // would put the next packet on the air with whatever it was given for this one.
        let mut chip = driven(Vec::new());
        let request = tx::Transmit {
            frequency_hz: 868_100_000,
            link: pamoja_lora::LinkSettings::new(7, 62_500),
            gain: tx::DEFAULT_GAINS[0],
            invert_polarity: true,
            public: true,
            payload: &[0x01],
        };

        let refused = chip
            .transmit(Chain::A, &request, Trigger::Immediate, 0x0102)
            .expect_err("a chain does not transmit at 62500 Hz");
        assert!(matches!(
            refused,
            ConcentratorError::Transmit(tx::TransmitError::Bandwidth { hertz: 62_500 })
        ));
    }

    #[test]
    fn a_send_loads_the_payload_before_it_arms_the_trigger() {
        // The delay is programmed, the buffer is opened and written and closed, and only
        // then is the trigger cleared and set. Arming first would send whatever was there.
        let mut steps = vec![
            writes(0x5205, 0x01),
            writes(0x5206, 0x02),
            reads(0x5207, 0),
            writes(0x5207, 1),
        ];
        steps.extend(sends(0x5300, vec![0xde, 0xad]));
        steps.extend(vec![
            reads(0x5207, 1),
            writes(0x5207, 0),
            reads(0x5200, 0),
            writes(0x5200, 0),
            reads(0x5200, 0),
            writes(0x5200, 1),
        ]);
        let mut chip = driven(steps);

        chip.send(Chain::A, &[0xde, 0xad], Trigger::Immediate, 0x0102)
            .expect("the bus answers");
    }

    #[test]
    fn a_timed_send_counts_down_the_addresses() {
        // A send one second out at this delay is 1000000 * 32 - 258 ticks, which is
        // 0x01e846fe. The least significant byte goes to the highest of the four addresses,
        // which is the one thing about this register that is easy to get backward.
        let mut steps = vec![
            writes(0x5205, 0x01),
            writes(0x5206, 0x02),
            reads(0x5207, 0),
            writes(0x5207, 1),
        ];
        steps.extend(sends(0x5300, vec![0x01]));
        steps.extend(vec![
            reads(0x5207, 1),
            writes(0x5207, 0),
            writes(0x5204, 0xfe),
            writes(0x5203, 0x46),
            writes(0x5202, 0xe8),
            writes(0x5201, 0x01),
            reads(0x5200, 0),
            writes(0x5200, 0),
            reads(0x5200, 0),
            writes(0x5200, 0b10),
        ]);
        let mut chip = driven(steps);

        chip.send(Chain::A, &[0x01], Trigger::At(1_000_000), 0x0102)
            .expect("the bus answers");
    }

    #[test]
    fn a_chain_that_will_take_a_packet_reads_free() {
        let mut chip = driven(vec![reads(0x5211, tx::STATUS_FREE)]);

        let status = chip.tx_status(Chain::A).expect("the bus answers");
        assert!(status.is_free(), "{status:?}");
    }

    #[test]
    fn a_concentrator_that_answers_reads_the_documented_version() {
        let mut chip = driven(vec![reads(0x5606, chip::EXPECTED_VERSION)]);
        assert_eq!(chip.version().expect("the bus answers"), 0x10);
    }

    #[test]
    fn a_board_that_is_not_there_says_what_it_read() {
        // An unpowered board reads zero and a bus nothing drives reads all ones, and the
        // caller is told which, because they are different faults to go and look for.
        for absent in [0x00, 0xff] {
            let mut chip = driven(vec![reads(0x5606, absent)]);
            match chip.check() {
                Err(ConcentratorError::NotAnswering { version }) => assert_eq!(version, absent),
                other => panic!("a silent board is reported, not accepted: {other:?}"),
            }
        }

        let mut chip = driven(vec![reads(0x5606, chip::EXPECTED_VERSION)]);
        assert!(chip.check().is_ok());
    }

    #[test]
    fn identifying_selects_the_byte_then_reads_it() {
        // The part number is not a register: it is selected in one transfer and read in the
        // next, so both go out in order.
        let mut chip = driven(vec![
            writes(0x6180, chip::MODEL_BYTE_ADDRESS),
            reads(0x6181, 0x03),
        ]);
        assert_eq!(chip.identify().expect("the bus answers"), Model::Sx1303);

        let mut chip = driven(vec![
            writes(0x6180, chip::MODEL_BYTE_ADDRESS),
            reads(0x6181, 0x02),
        ]);
        assert_eq!(chip.identify().expect("the bus answers"), Model::Sx1302);
    }

    #[test]
    fn a_narrow_register_is_read_before_it_is_written() {
        // The clock control shares its byte with the radio control beside it, so writing it
        // costs a read first and must leave the neighbor alone.
        let held = 0b0000_1000; // the host radio control bit, which must survive
        let mut chip = driven(vec![
            reads(0x5601, held),
            writes(0x5601, held | 0b0001_0000),
        ]);

        chip.write_register(register::COMMON_CLK32_RIF_CTRL, 1)
            .expect("the bus answers");
    }

    #[test]
    fn a_whole_byte_is_written_without_reading_it_first() {
        // The one-time programmable address is the whole byte, so there is nothing to preserve.
        let mut chip = driven(vec![writes(0x6180, 0xd0)]);
        chip.write_register(register::OTP_BYTE_ADDR, 0xd0)
            .expect("the bus answers");
    }

    #[test]
    fn resetting_a_front_end_clocks_the_chip_from_the_host_first() {
        // Clearing the clock control keeps the concentrator listening while the radio it
        // drives is held down, which is why it comes before anything else.
        let mut chip = driven(vec![
            reads(0x5601, 0),
            writes(0x5601, 0),
            reads(0x5783, 0),
            writes(0x5783, 0b0000_0100),
            reads(0x5783, 0b0000_0100),
            writes(0x5783, 0b0000_1100),
            reads(0x5783, 0b0000_1100),
            writes(0x5783, 0b0000_0100),
        ]);

        chip.reset_front_end(Chain::A, FrontEnd::Sx125x)
            .expect("the bus answers");
    }

    #[test]
    fn an_sx1250_is_held_again_while_it_calibrates() {
        // The newer front end calibrates itself on the way out, so it takes one more write
        // than the older one does.
        let mut chip = driven(vec![
            reads(0x5601, 0),
            writes(0x5601, 0),
            reads(0x5784, 0),
            writes(0x5784, 0b0000_0100),
            reads(0x5784, 0b0000_0100),
            writes(0x5784, 0b0000_1100),
            reads(0x5784, 0b0000_1100),
            writes(0x5784, 0b0000_0100),
            reads(0x5784, 0b0000_0100),
            writes(0x5784, 0b0000_1100),
        ]);

        chip.reset_front_end(Chain::B, FrontEnd::Sx1250)
            .expect("the bus answers");
    }

    #[test]
    fn an_image_of_the_wrong_size_never_reaches_the_bus() {
        // Nothing is scripted, so any transfer at all would fail the test.
        let mut chip = driven(vec![]);
        match chip.load_firmware(Mcu::Agc, &[0u8; 16]) {
            Err(ConcentratorError::Firmware(LoadError::WrongSize { offered })) => {
                assert_eq!(offered, 16);
            }
            other => panic!("a short image is refused before anything is written: {other:?}"),
        }
    }

    #[test]
    fn the_reset_line_is_pulsed_and_then_released() {
        let mut chip = driven(vec![]);
        chip.reset().expect("the line is drivable");

        let (_, pins, delay) = chip.release();
        assert_eq!(
            pins.driven(),
            [digital::PinState::High, digital::PinState::Low]
        );
        assert_eq!(
            delay.waits_ns().len(),
            2,
            "held, then given time to come up"
        );
    }
}
