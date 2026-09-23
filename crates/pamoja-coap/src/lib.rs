//! CoAP transport for the pamoja SDK.
//!
//! [`CoapTransport`] implements the core [`Transport`] and [`Receive`] traits on
//! top of the pure-Rust [`coap_lite`] message codec and a UDP socket, so an
//! application can talk to constrained RESTful devices through the same
//! protocol-agnostic surface it uses for every other transport.
//!
//! CoAP is connectionless: [`connect`](Transport::connect) binds a local UDP
//! socket and points it at the server, then spawns a background task that decodes
//! inbound datagrams. A [`send`](Transport::send) is a CoAP `PUT` to a resource
//! path, and a [`subscribe`](Transport::subscribe) registers an RFC 7641 observe on
//! a resource so the server's notifications are forwarded to an internal queue that
//! [`recv`](Receive::recv) drains.
//!
//! Delivery follows the configured [`Reliability`]: [`Reliability::Confirmable`]
//! messages are acknowledged with retransmission (at-least-once), while
//! [`Reliability::NonConfirmable`] messages are fire-and-forget (at-most-once),
//! which suits the cheapest, most power-constrained devices. A confirmable request
//! the server resets, or answers with a 4.xx or 5.xx code, fails rather than
//! counting as delivered, and a notification is delivered under the path its
//! observation named, since a notification carries only the registration's token.
//!
//! [`CoapServer`] is the other end: a gateway that takes the readings nodes send it
//! and holds the resources they observe, through the same traits.
//!
//! # Examples
//!
//! ```no_run
//! use pamoja_core::{Receive, Transport};
//! use pamoja_coap::{CoapConfig, CoapTransport};
//!
//! # async fn run() -> pamoja_core::Result<()> {
//! let mut transport = CoapTransport::new(CoapConfig::new("localhost", 5683));
//! transport.connect().await?;
//! transport.subscribe("sensors/temperature").await?;
//! transport.send("actuators/valve", b"open").await?;
//!
//! if let Some(message) = transport.recv().await? {
//!     println!("{}: {} bytes", message.topic, message.payload.len());
//! }
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use coap_lite::{CoapOption, MessageClass, MessageType, Packet, RequestType};
pub use pamoja_core::Message;
use pamoja_core::{Error, Receive, Result, Transport};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

mod server;

pub use server::{CoapPublisher, CoapServer};

/// Outstanding confirmable requests keyed by message id, each awaiting its answer.
type PendingAcks = Arc<Mutex<HashMap<u16, oneshot::Sender<Answer>>>>;

/// The resources this client observes, keyed by the token each registration used.
type Observations = Arc<Mutex<HashMap<Vec<u8>, Observation>>>;

/// What came back for a confirmable message.
enum Answer {
    /// An acknowledgment and the response it may have carried.
    Acknowledged(Reply),
    /// A Reset: the server received the message but could not process it.
    Reset,
}

/// A response piggybacked on an acknowledgment.
struct Reply {
    /// The response code, `Empty` when the response follows separately.
    code: MessageClass,
    /// The response payload, which for an error is the server's diagnostic.
    payload: Vec<u8>,
    /// The Observe option's value, present when the server registered an observation.
    observe: Option<u32>,
}

/// One resource this client observes.
struct Observation {
    /// The path the registration named, which each notification is delivered under.
    path: String,
    /// The sequence number and arrival of the freshest notification so far.
    freshest: Option<(u32, Instant)>,
}

/// The delivery guarantee applied to published and subscribed messages.
///
/// These map onto the CoAP message types defined in RFC 7252.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reliability {
    /// Fire and forget: the request is sent once and not acknowledged.
    NonConfirmable,
    /// The request is acknowledged, and retransmitted until an ACK arrives.
    Confirmable,
}

/// Connection settings for a [`CoapTransport`].
///
/// Construct with [`CoapConfig::new`] and refine with the chained setters; every
/// field has a sensible default so only the server address is required.
#[derive(Clone, Debug)]
pub struct CoapConfig {
    host: String,
    port: u16,
    bind: String,
    reliability: Reliability,
    ack_timeout: Duration,
    max_retransmits: u32,
}

impl CoapConfig {
    /// Creates a configuration pointing at the given CoAP server.
    ///
    /// # Arguments
    ///
    /// * `host` - the server hostname or IP address.
    /// * `port` - the server UDP port, conventionally `5683` for plaintext CoAP.
    ///
    /// # Returns
    ///
    /// A configuration that binds an ephemeral local port, uses confirmable
    /// delivery, waits two to three seconds for the first acknowledgment, and
    /// retransmits up to four times: RFC 7252's defaults.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            bind: "0.0.0.0:0".to_owned(),
            reliability: Reliability::Confirmable,
            ack_timeout: Duration::from_secs(2),
            max_retransmits: 4,
        }
    }

    /// Sets the local socket address the transport binds to.
    ///
    /// # Arguments
    ///
    /// * `addr` - the local `host:port` to bind, for example `"0.0.0.0:0"` to let
    ///   the operating system choose a free port.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn bind(mut self, addr: impl Into<String>) -> Self {
        self.bind = addr.into();
        self
    }

    /// Sets the delivery guarantee applied to sends and subscriptions.
    ///
    /// # Arguments
    ///
    /// * `reliability` - confirmable (acknowledged) or non-confirmable delivery.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn reliability(mut self, reliability: Reliability) -> Self {
        self.reliability = reliability;
        self
    }

    /// Sets how long to wait for the first acknowledgment of a confirmable request.
    ///
    /// The first wait is drawn between this and one and a half times it, and each
    /// wait after it doubles, as RFC 7252 section 4.2 describes. Section 4.8.1
    /// forbids setting it below the default two seconds on a network without
    /// congestion control; lower the retransmissions instead to give up sooner.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the initial acknowledgment timeout.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn ack_timeout(mut self, timeout: Duration) -> Self {
        self.ack_timeout = timeout;
        self
    }

    /// Sets how many times a confirmable request is retransmitted before failing.
    ///
    /// # Arguments
    ///
    /// * `count` - the maximum number of retransmissions after the first send.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn max_retransmits(mut self, count: u32) -> Self {
        self.max_retransmits = count;
        self
    }
}

/// A CoAP client that implements the core [`Transport`] trait.
///
/// A transport is created disconnected; [`connect`](Transport::connect) binds the
/// socket and spawns the background task that decodes inbound datagrams for the
/// life of the connection. Observe notifications are queued and read with
/// [`recv`](CoapTransport::recv).
pub struct CoapTransport {
    config: CoapConfig,
    socket: Option<Arc<UdpSocket>>,
    incoming: Option<mpsc::UnboundedReceiver<Message>>,
    pending: PendingAcks,
    observations: Observations,
    pump: Option<JoinHandle<()>>,
    next_id: u16,
}

impl CoapTransport {
    /// Creates a transport from the given configuration without connecting.
    ///
    /// # Arguments
    ///
    /// * `config` - the server connection settings.
    ///
    /// # Returns
    ///
    /// A disconnected transport ready for [`connect`](Transport::connect).
    pub fn new(config: CoapConfig) -> Self {
        Self {
            config,
            socket: None,
            incoming: None,
            pending: Arc::new(Mutex::new(HashMap::new())),
            observations: Arc::new(Mutex::new(HashMap::new())),
            pump: None,
            next_id: random() as u16,
        }
    }

    /// Reports whether the transport currently holds a bound socket.
    ///
    /// # Returns
    ///
    /// `true` once [`connect`](Transport::connect) has succeeded and before
    /// [`disconnect`](CoapTransport::disconnect) is called.
    pub fn is_connected(&self) -> bool {
        self.socket.is_some()
    }

    /// Closes the socket and stops the background task.
    ///
    /// Calling this on a transport that is not connected is a no-op.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the background task has been stopped and the socket released.
    ///
    /// # Errors
    ///
    /// This call is best-effort and currently always returns `Ok(())`.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(pump) = self.pump.take() {
            pump.abort();
        }
        self.socket = None;
        self.incoming = None;
        self.observations.lock().expect("observations lock").clear();
        Ok(())
    }

    /// Returns the next message id and advances the counter.
    ///
    /// The counter starts at a random value, as RFC 7252 section 4.4 strongly
    /// recommends, so a node that restarts does not reuse the ids a server still
    /// holds for spotting duplicates.
    fn next_message_id(&mut self) -> u16 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    /// Returns a fresh, random request token.
    ///
    /// RFC 7252 section 5.3.1 asks a client without transport security for a
    /// nontrivial, randomized token, so that a response cannot be spoofed by
    /// guessing it.
    fn next_request_token(&mut self) -> Vec<u8> {
        random().to_be_bytes()[..TOKEN_LENGTH].to_vec()
    }

    /// Transmits a confirmable datagram and waits for its acknowledgment,
    /// retransmitting with a doubling timeout up to the configured limit.
    ///
    /// The first wait is drawn between the configured timeout and one and a half
    /// times it, and a Reset ends the attempt as a failure, both as RFC 7252
    /// section 4.2 describes.
    async fn exchange(&mut self, id: u16, bytes: &[u8], socket: &UdpSocket) -> Result<Reply> {
        let spread = (random() >> 11) as f64 / (1u64 << 53) as f64;
        let mut timeout = self
            .config
            .ack_timeout
            .mul_f64(1.0 + spread * (ACK_RANDOM_FACTOR - 1.0));
        for _ in 0..=self.config.max_retransmits {
            let (tx, rx) = oneshot::channel();
            self.pending.lock().expect("pending lock").insert(id, tx);
            if let Err(error) = socket.send(bytes).await {
                if !refused(&error) {
                    return Err(Error::Transport(error.to_string()));
                }
            }
            match tokio::time::timeout(timeout, rx).await {
                Ok(Ok(Answer::Acknowledged(reply))) => return Ok(reply),
                Ok(Ok(Answer::Reset)) => {
                    return Err(Error::Transport(
                        "the server reset the request: it arrived, but the server could not \
                         process it"
                            .into(),
                    ))
                }
                Ok(Err(_)) => return Err(Error::Closed),
                Err(_) => {
                    self.pending.lock().expect("pending lock").remove(&id);
                    timeout = timeout.saturating_mul(2);
                }
            }
        }
        let sent = self.config.max_retransmits + 1;
        let times = if sent == 1 {
            "transmission"
        } else {
            "transmissions"
        };
        Err(Error::Transport(format!(
            "no acknowledgment after {sent} {times}"
        )))
    }
}

/// How long a random request token is, in bytes.
const TOKEN_LENGTH: usize = 4;

/// The factor RFC 7252 section 4.8 spreads the first acknowledgment wait over.
const ACK_RANDOM_FACTOR: f64 = 1.5;

/// Whether a receive failed only because an earlier datagram found no one
/// listening.
///
/// The operating system reports the ICMP error for a datagram sent to a closed
/// port on the socket's next receive: Linux as a refused connection on a
/// connected socket, and Windows as a reset on any socket. Neither says anything
/// about the datagram being waited for, so the socket keeps listening.
fn refused(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::ConnectionReset
    )
}

/// A random value for message ids, tokens, and the spread of the first wait.
///
/// Each `RandomState` is keyed from the operating system's randomness, which is
/// what these values need: to be unpredictable to someone off the path.
fn random() -> u64 {
    use std::hash::{BuildHasher, RandomState};
    RandomState::new().hash_one(std::time::SystemTime::now())
}

/// Reads the code an acknowledgment carried, failing on an error response.
///
/// An empty acknowledgment, or one carrying a 2.xx success, means the request
/// took. A 4.xx or 5.xx means it did not, and its payload is the server's
/// diagnostic, which RFC 7252 section 5.5.2 makes human-readable text.
fn answered(code: MessageClass, diagnostic: &[u8]) -> Result<()> {
    let raw = u8::from(code);
    let class = raw >> 5;
    if class != 4 && class != 5 {
        return Ok(());
    }
    let detail = raw & 0x1F;
    let name = response_name(class, detail)
        .map(|name| format!(" {name}"))
        .unwrap_or_default();
    let reason = match core::str::from_utf8(diagnostic) {
        Ok(text) if !text.is_empty() => format!(": {text}"),
        _ => String::new(),
    };
    Err(Error::Transport(format!(
        "the server answered {class}.{detail:02}{name}{reason}"
    )))
}

/// The name RFC 7252 table 6 gives an error response code.
fn response_name(class: u8, detail: u8) -> Option<&'static str> {
    Some(match (class, detail) {
        (4, 0) => "Bad Request",
        (4, 1) => "Unauthorized",
        (4, 2) => "Bad Option",
        (4, 3) => "Forbidden",
        (4, 4) => "Not Found",
        (4, 5) => "Method Not Allowed",
        (4, 6) => "Not Acceptable",
        (4, 12) => "Precondition Failed",
        (4, 13) => "Request Entity Too Large",
        (4, 15) => "Unsupported Content-Format",
        (5, 0) => "Internal Server Error",
        (5, 1) => "Not Implemented",
        (5, 2) => "Bad Gateway",
        (5, 3) => "Service Unavailable",
        (5, 4) => "Gateway Timeout",
        (5, 5) => "Proxying Not Supported",
        _ => return None,
    })
}

impl Transport for CoapTransport {
    async fn connect(&mut self) -> Result<()> {
        let server = tokio::net::lookup_host((self.config.host.as_str(), self.config.port))
            .await
            .map_err(|err| Error::Transport(err.to_string()))?
            .next()
            .ok_or_else(|| Error::Transport(format!("could not resolve {}", self.config.host)))?;

        let socket = UdpSocket::bind(&self.config.bind)
            .await
            .map_err(|err| Error::Transport(err.to_string()))?;
        socket
            .connect(server)
            .await
            .map_err(|err| Error::Transport(err.to_string()))?;
        let socket = Arc::new(socket);

        let (tx, rx) = mpsc::unbounded_channel();
        let pending = Arc::clone(&self.pending);
        let observations = Arc::clone(&self.observations);
        let pump_socket = Arc::clone(&socket);
        let pump = tokio::spawn(async move {
            let mut buf = vec![0u8; 1500];
            loop {
                let len = match pump_socket.recv(&mut buf).await {
                    Ok(len) => len,
                    Err(error) if refused(&error) => continue,
                    Err(_) => break,
                };
                let Ok(packet) = Packet::from_bytes(&buf[..len]) else {
                    continue;
                };
                if !dispatch(packet, &pending, &observations, &tx, &pump_socket).await {
                    break;
                }
            }
        });

        self.socket = Some(socket);
        self.incoming = Some(rx);
        self.pump = Some(pump);
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        let socket = self.socket.clone().ok_or(Error::Closed)?;
        let id = self.next_message_id();
        let token = self.next_request_token();

        let mut packet = Packet::new();
        packet.header.set_version(1);
        packet
            .header
            .set_type(message_type(self.config.reliability));
        packet.header.code = MessageClass::Request(RequestType::Put);
        packet.header.message_id = id;
        packet.set_token(token);
        add_path(&mut packet, topic);
        packet.payload = payload.to_vec();

        let bytes = packet
            .to_bytes()
            .map_err(|err| Error::Codec(err.to_string()))?;

        match self.config.reliability {
            Reliability::NonConfirmable => match socket.send(&bytes).await {
                Err(error) if !refused(&error) => Err(Error::Transport(error.to_string())),
                _ => Ok(()),
            },
            Reliability::Confirmable => {
                let reply = self.exchange(id, &bytes, &socket).await?;
                answered(reply.code, &reply.payload)
            }
        }
    }

    /// Registers an observation of a resource path, so its notifications arrive.
    ///
    /// A notification carries no path of its own, only the token of the request
    /// that registered it (RFC 7641 section 3.2), so the client remembers which
    /// path each token observes and delivers every notification under that path.
    /// Registering a path again reuses its token, which a server takes as renewing
    /// the observation rather than adding a second one (RFC 7641 section 4.1), so a
    /// node can register again now and then in case the server has restarted.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the server has registered the observation, or has acknowledged
    /// the request with its response to follow.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the server refuses the request, answers it
    /// without registering an observation, or never acknowledges it, and
    /// [`Error::Closed`] if the transport is not connected.
    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        let socket = self.socket.clone().ok_or(Error::Closed)?;
        let id = self.next_message_id();
        let path = topic.trim_matches('/');
        let observed = self
            .observations
            .lock()
            .expect("observations lock")
            .iter()
            .find(|(_, observation)| observation.path == path)
            .map(|(token, _)| token.clone());
        let token = observed.unwrap_or_else(|| self.next_request_token());

        let mut packet = Packet::new();
        packet.header.set_version(1);
        packet.header.set_type(MessageType::Confirmable);
        packet.header.code = MessageClass::Request(RequestType::Get);
        packet.header.message_id = id;
        packet.set_token(token.clone());
        // An empty observe option value registers the observation (RFC 7641).
        packet.add_option(CoapOption::Observe, Vec::new());
        add_path(&mut packet, topic);

        let bytes = packet
            .to_bytes()
            .map_err(|err| Error::Codec(err.to_string()))?;

        self.observations.lock().expect("observations lock").insert(
            token.clone(),
            Observation {
                path: topic.trim_matches('/').to_owned(),
                freshest: None,
            },
        );
        let registered = match self.exchange(id, &bytes, &socket).await {
            Ok(reply) => answered(reply.code, &reply.payload).and_then(|()| {
                if reply.code == MessageClass::Empty || reply.observe.is_some() {
                    Ok(())
                } else {
                    Err(Error::Transport(format!(
                        "the server answered {topic} without registering an observation, so \
                         no changes will follow"
                    )))
                }
            }),
            Err(error) => Err(error),
        };
        if registered.is_err() {
            self.observations
                .lock()
                .expect("observations lock")
                .remove(&token);
        }
        registered
    }
}

impl Receive for CoapTransport {
    /// Awaits the next notification from an observed resource.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next queued notification, or `None` once the
    /// background task has stopped and no further messages will arrive.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`] if the transport is not connected.
    async fn recv(&mut self) -> Result<Option<Message>> {
        let incoming = self.incoming.as_mut().ok_or(Error::Closed)?;
        Ok(incoming.recv().await)
    }
}

/// Maps a [`Reliability`] onto the CoAP message type used on the wire.
fn message_type(reliability: Reliability) -> MessageType {
    match reliability {
        Reliability::NonConfirmable => MessageType::NonConfirmable,
        Reliability::Confirmable => MessageType::Confirmable,
    }
}

/// Adds one `Uri-Path` option per non-empty segment of `topic`.
fn add_path(packet: &mut Packet, topic: &str) {
    for segment in topic.split('/').filter(|segment| !segment.is_empty()) {
        packet.add_option(CoapOption::UriPath, segment.as_bytes().to_vec());
    }
}

/// Reconstructs a resource path from a packet's `Uri-Path` options.
fn path_from_packet(packet: &Packet) -> String {
    match packet.get_option(CoapOption::UriPath) {
        Some(segments) => segments
            .iter()
            .map(|segment| String::from_utf8_lossy(segment).into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        None => String::new(),
    }
}

/// Routes one decoded packet, returning `false` when the inbound queue is gone.
async fn dispatch(
    packet: Packet,
    pending: &PendingAcks,
    observations: &Observations,
    tx: &mpsc::UnboundedSender<Message>,
    socket: &UdpSocket,
) -> bool {
    let observe = observe_value(&packet);
    match packet.header.get_type() {
        MessageType::Acknowledgement => {
            if let Some(waiter) = pending
                .lock()
                .expect("pending lock")
                .remove(&packet.header.message_id)
            {
                let _ = waiter.send(Answer::Acknowledged(Reply {
                    code: packet.header.code,
                    payload: packet.payload.clone(),
                    observe,
                }));
            }
            match observe {
                Some(sequence) if is_success(packet.header.code) => {
                    notify(packet, sequence, observations, tx) != Delivered::Gone
                }
                _ => true,
            }
        }
        MessageType::Reset => {
            if let Some(waiter) = pending
                .lock()
                .expect("pending lock")
                .remove(&packet.header.message_id)
            {
                let _ = waiter.send(Answer::Reset);
            }
            true
        }
        kind => {
            let confirmable = kind == MessageType::Confirmable;
            let message_id = packet.header.message_id;
            let delivered = match (packet.header.code, observe) {
                (MessageClass::Response(_), Some(sequence)) if is_success(packet.header.code) => {
                    notify(packet, sequence, observations, tx)
                }
                (MessageClass::Response(_), _) => end(&packet, observations),
                _ => Delivered::Unknown,
            };
            match delivered {
                Delivered::Unknown if confirmable => reset(message_id, socket).await,
                Delivered::Taken if confirmable => acknowledge(message_id, socket).await,
                _ => {}
            }
            delivered != Delivered::Gone
        }
    }
}

/// Forgets the observation a response without a sequence number ends.
///
/// A server that can no longer notify, such as when the resource is deleted,
/// answers with a response carrying no Observe option and removes the client from
/// its observers (RFC 7641 section 3.2), so the client forgets the token too.
fn end(packet: &Packet, observations: &Observations) -> Delivered {
    match observations
        .lock()
        .expect("observations lock")
        .remove(packet.get_token())
    {
        Some(_) => Delivered::Taken,
        None => Delivered::Unknown,
    }
}

/// What became of a notification.
#[derive(PartialEq, Eq)]
enum Delivered {
    /// It was queued for a receive, or dropped as older than one already queued.
    Taken,
    /// Its token matches no observation, so the client does not want it.
    Unknown,
    /// The queue behind the receive is gone.
    Gone,
}

/// Queues a notification under the path its token observes.
///
/// One no fresher than the last, by the rule of RFC 7641 section 3.4, is dropped
/// rather than delivered out of order.
fn notify(
    packet: Packet,
    sequence: u32,
    observations: &Observations,
    tx: &mpsc::UnboundedSender<Message>,
) -> Delivered {
    let path = {
        let mut observations = observations.lock().expect("observations lock");
        let Some(observation) = observations.get_mut(packet.get_token()) else {
            return Delivered::Unknown;
        };
        let now = Instant::now();
        if !fresher(observation.freshest, sequence, now) {
            return Delivered::Taken;
        }
        observation.freshest = Some((sequence, now));
        observation.path.clone()
    };
    match tx.send(Message::new(path, packet.payload)) {
        Ok(()) => Delivered::Taken,
        Err(_) => Delivered::Gone,
    }
}

/// Whether a notification numbered `incoming`, arriving at `now`, is newer than
/// the freshest so far, by the rule of RFC 7641 section 3.4.
fn fresher(freshest: Option<(u32, Instant)>, incoming: u32, now: Instant) -> bool {
    const HALF: u32 = 1 << 23;
    let Some((known, at)) = freshest else {
        return true;
    };
    (known < incoming && incoming - known < HALF)
        || (known > incoming && known - incoming > HALF)
        || now > at + Duration::from_secs(128)
}

/// Reads the Observe option's value, a big-endian integer of up to three bytes.
fn observe_value(packet: &Packet) -> Option<u32> {
    let value = packet.get_option(CoapOption::Observe)?.front()?;
    Some(
        value
            .iter()
            .fold(0u32, |sum, byte| (sum << 8) | u32::from(*byte)),
    )
}

/// Whether a code is a 2.xx success response.
fn is_success(code: MessageClass) -> bool {
    u8::from(code) >> 5 == 2
}

/// Sends an empty acknowledgment for a confirmable message.
async fn acknowledge(message_id: u16, socket: &UdpSocket) {
    empty(MessageType::Acknowledgement, message_id, socket).await;
}

/// Rejects a message with a Reset, which RFC 7641 section 3.6 also makes the way a
/// client says it no longer wants an observation's notifications.
async fn reset(message_id: u16, socket: &UdpSocket) {
    empty(MessageType::Reset, message_id, socket).await;
}

/// Sends an empty message of the given type, answering a message id.
async fn empty(kind: MessageType, message_id: u16, socket: &UdpSocket) {
    let mut packet = Packet::new();
    packet.header.set_version(1);
    packet.header.set_type(kind);
    packet.header.code = MessageClass::Empty;
    packet.header.message_id = message_id;
    if let Ok(bytes) = packet.to_bytes() {
        let _ = socket.send(&bytes).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reliability_defaults_to_confirmable() {
        let config = CoapConfig::new("localhost", 5683);
        assert_eq!(config.reliability, Reliability::Confirmable);
    }

    #[test]
    fn setters_update_the_configuration() {
        let config = CoapConfig::new("localhost", 5683)
            .reliability(Reliability::NonConfirmable)
            .ack_timeout(Duration::from_millis(250))
            .max_retransmits(1)
            .bind("127.0.0.1:0");
        assert_eq!(config.reliability, Reliability::NonConfirmable);
        assert_eq!(config.ack_timeout, Duration::from_millis(250));
        assert_eq!(config.max_retransmits, 1);
        assert_eq!(config.bind, "127.0.0.1:0");
    }

    #[test]
    fn a_notification_is_fresher_by_the_rule_of_rfc_7641_section_3_4() {
        let then = Instant::now();
        let soon = then + Duration::from_secs(1);
        assert!(
            fresher(None, 0, then),
            "the first notification is always fresh"
        );
        assert!(fresher(Some((1, then)), 2, soon));
        assert!(
            !fresher(Some((2, then)), 1, soon),
            "an older number is stale"
        );
        assert!(!fresher(Some((2, then)), 2, soon), "a repeat is stale");
        assert!(
            fresher(Some((0x00FF_FFFF, then)), 0, soon),
            "the 24-bit number wraps"
        );
        assert!(
            !fresher(Some((0, then)), 0x0080_0001, soon),
            "a jump past half the space reads as older"
        );
        assert!(
            fresher(Some((5, then)), 1, then + Duration::from_secs(129)),
            "after 128 seconds any notification is fresher"
        );
    }

    #[test]
    fn an_observe_value_is_a_big_endian_integer_of_up_to_three_bytes() {
        let mut packet = Packet::new();
        packet.add_option(CoapOption::Observe, vec![0x01, 0x02, 0x03]);
        assert_eq!(observe_value(&packet), Some(0x0001_0203));
        let mut empty = Packet::new();
        empty.add_option(CoapOption::Observe, Vec::new());
        assert_eq!(
            observe_value(&empty),
            Some(0),
            "a zero-length value is zero"
        );
        assert_eq!(observe_value(&Packet::new()), None);
    }

    #[test]
    fn path_round_trips_through_uri_path_options() {
        let mut packet = Packet::new();
        add_path(&mut packet, "sensors/1/temperature");
        assert_eq!(path_from_packet(&packet), "sensors/1/temperature");
    }

    #[test]
    fn leading_and_repeated_slashes_are_ignored() {
        let mut packet = Packet::new();
        add_path(&mut packet, "/sensors//1/");
        assert_eq!(path_from_packet(&packet), "sensors/1");
    }

    #[tokio::test]
    async fn send_before_connect_reports_closed() {
        let mut transport = CoapTransport::new(CoapConfig::new("localhost", 5683));
        assert!(matches!(
            transport.send("t", b"x").await,
            Err(Error::Closed)
        ));
    }

    #[tokio::test]
    async fn subscribe_before_connect_reports_closed() {
        let mut transport = CoapTransport::new(CoapConfig::new("localhost", 5683));
        assert!(matches!(transport.subscribe("t").await, Err(Error::Closed)));
    }

    #[tokio::test]
    async fn recv_before_connect_reports_closed() {
        let mut transport = CoapTransport::new(CoapConfig::new("localhost", 5683));
        assert!(matches!(transport.recv().await, Err(Error::Closed)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn confirmable_send_without_a_server_times_out() {
        let config = CoapConfig::new("127.0.0.1", 1)
            .ack_timeout(Duration::from_millis(20))
            .max_retransmits(1);
        let mut transport = CoapTransport::new(config);
        transport.connect().await.expect("bind socket");
        assert!(transport.send("sensors/1", b"x").await.is_err());
    }
}
