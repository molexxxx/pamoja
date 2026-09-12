//! The hardware reference in `docs/hardware.toml`: the parts, buses, radios and targets
//! pamoja drives, each described from the manufacturer's own document or the standard that
//! defines it. [`Hardware::table`] renders the page, and [`Hardware::check`] ties the entries
//! to the code: every driver module must be claimed by an entry, so a driver cannot be added
//! without the page following it. `cargo xtask links` fetches every `source`.
//! [`Hardware::for_guide`] gives a guide the parts its crates drive, so the two pages point
//! at each other.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use toml_edit::{DocumentMut, Item};

use crate::catalog::{escape, rustdoc_url, Catalog, SITE};

/// The repository, for the links to a driver's source.
const REPO: &str = "https://github.com/molexxxx/pamoja";

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
const DRIVERS: &[&str] = &["pamoja-sensors", "pamoja-actuators", "pamoja-radios"];

/// Module files that carry no driver, so no entry describes them.
const NOT_DRIVERS: &[&str] = &[
    "lib.rs",
    "error.rs",
    "driver.rs",
    "duty.rs",
    "linux.rs",
    "mesh.rs",
    "radio.rs",
];

/// A group of entries that share a heading on the page.
pub struct Group {
    /// The id entries name in their `group` field.
    pub key: String,
    /// The heading the group renders under.
    pub title: String,
    /// One line under the heading saying what the group covers.
    pub intent: String,
    /// The page that explains how to choose between the group's parts, as `<page>.md`
    /// under `docs/`. Empty for a group whose entries carry their own page.
    pub explains: String,
    /// What that page is called on its own heading.
    pub explains_title: String,
    /// One line saying what the reader gets there.
    pub explains_what: String,
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
    /// The page under `docs/` that wires a part to this board and runs the first program
    /// on it, as `boards/<board>.md`. Empty for everything but a board.
    pub page: String,
    /// The document: its title, and its number or revision where it has one.
    pub source_name: String,
    /// The short label the source column links.
    pub source_label: String,
    /// Where that document lives, on the vendor's or the body's own domain.
    pub source: String,
    /// Why `cargo xtask links` cannot fetch the source and a person checks it instead:
    /// a few vendors' sites refuse every scripted client. Empty for the rest.
    pub manual_check: String,
    /// Where to buy it: a few product pages, the cheapest reputable option first, each with
    /// the price the page listed on the day it was read. Empty for a bus, a protocol, or a
    /// specification, which cost nothing to speak, and for a part no reputable store lists.
    pub buy: Vec<Buy>,
    /// For a part with a price band and no store: the day the stores were last searched, so
    /// the card can say so. Empty otherwise.
    pub buy_checked: String,
    /// For an entry that is not itself a thing you buy, a piece of firmware, a standard, a
    /// protocol, or an instruction set: the heading its offers render under, naming them as
    /// hosts rather than as its own price. Empty for a part, which carries a cost line.
    pub hosts: String,
}

/// One place to buy a part, and what it cost there on the day the page was read.
pub struct Buy {
    /// The vendor, as a reader knows it.
    pub vendor: String,
    /// The product, as the vendor's page names it.
    pub name: String,
    /// The product page.
    pub url: String,
    /// The price as listed, with its currency.
    pub price: String,
    /// The day the page was read, as YYYY-MM-DD.
    pub checked: String,
    /// Whether the price was read from the page itself; a vendor that refuses scripted
    /// clients is priced from a listing of the page instead, and the page says so.
    pub verified: bool,
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
                explains: optional(table, "explains"),
                explains_title: optional(table, "explains_title"),
                explains_what: optional(table, "explains_what"),
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
                page: optional(table, "page"),
                source_name: string(table, "source_name", &context)?,
                source_label: string(table, "source_label", &context)?,
                source: string(table, "source", &context)?,
                manual_check: optional(table, "manual_check"),
                buy: buys(table, &context)?,
                buy_checked: optional(table, "buy_checked"),
                hosts: optional(table, "hosts"),
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

    /// Render the page body. Each group gets a heading, its line of intent, and one card
    /// per entry. A card breaks the entry down into labeled facts (its interface, each
    /// figure from its document, and its price band with the lowest listed price), says
    /// where to buy it with the price each page listed, and points at the document it was
    /// written from, the driver's source, its crates, and the guides that use it.
    ///
    /// # Arguments
    ///
    /// * `catalog` - the capability map, for the guides a part's crates belong to.
    ///
    /// # Returns
    ///
    /// The Markdown that replaces the `<!-- table: hardware -->` region.
    pub fn table(&self, catalog: &Catalog) -> String {
        let mut out = Vec::new();
        for group in &self.groups {
            let entries: Vec<&Entry> = self.in_group(&group.key).collect();
            if entries.is_empty() {
                continue;
            }
            let mut section = format!(
                "## {}\n\n{}\n\n{}\n<div class=\"hw-cards\">\n",
                group.title,
                group.intent,
                index(&entries, &group.title)
            );
            for entry in &entries {
                section.push_str(&card(entry, group, catalog, &self.entries));
            }
            section.push_str("</div>");
            out.push(section);
        }
        out.join("\n\n")
    }

    /// The parts a guide's crates drive, as one Markdown line for the guide's reference
    /// section, or nothing when the capability has no part on the page.
    ///
    /// # Arguments
    ///
    /// * `key` - the capability key of the guide.
    /// * `catalog` - the capability map.
    ///
    /// # Returns
    ///
    /// A `- Hardware: ...` line linking each part's card, with a leading newline, or an
    /// empty string.
    pub fn for_guide(&self, key: &str, catalog: &Catalog) -> String {
        let Some(capability) = catalog.capability(key) else {
            return String::new();
        };
        let parts: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| {
                entry
                    .crates
                    .iter()
                    .any(|krate| capability.crates.contains(krate))
            })
            .map(|entry| format!("[{}]({SITE}/hardware.html#{})", entry.name, entry.key))
            .collect();
        if parts.is_empty() {
            String::new()
        } else {
            format!("\n- Hardware: {}", parts.join(", "))
        }
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
    /// share a key, or when a driver module has no entry.
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
            if !entry.page.is_empty() && !root.join("docs").join(&entry.page).is_file() {
                return Err(format!(
                    "{at}: the board page `{}` is not under docs/",
                    entry.page
                ));
            }
            if !entry.buy.is_empty() && entry.cost == "not applicable" {
                return Err(format!(
                    "{at}: lists where to buy a thing that has no price"
                ));
            }
            if !entry.buy_checked.is_empty() {
                if !is_date(&entry.buy_checked) {
                    return Err(format!("{at}: `buy_checked` must be a YYYY-MM-DD date"));
                }
                if !entry.buy.is_empty() {
                    return Err(format!("{at}: `buy_checked` goes with an empty buy list"));
                }
            }
            let mut pages = BTreeSet::new();
            for buy in &entry.buy {
                if !buy.url.starts_with("https://") {
                    return Err(format!(
                        "{at}: the {} page must be an https URL",
                        buy.vendor
                    ));
                }
                if !pages.insert(buy.url.as_str()) {
                    return Err(format!("{at}: lists {} twice", buy.url));
                }
                if buy.price.trim().is_empty() {
                    return Err(format!("{at}: the {} line has no price", buy.vendor));
                }
                if !is_date(&buy.checked) {
                    return Err(format!(
                        "{at}: the {} line's `checked` must be a YYYY-MM-DD date",
                        buy.vendor
                    ));
                }
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

        Ok(())
    }
}

// One entry as a card: the head, the facts down one column, and a foot with where to buy
// it beside what to read and build with, each a list of rows of one shape. Nothing inside
// the card is separated by a blank line, so Markdown takes it as one block of HTML.
fn card(entry: &Entry, group: &Group, catalog: &Catalog, all: &[Entry]) -> String {
    let mut facts: Vec<(String, String)> = Vec::new();
    if !entry.interface.is_empty() {
        facts.push(("Interface".to_owned(), escape(&entry.interface)));
    }
    for spec in &entry.specs {
        match spec.split_once(": ") {
            Some((label, value)) => facts.push((escape(label), escape(value))),
            None => facts.push(("Note".to_owned(), escape(spec))),
        }
    }
    if entry.cost != "not applicable" && entry.hosts.is_empty() {
        facts.push((
            "Typical cost".to_owned(),
            format!("{} for a breakout module or a board", escape(&entry.cost)),
        ));
    }
    let facts: String = facts
        .iter()
        .map(|(label, value)| format!("<div><dt>{label}</dt><dd>{value}</dd></div>"))
        .collect();

    let document = match group.key.as_str() {
        "buses" => "Specification",
        "targets" | "standards" => "Documentation",
        _ => "Datasheet",
    };
    let mut links = vec![row(
        &entry.source,
        "",
        document,
        &format!("<small>{}</small>", escape(&entry.source_label)),
        "&#8599;",
    )];
    for module in &entry.modules {
        let file = module.rsplit('/').next().unwrap_or(module);
        links.push(row(
            &format!("{REPO}/blob/main/crates/{module}"),
            "",
            "Driver source",
            &format!("<small><code>{}</code></small>", escape(file)),
            "&#8599;",
        ));
    }
    // A part driven by more than one crate names what each one does, so its rows do not
    // read as the same link twice.
    for krate in &entry.crates {
        let title = match entry.crates.len() > 1 {
            true => catalog
                .ordered()
                .iter()
                .find(|capability| capability.crates.contains(krate))
                .map(|capability| format!("Crate for {}", escape(&capability.title)))
                .unwrap_or_else(|| "Crate".to_owned()),
            false => "Crate".to_owned(),
        };
        links.push(row(
            &rustdoc_url(krate),
            "",
            &title,
            &format!("<small><code>{krate}</code></small>"),
            "&#8599;",
        ));
    }
    if !entry.page.is_empty() {
        links.push(row(
            &format!("{SITE}/{}.html", entry.page.trim_end_matches(".md")),
            " guide",
            "Board page",
            "<small>wiring, setup, and the first program</small>",
            "&#8594;",
        ));
    }
    for capability in catalog.ordered() {
        let drives = capability
            .crates
            .iter()
            .any(|krate| entry.crates.contains(krate));
        if let (true, Some(guide)) = (drives, &capability.guide) {
            links.push(row(
                &format!("{SITE}/{}.html", guide.trim_end_matches(".md")),
                " guide",
                &format!("{} guide", escape(&capability.title)),
                &format!("<small>{}</small>", escape(clause(&capability.summary))),
                "&#8594;",
            ));
        }
    }
    if !group.explains.is_empty() {
        links.push(row(
            &format!("{SITE}/{}.html", group.explains.trim_end_matches(".md")),
            " guide",
            &escape(&group.explains_title),
            &format!("<small>{}</small>", escape(&group.explains_what)),
            "&#8594;",
        ));
    }

    // The left panel: the offers, else the parts on this page that speak the bus, else
    // where to search for the part; a part with a price band and no store says so first.
    let left = if !entry.buy.is_empty() {
        let same_day = entry.buy.iter().all(|b| b.checked == entry.buy[0].checked);
        let heading = if entry.hosts.is_empty() {
            "Where to buy"
        } else {
            entry.hosts.as_str()
        };
        let when = if same_day {
            format!("prices as listed on {}", entry.buy[0].checked)
        } else {
            "prices as listed on the day named".to_owned()
        };
        let offers: String = entry
            .buy
            .iter()
            .map(|b| {
                let mut price = escape(&b.price);
                if !same_day {
                    price.push_str(&format!(" on {}", b.checked));
                }
                let mut detail = format!("<small>{}</small>", escape(&b.name));
                if !b.verified {
                    detail.push_str(
                        "<small class=\"hw-note\">listed price; the page refuses scripted readers</small>",
                    );
                }
                format!(
                    "<li><a class=\"hw-row\" href=\"{}\"><span class=\"hw-main\"><b>{}</b>{detail}</span><span class=\"hw-price\">{price}</span></a></li>",
                    b.url,
                    escape(&b.vendor)
                )
            })
            .collect();
        format!("<section class=\"hw-buy\"><h4>{}</h4><ul class=\"hw-rows\">{offers}</ul><p class=\"hw-when\">{when}</p></section>\n", escape(heading))
    } else if entry.cost == "not applicable" && !speakers(entry, all).is_empty() {
        let parts: String = speakers(entry, all)
            .iter()
            .map(|part| {
                row(
                    &format!("#{}", part.key),
                    "",
                    &escape(&part.name),
                    &format!("<small>{}</small>", escape(&part.interface)),
                    "&#8595;",
                )
            })
            .collect();
        format!("<section class=\"hw-buy\"><h4>Parts on this bus</h4><ul class=\"hw-rows\">{parts}</ul></section>\n")
    } else {
        let note = if entry.cost == "not applicable" {
            String::new()
        } else {
            let since = if entry.buy_checked.is_empty() {
                String::new()
            } else {
                format!(" as of {}", escape(&entry.buy_checked))
            };
            format!("<p class=\"hw-none\">No reputable store lists this part{since}. The makers' stores and the larger distributors are searched as the page is maintained, and a listing that appears is added here with its price.</p>")
        };
        let searches: String = SEARCHES
            .iter()
            .map(|(store, url)| {
                row(
                    &format!("{url}{}", query(&entry.name)),
                    "",
                    &format!("Search {store}"),
                    &format!("<small>for {}</small>", escape(&entry.name)),
                    "&#8599;",
                )
            })
            .collect();
        format!("<section class=\"hw-buy\"><h4>Find parts</h4>{note}<ul class=\"hw-rows\">{searches}</ul></section>\n")
    };

    // The part name is a real Markdown heading, so it carries the page's anchor, its entry
    // in the table of contents, and its own row in the search index. Blank lines around it
    // close and reopen the surrounding block of HTML.
    format!(
        "<article class=\"hw-card\" aria-labelledby=\"{key}\">\n<header class=\"hw-head\">\n\n### {name} {{#{key}}}\n\n<p class=\"hw-by\">{vendor}</p>\n<p class=\"hw-summary\">{summary}</p>\n</header>\n<dl class=\"hw-facts\">{facts}</dl>\n<div class=\"hw-foot\">\n{left}<section class=\"hw-learn\"><h4>Read and build</h4><ul class=\"hw-rows\">{}</ul></section>\n</div>\n</article>\n",
        links.join(""),
        key = entry.key,
        name = escape(&entry.name),
        vendor = escape(&entry.vendor),
        summary = escape(&entry.summary),
    )
}

// The parts of a group as a row of anchors above its cards, so a reader reaches a part by
// name at any width rather than scrolling the wall.
fn index(entries: &[&Entry], title: &str) -> String {
    let links: String = entries
        .iter()
        .map(|entry| format!("<a href=\"#{}\">{}</a>", entry.key, escape(&entry.name)))
        .collect();
    format!(
        "<nav class=\"hw-index\" aria-label=\"{} index\">{links}</nav>\n",
        escape(title)
    )
}

/// Where a reader can search for a part by name, when no store on this page lists it.
const SEARCHES: [(&str, &str); 4] = [
    ("Adafruit", "https://www.adafruit.com/search?q="),
    ("SparkFun", "https://www.sparkfun.com/search/results?term="),
    (
        "Digi-Key",
        "https://www.digikey.com/en/products/result?keywords=",
    ),
    ("Mouser", "https://www.mouser.com/c/?q="),
];

// The parts on the page whose interface names this bus, sensors and actuators and radios,
// so a bus card leads to what hangs off it.
fn speakers<'a>(bus: &Entry, all: &'a [Entry]) -> Vec<&'a Entry> {
    let name = bus.name.to_ascii_lowercase();
    let token = name.split_whitespace().next().unwrap_or_default();
    if token.is_empty() {
        return Vec::new();
    }
    all.iter()
        .filter(|part| part.cost != "not applicable" && part.key != bus.key)
        .filter(|part| {
            part.interface
                .to_ascii_lowercase()
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
                .any(|word| word == token)
        })
        .collect()
}

// The opening clause of a capability's summary, which is what a card's guide row has room
// for. A summary that lists its parts or qualifies itself is cut at that turn.
fn clause(summary: &str) -> &str {
    let cut = [": ", "; ", ", and ", ". "]
        .iter()
        .filter_map(|mark| summary.find(mark))
        .min();
    match cut {
        Some(at) => summary[..at].trim_end_matches(['.', ',']),
        None => summary.trim_end_matches('.'),
    }
}

// A part name as a search query.
fn query(name: &str) -> String {
    name.split_whitespace()
        .map(|word| {
            word.chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                        c.to_string()
                    } else {
                        format!("%{:02X}", c as u32)
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("+")
}

// One row of a card's foot: what it is and a detail on the left, the way out on the right.
fn row(href: &str, class: &str, title: &str, detail: &str, go: &str) -> String {
    format!(
        "<li><a class=\"hw-row{class}\" href=\"{href}\"><span class=\"hw-main\"><b>{title}</b>{detail}</span><span class=\"hw-go\" aria-hidden=\"true\">{go}</span></a></li>"
    )
}

// A YYYY-MM-DD date, checked by shape.
fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
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

pub(crate) fn tables<'a>(
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

pub(crate) fn string(
    table: &dyn toml_edit::TableLike,
    key: &str,
    context: &str,
) -> Result<String, String> {
    table
        .get(key)
        .and_then(Item::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{context}: `{key}` must be a string"))
}

pub(crate) fn optional(table: &dyn toml_edit::TableLike, key: &str) -> String {
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

// The `[[entry.buy]]` tables of an entry, in file order.
fn buys(table: &dyn toml_edit::TableLike, context: &str) -> Result<Vec<Buy>, String> {
    let Some(item) = table.get("buy") else {
        return Ok(Vec::new());
    };
    let array = item
        .as_array_of_tables()
        .ok_or_else(|| format!("{context}: `buy` must be an array of tables"))?;
    array
        .iter()
        .map(|buy| {
            let buy = buy as &dyn toml_edit::TableLike;
            Ok(Buy {
                vendor: string(buy, "vendor", context)?,
                name: string(buy, "name", context)?,
                url: string(buy, "url", context)?,
                price: string(buy, "price", context)?,
                checked: string(buy, "checked", context)?,
                verified: buy.get("verified").and_then(Item::as_bool).unwrap_or(true),
            })
        })
        .collect()
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
source_name = "BME280 datasheet"
source_label = "datasheet"
source = "https://example.invalid/bme280"
"#;

    const CATALOG: &str = r#"
[[chapter]]
key = "sensing"
title = "Sensing and actuation"
intent = "Parts."

[[capability]]
key = "sensors"
chapter = "sensing"
title = "Sensor drivers"
summary = "Decoders."
crates = ["pamoja-sensors"]
node = "sensors"
python = "sensors"
dotnet = ["Bme280"]
guide = "guides/sensors.md"

[engine]
crates = ["pamoja-core"]
"#;

    fn catalog() -> Catalog {
        Catalog::parse(CATALOG).expect("the catalog parses")
    }

    #[test]
    fn an_entry_carries_its_source_and_its_modules() {
        let hardware = Hardware::parse(MINIMAL).expect("parses");
        let entry = &hardware.entries[0];
        assert_eq!(entry.name, "BME280");
        assert_eq!(entry.modules, ["pamoja-sensors/src/bme280.rs"]);
        assert_eq!(hardware.sources().len(), 1);
    }

    #[test]
    fn a_card_breaks_the_part_down_and_points_at_the_document_the_driver_and_the_guide() {
        let rendered = Hardware::parse(MINIMAL).expect("parses").table(&catalog());
        assert!(rendered.starts_with("## Sensors\n\nParts a driver decodes.\n\n<nav class=\"hw-index\" aria-label=\"Sensors index\"><a href=\"#bme280\">BME280</a></nav>\n\n<div class=\"hw-cards\">\n<article class=\"hw-card\" aria-labelledby=\"bme280\">\n"), "{rendered}");
        assert!(rendered.contains("<header class=\"hw-head\">\n\n### BME280 {#bme280}\n\n<p class=\"hw-by\">Bosch Sensortec</p>\n<p class=\"hw-summary\">Humidity, pressure and temperature on one die.</p>\n</header>"), "{rendered}");
        assert!(rendered.contains("<dl class=\"hw-facts\"><div><dt>Interface</dt><dd>I2C or SPI</dd></div><div><dt>Temperature</dt><dd>-40 to 85 C</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>"), "{rendered}");
        assert!(rendered.contains("<li><a class=\"hw-row\" href=\"https://example.invalid/bme280\"><span class=\"hw-main\"><b>Datasheet</b><small>datasheet</small></span><span class=\"hw-go\" aria-hidden=\"true\">&#8599;</span></a></li>"), "{rendered}");
        assert!(rendered.contains("<a class=\"hw-row\" href=\"https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/bme280.rs\"><span class=\"hw-main\"><b>Driver source</b><small><code>bme280.rs</code></small></span>"));
        assert!(rendered.contains("<a class=\"hw-row\" href=\"https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html\"><span class=\"hw-main\"><b>Crate</b><small><code>pamoja-sensors</code></small></span>"));
        assert!(rendered.contains("<a class=\"hw-row guide\" href=\"https://pamoja.molex.cloud/docs/guides/sensors.html\"><span class=\"hw-main\"><b>Sensor drivers guide</b><small>Decoders</small></span><span class=\"hw-go\" aria-hidden=\"true\">&#8594;</span></a>"));
        assert!(
            rendered.contains("<section class=\"hw-buy\"><h4>Find parts</h4><p class=\"hw-none\">No reputable store lists this part.") && rendered.contains("<b>Search Adafruit</b><small>for BME280</small>"),
            "a part no store lists says so and offers searches: {rendered}"
        );
        assert!(rendered.ends_with("</article>\n</div>"), "{rendered}");
    }

    #[test]
    fn a_bus_links_its_specification_and_a_guide_links_its_parts() {
        let bus = MINIMAL
            .replace("key = \"bme280\"", "key = \"i2c\"")
            .replace("group = \"sensors\"", "group = \"buses\"")
            .replace("cost = \"$5 to $20\"", "cost = \"not applicable\"")
            .replace("modules = [\"pamoja-sensors/src/bme280.rs\"]\n", "")
            + "\n[[group]]\nkey = \"buses\"\ntitle = \"Buses\"\nintent = \"Wires.\"\n";
        let hardware = Hardware::parse(&bus).expect("parses");
        let rendered = hardware.table(&catalog());
        assert!(
            rendered.contains("<b>Specification</b><small>datasheet</small>"),
            "{rendered}"
        );
        assert!(
            !rendered.contains("Typical cost"),
            "a bus has no price band"
        );
        assert_eq!(
            hardware.for_guide("sensors", &catalog()),
            "\n- Hardware: [BME280](https://pamoja.molex.cloud/docs/hardware.html#i2c)"
        );
        assert_eq!(hardware.for_guide("mqtt", &catalog()), "");
        let lonely = bus.replace("interface = \"I2C or SPI\"", "interface = \"none\"");
        let lonely = Hardware::parse(&lonely).expect("parses").table(&catalog());
        assert!(
            lonely.contains("<h4>Find parts</h4><ul class=\"hw-rows\">")
                && !lonely.contains("hw-none"),
            "{lonely}"
        );
        assert!(lonely.contains("<a class=\"hw-row\" href=\"https://www.adafruit.com/search?q=BME280\"><span class=\"hw-main\"><b>Search Adafruit</b><small>for BME280</small></span>"), "{lonely}");
        assert_eq!(query("Modbus RTU over RS-485"), "Modbus+RTU+over+RS-485");
        assert_eq!(query("CAN 2.0 & FD"), "CAN+2.0+%26+FD");
        let with_probe = format!(
            "{}\n[[entry]]\nkey = \"probe\"\ngroup = \"sensors\"\nname = \"Probe\"\nvendor = \"X\"\ninterface = \"I2C at 400 kHz\"\nsummary = \"A probe.\"\nspecs = [\"Range: 1\"]\ncost = \"under $5\"\nsource_name = \"d\"\nsource_label = \"d\"\nsource = \"https://example.invalid/p\"\n",
            bus.replace("name = \"BME280\"", "name = \"I2C\"")
        );
        let with_probe = Hardware::parse(&with_probe)
            .expect("parses")
            .table(&catalog());
        assert!(
            with_probe.contains("<h4>Parts on this bus</h4><ul class=\"hw-rows\"><li><a class=\"hw-row\" href=\"#probe\"><span class=\"hw-main\"><b>Probe</b><small>I2C at 400 kHz</small></span>"),
            "a bus leads to the parts that speak it: {with_probe}"
        );
    }

    const BUY: &str = r#"
[[entry.buy]]
vendor = "Adafruit"
name = "Adafruit BME280 breakout"
url = "https://www.adafruit.com/product/2652"
price = "US$14.95"
checked = "2026-09-06"

[[entry.buy]]
vendor = "Digi-Key"
name = "BME280 bare sensor"
url = "https://www.digikey.com/en/products/detail/bosch/BME280/5341156"
price = "US$5.34"
checked = "2026-09-06"
verified = false
"#;

    #[test]
    fn a_board_page_is_linked_from_the_card_and_must_exist() {
        let with_page = format!(
            "{MINIMAL}page = \"boards/nowhere.md\"
"
        );
        let hardware = Hardware::parse(&with_page).expect("parses");
        let rendered = hardware.table(&catalog());
        assert!(rendered.contains("<li><a class=\"hw-row guide\" href=\"https://pamoja.molex.cloud/docs/boards/nowhere.html\"><span class=\"hw-main\"><b>Board page</b><small>wiring, setup, and the first program</small></span><span class=\"hw-go\" aria-hidden=\"true\">&#8594;</span></a></li>"), "{rendered}");
        let error = hardware.check(&std::env::temp_dir()).unwrap_err();
        assert!(error.contains("board page `boards/nowhere.md`"), "{error}");
        assert!(!Hardware::parse(MINIMAL)
            .expect("parses")
            .table(&catalog())
            .contains("Board page"));
    }

    #[test]
    fn where_to_buy_is_listed_with_the_lowest_price_up_front() {
        let hardware = Hardware::parse(&format!("{MINIMAL}{BUY}")).expect("parses");
        let entry = &hardware.entries[0];
        assert_eq!(entry.buy.len(), 2);
        assert!(entry.buy[0].verified && !entry.buy[1].verified);
        let rendered = hardware.table(&catalog());
        // The price is told once, in the offer list. The cost fact keeps the band alone.
        assert!(
            rendered.contains(
                "<dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd>"
            ),
            "{rendered}"
        );
        assert!(!rendered.contains("the lowest listed price"), "{rendered}");
        assert!(rendered.contains("<div class=\"hw-foot\">\n<section class=\"hw-buy\"><h4>Where to buy</h4><ul class=\"hw-rows\"><li><a class=\"hw-row\" href=\"https://www.adafruit.com/product/2652\"><span class=\"hw-main\"><b>Adafruit</b><small>Adafruit BME280 breakout</small></span><span class=\"hw-price\">US$14.95</span></a></li><li><a class=\"hw-row\" href=\"https://www.digikey.com/en/products/detail/bosch/BME280/5341156\"><span class=\"hw-main\"><b>Digi-Key</b><small>BME280 bare sensor</small><small class=\"hw-note\">listed price; the page refuses scripted readers</small></span><span class=\"hw-price\">US$5.34</span></a></li></ul><p class=\"hw-when\">prices as listed on 2026-09-06</p></section>"), "{rendered}");
    }

    #[test]
    fn an_entry_that_is_not_a_part_carries_no_price_and_names_its_hosts() {
        let firmware = format!("{MINIMAL}hosts = \"Flight controllers that run it\"\n{BUY}");
        let rendered = Hardware::parse(&firmware)
            .expect("parses")
            .table(&catalog());
        assert!(!rendered.contains("Typical cost"), "{rendered}");
        assert!(
            rendered.contains("<h4>Flight controllers that run it</h4>"),
            "{rendered}"
        );
        assert!(!rendered.contains("<h4>Where to buy</h4>"), "{rendered}");
    }

    #[test]
    fn a_part_no_store_lists_says_so_with_the_day_the_stores_were_searched() {
        let searched = format!("{MINIMAL}buy_checked = \"2026-09-06\"\n");
        let rendered = Hardware::parse(&searched)
            .expect("parses")
            .table(&catalog());
        assert!(rendered.contains("<div class=\"hw-foot\">\n<section class=\"hw-buy\"><h4>Find parts</h4><p class=\"hw-none\">No reputable store lists this part as of 2026-09-06."), "{rendered}");
        let plain = Hardware::parse(MINIMAL).expect("parses").table(&catalog());
        assert!(
            plain.contains("<p class=\"hw-none\">No reputable store lists this part. The makers"),
            "{plain}"
        );
        let root = std::env::temp_dir();
        let bad_date = format!("{MINIMAL}buy_checked = \"soon\"\n");
        assert!(Hardware::parse(&bad_date)
            .unwrap()
            .check(&root)
            .unwrap_err()
            .contains("YYYY-MM-DD"));
        let both = format!("{MINIMAL}buy_checked = \"2026-09-06\"\n{BUY}");
        assert!(Hardware::parse(&both)
            .unwrap()
            .check(&root)
            .unwrap_err()
            .contains("empty buy list"));
    }

    #[test]
    fn a_buy_line_needs_an_https_page_a_price_and_a_date() {
        let root = std::env::temp_dir();
        for (bad, message) in [
            (BUY.replace("https://www.adafruit.com", "http://www.adafruit.com"), "must be an https URL"),
            (BUY.replace("price = \"US$14.95\"", "price = \"\""), "has no price"),
            (BUY.replace("checked = \"2026-09-06\"\n\n", "checked = \"6 Sep 2026\"\n\n"), "YYYY-MM-DD"),
            (format!("{BUY}\n[[entry.buy]]\nvendor = \"Again\"\nname = \"Same page\"\nurl = \"https://www.adafruit.com/product/2652\"\nprice = \"US$1\"\nchecked = \"2026-09-06\"\n"), "twice"),
        ] {
            let hardware = Hardware::parse(&format!("{MINIMAL}{bad}")).expect("parses");
            let err = hardware.check(&root).expect_err("refused");
            assert!(err.contains(message), "{err}");
        }
        let priceless = format!(
            "{}{BUY}",
            MINIMAL.replace("cost = \"$5 to $20\"", "cost = \"not applicable\"")
        );
        let err = Hardware::parse(&priceless)
            .expect("parses")
            .check(&root)
            .expect_err("refused");
        assert!(err.contains("has no price"), "{err}");
        assert!(is_date("2026-09-06") && !is_date("2026-9-6") && !is_date("2026-09-06x"));
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
