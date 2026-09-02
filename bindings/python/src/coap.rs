//! Generated Python bindings for CoAP.
//!
//! These mirror the `pamoja-coap` Rust API. CoAP is the transport for links
//! where MQTT is more than the budget allows: it runs over UDP, its headers are
//! a handful of bytes, and a node can fire a reading and forget it rather than
//! holding a session open.

use std::sync::Arc;
use std::time::Duration;

use pamoja_coap::{CoapConfig, CoapTransport, Reliability};
use pamoja_core::Transport;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use tokio::sync::Mutex;

use crate::transport::Message;
use crate::PamojaError;

/// A CoAP endpoint.
#[gen_stub_pyclass]
#[pyclass]
pub struct CoapClient {
    inner: Arc<Mutex<CoapTransport>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl CoapClient {
    /// Creates a disconnected endpoint from the given settings.
    #[new]
    #[pyo3(signature = (*, host, port, bind=None, reliability=None, ack_timeout_ms=None, max_retransmits=None))]
    fn new(
        host: String,
        port: u16,
        bind: Option<String>,
        reliability: Option<String>,
        ack_timeout_ms: Option<u32>,
        max_retransmits: Option<u32>,
    ) -> PyResult<Self> {
        let config = settings(
            host,
            port,
            bind,
            reliability,
            ack_timeout_ms,
            max_retransmits,
        )?;
        Ok(Self {
            inner: Arc::new(Mutex::new(CoapTransport::new(config))),
        })
    }

    /// Binds the local socket so the endpoint can carry traffic.
    fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport.connect().await.map_err(to_pyerr)
        })
    }

    /// Sends a payload to a resource path.
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

    /// Observes a resource path, so messages published to it reach `recv`.
    fn subscribe<'py>(&self, py: Python<'py>, topic: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport.subscribe(&topic).await.map_err(to_pyerr)
        })
    }

    /// Waits for the next message on an observed path, or `None` once closed.
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

    /// Whether the local socket is bound.
    fn is_connected<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let transport = inner.lock().await;
            Ok(transport.is_connected())
        })
    }

    /// Releases the socket the endpoint holds.
    fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport.disconnect().await.map_err(to_pyerr)
        })
    }
}

/// Builds the endpoint settings from the keyword arguments a caller passed.
///
/// Shared with the composable transport, so an endpoint and a ladder rung read
/// the same fields the same way.
pub(crate) fn settings(
    host: String,
    port: u16,
    bind: Option<String>,
    reliability: Option<String>,
    ack_timeout_ms: Option<u32>,
    max_retransmits: Option<u32>,
) -> PyResult<CoapConfig> {
    let mut config = CoapConfig::new(host, port);
    if let Some(bind) = bind {
        config = config.bind(bind);
    }
    config = config.reliability(match reliability.as_deref() {
        None | Some("Confirmable") => Reliability::Confirmable,
        Some("NonConfirmable") => Reliability::NonConfirmable,
        Some(other) => {
            return Err(PamojaError::new_err(format!("unknown reliability {other}")));
        }
    });
    if let Some(millis) = ack_timeout_ms {
        config = config.ack_timeout(Duration::from_millis(u64::from(millis)));
    }
    if let Some(count) = max_retransmits {
        config = config.max_retransmits(count);
    }
    Ok(config)
}

/// Maps a core error onto the one Python sees.
fn to_pyerr(error: pamoja_core::Error) -> PyErr {
    PamojaError::new_err(error.to_string())
}
