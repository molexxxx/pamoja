//! Generated Python bindings for the MQTT transport.
//!
//! These mirror the `pamoja-mqtt` Rust API one-to-one. The shared state lives
//! behind an async mutex so the awaitable methods own a clonable handle rather
//! than borrowing the Python object across an `await`.

use std::sync::Arc;
use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use tokio::sync::Mutex;

use pamoja_core::{Error, Transport};
use pamoja_mqtt::{MqttConfig, MqttTransport, QualityOfService};

use crate::PamojaError;

/// A message received from a subscribed topic.
#[gen_stub_pyclass]
#[pyclass]
pub struct MqttMessage {
    /// The topic the message was published to.
    #[pyo3(get)]
    topic: String,
    payload: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MqttMessage {
    /// The raw payload bytes.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.payload)
    }

    fn __repr__(&self) -> String {
        format!(
            "MqttMessage(topic={:?}, payload=<{} bytes>)",
            self.topic,
            self.payload.len()
        )
    }
}

/// An MQTT client transport backed by the native pamoja core.
#[gen_stub_pyclass]
#[pyclass]
pub struct MqttClient {
    inner: Arc<Mutex<MqttTransport>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MqttClient {
    /// Creates a disconnected client from the given options.
    #[new]
    #[pyo3(signature = (*, client_id, host, port, keep_alive_secs=None, capacity=None, qos=None))]
    fn new(
        client_id: String,
        host: String,
        port: u16,
        keep_alive_secs: Option<u32>,
        capacity: Option<u32>,
        qos: Option<String>,
    ) -> PyResult<Self> {
        let config = settings(client_id, host, port, keep_alive_secs, capacity, qos)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(MqttTransport::new(config))),
        })
    }

    /// Connects to the broker and starts the background event loop.
    fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport.connect().await.map_err(to_pyerr)
        })
    }

    /// Publishes a payload to a topic.
    fn publish<'py>(
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

    /// Subscribes to a topic filter.
    fn subscribe<'py>(&self, py: Python<'py>, topic: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport.subscribe(&topic).await.map_err(to_pyerr)
        })
    }

    /// Awaits the next message from any subscribed topic, or `None` once the
    /// connection has ended.
    fn recv<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            let message = transport.recv().await.map_err(to_pyerr)?;
            Ok(message.map(|message| MqttMessage {
                topic: message.topic,
                payload: message.payload,
            }))
        })
    }

    /// Reports whether the client currently holds an active connection.
    fn is_connected<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let transport = inner.lock().await;
            Ok(transport.is_connected())
        })
    }

    /// Closes the connection and stops the background event loop.
    fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport.disconnect().await.map_err(to_pyerr)
        })
    }
}

/// Maps a quality-of-service name onto the core enum.
fn parse_qos(value: &str) -> PyResult<QualityOfService> {
    match value {
        "AtMostOnce" => Ok(QualityOfService::AtMostOnce),
        "AtLeastOnce" => Ok(QualityOfService::AtLeastOnce),
        "ExactlyOnce" => Ok(QualityOfService::ExactlyOnce),
        other => Err(PamojaError::new_err(format!(
            "unknown quality of service: {other}"
        ))),
    }
}

/// Maps a core error onto a `PamojaError` so it surfaces as a Python exception.
fn to_pyerr(err: Error) -> PyErr {
    PamojaError::new_err(err.to_string())
}

/// Builds the broker settings from the keyword arguments a caller passed.
///
/// Shared with the composable transport, so a client and a ladder rung read the
/// same fields the same way.
pub(crate) fn settings(
    client_id: String,
    host: String,
    port: u16,
    keep_alive_secs: Option<u32>,
    capacity: Option<u32>,
    qos: Option<String>,
) -> PyResult<MqttConfig> {
    let mut config = MqttConfig::new(client_id, host, port);
    if let Some(secs) = keep_alive_secs {
        config = config.keep_alive(Duration::from_secs(u64::from(secs)));
    }
    if let Some(capacity) = capacity {
        config = config.capacity(capacity as usize);
    }
    if let Some(qos) = qos {
        config = config.qos(parse_qos(&qos)?);
    }
    Ok(config)
}
