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

use std::sync::Arc;

use pamoja_bus::BroadcastBus;
use pamoja_core::EventBus as CoreEventBus;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use tokio::sync::Mutex;

use crate::PamojaError;

/// One endpoint on an event bus.
///
/// An endpoint both publishes and receives. Each subscriber needs its own, taken
/// with `subscribe`, because an endpoint only sees events published after it
/// existed.
#[gen_stub_pyclass]
#[pyclass]
pub struct EventBus {
    inner: Arc<Mutex<BroadcastBus<Vec<u8>>>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl EventBus {
    /// Creates an event bus.
    ///
    /// `capacity` is how many events a slow subscriber may fall behind before it
    /// starts missing them.
    #[new]
    #[pyo3(signature = (capacity = 64))]
    fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BroadcastBus::new(capacity))),
        }
    }

    /// Takes another endpoint on the same bus.
    ///
    /// The new endpoint sees events published from now on, not those already
    /// sent, so subscribe before publishing anything it needs to see.
    fn subscribe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let bus = inner.lock().await;
            Ok(EventBus {
                inner: Arc::new(Mutex::new(bus.subscribe())),
            })
        })
    }

    /// Publishes an event to every subscriber.
    fn publish<'py>(&self, py: Python<'py>, event: Vec<u8>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let bus = inner.lock().await;
            bus.publish(event).await.map_err(to_pyerr)
        })
    }

    /// Waits for the next event on this endpoint, or `None` once the bus closes.
    fn next_event<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut bus = inner.lock().await;
            bus.next_event().await.map_err(to_pyerr)
        })
    }
}

/// Maps a core error onto the one Python sees.
fn to_pyerr(error: pamoja_core::Error) -> PyErr {
    PamojaError::new_err(error.to_string())
}
