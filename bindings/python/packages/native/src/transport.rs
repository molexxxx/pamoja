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
//!
//! A link written in Python enters the same way: [`PyTransport::from_handlers`]
//! wraps an object whose methods connect, send, subscribe, and receive, so a
//! vendor SDK or a class of the caller's own composes like a transport pamoja
//! ships.

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use pamoja_core::{Error, Message as CoreMessage, Receive, Result, Transport};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyTuple};
use pyo3_async_runtimes::TaskLocals;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

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

    /// Awaits the next message the erased transport delivers.
    fn recv(&mut self) -> Pin<Box<dyn Future<Output = Result<Option<CoreMessage>>> + Send + '_>>;
}

/// Newtype carrying one concrete transport behind [`DynTransport`].
struct Erased<T>(T);

impl<T: Transport + Receive + Send> DynTransport for Erased<T> {
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

    fn recv(&mut self) -> Pin<Box<dyn Future<Output = Result<Option<CoreMessage>>> + Send + '_>> {
        Box::pin(Receive::recv(&mut self.0))
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

impl Receive for AnyTransport {
    async fn recv(&mut self) -> Result<Option<CoreMessage>> {
        self.0.recv().await
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
    /// A link whose operations are the methods of a Python object.
    Host(HostTransport),
}

impl Kind {
    /// Whether the transport delivers messages, so a ladder listens on it rather
    /// than treating it as an uplink.
    pub(crate) fn listens(&self) -> bool {
        match self {
            Kind::Host(inner) => inner.listens,
            _ => true,
        }
    }
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
            Kind::Host(inner) => inner.connect().await,
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
            Kind::Host(inner) => inner.send(topic, payload).await,
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
            Kind::Host(inner) => inner.subscribe(topic).await,
        }
    }
}

impl Receive for Kind {
    async fn recv(&mut self) -> Result<Option<CoreMessage>> {
        match self {
            #[cfg(feature = "mqtt")]
            Kind::Mqtt(inner) => inner.recv().await,
            #[cfg(feature = "coap")]
            Kind::Coap(inner) => inner.recv().await,
            #[cfg(feature = "loopback")]
            Kind::Loopback(inner) => inner.recv().await,
            #[cfg(feature = "loopback")]
            Kind::Faulty(inner) => inner.recv().await,
            #[cfg(feature = "sim")]
            Kind::Degraded(inner) => inner.recv().await,
            Kind::Host(inner) => inner.recv().await,
        }
    }
}

/// A transport whose operations are the methods of a Python object.
///
/// Each call runs the method under the interpreter lock and, when the method
/// returned an awaitable, awaits it on the event loop the ladder was driven from.
/// A host with a `recv` method is asked for messages from a task that starts at
/// `connect` and calls it again as soon as it returns, queueing what it delivers,
/// so a receive through pamoja is cancel-safe whatever the method does.
pub(crate) struct HostTransport {
    handlers: Py<PyAny>,
    listens: bool,
    connected: bool,
    inbox: Option<mpsc::UnboundedReceiver<CoreMessage>>,
    pump: Option<JoinHandle<()>>,
}

impl HostTransport {
    /// Wraps `handlers`, refusing an object that lacks a required method.
    fn new(handlers: &Bound<'_, PyAny>) -> PyResult<Self> {
        for name in ["connect", "send", "subscribe"] {
            if !handlers.hasattr(name)? {
                return Err(PamojaError::new_err(format!(
                    "a transport handler needs a {name} method"
                )));
            }
        }
        Ok(Self {
            listens: handlers.hasattr("recv")?,
            handlers: handlers.clone().unbind(),
            connected: false,
            inbox: None,
            pump: None,
        })
    }

    /// Starts the task that asks the host for messages until it answers `None`.
    fn start_pump(&mut self, locals: TaskLocals) {
        if !self.listens {
            return;
        }
        if let Some(pump) = self.pump.take() {
            pump.abort();
        }
        let handlers = Python::attach(|py| self.handlers.clone_ref(py));
        let (sender, receiver) = mpsc::unbounded_channel();
        self.inbox = Some(receiver);
        self.pump = Some(tokio::spawn(async move {
            loop {
                let Ok(Some(message)) = receive(&handlers, &locals).await else {
                    break;
                };
                if sender.send(message).is_err() {
                    break;
                }
            }
        }));
    }
}

impl Drop for HostTransport {
    fn drop(&mut self) {
        if let Some(pump) = self.pump.take() {
            pump.abort();
        }
    }
}

/// The task locals the current call was made under, which is where the host's
/// coroutines are scheduled.
fn current_locals() -> Result<TaskLocals> {
    Python::attach(pyo3_async_runtimes::tokio::get_current_locals).map_err(host_error)
}

/// Calls a handler method with `args` and awaits its result if it is awaitable.
async fn call(
    handlers: &Py<PyAny>,
    locals: &TaskLocals,
    name: &str,
    args: impl for<'py> FnOnce(Python<'py>) -> PyResult<Bound<'py, PyTuple>> + Send,
) -> PyResult<Py<PyAny>> {
    let pending = Python::attach(|py| -> PyResult<_> {
        let result = handlers.bind(py).call_method1(name, args(py)?)?;
        if result.hasattr("__await__")? {
            Ok(Err(pyo3_async_runtimes::into_future_with_locals(
                locals, result,
            )?))
        } else {
            Ok(Ok(result.unbind()))
        }
    })?;
    match pending {
        Ok(value) => Ok(value),
        Err(future) => future.await,
    }
}

/// Asks the host for its next message and reads the answer: a [`Message`], a
/// `(topic, payload)` pair, or `None` once the link has ended.
async fn receive(handlers: &Py<PyAny>, locals: &TaskLocals) -> PyResult<Option<CoreMessage>> {
    let value = call(handlers, locals, "recv", |py| Ok(PyTuple::empty(py))).await?;
    Python::attach(|py| {
        let value = value.bind(py);
        if value.is_none() {
            return Ok(None);
        }
        if let Ok(message) = value.extract::<PyRef<'_, Message>>() {
            return Ok(Some(CoreMessage::new(
                message.topic.clone(),
                message.payload.clone(),
            )));
        }
        let (topic, payload): (String, Vec<u8>) = value.extract().map_err(|_| {
            PamojaError::new_err("recv must return a Message, a (topic, payload) pair, or None")
        })?;
        Ok(Some(CoreMessage::new(topic, payload)))
    })
}

/// Maps a Python exception onto the shared transport error.
fn host_error(error: PyErr) -> Error {
    Error::Transport(error.to_string())
}

impl Transport for HostTransport {
    async fn connect(&mut self) -> Result<()> {
        let locals = current_locals()?;
        call(&self.handlers, &locals, "connect", |py| {
            Ok(PyTuple::empty(py))
        })
        .await
        .map_err(host_error)?;
        self.connected = true;
        self.start_pump(locals);
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let locals = current_locals()?;
        let topic = topic.to_owned();
        let payload = payload.to_vec();
        call(&self.handlers, &locals, "send", move |py| {
            PyTuple::new(
                py,
                [
                    topic.into_pyobject(py)?.into_any(),
                    PyBytes::new(py, &payload).into_any(),
                ],
            )
        })
        .await
        .map(|_| ())
        .map_err(host_error)
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let locals = current_locals()?;
        let topic = topic.to_owned();
        call(&self.handlers, &locals, "subscribe", move |py| {
            PyTuple::new(py, [topic.into_pyobject(py)?.into_any()])
        })
        .await
        .map(|_| ())
        .map_err(host_error)
    }
}

impl Receive for HostTransport {
    async fn recv(&mut self) -> Result<Option<CoreMessage>> {
        if !self.connected {
            return Err(Error::Closed);
        }
        match self.inbox.as_mut() {
            Some(inbox) => Ok(inbox.recv().await),
            None => Ok(None),
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
    /// Creates a message, which is what a transport handler returns from `recv`.
    ///
    /// The payload is bytes, or text such as a reading written out.
    #[new]
    fn new(topic: String, payload: Payload) -> Self {
        Self {
            topic,
            payload: payload.into_bytes(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Message(topic={:?}, payload={} bytes)",
            self.topic,
            self.payload.len()
        )
    }

    /// The payload as text: words, or a number written out.
    ///
    /// Raises `ValueError` if the payload is not UTF-8 text.
    #[getter]
    fn text(&self) -> PyResult<String> {
        text_of(&self.payload)
    }

    /// The payload as a number written out as text, such as `21.5`.
    ///
    /// Raises `ValueError` if the payload is not text or the text is not a number.
    #[getter]
    fn number(&self) -> PyResult<f64> {
        number_of(&self.payload)
    }
}

/// Reads a payload as text.
pub(crate) fn text_of(payload: &[u8]) -> PyResult<String> {
    std::str::from_utf8(payload)
        .map(str::to_owned)
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("the payload is not UTF-8 text"))
}

/// Reads a payload as a number written out as text.
pub(crate) fn number_of(payload: &[u8]) -> PyResult<f64> {
    text_of(payload)?
        .trim()
        .parse()
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("the payload is not a number"))
}

/// A payload to send: text such as a reading written out, or raw bytes.
#[derive(FromPyObject)]
pub(crate) enum Payload {
    /// Words, or a number written out, sent as UTF-8.
    Text(String),
    /// Raw bytes.
    Bytes(Vec<u8>),
}

pyo3_stub_gen::impl_stub_type!(Payload = String | Vec<u8>);

impl Payload {
    /// The bytes that go on the wire.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        match self {
            Payload::Text(text) => text.into_bytes(),
            Payload::Bytes(bytes) => bytes,
        }
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

    /// Wraps an object whose methods are the link.
    ///
    /// `handlers` needs `connect()`, `send(topic, payload)`, and `subscribe(topic)`,
    /// each a coroutine function or a plain one. A `recv()` that returns a
    /// `Message`, a `(topic, payload)` pair, or `None` once the link has ended makes
    /// it a link that delivers: it is called again as soon as it returns, from the
    /// moment the transport connects. Without `recv` the transport only sends, and
    /// a ladder never listens on it.
    #[staticmethod]
    fn from_handlers(handlers: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::wrap(Kind::Host(HostTransport::new(handlers)?)))
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
