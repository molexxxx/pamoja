//! The parameters a manifest carries beside a control kind pamoja never shipped.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One parameter of a custom control kind: a number, a flag, or a piece of text.
///
/// A manifest writes it as the plain JSON value, so `"warn_below": 2.0`,
/// `"latching": true`, and `"zone": "north"` are all parameters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Param {
    /// A number; an integer in the manifest arrives as a whole number here.
    Number(f64),
    /// A flag.
    Flag(bool),
    /// A piece of text.
    Text(String),
}

/// The parameters of a custom control kind, by name.
///
/// These are every field a manifest carried beside `kind` for a kind the library does
/// not know, kept as data so the factory registered for the kind can read them. The
/// accessors answer `None` when a parameter is absent or of another type, so a factory
/// reports what it needed rather than guessing.
///
/// # Examples
///
/// ```
/// use pamoja_profile::{Param, Params};
///
/// let params = Params::new()
///     .with("warn_below", 2.0)
///     .with("latching", true)
///     .with("zone", "north");
/// assert_eq!(params.number("warn_below"), Some(2.0));
/// assert_eq!(params.flag("latching"), Some(true));
/// assert_eq!(params.text("zone"), Some("north"));
/// assert_eq!(params.number("zone"), None);
/// assert_eq!(params.get("missing"), None);
/// assert!(matches!(params.get("zone"), Some(Param::Text(_))));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Params(BTreeMap<String, Param>);

impl Params {
    /// Starts with no parameters.
    ///
    /// # Returns
    ///
    /// An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a parameter, replacing one of the same name.
    ///
    /// # Arguments
    ///
    /// * `name` - the parameter's name, as the manifest writes it.
    /// * `value` - a number, a flag, or text.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    pub fn with(mut self, name: impl Into<String>, value: impl Into<Param>) -> Self {
        self.0.insert(name.into(), value.into());
        self
    }

    /// Looks a parameter up by name.
    ///
    /// # Arguments
    ///
    /// * `name` - the parameter's name.
    ///
    /// # Returns
    ///
    /// The parameter, or `None` when the manifest carried none of that name.
    pub fn get(&self, name: &str) -> Option<&Param> {
        self.0.get(name)
    }

    /// Reads a numeric parameter.
    ///
    /// # Arguments
    ///
    /// * `name` - the parameter's name.
    ///
    /// # Returns
    ///
    /// The number, or `None` when the parameter is absent or not a number.
    pub fn number(&self, name: &str) -> Option<f64> {
        match self.0.get(name) {
            Some(Param::Number(value)) => Some(*value),
            _ => None,
        }
    }

    /// Reads a flag parameter.
    ///
    /// # Arguments
    ///
    /// * `name` - the parameter's name.
    ///
    /// # Returns
    ///
    /// The flag, or `None` when the parameter is absent or not a flag.
    pub fn flag(&self, name: &str) -> Option<bool> {
        match self.0.get(name) {
            Some(Param::Flag(value)) => Some(*value),
            _ => None,
        }
    }

    /// Reads a text parameter.
    ///
    /// # Arguments
    ///
    /// * `name` - the parameter's name.
    ///
    /// # Returns
    ///
    /// The text, or `None` when the parameter is absent or not text.
    pub fn text(&self, name: &str) -> Option<&str> {
        match self.0.get(name) {
            Some(Param::Text(value)) => Some(value),
            _ => None,
        }
    }

    /// Loads parameters from the JSON object a manifest carries beside a custom kind.
    ///
    /// # Arguments
    ///
    /// * `text` - the JSON object.
    ///
    /// # Returns
    ///
    /// The parameters.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) if the text is not an object
    /// of numbers, flags, and text.
    #[cfg(feature = "json")]
    pub fn from_json(text: &str) -> pamoja_core::Result<Self> {
        serde_json::from_str(text).map_err(|error| pamoja_core::Error::Codec(error.to_string()))
    }

    /// Writes the parameters as the JSON object a manifest carries beside a custom kind.
    ///
    /// # Returns
    ///
    /// The JSON text, on one line.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) if the parameters cannot be
    /// serialized.
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> pamoja_core::Result<String> {
        serde_json::to_string(self).map_err(|error| pamoja_core::Error::Codec(error.to_string()))
    }

    /// Walks the parameters in name order.
    ///
    /// # Returns
    ///
    /// Each name with its parameter.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Param)> {
        self.0.iter().map(|(name, value)| (name.as_str(), value))
    }

    /// Counts the parameters.
    ///
    /// # Returns
    ///
    /// How many the manifest carried.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no parameters.
    ///
    /// # Returns
    ///
    /// `true` when the manifest carried only the kind.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<BTreeMap<String, Param>> for Params {
    fn from(map: BTreeMap<String, Param>) -> Self {
        Self(map)
    }
}

impl From<Params> for BTreeMap<String, Param> {
    fn from(params: Params) -> Self {
        params.0
    }
}

impl From<f64> for Param {
    fn from(value: f64) -> Self {
        Param::Number(value)
    }
}

impl From<f32> for Param {
    fn from(value: f32) -> Self {
        Param::Number(f64::from(value))
    }
}

impl From<i32> for Param {
    fn from(value: i32) -> Self {
        Param::Number(f64::from(value))
    }
}

impl From<u32> for Param {
    fn from(value: u32) -> Self {
        Param::Number(f64::from(value))
    }
}

impl From<bool> for Param {
    fn from(value: bool) -> Self {
        Param::Flag(value)
    }
}

impl From<&str> for Param {
    fn from(value: &str) -> Self {
        Param::Text(value.to_owned())
    }
}

impl From<String> for Param {
    fn from(value: String) -> Self {
        Param::Text(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_keep_their_type_and_round_trip_as_json() {
        let params = Params::new()
            .with("warn_below", 2.5f32)
            .with("samples", 4u32)
            .with("latching", false)
            .with("zone", "north");
        let json = serde_json::to_string(&params).unwrap();
        assert_eq!(
            json,
            r#"{"latching":false,"samples":4.0,"warn_below":2.5,"zone":"north"}"#
        );
        let restored: Params = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, params);
        assert_eq!(restored.number("samples"), Some(4.0));
        assert_eq!(restored.flag("latching"), Some(false));
        assert_eq!(restored.text("zone"), Some("north"));
        assert_eq!(restored.len(), 4);
        assert!(!restored.is_empty());
        assert_eq!(
            restored.iter().map(|(name, _)| name).collect::<Vec<_>>(),
            ["latching", "samples", "warn_below", "zone"]
        );
    }

    #[test]
    fn an_integer_in_a_manifest_is_a_number() {
        let params: Params = serde_json::from_str(r#"{ "count": 3, "on": true }"#).unwrap();
        assert_eq!(params.number("count"), Some(3.0));
        assert_eq!(params.flag("on"), Some(true));
        assert_eq!(params.flag("count"), None);
    }
}
