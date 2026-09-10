//! The ready-to-run node a profile assembles around real components.

use core::time::Duration;

use pamoja_codec::Codec;
use pamoja_core::{Actuator, Result, Sensor, Transport};
use pamoja_power::PowerMode;

use crate::{Controller, Policy, Profile, Reaction};

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
/// Each [`tick`](Node::tick) reads the sensor, runs the node's policy, drives the
/// actuator when the policy calls for it, and publishes the reading over the
/// transport with the supplied codec. The control math lives in `pamoja-kit`, the
/// power schedule in `pamoja-power`, and the wire format in the codec, so the node
/// adds composition, not behavior.
///
/// [`Node::new`] runs the profile's [`Controller`], which reads real-world `f32`
/// units (degrees, percent, liters), the form the `pamoja-kit` controllers expect; a
/// driver is responsible for calibrating raw counts into those units before the node
/// sees them, or [`Sensor::map`] selects one. [`Node::with_policy`] runs a
/// [`Policy`] of your own instead, over whatever the sensor reads and whatever the
/// actuator takes.
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
pub struct Node<S, A, T, C, P = Controller> {
    profile: Profile,
    policy: P,
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
        Node::with_policy(profile, controller, sensor, actuator, transport, codec)
    }
}

impl<S, A, T, C, P> Node<S, A, T, C, P> {
    /// Assembles a node around a policy of your own.
    ///
    /// The profile still names the topic and the power schedule; the policy decides
    /// each reading, so a node can read a driver's whole measurement and command an
    /// actuator that takes more than on and off. A policy a [`PolicyRegistry`] resolved
    /// from the profile's own control kind goes here too.
    ///
    /// # Arguments
    ///
    /// * `profile` - the profile to assemble.
    /// * `policy` - what decides each reading.
    /// * `sensor` - the source of readings.
    /// * `actuator` - the output the policy commands.
    /// * `transport` - the link readings are published over; expected to be connected.
    /// * `codec` - the wire format readings are encoded with.
    ///
    /// # Returns
    ///
    /// A node ready to [`tick`](Node::tick).
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_codec::CborCodec;
    /// use pamoja_core::{Actuator, Result, Sensor, Transport};
    /// use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    /// use pamoja_profile::{Alert, Node, Policy, Profile, Reaction};
    /// use serde::{Deserialize, Serialize};
    ///
    /// // A driver reads two quantities at once; the policy decides on both.
    /// #[derive(Clone, Copy, Serialize, Deserialize)]
    /// struct Soil {
    ///     moisture: f32,
    ///     temperature: f32,
    /// }
    ///
    /// struct Probe;
    /// impl Sensor for Probe {
    ///     type Reading = Soil;
    ///     async fn read(&mut self) -> Result<Soil> {
    ///         Ok(Soil { moisture: 22.0, temperature: 1.5 })
    ///     }
    /// }
    ///
    /// struct Valve;
    /// impl Actuator for Valve {
    ///     type Command = bool;
    ///     async fn apply(&mut self, _open: bool) -> Result<()> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// // Water only when the bed is dry and not about to freeze.
    /// struct DripPolicy;
    /// impl Policy for DripPolicy {
    ///     type Reading = Soil;
    ///     type Command = bool;
    ///     fn evaluate(&mut self, soil: &Soil) -> Reaction {
    ///         let frost = soil.temperature < 2.0;
    ///         Reaction {
    ///             actuator: Some(soil.moisture < 25.0 && !frost),
    ///             alert: frost.then_some(Alert::Custom { code: "FrostRisk", value: soil.temperature }),
    ///         }
    ///     }
    /// }
    ///
    /// # async fn run() -> Result<()> {
    /// let mut link = LoopbackTransport::new(LoopbackBroker::new());
    /// link.connect().await?;
    /// let profile = Profile::irrigation_node();
    /// let mut node = Node::with_policy(profile, DripPolicy, Probe, Valve, link, CborCodec);
    /// let reaction = node.tick().await?;
    /// assert_eq!(reaction.actuator, Some(false)); // dry, but frost is closer
    /// assert_eq!(reaction.alert.map(Alert::kind), Some("FrostRisk"));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PolicyRegistry`]: crate::PolicyRegistry
    pub fn with_policy(
        profile: Profile,
        policy: P,
        sensor: S,
        actuator: A,
        transport: T,
        codec: C,
    ) -> Self {
        Self {
            profile,
            policy,
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

    /// Returns the policy deciding this node's readings.
    ///
    /// # Returns
    ///
    /// A reference to the node's policy.
    pub fn policy(&self) -> &P {
        &self.policy
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

impl<S, A, T, C, P> Node<S, A, T, C, P>
where
    S: Sensor,
    P: Policy<Reading = S::Reading>,
    P::Command: Clone,
    A: Actuator<Command = P::Command>,
    T: Transport,
    C: Codec<S::Reading>,
{
    /// Runs one read-decide-act-publish cycle.
    ///
    /// Reads the sensor, evaluates the node's policy, applies the resulting command
    /// to the actuator when the policy calls for one, and publishes the reading to
    /// the profile's topic.
    ///
    /// # Returns
    ///
    /// The [`Reaction`] the policy produced: the command that was applied (if any)
    /// and any alert the reading raised.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](pamoja_core::Error::Io) or
    /// [`Error::Closed`](pamoja_core::Error::Closed) if the sensor read or the
    /// actuator command fails, [`Error::Codec`](pamoja_core::Error::Codec) if the
    /// reading cannot be encoded, and [`Error::Transport`](pamoja_core::Error::Transport)
    /// or [`Error::Closed`](pamoja_core::Error::Closed) if the publish fails.
    pub async fn tick(&mut self) -> Result<Reaction<P::Command>> {
        let reading = self.sensor.read().await?;
        let reaction = self.policy.evaluate(&reading);
        if let Some(command) = reaction.actuator.clone() {
            self.actuator.apply(command).await?;
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
    use crate::Alert;

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
    struct Recording<C = bool>(Arc<Mutex<Vec<C>>>);

    impl<C: Send> Actuator for Recording<C> {
        type Command = C;

        async fn apply(&mut self, command: C) -> Result<()> {
            self.0.lock().expect("commands lock").push(command);
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

    // A sensor reading two quantities at once, as a driver for a combined part does.
    struct Climate(f32, f32);

    impl Sensor for Climate {
        type Reading = (f32, f32);

        async fn read(&mut self) -> Result<(f32, f32)> {
            Ok((self.0, self.1))
        }
    }

    // A policy over the pair, commanding a fan duty rather than an on/off.
    struct FanPolicy;

    impl Policy for FanPolicy {
        type Reading = (f32, f32);
        type Command = u8;

        fn evaluate(&mut self, &(temperature, humidity): &(f32, f32)) -> Reaction<u8> {
            let duty = if temperature > 30.0 { 100 } else { 0 };
            Reaction {
                actuator: Some(duty),
                alert: (humidity > 90.0).then_some(Alert::Custom {
                    code: "Muggy",
                    value: humidity,
                }),
            }
        }
    }

    #[tokio::test]
    async fn a_policy_of_your_own_reads_a_pair_and_commands_a_duty() {
        let (mut gateway, link) = connected_pair("farm/#").await;
        let duties = Arc::new(Mutex::new(Vec::new()));
        let mut node = Node::with_policy(
            Profile::irrigation_node(),
            FanPolicy,
            Climate(34.0, 95.0),
            Recording::<u8>(duties.clone()),
            link,
            CborCodec,
        );

        let reaction = node.tick().await.expect("tick");
        assert_eq!(reaction.actuator, Some(100));
        assert_eq!(reaction.alert.map(Alert::kind), Some("Muggy"));
        assert_eq!(*duties.lock().expect("duties"), vec![100]);
        let _: &FanPolicy = node.policy();

        let message = gateway.recv().await.expect("recv").expect("a reading");
        assert_eq!(message.topic, "farm/irrigation/soil-moisture");
        let reading: (f32, f32) = CborCodec.decode(&message.payload).expect("decode");
        assert_eq!(reading, (34.0, 95.0));
    }

    #[tokio::test]
    async fn a_node_runs_from_a_spawned_task() {
        // The generic shape a maker's runner takes: any sensor and actuator, spawned on a
        // multi-threaded runtime, which needs the whole tick to be Send.
        fn spawn_node<S, A, T, C>(mut node: Node<S, A, T, C>) -> tokio::task::JoinHandle<()>
        where
            S: Sensor<Reading = f32> + Send + 'static,
            A: Actuator<Command = bool> + Send + 'static,
            T: Transport + Send + 'static,
            C: Codec<f32> + Send + 'static,
        {
            tokio::spawn(async move {
                for _ in 0..2 {
                    node.tick().await.expect("a tick");
                }
            })
        }

        let (mut gateway, link) = connected_pair("cold-chain/#").await;
        let node = Node::new(
            Profile::vaccine_fridge_monitor(),
            ScriptedSensor::new(vec![9.0, 4.0]),
            Recording(Arc::new(Mutex::new(Vec::new()))),
            link,
            CborCodec,
        );
        spawn_node(node).await.expect("the task finishes");
        let first = gateway.recv().await.expect("recv").expect("a reading");
        let reading: f32 = CborCodec.decode(&first.payload).expect("decode");
        assert_eq!(reading, 9.0);
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
