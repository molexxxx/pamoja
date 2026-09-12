//! Bringing up the SX1261 that listens beside the concentrator.
//!
//! [`lbt`](super::lbt) builds the frames that ask this radio whether a channel is busy, but
//! two of the opcodes it sends are not in the SX1261's own command set. They exist only once
//! the radio's patch memory holds Semtech's patch, so a gateway that sends them to a radio
//! straight out of reset is talking to a chip that has never heard of them, and gets silence
//! rather than an answer. This module is the half that comes first: park the radio, load the
//! patch and prove it took, calibrate the image for the band, and set up the receiver the
//! check listens with.
//!
//! The radio takes commands rather than register writes. Every one of them is a single opcode
//! byte followed by its payload, which is what a [`Frame`] carries, and the few registers that
//! are written go inside the payload of a write command rather than on a bus of their own.
//!
//! Three things here are not what they look like:
//!
//! - The patch is 386 words rather than bytes, written four bytes at a time from
//!   [`PRAM_BASE`], and it does not take effect until [`commit`] is sent.
//! - Whether it took is read as text. [`PRAM_VERSION`] is compared against the last four
//!   characters of the version string, not the whole of it.
//! - The last level a spectral scan reports repeats the level before it, because it counts
//!   what was heard below the lowest threshold rather than at a new one.
//!
//! Nothing in this module opens a bus.

use super::sx1250;

/// Writes one of the radio's registers, with the address and value inside the payload.
pub const OP_WRITE_REGISTER: u8 = 0x0d;

/// Reads the radio's registers from an address given in the payload.
pub const OP_READ_REGISTER: u8 = 0x1d;

/// Reports what the radio failed at, which is how a refused calibration is noticed.
pub const OP_GET_DEVICE_ERRORS: u8 = 0x17;

/// Parks the radio, on either of the two clocks.
pub const OP_SET_STANDBY: u8 = 0x80;

/// Puts the radio into continuous receive.
pub const OP_SET_RX: u8 = 0x82;

/// Tunes the radio.
pub const OP_SET_RF_FREQUENCY: u8 = 0x86;

/// Chooses the modulation the receiver expects.
pub const OP_SET_PACKET_TYPE: u8 = 0x8a;

/// Sets the bitrate, the shaping, the bandwidth, and the deviation.
pub const OP_SET_MODULATION_PARAMS: u8 = 0x8b;

/// Sets the preamble, the sync word length, and how a packet's length is carried.
pub const OP_SET_PACKET_PARAMS: u8 = 0x8c;

/// Points the transmit and receive buffers at somewhere in the radio's memory.
pub const OP_SET_BUFFER_BASE_ADDRESS: u8 = 0x8f;

/// Calibrates the image rejection for one band.
pub const OP_CALIBRATE_IMAGE: u8 = 0x98;

/// Asks the radio what mode it is in and how the last command went.
pub const OP_GET_STATUS: u8 = 0xc0;

/// Puts the radio on frequency without receiving, which is where it waits between checks.
pub const OP_SET_FS: u8 = 0xc1;

/// Takes the patch just written into use.
///
/// This opcode is not in the radio's published command set. It appears only in the reference
/// implementation, where the patch does nothing until it is sent.
pub const OP_UPDATE_PRAM: u8 = 0xd9;

/// The register holding how long the radio averages a signal level over.
///
/// It is written twice for opposite reasons: with [`RSSI_WINDOW_ARMED`] to set the averaging
/// up before a check, and with zero to release the radio afterwards, which is what
/// [`lbt::stop`](super::lbt::stop) and an abandoned spectral scan both do.
pub const REG_RSSI_WINDOW: u16 = 0x089b;

/// The averaging window a carrier check listens with.
pub const RSSI_WINDOW_ARMED: u8 = 0x05 << 2;

/// The register that trades a little current for a lower noise floor.
pub const REG_SENSITIVITY: u16 = 0x08ac;

/// What [`REG_SENSITIVITY`] is set to, which the reference applies to every board.
pub const SENSITIVITY_VALUE: u8 = 0xcb;

/// The register that has to be opened before the patch is written, and closed after.
pub const REG_PATCH_UPDATE: u16 = 0x0610;

/// Opens [`REG_PATCH_UPDATE`] so the patch can be written.
pub const PATCH_UPDATE_OPEN: u8 = 0x10;

/// Closes [`REG_PATCH_UPDATE`] once the patch is written.
pub const PATCH_UPDATE_CLOSED: u8 = 0x00;

/// Where the version string the radio is running sits.
pub const REG_PRAM_VERSION: u16 = 0x0320;

/// Where a spectral scan reports whether it is finished.
pub const REG_SCAN_STATUS: u16 = 0x07cd;

/// Where a spectral scan leaves its counts.
pub const REG_SCAN_RESULTS: u16 = 0x0401;

/// Where the patch is written, one word every four addresses.
pub const PRAM_BASE: u16 = 0x8000;

/// How many words the patch holds.
pub const PRAM_WORDS: usize = 386;

/// The last four characters of the version string a loaded patch answers with.
pub const PRAM_VERSION: &str = "2D06";

/// How many characters the version string holds.
pub const VERSION_LEN: usize = 15;

/// How many bytes a version read moves: the address, one the radio answers over, then the
/// string itself.
pub const VERSION_TRANSFER: usize = 3 + VERSION_LEN;

/// The whole byte the radio answers with once it is parked and ready.
///
/// The reference checks for this value rather than for its fields, and so does
/// [`Status::parked`], because it is what a working radio replies. Read as fields it is a
/// mode of [`Mode::StandbyRc`] and a command status of 1, which the datasheet reserves. The
/// reference composes it from a mode constant that is shifted into its field and a ready
/// constant that is not, so the value is right and the arithmetic behind it is not.
pub const STANDBY_READY: u8 = 0x22;

/// The bits of a status byte that carry anything. The top and bottom bits are not answered.
pub const STATUS_MASK: u8 = 0x7e;

/// How long an image calibration takes before the radio can be asked whether it worked.
pub const CALIBRATION_WAIT_MS: u32 = 10;

/// How many levels a spectral scan reports.
pub const SCAN_LEVELS: usize = 33;

/// How many bytes a spectral scan's counts take: the address, one the radio answers over,
/// then two bytes for each level.
pub const SCAN_TRANSFER: usize = 3 + SCAN_LEVELS * 2;

/// The longest payload any command here carries, which is the packet settings.
pub const MAX_PAYLOAD: usize = 9;

/// One command: the opcode, and the payload that follows it.
///
/// A frame is the whole of what goes out for one command, so a caller writes the opcode and
/// then the payload and does not have to know which commands take one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    opcode: u8,
    payload: [u8; MAX_PAYLOAD],
    len: u8,
}

impl Frame {
    const fn build(opcode: u8, payload: [u8; MAX_PAYLOAD], len: u8) -> Frame {
        Frame {
            opcode,
            payload,
            len,
        }
    }

    /// The opcode byte this command starts with.
    ///
    /// # Returns
    ///
    /// The byte that goes out first.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::sx1261::{standby, Standby, OP_SET_STANDBY};
    ///
    /// assert_eq!(standby(Standby::Rc).opcode(), OP_SET_STANDBY);
    /// ```
    #[must_use]
    pub const fn opcode(&self) -> u8 {
        self.opcode
    }

    /// What follows the opcode.
    ///
    /// # Returns
    ///
    /// The payload, which is empty for the commands that take none.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::sx1261::{standby, Standby, free_running};
    ///
    /// assert_eq!(standby(Standby::Rc).payload(), &[0x00]);
    /// assert!(free_running().payload().is_empty(), "this one takes no payload");
    /// ```
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload[..self.len as usize]
    }
}

/// Which clock the radio is parked on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standby {
    /// The radio's own oscillator, which is where it wakes up and where the patch is loaded.
    Rc,
    /// The crystal, which costs more current and holds frequency.
    Xosc,
}

impl Standby {
    /// The byte that selects this clock.
    ///
    /// # Returns
    ///
    /// The value the command takes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::sx1261::Standby;
    ///
    /// assert_eq!(Standby::Rc.byte(), 0x00);
    /// assert_eq!(Standby::Xosc.byte(), 0x01);
    /// ```
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Standby::Rc => 0x00,
            Standby::Xosc => 0x01,
        }
    }
}

/// What the radio is doing, from the middle bits of a status byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Parked on its own oscillator.
    StandbyRc,
    /// Parked on the crystal.
    StandbyXosc,
    /// On frequency, neither receiving nor transmitting.
    FrequencySynthesis,
    /// Receiving.
    Receiving,
    /// Transmitting.
    Transmitting,
}

impl Mode {
    /// The bits this mode occupies in a status byte.
    ///
    /// # Returns
    ///
    /// The value as it sits in the byte, already in its field.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::sx1261::Mode;
    ///
    /// assert_eq!(Mode::StandbyRc.bits(), 0x20);
    /// assert_eq!(Mode::Receiving.bits(), 0x50);
    /// ```
    #[must_use]
    pub const fn bits(self) -> u8 {
        match self {
            Mode::StandbyRc => 0x20,
            Mode::StandbyXosc => 0x30,
            Mode::FrequencySynthesis => 0x40,
            Mode::Receiving => 0x50,
            Mode::Transmitting => 0x60,
        }
    }

    /// Reads the mode out of a status byte.
    ///
    /// # Arguments
    ///
    /// * `status` - the byte the radio answered with.
    ///
    /// # Returns
    ///
    /// The mode, or `None` for a value the datasheet does not give one for.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::sx1261::{Mode, STANDBY_READY};
    ///
    /// assert_eq!(Mode::of(STANDBY_READY), Some(Mode::StandbyRc));
    /// assert_eq!(Mode::of(0x00), None);
    /// ```
    #[must_use]
    pub const fn of(status: u8) -> Option<Mode> {
        match status & 0x70 {
            0x20 => Some(Mode::StandbyRc),
            0x30 => Some(Mode::StandbyXosc),
            0x40 => Some(Mode::FrequencySynthesis),
            0x50 => Some(Mode::Receiving),
            0x60 => Some(Mode::Transmitting),
            _ => None,
        }
    }
}

/// How the last command went, from the low bits of a status byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandStatus {
    /// There is something for the host to read.
    DataAvailable,
    /// The command ran out of time.
    Timeout,
    /// The radio could not process the command.
    ProcessingError,
    /// The radio understood the command and could not carry it out.
    ExecutionFailed,
    /// A transmission finished.
    TransmitDone,
}

impl CommandStatus {
    /// Reads the command status out of a status byte.
    ///
    /// # Arguments
    ///
    /// * `status` - the byte the radio answered with.
    ///
    /// # Returns
    ///
    /// The status, or `None` for a value the datasheet reserves.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::sx1261::CommandStatus;
    ///
    /// assert_eq!(CommandStatus::of(0x24), Some(CommandStatus::DataAvailable));
    /// assert_eq!(CommandStatus::of(0x2c), Some(CommandStatus::TransmitDone));
    /// ```
    #[must_use]
    pub const fn of(status: u8) -> Option<CommandStatus> {
        match (status >> 1) & 0x07 {
            0x02 => Some(CommandStatus::DataAvailable),
            0x03 => Some(CommandStatus::Timeout),
            0x04 => Some(CommandStatus::ProcessingError),
            0x05 => Some(CommandStatus::ExecutionFailed),
            0x06 => Some(CommandStatus::TransmitDone),
            _ => None,
        }
    }
}

/// What the radio answered when asked how it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    /// The byte as it arrived, with the bits the radio does not answer cleared.
    pub byte: u8,
    /// What the radio is doing.
    pub mode: Option<Mode>,
    /// How the last command went.
    pub command: Option<CommandStatus>,
}

impl Status {
    /// Reads a status byte.
    ///
    /// # Arguments
    ///
    /// * `byte` - what the radio answered.
    ///
    /// # Returns
    ///
    /// The byte and the fields inside it.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::sx1261::{Mode, Status, STANDBY_READY};
    ///
    /// let status = Status::read(STANDBY_READY);
    /// assert_eq!(status.mode, Some(Mode::StandbyRc));
    /// assert!(status.parked(), "this is the answer a parked radio gives");
    /// ```
    #[must_use]
    pub const fn read(byte: u8) -> Status {
        let byte = byte & STATUS_MASK;
        Status {
            byte,
            mode: Mode::of(byte),
            command: CommandStatus::of(byte),
        }
    }

    /// Whether the radio is parked and ready for the patch.
    ///
    /// # Returns
    ///
    /// Whether the answer is [`STANDBY_READY`], which is the whole byte a parked radio gives
    /// rather than a test of either field.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::sx1261::{Status, STANDBY_READY};
    ///
    /// assert!(Status::read(STANDBY_READY).parked());
    /// assert!(!Status::read(0x50).parked(), "receiving is not parked");
    /// ```
    #[must_use]
    pub const fn parked(&self) -> bool {
        self.byte == STANDBY_READY
    }
}

/// Turns a frequency into the number the radio is tuned with.
///
/// # Arguments
///
/// * `hertz` - the carrier.
///
/// # Returns
///
/// The value the tune command takes.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::frequency_value;
///
/// assert_eq!(frequency_value(868_100_000), 910_268_825);
/// ```
#[must_use]
pub const fn frequency_value(hertz: u32) -> u32 {
    ((hertz as u64 * (1 << 25)) / 32_000_000) as u32
}

/// Parks the radio.
///
/// # Arguments
///
/// * `clock` - which clock to park on.
///
/// # Returns
///
/// The command.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{standby, Standby, OP_SET_STANDBY};
///
/// let frame = standby(Standby::Rc);
/// assert_eq!(frame.opcode(), OP_SET_STANDBY);
/// assert_eq!(frame.payload(), &[0x00]);
/// ```
#[must_use]
pub const fn standby(clock: Standby) -> Frame {
    Frame::build(OP_SET_STANDBY, [clock.byte(), 0, 0, 0, 0, 0, 0, 0, 0], 1)
}

/// Puts the radio on frequency without receiving, which is where it waits between checks.
///
/// # Returns
///
/// The command, which takes no payload.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{free_running, OP_SET_FS};
///
/// assert_eq!(free_running().opcode(), OP_SET_FS);
/// assert!(free_running().payload().is_empty());
/// ```
#[must_use]
pub const fn free_running() -> Frame {
    Frame::build(OP_SET_FS, [0; MAX_PAYLOAD], 0)
}

/// Asks the radio how it is.
///
/// # Returns
///
/// The command. The radio answers over the byte the payload leaves for it, which
/// [`Status::read`] then reads.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{status, OP_GET_STATUS};
///
/// assert_eq!(status().opcode(), OP_GET_STATUS);
/// assert_eq!(status().payload(), &[0x00], "the radio answers over this byte");
/// ```
#[must_use]
pub const fn status() -> Frame {
    Frame::build(OP_GET_STATUS, [0; MAX_PAYLOAD], 1)
}

/// Writes one of the radio's registers.
///
/// # Arguments
///
/// * `register` - the address.
/// * `value` - what to write.
///
/// # Returns
///
/// The command.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{write_register, REG_SENSITIVITY, SENSITIVITY_VALUE};
///
/// let frame = write_register(REG_SENSITIVITY, SENSITIVITY_VALUE);
/// assert_eq!(frame.payload(), &[0x08, 0xac, 0xcb]);
/// ```
#[must_use]
pub const fn write_register(register: u16, value: u8) -> Frame {
    Frame::build(
        OP_WRITE_REGISTER,
        [
            (register >> 8) as u8,
            (register & 0xff) as u8,
            value,
            0,
            0,
            0,
            0,
            0,
            0,
        ],
        3,
    )
}

/// The commands that bring a parked radio up for carrier checks.
///
/// # Returns
///
/// Parking the radio, pointing its buffers somewhere, and trading a little current for a
/// lower noise floor. The radio is asked for its [`status`] between the first and the second,
/// and [`Status::parked`] has to hold before the rest are worth sending.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{setup, OP_SET_BUFFER_BASE_ADDRESS, OP_SET_STANDBY};
///
/// let frames = setup();
/// assert_eq!(frames[0].opcode(), OP_SET_STANDBY);
/// assert_eq!(frames[1].opcode(), OP_SET_BUFFER_BASE_ADDRESS);
/// assert_eq!(frames[1].payload(), &[0x80, 0x80]);
/// assert_eq!(frames[2].payload(), &[0x08, 0xac, 0xcb], "the sensitivity register");
/// ```
#[must_use]
pub const fn setup() -> [Frame; 3] {
    [
        standby(Standby::Rc),
        Frame::build(
            OP_SET_BUFFER_BASE_ADDRESS,
            [0x80, 0x80, 0, 0, 0, 0, 0, 0, 0],
            2,
        ),
        write_register(REG_SENSITIVITY, SENSITIVITY_VALUE),
    ]
}

/// Calibrates the image rejection for the band a carrier falls in.
///
/// # Arguments
///
/// * `hertz` - the carrier the radio will listen on.
///
/// # Returns
///
/// The command, or `None` for a frequency outside every band the radio is calibrated for.
/// The bands are the same ones the front ends use, so they are read from
/// [`sx1250::image_band`] rather than written out twice.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{calibrate, OP_CALIBRATE_IMAGE};
///
/// let frame = calibrate(868_100_000).expect("868 MHz is a band the radio knows");
/// assert_eq!(frame.opcode(), OP_CALIBRATE_IMAGE);
/// assert_eq!(frame.payload(), &[0xd7, 0xdb]);
/// assert!(calibrate(600_000_000).is_none(), "no band covers this");
/// ```
#[must_use]
pub const fn calibrate(hertz: u32) -> Option<Frame> {
    match sx1250::image_band(hertz) {
        Some([low, high]) => Some(Frame::build(
            OP_CALIBRATE_IMAGE,
            [low, high, 0, 0, 0, 0, 0, 0, 0],
            2,
        )),
        None => None,
    }
}

/// Asks the radio what it has failed at.
///
/// # Returns
///
/// The command. The radio answers over the bytes the payload leaves for it, which
/// [`calibration_failed`] then reads.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{device_errors, OP_GET_DEVICE_ERRORS};
///
/// assert_eq!(device_errors().opcode(), OP_GET_DEVICE_ERRORS);
/// assert_eq!(device_errors().payload().len(), 3);
/// ```
#[must_use]
pub const fn device_errors() -> Frame {
    Frame::build(OP_GET_DEVICE_ERRORS, [0; MAX_PAYLOAD], 3)
}

/// Whether the radio reports that an image calibration did not work.
///
/// # Arguments
///
/// * `answer` - the three bytes [`device_errors`] came back with.
///
/// # Returns
///
/// Whether the image calibration bit is set.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::calibration_failed;
///
/// assert!(!calibration_failed([0x00, 0x00, 0x00]));
/// assert!(calibration_failed([0x00, 0x00, 0x10]));
/// assert!(!calibration_failed([0x00, 0x00, 0x08]), "a different fault, not this one");
/// ```
#[must_use]
pub const fn calibration_failed(answer: [u8; 3]) -> bool {
    (answer[2] >> 4) & 0x01 != 0
}

/// Opens the radio so the patch can be written.
///
/// # Returns
///
/// The command that goes out before the first word.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::open_patch;
///
/// assert_eq!(open_patch().payload(), &[0x06, 0x10, 0x10]);
/// ```
#[must_use]
pub const fn open_patch() -> Frame {
    write_register(REG_PATCH_UPDATE, PATCH_UPDATE_OPEN)
}

/// Closes the radio once the patch is written.
///
/// # Returns
///
/// The command that goes out after the last word, before [`commit`].
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::close_patch;
///
/// assert_eq!(close_patch().payload(), &[0x06, 0x10, 0x00]);
/// ```
#[must_use]
pub const fn close_patch() -> Frame {
    write_register(REG_PATCH_UPDATE, PATCH_UPDATE_CLOSED)
}

/// Writes one word of the patch.
///
/// # Arguments
///
/// * `index` - which word, counting from zero.
/// * `word` - the word itself.
///
/// # Returns
///
/// The command. The address is four apart from its neighbors, because each word takes four
/// of them.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::patch_word;
///
/// assert_eq!(patch_word(0, 0x00337fe1).payload(), &[0x80, 0x00, 0x00, 0x33, 0x7f, 0xe1]);
/// assert_eq!(patch_word(1, 0).payload()[..2], [0x80, 0x04], "four addresses on");
/// ```
#[must_use]
pub const fn patch_word(index: usize, word: u32) -> Frame {
    let address = PRAM_BASE as usize + 4 * index;
    Frame::build(
        OP_WRITE_REGISTER,
        [
            (address >> 8) as u8,
            (address & 0xff) as u8,
            (word >> 24) as u8,
            (word >> 16) as u8,
            (word >> 8) as u8,
            word as u8,
            0,
            0,
            0,
        ],
        6,
    )
}

/// Takes the patch into use.
///
/// # Returns
///
/// The command, which takes no payload. Until it is sent the patch is written but not
/// running, and the version string still reads as whatever it did before.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{commit, OP_UPDATE_PRAM};
///
/// assert_eq!(commit().opcode(), OP_UPDATE_PRAM);
/// assert!(commit().payload().is_empty());
/// ```
#[must_use]
pub const fn commit() -> Frame {
    Frame::build(OP_UPDATE_PRAM, [0; MAX_PAYLOAD], 0)
}

/// Asks the radio which patch it is running.
///
/// # Returns
///
/// The address to read from, and how many bytes the read moves. The string arrives after the
/// address and the byte the radio answers over, which is what [`version`] accounts for.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{version_read, VERSION_TRANSFER};
///
/// let (address, length) = version_read();
/// assert_eq!(address, 0x0320);
/// assert_eq!(length, VERSION_TRANSFER);
/// ```
#[must_use]
pub const fn version_read() -> (u16, usize) {
    (REG_PRAM_VERSION, VERSION_TRANSFER)
}

/// Reads the version string out of what the radio answered.
///
/// # Arguments
///
/// * `answer` - the bytes the read moved, starting with the address that went out.
///
/// # Returns
///
/// The string, or `None` when the answer is too short or is not text.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::version;
///
/// let mut answer = [0u8; 18];
/// answer[3..].copy_from_slice(b"sx1261_v1.12D06");
/// assert_eq!(version(&answer), Some("sx1261_v1.12D06"));
/// ```
#[must_use]
pub fn version(answer: &[u8]) -> Option<&str> {
    let text = answer.get(3..VERSION_TRANSFER)?;
    core::str::from_utf8(text).ok()
}

/// Whether the radio is running the patch this crate writes.
///
/// # Arguments
///
/// * `answer` - the bytes a version read moved.
///
/// # Returns
///
/// Whether the string ends in [`PRAM_VERSION`]. Only the last four characters are compared,
/// because the rest of the string names a build rather than the patch.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::patched;
///
/// let mut answer = [0u8; 18];
/// answer[3..].copy_from_slice(b"sx1261_v1.12D06");
/// assert!(patched(&answer));
///
/// answer[3..].copy_from_slice(b"sx1261_v1.10000");
/// assert!(!patched(&answer), "a radio still running what it came with");
/// ```
#[must_use]
pub fn patched(answer: &[u8]) -> bool {
    let Some(text) = version(answer) else {
        return false;
    };
    let Some(tail) = text.get(text.len().saturating_sub(PRAM_VERSION.len())..) else {
        return false;
    };
    tail == PRAM_VERSION
}

/// What went wrong reading a patch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchError {
    /// The text does not hold as many words as the patch has.
    WrongCount {
        /// How many words were found.
        found: usize,
    },
}

impl core::fmt::Display for PatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PatchError::WrongCount { found } => write!(
                f,
                "the patch holds {found} words, and the radio takes {PRAM_WORDS}"
            ),
        }
    }
}

/// Reads a patch from the form Semtech ships it in.
///
/// The patch arrives as a C array of 32-bit words, written without leading zeros, so a word
/// is between one and eight characters long. That is read here rather than converted first,
/// so the file a board's vendor ships is the file that is loaded.
///
/// # Arguments
///
/// * `text` - the file's contents.
/// * `into` - where the words are put.
///
/// # Returns
///
/// Nothing, with `into` filled.
///
/// # Errors
///
/// [`PatchError::WrongCount`] when the text does not hold exactly [`PRAM_WORDS`] words. A
/// short count means the file is truncated or is not a patch, and a long one means it is
/// something else, so neither is loaded.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{read_patch, PRAM_WORDS};
///
/// let mut text = String::from("const uint32_t pram[] = {\n");
/// for _ in 0..PRAM_WORDS {
///     text.push_str("0x337fe1,\n");
/// }
/// text.push_str("};\n");
///
/// let mut patch = [0u32; PRAM_WORDS];
/// read_patch(&text, &mut patch).expect("that is a patch of the right length");
/// assert_eq!(patch[0], 0x337fe1);
/// ```
pub fn read_patch(text: &str, into: &mut [u32; PRAM_WORDS]) -> Result<(), PatchError> {
    let raw = text.as_bytes();
    let mut found = 0usize;
    let mut at = 0usize;

    while at + 2 < raw.len() {
        if raw[at] != b'0' || (raw[at + 1] | 0x20) != b'x' {
            at += 1;
            continue;
        }

        let mut value = 0u32;
        let mut digits = 0usize;
        while let Some(digit) = raw.get(at + 2 + digits).copied().and_then(hex_digit) {
            value = (value << 4) | digit as u32;
            digits += 1;
            if digits == 8 {
                break;
            }
        }

        if digits == 0 {
            at += 1;
            continue;
        }

        if let Some(slot) = into.get_mut(found) {
            *slot = value;
        }
        found += 1;
        at += 2 + digits;
    }

    if found == PRAM_WORDS {
        Ok(())
    } else {
        Err(PatchError::WrongCount { found })
    }
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// How wide the receiver listens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bandwidth {
    /// Matching a 125 kHz channel.
    Khz125,
    /// Matching a 250 kHz channel.
    Khz250,
}

impl Bandwidth {
    /// The byte that selects this width.
    ///
    /// # Returns
    ///
    /// The value the modulation command takes. The receiver is wider than the channel it
    /// covers, because it is measuring a level rather than decoding anything.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::sx1261::Bandwidth;
    ///
    /// assert_eq!(Bandwidth::Khz125.byte(), 0x0a);
    /// assert_eq!(Bandwidth::Khz250.byte(), 0x09);
    /// ```
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Bandwidth::Khz125 => 0x0a,
            Bandwidth::Khz250 => 0x09,
        }
    }
}

/// The commands that point the receiver at a channel and start it listening.
///
/// # Arguments
///
/// * `hertz` - the carrier to listen on.
/// * `bandwidth` - how wide to listen.
///
/// # Returns
///
/// Releasing whatever the radio was doing, putting it on frequency, tuning it, setting the
/// averaging window, and then the modulation, the packet shape, and continuous receive. The
/// radio is left listening, which is what a carrier check then reads a level from.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{receive, Bandwidth, OP_SET_RX};
///
/// let frames = receive(868_100_000, Bandwidth::Khz125);
/// assert_eq!(frames[0].payload(), &[0x08, 0x9b, 0x00], "release it first");
/// assert_eq!(frames[2].payload(), &[0x36, 0x41, 0x99, 0x99], "tuned to 868.1 MHz");
/// assert_eq!(frames[7].opcode(), OP_SET_RX);
/// assert_eq!(frames[7].payload(), &[0xff, 0xff, 0xff], "and it listens until told not to");
/// ```
#[must_use]
pub const fn receive(hertz: u32, bandwidth: Bandwidth) -> [Frame; 8] {
    let tuned = frequency_value(hertz);
    [
        write_register(REG_RSSI_WINDOW, 0x00),
        free_running(),
        Frame::build(
            OP_SET_RF_FREQUENCY,
            [
                (tuned >> 24) as u8,
                (tuned >> 16) as u8,
                (tuned >> 8) as u8,
                tuned as u8,
                0,
                0,
                0,
                0,
                0,
            ],
            4,
        ),
        write_register(REG_RSSI_WINDOW, RSSI_WINDOW_ARMED),
        Frame::build(OP_SET_PACKET_TYPE, [0x00; MAX_PAYLOAD], 1),
        Frame::build(
            OP_SET_MODULATION_PARAMS,
            [
                0x00,
                0x14,
                0x00,
                0x00,
                bandwidth.byte(),
                0x02,
                0xe9,
                0x0f,
                0x00,
            ],
            8,
        ),
        Frame::build(
            OP_SET_PACKET_PARAMS,
            [0x00, 0x20, 0x05, 0x20, 0x00, 0x01, 0xff, 0x00, 0x00],
            9,
        ),
        Frame::build(OP_SET_RX, [0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0], 3),
    ]
}

/// How far along a spectral scan is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanStatus {
    /// No scan has run.
    None,
    /// A scan is running.
    Running,
    /// A scan was stopped before it finished.
    Aborted,
    /// A scan finished, so its counts are worth reading.
    Completed,
}

impl ScanStatus {
    /// Reads what the radio answered.
    ///
    /// # Arguments
    ///
    /// * `byte` - the value read from [`REG_SCAN_STATUS`].
    ///
    /// # Returns
    ///
    /// The state, or `None` for a value that is none of them.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::sx1261::ScanStatus;
    ///
    /// assert_eq!(ScanStatus::of(0x00), Some(ScanStatus::None));
    /// assert_eq!(ScanStatus::of(0x0f), Some(ScanStatus::Running));
    /// assert_eq!(ScanStatus::of(0xff), Some(ScanStatus::Completed));
    /// assert_eq!(ScanStatus::of(0x01), None);
    /// ```
    #[must_use]
    pub const fn of(byte: u8) -> Option<ScanStatus> {
        match byte {
            0x00 => Some(ScanStatus::None),
            0x0f => Some(ScanStatus::Running),
            0xf0 => Some(ScanStatus::Aborted),
            0xff => Some(ScanStatus::Completed),
            _ => None,
        }
    }
}

/// One level a spectral scan counted at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level {
    /// The level itself, in dBm.
    pub dbm: i16,
    /// How many of the scan's samples were at least this strong.
    pub count: u16,
}

/// Reads what a spectral scan counted.
///
/// # Arguments
///
/// * `answer` - the bytes the read moved, starting with the address that went out.
/// * `rssi_offset` - the board's correction, in dB, from its own calibration.
///
/// # Returns
///
/// The levels, strongest first, four decibels apart, or `None` when the answer is too short.
///
/// The last level repeats the one before it rather than continuing down, because it counts
/// everything heard below the lowest threshold rather than at a level of its own. Subtracting
/// one count from the other is what a caller does with it.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1261::{scan_counts, SCAN_TRANSFER};
///
/// let mut answer = [0u8; SCAN_TRANSFER];
/// answer[3] = 0x01;
/// answer[4] = 0x2c;
///
/// let levels = scan_counts(&answer, -3).expect("that is a whole answer");
/// assert_eq!(levels[0].dbm, -3, "the strongest level, corrected for the board");
/// assert_eq!(levels[0].count, 300);
/// assert_eq!(levels[1].dbm, -7, "four decibels down");
/// assert_eq!(levels[32].dbm, levels[31].dbm, "everything below the last one");
/// ```
#[must_use]
pub fn scan_counts(answer: &[u8], rssi_offset: i8) -> Option<[Level; SCAN_LEVELS]> {
    let counts = answer.get(3..SCAN_TRANSFER)?;
    let mut levels = [Level { dbm: 0, count: 0 }; SCAN_LEVELS];
    for (at, level) in levels.iter_mut().enumerate() {
        let step = if at == SCAN_LEVELS - 1 { at - 1 } else { at };
        *level = Level {
            dbm: -(step as i16) * 4 + rssi_offset as i16,
            count: u16::from_be_bytes([counts[at * 2], counts[at * 2 + 1]]),
        };
    }
    Some(levels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_setup_matches_the_reference() {
        let frames = setup();
        assert_eq!(frames[0].opcode(), OP_SET_STANDBY);
        assert_eq!(frames[0].payload(), &[0x00]);
        assert_eq!(frames[1].opcode(), OP_SET_BUFFER_BASE_ADDRESS);
        assert_eq!(frames[1].payload(), &[0x80, 0x80]);
        assert_eq!(frames[2].opcode(), OP_WRITE_REGISTER);
        assert_eq!(frames[2].payload(), &[0x08, 0xac, 0xcb]);
    }

    #[test]
    fn every_calibration_band_matches_the_reference() {
        let bands = [
            (435_000_000u32, [0x6b, 0x6f]),
            (490_000_000, [0x75, 0x81]),
            (783_000_000, [0xc1, 0xc5]),
            (868_000_000, [0xd7, 0xdb]),
            (915_000_000, [0xe1, 0xe9]),
        ];
        for (hertz, expected) in bands {
            let frame = calibrate(hertz).expect("the band is covered");
            assert_eq!(frame.payload(), expected, "at {hertz} Hz");
        }
        assert!(calibrate(430_000_000).is_none(), "the edge is outside");
        assert!(calibrate(928_000_000).is_none(), "so is the other one");
    }

    #[test]
    fn a_patch_word_lands_four_addresses_on_from_the_last() {
        assert_eq!(
            patch_word(0, 0x0033_7fe1).payload(),
            &[0x80, 0x00, 0x00, 0x33, 0x7f, 0xe1]
        );
        assert_eq!(
            patch_word(1, 0x0033_7fdb).payload(),
            &[0x80, 0x04, 0x00, 0x33, 0x7f, 0xdb]
        );
        assert_eq!(patch_word(385, 0).payload()[..2], [0x86, 0x04]);
    }

    #[test]
    fn a_patch_is_read_from_the_form_it_ships_in() {
        let mut text = String::from("const uint32_t pram[] = {\n");
        for at in 0..PRAM_WORDS {
            text.push_str(&format!("0x{:x},\n", at));
        }
        text.push_str("};\n\n#define PRAM_COUNT 386\n");

        let mut patch = [0u32; PRAM_WORDS];
        read_patch(&text, &mut patch).expect("the count is right");
        assert_eq!(patch[0], 0);
        assert_eq!(patch[385], 385);
    }

    #[test]
    fn a_patch_of_the_wrong_length_is_refused_rather_than_loaded() {
        let mut patch = [0u32; PRAM_WORDS];
        assert_eq!(
            read_patch("0x1, 0x2,", &mut patch),
            Err(PatchError::WrongCount { found: 2 })
        );

        let mut text = String::new();
        for _ in 0..PRAM_WORDS + 1 {
            text.push_str("0x1,\n");
        }
        assert_eq!(
            read_patch(&text, &mut patch),
            Err(PatchError::WrongCount {
                found: PRAM_WORDS + 1
            })
        );
    }

    #[test]
    fn words_are_read_whole_rather_than_a_byte_at_a_time() {
        let mut text = String::new();
        for _ in 0..PRAM_WORDS {
            text.push_str("0x3f3fff,\n");
        }
        let mut patch = [0u32; PRAM_WORDS];
        read_patch(&text, &mut patch).expect("the count is right");
        assert_eq!(
            patch[0], 0x003f_3fff,
            "six characters are one word, not three"
        );
    }

    #[test]
    fn only_the_last_four_characters_of_the_version_are_compared() {
        let mut answer = [0u8; VERSION_TRANSFER];
        answer[3..].copy_from_slice(b"sx1261_v1.1_2D0");
        assert!(!patched(&answer), "this one is cut short of the version");

        let mut answer = [0u8; VERSION_TRANSFER];
        answer[3..].copy_from_slice(b"anything___2D06");
        assert!(patched(&answer), "the build before it does not matter");
    }

    #[test]
    fn a_short_version_answer_is_not_read_as_a_patched_radio() {
        assert!(!patched(&[]));
        assert!(!patched(&[0; 4]));
        assert_eq!(version(&[0; 4]), None);
    }

    #[test]
    fn the_receive_settings_match_the_reference() {
        let frames = receive(868_100_000, Bandwidth::Khz125);
        assert_eq!(frames[0].payload(), &[0x08, 0x9b, 0x00]);
        assert_eq!(frames[1].opcode(), OP_SET_FS);
        assert_eq!(frames[2].opcode(), OP_SET_RF_FREQUENCY);
        assert_eq!(frames[3].payload(), &[0x08, 0x9b, 0x14]);
        assert_eq!(frames[4].opcode(), OP_SET_PACKET_TYPE);
        assert_eq!(frames[4].payload(), &[0x00], "listening for FSK, not LoRa");
        assert_eq!(
            frames[5].payload(),
            &[0x00, 0x14, 0x00, 0x00, 0x0a, 0x02, 0xe9, 0x0f]
        );
        assert_eq!(
            frames[6].payload(),
            &[0x00, 0x20, 0x05, 0x20, 0x00, 0x01, 0xff, 0x00, 0x00]
        );
        assert_eq!(frames[7].payload(), &[0xff, 0xff, 0xff]);
    }

    #[test]
    fn a_wider_channel_listens_wider() {
        let narrow = receive(868_100_000, Bandwidth::Khz125);
        let wide = receive(868_100_000, Bandwidth::Khz250);
        assert_eq!(narrow[5].payload()[4], 0x0a);
        assert_eq!(wide[5].payload()[4], 0x09);
    }

    #[test]
    fn the_tuning_matches_the_front_ends_arithmetic() {
        // A whole megahertz lands on a round value, which is what makes it worth anchoring to.
        assert_eq!(frequency_value(868_000_000), 0x3640_0000);
        assert_eq!(frequency_value(915_000_000), 959447040);
        assert_eq!(frequency_value(868_100_000), 910_268_825);

        // The widening matters: a carrier this high overflows 32 bits before the divide.
        assert_eq!(frequency_value(928_000_000), 973078528);
    }

    #[test]
    fn a_status_byte_reads_as_its_fields() {
        assert_eq!(Status::read(STANDBY_READY).mode, Some(Mode::StandbyRc));
        assert_eq!(Status::read(0x54).mode, Some(Mode::Receiving));
        assert_eq!(
            Status::read(0x54).command,
            Some(CommandStatus::DataAvailable)
        );
        assert_eq!(Status::read(0xa2).byte, 0x22, "the top bit is not answered");
        assert!(Status::read(0xa3).parked(), "nor is the bottom one");
    }

    #[test]
    fn a_refused_calibration_is_told_apart_from_every_other_fault() {
        assert!(calibration_failed([0xff, 0xff, 0xff]));
        assert!(
            !calibration_failed([0xff, 0xff, 0xef]),
            "every fault but this"
        );
    }

    #[test]
    fn the_last_scan_level_counts_what_was_below_the_one_before_it() {
        let mut answer = [0u8; SCAN_TRANSFER];
        for at in 0..SCAN_LEVELS {
            answer[3 + at * 2] = 0x00;
            answer[3 + at * 2 + 1] = at as u8;
        }

        let levels = scan_counts(&answer, 0).expect("that is a whole answer");
        assert_eq!(levels[0].dbm, 0);
        assert_eq!(levels[31].dbm, -124);
        assert_eq!(levels[32].dbm, -124, "the same level, counted below it");
        assert_eq!(levels[32].count, 32);
    }

    #[test]
    fn a_scan_answer_that_is_cut_short_is_refused() {
        assert!(scan_counts(&[0u8; SCAN_TRANSFER - 1], 0).is_none());
        assert!(scan_counts(&[], 0).is_none());
    }

    #[test]
    fn the_board_correction_moves_every_level_together() {
        let answer = [0u8; SCAN_TRANSFER];
        let plain = scan_counts(&answer, 0).expect("a whole answer");
        let corrected = scan_counts(&answer, -3).expect("a whole answer");
        for at in 0..SCAN_LEVELS {
            assert_eq!(corrected[at].dbm, plain[at].dbm - 3);
        }
    }
}
