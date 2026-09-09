//! The ready-to-run node a profile assembles around real components.

use core::time::Duration;

use pamoja_codec::Codec;
use pamoja_core::{Actuator, Result, Sensor, Transport};
use pamoja_power::PowerMode;

use crate::{Controller, Profile, Reaction};

/// An actuator that accepts and ignores commands.
///
/// Profiles that only observe - such as a well-level monitor - have no output to
/// drive. [`Node::monitor`] wires this in their place so a node has one uniform
/// shape whether or not it switches an actuator.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoActuator;

impl Actuator for NoActuator {
    type Command = bool;

    async fn apply(&mut self, _command: bool) -> Result<()> {
        Ok(())
    }
}

/// A profile assembled around the components that make it run.
///
/// The node is the thin shell that ties a [`Profile`]'s decision logic to real I/O.
/// Each [`tick`](Node::tick) reads the sensor, runs the profile's
/// [`Controller`](crate::Controller), drives the actuator when the controller calls
/// for it, and publishes the reading over the transport with the supplied codec. The
/// control math lives in `pamoja-kit`, the power schedule in `pamoja-power`, and the
/// wire format in the codec, so the node adds composition, not behavior.
///
/// Readings are real-world `f32` units (degrees, percent, liters), the form the
/// `pamoja-kit` controllers expect; a driver is responsible for calibrating raw
/// counts into those units before the node sees them.
///
/// # Examples
///
/// ```
/// use pamoja_codec::CborCodec;
/// use pamoja_core::{Actuator, Result, Sensor, Transport};
/// use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
/// use pamoja_profile::{Node, Profile};
///
/// struct Probe(f32);
/// impl Sensor for Probe {
///     type Reading = f32;
///     async fn read(&mut self) -> Result<f32> {
///         Ok(self.0)
///     }
/// }
///
/// struct Cooler;
/// impl Actuator for Cooler {
///     type Command = bool;
///     async fn apply(&mut self, _on: bool) -> Result<()> {
///         Ok(())
///     }
/// }
///
/// # async fn run() -> Result<()> {
/// let broker = LoopbackBroker::new();
/// let mut link = LoopbackTransport::new(broker);
/// link.connect().await?;
///
/// // A warm fridge, assembled straight from its profile.
/// let mut node = Node::new(Profile::vaccine_fridge_monitor(), Probe(9.0), Cooler, link, CborCodec);
/// let reaction = node.tick().await?;
/// assert_eq!(reaction.actuator, Some(true)); // the cooler runs
/// assert!(reaction.alert.is_some()); // and 9 C is a spoilage excursion
/// # Ok(())
/// # }
/// ```
pub struct Node<S, A, T, C> {
    profile: Profile,
    controller: Controller,
    sensor: S,
    actuator: A,
    transport: T,
    codec: C,
}

impl<S, A, T, C> Node<S, A, T, C> {
    /// Assembles a node from a profile and the components that drive it.
    ///
    /// # Arguments
    ///
    /// * `profile` - the profile to assemble; its policy becomes the node's controller.
    /// * `sensor` - the source of readings.
    /// * `actuator` - the output the controller switches.
    /// * `transport` - the link readings are published over; expected to be connected.
    /// * `codec` - the wire format readings are encoded with.
    ///
    /// # Returns
    ///
    /// A node ready to [`tick`](Node::tick).
    pub fn new(profile: Profile, sensor: S, actuator: A, transport: T, codec: C) -> Self {
        let controller = profile.controller();
        Self {
            profile,
            controller,
            sensor,
            actuator,
            transport,
            codec,
        }
    }

    /// Returns the profile this node was assembled from.
    ///
    /// # Returns
    ///
    /// A reference to the node's [`Profile`].
    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// Returns the power mode and wait interval for the next cycle.
    ///
    /// This assembles the profile's [`PowerSchedule`](crate::PowerSchedule) into a
    /// `pamoja-power` governor: as the battery drains the interval stretches, and a
    /// charging panel eases the node back toward its active cadence. The node never
    /// sleeps; the caller waits the
    /// returned [`Duration`] before the next [`tick`](Node::tick), so timing stays
    /// outside the node and the decision logic remains synchronous and testable.
    ///
    /// # Arguments
    ///
    /// * `soc` - the battery state of charge in `[0.0, 1.0]`.
    /// * `charging` - whether the panel is currently delivering charge.
    ///
    /// # Returns
    ///
    /// The [`PowerMode`] to run in and how long to wait before the next cycle.
    pub fn schedule(&self, soc: f32, charging: bool) -> (PowerMode, Duration) {
        let plan = self.profile.power.plan();
        let mode = plan.mode_while_charging(soc, charging);
        (mode, plan.interval_for(mode))
    }
}

impl<S, T, C> Node<S, NoActuator, T, C> {
    /// Assembles a node for a profile that observes without driving an output.
    ///
    /// Wires a [`NoActuator`] in place of a real output, so a monitoring profile such
    /// as [`well_level`](Profile::well_level) reads and publishes with the same shape
    /// as a controlling one.
    ///
    /// # Arguments
    ///
    /// * `profile` - the profile to assemble.
    /// * `sensor` - the source of readings.
    /// * `transport` - the link readings are published over; expected to be connected.
    /// * `codec` - the wire format readings are encoded with.
    ///
    /// # Returns
    ///
    /// A node ready to [`tick`](Node::tick), with no output to switch.
    pub fn monitor(profile: Profile, sensor: S, transport: T, codec: C) -> Self {
        Node::new(profile, sensor, NoActuator, transport, codec)
    }
}

impl<S, A, T, C> Node<S, A, T, C>
where
    S: Sensor<Reading = f32>,
    A: Actuator<Command = bool>,
    T: Transport,
    C: Codec<f32>,
{
    /// Runs one read-decide-act-publish cycle.
    ///
    /// Reads the sensor, evaluates the profile's controller, applies the resulting
    /// command to the actuator when the controller calls for one, and publishes the
    /// reading to the profile's topic.
    ///
    /// # Returns
    ///
    /// The [`Reaction`] the controller produced: the actuator setting that was
    /// applied (if any) and any alert the reading raised.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](pamoja_core::Error::Io) or
    /// [`Error::Closed`](pamoja_core::Error::Closed) if the sensor read or the
    /// actuator command fails, [`Error::Codec`](pamoja_core::Error::Codec) if the
    /// reading cannot be encoded, and [`Error::Transport`](pamoja_core::Error::Transport)
    /// or [`Error::Closed`](pamoja_core::Error::Closed) if the publish fails.
    pub async fn tick(&mut self) -> Result<Reaction> {
        let reading = self.sensor.read().await?;
        let reaction = self.controller.evaluate(reading);
        if let Some(on) = reaction.actuator {
            self.actuator.apply(on).await?;
        }
        let payload = self.codec.encode(&reading)?;
        self.transport.send(&self.profile.topic, &payload).await?;
        Ok(reaction)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use pamoja_codec::CborCodec;
    use pamoja_core::Error;
    use pamoja_core::Receive;
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    use pamoja_power::PowerMode;

    use super::*;

    // A sensor that plays back a fixed list of readings, then reports closed.
    struct ScriptedSensor {
        readings: std::vec::IntoIter<f32>,
    }

    impl ScriptedSensor {
        fn new(readings: Vec<f32>) -> Self {
            Self {
                readings: readings.into_iter(),
            }
        }
    }

    impl Sensor for ScriptedSensor {
        type Reading = f32;

        async fn read(&mut self) -> Result<f32> {
            self.readings.next().ok_or(Error::Closed)
        }
    }

    // An actuator that records every command it is given.
    #[derive(Clone)]
    struct Recording(Arc<Mutex<Vec<bool>>>);

    impl Actuator for Recording {
        type Command = bool;

        async fn apply(&mut self, on: bool) -> Result<()> {
            self.0.lock().expect("commands lock").push(on);
            Ok(())
        }
    }

    async fn connected_pair(filter: &str) -> (LoopbackTransport, LoopbackTransport) {
        let broker = LoopbackBroker::new();
        let mut gateway = LoopbackTransport::new(broker.clone());
        let mut link = LoopbackTransport::new(broker);
        gateway.connect().await.expect("gateway connect");
        link.connect().await.expect("link connect");
        gateway.subscribe(filter).await.expect("subscribe");
        (gateway, link)
    }

    #[tokio::test]
    async fn tick_reads_actuates_and_publishes() {
        let (mut gateway, link) = connected_pair("cold-chain/#").await;
        let commands = Arc::new(Mutex::new(Vec::new()));
        let mut node = Node::new(
            Profile::vaccine_fridge_monitor(),
            ScriptedSensor::new(vec![9.0]),
            Recording(commands.clone()),
            link,
            CborCodec,
        );

        let reaction = node.tick().await.expect("tick");
        assert_eq!(reaction.actuator, Some(true));
        assert!(reaction.alert.is_some());
        assert_eq!(*commands.lock().expect("commands"), vec![true]);

        let message = gateway.recv().await.expect("recv").expect("a reading");
        assert_eq!(message.topic, "cold-chain/fridge/temperature");
        let reading: f32 = CborCodec.decode(&message.payload).expect("decode");
        assert_eq!(reading, 9.0);
    }

    #[tokio::test]
    async fn monitor_publishes_without_an_actuator() {
        let (mut gateway, link) = connected_pair("water/#").await;
        let mut node = Node::monitor(
            Profile::well_level(),
            ScriptedSensor::new(vec![3.2]),
            link,
            CborCodec,
        );

        let reaction = node.tick().await.expect("tick");
        assert_eq!(reaction.actuator, None);

        let message = gateway.recv().await.expect("recv").expect("a reading");
        assert_eq!(message.topic, "water/well/level");
        let reading: f32 = CborCodec.decode(&message.payload).expect("decode");
        assert_eq!(reading, 3.2);
    }

    #[test]
    fn schedule_follows_state_of_charge() {
        // The schedule reads only the profile, so the components can be placeholders.
        let node = Node::monitor(Profile::vaccine_fridge_monitor(), (), (), ());
        assert_eq!(node.schedule(0.9, false).0, PowerMode::Active);
        assert_eq!(node.schedule(0.1, false).0, PowerMode::Critical);
        // A charging panel eases off by one mode.
        assert_eq!(node.schedule(0.1, true).0, PowerMode::Saver);
        assert_eq!(node.schedule(0.9, false).1, Duration::from_secs(60));
    }
}
