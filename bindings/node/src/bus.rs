//! Generated Node bindings for the in-process event bus.
//!
//! These mirror the `pamoja-bus` Rust API: one publisher, many subscribers,
//! inside a single process. It is how the parts of a gateway talk to each other
//! without knowing about each other, so a sampler can announce a reading and
//! whatever cares about readings picks it up.
//!
//! The Rust bus carries any cloneable event; JavaScript has no such parameter,
//! so this one carries bytes. That is the shape the binding already exchanges,
//! and a caller who wants structure encodes it with `toCbor` on the way in.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::transport::{bytes_of, limit_of};
use napi::bindgen_prelude::Buffer;
use napi::Either;
use napi_derive::napi;
use pamoja_bus::BroadcastBus;
use pamoja_core::EventBus as CoreEventBus;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// The buffer depth a bus gets when none is given.
const DEFAULT_CAPACITY: usize = 64;

/// One endpoint on an event bus.
///
/// An endpoint both publishes and receives, and it receives what it publishes
/// itself. Each subscriber needs its own, taken with `subscribe`, because an
/// endpoint only sees events published after it existed. Only `next` and
/// `nextText` wait: publishing and subscribing return at once, even while a
/// `next` on the same endpoint is waiting.
#[napi]
pub struct EventBus {
    publisher: pamoja_bus::EventPublisher<Vec<u8>>,
    receiver: Mutex<BroadcastBus<Vec<u8>>>,
    missed: AtomicU64,
}

impl EventBus {
    /// Wraps one subscription as an endpoint.
    fn of(receiver: BroadcastBus<Vec<u8>>) -> Self {
        Self {
            publisher: receiver.publisher(),
            receiver: Mutex::new(receiver),
            missed: AtomicU64::new(0),
        }
    }

    /// Waits for this endpoint's next event, within `timeout_ms` when a limit is
    /// given, and records how many events it has missed.
    ///
    /// A wait that runs out takes no event. A promise that JavaScript merely stops
    /// awaiting does not stop the wait behind it, which would then take the next
    /// event, so the limit has to live here.
    async fn wait(&self, timeout_ms: Option<f64>) -> napi::Result<Vec<u8>> {
        let limit = limit_of(timeout_ms)?;
        let deadline = limit.and_then(|limit| Instant::now().checked_add(limit));
        let timed_out = || {
            napi::Error::from_reason(format!(
                "no event arrived within {} ms",
                limit.unwrap_or_default().as_millis()
            ))
        };
        let mut receiver = match deadline {
            Some(deadline) => tokio::time::timeout_at(deadline, self.receiver.lock())
                .await
                .map_err(|_| timed_out())?,
            None => self.receiver.lock().await,
        };
        let event = match deadline {
            Some(deadline) => tokio::time::timeout_at(deadline, receiver.next_event()).await,
            None => Ok(receiver.next_event().await),
        };
        self.missed.store(receiver.missed(), Ordering::Relaxed);
        match event {
            Err(_) => Err(timed_out()),
            Ok(Ok(Some(event))) => Ok(event),
            Ok(Ok(None)) => Err(napi::Error::from_reason("the event bus closed")),
            Ok(Err(error)) => Err(to_napi(error)),
        }
    }
}

#[napi]
impl EventBus {
    /// Creates an event bus.
    ///
    /// @param capacity - how many events a slow subscriber may fall behind
    ///   before it starts missing them, rounded up to the next power of two and
    ///   at most 1048576; 64 when not given.
    #[napi(constructor)]
    pub fn new(capacity: Option<f64>) -> napi::Result<Self> {
        Ok(Self::of(BroadcastBus::new(capacity_of(capacity)?)))
    }

    /// Takes another endpoint on the same bus.
    ///
    /// The new endpoint sees events published from now on, not those already
    /// sent, so subscribe before publishing anything it needs to see.
    #[napi]
    pub fn subscribe(&self) -> EventBus {
        Self::of(self.publisher.subscribe())
    }

    /// Takes a publish-only handle on the same bus, for a part that announces and
    /// never reads.
    #[napi]
    pub fn publisher(&self) -> EventPublisher {
        EventPublisher {
            inner: self.publisher.clone(),
        }
    }

    /// Publishes an event to every subscriber, this endpoint included: bytes, or
    /// text such as an event name.
    ///
    /// It never waits: a subscriber that has fallen behind loses its oldest event
    /// rather than holding up the publisher.
    #[napi]
    pub fn publish(&self, event: Either<Buffer, String>) {
        self.publisher.publish(bytes_of(event));
    }

    /// Waits for the next event on this endpoint.
    ///
    /// @param timeoutMs - how long to wait before throwing; the next event is then
    ///   left for the next call. Without it, the wait lasts until an event arrives.
    #[napi]
    pub async fn next(&self, timeout_ms: Option<f64>) -> napi::Result<Buffer> {
        self.wait(timeout_ms).await.map(Buffer::from)
    }

    /// Waits for the next event as text.
    ///
    /// Throws if the event is not UTF-8 text.
    ///
    /// @param timeoutMs - how long to wait before throwing; the next event is then
    ///   left for the next call. Without it, the wait lasts until an event arrives.
    #[napi]
    pub async fn next_text(&self, timeout_ms: Option<f64>) -> napi::Result<String> {
        let event = self.wait(timeout_ms).await?;
        String::from_utf8(event)
            .map_err(|_| napi::Error::from_reason("the event is not UTF-8 text"))
    }

    /// How many events this endpoint lost by falling behind, as of its last
    /// completed wait.
    #[napi(getter)]
    pub fn missed(&self) -> f64 {
        self.missed.load(Ordering::Relaxed) as f64
    }
}

/// A publish-only handle to an event bus.
///
/// It has no queue of its own, so a part that only announces never fills a
/// buffer it does not read, and publishing from it returns how many subscribers
/// the event was handed to.
#[napi]
pub struct EventPublisher {
    inner: pamoja_bus::EventPublisher<Vec<u8>>,
}

#[napi]
impl EventPublisher {
    /// Creates a bus with no subscribers yet, and a publisher on it.
    ///
    /// @param capacity - how many events a slow subscriber may fall behind
    ///   before it starts missing them, rounded up to the next power of two and
    ///   at most 1048576; 64 when not given.
    #[napi(constructor)]
    pub fn new(capacity: Option<f64>) -> napi::Result<Self> {
        Ok(Self {
            inner: pamoja_bus::EventPublisher::new(capacity_of(capacity)?),
        })
    }

    /// Hands an event to every current subscriber: bytes, or text such as an
    /// event name.
    ///
    /// It never waits.
    ///
    /// @returns How many subscribers the event was handed to; an event published
    ///   while no one is subscribed is dropped, and this is 0.
    #[napi]
    pub fn publish(&self, event: Either<Buffer, String>) -> u32 {
        u32::try_from(self.inner.publish(bytes_of(event))).unwrap_or(u32::MAX)
    }

    /// Subscribes to the bus, returning an endpoint that sees events published
    /// from now on.
    #[napi]
    pub fn subscribe(&self) -> EventBus {
        EventBus::of(self.inner.subscribe())
    }

    /// Takes another publisher on the same bus, for another part that announces.
    #[napi]
    pub fn publisher(&self) -> EventPublisher {
        EventPublisher {
            inner: self.inner.publisher(),
        }
    }
}

/// Reads a capacity, refusing one that is negative or not a number.
///
/// JavaScript hands a negative number to an unsigned parameter as a very large
/// one, so a capacity is taken as a number and checked here instead.
fn capacity_of(capacity: Option<f64>) -> napi::Result<usize> {
    match capacity {
        None => Ok(DEFAULT_CAPACITY),
        Some(capacity) if capacity >= 0.0 => Ok(capacity as usize),
        Some(_) => Err(napi::Error::from_reason("a capacity must be 0 or more")),
    }
}

/// Maps a core error onto the one JavaScript sees.
fn to_napi(error: pamoja_core::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
