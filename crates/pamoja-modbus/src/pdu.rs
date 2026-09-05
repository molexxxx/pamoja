//! The Modbus protocol data unit and the standard requests that build one.

use crate::adu::Adu;
use crate::error::ModbusError;

/// A Modbus protocol data unit: a function code followed by its data.
///
/// The PDU is the part of a frame that is the same on every transport. On RTU it sits
/// between the unit address and the CRC; wrap one with [`to_adu`](Pdu::to_adu) to get a
/// frame ready for the wire.
///
/// The constructors build the standard requests so callers state intent ("read three
/// holding registers") rather than packing bytes, encoding addresses and counts in the
/// big-endian order Modbus uses. For a function code this crate does not name, [`raw`](Pdu::raw)
/// carries arbitrary bytes through unchanged. The data is held in a fixed buffer, so a
/// PDU needs no allocation.
///
/// # Examples
///
/// ```
/// use pamoja_modbus::Pdu;
///
/// let pdu = Pdu::write_single_register(0x0001, 0x0003);
/// assert_eq!(pdu.as_bytes(), &[0x06, 0x00, 0x01, 0x00, 0x03]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pdu {
    bytes: [u8; Pdu::MAX_LEN],
    len: usize,
}

impl Pdu {
    /// The largest a Modbus RTU PDU may be, in bytes: the 256-byte ADU less the one-byte
    /// address and the two-byte CRC.
    pub const MAX_LEN: usize = 253;

    /// The most registers a single write-multiple-registers request may carry.
    pub const MAX_WRITE_REGISTERS: usize = 123;

    /// The most coils a single write-multiple-coils request may carry.
    pub const MAX_WRITE_COILS: usize = 1968;

    // Builds a five-byte request: a function code and two 16-bit words. Read requests
    // carry a starting address and a quantity; single-write requests carry an address
    // and a value; the byte layout is identical.
    fn pair(function: u8, first: u16, second: u16) -> Pdu {
        let mut bytes = [0u8; Self::MAX_LEN];
        bytes[0] = function;
        bytes[1..3].copy_from_slice(&first.to_be_bytes());
        bytes[3..5].copy_from_slice(&second.to_be_bytes());
        Pdu { bytes, len: 5 }
    }

    /// Builds a read-coils request (function `0x01`).
    ///
    /// # Arguments
    ///
    /// * `start` - the address of the first coil to read.
    /// * `count` - how many coils to read.
    ///
    /// # Returns
    ///
    /// The request PDU.
    pub fn read_coils(start: u16, count: u16) -> Pdu {
        Self::pair(0x01, start, count)
    }

    /// Builds a read-discrete-inputs request (function `0x02`).
    ///
    /// # Arguments
    ///
    /// * `start` - the address of the first discrete input to read.
    /// * `count` - how many inputs to read.
    ///
    /// # Returns
    ///
    /// The request PDU.
    pub fn read_discrete_inputs(start: u16, count: u16) -> Pdu {
        Self::pair(0x02, start, count)
    }

    /// Builds a read-holding-registers request (function `0x03`).
    ///
    /// # Arguments
    ///
    /// * `start` - the address of the first holding register to read.
    /// * `count` - how many registers to read.
    ///
    /// # Returns
    ///
    /// The request PDU.
    pub fn read_holding_registers(start: u16, count: u16) -> Pdu {
        Self::pair(0x03, start, count)
    }

    /// Builds a read-input-registers request (function `0x04`).
    ///
    /// # Arguments
    ///
    /// * `start` - the address of the first input register to read.
    /// * `count` - how many registers to read.
    ///
    /// # Returns
    ///
    /// The request PDU.
    pub fn read_input_registers(start: u16, count: u16) -> Pdu {
        Self::pair(0x04, start, count)
    }

    /// Builds a write-single-coil request (function `0x05`).
    ///
    /// # Arguments
    ///
    /// * `address` - the address of the coil to write.
    /// * `on` - the value to write: `true` drives the coil on, `false` off.
    ///
    /// # Returns
    ///
    /// The request PDU.
    pub fn write_single_coil(address: u16, on: bool) -> Pdu {
        Self::pair(0x05, address, if on { 0xFF00 } else { 0x0000 })
    }

    /// Builds a write-single-register request (function `0x06`).
    ///
    /// # Arguments
    ///
    /// * `address` - the address of the holding register to write.
    /// * `value` - the 16-bit value to write.
    ///
    /// # Returns
    ///
    /// The request PDU.
    pub fn write_single_register(address: u16, value: u16) -> Pdu {
        Self::pair(0x06, address, value)
    }

    /// Builds a write-multiple-registers request (function `0x10`).
    ///
    /// # Arguments
    ///
    /// * `start` - the address of the first holding register to write.
    /// * `values` - the 16-bit values to write to consecutive registers.
    ///
    /// # Returns
    ///
    /// The request PDU.
    ///
    /// # Errors
    ///
    /// Returns [`ModbusError::InvalidValueCount`] if `values` is empty or holds more than
    /// [`MAX_WRITE_REGISTERS`](Pdu::MAX_WRITE_REGISTERS) values.
    pub fn write_multiple_registers(start: u16, values: &[u16]) -> Result<Pdu, ModbusError> {
        let quantity = values.len();
        if quantity == 0 || quantity > Self::MAX_WRITE_REGISTERS {
            return Err(ModbusError::InvalidValueCount);
        }
        let byte_count = quantity * 2;
        let mut bytes = [0u8; Self::MAX_LEN];
        bytes[0] = 0x10;
        bytes[1..3].copy_from_slice(&start.to_be_bytes());
        bytes[3..5].copy_from_slice(&(quantity as u16).to_be_bytes());
        bytes[5] = byte_count as u8;
        for (i, &value) in values.iter().enumerate() {
            bytes[6 + i * 2..8 + i * 2].copy_from_slice(&value.to_be_bytes());
        }
        Ok(Pdu {
            bytes,
            len: 6 + byte_count,
        })
    }

    /// Builds the reply a device sends to a read-holding-registers request.
    ///
    /// This is the answering half of [`read_holding_registers`](Pdu::read_holding_registers):
    /// the function code, the byte count, then the registers big-endian. It lets a client
    /// be written and tested against what a device sends without a device on the line.
    ///
    /// # Arguments
    ///
    /// * `values` - the register values the device reports, in address order.
    ///
    /// # Returns
    ///
    /// The reply PDU.
    ///
    /// # Errors
    ///
    /// Returns [`ModbusError::InvalidValueCount`] if `values` is empty or holds more than
    /// [`MAX_WRITE_REGISTERS`](Pdu::MAX_WRITE_REGISTERS) values.
    pub fn read_holding_registers_reply(values: &[u16]) -> Result<Pdu, ModbusError> {
        Self::registers_reply(0x03, values)
    }

    /// Builds the reply a device sends to a read-input-registers request.
    ///
    /// This is the answering half of [`read_input_registers`](Pdu::read_input_registers).
    ///
    /// # Arguments
    ///
    /// * `values` - the register values the device reports, in address order.
    ///
    /// # Returns
    ///
    /// The reply PDU.
    ///
    /// # Errors
    ///
    /// Returns [`ModbusError::InvalidValueCount`] if `values` is empty or holds more than
    /// [`MAX_WRITE_REGISTERS`](Pdu::MAX_WRITE_REGISTERS) values.
    pub fn read_input_registers_reply(values: &[u16]) -> Result<Pdu, ModbusError> {
        Self::registers_reply(0x04, values)
    }

    fn registers_reply(function: u8, values: &[u16]) -> Result<Pdu, ModbusError> {
        let quantity = values.len();
        if quantity == 0 || quantity > Self::MAX_WRITE_REGISTERS {
            return Err(ModbusError::InvalidValueCount);
        }
        let byte_count = quantity * 2;
        let mut bytes = [0u8; Self::MAX_LEN];
        bytes[0] = function;
        bytes[1] = byte_count as u8;
        for (i, &value) in values.iter().enumerate() {
            bytes[2 + i * 2..4 + i * 2].copy_from_slice(&value.to_be_bytes());
        }
        Ok(Pdu {
            bytes,
            len: 2 + byte_count,
        })
    }

    /// Builds a write-multiple-coils request (function `0x0F`).
    ///
    /// The coils are packed into bytes least-significant bit first, the order Modbus
    /// uses; any unused bits in the final byte are left zero.
    ///
    /// # Arguments
    ///
    /// * `start` - the address of the first coil to write.
    /// * `values` - the coil states to write, one `bool` per coil.
    ///
    /// # Returns
    ///
    /// The request PDU.
    ///
    /// # Errors
    ///
    /// Returns [`ModbusError::InvalidValueCount`] if `values` is empty or holds more than
    /// [`MAX_WRITE_COILS`](Pdu::MAX_WRITE_COILS) values.
    pub fn write_multiple_coils(start: u16, values: &[bool]) -> Result<Pdu, ModbusError> {
        let quantity = values.len();
        if quantity == 0 || quantity > Self::MAX_WRITE_COILS {
            return Err(ModbusError::InvalidValueCount);
        }
        let byte_count = quantity.div_ceil(8);
        let mut bytes = [0u8; Self::MAX_LEN];
        bytes[0] = 0x0F;
        bytes[1..3].copy_from_slice(&start.to_be_bytes());
        bytes[3..5].copy_from_slice(&(quantity as u16).to_be_bytes());
        bytes[5] = byte_count as u8;
        for (i, &on) in values.iter().enumerate() {
            if on {
                bytes[6 + i / 8] |= 1u8 << (i % 8);
            }
        }
        Ok(Pdu {
            bytes,
            len: 6 + byte_count,
        })
    }

    /// Builds a PDU from a raw function code and data, the escape hatch for function
    /// codes this crate does not name.
    ///
    /// # Arguments
    ///
    /// * `function` - the function code byte.
    /// * `data` - the bytes that follow it, used verbatim.
    ///
    /// # Returns
    ///
    /// The PDU.
    ///
    /// # Errors
    ///
    /// Returns [`ModbusError::FrameTooLong`] if the function code plus `data` would not
    /// fit a PDU (more than [`MAX_LEN`](Pdu::MAX_LEN) bytes).
    pub fn raw(function: u8, data: &[u8]) -> Result<Pdu, ModbusError> {
        let len = 1 + data.len();
        if len > Self::MAX_LEN {
            return Err(ModbusError::FrameTooLong);
        }
        let mut bytes = [0u8; Self::MAX_LEN];
        bytes[0] = function;
        bytes[1..len].copy_from_slice(data);
        Ok(Pdu { bytes, len })
    }

    /// Returns the function code, the first byte of the PDU.
    ///
    /// # Returns
    ///
    /// The function code.
    pub fn function_code(&self) -> u8 {
        self.bytes[0]
    }

    /// Returns the PDU bytes: the function code followed by its data.
    ///
    /// # Returns
    ///
    /// The PDU as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Wraps this PDU into an RTU frame addressed to a unit, appending the CRC.
    ///
    /// # Arguments
    ///
    /// * `address` - the unit (slave) address the frame is for.
    ///
    /// # Returns
    ///
    /// The [`Adu`] ready to send.
    pub fn to_adu(&self, address: u8) -> Adu {
        Adu::assemble(address, self.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_holding_registers_matches_the_spec_example() {
        let pdu = Pdu::read_holding_registers(0x006B, 3);
        assert_eq!(pdu.as_bytes(), &[0x03, 0x00, 0x6B, 0x00, 0x03]);
        assert_eq!(pdu.function_code(), 0x03);
    }

    #[test]
    fn write_single_coil_encodes_on_and_off() {
        assert_eq!(
            Pdu::write_single_coil(0x00AC, true).as_bytes(),
            &[0x05, 0x00, 0xAC, 0xFF, 0x00]
        );
        assert_eq!(
            Pdu::write_single_coil(0x00AC, false).as_bytes(),
            &[0x05, 0x00, 0xAC, 0x00, 0x00]
        );
    }

    #[test]
    fn write_single_register_matches_the_spec_example() {
        assert_eq!(
            Pdu::write_single_register(0x0001, 0x0003).as_bytes(),
            &[0x06, 0x00, 0x01, 0x00, 0x03]
        );
    }

    #[test]
    fn write_multiple_registers_matches_the_spec_example() {
        let pdu = Pdu::write_multiple_registers(0x0001, &[0x000A, 0x0102]).unwrap();
        assert_eq!(
            pdu.as_bytes(),
            &[0x10, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00, 0x0A, 0x01, 0x02]
        );
    }

    #[test]
    fn write_multiple_coils_packs_bits_lsb_first() {
        // The spec's ten-coil example packs to 0xCD, 0x01.
        let values = [
            true, false, true, true, false, false, true, true, true, false,
        ];
        let pdu = Pdu::write_multiple_coils(0x0013, &values).unwrap();
        assert_eq!(
            pdu.as_bytes(),
            &[0x0F, 0x00, 0x13, 0x00, 0x0A, 0x02, 0xCD, 0x01]
        );
    }

    #[test]
    fn an_empty_write_is_rejected() {
        assert_eq!(
            Pdu::write_multiple_registers(0, &[]),
            Err(ModbusError::InvalidValueCount)
        );
        assert_eq!(
            Pdu::write_multiple_coils(0, &[]),
            Err(ModbusError::InvalidValueCount)
        );
    }

    #[test]
    fn an_oversized_write_is_rejected() {
        let registers = [0u16; Pdu::MAX_WRITE_REGISTERS + 1];
        assert_eq!(
            Pdu::write_multiple_registers(0, &registers),
            Err(ModbusError::InvalidValueCount)
        );
        let coils = [false; Pdu::MAX_WRITE_COILS + 1];
        assert_eq!(
            Pdu::write_multiple_coils(0, &coils),
            Err(ModbusError::InvalidValueCount)
        );
    }

    #[test]
    fn the_largest_write_still_fits_a_pdu() {
        let registers = [0u16; Pdu::MAX_WRITE_REGISTERS];
        let pdu = Pdu::write_multiple_registers(0, &registers).unwrap();
        assert!(pdu.as_bytes().len() <= Pdu::MAX_LEN);
    }

    #[test]
    fn raw_carries_arbitrary_bytes() {
        let pdu = Pdu::raw(0x2B, &[0x0E, 0x01, 0x00]).unwrap();
        assert_eq!(pdu.as_bytes(), &[0x2B, 0x0E, 0x01, 0x00]);
    }

    #[test]
    fn raw_rejects_an_oversized_pdu() {
        let data = [0u8; Pdu::MAX_LEN];
        assert_eq!(Pdu::raw(0x10, &data), Err(ModbusError::FrameTooLong));
    }

    #[test]
    fn to_adu_appends_the_address_and_crc() {
        let frame = Pdu::read_holding_registers(0x006B, 3).to_adu(0x11);
        assert_eq!(
            frame.as_bytes(),
            &[0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87]
        );
    }
    #[test]
    fn a_read_registers_reply_is_the_frame_the_specification_shows() {
        // The specification's worked example: three holding registers answered as the
        // function code, a byte count of six, then the values big-endian.
        let reply = Pdu::read_holding_registers_reply(&[0x022B, 0x0000, 0x0064])
            .expect("three registers fit a reply");
        assert_eq!(
            reply.as_bytes(),
            &[0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64]
        );

        let inputs = Pdu::read_input_registers_reply(&[1]).expect("one register fits");
        assert_eq!(inputs.as_bytes(), &[0x04, 0x02, 0x00, 0x01]);
    }

    #[test]
    fn a_reply_refuses_a_register_count_it_cannot_carry() {
        assert_eq!(
            Pdu::read_holding_registers_reply(&[]),
            Err(ModbusError::InvalidValueCount)
        );
        let too_many = [0u16; Pdu::MAX_WRITE_REGISTERS + 1];
        assert_eq!(
            Pdu::read_holding_registers_reply(&too_many),
            Err(ModbusError::InvalidValueCount)
        );
    }
}
