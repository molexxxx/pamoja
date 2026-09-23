//! Generated Node bindings for CoAP.
//!
//! These mirror the `pamoja-coap` Rust API. CoAP is the transport for links
//! where MQTT is more than the budget allows: it runs over UDP, its headers are
//! a handful of bytes, and a node can fire a reading and forget it rather than
//! holding a session open.

use std::sync::Arc;
use std::time::Duration;

use napi::bindgen_prelude::Buffer;
use napi::Either;
use napi_derive::napi;
use pamoja_coap::{
    CoapConfig, CoapPublisher, CoapServer as CoreServer, CoapTransport,
    Reliability as CoreReliability,
};
use pamoja_core::{Receive, Transport as CoreTransport};
use tokio::sync::Mutex;

use crate::transport::{bytes_of, message_of, within, TransportMessage};

/// Whether a CoAP request is acknowledged and retried.
#[napi(string_enum)]
pub enum Reliability {
    /// Fire and forget: the request is sent once and not acknowledged.
    NonConfirmable,
    /// The request is acknowledged, and retransmitted until an ACK arrives.
    Confirmable,
}

/// The settings a CoAP endpoint is built from.
#[napi(object)]
pub struct CoapClientOptions {
    /// The peer hostname or IP address.
    pub host: String,
    /// The peer UDP port, conventionally 5683 for plaintext CoAP.
    pub port: u16,
    /// The local address to bind. Defaults to an ephemeral port when omitted.
    pub bind: Option<String>,
    /// Whether requests are acknowledged and retried. Defaults to confirmable.
    pub reliability: Option<Reliability>,
    /// How long to wait for the first acknowledgment, in milliseconds, two seconds when
    /// omitted. Each wait after it doubles, and RFC 7252 forbids a first wait shorter
    /// than two seconds on a network without congestion control.
    pub ack_timeout_ms: Option<u32>,
    /// How many times to retransmit an unacknowledged request.
    pub max_retransmits: Option<u32>,
}

/// A CoAP endpoint.
#[napi]
pub struct CoapClient {
    inner: Arc<Mutex<CoapTransport>>,
}

#[napi]
impl CoapClient {
    /// Creates a disconnected endpoint from the given settings.
    #[napi(constructor)]
    pub fn new(options: CoapClientOptions) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CoapTransport::new(settings(options)))),
        }
    }

    /// Binds the local socket so the endpoint can carry traffic.
    #[napi]
    pub async fn connect(&self) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport.connect().await.map_err(to_napi)
    }

    /// Sends a payload to a resource path: bytes, or text such as a reading written
    /// out.
    #[napi]
    pub async fn send(&self, topic: String, payload: Either<Buffer, String>) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let payload = bytes_of(payload);
        let mut transport = inner.lock().await;
        transport.send(&topic, &payload).await.map_err(to_napi)
    }

    /// Observes a resource path, so messages published to it reach `recv`.
    #[napi]
    pub async fn subscribe(&self, topic: String) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport.subscribe(&topic).await.map_err(to_napi)
    }

    /// Waits for the next message on an observed path, or `null` once the
    /// endpoint is closed.
    ///
    /// @param timeoutMs - how long to wait before rejecting; a message that arrives later
    /// waits for the next receive.
    #[napi]
    pub async fn recv(&self, timeout_ms: Option<u32>) -> napi::Result<Option<TransportMessage>> {
        let inner = Arc::clone(&self.inner);
        within(timeout_ms, async move {
            let mut transport = inner.lock().await;
            let received = transport.recv().await.map_err(to_napi)?;
            Ok(received.map(message_of))
        })
        .await
    }

    /// Whether the local socket is bound.
    #[napi]
    pub async fn is_connected(&self) -> bool {
        let inner = Arc::clone(&self.inner);
        let transport = inner.lock().await;
        transport.is_connected()
    }

    /// Releases the socket the endpoint holds.
    #[napi]
    pub async fn disconnect(&self) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport.disconnect().await.map_err(to_napi)
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
#[napi]
pub struct CoapServer {
    server: Arc<Mutex<CoreServer>>,
    publisher: CoapPublisher,
}

#[napi]
impl CoapServer {
    /// Creates a server that will listen on a local address, such as `0.0.0.0:5683`,
    /// or port 0 for a free one.
    #[napi(constructor)]
    pub fn new(bind: String) -> Self {
        let server = CoreServer::new(bind);
        let publisher = server.publisher();
        Self {
            server: Arc::new(Mutex::new(server)),
            publisher,
        }
    }

    /// Binds the socket and starts answering requests. Rejects when the address
    /// cannot be bound.
    #[napi]
    pub async fn connect(&self) -> napi::Result<()> {
        let inner = Arc::clone(&self.server);
        let mut server = inner.lock().await;
        server.connect().await.map_err(to_napi)
    }

    /// Takes the readings sent to the paths a filter matches, with `+` for one
    /// level and `#` for the rest.
    #[napi]
    pub async fn subscribe(&self, filter: String) -> napi::Result<()> {
        let inner = Arc::clone(&self.server);
        let mut server = inner.lock().await;
        server.subscribe(&filter).await.map_err(to_napi)
    }

    /// Sets a resource's state and notifies every observer of it: bytes, or text
    /// such as a command. Rejects when the server is not connected.
    #[napi]
    pub async fn send(&self, path: String, payload: Either<Buffer, String>) -> napi::Result<()> {
        let publisher = self.publisher.clone();
        let payload = bytes_of(payload);
        publisher.publish(&path, &payload).await.map_err(to_napi)
    }

    /// Waits for the next reading sent to a path the server takes. Rejects when the
    /// server is not connected.
    ///
    /// @param timeoutMs - how long to wait before rejecting; a reading that arrives
    /// later waits for the next receive.
    #[napi]
    pub async fn recv(&self, timeout_ms: Option<u32>) -> napi::Result<Option<TransportMessage>> {
        let inner = Arc::clone(&self.server);
        within(timeout_ms, async move {
            let mut server = inner.lock().await;
            let received = server.recv().await.map_err(to_napi)?;
            Ok(received.map(message_of))
        })
        .await
    }

    /// Counts the clients observing a resource.
    #[napi]
    pub fn observers(&self, path: String) -> u32 {
        self.publisher.observers(&path) as u32
    }

    /// The port the server listens on, which names the one the system chose for
    /// port 0, or `null` while it is not connected.
    #[napi(getter)]
    pub fn local_port(&self) -> Option<u16> {
        self.publisher.local_addr().map(|address| address.port())
    }

    /// Whether the server holds a bound socket.
    #[napi(getter)]
    pub fn is_connected(&self) -> bool {
        self.publisher.is_connected()
    }

    /// Closes the socket, keeping the resources and filters.
    #[napi]
    pub async fn disconnect(&self) -> napi::Result<()> {
        let inner = Arc::clone(&self.server);
        let mut server = inner.lock().await;
        server.disconnect().await.map_err(to_napi)
    }
}

/// Reads the endpoint settings an options object describes.
///
/// Shared with the composable transport, so an endpoint and a ladder rung read
/// the same fields the same way.
pub(crate) fn settings(options: CoapClientOptions) -> CoapConfig {
    let mut config = CoapConfig::new(options.host, options.port);
    if let Some(bind) = options.bind {
        config = config.bind(bind);
    }
    config = config.reliability(match options.reliability {
        Some(Reliability::NonConfirmable) => CoreReliability::NonConfirmable,
        _ => CoreReliability::Confirmable,
    });
    if let Some(millis) = options.ack_timeout_ms {
        config = config.ack_timeout(Duration::from_millis(u64::from(millis)));
    }
    if let Some(count) = options.max_retransmits {
        config = config.max_retransmits(count);
    }
    config
}

/// Maps a core error onto the one JavaScript sees.
fn to_napi(error: pamoja_core::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
