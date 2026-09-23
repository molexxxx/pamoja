//! Generated Python bindings for CoAP.
//!
//! These mirror the `pamoja-coap` Rust API. CoAP is the transport for links
//! where MQTT is more than the budget allows: it runs over UDP, its headers are
//! a handful of bytes, and a node can fire a reading and forget it rather than
//! holding a session open.

use std::sync::Arc;
use std::time::Duration;

use pamoja_coap::{CoapConfig, CoapPublisher, CoapTransport, Reliability};
use pamoja_core::{Receive, Transport};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use tokio::sync::Mutex;

use crate::transport::Message;
use crate::transport::Payload;
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
        payload: Payload,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport
                .send(&topic, &payload.into_bytes())
                .await
                .map_err(to_pyerr)
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

/// A CoAP server: the end that nodes send their readings to and observe their
/// commands on.
///
/// A PUT or POST to a path matching one of its `subscribe` filters is answered
/// 2.04 Changed and delivered to `recv`, and one to any other path 4.04 Not Found.
/// `send` sets a resource's state, which a GET reads and every observer is
/// notified of. `send` does not wait for a `recv` that is waiting, so a gateway
/// sends commands while it listens for readings.
#[gen_stub_pyclass]
#[pyclass]
pub struct CoapServer {
    server: Arc<Mutex<pamoja_coap::CoapServer>>,
    publisher: CoapPublisher,
}

#[gen_stub_pymethods]
#[pymethods]
impl CoapServer {
    /// Creates a server that will listen on a local address, such as `0.0.0.0:5683`,
    /// or port 0 for a free one.
    #[new]
    fn new(bind: String) -> Self {
        let server = pamoja_coap::CoapServer::new(bind);
        let publisher = server.publisher();
        Self {
            server: Arc::new(Mutex::new(server)),
            publisher,
        }
    }

    /// Binds the socket and starts answering requests.
    fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.server);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut server = inner.lock().await;
            server.connect().await.map_err(to_pyerr)
        })
    }

    /// Takes the readings sent to the paths a filter matches, with `+` for one
    /// level and `#` for the rest.
    fn subscribe<'py>(&self, py: Python<'py>, filter: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.server);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut server = inner.lock().await;
            server.subscribe(&filter).await.map_err(to_pyerr)
        })
    }

    /// Sets a resource's state and notifies every observer of it.
    fn send<'py>(
        &self,
        py: Python<'py>,
        path: String,
        payload: Payload,
    ) -> PyResult<Bound<'py, PyAny>> {
        let publisher = self.publisher.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            publisher
                .publish(&path, &payload.into_bytes())
                .await
                .map_err(to_pyerr)
        })
    }

    /// Waits for the next reading sent to a path the server takes.
    fn recv<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.server);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut server = inner.lock().await;
            let received = server.recv().await.map_err(to_pyerr)?;
            Ok(received.map(|message| Message {
                topic: message.topic,
                payload: message.payload,
            }))
        })
    }

    /// Counts the clients observing a resource.
    fn observers(&self, path: &str) -> usize {
        self.publisher.observers(path)
    }

    /// The port the server listens on, which names the one the system chose for
    /// port 0, or `None` while it is not connected.
    #[getter]
    fn local_port(&self) -> Option<u16> {
        self.publisher.local_addr().map(|address| address.port())
    }

    /// Whether the server holds a bound socket.
    #[getter]
    fn is_connected(&self) -> bool {
        self.publisher.is_connected()
    }

    /// Closes the socket, keeping the resources and filters.
    fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.server);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut server = inner.lock().await;
            server.disconnect().await.map_err(to_pyerr)
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
