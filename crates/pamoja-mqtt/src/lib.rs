//! MQTT transport for the pamoja SDK.
//!
//! [`MqttTransport`] implements the core [`Transport`]
//! trait on top of the pure-Rust [`rumqttc`] client, so an application can publish
//! to and subscribe from an MQTT broker through the same protocol-agnostic surface
//! it uses for every other transport.
//!
//! Once [`connect`](Transport::connect) succeeds the transport owns a background
//! task that drives the MQTT event loop: it answers keep-alive pings, completes
//! delivery handshakes, and forwards inbound messages to an internal queue that
//! [`recv`](MqttTransport::recv) drains. Publishing and subscribing use the
//! default [`QualityOfService`] configured on the transport.
//!
//! # Examples
//!
//! ```no_run
//! use pamoja_core::Transport;
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

use std::time::Duration;

use pamoja_core::{Error, Result, Transport};
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
    /// A configuration with a 30-second keep-alive, a request capacity of 64, and
    /// a default quality of service of [`QualityOfService::AtLeastOnce`].
    pub fn new(client_id: impl Into<String>, host: impl Into<String>, port: u16) -> Self {
        Self {
            client_id: client_id.into(),
            host: host.into(),
            port,
            keep_alive: Duration::from_secs(30),
            capacity: 64,
            qos: QualityOfService::AtLeastOnce,
        }
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

/// A message received from a subscribed topic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// The topic the message was published to.
    pub topic: String,
    /// The raw payload bytes.
    pub payload: Vec<u8>,
}

/// An MQTT client that implements the core [`Transport`] trait.
///
/// A transport is created disconnected; [`connect`](Transport::connect) opens the
/// link and spawns the background task that runs the MQTT event loop for the life
/// of the connection. Inbound messages are queued and read with
/// [`recv`](MqttTransport::recv).
pub struct MqttTransport {
    config: MqttConfig,
    client: Option<AsyncClient>,
    incoming: Option<mpsc::UnboundedReceiver<Message>>,
    pump: Option<JoinHandle<()>>,
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
        }
    }

    /// Reports whether the transport currently holds an active connection.
    ///
    /// # Returns
    ///
    /// `true` once [`connect`](Transport::connect) has succeeded and before
    /// [`disconnect`](MqttTransport::disconnect) is called.
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Awaits the next message from any subscribed topic.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next queued message, or `None` once the event loop
    /// has stopped and no further messages will arrive.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`] if the transport
    /// is not connected.
    pub async fn recv(&mut self) -> Result<Option<Message>> {
        let incoming = self.incoming.as_mut().ok_or(Error::Closed)?;
        Ok(incoming.recv().await)
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
    async fn connect(&mut self) -> Result<()> {
        let mut options = MqttOptions::new(
            self.config.client_id.clone(),
            self.config.host.clone(),
            self.config.port,
        );
        options.set_keep_alive(self.config.keep_alive);

        let (client, mut eventloop) = AsyncClient::new(options, self.config.capacity);
        let (tx, rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = oneshot::channel::<Result<()>>();

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
                        let message = Message {
                            topic: publish.topic,
                            payload: publish.payload.to_vec(),
                        };
                        if tx.send(message).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(err) => {
                        if let Some(ready_tx) = ready_tx.take() {
                            let _ = ready_tx.send(Err(map_connection_error(err)));
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

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        let client = self.client.as_ref().ok_or(Error::Closed)?;
        client
            .publish(topic, self.config.qos.into(), false, payload.to_vec())
            .await
            .map_err(map_client_error)
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        let client = self.client.as_ref().ok_or(Error::Closed)?;
        client
            .subscribe(topic, self.config.qos.into())
            .await
            .map_err(map_client_error)
    }
}

/// Maps a `rumqttc` client error onto the shared transport error.
fn map_client_error(err: ClientError) -> Error {
    Error::Transport(err.to_string())
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
