//! The loopback transport itself.

use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::{self, UnboundedReceiver};

use pamoja_core::{Error, Message, Receive, Result, Transport};

use crate::broker::LoopbackBroker;

/// An in-process transport that routes through a shared [`LoopbackBroker`].
///
/// A transport is created disconnected; [`connect`](Transport::connect) registers
/// it with the broker so it can publish and receive. Inbound messages are read
/// with [`recv`](Receive::recv).
///
/// # Examples
///
/// ```
/// use pamoja_core::{Receive, Transport};
/// use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
///
/// # async fn run() -> pamoja_core::Result<()> {
/// let broker = LoopbackBroker::new();
/// let mut subscriber = LoopbackTransport::new(broker.clone());
/// let mut publisher = LoopbackTransport::new(broker);
/// subscriber.connect().await?;
/// publisher.connect().await?;
///
/// subscriber.subscribe("sensors/+/temperature").await?;
/// publisher.send("sensors/1/temperature", b"21.5").await?;
///
/// let message = subscriber.recv().await?.expect("a message");
/// assert_eq!(message.topic, "sensors/1/temperature");
/// assert_eq!(message.payload, b"21.5");
/// # Ok(())
/// # }
/// ```
pub struct LoopbackTransport {
    broker: LoopbackBroker,
    filters: Arc<Mutex<Vec<String>>>,
    incoming: Option<UnboundedReceiver<Message>>,
}

impl LoopbackTransport {
    /// Creates a disconnected transport bound to `broker`.
    ///
    /// # Arguments
    ///
    /// * `broker` - the shared broker this transport publishes to and receives from.
    ///
    /// # Returns
    ///
    /// A disconnected transport ready for [`connect`](Transport::connect).
    pub fn new(broker: LoopbackBroker) -> Self {
        Self {
            broker,
            filters: Arc::new(Mutex::new(Vec::new())),
            incoming: None,
        }
    }

    /// Reports whether the transport is connected to its broker.
    ///
    /// # Returns
    ///
    /// `true` once [`connect`](Transport::connect) has succeeded and before
    /// [`disconnect`](LoopbackTransport::disconnect) is called.
    pub fn is_connected(&self) -> bool {
        self.incoming.is_some()
    }

    /// Disconnects the transport from the broker.
    ///
    /// Its registration is pruned from the broker on the next publish.
    pub fn disconnect(&mut self) {
        self.incoming = None;
    }
}

impl Transport for LoopbackTransport {
    async fn connect(&mut self) -> Result<()> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.broker.register(Arc::clone(&self.filters), sender);
        self.incoming = Some(receiver);
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if self.incoming.is_none() {
            return Err(Error::Closed);
        }
        self.broker.publish(&Message::new(topic, payload));
        Ok(())
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        if self.incoming.is_none() {
            return Err(Error::Closed);
        }
        self.filters
            .lock()
            .expect("filters lock")
            .push(topic.to_owned());
        Ok(())
    }
}

impl Receive for LoopbackTransport {
    /// Awaits the next message from any subscribed topic.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next message, or `None` once the broker and all
    /// other transports have been dropped.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`](pamoja_core::Error::Closed) if the transport is
    /// not connected.
    async fn recv(&mut self) -> Result<Option<Message>> {
        let incoming = self.incoming.as_mut().ok_or(Error::Closed)?;
        Ok(incoming.recv().await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_and_subscribe_round_trip() {
        let broker = LoopbackBroker::new();
        let mut subscriber = LoopbackTransport::new(broker.clone());
        let mut publisher = LoopbackTransport::new(broker);
        subscriber.connect().await.expect("connect");
        publisher.connect().await.expect("connect");

        subscriber
            .subscribe("sensors/+/temperature")
            .await
            .expect("subscribe");
        publisher
            .send("sensors/1/temperature", b"21.5")
            .await
            .expect("send");

        let message = subscriber.recv().await.expect("recv").expect("a message");
        assert_eq!(message.topic, "sensors/1/temperature");
        assert_eq!(message.payload, b"21.5");
    }

    #[tokio::test]
    async fn non_matching_topics_are_not_delivered() {
        let broker = LoopbackBroker::new();
        let mut subscriber = LoopbackTransport::new(broker.clone());
        let mut publisher = LoopbackTransport::new(broker);
        subscriber.connect().await.expect("connect");
        publisher.connect().await.expect("connect");

        subscriber
            .subscribe("sensors/1/#")
            .await
            .expect("subscribe");
        publisher
            .send("sensors/2/temperature", b"x")
            .await
            .expect("send");
        publisher
            .send("sensors/1/humidity", b"y")
            .await
            .expect("send");

        let message = subscriber.recv().await.expect("recv").expect("a message");
        assert_eq!(message.topic, "sensors/1/humidity");
    }

    #[tokio::test]
    async fn operations_before_connect_report_closed() {
        let broker = LoopbackBroker::new();
        let mut transport = LoopbackTransport::new(broker);
        assert!(matches!(
            transport.send("t", b"x").await,
            Err(Error::Closed)
        ));
        assert!(matches!(transport.subscribe("t").await, Err(Error::Closed)));
        assert!(matches!(transport.recv().await, Err(Error::Closed)));
        assert!(!transport.is_connected());
    }
}
