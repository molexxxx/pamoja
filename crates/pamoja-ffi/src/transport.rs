//! The C ABI for composing transports.
//!
//! A ladder rung, a fault injector, and a degraded link all take "some
//! transport", which in Rust is any `impl Transport`. A C ABI has no generics,
//! so this module carries one tagged handle that holds whichever transport was
//! built and dispatches to it. One handle keeps the composing calls to a single
//! function each, rather than one per transport kind, and lets the set of kinds
//! grow without reshaping the surface.
//!
//! A transport built here is separate from a client handle such as
//! [`PamojaMqttClient`](crate::mqtt::PamojaMqttClient). A client is for driving
//! a link directly; a transport is for composition, and is consumed by whatever
//! it is composed into. Keeping them apart means nothing has to move out of a
//! live, shared handle.

use std::ffi::CString;
use std::future::Future;
use std::pin::Pin;
use std::ptr;

use pamoja_core::{Result, Transport};

use crate::{read_bytes, set_last_error, PamojaStatus};

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


/// An opaque handle to one message that arrived on a subscribed topic.
///
/// CoAP and the loopback broker both hand back a topic and a payload, so one
/// handle serves them rather than a near-identical type per transport.
pub struct PamojaMessage {
    topic: CString,
    payload: Vec<u8>,
}

impl PamojaMessage {
    /// Wraps a received message in a handle the caller owns.
    ///
    /// A topic carrying an interior null cannot cross as a C string, so it is
    /// replaced with an empty one rather than truncated at the null, which would
    /// hand the caller a plausible-looking wrong topic.
    pub(crate) fn into_raw(topic: String, payload: Vec<u8>) -> *mut Self {
        let topic = CString::new(topic).unwrap_or_default();
        Box::into_raw(Box::new(Self { topic, payload }))
    }
}

/// Returns the topic a message arrived on.
///
/// # Arguments
///
/// * `message` - the message.
///
/// # Returns
///
/// A null-terminated UTF-8 string owned by the message and valid until it is
/// freed, or null if `message` is null.
///
/// # Safety
///
/// `message` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_message_topic(
    message: *const PamojaMessage,
) -> *const std::ffi::c_char {
    if message.is_null() {
        return ptr::null();
    }
    (*message).topic.as_ptr()
}

/// Returns a pointer to a message payload.
///
/// # Arguments
///
/// * `message` - the message.
///
/// # Returns
///
/// A pointer to the bytes, valid until the message is freed, or null if
/// `message` is null.
///
/// # Safety
///
/// `message` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_message_payload(message: *const PamojaMessage) -> *const u8 {
    if message.is_null() {
        return ptr::null();
    }
    (*message).payload.as_ptr()
}

/// Returns the length in bytes of a message payload.
///
/// # Arguments
///
/// * `message` - the message.
///
/// # Returns
///
/// The length, or 0 if `message` is null.
///
/// # Safety
///
/// `message` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_message_payload_len(message: *const PamojaMessage) -> usize {
    if message.is_null() {
        return 0;
    }
    (*message).payload.len()
}

/// Releases a message handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `message` must be a handle from a call that produced one and that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_message_free(message: *mut PamojaMessage) {
    if !message.is_null() {
        drop(Box::from_raw(message));
    }
}

/// An opaque handle to one transport, ready to drive or to compose.
///
/// Release it with [`pamoja_transport_free`] unless it has been consumed by a
/// call that takes ownership, such as adding it to a ladder.
pub struct PamojaTransport {
    pub(crate) kind: Kind,
}

impl PamojaTransport {
    /// Wraps a transport kind in a handle the caller owns.
    pub(crate) fn into_raw(kind: Kind) -> *mut Self {
        Box::into_raw(Box::new(Self { kind }))
    }
}

/// Creates an MQTT transport from broker settings.
///
/// # Arguments
///
/// * `config` - the broker settings, read the same way a client reads them.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_transport_free`] or hand to a
/// call that consumes it, or null on failure.
///
/// # Safety
///
/// `config` must point to a valid config whose strings are valid
/// null-terminated UTF-8 for the duration of the call.
#[cfg(feature = "mqtt")]
#[no_mangle]
pub unsafe extern "C" fn pamoja_transport_mqtt(
    config: *const crate::mqtt::PamojaMqttConfig,
) -> *mut PamojaTransport {
    let Some(settings) = crate::mqtt::mqtt_settings(config) else {
        return ptr::null_mut();
    };
    PamojaTransport::into_raw(Kind::Mqtt(pamoja_mqtt::MqttTransport::new(settings)))
}

/// Wraps a transport so a set number of its next sends fail.
///
/// This is how a caller checks that a ladder falls through to its next rung, or
/// that a buffer fills, without unplugging anything.
///
/// # Arguments
///
/// * `transport` - the transport to wrap, consumed by this call.
/// * `failures` - how many upcoming sends to fail.
///
/// # Returns
///
/// A new handle owning the wrapped transport, or null if `transport` is null.
///
/// # Safety
///
/// `transport` must be a live handle that has not been freed or consumed. After
/// this call it must not be used again, whatever the result.
#[cfg(feature = "loopback")]
#[no_mangle]
pub unsafe extern "C" fn pamoja_transport_faulty(
    transport: *mut PamojaTransport,
    failures: usize,
) -> *mut PamojaTransport {
    let Some(inner) = take_transport(transport) else {
        return ptr::null_mut();
    };
    PamojaTransport::into_raw(Kind::Faulty(pamoja_loopback::Faulty::new(
        AnyTransport::new(inner),
        failures,
    )))
}

/// Wraps a transport in a link that loses packets and goes down.
///
/// # Arguments
///
/// * `transport` - the transport to wrap, consumed by this call.
/// * `drop_every` - lose one send in every this many, or 0 to lose none.
/// * `up` - how many sends the link stays up for, or 0 to never go down.
/// * `down` - how many sends it then stays down for.
///
/// # Returns
///
/// A new handle owning the wrapped transport, or null if `transport` is null.
///
/// # Safety
///
/// `transport` must be a live handle that has not been freed or consumed. After
/// this call it must not be used again, whatever the result.
#[cfg(feature = "sim")]
#[no_mangle]
pub unsafe extern "C" fn pamoja_transport_degraded(
    transport: *mut PamojaTransport,
    drop_every: u32,
    up: u32,
    down: u32,
) -> *mut PamojaTransport {
    let Some(inner) = take_transport(transport) else {
        return ptr::null_mut();
    };
    let mut link = pamoja_sim::DegradedLink::new(AnyTransport::new(inner));
    if drop_every != 0 {
        link = link.drop_every(drop_every);
    }
    if up != 0 {
        link = link.intermittent(up, down);
    }
    PamojaTransport::into_raw(Kind::Degraded(link))
}

/// Connects a transport.
///
/// # Arguments
///
/// * `transport` - the transport to connect.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once connected.
///
/// # Safety
///
/// `transport` must be a live handle that has not been freed or consumed.
#[no_mangle]
pub unsafe extern "C" fn pamoja_transport_connect(
    transport: *mut PamojaTransport,
) -> PamojaStatus {
    let Some(transport) = transport_handle(transport) else {
        return PamojaStatus::InvalidArgument;
    };
    status(crate::runtime().block_on(transport.kind.connect()))
}

/// Sends a payload to a topic over a transport.
///
/// # Arguments
///
/// * `transport` - the transport to send over.
/// * `topic` - the destination topic, as null-terminated UTF-8.
/// * `payload` - the bytes to send.
/// * `payload_len` - the length of `payload`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the transport has taken the payload.
///
/// # Safety
///
/// `transport` must be a live handle, `topic` a valid null-terminated UTF-8
/// string, and `payload` must point to at least `payload_len` readable bytes or
/// be null when that length is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_transport_send(
    transport: *mut PamojaTransport,
    topic: *const std::ffi::c_char,
    payload: *const u8,
    payload_len: usize,
) -> PamojaStatus {
    let Some(transport) = transport_handle(transport) else {
        return PamojaStatus::InvalidArgument;
    };
    let Some(topic) = crate::read_str(topic, "topic") else {
        return PamojaStatus::InvalidArgument;
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    status(crate::runtime().block_on(transport.kind.send(&topic, &payload)))
}

/// Subscribes a transport to a topic.
///
/// # Arguments
///
/// * `transport` - the transport to subscribe.
/// * `topic` - the topic to subscribe to, as null-terminated UTF-8.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once subscribed.
///
/// # Safety
///
/// `transport` must be a live handle and `topic` a valid null-terminated UTF-8
/// string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_transport_subscribe(
    transport: *mut PamojaTransport,
    topic: *const std::ffi::c_char,
) -> PamojaStatus {
    let Some(transport) = transport_handle(transport) else {
        return PamojaStatus::InvalidArgument;
    };
    let Some(topic) = crate::read_str(topic, "topic") else {
        return PamojaStatus::InvalidArgument;
    };
    status(crate::runtime().block_on(transport.kind.subscribe(&topic)))
}

/// Releases a transport handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `transport` must be a handle that has not already been freed or consumed by
/// a call that takes ownership, or null. After this call it must not be used
/// again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_transport_free(transport: *mut PamojaTransport) {
    if !transport.is_null() {
        drop(Box::from_raw(transport));
    }
}

/// Borrows a transport handle, rejecting a null pointer.
///
/// # Safety
///
/// `transport` must be a live handle from a call that produced one, or null.
unsafe fn transport_handle<'a>(
    transport: *mut PamojaTransport,
) -> Option<&'a mut PamojaTransport> {
    if transport.is_null() {
        set_last_error("transport must not be null".to_owned());
        return None;
    }
    Some(&mut *transport)
}

/// Takes ownership of a transport handle, leaving the caller nothing to free.
///
/// # Safety
///
/// `transport` must be a live handle that has not been freed or consumed, or
/// null. After this call the caller must not use it again.
pub(crate) unsafe fn take_transport(transport: *mut PamojaTransport) -> Option<Kind> {
    if transport.is_null() {
        set_last_error("transport must not be null".to_owned());
        return None;
    }
    Some(Box::from_raw(transport).kind)
}

/// Maps a transport result onto a status, recording any failure.
pub(crate) fn status(result: Result<()>) -> PamojaStatus {
    match result {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => {
            let status = PamojaStatus::from_error(&error);
            set_last_error(error.to_string());
            status
        }
    }
}
