//! The live Zenoh transport: a Zenoh session behind the core [`Transport`] trait.
//!
//! [`ZenohTransport`] opens a Zenoh session and exposes it through the protocol-agnostic
//! [`Transport`](pamoja_core::Transport) and [`Receive`](pamoja_core::Receive) surface, so Zenoh
//! serves as the efficient edge-to-edge and fleet transport alongside MQTT, CoAP, and the radios.
//! Like the other live transports it owns a background task per subscription that forwards samples
//! into a queue [`recv`](Receive::recv) drains. A sample arrives as a [`Message`] whose topic is the
//! key expression it was published to.

pub use pamoja_core::Message;
use pamoja_core::{Error, Receive, Result, Transport};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use zenoh::{Config, Session};

/// Connection settings for a [`ZenohTransport`].
///
/// The default is a Zenoh peer that discovers others by multicast scouting. The chained setters
/// pin explicit endpoints for a deterministic link, which is what fleets on routed networks use.
#[derive(Default)]
pub struct ZenohConfig {
    config: Config,
}

impl ZenohConfig {
    /// Creates a default configuration: a peer using multicast scouting.
    ///
    /// # Returns
    ///
    /// The configuration.
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    /// Adds a Zenoh endpoint to listen on, for example `tcp/0.0.0.0:7447`.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - the Zenoh locator to accept connections on.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn listen_on(mut self, endpoint: &str) -> Self {
        let _ = self
            .config
            .insert_json5("listen/endpoints", &format!("[\"{endpoint}\"]"));
        self
    }

    /// Adds a Zenoh endpoint to connect to, for example `tcp/192.168.1.10:7447`.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - the Zenoh locator of a peer or router to dial.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn connect_to(mut self, endpoint: &str) -> Self {
        let _ = self
            .config
            .insert_json5("connect/endpoints", &format!("[\"{endpoint}\"]"));
        self
    }

    /// Enables or disables multicast scouting (peer auto-discovery).
    ///
    /// # Arguments
    ///
    /// * `enabled` - whether to discover peers by multicast; turn off on networks that block it or
    ///   when using explicit endpoints.
    ///
    /// # Returns
    ///
    /// The updated configuration, for chaining.
    pub fn multicast_scouting(mut self, enabled: bool) -> Self {
        let value = if enabled { "true" } else { "false" };
        let _ = self
            .config
            .insert_json5("scouting/multicast/enabled", value);
        self
    }

    /// Consumes the wrapper and returns the underlying Zenoh configuration.
    ///
    /// # Returns
    ///
    /// The Zenoh [`Config`].
    pub fn into_zenoh(self) -> Config {
        self.config
    }
}

/// A Zenoh session that implements the core [`Transport`] and [`Receive`] traits.
///
/// Created disconnected; [`connect`](Transport::connect) opens the session. Each
/// [`subscribe`](Transport::subscribe) declares a Zenoh subscriber and spawns a task forwarding its
/// samples to an internal queue, and [`recv`](Receive::recv) awaits the next one.
pub struct ZenohTransport {
    config: Option<Config>,
    session: Option<Session>,
    incoming: Option<mpsc::UnboundedReceiver<Message>>,
    sender: Option<mpsc::UnboundedSender<Message>>,
    tasks: Vec<JoinHandle<()>>,
}

impl ZenohTransport {
    /// Creates a transport from the given configuration without connecting.
    ///
    /// # Arguments
    ///
    /// * `config` - the session settings.
    ///
    /// # Returns
    ///
    /// A disconnected transport ready for [`connect`](Transport::connect).
    pub fn new(config: ZenohConfig) -> Self {
        Self {
            config: Some(config.into_zenoh()),
            session: None,
            incoming: None,
            sender: None,
            tasks: Vec::new(),
        }
    }

    /// Reports whether the session is currently open.
    ///
    /// # Returns
    ///
    /// `true` once [`connect`](Transport::connect) has succeeded.
    pub fn is_connected(&self) -> bool {
        self.session.is_some()
    }

    /// Closes the session and stops the subscription tasks.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the session has been closed.
    ///
    /// # Errors
    ///
    /// Best-effort teardown that does not surface session-close errors, so it returns `Ok(())`.
    pub async fn disconnect(&mut self) -> Result<()> {
        for task in self.tasks.drain(..) {
            task.abort();
        }
        if let Some(session) = self.session.take() {
            let _ = session.close().await;
        }
        self.incoming = None;
        self.sender = None;
        Ok(())
    }
}

impl Transport for ZenohTransport {
    async fn connect(&mut self) -> Result<()> {
        let config = self.config.take().unwrap_or_default();
        let session = zenoh::open(config).await.map_err(map_err)?;
        let (sender, incoming) = mpsc::unbounded_channel();
        self.session = Some(session);
        self.sender = Some(sender);
        self.incoming = Some(incoming);
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        let session = self.session.as_ref().ok_or(Error::Closed)?;
        session.put(topic, payload.to_vec()).await.map_err(map_err)
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        let session = self.session.as_ref().ok_or(Error::Closed)?;
        let sender = self.sender.as_ref().ok_or(Error::Closed)?.clone();
        let subscriber = session
            .declare_subscriber(topic.to_string())
            .await
            .map_err(map_err)?;
        let task = tokio::spawn(async move {
            while let Ok(sample) = subscriber.recv_async().await {
                let message = Message::new(
                    sample.key_expr().as_str(),
                    sample.payload().to_bytes().to_vec(),
                );
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        self.tasks.push(task);
        Ok(())
    }
}

impl Receive for ZenohTransport {
    /// Awaits the next sample from any subscribed key expression.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next queued sample, with the key expression as its topic, or `None`
    /// once every subscription has ended.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`](pamoja_core::Error::Closed) if the transport is not connected.
    async fn recv(&mut self) -> Result<Option<Message>> {
        let incoming = self.incoming.as_mut().ok_or(Error::Closed)?;
        Ok(incoming.recv().await)
    }
}

fn map_err<E: core::fmt::Display>(err: E) -> Error {
    Error::Transport(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn send_before_connect_reports_closed() {
        let mut transport = ZenohTransport::new(ZenohConfig::new());
        assert!(matches!(
            transport.send("k", b"x").await,
            Err(Error::Closed)
        ));
        assert!(matches!(transport.recv().await, Err(Error::Closed)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn round_trips_over_a_tcp_endpoint() {
        let endpoint = "tcp/127.0.0.1:17447";
        let mut subscriber = ZenohTransport::new(
            ZenohConfig::new()
                .listen_on(endpoint)
                .multicast_scouting(false),
        );
        subscriber.connect().await.unwrap();
        subscriber.subscribe("pamoja/test/**").await.unwrap();

        let mut publisher = ZenohTransport::new(
            ZenohConfig::new()
                .connect_to(endpoint)
                .multicast_scouting(false),
        );
        publisher.connect().await.unwrap();

        // Publish until the subscriber sees it, so the test does not race link establishment.
        let received = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                publisher.send("pamoja/test/a", b"hello").await.unwrap();
                tokio::select! {
                    msg = subscriber.recv() => return msg.unwrap().unwrap(),
                    _ = tokio::time::sleep(Duration::from_millis(200)) => {}
                }
            }
        })
        .await
        .expect("a sample should arrive within the timeout");

        assert_eq!(received.payload, b"hello");
        assert_eq!(received.topic, "pamoja/test/a");

        subscriber.disconnect().await.unwrap();
        publisher.disconnect().await.unwrap();
    }
}
