//! Generated Node bindings for the in-process loopback broker.
//!
//! These mirror the `pamoja-loopback` Rust API so a caller can exercise the
//! publish-and-subscribe path with no broker, no network, and no hardware. That
//! matters more from a binding than from Rust: someone writing against the SDK
//! in JavaScript can drive a whole message flow in a unit test rather than
//! standing up infrastructure to find out whether their topics line up.

use std::sync::Arc;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_core::Transport as CoreTransport;
use pamoja_loopback::{LoopbackBroker as CoreBroker, LoopbackTransport as CoreLoopback};
use tokio::sync::Mutex;

use crate::transport::{Kind, Transport, TransportMessage};

/// An in-process broker.
///
/// Every transport built from one broker shares its traffic, so a message one
/// publishes reaches the others that subscribed to the topic.
#[napi]
pub struct LoopbackBroker {
    inner: CoreBroker,
}

#[napi]
impl LoopbackBroker {
    /// Creates a broker with no traffic.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreBroker::new(),
        }
    }

    /// Creates a link to this broker, for driving directly.
    #[napi]
    pub fn link(&self) -> LoopbackTransport {
        LoopbackTransport {
            inner: Arc::new(Mutex::new(CoreLoopback::new(self.inner.clone()))),
        }
    }

    /// Creates a link to this broker as a transport, for composing into a
    /// ladder or a wrapper.
    #[napi]
    pub fn rung(&self) -> Transport {
        Transport::wrap(Kind::Loopback(CoreLoopback::new(self.inner.clone())))
    }
}

impl Default for LoopbackBroker {
    fn default() -> Self {
        Self::new()
    }
}

/// One in-process link to a broker.
#[napi]
pub struct LoopbackTransport {
    inner: Arc<Mutex<CoreLoopback>>,
}

#[napi]
impl LoopbackTransport {
    /// Marks this link connected so it will carry traffic.
    #[napi]
    pub async fn connect(&self) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport
            .connect()
            .await
            .map_err(to_napi)
    }

    /// Publishes a payload to a topic on the broker.
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

    /// Subscribes this link to a topic.
    #[napi]
    pub async fn subscribe(&self, topic: String) -> napi::Result<()> {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport
            .subscribe(&topic)
            .await
            .map_err(to_napi)
    }

    /// Waits for the next message on a subscribed topic, or `null` once the link
    /// is closed.
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

    /// Whether this link is connected.
    #[napi]
    pub async fn is_connected(&self) -> bool {
        let inner = Arc::clone(&self.inner);
        let transport = inner.lock().await;
        transport.is_connected()
    }

    /// Marks this link disconnected, so sends over it fail.
    #[napi]
    pub async fn disconnect(&self) {
        let inner = Arc::clone(&self.inner);
        let mut transport = inner.lock().await;
        transport.disconnect();
    }
}

/// Maps a core error onto the one JavaScript sees.
fn to_napi(error: pamoja_core::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
