#![cfg_attr(not(test), no_std)]

//! Modbus RTU framing for the pamoja SDK.
//!
//! Modbus is the lingua franca of cheap industrial sensing. Soil NPK probes, energy
//! meters, water-quality transmitters, and pump controllers overwhelmingly speak Modbus
//! over RS485, a serial bus that reaches hundreds of meters down a single cable, which
//! is exactly what a dispersed farm or a rural water network needs. To talk to those
//! devices a node has to put the right bytes on the wire and trust the bytes it gets
//! back, and Modbus RTU pins down precisely what those bytes are.
//!
//! This crate is that byte layer, with no serial port and no allocation:
//!
//! - [`crc16`] - the CRC-16/MODBUS that every RTU frame ends with, the check that lets a
//!   receiver reject a frame mangled by electrical noise on a long cable.
//! - [`Pdu`] - the protocol data unit: a function code and its data. Constructors build
//!   the standard requests (read and write coils and registers) so callers never hand-pack
//!   a frame, with a [`raw`](Pdu::raw) escape hatch for the function codes this crate does
//!   not name.
//! - [`Adu`] - the RTU application data unit: a unit address, a PDU, and the CRC. It both
//!   [assembles](Adu::from_pdu) a frame to send and [parses](Adu::parse) one received,
//!   verifying the CRC so a corrupt frame never reaches the application.
//! - [`Response`] - reads the values back out of a reply: the 16-bit registers of a
//!   read-registers response and the packed bits of a read-coils response, plus the
//!   [`Exception`] a device returns when it refuses a request.
//!
//! Everything is exact integer and byte work, so the same framing runs on the smallest
//! microcontroller hanging off the bus. Driving the RS485 line itself arrives with the
//! hardware-I/O layer; this is the protocol half ahead of it.
//!
//! # Examples
//!
//! ```
//! use pamoja_modbus::{Adu, Pdu};
//!
//! // Ask unit 0x11 for three holding registers starting at 0x006B.
//! let request = Pdu::read_holding_registers(0x006B, 3).to_adu(0x11);
//! assert_eq!(request.as_bytes(), &[0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87]);
//!
//! // The device replies with three 16-bit registers; the frame carries its own CRC,
//! // so a receiver validates it before reading the values.
//! let on_wire = Adu::from_pdu(0x11, &[0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64])?;
//! let reply = Adu::parse(on_wire.as_bytes())?;
//! let registers: Vec<u16> = reply.response().registers()?.collect();
//! assert_eq!(registers, [0x022B, 0x0000, 0x0064]);
//! # Ok::<(), pamoja_modbus::ModbusError>(())
//! ```

mod adu;
mod crc;
mod error;
mod function;
mod pdu;
mod response;

pub use adu::Adu;
pub use crc::crc16;
pub use error::ModbusError;
pub use function::{Exception, Function};
pub use pdu::Pdu;
pub use response::{Coils, Registers, Response};
