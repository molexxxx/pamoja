//! An in-process loopback transport for hardware-free testing and examples.
//!
//! [`LoopbackTransport`] implements the core [`Transport`](pamoja_core::Transport)
//! and [`Receive`](pamoja_core::Receive) traits against a shared in-memory
//! [`LoopbackBroker`] instead of a network, so examples, the simulators, and the
//! cross-language conformance scenarios can exercise the full publish/subscribe
//! path with no broker process and no hardware. Topic filters follow MQTT semantics, including the `+` single-level
//! and `#` multi-level wildcards.
//!
//! Clone one broker into every transport that should share a namespace: a publish
//! on any transport is delivered to every transport whose subscriptions match.
//!
//! [`Faulty`] decorates any transport to inject send failures, so degraded-link
//! behavior such as store-and-forward retry can be tested deterministically.

mod broker;
mod faulty;
mod transport;

pub use broker::LoopbackBroker;
pub use faulty::Faulty;
pub use pamoja_core::Message;
pub use transport::LoopbackTransport;
