//! The presentation catalog a gateway serves so the dashboard can show custom elements.
//!
//! The page ships a built-in set of sensor types it knows how to draw and offer. A
//! deployment usually measures something beyond that set, and a [`Profile`] declares
//! those extras in its [`Presentation`](pamoja_profile::Presentation). This turns those
//! declarations into the small JSON catalog served at `GET /catalog`: the page appends
//! the custom presets to its own and applies the theme, so a new sensor type needs no
//! page change.
//!
//! The catalog is presentation only - which graphic, which band, which label, and which
//! groups an element is offered on, plus where the page's network map places each group.
//! Live values still travel in the [`State`](crate::State) snapshot.

use std::collections::BTreeMap;

use serde::Serialize;

use pamoja_profile::{LocalizedText, Presentation, Profile, Scope, Theme};

/// One custom sensor or stat the page should add to its built-in catalog.
///
/// Serialized to the same shape the page's catalog uses, so a served preset merges in
/// by `id` next to the defaults.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Preset {
    id: String,
    key: String,
    unit: String,
    viz: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<BTreeMap<String, String>>,
    scope: Scope,
    #[serde(skip_serializing_if = "Option::is_none")]
    band: Option<[f32; 2]>,
    #[serde(skip_serializing_if = "is_false")]
    stat: bool,
    #[serde(skip_serializing_if = "is_false")]
    span: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<String>,
}

/// The presentation catalog served at `GET /catalog`.
///
/// Build one from the profiles a deployment runs with [`from_profiles`](Catalog::from_profiles),
/// or from presentations directly with [`from_presentations`](Catalog::from_presentations).
/// The page fetches it on boot, appends its custom presets to the built-in ones, applies
/// the theme, and places groups on its network map where
/// [`with_site_position`](Catalog::with_site_position) says. A gateway with no custom
/// elements need not serve a catalog at all; the page then keeps its defaults.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    sensor_presets: Vec<Preset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    theme: Option<Theme>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    messages: BTreeMap<String, LocalizedText>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    site_positions: BTreeMap<String, [f32; 2]>,
}

impl Catalog {
    /// Builds a catalog from the presentation of each profile a deployment runs.
    ///
    /// Every [`ElementSpec`](pamoja_profile::ElementSpec) across the profiles becomes one
    /// preset, keyed and de-duplicated by its element key (the first wins). The theme is
    /// the first one a profile declares.
    ///
    /// # Arguments
    ///
    /// * `profiles` - the profiles whose presentation to gather.
    ///
    /// # Returns
    ///
    /// A catalog carrying the custom presets and theme.
    pub fn from_profiles(profiles: &[&Profile]) -> Self {
        let presentations: Vec<&Presentation> = profiles
            .iter()
            .filter_map(|profile| profile.presentation.as_ref())
            .collect();
        Self::from_presentations(&presentations)
    }

    /// Builds a catalog from presentations, for a deployment that declares its elements
    /// without a profile.
    ///
    /// The de-duplication and theme rules are those of
    /// [`from_profiles`](Catalog::from_profiles).
    ///
    /// # Arguments
    ///
    /// * `presentations` - the presentations whose elements to gather.
    ///
    /// # Returns
    ///
    /// A catalog carrying the custom presets and theme.
    pub fn from_presentations(presentations: &[&Presentation]) -> Self {
        let mut sensor_presets: Vec<Preset> = Vec::new();
        let mut theme: Option<Theme> = None;
        let mut messages: BTreeMap<String, LocalizedText> = BTreeMap::new();
        for presentation in presentations {
            if theme.is_none() {
                theme = presentation.theme.clone();
            }
            for (key, text) in &presentation.messages {
                messages.entry(key.clone()).or_insert_with(|| text.clone());
            }
            for element in &presentation.elements {
                if sensor_presets.iter().any(|p| p.key == element.key) {
                    continue;
                }
                sensor_presets.push(Preset {
                    id: element.key.clone(),
                    key: element.key.clone(),
                    unit: element.unit.clone(),
                    viz: element.viz.kind().to_owned(),
                    label: element.label.clone(),
                    labels: element.labels.clone(),
                    scope: element.scope.clone(),
                    band: element.band,
                    stat: element.stat,
                    span: element.span,
                    value: element.value,
                    state: element.state.clone(),
                });
            }
        }
        Self {
            sensor_presets,
            theme,
            messages,
            site_positions: BTreeMap::new(),
        }
    }

    /// Places a group on the page's network map.
    ///
    /// # Arguments
    ///
    /// * `group` - the group's id, or `__gateway` for the gateway itself.
    /// * `x` - the position across the map, `0.0` at the left edge to `1.0` at the right.
    /// * `y` - the position down the map, `0.0` at the top to `1.0` at the bottom.
    ///
    /// # Returns
    ///
    /// The catalog, for chaining.
    pub fn with_site_position(mut self, group: impl Into<String>, x: f32, y: f32) -> Self {
        self.site_positions.insert(group.into(), [x, y]);
        self
    }

    /// Whether the catalog carries nothing the page does not already have.
    ///
    /// # Returns
    ///
    /// `true` when there are no custom presets, theme, messages, or map positions, so a
    /// gateway can skip serving it.
    pub fn is_empty(&self) -> bool {
        self.sensor_presets.is_empty()
            && self.theme.is_none()
            && self.messages.is_empty()
            && self.site_positions.is_empty()
    }

    /// Serializes the catalog to the JSON served at `GET /catalog`.
    ///
    /// # Returns
    ///
    /// The JSON text of the catalog.
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] if the catalog cannot be serialized, which in
    /// practice only happens on a non-finite band value.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;
    use pamoja_profile::{ElementSpec, Presentation, Viz};

    fn water_profile() -> Profile {
        Profile::well_level().with_presentation(
            Presentation::new()
                .with_element(
                    ElementSpec::new("water_turbidity", "ntu", "Turbidity", Viz::Gauge)
                        .with_band(0.0, 5.0)
                        .on(Scope::Links(vec!["mesh".into()])),
                )
                .with_element(
                    ElementSpec::new("packets_dropped", "count", "Packets dropped", Viz::Count)
                        .as_stat(),
                )
                .with_message("event.filter_clog", "Filter clogged"),
        )
    }

    #[test]
    fn from_profiles_flattens_every_element_to_a_preset() {
        let profile = water_profile();
        let catalog = Catalog::from_profiles(&[&profile]);
        assert_eq!(catalog.sensor_presets.len(), 2);
        let turbidity = &catalog.sensor_presets[0];
        assert_eq!(turbidity.viz, "radial");
        assert_eq!(turbidity.band, Some([0.0, 5.0]));
        assert!(!turbidity.stat);
    }

    #[test]
    fn duplicate_keys_across_profiles_are_kept_once() {
        let profile = water_profile();
        let catalog = Catalog::from_profiles(&[&profile, &profile]);
        assert_eq!(
            catalog.sensor_presets.len(),
            2,
            "the second copy is skipped"
        );
    }

    #[test]
    fn a_profile_without_presentation_yields_an_empty_catalog() {
        let plain = Profile::well_level();
        assert!(Catalog::from_profiles(&[&plain]).is_empty());
    }

    #[test]
    fn a_presentation_alone_and_map_positions_make_a_catalog() {
        let presentation = Presentation::new().with_element(ElementSpec::new(
            "pump_speed",
            "percent",
            "Pump speed",
            Viz::Bar,
        ));
        let catalog = Catalog::from_presentations(&[&presentation])
            .with_site_position("farm-node", 0.3, 0.7)
            .with_site_position("__gateway", 0.5, 0.5);
        assert!(!catalog.is_empty());
        let json = catalog.to_json().expect("serialize");
        assert!(
            json.contains("\"sitePositions\":{\"__gateway\":[0.5,0.5],\"farm-node\":[0.3,0.7]}")
        );
        assert!(Catalog::from_presentations(&[])
            .with_site_position("hub", 0.1, 0.1)
            .to_json()
            .expect("serialize")
            .contains("sitePositions"));
        assert!(Catalog::from_presentations(&[]).is_empty());
    }

    #[test]
    fn the_json_uses_the_page_catalog_shape() {
        let profile = water_profile();
        let json = Catalog::from_profiles(&[&profile])
            .to_json()
            .expect("serialize");
        assert!(json.contains("\"sensorPresets\""));
        assert!(json.contains("\"viz\":\"count\""));
        // A scoped element carries its links; an always element omits the form entirely.
        assert!(json.contains("\"scope\":{\"links\":[\"mesh\"]}"));
        assert!(json.contains("\"scope\":\"always\""));
        // A profile-supplied message for a custom code rides along for the page to localize.
        assert!(json.contains("\"messages\":{\"event.filter_clog\":\"Filter clogged\"}"));
    }
}
