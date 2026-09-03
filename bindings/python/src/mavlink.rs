//! Generated Python bindings for the MAVLink wire protocol.
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
//! `message_crc_extra` derives a seed from a message definition the way the
//! specification does, and a [`Dialect`] carries the results, taking precedence
//! over the built-in registry.

use std::sync::Mutex;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_mavlink::dialect::{crc_extra as common_crc_extra, RawMessage};
use pamoja_mavlink::{
    crc16_mcrf4xx as core_crc16, message_crc_extra as core_crc_extra, signing, Frame as CoreFrame,
    Header, MavlinkError, Parser as CoreParser, Signer as CoreSigner, Verifier as CoreVerifier,
    Version,
};

/// Turns a MAVLink error into the exception a caller sees.
fn error_of(error: MavlinkError) -> PyErr {
    PyValueError::new_err(error.to_string())
}

/// The addressing fields a sender stamps on every frame.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, Copy)]
pub struct MavlinkHeader {
    /// The sending system's id.
    #[pyo3(get)]
    system_id: u8,
    /// The sending component's id.
    #[pyo3(get)]
    component_id: u8,
    /// The sender's sequence number, which wraps at 256.
    #[pyo3(get)]
    sequence: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl MavlinkHeader {
    #[new]
    #[pyo3(signature = (system_id, component_id, sequence = 0))]
    fn new(system_id: u8, component_id: u8, sequence: u8) -> Self {
        Self {
            system_id,
            component_id,
            sequence,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "MavlinkHeader(system_id={}, component_id={}, sequence={})",
            self.system_id, self.component_id, self.sequence
        )
    }
}

impl From<MavlinkHeader> for Header {
    fn from(header: MavlinkHeader) -> Self {
        Header::new(header.system_id, header.component_id, header.sequence)
    }
}

/// Returns the CRC-16/MCRF4XX checksum of a byte string.
///
/// This is the checksum every MAVLink frame carries, exposed because a host that
/// implements part of the protocol itself needs the same arithmetic.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_crc16_mcrf4xx(data: Vec<u8>) -> u16 {
    core_crc16(&data)
}

/// Derives the `CRC_EXTRA` seed of a message from its definition.
///
/// This is what makes a dialect this build has never seen usable: given a
/// message's name and its base fields in wire order as `(type, name, array_len)`
/// triples, the seed comes out the same as the one the dialect publishes, and a
/// frame carrying that message then checks like any other.
///
/// Extension fields are excluded from the seed and must not be listed, which is
/// what lets a peer that predates them still check the frame.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_message_crc_extra(name: &str, fields: Vec<(String, String, u8)>) -> u8 {
    let described: Vec<(&str, &str, u8)> = fields
        .iter()
        .map(|(type_name, field_name, array_len)| {
            (type_name.as_str(), field_name.as_str(), *array_len)
        })
        .collect();
    core_crc_extra(name, &described)
}

/// Returns the `CRC_EXTRA` the common dialect publishes for a message id, or
/// `None` for an id outside it, which is what a `Dialect` is for.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_known_crc_extra(msgid: u32) -> Option<u8> {
    common_crc_extra(msgid)
}

/// Converts Unix time into the timestamp MAVLink signing counts in.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_timestamp_from_unix_micros(unix_micros: u64) -> u64 {
    signing::timestamp_from_unix_micros(unix_micros)
}

/// The `CRC_EXTRA` seeds of a dialect beyond the common one.
///
/// Entries added here are consulted before the built-in common-dialect registry,
/// so a private dialect may also override an id the common one defines.
#[gen_stub_pyclass]
#[pyclass]
pub struct Dialect {
    seeds: Mutex<Vec<(u32, u8)>>,
}

impl Dialect {
    /// Returns the seed for a message id, preferring this table.
    fn resolve(&self, msgid: u32) -> Option<u8> {
        self.seeds
            .lock()
            .ok()
            .and_then(|seeds| {
                seeds
                    .iter()
                    .find(|(id, _)| *id == msgid)
                    .map(|(_, crc)| *crc)
            })
            .or_else(|| common_crc_extra(msgid))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Dialect {
    /// Creates an empty dialect table.
    #[new]
    fn new() -> Self {
        Self {
            seeds: Mutex::new(Vec::new()),
        }
    }

    /// Adds or replaces the seed for a message id.
    fn add(&self, msgid: u32, crc_extra: u8) -> PyResult<()> {
        let mut seeds = self
            .seeds
            .lock()
            .map_err(|_| PyValueError::new_err("the dialect is poisoned"))?;
        match seeds.iter_mut().find(|(id, _)| *id == msgid) {
            Some(entry) => entry.1 = crc_extra,
            None => seeds.push((msgid, crc_extra)),
        }
        Ok(())
    }

    /// Adds a message by its definition, deriving the seed, and returns it.
    ///
    /// This is the whole path for a vendor dialect: describe the message once,
    /// and every frame carrying it checks from then on.
    fn add_message(
        &self,
        msgid: u32,
        name: &str,
        fields: Vec<(String, String, u8)>,
    ) -> PyResult<u8> {
        let crc_extra = mavlink_message_crc_extra(name, fields);
        self.add(msgid, crc_extra)?;
        Ok(crc_extra)
    }

    /// Returns the seed this dialect resolves a message id to, or `None` if
    /// neither it nor the common dialect knows the id.
    fn crc_extra(&self, msgid: u32) -> Option<u8> {
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

/// One MAVLink frame, assembled or received.
#[gen_stub_pyclass]
#[pyclass]
pub struct MavlinkFrame {
    inner: CoreFrame,
}

impl MavlinkFrame {
    /// Wraps a frame the engine built, for another module in this crate to hand back.
    pub(crate) fn from_frame(inner: CoreFrame) -> Self {
        Self { inner }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl MavlinkFrame {
    /// Assembles a v2 frame carrying a message.
    ///
    /// This is the current wire format and what a modern autopilot expects.
    #[staticmethod]
    fn encode_v2(
        header: MavlinkHeader,
        msgid: u32,
        payload: Vec<u8>,
        crc_extra: u8,
    ) -> PyResult<Self> {
        CoreFrame::encode_v2(header.into(), msgid, &payload, crc_extra)
            .map(|inner| Self { inner })
            .map_err(error_of)
    }

    /// Assembles a v1 frame, for a peer that predates MAVLink 2.
    ///
    /// A v1 frame only carries message ids below 256.
    #[staticmethod]
    fn encode_v1(
        header: MavlinkHeader,
        msgid: u32,
        payload: Vec<u8>,
        crc_extra: u8,
    ) -> PyResult<Self> {
        CoreFrame::encode_v1(header.into(), msgid, &payload, crc_extra)
            .map(|inner| Self { inner })
            .map_err(error_of)
    }

    /// Parses one frame, checking it against a known `CRC_EXTRA`.
    ///
    /// Raises `ValueError` if the bytes are not a whole frame or the checksum
    /// does not match, which is what rejects a frame mangled in transit.
    #[staticmethod]
    fn parse(data: Vec<u8>, crc_extra: u8) -> PyResult<Self> {
        CoreFrame::parse(&data, crc_extra)
            .map(|inner| Self { inner })
            .map_err(error_of)
    }

    /// Parses one frame, looking its `CRC_EXTRA` up as it goes.
    ///
    /// This is what a receiver holding many message types uses: the id comes out
    /// of the frame, and the seed comes from the dialect or the common registry.
    #[staticmethod]
    #[pyo3(signature = (data, dialect = None))]
    fn parse_known(data: Vec<u8>, dialect: Option<&Dialect>) -> PyResult<Self> {
        CoreFrame::parse_with(&data, |msgid| lookup(dialect, msgid))
            .map(|inner| Self { inner })
            .map_err(error_of)
    }

    /// Assembles a v2 frame carrying a message this build does not type.
    ///
    /// The escape hatch a private dialect needs: supply the id, the payload, and
    /// the seed, and the frame is built and checked like any other.
    #[staticmethod]
    fn raw(header: MavlinkHeader, msgid: u32, crc_extra: u8, payload: Vec<u8>) -> PyResult<Self> {
        RawMessage {
            msgid,
            crc_extra,
            payload: &payload,
        }
        .to_frame(header.into())
        .map(|inner| Self { inner })
        .map_err(error_of)
    }

    /// Which wire format this frame uses: `1` or `2`.
    #[getter]
    fn version(&self) -> u8 {
        match self.inner.version() {
            Version::V1 => 1,
            Version::V2 => 2,
        }
    }

    /// The addressing fields the frame carries.
    #[getter]
    fn header(&self) -> MavlinkHeader {
        MavlinkHeader {
            system_id: self.inner.system_id(),
            component_id: self.inner.component_id(),
            sequence: self.inner.sequence(),
        }
    }

    /// The id of the message the frame carries.
    #[getter]
    fn message_id(&self) -> u32 {
        self.inner.message_id()
    }

    /// The incompatibility flags a v2 frame declares.
    #[getter]
    fn incompat_flags(&self) -> u8 {
        self.inner.incompat_flags()
    }

    /// Whether the frame carries a signature.
    ///
    /// This says only that the frame was signed, not that the signature is good;
    /// a `MavlinkVerifier` decides that.
    #[getter]
    fn signed(&self) -> bool {
        self.inner.is_signed()
    }

    /// The message payload.
    ///
    /// A v2 frame drops trailing zero bytes, so a payload can arrive shorter
    /// than the message's full length; a decoder zero-extends it.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.payload())
    }

    /// The whole frame, ready to put on the wire.
    #[getter]
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.as_bytes())
    }

    /// The signature block, or `None` when the frame is not signed.
    #[getter]
    fn signature<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.inner
            .signature()
            .map(|signature| PyBytes::new(py, signature))
    }

    fn __repr__(&self) -> String {
        format!(
            "MavlinkFrame(version={}, message_id={}, signed={})",
            self.version(),
            self.message_id(),
            self.signed()
        )
    }
}

/// A streaming frame parser, and the frames it has completed.
#[gen_stub_pyclass]
#[pyclass]
pub struct MavlinkParser {
    inner: Mutex<ParserState>,
}

/// The parser and the frames waiting to be taken from it.
struct ParserState {
    parser: CoreParser,
    ready: std::collections::VecDeque<CoreFrame>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MavlinkParser {
    /// Creates a parser with an empty buffer.
    #[new]
    fn new() -> Self {
        Self {
            inner: Mutex::new(ParserState {
                parser: CoreParser::new(),
                ready: std::collections::VecDeque::new(),
            }),
        }
    }

    /// Feeds bytes off a link and returns the frames that completed.
    ///
    /// Whatever a serial port or socket delivers can be pushed as it arrives,
    /// however it is split. Noise between frames is skipped rather than
    /// reported, which is what lets a parser join a stream already in progress.
    #[pyo3(signature = (data, dialect = None))]
    fn push(&self, data: Vec<u8>, dialect: Option<&Dialect>) -> PyResult<Vec<MavlinkFrame>> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| PyValueError::new_err("the parser is poisoned"))?;
        let seed = |msgid: u32| lookup(dialect, msgid);
        let mut found = Vec::new();
        for byte in data {
            if let Some(frame) = state.parser.push_byte(byte, &seed) {
                found.push(MavlinkFrame { inner: frame });
            }
        }
        Ok(found)
    }

    /// Feeds bytes and queues what completed, for a caller that drains later.
    #[pyo3(signature = (data, dialect = None))]
    fn feed(&self, data: Vec<u8>, dialect: Option<&Dialect>) -> PyResult<()> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| PyValueError::new_err("the parser is poisoned"))?;
        let seed = |msgid: u32| lookup(dialect, msgid);
        for byte in data {
            if let Some(frame) = state.parser.push_byte(byte, &seed) {
                state.ready.push_back(frame);
            }
        }
        Ok(())
    }

    /// Takes the next queued frame, or `None` when the parser needs more bytes.
    fn next_frame(&self) -> PyResult<Option<MavlinkFrame>> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| PyValueError::new_err("the parser is poisoned"))?;
        Ok(state.ready.pop_front().map(|inner| MavlinkFrame { inner }))
    }

    /// How many completed frames are waiting to be taken.
    #[getter]
    fn pending(&self) -> PyResult<usize> {
        let state = self
            .inner
            .lock()
            .map_err(|_| PyValueError::new_err("the parser is poisoned"))?;
        Ok(state.ready.len())
    }
}

/// A signing key and the monotonic timestamp that goes with it.
#[gen_stub_pyclass]
#[pyclass]
pub struct MavlinkSigner {
    inner: Mutex<CoreSigner>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MavlinkSigner {
    /// Creates a signer.
    ///
    /// The link id separates two links from one system, so traffic on one does
    /// not look like a replay of the other.
    #[new]
    #[pyo3(signature = (key, link_id = 0, timestamp = 0))]
    fn new(key: Vec<u8>, link_id: u8, timestamp: u64) -> PyResult<Self> {
        let key: [u8; signing::KEY_LEN] = key.as_slice().try_into().map_err(|_| {
            PyValueError::new_err(format!("a signing key is {} bytes", signing::KEY_LEN))
        })?;
        Ok(Self {
            inner: Mutex::new(CoreSigner::new(key, link_id, timestamp)),
        })
    }

    /// Signs a message into a v2 frame.
    ///
    /// Each call advances the timestamp, which is what makes a replayed frame
    /// detectable.
    fn sign(
        &self,
        header: MavlinkHeader,
        msgid: u32,
        payload: Vec<u8>,
        crc_extra: u8,
    ) -> PyResult<MavlinkFrame> {
        let mut signer = self
            .inner
            .lock()
            .map_err(|_| PyValueError::new_err("the signer is poisoned"))?;
        signer
            .sign(header.into(), msgid, &payload, crc_extra)
            .map(|inner| MavlinkFrame { inner })
            .map_err(error_of)
    }

    /// Which link this signer signs on.
    #[getter]
    fn link_id(&self) -> PyResult<u8> {
        let signer = self
            .inner
            .lock()
            .map_err(|_| PyValueError::new_err("the signer is poisoned"))?;
        Ok(signer.link_id())
    }
}

/// A signing key and the timestamps it has already accepted.
#[gen_stub_pyclass]
#[pyclass]
pub struct MavlinkVerifier {
    inner: Mutex<Option<CoreVerifier>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MavlinkVerifier {
    /// Creates a verifier.
    #[new]
    fn new(key: Vec<u8>) -> PyResult<Self> {
        let key: [u8; signing::KEY_LEN] = key.as_slice().try_into().map_err(|_| {
            PyValueError::new_err(format!("a signing key is {} bytes", signing::KEY_LEN))
        })?;
        Ok(Self {
            inner: Mutex::new(Some(CoreVerifier::new(key))),
        })
    }

    /// Sets how far a timestamp may run ahead of the last one accepted.
    ///
    /// A wider window tolerates a noisier link; a narrower one narrows the
    /// chance of a replay landing inside it.
    fn set_window(&self, window: u64) -> PyResult<()> {
        let mut held = self
            .inner
            .lock()
            .map_err(|_| PyValueError::new_err("the verifier is poisoned"))?;
        if let Some(verifier) = held.take() {
            *held = Some(verifier.with_window(window));
        }
        Ok(())
    }

    /// Checks a frame's signature and its place in the timestamp sequence.
    ///
    /// Raises `ValueError` when the frame is unsigned, the signature does not
    /// match the key, or the timestamp has been seen before.
    fn verify(&self, frame: &MavlinkFrame) -> PyResult<()> {
        let mut held = self
            .inner
            .lock()
            .map_err(|_| PyValueError::new_err("the verifier is poisoned"))?;
        let verifier = held
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("the verifier is unusable"))?;
        verifier.verify(&frame.inner).map_err(error_of)
    }
}
