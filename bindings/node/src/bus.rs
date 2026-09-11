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

use std::sync::Arc;

use crate::transport::bytes_of;
use napi::bindgen_prelude::Buffer;
use napi::Either;
use napi_derive::napi;
use pamoja_bus::BroadcastBus;
use pamoja_core::EventBus as CoreEventBus;
use tokio::sync::Mutex;

/// One endpoint on an event bus.
///
/// An endpoint both publishes and receives. Each subscriber needs its own, taken
/// with `subscribe`, because an endpoint only sees events published after it
/// existed.
#[napi]
pub struct EventBus {
    inner: Arc<Mutex<BroadcastBus<Vec<u8>>>>,
}

#[napi]
impl EventBus {
    /// Creates an event bus.
    ///
    /// @param capacity - how many events a slow subscriber may fall behind
    ///   before it starts missing them.
    #[napi(constructor)]
    pub fn new(capacity: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BroadcastBus::new(capacity as usize))),
        }
    }

    /// Takes another endpoint on the same bus.
    ///
    /// The new endpoint sees events published from now on, not those already
    /// sent, so subscribe before publishing anything it needs to see.
    #[napi]
    pub async fn subscribe(&self) -> EventBus {
        let taken = self.inner.lock().await.subscribe();
        Self {
            inner: Arc::new(Mutex::new(taken)),
        }
    }

    /// Publishes an event to every subscriber: bytes, or text such as an event name.
    #[napi]
    pub async fn publish(&self, event: Either<Buffer, String>) -> napi::Result<()> {
        self.inner
            .lock()
            .await
            .publish(bytes_of(event))
            .await
            .map_err(to_napi)
    }

    /// Waits for the next event on this endpoint, or `null` once the bus closes.
    #[napi]
    pub async fn next(&self) -> napi::Result<Option<Buffer>> {
        self.inner
            .lock()
            .await
            .next_event()
            .await
            .map(|event| event.map(Buffer::from))
            .map_err(to_napi)
    }

    /// Waits for the next event as text, or `null` once the bus closes.
    ///
    /// Throws if the event is not UTF-8 text.
    #[napi]
    pub async fn next_text(&self) -> napi::Result<Option<String>> {
        let event = self
            .inner
            .lock()
            .await
            .next_event()
            .await
            .map_err(to_napi)?;
        match event {
            Some(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| napi::Error::from_reason("the event is not UTF-8 text")),
            None => Ok(None),
        }
    }
}

/// Maps a core error onto the one JavaScript sees.
fn to_napi(error: pamoja_core::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
