//! A transport decorator that simulates a degraded radio link.

use pamoja_core::{Error, Result, Transport};

/// A [`Transport`] decorator that simulates an ongoing degraded link.
///
/// Where `Faulty` in `pamoja-loopback` fails a fixed number of upcoming sends, a
/// `DegradedLink` models a link that stays bad, so offline-first behavior can be
/// proven against a realistic pattern rather than a one-shot outage. It can drop a
/// configurable fraction of sends (a lossy radio) and cycle between reachable and
/// unreachable windows (a link that comes and goes). Both are deterministic - driven
/// by a send counter, not a clock or randomness - so a store-and-forward drain over
/// the link behaves the same way every run.
///
/// Connect and subscribe pass straight through; only [`send`](Transport::send) is
/// degraded, since that is the path store-and-forward depends on. A degraded send
/// returns [`Error::Transport`], which [`drain_to`](https://docs.rs/pamoja-sync)
/// leaves buffered, in order, to retry later.
///
/// # Examples
///
/// ```
/// use pamoja_core::Transport;
/// use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
/// use pamoja_sim::DegradedLink;
///
/// # async fn run() -> pamoja_core::Result<()> {
/// let broker = LoopbackBroker::new();
/// // A link that drops every second packet.
/// let mut link = DegradedLink::new(LoopbackTransport::new(broker)).drop_every(2);
/// link.connect().await?;
///
/// link.send("t", b"1").await?; // first send: delivered
/// assert!(link.send("t", b"2").await.is_err()); // second send: dropped
/// link.send("t", b"3").await?; // third send: delivered
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct DegradedLink<T> {
    inner: T,
    drop_every: u32,
    window: Option<(u32, u32)>,
    sends: u32,
}

impl<T> DegradedLink<T> {
    /// Wraps `inner` as a perfect link, until loss or intermittency is added.
    ///
    /// # Arguments
    ///
    /// * `inner` - the transport to decorate.
    ///
    /// # Returns
    ///
    /// A decorator that passes every send through until configured otherwise.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            drop_every: 0,
            window: None,
            sends: 0,
        }
    }

    /// Drops one in every `n` sends, simulating a lossy link.
    ///
    /// # Arguments
    ///
    /// * `n` - drop every `n`th send; `0` disables loss.
    ///
    /// # Returns
    ///
    /// The updated link, for chaining.
    pub fn drop_every(mut self, n: u32) -> Self {
        self.drop_every = n;
        self
    }

    /// Cycles between `up` reachable sends and `down` unreachable sends.
    ///
    /// Sends rejected during a down window return [`Error::Transport`], the same as a
    /// real link that is temporarily out of range.
    ///
    /// # Arguments
    ///
    /// * `up` - the number of sends that succeed at the start of each cycle.
    /// * `down` - the number of sends that fail before the cycle repeats.
    ///
    /// # Returns
    ///
    /// The updated link, for chaining.
    pub fn intermittent(mut self, up: u32, down: u32) -> Self {
        self.window = if up + down == 0 {
            None
        } else {
            Some((up, down))
        };
        self
    }

    /// Unwraps the decorator, returning the inner transport.
    ///
    /// # Returns
    ///
    /// The wrapped transport.
    pub fn into_inner(self) -> T {
        self.inner
    }

    // Whether the current send falls in a down window of the intermittent cycle.
    fn link_is_down(&self) -> bool {
        match self.window {
            Some((up, down)) => (self.sends - 1) % (up + down) >= up,
            None => false,
        }
    }

    // Whether the current send is the one dropped by the loss pattern.
    fn packet_lost(&self) -> bool {
        self.drop_every != 0 && self.sends % self.drop_every == 0
    }
}

impl<T: Transport + Send> Transport for DegradedLink<T> {
    async fn connect(&mut self) -> Result<()> {
        self.inner.connect().await
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        self.sends += 1;
        if self.link_is_down() {
            return Err(Error::Transport("link unreachable".to_owned()));
        }
        if self.packet_lost() {
            return Err(Error::Transport("packet lost on a lossy link".to_owned()));
        }
        self.inner.send(topic, payload).await
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        self.inner.subscribe(topic).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A transport that records the payloads it successfully sends.
    #[derive(Default)]
    struct CountingTransport {
        sent: Vec<Vec<u8>>,
    }

    impl Transport for CountingTransport {
        async fn connect(&mut self) -> Result<()> {
            Ok(())
        }

        async fn send(&mut self, _topic: &str, payload: &[u8]) -> Result<()> {
            self.sent.push(payload.to_vec());
            Ok(())
        }

        async fn subscribe(&mut self, _topic: &str) -> Result<()> {
            Ok(())
        }
    }

    async fn send_seq(link: &mut DegradedLink<CountingTransport>, count: u8) -> usize {
        let mut errors = 0;
        for i in 1..=count {
            if link.send("t", &[i]).await.is_err() {
                errors += 1;
            }
        }
        errors
    }

    #[tokio::test]
    async fn a_perfect_link_passes_every_send() {
        let mut link = DegradedLink::new(CountingTransport::default());
        assert_eq!(send_seq(&mut link, 3).await, 0);
        assert_eq!(link.into_inner().sent.len(), 3);
    }

    #[tokio::test]
    async fn loss_drops_every_nth_send() {
        let inner = CountingTransport::default();
        let mut link = DegradedLink::new(inner).drop_every(3);
        assert_eq!(send_seq(&mut link, 6).await, 2); // sends 3 and 6 are dropped
        assert_eq!(
            link.into_inner().sent,
            vec![vec![1], vec![2], vec![4], vec![5]]
        );
    }

    #[tokio::test]
    async fn intermittency_cycles_between_up_and_down() {
        let inner = CountingTransport::default();
        let mut link = DegradedLink::new(inner).intermittent(2, 1);
        // Period of three: two through, one rejected, repeating.
        assert_eq!(send_seq(&mut link, 6).await, 2); // sends 3 and 6 fail
        assert_eq!(
            link.into_inner().sent,
            vec![vec![1], vec![2], vec![4], vec![5]]
        );
    }

    #[tokio::test]
    async fn a_retry_after_a_drop_eventually_gets_through() {
        // The same payload, retried, advances the counter until a send lands.
        let inner = CountingTransport::default();
        let mut link = DegradedLink::new(inner).intermittent(1, 1);
        assert!(link.send("t", b"x").await.is_ok()); // up
        assert!(link.send("t", b"x").await.is_err()); // down
        assert!(link.send("t", b"x").await.is_ok()); // up again
        assert_eq!(link.into_inner().sent.len(), 2);
    }
}
