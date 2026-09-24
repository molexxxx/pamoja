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
//! Beyond [`send`](Transport::send), [`publish`](MqttTransport::publish) takes the
//! quality of service and retain flag of one message and returns a [`Delivery`] that
//! settles when the broker acknowledges it. A configuration can carry a username and
//! password, a last-will message the broker publishes if the client drops off without
//! saying goodbye, and, with the default `tls` feature, [`Tls`] settings for a broker
//! on port 8883.
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

mod delivery;
#[cfg(feature = "tls")]
mod tls;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub use delivery::Delivery;
pub use pamoja_core::Message;
use pamoja_core::{Error, Receive, Result, Transport};
use rumqttc::{
    AsyncClient, ClientError, ConnectReturnCode, ConnectionError, Event, LastWill, MqttOptions,
    Outgoing, Packet, QoS,
};
#[cfg(feature = "tls")]
pub use tls::Tls;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use delivery::{InFlight, Requested};

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

/// A message the broker publishes on the client's behalf if the client's connection ends
/// without a `DISCONNECT`: the network dropped, the power failed, or the keep-alive ran
/// out.
///
/// A device that publishes `online` to a status topic when it connects usually leaves a
/// will of `offline` on the same topic, retained, so anyone watching sees it go (OASIS MQTT
/// 3.1.1, section 3.1.2.5). A client that disconnects on purpose leaves no will behind.
///
/// # Examples
///
/// ```
/// use pamoja_mqtt::{MqttConfig, QualityOfService, Will};
///
/// let will = Will::new("sites/pump-3/status", b"offline")
///     .qos(QualityOfService::AtLeastOnce)
///     .retained();
/// let config = MqttConfig::new("pump-3", "localhost", 1883).last_will(will);
/// assert_eq!(config.will().map(Will::topic), Some("sites/pump-3/status"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Will {
    topic: String,
    payload: Vec<u8>,
    qos: QualityOfService,
    retain: bool,
}

impl Will {
    /// Creates a will to publish to a topic, at QoS 0 and not retained.
    ///
    /// # Arguments
    ///
    /// * `topic` - where the broker publishes it; a topic name, with no wildcard.
    /// * `payload` - what it publishes.
    ///
    /// # Returns
    ///
    /// The will.
    pub fn new(topic: impl Into<String>, payload: impl Into<Vec<u8>>) -> Will {
        Will {
            topic: topic.into(),
            payload: payload.into(),
            qos: QualityOfService::AtMostOnce,
            retain: false,
        }
    }

    /// Sets the quality of service the broker publishes the will at.
    ///
    /// # Arguments
    ///
    /// * `qos` - the delivery guarantee.
    ///
    /// # Returns
    ///
    /// The updated will, for chaining.
    pub fn qos(mut self, qos: QualityOfService) -> Will {
        self.qos = qos;
        self
    }

    /// Has the broker retain the will, so a client that subscribes later still sees it.
    ///
    /// # Returns
    ///
    /// The updated will, for chaining.
    pub fn retained(mut self) -> Will {
        self.retain = true;
        self
    }

    /// Returns the topic the will is published to.
    ///
    /// # Returns
    ///
    /// The topic.
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Returns what the will publishes.
    ///
    /// # Returns
    ///
    /// The payload.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Returns the quality of service the will is published at.
    ///
    /// # Returns
    ///
    /// The delivery guarantee.
    pub fn quality_of_service(&self) -> QualityOfService {
        self.qos
    }

    /// Returns whether the broker retains the will.
    ///
    /// # Returns
    ///
    /// `true` for a retained will.
    pub fn is_retained(&self) -> bool {
        self.retain
    }
}

/// How one message is published: its quality of service, and whether the broker keeps it.
///
/// A retained message is the one the broker hands to every client that subscribes to the
/// topic later, so a new dashboard sees the last reading at once rather than waiting for the
/// next (OASIS MQTT 3.1.1, section 3.3.1.3). Publishing an empty retained message clears the
/// one the broker holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PublishOptions {
    qos: Option<QualityOfService>,
    retain: bool,
}

impl PublishOptions {
    /// Options that publish at the connection's quality of service, not retained.
    ///
    /// # Returns
    ///
    /// The options.
    pub fn new() -> PublishOptions {
        PublishOptions::default()
    }

    /// Publishes at a quality of service other than the connection's.
    ///
    /// # Arguments
    ///
    /// * `qos` - the delivery guarantee for this message.
    ///
    /// # Returns
    ///
    /// The updated options, for chaining.
    pub fn qos(mut self, qos: QualityOfService) -> PublishOptions {
        self.qos = Some(qos);
        self
    }

    /// Has the broker retain the message for clients that subscribe later.
    ///
    /// # Returns
    ///
    /// The updated options, for chaining.
    pub fn retained(mut self) -> PublishOptions {
        self.retain = true;
        self
    }

    /// Returns whether the message is retained.
    ///
    /// # Returns
    ///
    /// `true` for a retained message.
    pub fn is_retained(&self) -> bool {
        self.retain
    }
}

/// Connection settings for an [`MqttTransport`].
///
/// Construct with [`MqttConfig::new`] and refine with the chained setters; every
/// field has a sensible default so only the broker address and client id are
/// required.
#[derive(Clone)]
pub struct MqttConfig {
    client_id: String,
    host: String,
    port: u16,
    keep_alive: Duration,
    capacity: usize,
    qos: QualityOfService,
    max_packet_size: usize,
    credentials: Option<(String, String)>,
    will: Option<Will>,
    #[cfg(feature = "tls")]
    tls: Option<Tls>,
}

// A password never reaches a log or a panic message through this type.
impl core::fmt::Debug for MqttConfig {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut debug = f.debug_struct("MqttConfig");
        debug
            .field("client_id", &self.client_id)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("keep_alive", &self.keep_alive)
            .field("capacity", &self.capacity)
            .field("qos", &self.qos)
            .field("max_packet_size", &self.max_packet_size)
            .field(
                "username",
                &self.credentials.as_ref().map(|(username, _)| username),
            )
            .field("will", &self.will);
        #[cfg(feature = "tls")]
        debug.field("tls", &self.tls);
        debug.finish_non_exhaustive()
    }
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
            credentials: None,
            will: None,
            #[cfg(feature = "tls")]
            tls: None,
        }
    }

    /// Signs in to the broker with a username and password.
    ///
    /// The password travels in the clear unless the connection uses TLS, so a broker
    /// reached over a network worth protecting wants both.
    ///
    /// # Arguments
    ///
    /// * `username` - the name the broker knows the client by.
    /// * `password` - its password.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.credentials = Some((username.into(), password.into()));
        self
    }

    /// Leaves a message for the broker to publish if the connection ends without a
    /// `DISCONNECT`.
    ///
    /// # Arguments
    ///
    /// * `will` - the message, its topic, and how it is published.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn last_will(mut self, will: Will) -> Self {
        self.will = Some(will);
        self
    }

    /// Secures the connection with TLS.
    ///
    /// # Arguments
    ///
    /// * `tls` - the authorities to trust, and any client certificate to present.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    #[cfg(feature = "tls")]
    pub fn tls(mut self, tls: Tls) -> Self {
        self.tls = Some(tls);
        self
    }

    /// Returns the username the client signs in with.
    ///
    /// # Returns
    ///
    /// The username, or `None` for a client that does not sign in.
    pub fn username(&self) -> Option<&str> {
        self.credentials
            .as_ref()
            .map(|(username, _)| username.as_str())
    }

    /// Returns the will the broker holds for this client.
    ///
    /// # Returns
    ///
    /// The will, or `None` when there is none.
    pub fn will(&self) -> Option<&Will> {
        self.will.as_ref()
    }

    /// Returns whether the connection uses TLS.
    ///
    /// # Returns
    ///
    /// `true` once [`tls`](MqttConfig::tls) has been set.
    pub fn uses_tls(&self) -> bool {
        #[cfg(feature = "tls")]
        return self.tls.is_some();
        #[cfg(not(feature = "tls"))]
        return false;
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

/// The messages arriving on a transport's connection, readable apart from the transport.
///
/// [`recv`](Receive::recv) borrows the transport for as long as it waits, so a program that
/// waits for commands in one task while it publishes readings from another takes an inbox
/// with [`MqttTransport::inbox`] and waits on that instead. The inbox follows the transport
/// across reconnects.
#[derive(Clone, Default)]
pub struct Inbox(Arc<tokio::sync::Mutex<Option<mpsc::UnboundedReceiver<Result<Message>>>>>);

impl Inbox {
    /// Awaits the next message from any subscribed topic.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next queued message, or `None` once the connection has
    /// ended and its reason has been reported.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`] if the transport is not connected, or
    /// [`Error::Transport`] once, saying why, when the connection ended on its own.
    pub async fn recv(&self) -> Result<Option<Message>> {
        let mut held = self.0.lock().await;
        let incoming = held.as_mut().ok_or(Error::Closed)?;
        incoming.recv().await.transpose()
    }

    async fn replace(&self, incoming: Option<mpsc::UnboundedReceiver<Result<Message>>>) {
        *self.0.lock().await = incoming;
    }
}

impl core::fmt::Debug for Inbox {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Inbox").finish_non_exhaustive()
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
    inbox: Inbox,
    pump: Option<JoinHandle<()>>,
    ended: Arc<AtomicBool>,
    requested: Requested,
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
            inbox: Inbox::default(),
            pump: None,
            ended: Arc::new(AtomicBool::new(false)),
            requested: Requested::default(),
        }
    }

    /// Publishes a message with options of its own, and follows it to the broker.
    ///
    /// The call returns once the message is queued for the broker, as
    /// [`send`](Transport::send) does. The [`Delivery`] it hands back settles when the
    /// broker acknowledges the message, so a caller that needs to know the message arrived
    /// awaits it, and one that does not drops it.
    ///
    /// # Arguments
    ///
    /// * `topic` - the topic to publish to, with no wildcard.
    /// * `payload` - the message.
    /// * `options` - the quality of service and retain flag for this message.
    ///
    /// # Returns
    ///
    /// The delivery to await for the broker's acknowledgment.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if MQTT does not allow the topic or the message is over
    /// the connection's packet limit, both before anything is sent, or [`Error::Closed`] if
    /// the transport is not connected.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pamoja_core::Transport;
    /// use pamoja_mqtt::{MqttConfig, MqttTransport, PublishOptions};
    ///
    /// # async fn run() -> pamoja_core::Result<()> {
    /// let mut transport = MqttTransport::new(MqttConfig::new("sensor-1", "localhost", 1883));
    /// transport.connect().await?;
    ///
    /// // The last reading stays with the broker for any dashboard that subscribes later,
    /// // and the call waits until the broker says it has it.
    /// transport
    ///     .publish("sites/tank-2/level", b"73", PublishOptions::new().retained())
    ///     .await?
    ///     .confirmed()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn publish(
        &mut self,
        topic: &str,
        payload: &[u8],
        options: PublishOptions,
    ) -> Result<Delivery> {
        let client = self.live()?.clone();
        check_topic(topic)?;
        let qos = options.qos.unwrap_or(self.config.qos);
        let size = publish_size(topic, payload.len(), qos);
        if size > self.config.max_packet_size {
            return Err(Error::Transport(format!(
                "the message makes a {size}-byte packet, over this connection's {}-byte limit",
                self.config.max_packet_size
            )));
        }
        let delivery = self.requested.push(qos.into());
        if let Err(error) = client
            .publish(topic, qos.into(), options.retain, payload.to_vec())
            .await
        {
            self.requested.withdraw();
            return Err(map_client_error(error));
        }
        Ok(delivery)
    }

    /// Returns a handle on the messages arriving on this transport's connection.
    ///
    /// # Returns
    ///
    /// The inbox, which a task can wait on while another publishes through the transport.
    pub fn inbox(&self) -> Inbox {
        self.inbox.clone()
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
    /// The client says goodbye with a `DISCONNECT`, so the broker discards the will
    /// rather than publishing it (OASIS MQTT 3.1.1, section 3.14). Calling this on a
    /// transport that is not connected is a no-op. A transport that is dropped without
    /// disconnecting closes its connection without a goodbye, and the broker publishes
    /// its will.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the goodbye has gone out, or a second has passed without the event
    /// loop sending it, and the loop has been stopped.
    ///
    /// # Errors
    ///
    /// This call is best-effort and does not surface broker errors raised while
    /// tearing down, so it currently always returns `Ok(())`.
    pub async fn disconnect(&mut self) -> Result<()> {
        let pump = self.pump.take();
        if let Some(client) = self.client.take() {
            if client.disconnect().await.is_ok() {
                if let Some(pump) = &pump {
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
                    while !pump.is_finished() && tokio::time::Instant::now() < deadline {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            }
        }
        if let Some(pump) = pump {
            pump.abort();
        }
        self.inbox.replace(None).await;
        self.requested = Requested::default();
        Ok(())
    }
}

impl Drop for MqttTransport {
    fn drop(&mut self) {
        if let Some(pump) = self.pump.take() {
            pump.abort();
        }
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
        if let Some((username, password)) = &self.config.credentials {
            options.set_credentials(username.clone(), password.clone());
        }
        if let Some(will) = &self.config.will {
            check_topic(&will.topic)?;
            options.set_last_will(LastWill::new(
                will.topic.clone(),
                will.payload.clone(),
                will.qos.into(),
                will.retain,
            ));
        }
        #[cfg(feature = "tls")]
        if let Some(tls) = &self.config.tls {
            options.set_transport(rumqttc::Transport::tls_with_config(
                rumqttc::TlsConfiguration::Rustls(tls.client_config()?),
            ));
        }

        let (client, mut eventloop) = AsyncClient::new(options, self.config.capacity);
        let (tx, rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = oneshot::channel::<Result<()>>();
        let ended = Arc::new(AtomicBool::new(false));
        let marked = Arc::clone(&ended);
        let requested = Requested::default();
        let reported = requested.clone();

        let pump = tokio::spawn(async move {
            let mut ready_tx = Some(ready_tx);
            let mut in_flight = InFlight::default();
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
                    Ok(Event::Outgoing(Outgoing::Publish(pkid))) => {
                        in_flight.sent(&reported, pkid);
                    }
                    Ok(Event::Incoming(Packet::PubAck(ack))) => in_flight.acknowledged(ack.pkid),
                    Ok(Event::Incoming(Packet::PubComp(comp))) => {
                        in_flight.acknowledged(comp.pkid);
                    }
                    Ok(Event::Outgoing(Outgoing::Disconnect)) => {
                        marked.store(true, Ordering::SeqCst);
                        in_flight.fail(&reported, "the client disconnected");
                        break;
                    }
                    Ok(_) => {}
                    Err(err) => {
                        marked.store(true, Ordering::SeqCst);
                        in_flight.fail(&reported, &err.to_string());
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
                self.inbox.replace(Some(rx)).await;
                self.pump = Some(pump);
                self.ended = ended;
                self.requested = requested;
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
    /// `Ok(())` once the message is queued for the broker, which is before the broker
    /// acknowledges it; [`publish`](MqttTransport::publish) waits for that.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if MQTT does not allow the topic or the message
    /// is over the connection's packet limit, both before anything is sent, or
    /// [`Error::Closed`] if the transport is not connected.
    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        self.publish(topic, payload, PublishOptions::new())
            .await
            .map(drop)
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
        self.inbox.recv().await
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
///
/// A broker that turns the client away says why in its `CONNACK` return code (OASIS MQTT
/// 3.1.1, section 3.2.2.3), and the two about who the client is are authentication errors.
fn map_connection_error(err: ConnectionError) -> Error {
    match err {
        ConnectionError::ConnectionRefused(ConnectReturnCode::BadUserNamePassword) => {
            Error::Auth("the broker refused the username or password".into())
        }
        ConnectionError::ConnectionRefused(ConnectReturnCode::NotAuthorized) => {
            Error::Auth("the broker does not authorize this client".into())
        }
        ConnectionError::ConnectionRefused(ConnectReturnCode::BadClientId) => {
            Error::Transport("the broker refused the client id".into())
        }
        other => Error::Transport(other.to_string()),
    }
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

    #[test]
    fn a_refusal_about_who_the_client_is_is_an_authentication_error() {
        for (code, kind) in [
            (
                ConnectReturnCode::BadUserNamePassword,
                "username or password",
            ),
            (ConnectReturnCode::NotAuthorized, "does not authorize"),
        ] {
            match map_connection_error(ConnectionError::ConnectionRefused(code)) {
                Error::Auth(reason) => assert!(reason.contains(kind), "{reason}"),
                other => panic!("{code:?} is an authentication error, got {other:?}"),
            }
        }
        assert!(matches!(
            map_connection_error(ConnectionError::ConnectionRefused(
                ConnectReturnCode::BadClientId
            )),
            Error::Transport(_)
        ));
    }

    #[test]
    fn a_will_and_options_carry_what_they_were_given() {
        let will = Will::new("sites/pump-3/status", b"offline".to_vec())
            .qos(QualityOfService::ExactlyOnce)
            .retained();
        assert_eq!(will.topic(), "sites/pump-3/status");
        assert_eq!(will.payload(), b"offline");
        assert_eq!(will.quality_of_service(), QualityOfService::ExactlyOnce);
        assert!(will.is_retained());
        assert!(!Will::new("t", b"x".to_vec()).is_retained());
        assert!(PublishOptions::new().retained().is_retained());
        assert!(!PublishOptions::new().is_retained());
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
