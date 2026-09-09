//! Cost-aware transport ladder for the pamoja SDK.
//!
//! A field node usually has more than one way to reach the wider network, and
//! those links differ wildly in cost, range, and availability: a local mesh hop is
//! nearly free, long-range radio is cheap but slow, cellular is metered, and
//! satellite is expensive. [`TransportLadder`] models that hierarchy. It holds a
//! set of [`Transport`] rungs ordered cheapest-first and,
//! on each send, uses the first rung that accepts the message. When no rung is
//! reachable, the message is buffered in a durable [`Store`]
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
//! # The ladder as a link
//!
//! A ladder is itself a [`Transport`] and a [`Receive`], so anything written against
//! one link, a profile node, a rule engine, a hand-written loop, runs over the
//! ladder unchanged and gains its buffering. A subscription is placed on every rung
//! that listens, and a receive hands up whichever rung delivers first, so a command
//! reaches the node over whatever link happens to be up. A rung added with
//! [`rung`](TransportLadder::rung) both sends and listens; a send-only link such as
//! a LoRa uplink is added with [`uplink`](TransportLadder::uplink) and is never
//! listened on.
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
use core::task::{Context, Poll};

use pamoja_core::{Error, Message, Receive, Result, Store, Transport};

/// The outcome of a [`TransportLadder::send`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    /// The message was delivered immediately over one of the ladder's rungs.
    Sent,
    /// No rung accepted the message, so it was buffered for a later
    /// [`flush`](TransportLadder::flush).
    Buffered,
}

/// The boxed future a rung's receive returns.
type Inbound<'a> = Pin<Box<dyn Future<Output = Result<Option<Message>>> + Send + 'a>>;

/// Object-safe erasure of a rung so a ladder can hold heterogeneous rungs.
///
/// The core traits use `async fn`, which is not dyn-compatible; this wrapper boxes
/// the returned futures so transports of different concrete types can live
/// together in one ordered list.
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

    /// Subscribes the underlying transport to a topic.
    fn subscribe<'a>(
        &'a mut self,
        topic: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// Awaits the next message the underlying transport delivers.
    fn recv(&mut self) -> Inbound<'_>;
}

/// Newtype that carries one two-way transport behind the object-safe
/// [`DynTransport`]. Erasing through a dedicated wrapper, rather than a blanket
/// impl over every `T: Transport`, keeps these boxed-future methods off the
/// transports themselves so their own `connect`/`send` stay unambiguous.
struct Erased<T>(T);

impl<T: Transport + Receive + Send> DynTransport for Erased<T> {
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

    fn subscribe<'a>(
        &'a mut self,
        topic: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(Transport::subscribe(&mut self.0, topic))
    }

    fn recv(&mut self) -> Inbound<'_> {
        Box::pin(Receive::recv(&mut self.0))
    }
}

/// Newtype that carries a send-only transport behind [`DynTransport`]. The ladder
/// never subscribes or listens on one, so its receive reports an ended link.
struct Uplink<T>(T);

impl<T: Transport + Send> DynTransport for Uplink<T> {
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

    fn subscribe<'a>(
        &'a mut self,
        topic: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(Transport::subscribe(&mut self.0, topic))
    }

    fn recv(&mut self) -> Inbound<'_> {
        Box::pin(async { Ok(None) })
    }
}

/// One link in the ladder and what the ladder knows about its state.
struct Rung {
    transport: Box<dyn DynTransport>,
    /// Whether the link delivers, so the ladder subscribes and listens on it.
    listens: bool,
    /// Whether the link is connected as far as the ladder has observed: set by a
    /// successful connect, cleared when the link reports itself closed or ended.
    connected: bool,
    /// The filters live on this link since it last connected.
    subscribed: Vec<String>,
}

impl Rung {
    /// Forgets the connection and every subscription placed on it, so the next
    /// [`connect`](TransportLadder::connect) starts the link over.
    fn drop_link(&mut self) {
        self.connected = false;
        self.subscribed.clear();
    }

    /// Places `filter` on the link if it is not there already.
    ///
    /// A link answering [`Error::Closed`] is forgotten, so the filter is placed
    /// again when it reconnects.
    async fn place(&mut self, filter: &str) -> Result<()> {
        if self.subscribed.iter().any(|placed| placed == filter) {
            return Ok(());
        }
        match self.transport.subscribe(filter).await {
            Ok(()) => {
                self.subscribed.push(filter.to_owned());
                Ok(())
            }
            Err(Error::Closed) => {
                self.drop_link();
                Err(Error::Closed)
            }
            Err(error) => Err(error),
        }
    }
}

/// Completes with the first rung to deliver, polling them in rotation.
///
/// Polling starts at a different rung each call so one busy link cannot starve the
/// others, and every future but the one that completed is dropped, which is why
/// [`Receive::recv`] has to be cancel-safe.
struct FirstReady<'a> {
    inbound: Vec<(usize, Inbound<'a>)>,
    start: usize,
}

impl Future for FirstReady<'_> {
    type Output = (usize, Result<Option<Message>>);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        let count = this.inbound.len();
        for offset in 0..count {
            let (index, future) = &mut this.inbound[(this.start + offset) % count];
            if let Poll::Ready(outcome) = future.as_mut().poll(cx) {
                return Poll::Ready((*index, outcome));
            }
        }
        Poll::Pending
    }
}

/// An ordered set of transports backed by an offline buffer.
///
/// Rungs are tried in the order they are added, so the cheapest, most-preferred
/// link is added first. A send that no rung accepts is buffered in the
/// [`Store`] and replayed by [`flush`](Self::flush). The ladder is itself a
/// [`Transport`] and a [`Receive`]: a subscription goes onto every rung that
/// listens, and a receive takes whichever rung delivers first.
pub struct TransportLadder<S> {
    rungs: Vec<Rung>,
    filters: Vec<String>,
    buffer: S,
    turn: usize,
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
    /// A ladder with no rungs; add them with [`rung`](Self::rung) and
    /// [`uplink`](Self::uplink).
    pub fn new(buffer: S) -> Self {
        Self {
            rungs: Vec::new(),
            filters: Vec::new(),
            buffer,
            turn: 0,
        }
    }

    /// Adds a rung that sends and listens, lowest-cost first.
    ///
    /// # Arguments
    ///
    /// * `transport` - a link to try. Rungs added earlier are preferred, so add
    ///   the cheapest link first and the costliest fallback last. The ladder
    ///   subscribes on it and listens to it as well as sending over it.
    ///
    /// # Returns
    ///
    /// The ladder, for chaining.
    pub fn rung(mut self, transport: impl Transport + Receive + Send + 'static) -> Self {
        self.rungs.push(Rung {
            transport: Box::new(Erased(transport)),
            listens: true,
            connected: false,
            subscribed: Vec::new(),
        });
        self
    }

    /// Adds a rung that only sends, lowest-cost first.
    ///
    /// This is the shape of a LoRa uplink, a satellite messenger, or any link that
    /// carries readings out but never a command back. The ladder sends over it in
    /// turn with the other rungs and never subscribes or listens on it.
    ///
    /// # Arguments
    ///
    /// * `transport` - a send-only link to try, in the same cheapest-first order as
    ///   [`rung`](Self::rung).
    ///
    /// # Returns
    ///
    /// The ladder, for chaining.
    pub fn uplink(mut self, transport: impl Transport + Send + 'static) -> Self {
        self.rungs.push(Rung {
            transport: Box::new(Uplink(transport)),
            listens: false,
            connected: false,
            subscribed: Vec::new(),
        });
        self
    }

    /// Connects every rung that is not connected, best-effort.
    ///
    /// A rung that fails to connect is left unreachable rather than failing the
    /// whole ladder; sends simply fall through to the next rung or the buffer, and
    /// the next call tries it again. A rung that connects receives every filter
    /// the ladder has been asked to [`subscribe`](Self::subscribe) to, so a
    /// subscription placed while a link was down takes effect when it returns.
    ///
    /// # Returns
    ///
    /// `Ok(())` once every rung has been given the chance to connect.
    ///
    /// # Errors
    ///
    /// This call is best-effort and currently always returns `Ok(())`.
    pub async fn connect(&mut self) -> Result<()> {
        for rung in self.rungs.iter_mut().filter(|rung| !rung.connected) {
            if rung.transport.connect().await.is_err() {
                continue;
            }
            rung.connected = true;
            if !rung.listens {
                continue;
            }
            for filter in &self.filters {
                if matches!(rung.place(filter).await, Err(Error::Closed)) {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Sends a payload, falling back down the rungs and then to the buffer.
    ///
    /// If the buffer is empty, each connected rung is tried in order and the first
    /// to accept the message delivers it. If every rung fails, or the buffer
    /// already holds a backlog, the message is buffered to preserve order. A rung
    /// that answers [`Error::Closed`] is treated as down until the next
    /// [`connect`](Self::connect).
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
    /// Returns [`Error::Io`] if the message must be buffered
    /// but the store cannot be written.
    pub async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<Delivery> {
        if self.buffer.len().await? == 0 && Self::deliver(&mut self.rungs, topic, payload).await {
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
    /// Returns [`Error::Io`] if the store cannot be read or
    /// written, or [`Error::Codec`] if a buffered record
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
    /// Returns [`Error::Io`] if the store length cannot be
    /// read.
    pub async fn buffered(&mut self) -> Result<usize> {
        self.buffer.len().await
    }

    /// Subscribes to a topic on every rung that listens.
    ///
    /// The filter is kept for the life of the ladder: a rung that is down when it
    /// is placed, or that later drops and reconnects, receives it on its next
    /// [`connect`](Self::connect). Subscribing while no rung is up therefore
    /// succeeds, in the same way a send with no rung up is buffered rather than
    /// refused.
    ///
    /// # Arguments
    ///
    /// * `topic` - the topic filter, in the syntax the rungs' links understand.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the filter is live on at least one rung, or is held for the
    /// rungs to pick up as they connect.
    ///
    /// # Errors
    ///
    /// Returns the rung's own [`Error::Transport`] if every connected rung refused
    /// the filter. A rung that answers [`Error::Closed`] is treated as down rather
    /// than as refusing.
    pub async fn subscribe(&mut self, topic: &str) -> Result<()> {
        if !self.filters.iter().any(|filter| filter == topic) {
            self.filters.push(topic.to_owned());
        }
        let mut live = false;
        let mut refusal = None;
        for rung in self
            .rungs
            .iter_mut()
            .filter(|rung| rung.listens && rung.connected)
        {
            match rung.place(topic).await {
                Ok(()) => live = true,
                Err(Error::Closed) => {}
                Err(error) => refusal = Some(error),
            }
        }
        match refusal {
            Some(error) if !live => Err(error),
            _ => Ok(()),
        }
    }

    /// Awaits the next message from any rung that listens.
    ///
    /// Every connected, listening rung is polled together and the first to deliver
    /// wins, starting from a different rung each call so a busy link cannot starve
    /// a quiet one. A rung whose link ends, or that answers [`Error::Closed`], is
    /// treated as down until the next [`connect`](Self::connect), and the wait
    /// continues on the rungs that remain.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next message any rung delivers. `None` is never
    /// returned: a ladder's links come back with [`connect`](Self::connect), so it
    /// reports a closed state rather than an ended one.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`] if no connected rung listens: none was added with
    /// [`rung`](Self::rung), [`connect`](Self::connect) has not been called, or
    /// every listening link has since ended. Returns a rung's own
    /// [`Error::Transport`] if its link fails while waiting.
    pub async fn recv(&mut self) -> Result<Option<Message>> {
        loop {
            let inbound: Vec<(usize, Inbound<'_>)> = self
                .rungs
                .iter_mut()
                .enumerate()
                .filter(|(_, rung)| rung.listens && rung.connected)
                .map(|(index, rung)| (index, rung.transport.recv()))
                .collect();
            if inbound.is_empty() {
                return Err(Error::Closed);
            }
            let start = self.turn % inbound.len();
            self.turn = self.turn.wrapping_add(1);
            let (index, outcome) = FirstReady { inbound, start }.await;
            match outcome {
                Ok(Some(message)) => return Ok(Some(message)),
                Ok(None) | Err(Error::Closed) => self.rungs[index].drop_link(),
                Err(error) => return Err(error),
            }
        }
    }

    /// Tries each connected rung in order, returning whether any accepted the
    /// message. A rung that answers [`Error::Closed`] is dropped until the next
    /// connect.
    async fn deliver(rungs: &mut [Rung], topic: &str, payload: &[u8]) -> bool {
        for rung in rungs.iter_mut().filter(|rung| rung.connected) {
            match rung.transport.send(topic, payload).await {
                Ok(()) => return true,
                Err(Error::Closed) => rung.drop_link(),
                Err(_) => {}
            }
        }
        false
    }
}

impl<S: Store + Send> Transport for TransportLadder<S> {
    async fn connect(&mut self) -> Result<()> {
        TransportLadder::connect(self).await
    }

    /// Sends through the ladder; a buffered message counts as accepted for
    /// delivery, which is what the buffer is for.
    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        TransportLadder::send(self, topic, payload)
            .await
            .map(|_| ())
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        TransportLadder::subscribe(self, topic).await
    }
}

impl<S: Store + Send> Receive for TransportLadder<S> {
    async fn recv(&mut self) -> Result<Option<Message>> {
        TransportLadder::recv(self).await
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

    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use pamoja_codec::CborCodec;
    use pamoja_core::{Actuator, Sensor};
    use pamoja_loopback::{Faulty, LoopbackBroker, LoopbackTransport};
    use pamoja_profile::{ControlSpec, Node, PowerSchedule, Profile};
    use pamoja_sync::MemoryStore;

    /// Accepts anything that can move between threads.
    fn assert_send<T: Send>(_value: T) {}

    /// Accepts anything that is a full link.
    fn assert_link<T: Transport + Receive>(_value: &T) {}

    /// What a scripted link has been asked to do, shared with the test.
    #[derive(Default)]
    struct Log {
        connects: usize,
        subscriptions: Vec<String>,
        sent: Vec<(String, Vec<u8>)>,
        closed: bool,
    }

    /// A link whose inbound side plays a script: each entry is a delivered message
    /// or the end of the link, and a link past its script waits forever.
    struct Scripted {
        inbound: VecDeque<Option<Message>>,
        log: Arc<Mutex<Log>>,
    }

    impl Scripted {
        fn new(inbound: impl IntoIterator<Item = Option<Message>>) -> (Self, Arc<Mutex<Log>>) {
            let log = Arc::new(Mutex::new(Log::default()));
            let link = Self {
                inbound: inbound.into_iter().collect(),
                log: Arc::clone(&log),
            };
            (link, log)
        }
    }

    impl Transport for Scripted {
        async fn connect(&mut self) -> Result<()> {
            self.log.lock().expect("log").connects += 1;
            Ok(())
        }

        async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
            let mut log = self.log.lock().expect("log");
            if log.closed {
                return Err(Error::Closed);
            }
            log.sent.push((topic.to_owned(), payload.to_vec()));
            Ok(())
        }

        async fn subscribe(&mut self, topic: &str) -> Result<()> {
            self.log
                .lock()
                .expect("log")
                .subscriptions
                .push(topic.to_owned());
            Ok(())
        }
    }

    impl Receive for Scripted {
        async fn recv(&mut self) -> Result<Option<Message>> {
            match self.inbound.pop_front() {
                Some(next) => Ok(next),
                None => std::future::pending().await,
            }
        }
    }

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
        assert_send(ladder.subscribe("commands/#"));
        assert_send(ladder.recv());
        assert_link(&ladder);
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

    #[tokio::test]
    async fn a_subscription_goes_on_every_rung_and_a_receive_takes_whichever_delivers() {
        // Two rungs on two brokers, as a mesh hop and a backhaul would be.
        let mesh = LoopbackBroker::new();
        let backhaul = LoopbackBroker::new();
        let mut ladder = TransportLadder::new(MemoryStore::new())
            .rung(LoopbackTransport::new(mesh.clone()))
            .rung(LoopbackTransport::new(backhaul.clone()));
        ladder.connect().await.expect("connect");
        ladder.subscribe("commands/#").await.expect("subscribe");

        // A command published on either broker reaches the ladder.
        let mut over_backhaul = LoopbackTransport::new(backhaul);
        over_backhaul.connect().await.expect("connect");
        over_backhaul
            .send("commands/valve", b"open")
            .await
            .expect("send");
        let command = ladder.recv().await.expect("recv").expect("a command");
        assert_eq!(command.topic, "commands/valve");
        assert_eq!(command.payload, b"open");

        let mut over_mesh = LoopbackTransport::new(mesh);
        over_mesh.connect().await.expect("connect");
        over_mesh
            .send("commands/valve", b"close")
            .await
            .expect("send");
        let command = ladder.recv().await.expect("recv").expect("a command");
        assert_eq!(command.payload, b"close");
    }

    #[tokio::test]
    async fn a_filter_placed_while_no_rung_is_up_takes_effect_on_connect() {
        let broker = LoopbackBroker::new();
        let mut ladder =
            TransportLadder::new(MemoryStore::new()).rung(LoopbackTransport::new(broker.clone()));

        // Nothing is connected yet, so the filter is held rather than refused, and
        // there is nothing to receive from.
        ladder.subscribe("commands/#").await.expect("held");
        assert!(matches!(ladder.recv().await, Err(Error::Closed)));

        ladder.connect().await.expect("connect");
        let mut publisher = LoopbackTransport::new(broker);
        publisher.connect().await.expect("connect");
        publisher.send("commands/1", b"go").await.expect("send");
        let command = ladder.recv().await.expect("recv").expect("a command");
        assert_eq!(command.payload, b"go");
    }

    #[tokio::test]
    async fn an_uplink_sends_but_is_never_listened_on() {
        let radio = LoopbackBroker::new();
        let mut ground = gateway(&radio).await;
        let mut ladder =
            TransportLadder::new(MemoryStore::new()).uplink(LoopbackTransport::new(radio.clone()));
        ladder.connect().await.expect("connect");

        // A reading goes out over the uplink like any rung.
        assert_eq!(
            ladder.send("sensors/1", b"21.5").await.expect("send"),
            Delivery::Sent
        );
        let reading = ground.recv().await.expect("recv").expect("a message");
        assert_eq!(reading.payload, b"21.5");

        // The uplink is not subscribed and not listened on: a message on its broker
        // never reaches the ladder, which has no listening rung at all.
        ladder.subscribe("commands/#").await.expect("held");
        ground.send("commands/1", b"go").await.expect("send");
        assert!(matches!(ladder.recv().await, Err(Error::Closed)));
    }

    #[tokio::test]
    async fn receiving_rotates_between_rungs_and_drops_one_whose_link_ends() {
        let (first, first_log) = Scripted::new([
            Some(Message::new("c", b"first-1")),
            None,
            Some(Message::new("c", b"never")),
        ]);
        let (second, second_log) = Scripted::new([
            Some(Message::new("c", b"second-1")),
            Some(Message::new("c", b"second-2")),
        ]);
        let mut ladder = TransportLadder::new(MemoryStore::new())
            .rung(first)
            .rung(second);
        ladder.connect().await.expect("connect");
        ladder.subscribe("c").await.expect("subscribe");
        assert_eq!(first_log.lock().expect("log").subscriptions, ["c"]);
        assert_eq!(second_log.lock().expect("log").subscriptions, ["c"]);

        // Both rungs are ready on every call; the starting rung rotates, so each
        // gets a turn rather than the first always winning.
        let one = ladder.recv().await.expect("recv").expect("a message");
        let two = ladder.recv().await.expect("recv").expect("a message");
        assert_eq!(one.payload, b"first-1");
        assert_eq!(two.payload, b"second-1");

        // The first link ends. The ladder drops it and keeps waiting on the second,
        // whose next message arrives instead of the ended link's leftover.
        let three = ladder.recv().await.expect("recv").expect("a message");
        assert_eq!(three.payload, b"second-2");

        // Connecting again brings the ended link back and places the filter on it
        // afresh; the link that stayed up is not connected or subscribed twice.
        ladder.connect().await.expect("connect");
        assert_eq!(first_log.lock().expect("log").connects, 2);
        assert_eq!(first_log.lock().expect("log").subscriptions, ["c", "c"]);
        assert_eq!(second_log.lock().expect("log").connects, 1);
        assert_eq!(second_log.lock().expect("log").subscriptions, ["c"]);
        let four = ladder.recv().await.expect("recv").expect("a message");
        assert_eq!(four.payload, b"never");
    }

    #[tokio::test]
    async fn a_rung_that_reports_closed_on_send_is_down_until_the_next_connect() {
        let (link, log) = Scripted::new([]);
        let mut ladder = TransportLadder::new(MemoryStore::new()).rung(link);
        ladder.connect().await.expect("connect");
        assert_eq!(ladder.send("t", b"1").await.expect("send"), Delivery::Sent);

        // The link closes underneath the ladder: the send buffers and the rung is
        // not tried again until it is connected again.
        log.lock().expect("log").closed = true;
        assert_eq!(
            ladder.send("t", b"2").await.expect("send"),
            Delivery::Buffered
        );
        log.lock().expect("log").closed = false;
        assert_eq!(ladder.flush().await.expect("flush"), 0);
        assert_eq!(log.lock().expect("log").connects, 1);

        ladder.connect().await.expect("connect");
        assert_eq!(log.lock().expect("log").connects, 2);
        assert_eq!(ladder.flush().await.expect("flush"), 1);
        assert_eq!(log.lock().expect("log").sent.len(), 2);
    }

    /// A reading that is the same every time.
    struct Probe(f32);

    impl Sensor for Probe {
        type Reading = f32;

        async fn read(&mut self) -> Result<f32> {
            Ok(self.0)
        }
    }

    /// An output that records what it was told.
    struct Valve(Vec<bool>);

    impl Actuator for Valve {
        type Command = bool;

        async fn apply(&mut self, open: bool) -> Result<()> {
            self.0.push(open);
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_profile_node_publishes_through_a_ladder() {
        let broker = LoopbackBroker::new();
        let mut observer = gateway(&broker).await;
        let mut ladder =
            TransportLadder::new(MemoryStore::new()).rung(LoopbackTransport::new(broker));
        ladder.connect().await.expect("connect");

        let profile = Profile {
            name: "raised-bed-drip".to_owned(),
            topic: "garden/bed-1/moisture".to_owned(),
            control: ControlSpec::Setpoint {
                setpoint: 37.5,
                hysteresis: 7.5,
                cooling: false,
                safe_band: 30.0,
            },
            power: PowerSchedule::new(300, 1800, 3600),
            presentation: None,
        };
        let mut node = Node::new(profile, Probe(20.0), Valve(Vec::new()), ladder, CborCodec);
        let reaction = node.tick().await.expect("a tick");
        assert_eq!(reaction.actuator, Some(true));

        let reading = observer.recv().await.expect("recv").expect("a reading");
        assert_eq!(reading.topic, "garden/bed-1/moisture");
        assert!(!reading.payload.is_empty());
    }
}
