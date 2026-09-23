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
    /// Messages already queued for it are dropped, and its registration is pruned
    /// from the broker on the next publish. Its filters are kept, so a later
    /// [`connect`](Transport::connect) hears the same topics again.
    pub fn disconnect(&mut self) {
        self.incoming = None;
    }
}

impl Transport for LoopbackTransport {
    async fn connect(&mut self) -> Result<()> {
        self.broker.in_reach()?;
        let (sender, receiver) = mpsc::unbounded_channel();
        self.broker.register(Arc::clone(&self.filters), sender);
        self.incoming = Some(receiver);
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if self.incoming.is_none() {
            return Err(Error::Closed);
        }
        self.broker.in_reach()?;
        self.broker.publish(&Message::new(topic, payload));
        Ok(())
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        if self.incoming.is_none() {
            return Err(Error::Closed);
        }
        self.broker.in_reach()?;
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
    /// A link holds its broker, so a connected link never ends on its own: this
    /// waits until a message arrives, however long that takes. Dropping the future
    /// before then leaves the next message queued.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next message.
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
    async fn a_connected_link_waits_for_a_message_rather_than_ending() {
        use std::future::Future;
        use std::task::{Context, Waker};

        let mut link = LoopbackTransport::new(LoopbackBroker::new());
        link.connect().await.expect("connect");
        link.subscribe("t").await.expect("subscribe");
        {
            let mut waiting = std::pin::pin!(link.recv());
            let mut context = Context::from_waker(Waker::noop());
            assert!(waiting.as_mut().poll(&mut context).is_pending());
        }

        link.send("t", b"1").await.expect("send");
        let message = link.recv().await.expect("recv").expect("a message");
        assert_eq!(message.payload, b"1", "a link hears its own publishes");
    }

    #[tokio::test]
    async fn a_message_matching_two_filters_arrives_once() {
        let broker = LoopbackBroker::new();
        let mut subscriber = LoopbackTransport::new(broker.clone());
        let mut publisher = LoopbackTransport::new(broker);
        subscriber.connect().await.expect("connect");
        publisher.connect().await.expect("connect");
        subscriber.subscribe("a/+").await.expect("subscribe");
        subscriber.subscribe("a/#").await.expect("subscribe");

        publisher.send("a/b", b"1").await.expect("send");
        publisher.send("a/c", b"2").await.expect("send");
        let first = subscriber.recv().await.expect("recv").expect("a message");
        let second = subscriber.recv().await.expect("recv").expect("a message");
        assert_eq!(first.topic, "a/b");
        assert_eq!(second.topic, "a/c");
    }

    #[tokio::test]
    async fn a_subscription_hears_only_what_is_published_after_it() {
        let broker = LoopbackBroker::new();
        let mut subscriber = LoopbackTransport::new(broker.clone());
        let mut publisher = LoopbackTransport::new(broker);
        subscriber.connect().await.expect("connect");
        publisher.connect().await.expect("connect");

        publisher.send("t", b"before").await.expect("send");
        subscriber.subscribe("t").await.expect("subscribe");
        publisher.send("t", b"after").await.expect("send");
        let message = subscriber.recv().await.expect("recv").expect("a message");
        assert_eq!(message.payload, b"after");
    }

    #[tokio::test]
    async fn a_reconnected_link_keeps_its_filters_and_drops_what_was_queued() {
        let broker = LoopbackBroker::new();
        let mut subscriber = LoopbackTransport::new(broker.clone());
        let mut publisher = LoopbackTransport::new(broker);
        subscriber.connect().await.expect("connect");
        publisher.connect().await.expect("connect");
        subscriber.subscribe("t").await.expect("subscribe");

        publisher.send("t", b"queued").await.expect("send");
        subscriber.disconnect();
        publisher.send("t", b"missed").await.expect("send");
        subscriber.connect().await.expect("reconnect");
        publisher.send("t", b"heard").await.expect("send");
        let message = subscriber.recv().await.expect("recv").expect("a message");
        assert_eq!(message.payload, b"heard");
    }

    #[tokio::test]
    async fn links_on_separate_brokers_never_hear_each_other() {
        let mut subscriber = LoopbackTransport::new(LoopbackBroker::new());
        let mut publisher = LoopbackTransport::new(LoopbackBroker::new());
        subscriber.connect().await.expect("connect");
        publisher.connect().await.expect("connect");
        subscriber.subscribe("t").await.expect("subscribe");
        publisher.send("t", b"elsewhere").await.expect("send");
        subscriber.send("t", b"here").await.expect("send");
        let message = subscriber.recv().await.expect("recv").expect("a message");
        assert_eq!(message.payload, b"here");
    }

    #[tokio::test]
    async fn an_unreachable_broker_refuses_its_links_until_it_is_back() {
        let broker = LoopbackBroker::new();
        let mut listener = LoopbackTransport::new(broker.clone());
        let mut node = LoopbackTransport::new(broker.clone());
        listener.connect().await.expect("connect");
        node.connect().await.expect("connect");
        listener.subscribe("t").await.expect("subscribe");

        broker.set_reachable(false);
        assert!(!broker.is_reachable());
        for refused in [
            node.send("t", b"lost").await,
            node.subscribe("u").await,
            LoopbackTransport::new(broker.clone()).connect().await,
        ] {
            match refused {
                Err(Error::Transport(reason)) => assert_eq!(reason, "the broker is out of reach"),
                other => panic!("an unreachable broker allowed {other:?}"),
            }
        }
        assert!(node.is_connected(), "an outage keeps the connection");

        broker.set_reachable(true);
        node.send("t", b"back").await.expect("send");
        let message = listener.recv().await.expect("recv").expect("a message");
        assert_eq!(
            message.payload, b"back",
            "nothing sent during the outage arrives"
        );
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
