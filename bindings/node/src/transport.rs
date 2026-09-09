//! Generated Node bindings for composing transports.
//!
//! A ladder rung, a fault injector, and a degraded link all take "some
//! transport", which in Rust is any `impl Transport`. JavaScript has no such
//! parameter, so this module carries one class that holds whichever transport
//! was built and dispatches to it.
//!
//! Composing consumes a transport, because the thing it is composed into owns it
//! from then on. A consumed transport is emptied rather than left aliasing what
//! now belongs to a ladder, so using one twice throws instead of quietly sharing
//! a link.
//!
//! A link written in JavaScript enters the same way: [`Transport::from_handlers`]
//! wraps an object whose methods connect, send, subscribe, and receive, so a
//! vendor SDK or a class of the caller's own composes like a transport pamoja
//! ships.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use napi::bindgen_prelude::{
    Buffer, FnArgs, FromNapiValue, Function, JsValuesTupleIntoVec, Object, Promise, ToNapiValue,
    Unknown,
};
use napi::threadsafe_function::ThreadsafeFunction;
use napi::{sys, Error as NapiError, JsValue, Status, ValueType};
use napi_derive::napi;
use pamoja_core::{Error, Message, Receive, Result, Transport as CoreTransport};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

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
    fn recv(&mut self) -> Pin<Box<dyn Future<Output = Result<Option<Message>>> + Send + '_>>;
}

/// Newtype carrying one concrete transport behind [`DynTransport`].
struct Erased<T>(T);

impl<T: CoreTransport + Receive + Send> DynTransport for Erased<T> {
    fn connect(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(CoreTransport::connect(&mut self.0))
    }

    fn send<'a>(
        &'a mut self,
        topic: &'a str,
        payload: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(CoreTransport::send(&mut self.0, topic, payload))
    }

    fn subscribe<'a>(
        &'a mut self,
        topic: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(CoreTransport::subscribe(&mut self.0, topic))
    }

    fn recv(&mut self) -> Pin<Box<dyn Future<Output = Result<Option<Message>>> + Send + '_>> {
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

impl CoreTransport for AnyTransport {
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
    async fn recv(&mut self) -> Result<Option<Message>> {
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
    /// A link whose operations are the methods of a JavaScript object.
    Host(HostTransport),
}

impl Kind {
    /// Whether the transport delivers messages, so a ladder listens on it rather
    /// than treating it as an uplink.
    pub(crate) fn listens(&self) -> bool {
        match self {
            Kind::Host(inner) => inner.recv.is_some(),
            _ => true,
        }
    }
}

impl CoreTransport for Kind {
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
    async fn recv(&mut self) -> Result<Option<Message>> {
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

/// No arguments for a handler: a zero-sized value the call turns into none.
struct NoArgs;

impl ToNapiValue for NoArgs {
    unsafe fn to_napi_value(env: sys::napi_env, _: Self) -> napi::Result<sys::napi_value> {
        let mut undefined = std::ptr::null_mut();
        napi::check_status!(
            unsafe { sys::napi_get_undefined(env, &mut undefined) },
            "Get undefined value failed"
        )?;
        Ok(undefined)
    }
}

/// A handler result that is not looked at, whatever it was.
struct Ignored;

impl FromNapiValue for Ignored {
    unsafe fn from_napi_value(_: sys::napi_env, _: sys::napi_value) -> napi::Result<Self> {
        Ok(Ignored)
    }
}

/// What a handler returned: a promise still to settle, or the value itself, so a
/// plain method and an `async` one are both accepted.
struct Settled<T: FromNapiValue + 'static>(SettledInner<T>);

enum SettledInner<T: FromNapiValue + 'static> {
    Now(Option<T>),
    Later(Promise<T>),
}

impl<T: FromNapiValue + 'static> FromNapiValue for Settled<T> {
    unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> napi::Result<Self> {
        let mut is_promise = false;
        napi::check_status!(
            unsafe { sys::napi_is_promise(env, value, &mut is_promise) },
            "Check for a promise failed"
        )?;
        Ok(Self(if is_promise {
            SettledInner::Later(unsafe { Promise::from_napi_value(env, value)? })
        } else {
            SettledInner::Now(Some(unsafe { T::from_napi_value(env, value)? }))
        }))
    }
}

impl<T: FromNapiValue + Unpin + 'static> Future for Settled<T> {
    type Output = napi::Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut self.get_mut().0 {
            SettledInner::Now(value) => Poll::Ready(value.take().ok_or_else(|| {
                NapiError::from_reason("a handler result was polled after it completed")
            })),
            SettledInner::Later(promise) => Pin::new(promise).poll(cx),
        }
    }
}

/// A handler bound to its object, callable from any thread, and not holding the
/// event loop open on its own.
type Handler<Args, T> = ThreadsafeFunction<Args, Settled<T>, Args, Status, false, true>;

/// Takes the method `name` off `handlers`, bound to it, as a threadsafe function.
fn handler<Args, T>(
    handlers: &Object,
    name: &str,
    required: bool,
) -> napi::Result<Option<Handler<Args, T>>>
where
    Args: JsValuesTupleIntoVec + 'static,
    T: FromNapiValue + 'static,
{
    let method = handlers.get::<Unknown>(name)?;
    let is_function = match &method {
        Some(method) => method.get_type()? == ValueType::Function,
        None => false,
    };
    let Some(method) = method.filter(|_| is_function) else {
        if required || method.is_some() {
            return Err(NapiError::from_reason(format!(
                "a transport handler needs a {name} method"
            )));
        }
        return Ok(None);
    };
    let method = method.coerce_to_object()?;
    let bind: Function<'_, Object<'_>, Function<'_, Args, Settled<T>>> = method
        .get("bind")?
        .ok_or_else(|| NapiError::from_reason(format!("{name} cannot be bound")))?;
    let bound = bind.apply(method, *handlers)?;
    Ok(Some(
        bound
            .build_threadsafe_function::<Args>()
            .weak::<true>()
            .build()?,
    ))
}

/// Maps a JavaScript exception or a rejected promise onto the shared transport error.
fn host_error(error: NapiError) -> Error {
    if error.reason.is_empty() {
        Error::Transport(format!("the handler failed with {:?}", error.status))
    } else {
        Error::Transport(error.reason)
    }
}

/// A transport whose operations are the methods of a JavaScript object.
///
/// Each call is made on the event loop through a threadsafe function and awaited
/// if it returned a promise. A host with a `recv` method is asked for messages from
/// a task that starts at `connect` and calls it again as soon as it settles,
/// queueing what it delivers, so a receive through pamoja is cancel-safe whatever
/// the method does.
pub(crate) struct HostTransport {
    connect: Handler<NoArgs, Ignored>,
    send: Handler<FnArgs<(String, Buffer)>, Ignored>,
    subscribe: Handler<String, Ignored>,
    recv: Option<Arc<Handler<NoArgs, Option<TransportMessage>>>>,
    connected: bool,
    inbox: Option<mpsc::UnboundedReceiver<Message>>,
    pump: Option<JoinHandle<()>>,
}

impl HostTransport {
    /// Wraps `handlers`, refusing an object that lacks a required method.
    fn new(handlers: &Object) -> napi::Result<Self> {
        Ok(Self {
            connect: handler(handlers, "connect", true)?.expect("required"),
            send: handler(handlers, "send", true)?.expect("required"),
            subscribe: handler(handlers, "subscribe", true)?.expect("required"),
            recv: handler(handlers, "recv", false)?.map(Arc::new),
            connected: false,
            inbox: None,
            pump: None,
        })
    }

    /// Starts the task that asks the host for messages until it answers `null`.
    fn start_pump(&mut self) {
        let Some(recv) = self.recv.as_ref().map(Arc::clone) else {
            return;
        };
        if let Some(pump) = self.pump.take() {
            pump.abort();
        }
        let (sender, receiver) = mpsc::unbounded_channel();
        self.inbox = Some(receiver);
        self.pump = Some(tokio::spawn(async move {
            loop {
                let next = match recv.call_async(NoArgs).await {
                    Ok(settled) => settled.await,
                    Err(error) => Err(error),
                };
                let Ok(Some(message)) = next else {
                    break;
                };
                let message = Message::new(message.topic, message.payload.to_vec());
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

impl CoreTransport for HostTransport {
    async fn connect(&mut self) -> Result<()> {
        self.connect
            .call_async(NoArgs)
            .await
            .map_err(host_error)?
            .await
            .map_err(host_error)?;
        self.connected = true;
        self.start_pump();
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let args = FnArgs::from((topic.to_owned(), Buffer::from(payload.to_vec())));
        self.send
            .call_async(args)
            .await
            .map_err(host_error)?
            .await
            .map_err(host_error)?;
        Ok(())
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        self.subscribe
            .call_async(topic.to_owned())
            .await
            .map_err(host_error)?
            .await
            .map_err(host_error)?;
        Ok(())
    }
}

impl Receive for HostTransport {
    async fn recv(&mut self) -> Result<Option<Message>> {
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
#[napi(object)]
pub struct TransportMessage {
    /// The topic it was published to.
    pub topic: String,
    /// The raw payload bytes.
    pub payload: Buffer,
}

/// One transport, ready to compose into a ladder or a wrapper.
///
/// Build one with the static factories, then hand it to whatever should own it.
/// A transport handed on is spent: calling anything on it afterwards throws.
/// Which faults a degraded link injects, each off unless named.
#[cfg(feature = "sim")]
#[napi(object)]
#[derive(Default)]
pub struct Faults {
    /// Lose one send in every this many.
    pub drop_every: Option<u32>,
    /// How many sends the link stays reachable for before it goes down.
    pub up: Option<u32>,
    /// How many sends it then stays unreachable for.
    pub down: Option<u32>,
}

#[napi]
pub struct Transport {
    inner: std::sync::Mutex<Option<Kind>>,
}

impl Transport {
    /// Wraps a transport kind in the class JavaScript holds.
    pub(crate) fn wrap(kind: Kind) -> Self {
        Self {
            inner: std::sync::Mutex::new(Some(kind)),
        }
    }

    /// Takes the transport, leaving this handle spent.
    pub(crate) fn take(&self) -> napi::Result<Kind> {
        self.inner
            .lock()
            .map_err(|_| napi::Error::from_reason("this transport is poisoned"))?
            .take()
            .ok_or_else(|| {
                napi::Error::from_reason(
                    "this transport was already added to a ladder or a wrapper",
                )
            })
    }
}

#[napi]
impl Transport {
    /// Creates an MQTT transport from broker settings.
    #[cfg(feature = "mqtt")]
    #[napi(factory)]
    pub fn mqtt(options: crate::mqtt::MqttClientOptions) -> Self {
        Self::wrap(Kind::Mqtt(pamoja_mqtt::MqttTransport::new(
            crate::mqtt::settings(options),
        )))
    }

    /// Creates a CoAP transport from endpoint settings.
    #[cfg(feature = "coap")]
    #[napi(factory)]
    pub fn coap(options: crate::coap::CoapClientOptions) -> Self {
        Self::wrap(Kind::Coap(pamoja_coap::CoapTransport::new(
            crate::coap::settings(options),
        )))
    }

    /// Wraps a link written in JavaScript as a transport.
    ///
    /// `handlers` needs `connect()`, `send(topic, payload)`, and
    /// `subscribe(topic)`, each returning a promise or nothing. A `recv()` that
    /// resolves to a message, or to `null` once the link has ended, makes it a
    /// link that delivers: it is called again as soon as it settles, from the
    /// moment the transport connects. Without `recv` the transport only sends,
    /// and a ladder never listens on it. The methods are called on `handlers`,
    /// so a class instance works as it is.
    #[napi(
        factory,
        ts_args_type = "handlers: { connect(): void | Promise<void>; send(topic: string, payload: Buffer): void | Promise<void>; subscribe(topic: string): void | Promise<void>; recv?(): TransportMessage | null | undefined | Promise<TransportMessage | null | undefined> }"
    )]
    pub fn from_handlers(handlers: Object) -> napi::Result<Self> {
        Ok(Self::wrap(Kind::Host(HostTransport::new(&handlers)?)))
    }

    /// Wraps a transport so its next `failures` sends fail.
    ///
    /// This is how a caller checks that a ladder falls through to its next rung,
    /// or that a buffer fills, without unplugging anything. The wrapped
    /// transport is consumed.
    #[cfg(feature = "loopback")]
    #[napi(factory)]
    pub fn faulty(inner: &Transport, failures: u32) -> napi::Result<Self> {
        Ok(Self::wrap(Kind::Faulty(pamoja_loopback::Faulty::new(
            AnyTransport::new(inner.take()?),
            failures as usize,
        ))))
    }

    /// Wraps a transport in a link that loses packets and goes down.
    ///
    /// The wrapped transport is consumed. Every fault is off unless the options
    /// name it, so a link that only drops packets says only that.
    ///
    /// @param inner - the transport to degrade.
    /// @param faults - `dropEvery` loses one send in every this many; `up` and
    /// `down` alternate that many sends of reachable and unreachable.
    #[cfg(feature = "sim")]
    #[napi(factory)]
    pub fn degraded(inner: &Transport, faults: Option<Faults>) -> napi::Result<Self> {
        let faults = faults.unwrap_or_default();
        let mut link = pamoja_sim::DegradedLink::new(AnyTransport::new(inner.take()?));
        if let Some(drop_every) = faults.drop_every.filter(|every| *every != 0) {
            link = link.drop_every(drop_every);
        }
        if let Some(up) = faults.up.filter(|up| *up != 0) {
            link = link.intermittent(up, faults.down.unwrap_or(0));
        }
        Ok(Self::wrap(Kind::Degraded(link)))
    }

    /// Whether this transport is still holdable, or has been handed on.
    #[napi(getter)]
    pub fn is_available(&self) -> bool {
        self.inner.lock().is_ok_and(|slot| slot.is_some())
    }
}
