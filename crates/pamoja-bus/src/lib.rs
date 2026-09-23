//! An in-memory typed publish/subscribe event bus.
//!
//! [`BroadcastBus`] implements the core [`EventBus`] trait
//! over a bounded broadcast channel: every event published is delivered to every
//! current subscriber. Producers such as sensors and transports publish events,
//! and consumers await them, all statically typed to one event type per bus.
//!
//! [`EventPublisher`] is the handle for a part that only announces. It has no
//! queue of its own, it clones into every task and callback that publishes, and
//! publishing from it never waits, so it can announce while a subscriber on the
//! same bus is waiting for its next event.
//!
//! The bus is bounded, so a subscriber that falls far enough behind drops the
//! events it missed and resumes from the most recent ones, and
//! [`BroadcastBus::missed`] counts what it lost. This keeps a slow consumer from
//! holding memory without bound, which matters on constrained devices.
//!
//! # Examples
//!
//! A pump controller's events reach a logger and a dashboard: each subscriber hears every
//! event published after it joined, in order.
//!
//! ```
//! use pamoja_bus::BroadcastBus;
//! use pamoja_core::EventBus;
//!
//! # tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
//! let bus = BroadcastBus::new(16);
//! let mut logger = bus.subscribe();
//! let mut dashboard = bus.subscribe();
//!
//! bus.publish("pump started").await?;
//! bus.publish("pressure low").await?;
//! assert_eq!(logger.next_event().await?, Some("pump started"));
//! assert_eq!(logger.next_event().await?, Some("pressure low"));
//! assert_eq!(dashboard.next_event().await?, Some("pump started"));
//! # Ok::<(), pamoja_core::Error>(())
//! # }).unwrap();
//! ```

use pamoja_core::{EventBus, Result};
use tokio::sync::broadcast;

/// The largest buffer a bus sets aside; a larger capacity is lowered to it.
///
/// The buffer is allocated in full when the bus is made, so a mistaken capacity
/// would otherwise cost its memory at once, or fail the allocation outright.
pub const MAX_CAPACITY: usize = 1 << 20;

/// Brings a requested capacity into the range a bus accepts.
fn bounded(capacity: usize) -> usize {
    capacity.clamp(1, MAX_CAPACITY)
}

/// A typed publish/subscribe bus that broadcasts each event to all subscribers.
///
/// Every handle can both publish and receive, and it receives what it publishes
/// itself. Use [`subscribe`](BroadcastBus::subscribe) to add an independent
/// consumer; an event published after a handle subscribes is delivered to it. A
/// subscriber only sees events published after it subscribed, mirroring a live
/// pub/sub channel.
///
/// Each handle holds a publisher of its own, so the bus stays open while any
/// handle exists and [`next_event`](EventBus::next_event) on a `BroadcastBus`
/// never returns `None`: it waits until an event arrives. A wait given up, by a
/// timeout or a `select!`, takes no event.
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
    missed: u64,
}

impl<E: Clone> BroadcastBus<E> {
    /// Creates a bus buffering unread events for each subscriber.
    ///
    /// # Arguments
    ///
    /// * `capacity` - the per-subscriber buffer depth, rounded up to the next
    ///   power of two; a subscriber further behind than the buffer holds drops
    ///   the events it missed. Values below one are raised to one, and values
    ///   above [`MAX_CAPACITY`] are lowered to it.
    ///
    /// # Returns
    ///
    /// A bus with one handle that can publish and receive.
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = broadcast::channel(bounded(capacity));
        Self {
            sender,
            receiver,
            missed: 0,
        }
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
            missed: 0,
        }
    }

    /// Creates a publish-only handle to the same bus.
    ///
    /// # Returns
    ///
    /// A handle that publishes without borrowing this one, so it can announce from
    /// another task while this handle waits in [`next_event`](EventBus::next_event).
    pub fn publisher(&self) -> EventPublisher<E> {
        EventPublisher {
            sender: self.sender.clone(),
        }
    }

    /// Counts the events this handle lost by falling behind.
    ///
    /// # Returns
    ///
    /// How many events were dropped from this handle's buffer before it read
    /// them, since it subscribed.
    pub fn missed(&self) -> u64 {
        self.missed
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
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    self.missed = self.missed.saturating_add(skipped);
                }
            }
        }
    }
}

/// A publish-only handle to a bus.
///
/// A publisher has no buffer to fill, so a part that only announces holds one
/// rather than a [`BroadcastBus`]. Cloning it is cheap, and publishing never
/// waits: a subscriber that falls behind loses events rather than holding up the
/// publisher.
///
/// # Examples
///
/// ```
/// use pamoja_core::EventBus;
/// use pamoja_bus::EventPublisher;
///
/// # async fn run() -> pamoja_core::Result<()> {
/// let power = EventPublisher::new(8);
/// let mut control = power.subscribe();
/// let mut logger = power.subscribe();
/// assert_eq!(power.publish("battery.low"), 2);
/// assert_eq!(control.next_event().await?, Some("battery.low"));
/// assert_eq!(logger.next_event().await?, Some("battery.low"));
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct EventPublisher<E> {
    sender: broadcast::Sender<E>,
}

impl<E: Clone> EventPublisher<E> {
    /// Creates a bus with no subscribers yet, and a publisher on it.
    ///
    /// # Arguments
    ///
    /// * `capacity` - the per-subscriber buffer depth, rounded up to the next
    ///   power of two. Values below one are raised to one, and values above
    ///   [`MAX_CAPACITY`] are lowered to it.
    ///
    /// # Returns
    ///
    /// A publisher to take subscribers from and publish to them.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(bounded(capacity));
        Self { sender }
    }

    /// Hands an event to every current subscriber.
    ///
    /// # Arguments
    ///
    /// * `event` - the event to broadcast.
    ///
    /// # Returns
    ///
    /// How many subscribers the event was handed to. An event published while no
    /// one is subscribed is dropped, and the count is 0.
    pub fn publish(&self, event: E) -> usize {
        self.sender.send(event).unwrap_or(0)
    }

    /// Subscribes to the bus.
    ///
    /// # Returns
    ///
    /// A handle that receives events published after this call and can also publish.
    pub fn subscribe(&self) -> BroadcastBus<E> {
        BroadcastBus {
            sender: self.sender.clone(),
            receiver: self.sender.subscribe(),
            missed: 0,
        }
    }

    /// Takes another publisher on the same bus, for another part that announces.
    ///
    /// # Returns
    ///
    /// A publisher with a lifetime of its own, the same as a clone of this one.
    pub fn publisher(&self) -> EventPublisher<E> {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

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
    async fn a_lagging_subscriber_skips_dropped_events_and_counts_them() {
        let bus = BroadcastBus::new(2);
        let mut subscriber = bus.subscribe();

        for value in 0..5 {
            bus.publish(value).await.expect("publish");
        }

        let mut seen = Vec::new();
        seen.push(subscriber.next_event().await.expect("next").expect("event"));
        seen.push(subscriber.next_event().await.expect("next").expect("event"));
        assert_eq!(seen, vec![3, 4]);
        assert_eq!(subscriber.missed(), 3);
    }

    #[tokio::test]
    async fn the_capacity_is_rounded_up_to_a_power_of_two() {
        let bus = BroadcastBus::new(3);
        let mut subscriber = bus.subscribe();

        for value in 0..5 {
            bus.publish(value).await.expect("publish");
        }

        assert_eq!(subscriber.next_event().await.expect("next"), Some(1));
        assert_eq!(subscriber.missed(), 1);
    }

    #[tokio::test]
    async fn a_capacity_beyond_the_maximum_is_lowered_rather_than_allocated() {
        let bus = BroadcastBus::new(usize::MAX);
        let mut subscriber = bus.subscribe();

        bus.publish(1u8).await.expect("publish");

        assert_eq!(subscriber.next_event().await.expect("next"), Some(1));
        assert_eq!(bounded(usize::MAX), MAX_CAPACITY);
        assert_eq!(bounded(0), 1);
    }

    #[tokio::test]
    async fn a_publisher_counts_the_subscribers_it_hands_an_event_to() {
        let power = EventPublisher::new(8);
        let mut control = power.subscribe();
        let mut logger = power.subscribe();

        assert_eq!(power.publish("battery.low"), 2);

        assert_eq!(
            control.next_event().await.expect("next"),
            Some("battery.low")
        );
        assert_eq!(
            logger.next_event().await.expect("next"),
            Some("battery.low")
        );
    }

    #[tokio::test]
    async fn an_event_published_to_no_subscribers_is_dropped() {
        let power = EventPublisher::new(8);
        assert_eq!(power.publish("before"), 0);

        let mut late = power.subscribe();
        assert_eq!(power.publish("after"), 1);
        assert_eq!(late.next_event().await.expect("next"), Some("after"));
    }

    #[tokio::test]
    async fn a_publisher_announces_while_a_subscriber_waits() {
        let mut subscriber = BroadcastBus::new(4);
        let publisher = subscriber.publisher();

        let waiting = tokio::spawn(async move { subscriber.next_event().await });
        tokio::task::yield_now().await;

        assert_eq!(publisher.publish(7), 1);
        let event = waiting.await.expect("the waiting task").expect("next");
        assert_eq!(event, Some(7));
    }

    #[tokio::test]
    async fn the_bus_stays_open_while_a_subscriber_holds_it() {
        let power: EventPublisher<u8> = EventPublisher::new(2);
        let mut subscriber = power.subscribe();
        drop(power);

        let wait = tokio::time::timeout(Duration::from_millis(20), subscriber.next_event()).await;
        assert!(wait.is_err(), "the wait ended with {wait:?}");
    }
}
