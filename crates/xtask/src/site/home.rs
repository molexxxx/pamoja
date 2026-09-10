//! The front page, rendered from the capability map and `web/home.toml`.
//!
//! Everything a visitor sees first, laid out as the first pages of a datasheet: what pamoja
//! is and how to install it, the same first example in four languages spliced from the
//! tests that run it, every capability as a row of one table drawn from the capability
//! map, nine scenarios played by the consoles in `web/js/consoles.js` as figures, the four
//! languages, where the project is going, and how backing will open. The copy that is not derived from the code lives in `web/home.toml`, and the checks
//! here keep it honest: a scenario must name library crates and have a console to play
//! it, and a roadmap tag that names a crate must agree with the workspace about whether
//! that crate ships.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use toml_edit::{DocumentMut, Item};

use crate::catalog::{Capability, Catalog, LANGUAGES};
use crate::regions;

use super::highlight::{self, escape};

/// The front page's data, as `web/home.toml` holds it.
pub struct Home {
    hero: Hero,
    scenarios: Vec<Scenario>,
    tracks: Vec<Track>,
    backing: Backing,
    milestones: Vec<Milestone>,
}

struct Hero {
    eyebrow: String,
    title: Vec<String>,
    lead: String,
}

struct Scenario {
    key: String,
    group: String,
    tab: String,
    eyebrow: String,
    title: String,
    body: String,
    crates: Vec<String>,
    readings: Vec<Reading>,
}

/// One line of a scenario's opening state, rendered into the figure so it reads before
/// the console takes it over.
struct Reading {
    label: String,
    value: String,
    unit: String,
}

struct Track {
    title: String,
    lead: String,
    tags: Vec<Tag>,
}

struct Tag {
    text: String,
    ships: bool,
    krate: Option<String>,
}

struct Backing {
    lead: String,
    note_title: String,
    note: String,
    offers: Vec<Offer>,
    opens: String,
    rungs: Vec<Rung>,
}

/// One thing backing will open, and whether it is open.
struct Offer {
    name: String,
    state: String,
}

/// One rung of the ladder a message climbs, in the order the ladder tries them.
struct Rung {
    name: String,
    cost: String,
    detail: String,
}

struct Milestone {
    state: String,
    title: String,
    detail: String,
}

/// The four first examples, one per language, as (tab label, panel id, file, anchor).
const QUICKSTARTS: [(&str, &str, &str); 4] = [
    ("Rust", "rust", "examples/tests/guides/quickstart.rs"),
    (
        "TypeScript",
        "typescript",
        "bindings/node/guides/quickstart.ts",
    ),
    ("Python", "python", "bindings/python/guides/quickstart.py"),
    (
        "C#",
        "c",
        "bindings/dotnet/samples/Pamoja.Guides/Quickstart.cs",
    ),
];

impl Home {
    /// Read `web/home.toml` under `root`.
    ///
    /// # Arguments
    ///
    /// * `root` - the repository root.
    ///
    /// # Returns
    ///
    /// The front page's data.
    ///
    /// # Errors
    ///
    /// When the file is missing or a field is absent or of the wrong type.
    pub fn load(root: &Path) -> Result<Home, String> {
        let path = root.join("web/home.toml");
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        Home::parse(&text)
    }

    /// Parse the front page's data from its TOML text.
    ///
    /// # Errors
    ///
    /// When a field is absent or of the wrong type.
    pub fn parse(text: &str) -> Result<Home, String> {
        let doc: DocumentMut = text
            .parse()
            .map_err(|err| format!("home.toml is not valid TOML: {err}"))?;
        let hero = doc
            .get("hero")
            .and_then(Item::as_table_like)
            .ok_or("home.toml has no [hero]")?;
        let backing = doc
            .get("backing")
            .and_then(Item::as_table_like)
            .ok_or("home.toml has no [backing]")?;

        let mut scenarios = Vec::new();
        for table in tables(&doc, "scenario")? {
            let key = string(table, "key", "scenario")?;
            let at = format!("scenario {key}");
            scenarios.push(Scenario {
                group: string(table, "group", &at)?,
                tab: string(table, "tab", &at)?,
                eyebrow: string(table, "eyebrow", &at)?,
                title: string(table, "title", &at)?,
                body: string(table, "body", &at)?,
                crates: strings(table, "crates", &at)?,
                readings: readings(table, &at)?,
                key,
            });
        }

        let mut tracks = Vec::new();
        for table in tables(&doc, "track")? {
            let key = string(table, "key", "track")?;
            let at = format!("track {key}");
            let tags = table
                .get("tags")
                .and_then(Item::as_array)
                .ok_or_else(|| format!("{at}: `tags` must be an array"))?
                .iter()
                .map(|value| {
                    let tag = value
                        .as_inline_table()
                        .ok_or_else(|| format!("{at}: every tag is an inline table"))?;
                    Ok(Tag {
                        text: tag
                            .get("text")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| format!("{at}: a tag needs `text`"))?
                            .to_owned(),
                        ships: tag
                            .get("ships")
                            .and_then(|v| v.as_bool())
                            .ok_or_else(|| format!("{at}: a tag needs `ships`"))?,
                        krate: tag.get("crate").and_then(|v| v.as_str()).map(str::to_owned),
                    })
                })
                .collect::<Result<Vec<Tag>, String>>()?;
            tracks.push(Track {
                title: string(table, "title", &at)?,
                lead: string(table, "lead", &at)?,
                tags,
            });
        }

        let mut milestones = Vec::new();
        for table in tables(&doc, "milestone")? {
            let title = string(table, "title", "milestone")?;
            let at = format!("milestone {title}");
            let state = string(table, "state", &at)?;
            if !matches!(state.as_str(), "now" | "next" | "later") {
                return Err(format!("{at}: `state` must be now, next, or later"));
            }
            milestones.push(Milestone {
                state,
                detail: string(table, "detail", &at)?,
                title,
            });
        }

        Ok(Home {
            hero: Hero {
                eyebrow: string(hero, "eyebrow", "hero")?,
                title: strings(hero, "title", "hero")?,
                lead: string(hero, "lead", "hero")?,
            },
            scenarios,
            tracks,
            milestones,
            backing: Backing {
                lead: string(backing, "lead", "backing")?,
                note_title: string(backing, "note_title", "backing")?,
                note: string(backing, "note", "backing")?,
                offers: rows(backing, "offers", &["name", "state"], "backing")?
                    .into_iter()
                    .map(|mut row| Offer {
                        state: row.remove(1),
                        name: row.remove(0),
                    })
                    .collect(),
                opens: string(backing, "opens", "backing")?,
                rungs: rows(backing, "rungs", &["name", "cost", "detail"], "backing")?
                    .into_iter()
                    .map(|mut row| Rung {
                        detail: row.remove(2),
                        cost: row.remove(1),
                        name: row.remove(0),
                    })
                    .collect(),
            },
        })
    }

    /// Check the data against the workspace and the consoles.
    ///
    /// # Arguments
    ///
    /// * `lib_crates` - every library crate in the workspace.
    /// * `consoles` - the source of `web/js/consoles.js`, whose specs play the scenarios.
    ///
    /// # Errors
    ///
    /// Every disagreement, one per line: a scenario naming a crate that does not exist or
    /// having no console, or a roadmap tag whose crate disagrees with the workspace.
    pub fn check(&self, lib_crates: &[String], consoles: &str) -> Result<(), String> {
        let mut problems = Vec::new();
        let is_crate = |name: &str| lib_crates.iter().any(|known| known == name);
        for scenario in &self.scenarios {
            if !consoles.contains(&format!("\n  {}: {{", scenario.key)) {
                problems.push(format!(
                    "scenario {} has no console in web/js/consoles.js",
                    scenario.key
                ));
            }
            if scenario.group != "field" && scenario.group != "robotics" {
                problems.push(format!(
                    "scenario {} has the unknown group {}",
                    scenario.key, scenario.group
                ));
            }
            for krate in &scenario.crates {
                if !is_crate(krate) {
                    problems.push(format!(
                        "scenario {} names {krate}, which is not a library crate",
                        scenario.key
                    ));
                }
            }
        }
        for track in &self.tracks {
            for tag in &track.tags {
                let Some(krate) = &tag.krate else {
                    continue;
                };
                match (tag.ships, is_crate(krate)) {
                    (true, false) => problems.push(format!(
                        "track {}: {} is marked as shipping but {krate} is not a library crate",
                        track.title, tag.text
                    )),
                    (false, true) => problems.push(format!(
                        "track {}: {} is marked as planned but {krate} ships",
                        track.title, tag.text
                    )),
                    _ => {}
                }
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "web/home.toml disagrees with the repository:\n  {}",
                problems.join("\n  ")
            ))
        }
    }

    /// The keys of the scenarios, in order.
    #[cfg(test)]
    pub fn scenario_keys(&self) -> Vec<&str> {
        self.scenarios
            .iter()
            .map(|scenario| scenario.key.as_str())
            .collect()
    }

    /// Render the page body.
    ///
    /// # Arguments
    ///
    /// * `root` - the repository root, for the first examples spliced from the tests.
    /// * `catalog` - the capability map.
    /// * `lib_crates` - every library crate, for the numbers.
    /// * `descriptions` - each crate's one-line description, for the engine rows.
    ///
    /// # Returns
    ///
    /// The `<main>` element and everything in it: the sheet's front, then its numbered
    /// sections.
    ///
    /// # Errors
    ///
    /// When a first example cannot be read or its anchor is missing.
    pub fn render(
        &self,
        root: &Path,
        catalog: &Catalog,
        lib_crates: &[String],
        descriptions: &BTreeMap<String, String>,
    ) -> Result<String, String> {
        let mut out = String::from("<main class=\"home sheet\" id=\"content\">\n");
        out.push_str(&self.front(catalog, lib_crates));
        out.push_str(&quickstart(root)?);
        out.push_str(&covers(catalog, descriptions));
        out.push_str(&self.runs());
        out.push_str(&reach(catalog));
        out.push_str(&self.roadmap());
        out.push_str(&self.backing());
        out.push_str("</main>\n");
        Ok(out)
    }

    // The sheet's first page: the title, the description, the features and specifications
    // columns, and beside them the typical application figure, the ordering table, and
    // the doors into the documentation.
    fn front(&self, catalog: &Catalog, lib_crates: &[String]) -> String {
        let title: Vec<String> = self.hero.title.iter().map(|line| escape(line)).collect();
        let mut what = escape(&self.hero.eyebrow);
        if let Some(first) = what.get(..1) {
            what.replace_range(..1, &first.to_uppercase());
        }
        let guides = catalog
            .capabilities
            .iter()
            .filter(|capability| capability.guide.is_some())
            .count();
        let stage = self.scenarios.first();
        let stage_key = stage
            .map(|scenario| scenario.key.as_str())
            .unwrap_or("farm");
        let stage_name = stage
            .map(|scenario| scenario.tab.to_lowercase())
            .unwrap_or_else(|| "farm".to_owned());
        let stage_still = stage.map(still).unwrap_or_default();
        let orders: String = LANGUAGES
            .iter()
            .map(|language| {
                format!(
                    "<tr data-lang=\"{}\"><th scope=\"row\" data-label=\"Language\"><button class=\"pin\" type=\"button\" data-lang=\"{}\" aria-label=\"Show every listing in {}\">{}</button></th><td data-label=\"Install\">{}</td></tr>\n",
                    language.key,
                    language.key,
                    language.name,
                    language.name,
                    command(&language.install(language.bundle()))
                )
            })
            .collect();
        format!(
            "<section class=\"front\" aria-labelledby=\"sheet-title\">\n\
             <div class=\"front-id\">\n\
             <h1 class=\"sheet-title\" id=\"sheet-title\">{}</h1>\n\
             <p class=\"sheet-desc\"><strong>{what}.</strong> {}</p>\n\
             </div>\n\
             <div class=\"front-fig\">\n\
             <figure class=\"fig fig-lead\" id=\"figure-1\">\n\
             <div class=\"diorama\" data-diorama=\"{stage_key}\">{stage_still}</div>\n\
             <p class=\"fig-note\">Scripted illustration</p>\n\
             <figcaption><b>Figure 1.</b> Typical application: a {stage_name} node. The readings are scripted, not measured. <a href=\"https://pamoja.molex.cloud/dashboard/\">Open the dashboard demo</a></figcaption>\n\
             </figure>\n\
             <div class=\"tbl\" id=\"table-1\">\n\
             <p class=\"tbl-caption\"><b>Table 1.</b> Ordering information. The language pin sets every listing on this site.</p>\n\
             <table class=\"order\">\n\
             <thead><tr><th scope=\"col\">Language</th><th scope=\"col\">Install</th></tr></thead>\n\
             <tbody>\n\
{orders}</tbody>\n\
             </table>\n\
             </div>\n\
             <p class=\"front-actions\"><a class=\"action\" href=\"/docs/index.html\">Get started</a><a class=\"action ghost\" href=\"/docs/reference/index.html\">API reference</a></p>\n\
             </div>\n\
             <div class=\"front-cols\">\n\
             <section class=\"sec\" aria-labelledby=\"features-title\">\n\
             <h2 id=\"features-title\"><span class=\"num\">1</span>Features</h2>\n\
             <ul class=\"features\">\n\
             <li>{} capabilities, each a crate in Rust and a package in TypeScript, Python, and C#</li>\n\
             <li>{} crates over one core, most of them <code>no_std</code>, every one cross-compiled for a Cortex-M4F in CI</li>\n\
             <li>{guides} guides, each showing the same example in four languages, spliced from the tests that run it</li>\n\
             <li>Offline first: store and forward, compact codecs, LoRa, LoRaWAN, and mesh as first-class links</li>\n\
             <li>Device identity, a secured session, signed updates with rollback, and a tamper-evident log</li>\n\
             <li>Builds and tests with nothing plugged in: a loopback link and simulated hardware stand in</li>\n\
             <li>MIT licensed, published in lockstep on crates.io, npm, PyPI, and NuGet</li>\n\
             </ul>\n\
             </section>\n\
             <section class=\"sec\" aria-labelledby=\"specs-title\">\n\
             <h2 id=\"specs-title\"><span class=\"num\">2</span>Specifications</h2>\n\
             <table class=\"specs\">\n\
             <tbody>\n\
             <tr><th scope=\"row\">Languages</th><td>Rust, TypeScript, Python, C#</td></tr>\n\
             <tr><th scope=\"row\">Capabilities</th><td>{}</td></tr>\n\
             <tr><th scope=\"row\">Crates in the workspace</th><td>{}</td></tr>\n\
             <tr><th scope=\"row\">Guides</th><td>{guides}, each in four languages</td></tr>\n\
             <tr><th scope=\"row\">Smallest target</th><td>Cortex-M4F, <code>no_std</code></td></tr>\n\
             <tr><th scope=\"row\">Third-party code</th><td>None in a one-capability Rust build</td></tr>\n\
             <tr><th scope=\"row\">Registries</th><td>crates.io, npm, PyPI, NuGet</td></tr>\n\
             <tr><th scope=\"row\">License</th><td>MIT</td></tr>\n\
             </tbody>\n\
             </table>\n\
             </section>\n\
             </div>\n\
             </section>\n",
            title.join("<br>"),
            escape(&self.hero.lead),
            catalog.capabilities.len(),
            lib_crates.len(),
            catalog.capabilities.len(),
            lib_crates.len(),
        )
    }

    // The typical applications: the scenario tabs, and each scenario as a figure with the
    // console that plays it, its place, its story, and the crates it names.
    fn runs(&self) -> String {
        let mut out = String::from(
            "<section class=\"sec sec-runs\" id=\"runs\" aria-labelledby=\"runs-title\">\n\
             <h2 id=\"runs-title\"><span class=\"num\">5</span>Typical applications</h2>\n\
             <p class=\"sec-lead\">Each figure is a scripted illustration of a node doing its job: the readings are drawn in the browser, not measured, and none of the nine is a deployment. Six are field nodes; three are robots. What the crates behind them actually do is in the guides, where every example runs in CI.</p>\n\
             <div class=\"stage-tabs\" role=\"tablist\" aria-label=\"Scenario\">\n",
        );
        for (index, scenario) in self.scenarios.iter().enumerate() {
            out.push_str(&format!(
                "<button class=\"stage-tab\" role=\"tab\" type=\"button\" id=\"tab-{key}\" aria-controls=\"scene-{key}\" aria-selected=\"{}\" data-scene=\"{key}\" data-group=\"{}\">{}</button>\n",
                index == 0,
                escape(&scenario.group),
                escape(&scenario.tab),
                key = escape(&scenario.key),
            ));
        }
        out.push_str("</div>\n<div class=\"stage\">\n");
        for (index, scenario) in self.scenarios.iter().enumerate() {
            let chips: String = scenario
                .crates
                .iter()
                .map(|krate| {
                    format!(
                        "<a class=\"crate\" href=\"docs/reference/rust/{}/index.html\"><code>{}</code></a>",
                        krate.replace('-', "_"),
                        escape(krate)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "<article class=\"scene\" id=\"scene-{key}\" role=\"tabpanel\" aria-labelledby=\"tab-{key}\"{}>\n\
                 <figure class=\"fig\">\n\
                 <div class=\"diorama\" data-diorama=\"{key}\">{}</div>\n\
                 <figcaption><b>Figure 5-{}.</b> {}</figcaption>\n\
                 </figure>\n\
                 <div class=\"scene-text\">\n\
                 <h3>{}</h3>\n\
                 <p class=\"scene-where\">{}</p>\n\
                 <p>{}</p>\n\
                 <p class=\"scene-crates\">Crates: {chips}</p>\n\
                 </div>\n\
                 </article>\n",
                if index == 0 { "" } else { " hidden" },
                still(scenario),
                index + 1,
                escape(&scenario.title),
                escape(&scenario.title),
                escape(&scenario.eyebrow),
                scenario.body,
                key = escape(&scenario.key),
            ));
        }
        out.push_str("</div>\n</section>\n");
        out
    }

    // Where the project is going, as a table: what ships in each track and what is committed.
    fn roadmap(&self) -> String {
        let mut out = String::from(
            "<section class=\"sec\" id=\"roadmap\" aria-labelledby=\"roadmap-title\">\n\
             <h2 id=\"roadmap-title\"><span class=\"num\">7</span>Direction</h2>\n\
             <p class=\"sec-lead\">Not a sensor library: a platform for physical things. What ships today is in the crates named above; the committed direction is beside it.</p>\n\
             <div class=\"tbl\" id=\"table-7-1\">\n\
             <p class=\"tbl-caption\"><b>Table 7-1.</b> Tracks, what ships, and what is committed</p>\n\
             <table class=\"tracks\">\n\
             <thead><tr><th scope=\"col\">Track</th><th scope=\"col\">Ships today</th><th scope=\"col\">Committed</th></tr></thead>\n\
             <tbody>\n",
        );
        for track in &self.tracks {
            let list = |ships: bool| -> String {
                track
                    .tags
                    .iter()
                    .filter(|tag| tag.ships == ships)
                    .map(|tag| format!("<span class=\"tag\">{}</span>", escape(&tag.text)))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            out.push_str(&format!(
                "<tr><th scope=\"row\" data-label=\"Track\"><b>{}</b><span class=\"what\">{}</span></th><td data-label=\"Ships today\">{}</td><td data-label=\"Committed\">{}</td></tr>\n",
                escape(&track.title),
                escape(&track.lead),
                list(true),
                list(false),
            ));
        }
        out.push_str("</tbody>\n</table>\n</div>\n</section>\n");
        out
    }

    // How the project will be backed, as prose and a milestone table.
    fn backing(&self) -> String {
        let b = &self.backing;
        let milestones: String = self
            .milestones
            .iter()
            .map(|milestone| {
                format!(
                    "<tr data-state=\"{}\"><th scope=\"row\" data-label=\"State\"><span class=\"state\">{}</span></th><td data-label=\"Milestone\"><b>{}</b></td><td data-label=\"Detail\">{}</td></tr>\n",
                    escape(&milestone.state),
                    escape(&milestone.state),
                    escape(&milestone.title),
                    escape(&milestone.detail)
                )
            })
            .collect();
        let offers: String = b
            .offers
            .iter()
            .map(|offer| {
                format!(
                    "<tr><th scope=\"row\">{}</th><td><span class=\"shut\">{}</span></td></tr>\n",
                    escape(&offer.name),
                    escape(&offer.state)
                )
            })
            .collect();
        // The ladder is drawn cheapest at the foot, which is the rung a message tries first;
        // the list keeps that order and the stylesheet stacks it upward.
        let rungs: String = b
            .rungs
            .iter()
            .map(|rung| {
                format!(
                    "<li><span class=\"rung-cost\">{}</span><b>{}</b><span class=\"rung-what\">{}</span></li>\n",
                    escape(&rung.cost),
                    escape(&rung.name),
                    escape(&rung.detail)
                )
            })
            .collect();
        format!(
            "<section class=\"sec\" id=\"back\" aria-labelledby=\"back-title\">\n\
             <h2 id=\"back-title\"><span class=\"num\">8</span>Backing<span class=\"stamp\">Not open</span></h2>\n\
             <div class=\"back-open\">\n\
             <p class=\"sec-lead\">{}</p>\n\
             <div class=\"tbl\" id=\"table-8-1\">\n\
             <p class=\"tbl-caption\"><b>Table 8-1.</b> What backing will open</p>\n\
             <table class=\"offers\">\n<tbody>\n{offers}</tbody>\n</table>\n\
             <p class=\"tbl-note\">{}</p>\n\
             </div>\n\
             </div>\n\
             <div class=\"tbl\" id=\"table-8-2\">\n\
             <p class=\"tbl-caption\"><b>Table 8-2.</b> How it opens, in order</p>\n\
             <table class=\"milestones\">\n\
             <thead><tr><th scope=\"col\">State</th><th scope=\"col\">Milestone</th><th scope=\"col\">Detail</th></tr></thead>\n\
             <tbody>\n{milestones}</tbody>\n\
             </table>\n\
             </div>\n\
             <div class=\"back-cost\">\n\
             <div class=\"back-why\">\n\
             <h3 id=\"paid-for\">8.1 {}</h3>\n\
             <p class=\"prose\">{}</p>\n\
             </div>\n\
             <figure class=\"fig fig-ladder\" id=\"figure-8-1\">\n\
             <ol class=\"ladder\">\n{rungs}</ol>\n\
             <figcaption><b>Figure 8-1.</b> The ladder a message climbs, cheapest rung first. A rung is reached only when every rung under it is gone, which is what keeps the dear ones rare.</figcaption>\n\
             </figure>\n\
             </div>\n\
             </section>\n",
            escape(&b.lead),
            escape(&b.opens),
            escape(&b.note_title),
            b.note,
        )
    }
}

// The first example in four languages, spliced from the tests that run it, in the same
// tab block the guides use, so the language a reader chose there is chosen here too. Each
// panel opens capped to a few lines of code, with the whole example one click away.
fn quickstart(root: &Path) -> Result<String, String> {
    let mut tabs = String::new();
    let mut panels = String::new();
    for (label, id, path) in QUICKSTARTS {
        let source =
            fs::read_to_string(root.join(path)).map_err(|err| format!("reading {path}: {err}"))?;
        let code = regions::extract(&source, "example")
            .ok_or_else(|| format!("{path} has no `ANCHOR: example` region"))?;
        let lang = match id {
            "rust" => "rust",
            "typescript" => "typescript",
            "python" => "python",
            _ => "csharp",
        };
        tabs.push_str(&format!(
            "<button class=\"lang-tab\" role=\"tab\" type=\"button\" id=\"quick-tab-{id}\" aria-controls=\"quick-{id}\" aria-selected=\"false\" data-lang=\"{id}\">{label}</button>\n"
        ));
        panels.push_str(&format!(
            "<section class=\"lang-panel capped\" id=\"quick-{id}\" role=\"tabpanel\" aria-labelledby=\"quick-tab-{id}\" data-lang=\"{id}\" tabindex=\"0\">\n\
             <p class=\"source\">Listing 3-1, {label}. From <a href=\"https://github.com/molexxxx/pamoja/blob/main/{path}\"><code>{path}</code></a>, which runs in CI.</p>\n\
             <figure class=\"code\" data-lang=\"{lang}\"><figcaption><span class=\"code-lang\">{label}</span><button class=\"reveal\" type=\"button\" aria-expanded=\"false\" aria-controls=\"quick-{id}\">Whole listing</button><button class=\"copy\" type=\"button\" aria-label=\"Copy this code\">copy</button></figcaption><pre><code>{}</code></pre></figure>\n\
             </section>\n",
            highlight::highlight(&code, lang)
        ));
    }
    Ok(format!(
        "<section class=\"sec quick\" id=\"quick\" aria-labelledby=\"quick-title\">\n\
         <h2 id=\"quick-title\"><span class=\"num\">3</span>Quickstart</h2>\n\
         <p class=\"sec-lead\">The same program in the language you already work in: a reading taken off a wire on a field node, sent over a link, and checked on the gateway that receives it, with nothing plugged in and nothing running.</p>\n\
         <div class=\"langs\">\n<div class=\"lang-tabs\" role=\"tablist\" aria-label=\"Language\">\n{tabs}</div>\n{panels}</div>\n\
         </section>\n"
    ))
}

// Every capability as a row of one table, grouped by chapter, with the package in each
// language and the guide; the engine crates open the table.
fn covers(catalog: &Catalog, descriptions: &BTreeMap<String, String>) -> String {
    let engine: String = catalog
        .engine
        .iter()
        .map(|krate| {
            format!(
                "<a href=\"https://crates.io/crates/{krate}\"><code>{}</code></a>",
                escape(krate)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let engine_what = catalog
        .engine
        .first()
        .and_then(|krate| descriptions.get(krate).cloned())
        .map(|what| what.trim_end_matches('.').to_owned())
        .unwrap_or_default();
    let mut cards = format!(
        "<article class=\"chapter chapter-engine\">\n\
         <p class=\"chapter-id\">4.1</p>\n\
         <h3><a href=\"docs/reference/rust.html\">Engine</a></h3>\n\
         <p class=\"chapter-what\">{}</p>\n\
         <p class=\"chapter-n\">{engine}</p>\n\
         </article>\n",
        escape(&engine_what)
    );
    for (index, chapter) in catalog.chapters.iter().enumerate() {
        let members: Vec<&Capability> = catalog.in_chapter(&chapter.key).collect();
        let count = members.len();
        // A heading opens the first guide under it, which is where a reader who wants the
        // heading itself starts; the sidebar then holds the rest of that heading's guides.
        let href = members
            .first()
            .and_then(|capability| capability.guide.as_ref())
            .map(|guide| format!("docs/{}.html", guide.strip_suffix(".md").unwrap_or(guide)))
            .unwrap_or_else(|| "docs/reference/index.html".to_owned());
        cards.push_str(&format!(
            "<article class=\"chapter\">\n\
             <p class=\"chapter-id\">4.{}</p>\n\
             <h3><a href=\"{href}\">{}</a></h3>\n\
             <p class=\"chapter-what\">{}</p>\n\
             <p class=\"chapter-n\">{count} {}</p>\n\
             </article>\n",
            index + 2,
            escape(&chapter.title),
            escape(&chapter.intent),
            if count == 1 {
                "capability"
            } else {
                "capabilities"
            }
        ));
    }

    format!(
        "<section class=\"sec\" id=\"covers\" aria-labelledby=\"covers-title\">\n\
         <h2 id=\"covers-title\"><span class=\"num\">4</span>Capabilities</h2>\n\
         <p class=\"sec-lead\">Every capability is a crate in Rust and a package in each binding, behind the traits in <code>pamoja-core</code>. On a microcontroller you bring in two crates and nothing else.</p>\n\
         <div class=\"tbl\" id=\"table-4-1\">\n\
         <p class=\"tbl-caption\"><b>Table 4-1.</b> The engine, then {} capabilities under {} headings. A heading that holds more than one is also one thing to install.</p>\n\
         <div class=\"chapters\">\n{cards}</div>\n\
         </div>\n\
         <p class=\"sec-note\">Every capability, with its package on crates.io, npm, PyPI, and NuGet and its API pages in all four languages, is on the <a href=\"docs/reference/index.html\">reference</a>. How a call reaches a crate, from a binding down through the engine, is drawn on the <a href=\"docs/about/architecture.html\">architecture</a> page.</p>\n\
         </section>\n",
        catalog.capabilities.len(),
        catalog.chapters.len()
    )
}

// The four languages as a table: the install line, the reference, and the guides.
fn reach(catalog: &Catalog) -> String {
    let guides = catalog
        .capabilities
        .iter()
        .filter(|capability| capability.guide.is_some())
        .count();
    let mut out = String::from(
        "<section class=\"sec sec-turn\" id=\"reach\" aria-labelledby=\"reach-title\">\n\
         <h2 id=\"reach-title\"><span class=\"num\">6</span>Language bindings</h2>\n\
         <p class=\"sec-lead\">One memory-safe engine, idiomatic bindings on top. Every capability is a package in each, and every guide shows the same task in all four. The install line for each is in Table 1.</p>\n\
         <div class=\"tbl\" id=\"table-6-1\">\n\
         <p class=\"tbl-caption\"><b>Table 6-1.</b> Bindings, their reference, and their guides</p>\n\
         <table class=\"bindings\">\n\
         <thead><tr><th scope=\"col\">Language</th><th scope=\"col\">Reference</th><th scope=\"col\">Guides</th></tr></thead>\n\
         <tbody>\n",
    );
    for language in &LANGUAGES {
        out.push_str(&format!(
            "<tr><th scope=\"row\" data-label=\"Language\"><b>{}</b></th><td data-label=\"Reference\"><a href=\"docs/reference/{}.html\">{} reference</a><span class=\"what\">Every {} with its API pages, generated by {}</span></td><td data-label=\"Guides\"><a href=\"docs/index.html\">{guides} guides</a></td></tr>\n",
            language.name,
            language.key,
            language.name,
            language.unit(),
            language.generator(),
        ));
    }
    out.push_str("</tbody>\n</table>\n</div>\n</section>\n");
    out
}

// A scenario's opening state as a table, so the figure reads before any script runs and
// a reader on a link that never delivers one still sees the node doing its job.
fn still(scenario: &Scenario) -> String {
    if scenario.readings.is_empty() {
        return format!(
            "<p class=\"diorama-still\">The {} console plays here in a browser that runs scripts.</p>",
            escape(&scenario.tab.to_lowercase())
        );
    }
    let rows: String = scenario
        .readings
        .iter()
        .map(|reading| {
            format!(
                "<tr><th scope=\"row\">{}</th><td>{}<span class=\"unit\">{}</span></td></tr>",
                escape(&reading.label),
                escape(&reading.value),
                escape(&reading.unit)
            )
        })
        .collect();
    format!("<table class=\"still\"><caption>Opening state</caption><tbody>{rows}</tbody></table>")
}

// An install line with the button that copies it, the same shape the reference pages use.
fn command(text: &str) -> String {
    let text = escape(text);
    format!(
        "<span class=\"pkg-get\"><code class=\"cmd\">{text}</code><button class=\"copy\" type=\"button\" data-copy=\"{text}\" aria-label=\"Copy the install command\">copy</button></span>"
    )
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

fn string(table: &dyn toml_edit::TableLike, key: &str, at: &str) -> Result<String, String> {
    table
        .get(key)
        .and_then(Item::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}: `{key}` must be a string"))
}

// A scenario's opening readings, when it declares any.
fn readings(table: &dyn toml_edit::TableLike, at: &str) -> Result<Vec<Reading>, String> {
    let Some(array) = table.get("readings").and_then(Item::as_array) else {
        return Ok(Vec::new());
    };
    array
        .iter()
        .map(|value| {
            let row = value
                .as_inline_table()
                .ok_or_else(|| format!("{at}: every reading is an inline table"))?;
            let field = |name: &str| -> Result<String, String> {
                row.get(name)
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{at}: a reading needs `{name}`"))
            };
            Ok(Reading {
                label: field("label")?,
                value: field("value")?,
                unit: field("unit")?,
            })
        })
        .collect()
}

// An array of inline tables, each read for the same named string fields, in field order.
fn rows(
    table: &dyn toml_edit::TableLike,
    key: &str,
    fields: &[&str],
    at: &str,
) -> Result<Vec<Vec<String>>, String> {
    let array = table
        .get(key)
        .and_then(Item::as_array)
        .ok_or_else(|| format!("{at}: `{key}` must be an array"))?;
    array
        .iter()
        .map(|value| {
            let row = value
                .as_inline_table()
                .ok_or_else(|| format!("{at}: every {key} entry is an inline table"))?;
            fields
                .iter()
                .map(|field| {
                    row.get(field)
                        .and_then(|v| v.as_str())
                        .map(str::to_owned)
                        .ok_or_else(|| format!("{at}: a {key} entry needs `{field}`"))
                })
                .collect()
        })
        .collect()
}

fn strings(table: &dyn toml_edit::TableLike, key: &str, at: &str) -> Result<Vec<String>, String> {
    let array = table
        .get(key)
        .and_then(Item::as_array)
        .ok_or_else(|| format!("{at}: `{key}` must be an array of strings"))?;
    array
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{at}: `{key}` must hold only strings"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[hero]
eyebrow = "an open SDK"
title = ["One core.", "Every language.", "For the devices that", "change lives."]
lead = "It runs on cheap hardware."

[[scenario]]
key = "farm"
group = "field"
tab = "Farm"
eyebrow = "Farms"
title = "Water when asked."
body = "A controller reads <code>soil</code> probes."
crates = ["pamoja-modbus"]

[[track]]
key = "radio"
title = "Radio"
lead = "The cheapest link first."
tags = [
  { text = "MQTT", ships = true, crate = "pamoja-mqtt" },
  { text = "satellite", ships = false, crate = "pamoja-satellite" },
]

[backing]
lead = "Free software; hardware costs."
note_title = "How the link gets paid for"
note = "By an NGO, <em>never</em> the family."
offers = [{ name = "Kits", state = "Not open" }]
opens = "Once the pilot has run."
rungs = [{ name = "Neighbour mesh", cost = "Free", detail = "A hop next door." }]

[[milestone]]
state = "now"
title = "Design the kit"
detail = "With partners."
"#;

    const CONSOLES: &str = "const SPECS = {\n  farm: {\n    id: 'x',\n  },\n};\n";

    #[test]
    fn parses_and_checks_the_data() {
        let home = Home::parse(SAMPLE).unwrap();
        assert_eq!(home.scenario_keys(), ["farm"]);
        assert_eq!(home.milestones[0].state, "now");
        assert_eq!(home.backing.note_title, "How the link gets paid for");
        assert_eq!(home.backing.offers[0].state, "Not open");
        assert_eq!(home.backing.rungs[0].cost, "Free");
        let crates = ["pamoja-modbus".to_owned(), "pamoja-mqtt".to_owned()];
        home.check(&crates, CONSOLES).unwrap();

        let err = home
            .check(&["pamoja-mqtt".to_owned()], CONSOLES)
            .unwrap_err();
        assert!(err.contains("scenario farm names pamoja-modbus, which is not a library crate"));

        let err = home.check(&crates, "const SPECS = {\n};\n").unwrap_err();
        assert!(err.contains("scenario farm has no console"));

        let shipped_satellite = [
            "pamoja-modbus".to_owned(),
            "pamoja-mqtt".to_owned(),
            "pamoja-satellite".to_owned(),
        ];
        let err = home.check(&shipped_satellite, CONSOLES).unwrap_err();
        assert!(err.contains("satellite is marked as planned but pamoja-satellite ships"));
    }

    #[test]
    fn a_milestone_state_must_be_one_of_three() {
        let err = Home::parse(&SAMPLE.replace("state = \"now\"", "state = \"soon\""))
            .err()
            .expect("an unknown state is an error");
        assert!(err.contains("must be now, next, or later"), "{err}");
    }

    #[test]
    fn a_missing_field_is_named() {
        let err = Home::parse(&SAMPLE.replace("tab = \"Farm\"\n", ""))
            .err()
            .expect("a missing field is an error");
        assert!(
            err.contains("scenario farm: `tab` must be a string"),
            "{err}"
        );
    }
}
