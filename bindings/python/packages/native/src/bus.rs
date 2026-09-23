//! Generated Python bindings for the in-process event bus.
//!
//! These mirror the `pamoja-bus` Rust API: one publisher, many subscribers,
//! inside a single process. It is how the parts of a gateway talk to each other
//! without knowing about each other, so a sampler can announce a reading and
//! whatever cares about readings picks it up.
//!
//! The Rust bus carries any cloneable event; Python has no such parameter, so
//! this one carries bytes. That is the shape the binding already exchanges, and
//! a caller who wants structure encodes it with `to_cbor` on the way in.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use pamoja_bus::BroadcastBus;
use pamoja_core::EventBus as CoreEventBus;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use tokio::sync::Mutex;

use crate::transport::Payload;
use crate::PamojaError;

/// What an endpoint's calls share: the publisher that never waits, and the
/// subscription a wait holds.
struct Endpoint {
    publisher: pamoja_bus::EventPublisher<Vec<u8>>,
    receiver: Mutex<BroadcastBus<Vec<u8>>>,
    missed: AtomicU64,
}

/// One endpoint on an event bus.
///
/// An endpoint both publishes and receives, and it receives what it publishes
/// itself. Each subscriber needs its own, taken with `subscribe`, because an
/// endpoint only sees events published after it existed. Only `next_event` and
/// `next_text` wait: publishing and subscribing return at once, even while a
/// wait on the same endpoint is open.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct EventBus {
    inner: Arc<Endpoint>,
}

impl EventBus {
    /// Wraps one subscription as an endpoint.
    fn of(receiver: BroadcastBus<Vec<u8>>) -> Self {
        Self {
            inner: Arc::new(Endpoint {
                publisher: receiver.publisher(),
                receiver: Mutex::new(receiver),
                missed: AtomicU64::new(0),
            }),
        }
    }
}

/// Waits for an endpoint's next event and records how many it has missed.
async fn wait(endpoint: Arc<Endpoint>) -> PyResult<Vec<u8>> {
    let mut receiver = endpoint.receiver.lock().await;
    let event = receiver.next_event().await;
    endpoint.missed.store(receiver.missed(), Ordering::Relaxed);
    match event {
        Ok(Some(event)) => Ok(event),
        Ok(None) => Err(PamojaError::new_err("the event bus closed")),
        Err(error) => Err(to_pyerr(error)),
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl EventBus {
    /// Creates an event bus.
    ///
    /// `capacity` is how many events a slow subscriber may fall behind before it
    /// starts missing them, rounded up to the next power of two and at most
    /// 1048576.
    #[new]
    #[pyo3(signature = (capacity = 64))]
    fn new(capacity: usize) -> Self {
        Self::of(BroadcastBus::new(capacity))
    }

    /// Takes another endpoint on the same bus.
    ///
    /// The new endpoint sees events published from now on, not those already
    /// sent, so subscribe before publishing anything it needs to see.
    fn subscribe(&self) -> EventBus {
        Self::of(self.inner.publisher.subscribe())
    }

    /// Takes a publish-only handle on the same bus, for a part that announces and
    /// never reads.
    fn publisher(&self) -> EventPublisher {
        EventPublisher {
            inner: self.inner.publisher.clone(),
        }
    }

    /// Publishes an event to every subscriber, this endpoint included: bytes, or
    /// text such as an event name.
    ///
    /// It never waits: a subscriber that has fallen behind loses its oldest event
    /// rather than holding up the publisher.
    fn publish(&self, event: Payload) {
        self.inner.publisher.publish(event.into_bytes());
    }

    /// Waits for the next event on this endpoint.
    ///
    /// The wait lasts until an event arrives. `asyncio.wait_for` gives up sooner,
    /// and the wait it cancels takes no event.
    fn next_event<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let endpoint = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, wait(endpoint))
    }

    /// Waits for the next event as text.
    ///
    /// Raises `ValueError` if the event is not UTF-8 text.
    fn next_text<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let endpoint = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let event = wait(endpoint).await?;
            String::from_utf8(event)
                .map_err(|_| pyo3::exceptions::PyValueError::new_err("the event is not UTF-8 text"))
        })
    }

    /// How many events this endpoint lost by falling behind, as of its last
    /// completed wait.
    #[getter]
    fn missed(&self) -> u64 {
        self.inner.missed.load(Ordering::Relaxed)
    }
}

/// A publish-only handle to an event bus.
///
/// It has no queue of its own, so a part that only announces never fills a
/// buffer it does not read. Publishing never waits, so a callback on another
/// thread can call it without an event loop.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct EventPublisher {
    inner: pamoja_bus::EventPublisher<Vec<u8>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl EventPublisher {
    /// Creates a bus with no subscribers yet, and a publisher on it.
    ///
    /// `capacity` is how many events a slow subscriber may fall behind before it
    /// starts missing them, rounded up to the next power of two and at most
    /// 1048576.
    #[new]
    #[pyo3(signature = (capacity = 64))]
    fn new(capacity: usize) -> Self {
        Self {
            inner: pamoja_bus::EventPublisher::new(capacity),
        }
    }

    /// Hands an event to every current subscriber: bytes, or text such as an
    /// event name.
    ///
    /// Returns how many subscribers the event was handed to. An event published
    /// while no one is subscribed is dropped, and the count is 0.
    fn publish(&self, event: Payload) -> usize {
        self.inner.publish(event.into_bytes())
    }

    /// Subscribes to the bus, returning an endpoint that sees events published
    /// from now on.
    fn subscribe(&self) -> EventBus {
        EventBus::of(self.inner.subscribe())
    }

    /// Takes another publisher on the same bus, for another part that announces.
    fn publisher(&self) -> EventPublisher {
        EventPublisher {
            inner: self.inner.publisher(),
        }
    }
}

/// Maps a core error onto the one Python sees.
fn to_pyerr(error: pamoja_core::Error) -> PyErr {
    PamojaError::new_err(error.to_string())
}
