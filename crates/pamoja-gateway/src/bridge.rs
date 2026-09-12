//! Carrying messages between a radio and the link that leaves the site.
//!
//! A gateway is two links and a rule for moving traffic between them. On one side is the
//! air, which in a pamoja deployment is a radio carrying topics in mesh frames; on the other
//! is whatever reaches the network, usually a broker over the site uplink. A [`Bridge`] holds
//! both, forwards what the air hears to the upstream link, delivers what comes back down,
//! and counts what it moved.
//!
//! Both sides are ordinary pamoja transports, so the same bridge carries a LoRa mesh to MQTT,
//! a serial link to CoAP, or a test queue to another test queue. Nothing here opens a socket
//! or names a radio.
//!
//! A prefix keeps one broker able to hold many sites, and a direction segment keeps a bridge
//! from talking to itself: what the air heard is published as `<prefix>/up/<topic>`, and what
//! the site is sent is subscribed as `<prefix>/down/<filter>` and delivered to the air with
//! both parts taken off. The two spaces never overlap, so a broker handing a publisher its
//! own messages back, as brokers do, cannot send a reading round again as a command.
//!
//! # Examples
//!
//! A reading heard on one link arrives on the other, under the site prefix.
//!
//! ```
//! use pamoja_core::Transport;
//! use pamoja_gateway::bridge::Bridge;
//! use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
//!
//! # let runtime = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
//! # runtime.block_on(async {
//! let air = LoopbackTransport::new(LoopbackBroker::new());
//! let upstream = LoopbackTransport::new(LoopbackBroker::new());
//! let mut bridge = Bridge::new(air, upstream).with_prefix("sites/ridge");
//! bridge.connect().await.expect("both links connect");
//!
//! // A node publishes on the air, and one pass carries it upstream, where it arrives as
//! // sites/ridge/up/soil/1.
//! bridge.air_mut().send_text("soil/1", "21.5").await.expect("the air takes it");
//! assert_eq!(bridge.tick().await.expect("the pass runs"), 1);
//! assert_eq!(bridge.traffic().forwarded, 1);
//! assert_eq!(bridge.traffic().delivered, 0);
//! # });
//! ```

use pamoja_core::{topic_matches, Message, Receive, Result, Transport};

/// The segment a site publishes under, which is everything its nodes said.
pub const UPLINK_SEGMENT: &str = "up";

/// The segment a site listens under, which is everything it is told.
pub const DOWNLINK_SEGMENT: &str = "down";

/// What a bridge has moved since it was built.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Traffic {
    /// Messages heard on the air.
    pub heard: u64,
    /// Messages passed upstream.
    pub forwarded: u64,
    /// Messages delivered from upstream to the air.
    pub delivered: u64,
    /// Messages dropped because no filter selected them.
    pub dropped: u64,
}

/// Which way a message crossed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// From the air to the link that leaves the site.
    Up,
    /// From that link back to the air.
    Down,
}

/// One message that crossed, for a caller that shows what a gateway is doing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Crossing {
    /// Which way it went.
    pub direction: Direction,
    /// The topic it went out on, as the side receiving it sees the topic.
    pub topic: String,
    /// What it carried, so whatever watches the traffic reads the same bytes the far side
    /// does rather than asking the link for them again.
    pub payload: Vec<u8>,
}

/// Two links and the rule that moves traffic between them.
///
/// The air side is read for anything a node published; the upstream side is read for anything
/// the network sent back. What crosses in each direction is chosen by filters, which follow
/// MQTT wildcards, and every topic crossing upward gains the site prefix while every topic
/// crossing downward loses it.
pub struct Bridge<A, U> {
    air: A,
    upstream: U,
    prefix: Option<String>,
    forward: Vec<String>,
    deliver: Vec<String>,
    traffic: Traffic,
}

impl<A, U> Bridge<A, U>
where
    A: Transport + Receive,
    U: Transport + Receive,
{
    /// Builds a bridge over two links, carrying everything both ways.
    ///
    /// # Arguments
    ///
    /// * `air` - the link the nodes are on.
    /// * `upstream` - the link that leaves the site.
    ///
    /// # Returns
    ///
    /// The bridge, with no prefix and no filters.
    pub fn new(air: A, upstream: U) -> Bridge<A, U> {
        Bridge {
            air,
            upstream,
            prefix: None,
            forward: Vec::new(),
            deliver: Vec::new(),
            traffic: Traffic::default(),
        }
    }

    /// Sets the prefix that names this site upstream.
    ///
    /// # Arguments
    ///
    /// * `prefix` - the prefix, without a trailing separator.
    ///
    /// # Returns
    ///
    /// The updated bridge, for chaining.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Bridge<A, U> {
        let prefix = prefix.into();
        self.prefix = (!prefix.is_empty()).then_some(prefix);
        self
    }

    /// Carries only the air topics a filter selects; add more than one to carry several.
    ///
    /// # Arguments
    ///
    /// * `filter` - an MQTT topic filter.
    ///
    /// # Returns
    ///
    /// The updated bridge, for chaining.
    pub fn forwarding(mut self, filter: impl Into<String>) -> Bridge<A, U> {
        self.forward.push(filter.into());
        self
    }

    /// Delivers only the upstream topics a filter selects, matched after the prefix is taken
    /// off.
    ///
    /// # Arguments
    ///
    /// * `filter` - an MQTT topic filter.
    ///
    /// # Returns
    ///
    /// The updated bridge, for chaining.
    pub fn delivering(mut self, filter: impl Into<String>) -> Bridge<A, U> {
        self.deliver.push(filter.into());
        self
    }

    /// Returns what has crossed so far.
    ///
    /// # Returns
    ///
    /// The counts.
    pub const fn traffic(&self) -> Traffic {
        self.traffic
    }

    /// Returns the air side, to read its own state.
    ///
    /// # Returns
    ///
    /// The link the nodes are on.
    pub const fn air(&self) -> &A {
        &self.air
    }

    /// Returns the air side for sending directly, which a test or a local command needs.
    ///
    /// # Returns
    ///
    /// The link the nodes are on.
    pub fn air_mut(&mut self) -> &mut A {
        &mut self.air
    }

    /// Returns the upstream side.
    ///
    /// # Returns
    ///
    /// The link that leaves the site.
    pub const fn upstream(&self) -> &U {
        &self.upstream
    }

    /// Returns the upstream side for sending directly.
    ///
    /// # Returns
    ///
    /// The link that leaves the site.
    pub fn upstream_mut(&mut self) -> &mut U {
        &mut self.upstream
    }

    /// Connects both links and subscribes each to what it must carry.
    ///
    /// # Returns
    ///
    /// `Ok(())` once both links are connected and subscribed.
    ///
    /// # Errors
    ///
    /// Whatever either link returns from connecting or subscribing.
    pub async fn connect(&mut self) -> Result<()> {
        self.air.connect().await?;
        self.upstream.connect().await?;

        if self.forward.is_empty() {
            self.air.subscribe("#").await?;
        } else {
            for filter in &self.forward {
                self.air.subscribe(filter).await?;
            }
        }

        if self.deliver.is_empty() {
            let topic = self.down_filter("#");
            self.upstream.subscribe(&topic).await?;
        } else {
            for filter in &self.deliver {
                let topic = self.down_filter(filter);
                self.upstream.subscribe(&topic).await?;
            }
        }

        Ok(())
    }

    /// Carries whatever is waiting on either link, without blocking on an empty one.
    ///
    /// This is the pass a caller drives itself, in a test or beside other work. A message
    /// only crosses when its link already holds one, so a quiet bridge returns at once.
    ///
    /// # Returns
    ///
    /// How many messages crossed, in both directions together.
    ///
    /// # Errors
    ///
    /// Whatever either link returns while receiving or sending.
    pub async fn tick(&mut self) -> Result<usize> {
        let mut crossed = 0;

        while let Some(message) = self.ready(Side::Air).await? {
            crossed += usize::from(self.carry_up(message).await?.is_some());
        }
        while let Some(message) = self.ready(Side::Upstream).await? {
            crossed += usize::from(self.carry_down(message).await?.is_some());
        }

        Ok(crossed)
    }

    /// Carries traffic until one of the links ends.
    ///
    /// Both links are awaited together, so whichever speaks first is carried and neither
    /// waits on the other. It returns when a link reports that no further messages will
    /// arrive, which is how a closed radio or a dropped uplink ends the loop.
    ///
    /// # Returns
    ///
    /// `Ok(())` once one of the links has ended.
    ///
    /// # Errors
    ///
    /// Whatever either link returns while receiving or sending.
    pub async fn run(&mut self) -> Result<()> {
        while self.carry_once().await?.is_some() {}
        Ok(())
    }

    /// Carries the next message either link offers, and reports what it was.
    ///
    /// This is the pass to drive when something else must see the traffic: a dashboard
    /// counting what each node sent, a log, or a rule watching a topic. Both links are
    /// awaited together, so whichever speaks first is carried. A message no filter selects
    /// is counted and passed over, and the pass goes on waiting for one that does cross.
    ///
    /// # Returns
    ///
    /// What crossed, or `None` once one of the links has ended.
    ///
    /// # Errors
    ///
    /// Whatever either link returns while receiving or sending.
    pub async fn carry_once(&mut self) -> Result<Option<Crossing>> {
        loop {
            let carried = tokio::select! {
                heard = self.air.recv() => heard?.map(|message| (Side::Air, message)),
                sent = self.upstream.recv() => sent?.map(|message| (Side::Upstream, message)),
            };

            let Some((side, message)) = carried else {
                return Ok(None);
            };
            let payload = message.payload.clone();

            let crossed = match side {
                Side::Air => self.carry_up(message).await?.map(|topic| Crossing {
                    direction: Direction::Up,
                    topic,
                    payload,
                }),
                Side::Upstream => self.carry_down(message).await?.map(|topic| Crossing {
                    direction: Direction::Down,
                    topic,
                    payload,
                }),
            };

            if crossed.is_some() {
                return Ok(crossed);
            }
        }
    }

    // Takes a message already waiting on one side, and nothing if it is quiet.
    async fn ready(&mut self, side: Side) -> Result<Option<Message>> {
        let waiting = match side {
            Side::Air => now_or_never(self.air.recv()).await,
            Side::Upstream => now_or_never(self.upstream.recv()).await,
        };

        match waiting {
            Some(result) => result,
            None => Ok(None),
        }
    }

    // Sends what the air heard upstream, under the site prefix, and reports the topic it
    // went out on, or nothing when a filter refused it.
    async fn carry_up(&mut self, message: Message) -> Result<Option<String>> {
        self.traffic.heard += 1;
        if !self.selected(&self.forward, &message.topic) {
            self.traffic.dropped += 1;
            return Ok(None);
        }

        let topic = self.up_topic(&message.topic);
        self.upstream.send(&topic, &message.payload).await?;
        self.traffic.forwarded += 1;
        Ok(Some(topic))
    }

    // Sends what came back down onto the air, with the site and direction taken off, and
    // reports the topic a node sees, or nothing when it was not addressed to this site.
    async fn carry_down(&mut self, message: Message) -> Result<Option<String>> {
        let Some(topic) = self.air_topic(&message.topic) else {
            self.traffic.dropped += 1;
            return Ok(None);
        };
        if !self.selected(&self.deliver, &topic) {
            self.traffic.dropped += 1;
            return Ok(None);
        }

        self.air.send(&topic, &message.payload).await?;
        self.traffic.delivered += 1;
        Ok(Some(topic))
    }

    // No filters carries everything; otherwise one of them must select the topic.
    fn selected(&self, filters: &[String], topic: &str) -> bool {
        filters.is_empty() || filters.iter().any(|filter| topic_matches(filter, topic))
    }

    // The topic an air topic travels under upstream.
    fn up_topic(&self, topic: &str) -> String {
        match &self.prefix {
            Some(prefix) => format!("{prefix}/{UPLINK_SEGMENT}/{topic}"),
            None => format!("{UPLINK_SEGMENT}/{topic}"),
        }
    }

    // The upstream filter a downlink filter is subscribed as.
    fn down_filter(&self, filter: &str) -> String {
        match &self.prefix {
            Some(prefix) => format!("{prefix}/{DOWNLINK_SEGMENT}/{filter}"),
            None => format!("{DOWNLINK_SEGMENT}/{filter}"),
        }
    }

    // The air topic an upstream topic names, or nothing when it belongs to another site or is
    // a reading this bridge published rather than a command it was sent.
    fn air_topic(&self, topic: &str) -> Option<String> {
        let rest = match &self.prefix {
            Some(prefix) => topic
                .strip_prefix(prefix.as_str())
                .and_then(|rest| rest.strip_prefix('/'))?,
            None => topic,
        };
        rest.strip_prefix(DOWNLINK_SEGMENT)
            .and_then(|rest| rest.strip_prefix('/'))
            .map(str::to_owned)
    }
}

// Which link a message came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Air,
    Upstream,
}

// Polls a future once, which is how a pass takes only what is already waiting. The future is
// dropped when it is not ready, and every transport here keeps an undelivered message queued
// for the next call, which is the cancel safety `Receive` requires.
async fn now_or_never<F: core::future::Future>(future: F) -> Option<F::Output> {
    let mut future = core::pin::pin!(future);
    core::future::poll_fn(move |context| {
        core::task::Poll::Ready(match future.as_mut().poll(context) {
            core::task::Poll::Ready(value) => Some(value),
            core::task::Poll::Pending => None,
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::VecDeque;

    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};

    // A link that hands over everything it holds and keeps what it was told to send. A radio
    // behaves this way: it hears what is on the air, and filtering is a matter for whatever
    // reads it rather than for the link.
    #[derive(Default)]
    struct Queue {
        waiting: VecDeque<Message>,
        sent: Vec<Message>,
        subscribed: Vec<String>,
    }

    impl Queue {
        fn hearing(messages: &[(&str, &str)]) -> Queue {
            Queue {
                waiting: messages
                    .iter()
                    .map(|(topic, payload)| Message::new(*topic, payload.as_bytes()))
                    .collect(),
                sent: Vec::new(),
                subscribed: Vec::new(),
            }
        }
    }

    impl Transport for Queue {
        async fn connect(&mut self) -> Result<()> {
            Ok(())
        }

        async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
            self.sent.push(Message::new(topic, payload));
            Ok(())
        }

        async fn subscribe(&mut self, topic: &str) -> Result<()> {
            self.subscribed.push(topic.to_owned());
            Ok(())
        }
    }

    impl Receive for Queue {
        async fn recv(&mut self) -> Result<Option<Message>> {
            match self.waiting.pop_front() {
                Some(message) => Ok(Some(message)),
                // A returned None means the link has ended, which an empty queue has not,
                // so it waits the way a real link waits.
                None => core::future::pending::<Result<Option<Message>>>().await,
            }
        }
    }

    #[tokio::test]
    async fn each_side_subscribes_to_its_own_direction() {
        let mut bridge = Bridge::new(Queue::default(), Queue::default()).with_prefix("sites/ridge");
        bridge.connect().await.expect("both links connect");

        assert_eq!(bridge.air().subscribed, ["#"]);
        assert_eq!(bridge.upstream().subscribed, ["sites/ridge/down/#"]);
    }

    #[tokio::test]
    async fn a_reading_goes_up_under_the_site_prefix() {
        let mut bridge = Bridge::new(Queue::hearing(&[("soil/1", "21.5")]), Queue::default())
            .with_prefix("sites/ridge");
        bridge.connect().await.expect("both links connect");
        assert_eq!(bridge.tick().await.expect("the pass runs"), 1);

        let sent = &bridge.upstream().sent;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].topic, "sites/ridge/up/soil/1");
        assert_eq!(sent[0].payload, b"21.5");
        assert_eq!(bridge.traffic().forwarded, 1);
        assert_eq!(bridge.traffic().heard, 1);
    }

    #[tokio::test]
    async fn a_command_comes_down_with_the_site_and_direction_taken_off() {
        let mut bridge = Bridge::new(
            Queue::default(),
            Queue::hearing(&[("sites/ridge/down/valve/1/set", "open")]),
        )
        .with_prefix("sites/ridge");
        bridge.connect().await.expect("both links connect");
        assert_eq!(bridge.tick().await.expect("the pass runs"), 1);

        let sent = &bridge.air().sent;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].topic, "valve/1/set");
        assert_eq!(sent[0].payload, b"open");
        assert_eq!(bridge.traffic().delivered, 1);
    }

    #[tokio::test]
    async fn what_goes_up_does_not_come_back_down() {
        // A broker hands a publisher its own messages back, so the two directions live in
        // separate spaces: a reading published upstream cannot arrive as a command.
        let mut bridge = Bridge::new(
            LoopbackTransport::new(LoopbackBroker::new()),
            LoopbackTransport::new(LoopbackBroker::new()),
        )
        .with_prefix("sites/ridge");
        bridge.connect().await.expect("both links connect");

        bridge
            .air_mut()
            .send_text("soil/1", "21.5")
            .await
            .expect("the air takes it");
        assert_eq!(bridge.tick().await.expect("the pass runs"), 1);
        assert_eq!(bridge.traffic().forwarded, 1);
        assert_eq!(bridge.traffic().delivered, 0);
    }

    #[tokio::test]
    async fn another_sites_traffic_is_left_alone() {
        let mut bridge = Bridge::new(
            Queue::default(),
            Queue::hearing(&[("sites/valley/down/valve/1/set", "open")]),
        )
        .with_prefix("sites/ridge");
        bridge.connect().await.expect("both links connect");
        bridge.tick().await.expect("the pass runs");

        assert!(bridge.air().sent.is_empty());
        assert_eq!(bridge.traffic().delivered, 0);
        assert_eq!(bridge.traffic().dropped, 1);
    }

    #[tokio::test]
    async fn only_what_a_filter_selects_crosses() {
        let mut bridge = Bridge::new(
            Queue::hearing(&[("soil/1", "21.5"), ("debug/log", "noise")]),
            Queue::default(),
        )
        .forwarding("soil/#");
        bridge.connect().await.expect("both links connect");
        bridge.tick().await.expect("the pass runs");

        assert_eq!(bridge.traffic().heard, 2);
        assert_eq!(bridge.traffic().forwarded, 1);
        assert_eq!(bridge.traffic().dropped, 1);
        assert_eq!(bridge.upstream().sent.len(), 1);
        assert_eq!(bridge.upstream().sent[0].topic, "up/soil/1");
    }

    #[tokio::test]
    async fn each_crossing_is_reported_to_whoever_drives_it() {
        let mut bridge = Bridge::new(
            Queue::hearing(&[("debug/log", "noise"), ("soil/1", "21.5")]),
            Queue::default(),
        )
        .with_prefix("sites/ridge")
        .forwarding("soil/#");
        bridge.connect().await.expect("both links connect");

        let crossed = bridge
            .carry_once()
            .await
            .expect("a pass runs")
            .expect("one message crosses");
        assert_eq!(crossed.direction, Direction::Up);
        assert_eq!(crossed.topic, "sites/ridge/up/soil/1");
        assert_eq!(crossed.payload, b"21.5");
        assert_eq!(bridge.traffic().dropped, 1);
        assert_eq!(bridge.traffic().forwarded, 1);
    }

    #[tokio::test]
    async fn a_quiet_bridge_returns_at_once() {
        let mut bridge = Bridge::new(Queue::default(), Queue::default());
        bridge.connect().await.expect("both links connect");
        assert_eq!(bridge.tick().await.expect("the pass runs"), 0);
        assert_eq!(bridge.traffic(), Traffic::default());
    }
}
