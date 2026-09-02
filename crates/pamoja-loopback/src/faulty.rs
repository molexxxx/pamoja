//! A transport decorator that injects send failures for degraded-link testing.

use pamoja_core::{Error, Result, Transport};

/// Wraps a [`Transport`] and fails a configurable number of upcoming sends.
///
/// This simulates an intermittent link so offline-first behavior can be proven
/// rather than assumed: pair it with a store-and-forward drain and assert that
/// every record still arrives, in order, once the link recovers.
///
/// # Examples
///
/// ```
/// use pamoja_core::Transport;
/// use pamoja_loopback::{Faulty, LoopbackBroker, LoopbackTransport};
///
/// # async fn run() -> pamoja_core::Result<()> {
/// let broker = LoopbackBroker::new();
/// let mut node = Faulty::new(LoopbackTransport::new(broker), 1);
/// node.connect().await?;
///
/// // The first send fails, simulating a dropped link; the next succeeds.
/// assert!(node.send("t", b"x").await.is_err());
/// node.send("t", b"x").await?;
/// # Ok(())
/// # }
/// ```
pub struct Faulty<T> {
    inner: T,
    upcoming_failures: usize,
}

impl<T> Faulty<T> {
    /// Wraps `inner`, failing its next `failures` sends before passing through.
    ///
    /// # Arguments
    ///
    /// * `inner` - the transport to decorate.
    /// * `failures` - how many of the next [`send`](Transport::send) calls fail.
    ///
    /// # Returns
    ///
    /// A decorator that injects the requested failures, then delegates to `inner`.
    pub fn new(inner: T, failures: usize) -> Self {
        Self {
            inner,
            upcoming_failures: failures,
        }
    }

    /// Arms the decorator to fail the next `count` sends.
    ///
    /// # Arguments
    ///
    /// * `count` - the number of upcoming sends to fail, simulating another link
    ///   outage.
    pub fn fail_next(&mut self, count: usize) {
        self.upcoming_failures = count;
    }
}

impl<T: Transport + Send> Transport for Faulty<T> {
    async fn connect(&mut self) -> Result<()> {
        self.inner.connect().await
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if self.upcoming_failures > 0 {
            self.upcoming_failures -= 1;
            return Err(Error::Transport("simulated link failure".to_owned()));
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
    use crate::{LoopbackBroker, LoopbackTransport};

    #[tokio::test]
    async fn fails_the_configured_sends_then_passes_through() {
        let broker = LoopbackBroker::new();
        let mut gateway = LoopbackTransport::new(broker.clone());
        gateway.connect().await.expect("connect");
        gateway.subscribe("#").await.expect("subscribe");

        let mut node = Faulty::new(LoopbackTransport::new(broker), 2);
        node.connect().await.expect("connect");

        assert!(node.send("t", b"1").await.is_err());
        assert!(node.send("t", b"2").await.is_err());
        node.send("t", b"3")
            .await
            .expect("third send passes through");

        // Only the delivered payload reaches the gateway.
        let message = gateway.recv().await.expect("recv").expect("a message");
        assert_eq!(message.payload, b"3");
    }
}
