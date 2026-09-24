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

    /// Checks the rules for what a file can say that no engine could run.
    ///
    /// # Returns
    ///
    /// Nothing when every rule is usable.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`] naming the first problem and the rule it is in: a rule
    /// without a name or with a name another rule has, a condition without a topic or
    /// watching a filter with `+` or `#`, a threshold that is not a finite number, a
    /// hysteresis that is not a finite number of zero or more, a rule with nothing to
    /// do, or an action naming no actuator, no topic, or a filter to publish to.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::{Action, Condition, Rule, Rules};
    ///
    /// // A rule on a filter would subscribe to every bed and match none of them, since a
    /// // rule compares each message's topic with its own exactly.
    /// let every_bed = Rules::new().with(
    ///     Rule::new("water-when-dry", Condition::below("garden/+/moisture", 30.0))
    ///         .then(Action::Drive { actuator: "bed-valve".into(), on: true }),
    /// );
    /// let refused = every_bed.check().unwrap_err().to_string();
    /// assert!(refused.contains("a rule watches one topic exactly"));
    /// ```
    pub fn check(&self) -> Result<()> {
        let refuse = |reason: String| Err(Error::Codec(reason));
        let filter = |topic: &str| topic.contains(['+', '#']);
        let mut names = BTreeSet::new();
        for rule in &self.rules {
            let name = &rule.name;
            if name.trim().is_empty() {
                return refuse("a rule needs a name".to_owned());
            }
            if !names.insert(name.as_str()) {
                return refuse(format!("two rules share the name `{name}`"));
            }
            let topic = &rule.when.topic;
            if topic.trim().is_empty() {
                return refuse(format!("the rule `{name}` needs a topic to watch"));
            }
            if filter(topic) {
                return refuse(format!(
                    "the rule `{name}` watches `{topic}`, a filter; a rule watches one topic exactly"
                ));
            }
            if !rule.when.threshold.is_finite() {
                return refuse(format!(
                    "the rule `{name}` needs a threshold that is a finite number, not {}",
                    rule.when.threshold
                ));
            }
            if !(rule.when.hysteresis.is_finite() && rule.when.hysteresis >= 0.0) {
                return refuse(format!(
                    "the rule `{name}` needs a hysteresis that is a finite number of zero or more, not {}",
                    rule.when.hysteresis
                ));
            }
            if rule.then.is_empty() && rule.otherwise.is_empty() {
                return refuse(format!("the rule `{name}` has nothing to do"));
            }
            for action in rule.then.iter().chain(&rule.otherwise) {
                match action {
                    Action::Drive { actuator, .. } if actuator.trim().is_empty() => {
                        return refuse(format!(
                            "the rule `{name}` drives an actuator with no name"
                        ));
                    }
                    Action::Publish { topic, .. } if topic.trim().is_empty() => {
                        return refuse(format!("the rule `{name}` publishes to no topic"));
                    }
                    Action::Publish { topic, .. } if filter(topic) => {
                        return refuse(format!(
                            "the rule `{name}` publishes to `{topic}`, a filter rather than a topic"
                        ));
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
    /// What the rule calls for on this edge: its `then` actions when it set and its
    /// `otherwise` actions when it cleared, in the order the file gives them.
    pub actions: Vec<Action>,
}

/// The decisions a set of rules makes, reading by reading, with no link and no outputs
/// of its own.
///
/// A [`RuleEngine`] receives each message, decodes it, and carries out what a rule calls
/// for. This is its deciding half on its own, for a program that moves its own messages
/// and drives its own outputs, as the program does in every language but Rust: hand it a
/// topic and a reading, and it says which rules set or cleared and what each calls for.
/// The engine runs on one, so the two decide alike.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Edge;
/// use pamoja_profile::{Action, Condition, Rule, RuleEvaluator, Rules};
///
/// // Open a roof vent when a greenhouse passes 30 C, and close it once it is back
/// // under 27 C.
/// let rules = Rules::new().with(
///     Rule::new("vent-when-hot", Condition::above("greenhouse/air", 30.0).with_hysteresis(3.0))
///         .then(Action::Drive { actuator: "roof-vent".into(), on: true })
///         .otherwise(Action::Drive { actuator: "roof-vent".into(), on: false }),
/// );
/// let mut evaluator = RuleEvaluator::new(rules)?;
/// assert!(evaluator.evaluate("greenhouse/air", 29.0)?.is_empty());
///
/// let fired = evaluator.evaluate("greenhouse/air", 31.5)?;
/// assert_eq!(fired[0].edge, Edge::Set);
/// assert_eq!(fired[0].actions, [Action::Drive { actuator: "roof-vent".into(), on: true }]);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
pub struct RuleEvaluator {
    armed: Vec<Armed>,
}

impl RuleEvaluator {
    /// Arms a set of rules, every condition starting cleared.
    ///
    /// # Arguments
    ///
    /// * `rules` - the rules to judge readings by.
    ///
    /// # Returns
    ///
    /// The evaluator.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`] naming the first problem [`Rules::check`] finds.
    pub fn new(rules: Rules) -> Result<Self> {
        rules.check()?;
        Ok(Self::unchecked(rules))
    }

    /// Arms a set of rules without checking them, for an engine that checks as it
    /// connects.
    fn unchecked(rules: Rules) -> Self {
        Self {
            armed: rules
                .rules
                .into_iter()
                .map(|rule| Armed {
                    trigger: rule.when.trigger(),
                    rule,
                })
                .collect(),
        }
    }

    /// Judges one reading from one topic against every rule that watches it.
    ///
    /// # Arguments
    ///
    /// * `topic` - the topic the reading arrived on.
    /// * `reading` - the reading, already decoded from the message.
    ///
    /// # Returns
    ///
    /// What fired, in the order the rules are listed: each rule whose condition set or
    /// cleared, with the actions it calls for. It is empty when no rule watches the
    /// topic or the reading changed nothing.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`] when a rule watches the topic and the reading is not a
    /// finite number, such as the NaN a failed probe reports, which would otherwise
    /// leave every condition as it was with nothing to say why.
    pub fn evaluate(&mut self, topic: &str, reading: f32) -> Result<Vec<Fired>> {
        let watching = self.watching(topic);
        if watching.is_empty() {
            return Ok(Vec::new());
        }
        finite(topic, reading)?;
        Ok(watching
            .into_iter()
            .filter_map(|index| self.judge(index, reading))
            .collect())
    }

    /// Whether any rule watches a topic, so a program knows whether to decode a message
    /// before handing it over.
    ///
    /// # Arguments
    ///
    /// * `topic` - the topic a message arrived on.
    ///
    /// # Returns
    ///
    /// `true` when some rule watches it.
    pub fn watches(&self, topic: &str) -> bool {
        self.armed
            .iter()
            .any(|armed| armed.rule.when.topic == topic)
    }

    /// Lists the topics the rules watch, which are the topics to subscribe to.
    ///
    /// # Returns
    ///
    /// Each topic once, in name order.
    pub fn topics(&self) -> Vec<&str> {
        let topics: BTreeSet<&str> = self
            .armed
            .iter()
            .map(|armed| armed.rule.when.topic.as_str())
            .collect();
        topics.into_iter().collect()
    }

    /// Lists the actuators the rules drive, which are the outputs a program has to have.
    ///
    /// # Returns
    ///
    /// Each actuator once, in name order.
    pub fn actuators(&self) -> Vec<&str> {
        let actuators: BTreeSet<&str> = self
            .armed
            .iter()
            .flat_map(|armed| armed.rule.actuators())
            .collect();
        actuators.into_iter().collect()
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

    /// The positions of the rules that watch a topic, in the order they are listed.
    fn watching(&self, topic: &str) -> Vec<usize> {
        self.armed
            .iter()
            .enumerate()
            .filter(|(_, armed)| armed.rule.when.topic == topic)
            .map(|(index, _)| index)
            .collect()
    }

    /// Hands one rule a reading, and says what it did if its condition moved.
    fn judge(&mut self, index: usize, reading: f32) -> Option<Fired> {
        let armed = &mut self.armed[index];
        let edge = armed.trigger.update(reading)?;
        let actions = match edge {
            Edge::Set => armed.rule.then.clone(),
            Edge::Cleared => armed.rule.otherwise.clone(),
        };
        Some(Fired {
            rule: armed.rule.name.clone(),
            edge,
            reading,
            actions,
        })
    }
}

/// Refuses a reading on a watched topic that is not a finite number.
fn finite(topic: &str, reading: f32) -> Result<()> {
    if reading.is_finite() {
        Ok(())
    } else {
        Err(Error::Codec(format!(
            "the reading on `{topic}` is {reading}, which is not a finite number"
        )))
    }
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
    evaluator: RuleEvaluator,
    link: L,
    codec: C,
    actuators: BTreeMap<String, Box<dyn ErasedActuator>>,
}

impl<L, C> RuleEngine<L, C> {
    /// Assembles an engine around a link and the codec the readings arrive in.
    ///
    /// # Arguments
    ///
    /// * `rules` - the rules to run, checked when the engine connects.
    /// * `link` - the link readings arrive on and actions publish over; connected by
    ///   [`connect`](RuleEngine::connect).
    /// * `codec` - the wire format the nodes publish their readings in.
    ///
    /// # Returns
    ///
    /// An engine with no actuators yet.
    pub fn new(rules: Rules, link: L, codec: C) -> Self {
        Self {
            evaluator: RuleEvaluator::unchecked(rules),
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
        self.evaluator.rules()
    }

    /// Lists the topics the rules watch, which the engine subscribes to as it connects.
    ///
    /// # Returns
    ///
    /// Each topic once, in name order.
    pub fn topics(&self) -> Vec<&str> {
        self.evaluator.topics()
    }

    /// Lists the actuators the rules drive, each of which the engine has to be given.
    ///
    /// # Returns
    ///
    /// Each actuator once, in name order.
    pub fn actuators(&self) -> Vec<&str> {
        self.evaluator.actuators()
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
        self.evaluator.is_set(rule)
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
    /// Returns [`Error::Codec`] if a rule cannot be run (see [`Rules::check`]),
    /// [`Error::Unsupported`] if a rule drives an actuator the engine was not given, and
    /// the link's own error if it cannot connect or subscribe.
    pub async fn connect(&mut self) -> Result<()> {
        Rules {
            rules: self.evaluator.rules().cloned().collect(),
        }
        .check()?;
        if self
            .evaluator
            .actuators()
            .iter()
            .any(|actuator| !self.actuators.contains_key(*actuator))
        {
            return Err(Error::Unsupported(
                "a rule drives an actuator the engine was not given",
            ));
        }
        self.link.connect().await?;
        for topic in self.evaluator.topics() {
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
    /// [`Error::Codec`] if a reading on a watched topic does not decode or is not a
    /// finite number, and the actuator's error if a drive fails.
    pub async fn step(&mut self) -> Result<Option<Vec<Fired>>> {
        let Some(message) = self.link.recv().await? else {
            return Ok(None);
        };
        let watching = self.evaluator.watching(&message.topic);
        let mut fired = Vec::new();
        if watching.is_empty() {
            return Ok(Some(fired));
        }
        let reading: f32 = self.codec.decode(&message.payload)?;
        finite(&message.topic, reading)?;
        for index in watching {
            let Some(one) = self.evaluator.judge(index, reading) else {
                continue;
            };
            for action in &one.actions {
                self.run(action).await?;
            }
            fired.push(one);
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
        let publish = || Action::Publish {
            topic: "x".into(),
            payload: "y".into(),
        };
        let refused = |rules: Rules| match rules.check() {
            Err(Error::Codec(reason)) => reason,
            other => panic!("expected a codec error, got {other:?}"),
        };

        let unnamed = Rules::new().with(Rule::new("", Condition::above("t", 1.0)).then(publish()));
        assert_eq!(refused(unnamed), "a rule needs a name");
        let idle = Rules::new().with(Rule::new("idle", Condition::above("t", 1.0)));
        assert_eq!(refused(idle), "the rule `idle` has nothing to do");
        let twice = Rules::new()
            .with(Rule::new("a", Condition::above("t", 1.0)).then(publish()))
            .with(Rule::new("a", Condition::above("t", 1.0)).then(publish()));
        assert_eq!(refused(twice), "two rules share the name `a`");
        let negative = Rules::new()
            .with(Rule::new("a", Condition::above("t", 1.0).with_hysteresis(-1.0)).then(publish()));
        assert!(refused(negative).contains("a finite number of zero or more, not -1"));
        let nameless_actuator = Rules::new().with(Rule::new("a", Condition::above("t", 1.0)).then(
            Action::Drive {
                actuator: " ".into(),
                on: true,
            },
        ));
        assert_eq!(
            refused(nameless_actuator),
            "the rule `a` drives an actuator with no name"
        );
        let filter =
            Rules::new().with(Rule::new("a", Condition::above("t/+", 1.0)).then(publish()));
        assert!(refused(filter).contains("a rule watches one topic exactly"));
        let broadcast = Rules::new().with(Rule::new("a", Condition::above("t", 1.0)).then(
            Action::Publish {
                topic: "alerts/#".into(),
                payload: "hot".into(),
            },
        ));
        assert!(refused(broadcast).contains("a filter rather than a topic"));
    }

    #[test]
    fn an_evaluator_decides_what_the_engine_would_and_says_what_to_do() {
        let file = Rules::from_json(FILE).unwrap().with(
            Rule::new(
                "flood-alarm",
                Condition::above("garden/bed-1/moisture", 60.0).with_hysteresis(5.0),
            )
            .then(Action::Publish {
                topic: "garden/alarm".into(),
                payload: "waterlogged".into(),
            }),
        );
        let mut evaluator = RuleEvaluator::new(file).expect("a usable file");
        assert_eq!(evaluator.topics(), ["garden/bed-1/moisture"]);
        assert_eq!(evaluator.actuators(), ["bed-valve"]);
        assert!(evaluator.watches("garden/bed-1/moisture"));
        assert!(!evaluator.watches("garden/bed-2/moisture"));
        assert!(evaluator
            .evaluate("garden/bed-2/moisture", 5.0)
            .unwrap()
            .is_empty());

        let dry = evaluator.evaluate("garden/bed-1/moisture", 28.0).unwrap();
        assert_eq!(dry.len(), 1);
        assert_eq!(
            (dry[0].rule.as_str(), dry[0].edge),
            ("water-when-dry", Edge::Set)
        );
        assert_eq!(dry[0].actions.len(), 2);
        assert_eq!(evaluator.is_set("water-when-dry"), Some(true));

        let soaked = evaluator.evaluate("garden/bed-1/moisture", 65.0).unwrap();
        let names: Vec<&str> = soaked.iter().map(|fired| fired.rule.as_str()).collect();
        assert_eq!(
            names,
            ["water-when-dry", "flood-alarm"],
            "in the order the file lists them"
        );
        assert_eq!(soaked[0].edge, Edge::Cleared);
        assert_eq!(
            soaked[1].actions,
            [Action::Publish {
                topic: "garden/alarm".into(),
                payload: "waterlogged".into(),
            }]
        );

        let failed = evaluator.evaluate("garden/bed-1/moisture", f32::NAN);
        assert!(
            matches!(&failed, Err(Error::Codec(reason)) if reason.contains("not a finite number")),
            "{failed:?}"
        );
        assert!(
            evaluator
                .evaluate("garden/bed-2/moisture", f32::NAN)
                .unwrap()
                .is_empty(),
            "a topic no rule watches is not judged"
        );
        assert_eq!(evaluator.is_set("flood-alarm"), Some(true));

        let unusable = Rules::new().with(Rule::new("idle", Condition::above("t", 1.0)));
        assert!(matches!(RuleEvaluator::new(unusable), Err(Error::Codec(_))));
    }

    #[tokio::test]
    async fn one_node_reading_drives_another_node_actuator_over_the_broker() {
        let broker = LoopbackBroker::new();
        let mut probe = LoopbackTransport::new(broker.clone());
        let mut watcher = LoopbackTransport::new(broker.clone());
        probe.connect().await.unwrap();
        watcher.connect().await.unwrap();
        watcher.subscribe("garden/bed-1/valve").await.unwrap();

        // The engine's link carries one topic no rule watches, subscribed before the
        // engine takes it over.
        let mut link = LoopbackTransport::new(broker);
        link.connect().await.unwrap();
        link.subscribe("garden/bed-1/note").await.unwrap();

        let valve = Valve::default();
        let mut engine = RuleEngine::new(Rules::from_json(FILE).unwrap(), link, JsonCodec)
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

        // The engine reports each edge with the actions it carried out.
        let payload = JsonCodec.encode(&27.0f32).unwrap();
        probe.send("garden/bed-1/moisture", &payload).await.unwrap();
        let fired = engine.step().await.unwrap().expect("the link is up");
        assert_eq!(
            fired[0].actions,
            Rules::from_json(FILE).unwrap().rules[0].then
        );
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
