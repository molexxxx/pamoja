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
    fn a_transfer_is_counted_once_however_many_operations_it_holds() {
        let mut part = I2cPart::new(0x76).holding(0xd0, &[0x60]);
        let mut read = [0u8; 1];

        part.write(0x76, &[0xe0, 0xb6]).unwrap();
        part.write_read(0x76, &[0xd0], &mut read).unwrap();
        assert_eq!(part.transfers(), 2);
    }
}
