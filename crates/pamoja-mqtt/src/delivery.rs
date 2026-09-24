//! Following each published message until the broker acknowledges it.
//!
//! `rumqttc` hands out no packet identifier when a message is published, so the event
//! loop's own record is read instead. It takes publish requests in order and reports each
//! as it goes out, with the identifier it chose, and later reports the broker's `PUBACK`
//! or `PUBCOMP` under that identifier. Every publish joins a queue as it is requested; the
//! queue's head is matched to the next outgoing report, and the acknowledgment settles it.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};

use pamoja_core::{Error, Result};
use rumqttc::QoS;
use tokio::sync::oneshot;

/// The broker's acknowledgment of one published message, still to come.
///
/// A message published at QoS 1 is settled by the broker's `PUBACK`, and one at QoS 2 by
/// its `PUBCOMP`, the last step of the exchange that delivers it exactly once (OASIS MQTT
/// 3.1.1, sections 4.3.2 and 4.3.3). MQTT acknowledges nothing at QoS 0, so such a message
/// is settled once the connection has taken it.
#[derive(Debug)]
#[must_use = "a delivery says nothing until it is awaited"]
pub struct Delivery {
    settled: oneshot::Receiver<Result<()>>,
}

impl Delivery {
    /// Waits for the broker to acknowledge the message.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the broker holds the message.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the connection ended before the acknowledgment
    /// arrived, or [`Error::Closed`] if the transport was disconnected first. Either way
    /// the message may or may not have reached the broker.
    pub async fn confirmed(self) -> Result<()> {
        self.settled.await.unwrap_or(Err(Error::Closed))
    }
}

struct Waiting {
    qos: QoS,
    settle: oneshot::Sender<Result<()>>,
}

/// The publishes the event loop has yet to report, shared by the transport that requests
/// them and the task that runs the loop.
#[derive(Clone, Default)]
pub(crate) struct Requested(Arc<Mutex<VecDeque<Waiting>>>);

impl Requested {
    /// Queues a publish about to be requested.
    pub(crate) fn push(&self, qos: QoS) -> Delivery {
        let (settle, settled) = oneshot::channel();
        self.lock().push_back(Waiting { qos, settle });
        Delivery { settled }
    }

    /// Takes back the publish queued last, when its request never reached the loop.
    pub(crate) fn withdraw(&self) {
        self.lock().pop_back();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, VecDeque<Waiting>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What the event loop's task keeps: publishes that went out and wait for their
/// acknowledgment, by packet identifier.
#[derive(Default)]
pub(crate) struct InFlight {
    by_id: HashMap<u16, oneshot::Sender<Result<()>>>,
}

impl InFlight {
    /// Matches the loop's report of an outgoing publish to the request at the queue's head.
    pub(crate) fn sent(&mut self, requested: &Requested, pkid: u16) {
        let Some(waiting) = requested.lock().pop_front() else {
            return;
        };
        if waiting.qos == QoS::AtMostOnce {
            let _ = waiting.settle.send(Ok(()));
        } else {
            self.by_id.insert(pkid, waiting.settle);
        }
    }

    /// Settles the publish the broker acknowledged.
    pub(crate) fn acknowledged(&mut self, pkid: u16) {
        if let Some(settle) = self.by_id.remove(&pkid) {
            let _ = settle.send(Ok(()));
        }
    }

    /// Fails every publish still waiting, when the connection ends.
    pub(crate) fn fail(&mut self, requested: &Requested, reason: &str) {
        let waiting = requested
            .lock()
            .drain(..)
            .map(|waiting| waiting.settle)
            .collect::<Vec<_>>();
        for settle in self.by_id.drain().map(|(_, settle)| settle).chain(waiting) {
            let _ = settle.send(Err(Error::Transport(format!(
                "the connection ended before the broker acknowledged the message: {reason}"
            ))));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn each_report_settles_the_request_it_belongs_to() {
        let requested = Requested::default();
        let mut in_flight = InFlight::default();
        let quick = requested.push(QoS::AtMostOnce);
        let once = requested.push(QoS::AtLeastOnce);
        let exact = requested.push(QoS::ExactlyOnce);

        in_flight.sent(&requested, 0);
        in_flight.sent(&requested, 7);
        in_flight.sent(&requested, 8);
        quick
            .confirmed()
            .await
            .expect("QoS 0 settles once it goes out");

        in_flight.acknowledged(8);
        in_flight.acknowledged(7);
        exact
            .confirmed()
            .await
            .expect("settled by its own identifier");
        once.confirmed()
            .await
            .expect("whatever order the answers come in");
    }

    #[tokio::test]
    async fn an_ended_connection_fails_what_is_still_waiting() {
        let requested = Requested::default();
        let mut in_flight = InFlight::default();
        let out = requested.push(QoS::AtLeastOnce);
        let queued = requested.push(QoS::AtLeastOnce);
        in_flight.sent(&requested, 1);
        in_flight.fail(&requested, "the broker went away");
        for delivery in [out, queued] {
            match delivery.confirmed().await {
                Err(Error::Transport(reason)) => assert!(reason.contains("went away"), "{reason}"),
                other => panic!("a delivery cut off is refused, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn a_withdrawn_request_is_closed() {
        let requested = Requested::default();
        let delivery = requested.push(QoS::AtLeastOnce);
        requested.withdraw();
        assert!(matches!(delivery.confirmed().await, Err(Error::Closed)));
    }
}
