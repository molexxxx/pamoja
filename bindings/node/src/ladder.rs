//! Generated Node bindings for the cost-aware transport ladder.
//!
//! These mirror the `pamoja-ladder` Rust API. A ladder is the answer to a node
//! that has more than one way to reach the network and no single one that always
//! works: rungs are tried in the order they were added, cheapest first, and a
//! message no rung accepts goes into a buffer rather than being lost.
//!
//! The Rust ladder is generic over its buffer, which cannot reach JavaScript, so
//! this one is built over the store class from [`crate::sync`]. That class
//! already covers both an in-memory and a file-backed queue, so nothing is given
//! up: a caller still chooses whether the buffer survives a restart.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_ladder::{Delivery as CoreDelivery, TransportLadder};

use crate::sync::{Store, StoreKind};
use crate::transport::Transport;

/// What became of a message handed to a ladder.
#[napi(string_enum)]
pub enum Delivery {
    /// A rung took the message and it is on its way.
    Sent,
    /// No rung would take it, so it is in the buffer awaiting a flush.
    Buffered,
}

/// An ordered set of transports backed by an offline buffer.
#[napi]
pub struct Ladder {
    inner: Option<TransportLadder<StoreKind>>,
}

#[napi]
impl Ladder {
    /// Creates a ladder with no rungs, buffering into a store.
    ///
    /// The store is consumed: the ladder owns it from here on.
    #[napi(constructor)]
    pub fn new(store: &mut Store) -> napi::Result<Self> {
        Ok(Self {
            inner: Some(TransportLadder::new(store.take()?)),
        })
    }

    /// Adds a rung, which is tried after the rungs already added.
    ///
    /// Add the cheapest, most-preferred link first and the costliest fallback
    /// last, because a send takes the first rung that accepts it. The transport
    /// is consumed.
    #[napi]
    pub fn rung(&mut self, transport: &mut Transport) -> napi::Result<()> {
        let transport = transport.take()?;
        let ladder = self.take()?;
        self.inner = Some(ladder.rung(transport));
        Ok(())
    }

    /// Connects every rung, so a send can be tried against each in turn.
    ///
    /// A rung that will not connect is left in the ladder: it may come back, and
    /// a send simply falls through it until it does.
    #[napi]
    pub async unsafe fn connect(&mut self) -> napi::Result<()> {
        self.borrow()?.connect().await.map_err(to_napi)
    }

    /// Sends a payload, falling through the rungs and buffering if none take it.
    ///
    /// Buffering is a success, not a failure: it is what the ladder exists to do.
    #[napi]
    pub async unsafe fn send(&mut self, topic: String, payload: Buffer) -> napi::Result<Delivery> {
        let payload = payload.to_vec();
        self.borrow()?
            .send(&topic, &payload)
            .await
            .map(|delivery| match delivery {
                CoreDelivery::Sent => Delivery::Sent,
                CoreDelivery::Buffered => Delivery::Buffered,
            })
            .map_err(to_napi)
    }

    /// Replays the buffer over the rungs, oldest message first, and reports how
    /// many went out.
    #[napi]
    pub async unsafe fn flush(&mut self) -> napi::Result<u32> {
        self.borrow()?
            .flush()
            .await
            .map(|sent| sent as u32)
            .map_err(to_napi)
    }

    /// How many messages are waiting in the buffer.
    #[napi]
    pub async unsafe fn buffered(&mut self) -> napi::Result<u32> {
        self.borrow()?
            .buffered()
            .await
            .map(|count| count as u32)
            .map_err(to_napi)
    }
}

impl Ladder {
    /// Takes the ladder out so a rung can be added to it by value.
    fn take(&mut self) -> napi::Result<TransportLadder<StoreKind>> {
        self.inner
            .take()
            .ok_or_else(|| napi::Error::from_reason("this ladder is no longer usable"))
    }

    /// Borrows the ladder, refusing one left empty by a failed rung.
    fn borrow(&mut self) -> napi::Result<&mut TransportLadder<StoreKind>> {
        self.inner
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("this ladder is no longer usable"))
    }
}

/// Maps a core error onto the one JavaScript sees.
fn to_napi(error: pamoja_core::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
