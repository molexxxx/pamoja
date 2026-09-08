//! Reading values back out of a Modbus response PDU.

use crate::error::ModbusError;
use crate::function::Exception;

/// A borrowed view over a response PDU, for reading the values a device returned.
///
/// A read response is a function code, a byte count, and then the data. [`registers`](Response::registers)
/// and [`coils`](Response::coils) decode that data into the 16-bit words or the packed
/// bits it represents; [`exception`](Response::exception) recognizes the alternative, a
/// device that refused the request. The view borrows the PDU and copies nothing.
///
/// # Examples
///
/// ```
/// use pamoja_modbus::Response;
///
/// // A read-holding-registers reply: function 0x03, byte count 6, three registers.
/// let pdu = [0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64];
/// let values: Vec<u16> = Response::new(&pdu).registers().unwrap().collect();
/// assert_eq!(values, [0x022B, 0x0000, 0x0064]);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Response<'a> {
    pdu: &'a [u8],
}

impl<'a> Response<'a> {
    /// Wraps a response PDU for reading.
    ///
    /// # Arguments
    ///
    /// * `pdu` - the response PDU, a function code followed by its data.
    ///
    /// # Returns
    ///
    /// The response view.
    pub fn new(pdu: &'a [u8]) -> Self {
        Response { pdu }
    }

    /// Returns the function code, the first byte of the PDU.
    ///
    /// # Returns
    ///
    /// The function code, or `0` if the PDU is empty.
    pub fn function_code(&self) -> u8 {
        self.pdu.first().copied().unwrap_or(0)
    }

    /// Returns the exception a device reported, if this is an exception response.
    ///
    /// # Returns
    ///
    /// The [`Exception`] if the function code's high bit is set and a defined exception
    /// byte follows it, otherwise [`None`].
    pub fn exception(&self) -> Option<Exception> {
        if self.function_code() & 0x80 == 0 {
            return None;
        }
        self.pdu.get(1).and_then(|&code| Exception::from_code(code))
    }

    // The data carried after the function code and byte-count header, validated so its
    // length matches the declared byte count.
    fn payload(&self) -> Result<&'a [u8], ModbusError> {
        if self.pdu.len() < 2 {
            return Err(ModbusError::MalformedResponse);
        }
        let byte_count = usize::from(self.pdu[1]);
        let data = &self.pdu[2..];
        if data.len() != byte_count {
            return Err(ModbusError::MalformedResponse);
        }
        Ok(data)
    }

    /// Reads the 16-bit registers from a read-registers response.
    ///
    /// # Returns
    ///
    /// An iterator over the registers in order, each decoded from its big-endian pair.
    ///
    /// # Errors
    ///
    /// Returns [`ModbusError::MalformedResponse`] if the PDU is truncated, its declared
    /// byte count does not match its data, or that data is not a whole number of registers.
    pub fn registers(&self) -> Result<Registers<'a>, ModbusError> {
        let data = self.payload()?;
        if data.len() % 2 != 0 {
            return Err(ModbusError::MalformedResponse);
        }
        Ok(Registers { data })
    }

    /// Reads the coils or discrete inputs from a read-bits response.
    ///
    /// The response packs the bits least-significant first; this unpacks exactly `count`
    /// of them and ignores the padding in the final byte.
    ///
    /// # Arguments
    ///
    /// * `count` - how many bits to read, the quantity the request asked for.
    ///
    /// # Returns
    ///
    /// An iterator over `count` bits in order.
    ///
    /// # Errors
    ///
    /// Returns [`ModbusError::MalformedResponse`] if the PDU is truncated or its declared
    /// byte count does not match the data or the requested `count`.
    pub fn coils(&self, count: u16) -> Result<Coils<'a>, ModbusError> {
        let data = self.payload()?;
        if data.len() != usize::from(count).div_ceil(8) {
            return Err(ModbusError::MalformedResponse);
        }
        Ok(Coils {
            data,
            index: 0,
            remaining: usize::from(count),
        })
    }
}

/// An iterator over the 16-bit registers of a read-registers response.
#[derive(Clone, Copy, Debug)]
pub struct Registers<'a> {
    data: &'a [u8],
}

impl Iterator for Registers<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        if self.data.len() < 2 {
            return None;
        }
        let value = u16::from_be_bytes([self.data[0], self.data[1]]);
        self.data = &self.data[2..];
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.data.len() / 2;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Registers<'_> {}

/// An iterator over the bits of a read-coils or read-discrete-inputs response.
#[derive(Clone, Copy, Debug)]
pub struct Coils<'a> {
    data: &'a [u8],
    index: usize,
    remaining: usize,
}

impl Iterator for Coils<'_> {
    type Item = bool;

    fn next(&mut self) -> Option<bool> {
        if self.remaining == 0 {
            return None;
        }
        let bit = (self.data[self.index / 8] >> (self.index % 8)) & 1;
        self.index += 1;
        self.remaining -= 1;
        Some(bit != 0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for Coils<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_decode_in_order() {
        let pdu = [0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64];
        let values: [u16; 3] = {
            let mut it = Response::new(&pdu).registers().unwrap();
            [it.next().unwrap(), it.next().unwrap(), it.next().unwrap()]
        };
        assert_eq!(values, [0x022B, 0x0000, 0x0064]);
    }

    #[test]
    fn registers_report_an_exact_length() {
        let pdu = [0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64];
        assert_eq!(Response::new(&pdu).registers().unwrap().len(), 3);
    }

    #[test]
    fn registers_reject_a_byte_count_mismatch() {
        // Byte count says six, but only two data bytes follow.
        let pdu = [0x03, 0x06, 0x02, 0x2B];
        assert_eq!(
            Response::new(&pdu).registers().err(),
            Some(ModbusError::MalformedResponse)
        );
    }

    #[test]
    fn coils_unpack_lsb_first_and_drop_padding() {
        // Byte count one, data 0x05 is bits 1, 0, 1 in the low three positions.
        let pdu = [0x01, 0x01, 0x05];
        let bits: [bool; 3] = {
            let mut it = Response::new(&pdu).coils(3).unwrap();
            [it.next().unwrap(), it.next().unwrap(), it.next().unwrap()]
        };
        assert_eq!(bits, [true, false, true]);
    }

    #[test]
    fn coils_reject_a_count_that_does_not_match_the_byte_count() {
        let pdu = [0x01, 0x01, 0x05];
        // Nine coils need two bytes, but only one is present.
        assert_eq!(
            Response::new(&pdu).coils(9).err(),
            Some(ModbusError::MalformedResponse)
        );
    }

    #[test]
    fn an_exception_response_reads_as_an_exception() {
        let pdu = [0x83, 0x02];
        assert_eq!(
            Response::new(&pdu).exception(),
            Some(Exception::IllegalDataAddress)
        );
    }

    #[test]
    fn a_normal_response_has_no_exception() {
        let pdu = [0x03, 0x02, 0x00, 0x64];
        assert_eq!(Response::new(&pdu).exception(), None);
    }
}
