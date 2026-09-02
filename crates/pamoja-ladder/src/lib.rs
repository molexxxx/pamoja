//! Cost-aware transport ladder for the pamoja SDK.
//!
//! A field node usually has more than one way to reach the wider network, and
//! those links differ wildly in cost, range, and availability: a local mesh hop is
//! nearly free, long-range radio is cheap but slow, cellular is metered, and
//! satellite is expensive. [`TransportLadder`] models that hierarchy. It holds a
//! set of [`Transport`](pamoja_core::Transport) rungs ordered cheapest-first and,
//! on each send, uses the first rung that accepts the message. When no rung is
//! reachable, the message is buffered in a durable [`Store`](pamoja_core::Store)
//! and replayed later, so connectivity degrades gracefully instead of failing.
//!
//! This is the offline-first behavior the target deployments need on day one: an
//! irrigation node or a fridge alarm keeps recording while every link is down and
//! loses nothing once one returns.
//!
//! # Ordering and the buffer
//!
//! Delivery is in order. Once anything is buffered, later sends are buffered too
//! rather than jumping ahead of the backlog over a recovered link;
//! [`flush`](TransportLadder::flush) drains the backlog oldest-first, removing each
//! record only after a rung accepts it. The pattern is to call
//! [`flush`](TransportLadder::flush) when a link event suggests connectivity may
//! have returned, and [`send`](TransportLadder::send) for new data.
//!
//! # Examples
//!
//! ```
//! use pamoja_ladder::{Delivery, TransportLadder};
//! use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
//! use pamoja_sync::MemoryStore;
//!
//! # async fn run() -> pamoja_core::Result<()> {
//! let broker = LoopbackBroker::new();
//! let mut ladder =
//!     TransportLadder::new(MemoryStore::new()).rung(LoopbackTransport::new(broker.clone()));
//! ladder.connect().await?;
//!
//! match ladder.send("sensors/1/temperature", b"21.5").await? {
//!     Delivery::Sent => println!("delivered over a live link"),
//!     Delivery::Buffered => println!("no link, buffered for later"),
//! }
//! # Ok(())
//! # }
//! ```

use core::future::Future;
use core::pin::Pin;

use pamoja_core::{Error, Result, Store, Transport};

/// The outcome of a [`TransportLadder::send`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    /// The message was delivered immediately over one of the ladder's rungs.
    Sent,
    /// No rung accepted the message, so it was buffered for a later
    /// [`flush`](TransportLadder::flush).
    Buffered,
}

/// Object-safe erasure of [`Transport`] so a ladder can hold heterogeneous rungs.
///
/// The core [`Transport`] trait uses `async fn`, which is not dyn-compatible; this
/// wrapper boxes the returned futures so transports of different concrete types can
/// live together in one ordered list.
///
/// The boxed futures are `Send` so a ladder can be driven from a multi-threaded
/// runtime, which is where one usually lives: a gateway ticks it from a task
/// rather than blocking a thread on it.
trait DynTransport: Send {
    /// Connects the underlying transport.
    fn connect(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;

    /// Sends a payload to a topic over the underlying transport.
    fn send<'a>(
        &'a mut self,
        topic: &'a str,
        payload: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}

/// Newtype that carries one concrete transport behind the object-safe
/// [`DynTransport`]. Erasing through a dedicated wrapper, rather than a blanket
/// impl over every `T: Transport`, keeps these boxed-future methods off the
/// transports themselves so their own `connect`/`send` stay unambiguous.
struct Erased<T>(T);

impl<T: Transport + Send> DynTransport for Erased<T> {
    fn connect(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(Transport::connect(&mut self.0))
    }

    fn send<'a>(
        &'a mut self,
        topic: &'a str,
        payload: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(Transport::send(&mut self.0, topic, payload))
    }
}

/// An ordered set of transports backed by an offline buffer.
///
/// Rungs are tried in the order they are added, so the cheapest, most-preferred
/// link is added first. A send that no rung accepts is buffered in the
/// [`Store`](pamoja_core::Store) and replayed by [`flush`](Self::flush).
pub struct TransportLadder<S> {
    rungs: Vec<Box<dyn DynTransport>>,
    buffer: S,
}

impl<S: Store> TransportLadder<S> {
    /// Creates an empty ladder that buffers into `buffer`.
    ///
    /// # Arguments
    ///
    /// * `buffer` - the durable queue that holds messages while no rung is
    ///   reachable.
    ///
    /// # Returns
    ///
    /// A ladder with no rungs; add them with [`rung`](Self::rung).
    pub fn new(buffer: S) -> Self {
        Self {
            rungs: Vec::new(),
            buffer,
        }
    }

    /// Adds a rung, lowest-cost first.
    ///
    /// # Arguments
    ///
    /// * `transport` - a transport to try. Rungs added earlier are preferred, so
    ///   add the cheapest link first and the costliest fallback last.
    ///
    /// # Returns
    ///
    /// The ladder, for chaining.
    pub fn rung(mut self, transport: impl Transport + Send + 'static) -> Self {
        self.rungs.push(Box::new(Erased(transport)));
        self
    }

    /// Connects every rung, best-effort.
    ///
    /// A rung that fails to connect is left unreachable rather than failing the
    /// whole ladder; sends simply fall through to the next rung or the buffer.
    ///
    /// # Returns
    ///
    /// `Ok(())` once every rung has been given the chance to connect.
    ///
    /// # Errors
    ///
    /// This call is best-effort and currently always returns `Ok(())`.
    pub async fn connect(&mut self) -> Result<()> {
        for rung in self.rungs.iter_mut() {
            let _ = rung.connect().await;
        }
        Ok(())
    }

    /// Sends a payload, falling back down the rungs and then to the buffer.
    ///
    /// If the buffer is empty, each rung is tried in order and the first to accept
    /// the message delivers it. If every rung fails, or the buffer already holds a
    /// backlog, the message is buffered to preserve order.
    ///
    /// # Arguments
    ///
    /// * `topic` - the destination topic.
    /// * `payload` - the bytes to send.
    ///
    /// # Returns
    ///
    /// [`Delivery::Sent`] if a rung delivered the message, or [`Delivery::Buffered`]
    /// if it was queued for a later [`flush`](Self::flush).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](pamoja_core::Error::Io) if the message must be buffered
    /// but the store cannot be written.
    pub async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<Delivery> {
        if self.buffer.is_empty().await? && Self::deliver(&mut self.rungs, topic, payload).await {
            return Ok(Delivery::Sent);
        }
        self.buffer.append(&frame(topic, payload)).await?;
        Ok(Delivery::Buffered)
    }

    /// Drains the buffer across the rungs, oldest record first.
    ///
    /// Each record is sent before it is removed, so the first record no rung can
    /// deliver halts the drain and leaves it, and everything after it, buffered in
    /// order for a later retry.
    ///
    /// # Returns
    ///
    /// The number of records forwarded before the buffer emptied or a rung refused
    /// one.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](pamoja_core::Error::Io) if the store cannot be read or
    /// written, or [`Error::Codec`](pamoja_core::Error::Codec) if a buffered record
    /// cannot be decoded.
    pub async fn flush(&mut self) -> Result<usize> {
        let mut forwarded = 0;
        while let Some(record) = self.buffer.peek().await? {
            let (topic, payload) = unframe(&record)?;
            if !Self::deliver(&mut self.rungs, &topic, &payload).await {
                break;
            }
            self.buffer.pop().await?;
            forwarded += 1;
        }
        Ok(forwarded)
    }

    /// Returns how many messages are currently buffered.
    ///
    /// Takes the ladder mutably, like the rest of its surface. Reading through a
    /// shared borrow would hold one across the await, which would in turn oblige
    /// every rung to be `Sync` rather than only `Send`, and that is a heavier
    /// requirement than a transport should have to meet.
    ///
    /// # Returns
    ///
    /// The number of records waiting for a [`flush`](Self::flush).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](pamoja_core::Error::Io) if the store length cannot be
    /// read.
    pub async fn buffered(&mut self) -> Result<usize> {
        self.buffer.len().await
    }

    /// Tries each rung in order, returning whether any accepted the message.
    async fn deliver(rungs: &mut [Box<dyn DynTransport>], topic: &str, payload: &[u8]) -> bool {
        for rung in rungs.iter_mut() {
            if rung.send(topic, payload).await.is_ok() {
                return true;
            }
        }
        false
    }
}

/// Frames a topic and payload into one record for the buffer.
///
/// The layout is a four-byte big-endian topic length, the topic bytes, then the
/// payload, so [`unframe`] can split them back apart.
fn frame(topic: &str, payload: &[u8]) -> Vec<u8> {
    let mut record = Vec::with_capacity(4 + topic.len() + payload.len());
    record.extend_from_slice(&(topic.len() as u32).to_be_bytes());
    record.extend_from_slice(topic.as_bytes());
    record.extend_from_slice(payload);
    record
}

/// Splits a buffered record back into its topic and payload.
fn unframe(record: &[u8]) -> Result<(String, Vec<u8>)> {
    let header: [u8; 4] = record
        .get(..4)
        .ok_or_else(|| Error::Codec("ladder record is missing its length header".to_owned()))?
        .try_into()
        .expect("a four-byte slice");
    let topic_len = u32::from_be_bytes(header) as usize;
    let topic_bytes = record
        .get(4..4 + topic_len)
        .ok_or_else(|| Error::Codec("ladder record topic is truncated".to_owned()))?;
    let topic =
        String::from_utf8(topic_bytes.to_vec()).map_err(|err| Error::Codec(err.to_string()))?;
    let payload = record[4 + topic_len..].to_vec();
    Ok((topic, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    use pamoja_loopback::{Faulty, LoopbackBroker, LoopbackTransport};
    use pamoja_sync::MemoryStore;

    /// Accepts anything that can move between threads.
    fn assert_send<T: Send>(_value: T) {}

    #[test]
    fn a_ladder_can_be_driven_from_a_spawned_task() {
        // A gateway ticks its ladder from a task on a threaded runtime, so the
        // futures have to be Send. This does not run them; it fails to compile
        // if a rung ever stops promising it.
        let mut ladder = TransportLadder::new(MemoryStore::new())
            .rung(LoopbackTransport::new(LoopbackBroker::new()));
        assert_send(ladder.connect());
        assert_send(ladder.send("sensors/1", b"21.5"));
        assert_send(ladder.flush());
        assert_send(ladder.buffered());
    }

    /// Subscribes a gateway to everything on a broker so the test can observe it.
    async fn gateway(broker: &LoopbackBroker) -> LoopbackTransport {
        let mut gateway = LoopbackTransport::new(broker.clone());
        gateway.connect().await.expect("connect gateway");
        gateway.subscribe("#").await.expect("subscribe gateway");
        gateway
    }

    #[test]
    fn frame_round_trips_topic_and_payload() {
        let record = frame("sensors/1/temperature", b"21.5");
        let (topic, payload) = unframe(&record).expect("unframe");
        assert_eq!(topic, "sensors/1/temperature");
        assert_eq!(payload, b"21.5");
    }

    #[test]
    fn unframe_rejects_a_truncated_record() {
        assert!(matches!(unframe(&[0, 0]), Err(Error::Codec(_))));
        // Claims a four-byte topic but carries only one.
        assert!(matches!(unframe(&[0, 0, 0, 4, b'a']), Err(Error::Codec(_))));
    }

    #[tokio::test]
    async fn send_delivers_over_the_first_working_rung() {
        let broker = LoopbackBroker::new();
        let mut observer = gateway(&broker).await;

        let mut ladder =
            TransportLadder::new(MemoryStore::new()).rung(LoopbackTransport::new(broker.clone()));
        ladder.connect().await.expect("connect");

        let delivery = ladder
            .send("sensors/1/temperature", b"21.5")
            .await
            .expect("send");
        assert_eq!(delivery, Delivery::Sent);
        assert_eq!(ladder.buffered().await.expect("buffered"), 0);

        let message = observer.recv().await.expect("recv").expect("a message");
        assert_eq!(message.topic, "sensors/1/temperature");
        assert_eq!(message.payload, b"21.5");
    }

    #[tokio::test]
    async fn send_falls_over_to_a_cheaper_rung_that_is_down() {
        // The preferred rung publishes to its own broker but is broken; the
        // fallback rung publishes to a second broker and works.
        let preferred_broker = LoopbackBroker::new();
        let fallback_broker = LoopbackBroker::new();
        let mut preferred_observer = gateway(&preferred_broker).await;
        let mut fallback_observer = gateway(&fallback_broker).await;

        let preferred = Faulty::new(LoopbackTransport::new(preferred_broker.clone()), 1);
        let fallback = LoopbackTransport::new(fallback_broker.clone());
        let mut ladder = TransportLadder::new(MemoryStore::new())
            .rung(preferred)
            .rung(fallback);
        ladder.connect().await.expect("connect");

        let delivery = ladder.send("t", b"x").await.expect("send");
        assert_eq!(delivery, Delivery::Sent);

        let message = fallback_observer
            .recv()
            .await
            .expect("recv")
            .expect("a message");
        assert_eq!(message.payload, b"x");
        // The broken rung delivered nothing: its observer never sees a message.
        let starved =
            tokio::time::timeout(Duration::from_millis(50), preferred_observer.recv()).await;
        assert!(
            starved.is_err(),
            "the broken rung must not deliver anything"
        );
    }

    #[tokio::test]
    async fn buffers_when_every_rung_is_down_then_flushes_in_order() {
        let broker = LoopbackBroker::new();
        let mut observer = gateway(&broker).await;

        // One simulated outage on the only rung, then it recovers.
        let rung = Faulty::new(LoopbackTransport::new(broker.clone()), 1);
        let mut ladder = TransportLadder::new(MemoryStore::new()).rung(rung);
        ladder.connect().await.expect("connect");

        // First send hits the outage and buffers; the next two preserve order by
        // buffering behind it rather than racing ahead.
        assert_eq!(
            ladder.send("out", b"a").await.expect("send"),
            Delivery::Buffered
        );
        assert_eq!(
            ladder.send("out", b"b").await.expect("send"),
            Delivery::Buffered
        );
        assert_eq!(
            ladder.send("out", b"c").await.expect("send"),
            Delivery::Buffered
        );
        assert_eq!(ladder.buffered().await.expect("buffered"), 3);

        // The link is back: drain everything in the order it was accepted.
        let forwarded = ladder.flush().await.expect("flush");
        assert_eq!(forwarded, 3);
        assert_eq!(ladder.buffered().await.expect("buffered"), 0);

        for expected in [b"a", b"b", b"c"] {
            let message = observer.recv().await.expect("recv").expect("a message");
            assert_eq!(message.topic, "out");
            assert_eq!(message.payload, expected);
        }
    }

    #[tokio::test]
    async fn flush_stops_at_the_first_record_no_rung_accepts() {
        let broker = LoopbackBroker::new();

        // Three outages: the first buffers, the next two buffer behind it, and the
        // flush attempt spends the remaining outages without draining anything.
        let rung = Faulty::new(LoopbackTransport::new(broker.clone()), 3);
        let mut ladder = TransportLadder::new(MemoryStore::new()).rung(rung);
        ladder.connect().await.expect("connect");

        ladder.send("out", b"a").await.expect("send");
        ladder.send("out", b"b").await.expect("send");
        ladder.send("out", b"c").await.expect("send");

        // The link is still down on the first drained record, so nothing forwards
        // and the backlog stays intact and ordered.
        assert_eq!(ladder.flush().await.expect("flush"), 0);
        assert_eq!(ladder.buffered().await.expect("buffered"), 3);
    }
}
