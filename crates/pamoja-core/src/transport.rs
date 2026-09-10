//! The transport abstraction: how bytes move between the SDK and a device or peer.

use alloc::string::String;
use alloc::vec::Vec;
use core::future::Future;

use crate::error::Result;

/// A message that arrived on a subscribed topic.
///
/// Every link that can deliver hands back the same shape, so a node reads a
/// command the same way whether it came over MQTT, CoAP, Zenoh, or the in-process
/// broker used in tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// The topic the message was published to.
    pub topic: String,
    /// The raw payload bytes.
    pub payload: Vec<u8>,
}

impl Message {
    /// Creates a message from a topic and a payload.
    ///
    /// # Arguments
    ///
    /// * `topic` - the topic the message was published to.
    /// * `payload` - the raw payload bytes.
    ///
    /// # Returns
    ///
    /// The message.
    pub fn new(topic: impl Into<String>, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            topic: topic.into(),
            payload: payload.into(),
        }
    }

    /// Reads the payload as text.
    ///
    /// Most readings and commands on a topic are words or a number written out, and
    /// this is the accessor for them; a payload that is a codec's bytes goes through
    /// the codec instead.
    ///
    /// # Returns
    ///
    /// The payload as a string slice.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](crate::Error::Codec) if the payload is not UTF-8 text.
    pub fn text(&self) -> Result<&str> {
        core::str::from_utf8(&self.payload)
            .map_err(|_| crate::Error::Codec("the payload is not UTF-8 text".into()))
    }

    /// Reads the payload as a number written out as text, such as `21.5`.
    ///
    /// # Returns
    ///
    /// The number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](crate::Error::Codec) if the payload is not UTF-8 text
    /// or the text is not a number.
    pub fn number(&self) -> Result<f64> {
        self.text()?
            .trim()
            .parse()
            .map_err(|_| crate::Error::Codec("the payload is not a number".into()))
    }
}

/// A bidirectional, topic-addressed message transport.
///
/// Implementations include MQTT, CoAP, LoRa, serial, and CAN. They are expected
/// to handle reconnection and backpressure internally so that callers see a
/// uniform, protocol-agnostic surface.
///
/// This trait is the outbound half of a link: connect, publish, and register
/// interest in a topic. A link that can also deliver what it subscribed to
/// implements [`Receive`] as well. The two are separate because not every link
/// listens: a LoRa uplink or a satellite messenger only sends, and it still belongs
/// on a transport ladder.
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

    /// Publishes text to a topic: words, or a number written out.
    ///
    /// This is [`send`](Transport::send) with the text's UTF-8 bytes, so a reading or a
    /// command that is plain text needs no encoding step on either end; the receiver
    /// reads it back with [`Message::text`] or [`Message::number`].
    ///
    /// # Arguments
    ///
    /// * `topic` - the destination topic.
    /// * `text` - the text to publish.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the transport has accepted the message.
    ///
    /// # Errors
    ///
    /// Whatever [`send`](Transport::send) returns.
    fn send_text(&mut self, topic: &str, text: &str) -> impl Future<Output = Result<()>> + Send {
        self.send(topic, text.as_bytes())
    }

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

/// The inbound half of a link: the messages that arrive on subscribed topics.
///
/// A transport that can deliver implements this beside [`Transport`], and anything
/// that waits for a command, a reply, or another node's reading takes `Transport +
/// Receive`. A ladder listens on every rung that implements it and hands up
/// whichever delivers first.
///
/// # Cancel safety
///
/// Dropping the future [`recv`](Receive::recv) returns before it completes must
/// not lose a message: the message stays queued for the next call. A ladder and a
/// `select!` both depend on this, since they drop every future but the one that
/// completed. Reading from a channel whose receive is itself cancel-safe, which the
/// queue behind every transport here is, satisfies it.
///
/// # Examples
///
/// A subscriber loop that stops when the link ends:
///
/// ```
/// use pamoja_core::{Message, Receive, Result};
///
/// async fn drain(link: &mut impl Receive) -> Result<Vec<Message>> {
///     let mut seen = Vec::new();
///     while let Some(message) = link.recv().await? {
///         seen.push(message);
///     }
///     Ok(seen)
/// }
/// ```
pub trait Receive {
    /// Awaits the next message from any subscribed topic.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next message, or `None` once the link has ended and
    /// no further messages will arrive.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`](crate::Error::Closed) if the transport is not
    /// connected, or [`Error::Transport`](crate::Error::Transport) if the link
    /// fails while waiting.
    fn recv(&mut self) -> impl Future<Output = Result<Option<Message>>> + Send;
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_message_reads_as_text_or_a_number() {
        let reading = super::Message::new("garden/bed-1/moisture", "28.5");
        assert_eq!(reading.text().unwrap(), "28.5");
        assert_eq!(reading.number().unwrap(), 28.5);
        let spaced = super::Message::new("t", " 42 \n");
        assert_eq!(spaced.number().unwrap(), 42.0);
        let words = super::Message::new("garden/bed-1/valve", "open");
        assert_eq!(words.text().unwrap(), "open");
        assert!(matches!(words.number(), Err(crate::Error::Codec(_))));
        let raw = super::Message::new("t", vec![0xff, 0xfe]);
        assert!(matches!(raw.text(), Err(crate::Error::Codec(_))));
    }

    use super::*;

    #[test]
    fn a_message_is_built_from_any_topic_and_payload_shape() {
        let message = Message::new("sensors/1", b"21.5");
        assert_eq!(message.topic, "sensors/1");
        assert_eq!(message.payload, b"21.5");
        assert_eq!(
            message,
            Message::new(String::from("sensors/1"), vec![0x32, 0x31, 0x2E, 0x35])
        );
    }
}
