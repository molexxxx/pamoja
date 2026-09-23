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
