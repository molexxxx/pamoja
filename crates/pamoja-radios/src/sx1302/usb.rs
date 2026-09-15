//! A concentrator on a USB card, driven through its bridge.
//!
//! [`bridge`] builds the messages; this sends them down a port and reads
//! what comes back. The port is anything that reads and writes bytes, so a test hands over
//! a script and a gateway hands over the serial device the card enumerates as.
//!
//! The concentrator driver wants an SPI device and a reset pin. Over USB both are the bridge,
//! so [`BridgeSpi`] and [`BridgePin`] each hold a share of one [`Bridge`] and turn the
//! driver's transfers and pin writes into messages. The driver does not change: every frame
//! it builds goes out inside a message exactly as it would have gone out on the wire.
//!
//! One message carries one transfer, which costs a round trip per register. The bridge can
//! carry many at once, and a driver that gathers its writes into a [`Bulk`] and sends them
//! through [`Bridge::bulk`] saves those trips; the firmware load is where that matters.

use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use embedded_hal::digital::{self, OutputPin};
use embedded_hal::spi::{self, Operation, SpiDevice};

use super::bridge::{
    self, BridgeError, Bulk, Header, Identity, Replies, Status, ACK, GPIO_PORT, HEADER_LEN,
    MAX_MESSAGE, ORDER_ERROR, ORDER_PING, ORDER_RESET, ORDER_SPI, ORDER_STATUS, ORDER_WRITE_GPIO,
    PIN_POWER, PIN_RADIO_RESET, PIN_RESET, TARGET_CONCENTRATOR, TARGET_LISTENER,
};

/// What went wrong talking to the bridge.
#[derive(Debug)]
pub enum UsbError {
    /// The port failed.
    Io(io::Error),
    /// The bridge answered something it should not have.
    Bridge(BridgeError),
    /// The bridge answered with more than there was room for.
    TooLong {
        /// How many bytes it announced.
        size: usize,
        /// How many there was room for.
        room: usize,
    },
    /// A frame the driver built is longer than one message carries.
    FrameTooLong {
        /// How long it is.
        len: usize,
    },
    /// The bridge could not read the order at all.
    Rejected,
    /// Another holder of the bridge panicked while holding it.
    Poisoned,
}

impl core::fmt::Display for UsbError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UsbError::Io(error) => write!(f, "the port failed: {error}"),
            UsbError::Bridge(error) => error.fmt(f),
            UsbError::TooLong { size, room } => {
                write!(
                    f,
                    "the bridge answered {size} bytes and there was room for {room}"
                )
            }
            UsbError::FrameTooLong { len } => {
                write!(
                    f,
                    "a {len} byte frame does not fit one message to the bridge"
                )
            }
            UsbError::Rejected => f.write_str("the bridge could not read the order"),
            UsbError::Poisoned => f.write_str("the bridge was left locked by a panic"),
        }
    }
}

impl std::error::Error for UsbError {}

impl From<io::Error> for UsbError {
    fn from(error: io::Error) -> UsbError {
        UsbError::Io(error)
    }
}

impl From<BridgeError> for UsbError {
    fn from(error: BridgeError) -> UsbError {
        UsbError::Bridge(error)
    }
}

impl<T> From<PoisonError<T>> for UsbError {
    fn from(_: PoisonError<T>) -> UsbError {
        UsbError::Poisoned
    }
}

impl spi::Error for UsbError {
    fn kind(&self) -> spi::ErrorKind {
        spi::ErrorKind::Other
    }
}

impl digital::Error for UsbError {
    fn kind(&self) -> digital::ErrorKind {
        digital::ErrorKind::Other
    }
}

/// The bridge on a USB card, reached over a port.
pub struct Bridge<P> {
    port: P,
    next_id: u8,
}

impl<P: Read + Write> Bridge<P> {
    /// Wraps a port. Nothing is sent until the bridge is asked something.
    ///
    /// # Arguments
    ///
    /// * `port` - the serial device the card enumerates as, opened raw.
    ///
    /// # Returns
    ///
    /// The bridge.
    pub fn new(port: P) -> Bridge<P> {
        Bridge { port, next_id: 0 }
    }

    /// Asks the bridge who it is.
    ///
    /// # Returns
    ///
    /// Its unique identifier and firmware version.
    ///
    /// # Errors
    ///
    /// When the port fails or the bridge answers something else.
    pub fn ping(&mut self) -> Result<Identity, UsbError> {
        self.send(ORDER_PING, &[])?;
        let mut answer = [0u8; bridge::PING_LEN];
        let got = self.receive(ORDER_PING, &mut answer)?;
        Ok(Identity::parse(&answer[..got])?)
    }

    /// Asks the bridge for its clock and temperature.
    ///
    /// # Returns
    ///
    /// The status.
    ///
    /// # Errors
    ///
    /// When the port fails or the bridge answers something else.
    pub fn status(&mut self) -> Result<Status, UsbError> {
        self.send(ORDER_STATUS, &[])?;
        let mut answer = [0u8; bridge::STATUS_LEN];
        let got = self.receive(ORDER_STATUS, &mut answer)?;
        Ok(Status::parse(&answer[..got])?)
    }

    /// Drives one of the card's pins.
    ///
    /// # Arguments
    ///
    /// * `pin` - which pin, such as [`PIN_RESET`].
    /// * `high` - whether to drive it high.
    ///
    /// # Errors
    ///
    /// When the port fails or the bridge refuses.
    pub fn write_gpio(&mut self, pin: u8, high: bool) -> Result<(), UsbError> {
        self.send(ORDER_WRITE_GPIO, &bridge::write_gpio(GPIO_PORT, pin, high))?;
        let mut answer = [0u8; 1];
        let got = self.receive(ORDER_WRITE_GPIO, &mut answer)?;
        Ok(bridge::took(&answer[..got])?)
    }

    /// Resets the card.
    ///
    /// # Errors
    ///
    /// When the port fails or the bridge refuses.
    pub fn reset_card(&mut self) -> Result<(), UsbError> {
        self.send(ORDER_RESET, &bridge::reset())?;
        let mut answer = [0u8; 1];
        let got = self.receive(ORDER_RESET, &mut answer)?;
        Ok(bridge::took(&answer[..got])?)
    }

    /// Brings the card up the way the reference does before it is used.
    ///
    /// The supply is switched on, the concentrator's reset is pulsed, then the radio beside
    /// it has its own reset pulsed, which is active low.
    ///
    /// # Errors
    ///
    /// When any of those pin writes fails.
    pub fn power_up(&mut self) -> Result<(), UsbError> {
        self.write_gpio(PIN_POWER, true)?;
        self.write_gpio(PIN_RESET, true)?;
        self.write_gpio(PIN_RESET, false)?;
        self.write_gpio(PIN_RADIO_RESET, false)?;
        self.write_gpio(PIN_RADIO_RESET, true)
    }

    /// Sends a bulk of SPI transfers and reads back what the chip clocked.
    ///
    /// # Arguments
    ///
    /// * `bulk` - the entries.
    /// * `into` - where the answer goes, which needs as much room as the entries took plus
    ///   nothing, since the answer echoes each entry at the same length.
    ///
    /// # Returns
    ///
    /// How many bytes of answer were read, which [`Replies`] then walks.
    ///
    /// # Errors
    ///
    /// When the port fails, the bridge answers another order, or the answer does not fit.
    pub fn bulk(&mut self, bulk: &Bulk, into: &mut [u8]) -> Result<usize, UsbError> {
        self.send(ORDER_SPI, bulk.as_bytes())?;
        self.receive(ORDER_SPI, into)
    }

    // Writes a header and a payload.
    fn send(&mut self, order: u8, payload: &[u8]) -> Result<(), UsbError> {
        if payload.len() > MAX_MESSAGE {
            return Err(UsbError::FrameTooLong { len: payload.len() });
        }
        let header = Header {
            id: self.next_id,
            size: payload.len() as u16,
            order,
        };
        self.next_id = self.next_id.wrapping_add(1);
        self.port.write_all(&header.to_bytes())?;
        if !payload.is_empty() {
            self.port.write_all(payload)?;
        }
        self.port.flush()?;
        Ok(())
    }

    // Reads a header, checks it answers the order, then reads its payload.
    fn receive(&mut self, order: u8, into: &mut [u8]) -> Result<usize, UsbError> {
        let mut raw = [0u8; HEADER_LEN];
        self.port.read_exact(&mut raw)?;
        let header = Header::from_bytes(raw);
        if header.order == ORDER_ERROR {
            return Err(UsbError::Rejected);
        }
        if !header.answers(order) {
            return Err(BridgeError::WrongAnswer {
                asked: order,
                answered: header.order.wrapping_sub(ACK),
            }
            .into());
        }
        let size = usize::from(header.size);
        if size > into.len() {
            return Err(UsbError::TooLong {
                size,
                room: into.len(),
            });
        }
        self.port.read_exact(&mut into[..size])?;
        Ok(size)
    }
}

/// One bridge, held by everything that needs it.
///
/// The concentrator driver takes its SPI device and its reset pin as two values it owns, and
/// over USB both are the same bridge. This is how they share it.
pub struct Shared<P>(Arc<Mutex<Bridge<P>>>);

impl<P> Clone for Shared<P> {
    fn clone(&self) -> Shared<P> {
        Shared(Arc::clone(&self.0))
    }
}

impl<P: Read + Write> Shared<P> {
    /// Shares a bridge.
    ///
    /// # Arguments
    ///
    /// * `bridge` - the bridge to share.
    ///
    /// # Returns
    ///
    /// A handle, which clones cheaply.
    pub fn new(bridge: Bridge<P>) -> Shared<P> {
        Shared(Arc::new(Mutex::new(bridge)))
    }

    /// The SPI device the concentrator answers on.
    ///
    /// # Returns
    ///
    /// A device the driver takes, carrying every frame to the concentrator and the front ends
    /// behind it.
    pub fn concentrator(&self) -> BridgeSpi<P> {
        BridgeSpi {
            bridge: self.clone(),
            target: TARGET_CONCENTRATOR,
        }
    }

    /// The SPI device the radio beside the concentrator answers on.
    ///
    /// # Returns
    ///
    /// A device carrying every frame to that radio, which is what a carrier check drives.
    pub fn listener(&self) -> BridgeSpi<P> {
        BridgeSpi {
            bridge: self.clone(),
            target: TARGET_LISTENER,
        }
    }

    /// One of the card's pins, as a pin the driver drives.
    ///
    /// # Arguments
    ///
    /// * `pin` - which one, such as [`PIN_RESET`].
    ///
    /// # Returns
    ///
    /// The pin.
    pub fn pin(&self, pin: u8) -> BridgePin<P> {
        BridgePin {
            bridge: self.clone(),
            pin,
        }
    }

    /// Does something with the bridge itself, holding it for the duration.
    ///
    /// # Arguments
    ///
    /// * `with` - what to do.
    ///
    /// # Returns
    ///
    /// Whatever it returned.
    ///
    /// # Errors
    ///
    /// [`UsbError::Poisoned`] when another holder panicked, and whatever `with` returned.
    pub fn with<T>(
        &self,
        with: impl FnOnce(&mut Bridge<P>) -> Result<T, UsbError>,
    ) -> Result<T, UsbError> {
        let mut bridge = self.0.lock()?;
        with(&mut bridge)
    }
}

/// An SPI device that carries each transfer to the bridge.
pub struct BridgeSpi<P> {
    bridge: Shared<P>,
    target: u8,
}

impl<P> spi::ErrorType for BridgeSpi<P> {
    type Error = UsbError;
}

impl<P: Read + Write> SpiDevice<u8> for BridgeSpi<P> {
    // One transaction is one time the chip select is held, so it is one raw frame: what every
    // operation clocks out, in order, with zeros where the host only listens. The bridge
    // clocks that frame and echoes what came back, and each operation that listens takes its
    // own slice of the echo.
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), UsbError> {
        let mut frame = Vec::new();
        for operation in operations.iter() {
            match operation {
                Operation::Write(bytes) | Operation::Transfer(_, bytes) => {
                    frame.extend_from_slice(bytes);
                }
                Operation::Read(buffer) => frame.resize(frame.len() + buffer.len(), 0),
                Operation::TransferInPlace(buffer) => frame.extend_from_slice(buffer),
                Operation::DelayNs(_) => {}
            }
        }
        if frame.len() + 5 > bridge::BULK_CHUNK {
            return Err(UsbError::FrameTooLong { len: frame.len() });
        }

        let mut bulk = Bulk::new();
        bulk.transfer(self.target, &frame)?;
        let mut answer = vec![0u8; frame.len() + 5];
        let got = self.bridge.with(|bridge| bridge.bulk(&bulk, &mut answer))?;

        let echoed = Replies::new(&answer[..got])
            .next()
            .ok_or(BridgeError::Short { got, needs: 5 })??
            .frame;
        if echoed.len() < frame.len() {
            return Err(BridgeError::Short {
                got: echoed.len(),
                needs: frame.len(),
            }
            .into());
        }

        let mut at = 0;
        for operation in operations.iter_mut() {
            match operation {
                Operation::Write(bytes) => at += bytes.len(),
                Operation::Transfer(read, write) => {
                    let take = read.len().min(write.len());
                    read[..take].copy_from_slice(&echoed[at..at + take]);
                    at += write.len();
                }
                Operation::Read(buffer) => {
                    buffer.copy_from_slice(&echoed[at..at + buffer.len()]);
                    at += buffer.len();
                }
                Operation::TransferInPlace(buffer) => {
                    buffer.copy_from_slice(&echoed[at..at + buffer.len()]);
                    at += buffer.len();
                }
                Operation::DelayNs(ns) => std::thread::sleep(Duration::from_nanos(u64::from(*ns))),
            }
        }
        Ok(())
    }
}

/// One of the card's pins, driven through the bridge.
pub struct BridgePin<P> {
    bridge: Shared<P>,
    pin: u8,
}

impl<P> digital::ErrorType for BridgePin<P> {
    type Error = UsbError;
}

impl<P: Read + Write> OutputPin for BridgePin<P> {
    fn set_low(&mut self) -> Result<(), UsbError> {
        let pin = self.pin;
        self.bridge.with(|bridge| bridge.write_gpio(pin, false))
    }

    fn set_high(&mut self) -> Result<(), UsbError> {
        let pin = self.pin;
        self.bridge.with(|bridge| bridge.write_gpio(pin, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sx1302::spi as frame;
    use std::collections::VecDeque;

    /// A port that records what is written and answers from a script.
    struct FakePort {
        written: Vec<u8>,
        answers: VecDeque<u8>,
    }

    impl FakePort {
        fn answering(answers: &[&[u8]]) -> FakePort {
            FakePort {
                written: Vec::new(),
                answers: answers.iter().flat_map(|a| a.iter().copied()).collect(),
            }
        }
    }

    impl Read for FakePort {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let mut n = 0;
            while n < buf.len() {
                match self.answers.pop_front() {
                    Some(byte) => {
                        buf[n] = byte;
                        n += 1;
                    }
                    None => break,
                }
            }
            Ok(n)
        }
    }

    impl Write for FakePort {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn answer(order: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = Header {
            id: 0,
            size: payload.len() as u16,
            order: order + ACK,
        }
        .to_bytes()
        .to_vec();
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn a_ping_goes_out_as_an_empty_message_and_the_identity_comes_back() {
        let mut ping = [0u8; bridge::PING_LEN];
        ping[12..].copy_from_slice(b"V01.00.00");
        let mut bridge = Bridge::new(FakePort::answering(&[&answer(ORDER_PING, &ping)]));

        let identity = bridge.ping().expect("the bridge answers");
        assert!(identity.matches_firmware());
        assert_eq!(bridge.port.written, [0, 0, 0, ORDER_PING], "no payload");
    }

    #[test]
    fn a_register_read_through_the_driver_path_finds_the_chip_id() {
        // The reference reads the version register; the bridge echoes the frame with the
        // chip's answer in the last byte.
        let out = frame::read(frame::TARGET_CONCENTRATOR, 0x5600);
        let mut echoed = out.to_vec();
        *echoed.last_mut().unwrap() = 0x60;
        let mut reply = vec![0u8, 0x01, 0, 0, echoed.len() as u8];
        reply.extend_from_slice(&echoed);

        let shared = Shared::new(Bridge::new(FakePort::answering(&[&answer(
            ORDER_SPI, &reply,
        )])));
        let mut spi = shared.concentrator();

        let mut back = [0u8; frame::READ_LEN];
        spi.transfer(&mut back, &out).expect("the transfer runs");
        assert_eq!(frame::read_value(&back), Some(0x60));

        // What went down the port: a header for one bulk order, then one entry carrying the
        // frame the driver built, byte for byte.
        let written = shared.with(|b| Ok(b.port.written.clone())).unwrap();
        assert_eq!(&written[..4], &[0, 0, (5 + out.len()) as u8, ORDER_SPI]);
        assert_eq!(
            &written[4..9],
            &[0, 0x01, TARGET_CONCENTRATOR, 0, out.len() as u8]
        );
        assert_eq!(&written[9..], &out);
    }

    #[test]
    fn a_burst_read_is_one_frame_and_the_read_takes_its_own_slice_of_the_echo() {
        let header = frame::burst_read_header(frame::TARGET_CONCENTRATOR, 0x1000);
        let data = [0x11u8, 0x22, 0x33];
        let mut echoed = header.to_vec();
        echoed.extend_from_slice(&data);
        let mut reply = vec![0u8, 0x01, 0, 0, echoed.len() as u8];
        reply.extend_from_slice(&echoed);

        let shared = Shared::new(Bridge::new(FakePort::answering(&[&answer(
            ORDER_SPI, &reply,
        )])));
        let mut spi = shared.concentrator();

        let mut buffer = [0u8; 3];
        spi.transaction(&mut [Operation::Write(&header), Operation::Read(&mut buffer)])
            .expect("the transfer runs");
        assert_eq!(buffer, data);
    }

    #[test]
    fn the_second_radio_goes_out_under_its_own_target() {
        let out = [0xc0u8, 0x00];
        let mut reply = vec![0u8, 0x01, 0, 0, 2];
        reply.extend_from_slice(&[0xc0, 0x22]);
        let shared = Shared::new(Bridge::new(FakePort::answering(&[&answer(
            ORDER_SPI, &reply,
        )])));
        let mut spi = shared.listener();

        let mut back = [0u8; 2];
        spi.transfer(&mut back, &out).unwrap();
        let written = shared.with(|b| Ok(b.port.written.clone())).unwrap();
        assert_eq!(written[6], TARGET_LISTENER);
    }

    #[test]
    fn a_pin_write_is_a_port_a_pin_and_a_level_and_a_refusal_is_an_error() {
        let shared = Shared::new(Bridge::new(FakePort::answering(&[
            &answer(ORDER_WRITE_GPIO, &[0]),
            &answer(ORDER_WRITE_GPIO, &[1]),
        ])));
        let mut reset = shared.pin(PIN_RESET);

        reset.set_high().expect("the bridge took it");
        let written = shared.with(|b| Ok(b.port.written.clone())).unwrap();
        assert_eq!(&written[4..], &[GPIO_PORT, PIN_RESET, 1]);

        assert!(matches!(
            reset.set_low(),
            Err(UsbError::Bridge(BridgeError::Refused { status: 1 }))
        ));
    }

    #[test]
    fn powering_up_pulses_the_pins_in_the_order_the_reference_does() {
        let ok = answer(ORDER_WRITE_GPIO, &[0]);
        let mut bridge = Bridge::new(FakePort::answering(&[&ok, &ok, &ok, &ok, &ok]));
        bridge.power_up().expect("every write takes");

        let pins: Vec<[u8; 3]> = bridge
            .port
            .written
            .chunks(7)
            .map(|message| [message[4], message[5], message[6]])
            .collect();
        assert_eq!(
            pins,
            [
                [0, PIN_POWER, 1],
                [0, PIN_RESET, 1],
                [0, PIN_RESET, 0],
                [0, PIN_RADIO_RESET, 0],
                [0, PIN_RADIO_RESET, 1],
            ]
        );
    }

    #[test]
    fn an_answer_to_a_different_order_is_refused() {
        let mut bridge = Bridge::new(FakePort::answering(&[&answer(ORDER_PING, &[0u8; 21])]));
        assert!(matches!(
            bridge.status(),
            Err(UsbError::Bridge(BridgeError::WrongAnswer {
                asked: ORDER_STATUS,
                answered: ORDER_PING
            }))
        ));

        let mut bridge = Bridge::new(FakePort::answering(&[&[0u8, 0, 0, ORDER_ERROR]]));
        assert!(matches!(bridge.ping(), Err(UsbError::Rejected)));
    }

    #[test]
    fn message_identifiers_count_up_so_a_trace_can_be_followed() {
        let ok = answer(ORDER_WRITE_GPIO, &[0]);
        let mut bridge = Bridge::new(FakePort::answering(&[&ok, &ok, &ok]));
        for _ in 0..3 {
            bridge.write_gpio(PIN_POWER, true).unwrap();
        }
        let ids: Vec<u8> = bridge.port.written.chunks(7).map(|m| m[0]).collect();
        assert_eq!(ids, [0, 1, 2]);
    }
}
