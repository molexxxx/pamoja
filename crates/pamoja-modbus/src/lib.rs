#![cfg_attr(not(any(test, feature = "port")), no_std)]
// The crate doc links the feature-gated types, which a build without the features lacks.
#![cfg_attr(not(feature = "port"), allow(rustdoc::broken_intra_doc_links))]

//! Modbus RTU for the pamoja SDK: the frames, a server, and a client on a serial line.
//!
//! Modbus is the lingua franca of cheap industrial sensing. Soil NPK probes, energy
//! meters, water-quality transmitters, and pump controllers overwhelmingly speak Modbus
//! over RS485, a serial bus that reaches hundreds of meters down a single cable, which
//! is exactly what a dispersed farm or a rural water network needs. To talk to those
//! devices a node has to put the right bytes on the wire and trust the bytes it gets
//! back, and Modbus RTU pins down precisely what those bytes are.
//!
//! The byte layer needs no serial port and no allocation:
//!
//! - [`crc16`] - the CRC-16/MODBUS that every RTU frame ends with, the check that lets a
//!   receiver reject a frame mangled by electrical noise on a long cable.
//! - [`Pdu`] - the protocol data unit: a function code and its data. Constructors build
//!   the standard requests (read and write coils and registers), the replies a device sends
//!   to each, and the exception it sends instead, so callers never hand-pack a frame, with a
//!   [`raw`](Pdu::raw) escape hatch for the function codes this crate does not name.
//! - [`Adu`] - the RTU application data unit: a unit address, a PDU, and the CRC. It both
//!   [assembles](Adu::from_pdu) a frame to send and [parses](Adu::parse) one received,
//!   verifying the CRC so a corrupt frame never reaches the application.
//! - [`Response`] - reads the values back out of a reply: the 16-bit registers of a
//!   read-registers response and the packed bits of a read-coils response, plus the
//!   [`Exception`] a device returns when it refuses a request.
//! - [`Request`] - reads a request the way a device does, checked against the quantity and
//!   value limits the specification sets.
//!
//! With the `alloc` feature, [`Server`] is a device: a unit address and the four tables it
//! serves, answering each frame with the reply or the exception the specification gives,
//! and staying silent where the serial line specification says a device does.
//!
//! With the `port` feature, which needs an operating system, [`Client`] runs transactions
//! over a [`pamoja_hal::port::SerialPort`]: it leaves the line silent for 3.5 characters
//! before each request, reads the reply to the length the request implies against a
//! response timeout, and checks the reply's CRC, unit, and function before a value is read
//! out of it. A server answers on a simulated port, and a [`Line`] carries several, so a
//! polling loop runs end to end with nothing plugged in.
//!
//! # Examples
//!
//! ```
//! use pamoja_modbus::{Adu, Exception, Function, Pdu, Request};
//!
//! // An energy meter at unit 17 keeps its voltage in tenths of a volt, its current in
//! // milliamps, and a fault word in the three holding registers from 107.
//! let request = Pdu::read_holding_registers(107, 3).to_adu(17);
//!
//! // The meter reads the request the way the specification says a device does.
//! let asked = Request::parse(request.pdu()).expect("a request the meter can serve");
//! assert_eq!(asked.span(), (107, 3));
//!
//! // Its reply carries the unit and a CRC, and a receiver checks both before it reads a
//! // value, so a frame mangled on a long cable never reaches the application.
//! let reply = Pdu::read_holding_registers_reply(&[2301, 418, 0])?.to_adu(17);
//! let received = Adu::parse(reply.as_bytes())?;
//! let registers: Vec<u16> = received.response().registers()?.collect();
//! assert_eq!(registers, [2301, 418, 0]);
//! assert_eq!(f64::from(registers[0]) / 10.0, 230.1);
//!
//! // Asked for a register it does not have, the meter answers with an exception instead.
//! let function = Function::ReadHoldingRegisters.code();
//! let refused = Pdu::exception(function, Exception::IllegalDataAddress).to_adu(17);
//! let answer = Adu::parse(refused.as_bytes())?;
//! assert_eq!(answer.exception(), Some(Exception::IllegalDataAddress));
//! # Ok::<(), pamoja_modbus::ModbusError>(())
//! ```

#[cfg(feature = "alloc")]
extern crate alloc;

mod adu;
#[cfg(feature = "port")]
mod client;
mod crc;
mod error;
mod function;
mod pdu;
mod request;
mod response;
#[cfg(feature = "alloc")]
mod server;

pub use adu::Adu;
#[cfg(feature = "port")]
pub use client::{Client, ClientError, RESPONSE_TIMEOUT, TURNAROUND};
pub use crc::crc16;
pub use error::ModbusError;
pub use function::{Exception, Function};
pub use pdu::Pdu;
pub use request::Request;
pub use response::{Coils, Registers, Response};
#[cfg(feature = "port")]
pub use server::Line;
#[cfg(feature = "alloc")]
pub use server::{Server, BROADCAST, UNITS};
