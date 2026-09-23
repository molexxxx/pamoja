//! The Modbus RTU application data unit: the frame that goes on the wire.

use crate::crc::crc16;
use crate::error::ModbusError;
use crate::function::Exception;
use crate::response::Response;

/// A Modbus RTU frame: a unit address, a PDU, and a trailing CRC.
///
/// This is the complete unit of bytes an RTU transmitter puts on the bus and a receiver
/// pulls off it. [`from_pdu`](Adu::from_pdu) builds one to send by appending the CRC;
/// [`parse`](Adu::parse) reads one received, verifying the CRC so a frame corrupted in
/// transit never reaches the application. The frame lives in a fixed buffer, so neither
/// path allocates.
///
/// # Examples
///
/// ```
/// use pamoja_modbus::{Adu, Function, Pdu};
///
/// // A read of three holding registers, addressed to unit 17.
/// let pdu = Pdu::read_holding_registers(107, 3);
/// let frame = Adu::from_pdu(17, pdu.as_bytes()).unwrap();
/// assert_eq!(frame.address(), 17);
/// assert_eq!(frame.function_code(), Function::ReadHoldingRegisters.code());
///
/// // A receiver checks the same bytes against the CRC they carry, and finds the PDU.
/// let received = Adu::parse(frame.as_bytes()).unwrap();
/// assert_eq!(received.pdu(), pdu.as_bytes());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Adu {
    bytes: [u8; Adu::MAX_LEN],
    len: usize,
}

impl Adu {
    /// The largest a Modbus RTU frame may be, in bytes.
    pub const MAX_LEN: usize = 256;

    // The smallest valid frame: address, function code, and the two CRC bytes.
    const MIN_LEN: usize = 4;

    // Lays an address, a PDU, and the CRC into a frame. The caller guarantees the PDU
    // fits, which every internal caller does by construction.
    pub(crate) fn assemble(address: u8, pdu: &[u8]) -> Adu {
        let len = 1 + pdu.len() + 2;
        let mut bytes = [0u8; Self::MAX_LEN];
        bytes[0] = address;
        bytes[1..1 + pdu.len()].copy_from_slice(pdu);
        let crc = crc16(&bytes[..1 + pdu.len()]);
        bytes[1 + pdu.len()..len].copy_from_slice(&crc.to_le_bytes());
        Adu { bytes, len }
    }

    /// Builds a frame for a unit address and PDU, appending the CRC.
    ///
    /// # Arguments
    ///
    /// * `address` - the unit (slave) address the frame is for.
    /// * `pdu` - the protocol data unit: a function code followed by its data.
    ///
    /// # Returns
    ///
    /// The frame ready to send.
    ///
    /// # Errors
    ///
    /// Returns [`ModbusError::FrameTooLong`] if `pdu` is longer than a PDU may be, so the
    /// frame would exceed [`MAX_LEN`](Adu::MAX_LEN) bytes.
    pub fn from_pdu(address: u8, pdu: &[u8]) -> Result<Adu, ModbusError> {
        if 1 + pdu.len() + 2 > Self::MAX_LEN {
            return Err(ModbusError::FrameTooLong);
        }
        Ok(Self::assemble(address, pdu))
    }

    /// Parses a received frame, verifying its CRC.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the raw frame as it came off the wire, CRC included.
    ///
    /// # Returns
    ///
    /// The validated frame.
    ///
    /// # Errors
    ///
    /// Returns [`ModbusError::FrameTooShort`] if `bytes` is shorter than a valid frame,
    /// [`ModbusError::FrameTooLong`] if it is longer than [`MAX_LEN`](Adu::MAX_LEN), or
    /// [`ModbusError::CrcMismatch`] if the trailing CRC does not match the contents.
    pub fn parse(bytes: &[u8]) -> Result<Adu, ModbusError> {
        if bytes.len() < Self::MIN_LEN {
            return Err(ModbusError::FrameTooShort);
        }
        if bytes.len() > Self::MAX_LEN {
            return Err(ModbusError::FrameTooLong);
        }
        let split = bytes.len() - 2;
        let expected = crc16(&bytes[..split]);
        let found = u16::from_le_bytes([bytes[split], bytes[split + 1]]);
        if expected != found {
            return Err(ModbusError::CrcMismatch { expected, found });
        }
        let mut buffer = [0u8; Self::MAX_LEN];
        buffer[..bytes.len()].copy_from_slice(bytes);
        Ok(Adu {
            bytes: buffer,
            len: bytes.len(),
        })
    }

    /// Returns the unit address, the first byte of the frame.
    ///
    /// # Returns
    ///
    /// The unit (slave) address.
    pub fn address(&self) -> u8 {
        self.bytes[0]
    }

    /// Returns the function code, the first byte of the PDU.
    ///
    /// # Returns
    ///
    /// The function code. An exception response has its high bit set.
    pub fn function_code(&self) -> u8 {
        self.bytes[1]
    }

    /// Returns the PDU: the frame without its address and CRC.
    ///
    /// # Returns
    ///
    /// The protocol data unit as a byte slice.
    pub fn pdu(&self) -> &[u8] {
        &self.bytes[1..self.len - 2]
    }

    /// Returns the whole frame, CRC included, ready for the wire.
    ///
    /// # Returns
    ///
    /// The frame as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns the exception a device reported, if this frame is an exception response.
    ///
    /// # Returns
    ///
    /// The [`Exception`] if the function code's high bit is set and an exception byte
    /// follows it, otherwise [`None`] (including for a defined-but-unknown exception code).
    pub fn exception(&self) -> Option<Exception> {
        self.response().exception()
    }

    /// Returns a reader over this frame's PDU for decoding a response.
    ///
    /// # Returns
    ///
    /// A [`Response`] borrowing the PDU.
    pub fn response(&self) -> Response<'_> {
        Response::new(self.pdu())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_pdu_then_parse_round_trips() {
        let frame = Adu::from_pdu(0x11, &[0x03, 0x00, 0x6B, 0x00, 0x03]).unwrap();
        let parsed = Adu::parse(frame.as_bytes()).unwrap();
        assert_eq!(parsed.address(), 0x11);
        assert_eq!(parsed.function_code(), 0x03);
        assert_eq!(parsed.pdu(), &[0x03, 0x00, 0x6B, 0x00, 0x03]);
    }

    #[test]
    fn parse_accepts_the_spec_request_frame() {
        let parsed = Adu::parse(&[0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87]).unwrap();
        assert_eq!(parsed.address(), 0x11);
        assert_eq!(parsed.pdu(), &[0x03, 0x00, 0x6B, 0x00, 0x03]);
    }

    #[test]
    fn parse_rejects_a_corrupt_crc() {
        let result = Adu::parse(&[0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x00, 0x00]);
        assert_eq!(
            result,
            Err(ModbusError::CrcMismatch {
                expected: 0x8776,
                found: 0x0000
            })
        );
    }

    #[test]
    fn parse_rejects_a_short_frame() {
        assert_eq!(
            Adu::parse(&[0x11, 0x03, 0x76]),
            Err(ModbusError::FrameTooShort)
        );
    }

    #[test]
    fn parse_rejects_an_oversized_frame() {
        let frame = [0u8; Adu::MAX_LEN + 1];
        assert_eq!(Adu::parse(&frame), Err(ModbusError::FrameTooLong));
    }

    #[test]
    fn an_exception_response_surfaces_its_code() {
        // Read holding registers (0x03) refused with illegal data address (0x02).
        let frame = Adu::from_pdu(0x11, &[0x83, 0x02]).unwrap();
        let parsed = Adu::parse(frame.as_bytes()).unwrap();
        assert_eq!(parsed.exception(), Some(Exception::IllegalDataAddress));
    }

    #[test]
    fn a_normal_response_has_no_exception() {
        let frame = Adu::from_pdu(0x11, &[0x03, 0x02, 0x00, 0x64]).unwrap();
        assert_eq!(frame.exception(), None);
    }

    #[test]
    fn a_maximum_length_frame_round_trips() {
        // The largest PDU is 253 bytes; with the address and CRC that is the 256-byte
        // maximum RTU frame.
        let pdu = [0xAB; 253];
        let frame = Adu::from_pdu(0x01, &pdu).unwrap();
        assert_eq!(frame.as_bytes().len(), Adu::MAX_LEN);
        let parsed = Adu::parse(frame.as_bytes()).unwrap();
        assert_eq!(parsed.pdu(), &pdu[..]);
    }

    #[test]
    fn an_exception_bit_without_a_code_is_not_an_exception() {
        // A reply whose function code has the exception bit set but carries no exception
        // byte must read as no exception rather than panic.
        let frame = Adu::from_pdu(0x11, &[0x83]).unwrap();
        let parsed = Adu::parse(frame.as_bytes()).unwrap();
        assert_eq!(parsed.exception(), None);
    }
}
