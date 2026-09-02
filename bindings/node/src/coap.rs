//! Generated Node bindings for CoAP.
//!
//! These mirror the `pamoja-coap` Rust API. CoAP is the transport for links
//! where MQTT is more than the budget allows: it runs over UDP, its headers are
//! a handful of bytes, and a node can fire a reading and forget it rather than
//! holding a session open.

use std::sync::Arc;
use std::time::Duration;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_coap::{CoapConfig, CoapTransport, Reliability as CoreReliability};
use pamoja_core::Transport as CoreTransport;
use tokio::sync::Mutex;

use crate::transport::TransportMessage;

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
    /// How long to wait for an acknowledgement, in milliseconds.
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
        transport
            .connect()
            .await
            .map_err(to_napi)
    }

    /// Sends a payload to a resource path.
    #[napi]
    pub async fn send(&self, topic: String, payload: Buffer) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let payload = payload.to_vec();
        let mut transport = inner.lock().await;
        transport
            .send(&topic, &payload)
            .await
            .map_err(to_napi)
    }

    /// Observes a resource path, so messages published to it reach `recv`.
    #[napi]
    pub async fn subscribe(&self, topic: String) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport
            .subscribe(&topic)
            .await
            .map_err(to_napi)
    }

    /// Waits for the next message on an observed path, or `null` once the
    /// endpoint is closed.
    #[napi]
    pub async fn recv(&self) -> napi::Result<Option<TransportMessage>> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        let received = transport.recv().await.map_err(to_napi)?;
        Ok(received.map(|message| TransportMessage {
            topic: message.topic,
            payload: message.payload.into(),
        }))
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
        transport
            .disconnect()
            .await
            .map_err(to_napi)
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
