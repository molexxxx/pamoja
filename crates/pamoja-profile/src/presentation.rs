//! How a profile presents its custom elements on the local-first dashboard.
//!
//! The dashboard renders a language-neutral fleet snapshot and, by default, picks a
//! graphic for each reading from its key and unit. That covers the common quantities,
//! but a community often measures something we never anticipated - a water turbidity
//! probe, a pH meter, a custom node stat. A [`Presentation`] lets a profile *declare*
//! those elements as plain data: the graphic to draw them with, their safe band, a
//! label, which groups they are offered on, and a small theme. It is part of the same
//! shareable manifest a community already authors, so a new sensor type needs no code
//! and no change to the dashboard.
//!
//! The declaration is presentation only. Values still travel in the snapshot as raw
//! numbers and stable keys; this names how to *show* them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The graphic a reading is drawn with on the dashboard.
///
/// The names are the instrument, not the quantity, so a profile chooses the shape that
/// reads best for its data: a 270-degree arch [`Gauge`](Viz::Gauge) for a fraction, a
/// [`Bar`](Viz::Bar) for a tank, a [`Switch`](Viz::Switch) for an on/off state. Each
/// maps to one of the dashboard's hand-drawn visualizations through
/// [`kind`](Viz::kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Viz {
    /// A rolling sparkline of recent values. The default for an unfamiliar quantity.
    Spark,
    /// A 270-degree arch gauge, for a fraction or percentage.
    Gauge,
    /// A half-dial with a needle, for a pressure or flow reading.
    Dial,
    /// A horizontal bar with a safe-band tick, for a level or stock.
    Bar,
    /// A thermometer, for a temperature.
    Thermometer,
    /// A liquid-filled droplet, for humidity or moisture.
    Droplet,
    /// A segmented battery cell, for a state of charge or voltage.
    Battery,
    /// An anemometer, for wind speed.
    Wind,
    /// A sun whose corona grows with the reading, for illuminance.
    Sun,
    /// An acoustic waveform, for sound level or an acoustic event.
    Wave,
    /// A labeled state chip, lit when the state reads as "on". For a discrete state.
    Switch,
    /// A pipe valve, open along the flow or closed across it. For a controllable valve.
    Valve,
    /// A row of hash-chained blocks, for a tamper-evident record count.
    Chain,
    /// A neighbour-mesh topology map, for a mesh node's peers.
    Mesh,
    /// A plain numeric counter, for a node or network stat.
    Count,
}

impl Viz {
    /// Every graphic, in a stable order, so a caller can enumerate the available
    /// visualizations (and a build can check each one still renders).
    pub const ALL: [Viz; 15] = [
        Viz::Spark,
        Viz::Gauge,
        Viz::Dial,
        Viz::Bar,
        Viz::Thermometer,
        Viz::Droplet,
        Viz::Battery,
        Viz::Wind,
        Viz::Sun,
        Viz::Wave,
        Viz::Switch,
        Viz::Valve,
        Viz::Chain,
        Viz::Mesh,
        Viz::Count,
    ];

    /// Returns the dashboard visualization kind this graphic renders as.
    ///
    /// The dashboard's renderer dispatches on a small set of internal kind strings; a
    /// few friendly names differ from them ([`Gauge`](Viz::Gauge) draws the `radial`
    /// arch, [`Thermometer`](Viz::Thermometer) the `therm` instrument,
    /// [`Switch`](Viz::Switch) the `chip`). This is the value carried on the wire so the
    /// page needs no lookup of its own.
    ///
    /// # Returns
    ///
    /// The stable visualization kind, such as `"radial"` or `"bar"`.
    pub fn kind(self) -> &'static str {
        match self {
            Viz::Spark => "spark",
            Viz::Gauge => "radial",
            Viz::Dial => "dial",
            Viz::Bar => "bar",
            Viz::Thermometer => "therm",
            Viz::Droplet => "droplet",
            Viz::Battery => "battery",
            Viz::Wind => "wind",
            Viz::Sun => "sun",
            Viz::Wave => "wave",
            Viz::Switch => "chip",
            Viz::Valve => "valve",
            Viz::Chain => "chain",
            Viz::Mesh => "mesh",
            Viz::Count => "count",
        }
    }
}

/// Which groups a declared element is offered on when a user adds a sensor.
///
/// A custom element rarely makes sense everywhere: a mesh-routing stat belongs only on
/// a mesh node, while a quality-of-life detector a community wants on every node is
/// [`Always`](Scope::Always). This gates the add-sensor dialog so a profile's element
/// appears only where it applies.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Offered on every group, whatever its link.
    #[default]
    Always,
    /// Offered only on groups whose link kind is one of these, such as `["mesh"]`.
    Links(Vec<String>),
}

/// A custom sensor or node stat a profile contributes to the dashboard.
///
/// This is the unit of a [`Presentation`]: one element keyed by a stable, language-
/// neutral key, drawn with a chosen [`Viz`], scoped to the groups it belongs on, and
/// labeled for people who do not read the key. The snapshot still carries the raw
/// value under `key`; this names how to show it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElementSpec {
    /// The stable, language-neutral element key, such as `"water_turbidity"`.
    pub key: String,
    /// The canonical unit name, such as `"ntu"`, `"ph"`, or `"count"`.
    pub unit: String,
    /// A human-readable fallback label, shown when no localized label is available.
    pub label: String,
    /// Optional per-locale labels, keyed by locale tag (`"en"`, `"sw"`, ...). A locale
    /// present here is shown in that locale; otherwise the page falls back to
    /// [`label`](ElementSpec::label).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<BTreeMap<String, String>>,
    /// The graphic this element is drawn with.
    pub viz: Viz,
    /// The safe band `[low, high]` in the element's unit, drawn as the gauge's safe zone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band: Option<[f32; 2]>,
    /// Whether this is a node or network stat rather than a measurement of the world.
    /// Stats are counted and rendered apart from sensors. Defaults `false`.
    #[serde(default)]
    pub stat: bool,
    /// Which groups this element is offered on. Defaults to [`Scope::Always`].
    #[serde(default)]
    pub scope: Scope,
    /// Whether the element's tile spans two columns, for a wide graphic. Defaults `false`.
    #[serde(default)]
    pub span: bool,
    /// A starting numeric value for the add-sensor dialog, before a real sample arrives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f32>,
    /// A starting discrete state code, such as `"state.closed"`, for a non-numeric element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl ElementSpec {
    /// Declares a numeric element drawn with the given graphic.
    ///
    /// # Arguments
    ///
    /// * `key` - the stable, language-neutral element key.
    /// * `unit` - the canonical unit name.
    /// * `label` - a human-readable fallback label.
    /// * `viz` - the graphic to draw it with.
    ///
    /// # Returns
    ///
    /// A measurement element offered on every group, with no band yet.
    pub fn new(
        key: impl Into<String>,
        unit: impl Into<String>,
        label: impl Into<String>,
        viz: Viz,
    ) -> Self {
        Self {
            key: key.into(),
            unit: unit.into(),
            label: label.into(),
            labels: None,
            viz,
            band: None,
            stat: false,
            scope: Scope::Always,
            span: false,
            value: None,
            state: None,
        }
    }

    /// Sets the safe band drawn as the graphic's safe zone.
    ///
    /// # Arguments
    ///
    /// * `low` - the bottom of the safe band.
    /// * `high` - the top of the safe band.
    ///
    /// # Returns
    ///
    /// The element, for chaining.
    pub fn with_band(mut self, low: f32, high: f32) -> Self {
        self.band = Some([low, high]);
        self
    }

    /// Restricts the groups this element is offered on.
    ///
    /// # Arguments
    ///
    /// * `scope` - the groups the add-sensor dialog offers this element on.
    ///
    /// # Returns
    ///
    /// The element, for chaining.
    pub fn on(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Marks the element as a node or network stat rather than a measurement.
    ///
    /// # Returns
    ///
    /// The element, for chaining.
    pub fn as_stat(mut self) -> Self {
        self.stat = true;
        self
    }

    /// Sets a starting value shown until the first real sample arrives.
    ///
    /// # Arguments
    ///
    /// * `value` - the starting numeric value.
    ///
    /// # Returns
    ///
    /// The element, for chaining.
    pub fn with_value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets a starting discrete state code for a non-numeric element.
    ///
    /// # Arguments
    ///
    /// * `state` - the starting state code, such as `"state.closed"`.
    ///
    /// # Returns
    ///
    /// The element, for chaining.
    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Spans the element's tile across two columns, for a wide graphic.
    ///
    /// # Returns
    ///
    /// The element, for chaining.
    pub fn wide(mut self) -> Self {
        self.span = true;
        self
    }

    /// Adds a localized label for one locale.
    ///
    /// # Arguments
    ///
    /// * `locale` - the locale tag, such as `"sw"`.
    /// * `label` - the element's label in that locale.
    ///
    /// # Returns
    ///
    /// The element, for chaining.
    pub fn with_locale_label(
        mut self,
        locale: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        self.labels
            .get_or_insert_with(BTreeMap::new)
            .insert(locale.into(), label.into());
        self
    }
}

/// A small set of theme tokens a profile can set on the dashboard.
///
/// Each token, when present, tints one of the page's CSS custom properties, so a
/// deployment can carry its own brand accent and status palette. Modest by design: it
/// tints the existing console rather than restyling it. Colors are any CSS color.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    /// The brand/interaction accent (links, focus glow, brand mark), such as `"#3fb1c8"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    /// The healthy/ok status color, which also tints an in-band gauge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<String>,
    /// The warning status color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warn: Option<String>,
    /// The alarm status color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alarm: Option<String>,
    /// The unfilled track/rail color behind gauges and progress bars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<String>,
}

/// A piece of human-facing text a profile supplies, either one string for every locale or
/// a per-locale map.
///
/// Used for the messages a profile localizes for the dashboard - its custom discrete state
/// codes and event codes. In a manifest it is a bare string for the simple case or an
/// object keyed by locale tag:
///
/// ```json
/// { "state.flushing": "Flushing", "event.filter_clog": { "en": "Filter clogged", "sw": "Kichujio kimeziba" } }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocalizedText {
    /// One text shown in every locale that has no specific translation.
    Plain(String),
    /// Per-locale text, keyed by locale tag (`"en"`, `"sw"`, ...).
    PerLocale(BTreeMap<String, String>),
}

impl From<String> for LocalizedText {
    fn from(text: String) -> Self {
        LocalizedText::Plain(text)
    }
}

impl From<&str> for LocalizedText {
    fn from(text: &str) -> Self {
        LocalizedText::Plain(text.to_owned())
    }
}

impl From<BTreeMap<String, String>> for LocalizedText {
    fn from(map: BTreeMap<String, String>) -> Self {
        LocalizedText::PerLocale(map)
    }
}

/// How a profile presents itself on the dashboard: its custom elements and theme.
///
/// A [`Profile`](crate::Profile) carries an optional `presentation`, so a deployment's
/// dashboard offers exactly the sensor types its profiles introduce and renders them
/// the way the profile intends. The dashboard turns these declarations into the catalog
/// it serves to the page.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Presentation {
    /// The custom sensors and node stats this profile contributes.
    #[serde(default)]
    pub elements: Vec<ElementSpec>,
    /// An optional theme that tints the dashboard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    /// Localized text for the stable codes this profile introduces, keyed by the page's
    /// message key (a discrete state such as `"state.flushing"` or an event such as
    /// `"event.filter_clog"`). The dashboard ships no translation for a code it never knew,
    /// so a profile that emits a custom state or event supplies its wording here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub messages: BTreeMap<String, LocalizedText>,
}

impl Presentation {
    /// Starts an empty presentation.
    ///
    /// # Returns
    ///
    /// A presentation with no elements and no theme.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads a presentation from its JSON text, the `presentation` object of a manifest.
    ///
    /// # Arguments
    ///
    /// * `text` - the JSON object.
    ///
    /// # Returns
    ///
    /// The presentation.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) if the text is not a
    /// presentation.
    #[cfg(feature = "json")]
    pub fn from_json(text: &str) -> pamoja_core::Result<Self> {
        serde_json::from_str(text).map_err(|error| pamoja_core::Error::Codec(error.to_string()))
    }

    /// Serializes this presentation to the JSON object a manifest carries.
    ///
    /// # Returns
    ///
    /// The pretty-printed JSON text.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) if the presentation cannot be
    /// serialized.
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> pamoja_core::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|error| pamoja_core::Error::Codec(error.to_string()))
    }

    /// Adds a custom element.
    ///
    /// # Arguments
    ///
    /// * `element` - the sensor or stat to contribute.
    ///
    /// # Returns
    ///
    /// The presentation, for chaining.
    pub fn with_element(mut self, element: ElementSpec) -> Self {
        self.elements.push(element);
        self
    }

    /// Sets the theme that tints the dashboard.
    ///
    /// # Arguments
    ///
    /// * `theme` - the theme tokens to apply.
    ///
    /// # Returns
    ///
    /// The presentation, for chaining.
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Supplies localized text for a stable code this profile introduces.
    ///
    /// Use this for a custom discrete state or event the dashboard ships no wording for, so
    /// the page renders it as words rather than the raw code.
    ///
    /// # Arguments
    ///
    /// * `key` - the page message key, such as `"state.flushing"` or `"event.filter_clog"`.
    /// * `text` - the text, one string for every locale or a per-locale map.
    ///
    /// # Returns
    ///
    /// The presentation, for chaining.
    pub fn with_message(mut self, key: impl Into<String>, text: impl Into<LocalizedText>) -> Self {
        self.messages.insert(key.into(), text.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_viz_maps_to_its_documented_render_kind() {
        // The full set of graphics and the render kind each draws as. ALL and this table must
        // agree, and every kind is distinct, so a new graphic cannot silently collide or be
        // left out of the enumerated set.
        let table = [
            (Viz::Spark, "spark"),
            (Viz::Gauge, "radial"),
            (Viz::Dial, "dial"),
            (Viz::Bar, "bar"),
            (Viz::Thermometer, "therm"),
            (Viz::Droplet, "droplet"),
            (Viz::Battery, "battery"),
            (Viz::Wind, "wind"),
            (Viz::Sun, "sun"),
            (Viz::Wave, "wave"),
            (Viz::Switch, "chip"),
            (Viz::Valve, "valve"),
            (Viz::Chain, "chain"),
            (Viz::Mesh, "mesh"),
            (Viz::Count, "count"),
        ];
        assert_eq!(
            table.len(),
            Viz::ALL.len(),
            "the table covers every variant in ALL"
        );
        let mut kinds = std::collections::HashSet::new();
        for (viz, kind) in table {
            assert_eq!(viz.kind(), kind);
            assert!(Viz::ALL.contains(&viz), "{kind} is in ALL");
            assert!(kinds.insert(kind), "{kind} is unique");
        }
    }

    #[test]
    fn an_element_builds_with_band_and_scope() {
        let element = ElementSpec::new("water_turbidity", "ntu", "Turbidity", Viz::Gauge)
            .with_band(0.0, 5.0)
            .on(Scope::Links(vec!["mesh".into()]));
        assert_eq!(element.band, Some([0.0, 5.0]));
        assert!(matches!(element.scope, Scope::Links(_)));
        assert!(!element.stat);
    }

    #[cfg(feature = "json")]
    #[test]
    fn viz_serializes_to_its_friendly_name() {
        assert_eq!(serde_json::to_string(&Viz::Gauge).unwrap(), "\"gauge\"");
        assert_eq!(serde_json::to_string(&Viz::Switch).unwrap(), "\"switch\"");
    }

    #[cfg(feature = "json")]
    #[test]
    fn scope_round_trips_in_both_forms() {
        assert_eq!(serde_json::to_string(&Scope::Always).unwrap(), "\"always\"");
        let links = Scope::Links(vec!["mesh".into()]);
        let json = serde_json::to_string(&links).unwrap();
        assert_eq!(json, r#"{"links":["mesh"]}"#);
        assert_eq!(serde_json::from_str::<Scope>(&json).unwrap(), links);
    }

    #[cfg(feature = "json")]
    #[test]
    fn a_presentation_round_trips_through_json() {
        let presentation = Presentation::new()
            .with_element(
                ElementSpec::new("water_turbidity", "ntu", "Turbidity", Viz::Gauge)
                    .with_band(0.0, 5.0)
                    .with_locale_label("sw", "Utiririko"),
            )
            .with_element(
                ElementSpec::new("packets_dropped", "count", "Packets dropped", Viz::Count)
                    .as_stat()
                    .on(Scope::Links(vec!["mesh".into()])),
            )
            .with_theme(Theme {
                accent: Some("#3fb1c8".into()),
                ..Theme::default()
            })
            .with_message("state.flushing", "Flushing")
            .with_message(
                "event.filter_clog",
                BTreeMap::from([
                    ("en".to_owned(), "Filter clogged".to_owned()),
                    ("sw".to_owned(), "Kichujio kimeziba".to_owned()),
                ]),
            );
        let json = serde_json::to_string(&presentation).unwrap();
        let restored: Presentation = serde_json::from_str(&json).unwrap();
        assert_eq!(presentation, restored);
    }

    #[cfg(feature = "json")]
    #[test]
    fn a_message_is_a_bare_string_or_a_locale_map_on_the_wire() {
        let presentation = Presentation::new()
            .with_message("state.flushing", "Flushing")
            .with_message(
                "event.filter_clog",
                BTreeMap::from([("en".to_owned(), "Filter clogged".to_owned())]),
            );
        let json = serde_json::to_string(&presentation.messages).unwrap();
        assert_eq!(
            json,
            r#"{"event.filter_clog":{"en":"Filter clogged"},"state.flushing":"Flushing"}"#
        );
    }
}
