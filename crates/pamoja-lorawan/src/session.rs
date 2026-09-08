//! The activated session and the data frames it secures.

use crate::crypto::Cipher;
use crate::error::LorawanError;
use crate::frame::{
    Direction, PhyPayload, MAX_FRAME, MAX_PAYLOAD, MTYPE_CONFIRMED_DOWN, MTYPE_CONFIRMED_UP,
    MTYPE_MASK, MTYPE_UNCONFIRMED_DOWN, MTYPE_UNCONFIRMED_UP,
};

// The fixed header bytes of a data frame: MHDR, DevAddr, FCtrl, and FCnt.
const FHDR_LEN: usize = 8;
// The smallest data frame: the fixed header and the MIC, with no port or payload.
const MIN_FRAME: usize = FHDR_LEN + 4;

// FCtrl flag bits.
const FCTRL_ADR: u8 = 0x80;
const FCTRL_ACK: u8 = 0x20;
const FCTRL_FPENDING: u8 = 0x10;
const FCTRL_FOPTS_LEN: u8 = 0x0F;

/// An activated LoRaWAN session: a device address and the two session keys.
///
/// This is the state a device holds once it is activated, whether by personalization
/// (the address and keys provisioned directly) or by a join exchange. It secures every
/// data frame: the network session key authenticates the whole frame through its MIC, and
/// the application session key encrypts the payload, with the device address and frame
/// counter folded into both so a frame is bound to its place in the stream.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::{Session, Uplink};
///
/// let session = Session::new(0x2601_1BDA, [0x11; 16], [0x22; 16]);
/// let frame = session.encode_uplink(&Uplink::new(1, 1, b"hello")).unwrap();
///
/// // The receiver, holding the same session, recovers the payload.
/// let rx = session.decode(frame.as_bytes(), 1).unwrap();
/// assert_eq!(rx.payload(), b"hello");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Session {
    dev_addr: u32,
    nwk_skey: [u8; 16],
    app_skey: [u8; 16],
}

impl Session {
    /// Creates a session from a device address and its two session keys.
    ///
    /// # Arguments
    ///
    /// * `dev_addr` - the device address the network assigned.
    /// * `nwk_skey` - the network session key, which authenticates frames.
    /// * `app_skey` - the application session key, which encrypts payloads.
    ///
    /// # Returns
    ///
    /// The session.
    pub fn new(dev_addr: u32, nwk_skey: [u8; 16], app_skey: [u8; 16]) -> Self {
        Session {
            dev_addr,
            nwk_skey,
            app_skey,
        }
    }

    /// Returns the device address this session is bound to.
    ///
    /// # Returns
    ///
    /// The device address.
    pub fn dev_addr(&self) -> u32 {
        self.dev_addr
    }

    /// Encodes an uplink data frame, encrypting the payload and appending the MIC.
    ///
    /// # Arguments
    ///
    /// * `uplink` - the uplink to send.
    ///
    /// # Returns
    ///
    /// The frame ready for the radio.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::PayloadTooLong`] if the payload and options do not fit a
    /// single frame.
    pub fn encode_uplink(&self, uplink: &Uplink) -> Result<PhyPayload, LorawanError> {
        let mtype = if uplink.confirmed {
            MTYPE_CONFIRMED_UP
        } else {
            MTYPE_UNCONFIRMED_UP
        };
        let mut fctrl = 0;
        if uplink.adr {
            fctrl |= FCTRL_ADR;
        }
        if uplink.ack {
            fctrl |= FCTRL_ACK;
        }
        self.encode(
            Direction::Uplink,
            mtype,
            fctrl,
            uplink.fcnt,
            uplink.fport,
            uplink.fopts,
            uplink.payload,
        )
    }

    /// Encodes a downlink data frame, encrypting the payload and appending the MIC.
    ///
    /// # Arguments
    ///
    /// * `downlink` - the downlink to send.
    ///
    /// # Returns
    ///
    /// The frame ready for the radio.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::PayloadTooLong`] if the payload and options do not fit a
    /// single frame.
    pub fn encode_downlink(&self, downlink: &Downlink) -> Result<PhyPayload, LorawanError> {
        let mtype = if downlink.confirmed {
            MTYPE_CONFIRMED_DOWN
        } else {
            MTYPE_UNCONFIRMED_DOWN
        };
        let mut fctrl = 0;
        if downlink.adr {
            fctrl |= FCTRL_ADR;
        }
        if downlink.ack {
            fctrl |= FCTRL_ACK;
        }
        if downlink.fpending {
            fctrl |= FCTRL_FPENDING;
        }
        self.encode(
            Direction::Downlink,
            mtype,
            fctrl,
            downlink.fcnt,
            downlink.fport,
            downlink.fopts,
            downlink.payload,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn encode(
        &self,
        direction: Direction,
        mtype: u8,
        fctrl: u8,
        fcnt: u32,
        fport: u8,
        fopts: &[u8],
        payload: &[u8],
    ) -> Result<PhyPayload, LorawanError> {
        if fopts.len() > usize::from(FCTRL_FOPTS_LEN) {
            return Err(LorawanError::PayloadTooLong);
        }
        let len = MIN_FRAME + fopts.len() + 1 + payload.len();
        if len > MAX_FRAME {
            return Err(LorawanError::PayloadTooLong);
        }

        let mut buf = [0u8; MAX_FRAME];
        buf[0] = mtype;
        buf[1..5].copy_from_slice(&self.dev_addr.to_le_bytes());
        buf[5] = fctrl | (fopts.len() as u8);
        buf[6..8].copy_from_slice(&(fcnt as u16).to_le_bytes());
        let mut at = FHDR_LEN;
        buf[at..at + fopts.len()].copy_from_slice(fopts);
        at += fopts.len();
        buf[at] = fport;
        at += 1;

        let key = self.payload_key(fport);
        crypt_payload(
            key,
            self.dev_addr,
            direction,
            fcnt,
            payload,
            &mut buf[at..at + payload.len()],
        );
        at += payload.len();

        let mic = self.mic(direction, fcnt, &buf[..at]);
        buf[at..at + 4].copy_from_slice(&mic);
        at += 4;

        PhyPayload::new(&buf[..at])
    }

    /// Decodes a received data frame: verifies the MIC, then decrypts the payload.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the raw frame as it came off the radio.
    /// * `fcnt` - the full 32-bit frame counter expected for this frame; its low 16 bits
    ///   must match the counter the frame carries.
    ///
    /// # Returns
    ///
    /// The decoded frame, with its payload decrypted.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::FrameTooShort`] if the frame is too small,
    /// [`LorawanError::UnsupportedMType`] if it is not a data frame,
    /// [`LorawanError::FcntMismatch`] if the counter does not match, or
    /// [`LorawanError::MicMismatch`] if the MIC does not verify.
    pub fn decode(&self, bytes: &[u8], fcnt: u32) -> Result<RxData, LorawanError> {
        if bytes.len() < MIN_FRAME {
            return Err(LorawanError::FrameTooShort);
        }
        let mtype = bytes[0] & MTYPE_MASK;
        let (direction, confirmed) = match mtype {
            MTYPE_UNCONFIRMED_UP => (Direction::Uplink, false),
            MTYPE_CONFIRMED_UP => (Direction::Uplink, true),
            MTYPE_UNCONFIRMED_DOWN => (Direction::Downlink, false),
            MTYPE_CONFIRMED_DOWN => (Direction::Downlink, true),
            other => return Err(LorawanError::UnsupportedMType(other)),
        };

        let dev_addr = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        let fctrl = bytes[5];
        let fopts_len = usize::from(fctrl & FCTRL_FOPTS_LEN);
        let fcnt_low = u16::from_le_bytes([bytes[6], bytes[7]]);
        if fcnt as u16 != fcnt_low {
            return Err(LorawanError::FcntMismatch);
        }

        let mic_start = bytes.len() - 4;
        let body_start = FHDR_LEN + fopts_len;
        if mic_start < body_start {
            return Err(LorawanError::FrameTooShort);
        }
        let expected = self.mic(direction, fcnt, &bytes[..mic_start]);
        if bytes[mic_start..] != expected[..] {
            return Err(LorawanError::MicMismatch);
        }

        let mut fopts = [0u8; FCTRL_FOPTS_LEN as usize];
        fopts[..fopts_len].copy_from_slice(&bytes[FHDR_LEN..FHDR_LEN + fopts_len]);

        let mut payload = [0u8; MAX_PAYLOAD];
        let (fport, payload_len) = if mic_start > body_start {
            let fport = bytes[body_start];
            let encrypted = &bytes[body_start + 1..mic_start];
            let key = self.payload_key(fport);
            crypt_payload(
                key,
                dev_addr,
                direction,
                fcnt,
                encrypted,
                &mut payload[..encrypted.len()],
            );
            (Some(fport), encrypted.len())
        } else {
            (None, 0)
        };

        Ok(RxData {
            direction,
            dev_addr,
            fcnt_low,
            confirmed,
            adr: fctrl & FCTRL_ADR != 0,
            ack: fctrl & FCTRL_ACK != 0,
            fpending: fctrl & FCTRL_FPENDING != 0,
            fport,
            fopts,
            fopts_len,
            payload,
            payload_len,
        })
    }

    // The key that encrypts a payload: the network key for port 0 (MAC commands), the
    // application key for every other port.
    fn payload_key(&self, fport: u8) -> &[u8; 16] {
        if fport == 0 {
            &self.nwk_skey
        } else {
            &self.app_skey
        }
    }

    // The four-byte MIC over a frame's contents, per the spec's B0 block.
    fn mic(&self, direction: Direction, fcnt: u32, msg: &[u8]) -> [u8; 4] {
        let mut block = [0u8; 16 + MAX_FRAME];
        block[0] = 0x49;
        block[5] = direction.bit();
        block[6..10].copy_from_slice(&self.dev_addr.to_le_bytes());
        block[10..14].copy_from_slice(&fcnt.to_le_bytes());
        block[15] = msg.len() as u8;
        block[16..16 + msg.len()].copy_from_slice(msg);
        let tag = Cipher::new(&self.nwk_skey).cmac(&block[..16 + msg.len()]);
        [tag[0], tag[1], tag[2], tag[3]]
    }
}

// Encrypts (or, being a XOR keystream, decrypts) a payload in place into `output`, per the
// spec's A_i block construction.
fn crypt_payload(
    key: &[u8; 16],
    dev_addr: u32,
    direction: Direction,
    fcnt: u32,
    input: &[u8],
    output: &mut [u8],
) {
    let cipher = Cipher::new(key);
    let blocks = input.len().div_ceil(16);
    for i in 0..blocks {
        let mut a = [0u8; 16];
        a[0] = 0x01;
        a[5] = direction.bit();
        a[6..10].copy_from_slice(&dev_addr.to_le_bytes());
        a[10..14].copy_from_slice(&fcnt.to_le_bytes());
        a[15] = (i + 1) as u8;
        let stream = cipher.encrypt_block(&a);

        let start = i * 16;
        let end = (start + 16).min(input.len());
        for j in start..end {
            output[j] = input[j] ^ stream[j - start];
        }
    }
}

/// An uplink data frame to encode, built up from the fields a sender sets.
///
/// Construct one with [`new`](Uplink::new) and turn on whatever applies; the rest default
/// off. A higher port carries application data; port `0` carries MAC commands.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::Uplink;
///
/// let uplink = Uplink::new(7, 2, b"reading").confirmed().with_adr();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Uplink<'a> {
    fcnt: u32,
    fport: u8,
    payload: &'a [u8],
    confirmed: bool,
    adr: bool,
    ack: bool,
    fopts: &'a [u8],
}

impl<'a> Uplink<'a> {
    /// Creates an unconfirmed uplink with no options set.
    ///
    /// # Arguments
    ///
    /// * `fcnt` - the frame counter for this uplink.
    /// * `fport` - the port; `0` for MAC commands, otherwise an application port.
    /// * `payload` - the application payload to carry.
    ///
    /// # Returns
    ///
    /// The uplink.
    pub fn new(fcnt: u32, fport: u8, payload: &'a [u8]) -> Self {
        Uplink {
            fcnt,
            fport,
            payload,
            confirmed: false,
            adr: false,
            ack: false,
            fopts: &[],
        }
    }

    /// Marks the uplink as confirmed, asking the network to acknowledge it.
    ///
    /// # Returns
    ///
    /// The uplink, for chaining.
    pub fn confirmed(mut self) -> Self {
        self.confirmed = true;
        self
    }

    /// Sets the adaptive-data-rate bit, letting the network manage the data rate.
    ///
    /// # Returns
    ///
    /// The uplink, for chaining.
    pub fn with_adr(mut self) -> Self {
        self.adr = true;
        self
    }

    /// Sets the acknowledgment bit, confirming a previously received downlink.
    ///
    /// # Returns
    ///
    /// The uplink, for chaining.
    pub fn with_ack(mut self) -> Self {
        self.ack = true;
        self
    }

    /// Carries MAC command options in the frame header.
    ///
    /// # Arguments
    ///
    /// * `fopts` - the frame options, up to 15 bytes.
    ///
    /// # Returns
    ///
    /// The uplink, for chaining.
    pub fn with_fopts(mut self, fopts: &'a [u8]) -> Self {
        self.fopts = fopts;
        self
    }
}

/// A downlink data frame to encode, built up from the fields a sender sets.
///
/// Construct one with [`new`](Downlink::new) and turn on whatever applies; the rest
/// default off.
#[derive(Clone, Copy, Debug)]
pub struct Downlink<'a> {
    fcnt: u32,
    fport: u8,
    payload: &'a [u8],
    confirmed: bool,
    adr: bool,
    ack: bool,
    fpending: bool,
    fopts: &'a [u8],
}

impl<'a> Downlink<'a> {
    /// Creates an unconfirmed downlink with no options set.
    ///
    /// # Arguments
    ///
    /// * `fcnt` - the frame counter for this downlink.
    /// * `fport` - the port; `0` for MAC commands, otherwise an application port.
    /// * `payload` - the application payload to carry.
    ///
    /// # Returns
    ///
    /// The downlink.
    pub fn new(fcnt: u32, fport: u8, payload: &'a [u8]) -> Self {
        Downlink {
            fcnt,
            fport,
            payload,
            confirmed: false,
            adr: false,
            ack: false,
            fpending: false,
            fopts: &[],
        }
    }

    /// Marks the downlink as confirmed, asking the device to acknowledge it.
    ///
    /// # Returns
    ///
    /// The downlink, for chaining.
    pub fn confirmed(mut self) -> Self {
        self.confirmed = true;
        self
    }

    /// Sets the adaptive-data-rate bit.
    ///
    /// # Returns
    ///
    /// The downlink, for chaining.
    pub fn with_adr(mut self) -> Self {
        self.adr = true;
        self
    }

    /// Sets the acknowledgment bit, confirming a previously received uplink.
    ///
    /// # Returns
    ///
    /// The downlink, for chaining.
    pub fn with_ack(mut self) -> Self {
        self.ack = true;
        self
    }

    /// Sets the frame-pending bit, signaling more downlinks are waiting.
    ///
    /// # Returns
    ///
    /// The downlink, for chaining.
    pub fn with_fpending(mut self) -> Self {
        self.fpending = true;
        self
    }

    /// Carries MAC command options in the frame header.
    ///
    /// # Arguments
    ///
    /// * `fopts` - the frame options, up to 15 bytes.
    ///
    /// # Returns
    ///
    /// The downlink, for chaining.
    pub fn with_fopts(mut self, fopts: &'a [u8]) -> Self {
        self.fopts = fopts;
        self
    }
}

/// A decoded data frame, with its payload decrypted.
///
/// What [`Session::decode`] returns once a frame's MIC has verified: the header fields and
/// the recovered payload, held in fixed buffers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RxData {
    direction: Direction,
    dev_addr: u32,
    fcnt_low: u16,
    confirmed: bool,
    adr: bool,
    ack: bool,
    fpending: bool,
    fport: Option<u8>,
    fopts: [u8; FCTRL_FOPTS_LEN as usize],
    fopts_len: usize,
    payload: [u8; MAX_PAYLOAD],
    payload_len: usize,
}

impl RxData {
    /// Returns the direction the frame traveled.
    ///
    /// # Returns
    ///
    /// [`Direction::Uplink`] or [`Direction::Downlink`].
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Returns the device address the frame carried.
    ///
    /// # Returns
    ///
    /// The device address.
    pub fn dev_addr(&self) -> u32 {
        self.dev_addr
    }

    /// Returns the low 16 bits of the frame counter the frame carried.
    ///
    /// # Returns
    ///
    /// The frame counter's low half.
    pub fn fcnt(&self) -> u16 {
        self.fcnt_low
    }

    /// Reports whether the frame is a confirmed frame that expects an acknowledgment.
    ///
    /// # Returns
    ///
    /// `true` for a confirmed frame.
    pub fn confirmed(&self) -> bool {
        self.confirmed
    }

    /// Reports whether the adaptive-data-rate bit is set.
    ///
    /// # Returns
    ///
    /// `true` if the bit is set.
    pub fn adr(&self) -> bool {
        self.adr
    }

    /// Reports whether the acknowledgment bit is set.
    ///
    /// # Returns
    ///
    /// `true` if the bit is set.
    pub fn ack(&self) -> bool {
        self.ack
    }

    /// Reports whether the frame-pending bit is set (downlink only).
    ///
    /// # Returns
    ///
    /// `true` if the bit is set.
    pub fn fpending(&self) -> bool {
        self.fpending
    }

    /// Returns the port the frame was sent on, if it carried a port and payload.
    ///
    /// # Returns
    ///
    /// The port, or [`None`] for a frame with no port or payload.
    pub fn fport(&self) -> Option<u8> {
        self.fport
    }

    /// Returns the frame options carried in the header.
    ///
    /// # Returns
    ///
    /// The frame option bytes, which may be empty.
    pub fn fopts(&self) -> &[u8] {
        &self.fopts[..self.fopts_len]
    }

    /// Returns the decrypted payload.
    ///
    /// # Returns
    ///
    /// The application payload, which may be empty.
    pub fn payload(&self) -> &[u8] {
        &self.payload[..self.payload_len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NWK_SKEY: [u8; 16] = [0x01; 16];
    const APP_SKEY: [u8; 16] = [0x02; 16];
    const DEV_ADDR: u32 = 0x2601_1BDA;

    fn session() -> Session {
        Session::new(DEV_ADDR, NWK_SKEY, APP_SKEY)
    }

    #[test]
    fn an_uplink_round_trips() {
        let session = session();
        let frame = session
            .encode_uplink(&Uplink::new(10, 1, b"temperature"))
            .unwrap();
        let rx = session.decode(frame.as_bytes(), 10).unwrap();
        assert_eq!(rx.direction(), Direction::Uplink);
        assert_eq!(rx.dev_addr(), DEV_ADDR);
        assert_eq!(rx.fcnt(), 10);
        assert_eq!(rx.fport(), Some(1));
        assert_eq!(rx.payload(), b"temperature");
        assert!(!rx.confirmed());
    }

    #[test]
    fn the_payload_is_encrypted_on_the_wire() {
        let session = session();
        let frame = session
            .encode_uplink(&Uplink::new(1, 1, b"secret"))
            .unwrap();
        // The plaintext must not appear in the encoded frame.
        assert!(frame
            .as_bytes()
            .windows(b"secret".len())
            .all(|window| window != b"secret"));
    }

    #[test]
    fn the_header_is_laid_out_as_the_spec_requires() {
        let session = session();
        let frame = session
            .encode_uplink(&Uplink::new(0x0102, 1, b"x"))
            .unwrap();
        let bytes = frame.as_bytes();
        assert_eq!(bytes[0], MTYPE_UNCONFIRMED_UP);
        // DevAddr little-endian.
        assert_eq!(&bytes[1..5], &DEV_ADDR.to_le_bytes());
        // FCnt little-endian, low 16 bits.
        assert_eq!(&bytes[6..8], &0x0102u16.to_le_bytes());
    }

    #[test]
    fn a_confirmed_downlink_round_trips_with_its_flags() {
        let session = session();
        let frame = session
            .encode_downlink(&Downlink::new(5, 2, b"cmd").confirmed().with_fpending())
            .unwrap();
        let rx = session.decode(frame.as_bytes(), 5).unwrap();
        assert_eq!(rx.direction(), Direction::Downlink);
        assert!(rx.confirmed());
        assert!(rx.fpending());
        assert_eq!(rx.payload(), b"cmd");
    }

    #[test]
    fn frame_options_round_trip() {
        let session = session();
        let frame = session
            .encode_uplink(&Uplink::new(3, 1, b"d").with_fopts(&[0x02, 0x03]))
            .unwrap();
        let rx = session.decode(frame.as_bytes(), 3).unwrap();
        assert_eq!(rx.fopts(), &[0x02, 0x03]);
        assert_eq!(rx.payload(), b"d");
    }

    #[test]
    fn an_empty_payload_round_trips() {
        let session = session();
        let frame = session.encode_uplink(&Uplink::new(1, 1, b"")).unwrap();
        let rx = session.decode(frame.as_bytes(), 1).unwrap();
        assert_eq!(rx.payload(), b"");
        assert_eq!(rx.fport(), Some(1));
    }

    #[test]
    fn a_tampered_payload_fails_the_mic() {
        let session = session();
        let frame = session.encode_uplink(&Uplink::new(1, 1, b"data")).unwrap();
        let mut bytes = frame.as_bytes().to_vec();
        let last = bytes.len() - 5; // a payload byte, before the 4-byte MIC
        bytes[last] ^= 0xff;
        assert_eq!(session.decode(&bytes, 1), Err(LorawanError::MicMismatch));
    }

    #[test]
    fn the_wrong_counter_is_rejected() {
        let session = session();
        let frame = session.encode_uplink(&Uplink::new(7, 1, b"data")).unwrap();
        assert_eq!(
            session.decode(frame.as_bytes(), 8),
            Err(LorawanError::FcntMismatch)
        );
    }

    #[test]
    fn a_join_frame_is_not_decoded_here() {
        let session = session();
        // MHDR 0x00 is a join-request, not a data frame.
        let bytes = [0u8; MIN_FRAME];
        assert_eq!(
            session.decode(&bytes, 0),
            Err(LorawanError::UnsupportedMType(0x00))
        );
    }

    #[test]
    fn a_short_frame_is_rejected() {
        let session = session();
        assert_eq!(
            session.decode(&[0x40, 0x00, 0x00], 0),
            Err(LorawanError::FrameTooShort)
        );
    }

    #[test]
    fn port_zero_uses_the_network_key() {
        // A port-0 payload is encrypted with the network key, so decoding it with a
        // session whose application key differs still recovers it.
        let session = Session::new(DEV_ADDR, NWK_SKEY, APP_SKEY);
        let frame = session.encode_uplink(&Uplink::new(1, 0, b"mac")).unwrap();
        let other = Session::new(DEV_ADDR, NWK_SKEY, [0x33; 16]);
        let rx = other.decode(frame.as_bytes(), 1).unwrap();
        assert_eq!(rx.payload(), b"mac");
    }

    #[test]
    fn the_largest_payload_round_trips() {
        let session = session();
        let payload = [0xAB; MAX_PAYLOAD];
        let frame = session.encode_uplink(&Uplink::new(1, 1, &payload)).unwrap();
        assert_eq!(frame.as_bytes().len(), crate::MAX_FRAME);
        let rx = session.decode(frame.as_bytes(), 1).unwrap();
        assert_eq!(rx.payload(), &payload[..]);
    }

    #[test]
    fn the_full_frame_counter_is_bound_into_the_mic() {
        let session = session();
        // Only the low 16 bits of the counter travel on the wire, but the whole 32-bit
        // value is folded into the MIC.
        let frame = session
            .encode_uplink(&Uplink::new(0x0001_0001, 1, b"x"))
            .unwrap();
        // The right low bits but the wrong upper bits must still fail the MIC.
        assert_eq!(
            session.decode(frame.as_bytes(), 0x0000_0001),
            Err(LorawanError::MicMismatch)
        );
        // The full counter verifies.
        let rx = session.decode(frame.as_bytes(), 0x0001_0001).unwrap();
        assert_eq!(rx.fcnt(), 0x0001);
    }

    #[test]
    fn another_sessions_keys_cannot_read_a_frame() {
        let session = session();
        let frame = session
            .encode_uplink(&Uplink::new(1, 1, b"secret"))
            .unwrap();
        let stranger = Session::new(DEV_ADDR, [0xAA; 16], [0xBB; 16]);
        assert_eq!(
            stranger.decode(frame.as_bytes(), 1),
            Err(LorawanError::MicMismatch)
        );
    }
}
