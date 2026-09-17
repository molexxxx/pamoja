//! Reaching a concentrator through the microcontroller on a USB card.
//!
//! A CoreCell on SPI is wired straight to the host. The USB version of the same card puts
//! an STM32 between them: the host talks to the microcontroller over a serial port, and the
//! microcontroller does the SPI on the host's behalf, drives the reset and supply pins, and
//! reports its own clock and temperature. Everything the concentrator driver sends still
//! goes out as the same SPI frames; they are carried inside a message to the bridge rather
//! than clocked out by the host.
//!
//! Every message is a four byte header, an identifier the host picks, a size, and an order,
//! then that many bytes of payload. The bridge answers with the same shape, the order moved
//! up by [`ACK`]. One order carries many SPI transfers at once: each is an entry naming a
//! target, a length, and the raw frame, and the answer echoes every entry with a status and
//! the bytes the chip clocked back, which is where a read finds its value.
//!
//! This module builds those messages and reads the answers. It opens no port.

/// The most bytes one message carries, on either side.
pub const MAX_MESSAGE: usize = 4200;

/// The most bytes of SPI entries one bulk order carries.
pub const BULK_CHUNK: usize = 4096;

/// The most SPI entries one bulk order carries.
pub const BULK_ENTRIES: usize = 255;

/// How long a header is: an identifier, a size, and an order.
pub const HEADER_LEN: usize = 4;

/// The bridge firmware this crate was written against, as the ping reports it after its
/// first character.
///
/// The first character says whether the firmware is a release or a debug build, and the
/// reference compares what follows. A different version is a warning there rather than a
/// refusal, and it is here too.
pub const FIRMWARE_VERSION: &str = "01.00.00";

/// What the bridge adds to an order to answer it.
pub const ACK: u8 = 0x40;

/// The bridge answering that it could not read an order.
pub const ORDER_ERROR: u8 = 0xff;

/// Asks the bridge who it is: a unique identifier and a firmware version.
pub const ORDER_PING: u8 = 0x00;

/// Asks the bridge for its clock and its temperature.
pub const ORDER_STATUS: u8 = 0x01;

/// Drops the bridge into its bootloader, for a firmware update.
pub const ORDER_BOOTLOADER: u8 = 0x02;

/// Resets the card.
pub const ORDER_RESET: u8 = 0x03;

/// Drives one of the bridge's pins.
pub const ORDER_WRITE_GPIO: u8 = 0x04;

/// Carries SPI transfers to the concentrator, or the radio beside it.
pub const ORDER_SPI: u8 = 0x05;

/// The nominal speed the port is opened at.
///
/// A USB serial port does not really have one, so this only has to be a value the port
/// accepts; the reference uses it and so does this crate.
pub const BAUD: u32 = 115_200;

/// The bridge pin port every card pin is on.
pub const GPIO_PORT: u8 = 0;

/// The pin that switches the concentrator's supply on.
pub const PIN_POWER: u8 = 1;

/// The pin that resets the concentrator, active high.
pub const PIN_RESET: u8 = 2;

/// The pin that resets the radio beside the concentrator, active low.
pub const PIN_RADIO_RESET: u8 = 8;

/// The concentrator, and the front ends behind it, as an SPI target.
pub const TARGET_CONCENTRATOR: u8 = 0;

/// The radio beside the concentrator that listens before it talks.
pub const TARGET_LISTENER: u8 = 1;

/// An SPI entry that clocks a frame out and the answer back.
pub const ENTRY_TRANSFER: u8 = 0x01;

/// An SPI entry that changes some bits of one register, done on the bridge.
pub const ENTRY_MODIFY: u8 = 0x02;

/// How many bytes a ping answers with.
pub const PING_LEN: usize = 21;

/// How many bytes a status answers with.
pub const STATUS_LEN: usize = 6;

/// The header on every message, either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// An identifier the host picks. The bridge does not check it and the reference does
    /// not compare it on the way back, so it is a courtesy rather than a guard.
    pub id: u8,
    /// How many bytes of payload follow.
    pub size: u16,
    /// What the message is, or, from the bridge, what it answers.
    pub order: u8,
}

impl Header {
    /// The bytes that go out.
    ///
    /// # Returns
    ///
    /// The identifier, the size high byte first, then the order.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::bridge::{Header, ORDER_WRITE_GPIO};
    ///
    /// let header = Header { id: 7, size: 3, order: ORDER_WRITE_GPIO };
    /// assert_eq!(header.to_bytes(), [7, 0, 3, 0x04]);
    /// ```
    #[must_use]
    pub const fn to_bytes(self) -> [u8; HEADER_LEN] {
        [self.id, (self.size >> 8) as u8, self.size as u8, self.order]
    }

    /// Reads a header the bridge sent.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the four bytes.
    ///
    /// # Returns
    ///
    /// The header.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; HEADER_LEN]) -> Header {
        Header {
            id: bytes[0],
            size: ((bytes[1] as u16) << 8) | bytes[2] as u16,
            order: bytes[3],
        }
    }

    /// Whether this header answers an order, rather than carrying one.
    ///
    /// # Returns
    ///
    /// Whether the order is in the range the bridge answers with.
    #[must_use]
    pub const fn is_answer(&self) -> bool {
        self.order >= ACK && self.order <= ACK + 6
    }

    /// Whether this header answers a particular order.
    ///
    /// # Arguments
    ///
    /// * `order` - the order that was sent.
    ///
    /// # Returns
    ///
    /// Whether the bridge answered that one.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::bridge::{Header, ACK, ORDER_PING};
    ///
    /// let answer = Header::from_bytes([7, 0, 21, ORDER_PING + ACK]);
    /// assert!(answer.answers(ORDER_PING));
    /// assert!(!answer.answers(ORDER_PING + 1));
    /// ```
    #[must_use]
    pub const fn answers(&self, order: u8) -> bool {
        self.order == order + ACK
    }
}

/// What the bridge refused, or answered wrongly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeError {
    /// The bridge answered a different order than the one sent.
    WrongAnswer {
        /// What was asked.
        asked: u8,
        /// What came back.
        answered: u8,
    },
    /// The answer is shorter than that order's answer is.
    Short {
        /// How many bytes came back.
        got: usize,
        /// How many the answer needs.
        needs: usize,
    },
    /// A pin write or a reset did not take.
    Refused {
        /// The status the bridge gave.
        status: u8,
    },
    /// One SPI entry in a bulk order failed.
    Transfer {
        /// Which entry.
        id: u8,
        /// How, as the bridge reports it: 1 failed, 2 a wrong parameter, 3 a timeout.
        status: u8,
    },
    /// An SPI entry in the answer is not one of the two kinds.
    UnknownEntry {
        /// Which entry.
        id: u8,
        /// What kind it claimed to be.
        kind: u8,
    },
    /// A bulk order holds more than the bridge takes.
    TooMany,
}

impl core::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BridgeError::WrongAnswer { asked, answered } => write!(
                f,
                "the bridge answered order {answered:#04x} when {asked:#04x} was sent"
            ),
            BridgeError::Short { got, needs } => {
                write!(f, "the bridge answered {got} bytes and {needs} were needed")
            }
            BridgeError::Refused { status } => {
                write!(f, "the bridge refused with status {status}")
            }
            BridgeError::Transfer { id, status } => write!(
                f,
                "SPI entry {id} failed on the bridge with status {status}"
            ),
            BridgeError::UnknownEntry { id, kind } => {
                write!(f, "SPI entry {id} came back as kind {kind:#04x}")
            }
            BridgeError::TooMany => {
                f.write_str("a bulk order holds more entries or bytes than the bridge takes")
            }
        }
    }
}

impl core::error::Error for BridgeError {}

/// Who a bridge says it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    /// The microcontroller's unique identifier, twelve bytes.
    pub unique_id: [u8; 12],
    /// The firmware version, nine characters such as `V01.00.00`.
    pub version: [u8; 9],
}

impl Identity {
    /// Reads a ping's answer.
    ///
    /// # Arguments
    ///
    /// * `payload` - what followed the header.
    ///
    /// # Returns
    ///
    /// The identity.
    ///
    /// # Errors
    ///
    /// [`BridgeError::Short`] when fewer than [`PING_LEN`] bytes came back.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::bridge::Identity;
    ///
    /// let mut payload = [0u8; 21];
    /// payload[12..].copy_from_slice(b"V01.00.00");
    /// let identity = Identity::parse(&payload).unwrap();
    /// assert_eq!(identity.version_str(), Some("V01.00.00"));
    /// assert!(identity.matches_firmware());
    /// ```
    pub fn parse(payload: &[u8]) -> Result<Identity, BridgeError> {
        if payload.len() < PING_LEN {
            return Err(BridgeError::Short {
                got: payload.len(),
                needs: PING_LEN,
            });
        }
        let mut unique_id = [0u8; 12];
        unique_id.copy_from_slice(&payload[..12]);
        let mut version = [0u8; 9];
        version.copy_from_slice(&payload[12..21]);
        Ok(Identity { unique_id, version })
    }

    /// The version as text.
    ///
    /// # Returns
    ///
    /// The nine characters, or `None` when the bridge sent something other than text.
    #[must_use]
    pub fn version_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.version).ok()
    }

    /// Whether the bridge runs the firmware this crate was written against.
    ///
    /// # Returns
    ///
    /// Whether the version after its first character is [`FIRMWARE_VERSION`]. The first
    /// character only says release or debug.
    #[must_use]
    pub fn matches_firmware(&self) -> bool {
        self.version_str()
            .and_then(|text| text.get(1..))
            .is_some_and(|rest| rest == FIRMWARE_VERSION)
    }
}

/// How a bridge is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    /// The bridge's own clock, in milliseconds since it started.
    pub system_time_ms: u32,
    /// The temperature on the card, in hundredths of a degree Celsius.
    pub temperature_centi: i16,
}

impl Status {
    /// Reads a status answer.
    ///
    /// # Arguments
    ///
    /// * `payload` - what followed the header.
    ///
    /// # Returns
    ///
    /// The status.
    ///
    /// # Errors
    ///
    /// [`BridgeError::Short`] when fewer than [`STATUS_LEN`] bytes came back.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::bridge::Status;
    ///
    /// // 1000 ms up, 24.50 degrees.
    /// let status = Status::parse(&[0, 0, 0x03, 0xe8, 0x09, 0x92]).unwrap();
    /// assert_eq!(status.system_time_ms, 1000);
    /// assert_eq!(status.temperature_centi, 2450);
    /// ```
    pub fn parse(payload: &[u8]) -> Result<Status, BridgeError> {
        if payload.len() < STATUS_LEN {
            return Err(BridgeError::Short {
                got: payload.len(),
                needs: STATUS_LEN,
            });
        }
        Ok(Status {
            system_time_ms: u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]),
            temperature_centi: i16::from_be_bytes([payload[4], payload[5]]),
        })
    }

    /// The temperature in degrees Celsius.
    ///
    /// # Returns
    ///
    /// The value, to a hundredth.
    #[must_use]
    pub fn celsius(&self) -> f32 {
        f32::from(self.temperature_centi) / 100.0
    }
}

/// The payload that drives a pin.
///
/// # Arguments
///
/// * `port` - the pin's port, which is [`GPIO_PORT`] for every pin on the card.
/// * `pin` - which pin.
/// * `high` - whether to drive it high.
///
/// # Returns
///
/// The three bytes.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::bridge::{write_gpio, GPIO_PORT, PIN_RESET};
///
/// assert_eq!(write_gpio(GPIO_PORT, PIN_RESET, true), [0, 2, 1]);
/// ```
#[must_use]
pub const fn write_gpio(port: u8, pin: u8, high: bool) -> [u8; 3] {
    [port, pin, high as u8]
}

/// The payload that resets the card.
///
/// # Returns
///
/// The one byte, naming the only reset the bridge has.
#[must_use]
pub const fn reset() -> [u8; 1] {
    [0]
}

/// Reads the one byte status a pin write or a reset answers with.
///
/// # Arguments
///
/// * `payload` - what followed the header.
///
/// # Returns
///
/// Nothing, when the bridge did it.
///
/// # Errors
///
/// [`BridgeError::Short`] when nothing came back, and [`BridgeError::Refused`] with the
/// status when the bridge could not do it.
pub fn took(payload: &[u8]) -> Result<(), BridgeError> {
    match payload.first() {
        None => Err(BridgeError::Short { got: 0, needs: 1 }),
        Some(0) => Ok(()),
        Some(&status) => Err(BridgeError::Refused { status }),
    }
}

/// SPI transfers gathered into one order to the bridge.
///
/// Each transfer is an entry: who it is for, how long it is, and the raw frame the chip
/// sees, target byte first, exactly as [`spi`](super::spi) builds it. The bridge answers
/// with every entry echoed and the bytes the chip clocked back in place of the frame, which
/// is what [`Replies`] walks.
#[derive(Clone, Debug)]
pub struct Bulk {
    bytes: [u8; BULK_CHUNK],
    len: usize,
    entries: u8,
}

impl Default for Bulk {
    fn default() -> Bulk {
        Bulk::new()
    }
}

impl Bulk {
    /// An empty order.
    ///
    /// # Returns
    ///
    /// Room for [`BULK_CHUNK`] bytes of entries.
    #[must_use]
    pub const fn new() -> Bulk {
        Bulk {
            bytes: [0; BULK_CHUNK],
            len: 0,
            entries: 0,
        }
    }

    /// Adds a transfer.
    ///
    /// # Arguments
    ///
    /// * `target` - [`TARGET_CONCENTRATOR`] or [`TARGET_LISTENER`].
    /// * `frame` - the raw frame, target byte first, as the chip sees it. For a read it
    ///   includes the dummy byte and room for the answer, as [`spi::read`](super::spi::read)
    ///   builds it.
    ///
    /// # Returns
    ///
    /// The entry's identifier, which is its position, so the reply can be matched.
    ///
    /// # Errors
    ///
    /// [`BridgeError::TooMany`] when the order is full, by entries or by bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::bridge::{Bulk, TARGET_CONCENTRATOR};
    /// use pamoja_radios::sx1302::spi;
    ///
    /// let mut bulk = Bulk::new();
    /// let frame = spi::write(spi::TARGET_CONCENTRATOR, 0x5b00, 0x01);
    /// bulk.transfer(TARGET_CONCENTRATOR, &frame).unwrap();
    ///
    /// // Identifier, kind, target, the frame's length, then the frame itself.
    /// assert_eq!(&bulk.as_bytes()[..5], &[0, 0x01, 0, 0, 4]);
    /// assert_eq!(&bulk.as_bytes()[5..], &frame);
    /// ```
    pub fn transfer(&mut self, target: u8, frame: &[u8]) -> Result<u8, BridgeError> {
        let needed = 5 + frame.len();
        if self.entries as usize >= BULK_ENTRIES
            || self.len + needed > BULK_CHUNK
            || frame.len() > u16::MAX as usize
        {
            return Err(BridgeError::TooMany);
        }
        let id = self.entries;
        let at = self.len;
        self.bytes[at] = id;
        self.bytes[at + 1] = ENTRY_TRANSFER;
        self.bytes[at + 2] = target;
        self.bytes[at + 3] = (frame.len() >> 8) as u8;
        self.bytes[at + 4] = frame.len() as u8;
        self.bytes[at + 5..at + needed].copy_from_slice(frame);
        self.len += needed;
        self.entries += 1;
        Ok(id)
    }

    /// Adds a change to some bits of one concentrator register, done on the bridge.
    ///
    /// The bridge reads the register, replaces the field, and writes it back, which saves
    /// the host a round trip on a register it shares with something else.
    ///
    /// # Arguments
    ///
    /// * `address` - the register.
    /// * `offset` - which bit the field starts at.
    /// * `width` - how many bits it spans.
    /// * `value` - what to put there, right aligned.
    ///
    /// # Returns
    ///
    /// The entry's identifier.
    ///
    /// # Errors
    ///
    /// [`BridgeError::TooMany`] when the order is full.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::bridge::Bulk;
    ///
    /// let mut bulk = Bulk::new();
    /// bulk.modify(0x5b21, 4, 2, 0b11).unwrap();
    ///
    /// // A two bit field at bit four is the mask 0x30, and the value moves up to meet it.
    /// assert_eq!(bulk.as_bytes(), &[0, 0x02, 0x5b, 0x21, 0x30, 0x30]);
    /// ```
    pub fn modify(
        &mut self,
        address: u16,
        offset: u8,
        width: u8,
        value: u8,
    ) -> Result<u8, BridgeError> {
        if self.entries as usize >= BULK_ENTRIES || self.len + 6 > BULK_CHUNK {
            return Err(BridgeError::TooMany);
        }
        let id = self.entries;
        let at = self.len;
        let mask = (((1u16 << width) - 1) as u8) << offset;
        self.bytes[at] = id;
        self.bytes[at + 1] = ENTRY_MODIFY;
        self.bytes[at + 2] = (address >> 8) as u8;
        self.bytes[at + 3] = address as u8;
        self.bytes[at + 4] = mask;
        self.bytes[at + 5] = value << offset;
        self.len += 6;
        self.entries += 1;
        Ok(id)
    }

    /// The entries, as the order carries them.
    ///
    /// # Returns
    ///
    /// The bytes that follow the header.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// How many entries are in the order.
    ///
    /// # Returns
    ///
    /// The count.
    #[must_use]
    pub const fn entries(&self) -> u8 {
        self.entries
    }

    /// Whether the order holds nothing yet.
    ///
    /// # Returns
    ///
    /// Whether there are no entries.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries == 0
    }

    /// The header this order goes out under.
    ///
    /// # Arguments
    ///
    /// * `id` - the message identifier to use.
    ///
    /// # Returns
    ///
    /// The header.
    #[must_use]
    pub const fn header(&self, id: u8) -> Header {
        Header {
            id,
            size: self.len as u16,
            order: ORDER_SPI,
        }
    }

    /// Starts over.
    pub fn clear(&mut self) {
        self.len = 0;
        self.entries = 0;
    }
}

/// One entry of a bulk answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reply<'a> {
    /// Which entry, matching what [`Bulk::transfer`] or [`Bulk::modify`] returned.
    pub id: u8,
    /// [`ENTRY_TRANSFER`] or [`ENTRY_MODIFY`].
    pub kind: u8,
    /// The bytes the chip clocked back, in place of the frame that went out. Empty for a
    /// modify, which answers with nothing.
    pub frame: &'a [u8],
}

/// Walks the entries of a bulk answer.
///
/// Every entry is checked as it is reached: a failed one is an error rather than an entry,
/// because a read whose transfer failed holds nothing worth returning.
pub struct Replies<'a> {
    payload: &'a [u8],
    at: usize,
}

impl<'a> Replies<'a> {
    /// Walks what followed the header of a bulk answer.
    ///
    /// # Arguments
    ///
    /// * `payload` - the answer's payload.
    ///
    /// # Returns
    ///
    /// The walk.
    #[must_use]
    pub const fn new(payload: &'a [u8]) -> Replies<'a> {
        Replies { payload, at: 0 }
    }
}

impl<'a> Iterator for Replies<'a> {
    type Item = Result<Reply<'a>, BridgeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.at >= self.payload.len() {
            return None;
        }
        let rest = &self.payload[self.at..];
        if rest.len() < 5 {
            self.at = self.payload.len();
            return Some(Err(BridgeError::Short {
                got: rest.len(),
                needs: 5,
            }));
        }
        let (id, kind, status) = (rest[0], rest[1], rest[2]);
        if kind != ENTRY_TRANSFER && kind != ENTRY_MODIFY {
            self.at = self.payload.len();
            return Some(Err(BridgeError::UnknownEntry { id, kind }));
        }
        if status != 0 {
            self.at = self.payload.len();
            return Some(Err(BridgeError::Transfer { id, status }));
        }
        let frame_len = if kind == ENTRY_TRANSFER {
            usize::from(u16::from_be_bytes([rest[3], rest[4]]))
        } else {
            0
        };
        if rest.len() < 5 + frame_len {
            self.at = self.payload.len();
            return Some(Err(BridgeError::Short {
                got: rest.len(),
                needs: 5 + frame_len,
            }));
        }
        self.at += 5 + frame_len;
        Some(Ok(Reply {
            id,
            kind,
            frame: &rest[5..5 + frame_len],
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sx1302::spi;

    #[test]
    fn a_header_is_the_identifier_the_size_high_byte_first_and_the_order() {
        let header = Header {
            id: 0x2a,
            size: 0x0102,
            order: ORDER_SPI,
        };
        assert_eq!(header.to_bytes(), [0x2a, 0x01, 0x02, 0x05]);
        assert_eq!(Header::from_bytes(header.to_bytes()), header);
    }

    #[test]
    fn an_answer_is_the_order_moved_up_and_nothing_else_is_one() {
        for order in [
            ORDER_PING,
            ORDER_STATUS,
            ORDER_BOOTLOADER,
            ORDER_RESET,
            ORDER_WRITE_GPIO,
            ORDER_SPI,
        ] {
            let answer = Header::from_bytes([0, 0, 0, order + ACK]);
            assert!(answer.is_answer());
            assert!(answer.answers(order));
        }
        assert!(!Header::from_bytes([0, 0, 0, ORDER_SPI]).is_answer());
        assert!(!Header::from_bytes([0, 0, 0, ORDER_ERROR]).is_answer());
    }

    #[test]
    fn a_ping_carries_the_identifier_then_the_version_and_the_first_character_is_ignored() {
        let mut payload = [0u8; PING_LEN];
        payload[..12].copy_from_slice(&[0xde, 0xad, 0xbe, 0xef, 1, 2, 3, 4, 5, 6, 7, 8]);
        payload[12..].copy_from_slice(b"D01.00.00");

        let identity = Identity::parse(&payload).expect("a whole ping");
        assert_eq!(&identity.unique_id[..4], &[0xde, 0xad, 0xbe, 0xef]);
        assert!(
            identity.matches_firmware(),
            "a debug build of the same firmware"
        );

        payload[12..].copy_from_slice(b"V01.00.01");
        assert!(!Identity::parse(&payload).unwrap().matches_firmware());
        assert_eq!(
            Identity::parse(&payload[..20]),
            Err(BridgeError::Short { got: 20, needs: 21 })
        );
    }

    #[test]
    fn a_status_is_a_big_endian_clock_and_a_signed_temperature_in_hundredths() {
        let status = Status::parse(&[0x00, 0x01, 0x86, 0xa0, 0xff, 0x38]).expect("whole");
        assert_eq!(status.system_time_ms, 100_000);
        assert_eq!(status.temperature_centi, -200, "two degrees below zero");
        assert_eq!(status.celsius(), -2.0);
    }

    #[test]
    fn a_pin_write_and_a_reset_answer_with_one_status_byte() {
        assert_eq!(write_gpio(GPIO_PORT, PIN_POWER, true), [0, 1, 1]);
        assert_eq!(write_gpio(GPIO_PORT, PIN_RADIO_RESET, false), [0, 8, 0]);
        assert_eq!(reset(), [0]);

        assert_eq!(took(&[0]), Ok(()));
        assert_eq!(took(&[2]), Err(BridgeError::Refused { status: 2 }));
        assert_eq!(took(&[]), Err(BridgeError::Short { got: 0, needs: 1 }));
    }

    #[test]
    fn a_transfer_entry_carries_the_raw_frame_the_way_the_reference_builds_it() {
        // The reference writes one byte to 0x5b00 on the concentrator: the entry is the
        // request metadata, then the mux byte, the command with the write bit, the address
        // low byte, and the data.
        let mut bulk = Bulk::new();
        let frame = spi::write(spi::TARGET_CONCENTRATOR, 0x5b00, 0xab);
        let id = bulk.transfer(TARGET_CONCENTRATOR, &frame).unwrap();

        assert_eq!(id, 0);
        assert_eq!(
            bulk.as_bytes(),
            &[0, 0x01, 0, 0, 4, 0x00, 0x80 | 0x5b, 0x00, 0xab]
        );

        // A read costs one more byte, the dummy the chip turns the bus around on.
        let read = spi::read(spi::TARGET_RADIO_A, 0x0123);
        let id = bulk.transfer(TARGET_CONCENTRATOR, &read).unwrap();
        assert_eq!(id, 1);
        assert_eq!(&bulk.as_bytes()[9..14], &[1, 0x01, 0, 0, 5]);
        assert_eq!(&bulk.as_bytes()[14..], &[0x01, 0x01, 0x23, 0x00, 0x00]);
        assert_eq!(bulk.entries(), 2);
    }

    #[test]
    fn a_modify_entry_is_an_address_a_mask_and_a_value_moved_into_place() {
        let mut bulk = Bulk::new();
        bulk.modify(0x5b21, 4, 2, 0b11).unwrap();
        assert_eq!(bulk.as_bytes(), &[0, 0x02, 0x5b, 0x21, 0x30, 0x30]);

        bulk.clear();
        bulk.modify(0x0001, 0, 8, 0xff).unwrap();
        assert_eq!(bulk.as_bytes(), &[0, 0x02, 0x00, 0x01, 0xff, 0xff]);
    }

    #[test]
    fn an_order_refuses_more_than_the_bridge_takes() {
        let mut bulk = Bulk::new();
        let frame = [0u8; 1024];
        for _ in 0..3 {
            bulk.transfer(TARGET_CONCENTRATOR, &frame).unwrap();
        }
        // Three fill 3087 of 4096 bytes; a fourth of this size does not fit.
        assert_eq!(
            bulk.transfer(TARGET_CONCENTRATOR, &frame),
            Err(BridgeError::TooMany)
        );

        let mut many = Bulk::new();
        for _ in 0..BULK_ENTRIES {
            many.modify(0, 0, 1, 0).unwrap();
        }
        assert_eq!(many.modify(0, 0, 1, 0), Err(BridgeError::TooMany));
    }

    #[test]
    fn a_bulk_answer_walks_every_entry_and_hands_back_what_the_chip_clocked() {
        // Two transfers and a modify, all fine: the second transfer carries a read's answer.
        let payload = [
            0, 0x01, 0, 0, 4, 0x00, 0xdb, 0x00, 0x00, // entry 0, four echoed bytes
            1, 0x01, 0, 0, 5, 0x00, 0x00, 0x00, 0x00, 0x60, // entry 1, chip id in the last
            2, 0x02, 0, 0, 0, // a modify answers with nothing
        ];
        let replies: Vec<Reply<'_>> = Replies::new(&payload).map(|r| r.unwrap()).collect();

        assert_eq!(replies.len(), 3);
        assert_eq!(replies[1].id, 1);
        assert_eq!(spi::read_value(replies[1].frame), Some(0x60));
        assert_eq!(replies[2].kind, ENTRY_MODIFY);
        assert!(replies[2].frame.is_empty());
    }

    #[test]
    fn a_failed_entry_ends_the_walk_with_why() {
        let payload = [
            0, 0x01, 0, 0, 1, 0xaa, // fine
            1, 0x01, 3, 0, 0, // timed out
            2, 0x02, 0, 0, 0, // never reached
        ];
        let mut replies = Replies::new(&payload);
        assert!(replies.next().unwrap().is_ok());
        assert_eq!(
            replies.next().unwrap(),
            Err(BridgeError::Transfer { id: 1, status: 3 })
        );
        assert!(
            replies.next().is_none(),
            "nothing after a failure is trusted"
        );
    }

    #[test]
    fn an_answer_cut_short_is_refused_rather_than_read_past_its_end() {
        let payload = [0, 0x01, 0, 0, 9, 0xaa, 0xbb];
        assert_eq!(
            Replies::new(&payload).next().unwrap(),
            Err(BridgeError::Short { got: 7, needs: 14 })
        );
        assert_eq!(
            Replies::new(&[0, 0x01, 0]).next().unwrap(),
            Err(BridgeError::Short { got: 3, needs: 5 })
        );
        assert_eq!(
            Replies::new(&[0, 0x07, 0, 0, 0]).next().unwrap(),
            Err(BridgeError::UnknownEntry { id: 0, kind: 0x07 })
        );
    }
}
