//! Reading a frame far enough to route it, before any key is involved.
//!
//! [`Session::decode`](crate::Session::decode) needs the session a frame belongs to, but a
//! gateway or network server holds many sessions and has to work out which one a frame is
//! for. That answer is in the header, which travels in the clear: the device address, the
//! frame counter, and what kind of message it is. [`FrameHeader::parse`] reads exactly that
//! much and nothing more, so a receiver can look the session up and then decode.
//!
//! Nothing here is authenticated. The MIC covers the header, but checking it needs the
//! session key, so treat everything this reports as a routing hint until
//! [`Session::decode`](crate::Session::decode) has verified the frame.

use crate::error::LorawanError;
use crate::frame::{
    Direction, MTYPE_CONFIRMED_DOWN, MTYPE_CONFIRMED_UP, MTYPE_JOIN_ACCEPT, MTYPE_JOIN_REQUEST,
    MTYPE_MASK, MTYPE_UNCONFIRMED_DOWN, MTYPE_UNCONFIRMED_UP,
};

// A data frame's fixed header: MHDR, DevAddr, FCtrl, and FCnt, before any frame options.
const FHDR_LEN: usize = 8;
// The shortest data frame is that header plus its MIC.
const MIN_DATA_FRAME: usize = FHDR_LEN + 4;

// The FCtrl bits, as the spec lays them out.
const FCTRL_ADR: u8 = 0x80;
const FCTRL_ACK: u8 = 0x20;
const FCTRL_FPENDING: u8 = 0x10;
const FCTRL_FOPTS_LEN: u8 = 0x0F;

/// What kind of message a frame is, read from its header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageType {
    /// A device asking to join a network.
    JoinRequest,
    /// A network admitting a device.
    JoinAccept,
    /// Data from a device that does not need acknowledging.
    UnconfirmedUp,
    /// Data from a device that asks to be acknowledged.
    ConfirmedUp,
    /// Data to a device that does not need acknowledging.
    UnconfirmedDown,
    /// Data to a device that asks to be acknowledged.
    ConfirmedDown,
}

impl MessageType {
    /// Reports whether this is a data frame rather than part of a join exchange.
    ///
    /// # Returns
    ///
    /// `true` for the four data types, which are the ones carrying a device address.
    pub fn is_data(self) -> bool {
        !matches!(self, MessageType::JoinRequest | MessageType::JoinAccept)
    }

    /// Returns the direction this message type travels.
    ///
    /// # Returns
    ///
    /// The direction, or [`None`] for a join-accept, which the spec secures as a
    /// downlink but which carries no data-frame direction bit.
    pub fn direction(self) -> Option<Direction> {
        match self {
            MessageType::JoinRequest | MessageType::UnconfirmedUp | MessageType::ConfirmedUp => {
                Some(Direction::Uplink)
            }
            MessageType::UnconfirmedDown | MessageType::ConfirmedDown => Some(Direction::Downlink),
            MessageType::JoinAccept => None,
        }
    }
}

/// A frame read only as far as its unencrypted header.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::{FrameHeader, MessageType, Session, Uplink};
///
/// let session = Session::new(0x2601_1BDA, [0x2B; 16], [0x99; 16]);
/// let frame = session.encode_uplink(&Uplink::new(42, 1, b"temp=4.8"))?;
///
/// // A gateway reads the address out of the frame to find the right session.
/// let header = FrameHeader::parse(frame.as_bytes())?;
/// assert_eq!(header.message_type(), MessageType::UnconfirmedUp);
/// assert_eq!(header.dev_addr(), Some(0x2601_1BDA));
/// assert_eq!(header.fcnt(), Some(42));
/// assert_eq!(header.fport(), Some(1));
///
/// // Only then can it verify and decrypt.
/// let rx = session.decode(frame.as_bytes(), 42)?;
/// assert_eq!(rx.payload(), b"temp=4.8");
/// # Ok::<(), pamoja_lorawan::LorawanError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameHeader {
    message_type: MessageType,
    dev_addr: Option<u32>,
    fcnt: Option<u16>,
    fport: Option<u8>,
    adr: bool,
    ack: bool,
    fpending: bool,
    fopts_len: usize,
    payload_len: usize,
}

impl FrameHeader {
    /// Reads a frame far enough to route it, without any key.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the raw frame as it came off the radio.
    ///
    /// # Returns
    ///
    /// What the header says the frame is.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::FrameTooShort`] if the frame is empty or shorter than the
    /// header and MIC a data frame needs, [`LorawanError::UnsupportedMType`] if the message
    /// type is one this crate does not read, or [`LorawanError::MalformedFrame`] if the
    /// frame options run past the end of the frame.
    pub fn parse(bytes: &[u8]) -> Result<FrameHeader, LorawanError> {
        if bytes.is_empty() {
            return Err(LorawanError::FrameTooShort);
        }

        let message_type = match bytes[0] & MTYPE_MASK {
            MTYPE_JOIN_REQUEST => MessageType::JoinRequest,
            MTYPE_JOIN_ACCEPT => MessageType::JoinAccept,
            MTYPE_UNCONFIRMED_UP => MessageType::UnconfirmedUp,
            MTYPE_CONFIRMED_UP => MessageType::ConfirmedUp,
            MTYPE_UNCONFIRMED_DOWN => MessageType::UnconfirmedDown,
            MTYPE_CONFIRMED_DOWN => MessageType::ConfirmedDown,
            other => return Err(LorawanError::UnsupportedMType(other)),
        };

        // A join frame is opaque without the root key, so its type is all there is to read.
        if !message_type.is_data() {
            return Ok(FrameHeader {
                message_type,
                dev_addr: None,
                fcnt: None,
                fport: None,
                adr: false,
                ack: false,
                fpending: false,
                fopts_len: 0,
                payload_len: 0,
            });
        }

        if bytes.len() < MIN_DATA_FRAME {
            return Err(LorawanError::FrameTooShort);
        }

        let fctrl = bytes[5];
        let fopts_len = usize::from(fctrl & FCTRL_FOPTS_LEN);
        let after_fopts = FHDR_LEN + fopts_len;
        if bytes.len() < after_fopts + 4 {
            return Err(LorawanError::MalformedFrame);
        }

        // A port is present only when something follows the frame options.
        let remaining = bytes.len() - after_fopts - 4;
        let (fport, payload_len) = if remaining == 0 {
            (None, 0)
        } else {
            (Some(bytes[after_fopts]), remaining - 1)
        };

        Ok(FrameHeader {
            message_type,
            dev_addr: Some(u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]])),
            fcnt: Some(u16::from_le_bytes([bytes[6], bytes[7]])),
            fport,
            adr: fctrl & FCTRL_ADR != 0,
            ack: fctrl & FCTRL_ACK != 0,
            fpending: fctrl & FCTRL_FPENDING != 0,
            fopts_len,
            payload_len,
        })
    }

    /// Returns what kind of message the frame is.
    ///
    /// # Returns
    ///
    /// The message type.
    pub fn message_type(&self) -> MessageType {
        self.message_type
    }

    /// Returns the direction the frame travels.
    ///
    /// # Returns
    ///
    /// The direction, or [`None`] for a join-accept.
    pub fn direction(&self) -> Option<Direction> {
        self.message_type.direction()
    }

    /// Returns the device address the frame carries.
    ///
    /// This is what a receiver looks a session up by.
    ///
    /// # Returns
    ///
    /// The address, or [`None`] for a join frame, which carries none.
    pub fn dev_addr(&self) -> Option<u32> {
        self.dev_addr
    }

    /// Returns the low 16 bits of the frame counter.
    ///
    /// # Returns
    ///
    /// The counter, or [`None`] for a join frame.
    pub fn fcnt(&self) -> Option<u16> {
        self.fcnt
    }

    /// Returns the port the frame was sent on.
    ///
    /// # Returns
    ///
    /// The port, or [`None`] for a join frame or a data frame carrying only frame
    /// options.
    pub fn fport(&self) -> Option<u8> {
        self.fport
    }

    /// Reports whether the frame asks to be acknowledged.
    ///
    /// # Returns
    ///
    /// `true` for a confirmed data frame.
    pub fn confirmed(&self) -> bool {
        matches!(
            self.message_type,
            MessageType::ConfirmedUp | MessageType::ConfirmedDown
        )
    }

    /// Reports whether the frame takes part in adaptive data rate.
    ///
    /// # Returns
    ///
    /// The ADR bit.
    pub fn adr(&self) -> bool {
        self.adr
    }

    /// Reports whether the frame acknowledges the last confirmed one.
    ///
    /// # Returns
    ///
    /// The ACK bit.
    pub fn ack(&self) -> bool {
        self.ack
    }

    /// Reports whether the network has more downlink data waiting.
    ///
    /// # Returns
    ///
    /// The frame-pending bit.
    pub fn fpending(&self) -> bool {
        self.fpending
    }

    /// Returns how many bytes of frame options the header carries.
    ///
    /// # Returns
    ///
    /// The length, from 0 to 15.
    pub fn fopts_len(&self) -> usize {
        self.fopts_len
    }

    /// Returns the length of the still-encrypted payload.
    ///
    /// # Returns
    ///
    /// The payload length in bytes, which is 0 when the frame carries only options.
    pub fn payload_len(&self) -> usize {
        self.payload_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Device, Downlink, JoinGrant, Session, Uplink};

    const NWK_SKEY: [u8; 16] = [0x2B; 16];
    const APP_SKEY: [u8; 16] = [0x99; 16];
    const DEV_ADDR: u32 = 0x2601_1BDA;

    fn session() -> Session {
        Session::new(DEV_ADDR, NWK_SKEY, APP_SKEY)
    }

    #[test]
    fn an_uplink_reports_the_address_a_receiver_routes_by() {
        let frame = session()
            .encode_uplink(&Uplink::new(42, 1, b"temp=4.8").confirmed().with_adr())
            .unwrap();
        let header = FrameHeader::parse(frame.as_bytes()).unwrap();

        assert_eq!(header.message_type(), MessageType::ConfirmedUp);
        assert_eq!(header.direction(), Some(Direction::Uplink));
        assert_eq!(header.dev_addr(), Some(DEV_ADDR));
        assert_eq!(header.fcnt(), Some(42));
        assert_eq!(header.fport(), Some(1));
        assert!(header.confirmed());
        assert!(header.adr());
        assert!(!header.ack());
        assert_eq!(header.fopts_len(), 0);
        assert_eq!(header.payload_len(), 8);
    }

    #[test]
    fn a_downlink_reports_its_options_and_pending_flag() {
        let fopts = [0x03u8, 0x50, 0x00];
        let frame = session()
            .encode_downlink(
                &Downlink::new(7, 2, b"ack")
                    .with_fpending()
                    .with_fopts(&fopts),
            )
            .unwrap();
        let header = FrameHeader::parse(frame.as_bytes()).unwrap();

        assert_eq!(header.message_type(), MessageType::UnconfirmedDown);
        assert_eq!(header.direction(), Some(Direction::Downlink));
        assert!(header.fpending());
        assert_eq!(header.fopts_len(), fopts.len());
        assert_eq!(header.fport(), Some(2));
        assert_eq!(header.payload_len(), 3);
    }

    #[test]
    fn a_frame_carrying_only_options_has_no_port() {
        let fopts = [0x02u8, 0x01];
        let frame = session()
            .encode_uplink(&Uplink::new(1, 0, b"").with_fopts(&fopts))
            .unwrap();
        let header = FrameHeader::parse(frame.as_bytes()).unwrap();

        assert_eq!(header.fopts_len(), fopts.len());
        assert_eq!(header.payload_len(), 0);
    }

    #[test]
    fn the_join_frames_report_their_type_and_nothing_else() {
        let device = Device::new([0x11; 8], [0x22; 8], [0xAB; 16]);
        let request = FrameHeader::parse(device.join_request(0x1234).as_bytes()).unwrap();
        assert_eq!(request.message_type(), MessageType::JoinRequest);
        assert!(!request.message_type().is_data());
        assert_eq!(request.dev_addr(), None);
        assert_eq!(request.fcnt(), None);
        assert_eq!(request.direction(), Some(Direction::Uplink));

        let grant = JoinGrant::new(0x0003_0201, 0x0006_0504, DEV_ADDR);
        let accept = FrameHeader::parse(grant.accept(&[0xAB; 16], 0x1234).as_bytes()).unwrap();
        assert_eq!(accept.message_type(), MessageType::JoinAccept);
        assert_eq!(
            accept.direction(),
            None,
            "a join-accept carries no direction bit"
        );
    }

    #[test]
    fn the_header_a_gateway_reads_agrees_with_the_decoded_frame() {
        let session = session();
        let frame = session
            .encode_uplink(&Uplink::new(9, 3, b"reading").with_ack())
            .unwrap();
        let header = FrameHeader::parse(frame.as_bytes()).unwrap();
        let rx = session.decode(frame.as_bytes(), 9).unwrap();

        assert_eq!(header.dev_addr(), Some(rx.dev_addr()));
        assert_eq!(header.fcnt(), Some(rx.fcnt()));
        assert_eq!(header.fport(), rx.fport());
        assert_eq!(header.ack(), rx.ack());
        assert_eq!(header.payload_len(), rx.payload().len());
    }

    #[test]
    fn a_truncated_frame_is_refused() {
        assert_eq!(FrameHeader::parse(&[]), Err(LorawanError::FrameTooShort));
        assert_eq!(
            FrameHeader::parse(&[0x40, 0x01, 0x02]),
            Err(LorawanError::FrameTooShort)
        );
    }

    #[test]
    fn frame_options_running_past_the_end_are_malformed() {
        // The FCtrl claims fifteen bytes of options a frame this short cannot hold.
        let mut frame = [0u8; MIN_DATA_FRAME];
        frame[0] = MTYPE_UNCONFIRMED_UP;
        frame[5] = 0x0F;
        assert_eq!(
            FrameHeader::parse(&frame),
            Err(LorawanError::MalformedFrame)
        );
    }

    #[test]
    fn a_message_type_this_crate_does_not_read_is_reported() {
        // 0xC0 is the proprietary message type.
        assert_eq!(
            FrameHeader::parse(&[0xC0; 16]),
            Err(LorawanError::UnsupportedMType(0xC0))
        );
    }
}
