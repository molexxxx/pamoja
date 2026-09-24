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
use pamoja_mqtt::{Inbox, MqttConfig, MqttTransport, PublishOptions, QualityOfService, Tls, Will};

use crate::transport::Payload;
use crate::PamojaError;

/// A message the broker publishes on the client's behalf if its connection ends without a
/// disconnect: the network dropped, the power failed, or the keep-alive ran out. A client
/// that disconnects leaves no will behind.
#[gen_stub_pyclass]
#[pyclass(frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct MqttWill {
    /// The topic the broker publishes it to.
    #[pyo3(get)]
    topic: String,
    payload: Vec<u8>,
    /// The quality of service it is published at, by name.
    #[pyo3(get)]
    qos: String,
    /// Whether the broker retains it for clients that subscribe later.
    #[pyo3(get)]
    retain: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl MqttWill {
    /// Creates a will to publish `payload` to `topic`, which holds no wildcard. `qos` is
    /// `"AtMostOnce"` unless given.
    #[new]
    #[pyo3(signature = (topic, payload, *, qos = None, retain = false))]
    fn new(topic: String, payload: Payload, qos: Option<String>, retain: bool) -> PyResult<Self> {
        let qos = qos.unwrap_or_else(|| "AtMostOnce".to_owned());
        parse_qos(&qos)?;
        Ok(Self {
            topic,
            payload: payload.into_bytes(),
            qos,
            retain,
        })
    }

    /// What the will publishes.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.payload)
    }

    fn __repr__(&self) -> String {
        format!(
            "MqttWill(topic={:?}, payload=<{} bytes>, qos={:?}, retain={})",
            self.topic,
            self.payload.len(),
            self.qos,
            if self.retain { "True" } else { "False" }
        )
    }
}

/// How a connection is secured with TLS, conventionally on port 8883.
#[gen_stub_pyclass]
#[pyclass(frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct MqttTls {
    ca_pem: Option<Vec<u8>>,
    identity: Option<(Vec<u8>, Vec<u8>)>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MqttTls {
    /// Trusts the certificate authorities in `ca_pem`, or the system's without it, and
    /// presents `certificate_pem` with its `key_pem` to a broker that asks for a client
    /// certificate. Raises `PamojaError` when only one of the pair is given.
    #[new]
    #[pyo3(signature = (*, ca_pem = None, certificate_pem = None, key_pem = None))]
    fn new(
        ca_pem: Option<Payload>,
        certificate_pem: Option<Payload>,
        key_pem: Option<Payload>,
    ) -> PyResult<Self> {
        let identity = match (certificate_pem, key_pem) {
            (Some(certificate), Some(key)) => Some((certificate.into_bytes(), key.into_bytes())),
            (None, None) => None,
            _ => {
                return Err(PamojaError::new_err(
                    "a client certificate and its key come together",
                ))
            }
        };
        Ok(Self {
            ca_pem: ca_pem.map(Payload::into_bytes),
            identity,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "MqttTls(ca_pem={}, client_certificate={})",
            if self.ca_pem.is_some() {
                "<given>"
            } else {
                "None"
            },
            if self.identity.is_some() {
                "True"
            } else {
                "False"
            }
        )
    }
}

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
    /// The payload as text: words, or a number written out.
    ///
    /// Raises `ValueError` if the payload is not UTF-8 text.
    #[getter]
    fn text(&self) -> PyResult<String> {
        crate::transport::text_of(&self.payload)
    }

    /// The payload as a number written out as text, such as `21.5`.
    ///
    /// Raises `ValueError` if the payload is not text or the text is not a number.
    #[getter]
    fn number(&self) -> PyResult<f64> {
        crate::transport::number_of(&self.payload)
    }

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
    inbox: Inbox,
}

#[gen_stub_pymethods]
#[pymethods]
impl MqttClient {
    /// Creates a disconnected client from the given options.
    ///
    /// `max_packet_size` is the largest packet the connection sends or accepts, in
    /// bytes, 10,240 when omitted. A publish that would be larger is refused and the
    /// connection stays up, but a larger packet arriving from the broker ends it.
    /// `username` and `password` sign in to the broker, and a password needs a username;
    /// `will` is published if the connection ends without a goodbye, and `tls` secures the
    /// connection.
    #[new]
    #[pyo3(signature = (*, client_id, host, port, keep_alive_secs=None, capacity=None, qos=None, max_packet_size=None, username=None, password=None, will=None, tls=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        client_id: String,
        host: String,
        port: u16,
        keep_alive_secs: Option<u32>,
        capacity: Option<u32>,
        qos: Option<String>,
        max_packet_size: Option<u32>,
        username: Option<String>,
        password: Option<String>,
        will: Option<PyRef<'_, MqttWill>>,
        tls: Option<PyRef<'_, MqttTls>>,
    ) -> PyResult<Self> {
        let transport = MqttTransport::new(settings(Broker {
            client_id,
            host,
            port,
            keep_alive_secs,
            capacity,
            qos,
            max_packet_size,
            username,
            password,
            will: will.map(|will| will.clone()),
            tls: tls.map(|tls| tls.clone()),
        })?);
        Ok(Self {
            inbox: transport.inbox(),
            inner: Arc::new(Mutex::new(transport)),
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

    /// Publishes a payload to a topic, returning once it is queued for the broker.
    /// `qos` is the client's unless given, and `retain` has the broker keep the message
    /// for clients that subscribe later; an empty retained message clears the one it holds.
    #[pyo3(signature = (topic, payload, *, qos = None, retain = false))]
    fn publish<'py>(
        &self,
        py: Python<'py>,
        topic: String,
        payload: Payload,
        qos: Option<String>,
        retain: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        let options = publish_options(qos, retain)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut transport = inner.lock().await;
            transport
                .publish(&topic, &payload.into_bytes(), options)
                .await
                .map(drop)
                .map_err(to_pyerr)
        })
    }

    /// Publishes a payload to a topic and returns once the broker acknowledges it: its
    /// `PUBACK` at `"AtLeastOnce"`, its `PUBCOMP` at `"ExactlyOnce"`, and once the
    /// connection has taken it at `"AtMostOnce"`, where MQTT acknowledges nothing. Raises
    /// `PamojaError` if the connection ends first, when the message may or may not have
    /// arrived.
    #[pyo3(signature = (topic, payload, *, qos = None, retain = false))]
    fn publish_confirmed<'py>(
        &self,
        py: Python<'py>,
        topic: String,
        payload: Payload,
        qos: Option<String>,
        retain: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        let options = publish_options(qos, retain)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let delivery = {
                let mut transport = inner.lock().await;
                transport
                    .publish(&topic, &payload.into_bytes(), options)
                    .await
                    .map_err(to_pyerr)?
            };
            delivery.confirmed().await.map_err(to_pyerr)
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
    /// connection has ended. A connection that ends on its own raises `PamojaError`
    /// once, saying why.
    fn recv<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inbox = self.inbox.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let message = inbox.recv().await.map_err(to_pyerr)?;
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

/// Reads how one message is published.
fn publish_options(qos: Option<String>, retain: bool) -> PyResult<PublishOptions> {
    let mut options = PublishOptions::new();
    if let Some(qos) = qos {
        options = options.qos(parse_qos(&qos)?);
    }
    if retain {
        options = options.retained();
    }
    Ok(options)
}

/// The broker settings a client or a transport is built from, as the keyword arguments
/// named them.
pub(crate) struct Broker {
    pub(crate) client_id: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) keep_alive_secs: Option<u32>,
    pub(crate) capacity: Option<u32>,
    pub(crate) qos: Option<String>,
    pub(crate) max_packet_size: Option<u32>,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) will: Option<MqttWill>,
    pub(crate) tls: Option<MqttTls>,
}

/// Builds the broker settings from the keyword arguments a caller passed.
///
/// Shared with the composable transport, so a client and a ladder rung read the
/// same fields the same way.
pub(crate) fn settings(broker: Broker) -> PyResult<MqttConfig> {
    let mut config = MqttConfig::new(broker.client_id, broker.host, broker.port);
    if let Some(secs) = broker.keep_alive_secs {
        config = config.keep_alive(Duration::from_secs(u64::from(secs)));
    }
    if let Some(capacity) = broker.capacity {
        config = config.capacity(capacity as usize);
    }
    if let Some(qos) = broker.qos {
        config = config.qos(parse_qos(&qos)?);
    }
    if let Some(bytes) = broker.max_packet_size {
        config = config.max_packet_size(bytes as usize);
    }
    match (broker.username, broker.password) {
        (Some(username), password) => {
            config = config.credentials(username, password.unwrap_or_default());
        }
        (None, Some(_)) => {
            return Err(PamojaError::new_err(
                "a password needs a username: MQTT sends no password alone",
            ))
        }
        (None, None) => {}
    }
    if let Some(will) = broker.will {
        let mut built = Will::new(will.topic, will.payload).qos(parse_qos(&will.qos)?);
        if will.retain {
            built = built.retained();
        }
        config = config.last_will(built);
    }
    if let Some(tls) = broker.tls {
        let mut built = match tls.ca_pem {
            Some(pem) => Tls::with_ca_pem(pem),
            None => Tls::system_roots(),
        };
        if let Some((certificate, key)) = tls.identity {
            built = built.client_certificate(certificate, key);
        }
        config = config.tls(built);
    }
    Ok(config)
}
