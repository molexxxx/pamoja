//! An in-process loopback transport for hardware-free testing and examples.
//!
//! [`LoopbackTransport`] implements the core [`Transport`](pamoja_core::Transport)
//! and [`Receive`](pamoja_core::Receive) traits against a shared in-memory
//! [`LoopbackBroker`] instead of a network, so examples, the simulators, and the
//! cross-language conformance scenarios can exercise the full publish/subscribe
//! path with no broker process and no hardware. Topic filters follow MQTT
//! semantics, including the `+` single-level and `#` multi-level wildcards.
//!
//! Clone one broker into every transport that should share a namespace: a publish
//! on any transport is delivered to every transport whose subscriptions match.
//!
//! [`Faulty`] decorates any transport to inject send failures, so degraded-link
//! behavior such as store-and-forward retry can be tested deterministically.
//!
//! # Examples
//!
//! A sensor and a gateway on one broker, with no network between them: the gateway
//! subscribes to every node's temperature, and a reading published by one node reaches it.
//!
//! ```
//! use pamoja_core::{Receive, Transport};
//! use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
//!
//! # tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
//! let broker = LoopbackBroker::new();
//! let mut gateway = LoopbackTransport::new(broker.clone());
//! let mut sensor = LoopbackTransport::new(broker);
//! gateway.connect().await?;
//! sensor.connect().await?;
//!
//! gateway.subscribe("sensors/+/temperature").await?;
//! sensor.send("sensors/7/temperature", b"21.5").await?;
//!
//! let reading = gateway.recv().await?.expect("the reading arrives");
//! assert_eq!(reading.topic, "sensors/7/temperature");
//! assert_eq!(reading.payload, b"21.5");
//! # Ok::<(), pamoja_core::Error>(())
//! # }).unwrap();
//! ```

mod broker;
mod faulty;
mod transport;

pub use broker::LoopbackBroker;
pub use faulty::Faulty;
pub use pamoja_core::Message;
pub use transport::LoopbackTransport;
