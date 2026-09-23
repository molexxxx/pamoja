//! A part that is not there, answering from a register map.
//!
//! [`script`](crate::script) plays one exact conversation: these transfers, in this order,
//! and anything else is refused. That is what proves a driver follows a datasheet, and it is
//! a lot to write out before a driver has been run even once.
//!
//! This is the other half. An [`I2cPart`] is a register map with an address on the front of
//! it: a write puts bytes at a register and the ones after it, a read takes them back, and
//! the part does not mind what order any of that happens in. Give it a chip id and whatever
//! a driver reads, and the driver runs against it with nothing plugged in and no transfer
//! sequence written down.
//!
//! Parts come in three shapes, and there is one for each:
//!
//! - [`I2cPart`], registers a byte wide, as Bosch's BME280 and BMP280 have.
//! - [`WordPart`], registers sixteen bits wide, as Texas Instruments' parts have, with bits
//!   the part sets for itself marked read-only.
//! - `CommandPart` (feature `alloc`), commands and the replies they leave, as Sensirion's
//!   parts take.
//!
//! A simulated bus holds any of them as a `Part`, and gives each back as its own kind.
//!
//! Which to reach for:
//!
//! - A part here, when the question is what a driver does with what it reads, or when you
//!   are writing a driver and want to run it before the hardware arrives.
//! - A [script](crate::script), when the question is whether the exact transfers a datasheet
//!   prescribes went out, in order and no others.
//!
//! Almost every I2C part addresses its registers with one byte and moves through them as it
//! is read, so a map of 256 registers holds one whole part and needs no allocator.

use embedded_hal::i2c::{
    ErrorKind, ErrorType, I2c, NoAcknowledgeSource, Operation, SevenBitAddress,
};

/// How many registers a part holds, which is every address one byte reaches.
pub const REGISTERS: usize = 256;

/// What a simulated part refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartError {
    /// A transfer was addressed to something else on the bus.
    WrongAddress {
        /// The address the transfer went to.
        asked: u8,
        /// The address this part answers to.
        answers: u8,
    },
    /// A transfer asked for a register before saying which one.
    NoRegister,
    /// A read reached a part that takes commands when no command had left it a reply, which
    /// a real part answers by not acknowledging.
    NoReply,
}

impl core::fmt::Display for PartError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PartError::WrongAddress { asked, answers } => write!(
                f,
                "a transfer to {asked:#04x} reached the part at {answers:#04x}"
            ),
            PartError::NoRegister => {
                f.write_str("a read came before anything said which register to read")
            }
            PartError::NoReply => f.write_str("a read came when no command had left a reply"),
        }
    }
}

impl core::error::Error for PartError {}

impl embedded_hal::i2c::Error for PartError {
    fn kind(&self) -> ErrorKind {
        match self {
            PartError::WrongAddress { .. } => {
                ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)
            }
            PartError::NoRegister => ErrorKind::Other,
            PartError::NoReply => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address),
        }
    }
}

/// A part on an I2C bus, answering from its registers.
///
/// # Examples
///
/// ```
/// use pamoja_hal::i2c::I2c;
/// use pamoja_hal::sim::I2cPart;
///
/// // A part at 0x76 whose chip id register reads 0x60.
/// let mut part = I2cPart::new(0x76).holding(0xd0, &[0x60]);
///
/// let mut id = [0u8; 1];
/// part.write_read(0x76, &[0xd0], &mut id).unwrap();
/// assert_eq!(id, [0x60]);
///
/// // What a driver writes stays written, so a test can read its configuration back.
/// part.write(0x76, &[0xf4, 0x25]).unwrap();
/// assert_eq!(part.register(0xf4), 0x25);
/// ```
#[derive(Clone, Debug)]
pub struct I2cPart {
    address: u8,
    registers: [u8; REGISTERS],
    pointer: Option<u8>,
    transfers: usize,
}

impl I2cPart {
    /// A part answering at one address, with every register reading zero.
    ///
    /// # Arguments
    ///
    /// * `address` - the address it answers to.
    ///
    /// # Returns
    ///
    /// The part.
    #[must_use]
    pub const fn new(address: u8) -> I2cPart {
        I2cPart {
            address,
            registers: [0; REGISTERS],
            pointer: None,
            transfers: 0,
        }
    }

    /// The same part, holding these bytes from a register on.
    ///
    /// # Arguments
    ///
    /// * `first` - the register the bytes start at.
    /// * `bytes` - what to put there. Past the last register it wraps to the first.
    ///
    /// # Returns
    ///
    /// The part.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_hal::sim::I2cPart;
    ///
    /// let part = I2cPart::new(0x48).holding(0x00, &[0x12, 0x34]);
    /// assert_eq!(part.register(0x01), 0x34);
    /// ```
    #[must_use]
    pub fn holding(mut self, first: u8, bytes: &[u8]) -> I2cPart {
        self.load(first, bytes);
        self
    }

    /// Puts bytes in the part, from a register on.
    ///
    /// # Arguments
    ///
    /// * `first` - the register the bytes start at.
    /// * `bytes` - what to put there. Past the last register it wraps to the first.
    pub fn load(&mut self, first: u8, bytes: &[u8]) {
        for (offset, byte) in bytes.iter().enumerate() {
            let at = first.wrapping_add(offset as u8);
            self.registers[at as usize] = *byte;
        }
    }

    /// What a register holds now.
    ///
    /// # Arguments
    ///
    /// * `register` - which one.
    ///
    /// # Returns
    ///
    /// Its value, which is what a driver wrote if it wrote one.
    #[must_use]
    pub const fn register(&self, register: u8) -> u8 {
        self.registers[register as usize]
    }

    /// The address this part answers to.
    ///
    /// # Returns
    ///
    /// The address.
    #[must_use]
    pub const fn address(&self) -> u8 {
        self.address
    }

    /// How many transfers the part has served.
    ///
    /// # Returns
    ///
    /// The count, one for each transfer a driver made, however many operations it held.
    #[must_use]
    pub const fn transfers(&self) -> usize {
        self.transfers
    }

    fn check(&self, address: u8) -> Result<(), PartError> {
        if address == self.address {
            Ok(())
        } else {
            Err(PartError::WrongAddress {
                asked: address,
                answers: self.address,
            })
        }
    }

    // A write names a register and then fills it and the ones after it.
    fn take_write(&mut self, bytes: &[u8]) {
        let Some((&first, rest)) = bytes.split_first() else {
            return;
        };
        self.pointer = Some(first);
        self.load(first, rest);
        self.pointer = Some(first.wrapping_add(rest.len() as u8));
    }

    // A read takes from wherever the last write left off, moving on as it goes.
    fn take_read(&mut self, buffer: &mut [u8]) -> Result<(), PartError> {
        let Some(first) = self.pointer else {
            return Err(PartError::NoRegister);
        };
        let mut at = first;
        for slot in buffer.iter_mut() {
            *slot = self.registers[at as usize];
            at = at.wrapping_add(1);
        }
        self.pointer = Some(at);
        Ok(())
    }
}

impl ErrorType for I2cPart {
    type Error = PartError;
}

impl I2c<SevenBitAddress> for I2cPart {
    fn transaction(
        &mut self,
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.check(address)?;
        self.transfers += 1;
        for operation in operations {
            match operation {
                Operation::Write(bytes) => self.take_write(bytes),
                Operation::Read(buffer) => self.take_read(buffer)?,
            }
        }
        Ok(())
    }
}

/// A part on an I2C bus whose registers are sixteen bits wide, answering from them.
///
/// This is how Texas Instruments lays out parts such as the TMP117, the INA219 and INA226,
/// the OPT3001, the ADS1115, and the HDC1080. A pointer byte names a register and a register
/// travels most significant byte first. A write of the pointer alone aims the next read; a
/// write of the pointer and a word stores the word and leaves the pointer where it was; a read
/// takes words from the pointer on, carrying into the next register when it runs past one, as
/// an HDC1080 does after a triggered measurement.
///
/// Some bits are the part's to set, however a driver writes the register: a conversion-ready
/// flag, an overflow flag. [`WordPart::read_only`] marks them, and they then read as the part
/// holds them.
///
/// # Examples
///
/// ```
/// use pamoja_hal::i2c::I2c;
/// use pamoja_hal::sim::WordPart;
///
/// // A TMP117's device id register, 0x0F, reads 0x0117.
/// let mut part = WordPart::new(0x48).holding(0x0F, 0x0117);
/// let mut id = [0u8; 2];
/// part.write_read(0x48, &[0x0F], &mut id).unwrap();
/// assert_eq!(u16::from_be_bytes(id), 0x0117);
///
/// // Bit 7 of register 0x01 is a ready flag the part sets; a driver writing zero there
/// // does not clear it.
/// let mut part = WordPart::new(0x44).holding(0x01, 0x0080).read_only(0x01, 0x0080);
/// part.write(0x44, &[0x01, 0xCA, 0x10]).unwrap();
/// assert_eq!(part.word(0x01), 0xCA90);
/// ```
#[derive(Clone, Debug)]
pub struct WordPart {
    address: u8,
    registers: [u16; REGISTERS],
    fixed: [u16; REGISTERS],
    pointer: Option<u8>,
    transfers: usize,
}

impl WordPart {
    /// A part answering at one address, with every register reading zero.
    ///
    /// # Arguments
    ///
    /// * `address` - the address it answers to.
    ///
    /// # Returns
    ///
    /// The part.
    #[must_use]
    pub const fn new(address: u8) -> WordPart {
        WordPart {
            address,
            registers: [0; REGISTERS],
            fixed: [0; REGISTERS],
            pointer: None,
            transfers: 0,
        }
    }

    /// The same part, holding a value in one register.
    ///
    /// # Arguments
    ///
    /// * `register` - the register.
    /// * `value` - what it holds, read-only bits included.
    ///
    /// # Returns
    ///
    /// The part.
    #[must_use]
    pub fn holding(mut self, register: u8, value: u16) -> WordPart {
        self.set(register, value);
        self
    }

    /// The same part, with some bits of one register left as the part holds them however a
    /// driver writes the register.
    ///
    /// # Arguments
    ///
    /// * `register` - the register.
    /// * `mask` - the bits that are the part's to set.
    ///
    /// # Returns
    ///
    /// The part.
    #[must_use]
    pub fn read_only(mut self, register: u8, mask: u16) -> WordPart {
        self.fixed[register as usize] |= mask;
        self
    }

    /// Puts a value in one register, read-only bits included, the way the part itself would.
    ///
    /// # Arguments
    ///
    /// * `register` - the register.
    /// * `value` - what it holds.
    pub fn set(&mut self, register: u8, value: u16) {
        self.registers[register as usize] = value;
    }

    /// What one register holds now.
    ///
    /// # Arguments
    ///
    /// * `register` - which one.
    ///
    /// # Returns
    ///
    /// Its value, which is what a driver wrote there if it wrote one, apart from the
    /// read-only bits.
    #[must_use]
    pub const fn word(&self, register: u8) -> u16 {
        self.registers[register as usize]
    }

    /// The address this part answers to.
    ///
    /// # Returns
    ///
    /// The address.
    #[must_use]
    pub const fn address(&self) -> u8 {
        self.address
    }

    /// How many transfers the part has served.
    ///
    /// # Returns
    ///
    /// The count, one for each transfer a driver made, however many operations it held.
    #[must_use]
    pub const fn transfers(&self) -> usize {
        self.transfers
    }

    fn check(&self, address: u8) -> Result<(), PartError> {
        if address == self.address {
            Ok(())
        } else {
            Err(PartError::WrongAddress {
                asked: address,
                answers: self.address,
            })
        }
    }

    fn take_write(&mut self, bytes: &[u8]) {
        let Some((&pointer, words)) = bytes.split_first() else {
            return;
        };
        self.pointer = Some(pointer);
        let mut at = pointer;
        for pair in words.as_chunks::<2>().0 {
            let written = u16::from_be_bytes(*pair);
            let fixed = self.fixed[at as usize];
            let held = self.registers[at as usize];
            self.registers[at as usize] = (written & !fixed) | (held & fixed);
            at = at.wrapping_add(1);
        }
    }

    fn take_read(&mut self, buffer: &mut [u8]) -> Result<(), PartError> {
        let Some(mut at) = self.pointer else {
            return Err(PartError::NoRegister);
        };
        for pair in buffer.chunks_mut(2) {
            let bytes = self.registers[at as usize].to_be_bytes();
            pair.copy_from_slice(&bytes[..pair.len()]);
            at = at.wrapping_add(1);
        }
        Ok(())
    }
}

impl ErrorType for WordPart {
    type Error = PartError;
}

impl I2c<SevenBitAddress> for WordPart {
    fn transaction(
        &mut self,
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.check(address)?;
        self.transfers += 1;
        for operation in operations {
            match operation {
                Operation::Write(bytes) => self.take_write(bytes),
                Operation::Read(buffer) => self.take_read(buffer)?,
            }
        }
        Ok(())
    }
}

/// A part on an I2C bus that takes commands rather than register addresses, answering each
/// with the reply it was given for that command.
///
/// This is how Sensirion lays out parts such as the SHT3x and the SCD4x. A write sends a
/// command, and any arguments after it; a read then takes the reply that command left, once.
/// A command given no reply leaves none, and a read then is not acknowledged, which is what
/// the real parts do when asked for data they do not have. A read longer than the reply gets
/// `0xFF` for the rest, the level an idle bus reads.
///
/// # Examples
///
/// ```
/// use pamoja_hal::i2c::I2c;
/// use pamoja_hal::sim::CommandPart;
///
/// // A part whose 16-bit status command, 0xF32D, answers with three bytes.
/// let mut part = CommandPart::new(0x44, 2).answering(&[0xF3, 0x2D], &[0x80, 0x10, 0xE1]);
/// part.write(0x44, &[0xF3, 0x2D]).unwrap();
/// let mut status = [0u8; 3];
/// part.read(0x44, &mut status).unwrap();
/// assert_eq!(status, [0x80, 0x10, 0xE1]);
///
/// // The reply was taken, so a second read finds nothing waiting.
/// assert!(part.read(0x44, &mut status).is_err());
/// assert_eq!(part.received().len(), 1);
/// assert_eq!(part.received()[0], [0xF3, 0x2D]);
/// ```
#[cfg(feature = "alloc")]
#[derive(Clone, Debug)]
pub struct CommandPart {
    address: u8,
    width: usize,
    replies: alloc::vec::Vec<(alloc::vec::Vec<u8>, alloc::vec::Vec<u8>)>,
    waiting: Option<alloc::vec::Vec<u8>>,
    received: alloc::vec::Vec<alloc::vec::Vec<u8>>,
    transfers: usize,
}

#[cfg(feature = "alloc")]
impl CommandPart {
    /// A part answering at one address that has been given no replies yet.
    ///
    /// # Arguments
    ///
    /// * `address` - the address it answers to.
    /// * `width` - how many bytes a command takes: two for Sensirion's 16-bit commands.
    ///
    /// # Returns
    ///
    /// The part.
    #[must_use]
    pub fn new(address: u8, width: usize) -> CommandPart {
        CommandPart {
            address,
            width: width.max(1),
            replies: alloc::vec::Vec::new(),
            waiting: None,
            received: alloc::vec::Vec::new(),
            transfers: 0,
        }
    }

    /// The same part, answering one command with a reply.
    ///
    /// # Arguments
    ///
    /// * `command` - the command's bytes.
    /// * `reply` - what a read after it returns, in place of any reply given before.
    ///
    /// # Returns
    ///
    /// The part.
    #[must_use]
    pub fn answering(mut self, command: &[u8], reply: &[u8]) -> CommandPart {
        self.answer(command, reply);
        self
    }

    /// Answers one command with a reply from now on, in place of any reply given before.
    ///
    /// # Arguments
    ///
    /// * `command` - the command's bytes.
    /// * `reply` - what a read after it returns.
    pub fn answer(&mut self, command: &[u8], reply: &[u8]) {
        match self.replies.iter_mut().find(|(held, _)| held == command) {
            Some((_, held)) => *held = reply.to_vec(),
            None => self.replies.push((command.to_vec(), reply.to_vec())),
        }
    }

    /// Every write the part has received, oldest first.
    ///
    /// # Returns
    ///
    /// The writes, each a command and whatever arguments followed it.
    #[must_use]
    pub fn received(&self) -> &[alloc::vec::Vec<u8>] {
        &self.received
    }

    /// The address this part answers to.
    ///
    /// # Returns
    ///
    /// The address.
    #[must_use]
    pub const fn address(&self) -> u8 {
        self.address
    }

    /// How many transfers the part has served.
    ///
    /// # Returns
    ///
    /// The count, one for each transfer a driver made, however many operations it held.
    #[must_use]
    pub const fn transfers(&self) -> usize {
        self.transfers
    }

    fn check(&self, address: u8) -> Result<(), PartError> {
        if address == self.address {
            Ok(())
        } else {
            Err(PartError::WrongAddress {
                asked: address,
                answers: self.address,
            })
        }
    }

    fn take_write(&mut self, bytes: &[u8]) {
        self.received.push(bytes.to_vec());
        let command = &bytes[..bytes.len().min(self.width)];
        self.waiting = self
            .replies
            .iter()
            .find(|(held, _)| held == command)
            .map(|(_, reply)| reply.clone());
    }

    fn take_read(&mut self, buffer: &mut [u8]) -> Result<(), PartError> {
        let reply = self.waiting.take().ok_or(PartError::NoReply)?;
        for (index, slot) in buffer.iter_mut().enumerate() {
            *slot = reply.get(index).copied().unwrap_or(0xFF);
        }
        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl ErrorType for CommandPart {
    type Error = PartError;
}

#[cfg(feature = "alloc")]
impl I2c<SevenBitAddress> for CommandPart {
    fn transaction(
        &mut self,
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.check(address)?;
        self.transfers += 1;
        for operation in operations {
            match operation {
                Operation::Write(bytes) => self.take_write(bytes),
                Operation::Read(buffer) => self.take_read(buffer)?,
            }
        }
        Ok(())
    }
}

/// A simulated part of any of the three kinds, as a simulated bus holds it.
///
/// A word part carries two maps of 256 sixteen-bit registers, four times what a byte part
/// holds, so it is boxed to keep every `Part` the size of the smaller two.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug)]
pub enum Part {
    /// A part whose registers are a byte wide.
    Bytes(I2cPart),
    /// A part whose registers are sixteen bits wide.
    Words(alloc::boxed::Box<WordPart>),
    /// A part that takes commands.
    Commands(CommandPart),
}

#[cfg(feature = "alloc")]
impl Part {
    /// The address this part answers to.
    ///
    /// # Returns
    ///
    /// The address.
    #[must_use]
    pub fn address(&self) -> u8 {
        match self {
            Part::Bytes(part) => part.address(),
            Part::Words(part) => part.address(),
            Part::Commands(part) => part.address(),
        }
    }

    /// How many transfers the part has served.
    ///
    /// # Returns
    ///
    /// The count.
    #[must_use]
    pub fn transfers(&self) -> usize {
        match self {
            Part::Bytes(part) => part.transfers(),
            Part::Words(part) => part.transfers(),
            Part::Commands(part) => part.transfers(),
        }
    }
}

#[cfg(feature = "alloc")]
impl From<I2cPart> for Part {
    fn from(part: I2cPart) -> Part {
        Part::Bytes(part)
    }
}

#[cfg(feature = "alloc")]
impl From<WordPart> for Part {
    fn from(part: WordPart) -> Part {
        Part::Words(alloc::boxed::Box::new(part))
    }
}

#[cfg(feature = "alloc")]
impl From<CommandPart> for Part {
    fn from(part: CommandPart) -> Part {
        Part::Commands(part)
    }
}

#[cfg(feature = "alloc")]
impl ErrorType for Part {
    type Error = PartError;
}

#[cfg(feature = "alloc")]
impl I2c<SevenBitAddress> for Part {
    fn transaction(
        &mut self,
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        match self {
            Part::Bytes(part) => part.transaction(address, operations),
            Part::Words(part) => part.transaction(address, operations),
            Part::Commands(part) => part.transaction(address, operations),
        }
    }
}

/// A kind of simulated part, taken back out of a [`Part`].
#[cfg(feature = "alloc")]
pub trait FromPart: Sized {
    /// The part, when it is this kind.
    ///
    /// # Arguments
    ///
    /// * `part` - any simulated part.
    ///
    /// # Returns
    ///
    /// The part as this kind, or `None` when it is another.
    fn from_part(part: Part) -> Option<Self>;
}

#[cfg(feature = "alloc")]
impl FromPart for Part {
    fn from_part(part: Part) -> Option<Part> {
        Some(part)
    }
}

#[cfg(feature = "alloc")]
impl FromPart for I2cPart {
    fn from_part(part: Part) -> Option<I2cPart> {
        match part {
            Part::Bytes(part) => Some(part),
            _ => None,
        }
    }
}

#[cfg(feature = "alloc")]
impl FromPart for WordPart {
    fn from_part(part: Part) -> Option<WordPart> {
        match part {
            Part::Words(part) => Some(*part),
            _ => None,
        }
    }
}

#[cfg(feature = "alloc")]
impl FromPart for CommandPart {
    fn from_part(part: Part) -> Option<CommandPart> {
        match part {
            Part::Commands(part) => Some(part),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_takes_what_a_write_left() {
        let mut part = I2cPart::new(0x76).holding(0x88, &[1, 2, 3, 4]);

        let mut read = [0u8; 4];
        part.write_read(0x76, &[0x88], &mut read).unwrap();
        assert_eq!(read, [1, 2, 3, 4], "a block read walks the registers");
    }

    #[test]
    fn what_a_driver_writes_is_what_the_part_holds() {
        let mut part = I2cPart::new(0x76);

        part.write(0x76, &[0xf4, 0x25]).unwrap();
        assert_eq!(part.register(0xf4), 0x25);

        // A longer write fills the registers after it, which is how a part takes a block.
        part.write(0x76, &[0x10, 0xaa, 0xbb, 0xcc]).unwrap();
        assert_eq!(part.register(0x10), 0xaa);
        assert_eq!(part.register(0x12), 0xcc);
    }

    #[test]
    fn a_transfer_to_another_address_is_refused() {
        let mut part = I2cPart::new(0x76);
        let mut read = [0u8; 1];

        assert_eq!(
            part.write_read(0x77, &[0xd0], &mut read),
            Err(PartError::WrongAddress {
                asked: 0x77,
                answers: 0x76
            })
        );
    }

    #[test]
    fn a_read_before_any_register_was_named_is_refused() {
        let mut part = I2cPart::new(0x76);
        let mut read = [0u8; 1];

        assert_eq!(part.read(0x76, &mut read), Err(PartError::NoRegister));
    }

    #[test]
    fn a_plain_read_carries_on_from_the_last_one() {
        let mut part = I2cPart::new(0x76).holding(0x00, &[10, 20, 30, 40]);

        let mut first = [0u8; 2];
        part.write_read(0x76, &[0x00], &mut first).unwrap();
        assert_eq!(first, [10, 20]);

        // The part has moved on, the way a driver reading a burst in pieces expects.
        let mut next = [0u8; 2];
        part.read(0x76, &mut next).unwrap();
        assert_eq!(next, [30, 40]);
    }

    #[test]
    fn the_registers_wrap_rather_than_running_off_the_end() {
        let mut part = I2cPart::new(0x76).holding(0xff, &[0x99]);
        part.load(0x00, &[0x11]);

        let mut read = [0u8; 2];
        part.write_read(0x76, &[0xff], &mut read).unwrap();
        assert_eq!(
            read,
            [0x99, 0x11],
            "past the last register is the first one"
        );
    }

    #[test]
    fn a_word_part_keeps_its_read_only_bits_and_reads_on_into_the_next_register() {
        let mut part = WordPart::new(0x40)
            .holding(0x00, 0x6660)
            .holding(0x01, 0x8A00)
            .holding(0x02, 0x1000)
            .read_only(0x02, 0x0003);

        let mut both = [0u8; 4];
        part.write(0x40, &[0x00]).unwrap();
        part.read(0x40, &mut both).unwrap();
        assert_eq!(both, [0x66, 0x60, 0x8A, 0x00], "temperature, then humidity");

        part.write(0x40, &[0x02, 0xFF, 0xFC]).unwrap();
        assert_eq!(
            part.word(0x02),
            0xFFFC,
            "the read-only bits kept their zeros"
        );
        part.set(0x02, 0x0003);
        assert_eq!(part.word(0x02), 0x0003, "the part itself may set them");

        let mut odd = [0u8; 3];
        part.write_read(0x40, &[0x00], &mut odd).unwrap();
        assert_eq!(odd, [0x66, 0x60, 0x8A]);
        assert_eq!(
            WordPart::new(0x40).read(0x40, &mut odd),
            Err(PartError::NoRegister)
        );
        assert_eq!(
            part.write(0x41, &[0x00]),
            Err(PartError::WrongAddress {
                asked: 0x41,
                answers: 0x40
            })
        );
    }

    #[test]
    fn a_command_part_answers_by_command_and_keeps_every_argument() {
        let mut part = CommandPart::new(0x62, 2)
            .answering(&[0xE4, 0xB8], &[0x80, 0x06])
            .answering(&[0xE4, 0xB8], &[0x80, 0x06, 0x95]);

        part.write(0x62, &[0x24, 0x1D, 0x07, 0x92, 0x3A]).unwrap();
        let mut ready = [0u8; 4];
        assert_eq!(
            part.read(0x62, &mut ready),
            Err(PartError::NoReply),
            "a command with no reply leaves nothing"
        );

        part.write(0x62, &[0xE4, 0xB8]).unwrap();
        part.read(0x62, &mut ready).unwrap();
        assert_eq!(
            ready,
            [0x80, 0x06, 0x95, 0xFF],
            "the later reply, then an idle bus"
        );
        assert_eq!(part.received()[0], [0x24, 0x1D, 0x07, 0x92, 0x3A]);
        assert_eq!(part.transfers(), 4);
    }

    #[test]
    fn a_transfer_is_counted_once_however_many_operations_it_holds() {
        let mut part = I2cPart::new(0x76).holding(0xd0, &[0x60]);
        let mut read = [0u8; 1];

        part.write(0x76, &[0xe0, 0xb6]).unwrap();
        part.write_read(0x76, &[0xd0], &mut read).unwrap();
        assert_eq!(part.transfers(), 2);
    }
}
