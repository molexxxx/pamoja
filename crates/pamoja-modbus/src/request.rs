//! Reading a request the way a device does.

use crate::function::{Exception, Function};
use crate::pdu::Pdu;
use crate::response::{Coils, Registers};

/// A request as a device reads it, checked against the limits the specification sets.
///
/// [`parse`](Request::parse) follows the order of the specification's state diagrams: a
/// function the device does not serve is refused with
/// [`IllegalFunction`](Exception::IllegalFunction), and a quantity, a byte count, or a coil
/// value outside what the function allows with [`IllegalDataValue`](Exception::IllegalDataValue).
/// Whether the addresses exist is the device's own question, answered with
/// [`IllegalDataAddress`](Exception::IllegalDataAddress) once the request has parsed.
///
/// # Examples
///
/// ```
/// use pamoja_modbus::{Exception, Pdu, Request};
///
/// let pdu = Pdu::read_holding_registers(0x006B, 3);
/// assert_eq!(
///     Request::parse(pdu.as_bytes()),
///     Ok(Request::ReadHoldingRegisters { start: 0x006B, quantity: 3 })
/// );
///
/// // 126 registers is one more than a reply can carry.
/// let greedy = Pdu::read_holding_registers(0, 126);
/// assert_eq!(Request::parse(greedy.as_bytes()), Err(Exception::IllegalDataValue));
/// ```
#[derive(Clone, Copy, Debug)]
pub enum Request<'a> {
    /// Read coils, function `0x01`.
    ReadCoils {
        /// The first coil's address.
        start: u16,
        /// How many coils, 1 to 2000.
        quantity: u16,
    },
    /// Read discrete inputs, function `0x02`.
    ReadDiscreteInputs {
        /// The first input's address.
        start: u16,
        /// How many inputs, 1 to 2000.
        quantity: u16,
    },
    /// Read holding registers, function `0x03`.
    ReadHoldingRegisters {
        /// The first register's address.
        start: u16,
        /// How many registers, 1 to 125.
        quantity: u16,
    },
    /// Read input registers, function `0x04`.
    ReadInputRegisters {
        /// The first register's address.
        start: u16,
        /// How many registers, 1 to 125.
        quantity: u16,
    },
    /// Write one coil, function `0x05`.
    WriteSingleCoil {
        /// The coil's address.
        address: u16,
        /// The state to write.
        on: bool,
    },
    /// Write one holding register, function `0x06`.
    WriteSingleRegister {
        /// The register's address.
        address: u16,
        /// The value to write.
        value: u16,
    },
    /// Write a run of coils, function `0x0F`.
    WriteMultipleCoils {
        /// The first coil's address.
        start: u16,
        /// The states to write, 1 to 1968 of them, in address order.
        values: Coils<'a>,
    },
    /// Write a run of holding registers, function `0x10`.
    WriteMultipleRegisters {
        /// The first register's address.
        start: u16,
        /// The values to write, 1 to 123 of them, in address order.
        values: Registers<'a>,
    },
}

impl PartialEq for Request<'_> {
    fn eq(&self, other: &Self) -> bool {
        use Request::*;
        match (self, other) {
            (
                ReadCoils {
                    start: a,
                    quantity: b,
                },
                ReadCoils {
                    start: c,
                    quantity: d,
                },
            )
            | (
                ReadDiscreteInputs {
                    start: a,
                    quantity: b,
                },
                ReadDiscreteInputs {
                    start: c,
                    quantity: d,
                },
            )
            | (
                ReadHoldingRegisters {
                    start: a,
                    quantity: b,
                },
                ReadHoldingRegisters {
                    start: c,
                    quantity: d,
                },
            )
            | (
                ReadInputRegisters {
                    start: a,
                    quantity: b,
                },
                ReadInputRegisters {
                    start: c,
                    quantity: d,
                },
            ) => a == c && b == d,
            (WriteSingleCoil { address: a, on: b }, WriteSingleCoil { address: c, on: d }) => {
                a == c && b == d
            }
            (
                WriteSingleRegister {
                    address: a,
                    value: b,
                },
                WriteSingleRegister {
                    address: c,
                    value: d,
                },
            ) => a == c && b == d,
            (
                WriteMultipleCoils {
                    start: a,
                    values: b,
                },
                WriteMultipleCoils {
                    start: c,
                    values: d,
                },
            ) => a == c && b.eq(*d),
            (
                WriteMultipleRegisters {
                    start: a,
                    values: b,
                },
                WriteMultipleRegisters {
                    start: c,
                    values: d,
                },
            ) => a == c && b.eq(*d),
            _ => false,
        }
    }
}

fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}

impl<'a> Request<'a> {
    /// Reads a request PDU, checking it the way the specification's state diagrams do.
    ///
    /// # Arguments
    ///
    /// * `pdu` - the request: a function code followed by its data.
    ///
    /// # Returns
    ///
    /// The request.
    ///
    /// # Errors
    ///
    /// [`Exception::IllegalFunction`] for an empty PDU or a function this type does not
    /// name, and [`Exception::IllegalDataValue`] for a PDU too short for its function, a
    /// quantity outside the function's range, a byte count that does not match the
    /// quantity, or a single coil written with anything but `0x0000` or `0xFF00`.
    pub fn parse(pdu: &'a [u8]) -> Result<Request<'a>, Exception> {
        let (&code, data) = pdu.split_first().ok_or(Exception::IllegalFunction)?;
        let function = Function::from_code(code).ok_or(Exception::IllegalFunction)?;
        if data.len() < 4 {
            return Err(Exception::IllegalDataValue);
        }
        let first = word(data, 0);
        let second = word(data, 2);
        let within = |quantity: u16, most: usize| {
            if quantity == 0 || usize::from(quantity) > most {
                Err(Exception::IllegalDataValue)
            } else {
                Ok(quantity)
            }
        };
        let fixed = |request: Request<'a>| {
            if data.len() == 4 {
                Ok(request)
            } else {
                Err(Exception::IllegalDataValue)
            }
        };
        match function {
            Function::ReadCoils => fixed(Request::ReadCoils {
                start: first,
                quantity: within(second, Pdu::MAX_READ_BITS)?,
            }),
            Function::ReadDiscreteInputs => fixed(Request::ReadDiscreteInputs {
                start: first,
                quantity: within(second, Pdu::MAX_READ_BITS)?,
            }),
            Function::ReadHoldingRegisters => fixed(Request::ReadHoldingRegisters {
                start: first,
                quantity: within(second, Pdu::MAX_READ_REGISTERS)?,
            }),
            Function::ReadInputRegisters => fixed(Request::ReadInputRegisters {
                start: first,
                quantity: within(second, Pdu::MAX_READ_REGISTERS)?,
            }),
            Function::WriteSingleCoil => {
                let on = match second {
                    0xFF00 => true,
                    0x0000 => false,
                    _ => return Err(Exception::IllegalDataValue),
                };
                fixed(Request::WriteSingleCoil { address: first, on })
            }
            Function::WriteSingleRegister => fixed(Request::WriteSingleRegister {
                address: first,
                value: second,
            }),
            Function::WriteMultipleCoils => {
                let quantity = within(second, Pdu::MAX_WRITE_COILS)?;
                let bytes = Self::counted(data, usize::from(quantity).div_ceil(8))?;
                Ok(Request::WriteMultipleCoils {
                    start: first,
                    values: Coils::over(bytes, usize::from(quantity)),
                })
            }
            Function::WriteMultipleRegisters => {
                let quantity = within(second, Pdu::MAX_WRITE_REGISTERS)?;
                let bytes = Self::counted(data, usize::from(quantity) * 2)?;
                Ok(Request::WriteMultipleRegisters {
                    start: first,
                    values: Registers::over(bytes),
                })
            }
        }
    }

    // The bytes after a byte count, which has to be what the quantity needs and match what
    // follows it.
    fn counted(data: &'a [u8], needed: usize) -> Result<&'a [u8], Exception> {
        match data.get(4) {
            Some(&count) if usize::from(count) == needed && data.len() == 5 + needed => {
                Ok(&data[5..])
            }
            _ => Err(Exception::IllegalDataValue),
        }
    }

    /// Returns the function the request asks for.
    pub fn function(&self) -> Function {
        match self {
            Request::ReadCoils { .. } => Function::ReadCoils,
            Request::ReadDiscreteInputs { .. } => Function::ReadDiscreteInputs,
            Request::ReadHoldingRegisters { .. } => Function::ReadHoldingRegisters,
            Request::ReadInputRegisters { .. } => Function::ReadInputRegisters,
            Request::WriteSingleCoil { .. } => Function::WriteSingleCoil,
            Request::WriteSingleRegister { .. } => Function::WriteSingleRegister,
            Request::WriteMultipleCoils { .. } => Function::WriteMultipleCoils,
            Request::WriteMultipleRegisters { .. } => Function::WriteMultipleRegisters,
        }
    }

    /// Returns the addresses the request reads or writes, as its first address and how many
    /// follow, which a device checks against the addresses it has.
    ///
    /// # Returns
    ///
    /// The first address and the count.
    pub fn span(&self) -> (u16, usize) {
        match *self {
            Request::ReadCoils { start, quantity }
            | Request::ReadDiscreteInputs { start, quantity }
            | Request::ReadHoldingRegisters { start, quantity }
            | Request::ReadInputRegisters { start, quantity } => (start, usize::from(quantity)),
            Request::WriteSingleCoil { address, .. }
            | Request::WriteSingleRegister { address, .. } => (address, 1),
            Request::WriteMultipleCoils { start, values } => (start, values.len()),
            Request::WriteMultipleRegisters { start, values } => (start, values.len()),
        }
    }

    /// Returns whether the request only reads.
    pub fn reads(&self) -> bool {
        matches!(
            self,
            Request::ReadCoils { .. }
                | Request::ReadDiscreteInputs { .. }
                | Request::ReadHoldingRegisters { .. }
                | Request::ReadInputRegisters { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_specification_examples_parse_to_what_they_ask() {
        // The request of each worked example in sections 6.1 to 6.6, 6.11, and 6.12.
        let read_coils = Pdu::read_coils(0x0013, 0x0013);
        assert_eq!(
            Request::parse(read_coils.as_bytes()),
            Ok(Request::ReadCoils {
                start: 0x0013,
                quantity: 0x0013
            })
        );
        let inputs = Pdu::read_discrete_inputs(0x00C4, 0x0016);
        assert_eq!(
            Request::parse(inputs.as_bytes()).map(|request| request.span()),
            Ok((0x00C4, 22))
        );
        let input_registers = Pdu::read_input_registers(0x0008, 1);
        assert!(Request::parse(input_registers.as_bytes()).is_ok_and(|r| r.reads()));
        let coil = Pdu::write_single_coil(0x00AC, true);
        assert_eq!(
            Request::parse(coil.as_bytes()),
            Ok(Request::WriteSingleCoil {
                address: 0x00AC,
                on: true
            })
        );
        let register = Pdu::write_single_register(0x0001, 0x0003);
        assert_eq!(
            Request::parse(register.as_bytes()).map(|r| r.function()),
            Ok(Function::WriteSingleRegister)
        );
        let ten = [
            true, false, true, true, false, false, true, true, true, false,
        ];
        let coils = Pdu::write_multiple_coils(0x0013, &ten).unwrap();
        match Request::parse(coils.as_bytes()) {
            Ok(Request::WriteMultipleCoils { start, values }) => {
                assert_eq!(start, 0x0013);
                assert!(values.eq(ten));
            }
            other => panic!("expected ten coils, got {other:?}"),
        }
        let registers = Pdu::write_multiple_registers(0x0001, &[0x000A, 0x0102]).unwrap();
        match Request::parse(registers.as_bytes()) {
            Ok(Request::WriteMultipleRegisters { start, values }) => {
                assert_eq!(start, 0x0001);
                assert!(values.eq([0x000A, 0x0102]));
            }
            other => panic!("expected two registers, got {other:?}"),
        }
    }

    #[test]
    fn a_quantity_outside_the_range_is_an_illegal_value() {
        for pdu in [
            Pdu::read_coils(0, 0),
            Pdu::read_coils(0, 2001),
            Pdu::read_discrete_inputs(0, 2001),
            Pdu::read_holding_registers(0, 0),
            Pdu::read_holding_registers(0, 126),
            Pdu::read_input_registers(0, 126),
        ] {
            assert_eq!(
                Request::parse(pdu.as_bytes()),
                Err(Exception::IllegalDataValue),
                "{:02x?}",
                pdu.as_bytes()
            );
        }
        assert!(Request::parse(Pdu::read_coils(0, 2000).as_bytes()).is_ok());
        assert!(Request::parse(Pdu::read_holding_registers(0, 125).as_bytes()).is_ok());
    }

    #[test]
    fn a_coil_value_other_than_on_or_off_is_an_illegal_value() {
        // 6.5: only FF 00 and 00 00 are coil values; anything else does not affect the coil.
        let half = Pdu::raw(0x05, &[0x00, 0xAC, 0x12, 0x34]).unwrap();
        assert_eq!(
            Request::parse(half.as_bytes()),
            Err(Exception::IllegalDataValue)
        );
    }

    #[test]
    fn a_byte_count_that_does_not_match_is_an_illegal_value() {
        // Two registers announced, three bytes counted.
        let short = Pdu::raw(0x10, &[0x00, 0x01, 0x00, 0x02, 0x03, 0x00, 0x0A, 0x01]).unwrap();
        assert_eq!(
            Request::parse(short.as_bytes()),
            Err(Exception::IllegalDataValue)
        );
        // Ten coils need two bytes.
        let coils = Pdu::raw(0x0F, &[0x00, 0x13, 0x00, 0x0A, 0x01, 0xCD]).unwrap();
        assert_eq!(
            Request::parse(coils.as_bytes()),
            Err(Exception::IllegalDataValue)
        );
        // A read with a byte too many.
        let long = Pdu::raw(0x03, &[0x00, 0x00, 0x00, 0x01, 0x00]).unwrap();
        assert_eq!(
            Request::parse(long.as_bytes()),
            Err(Exception::IllegalDataValue)
        );
    }

    #[test]
    fn an_unknown_function_is_an_illegal_function() {
        let diagnostics = Pdu::raw(0x08, &[0x00, 0x00, 0xA5, 0x37]).unwrap();
        assert_eq!(
            Request::parse(diagnostics.as_bytes()),
            Err(Exception::IllegalFunction)
        );
        assert_eq!(Request::parse(&[]), Err(Exception::IllegalFunction));
        let truncated = Pdu::raw(0x03, &[0x00]).unwrap();
        assert_eq!(
            Request::parse(truncated.as_bytes()),
            Err(Exception::IllegalDataValue)
        );
    }
}
