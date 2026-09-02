//! Generated Python bindings for the in-process loopback broker.
//!
//! These mirror the `pamoja-loopback` Rust API so a caller can exercise the
//! publish-and-subscribe path with no broker, no network, and no hardware. That
//! matters more from a binding than from Rust: someone writing against the SDK
//! in Python can drive a whole message flow in a unit test rather than standing
//! up infrastructure to find out whether their topics line up.

use std::sync::Arc;

use pamoja_core::Transport;
use pamoja_loopback::{LoopbackBroker as CoreBroker, LoopbackTransport as CoreLoopback};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use tokio::sync::Mutex;

use crate::transport::{Kind, Message, PyTransport};
use crate::PamojaError;

/// An in-process broker.
///
/// Every transport built from one broker shares its traffic, so a message one
/// publishes reaches the others that subscribed to the topic.
#[gen_stub_pyclass]
#[pyclass]
pub struct LoopbackBroker {
    inner: CoreBroker,
}

#[gen_stub_pymethods]
#[pymethods]
impl LoopbackBroker {
    /// Creates a broker with no traffic.
    #[new]
    fn new() -> Self {
        Self {
            inner: CoreBroker::new(),
        }
    }

    /// Creates a link to this broker, for driving directly.
    fn link(&self) -> LoopbackTransport {
        LoopbackTransport {
            inner: Arc::new(Mutex::new(CoreLoopback::new(self.inner.clone()))),
        }
    }

    /// Creates a link to this broker as a transport, for composing into a ladder
    /// or a wrapper.
    fn rung(&self) -> PyTransport {
        PyTransport::wrap(Kind::Loopback(CoreLoopback::new(self.inner.clone())))
    }
}

/// One in-process link to a broker.
#[gen_stub_pyclass]
#[pyclass]
pub struct LoopbackTransport {
    inner: Arc<Mutex<CoreLoopback>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LoopbackTransport {
    /// Marks this link connected so it will carry traffic.
    fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport.connect().await.map_err(to_pyerr)
        })
    }

    /// Publishes a payload to a topic on the broker.
    fn send<'py>(
        &self,
        py: Python<'py>,
        topic: String,
        payload: Vec<u8>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport.send(&topic, &payload).await.map_err(to_pyerr)
        })
    }

    /// Subscribes this link to a topic.
    fn subscribe<'py>(&self, py: Python<'py>, topic: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport.subscribe(&topic).await.map_err(to_pyerr)
        })
    }

    /// Waits for the next message on a subscribed topic, or `None` once closed.
    fn recv<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            let received = transport.recv().await.map_err(to_pyerr)?;
            Ok(received.map(|message| Message {
                topic: message.topic,
                payload: message.payload,
            }))
        })
    }

    /// Whether this link is connected.
    fn is_connected<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let transport = inner.lock().await;
            Ok(transport.is_connected())
        })
    }

    /// Marks this link disconnected, so sends over it fail.
    fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport.disconnect();
            Ok(())
        })
    }
}

/// Maps a core error onto the one Python sees.
fn to_pyerr(error: pamoja_core::Error) -> PyErr {
    PamojaError::new_err(error.to_string())
}
