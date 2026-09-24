//! The published JSON Schemas under `schema/`: one per file format a node reads, served on
//! the site at `/schema/` so an editor checks a manifest or a rule file as it is typed.
//! [`table`] renders a format's fields as the tables its guide shows, so the page a reader
//! checks a file against is the schema an editor checks it with.

use std::fmt;
use std::fs;
use std::path::Path;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

/// A JSON document's object keys in the order the file writes them, at every depth, so a
/// table lists a format's fields in the order its schema declares them rather than
/// alphabetically. An array's items are keyed by their index.
struct Order(Vec<(String, Order)>);

impl Order {
    /// Walks a JSON pointer down the document.
    fn at(&self, pointer: &str) -> Option<&Order> {
        pointer.split('/').skip(1).try_fold(self, |node, token| {
            node.0
                .iter()
                .find(|(key, _)| key == token)
                .map(|(_, child)| child)
        })
    }

    /// The keys of the object here, in the file's order.
    fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(key, _)| key.as_str())
    }
}

impl<'de> Deserialize<'de> for Order {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(OrderVisitor)
    }
}

struct OrderVisitor;

impl<'de> Visitor<'de> for OrderVisitor {
    type Value = Order;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Order, A::Error> {
        let mut keys = Vec::new();
        while let Some(entry) = map.next_entry::<String, Order>()? {
            keys.push(entry);
        }
        Ok(Order(keys))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Order, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element::<Order>()? {
            items.push((items.len().to_string(), item));
        }
        Ok(Order(items))
    }

    fn visit_bool<E>(self, _: bool) -> Result<Order, E> {
        Ok(Order(Vec::new()))
    }

    fn visit_i64<E>(self, _: i64) -> Result<Order, E> {
        Ok(Order(Vec::new()))
    }

    fn visit_u64<E>(self, _: u64) -> Result<Order, E> {
        Ok(Order(Vec::new()))
    }

    fn visit_f64<E>(self, _: f64) -> Result<Order, E> {
        Ok(Order(Vec::new()))
    }

    fn visit_str<E>(self, _: &str) -> Result<Order, E> {
        Ok(Order(Vec::new()))
    }

    fn visit_unit<E>(self) -> Result<Order, E> {
        Ok(Order(Vec::new()))
    }
}

/// The objects each format's tables walk, as (JSON pointer, heading, anchor).
const PROFILE: [(&str, &str, &str); 11] = [
    ("", "The profile file", "profile-fields"),
    ("/definitions/reads", "reads", "profile-reads"),
    (
        "/definitions/setpoint",
        "control, kind setpoint",
        "profile-setpoint",
    ),
    ("/definitions/level", "control, kind level", "profile-level"),
    ("/definitions/surge", "control, kind surge", "profile-surge"),
    (
        "/definitions/monitor",
        "control, kind monitor",
        "profile-monitor",
    ),
    (
        "/definitions/custom",
        "control, a kind of your own",
        "profile-custom",
    ),
    ("/definitions/power", "power", "profile-power"),
    (
        "/definitions/presentation",
        "presentation",
        "profile-presentation",
    ),
    (
        "/definitions/element",
        "presentation elements",
        "profile-element",
    ),
    ("/definitions/theme", "presentation theme", "profile-theme"),
];

const RULES: [(&str, &str, &str); 5] = [
    ("", "The rule file", "rules-fields"),
    ("/definitions/rule", "rules", "rules-rule"),
    ("/definitions/condition", "when", "rules-when"),
    (
        "/definitions/action/oneOf/0",
        "then and otherwise, drive",
        "rules-drive",
    ),
    (
        "/definitions/action/oneOf/1",
        "then and otherwise, publish",
        "rules-publish",
    ),
];

/// The objects a format's tables walk.
fn sections(format: &str) -> Result<Vec<(&'static str, &'static str, &'static str)>, String> {
    match format {
        "profile" => Ok(PROFILE.to_vec()),
        "rules" => Ok(RULES.to_vec()),
        other => Err(format!("no schema is published for `{other}`")),
    }
}

/// Reads a format's schema as text, with the path it came from for an error to name.
fn read(root: &Path, format: &str) -> Result<(String, String), String> {
    let path = root.join("schema").join(format!("{format}-1.json"));
    let text =
        fs::read_to_string(&path).map_err(|err| format!("reading {}: {err}", path.display()))?;
    Ok((text, path.display().to_string()))
}

/// Renders a format's fields as one table per object, each under a heading of its own.
///
/// # Arguments
///
/// * `root` - the repository root.
/// * `format` - `profile` or `rules`.
///
/// # Returns
///
/// The Markdown that replaces a `<!-- table: schema <format> -->` region.
///
/// # Errors
///
/// When the schema cannot be read or lacks an object the tables walk.
pub fn table(root: &Path, format: &str) -> Result<String, String> {
    let (text, path) = read(root, format)?;
    let schema: Value = serde_json::from_str(&text).map_err(|err| format!("{path}: {err}"))?;
    let order: Order = serde_json::from_str(&text).map_err(|err| format!("{path}: {err}"))?;
    let sections = sections(format)?;
    let mut out = String::new();
    for (pointer, heading, anchor) in &sections {
        let object = schema
            .pointer(pointer)
            .ok_or_else(|| format!("schema/{format}-1.json has no `{pointer}`"))?;
        out.push_str(&format!("### {heading} {{#{anchor}}}\n\n"));
        if let Some(description) = object["description"].as_str() {
            out.push_str(description);
            out.push_str("\n\n");
        }
        out.push_str("| Field | Value | Required | What it does |\n| --- | --- | --- | --- |\n");
        let required: Vec<&str> = object["required"]
            .as_array()
            .map(|names| names.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let properties = object["properties"]
            .as_object()
            .ok_or_else(|| format!("`{pointer}` in schema/{format}-1.json has no properties"))?;
        let declared = order
            .at(&format!("{pointer}/properties"))
            .ok_or_else(|| format!("`{pointer}` in schema/{format}-1.json has no properties"))?;
        for name in declared.keys() {
            let property = &properties[name];
            let needed = if required.contains(&name) {
                "yes".to_owned()
            } else {
                match &property["default"] {
                    Value::Null => "no".to_owned(),
                    default => format!("no, `{}`", unquote(default)),
                }
            };
            let described = property["description"].as_str().or_else(|| {
                property["$ref"]
                    .as_str()
                    .and_then(|target| schema.pointer(target.trim_start_matches('#')))
                    .and_then(|target| target["description"].as_str())
            });
            out.push_str(&format!(
                "| `{name}` | {} | {needed} | {} |\n",
                value_of(property, &sections),
                cell(described.unwrap_or_default())
            ));
        }
        if let Some(extra) = object
            .get("additionalProperties")
            .filter(|extra| extra.is_object())
        {
            out.push_str(&format!(
                "| any other field | {} | no | {} |\n",
                value_of(extra, &sections),
                cell(extra["description"].as_str().unwrap_or_default())
            ));
        }
        out.push('\n');
    }
    Ok(out.trim_end().to_owned())
}

/// Says in words what a field may hold, linking an object to the table that lists its
/// own fields.
fn value_of(property: &Value, sections: &[(&str, &str, &str)]) -> String {
    if let Some(target) = property["$ref"].as_str() {
        let pointer = target.trim_start_matches('#');
        return match sections.iter().find(|(at, _, _)| *at == pointer) {
            Some((_, heading, anchor)) => format!("object, see [{heading}](#{anchor})"),
            None if pointer == "/definitions/control" => {
                "object, one of the control kinds below".to_owned()
            }
            None => "object".to_owned(),
        };
    }
    if let Some(constant) = property.get("const") {
        return format!("`{}`", unquote(constant));
    }
    if let Some(choices) = property["enum"].as_array() {
        let words: Vec<String> = choices
            .iter()
            .map(|choice| format!("`{}`", unquote(choice)))
            .collect();
        return format!("one of {}", words.join(", "));
    }
    if let Some(alternatives) = property["oneOf"].as_array() {
        let words: Vec<String> = alternatives
            .iter()
            .map(|one| value_of(one, sections))
            .collect();
        return words.join(", or ");
    }
    let kind = match &property["type"] {
        Value::String(kind) => noun(kind, property, sections),
        Value::Array(kinds) => {
            let words: Vec<String> = kinds
                .iter()
                .filter_map(Value::as_str)
                .map(|kind| noun(kind, property, sections))
                .collect();
            words.join(", ")
        }
        _ => "any".to_owned(),
    };
    let mut limits = Vec::new();
    if let Some(low) = property["exclusiveMinimum"].as_f64() {
        limits.push(format!("above {low}"));
    }
    if let Some(low) = property["minimum"].as_f64() {
        limits.push(format!("at least {low}"));
    }
    if let Some(high) = property["maximum"].as_f64() {
        limits.push(format!("at most {high}"));
    }
    if limits.is_empty() {
        kind
    } else {
        format!("{kind}, {}", limits.join(", "))
    }
}

/// Names one JSON type the way the guides do.
fn noun(kind: &str, property: &Value, sections: &[(&str, &str, &str)]) -> String {
    match kind {
        "string" => "text".to_owned(),
        "number" => "number".to_owned(),
        "integer" => "whole number".to_owned(),
        "boolean" => "true or false".to_owned(),
        "array" => {
            let items = &property["items"];
            let target = items["$ref"]
                .as_str()
                .unwrap_or_default()
                .trim_start_matches('#');
            match (property["minItems"].as_u64(), property["maxItems"].as_u64()) {
                _ if target == "/definitions/action" => {
                    "list of [drive](#rules-drive) and [publish](#rules-publish) actions".to_owned()
                }
                _ if !target.is_empty() => match sections.iter().find(|(at, _, _)| *at == target) {
                    Some((_, heading, anchor)) => {
                        format!("list of objects, see [{heading}](#{anchor})")
                    }
                    None => "list of objects".to_owned(),
                },
                (Some(least), Some(most)) if least == most => {
                    format!("list of {least} {}s", value_of(items, sections))
                }
                _ if !items.is_null() => format!("list of {}", value_of(items, sections)),
                _ => "list".to_owned(),
            }
        }
        "object" if property["additionalProperties"].is_object() => "object of texts".to_owned(),
        "object" => "object".to_owned(),
        other => other.to_owned(),
    }
}

/// A JSON value as the bare word a table shows.
fn unquote(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

/// Keeps a description on one table row.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::repo_root;
    use pamoja_profile::{Profile, Rules};

    fn validator(format: &str) -> jsonschema::Validator {
        let (text, _) = read(&repo_root(), format).expect("the schema reads");
        let schema: Value = serde_json::from_str(&text).expect("the schema is JSON");
        jsonschema::draft7::new(&schema).expect("the schema is valid draft-07")
    }

    fn valid(validator: &jsonschema::Validator, text: &str) -> bool {
        let value: Value = serde_json::from_str(text).expect("the case is JSON");
        validator.is_valid(&value)
    }

    const BROODER: &str = r##"{
  "$schema": "https://pamoja.molex.cloud/schema/profile-1.json",
  "name": "brooder-heater",
  "description": "Keeps a brooder at 32 C.",
  "reads": { "quantity": "temperature", "unit": "celsius" },
  "topic": "poultry/brooder/temperature",
  "control": { "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5, "cooling": false, "safe_band": 4.0 },
  "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800, "saver_below": 0.5, "critical_below": 0.2, "hysteresis": 0.05 },
  "presentation": {
    "elements": [
      { "key": "brooder_temperature", "unit": "celsius", "label": "Brooder", "labels": { "sw": "Joto" }, "viz": "thermometer", "band": [28.0, 36.0], "stat": false, "scope": "always", "span": false },
      { "key": "heat_lamp", "unit": "state", "label": "Heat lamp", "viz": "switch", "scope": { "links": ["mesh"] }, "state": "state.lamp_off" }
    ],
    "theme": { "accent": "#c8553d" },
    "messages": { "state.lamp_off": { "en": "Off", "sw": "Imezimwa" }, "state.lamp_on": "On" }
  }
}"##;

    #[test]
    fn every_shipped_manifest_and_preset_is_valid_against_the_schema() {
        let profile = validator("profile");
        let dir = repo_root().join("profiles");
        let mut checked = 0;
        for entry in fs::read_dir(&dir).expect("the profiles directory") {
            let path = entry.expect("an entry").path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let text = fs::read_to_string(&path).expect("a readable manifest");
                assert!(
                    valid(&profile, &text),
                    "{} fails the schema",
                    path.display()
                );
                checked += 1;
            }
        }
        assert!(checked >= 8, "only {checked} manifests were checked");
        for preset in [
            Profile::vaccine_fridge_monitor(),
            Profile::irrigation_node(),
            Profile::well_level(),
            Profile::flood_sensor(),
        ] {
            assert!(
                valid(&profile, &preset.to_json().unwrap()),
                "{}",
                preset.name
            );
        }
    }

    #[test]
    fn the_profile_schema_and_the_parser_accept_and_refuse_the_same_manifests() {
        let profile = validator("profile");
        let cases: &[(&str, &str, bool)] = &[
            ("", "", true),
            ("\"topic\"", "\"topik\"", false),
            ("\"saver_secs\"", "\"saver_sec\"", false),
            ("\"safe_band\": 4.0", "\"safe_band\": 4.0, \"deadband\": 1.0", false),
            (
                "{ \"kind\": \"setpoint\", \"setpoint\": 32.0, \"hysteresis\": 0.5, \"cooling\": false, \"safe_band\": 4.0 }",
                "{ \"kind\": \"frost_guard\", \"warn_below\": 2.0, \"latching\": true, \"zone\": \"north\" }",
                true,
            ),
            (
                "{ \"kind\": \"setpoint\", \"setpoint\": 32.0, \"hysteresis\": 0.5, \"cooling\": false, \"safe_band\": 4.0 }",
                "{ \"kind\": \"\", \"warn_below\": 2.0 }",
                false,
            ),
            (
                "{ \"kind\": \"setpoint\", \"setpoint\": 32.0, \"hysteresis\": 0.5, \"cooling\": false, \"safe_band\": 4.0 }",
                "{ \"kind\": \"frost_guard\", \"zone\": null }",
                false,
            ),
            (
                "{ \"kind\": \"setpoint\", \"setpoint\": 32.0, \"hysteresis\": 0.5, \"cooling\": false, \"safe_band\": 4.0 }",
                "{ \"kind\": \"level\", \"empty\": 0.5, \"warn_within\": 0 }",
                false,
            ),
            (
                "{ \"kind\": \"setpoint\", \"setpoint\": 32.0, \"hysteresis\": 0.5, \"cooling\": false, \"safe_band\": 4.0 }",
                "{ \"kind\": \"surge\", \"rising\": true, \"limit\": 0.3 }",
                true,
            ),
            (
                "{ \"kind\": \"setpoint\", \"setpoint\": 32.0, \"hysteresis\": 0.5, \"cooling\": false, \"safe_band\": 4.0 }",
                "{ \"kind\": \"monitor\" }",
                true,
            ),
            ("\"hysteresis\": 0.5,", "\"hysteresis\": 0.0,", false),
            ("\"thermometer\"", "\"hologram\"", false),
            ("\"unit\": \"celsius\" }", "\"units\": \"celsius\" }", false),
            ("\"quantity\": \"temperature\"", "\"quantity\": \"Temperature\"", false),
            ("\"active_secs\": 120", "\"active_secs\": 0", false),
            ("\"saver_below\": 0.5", "\"saver_below\": 1.5", false),
            ("\"poultry/brooder/temperature\"", "\"poultry/+/temperature\"", false),
            ("[28.0, 36.0]", "[28.0, 32.0, 36.0]", false),
            ("\"state.lamp_off\" }", "\"closed\" }", false),
            ("\"state.lamp_on\": \"On\"", "\"lamp_on\": \"On\"", false),
            ("{ \"en\": \"Off\", \"sw\": \"Imezimwa\" }", "{ \"sw\": \"Imezimwa\" }", false),
            ("\"state\": \"state.lamp_off\"", "\"value\": 1.0, \"state\": \"state.lamp_off\"", false),
            ("\"accent\"", "\"acent\"", false),
            ("profile-1.json", "profile-2.json", false),
            ("https://pamoja.molex.cloud/schema/profile-1.json", "./profile-1.json", true),
            (
                "\"$schema\": \"https://pamoja.molex.cloud/schema/profile-1.json\",",
                "",
                true,
            ),
        ];
        for (from, to, accepted) in cases {
            let text = if from.is_empty() {
                BROODER.to_owned()
            } else {
                assert!(BROODER.contains(from), "{from} is not in the base manifest");
                BROODER.replacen(from, to, 1)
            };
            let parsed = Profile::from_json(&text);
            assert_eq!(
                parsed.is_ok(),
                *accepted,
                "the parser on {from} -> {to}: {parsed:?}"
            );
            assert_eq!(
                valid(&profile, &text),
                *accepted,
                "the schema on {from} -> {to}"
            );
        }
    }

    #[test]
    fn the_parser_refuses_what_a_schema_cannot_say() {
        let profile = validator("profile");
        for (from, to) in [
            ("\"safe_band\": 4.0", "\"safe_band\": 0.25"),
            ("\"saver_secs\": 600", "\"saver_secs\": 60"),
            ("\"critical_below\": 0.2", "\"critical_below\": 0.6"),
            ("[28.0, 36.0]", "[36.0, 28.0]"),
            ("\"key\": \"heat_lamp\"", "\"key\": \"brooder_temperature\""),
        ] {
            let text = BROODER.replacen(from, to, 1);
            assert!(valid(&profile, &text), "the schema allows {to}");
            assert!(
                Profile::from_json(&text).is_err(),
                "the parser refuses {to}"
            );
        }
    }

    const GARDEN: &str = r#"{
  "$schema": "https://pamoja.molex.cloud/schema/rules-1.json",
  "rules": [
    { "name": "water-when-dry",
      "when": { "topic": "garden/bed-1/moisture", "below": 30.0, "hysteresis": 5.0 },
      "then": [ { "drive": "bed-valve", "on": true }, { "publish": "garden/bed-1/valve", "payload": "open" } ],
      "otherwise": [ { "drive": "bed-valve", "on": false } ] },
    { "name": "flood-alarm",
      "when": { "topic": "garden/bed-1/moisture", "above": 60.0 },
      "then": [ { "publish": "garden/alarm", "payload": "waterlogged" } ] }
  ]
}"#;

    #[test]
    fn the_rules_schema_and_the_parser_accept_and_refuse_the_same_files() {
        let rules = validator("rules");
        let cases: &[(&str, &str, bool)] = &[
            ("", "", true),
            ("\"below\": 30.0", "\"above\": 30.0", true),
            ("\"below\": 30.0", "\"below\": 30.0, \"above\": 40.0", false),
            ("\"below\": 30.0, ", "", false),
            (
                "\"below\": 30.0",
                "\"compare\": \"below\", \"threshold\": 30.0",
                false,
            ),
            ("\"hysteresis\": 5.0", "\"hysterisis\": 5.0", false),
            ("\"hysteresis\": 5.0", "\"hysteresis\": -1.0", false),
            (
                "{ \"drive\": \"bed-valve\", \"on\": true }",
                "{ \"drive\": \"bed-valve\" }",
                false,
            ),
            (
                "{ \"publish\": \"garden/alarm\", \"payload\": \"waterlogged\" }",
                "{ \"publish\": \"garden/alarm\" }",
                false,
            ),
            (
                "{ \"drive\": \"bed-valve\", \"on\": false }",
                "{ \"do\": \"drive\", \"actuator\": \"bed-valve\", \"on\": false }",
                false,
            ),
            ("\"garden/alarm\"", "\"garden/#\"", false),
            (
                "\"garden/bed-1/moisture\", \"above\"",
                "\"garden/+/moisture\", \"above\"",
                false,
            ),
            (
                "\"then\": [ { \"publish\": \"garden/alarm\", \"payload\": \"waterlogged\" } ]",
                "\"then\": []",
                false,
            ),
            ("rules-1.json", "rules-2.json", false),
        ];
        for (from, to, accepted) in cases {
            let text = if from.is_empty() {
                GARDEN.to_owned()
            } else {
                assert!(GARDEN.contains(from), "{from} is not in the base file");
                GARDEN.replacen(from, to, 1)
            };
            let parsed = Rules::from_json(&text);
            assert_eq!(
                parsed.is_ok(),
                *accepted,
                "the parser on {from} -> {to}: {parsed:?}"
            );
            assert_eq!(
                valid(&rules, &text),
                *accepted,
                "the schema on {from} -> {to}"
            );
        }
        assert!(Rules::from_json(&Rules::from_json(GARDEN).unwrap().to_json().unwrap()).is_ok());
        assert!(valid(
            &rules,
            &Rules::from_json(GARDEN).unwrap().to_json().unwrap()
        ));
    }

    #[test]
    fn every_field_is_described_and_every_table_renders() {
        for format in ["profile", "rules"] {
            let rendered = table(&repo_root(), format).expect("the tables render");
            for (_, heading, anchor) in sections(format).unwrap() {
                assert!(
                    rendered.contains(&format!("### {heading} {{#{anchor}}}")),
                    "{format}: {heading}"
                );
            }
            assert!(
                !rendered.contains("|  |"),
                "{format} has a field with no description:\n{rendered}"
            );
        }
        let profile = table(&repo_root(), "profile").unwrap();
        assert!(
            profile.contains("| `reads` | object, see [reads](#profile-reads) | no |"),
            "{profile}"
        );
        assert!(
            profile.contains("| `hysteresis` | number, above 0 | yes |"),
            "{profile}"
        );
        assert!(
            profile.contains("| `saver_below` | number, above 0, at most 1 | no, `0.5` |"),
            "{profile}"
        );
        assert!(
            profile.contains("| `band` | list of 2 numbers | no |"),
            "{profile}"
        );
        assert!(
            profile.contains("| any other field | number, true or false, text | no |"),
            "{profile}"
        );
        let rules = table(&repo_root(), "rules").unwrap();
        assert!(rules.contains("| `drive` | text | yes |"), "{rules}");
        assert!(rules.contains("| `then` | list of [drive](#rules-drive) and [publish](#rules-publish) actions | no |"), "{rules}");
        assert!(
            rules.contains("| `rules` | list of objects, see [rules](#rules-rule) | yes |"),
            "{rules}"
        );
    }
}
