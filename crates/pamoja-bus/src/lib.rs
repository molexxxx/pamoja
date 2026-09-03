//! An in-memory typed publish/subscribe event bus.
//!
//! [`BroadcastBus`] implements the core [`EventBus`] trait
//! over a bounded broadcast channel: every event published is delivered to every
//! current subscriber. Producers such as sensors and transports publish events,
//! and consumers await them, all statically typed to one event type per bus.
//!
//! The bus is bounded, so a subscriber that falls far enough behind drops the
//! events it missed and resumes from the most recent ones. This keeps a slow
//! consumer from holding memory without bound, which matters on constrained
//! devices.

use pamoja_core::{EventBus, Result};
use tokio::sync::broadcast;

/// A typed publish/subscribe bus that broadcasts each event to all subscribers.
///
/// Every handle can both publish and receive. Use [`subscribe`](BroadcastBus::subscribe)
/// to add an independent consumer; an event published after a handle subscribes
/// is delivered to it. A subscriber only sees events published after it
/// subscribed, mirroring a live pub/sub channel.
///
/// # Examples
///
/// ```
/// use pamoja_core::EventBus;
/// use pamoja_bus::BroadcastBus;
///
/// # async fn run() -> pamoja_core::Result<()> {
/// let bus = BroadcastBus::new(16);
/// let mut subscriber = bus.subscribe();
/// bus.publish("reading").await?;
/// assert_eq!(subscriber.next_event().await?, Some("reading"));
/// # Ok(())
/// # }
/// ```
pub struct BroadcastBus<E> {
    sender: broadcast::Sender<E>,
    receiver: broadcast::Receiver<E>,
}

impl<E: Clone> BroadcastBus<E> {
    /// Creates a bus buffering up to `capacity` unread events per subscriber.
    ///
    /// # Arguments
    ///
    /// * `capacity` - the per-subscriber buffer depth; a subscriber further behind
    ///   than this drops the events it missed. Values below one are raised to one.
    ///
    /// # Returns
    ///
    /// A bus with one handle that can publish and receive.
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = broadcast::channel(capacity.max(1));
        Self { sender, receiver }
    }

    /// Creates another handle to the same bus with its own independent subscription.
    ///
    /// # Returns
    ///
    /// A handle that receives events published after this call and can also publish.
    pub fn subscribe(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.sender.subscribe(),
        }
    }
}

impl<E: Clone> EventBus for BroadcastBus<E> {
    type Event = E;

    async fn publish(&self, event: Self::Event) -> Result<()> {
        // `send` errors only when there are no receivers; this handle holds its
        // own, so a publish always succeeds.
        let _ = self.sender.send(event);
        Ok(())
    }

    async fn next_event(&mut self) -> Result<Option<Self::Event>> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => return Ok(Some(event)),
                Err(broadcast::error::RecvError::Closed) => return Ok(None),
                // The subscriber fell behind; skip the dropped events and resume.
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn delivers_a_published_event() {
        let mut bus = BroadcastBus::new(8);
        bus.publish(1).await.expect("publish");
        assert_eq!(bus.next_event().await.expect("next"), Some(1));
    }

    #[tokio::test]
    async fn fans_out_to_every_subscriber() {
        let bus = BroadcastBus::new(8);
        let mut first = bus.subscribe();
        let mut second = bus.subscribe();

        bus.publish("event").await.expect("publish");

        assert_eq!(first.next_event().await.expect("next"), Some("event"));
        assert_eq!(second.next_event().await.expect("next"), Some("event"));
    }

    #[tokio::test]
    async fn a_lagging_subscriber_skips_dropped_events_and_resumes() {
        let bus = BroadcastBus::new(2);
        let mut subscriber = bus.subscribe();

        for value in 0..5 {
            bus.publish(value).await.expect("publish");
        }

        // Capacity is two, so the three oldest events are dropped; the subscriber
        // resumes with the two most recent, in order.
        let mut seen = Vec::new();
        seen.push(subscriber.next_event().await.expect("next").expect("event"));
        seen.push(subscriber.next_event().await.expect("next").expect("event"));
        assert_eq!(seen, vec![3, 4]);
    }
}
