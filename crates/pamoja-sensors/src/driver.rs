//! What the drivers share: how a register-mapped part is reached over a bus.
//!
//! Bosch's BME280 and BMP280 speak one register protocol over two buses, I2C and SPI.
//! [`RegisterBus`] names the two operations that protocol needs, a multi-byte read
//! starting at a register and a single-byte write, and [`I2cRegisters`] and
//! [`SpiRegisters`] implement them: over I2C the register address is written and the
//! data read back in one transaction, over SPI bit 7 of the control byte is the
//! read/write flag and the address increments on its own, as the BME280 datasheet's
//! interface chapter lays out. A driver written against the trait runs on either bus.
//!
//! Texas Instruments' parts keep 16-bit registers that travel most significant byte
//! first over I2C; [`read_word`] and [`write_word`] are that shape.

use embedded_hal::i2c::I2c;
use embedded_hal::spi::{Operation, SpiDevice};

/// The read/write flag in a Bosch SPI control byte: set to read, clear to write.
pub const SPI_READ: u8 = 0x80;

/// The mask that leaves the seven address bits of a Bosch SPI control byte.
pub const SPI_ADDRESS_MASK: u8 = 0x7F;

/// A part's register map reached over some bus.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::{I2cScript, I2cStep};
/// use pamoja_sensors::driver::{I2cRegisters, RegisterBus};
///
/// let script = I2cScript::new([
///     I2cStep::write_read(0x76, [0xD0], [0x60]),
///     I2cStep::write(0x76, [0xE0, 0xB6]),
/// ]);
/// let mut part = I2cRegisters::new(script, 0x76);
/// assert_eq!(part.read_register(0xD0)?, 0x60);
/// part.write_register(0xE0, 0xB6)?;
/// assert!(part.release().done());
/// # Ok::<(), pamoja_hal::script::ScriptError>(())
/// ```
pub trait RegisterBus {
    /// The error the bus underneath reports.
    type Error;

    /// Reads consecutive registers starting at `first`, as many as `buffer` holds.
    ///
    /// # Arguments
    ///
    /// * `first` - the address of the first register.
    /// * `buffer` - where the register values go, one byte per register.
    ///
    /// # Errors
    ///
    /// Returns the bus error if the part does not answer.
    fn read_registers(&mut self, first: u8, buffer: &mut [u8]) -> Result<(), Self::Error>;

    /// Writes one register.
    ///
    /// # Arguments
    ///
    /// * `register` - the register address.
    /// * `value` - the byte to write.
    ///
    /// # Errors
    ///
    /// Returns the bus error if the part does not answer.
    fn write_register(&mut self, register: u8, value: u8) -> Result<(), Self::Error>;

    /// Reads one register.
    ///
    /// # Arguments
    ///
    /// * `register` - the register address.
    ///
    /// # Returns
    ///
    /// The register's byte.
    ///
    /// # Errors
    ///
    /// Returns the bus error if the part does not answer.
    fn read_register(&mut self, register: u8) -> Result<u8, Self::Error> {
        let mut byte = [0u8; 1];
        self.read_registers(register, &mut byte)?;
        Ok(byte[0])
    }
}

/// A register map behind an I2C address.
///
/// A read writes the register address and reads the values back in one transaction,
/// with a repeated start between them; a write sends the address and the value.
#[derive(Debug)]
pub struct I2cRegisters<I2C> {
    bus: I2C,
    address: u8,
}

impl<I2C> I2cRegisters<I2C> {
    /// Wraps an I2C bus and the part's 7-bit address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `address` - the part's 7-bit address.
    ///
    /// # Returns
    ///
    /// The register map.
    pub fn new(bus: I2C, address: u8) -> I2cRegisters<I2C> {
        I2cRegisters { bus, address }
    }

    /// Returns the part's 7-bit address.
    pub fn address(&self) -> u8 {
        self.address
    }

    /// Gives back the bus.
    ///
    /// # Returns
    ///
    /// The bus the map was built from.
    pub fn release(self) -> I2C {
        self.bus
    }
}

impl<I2C: I2c> RegisterBus for I2cRegisters<I2C> {
    type Error = I2C::Error;

    fn read_registers(&mut self, first: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.bus.write_read(self.address, &[first], buffer)
    }

    fn write_register(&mut self, register: u8, value: u8) -> Result<(), Self::Error> {
        self.bus.write(self.address, &[register, value])
    }
}

/// A register map behind an SPI chip select, in the Bosch framing.
///
/// The first byte of every transfer is the control byte: the register address with
/// bit 7 set to read or cleared to write. On a read the part then streams consecutive
/// registers; on a write the value follows. The part accepts SPI modes 0 and 3.
#[derive(Debug)]
pub struct SpiRegisters<SPI> {
    device: SPI,
}

impl<SPI> SpiRegisters<SPI> {
    /// Wraps an SPI device, which is the bus plus the part's chip-select line.
    ///
    /// # Arguments
    ///
    /// * `device` - the SPI device.
    ///
    /// # Returns
    ///
    /// The register map.
    pub fn new(device: SPI) -> SpiRegisters<SPI> {
        SpiRegisters { device }
    }

    /// Gives back the device.
    ///
    /// # Returns
    ///
    /// The device the map was built from.
    pub fn release(self) -> SPI {
        self.device
    }
}

impl<SPI: SpiDevice> RegisterBus for SpiRegisters<SPI> {
    type Error = SPI::Error;

    fn read_registers(&mut self, first: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        let control = [first | SPI_READ];
        self.device
            .transaction(&mut [Operation::Write(&control), Operation::Read(buffer)])
    }

    fn write_register(&mut self, register: u8, value: u8) -> Result<(), Self::Error> {
        self.device.write(&[register & SPI_ADDRESS_MASK, value])
    }
}

/// Reads a 16-bit register that travels most significant byte first.
///
/// # Arguments
///
/// * `bus` - the I2C bus.
/// * `address` - the part's 7-bit address.
/// * `register` - the register address, or pointer value, to read.
///
/// # Returns
///
/// The register's value.
///
/// # Errors
///
/// Returns the bus error if the part does not answer.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::{I2cScript, I2cStep};
/// use pamoja_sensors::driver::read_word;
///
/// // A TMP117 reads its device id register as 0x0117.
/// let mut bus = I2cScript::new([I2cStep::write_read(0x48, [0x0F], [0x01, 0x17])]);
/// assert_eq!(read_word(&mut bus, 0x48, 0x0F)?, 0x0117);
/// # Ok::<(), pamoja_hal::script::ScriptError>(())
/// ```
pub fn read_word<I2C: I2c>(bus: &mut I2C, address: u8, register: u8) -> Result<u16, I2C::Error> {
    let mut bytes = [0u8; 2];
    bus.write_read(address, &[register], &mut bytes)?;
    Ok(u16::from_be_bytes(bytes))
}

/// Writes a 16-bit register that travels most significant byte first.
///
/// # Arguments
///
/// * `bus` - the I2C bus.
/// * `address` - the part's 7-bit address.
/// * `register` - the register address, or pointer value, to write.
/// * `value` - the value to write.
///
/// # Errors
///
/// Returns the bus error if the part does not answer.
pub fn write_word<I2C: I2c>(
    bus: &mut I2C,
    address: u8,
    register: u8,
    value: u16,
) -> Result<(), I2C::Error> {
    let [high, low] = value.to_be_bytes();
    bus.write(address, &[register, high, low])
}

#[cfg(test)]
mod tests {
    use super::*;
    use pamoja_hal::script::{I2cScript, I2cStep, SpiScript, SpiStep};

    #[test]
    fn i2c_registers_read_with_a_repeated_start_and_write_address_then_value() {
        let script = I2cScript::new([
            I2cStep::write_read(0x77, [0xF7], [1, 2, 3, 4, 5, 6, 7, 8]),
            I2cStep::write(0x77, [0xF4, 0x25]),
        ]);
        let mut part = I2cRegisters::new(script, 0x77);
        let mut data = [0u8; 8];
        part.read_registers(0xF7, &mut data).unwrap();
        assert_eq!(data, [1, 2, 3, 4, 5, 6, 7, 8]);
        part.write_register(0xF4, 0x25).unwrap();
        assert_eq!(part.address(), 0x77);
        assert!(part.release().done());
    }

    #[test]
    fn spi_registers_set_the_read_flag_and_clear_it_to_write() {
        let script = SpiScript::new([
            SpiStep::write([0xD0]),
            SpiStep::read([0x60]),
            SpiStep::write([0x74, 0x25]),
        ]);
        let mut part = SpiRegisters::new(script);
        assert_eq!(part.read_register(0xD0).unwrap(), 0x60);
        part.write_register(0xF4, 0x25).unwrap();
        assert!(part.release().done());
    }

    #[test]
    fn words_travel_most_significant_byte_first() {
        let mut bus = I2cScript::new([
            I2cStep::write(0x40, [0x05, 0x10, 0x00]),
            I2cStep::write_read(0x40, [0x02], [0x1F, 0x40]),
        ]);
        write_word(&mut bus, 0x40, 0x05, 0x1000).unwrap();
        assert_eq!(read_word(&mut bus, 0x40, 0x02).unwrap(), 0x1F40);
        assert!(bus.done());
    }
}
