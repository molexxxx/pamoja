//! Rules between nodes, as data: what one node's reading calls for at another node's
//! actuator, written as a file, and the engine that runs it off a link.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;

use pamoja_codec::Codec;
use pamoja_core::{Actuator, Error, Receive, Result, Transport};
use pamoja_kit::{Edge, Trigger};
use serde::{Deserialize, Serialize};

/// Which side of a line a reading must be on for a condition to hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compare {
    /// The condition holds while the reading is above the threshold.
    Above,
    /// The condition holds while the reading is below the threshold.
    Below,
}

/// A line drawn through one topic's readings, with the release band that stops it
/// firing over and over.
///
/// In a rule file it is the `when` object:
///
/// ```json
/// { "topic": "garden/bed-1/moisture", "compare": "below", "threshold": 30.0, "hysteresis": 5.0 }
/// ```
///
/// The condition becomes true the moment a reading crosses the threshold in the named
/// direction, and stops holding once a reading has come back past the threshold by the
/// hysteresis; readings in between leave it as it was. That is a
/// [`Trigger`](pamoja_kit::Trigger), and [`trigger`](Condition::trigger) builds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    /// The topic whose readings are watched, exactly as the node publishes to it.
    pub topic: String,
    /// Which side of the threshold the condition holds on.
    pub compare: Compare,
    /// The line a reading crosses.
    pub threshold: f32,
    /// How far back past the line a reading must come for the condition to clear.
    #[serde(default)]
    pub hysteresis: f32,
}

impl Condition {
    /// A condition that holds while a topic's readings are above a line.
    ///
    /// # Arguments
    ///
    /// * `topic` - the topic whose readings are watched.
    /// * `threshold` - the line.
    ///
    /// # Returns
    ///
    /// The condition, with no release band yet.
    pub fn above(topic: impl Into<String>, threshold: f32) -> Self {
        Self {
            topic: topic.into(),
            compare: Compare::Above,
            threshold,
            hysteresis: 0.0,
        }
    }

    /// A condition that holds while a topic's readings are below a line.
    ///
    /// # Arguments
    ///
    /// * `topic` - the topic whose readings are watched.
    /// * `threshold` - the line.
    ///
    /// # Returns
    ///
    /// The condition, with no release band yet.
    pub fn below(topic: impl Into<String>, threshold: f32) -> Self {
        Self {
            topic: topic.into(),
            compare: Compare::Below,
            threshold,
            hysteresis: 0.0,
        }
    }

    /// Sets the release band on the far side of the line.
    ///
    /// # Arguments
    ///
    /// * `hysteresis` - how far back past the line a reading must come to clear the
    ///   condition.
    ///
    /// # Returns
    ///
    /// The condition, for chaining.
    pub fn with_hysteresis(mut self, hysteresis: f32) -> Self {
        self.hysteresis = hysteresis;
        self
    }

    /// Builds the trigger that decides this condition, reading by reading.
    ///
    /// # Returns
    ///
    /// A trigger that starts cleared.
    pub fn trigger(&self) -> Trigger {
        match self.compare {
            Compare::Above => Trigger::above(self.threshold, self.hysteresis),
            Compare::Below => Trigger::below(self.threshold, self.hysteresis),
        }
    }
}

/// What a rule does when its condition sets or clears.
///
/// In a rule file each action is an object tagged by `do`:
///
/// ```json
/// { "do": "drive", "actuator": "bed-valve", "on": true }
/// { "do": "publish", "topic": "garden/bed-1/valve", "payload": "open" }
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum Action {
    /// Switches an actuator the engine was given under this name.
    Drive {
        /// The name the actuator was registered under.
        actuator: String,
        /// The setting to apply.
        on: bool,
    },
    /// Publishes a message over the engine's link.
    Publish {
        /// The topic to publish to.
        topic: String,
        /// The text to publish.
        payload: String,
    },
}

/// One rule: a condition over a topic, and what to do as it sets and clears.
///
/// ```json
/// {
///   "name": "water-when-dry",
///   "when": { "topic": "garden/bed-1/moisture", "compare": "below", "threshold": 30.0, "hysteresis": 5.0 },
///   "then": [ { "do": "drive", "actuator": "bed-valve", "on": true } ],
///   "otherwise": [ { "do": "drive", "actuator": "bed-valve", "on": false } ]
/// }
/// ```
///
/// `then` runs once when the condition becomes true and `otherwise` once when it stops
/// holding; either may be left out. The name is what the engine reports when the rule
/// fires.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    /// A stable name for the rule, such as `"water-when-dry"`.
    pub name: String,
    /// The condition the rule watches.
    pub when: Condition,
    /// What to do the moment the condition becomes true.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then: Vec<Action>,
    /// What to do the moment the condition stops holding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub otherwise: Vec<Action>,
}

impl Rule {
    /// Starts a rule with a name and a condition and nothing to do yet.
    ///
    /// # Arguments
    ///
    /// * `name` - a stable name for the rule.
    /// * `when` - the condition it watches.
    ///
    /// # Returns
    ///
    /// The rule, for chaining.
    pub fn new(name: impl Into<String>, when: Condition) -> Self {
        Self {
            name: name.into(),
            when,
            then: Vec::new(),
            otherwise: Vec::new(),
        }
    }

    /// Adds an action for the moment the condition becomes true.
    ///
    /// # Arguments
    ///
    /// * `action` - what to do.
    ///
    /// # Returns
    ///
    /// The rule, for chaining.
    pub fn then(mut self, action: Action) -> Self {
        self.then.push(action);
        self
    }

    /// Adds an action for the moment the condition stops holding.
    ///
    /// # Arguments
    ///
    /// * `action` - what to do.
    ///
    /// # Returns
    ///
    /// The rule, for chaining.
    pub fn otherwise(mut self, action: Action) -> Self {
        self.otherwise.push(action);
        self
    }

    /// Names every actuator the rule drives.
    ///
    /// # Returns
    ///
    /// Each actuator name, once, in name order.
    pub fn actuators(&self) -> impl Iterator<Item = &str> {
        let mut names: BTreeSet<&str> = BTreeSet::new();
        for action in self.then.iter().chain(&self.otherwise) {
            if let Action::Drive { actuator, .. } = action {
                names.insert(actuator);
            }
        }
        names.into_iter()
    }
}

/// A set of rules, as a file a fleet shares.
///
/// ```json
/// { "rules": [ ... ] }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rules {
    /// The rules, in the order they are checked against each message.
    pub rules: Vec<Rule>,
}

impl Rules {
    /// Starts with no rules.
    ///
    /// # Returns
    ///
    /// An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a rule.
    ///
    /// # Arguments
    ///
    /// * `rule` - the rule to add.
    ///
    /// # Returns
    ///
    /// The set, for chaining.
    pub fn with(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Loads rules from their JSON file.
    ///
    /// # Arguments
    ///
    /// * `text` - the file's contents.
    ///
    /// # Returns
    ///
    /// The rules.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`] if the text is not a rule file.
    #[cfg(feature = "json")]
    pub fn from_json(text: &str) -> Result<Self> {
        serde_json::from_str(text).map_err(|error| Error::Codec(error.to_string()))
    }

    /// Writes the rules as the JSON file a fleet shares.
    ///
    /// # Returns
    ///
    /// The pretty-printed JSON text.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`] if the rules cannot be serialized.
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|error| Error::Codec(error.to_string()))
    }

    /// Checks the rules for what a file can say that an engine cannot run.
    ///
    /// # Returns
    ///
    /// Nothing when every rule is usable.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unsupported`] naming the first problem: a rule without a name
    /// or with a name another rule has, a condition without a topic, a threshold or
    /// hysteresis that is not a finite non-negative number, a rule with nothing to do,
    /// or an action naming an empty actuator or topic.
    pub fn check(&self) -> Result<()> {
        let mut names = BTreeSet::new();
        for rule in &self.rules {
            if rule.name.trim().is_empty() {
                return Err(Error::Unsupported("a rule needs a name"));
            }
            if !names.insert(rule.name.as_str()) {
                return Err(Error::Unsupported("two rules share a name"));
            }
            if rule.when.topic.trim().is_empty() {
                return Err(Error::Unsupported("a condition needs a topic"));
            }
            if !rule.when.threshold.is_finite() {
                return Err(Error::Unsupported("a threshold must be a finite number"));
            }
            if !(rule.when.hysteresis.is_finite() && rule.when.hysteresis >= 0.0) {
                return Err(Error::Unsupported(
                    "a hysteresis must be a finite number of zero or more",
                ));
            }
            if rule.then.is_empty() && rule.otherwise.is_empty() {
                return Err(Error::Unsupported("a rule needs something to do"));
            }
            for action in rule.then.iter().chain(&rule.otherwise) {
                match action {
                    Action::Drive { actuator, .. } if actuator.trim().is_empty() => {
                        return Err(Error::Unsupported("a drive action needs an actuator name"));
                    }
                    Action::Publish { topic, .. } if topic.trim().is_empty() => {
                        return Err(Error::Unsupported("a publish action needs a topic"));
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

/// What a rule did on one message.
#[derive(Clone, Debug, PartialEq)]
pub struct Fired {
    /// The rule's name.
    pub rule: String,
    /// Whether its condition set or cleared.
    pub edge: Edge,
    /// The reading that did it.
    pub reading: f32,
}

// An actuator behind a trait object, so the engine holds any number of them by name.
// The core trait's futures are `Send`, which is what lets one be boxed like this.
trait ErasedActuator: Send {
    fn apply<'a>(&'a mut self, on: bool) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}

impl<A: Actuator<Command = bool> + Send> ErasedActuator for A {
    fn apply<'a>(&'a mut self, on: bool) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(Actuator::apply(self, on))
    }
}

// A rule with the trigger that carries its condition's state between messages.
struct Armed {
    rule: Rule,
    trigger: Trigger,
}

/// Runs a set of rules off a link: every reading that arrives on a watched topic is
/// judged, and the actions a set or cleared condition calls for are carried out.
///
/// The engine subscribes to each rule's topic on [`connect`](RuleEngine::connect) and
/// then handles one message per [`step`](RuleEngine::step), decoding the reading with
/// the codec the nodes publish in, switching the actuators it was given by name, and
/// publishing over the same link. The node that publishes the reading and the node whose
/// actuator moves can be anywhere the link reaches; the engine is the third party
/// between them, and its rules are a file.
///
/// # Examples
///
/// ```
/// use pamoja_codec::JsonCodec;
/// use pamoja_core::{Actuator, Result, Transport};
/// use pamoja_kit::Edge;
/// use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
/// use pamoja_profile::{Action, Condition, Rule, RuleEngine, Rules};
///
/// struct Valve(bool);
/// impl Actuator for Valve {
///     type Command = bool;
///     async fn apply(&mut self, open: bool) -> Result<()> {
///         self.0 = open;
///         Ok(())
///     }
/// }
///
/// # async fn run() -> Result<()> {
/// let broker = LoopbackBroker::new();
/// let mut probe = LoopbackTransport::new(broker.clone());
/// probe.connect().await?;
///
/// // Water the bed when it dries past 30, and stop once it is wetter than 35.
/// let rules = Rules::new().with(
///     Rule::new("water-when-dry", Condition::below("garden/bed-1/moisture", 30.0).with_hysteresis(5.0))
///         .then(Action::Drive { actuator: "bed-valve".into(), on: true })
///         .otherwise(Action::Drive { actuator: "bed-valve".into(), on: false }),
/// );
/// let mut engine = RuleEngine::new(rules, LoopbackTransport::new(broker), JsonCodec)
///     .with_actuator("bed-valve", Valve(false));
/// engine.connect().await?;
///
/// probe.send("garden/bed-1/moisture", b"28.0").await?;
/// let fired = engine.step().await?.expect("the link is up");
/// assert_eq!(fired[0].rule, "water-when-dry");
/// assert_eq!(fired[0].edge, Edge::Set);
/// # Ok(())
/// # }
/// ```
pub struct RuleEngine<L, C> {
    armed: Vec<Armed>,
    link: L,
    codec: C,
    actuators: BTreeMap<String, Box<dyn ErasedActuator>>,
}

impl<L, C> RuleEngine<L, C> {
    /// Assembles an engine around a link and the codec the readings arrive in.
    ///
    /// # Arguments
    ///
    /// * `rules` - the rules to run.
    /// * `link` - the link readings arrive on and actions publish over; connected by
    ///   [`connect`](RuleEngine::connect).
    /// * `codec` - the wire format the nodes publish their readings in.
    ///
    /// # Returns
    ///
    /// An engine with no actuators yet.
    pub fn new(rules: Rules, link: L, codec: C) -> Self {
        Self {
            armed: rules
                .rules
                .into_iter()
                .map(|rule| Armed {
                    trigger: rule.when.trigger(),
                    rule,
                })
                .collect(),
            link,
            codec,
            actuators: BTreeMap::new(),
        }
    }

    /// Gives the engine an actuator a rule may drive by name.
    ///
    /// # Arguments
    ///
    /// * `name` - the name a `drive` action uses.
    /// * `actuator` - the output, which the engine owns from here on.
    ///
    /// # Returns
    ///
    /// The engine, for chaining.
    pub fn with_actuator(
        mut self,
        name: impl Into<String>,
        actuator: impl Actuator<Command = bool> + Send + 'static,
    ) -> Self {
        self.actuators.insert(name.into(), Box::new(actuator));
        self
    }

    /// Walks the rules in the order they are checked.
    ///
    /// # Returns
    ///
    /// Each rule.
    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.armed.iter().map(|armed| &armed.rule)
    }

    /// Whether a rule's condition currently holds.
    ///
    /// # Arguments
    ///
    /// * `rule` - the rule's name.
    ///
    /// # Returns
    ///
    /// The condition's state, or `None` for a name no rule has.
    pub fn is_set(&self, rule: &str) -> Option<bool> {
        self.armed
            .iter()
            .find(|armed| armed.rule.name == rule)
            .map(|armed| armed.trigger.is_set())
    }
}

impl<L, C> RuleEngine<L, C>
where
    L: Transport + Receive,
    C: Codec<f32>,
{
    /// Checks the rules, connects the link, and subscribes to every watched topic.
    ///
    /// # Returns
    ///
    /// Nothing once the engine is listening.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unsupported`] if a rule cannot be run (see [`Rules::check`]) or
    /// drives an actuator the engine was not given, and the link's own error if it
    /// cannot connect or subscribe.
    pub async fn connect(&mut self) -> Result<()> {
        let rules = Rules {
            rules: self.armed.iter().map(|armed| armed.rule.clone()).collect(),
        };
        rules.check()?;
        for rule in &rules.rules {
            for actuator in rule.actuators() {
                if !self.actuators.contains_key(actuator) {
                    return Err(Error::Unsupported(
                        "a rule drives an actuator the engine was not given",
                    ));
                }
            }
        }
        self.link.connect().await?;
        let topics: BTreeSet<&str> = rules
            .rules
            .iter()
            .map(|rule| rule.when.topic.as_str())
            .collect();
        for topic in topics {
            self.link.subscribe(topic).await?;
        }
        Ok(())
    }

    /// Waits for one message and runs every rule that watches its topic.
    ///
    /// # Returns
    ///
    /// What fired, which is empty when the message set or cleared nothing, or `None`
    /// once the link has ended.
    ///
    /// # Errors
    ///
    /// Returns the link's error if receiving or publishing fails,
    /// [`Error::Codec`] if a reading on a watched topic does not decode, and the
    /// actuator's error if a drive fails.
    pub async fn step(&mut self) -> Result<Option<Vec<Fired>>> {
        let Some(message) = self.link.recv().await? else {
            return Ok(None);
        };
        let watching: Vec<usize> = self
            .armed
            .iter()
            .enumerate()
            .filter(|(_, armed)| armed.rule.when.topic == message.topic)
            .map(|(index, _)| index)
            .collect();
        let mut fired = Vec::new();
        if watching.is_empty() {
            return Ok(Some(fired));
        }
        let reading: f32 = self.codec.decode(&message.payload)?;
        for index in watching {
            let Some(edge) = self.armed[index].trigger.update(reading) else {
                continue;
            };
            let actions = match edge {
                Edge::Set => self.armed[index].rule.then.clone(),
                Edge::Cleared => self.armed[index].rule.otherwise.clone(),
            };
            for action in &actions {
                self.run(action).await?;
            }
            fired.push(Fired {
                rule: self.armed[index].rule.name.clone(),
                edge,
                reading,
            });
        }
        Ok(Some(fired))
    }

    async fn run(&mut self, action: &Action) -> Result<()> {
        match action {
            Action::Drive { actuator, on } => {
                let actuator = self.actuators.get_mut(actuator).ok_or(Error::Unsupported(
                    "a rule drives an actuator the engine was not given",
                ))?;
                actuator.apply(*on).await
            }
            Action::Publish { topic, payload } => self.link.send(topic, payload.as_bytes()).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use pamoja_codec::JsonCodec;
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};

    use super::*;

    const FILE: &str = r#"{
  "rules": [
    {
      "name": "water-when-dry",
      "when": { "topic": "garden/bed-1/moisture", "compare": "below", "threshold": 30.0, "hysteresis": 5.0 },
      "then": [
        { "do": "drive", "actuator": "bed-valve", "on": true },
        { "do": "publish", "topic": "garden/bed-1/valve", "payload": "open" }
      ],
      "otherwise": [
        { "do": "drive", "actuator": "bed-valve", "on": false },
        { "do": "publish", "topic": "garden/bed-1/valve", "payload": "closed" }
      ]
    }
  ]
}"#;

    #[derive(Clone, Default)]
    struct Valve(Arc<Mutex<Vec<bool>>>);

    impl Actuator for Valve {
        type Command = bool;

        async fn apply(&mut self, open: bool) -> Result<()> {
            self.0.lock().expect("valve lock").push(open);
            Ok(())
        }
    }

    #[test]
    fn a_rule_file_round_trips_and_builds_its_trigger() {
        let rules = Rules::from_json(FILE).expect("parses");
        assert_eq!(rules.rules.len(), 1);
        let rule = &rules.rules[0];
        assert_eq!(rule.name, "water-when-dry");
        assert_eq!(rule.when.compare, Compare::Below);
        assert_eq!(rule.when.hysteresis, 5.0);
        assert_eq!(rule.actuators().collect::<Vec<_>>(), ["bed-valve"]);
        let mut trigger = rule.when.trigger();
        assert_eq!(trigger.update(28.0), Some(Edge::Set));
        let shared = rules.to_json().unwrap();
        assert!(shared.contains("\"do\": \"publish\""), "{shared}");
        assert_eq!(Rules::from_json(&shared).unwrap(), rules);

        let built = Rules::new().with(
            Rule::new(
                "water-when-dry",
                Condition::below("garden/bed-1/moisture", 30.0).with_hysteresis(5.0),
            )
            .then(Action::Drive {
                actuator: "bed-valve".into(),
                on: true,
            })
            .then(Action::Publish {
                topic: "garden/bed-1/valve".into(),
                payload: "open".into(),
            })
            .otherwise(Action::Drive {
                actuator: "bed-valve".into(),
                on: false,
            })
            .otherwise(Action::Publish {
                topic: "garden/bed-1/valve".into(),
                payload: "closed".into(),
            }),
        );
        assert_eq!(built, rules);
        assert_eq!(Condition::above("t", 1.0).compare, Compare::Above);
    }

    #[test]
    fn a_rule_file_is_checked_before_it_runs() {
        let ok = Rules::from_json(FILE).unwrap();
        ok.check().expect("a usable file");
        let unnamed = Rules::new().with(Rule::new("", Condition::above("t", 1.0)).then(
            Action::Publish {
                topic: "x".into(),
                payload: "y".into(),
            },
        ));
        assert!(matches!(unnamed.check(), Err(Error::Unsupported(_))));
        let idle = Rules::new().with(Rule::new("idle", Condition::above("t", 1.0)));
        assert!(matches!(idle.check(), Err(Error::Unsupported(_))));
        let twice = Rules::new()
            .with(
                Rule::new("a", Condition::above("t", 1.0)).then(Action::Publish {
                    topic: "x".into(),
                    payload: "y".into(),
                }),
            )
            .with(
                Rule::new("a", Condition::above("t", 1.0)).then(Action::Publish {
                    topic: "x".into(),
                    payload: "y".into(),
                }),
            );
        assert!(matches!(twice.check(), Err(Error::Unsupported(_))));
        let negative = Rules::new().with(
            Rule::new("a", Condition::above("t", 1.0).with_hysteresis(-1.0)).then(
                Action::Publish {
                    topic: "x".into(),
                    payload: "y".into(),
                },
            ),
        );
        assert!(matches!(negative.check(), Err(Error::Unsupported(_))));
        let nameless_actuator = Rules::new().with(Rule::new("a", Condition::above("t", 1.0)).then(
            Action::Drive {
                actuator: " ".into(),
                on: true,
            },
        ));
        assert!(matches!(
            nameless_actuator.check(),
            Err(Error::Unsupported(_))
        ));
    }

    #[tokio::test]
    async fn one_node_reading_drives_another_node_actuator_over_the_broker() {
        let broker = LoopbackBroker::new();
        let mut probe = LoopbackTransport::new(broker.clone());
        let mut watcher = LoopbackTransport::new(broker.clone());
        probe.connect().await.unwrap();
        watcher.connect().await.unwrap();
        watcher.subscribe("garden/bed-1/valve").await.unwrap();

        let valve = Valve::default();
        let mut engine = RuleEngine::new(
            Rules::from_json(FILE).unwrap(),
            LoopbackTransport::new(broker),
            JsonCodec,
        )
        .with_actuator("bed-valve", valve.clone());
        engine.connect().await.unwrap();
        assert_eq!(engine.rules().count(), 1);
        assert_eq!(engine.is_set("water-when-dry"), Some(false));
        assert_eq!(engine.is_set("nowhere"), None);

        let mut edges = Vec::new();
        for reading in [42.0f32, 31.0, 28.0, 33.0, 36.0] {
            let payload = JsonCodec.encode(&reading).unwrap();
            probe.send("garden/bed-1/moisture", &payload).await.unwrap();
            let fired = engine.step().await.unwrap().expect("the link is up");
            edges.push(fired.first().map(|fired| (fired.edge, fired.reading)));
        }
        assert_eq!(
            edges,
            [
                None,
                None,
                Some((Edge::Set, 28.0)),
                None,
                Some((Edge::Cleared, 36.0))
            ]
        );
        assert_eq!(*valve.0.lock().unwrap(), vec![true, false]);
        assert!(!engine.is_set("water-when-dry").unwrap());

        let open = watcher.recv().await.unwrap().expect("open");
        let closed = watcher.recv().await.unwrap().expect("closed");
        assert_eq!(
            (open.topic.as_str(), open.payload.as_slice()),
            ("garden/bed-1/valve", b"open".as_slice())
        );
        assert_eq!(closed.payload, b"closed");

        // A message on a topic no rule watches is not decoded at all.
        probe
            .send("garden/bed-1/note", b"not a number")
            .await
            .unwrap();
        assert_eq!(engine.step().await.unwrap(), Some(Vec::new()));
    }

    #[tokio::test]
    async fn an_actuator_a_rule_names_must_be_given() {
        let broker = LoopbackBroker::new();
        let mut engine = RuleEngine::new(
            Rules::from_json(FILE).unwrap(),
            LoopbackTransport::new(broker),
            JsonCodec,
        );
        assert!(matches!(engine.connect().await, Err(Error::Unsupported(_))));
    }
}
