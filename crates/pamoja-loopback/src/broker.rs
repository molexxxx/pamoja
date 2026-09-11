//! The shared in-memory broker that routes loopback messages.

use std::sync::{Arc, Mutex};

use pamoja_core::{topic_matches, Message};
use tokio::sync::mpsc::UnboundedSender;

/// A shared, in-process router for [`LoopbackTransport`](crate::LoopbackTransport)s.
///
/// Clone a single broker into every transport that should share a namespace; a
/// publish on one transport is delivered to every transport whose subscriptions
/// match the topic. The broker is cheap to clone, and all clones share one
/// routing table.
#[derive(Clone, Default)]
pub struct LoopbackBroker {
    subscriptions: Arc<Mutex<Vec<Subscription>>>,
}

/// One connected transport's topic filters and delivery channel.
struct Subscription {
    filters: Arc<Mutex<Vec<String>>>,
    sender: UnboundedSender<Message>,
}

impl LoopbackBroker {
    /// Creates an empty broker.
    ///
    /// # Returns
    ///
    /// A broker with no registered transports.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a transport's filters and delivery channel.
    pub(crate) fn register(
        &self,
        filters: Arc<Mutex<Vec<String>>>,
        sender: UnboundedSender<Message>,
    ) {
        self.subscriptions
            .lock()
            .expect("broker lock")
            .push(Subscription { filters, sender });
    }

    /// Delivers a message to every subscription whose filters match its topic,
    /// pruning channels whose receiver has been dropped.
    pub(crate) fn publish(&self, message: &Message) {
        let mut subscriptions = self.subscriptions.lock().expect("broker lock");
        subscriptions.retain(|subscription| {
            if subscription.sender.is_closed() {
                return false;
            }
            let matched = subscription
                .filters
                .lock()
                .expect("filters lock")
                .iter()
                .any(|filter| topic_matches(filter, &message.topic));
            if matched {
                let _ = subscription.sender.send(message.clone());
            }
            true
        });
    }
}
