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

use std::future::Future;
use std::pin::Pin;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_core::{Result, Transport as CoreTransport};

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

impl<T: CoreTransport + Send> DynTransport for Erased<T> {
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
#[napi]
pub struct Transport {
    inner: Option<Kind>,
}

impl Transport {
    /// Wraps a transport kind in the class JavaScript holds.
    pub(crate) fn wrap(kind: Kind) -> Self {
        Self { inner: Some(kind) }
    }

    /// Takes the transport, leaving this handle spent.
    pub(crate) fn take(&mut self) -> napi::Result<Kind> {
        self.inner.take().ok_or_else(|| {
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

    /// Wraps a transport so its next `failures` sends fail.
    ///
    /// This is how a caller checks that a ladder falls through to its next rung,
    /// or that a buffer fills, without unplugging anything. The wrapped
    /// transport is consumed.
    #[cfg(feature = "loopback")]
    #[napi(factory)]
    pub fn faulty(inner: &mut Transport, failures: u32) -> napi::Result<Self> {
        Ok(Self::wrap(Kind::Faulty(pamoja_loopback::Faulty::new(
            AnyTransport::new(inner.take()?),
            failures as usize,
        ))))
    }

    /// Wraps a transport in a link that loses packets and goes down.
    ///
    /// The wrapped transport is consumed.
    ///
    /// @param inner - the transport to degrade.
    /// @param dropEvery - lose one send in every this many, or 0 to lose none.
    /// @param up - how many sends the link stays up for, or 0 to never go down.
    /// @param down - how many sends it then stays down for.
    #[cfg(feature = "sim")]
    #[napi(factory)]
    pub fn degraded(
        inner: &mut Transport,
        drop_every: u32,
        up: u32,
        down: u32,
    ) -> napi::Result<Self> {
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
    #[napi(getter)]
    pub fn is_available(&self) -> bool {
        self.inner.is_some()
    }
}
