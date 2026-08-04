//! Generated Node bindings for the MQTT transport.
//!
//! These mirror the `pamoja-mqtt` Rust API one-to-one. The shared state lives
//! behind an async mutex so the napi-generated async methods own a clonable
//! handle rather than borrowing the JavaScript object across an `await`.

use std::sync::Arc;
use std::time::Duration;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_core::{Error, Transport};
use pamoja_mqtt::{MqttConfig, MqttTransport, QualityOfService};
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
    pub port: u16,
    /// Keep-alive interval in seconds. Defaults to 30 when omitted.
    pub keep_alive_secs: Option<u32>,
    /// Bound on outstanding client requests. Defaults to 64 when omitted.
    pub capacity: Option<u32>,
    /// Default quality of service. Defaults to `AtLeastOnce` when omitted.
    pub qos: Option<Qos>,
}

/// A message received from a subscribed topic.
#[napi(object)]
pub struct MqttMessage {
    /// The topic the message was published to.
    pub topic: String,
    /// The raw payload bytes.
    pub payload: Buffer,
}

/// An MQTT client transport backed by the native pamoja core.
#[napi]
pub struct MqttClient {
    inner: Arc<Mutex<MqttTransport>>,
}

#[napi]
impl MqttClient {
    /// Creates a disconnected client from the given options.
    #[napi(constructor)]
    pub fn new(options: MqttClientOptions) -> Self {
        let mut config = MqttConfig::new(options.client_id, options.host, options.port);
        if let Some(secs) = options.keep_alive_secs {
            config = config.keep_alive(Duration::from_secs(u64::from(secs)));
        }
        if let Some(capacity) = options.capacity {
            config = config.capacity(capacity as usize);
        }
        if let Some(qos) = options.qos {
            config = config.qos(qos.into());
        }
        Self {
            inner: Arc::new(Mutex::new(MqttTransport::new(config))),
        }
    }

    /// Connects to the broker and starts the background event loop.
    #[napi]
    pub async fn connect(&self) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport.connect().await.map_err(to_napi)
    }

    /// Publishes a payload to a topic.
    #[napi]
    pub async fn publish(&self, topic: String, payload: Buffer) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport
            .send(&topic, payload.as_ref())
            .await
            .map_err(to_napi)
    }

    /// Subscribes to a topic filter.
    #[napi]
    pub async fn subscribe(&self, topic: String) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport.subscribe(&topic).await.map_err(to_napi)
    }

    /// Awaits the next message from any subscribed topic, or `null` once the
    /// connection has ended.
    #[napi]
    pub async fn recv(&self) -> napi::Result<Option<MqttMessage>> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        let message = transport.recv().await.map_err(to_napi)?;
        Ok(message.map(|message| MqttMessage {
            topic: message.topic,
            payload: message.payload.into(),
        }))
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
