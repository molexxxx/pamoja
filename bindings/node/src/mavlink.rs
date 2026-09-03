//! Generated Node bindings for the MAVLink wire protocol.
//!
//! These mirror the `pamoja-mavlink` Rust API. MAVLink is the language drones
//! speak, so talking to a PX4 or ArduPilot autopilot means putting exactly the
//! right bytes on the wire and trusting the bytes that come back: v1 and v2
//! frames, the CRC-16/MCRF4XX checksum every frame carries, the per-message
//! `CRC_EXTRA` seed that catches a frame whose shape does not match, and
//! MAVLink 2 signing.
//!
//! # Any dialect, not only the common one
//!
//! A receiver must know a message's `CRC_EXTRA` before it can check the frame
//! carrying it. The common dialect's seeds are built in, but a vehicle running a
//! vendor or private dialect uses ids this build has never heard of.
//! [`messageCrcExtra`](message_crc_extra) derives a seed from a message
//! definition the way the specification does, and a [`Dialect`] carries the
//! results, taking precedence over the built-in registry.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use pamoja_mavlink::dialect::{crc_extra as common_crc_extra, RawMessage};
use pamoja_mavlink::{
    crc16_mcrf4xx as core_crc16, message_crc_extra as core_crc_extra, signing, Frame as CoreFrame,
    Header, MavlinkError, Parser as CoreParser, Signer as CoreSigner, Verifier as CoreVerifier,
    Version,
};

/// The largest payload a frame can carry, in bytes.
#[napi]
pub const MAVLINK_MAX_PAYLOAD: u32 = pamoja_mavlink::MAX_PAYLOAD as u32;

/// The largest frame, in bytes, header, checksum and signature included.
#[napi]
pub const MAVLINK_MAX_FRAME: u32 = pamoja_mavlink::MAX_FRAME as u32;

/// The length of a v2 signature block, in bytes.
#[napi]
pub const MAVLINK_SIGNATURE_LEN: u32 = pamoja_mavlink::SIGNATURE_LEN as u32;

/// The length of a signing key, in bytes.
#[napi]
pub const MAVLINK_KEY_LEN: u32 = signing::KEY_LEN as u32;

/// The default window a verifier accepts a timestamp within.
#[napi]
pub const MAVLINK_DEFAULT_TIMESTAMP_WINDOW: f64 = signing::DEFAULT_TIMESTAMP_WINDOW as f64;

/// Which MAVLink wire format a frame uses.
#[napi(string_enum)]
pub enum MavlinkVersion {
    /// The original six-byte-header format.
    V1,
    /// The current format: a 24-bit message id, flag bytes, and optional signing.
    V2,
}

/// The addressing fields a sender stamps on every frame.
#[napi(object)]
pub struct MavlinkHeader {
    /// The sending system's id.
    pub system_id: u8,
    /// The sending component's id.
    pub component_id: u8,
    /// The sender's sequence number, which wraps at 256.
    pub sequence: u8,
}

impl From<MavlinkHeader> for Header {
    fn from(header: MavlinkHeader) -> Self {
        Header::new(header.system_id, header.component_id, header.sequence)
    }
}

/// One field of a message definition, as the `CRC_EXTRA` derivation reads it.
#[napi(object)]
pub struct MavlinkField {
    /// The field's type name as the dialect writes it, such as `uint8_t`.
    pub type_name: String,
    /// The field's name as the dialect writes it, such as `custom_mode`.
    pub field_name: String,
    /// The element count for an array field; omit or pass `0` for a scalar.
    pub array_len: Option<u8>,
}

/// Turns a MAVLink error into the exception a caller sees.
pub(crate) fn error_of(error: MavlinkError) -> Error {
    let status = match error {
        MavlinkError::Unsigned | MavlinkError::BadSignature | MavlinkError::ReplayedTimestamp => {
            Status::GenericFailure
        }
        _ => Status::InvalidArg,
    };
    Error::new(status, error.to_string())
}

/// Returns the CRC-16/MCRF4XX checksum of a byte string.
///
/// This is the checksum every MAVLink frame carries, exposed because a host that
/// implements part of the protocol itself needs the same arithmetic.
#[napi]
pub fn mavlink_crc16_mcrf4xx(bytes: Buffer) -> u16 {
    core_crc16(&bytes)
}

/// Derives the `CRC_EXTRA` seed of a message from its definition.
///
/// This is what makes a dialect this build has never seen usable: given a
/// message's name and its base fields in wire order, the seed comes out the same
/// as the one the dialect publishes, and a frame carrying that message then
/// checks like any other. Extension fields are excluded from the seed and must
/// not be listed.
#[napi]
pub fn mavlink_message_crc_extra(name: String, fields: Vec<MavlinkField>) -> u8 {
    let described: Vec<(&str, &str, u8)> = fields
        .iter()
        .map(|field| {
            (
                field.type_name.as_str(),
                field.field_name.as_str(),
                field.array_len.unwrap_or(0),
            )
        })
        .collect();
    core_crc_extra(&name, &described)
}

/// Returns the `CRC_EXTRA` the common dialect publishes for a message id, or
/// null for an id outside it, which is what a `Dialect` is for.
#[napi]
pub fn mavlink_known_crc_extra(msgid: u32) -> Option<u8> {
    common_crc_extra(msgid)
}

/// Converts Unix time into the timestamp MAVLink signing counts in.
#[napi]
pub fn mavlink_timestamp_from_unix_micros(unix_micros: f64) -> f64 {
    signing::timestamp_from_unix_micros(unix_micros as u64) as f64
}

/// The `CRC_EXTRA` seeds of a dialect beyond the common one.
///
/// Entries added here are consulted before the built-in common-dialect registry,
/// so a private dialect may also override an id the common one defines.
#[napi]
pub struct Dialect {
    seeds: Vec<(u32, u8)>,
}

impl Dialect {
    /// Returns the seed for a message id, preferring this table.
    fn resolve(&self, msgid: u32) -> Option<u8> {
        self.seeds
            .iter()
            .find(|(id, _)| *id == msgid)
            .map(|(_, crc)| *crc)
            .or_else(|| common_crc_extra(msgid))
    }
}

#[napi]
impl Dialect {
    /// Creates an empty dialect table.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self { seeds: Vec::new() }
    }

    /// Adds or replaces the seed for a message id.
    #[napi]
    pub fn add(&mut self, msgid: u32, crc_extra: u8) {
        match self.seeds.iter_mut().find(|(id, _)| *id == msgid) {
            Some(entry) => entry.1 = crc_extra,
            None => self.seeds.push((msgid, crc_extra)),
        }
    }

    /// Adds a message by its definition, deriving the seed, and returns it.
    ///
    /// This is the whole path for a vendor dialect: describe the message once,
    /// and every frame carrying it checks from then on.
    #[napi]
    pub fn add_message(&mut self, msgid: u32, name: String, fields: Vec<MavlinkField>) -> u8 {
        let crc_extra = mavlink_message_crc_extra(name, fields);
        self.add(msgid, crc_extra);
        crc_extra
    }

    /// Returns the seed this dialect resolves a message id to, or null if
    /// neither it nor the common dialect knows the id.
    #[napi]
    pub fn crc_extra(&self, msgid: u32) -> Option<u8> {
        self.resolve(msgid)
    }
}

/// Looks a message id up in an optional dialect, then the common one.
fn lookup(dialect: Option<&Dialect>, msgid: u32) -> Option<u8> {
    match dialect {
        Some(dialect) => dialect.resolve(msgid),
        None => common_crc_extra(msgid),
    }
}

impl Default for Dialect {
    fn default() -> Self {
        Self::new()
    }
}

/// One MAVLink frame, assembled or received.
#[napi]
pub struct MavlinkFrame {
    inner: CoreFrame,
}

impl MavlinkFrame {
    /// Wraps a frame the engine built, for another module in this crate to hand back.
    pub(crate) fn from_frame(inner: CoreFrame) -> Self {
        Self { inner }
    }

    /// Returns the frame this wraps, for another module in this crate to read.
    pub(crate) fn frame(&self) -> &CoreFrame {
        &self.inner
    }
}

#[napi]
impl MavlinkFrame {
    /// Assembles a v2 frame carrying a message.
    ///
    /// This is the current wire format and what a modern autopilot expects.
    #[napi(factory)]
    pub fn encode_v2(
        header: MavlinkHeader,
        msgid: u32,
        payload: Buffer,
        crc_extra: u8,
    ) -> Result<Self> {
        CoreFrame::encode_v2(header.into(), msgid, &payload, crc_extra)
            .map(|inner| Self { inner })
            .map_err(error_of)
    }

    /// Assembles a v1 frame, for a peer that predates MAVLink 2.
    ///
    /// A v1 frame only carries message ids below 256.
    #[napi(factory)]
    pub fn encode_v1(
        header: MavlinkHeader,
        msgid: u32,
        payload: Buffer,
        crc_extra: u8,
    ) -> Result<Self> {
        CoreFrame::encode_v1(header.into(), msgid, &payload, crc_extra)
            .map(|inner| Self { inner })
            .map_err(error_of)
    }

    /// Parses one frame, checking it against a known `CRC_EXTRA`.
    ///
    /// Throws if the bytes are not a whole frame or the checksum does not match,
    /// which is what rejects a frame mangled in transit.
    #[napi(factory)]
    pub fn parse(bytes: Buffer, crc_extra: u8) -> Result<Self> {
        CoreFrame::parse(&bytes, crc_extra)
            .map(|inner| Self { inner })
            .map_err(error_of)
    }

    /// Parses one frame, looking its `CRC_EXTRA` up as it goes.
    ///
    /// This is what a receiver holding many message types uses: the id comes out
    /// of the frame, and the seed comes from the dialect or the common registry.
    #[napi(factory)]
    pub fn parse_known(bytes: Buffer, dialect: Option<&Dialect>) -> Result<Self> {
        CoreFrame::parse_with(&bytes, |msgid| lookup(dialect, msgid))
            .map(|inner| Self { inner })
            .map_err(error_of)
    }

    /// Assembles a v2 frame carrying a message this build does not type.
    ///
    /// The escape hatch a private dialect needs: supply the id, the payload, and
    /// the seed, and the frame is built and checked like any other.
    #[napi(factory)]
    pub fn raw(header: MavlinkHeader, msgid: u32, crc_extra: u8, payload: Buffer) -> Result<Self> {
        RawMessage {
            msgid,
            crc_extra,
            payload: &payload,
        }
        .to_frame(header.into())
        .map(|inner| Self { inner })
        .map_err(error_of)
    }

    /// Which wire format this frame uses.
    #[napi(getter)]
    pub fn version(&self) -> MavlinkVersion {
        match self.inner.version() {
            Version::V1 => MavlinkVersion::V1,
            Version::V2 => MavlinkVersion::V2,
        }
    }

    /// The addressing fields the frame carries.
    #[napi(getter)]
    pub fn header(&self) -> MavlinkHeader {
        MavlinkHeader {
            system_id: self.inner.system_id(),
            component_id: self.inner.component_id(),
            sequence: self.inner.sequence(),
        }
    }

    /// The id of the message the frame carries.
    #[napi(getter)]
    pub fn message_id(&self) -> u32 {
        self.inner.message_id()
    }

    /// The incompatibility flags a v2 frame declares.
    #[napi(getter)]
    pub fn incompat_flags(&self) -> u8 {
        self.inner.incompat_flags()
    }

    /// Whether the frame carries a signature.
    ///
    /// This says only that the frame was signed, not that the signature is good;
    /// a `Verifier` decides that.
    #[napi(getter)]
    pub fn signed(&self) -> bool {
        self.inner.is_signed()
    }

    /// The message payload.
    ///
    /// A v2 frame drops trailing zero bytes, so a payload can arrive shorter
    /// than the message's full length; a decoder zero-extends it.
    #[napi(getter)]
    pub fn payload(&self) -> Buffer {
        Buffer::from(self.inner.payload().to_vec())
    }

    /// The whole frame, ready to put on the wire.
    #[napi(getter)]
    pub fn bytes(&self) -> Buffer {
        Buffer::from(self.inner.as_bytes().to_vec())
    }

    /// The signature block, or null when the frame is not signed.
    #[napi(getter)]
    pub fn signature(&self) -> Option<Buffer> {
        self.inner
            .signature()
            .map(|signature| Buffer::from(signature.to_vec()))
    }
}

/// A streaming frame parser, and the frames it has completed.
#[napi]
pub struct MavlinkParser {
    parser: CoreParser,
    ready: std::collections::VecDeque<CoreFrame>,
}

#[napi]
impl MavlinkParser {
    /// Creates a parser with an empty buffer.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            parser: CoreParser::new(),
            ready: std::collections::VecDeque::new(),
        }
    }

    /// Feeds bytes off a link and returns the frames that completed.
    ///
    /// Whatever a serial port or socket delivers can be pushed as it arrives,
    /// however it is split. Noise between frames is skipped rather than
    /// reported, which is what lets a parser join a stream already in progress.
    #[napi]
    pub fn push(&mut self, bytes: Buffer, dialect: Option<&Dialect>) -> Vec<MavlinkFrame> {
        let seed = |msgid: u32| lookup(dialect, msgid);
        let mut found = Vec::new();
        for &byte in bytes.as_ref() {
            if let Some(frame) = self.parser.push_byte(byte, &seed) {
                found.push(MavlinkFrame { inner: frame });
            }
        }
        found
    }

    /// Feeds bytes and queues what completed, for a caller that drains later.
    #[napi]
    pub fn feed(&mut self, bytes: Buffer, dialect: Option<&Dialect>) {
        let seed = |msgid: u32| lookup(dialect, msgid);
        for &byte in bytes.as_ref() {
            if let Some(frame) = self.parser.push_byte(byte, &seed) {
                self.ready.push_back(frame);
            }
        }
    }

    /// Takes the next queued frame, or null when the parser needs more bytes.
    #[napi]
    pub fn next_frame(&mut self) -> Option<MavlinkFrame> {
        self.ready.pop_front().map(|inner| MavlinkFrame { inner })
    }

    /// How many completed frames are waiting to be taken.
    #[napi(getter)]
    pub fn pending(&self) -> u32 {
        self.ready.len() as u32
    }
}

impl Default for MavlinkParser {
    fn default() -> Self {
        Self::new()
    }
}

/// A signing key and the monotonic timestamp that goes with it.
#[napi]
pub struct MavlinkSigner {
    inner: CoreSigner,
}

#[napi]
impl MavlinkSigner {
    /// Creates a signer.
    ///
    /// The link id separates two links from one system, so traffic on one does
    /// not look like a replay of the other.
    #[napi(constructor)]
    pub fn new(key: Buffer, link_id: u8, timestamp: f64) -> Result<Self> {
        let key: [u8; signing::KEY_LEN] = key.as_ref().try_into().map_err(|_| {
            Error::new(
                Status::InvalidArg,
                format!("a signing key is {} bytes", signing::KEY_LEN),
            )
        })?;
        Ok(Self {
            inner: CoreSigner::new(key, link_id, timestamp as u64),
        })
    }

    /// Signs a message into a v2 frame.
    ///
    /// Each call advances the timestamp, which is what makes a replayed frame
    /// detectable.
    #[napi]
    pub fn sign(
        &mut self,
        header: MavlinkHeader,
        msgid: u32,
        payload: Buffer,
        crc_extra: u8,
    ) -> Result<MavlinkFrame> {
        self.inner
            .sign(header.into(), msgid, &payload, crc_extra)
            .map(|inner| MavlinkFrame { inner })
            .map_err(error_of)
    }

    /// Which link this signer signs on.
    #[napi(getter)]
    pub fn link_id(&self) -> u8 {
        self.inner.link_id()
    }
}

/// A signing key and the timestamps it has already accepted.
#[napi]
pub struct MavlinkVerifier {
    inner: Option<CoreVerifier>,
}

#[napi]
impl MavlinkVerifier {
    /// Creates a verifier.
    #[napi(constructor)]
    pub fn new(key: Buffer) -> Result<Self> {
        let key: [u8; signing::KEY_LEN] = key.as_ref().try_into().map_err(|_| {
            Error::new(
                Status::InvalidArg,
                format!("a signing key is {} bytes", signing::KEY_LEN),
            )
        })?;
        Ok(Self {
            inner: Some(CoreVerifier::new(key)),
        })
    }

    /// Sets how far a timestamp may run ahead of the last one accepted.
    ///
    /// A wider window tolerates a noisier link; a narrower one narrows the
    /// chance of a replay landing inside it.
    #[napi]
    pub fn set_window(&mut self, window: f64) {
        if let Some(verifier) = self.inner.take() {
            self.inner = Some(verifier.with_window(window as u64));
        }
    }

    /// Checks a frame's signature and its place in the timestamp sequence.
    ///
    /// Throws when the frame is unsigned, the signature does not match the key,
    /// or the timestamp has been seen before.
    #[napi]
    pub fn verify(&mut self, frame: &MavlinkFrame) -> Result<()> {
        let verifier = self.inner.as_mut().ok_or_else(|| {
            Error::new(
                Status::GenericFailure,
                "the verifier is unusable".to_owned(),
            )
        })?;
        verifier.verify(&frame.inner).map_err(error_of)
    }
}
