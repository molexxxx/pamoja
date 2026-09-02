//! Generated Python bindings for composing transports.
//!
//! A ladder rung, a fault injector, and a degraded link all take "some
//! transport", which in Rust is any `impl Transport`. Python has no such
//! parameter, so this module carries one class that holds whichever transport
//! was built and dispatches to it.
//!
//! Composing consumes a transport, because the thing it is composed into owns it
//! from then on. A consumed transport is emptied rather than left aliasing what
//! now belongs to a ladder, so using one twice raises instead of quietly sharing
//! a link.

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use pamoja_core::{Result, Transport};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::PamojaError;

/// Object-safe erasure of a transport, so a wrapper can hold any of them.
///
/// The core trait returns `impl Future`, which is not dyn-compatible; this one
/// boxes the future so a wrapping kind can hold a transport without naming its
/// concrete type. That is what keeps the union below from naming itself: a
/// nested transport is reached through this trait, whose futures are already a
/// type the compiler can name.
trait DynTransport: Send {
    /// Connects the erased transport.
    fn connect(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;

    /// Sends a payload over the erased transport.
    fn send<'a>(
        &'a mut self,
        topic: &'a str,
        payload: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// Subscribes the erased transport to a topic.
    fn subscribe<'a>(
        &'a mut self,
        topic: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}

/// Newtype carrying one concrete transport behind [`DynTransport`].
struct Erased<T>(T);

impl<T: Transport + Send> DynTransport for Erased<T> {
    fn connect(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(Transport::connect(&mut self.0))
    }

    fn send<'a>(
        &'a mut self,
        topic: &'a str,
        payload: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(Transport::send(&mut self.0, topic, payload))
    }

    fn subscribe<'a>(
        &'a mut self,
        topic: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(Transport::subscribe(&mut self.0, topic))
    }
}

/// A transport of any kind, ready to be nested inside a wrapper.
pub(crate) struct AnyTransport(Box<dyn DynTransport>);

impl AnyTransport {
    /// Erases one transport so a wrapper can hold it.
    fn new(transport: Kind) -> Self {
        Self(Box::new(Erased(transport)))
    }
}

impl Transport for AnyTransport {
    async fn connect(&mut self) -> Result<()> {
        self.0.connect().await
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        self.0.send(topic, payload).await
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        self.0.subscribe(topic).await
    }
}

/// One transport, whichever kind it was built as.
///
/// A wrapping kind holds its inner transport erased rather than as this enum, so
/// a faulty link can wrap a degraded one to any depth without the enum naming
/// itself. Naming itself would make the hidden type of each method depend on
/// knowing that same type, which is a cycle rather than recursion.
pub(crate) enum Kind {
    /// An MQTT broker connection.
    #[cfg(feature = "mqtt")]
    Mqtt(pamoja_mqtt::MqttTransport),
    /// A CoAP endpoint.
    #[cfg(feature = "coap")]
    Coap(pamoja_coap::CoapTransport),
    /// An in-process link to a loopback broker.
    #[cfg(feature = "loopback")]
    Loopback(pamoja_loopback::LoopbackTransport),
    /// Another transport with a set number of sends made to fail.
    #[cfg(feature = "loopback")]
    Faulty(pamoja_loopback::Faulty<AnyTransport>),
    /// Another transport carrying loss and outages.
    #[cfg(feature = "sim")]
    Degraded(pamoja_sim::DegradedLink<AnyTransport>),
}

impl Transport for Kind {
    async fn connect(&mut self) -> Result<()> {
        match self {
            #[cfg(feature = "mqtt")]
            Kind::Mqtt(inner) => inner.connect().await,
            #[cfg(feature = "coap")]
            Kind::Coap(inner) => inner.connect().await,
            #[cfg(feature = "loopback")]
            Kind::Loopback(inner) => inner.connect().await,
            #[cfg(feature = "loopback")]
            Kind::Faulty(inner) => inner.connect().await,
            #[cfg(feature = "sim")]
            Kind::Degraded(inner) => inner.connect().await,
        }
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        match self {
            #[cfg(feature = "mqtt")]
            Kind::Mqtt(inner) => inner.send(topic, payload).await,
            #[cfg(feature = "coap")]
            Kind::Coap(inner) => inner.send(topic, payload).await,
            #[cfg(feature = "loopback")]
            Kind::Loopback(inner) => inner.send(topic, payload).await,
            #[cfg(feature = "loopback")]
            Kind::Faulty(inner) => inner.send(topic, payload).await,
            #[cfg(feature = "sim")]
            Kind::Degraded(inner) => inner.send(topic, payload).await,
        }
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        match self {
            #[cfg(feature = "mqtt")]
            Kind::Mqtt(inner) => inner.subscribe(topic).await,
            #[cfg(feature = "coap")]
            Kind::Coap(inner) => inner.subscribe(topic).await,
            #[cfg(feature = "loopback")]
            Kind::Loopback(inner) => inner.subscribe(topic).await,
            #[cfg(feature = "loopback")]
            Kind::Faulty(inner) => inner.subscribe(topic).await,
            #[cfg(feature = "sim")]
            Kind::Degraded(inner) => inner.subscribe(topic).await,
        }
    }
}

/// A message that arrived on a subscribed topic.
///
/// CoAP and the loopback broker both hand back a topic and a payload, so one
/// class serves them rather than a near-identical type per transport.
#[gen_stub_pyclass]
#[pyclass]
pub struct Message {
    /// The topic it was published to.
    #[pyo3(get)]
    pub(crate) topic: String,
    /// The raw payload bytes.
    #[pyo3(get)]
    pub(crate) payload: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Message {
    fn __repr__(&self) -> String {
        format!(
            "Message(topic={:?}, payload={} bytes)",
            self.topic,
            self.payload.len()
        )
    }
}

/// One transport, ready to compose into a ladder or a wrapper.
///
/// Build one with the module constructors, then hand it to whatever should own
/// it. A transport handed on is spent: using it afterwards raises.
#[gen_stub_pyclass]
#[pyclass]
pub struct PyTransport {
    inner: Mutex<Option<Kind>>,
}

impl PyTransport {
    /// Wraps a transport kind in the class Python holds.
    pub(crate) fn wrap(kind: Kind) -> Self {
        Self {
            inner: Mutex::new(Some(kind)),
        }
    }

    /// Takes the transport, leaving this handle spent.
    ///
    /// The transport is behind a lock so a shared reference can empty it, which
    /// is what Python hands a method, and so the class is `Sync` as a `pyclass`
    /// has to be.
    pub(crate) fn take(&self) -> PyResult<Kind> {
        self.inner
            .lock()
            .map_err(|_| PamojaError::new_err("this transport is poisoned"))?
            .take()
            .ok_or_else(|| {
                PamojaError::new_err("this transport was already added to a ladder or a wrapper")
            })
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyTransport {
    /// Creates an MQTT transport from broker settings.
    #[cfg(feature = "mqtt")]
    #[staticmethod]
    #[pyo3(signature = (*, client_id, host, port, keep_alive_secs=None, capacity=None, qos=None))]
    fn mqtt(
        client_id: String,
        host: String,
        port: u16,
        keep_alive_secs: Option<u32>,
        capacity: Option<u32>,
        qos: Option<String>,
    ) -> PyResult<Self> {
        let config = crate::mqtt::settings(client_id, host, port, keep_alive_secs, capacity, qos)?;
        Ok(Self::wrap(Kind::Mqtt(pamoja_mqtt::MqttTransport::new(
            config,
        ))))
    }

    /// Creates a CoAP transport from endpoint settings.
    #[cfg(feature = "coap")]
    #[staticmethod]
    #[pyo3(signature = (*, host, port, bind=None, reliability=None, ack_timeout_ms=None, max_retransmits=None))]
    fn coap(
        host: String,
        port: u16,
        bind: Option<String>,
        reliability: Option<String>,
        ack_timeout_ms: Option<u32>,
        max_retransmits: Option<u32>,
    ) -> PyResult<Self> {
        let config = crate::coap::settings(
            host,
            port,
            bind,
            reliability,
            ack_timeout_ms,
            max_retransmits,
        )?;
        Ok(Self::wrap(Kind::Coap(pamoja_coap::CoapTransport::new(
            config,
        ))))
    }

    /// Wraps a transport so its next `failures` sends fail.
    ///
    /// This is how a caller checks that a ladder falls through to its next rung,
    /// or that a buffer fills, without unplugging anything. The wrapped
    /// transport is consumed.
    #[cfg(feature = "loopback")]
    #[staticmethod]
    fn faulty(inner: &PyTransport, failures: usize) -> PyResult<Self> {
        Ok(Self::wrap(Kind::Faulty(pamoja_loopback::Faulty::new(
            AnyTransport::new(inner.take()?),
            failures,
        ))))
    }

    /// Wraps a transport in a link that loses packets and goes down.
    ///
    /// The wrapped transport is consumed.
    #[cfg(feature = "sim")]
    #[staticmethod]
    #[pyo3(signature = (inner, drop_every = 0, up = 0, down = 0))]
    fn degraded(inner: &PyTransport, drop_every: u32, up: u32, down: u32) -> PyResult<Self> {
        let mut link = pamoja_sim::DegradedLink::new(AnyTransport::new(inner.take()?));
        if drop_every != 0 {
            link = link.drop_every(drop_every);
        }
        if up != 0 {
            link = link.intermittent(up, down);
        }
        Ok(Self::wrap(Kind::Degraded(link)))
    }

    /// Whether this transport is still holdable, or has been handed on.
    #[getter]
    fn is_available(&self) -> bool {
        self.inner
            .lock()
            .map(|held| held.is_some())
            .unwrap_or(false)
    }
}
