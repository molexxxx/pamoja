//! Generated Node bindings for the MQTT transport.
//!
//! These mirror the `pamoja-mqtt` Rust API one-to-one. The shared state lives
//! behind an async mutex so the napi-generated async methods own a clonable
//! handle rather than borrowing the JavaScript object across an `await`.

use crate::checked::{self, OptionalWhole};
use std::sync::Arc;
use std::time::Duration;

use napi::bindgen_prelude::Buffer;
use napi::Either;
use napi_derive::napi;

use crate::transport::{bytes_of, within};
use pamoja_core::{Error, Transport};
use pamoja_mqtt::{Inbox, MqttConfig, MqttTransport, PublishOptions, QualityOfService, Tls, Will};
use tokio::sync::Mutex;

/// MQTT delivery guarantee, mirroring the protocol's quality-of-service levels.
// The shared `Once` suffix is the protocol's own vocabulary; renaming would
// distort the published JavaScript API, so the variant-name lint is allowed here.
#[allow(clippy::enum_variant_names)]
#[napi(string_enum)]
pub enum Qos {
    /// Fire and forget; the broker does not acknowledge delivery.
    AtMostOnce,
    /// Delivered at least once and acknowledged.
    AtLeastOnce,
    /// Delivered exactly once via a four-step handshake.
    ExactlyOnce,
}

impl From<Qos> for QualityOfService {
    fn from(value: Qos) -> Self {
        match value {
            Qos::AtMostOnce => QualityOfService::AtMostOnce,
            Qos::AtLeastOnce => QualityOfService::AtLeastOnce,
            Qos::ExactlyOnce => QualityOfService::ExactlyOnce,
        }
    }
}

/// Connection settings for an [`MqttClient`].
#[napi(object)]
pub struct MqttClientOptions {
    /// The MQTT client identifier presented to the broker.
    pub client_id: String,
    /// The broker hostname or IP address.
    pub host: String,
    /// The broker TCP port, conventionally 1883 for plaintext MQTT.
    pub port: checked::u16,
    /// Keep-alive interval in seconds. Defaults to 30 when omitted.
    pub keep_alive_secs: Option<checked::u32>,
    /// Bound on outstanding client requests. Defaults to 64 when omitted.
    pub capacity: Option<checked::u32>,
    /// Default quality of service. Defaults to `AtLeastOnce` when omitted.
    pub qos: Option<Qos>,
    /// The largest packet the connection sends or accepts, in bytes. Defaults to 10,240
    /// when omitted. A publish that would be larger is refused and the connection stays
    /// up, but a larger packet arriving from the broker ends the connection.
    pub max_packet_size: Option<checked::u32>,
    /// The name to sign in to the broker with.
    pub username: Option<String>,
    /// The password to sign in with, which needs a username. It travels in the clear
    /// unless the connection uses TLS.
    pub password: Option<String>,
    /// A message the broker publishes if the connection ends without a goodbye.
    pub will: Option<MqttWill>,
    /// TLS settings; a connection with them is secured, conventionally on port 8883.
    pub tls: Option<MqttTls>,
}

/// A message the broker publishes on the client's behalf if its connection ends without a
/// disconnect: the network dropped, the power failed, or the keep-alive ran out.
#[napi(object)]
pub struct MqttWill {
    /// The topic the broker publishes it to, with no wildcard.
    pub topic: String,
    /// What it publishes; text is sent as UTF-8.
    pub payload: Either<Buffer, String>,
    /// The quality of service it is published at. Defaults to `AtMostOnce`.
    pub qos: Option<Qos>,
    /// Whether the broker retains it for clients that subscribe later.
    pub retain: Option<bool>,
}

/// How a connection is secured with TLS.
#[napi(object)]
pub struct MqttTls {
    /// The certificate authorities to trust, as PEM. Without it the system's are trusted.
    pub ca_pem: Option<Either<Buffer, String>>,
    /// A client certificate to present, as PEM, for a broker that asks for one.
    pub certificate_pem: Option<Either<Buffer, String>>,
    /// The client certificate's private key, as PEM.
    pub key_pem: Option<Either<Buffer, String>>,
}

/// How one message is published.
#[napi(object)]
pub struct MqttPublishOptions {
    /// The quality of service for this message. Defaults to the client's.
    pub qos: Option<Qos>,
    /// Whether the broker keeps it for clients that subscribe later. An empty retained
    /// message clears the one the broker holds.
    pub retain: Option<bool>,
}

/// A message received from a subscribed topic.
#[napi(object)]
pub struct MqttMessage {
    /// The topic the message was published to.
    pub topic: String,
    /// The raw payload bytes.
    pub payload: Buffer,
    /// The payload as text, when it is UTF-8: words, or a number written out.
    pub text: Option<String>,
    /// The payload as a number, when its text is one, such as `21.5`.
    pub number: Option<f64>,
}

/// An MQTT client transport backed by the native pamoja core.
#[napi]
pub struct MqttClient {
    inner: Arc<Mutex<MqttTransport>>,
    inbox: Inbox,
}

#[napi]
impl MqttClient {
    /// Creates a disconnected client from the given options.
    ///
    /// @throws If a password comes without a username, or a TLS client certificate
    ///   without its key.
    #[napi(constructor)]
    pub fn new(options: MqttClientOptions) -> napi::Result<Self> {
        let transport = MqttTransport::new(settings(options)?);
        Ok(Self {
            inbox: transport.inbox(),
            inner: Arc::new(Mutex::new(transport)),
        })
    }

    /// Connects to the broker and starts the background event loop.
    #[napi]
    pub async fn connect(&self) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport.connect().await.map_err(to_napi)
    }

    /// Publishes a payload to a topic, resolving once it is queued for the broker.
    #[napi]
    pub async fn publish(
        &self,
        topic: String,
        payload: Either<Buffer, String>,
        options: Option<MqttPublishOptions>,
    ) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let payload = bytes_of(payload);
        let mut transport = inner.lock().await;
        transport
            .publish(&topic, &payload, publish_options(options))
            .await
            .map(drop)
            .map_err(to_napi)
    }

    /// Publishes a payload to a topic, resolving once the broker acknowledges it: its
    /// `PUBACK` at `AtLeastOnce`, its `PUBCOMP` at `ExactlyOnce`, and once the connection
    /// has taken it at `AtMostOnce`, where MQTT acknowledges nothing. It rejects if the
    /// connection ends first, when the message may or may not have arrived.
    #[napi]
    pub async fn publish_confirmed(
        &self,
        topic: String,
        payload: Either<Buffer, String>,
        options: Option<MqttPublishOptions>,
    ) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let payload = bytes_of(payload);
        let delivery = {
            let mut transport = inner.lock().await;
            transport
                .publish(&topic, &payload, publish_options(options))
                .await
                .map_err(to_napi)?
        };
        delivery.confirmed().await.map_err(to_napi)
    }

    /// Subscribes to a topic filter.
    #[napi]
    pub async fn subscribe(&self, topic: String) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport.subscribe(&topic).await.map_err(to_napi)
    }

    /// Awaits the next message from any subscribed topic, or `null` once the
    /// connection has ended. A connection that ends on its own rejects one receive
    /// with the reason.
    ///
    /// @param timeoutMs - how long to wait before rejecting; a message that arrives later
    /// waits for the next receive.
    #[napi]
    pub async fn recv(&self, timeout_ms: Option<f64>) -> napi::Result<Option<MqttMessage>> {
        let inbox = self.inbox.clone();
        within(timeout_ms, async move {
            let message = inbox.recv().await.map_err(to_napi)?;
            Ok(message.map(|message| MqttMessage {
                text: message.text().ok().map(str::to_owned),
                number: message.number().ok(),
                topic: message.topic,
                payload: message.payload.into(),
            }))
        })
        .await
    }

    /// Reports whether the client currently holds an active connection.
    #[napi]
    pub async fn is_connected(&self) -> bool {
        let inner = Arc::clone(&self.inner);
        let transport = inner.lock().await;
        transport.is_connected()
    }

    /// Closes the connection and stops the background event loop.
    #[napi]
    pub async fn disconnect(&self) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport.disconnect().await.map_err(to_napi)
    }
}

/// Maps a core error onto a napi error so it surfaces as a rejected promise.
fn to_napi(err: Error) -> napi::Error {
    napi::Error::from_reason(err.to_string())
}

/// Reads how one message is published.
fn publish_options(options: Option<MqttPublishOptions>) -> PublishOptions {
    let mut built = PublishOptions::new();
    if let Some(options) = options {
        if let Some(qos) = options.qos {
            built = built.qos(qos.into());
        }
        if options.retain == Some(true) {
            built = built.retained();
        }
    }
    built
}

/// Reads the broker settings an options object describes.
///
/// Shared with the composable transport, so a client and a ladder rung read the
/// same fields the same way.
pub(crate) fn settings(options: MqttClientOptions) -> napi::Result<MqttConfig> {
    let mut config = MqttConfig::new(options.client_id, options.host, options.port.get());
    match (options.username, options.password) {
        (Some(username), password) => {
            config = config.credentials(username, password.unwrap_or_default());
        }
        (None, Some(_)) => {
            return Err(napi::Error::from_reason(
                "a password needs a username: MQTT sends no password alone",
            ))
        }
        (None, None) => {}
    }
    if let Some(will) = options.will {
        let mut built = Will::new(will.topic, bytes_of(will.payload));
        if let Some(qos) = will.qos {
            built = built.qos(qos.into());
        }
        if will.retain == Some(true) {
            built = built.retained();
        }
        config = config.last_will(built);
    }
    if let Some(tls) = options.tls {
        let mut built = match tls.ca_pem {
            Some(pem) => Tls::with_ca_pem(bytes_of(pem)),
            None => Tls::system_roots(),
        };
        match (tls.certificate_pem, tls.key_pem) {
            (Some(certificate), Some(key)) => {
                built = built.client_certificate(bytes_of(certificate), bytes_of(key));
            }
            (None, None) => {}
            _ => {
                return Err(napi::Error::from_reason(
                    "a client certificate and its key come together",
                ))
            }
        }
        config = config.tls(built);
    }
    if let Some(secs) = options.keep_alive_secs.get() {
        config = config.keep_alive(Duration::from_secs(u64::from(secs)));
    }
    if let Some(capacity) = options.capacity.get() {
        config = config.capacity(capacity as usize);
    }
    if let Some(qos) = options.qos {
        config = config.qos(qos.into());
    }
    if let Some(bytes) = options.max_packet_size.get() {
        config = config.max_packet_size(bytes as usize);
    }
    Ok(config)
}
