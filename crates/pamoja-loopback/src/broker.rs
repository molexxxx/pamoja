//! The shared in-memory broker that routes loopback messages.

use std::sync::{Arc, Mutex};

use pamoja_core::Message;
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

/// Returns whether an MQTT-style topic `filter` matches a concrete `topic`.
///
/// `+` matches exactly one level, and `#` matches the remaining levels including
/// none. Following the MQTT specification, a filter that begins with a wildcard does
/// not match a topic that begins with `$`, which is reserved for system topics.
fn topic_matches(filter: &str, topic: &str) -> bool {
    if topic.starts_with('$') {
        if let Some(first) = filter.split('/').next() {
            if first == "#" || first == "+" {
                return false;
            }
        }
    }
    let mut filter_levels = filter.split('/');
    let mut topic_levels = topic.split('/');
    loop {
        match (filter_levels.next(), topic_levels.next()) {
            (Some("#"), _) => return true,
            (Some("+"), Some(_)) => {}
            (Some(filter_level), Some(topic_level)) if filter_level == topic_level => {}
            (None, None) => return true,
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::topic_matches;

    #[test]
    fn exact_topics_match() {
        assert!(topic_matches("a/b/c", "a/b/c"));
        assert!(!topic_matches("a/b/c", "a/b/d"));
        assert!(!topic_matches("a/b", "a/b/c"));
        assert!(!topic_matches("a/b/c", "a/b"));
    }

    #[test]
    fn single_level_wildcard_matches_one_level() {
        assert!(topic_matches("a/+/c", "a/b/c"));
        assert!(topic_matches(
            "sensors/+/temperature",
            "sensors/1/temperature"
        ));
        assert!(!topic_matches("a/+/c", "a/b/c/d"));
        assert!(!topic_matches("a/+", "a"));
    }

    #[test]
    fn multi_level_wildcard_matches_the_rest() {
        assert!(topic_matches("a/#", "a/b/c"));
        assert!(topic_matches("a/#", "a"));
        assert!(topic_matches("#", "a/b/c"));
        assert!(!topic_matches("a/#", "b/c"));
    }

    #[test]
    fn leading_wildcards_do_not_match_dollar_topics() {
        // MQTT reserves $-prefixed topics from wildcard subscriptions.
        assert!(!topic_matches("#", "$SYS/broker/uptime"));
        assert!(!topic_matches("+/broker", "$SYS/broker"));
        // An explicit filter still reaches a $ topic.
        assert!(topic_matches("$SYS/#", "$SYS/broker/uptime"));
        assert!(topic_matches("$SYS/+", "$SYS/uptime"));
    }
}
