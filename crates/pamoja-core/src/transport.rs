//! The transport abstraction: how bytes move between the SDK and a device or peer.

use core::future::Future;

use crate::error::Result;

/// A bidirectional, topic-addressed message transport.
///
/// Implementations include MQTT, CoAP, LoRa, serial, and CAN. They are expected
/// to handle reconnection and backpressure internally so that callers see a
/// uniform, protocol-agnostic surface.
///
/// The returned futures are `Send`, so a transport can be driven from a task on
/// a multi-threaded runtime and can be erased behind a trait object that is.
/// Both matter in practice: a gateway ticks its links from spawned tasks, and a
/// transport ladder holds rungs of different concrete types in one list. An
/// implementation written as `async fn` satisfies this as long as everything it
/// holds across an await is `Send`, which every transport here already is.
pub trait Transport {
    /// Establishes the connection to the broker, peer, or bus.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the transport is connected and ready to carry traffic.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`](crate::Error::Transport) if the connection
    /// cannot be established.
    fn connect(&mut self) -> impl Future<Output = Result<()>> + Send;

    /// Publishes a payload to a topic.
    ///
    /// # Arguments
    ///
    /// * `topic` - the destination topic or channel address.
    /// * `payload` - the raw bytes to publish.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the payload has been handed to the transport for delivery.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`](crate::Error::Transport) if the payload cannot
    /// be sent, or [`Error::Closed`](crate::Error::Closed) if the transport is not
    /// connected.
    fn send(&mut self, topic: &str, payload: &[u8]) -> impl Future<Output = Result<()>> + Send;

    /// Subscribes to a topic so that matching payloads are routed to this transport.
    ///
    /// # Arguments
    ///
    /// * `topic` - the topic or channel filter to subscribe to.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the subscription is registered with the transport.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`](crate::Error::Transport) if the subscription
    /// is rejected, or [`Error::Closed`](crate::Error::Closed) if the transport is
    /// not connected.
    fn subscribe(&mut self, topic: &str) -> impl Future<Output = Result<()>> + Send;
}
