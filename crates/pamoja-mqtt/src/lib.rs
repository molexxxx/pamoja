//! MQTT transport for the pamoja SDK.
//!
//! [`MqttTransport`] implements the core [`Transport`] and [`Receive`] traits on
//! top of the pure-Rust [`rumqttc`] client, so an application can publish to and
//! subscribe from an MQTT broker through the same protocol-agnostic surface it uses
//! for every other transport.
//!
//! Once [`connect`](Transport::connect) succeeds the transport owns a background
//! task that drives the MQTT event loop: it answers keep-alive pings, completes
//! delivery handshakes, and forwards inbound messages to an internal queue that
//! [`recv`](Receive::recv) drains. Publishing and subscribing use the default
//! [`QualityOfService`] configured on the transport.
//!
//! The client speaks MQTT 3.1.1 with a clean session, so a connection starts with
//! no subscriptions and a reconnect places them again. Topics and filters are
//! checked against the rules of the OASIS MQTT 3.1.1 standard, section 4.7, before
//! anything is sent, and a message too large for the connection's packet limit is
//! refused rather than ending the connection.
//!
//! # Examples
//!
//! ```no_run
//! use pamoja_core::{Receive, Transport};
//! use pamoja_mqtt::{MqttConfig, MqttTransport};
//!
//! # async fn run() -> pamoja_core::Result<()> {
//! let mut transport = MqttTransport::new(MqttConfig::new("sensor-1", "localhost", 1883));
//! transport.connect().await?;
//! transport.subscribe("sensors/+/temperature").await?;
//! transport.send("sensors/1/temperature", b"21.5").await?;
//!
//! if let Some(message) = transport.recv().await? {
//!     println!("{}: {} bytes", message.topic, message.payload.len());
//! }
//! # Ok(())
//! # }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub use pamoja_core::Message;
use pamoja_core::{Error, Receive, Result, Transport};
use rumqttc::{AsyncClient, ClientError, ConnectionError, Event, MqttOptions, Packet, QoS};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

/// The delivery guarantee applied to published and subscribed messages.
///
/// These map one-to-one onto the MQTT protocol's quality-of-service levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualityOfService {
    /// Fire and forget: the broker does not acknowledge delivery.
    AtMostOnce,
    /// The message is delivered at least once and acknowledged.
    AtLeastOnce,
    /// The message is delivered exactly once via a four-step handshake.
    ExactlyOnce,
}

impl From<QualityOfService> for QoS {
    fn from(value: QualityOfService) -> Self {
        match value {
            QualityOfService::AtMostOnce => QoS::AtMostOnce,
            QualityOfService::AtLeastOnce => QoS::AtLeastOnce,
            QualityOfService::ExactlyOnce => QoS::ExactlyOnce,
        }
    }
}

/// The largest packet a connection sends or accepts unless configured otherwise, in
/// bytes.
pub const DEFAULT_MAX_PACKET_SIZE: usize = 10 * 1024;

/// Connection settings for an [`MqttTransport`].
///
/// Construct with [`MqttConfig::new`] and refine with the chained setters; every
/// field has a sensible default so only the broker address and client id are
/// required.
#[derive(Clone, Debug)]
pub struct MqttConfig {
    client_id: String,
    host: String,
    port: u16,
    keep_alive: Duration,
    capacity: usize,
    qos: QualityOfService,
    max_packet_size: usize,
}

impl MqttConfig {
    /// Creates a configuration for the given client id and broker address.
    ///
    /// # Arguments
    ///
    /// * `client_id` - the MQTT client identifier presented to the broker.
    /// * `host` - the broker hostname or IP address.
    /// * `port` - the broker TCP port, conventionally `1883` for plaintext MQTT.
    ///
    /// # Returns
    ///
    /// A configuration with a 30-second keep-alive, a request capacity of 64, a
    /// default quality of service of [`QualityOfService::AtLeastOnce`], and a
    /// packet limit of [`DEFAULT_MAX_PACKET_SIZE`].
    pub fn new(client_id: impl Into<String>, host: impl Into<String>, port: u16) -> Self {
        Self {
            client_id: client_id.into(),
            host: host.into(),
            port,
            keep_alive: Duration::from_secs(30),
            capacity: 64,
            qos: QualityOfService::AtLeastOnce,
            max_packet_size: DEFAULT_MAX_PACKET_SIZE,
        }
    }

    /// Sets the largest packet the connection sends or accepts, in bytes.
    ///
    /// A packet is a message's topic and payload plus a few bytes of framing. A send
    /// whose packet would be larger is refused and the connection stays up, but a
    /// larger packet arriving from the broker ends the connection, so every client
    /// that shares a topic needs a limit that fits it. MQTT itself allows a packet of
    /// up to 268,435,455 bytes after its fixed header.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the limit, which applies to both directions.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn max_packet_size(mut self, bytes: usize) -> Self {
        self.max_packet_size = bytes;
        self
    }

    /// Sets the keep-alive interval used to hold the connection open.
    ///
    /// # Arguments
    ///
    /// * `interval` - how often the client pings the broker when otherwise idle.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn keep_alive(mut self, interval: Duration) -> Self {
        self.keep_alive = interval;
        self
    }

    /// Sets the bound on outstanding client requests buffered toward the broker.
    ///
    /// # Arguments
    ///
    /// * `capacity` - the request channel capacity; values below one are clamped
    ///   to one.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity.max(1);
        self
    }

    /// Sets the default quality of service for publishes and subscriptions.
    ///
    /// # Arguments
    ///
    /// * `qos` - the delivery guarantee applied by [`send`](Transport::send) and
    ///   [`subscribe`](Transport::subscribe).
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn qos(mut self, qos: QualityOfService) -> Self {
        self.qos = qos;
        self
    }
}

/// An MQTT client that implements the core [`Transport`] and [`Receive`] traits.
///
/// A transport is created disconnected; [`connect`](Transport::connect) opens the
/// link and spawns the background task that runs the MQTT event loop for the life
/// of the connection. Inbound messages are queued and read with
/// [`recv`](Receive::recv).
///
/// A connection can end without being asked to: the broker restarts, another client
/// connects with the same client id, or a packet over the limit arrives. From then
/// on [`is_connected`](MqttTransport::is_connected) is `false`, a send answers
/// [`Error::Closed`], and the next receive reports why the connection ended, until
/// [`connect`](Transport::connect) opens a new one.
pub struct MqttTransport {
    config: MqttConfig,
    client: Option<AsyncClient>,
    incoming: Option<mpsc::UnboundedReceiver<Result<Message>>>,
    pump: Option<JoinHandle<()>>,
    ended: Arc<AtomicBool>,
}

impl MqttTransport {
    /// Creates a transport from the given configuration without connecting.
    ///
    /// # Arguments
    ///
    /// * `config` - the broker connection settings.
    ///
    /// # Returns
    ///
    /// A disconnected transport ready for [`connect`](Transport::connect).
    pub fn new(config: MqttConfig) -> Self {
        Self {
            config,
            client: None,
            incoming: None,
            pump: None,
            ended: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Reports whether the transport currently holds an active connection.
    ///
    /// # Returns
    ///
    /// `true` once [`connect`](Transport::connect) has succeeded, until
    /// [`disconnect`](MqttTransport::disconnect) is called or the connection ends
    /// on its own.
    pub fn is_connected(&self) -> bool {
        self.live().is_ok()
    }

    /// The client, while its connection is up.
    ///
    /// The event loop marks the connection ended before it reports why, so a
    /// receive that has seen the report never finds the transport still up.
    fn live(&self) -> Result<&AsyncClient> {
        match (&self.client, &self.pump) {
            (Some(client), Some(pump))
                if !pump.is_finished() && !self.ended.load(Ordering::SeqCst) =>
            {
                Ok(client)
            }
            _ => Err(Error::Closed),
        }
    }

    /// Closes the connection and stops the background event loop.
    ///
    /// Calling this on a transport that is not connected is a no-op.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the disconnect request has been issued and the event loop
    /// task has been stopped.
    ///
    /// # Errors
    ///
    /// This call is best-effort and does not surface broker errors raised while
    /// tearing down, so it currently always returns `Ok(())`.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(client) = self.client.take() {
            let _ = client.disconnect().await;
        }
        if let Some(pump) = self.pump.take() {
            pump.abort();
        }
        self.incoming = None;
        Ok(())
    }
}

impl Transport for MqttTransport {
    /// Opens a connection to the broker, closing any the transport already holds.
    ///
    /// The session is clean, so subscriptions from an earlier connection are gone
    /// and have to be placed again.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the broker has accepted the connection.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the broker cannot be reached or refuses the
    /// connection.
    async fn connect(&mut self) -> Result<()> {
        self.disconnect().await?;
        let mut options = MqttOptions::new(
            self.config.client_id.clone(),
            self.config.host.clone(),
            self.config.port,
        );
        options.set_keep_alive(self.config.keep_alive);
        options.set_max_packet_size(self.config.max_packet_size, self.config.max_packet_size);

        let (client, mut eventloop) = AsyncClient::new(options, self.config.capacity);
        let (tx, rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = oneshot::channel::<Result<()>>();
        let ended = Arc::new(AtomicBool::new(false));
        let marked = Arc::clone(&ended);

        let pump = tokio::spawn(async move {
            let mut ready_tx = Some(ready_tx);
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::ConnAck(_))) => {
                        if let Some(ready_tx) = ready_tx.take() {
                            let _ = ready_tx.send(Ok(()));
                        }
                    }
                    Ok(Event::Incoming(Packet::Publish(publish))) => {
                        let message = Message::new(publish.topic, publish.payload.to_vec());
                        if tx.send(Ok(message)).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(err) => {
                        marked.store(true, Ordering::SeqCst);
                        match ready_tx.take() {
                            Some(ready_tx) => {
                                let _ = ready_tx.send(Err(map_connection_error(err)));
                            }
                            None => {
                                let _ = tx.send(Err(Error::Transport(format!(
                                    "the connection to the broker ended: {err}"
                                ))));
                            }
                        }
                        break;
                    }
                }
            }
        });

        match ready_rx.await {
            Ok(Ok(())) => {
                self.client = Some(client);
                self.incoming = Some(rx);
                self.pump = Some(pump);
                self.ended = ended;
                Ok(())
            }
            Ok(Err(err)) => {
                pump.abort();
                Err(err)
            }
            Err(_) => {
                pump.abort();
                Err(Error::Transport(
                    "event loop closed before the connection was established".into(),
                ))
            }
        }
    }

    /// Publishes a payload to a topic under the configured quality of service.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the message is queued for the broker.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if MQTT does not allow the topic or the message
    /// is over the connection's packet limit, both before anything is sent, or
    /// [`Error::Closed`] if the transport is not connected.
    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        let client = self.live()?;
        check_topic(topic)?;
        let size = publish_size(topic, payload.len(), self.config.qos);
        if size > self.config.max_packet_size {
            return Err(Error::Transport(format!(
                "the message makes a {size}-byte packet, over this connection's {}-byte limit",
                self.config.max_packet_size
            )));
        }
        client
            .publish(topic, self.config.qos.into(), false, payload.to_vec())
            .await
            .map_err(map_client_error)
    }

    /// Subscribes to a topic filter under the configured quality of service.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the subscription is queued for the broker.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the filter places a wildcard where MQTT does
    /// not allow one, or [`Error::Closed`] if the transport is not connected.
    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        let client = self.live()?;
        check_filter(topic)?;
        client
            .subscribe(topic, self.config.qos.into())
            .await
            .map_err(map_client_error)
    }
}

impl Receive for MqttTransport {
    /// Awaits the next message from any subscribed topic.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next queued message, or `None` once the connection
    /// has ended and its reason has been reported.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`] if the transport is not connected, or
    /// [`Error::Transport`] once, saying why, when the connection ended on its own.
    async fn recv(&mut self) -> Result<Option<Message>> {
        let incoming = self.incoming.as_mut().ok_or(Error::Closed)?;
        incoming.recv().await.transpose()
    }
}

/// Refuses a topic MQTT does not allow a message to be published to.
///
/// The rules are the OASIS MQTT 3.1.1 standard's: a topic name holds no wildcard
/// (section 4.7.1) and is between one and 65,535 bytes of UTF-8 with no null
/// character (section 4.7.3).
fn check_topic(topic: &str) -> Result<()> {
    if topic.contains(['+', '#']) {
        return Err(Error::Transport(format!(
            "the topic {topic} holds a wildcard, which only a subscription may use"
        )));
    }
    check_length(topic)
}

/// Refuses a filter that places a wildcard where MQTT does not allow one.
///
/// The rules are the OASIS MQTT 3.1.1 standard's, section 4.7.1: `#` stands alone
/// in the last level, and `+` fills a whole level.
fn check_filter(filter: &str) -> Result<()> {
    let levels: Vec<&str> = filter.split('/').collect();
    let last = levels.len() - 1;
    let misplaced = levels.iter().enumerate().any(|(at, level)| {
        let hash = level.contains('#') && (*level != "#" || at != last);
        let plus = level.contains('+') && *level != "+";
        hash || plus
    });
    if misplaced {
        return Err(Error::Transport(format!(
            "the filter {filter} places a wildcard where MQTT does not allow one: \
             # only alone in the last level, + only as a whole level"
        )));
    }
    check_length(filter)
}

/// Refuses a topic or filter outside the lengths and characters MQTT allows.
fn check_length(topic: &str) -> Result<()> {
    if topic.is_empty() {
        return Err(Error::Transport(
            "an MQTT topic has at least one character".into(),
        ));
    }
    if topic.contains('\0') {
        return Err(Error::Transport(
            "an MQTT topic cannot hold a null character".into(),
        ));
    }
    if topic.len() > usize::from(u16::MAX) {
        return Err(Error::Transport(
            "an MQTT topic is at most 65,535 bytes".into(),
        ));
    }
    Ok(())
}

/// The size of the packet that publishes a message, framing included.
///
/// A publish is a fixed-header byte, a remaining length of one to four bytes, a
/// two-byte topic length, the topic, a two-byte packet identifier above QoS 0, and
/// the payload (OASIS MQTT 3.1.1, sections 2.2.3 and 3.3). This is the size the
/// event loop checks against the limit, so a send that would fail there is refused
/// before it can end the connection.
fn publish_size(topic: &str, payload: usize, qos: QualityOfService) -> usize {
    let packet_id = if qos == QualityOfService::AtMostOnce {
        0
    } else {
        2
    };
    let remaining = 2 + topic.len() + packet_id + payload;
    let length_bytes = match remaining {
        0..=127 => 1,
        128..=16_383 => 2,
        16_384..=2_097_151 => 3,
        _ => 4,
    };
    1 + length_bytes + remaining
}

/// Maps a `rumqttc` client error onto the shared transport error.
///
/// Topics and filters are checked before a request is made, so the one failure
/// left is a request the event loop can no longer take: the connection has ended.
fn map_client_error(_: ClientError) -> Error {
    Error::Closed
}

/// Maps a `rumqttc` event-loop error onto the shared transport error.
fn map_connection_error(err: ConnectionError) -> Error {
    Error::Transport(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// Returns a TCP port with no listener bound, for negative connection tests.
    fn unused_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        listener.local_addr().expect("local addr").port()
    }

    #[test]
    fn quality_of_service_maps_to_rumqttc() {
        assert_eq!(QoS::from(QualityOfService::AtMostOnce), QoS::AtMostOnce);
        assert_eq!(QoS::from(QualityOfService::AtLeastOnce), QoS::AtLeastOnce);
        assert_eq!(QoS::from(QualityOfService::ExactlyOnce), QoS::ExactlyOnce);
    }

    #[test]
    fn capacity_is_clamped_to_at_least_one() {
        let config = MqttConfig::new("c", "localhost", 1883).capacity(0);
        assert_eq!(config.capacity, 1);
    }

    #[test]
    fn filters_follow_the_examples_in_section_4_7_1() {
        for valid in [
            "sport/tennis/player1/#",
            "#",
            "sport/tennis/#",
            "+",
            "+/tennis/#",
            "sport/+/player1",
            "/+",
            "+/+",
        ] {
            assert!(check_filter(valid).is_ok(), "{valid} is a valid filter");
        }
        for invalid in ["sport/tennis#", "sport/tennis/#/ranking", "sport+"] {
            assert!(
                matches!(check_filter(invalid), Err(Error::Transport(_))),
                "{invalid} is not a valid filter"
            );
        }
    }

    #[test]
    fn a_topic_to_publish_to_holds_no_wildcard_and_has_a_length() {
        assert!(check_topic("sport/tennis/player1").is_ok());
        assert!(check_topic("/finance").is_ok());
        for refused in ["sport/+/player1", "sport/#", "", "a\0b"] {
            assert!(
                matches!(check_topic(refused), Err(Error::Transport(_))),
                "{refused:?} is refused"
            );
        }
        let longest = "t".repeat(65_535);
        assert!(check_topic(&longest).is_ok());
        assert!(check_topic(&format!("{longest}t")).is_err());
    }

    #[test]
    fn a_publish_packet_grows_its_length_field_at_the_table_2_4_boundaries() {
        use QualityOfService::{AtLeastOnce, AtMostOnce};
        assert_eq!(publish_size("a", 124, AtMostOnce), 1 + 1 + 127);
        assert_eq!(publish_size("a", 125, AtMostOnce), 1 + 2 + 128);
        assert_eq!(publish_size("a", 122, AtLeastOnce), 1 + 1 + 127);
        assert_eq!(publish_size("a", 16_380, AtMostOnce), 1 + 2 + 16_383);
        assert_eq!(publish_size("a", 16_381, AtMostOnce), 1 + 3 + 16_384);
        assert_eq!(publish_size("a", 2_097_148, AtMostOnce), 1 + 3 + 2_097_151);
        assert_eq!(publish_size("a", 2_097_149, AtMostOnce), 1 + 4 + 2_097_152);
    }

    #[test]
    fn a_temperature_reading_packs_into_the_sizes_the_guide_lists() {
        use QualityOfService::{AtLeastOnce, AtMostOnce};
        let topic = "sensors/1/temperature";
        assert_eq!(publish_size(topic, 100, AtMostOnce), 125);
        assert_eq!(publish_size(topic, 100, AtLeastOnce), 127);
        assert_eq!(publish_size(topic, 10_000, AtMostOnce), 10_026);
        assert_eq!(publish_size(topic, 10_000, AtLeastOnce), 10_028);
        assert_eq!(publish_size(topic, 10_215, AtMostOnce), 10_241);
        assert_eq!(publish_size(topic, 10_215, AtLeastOnce), 10_243);
        assert!(publish_size(topic, 10_214, AtMostOnce) <= DEFAULT_MAX_PACKET_SIZE);
    }

    #[test]
    fn the_packet_limit_defaults_to_ten_kibibytes_and_can_be_raised() {
        let config = MqttConfig::new("c", "localhost", 1883);
        assert_eq!(config.max_packet_size, DEFAULT_MAX_PACKET_SIZE);
        assert_eq!(config.max_packet_size(65_536).max_packet_size, 65_536);
    }

    #[tokio::test]
    async fn send_before_connect_reports_closed() {
        let mut transport = MqttTransport::new(MqttConfig::new("c", "localhost", 1883));
        assert!(matches!(
            transport.send("t", b"x").await,
            Err(Error::Closed)
        ));
    }

    #[tokio::test]
    async fn subscribe_before_connect_reports_closed() {
        let mut transport = MqttTransport::new(MqttConfig::new("c", "localhost", 1883));
        assert!(matches!(transport.subscribe("t").await, Err(Error::Closed)));
    }

    #[tokio::test]
    async fn recv_before_connect_reports_closed() {
        let mut transport = MqttTransport::new(MqttConfig::new("c", "localhost", 1883));
        assert!(matches!(transport.recv().await, Err(Error::Closed)));
    }

    #[tokio::test]
    async fn connect_to_unavailable_broker_fails() {
        let config = MqttConfig::new("c", "127.0.0.1", unused_port());
        let mut transport = MqttTransport::new(config.keep_alive(Duration::from_secs(1)));
        assert!(matches!(
            transport.connect().await,
            Err(Error::Transport(_))
        ));
        assert!(!transport.is_connected());
    }
}
