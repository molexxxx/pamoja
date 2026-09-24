//! Generated Node bindings for CAN bus framing.
//!
//! These mirror the `pamoja-can` Rust API: classic CAN 2.0 and CAN-FD frames, the
//! length encoding CAN-FD uses above eight bytes, and the J1939 identifier that
//! trucks, tractors, and gensets ride on top of it.
//!
//! A frame is a small value rather than a resource, so it crosses as a plain
//! object; the identifier a J1939 message decodes to does the same, with a
//! `destination` of `null` for a broadcast rather than a flag to check first.
//! A `CanBus` is a node on a bus, simulated or a kernel interface, whose sends and
//! receives run on a worker thread.

use crate::checked;
use std::time::Duration;

use napi::bindgen_prelude::{spawn_blocking, Buffer};
use napi_derive::napi;
use pamoja_can::bus::{BusError, CanBus, CanBusKind, Filter, OpenError};
use pamoja_can::{
    dlc_to_len, len_to_dlc, priority, CanError, CanId, Frame, J1939Id, Signals, BROADCAST_ADDRESS,
    NOT_AVAILABLE,
};

/// A CAN frame: an identifier, its flags, and its payload.
#[napi(object)]
pub struct CanFrame {
    /// The arbitration identifier, already masked to 11 or 29 bits.
    pub id: checked::u32,
    /// Whether the identifier is a 29-bit extended one.
    pub extended: bool,
    /// Whether this is a CAN-FD frame rather than classic CAN 2.0.
    pub fd: bool,
    /// Whether this is a remote transmission request, which carries no payload.
    pub remote: bool,
    /// The data length: the payload length, or the length a remote frame requests.
    pub len: checked::u8,
    /// The data length code as it appears on the wire.
    pub dlc: checked::u8,
    /// The payload, empty for a remote frame.
    pub data: Buffer,
}

/// The fields J1939 packs into an extended CAN identifier.
#[napi(object)]
pub struct J1939Message {
    /// The parameter group number, which names what the message carries.
    pub pgn: u32,
    /// The message priority, 0 (highest) to 7.
    pub priority: u8,
    /// The source address: the node that sent the message.
    pub source: u8,
    /// The PDU format byte of the parameter group.
    pub pdu_format: u8,
    /// The destination address for an addressed (PDU1) message, or `null` for a
    /// broadcast (PDU2) one.
    pub destination: Option<u8>,
    /// Whether the message is a broadcast.
    pub broadcast: bool,
}

/// Builds a classic CAN 2.0 frame, which carries up to eight bytes.
#[napi]
pub fn can_frame(id: checked::u32, extended: bool, data: Buffer) -> napi::Result<CanFrame> {
    Frame::new(identifier(id.get(), extended), data.as_ref())
        .map(describe)
        .map_err(to_napi)
}

/// Builds a CAN-FD frame, which carries up to 64 bytes at the discrete CAN-FD lengths.
#[napi]
pub fn can_fd_frame(id: checked::u32, extended: bool, data: Buffer) -> napi::Result<CanFrame> {
    Frame::fd(identifier(id.get(), extended), data.as_ref())
        .map(describe)
        .map_err(to_napi)
}

/// Builds a remote transmission request, which asks another node to send.
#[napi]
pub fn can_remote_frame(id: checked::u32, extended: bool, len: checked::u8) -> CanFrame {
    describe(Frame::remote(
        identifier(id.get(), extended),
        len.get() as usize,
    ))
}

/// Returns the data length code that encodes a payload length.
#[napi]
pub fn can_len_to_dlc(len: checked::u32) -> u8 {
    len_to_dlc(len.get() as usize)
}

/// Returns the payload length a data length code encodes.
#[napi]
pub fn can_dlc_to_len(dlc: checked::u8) -> u32 {
    dlc_to_len(dlc.get()) as u32
}

/// Decodes the J1939 fields out of an extended CAN identifier.
///
/// Returns `null` for a standard 11-bit identifier, which J1939 does not use.
#[napi]
pub fn j1939_decode(id: checked::u32, extended: bool) -> Option<J1939Message> {
    J1939Id::from_id(identifier(id.get(), extended)).map(|message| J1939Message {
        pgn: message.pgn(),
        priority: message.priority(),
        source: message.source(),
        pdu_format: message.pdu_format(),
        destination: message.destination(),
        broadcast: message.is_broadcast(),
    })
}

/// Composes the extended CAN identifier a set of J1939 fields describes.
///
/// The destination is used only for an addressed (PDU1) parameter group and
/// ignored for a broadcast (PDU2) one.
#[napi]
pub fn j1939_compose(
    priority: checked::u8,
    pgn: checked::u32,
    source: checked::u8,
    destination: checked::u8,
) -> u32 {
    J1939Id::from_parts(priority.get(), pgn.get(), source.get(), destination.get())
        .to_id()
        .raw()
}

/// Composes the identifier of a J1939 broadcast, which every node on the bus reads.
///
/// This is the ordinary case: most parameter groups are broadcast, and a caller
/// should not have to know that a broadcast is addressed to `0xFF`.
#[napi]
pub fn j1939_broadcast(priority: checked::u8, pgn: checked::u32, source: checked::u8) -> u32 {
    J1939Id::broadcast(priority.get(), pgn.get(), source.get())
        .to_id()
        .raw()
}

/// The byte a J1939 sender writes for a signal it is not reporting.
#[napi]
pub const J1939_NOT_AVAILABLE: u8 = NOT_AVAILABLE;

/// The destination address every node on the bus reads.
#[napi]
pub const J1939_BROADCAST_ADDRESS: u8 = BROADCAST_ADDRESS;

/// The priority a control message takes, ahead of ordinary traffic.
#[napi]
pub const J1939_PRIORITY_CONTROL: u8 = priority::CONTROL;

/// The priority ordinary traffic takes.
#[napi]
pub const J1939_PRIORITY_DEFAULT: u8 = priority::DEFAULT;

/// The priority that yields to everything else on the bus.
#[napi]
pub const J1939_PRIORITY_LOWEST: u8 = priority::LOWEST;

/// The eight data bytes of a J1939 frame, addressed by the signals inside them.
///
/// A parameter group places each signal at a fixed byte offset, little-endian. A
/// payload starts with every signal marked not available, so a controller writes
/// only the signals it actually reports.
#[napi(js_name = "Signals")]
pub struct CanSignals {
    inner: Signals,
}

impl Default for CanSignals {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl CanSignals {
    /// Builds a payload with every signal marked not available.
    #[napi(constructor)]
    pub fn new() -> CanSignals {
        CanSignals {
            inner: Signals::new(),
        }
    }

    /// Reads the eight data bytes of a frame that arrived off the bus.
    #[napi(factory)]
    pub fn from_bytes(bytes: Buffer) -> napi::Result<CanSignals> {
        let bytes: [u8; 8] = bytes
            .as_ref()
            .try_into()
            .map_err(|_| napi::Error::from_reason("a J1939 payload is exactly eight bytes"))?;
        Ok(CanSignals {
            inner: Signals::from_bytes(bytes),
        })
    }

    /// Writes a one-byte signal at the offset its parameter group defines.
    #[napi]
    pub fn set_u8(&mut self, at: checked::u32, value: checked::u8) {
        self.inner.set_u8(at.get() as usize, value.get());
    }

    /// Writes a two-byte little-endian signal at the offset its group defines.
    #[napi]
    pub fn set_u16(&mut self, at: checked::u32, value: checked::u16) {
        self.inner.set_u16(at.get() as usize, value.get());
    }

    /// Reads a one-byte signal, or `null` if the offset is past the payload.
    #[napi]
    pub fn u8(&self, at: checked::u32) -> Option<u8> {
        self.inner.u8(at.get() as usize)
    }

    /// Reads a two-byte little-endian signal, or `null` if it would run past the
    /// payload.
    #[napi]
    pub fn u16(&self, at: checked::u32) -> Option<u16> {
        self.inner.u16(at.get() as usize)
    }

    /// The eight data bytes, ready to put in a frame.
    #[napi(getter)]
    pub fn bytes(&self) -> Buffer {
        self.inner.as_bytes().to_vec().into()
    }
}

/// Describes a built frame as the plain object JavaScript receives.
fn describe(frame: Frame) -> CanFrame {
    CanFrame {
        id: frame.id().raw().into(),
        extended: frame.id().is_extended(),
        fd: frame.is_fd(),
        remote: frame.is_remote(),
        len: (frame.len() as u8).into(),
        dlc: frame.dlc().into(),
        data: frame.data().into(),
    }
}

/// Builds an identifier of the requested width, masking the value to fit it.
fn identifier(id: u32, extended: bool) -> CanId {
    if extended {
        CanId::extended(id)
    } else {
        CanId::standard(id as u16)
    }
}

/// Maps a framing error onto a thrown exception.
fn to_napi(error: CanError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

/// What a node's bus is.
#[napi(string_enum, js_name = "CanBusKind")]
pub enum CanBusKindName {
    /// A kernel CAN interface reached through SocketCAN.
    Device,
    /// A bus inside the program.
    Simulated,
}

/// A frame a node keeps: one whose identifier, masked, equals `id`, masked, and whose format is
/// the filter's.
#[napi(object)]
pub struct CanFilter {
    /// The identifier to match.
    pub id: checked::u32,
    /// The identifier bits that have to match.
    pub mask: checked::u32,
    /// Whether the identifier is a 29-bit extended one.
    pub extended: bool,
}

impl From<Filter> for CanFilter {
    fn from(filter: Filter) -> Self {
        CanFilter {
            id: filter.id().raw().into(),
            mask: filter.mask().into(),
            extended: filter.id().is_extended(),
        }
    }
}

fn filter_of(filter: &CanFilter) -> Filter {
    Filter::new(
        identifier(filter.id.get(), filter.extended),
        filter.mask.get(),
    )
}

/// A filter that passes one identifier and nothing else.
#[napi]
pub fn can_filter_exact(id: checked::u32, extended: bool) -> CanFilter {
    Filter::exact(identifier(id.get(), extended)).into()
}

/// A filter that passes one J1939 parameter group at any priority, from any source, and for an
/// addressed group, to any destination.
#[napi]
pub fn can_filter_pgn(pgn: checked::u32) -> CanFilter {
    Filter::pgn(pgn.get()).into()
}

/// Whether a frame with an identifier passes a filter.
#[napi]
pub fn can_filter_matches(filter: CanFilter, id: checked::u32, extended: bool) -> bool {
    filter_of(&filter).matches(identifier(id.get(), extended))
}

/// Rebuilds a frame from the plain object JavaScript holds.
fn frame_of(frame: &CanFrame) -> napi::Result<Frame> {
    let id = identifier(frame.id.get(), frame.extended);
    let built = if frame.remote {
        Ok(Frame::remote(id, usize::from(frame.len.get())))
    } else if frame.fd {
        Frame::fd(id, frame.data.as_ref())
    } else {
        Frame::new(id, frame.data.as_ref())
    };
    built.map_err(to_napi)
}

/// One node's place on a CAN bus.
///
/// `CanBus.open(interface)` opens a kernel CAN interface, such as `can0`, through SocketCAN on a
/// Linux board and throws anywhere else. `CanBus.simulated()` makes a bus inside the program,
/// and `join()` puts another node on the same bus. A node hears every frame the others send and
/// none of its own, and keeps only the frames its filters pass. `send` and `receive` return
/// promises; a receive on a simulated bus with nothing waiting resolves at once with `null` and
/// counts its timeout in `waitedMicros`.
#[napi(js_name = "CanBus")]
pub struct CanBusNode {
    inner: CanBus,
}

fn open_error(error: OpenError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

fn bus_error(error: BusError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

fn finished(error: impl std::fmt::Display) -> napi::Error {
    napi::Error::from_reason(format!("the bus call did not finish: {error}"))
}

#[napi]
impl CanBusNode {
    /// Opens a kernel CAN interface through SocketCAN, as one node on its bus. Bring the
    /// interface up first, with `ip link set can0 up type can bitrate 250000`.
    #[napi(factory)]
    pub fn open(interface: String) -> napi::Result<Self> {
        CanBus::open(&interface)
            .map(|inner| CanBusNode { inner })
            .map_err(open_error)
    }

    /// A new bus inside the program, with this node the first on it.
    #[napi(factory)]
    pub fn simulated() -> Self {
        CanBusNode {
            inner: CanBus::simulated(),
        }
    }

    /// Puts another node on the same bus.
    #[napi]
    pub fn join(&self) -> napi::Result<CanBusNode> {
        self.inner
            .join()
            .map(|inner| CanBusNode { inner })
            .map_err(open_error)
    }

    /// What the bus is.
    #[napi(getter)]
    pub fn kind(&self) -> CanBusKindName {
        match self.inner.kind() {
            CanBusKind::Device => CanBusKindName::Device,
            CanBusKind::Simulated => CanBusKindName::Simulated,
        }
    }

    /// The kernel interface the node is on, or `null` on a simulated bus.
    #[napi(getter)]
    pub fn interface(&self) -> Option<String> {
        self.inner.interface()
    }

    /// Sends a frame to every other node on the bus.
    #[napi]
    pub async fn send(&self, frame: CanFrame) -> napi::Result<()> {
        let frame = frame_of(&frame)?;
        let bus = self.inner.clone();
        spawn_blocking(move || bus.send(&frame))
            .await
            .map_err(finished)?
            .map_err(bus_error)
    }

    /// Resolves with the next frame the node keeps, waiting up to `timeoutMs` for one, or with
    /// `null` when the timeout passed with nothing.
    #[napi]
    pub async fn receive(&self, timeout_ms: f64) -> napi::Result<Option<CanFrame>> {
        if !(timeout_ms.is_finite() && timeout_ms >= 0.0) {
            return Err(napi::Error::from_reason(
                "a time must be a finite number of milliseconds, zero or more",
            ));
        }
        let timeout = Duration::from_secs_f64(timeout_ms / 1_000.0);
        let bus = self.inner.clone();
        let frame = spawn_blocking(move || bus.receive(timeout))
            .await
            .map_err(finished)?
            .map_err(bus_error)?;
        Ok(frame.map(describe))
    }

    /// Keeps only the frames that pass at least one of the filters, from now on; an empty list
    /// keeps nothing.
    #[napi]
    pub fn set_filters(&self, filters: Vec<CanFilter>) -> napi::Result<()> {
        let filters: Vec<Filter> = filters.iter().map(filter_of).collect();
        self.inner.set_filters(&filters).map_err(bus_error)
    }

    /// Keeps every frame again, as a node does when it joins.
    #[napi]
    pub fn clear_filters(&self) -> napi::Result<()> {
        self.inner.clear_filters().map_err(bus_error)
    }

    /// How many frames the node has sent.
    #[napi(getter)]
    pub fn sent(&self) -> f64 {
        self.inner.sent() as f64
    }

    /// How many frames the node has received.
    #[napi(getter)]
    pub fn received(&self) -> f64 {
        self.inner.received() as f64
    }

    /// How long receives on the node have waited without a frame, in microseconds, whether or
    /// not the process slept through it.
    #[napi(getter, js_name = "waitedMicros")]
    pub fn waited_micros(&self) -> f64 {
        self.inner.waited_micros() as f64
    }
}
