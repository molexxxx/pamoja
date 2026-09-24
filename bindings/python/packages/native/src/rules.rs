//! Generated Python bindings for rules between nodes.
//!
//! A rule file is judged here reading by reading, with no link: the program hands over
//! the topic and the reading each message carried, and learns which rules set or cleared
//! and what each calls for, which it then carries out with its own link and outputs.
//! That is the deciding half of the Rust `RuleEngine`, which owns a link and actuators
//! and so stays in Rust.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use pamoja_kit::Edge;
use pamoja_profile::{Action, RuleEvaluator as CoreEvaluator, Rules};

use crate::PamojaError;

/// One thing a rule calls for, as the rule file writes it.
///
/// Only the attributes belonging to `kind` are set; the rest are `None`.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct RuleAction {
    /// `drive` to switch an output, or `publish` to send a message.
    #[pyo3(get)]
    kind: String,
    /// The output to switch, for a drive.
    #[pyo3(get)]
    actuator: Option<String>,
    /// The setting to switch it to, for a drive.
    #[pyo3(get)]
    on: Option<bool>,
    /// The topic to publish to, for a publish.
    #[pyo3(get)]
    topic: Option<String>,
    /// The text to publish, for a publish.
    #[pyo3(get)]
    payload: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl RuleAction {
    fn __repr__(&self) -> String {
        match (&self.actuator, self.on, &self.topic, &self.payload) {
            (Some(actuator), Some(on), _, _) => format!(
                "RuleAction(kind='drive', actuator='{actuator}', on={})",
                if on { "True" } else { "False" }
            ),
            (_, _, Some(topic), Some(payload)) => {
                format!("RuleAction(kind='publish', topic='{topic}', payload='{payload}')")
            }
            _ => format!("RuleAction(kind='{}')", self.kind),
        }
    }
}

/// What one rule did with one reading.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct RuleFired {
    /// The rule's name.
    #[pyo3(get)]
    rule: String,
    /// `set` when the rule's condition became true, `cleared` when it stopped holding.
    #[pyo3(get)]
    edge: String,
    /// The reading that did it.
    #[pyo3(get)]
    reading: f32,
    /// What the rule calls for on this edge: its `then` actions when it set and its
    /// `otherwise` actions when it cleared, in the order the file gives them.
    #[pyo3(get)]
    actions: Vec<Py<RuleAction>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl RuleFired {
    fn __repr__(&self) -> String {
        format!(
            "RuleFired(rule='{}', edge='{}', reading={}, actions={})",
            self.rule,
            self.edge,
            self.reading,
            self.actions.len()
        )
    }
}

/// The decisions a rule file makes, reading by reading, with no link and no outputs of
/// its own.
#[gen_stub_pyclass]
#[pyclass]
pub struct RuleEvaluator {
    inner: CoreEvaluator,
}

#[gen_stub_pymethods]
#[pymethods]
impl RuleEvaluator {
    /// Loads a rule file and arms its rules, every condition starting cleared.
    ///
    /// Raises `PamojaError` if the text is not a rule file, or holds a rule no engine
    /// could run, such as one that watches a filter or has nothing to do.
    #[staticmethod]
    fn from_json(text: &str) -> PyResult<Self> {
        Rules::from_json(text)
            .and_then(CoreEvaluator::new)
            .map(|inner| Self { inner })
            .map_err(|error| PamojaError::new_err(error.to_string()))
    }

    /// Writes the rules back out as the file a fleet shares.
    fn to_json(&self) -> PyResult<String> {
        Rules {
            rules: self.inner.rules().cloned().collect(),
        }
        .to_json()
        .map_err(|error| PamojaError::new_err(error.to_string()))
    }

    /// Judges one reading from one topic against every rule that watches it, and returns
    /// what fired in the order the rules are listed: empty when no rule watches the topic
    /// or the reading changed nothing.
    ///
    /// Raises `PamojaError` if a rule watches the topic and the reading is not a finite
    /// number.
    fn evaluate(&mut self, py: Python<'_>, topic: &str, reading: f32) -> PyResult<Vec<RuleFired>> {
        let fired = self
            .inner
            .evaluate(topic, reading)
            .map_err(|error| PamojaError::new_err(error.to_string()))?;
        fired
            .into_iter()
            .map(|one| {
                let actions = one
                    .actions
                    .into_iter()
                    .map(|action| Py::new(py, action_of(action)))
                    .collect::<PyResult<Vec<_>>>()?;
                Ok(RuleFired {
                    rule: one.rule,
                    edge: match one.edge {
                        Edge::Set => "set",
                        Edge::Cleared => "cleared",
                    }
                    .to_owned(),
                    reading: one.reading,
                    actions,
                })
            })
            .collect()
    }

    /// Whether any rule watches a topic, so the program knows whether to decode a message.
    fn watches(&self, topic: &str) -> bool {
        self.inner.watches(topic)
    }

    /// The topics the rules watch, each once, in name order: the topics to subscribe to.
    #[getter]
    fn topics(&self) -> Vec<String> {
        self.inner.topics().into_iter().map(str::to_owned).collect()
    }

    /// The actuators the rules drive, each once, in name order: the outputs the program
    /// has to have.
    #[getter]
    fn actuators(&self) -> Vec<String> {
        self.inner
            .actuators()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// Whether a rule's condition currently holds, or `None` for a name no rule has.
    fn is_set(&self, rule: &str) -> Option<bool> {
        self.inner.is_set(rule)
    }
}

/// Flattens one action into the object Python sees.
fn action_of(action: Action) -> RuleAction {
    match action {
        Action::Drive { actuator, on } => RuleAction {
            kind: "drive".to_owned(),
            actuator: Some(actuator),
            on: Some(on),
            topic: None,
            payload: None,
        },
        Action::Publish { topic, payload } => RuleAction {
            kind: "publish".to_owned(),
            actuator: None,
            on: None,
            topic: Some(topic),
            payload: Some(payload),
        },
    }
}
