//! Generated Python bindings for the cost-aware transport ladder.
//!
//! These mirror the `pamoja-ladder` Rust API. A ladder is the answer to a node
//! that has more than one way to reach the network and no single one that always
//! works: rungs are tried in the order they were added, cheapest first, and a
//! message no rung accepts goes into a buffer rather than being lost.
//!
//! The Rust ladder is generic over its buffer, which cannot reach Python, so
//! this one is built over the store class from [`crate::sync`]. That class
//! already covers both an in-memory and a file-backed queue, so nothing is given
//! up: a caller still chooses whether the buffer survives a restart.

use std::sync::Arc;

use pamoja_ladder::{Delivery, TransportLadder};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use tokio::sync::Mutex;

use crate::sync::{SharedStore, Store};
use crate::transport::PyTransport;
use crate::PamojaError;

/// An ordered set of transports backed by an offline buffer.
#[gen_stub_pyclass]
#[pyclass]
pub struct Ladder {
    inner: Arc<Mutex<Option<TransportLadder<SharedStore>>>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Ladder {
    /// Creates a ladder with no rungs, buffering into a store.
    ///
    /// The store is consumed: the ladder owns it from here on.
    #[new]
    fn new(store: &Store) -> PyResult<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(Some(TransportLadder::new(store.take()?)))),
        })
    }

    /// Adds a rung, which is tried after the rungs already added.
    ///
    /// Add the cheapest, most-preferred link first and the costliest fallback
    /// last, because a send takes the first rung that accepts it. The transport
    /// is consumed.
    fn rung<'py>(&self, py: Python<'py>, transport: &PyTransport) -> PyResult<Bound<'py, PyAny>> {
        let transport = transport.take()?;
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            let ladder = guard.take().ok_or_else(unusable)?;
            *guard = Some(ladder.rung(transport));
            Ok(())
        })
    }

    /// Connects every rung, so a send can be tried against each in turn.
    ///
    /// A rung that will not connect is left in the ladder: it may come back, and
    /// a send simply falls through it until it does.
    fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            let ladder = guard.as_mut().ok_or_else(unusable)?;
            ladder.connect().await.map_err(to_pyerr)
        })
    }

    /// Sends a payload, falling through the rungs and buffering if none take it.
    ///
    /// Returns `"Sent"` or `"Buffered"`. Buffering is a success, not a failure:
    /// it is what the ladder exists to do.
    fn send<'py>(
        &self,
        py: Python<'py>,
        topic: String,
        payload: Vec<u8>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            let ladder = guard.as_mut().ok_or_else(unusable)?;
            ladder
                .send(&topic, &payload)
                .await
                .map(|delivery| match delivery {
                    Delivery::Sent => "Sent",
                    Delivery::Buffered => "Buffered",
                })
                .map_err(to_pyerr)
        })
    }

    /// Replays the buffer over the rungs, oldest message first, and reports how
    /// many went out.
    fn flush<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            let ladder = guard.as_mut().ok_or_else(unusable)?;
            ladder.flush().await.map_err(to_pyerr)
        })
    }

    /// How many messages are waiting in the buffer.
    fn buffered<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            let ladder = guard.as_mut().ok_or_else(unusable)?;
            ladder.buffered().await.map_err(to_pyerr)
        })
    }
}

/// The error a ladder left empty by a failed rung reports.
fn unusable() -> PyErr {
    PamojaError::new_err("this ladder is no longer usable")
}

/// Maps a core error onto the one Python sees.
fn to_pyerr(error: pamoja_core::Error) -> PyErr {
    PamojaError::new_err(error.to_string())
}
