//! The shared in-memory broker that routes loopback messages.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pamoja_core::{topic_matches, Error, Message, Result};
use tokio::sync::mpsc::UnboundedSender;

/// A shared, in-process router for [`LoopbackTransport`](crate::LoopbackTransport)s.
///
/// Clone a single broker into every transport that should share a namespace; a
/// publish on one transport is delivered to every transport whose subscriptions
/// match the topic. The broker is cheap to clone, and all clones share one
/// routing table.
///
/// A broker stands for the network its links share, so taking it out of reach
/// with [`set_reachable`](LoopbackBroker::set_reachable) is how a test puts every
/// link on it out of range at once, including links a ladder now owns.
#[derive(Clone)]
pub struct LoopbackBroker {
    subscriptions: Arc<Mutex<Vec<Subscription>>>,
    reachable: Arc<AtomicBool>,
}

/// One connected transport's topic filters and delivery channel.
struct Subscription {
    filters: Arc<Mutex<Vec<String>>>,
    sender: UnboundedSender<Message>,
}

impl Default for LoopbackBroker {
    fn default() -> Self {
        Self {
            subscriptions: Arc::default(),
            reachable: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl LoopbackBroker {
    /// Creates an empty broker.
    ///
    /// # Returns
    ///
    /// A broker with no registered transports, in reach.
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the broker out of reach, or brings it back.
    ///
    /// Out of reach, every link on the broker fails to connect, send, or
    /// subscribe with [`Error::Transport`], the error an out-of-range radio or a
    /// dropped network raises, and nothing is delivered. A link keeps its
    /// connection and its filters through the outage, so it carries traffic again
    /// as soon as the broker is back.
    ///
    /// # Arguments
    ///
    /// * `reachable` - `false` to take the broker out of reach, `true` to bring it
    ///   back.
    pub fn set_reachable(&self, reachable: bool) {
        self.reachable.store(reachable, Ordering::SeqCst);
    }

    /// Reports whether the broker is in reach.
    ///
    /// # Returns
    ///
    /// `true` unless [`set_reachable`](LoopbackBroker::set_reachable) took it out.
    pub fn is_reachable(&self) -> bool {
        self.reachable.load(Ordering::SeqCst)
    }

    /// Fails with the error a link on an unreachable broker reports.
    pub(crate) fn in_reach(&self) -> Result<()> {
        if self.is_reachable() {
            Ok(())
        } else {
            Err(Error::Transport("the broker is out of reach".to_owned()))
        }
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
