//! One CAN bus, joined by each node on it.
//!
//! CAN has no master: every node sees every frame on the wire and keeps the ones it wants.
//! [`CanBus`] is one node's place on a bus. On a Linux board it is a SocketCAN socket on an
//! interface such as `can0`, opened with [`CanBus::open`]; anywhere, [`CanBus::simulated`]
//! makes a bus inside the program. [`CanBus::join`] puts another node on the same bus. A node
//! hears every frame the others send and none of its own, as a SocketCAN socket does, and
//! keeps only the frames its [`Filter`]s pass.
//!
//! A receive on a simulated bus never waits: when nothing is waiting it returns at once and
//! adds its timeout to [`CanBus::waited_micros`], so a test of a controller that has gone
//! quiet runs at once and still says how long a real one would have taken.
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//!
//! use pamoja_can::bus::CanBus;
//! use pamoja_can::{CanId, Frame};
//!
//! // Two nodes on one bus with nothing plugged in.
//! let controller = CanBus::simulated();
//! let gateway = controller.join()?;
//!
//! // The controller reports a motor speed of 500 rpm, big-endian.
//! let speed = 500u16.to_be_bytes();
//! controller.send(&Frame::new(CanId::standard(0x20A), &speed)?)?;
//! let heard = gateway.receive(Duration::from_millis(10))?.expect("a frame is waiting");
//! assert_eq!(heard.data(), speed);
//!
//! // A node does not hear its own frames, and a receive with nothing waiting is counted.
//! assert_eq!(controller.receive(Duration::from_millis(250))?, None);
//! assert_eq!(controller.waited_micros(), 250_000);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use crate::frame::Frame;
use crate::id::CanId;
use crate::j1939::J1939Id;

#[cfg(all(feature = "linux", target_os = "linux"))]
mod socketcan;

/// What a node's bus is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanBusKind {
    /// A kernel CAN interface reached through SocketCAN: `can0` for a controller, `vcan0` for a
    /// virtual one.
    Device,
    /// A bus inside the program, with every node on it simulated.
    Simulated,
}

/// A frame a node keeps: one whose identifier, masked, equals the filter's, and whose width,
/// standard or extended, is the filter's.
///
/// This is the SocketCAN filter, `received_id & mask == id & mask`, with the frame format
/// always part of the match, so a filter for an extended identifier never passes a standard
/// frame that happens to share its low bits.
///
/// # Examples
///
/// ```
/// use pamoja_can::bus::Filter;
/// use pamoja_can::{CanId, J1939Id};
///
/// // Engine speed, parameter group 61444, from any engine controller at any priority.
/// let engine_speed = Filter::pgn(61_444);
/// assert!(engine_speed.matches(J1939Id::broadcast(3, 61_444, 0x00).to_id()));
/// assert!(engine_speed.matches(J1939Id::broadcast(6, 61_444, 0x01).to_id()));
/// assert!(!engine_speed.matches(J1939Id::broadcast(3, 61_443, 0x00).to_id()));
///
/// // One standard identifier and nothing else.
/// let motor = Filter::exact(CanId::standard(0x20A));
/// assert!(motor.matches(CanId::standard(0x20A)));
/// assert!(!motor.matches(CanId::extended(0x20A)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Filter {
    id: CanId,
    mask: u32,
}

fn width(id: CanId) -> u32 {
    if id.is_extended() {
        CanId::EXTENDED_MASK
    } else {
        CanId::STANDARD_MASK
    }
}

impl Filter {
    /// A filter on the identifier bits a mask selects.
    ///
    /// # Arguments
    ///
    /// * `id` - the identifier to match, which also fixes the frame format.
    /// * `mask` - the identifier bits that have to match; bits outside the identifier's width
    ///   are dropped.
    ///
    /// # Returns
    ///
    /// The filter.
    pub fn new(id: CanId, mask: u32) -> Filter {
        Filter {
            id,
            mask: mask & width(id),
        }
    }

    /// A filter that passes one identifier and nothing else.
    ///
    /// # Arguments
    ///
    /// * `id` - the identifier.
    ///
    /// # Returns
    ///
    /// The filter.
    pub fn exact(id: CanId) -> Filter {
        Filter::new(id, width(id))
    }

    /// A filter that passes one J1939 parameter group at any priority, from any source, and
    /// for an addressed group, to any destination.
    ///
    /// # Arguments
    ///
    /// * `pgn` - the parameter group number.
    ///
    /// # Returns
    ///
    /// The filter.
    pub fn pgn(pgn: u32) -> Filter {
        let id = J1939Id::from_parts(0, pgn, 0, 0);
        let mask = if id.is_broadcast() {
            0x03FF_FF00
        } else {
            0x03FF_0000
        };
        Filter::new(id.to_id(), mask)
    }

    /// Returns the identifier the filter matches.
    pub fn id(&self) -> CanId {
        self.id
    }

    /// Returns the identifier bits that have to match.
    pub fn mask(&self) -> u32 {
        self.mask
    }

    /// Reports whether a frame with an identifier passes the filter.
    ///
    /// # Arguments
    ///
    /// * `id` - the frame's identifier.
    ///
    /// # Returns
    ///
    /// `true` when the frame formats agree and the masked bits are equal.
    pub fn matches(&self, id: CanId) -> bool {
        id.is_extended() == self.id.is_extended()
            && id.raw() & self.mask == self.id.raw() & self.mask
    }
}

/// Why a CAN interface could not be opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError {
    /// This platform has no SocketCAN, or this build left out the `linux` feature.
    Unsupported,
    /// The interface could not be found or bound.
    Device {
        /// The interface, such as `can0`.
        interface: String,
        /// Why, in the kernel's words.
        reason: String,
    },
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Unsupported => f.write_str(
                "a CAN interface is opened through SocketCAN, which only Linux has here",
            ),
            OpenError::Device { interface, reason } => write!(f, "{interface}: {reason}"),
        }
    }
}

impl std::error::Error for OpenError {}

/// Why a send or a receive on a [`CanBus`] failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BusError {
    /// The kernel's interface failed, or refused the frame: a CAN FD frame on an interface
    /// that runs classic CAN, or a bus that has gone bus-off.
    Device {
        /// The interface and the kernel's reason.
        reason: String,
    },
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BusError::Device { reason } => f.write_str(reason),
        }
    }
}

impl std::error::Error for BusError {}

/// One node's place on a CAN bus, shared by every clone of this handle.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use pamoja_can::bus::{CanBus, Filter};
/// use pamoja_can::{Frame, J1939Id, Signals};
///
/// // EEC1 carries engine speed, and ET1 the engine's temperatures.
/// const EEC1: u32 = 61_444;
/// const ET1: u32 = 65_262;
///
/// let engine = CanBus::simulated();
/// let gateway = engine.join()?;
/// gateway.set_filters(&[Filter::pgn(EEC1)])?;
///
/// // The gateway keeps engine speed and lets the engine's other groups go by. Each frame
/// // here reports nothing yet, every signal marked not available.
/// let nothing = Signals::new();
/// let speed = J1939Id::broadcast(3, EEC1, 0).to_id();
/// let temperatures = J1939Id::broadcast(6, ET1, 0).to_id();
/// engine.send(&Frame::new(temperatures, nothing.as_bytes())?)?;
/// engine.send(&Frame::new(speed, nothing.as_bytes())?)?;
///
/// let heard = gateway.receive(Duration::from_millis(10))?.expect("engine speed");
/// assert_eq!(heard.id(), speed);
/// assert_eq!(gateway.receive(Duration::from_millis(10))?, None);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct CanBus {
    node: Arc<Mutex<Node>>,
}

struct Node {
    backend: Backend,
    sent: usize,
    received: usize,
    waited_ns: u64,
}

enum Backend {
    #[cfg(all(feature = "linux", target_os = "linux"))]
    Device {
        interface: String,
        socket: Arc<socketcan::Socket>,
    },
    Simulated {
        network: Arc<Mutex<Network>>,
        seat: usize,
    },
}

#[derive(Default)]
struct Network {
    seats: Vec<Seat>,
}

struct Seat {
    inbox: VecDeque<Frame>,
    filters: Option<Vec<Filter>>,
    joined: bool,
}

impl Seat {
    fn new() -> Seat {
        Seat {
            inbox: VecDeque::new(),
            filters: None,
            joined: true,
        }
    }

    fn keeps(&self, frame: &Frame) -> bool {
        self.joined
            && self
                .filters
                .as_ref()
                .is_none_or(|filters| filters.iter().any(|filter| filter.matches(frame.id())))
    }
}

impl Drop for Node {
    // Without the `linux` feature a node is always simulated, and the pattern cannot fail.
    #[allow(irrefutable_let_patterns)]
    fn drop(&mut self) {
        if let Backend::Simulated { network, seat } = &self.backend {
            let mut network = lock(network);
            let seat = &mut network.seats[*seat];
            seat.joined = false;
            seat.inbox.clear();
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

impl CanBus {
    fn with(backend: Backend) -> CanBus {
        CanBus {
            node: Arc::new(Mutex::new(Node {
                backend,
                sent: 0,
                received: 0,
                waited_ns: 0,
            })),
        }
    }

    /// Opens a kernel CAN interface through SocketCAN, as one node on its bus.
    ///
    /// The interface is brought up and given its bit rate outside the program, with
    /// `ip link set can0 up type can bitrate 250000`. The node takes CAN FD frames too where the
    /// interface runs CAN FD.
    ///
    /// # Arguments
    ///
    /// * `interface` - the interface: `can0` for the first controller, `vcan0` for a virtual
    ///   one.
    ///
    /// # Returns
    ///
    /// The node.
    ///
    /// # Errors
    ///
    /// [`OpenError::Unsupported`] anywhere but Linux, or in a build without the `linux`
    /// feature, and [`OpenError::Device`] when the interface does not exist or cannot be bound.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pamoja_can::bus::CanBus;
    ///
    /// let bus = CanBus::open("can0")?;
    /// # Ok::<(), pamoja_can::bus::OpenError>(())
    /// ```
    pub fn open(interface: &str) -> Result<CanBus, OpenError> {
        #[cfg(all(feature = "linux", target_os = "linux"))]
        {
            let socket = socketcan::Socket::open(interface).map_err(|error| OpenError::Device {
                interface: interface.to_string(),
                reason: error.to_string(),
            })?;
            Ok(CanBus::with(Backend::Device {
                interface: interface.to_string(),
                socket: Arc::new(socket),
            }))
        }
        #[cfg(not(all(feature = "linux", target_os = "linux")))]
        {
            let _ = interface;
            Err(OpenError::Unsupported)
        }
    }

    /// A new bus inside the program, with this node the first on it.
    ///
    /// # Returns
    ///
    /// The node.
    pub fn simulated() -> CanBus {
        let mut network = Network::default();
        network.seats.push(Seat::new());
        CanBus::with(Backend::Simulated {
            network: Arc::new(Mutex::new(network)),
            seat: 0,
        })
    }

    /// Puts another node on the same bus: another socket on the same interface, or another
    /// node on the same simulated bus. It hears what this node sends, and this node hears it.
    ///
    /// # Returns
    ///
    /// The new node.
    ///
    /// # Errors
    ///
    /// [`OpenError::Device`] when the kernel refuses another socket on the interface.
    pub fn join(&self) -> Result<CanBus, OpenError> {
        let node = lock(&self.node);
        match &node.backend {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Device { interface, .. } => {
                let interface = interface.clone();
                drop(node);
                CanBus::open(&interface)
            }
            Backend::Simulated { network, .. } => {
                let mut shared = lock(network);
                shared.seats.push(Seat::new());
                let seat = shared.seats.len() - 1;
                Ok(CanBus::with(Backend::Simulated {
                    network: Arc::clone(network),
                    seat,
                }))
            }
        }
    }

    /// Returns what the bus is.
    pub fn kind(&self) -> CanBusKind {
        match lock(&self.node).backend {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Device { .. } => CanBusKind::Device,
            Backend::Simulated { .. } => CanBusKind::Simulated,
        }
    }

    /// Returns the kernel interface the node is on, or `None` on a simulated bus.
    pub fn interface(&self) -> Option<String> {
        match &lock(&self.node).backend {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Device { interface, .. } => Some(interface.clone()),
            Backend::Simulated { .. } => None,
        }
    }

    /// Sends a frame to every other node on the bus.
    ///
    /// # Arguments
    ///
    /// * `frame` - the frame.
    ///
    /// # Errors
    ///
    /// [`BusError::Device`] when the kernel refuses the frame or the interface fails.
    pub fn send(&self, frame: &Frame) -> Result<(), BusError> {
        let mut node = lock(&self.node);
        match &node.backend {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Device { interface, socket } => {
                let (interface, socket) = (interface.clone(), Arc::clone(socket));
                drop(node);
                socket.send(frame).map_err(|error| BusError::Device {
                    reason: format!("{interface}: {error}"),
                })?;
                node = lock(&self.node);
            }
            Backend::Simulated { network, seat } => {
                let mut network = lock(network);
                for (index, other) in network.seats.iter_mut().enumerate() {
                    if index != *seat && other.keeps(frame) {
                        other.inbox.push_back(*frame);
                    }
                }
            }
        }
        node.sent += 1;
        Ok(())
    }

    /// Takes the next frame this node keeps, waiting up to `timeout` for one when none has
    /// arrived.
    ///
    /// On a simulated bus nothing waits: with no frame there it returns at once, and the
    /// timeout is added to [`CanBus::waited_micros`].
    ///
    /// # Arguments
    ///
    /// * `timeout` - how long to wait.
    ///
    /// # Returns
    ///
    /// The frame, or `None` when the timeout passed with nothing.
    ///
    /// # Errors
    ///
    /// [`BusError::Device`] when the interface fails.
    pub fn receive(&self, timeout: Duration) -> Result<Option<Frame>, BusError> {
        let mut node = lock(&self.node);
        let frame = match &node.backend {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Device { interface, socket } => {
                let (interface, socket) = (interface.clone(), Arc::clone(socket));
                drop(node);
                let frame = socket.receive(timeout).map_err(|error| BusError::Device {
                    reason: format!("{interface}: {error}"),
                })?;
                node = lock(&self.node);
                frame
            }
            Backend::Simulated { network, seat } => lock(network).seats[*seat].inbox.pop_front(),
        };
        match frame {
            Some(_) => node.received += 1,
            None => node.waited_ns = node.waited_ns.saturating_add(nanos(timeout)),
        }
        Ok(frame)
    }

    /// Keeps only the frames that pass at least one of the filters, from now on.
    ///
    /// An empty list keeps nothing, as SocketCAN's does; [`clear_filters`](CanBus::clear_filters)
    /// keeps everything again.
    ///
    /// # Arguments
    ///
    /// * `filters` - the filters, at most 512, the most SocketCAN takes.
    ///
    /// # Errors
    ///
    /// [`BusError::Device`] when the kernel refuses the filters.
    pub fn set_filters(&self, filters: &[Filter]) -> Result<(), BusError> {
        self.filter(Some(filters.to_vec()))
    }

    /// Keeps every frame again, as a node does when it joins.
    ///
    /// # Errors
    ///
    /// [`BusError::Device`] when the kernel refuses the change.
    pub fn clear_filters(&self) -> Result<(), BusError> {
        self.filter(None)
    }

    fn filter(&self, filters: Option<Vec<Filter>>) -> Result<(), BusError> {
        let node = lock(&self.node);
        match &node.backend {
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Device { interface, socket } => socket
                .set_filters(filters.as_deref())
                .map_err(|error| BusError::Device {
                    reason: format!("{interface}: {error}"),
                }),
            Backend::Simulated { network, seat } => {
                lock(network).seats[*seat].filters = filters;
                Ok(())
            }
        }
    }

    /// Returns how many frames this node and its clones have sent.
    pub fn sent(&self) -> usize {
        lock(&self.node).sent
    }

    /// Returns how many frames this node and its clones have received.
    pub fn received(&self) -> usize {
        lock(&self.node).received
    }

    /// Returns how long receives on this node have waited without a frame, in microseconds,
    /// whether or not the process slept through it.
    pub fn waited_micros(&self) -> u64 {
        lock(&self.node).waited_ns / 1_000
    }
}

impl fmt::Debug for CanBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node = lock(&self.node);
        f.debug_struct("CanBus")
            .field(
                "kind",
                &match node.backend {
                    #[cfg(all(feature = "linux", target_os = "linux"))]
                    Backend::Device { .. } => CanBusKind::Device,
                    Backend::Simulated { .. } => CanBusKind::Simulated,
                },
            )
            .field("sent", &node.sent)
            .field("received", &node.received)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speed() -> CanId {
        J1939Id::broadcast(3, 61_444, 0x00).to_id()
    }

    #[test]
    fn every_other_node_hears_a_frame_and_the_sender_does_not() {
        let engine = CanBus::simulated();
        let gateway = engine.join().unwrap();
        let logger = gateway.join().unwrap();
        let frame = Frame::new(speed(), &[0xFF; 8]).unwrap();
        engine.send(&frame).unwrap();

        assert_eq!(gateway.receive(Duration::ZERO).unwrap(), Some(frame));
        assert_eq!(logger.receive(Duration::ZERO).unwrap(), Some(frame));
        assert_eq!(engine.receive(Duration::from_millis(5)).unwrap(), None);
        assert_eq!(engine.waited_micros(), 5_000);
        assert_eq!((engine.sent(), gateway.received()), (1, 1));
        assert_eq!(engine.kind(), CanBusKind::Simulated);
        assert_eq!(engine.interface(), None);
    }

    #[test]
    fn frames_arrive_in_the_order_they_were_sent() {
        let one = CanBus::simulated();
        let other = one.join().unwrap();
        for value in 0u8..5 {
            one.send(&Frame::new(CanId::standard(0x100), &[value]).unwrap())
                .unwrap();
        }
        let heard: Vec<u8> = (0..5)
            .map(|_| other.receive(Duration::ZERO).unwrap().unwrap().data()[0])
            .collect();
        assert_eq!(heard, [0, 1, 2, 3, 4]);
    }

    #[test]
    fn a_filter_keeps_what_it_passes_and_an_empty_list_keeps_nothing() {
        let engine = CanBus::simulated();
        let gateway = engine.join().unwrap();
        gateway.set_filters(&[Filter::pgn(61_444)]).unwrap();
        let other = J1939Id::broadcast(6, 65_262, 0x00).to_id();
        engine.send(&Frame::new(other, &[0; 8]).unwrap()).unwrap();
        engine.send(&Frame::new(speed(), &[0; 8]).unwrap()).unwrap();
        assert_eq!(
            gateway.receive(Duration::ZERO).unwrap().unwrap().id(),
            speed()
        );
        assert_eq!(gateway.receive(Duration::ZERO).unwrap(), None);

        gateway.set_filters(&[]).unwrap();
        engine.send(&Frame::new(speed(), &[0; 8]).unwrap()).unwrap();
        assert_eq!(gateway.receive(Duration::ZERO).unwrap(), None);

        gateway.clear_filters().unwrap();
        engine.send(&Frame::new(other, &[0; 8]).unwrap()).unwrap();
        assert!(gateway.receive(Duration::ZERO).unwrap().is_some());
    }

    #[test]
    fn a_pgn_filter_ignores_priority_source_and_an_addressed_groups_destination() {
        // Engine speed is a broadcast group, whose PS byte is part of the group.
        let broadcast = Filter::pgn(61_444);
        assert!(broadcast.matches(J1939Id::broadcast(7, 61_444, 0x21).to_id()));
        assert!(!broadcast.matches(J1939Id::broadcast(3, 61_445, 0x00).to_id()));
        // The request group, 59904, is addressed: its PS byte is the destination.
        let request = Filter::pgn(59_904);
        assert!(request.matches(J1939Id::from_parts(6, 59_904, 0x01, 0x00).to_id()));
        assert!(request.matches(J1939Id::from_parts(6, 59_904, 0xF9, 0x21).to_id()));
        assert!(!request.matches(J1939Id::from_parts(6, 60_160, 0x01, 0x00).to_id()));
        assert!(!request.matches(CanId::standard(0x000)));
    }

    #[test]
    fn a_node_that_leaves_stops_collecting_frames() {
        let engine = CanBus::simulated();
        let gateway = engine.join().unwrap();
        drop(gateway);
        engine.send(&Frame::new(speed(), &[0; 8]).unwrap()).unwrap();
        let network = match &lock(&engine.node).backend {
            Backend::Simulated { network, .. } => Arc::clone(network),
            #[cfg(all(feature = "linux", target_os = "linux"))]
            Backend::Device { .. } => unreachable!(),
        };
        assert!(lock(&network).seats[1].inbox.is_empty());
    }

    #[test]
    fn opening_an_interface_off_linux_is_unsupported() {
        if !cfg!(all(feature = "linux", target_os = "linux")) {
            assert_eq!(CanBus::open("can0").err(), Some(OpenError::Unsupported));
        }
    }

    // Runs against a virtual interface named by PAMOJA_VCAN, which CI creates with
    // `ip link add dev vcan0 type vcan`, and passes quietly where there is none.
    #[cfg(all(feature = "linux", target_os = "linux"))]
    #[test]
    fn a_virtual_interface_carries_every_kind_of_frame_between_two_sockets() {
        let Ok(interface) = std::env::var("PAMOJA_VCAN") else {
            return;
        };
        let engine = CanBus::open(&interface).unwrap();
        let gateway = engine.join().unwrap();
        assert_eq!(engine.kind(), CanBusKind::Device);
        assert_eq!(gateway.interface().as_deref(), Some(interface.as_str()));

        let frames = [
            Frame::new(CanId::standard(0x20A), &[0x01, 0xF4]).unwrap(),
            Frame::new(speed(), &[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]).unwrap(),
            Frame::remote(CanId::standard(0x301), 4),
            Frame::fd(CanId::extended(0x1ABC_DEF0), &[0xA5; 32]).unwrap(),
        ];
        for frame in &frames {
            engine.send(frame).unwrap();
        }
        for frame in &frames {
            let heard = gateway.receive(Duration::from_millis(500)).unwrap();
            assert_eq!(heard, Some(*frame));
        }
        assert_eq!(engine.receive(Duration::from_millis(20)).unwrap(), None);

        gateway.set_filters(&[Filter::pgn(61_444)]).unwrap();
        engine
            .send(&Frame::new(CanId::standard(0x20A), &[0]).unwrap())
            .unwrap();
        engine.send(&frames[1]).unwrap();
        let kept = gateway.receive(Duration::from_millis(500)).unwrap();
        assert_eq!(kept.map(|frame| frame.id()), Some(speed()));
        assert_eq!(gateway.receive(Duration::from_millis(20)).unwrap(), None);
        gateway.clear_filters().unwrap();
        assert_eq!((engine.sent(), gateway.received()), (6, 5));
    }
}
