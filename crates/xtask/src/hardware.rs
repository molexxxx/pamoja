//! The hardware reference in `docs/hardware.toml`: the parts, buses, radios and targets
//! pamoja drives, each described from the manufacturer's own document or the standard that
//! defines it. [`Hardware::table`] renders the page, and [`Hardware::check`] ties the entries
//! to the code: every driver module must be claimed by an entry, and the LoRaWAN
//! entry must name every regional plan the crate implements, so a driver or a plan cannot be
//! added without the page following it. `cargo xtask links` fetches every `source`.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use toml_edit::{DocumentMut, Item};

/// The price bands entries may use. They are indicative street prices in USD for a breakout
/// module or board, deliberately coarse, and not quotes. A bus or a specification costs
/// nothing to speak, so those entries say so rather than carrying a number.
pub const BANDS: &[&str] = &[
    "under $5",
    "$5 to $20",
    "$20 to $60",
    "$60 to $200",
    "over $200",
    "not applicable",
];

/// The driver crates whose modules must each be documented by an entry.
const DRIVERS: &[&str] = &["pamoja-sensors", "pamoja-actuators"];

/// Module files that carry no driver, so no entry describes them.
const NOT_DRIVERS: &[&str] = &["lib.rs", "error.rs"];

/// A group of entries that share a heading on the page.
pub struct Group {
    /// The id entries name in their `group` field.
    pub key: String,
    /// The heading the group renders under.
    pub title: String,
    /// One line under the heading saying what the group covers.
    pub intent: String,
}

/// One part, bus, radio, or target, and the document it is described from.
pub struct Entry {
    /// The id, unique across the file.
    pub key: String,
    /// The group it renders under.
    pub group: String,
    /// What it is called.
    pub name: String,
    /// Who makes it, or the body that defines it.
    pub vendor: String,
    /// How it connects, empty for a target that hosts pamoja rather than attaching to it.
    pub interface: String,
    /// One sentence on what it is for.
    pub summary: String,
    /// Figures from the source document, each already written as "label: value".
    pub specs: Vec<String>,
    /// One of [`BANDS`].
    pub cost: String,
    /// The crates that speak to it.
    pub crates: Vec<String>,
    /// The driver modules that target it, as `<crate>/src/<file>.rs`.
    pub modules: Vec<String>,
    /// The LoRaWAN regional plans it covers, checked against the `Region` enum.
    pub regions: Vec<String>,
    /// The document: its title, and its number or revision where it has one.
    pub source_name: String,
    /// The short label the source column links.
    pub source_label: String,
    /// Where that document lives, on the vendor's or the body's own domain.
    pub source: String,
    /// Why `cargo xtask links` cannot fetch the source and a person checks it instead:
    /// a few vendors' sites refuse every scripted client. Empty for the rest.
    pub manual_check: String,
}

/// The whole reference: groups in page order, entries in file order.
pub struct Hardware {
    /// The headings, in the order they render.
    pub groups: Vec<Group>,
    /// Every entry, in the order it appears in the file.
    pub entries: Vec<Entry>,
}

impl Hardware {
    /// Read `docs/hardware.toml` under `root`.
    ///
    /// # Arguments
    ///
    /// * `root` - the repository root.
    ///
    /// # Returns
    ///
    /// The parsed reference.
    ///
    /// # Errors
    ///
    /// When the file is missing or a required field is absent or the wrong type.
    pub fn load(root: &Path) -> Result<Hardware, String> {
        let path = root.join("docs/hardware.toml");
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        Hardware::parse(&text)
    }

    /// Parse the reference from its TOML text.
    ///
    /// # Arguments
    ///
    /// * `text` - the contents of `docs/hardware.toml`.
    ///
    /// # Returns
    ///
    /// The parsed reference.
    ///
    /// # Errors
    ///
    /// When a required field is missing or has the wrong type.
    pub fn parse(text: &str) -> Result<Hardware, String> {
        let doc: DocumentMut = text
            .parse()
            .map_err(|err| format!("hardware.toml is not valid TOML: {err}"))?;

        let mut groups = Vec::new();
        for table in tables(&doc, "group")? {
            groups.push(Group {
                key: string(table, "key", "group")?,
                title: string(table, "title", "group")?,
                intent: string(table, "intent", "group")?,
            });
        }

        let mut entries = Vec::new();
        for table in tables(&doc, "entry")? {
            let key = string(table, "key", "entry")?;
            let context = format!("entry {key}");
            entries.push(Entry {
                group: string(table, "group", &context)?,
                name: string(table, "name", &context)?,
                vendor: string(table, "vendor", &context)?,
                interface: optional(table, "interface"),
                summary: string(table, "summary", &context)?,
                specs: strings(table, "specs", &context)?,
                cost: string(table, "cost", &context)?,
                crates: optional_strings(table, "crates", &context)?,
                modules: optional_strings(table, "modules", &context)?,
                regions: optional_strings(table, "regions", &context)?,
                source_name: string(table, "source_name", &context)?,
                source_label: string(table, "source_label", &context)?,
                source: string(table, "source", &context)?,
                manual_check: optional(table, "manual_check"),
                key,
            });
        }

        Ok(Hardware { groups, entries })
    }

    /// The entries of one group, in file order.
    ///
    /// # Arguments
    ///
    /// * `group` - the group key.
    ///
    /// # Returns
    ///
    /// The entries naming that group.
    pub fn in_group<'a>(&'a self, group: &'a str) -> impl Iterator<Item = &'a Entry> {
        self.entries.iter().filter(move |e| e.group == group)
    }

    /// Every source URL on the page, in file order.
    ///
    /// # Returns
    ///
    /// One (entry key, document name, URL, manual-check reason) per entry; the reason is
    /// empty for a source the link check fetches itself.
    pub fn sources(&self) -> Vec<(&str, &str, &str, &str)> {
        self.entries
            .iter()
            .map(|e| {
                (
                    e.key.as_str(),
                    e.source_name.as_str(),
                    e.source.as_str(),
                    e.manual_check.as_str(),
                )
            })
            .collect()
    }

    /// Render the page body. Each group gets a heading, its line of intent, a compact table
    /// for scanning (part, interface, cost, source), and then one block per part carrying
    /// the summary and the figures from its document.
    ///
    /// # Returns
    ///
    /// The Markdown that replaces the `<!-- table: hardware -->` region.
    pub fn table(&self) -> String {
        let mut out = Vec::new();
        for group in &self.groups {
            let entries: Vec<&Entry> = self.in_group(&group.key).collect();
            if entries.is_empty() {
                continue;
            }
            let mut section = format!("### {}\n\n{}\n\n", group.title, group.intent);
            section.push_str("| Part | Interface | Cost | Source |\n| --- | --- | --- | --- |\n");
            for entry in &entries {
                section.push_str(&format!(
                    "| [{}](#{}) | {} | {} | [{}]({}) |\n",
                    entry.name,
                    entry.key,
                    if entry.interface.is_empty() {
                        "-"
                    } else {
                        entry.interface.as_str()
                    },
                    entry.cost,
                    entry.source_label,
                    entry.source,
                ));
            }
            for entry in &entries {
                section.push_str(&format!(
                    "\n#### {} {{#{}}}\n\n**{}.** {}\n\n",
                    entry.name, entry.key, entry.vendor, entry.summary
                ));
                for spec in &entry.specs {
                    section.push_str(&format!("- {spec}\n"));
                }
                section.push_str(&format!(
                    "\nFrom [{}]({}).\n",
                    entry.source_name, entry.source
                ));
            }
            out.push(section.trim_end().to_owned());
        }
        out.join("\n\n")
    }

    /// Check the reference against the code and against its own rules.
    ///
    /// # Arguments
    ///
    /// * `root` - the repository root, holding the driver crates.
    ///
    /// # Returns
    ///
    /// Nothing when the page is current.
    ///
    /// # Errors
    ///
    /// When an entry names an unknown group, crate, module, or price band, when two entries
    /// share a key, when a driver module has no entry, or when the LoRaWAN entry does not
    /// name every regional plan the crate implements.
    pub fn check(&self, root: &Path) -> Result<(), String> {
        let groups: BTreeSet<&str> = self.groups.iter().map(|g| g.key.as_str()).collect();
        let mut seen = BTreeSet::new();
        let mut claimed: Vec<&str> = Vec::new();

        for entry in &self.entries {
            let at = format!("hardware.toml entry {}", entry.key);
            if !seen.insert(entry.key.as_str()) {
                return Err(format!("{at}: two entries share this key"));
            }
            if !groups.contains(entry.group.as_str()) {
                return Err(format!("{at}: unknown group `{}`", entry.group));
            }
            if !BANDS.contains(&entry.cost.as_str()) {
                return Err(format!(
                    "{at}: cost `{}` is not one of {}",
                    entry.cost,
                    BANDS.join(", ")
                ));
            }
            if !entry.source.starts_with("https://") {
                return Err(format!("{at}: the source must be an https URL"));
            }
            if entry.specs.is_empty() {
                return Err(format!("{at}: needs at least one figure from its source"));
            }
            for krate in &entry.crates {
                if !root.join("crates").join(krate).join("Cargo.toml").is_file() {
                    return Err(format!("{at}: names the unknown crate `{krate}`"));
                }
            }
            for module in &entry.modules {
                if !root.join("crates").join(module).is_file() {
                    return Err(format!("{at}: names the missing module `{module}`"));
                }
                claimed.push(module.as_str());
            }
        }

        // A module may be claimed by more than one entry: the stepper sequencer drives both
        // the A4988 and the DRV8825, and neither is more the part it targets than the other.
        let documented: BTreeSet<&str> = claimed.into_iter().collect();
        for module in driver_modules(root)? {
            if !documented.contains(module.as_str()) {
                return Err(format!(
                    "hardware.toml: the driver `{module}` has no entry; add the part it targets, \
                     with the manufacturer's document, so the hardware page stays current"
                ));
            }
        }

        self.check_regions(root)
    }

    // Every variant of the LoRaWAN `Region` enum must appear on the page, so a plan added to
    // the crate cannot go unlisted.
    fn check_regions(&self, root: &Path) -> Result<(), String> {
        let listed: BTreeSet<&str> = self
            .entries
            .iter()
            .flat_map(|entry| entry.regions.iter())
            .map(String::as_str)
            .collect();
        if listed.is_empty() {
            return Err("hardware.toml: no entry lists the LoRaWAN regional plans".to_owned());
        }
        for region in regions(root)? {
            if !listed.contains(region.as_str()) {
                return Err(format!(
                    "hardware.toml: the crate implements the {region} channel plan, which no \
                     entry lists"
                ));
            }
        }
        Ok(())
    }
}

// The driver module files under the crates whose modules each target a real part.
fn driver_modules(root: &Path) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for krate in DRIVERS {
        let dir = root.join("crates").join(krate).join("src");
        let entries =
            fs::read_dir(&dir).map_err(|err| format!("reading {}: {err}", dir.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|err| format!("reading {}: {err}", dir.display()))?
                .path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if path.is_file() && name.ends_with(".rs") && !NOT_DRIVERS.contains(&name) {
                out.push(format!("{krate}/src/{name}"));
            }
        }
    }
    out.sort();
    Ok(out)
}

// The variants of the `Region` enum, read from the crate that defines the channel plans.
fn regions(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join("crates/pamoja-lora/src/region/plans.rs");
    let source =
        fs::read_to_string(&path).map_err(|err| format!("reading {}: {err}", path.display()))?;
    let file =
        syn::parse_file(&source).map_err(|err| format!("parsing {}: {err}", path.display()))?;
    for item in file.items {
        if let syn::Item::Enum(item) = item {
            if item.ident == "Region" {
                return Ok(item
                    .variants
                    .iter()
                    .map(|variant| variant.ident.to_string().to_uppercase())
                    .collect());
            }
        }
    }
    Err(format!("{} declares no `Region` enum", path.display()))
}

fn tables<'a>(
    doc: &'a DocumentMut,
    name: &str,
) -> Result<Vec<&'a dyn toml_edit::TableLike>, String> {
    let Some(item) = doc.get(name) else {
        return Ok(Vec::new());
    };
    let array = item
        .as_array_of_tables()
        .ok_or_else(|| format!("[[{name}]] must be an array of tables"))?;
    Ok(array
        .iter()
        .map(|table| table as &dyn toml_edit::TableLike)
        .collect())
}

fn string(table: &dyn toml_edit::TableLike, key: &str, context: &str) -> Result<String, String> {
    table
        .get(key)
        .and_then(Item::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{context}: `{key}` must be a string"))
}

fn optional(table: &dyn toml_edit::TableLike, key: &str) -> String {
    table
        .get(key)
        .and_then(Item::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn strings(
    table: &dyn toml_edit::TableLike,
    key: &str,
    context: &str,
) -> Result<Vec<String>, String> {
    let array = table
        .get(key)
        .and_then(Item::as_array)
        .ok_or_else(|| format!("{context}: `{key}` must be an array of strings"))?;
    array
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{context}: `{key}` must hold only strings"))
        })
        .collect()
}

fn optional_strings(
    table: &dyn toml_edit::TableLike,
    key: &str,
    context: &str,
) -> Result<Vec<String>, String> {
    if table.get(key).is_none() {
        return Ok(Vec::new());
    }
    strings(table, key, context)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
[[group]]
key = "sensors"
title = "Sensors"
intent = "Parts a driver decodes."

[[entry]]
key = "bme280"
group = "sensors"
name = "BME280"
vendor = "Bosch Sensortec"
interface = "I2C or SPI"
summary = "Humidity, pressure and temperature on one die."
specs = ["Temperature: -40 to 85 C"]
cost = "$5 to $20"
crates = ["pamoja-sensors"]
modules = ["pamoja-sensors/src/bme280.rs"]
regions = ["EU868"]
source_name = "BME280 datasheet"
source_label = "datasheet"
source = "https://example.invalid/bme280"
"#;

    #[test]
    fn an_entry_carries_its_source_and_its_modules() {
        let hardware = Hardware::parse(MINIMAL).expect("parses");
        let entry = &hardware.entries[0];
        assert_eq!(entry.name, "BME280");
        assert_eq!(entry.modules, ["pamoja-sensors/src/bme280.rs"]);
        assert_eq!(hardware.sources().len(), 1);
    }

    #[test]
    fn the_table_links_the_source_and_lists_the_figures() {
        let rendered = Hardware::parse(MINIMAL).expect("parses").table();
        assert!(rendered.contains("### Sensors"));
        assert!(rendered.contains("| [BME280](#bme280) | I2C or SPI | $5 to $20 | [datasheet](https://example.invalid/bme280) |"));
        assert!(rendered.contains("#### BME280 {#bme280}"));
        assert!(rendered.contains("- Temperature: -40 to 85 C"));
        assert!(rendered.contains("From [BME280 datasheet](https://example.invalid/bme280)."));
    }

    #[test]
    fn a_price_outside_the_bands_is_refused() {
        let text = MINIMAL.replace("$5 to $20", "$12.34");
        let hardware = Hardware::parse(&text).expect("parses");
        let err = hardware
            .check(Path::new("."))
            .expect_err("an invented price is not a band");
        assert!(err.contains("is not one of"), "{err}");
    }

    #[test]
    fn an_entry_in_no_group_is_refused() {
        let text = MINIMAL.replace("group = \"sensors\"\nname", "group = \"radios\"\nname");
        let err = Hardware::parse(&text)
            .expect("parses")
            .check(Path::new("."))
            .expect_err("the group does not exist");
        assert!(err.contains("unknown group"), "{err}");
    }

    #[test]
    fn a_source_that_is_not_https_is_refused() {
        let text = MINIMAL.replace("https://example.invalid", "http://example.invalid");
        let err = Hardware::parse(&text)
            .expect("parses")
            .check(Path::new("."))
            .expect_err("an http source is not accepted");
        assert!(err.contains("https"), "{err}");
    }
}
