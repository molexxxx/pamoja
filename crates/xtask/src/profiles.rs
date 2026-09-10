//! The shared profiles in `profiles/`: one JSON manifest per profile, read by the same
//! parser a device uses, checked for the mistakes a hand-written manifest makes, and kept
//! in the canonical form `Profile::to_json` writes so a diff shows a change of meaning and
//! nothing else. [`Profiles::table`] renders the catalog page, and `cargo xtask profiles`
//! rewrites every manifest into canonical form (`--check` reports one that is not).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use pamoja_profile::{ControlSpec, ElementSpec, LocalizedText, Presentation, Profile};

use crate::catalog::{command, escape};
use crate::docs::repo_root;

/// The repository, for the link to each manifest.
const REPO: &str = "https://github.com/molexxxx/pamoja";

/// Where a manifest downloads from as a bare file.
const RAW: &str = "https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles";

/// The directory the manifests live in, from the repository root.
const DIR: &str = "profiles";

/// One shared manifest: where it lives, what it holds, and the text it was read from.
pub struct Manifest {
    /// The file name without `.json`, which the profile's name must equal.
    pub stem: String,
    /// The text as committed.
    pub text: String,
    /// The profile the parser made of it.
    pub profile: Profile,
}

impl Manifest {
    /// Parses one manifest's text.
    ///
    /// # Arguments
    ///
    /// * `stem` - the file name without `.json`.
    /// * `text` - the file's contents.
    ///
    /// # Returns
    ///
    /// The manifest.
    ///
    /// # Errors
    ///
    /// When the text is not a profile, with the parser's reason.
    pub fn parse(stem: &str, text: &str) -> Result<Manifest, String> {
        let profile = Profile::from_json(text)
            .map_err(|err| format!("{DIR}/{stem}.json does not parse as a profile: {err}"))?;
        Ok(Manifest {
            stem: stem.to_owned(),
            text: text.to_owned(),
            profile,
        })
    }

    /// The text this manifest takes once rewritten by the profile's own serializer.
    ///
    /// # Returns
    ///
    /// The canonical text, ending in a newline.
    ///
    /// # Errors
    ///
    /// When the profile cannot be serialized.
    pub fn canonical(&self) -> Result<String, String> {
        self.profile
            .to_json()
            .map(|json| json + "\n")
            .map_err(|err| format!("{DIR}/{}.json cannot be serialized: {err}", self.stem))
    }
}

/// Every manifest under `profiles/`, in file-name order.
pub struct Profiles {
    /// The manifests.
    pub manifests: Vec<Manifest>,
}

impl Profiles {
    /// Reads every `profiles/*.json` under the repository root.
    ///
    /// # Arguments
    ///
    /// * `root` - the repository root.
    ///
    /// # Returns
    ///
    /// The manifests, sorted by file name.
    ///
    /// # Errors
    ///
    /// When the directory cannot be read or a file does not parse.
    pub fn load(root: &Path) -> Result<Profiles, String> {
        let dir = root.join(DIR);
        let mut paths: Vec<_> = fs::read_dir(&dir)
            .map_err(|err| format!("reading {}: {err}", dir.display()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .collect();
        paths.sort();
        let mut manifests = Vec::new();
        for path in paths {
            let stem = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            let text = fs::read_to_string(&path)
                .map_err(|err| format!("reading {}: {err}", path.display()))?;
            manifests.push(Manifest::parse(&stem, &text)?);
        }
        Ok(Profiles { manifests })
    }

    /// Checks every manifest for what the parser accepts but a device or the dashboard
    /// would not make sense of.
    ///
    /// # Arguments
    ///
    /// * `root` - the repository root, for the dashboard's shipped state codes.
    ///
    /// # Returns
    ///
    /// Nothing when every manifest passes.
    ///
    /// # Errors
    ///
    /// The first problem found, naming the file.
    pub fn check(&self, root: &Path) -> Result<(), String> {
        let shipped = shipped_codes(root)?;
        let mut names = BTreeSet::new();
        for manifest in &self.manifests {
            let at = format!("{DIR}/{}.json", manifest.stem);
            let profile = &manifest.profile;
            if profile.name != manifest.stem {
                return Err(format!(
                    "{at}: the profile is named `{}`, but the file must carry the profile's name",
                    profile.name
                ));
            }
            if !is_kebab_case(&profile.name) {
                return Err(format!(
                    "{at}: the name must be lowercase words joined by single hyphens"
                ));
            }
            if !names.insert(profile.name.as_str()) {
                return Err(format!("{at}: another manifest carries this name"));
            }
            match profile.description.as_deref().map(str::trim) {
                None | Some("") => {
                    return Err(format!(
                        "{at}: a shared profile needs a `description` saying what it is for"
                    ))
                }
                Some(text) if !text.ends_with('.') => {
                    return Err(format!(
                        "{at}: the description is a sentence or two, ending in a period"
                    ))
                }
                Some(_) => {}
            }
            check_topic(&at, &profile.topic)?;
            check_control(&at, &profile.control)?;
            check_power(&at, profile)?;
            if let Some(presentation) = &profile.presentation {
                check_presentation(&at, presentation, &shipped)?;
            }
        }
        Ok(())
    }

    /// Renders the catalog: one card per manifest, with what it does in words and the line
    /// that downloads it.
    ///
    /// # Returns
    ///
    /// The Markdown that replaces the `<!-- table: profiles -->` region.
    pub fn table(&self) -> String {
        let mut out = String::from("<div class=\"pkgs\">\n");
        for manifest in &self.manifests {
            out.push_str(&card(manifest));
        }
        out.push_str("</div>");
        out
    }
}

/// Rewrites every manifest into canonical form, or with `--check` reports the ones that
/// are not.
///
/// # Arguments
///
/// * `args` - the task's arguments.
///
/// # Returns
///
/// Success when every manifest is valid and canonical (or was just made so).
pub fn run(args: &[String]) -> ExitCode {
    let check = args.iter().any(|arg| arg == "--check");
    let root = repo_root();
    let profiles = match Profiles::load(&root).and_then(|profiles| {
        profiles.check(&root)?;
        Ok(profiles)
    }) {
        Ok(profiles) => profiles,
        Err(message) => {
            eprintln!("xtask profiles: {message}");
            return ExitCode::FAILURE;
        }
    };
    let mut stale = Vec::new();
    for manifest in &profiles.manifests {
        let canonical = match manifest.canonical() {
            Ok(canonical) => canonical,
            Err(message) => {
                eprintln!("xtask profiles: {message}");
                return ExitCode::FAILURE;
            }
        };
        if canonical != manifest.text {
            stale.push((manifest.stem.clone(), canonical));
        }
    }
    if check {
        for (stem, _) in &stale {
            eprintln!("xtask profiles: {DIR}/{stem}.json is not in canonical form; run `cargo xtask profiles`");
        }
        if stale.is_empty() {
            println!(
                "profiles: {} manifests valid and canonical",
                profiles.manifests.len()
            );
            return ExitCode::SUCCESS;
        }
        return ExitCode::FAILURE;
    }
    for (stem, canonical) in &stale {
        let path = root.join(DIR).join(format!("{stem}.json"));
        if let Err(err) = fs::write(&path, canonical) {
            eprintln!("xtask profiles: writing {}: {err}", path.display());
            return ExitCode::FAILURE;
        }
    }
    println!(
        "profiles: {} manifests valid, {} rewritten into canonical form",
        profiles.manifests.len(),
        stale.len()
    );
    ExitCode::SUCCESS
}

// The discrete state codes the dashboard already has words for, from its English bundle;
// a manifest that uses any other code must supply the words itself.
fn shipped_codes(root: &Path) -> Result<BTreeSet<String>, String> {
    let path = root.join("crates/pamoja-dashboard/web/app/i18n/en.json");
    let text =
        fs::read_to_string(&path).map_err(|err| format!("reading {}: {err}", path.display()))?;
    let bundle: serde_json::Value =
        serde_json::from_str(&text).map_err(|err| format!("parsing {}: {err}", path.display()))?;
    Ok(bundle
        .get("messages")
        .and_then(serde_json::Value::as_object)
        .map(|messages| messages.keys().cloned().collect())
        .unwrap_or_default())
}

fn is_kebab_case(name: &str) -> bool {
    !name.is_empty()
        && name.split('-').all(|word| {
            !word.is_empty()
                && word
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn is_snake_case(key: &str) -> bool {
    key.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn check_topic(at: &str, topic: &str) -> Result<(), String> {
    if topic.is_empty() {
        return Err(format!("{at}: the topic is empty"));
    }
    if topic.chars().any(char::is_whitespace) {
        return Err(format!("{at}: the topic `{topic}` contains whitespace"));
    }
    if topic.contains(['+', '#']) {
        return Err(format!(
            "{at}: the topic `{topic}` is a filter; a profile publishes to one topic"
        ));
    }
    if topic.starts_with('/') || topic.ends_with('/') || topic.contains("//") {
        return Err(format!(
            "{at}: the topic `{topic}` has an empty segment; separate non-empty words with single slashes"
        ));
    }
    Ok(())
}

fn check_control(at: &str, control: &ControlSpec) -> Result<(), String> {
    let finite = |name: &str, value: f32| {
        if value.is_finite() {
            Ok(())
        } else {
            Err(format!("{at}: `{name}` must be a finite number"))
        }
    };
    match *control {
        ControlSpec::Setpoint {
            setpoint,
            hysteresis,
            safe_band,
            ..
        } => {
            finite("setpoint", setpoint)?;
            finite("hysteresis", hysteresis)?;
            finite("safe_band", safe_band)?;
            if hysteresis <= 0.0 {
                return Err(format!(
                    "{at}: `hysteresis` must be above zero, or the output chatters at the setpoint"
                ));
            }
            if safe_band < hysteresis {
                return Err(format!(
                    "{at}: `safe_band` ({safe_band}) is narrower than `hysteresis` ({hysteresis}), so an alert would fire inside the deadband"
                ));
            }
        }
        ControlSpec::Level { empty, warn_within } => {
            finite("empty", empty)?;
            if warn_within == 0 {
                return Err(format!(
                    "{at}: `warn_within` must be at least one sample, or the warning never comes"
                ));
            }
        }
        ControlSpec::Surge { limit, .. } => {
            finite("limit", limit)?;
            if limit <= 0.0 {
                return Err(format!(
                    "{at}: `limit` must be above zero, or every sample is a surge"
                ));
            }
        }
        ControlSpec::Monitor => {}
    }
    Ok(())
}

fn check_power(at: &str, profile: &Profile) -> Result<(), String> {
    let power = &profile.power;
    if power.active_secs == 0 {
        return Err(format!("{at}: `active_secs` must be at least one second"));
    }
    if !(power.active_secs <= power.saver_secs && power.saver_secs <= power.critical_secs) {
        return Err(format!(
            "{at}: the intervals must not shorten as the battery drains: active {} s, saver {} s, critical {} s",
            power.active_secs, power.saver_secs, power.critical_secs
        ));
    }
    if !(power.saver_below > 0.0 && power.saver_below <= 1.0) {
        return Err(format!(
            "{at}: `saver_below` is a state of charge between 0 and 1"
        ));
    }
    if !(power.critical_below > 0.0 && power.critical_below < power.saver_below) {
        return Err(format!(
            "{at}: `critical_below` must sit between 0 and `saver_below`"
        ));
    }
    Ok(())
}

fn check_presentation(
    at: &str,
    presentation: &Presentation,
    shipped: &BTreeSet<String>,
) -> Result<(), String> {
    let mut keys = BTreeSet::new();
    for element in &presentation.elements {
        check_element(at, element, shipped, &presentation.messages)?;
        if !keys.insert(element.key.as_str()) {
            return Err(format!(
                "{at}: the element `{}` is declared twice",
                element.key
            ));
        }
    }
    for (code, text) in &presentation.messages {
        if !(code.starts_with("state.") || code.starts_with("event.")) {
            return Err(format!(
                "{at}: the message `{code}` is neither a `state.` nor an `event.` code"
            ));
        }
        match text {
            LocalizedText::Plain(text) if text.trim().is_empty() => {
                return Err(format!("{at}: the message `{code}` is empty"));
            }
            LocalizedText::PerLocale(map) if !map.contains_key("en") => {
                return Err(format!(
                    "{at}: the message `{code}` gives no `en` text, which every other locale falls back to"
                ));
            }
            _ => {}
        }
    }
    if let Some(theme) = &presentation.theme {
        for (name, color) in [
            ("accent", &theme.accent),
            ("ok", &theme.ok),
            ("warn", &theme.warn),
            ("alarm", &theme.alarm),
            ("track", &theme.track),
        ] {
            if color
                .as_deref()
                .is_some_and(|color| color.trim().is_empty())
            {
                return Err(format!("{at}: the theme's `{name}` color is empty"));
            }
        }
    }
    Ok(())
}

fn check_element(
    at: &str,
    element: &ElementSpec,
    shipped: &BTreeSet<String>,
    messages: &std::collections::BTreeMap<String, LocalizedText>,
) -> Result<(), String> {
    let key = &element.key;
    if !is_snake_case(key) {
        return Err(format!(
            "{at}: the element key `{key}` must be lowercase words joined by underscores"
        ));
    }
    if element.unit.trim().is_empty() {
        return Err(format!("{at}: the element `{key}` has no unit"));
    }
    if element.label.trim().is_empty() {
        return Err(format!("{at}: the element `{key}` has no label"));
    }
    if let Some([low, high]) = element.band {
        if !(low.is_finite() && high.is_finite() && low < high) {
            return Err(format!(
                "{at}: the element `{key}` has a band of {low} to {high}; the low end comes first"
            ));
        }
    }
    if element.value.is_some_and(|value| !value.is_finite()) {
        return Err(format!(
            "{at}: the element `{key}` has a starting value that is not a finite number"
        ));
    }
    if let Some(state) = &element.state {
        if element.value.is_some() {
            return Err(format!(
                "{at}: the element `{key}` starts with both a value and a state; a reading is one or the other"
            ));
        }
        if !state.starts_with("state.") {
            return Err(format!(
                "{at}: the element `{key}` starts in `{state}`, which is not a `state.` code"
            ));
        }
        if !shipped.contains(state) && !messages.contains_key(state) {
            return Err(format!(
                "{at}: the element `{key}` starts in `{state}`, a code the dashboard has no words for; add it under `messages`"
            ));
        }
    }
    Ok(())
}

fn card(manifest: &Manifest) -> String {
    let profile = &manifest.profile;
    let name = &manifest.stem;
    let facts: String = facts(profile)
        .iter()
        .map(|fact| format!("<li>{fact}</li>"))
        .collect();
    format!(
        "<div class=\"pkg stack\" id=\"profile-{name}\">\n<div class=\"pkg-head\">\n<div class=\"pkg-what\"><a class=\"pkg-title\" href=\"{REPO}/blob/main/{DIR}/{name}.json\">{name}</a><code class=\"pkg-import\">{DIR}/{name}.json</code><p>{}</p><ul class=\"pkg-proves\">{facts}</ul></div>\n{}\n</div>\n</div>\n",
        escape(profile.description.as_deref().unwrap_or_default()),
        command(&format!("curl -O {RAW}/{name}.json")),
    )
}

// What the manifest does, in the words the guides use, so a reader chooses without opening
// the file.
fn facts(profile: &Profile) -> Vec<String> {
    let mut out = vec![format!(
        "Publishes on <code>{}</code>.",
        escape(&profile.topic)
    )];
    out.push(policy_sentence(&profile.control));
    let power = &profile.power;
    out.push(format!(
        "Samples every {}, every {} below {} charge, and every {} below {}.",
        period(power.active_secs),
        period(power.saver_secs),
        percent(power.saver_below),
        period(power.critical_secs),
        percent(power.critical_below)
    ));
    if let Some(presentation) = &profile.presentation {
        if let Some(sentence) = presentation_sentence(presentation) {
            out.push(sentence);
        }
    }
    out
}

fn policy_sentence(control: &ControlSpec) -> String {
    match *control {
        ControlSpec::Setpoint {
            setpoint,
            hysteresis,
            cooling,
            safe_band,
        } => {
            let (on, off) = if cooling {
                (setpoint + hysteresis, setpoint - hysteresis)
            } else {
                (setpoint - hysteresis, setpoint + hysteresis)
            };
            format!(
                "Holds {} by switching the output on {} {} and off {} {}, and alerts outside {} to {}.",
                number(setpoint),
                if cooling { "above" } else { "below" },
                number(on),
                if cooling { "below" } else { "above" },
                number(off),
                number(setpoint - safe_band),
                number(setpoint + safe_band)
            )
        }
        ControlSpec::Level { empty, warn_within } => {
            format!(
            "Watches a falling level and warns once it is on course to reach {} within {} more {}.",
            number(empty),
            warn_within,
            if warn_within == 1 { "sample" } else { "samples" }
        )
        }
        ControlSpec::Surge { rising, limit } => format!(
            "Warns when the reading {} by more than {} in one sample.",
            if rising { "rises" } else { "falls" },
            number(limit)
        ),
        ControlSpec::Monitor => "Reports readings and drives nothing.".to_owned(),
    }
}

fn presentation_sentence(presentation: &Presentation) -> Option<String> {
    if presentation.elements.is_empty() {
        return None;
    }
    let elements: Vec<String> = presentation
        .elements
        .iter()
        .map(|element| {
            let mut text = format!(
                "{} as a {} in {}",
                escape(&element.label),
                viz_name(element),
                escape(&element.unit)
            );
            if let Some([low, high]) = element.band {
                text.push_str(&format!(
                    " with a safe band of {} to {}",
                    number(low),
                    number(high)
                ));
            }
            if element.stat {
                text.push_str(", a node stat");
            }
            text
        })
        .collect();
    Some(format!("Draws {}.", elements.join("; ")))
}

// The graphic's manifest name, from the same serializer the manifest uses.
fn viz_name(element: &ElementSpec) -> String {
    serde_json::to_value(element.viz)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn number(value: f32) -> String {
    format!("{value}")
}

fn percent(fraction: f32) -> String {
    let value = (f64::from(fraction) * 10_000.0).round() / 100.0;
    format!("{value} %")
}

fn period(secs: u64) -> String {
    if secs.is_multiple_of(3600) {
        let hours = secs / 3600;
        format!("{hours} {}", if hours == 1 { "hour" } else { "hours" })
    } else if secs.is_multiple_of(60) {
        format!("{} min", secs / 60)
    } else {
        format!("{secs} s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"{
  "name": "tank-level",
  "description": "Warns before a rain tank runs dry.",
  "topic": "water/tank/level",
  "control": { "kind": "level", "empty": 0.0, "warn_within": 4 },
  "power": { "active_secs": 600, "saver_secs": 1800, "critical_secs": 3600 }
}"#;

    fn manifest(stem: &str, text: &str) -> Profiles {
        Profiles {
            manifests: vec![Manifest::parse(stem, text).expect("parses")],
        }
    }

    fn error_of(stem: &str, text: &str) -> String {
        manifest(stem, text)
            .check(&repo_root())
            .expect_err("the manifest is rejected")
    }

    #[test]
    fn the_shipped_manifests_are_valid_and_canonical() {
        let root = repo_root();
        let profiles = Profiles::load(&root).expect("the manifests load");
        assert!(profiles.manifests.len() >= 4);
        profiles
            .check(&root)
            .expect("every shipped manifest passes");
        for manifest in &profiles.manifests {
            assert_eq!(
                manifest.canonical().unwrap(),
                manifest.text,
                "profiles/{}.json is not in canonical form; run cargo xtask profiles",
                manifest.stem
            );
        }
    }

    #[test]
    fn the_presets_and_their_manifests_cannot_drift() {
        let root = repo_root();
        for preset in [
            Profile::vaccine_fridge_monitor(),
            Profile::irrigation_node(),
            Profile::well_level(),
            Profile::flood_sensor(),
        ] {
            let path = root.join(DIR).join(format!("{}.json", preset.name));
            let text = fs::read_to_string(&path).expect("the preset ships as a manifest");
            assert_eq!(text, preset.to_json().unwrap() + "\n", "{}", path.display());
        }
    }

    #[test]
    fn a_valid_manifest_passes_and_renders() {
        let profiles = manifest("tank-level", MINIMAL);
        profiles.check(&repo_root()).expect("valid");
        let rendered = profiles.table();
        assert!(
            rendered.contains("<div class=\"pkg stack\" id=\"profile-tank-level\">"),
            "{rendered}"
        );
        assert!(
            rendered.contains("<p>Warns before a rain tank runs dry.</p>"),
            "{rendered}"
        );
        assert!(
            rendered.contains("<li>Publishes on <code>water/tank/level</code>.</li>"),
            "{rendered}"
        );
        assert!(rendered.contains("<li>Watches a falling level and warns once it is on course to reach 0 within 4 more samples.</li>"), "{rendered}");
        assert!(rendered.contains("<li>Samples every 10 min, every 30 min below 50 % charge, and every 1 hour below 20 %.</li>"), "{rendered}");
        assert!(rendered.contains("curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/tank-level.json"), "{rendered}");
    }

    #[test]
    fn the_policy_reads_as_a_sentence() {
        let fridge = policy_sentence(&Profile::vaccine_fridge_monitor().control);
        assert_eq!(fridge, "Holds 5 by switching the output on above 5.5 and off below 4.5, and alerts outside 2 to 8.");
        let drip = policy_sentence(&Profile::irrigation_node().control);
        assert_eq!(drip, "Holds 35 by switching the output on below 30 and off above 40, and alerts outside 10 to 60.");
        assert_eq!(
            policy_sentence(&Profile::flood_sensor().control),
            "Warns when the reading rises by more than 0.3 in one sample."
        );
        assert_eq!(
            policy_sentence(&ControlSpec::Monitor),
            "Reports readings and drives nothing."
        );
    }

    #[test]
    fn a_presentation_is_described_from_its_elements() {
        let text = r#"{
  "name": "soil-bed",
  "description": "Waters a raised bed.",
  "topic": "garden/bed/moisture",
  "control": { "kind": "monitor" },
  "power": { "active_secs": 60, "saver_secs": 300, "critical_secs": 900 },
  "presentation": {
    "elements": [
      { "key": "soil_moisture", "unit": "percent", "label": "Soil moisture", "viz": "droplet", "band": [25.0, 60.0] },
      { "key": "valve", "unit": "state", "label": "Valve", "viz": "valve", "state": "state.closed" },
      { "key": "packets_dropped", "unit": "count", "label": "Packets dropped", "viz": "count", "stat": true }
    ],
    "messages": { "event.dry": { "en": "Bed dry", "sw": "Kitalu kimekauka" } }
  }
}"#;
        let profiles = manifest("soil-bed", text);
        profiles.check(&repo_root()).expect("valid");
        let rendered = profiles.table();
        assert!(rendered.contains("<li>Draws Soil moisture as a droplet in percent with a safe band of 25 to 60; Valve as a valve in state; Packets dropped as a count in count, a node stat.</li>"), "{rendered}");
    }

    #[test]
    fn the_file_name_and_the_profile_name_agree() {
        assert!(error_of("tank", MINIMAL).contains("named `tank-level`"));
        let renamed = MINIMAL.replace("tank-level", "Tank_Level");
        assert!(error_of("Tank_Level", &renamed).contains("lowercase words"));
    }

    #[test]
    fn two_manifests_cannot_share_a_name() {
        let profiles = Profiles {
            manifests: vec![
                Manifest::parse("tank-level", MINIMAL).unwrap(),
                Manifest::parse("tank-level", MINIMAL).unwrap(),
            ],
        };
        assert!(profiles
            .check(&repo_root())
            .unwrap_err()
            .contains("another manifest"));
    }

    #[test]
    fn a_shared_profile_explains_itself() {
        let silent = MINIMAL.replace(
            "  \"description\": \"Warns before a rain tank runs dry.\",\n",
            "",
        );
        assert!(error_of("tank-level", &silent).contains("needs a `description`"));
        let fragment = MINIMAL.replace("runs dry.", "runs dry");
        assert!(error_of("tank-level", &fragment).contains("ending in a period"));
    }

    #[test]
    fn a_topic_is_one_publishable_path() {
        for (bad, reason) in [
            ("water/#", "is a filter"),
            ("water/+/level", "is a filter"),
            ("/water/tank", "empty segment"),
            ("water//tank", "empty segment"),
            ("water tank", "whitespace"),
        ] {
            let text = MINIMAL.replace("water/tank/level", bad);
            assert!(error_of("tank-level", &text).contains(reason), "{bad}");
        }
    }

    #[test]
    fn a_policy_must_be_able_to_act() {
        let chatter = MINIMAL.replace(
            r#"{ "kind": "level", "empty": 0.0, "warn_within": 4 }"#,
            r#"{ "kind": "setpoint", "setpoint": 5.0, "hysteresis": 0.0, "cooling": true, "safe_band": 3.0 }"#,
        );
        assert!(error_of("tank-level", &chatter).contains("chatters"));
        let inside = MINIMAL.replace(
            r#"{ "kind": "level", "empty": 0.0, "warn_within": 4 }"#,
            r#"{ "kind": "setpoint", "setpoint": 5.0, "hysteresis": 2.0, "cooling": true, "safe_band": 1.0 }"#,
        );
        assert!(error_of("tank-level", &inside).contains("inside the deadband"));
        let never = MINIMAL.replace("\"warn_within\": 4", "\"warn_within\": 0");
        assert!(error_of("tank-level", &never).contains("never comes"));
        let always = MINIMAL.replace(
            r#"{ "kind": "level", "empty": 0.0, "warn_within": 4 }"#,
            r#"{ "kind": "surge", "rising": true, "limit": 0.0 }"#,
        );
        assert!(error_of("tank-level", &always).contains("every sample is a surge"));
    }

    #[test]
    fn the_schedule_slows_as_the_battery_drains() {
        let faster = MINIMAL.replace("\"saver_secs\": 1800", "\"saver_secs\": 30");
        assert!(error_of("tank-level", &faster).contains("must not shorten"));
        let thresholds = MINIMAL.replace(
            "\"critical_secs\": 3600 }",
            "\"critical_secs\": 3600, \"saver_below\": 0.2, \"critical_below\": 0.5 }",
        );
        assert!(error_of("tank-level", &thresholds).contains("`critical_below`"));
    }

    #[test]
    fn a_presentation_names_what_the_dashboard_can_draw() {
        let with = |elements: &str, messages: &str| {
            MINIMAL.replace(
                "\"critical_secs\": 3600 }\n}",
                &format!("\"critical_secs\": 3600 }},\n  \"presentation\": {{ \"elements\": [{elements}]{messages} }}\n}}"),
            )
        };
        let key = with(
            r#"{ "key": "Turbidity", "unit": "ntu", "label": "Turbidity", "viz": "gauge" }"#,
            "",
        );
        assert!(error_of("tank-level", &key).contains("underscores"));
        let twice = with(
            r#"{ "key": "turbidity", "unit": "ntu", "label": "Turbidity", "viz": "gauge" }, { "key": "turbidity", "unit": "ntu", "label": "Again", "viz": "bar" }"#,
            "",
        );
        assert!(error_of("tank-level", &twice).contains("declared twice"));
        let band = with(
            r#"{ "key": "turbidity", "unit": "ntu", "label": "Turbidity", "viz": "gauge", "band": [5.0, 0.0] }"#,
            "",
        );
        assert!(error_of("tank-level", &band).contains("low end comes first"));
        let unknown_state = with(
            r#"{ "key": "pump", "unit": "state", "label": "Pump", "viz": "switch", "state": "state.priming" }"#,
            "",
        );
        assert!(error_of("tank-level", &unknown_state).contains("no words for"));
        let worded = with(
            r#"{ "key": "pump", "unit": "state", "label": "Pump", "viz": "switch", "state": "state.priming" }"#,
            r#", "messages": { "state.priming": "Priming" }"#,
        );
        manifest("tank-level", &worded)
            .check(&repo_root())
            .expect("a worded state passes");
        let shipped = with(
            r#"{ "key": "gate", "unit": "state", "label": "Gate", "viz": "valve", "state": "state.closed" }"#,
            "",
        );
        manifest("tank-level", &shipped)
            .check(&repo_root())
            .expect("a state the dashboard ships passes");
        let no_english = with(
            "",
            r#", "messages": { "event.dry": { "sw": "Kimekauka" } }"#,
        );
        assert!(error_of("tank-level", &no_english).contains("no `en` text"));
        let odd_code = with("", r#", "messages": { "dry": "Dry" }"#);
        assert!(error_of("tank-level", &odd_code).contains("neither a `state.`"));
    }

    #[test]
    fn periods_read_in_the_largest_whole_unit() {
        assert_eq!(period(45), "45 s");
        assert_eq!(period(60), "1 min");
        assert_eq!(period(300), "5 min");
        assert_eq!(period(3600), "1 hour");
        assert_eq!(period(7200), "2 hours");
        assert_eq!(percent(0.5), "50 %");
        assert_eq!(percent(0.15), "15 %");
        assert_eq!(percent(0.125), "12.5 %");
        assert_eq!(number(0.3), "0.3");
    }
}
