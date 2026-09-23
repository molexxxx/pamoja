//! A Modbus client on a serial line: the gateway's side of the conversation.

use std::fmt;
use std::time::{Duration, Instant};
use std::vec::Vec;

use pamoja_hal::port::{PortError, PortKind, SerialPort, Settings};

use crate::adu::Adu;
use crate::error::ModbusError;
use crate::function::Exception;
use crate::pdu::Pdu;
use crate::request::Request;
use crate::server::BROADCAST;

/// How long a client waits for a reply unless told otherwise: one second, the low end of what
/// the serial line specification suggests at 9600 baud.
pub const RESPONSE_TIMEOUT: Duration = Duration::from_secs(1);

/// How long a client leaves the line quiet after a broadcast unless told otherwise, so every
/// device has carried it out before the next request: 100 milliseconds, the low end of the
/// range the serial line specification gives.
pub const TURNAROUND: Duration = Duration::from_millis(100);

/// Why a transaction failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientError {
    /// The request could not be built: a quantity outside what one request carries.
    Request(ModbusError),
    /// A read was addressed to the broadcast address, which no device answers.
    BroadcastRead,
    /// The serial port failed.
    Port(PortError),
    /// No complete reply arrived within the response timeout.
    Timeout {
        /// The unit that was asked.
        unit: u8,
        /// How many bytes of a reply had arrived.
        received: usize,
    },
    /// The reply failed its CRC or is not the shape its function gives.
    Frame(ModbusError),
    /// A reply came back from a unit other than the one asked.
    WrongUnit {
        /// The unit that was asked.
        expected: u8,
        /// The unit that answered.
        found: u8,
    },
    /// A reply answered a function other than the one asked.
    WrongFunction {
        /// The function that was asked.
        expected: u8,
        /// The function the reply names.
        found: u8,
    },
    /// A well-formed reply that does not answer the request: a write echoed other values, or
    /// a read returned another count.
    Mismatch {
        /// The unit that answered.
        unit: u8,
    },
    /// The device refused the request.
    Exception {
        /// The unit that refused.
        unit: u8,
        /// The function it refused.
        function: u8,
        /// Why.
        exception: Exception,
    },
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::Request(error) => write!(f, "the request cannot be sent: {error}"),
            ClientError::BroadcastRead => {
                f.write_str("a read cannot be broadcast, since no device answers a broadcast")
            }
            ClientError::Port(error) => write!(f, "the serial port failed: {error}"),
            ClientError::Timeout { unit, received: 0 } => {
                write!(f, "unit {unit} did not answer within the response timeout")
            }
            ClientError::Timeout { unit, received } => write!(
                f,
                "unit {unit} sent {received} bytes of a reply and then went quiet"
            ),
            ClientError::Frame(error) => write!(f, "the reply is not a valid frame: {error}"),
            ClientError::WrongUnit { expected, found } => {
                write!(f, "unit {found} answered a request for unit {expected}")
            }
            ClientError::WrongFunction { expected, found } => write!(
                f,
                "the reply names function {found:#04x}, not the {expected:#04x} asked for"
            ),
            ClientError::Mismatch { unit } => {
                write!(
                    f,
                    "unit {unit} answered with values the request did not ask for"
                )
            }
            ClientError::Exception {
                unit,
                function,
                exception,
            } => write!(
                f,
                "unit {unit} refused function {function:#04x} with exception {:#04x}, {exception}",
                exception.code()
            ),
        }
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClientError::Request(error) | ClientError::Frame(error) => Some(error),
            ClientError::Port(error) => Some(error),
            _ => None,
        }
    }
}

/// A Modbus RTU client on a serial line: the gateway, the master in the specification's
/// words, that sends each request and waits for its reply.
///
/// Each transaction follows the Modbus over Serial Line specification. The client leaves
/// the line silent for 3.5 characters, or 1.75 ms above 19200 baud, so the request starts a
/// new frame; drops anything stale waiting in the port; writes the request; and reads the
/// reply to the length the request implies, against the response timeout. The reply's CRC,
/// unit, and function are checked before a value is read out of it, and a refusal comes
/// back as [`ClientError::Exception`]. A write to [`BROADCAST`] reaches every device and
/// draws no reply, so the client waits out the turnaround delay instead. The specification
/// reserves units 248 to 255, and a client still sends to them, since some devices answer on
/// one.
///
/// On the kernel's serial device every wait is real. On a simulated, paired, or scripted
/// port nothing waits: the waits are counted in the port's
/// [`waited_micros`](SerialPort::waited_micros), so a test of a meter that never answers
/// runs at once and still says how long a real one would have taken.
///
/// # Examples
///
/// ```
/// use std::sync::{Arc, Mutex};
///
/// use pamoja_hal::port::{Parity, SerialPort, Settings};
/// use pamoja_modbus::{Client, Server};
///
/// // A power meter at unit 17, simulated on the far end of a 19200 8E1 line.
/// let meter = Server::new(17)?.with_holding_registers(107, &[2301, 418, 0]);
/// let meter = Arc::new(Mutex::new(meter));
/// let line = Settings::new(19_200).with_parity(Parity::Even);
/// let mut client = Client::new(SerialPort::simulated(line, Arc::clone(&meter)));
///
/// assert_eq!(client.read_holding_registers(17, 107, 3)?, [2301, 418, 0]);
/// client.write_single_register(17, 109, 1)?;
/// assert_eq!(meter.lock().unwrap().holding_register(109), Some(1));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct Client {
    port: SerialPort,
    response_timeout: Duration,
    turnaround: Duration,
}

enum Expect {
    Bits { quantity: u16 },
    Registers { quantity: u16 },
    Echo,
}

impl Client {
    /// Makes a client on a port, with a one-second response timeout and a 100 ms turnaround.
    ///
    /// # Arguments
    ///
    /// * `port` - the line, opened at the speed and format the devices on it use.
    ///
    /// # Returns
    ///
    /// The client.
    pub fn new(port: SerialPort) -> Client {
        Client {
            port,
            response_timeout: RESPONSE_TIMEOUT,
            turnaround: TURNAROUND,
        }
    }

    /// Sets how long the client waits for a whole reply once a request has gone out.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the response timeout; a slow device or a slow line needs more.
    ///
    /// # Returns
    ///
    /// The client.
    #[must_use]
    pub fn with_response_timeout(mut self, timeout: Duration) -> Client {
        self.response_timeout = timeout;
        self
    }

    /// Sets how long the client leaves the line quiet after a broadcast.
    ///
    /// # Arguments
    ///
    /// * `delay` - the turnaround delay, long enough for the slowest device to carry out
    ///   the write.
    ///
    /// # Returns
    ///
    /// The client.
    #[must_use]
    pub fn with_turnaround(mut self, delay: Duration) -> Client {
        self.turnaround = delay;
        self
    }

    /// Changes how long the client waits for a whole reply, as
    /// [`with_response_timeout`](Client::with_response_timeout) does when the client is made.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the response timeout.
    pub fn set_response_timeout(&mut self, timeout: Duration) {
        self.response_timeout = timeout;
    }

    /// Changes how long the client leaves the line quiet after a broadcast, as
    /// [`with_turnaround`](Client::with_turnaround) does when the client is made.
    ///
    /// # Arguments
    ///
    /// * `delay` - the turnaround delay.
    pub fn set_turnaround(&mut self, delay: Duration) {
        self.turnaround = delay;
    }

    /// Returns the port the client runs on.
    pub fn port(&self) -> &SerialPort {
        &self.port
    }

    /// Returns how long the client waits for a reply.
    pub fn response_timeout(&self) -> Duration {
        self.response_timeout
    }

    /// Returns how long the client leaves the line quiet after a broadcast.
    pub fn turnaround(&self) -> Duration {
        self.turnaround
    }

    /// Returns the silence that separates two frames, 3.5 characters at the line's speed and
    /// format, and a fixed 1.75 ms above 19200 baud, where the specification stops scaling it.
    ///
    /// # Arguments
    ///
    /// * `settings` - the line's speed and character format.
    ///
    /// # Returns
    ///
    /// The silence, rounded up to the nanosecond.
    pub fn frame_gap(settings: Settings) -> Duration {
        if settings.baud > 19_200 {
            Duration::from_micros(1_750)
        } else {
            Duration::from_nanos((settings.character_nanos() * 7).div_ceil(2))
        }
    }

    /// Reads coils, function `0x01`.
    ///
    /// # Arguments
    ///
    /// * `unit` - the device, 1 to 247.
    /// * `start` - the first coil's address.
    /// * `quantity` - how many, 1 to 2000.
    ///
    /// # Returns
    ///
    /// The coils' states, in address order.
    ///
    /// # Errors
    ///
    /// Any [`ClientError`]: a quantity out of range, a broadcast, a port failure, a timeout,
    /// a reply that fails its checks, or the device's exception.
    pub fn read_coils(
        &mut self,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> Result<Vec<bool>, ClientError> {
        self.read_bits(unit, Pdu::read_coils(start, quantity), quantity)
    }

    /// Reads discrete inputs, function `0x02`.
    ///
    /// # Arguments
    ///
    /// * `unit` - the device, 1 to 247.
    /// * `start` - the first input's address.
    /// * `quantity` - how many, 1 to 2000.
    ///
    /// # Returns
    ///
    /// The inputs' states, in address order.
    ///
    /// # Errors
    ///
    /// Any [`ClientError`], as for [`read_coils`](Client::read_coils).
    pub fn read_discrete_inputs(
        &mut self,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> Result<Vec<bool>, ClientError> {
        self.read_bits(unit, Pdu::read_discrete_inputs(start, quantity), quantity)
    }

    /// Reads holding registers, function `0x03`.
    ///
    /// # Arguments
    ///
    /// * `unit` - the device, 1 to 247.
    /// * `start` - the first register's address.
    /// * `quantity` - how many, 1 to 125.
    ///
    /// # Returns
    ///
    /// The registers' values, in address order.
    ///
    /// # Errors
    ///
    /// Any [`ClientError`], as for [`read_coils`](Client::read_coils).
    pub fn read_holding_registers(
        &mut self,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> Result<Vec<u16>, ClientError> {
        let pdu = Pdu::read_holding_registers(start, quantity);
        self.read_words(unit, pdu, quantity)
    }

    /// Reads input registers, function `0x04`.
    ///
    /// # Arguments
    ///
    /// * `unit` - the device, 1 to 247.
    /// * `start` - the first register's address.
    /// * `quantity` - how many, 1 to 125.
    ///
    /// # Returns
    ///
    /// The registers' values, in address order.
    ///
    /// # Errors
    ///
    /// Any [`ClientError`], as for [`read_coils`](Client::read_coils).
    pub fn read_input_registers(
        &mut self,
        unit: u8,
        start: u16,
        quantity: u16,
    ) -> Result<Vec<u16>, ClientError> {
        let pdu = Pdu::read_input_registers(start, quantity);
        self.read_words(unit, pdu, quantity)
    }

    /// Writes one coil, function `0x05`.
    ///
    /// # Arguments
    ///
    /// * `unit` - the device, 1 to 247, or [`BROADCAST`] for every device.
    /// * `address` - the coil's address.
    /// * `on` - the state to write.
    ///
    /// # Errors
    ///
    /// Any [`ClientError`] but [`BroadcastRead`](ClientError::BroadcastRead).
    pub fn write_single_coil(
        &mut self,
        unit: u8,
        address: u16,
        on: bool,
    ) -> Result<(), ClientError> {
        let pdu = Pdu::write_single_coil(address, on);
        self.write(unit, pdu, pdu)
    }

    /// Writes one holding register, function `0x06`.
    ///
    /// # Arguments
    ///
    /// * `unit` - the device, 1 to 247, or [`BROADCAST`] for every device.
    /// * `address` - the register's address.
    /// * `value` - the value to write.
    ///
    /// # Errors
    ///
    /// Any [`ClientError`] but [`BroadcastRead`](ClientError::BroadcastRead).
    pub fn write_single_register(
        &mut self,
        unit: u8,
        address: u16,
        value: u16,
    ) -> Result<(), ClientError> {
        let pdu = Pdu::write_single_register(address, value);
        self.write(unit, pdu, pdu)
    }

    /// Writes a run of coils, function `0x0F`.
    ///
    /// # Arguments
    ///
    /// * `unit` - the device, 1 to 247, or [`BROADCAST`] for every device.
    /// * `start` - the first coil's address.
    /// * `values` - the states to write, 1 to 1968 of them, in address order.
    ///
    /// # Errors
    ///
    /// Any [`ClientError`] but [`BroadcastRead`](ClientError::BroadcastRead).
    pub fn write_multiple_coils(
        &mut self,
        unit: u8,
        start: u16,
        values: &[bool],
    ) -> Result<(), ClientError> {
        let pdu = Pdu::write_multiple_coils(start, values).map_err(ClientError::Request)?;
        let echo = Pdu::write_multiple_coils_reply(start, values.len() as u16);
        self.write(unit, pdu, echo)
    }

    /// Writes a run of holding registers, function `0x10`.
    ///
    /// # Arguments
    ///
    /// * `unit` - the device, 1 to 247, or [`BROADCAST`] for every device.
    /// * `start` - the first register's address.
    /// * `values` - the values to write, 1 to 123 of them, in address order.
    ///
    /// # Errors
    ///
    /// Any [`ClientError`] but [`BroadcastRead`](ClientError::BroadcastRead).
    pub fn write_multiple_registers(
        &mut self,
        unit: u8,
        start: u16,
        values: &[u16],
    ) -> Result<(), ClientError> {
        let pdu = Pdu::write_multiple_registers(start, values).map_err(ClientError::Request)?;
        let echo = Pdu::write_multiple_registers_reply(start, values.len() as u16);
        self.write(unit, pdu, echo)
    }

    fn read_bits(&mut self, unit: u8, pdu: Pdu, quantity: u16) -> Result<Vec<bool>, ClientError> {
        let reply = self
            .transact(unit, pdu, Expect::Bits { quantity })?
            .ok_or(ClientError::BroadcastRead)?;
        let bits = reply
            .response()
            .coils(quantity)
            .map_err(|_| ClientError::Mismatch { unit })?;
        Ok(bits.collect())
    }

    fn read_words(&mut self, unit: u8, pdu: Pdu, quantity: u16) -> Result<Vec<u16>, ClientError> {
        let reply = self
            .transact(unit, pdu, Expect::Registers { quantity })?
            .ok_or(ClientError::BroadcastRead)?;
        let words = reply
            .response()
            .registers()
            .map_err(|_| ClientError::Frame(ModbusError::MalformedResponse))?;
        if words.len() != usize::from(quantity) {
            return Err(ClientError::Mismatch { unit });
        }
        Ok(words.collect())
    }

    fn write(&mut self, unit: u8, pdu: Pdu, echo: Pdu) -> Result<(), ClientError> {
        match self.transact(unit, pdu, Expect::Echo)? {
            Some(reply) if reply.pdu() != echo.as_bytes() => Err(ClientError::Mismatch { unit }),
            _ => Ok(()),
        }
    }

    // Runs one transaction, returning the reply, or nothing for a broadcast, which no device
    // answers.
    fn transact(&mut self, unit: u8, pdu: Pdu, expect: Expect) -> Result<Option<Adu>, ClientError> {
        let request = Request::parse(pdu.as_bytes())
            .map_err(|_| ClientError::Request(ModbusError::InvalidValueCount))?;
        if unit == BROADCAST && request.reads() {
            return Err(ClientError::BroadcastRead);
        }
        let frame = pdu.to_adu(unit);
        self.port.wait(Client::frame_gap(self.port.settings()));
        self.port.discard_input().map_err(ClientError::Port)?;
        self.port
            .write(frame.as_bytes())
            .map_err(ClientError::Port)?;
        if unit == BROADCAST {
            self.port.wait(self.turnaround);
            return Ok(None);
        }
        let reply = self.receive(unit, &expect)?;
        let adu = Adu::parse(&reply).map_err(ClientError::Frame)?;
        if adu.address() != unit {
            return Err(ClientError::WrongUnit {
                expected: unit,
                found: adu.address(),
            });
        }
        let function = pdu.function_code();
        let found = adu.function_code();
        if found & 0x7F != function {
            return Err(ClientError::WrongFunction {
                expected: function,
                found,
            });
        }
        if found & 0x80 != 0 {
            let exception = adu
                .exception()
                .ok_or(ClientError::Frame(ModbusError::MalformedResponse))?;
            return Err(ClientError::Exception {
                unit,
                function,
                exception,
            });
        }
        Ok(Some(adu))
    }

    // Reads one reply: the length a read's byte count gives, 8 bytes for a write's echo, and
    // 5 for an exception, all against one response timeout.
    fn receive(&mut self, unit: u8, expect: &Expect) -> Result<Vec<u8>, ClientError> {
        let clock = self.port.kind() == PortKind::Device;
        let started = Instant::now();
        let mut frame = [0u8; Adu::MAX_LEN];
        let mut want = match expect {
            Expect::Bits { quantity, .. } => 5 + usize::from(*quantity).div_ceil(8),
            Expect::Registers { quantity } => 5 + 2 * usize::from(*quantity),
            Expect::Echo => 8,
        };
        let mut got = 0;
        while got < want {
            let timeout = if clock {
                self.response_timeout.saturating_sub(started.elapsed())
            } else {
                self.response_timeout
            };
            let count = self
                .port
                .read(&mut frame[got..want], timeout)
                .map_err(ClientError::Port)?;
            if count == 0 {
                return Err(ClientError::Timeout {
                    unit,
                    received: got,
                });
            }
            got += count;
            if got >= 2 && frame[1] & 0x80 != 0 {
                want = 5;
            } else if got >= 3 && !matches!(expect, Expect::Echo) {
                want = (5 + usize::from(frame[2])).min(Adu::MAX_LEN);
            }
        }
        Ok(frame[..want].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use pamoja_hal::port::{Parity, PortStep, StopBits};

    use crate::server::{Line, Server};

    fn line() -> Settings {
        Settings::new(19_200).with_parity(Parity::Even)
    }

    fn meter() -> Server {
        Server::new(17)
            .unwrap()
            .with_holding_registers(107, &[2301, 418, 0])
            .with_coils(0, &[false; 4])
    }

    #[test]
    fn the_frame_gap_is_three_and_a_half_characters_up_to_19200_and_fixed_above() {
        let at = |baud| Client::frame_gap(Settings::new(baud).with_parity(Parity::Even));
        // 11 bits a character at 9600 is 1145.83 us, and 3.5 of them 4010.42 us.
        assert_eq!(at(9_600), Duration::from_nanos(4_010_419));
        assert_eq!(at(19_200), Duration::from_nanos(2_005_210));
        for baud in [38_400, 57_600, 115_200] {
            assert_eq!(at(baud), Duration::from_micros(1_750));
        }
        // No parity keeps 11 bits a character with a second stop bit.
        let two = Settings::new(9_600).with_stop_bits(StopBits::Two);
        assert_eq!(Client::frame_gap(two), at(9_600));
    }

    #[test]
    fn a_read_puts_the_specification_frame_on_the_wire_and_decodes_the_reply() {
        let reply = Pdu::read_holding_registers_reply(&[0x022B, 0x0000, 0x0064])
            .unwrap()
            .to_adu(0x11);
        let port = SerialPort::scripted(
            line(),
            [
                PortStep::write([0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87]),
                PortStep::read(reply.as_bytes()),
            ],
        );
        let mut client = Client::new(port.clone());
        let values = client.read_holding_registers(0x11, 0x006B, 3).unwrap();
        assert_eq!(values, [0x022B, 0x0000, 0x0064]);
        assert_eq!(port.remaining(), Some(0));
        assert_eq!(port.waited_micros(), 2_005, "only the frame gap was waited");
    }

    #[test]
    fn every_function_round_trips_against_a_server() {
        let meter = Arc::new(Mutex::new(
            meter().with_discrete_inputs(0x00C4, &[true, false, true]),
        ));
        let mut client = Client::new(SerialPort::simulated(line(), Arc::clone(&meter)));
        assert_eq!(
            client.read_holding_registers(17, 107, 3).unwrap(),
            [2301, 418, 0]
        );
        client.write_single_register(17, 109, 7).unwrap();
        client
            .write_multiple_registers(17, 107, &[2300, 420])
            .unwrap();
        assert_eq!(
            client.read_holding_registers(17, 107, 3).unwrap(),
            [2300, 420, 7]
        );
        client.write_single_coil(17, 1, true).unwrap();
        client.write_multiple_coils(17, 2, &[true, true]).unwrap();
        assert_eq!(
            client.read_coils(17, 0, 4).unwrap(),
            [false, true, true, true]
        );
        assert_eq!(
            client.read_discrete_inputs(17, 0x00C4, 3).unwrap(),
            [true, false, true]
        );
        let device = meter.lock().unwrap();
        assert_eq!(device.holding_register(109), Some(7));
        assert_eq!(device.served(), 8);
    }

    #[test]
    fn a_refusal_comes_back_as_the_devices_exception() {
        let mut client = Client::new(SerialPort::simulated(line(), meter()));
        assert_eq!(
            client.read_holding_registers(17, 108, 3),
            Err(ClientError::Exception {
                unit: 17,
                function: 0x03,
                exception: Exception::IllegalDataAddress
            })
        );
        assert_eq!(
            client.write_single_coil(17, 9, true),
            Err(ClientError::Exception {
                unit: 17,
                function: 0x05,
                exception: Exception::IllegalDataAddress
            })
        );
    }

    #[test]
    fn a_unit_that_never_answers_times_out_after_the_response_timeout() {
        let port = SerialPort::simulated(line(), meter());
        let mut client =
            Client::new(port.clone()).with_response_timeout(Duration::from_millis(250));
        assert_eq!(
            client.read_holding_registers(18, 107, 1),
            Err(ClientError::Timeout {
                unit: 18,
                received: 0
            })
        );
        assert_eq!(port.waited_micros(), 2_005 + 250_000);
    }

    #[test]
    fn a_broadcast_write_waits_out_the_turnaround_and_every_device_carries_it_out() {
        let mut line_of = Line::new();
        let first = line_of.attach(meter());
        let second = line_of.attach(Server::new(18).unwrap().with_holding_registers(109, &[0]));
        let port = SerialPort::simulated(line(), line_of);
        let mut client = Client::new(port.clone());
        client.write_single_register(BROADCAST, 109, 1).unwrap();
        client
            .write_multiple_registers(BROADCAST, 109, &[1])
            .unwrap();
        assert_eq!(first.lock().unwrap().holding_register(109), Some(1));
        assert_eq!(second.lock().unwrap().holding_register(109), Some(1));
        assert_eq!(port.received(), 0, "nobody answers a broadcast");
        assert_eq!(port.waited_micros(), 2 * (2_005 + 100_000));

        assert_eq!(
            client.read_holding_registers(BROADCAST, 109, 1),
            Err(ClientError::BroadcastRead)
        );
        assert_eq!(port.written(), 8 + 11, "the broadcast read never went out");
    }

    #[test]
    fn a_reply_that_fails_its_checks_is_refused_before_a_value_is_read() {
        let request = Pdu::read_holding_registers(107, 1).to_adu(17);
        let good = Pdu::read_holding_registers_reply(&[2301]).unwrap();

        let mut corrupt = good.to_adu(17).as_bytes().to_vec();
        corrupt[3] ^= 0x01;
        let scripted = |reply: Vec<u8>| {
            SerialPort::scripted(
                line(),
                [PortStep::write(request.as_bytes()), PortStep::read(reply)],
            )
        };
        let mut client = Client::new(scripted(corrupt));
        assert!(matches!(
            client.read_holding_registers(17, 107, 1),
            Err(ClientError::Frame(ModbusError::CrcMismatch { .. }))
        ));

        let mut client = Client::new(scripted(good.to_adu(18).as_bytes().to_vec()));
        assert_eq!(
            client.read_holding_registers(17, 107, 1),
            Err(ClientError::WrongUnit {
                expected: 17,
                found: 18
            })
        );

        let other = Pdu::read_input_registers_reply(&[2301]).unwrap().to_adu(17);
        let mut client = Client::new(scripted(other.as_bytes().to_vec()));
        assert_eq!(
            client.read_holding_registers(17, 107, 1),
            Err(ClientError::WrongFunction {
                expected: 0x03,
                found: 0x04
            })
        );

        let two = Pdu::read_holding_registers_reply(&[2301, 418])
            .unwrap()
            .to_adu(17);
        let mut client = Client::new(scripted(two.as_bytes().to_vec()));
        assert_eq!(
            client.read_holding_registers(17, 107, 1),
            Err(ClientError::Mismatch { unit: 17 })
        );

        let half = good.to_adu(17).as_bytes()[..4].to_vec();
        let mut client = Client::new(scripted(half));
        assert_eq!(
            client.read_holding_registers(17, 107, 1),
            Err(ClientError::Timeout {
                unit: 17,
                received: 4
            })
        );
    }

    #[test]
    fn a_request_the_protocol_cannot_carry_never_reaches_the_line() {
        let port = SerialPort::simulated(line(), meter());
        let mut client = Client::new(port.clone());
        assert_eq!(
            client.read_holding_registers(17, 0, 126),
            Err(ClientError::Request(ModbusError::InvalidValueCount))
        );
        assert_eq!(
            client.write_multiple_registers(17, 0, &[0; 124]),
            Err(ClientError::Request(ModbusError::InvalidValueCount))
        );
        assert_eq!(port.written(), 0);
    }

    #[test]
    fn a_reserved_unit_is_still_asked_since_some_devices_answer_on_one() {
        let request = Pdu::read_input_registers(0, 1).to_adu(248);
        let reply = Pdu::read_input_registers_reply(&[2301])
            .unwrap()
            .to_adu(248);
        let port = SerialPort::scripted(
            line(),
            [
                PortStep::write(request.as_bytes()),
                PortStep::read(reply.as_bytes()),
            ],
        );
        let mut client = Client::new(port);
        assert_eq!(client.read_input_registers(248, 0, 1), Ok(vec![2301]));
    }

    #[test]
    fn a_stale_reply_is_dropped_before_the_next_request() {
        let (gateway, device) = SerialPort::pair(line());
        device
            .write(
                Pdu::read_holding_registers_reply(&[1])
                    .unwrap()
                    .to_adu(17)
                    .as_bytes(),
            )
            .unwrap();
        let mut client = Client::new(gateway).with_response_timeout(Duration::from_millis(5));
        assert_eq!(
            client.read_holding_registers(17, 107, 1),
            Err(ClientError::Timeout {
                unit: 17,
                received: 0
            })
        );
    }
}
