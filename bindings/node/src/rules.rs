//! Generated Node bindings for rules between nodes.
//!
//! A rule file is judged here reading by reading, with no link: the program hands over
//! the topic and the reading each message carried, and learns which rules set or cleared
//! and what each calls for, which it then carries out with its own link and outputs.
//! That is the deciding half of the Rust `RuleEngine`, which owns a link and actuators
//! and so stays in Rust.

use napi_derive::napi;
use pamoja_kit::Edge as CoreEdge;
use pamoja_profile::{Action, Fired, RuleEvaluator as CoreEvaluator, Rules};

use crate::kit::Edge;

/// What a rule calls for when its condition sets or clears.
#[napi(string_enum = "lowercase")]
pub enum RuleActionKind {
    /// Switch the output the program holds under `actuator`.
    Drive,
    /// Publish `payload` to `topic`.
    Publish,
}

/// One thing a rule calls for, as the rule file writes it. Only the fields belonging to
/// `kind` are set.
#[napi(object)]
pub struct RuleAction {
    /// Whether to switch an output or publish a message.
    pub kind: RuleActionKind,
    /// The output to switch, for a drive.
    pub actuator: Option<String>,
    /// The setting to switch it to, for a drive.
    pub on: Option<bool>,
    /// The topic to publish to, for a publish.
    pub topic: Option<String>,
    /// The text to publish, for a publish.
    pub payload: Option<String>,
}

/// What one rule did with one reading.
#[napi(object)]
pub struct RuleFired {
    /// The rule's name.
    pub rule: String,
    /// Whether its condition set or cleared.
    pub edge: Edge,
    /// The reading that did it.
    pub reading: f64,
    /// What the rule calls for on this edge: its `then` actions when it set and its
    /// `otherwise` actions when it cleared, in the order the file gives them.
    pub actions: Vec<RuleAction>,
}

/// The decisions a rule file makes, reading by reading, with no link and no outputs of
/// its own.
#[napi]
pub struct RuleEvaluator {
    inner: CoreEvaluator,
}

#[napi]
impl RuleEvaluator {
    /// Loads a rule file and arms its rules, every condition starting cleared.
    ///
    /// Throws if the text is not a rule file, or holds a rule no engine could run, such
    /// as one that watches a filter or has nothing to do.
    #[napi(factory)]
    pub fn from_json(text: String) -> napi::Result<Self> {
        Rules::from_json(&text)
            .and_then(CoreEvaluator::new)
            .map(|inner| Self { inner })
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// Writes the rules back out as the file a fleet shares.
    #[napi]
    pub fn to_json(&self) -> napi::Result<String> {
        Rules {
            rules: self.inner.rules().cloned().collect(),
        }
        .to_json()
        .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// Judges one reading from one topic against every rule that watches it, and returns
    /// what fired in the order the rules are listed: empty when no rule watches the topic
    /// or the reading changed nothing.
    ///
    /// Throws if a rule watches the topic and the reading is not a finite number.
    #[napi]
    pub fn evaluate(&mut self, topic: String, reading: f64) -> napi::Result<Vec<RuleFired>> {
        self.inner
            .evaluate(&topic, reading as f32)
            .map(|fired| fired.into_iter().map(fired_of).collect())
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// Whether any rule watches a topic, so the program knows whether to decode a message.
    #[napi]
    pub fn watches(&self, topic: String) -> bool {
        self.inner.watches(&topic)
    }

    /// The topics the rules watch, each once, in name order: the topics to subscribe to.
    #[napi(getter)]
    pub fn topics(&self) -> Vec<String> {
        self.inner.topics().into_iter().map(str::to_owned).collect()
    }

    /// The actuators the rules drive, each once, in name order: the outputs the program
    /// has to have.
    #[napi(getter)]
    pub fn actuators(&self) -> Vec<String> {
        self.inner
            .actuators()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// Whether a rule's condition currently holds, or `null` for a name no rule has.
    #[napi]
    pub fn is_set(&self, rule: String) -> Option<bool> {
        self.inner.is_set(&rule)
    }
}

/// Flattens what one rule did into the object JavaScript sees.
fn fired_of(fired: Fired) -> RuleFired {
    RuleFired {
        rule: fired.rule,
        edge: match fired.edge {
            CoreEdge::Set => Edge::Set,
            CoreEdge::Cleared => Edge::Cleared,
        },
        reading: f64::from(fired.reading),
        actions: fired.actions.into_iter().map(action_of).collect(),
    }
}

/// Flattens one action into the object JavaScript sees.
fn action_of(action: Action) -> RuleAction {
    match action {
        Action::Drive { actuator, on } => RuleAction {
            kind: RuleActionKind::Drive,
            actuator: Some(actuator),
            on: Some(on),
            topic: None,
            payload: None,
        },
        Action::Publish { topic, payload } => RuleAction {
            kind: RuleActionKind::Publish,
            actuator: None,
            on: None,
            topic: Some(topic),
            payload: Some(payload),
        },
    }
}
