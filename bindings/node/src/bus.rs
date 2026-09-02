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

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_bus::BroadcastBus;
use pamoja_core::EventBus as CoreEventBus;

/// One endpoint on an event bus.
///
/// An endpoint both publishes and receives. Each subscriber needs its own, taken
/// with `subscribe`, because an endpoint only sees events published after it
/// existed.
#[napi]
pub struct EventBus {
    inner: BroadcastBus<Vec<u8>>,
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
            inner: BroadcastBus::new(capacity as usize),
        }
    }

    /// Takes another endpoint on the same bus.
    ///
    /// The new endpoint sees events published from now on, not those already
    /// sent, so subscribe before publishing anything it needs to see.
    #[napi]
    pub fn subscribe(&self) -> EventBus {
        Self {
            inner: self.inner.subscribe(),
        }
    }

    /// Publishes an event to every subscriber.
    #[napi]
    pub async fn publish(&self, event: Buffer) -> napi::Result<()> {
        self.inner
            .publish(event.to_vec())
            .await
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// Waits for the next event on this endpoint, or `null` once the bus closes.
    #[napi]
    pub async unsafe fn next(&mut self) -> napi::Result<Option<Buffer>> {
        self.inner
            .next_event()
            .await
            .map(|event| event.map(Buffer::from))
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }
}
