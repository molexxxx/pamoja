//! Modbus function codes and the exception codes a device returns when it refuses.

/// A Modbus function code, naming the operation a request asks for.
///
/// This enum covers the function codes that read and write the four Modbus data tables
/// (coils, discrete inputs, holding registers, input registers), which is what the great
/// majority of field devices use. A function code outside this set still travels fine
/// through [`Pdu::raw`](crate::Pdu::raw) and [`Adu`](crate::Adu); this enum is the typed
/// view of the common ones, not a limit on what the framing carries.
///
/// # Examples
///
/// ```
/// use pamoja_modbus::Function;
///
/// assert_eq!(Function::ReadHoldingRegisters.code(), 0x03);
/// assert_eq!(Function::from_code(0x10), Some(Function::WriteMultipleRegisters));
/// assert_eq!(Function::from_code(0x99), None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Function {
    /// Read one or more coils (read/write bits). Function code `0x01`.
    ReadCoils,
    /// Read one or more discrete inputs (read-only bits). Function code `0x02`.
    ReadDiscreteInputs,
    /// Read one or more holding registers (read/write 16-bit words). Function code `0x03`.
    ReadHoldingRegisters,
    /// Read one or more input registers (read-only 16-bit words). Function code `0x04`.
    ReadInputRegisters,
    /// Write a single coil. Function code `0x05`.
    WriteSingleCoil,
    /// Write a single holding register. Function code `0x06`.
    WriteSingleRegister,
    /// Write a contiguous block of coils. Function code `0x0F`.
    WriteMultipleCoils,
    /// Write a contiguous block of holding registers. Function code `0x10`.
    WriteMultipleRegisters,
}

impl Function {
    /// Returns the wire byte for this function.
    ///
    /// # Returns
    ///
    /// The function code as it appears as the first byte of a PDU.
    pub fn code(self) -> u8 {
        match self {
            Function::ReadCoils => 0x01,
            Function::ReadDiscreteInputs => 0x02,
            Function::ReadHoldingRegisters => 0x03,
            Function::ReadInputRegisters => 0x04,
            Function::WriteSingleCoil => 0x05,
            Function::WriteSingleRegister => 0x06,
            Function::WriteMultipleCoils => 0x0F,
            Function::WriteMultipleRegisters => 0x10,
        }
    }

    /// Returns the function a wire byte names, if this crate models it.
    ///
    /// # Arguments
    ///
    /// * `code` - the function code byte from the start of a PDU.
    ///
    /// # Returns
    ///
    /// The matching [`Function`], or [`None`] for a code this enum does not name
    /// (including the exception responses, whose high bit is set).
    pub fn from_code(code: u8) -> Option<Function> {
        match code {
            0x01 => Some(Function::ReadCoils),
            0x02 => Some(Function::ReadDiscreteInputs),
            0x03 => Some(Function::ReadHoldingRegisters),
            0x04 => Some(Function::ReadInputRegisters),
            0x05 => Some(Function::WriteSingleCoil),
            0x06 => Some(Function::WriteSingleRegister),
            0x0F => Some(Function::WriteMultipleCoils),
            0x10 => Some(Function::WriteMultipleRegisters),
            _ => None,
        }
    }
}

/// A Modbus exception code: the reason a device gives for refusing a request.
///
/// A device that cannot serve a request replies with the request's function code with
/// its high bit set, followed by one of these codes. [`Adu::exception`](crate::Adu::exception)
/// and [`Response::exception`](crate::Response::exception) surface it.
///
/// # Examples
///
/// ```
/// use pamoja_modbus::Exception;
///
/// assert_eq!(Exception::IllegalDataAddress.code(), 0x02);
/// assert_eq!(Exception::from_code(0x01), Some(Exception::IllegalFunction));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exception {
    /// The function code is not allowed for this device. Exception code `0x01`.
    IllegalFunction,
    /// The data address is not allowed for this device. Exception code `0x02`.
    IllegalDataAddress,
    /// A value in the request is not allowed for this device. Exception code `0x03`.
    IllegalDataValue,
    /// The device failed while serving the request. Exception code `0x04`.
    ServerDeviceFailure,
    /// The device accepted a long-running request and is still processing it. Exception code `0x05`.
    Acknowledge,
    /// The device is busy with a long-running request; retry later. Exception code `0x06`.
    ServerDeviceBusy,
    /// The device detected a parity error in its memory. Exception code `0x08`.
    MemoryParityError,
    /// A gateway could not route the request to the target path. Exception code `0x0A`.
    GatewayPathUnavailable,
    /// A gateway got no response from the target device, which usually means the device is
    /// not on the network. Exception code `0x0B`.
    GatewayTargetFailedToRespond,
}

impl Exception {
    /// Returns the wire byte for this exception.
    ///
    /// # Returns
    ///
    /// The exception code as it appears after the function code in an exception response.
    pub fn code(self) -> u8 {
        match self {
            Exception::IllegalFunction => 0x01,
            Exception::IllegalDataAddress => 0x02,
            Exception::IllegalDataValue => 0x03,
            Exception::ServerDeviceFailure => 0x04,
            Exception::Acknowledge => 0x05,
            Exception::ServerDeviceBusy => 0x06,
            Exception::MemoryParityError => 0x08,
            Exception::GatewayPathUnavailable => 0x0A,
            Exception::GatewayTargetFailedToRespond => 0x0B,
        }
    }

    /// Returns the exception a wire byte names, if it is a defined code.
    ///
    /// # Arguments
    ///
    /// * `code` - the exception code byte following the function code.
    ///
    /// # Returns
    ///
    /// The matching [`Exception`], or [`None`] for a code this enum does not name.
    pub fn from_code(code: u8) -> Option<Exception> {
        match code {
            0x01 => Some(Exception::IllegalFunction),
            0x02 => Some(Exception::IllegalDataAddress),
            0x03 => Some(Exception::IllegalDataValue),
            0x04 => Some(Exception::ServerDeviceFailure),
            0x05 => Some(Exception::Acknowledge),
            0x06 => Some(Exception::ServerDeviceBusy),
            0x08 => Some(Exception::MemoryParityError),
            0x0A => Some(Exception::GatewayPathUnavailable),
            0x0B => Some(Exception::GatewayTargetFailedToRespond),
            _ => None,
        }
    }
}

/// Writes the exception's name as the application protocol specification gives it, in lower
/// case: `illegal data address` for `0x02`.
impl core::fmt::Display for Exception {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Exception::IllegalFunction => "illegal function",
            Exception::IllegalDataAddress => "illegal data address",
            Exception::IllegalDataValue => "illegal data value",
            Exception::ServerDeviceFailure => "server device failure",
            Exception::Acknowledge => "acknowledge",
            Exception::ServerDeviceBusy => "server device busy",
            Exception::MemoryParityError => "memory parity error",
            Exception::GatewayPathUnavailable => "gateway path unavailable",
            Exception::GatewayTargetFailedToRespond => "gateway target device failed to respond",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_codes_round_trip() {
        for function in [
            Function::ReadCoils,
            Function::ReadDiscreteInputs,
            Function::ReadHoldingRegisters,
            Function::ReadInputRegisters,
            Function::WriteSingleCoil,
            Function::WriteSingleRegister,
            Function::WriteMultipleCoils,
            Function::WriteMultipleRegisters,
        ] {
            assert_eq!(Function::from_code(function.code()), Some(function));
        }
    }

    #[test]
    fn an_exception_response_byte_is_not_a_function() {
        // 0x83 is read-holding-registers (0x03) with the exception bit set.
        assert_eq!(Function::from_code(0x83), None);
    }

    #[test]
    fn exception_codes_round_trip() {
        for exception in [
            Exception::IllegalFunction,
            Exception::IllegalDataAddress,
            Exception::IllegalDataValue,
            Exception::ServerDeviceFailure,
            Exception::Acknowledge,
            Exception::ServerDeviceBusy,
            Exception::MemoryParityError,
            Exception::GatewayPathUnavailable,
            Exception::GatewayTargetFailedToRespond,
        ] {
            assert_eq!(Exception::from_code(exception.code()), Some(exception));
        }
    }

    #[test]
    fn an_undefined_exception_code_is_none() {
        assert_eq!(Exception::from_code(0x07), None);
    }
}
