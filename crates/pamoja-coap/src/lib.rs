//! CoAP transport for the pamoja SDK.
//!
//! [`CoapTransport`] implements the core [`Transport`]
//! trait on top of the pure-Rust [`coap_lite`] message codec and a UDP socket, so
//! an application can talk to constrained RESTful devices through the same
//! protocol-agnostic surface it uses for every other transport.
//!
//! CoAP is connectionless: [`connect`](Transport::connect) binds a local UDP
//! socket and points it at the server, then spawns a background task that decodes
//! inbound datagrams. A [`send`](Transport::send) is a CoAP `PUT` to a resource
//! path, and a [`subscribe`](Transport::subscribe) registers an RFC 7641 observe on
//! a resource so the server's notifications are forwarded to an internal queue that
//! [`recv`](CoapTransport::recv) drains.
//!
//! Delivery follows the configured [`Reliability`]: [`Reliability::Confirmable`]
//! messages are acknowledged with retransmission (at-least-once), while
//! [`Reliability::NonConfirmable`] messages are fire-and-forget (at-most-once),
//! which suits the cheapest, most power-constrained devices.
//!
//! # Examples
//!
//! ```no_run
//! use pamoja_core::Transport;
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
use std::time::Duration;

use coap_lite::{CoapOption, MessageClass, MessageType, Packet, RequestType};
use pamoja_core::{Error, Result, Transport};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

/// Outstanding confirmable requests keyed by message id, each awaiting its ACK.
type PendingAcks = Arc<Mutex<HashMap<u16, oneshot::Sender<()>>>>;

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
    /// delivery, waits two seconds for the first acknowledgement, and retransmits
    /// up to four times.
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

    /// Sets how long to wait for the first acknowledgement of a confirmable request.
    ///
    /// The wait doubles for each retransmission, following the CoAP backoff.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the initial acknowledgement timeout.
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

/// A message received from an observed resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// The resource path the message was published to.
    pub topic: String,
    /// The raw payload bytes.
    pub payload: Vec<u8>,
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
    pump: Option<JoinHandle<()>>,
    next_id: u16,
    next_token: u16,
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
            pump: None,
            next_id: 0,
            next_token: 0,
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

    /// Awaits the next notification from an observed resource.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next queued notification, or `None` once the
    /// background task has stopped and no further messages will arrive.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`] if the transport is
    /// not connected.
    pub async fn recv(&mut self) -> Result<Option<Message>> {
        let incoming = self.incoming.as_mut().ok_or(Error::Closed)?;
        Ok(incoming.recv().await)
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
        Ok(())
    }

    /// Returns the next message id and advances the counter.
    fn next_message_id(&mut self) -> u16 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    /// Returns a fresh request token and advances the counter.
    fn next_request_token(&mut self) -> Vec<u8> {
        let token = self.next_token;
        self.next_token = self.next_token.wrapping_add(1);
        token.to_be_bytes().to_vec()
    }

    /// Transmits a confirmable datagram and waits for its acknowledgement,
    /// retransmitting with a doubling timeout up to the configured limit.
    async fn send_confirmable(&mut self, id: u16, bytes: &[u8], socket: &UdpSocket) -> Result<()> {
        let mut timeout = self.config.ack_timeout;
        for _ in 0..=self.config.max_retransmits {
            let (tx, rx) = oneshot::channel();
            self.pending.lock().expect("pending lock").insert(id, tx);
            socket
                .send(bytes)
                .await
                .map_err(|err| Error::Transport(err.to_string()))?;
            match tokio::time::timeout(timeout, rx).await {
                Ok(Ok(())) => return Ok(()),
                Ok(Err(_)) => return Err(Error::Closed),
                Err(_) => {
                    self.pending.lock().expect("pending lock").remove(&id);
                    timeout = timeout.saturating_mul(2);
                }
            }
        }
        Err(Error::Transport(format!(
            "no acknowledgement for message {id}"
        )))
    }
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
        let pump_socket = Arc::clone(&socket);
        let pump = tokio::spawn(async move {
            let mut buf = vec![0u8; 1500];
            while let Ok(len) = pump_socket.recv(&mut buf).await {
                let Ok(packet) = Packet::from_bytes(&buf[..len]) else {
                    continue;
                };
                if !dispatch(packet, &pending, &tx, &pump_socket).await {
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
            Reliability::NonConfirmable => socket
                .send(&bytes)
                .await
                .map(|_| ())
                .map_err(|err| Error::Transport(err.to_string())),
            Reliability::Confirmable => self.send_confirmable(id, &bytes, &socket).await,
        }
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        let socket = self.socket.clone().ok_or(Error::Closed)?;
        let id = self.next_message_id();
        let token = self.next_request_token();

        let mut packet = Packet::new();
        packet.header.set_version(1);
        packet.header.set_type(MessageType::Confirmable);
        packet.header.code = MessageClass::Request(RequestType::Get);
        packet.header.message_id = id;
        packet.set_token(token);
        // An empty observe option value registers the observation (RFC 7641).
        packet.add_option(CoapOption::Observe, Vec::new());
        add_path(&mut packet, topic);

        let bytes = packet
            .to_bytes()
            .map_err(|err| Error::Codec(err.to_string()))?;

        self.send_confirmable(id, &bytes, &socket).await
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
    tx: &mpsc::UnboundedSender<Message>,
    socket: &UdpSocket,
) -> bool {
    match packet.header.get_type() {
        MessageType::Acknowledgement => {
            if let Some(waiter) = pending
                .lock()
                .expect("pending lock")
                .remove(&packet.header.message_id)
            {
                let _ = waiter.send(());
            }
            // A piggybacked observe notification rides in on the ACK.
            if packet.get_option(CoapOption::Observe).is_some() {
                return enqueue(packet, tx);
            }
            true
        }
        MessageType::Confirmable => {
            acknowledge(&packet, socket).await;
            enqueue(packet, tx)
        }
        MessageType::NonConfirmable => enqueue(packet, tx),
        MessageType::Reset => {
            if let Some(waiter) = pending
                .lock()
                .expect("pending lock")
                .remove(&packet.header.message_id)
            {
                let _ = waiter.send(());
            }
            true
        }
    }
}

/// Sends an empty acknowledgement for a confirmable notification.
async fn acknowledge(packet: &Packet, socket: &UdpSocket) {
    let mut ack = Packet::new();
    ack.header.set_version(1);
    ack.header.set_type(MessageType::Acknowledgement);
    ack.header.code = MessageClass::Empty;
    ack.header.message_id = packet.header.message_id;
    if let Ok(bytes) = ack.to_bytes() {
        let _ = socket.send(&bytes).await;
    }
}

/// Queues a notification, returning `false` once the receiver has been dropped.
fn enqueue(packet: Packet, tx: &mpsc::UnboundedSender<Message>) -> bool {
    let message = Message {
        topic: path_from_packet(&packet),
        payload: packet.payload,
    };
    tx.send(message).is_ok()
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
