//! The front page, rendered from the capability map and `web/home.toml`.
//!
//! Everything a visitor sees first: what pamoja is and how to install it, the same first
//! example in four languages spliced from the tests that run it, every capability as a
//! wall of cards drawn from the capability map, nine scenarios played by the consoles in
//! `web/js/consoles.js`, the four languages, where the project is going, and how to back
//! it. The copy that is not derived from the code lives in `web/home.toml`, and the checks
//! here keep it honest: a scenario must name library crates and have a console to play
//! it, and a roadmap tag that names a crate must agree with the workspace about whether
//! that crate ships.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use toml_edit::{DocumentMut, Item};

use crate::catalog::{Capability, Catalog, Language, LANGUAGES};
use crate::regions;

use super::highlight::{self, escape};

/// The front page's data, as `web/home.toml` holds it.
pub struct Home {
    hero: Hero,
    scenarios: Vec<Scenario>,
    tracks: Vec<Track>,
    backing: Backing,
    milestones: Vec<Milestone>,
    tiers: Vec<Tier>,
    uplinks: Vec<Uplink>,
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
    accent: String,
    crates: Vec<String>,
}

struct Track {
    title: String,
    accent: String,
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
    preview: String,
    goal: String,
    uplink_title: String,
    uplink_lead: String,
    note_title: String,
    note: String,
    form_note: String,
}

struct Milestone {
    state: String,
    title: String,
    detail: String,
}

struct Tier {
    name: String,
    amount: i64,
    accent: String,
    featured: bool,
    headline: String,
    items: Vec<String>,
}

struct Uplink {
    name: String,
    amount: Option<i64>,
    per: String,
    accent: String,
    role: String,
    headline: String,
    items: Vec<String>,
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

/// The accents the chapters cycle through on the wall of cards.
const ACCENTS: [&str; 6] = ["teal", "amber", "coral", "sky", "forest", "cream"];

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
                accent: string(table, "accent", &at)?,
                crates: strings(table, "crates", &at)?,
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
                accent: string(table, "accent", &at)?,
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

        let mut tiers = Vec::new();
        for table in tables(&doc, "tier")? {
            let name = string(table, "name", "tier")?;
            let at = format!("tier {name}");
            tiers.push(Tier {
                amount: integer(table, "amount", &at)?,
                accent: string(table, "accent", &at)?,
                featured: table
                    .get("featured")
                    .and_then(Item::as_bool)
                    .unwrap_or(false),
                headline: string(table, "headline", &at)?,
                items: strings(table, "items", &at)?,
                name,
            });
        }

        let mut uplinks = Vec::new();
        for table in tables(&doc, "uplink")? {
            let name = string(table, "name", "uplink")?;
            let at = format!("uplink {name}");
            uplinks.push(Uplink {
                amount: table.get("amount").and_then(Item::as_integer),
                per: string(table, "per", &at)?,
                accent: string(table, "accent", &at)?,
                role: string(table, "role", &at)?,
                headline: string(table, "headline", &at)?,
                items: strings(table, "items", &at)?,
                name,
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
                preview: string(backing, "preview", "backing")?,
                goal: string(backing, "goal", "backing")?,
                uplink_title: string(backing, "uplink_title", "backing")?,
                uplink_lead: string(backing, "uplink_lead", "backing")?,
                note_title: string(backing, "note_title", "backing")?,
                note: string(backing, "note", "backing")?,
                form_note: string(backing, "form_note", "backing")?,
            },
            tiers,
            uplinks,
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
    /// * `descriptions` - each crate's one-line description, for the engine cards.
    ///
    /// # Returns
    ///
    /// The `<main>` element and everything in it.
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
        let mut out = String::from("<main class=\"home\" id=\"content\">\n");
        out.push_str(&self.hero(catalog, lib_crates));
        out.push_str(&quickstart(root)?);
        out.push_str(&covers(catalog, descriptions));
        out.push_str(&self.runs());
        out.push_str(&reach(catalog));
        out.push_str(&self.roadmap());
        out.push_str(&self.backing());
        out.push_str("</main>\n");
        Ok(out)
    }

    fn hero(&self, catalog: &Catalog, lib_crates: &[String]) -> String {
        let [a, b, c, d] = <[String; 4]>::try_from(self.hero.title.clone()).unwrap_or_else(|_| {
            [
                "One core.".into(),
                "Every language.".into(),
                "For the devices that".into(),
                "change lives.".into(),
            ]
        });
        let installs: String = LANGUAGES
            .iter()
            .map(|language| {
                format!(
                    "<li><span class=\"install-lang\">{}</span>{}</li>\n",
                    language.name,
                    command(&language.install(language.bundle()))
                )
            })
            .collect();
        let guides = catalog
            .capabilities
            .iter()
            .filter(|capability| capability.guide.is_some())
            .count();
        // The hero plays the first scenario, so the page opens on a node at work.
        let stage = self
            .scenarios
            .first()
            .map(|scenario| scenario.key.as_str())
            .unwrap_or("farm");
        format!(
            "<section class=\"hero\" aria-labelledby=\"hero-title\">\n\
             <div class=\"hero-copy\">\n\
             <p class=\"eyebrow\">{}</p>\n\
             <h1 class=\"hero-title\" id=\"hero-title\">{} <span class=\"grad\">{}</span><br>{} <span class=\"grad-warm\">{}</span></h1>\n\
             <p class=\"hero-lead\">{}</p>\n\
             <ul class=\"installs\" aria-label=\"Install\">\n{installs}</ul>\n\
             <div class=\"hero-doors\">\n\
             <a class=\"btn btn-warm\" href=\"/docs/index.html\">Read the docs</a>\n\
             <a class=\"btn btn-ghost\" href=\"/docs/reference/rust.html\">API reference</a>\n\
             <a class=\"btn btn-ghost\" href=\"/docs/hardware.html\">Hardware</a>\n\
             </div>\n\
             <ul class=\"hero-facts\"><li>{} capabilities</li><li>{} crates</li><li>{} languages</li><li>{guides} guides, every example run in CI</li><li>MIT, forever</li></ul>\n\
             </div>\n\
             <div class=\"hero-side\">\n\
             <figure class=\"hero-stage diorama\" data-diorama=\"{stage}\"><p class=\"diorama-still\">A node at work plays here in a browser that runs scripts.</p></figure>\n\
             <p class=\"stage-note\">Played from the same crates a real node runs. <a href=\"https://pamoja.molex.cloud/dashboard/\">Open the dashboard demo</a></p>\n\
             </div>\n\
             </section>\n",
            escape(&self.hero.eyebrow),
            escape(&a),
            escape(&b),
            escape(&c),
            escape(&d),
            escape(&self.hero.lead),
            catalog.capabilities.len(),
            lib_crates.len(),
            LANGUAGES.len(),
        )
    }
    fn runs(&self) -> String {
        let mut out = String::from(
            "<section class=\"runs\" aria-labelledby=\"runs-title\">\n\
             <div class=\"section-head\">\n\
             <p class=\"eyebrow\">Where it runs</p>\n\
             <h2 class=\"section-title\" id=\"runs-title\">Nine places nothing else quite reaches.</h2>\n\
             <p class=\"section-lead\">Each console below is a node doing its job, played from the same crates a real one would run. Six are field deployments; three are robots. Pick one.</p>\n\
             </div>\n\
             <div class=\"stage-tabs\" role=\"tablist\" aria-label=\"Scenario\">\n",
        );
        for (index, scenario) in self.scenarios.iter().enumerate() {
            out.push_str(&format!(
                "<button class=\"stage-tab\" role=\"tab\" type=\"button\" id=\"tab-{key}\" aria-controls=\"scene-{key}\" aria-selected=\"{}\" data-scene=\"{key}\" data-group=\"{}\" data-accent=\"{}\">{}</button>\n",
                index == 0,
                escape(&scenario.group),
                escape(&scenario.accent),
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
                        "<a class=\"crate-chip\" href=\"docs/reference/rust/{}/index.html\">pamoja-<b>{}</b></a>",
                        krate.replace('-', "_"),
                        escape(krate.trim_start_matches("pamoja-"))
                    )
                })
                .collect();
            out.push_str(&format!(
                "<article class=\"scene\" id=\"scene-{key}\" role=\"tabpanel\" aria-labelledby=\"tab-{key}\" data-accent=\"{}\"{}>\n\
                 <figure class=\"diorama\" data-diorama=\"{key}\"><p class=\"diorama-still\">The {} console plays here in a browser that runs scripts.</p></figure>\n\
                 <div class=\"scene-card\">\n\
                 <p class=\"eyebrow\">{}</p>\n\
                 <h3>{}</h3>\n\
                 <p>{}</p>\n\
                 <div class=\"crate-chips\">{chips}</div>\n\
                 </div>\n\
                 </article>\n",
                escape(&scenario.accent),
                if index == 0 { "" } else { " hidden" },
                escape(&scenario.tab.to_lowercase()),
                escape(&scenario.eyebrow),
                escape(&scenario.title),
                scenario.body,
                key = escape(&scenario.key),
            ));
        }
        out.push_str("</div>\n</section>\n");
        out
    }

    fn roadmap(&self) -> String {
        let mut out = String::from(
            "<section class=\"roadmap\" aria-labelledby=\"roadmap-title\">\n\
             <div class=\"section-head\">\n\
             <p class=\"eyebrow\">Where it is going</p>\n\
             <h2 class=\"section-title\" id=\"roadmap-title\">Not a sensor library. A platform for physical things.</h2>\n\
             <p class=\"section-lead\">Solid tags ship today, in the crates named above; the outlined ones are the committed direction.</p>\n\
             </div>\n\
             <div class=\"tracks\">\n",
        );
        for track in &self.tracks {
            let tags: String = track
                .tags
                .iter()
                .map(|tag| {
                    format!(
                        "<span class=\"track-tag{}\">{}</span>",
                        if tag.ships { " on" } else { "" },
                        escape(&tag.text)
                    )
                })
                .collect();
            out.push_str(&format!(
                "<article class=\"track\" data-accent=\"{}\">\n<h3>{}</h3>\n<p>{}</p>\n<div class=\"track-tags\">{tags}</div>\n</article>\n",
                escape(&track.accent),
                escape(&track.title),
                escape(&track.lead)
            ));
        }
        out.push_str("</div>\n</section>\n");
        out
    }

    fn backing(&self) -> String {
        let b = &self.backing;
        let milestones: String = self
            .milestones
            .iter()
            .map(|milestone| {
                format!(
                    "<li class=\"milestone\" data-state=\"{}\"><span class=\"ms-state\">{}</span><h3>{}</h3><p>{}</p></li>\n",
                    escape(&milestone.state),
                    escape(&milestone.state),
                    escape(&milestone.title),
                    escape(&milestone.detail)
                )
            })
            .collect();
        let mut out = format!(
            "<section class=\"back\" id=\"back\" aria-labelledby=\"back-title\">\n\
             <div class=\"section-head\">\n\
             <p class=\"eyebrow\">Back the mission</p>\n\
             <h2 class=\"section-title\" id=\"back-title\">Raise a node. Reach a place that was off the map.</h2>\n\
             <p class=\"section-lead\">{}</p>\n\
             </div>\n\
             <aside class=\"preview-banner\"><span class=\"preview-pill\">Preview</span><p>{}</p></aside>\n\
             <ol class=\"milestones\" aria-label=\"How backing will open\">\n{milestones}</ol>\n\
             <div class=\"goal\"><div class=\"goal-meta\"><span>Planned first campaign: <strong>{}</strong></span><span class=\"goal-count\">0 of 100 · opens later</span></div><div class=\"goal-bar\"><i style=\"width:0%\"></i></div></div>\n\
             <div class=\"tiers\">\n",
            escape(&b.lead),
            escape(&b.preview),
            escape(&b.goal)
        );
        for tier in &self.tiers {
            let items: String = tier
                .items
                .iter()
                .map(|item| format!("<li>{}</li>", escape(item)))
                .collect();
            out.push_str(&format!(
                "<article class=\"tier{}\" data-accent=\"{}\">\n{}<h3>{}</h3>\n<div class=\"amt\">${}<span> one-time</span></div>\n<p class=\"head\">{}</p>\n<ul>{items}</ul>\n<button class=\"btn btn-ghost soon\" type=\"button\">Back the {}</button>\n</article>\n",
                if tier.featured { " featured" } else { "" },
                escape(&tier.accent),
                if tier.featured {
                    "<span class=\"tag\">most impact</span>\n"
                } else {
                    ""
                },
                escape(&tier.name),
                tier.amount,
                escape(&tier.headline),
                escape(&tier.name)
            ));
        }
        out.push_str(&format!(
            "</div>\n<div class=\"uplink-head\"><h3>{}</h3><p>{}</p></div>\n<div class=\"tiers uplinks\">\n",
            escape(&b.uplink_title),
            escape(&b.uplink_lead)
        ));
        for uplink in &self.uplinks {
            let items: String = uplink
                .items
                .iter()
                .map(|item| format!("<li>{}</li>", escape(item)))
                .collect();
            let amount = match uplink.amount {
                Some(amount) => format!("${amount}<span> {}</span>", escape(&uplink.per)),
                None => format!("<span class=\"partner-amt\">{}</span>", escape(&uplink.per)),
            };
            out.push_str(&format!(
                "<article class=\"tier uplink\" data-accent=\"{}\">\n<span class=\"tag soft\">{}</span>\n<h3>{}</h3>\n<div class=\"amt\">{amount}</div>\n<p class=\"head\">{}</p>\n<ul>{items}</ul>\n<button class=\"btn btn-ghost soon\" type=\"button\">{}</button>\n</article>\n",
                escape(&uplink.accent),
                if uplink.amount.is_some() { "recurring" } else { "partner" },
                escape(&uplink.name),
                escape(&uplink.headline),
                if uplink.role == "vendor" { "Become a partner" } else { "Sponsor this" }
            ));
        }
        out.push_str(&format!(
            "</div>\n<aside class=\"uplink-note\"><h3>{}</h3><p>{}</p></aside>\n\
             <div class=\"back-grid\">\n\
             <form class=\"pledge\" novalidate>\n<h3>Pledge or partner</h3>\n\
             <fieldset class=\"pledge-fields\" disabled>\n\
             <div class=\"role-toggle\" role=\"tablist\"><button type=\"button\" class=\"role active\" data-role=\"donor\" role=\"tab\" aria-selected=\"true\">I want to donate</button><button type=\"button\" class=\"role\" data-role=\"vendor\" role=\"tab\" aria-selected=\"false\">I am a vendor or partner</button></div>\n\
             <div class=\"field-row\"><label>Name<input name=\"name\" type=\"text\" autocomplete=\"name\"></label><label>Email<input name=\"email\" type=\"email\" autocomplete=\"email\"></label></div>\n\
             <label data-when=\"donor\">Amount (USD)<input name=\"amount\" type=\"number\" min=\"1\" step=\"1\" value=\"40\" inputmode=\"numeric\"></label>\n\
             <label data-when=\"vendor\" hidden>Company or organisation<input name=\"org\" type=\"text\" autocomplete=\"organization\"></label>\n\
             <label>Message<textarea name=\"message\" rows=\"3\" placeholder=\"What would you like to back, or how can you help?\"></textarea></label>\n\
             <button type=\"submit\" class=\"btn btn-warm btn-block\">Backing opens later</button>\n\
             </fieldset>\n\
             <p class=\"form-note\">{}</p>\n\
             </form>\n\
             <div class=\"back-side\">\n<h3>The software is already open</h3>\n<p>Use it, fork it, ship it. No sign-up, no cost, no lock-in.</p>\n\
             <div class=\"pkg-links\">\n\
             <a href=\"https://github.com/molexxxx/pamoja\">GitHub</a>\n\
             <a href=\"https://crates.io/crates/pamoja\">crates.io</a>\n\
             <a href=\"https://www.npmjs.com/package/pamoja\">npm</a>\n\
             <a href=\"https://pypi.org/project/pamoja/\">PyPI</a>\n\
             <a href=\"https://www.nuget.org/packages/Pamoja\">NuGet</a>\n\
             </div>\n\
             <div class=\"why-list\"><div><strong>$2</strong><span>microcontroller floor</span></div><div><strong>256&nbsp;KB</strong><span>RAM target</span></div><div><strong>MIT</strong><span>licensed, forever</span></div></div>\n\
             </div>\n</div>\n</section>\n",
            escape(&b.note_title),
            b.note,
            escape(&b.form_note)
        ));
        out
    }
}

// The first example in four languages, spliced from the tests that run it, in the same
// tab block the guides use, so the language a reader chose there is chosen here too. Each
// panel opens capped to a screen or so of code, with the whole example one click away.
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
             <p class=\"source\">From <a href=\"https://github.com/molexxxx/pamoja/blob/main/{path}\"><code>{path}</code></a>, which runs in CI:</p>\n\
             <figure class=\"code\" data-lang=\"{lang}\"><figcaption><span class=\"code-lang\">{label}</span><button class=\"copy\" type=\"button\" aria-label=\"Copy this code\">copy</button></figcaption><pre><code>{}</code></pre></figure>\n\
             <button class=\"reveal\" type=\"button\">Show the whole example</button>\n\
             </section>\n",
            highlight::highlight(&code, lang)
        ));
    }
    Ok(format!(
        "<section class=\"quick\" aria-labelledby=\"quick-title\">\n\
         <div class=\"section-head\">\n\
         <p class=\"eyebrow\">A first example</p>\n\
         <h2 class=\"section-title\" id=\"quick-title\">The same program, in the language you already work in.</h2>\n\
         <p class=\"section-lead\">A reading taken off a wire on a field node, sent over a link, and checked on the gateway that receives it, with nothing plugged in and nothing running.</p>\n\
         </div>\n\
         <div class=\"langs\">\n<div class=\"lang-tabs\" role=\"tablist\" aria-label=\"Language\">\n{tabs}</div>\n{panels}</div>\n\
         </section>\n"
    ))
}

// Every capability as a card, in chapter order, with the engine crates first; a card
// opens into the four package pages and the guide.
fn covers(catalog: &Catalog, descriptions: &BTreeMap<String, String>) -> String {
    let mut out = String::from(
        "<section class=\"covers\" id=\"covers\" aria-labelledby=\"covers-title\">\n\
         <div class=\"section-head\">\n\
         <p class=\"eyebrow\">What it covers</p>\n\
         <h2 class=\"section-title\" id=\"covers-title\">Compile in only what you need.</h2>\n\
         <p class=\"section-lead\">Every capability is a crate in Rust and a package in each binding, behind the traits in <code>pamoja-core</code>. On a microcontroller you bring in two crates and nothing else.</p>\n\
         </div>\n\
         <div class=\"bento-filter\" role=\"tablist\" aria-label=\"Filter by chapter\">\n\
         <button class=\"chip-btn active\" role=\"tab\" type=\"button\" aria-selected=\"true\" data-chapter=\"all\">All</button>\n",
    );
    for chapter in &catalog.chapters {
        out.push_str(&format!(
            "<button class=\"chip-btn\" role=\"tab\" type=\"button\" aria-selected=\"false\" data-chapter=\"{}\">{}</button>\n",
            escape(&chapter.key),
            escape(&chapter.title)
        ));
    }
    out.push_str("</div>\n<div class=\"bento\">\n");

    for (index, krate) in catalog.engine.iter().enumerate() {
        let big = index == 0;
        let description = descriptions.get(krate).cloned().unwrap_or_default();
        out.push_str(&format!(
            "<article class=\"bento-card{}\" data-chapter=\"engine\" data-accent=\"amber\" tabindex=\"0\">\n\
             <div class=\"bc-face\"><p class=\"bc-role\">Engine</p><h3 class=\"bc-name\">{krate}</h3><p class=\"bc-summary\">{}</p>{}</div>\n\
             <div class=\"bc-pop\"><div class=\"pkg-btns\"><a class=\"pkg-btn crates\" href=\"https://crates.io/crates/{krate}\">crates.io</a><a class=\"pkg-btn api rust\" href=\"docs/reference/rust/{}/index.html\">API reference</a></div></div>\n\
             </article>\n",
            if big { " span-big" } else { "" },
            escape(&description),
            if big {
                command(&format!("cargo add {krate}"))
            } else {
                String::new()
            },
            krate.replace('-', "_")
        ));
    }

    for (chapter_index, chapter) in catalog.chapters.iter().enumerate() {
        let accent = ACCENTS[chapter_index % ACCENTS.len()];
        for capability in catalog.in_chapter(&chapter.key) {
            out.push_str(&card(capability, chapter, accent));
        }
    }
    out.push_str("</div>\n</section>\n");
    out
}

fn card(capability: &Capability, chapter: &crate::catalog::Chapter, accent: &str) -> String {
    let crates: String = capability
        .crates
        .iter()
        .map(|krate| {
            format!(
                "<span class=\"crate-chip\">pamoja-<b>{}</b></span>",
                escape(krate.trim_start_matches("pamoja-"))
            )
        })
        .collect();
    let mut links: Vec<String> = LANGUAGES
        .iter()
        .map(|language: &Language| {
            let package = language.package(capability);
            format!(
                "<a class=\"pkg-btn {}\" href=\"{}\" title=\"{}\">{}</a>",
                language.key,
                language.registry_url(&package),
                escape(&package),
                language.registry
            )
        })
        .collect();
    if let Some(guide) = &capability.guide {
        links.push(format!(
            "<a class=\"pkg-btn guide\" href=\"docs/{}.html\">Guide</a>",
            guide.trim_end_matches(".md")
        ));
    }
    format!(
        "<article class=\"bento-card\" data-chapter=\"{}\" data-accent=\"{accent}\" tabindex=\"0\">\n\
         <div class=\"bc-face\"><p class=\"bc-role\">{}</p><h3 class=\"bc-name\">{}</h3><p class=\"bc-summary\">{}</p><div class=\"crate-chips\">{crates}</div></div>\n\
         <div class=\"bc-pop\"><div class=\"pkg-btns\">{}</div></div>\n\
         </article>\n",
        escape(&chapter.key),
        escape(&chapter.title),
        escape(&capability.title),
        escape(&capability.summary),
        links.join("")
    )
}

// The four languages as doors: the install line, and where the reference and the guides are.
fn reach(catalog: &Catalog) -> String {
    let guides = catalog
        .capabilities
        .iter()
        .filter(|capability| capability.guide.is_some())
        .count();
    let mut out = String::from(
        "<section class=\"reach\" aria-labelledby=\"reach-title\">\n\
         <div class=\"section-head\">\n\
         <p class=\"eyebrow\">Reach</p>\n\
         <h2 class=\"section-title\" id=\"reach-title\">The same shape, in every language.</h2>\n\
         <p class=\"section-lead\">One memory-safe engine, idiomatic bindings on top. Every capability is a package in each, and every guide shows the same task in all four.</p>\n\
         </div>\n\
         <div class=\"doors\">\n",
    );
    for language in &LANGUAGES {
        out.push_str(&format!(
            "<article class=\"door-card\">\n<h3>{}</h3>\n{}\n<p class=\"door-what\">Every {} with its API pages, generated by {}.</p>\n\
             <div class=\"door-links\"><a href=\"docs/reference/{}.html\">{} reference</a><a href=\"docs/index.html\">{guides} guides</a></div>\n</article>\n",
            language.name,
            command(&language.install(language.bundle())),
            language.unit(),
            language.generator(),
            language.key,
            language.name
        ));
    }
    out.push_str("</div>\n</section>\n");
    out
}

// An install line with the button that copies it, the same shape the reference pages use.
fn command(text: &str) -> String {
    let text = escape(text);
    format!(
        "<div class=\"pkg-get\"><code class=\"cmd\">{text}</code><button class=\"copy\" type=\"button\" data-copy=\"{text}\" aria-label=\"Copy the install command\">copy</button></div>"
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

fn integer(table: &dyn toml_edit::TableLike, key: &str, at: &str) -> Result<i64, String> {
    table
        .get(key)
        .and_then(Item::as_integer)
        .ok_or_else(|| format!("{at}: `{key}` must be an integer"))
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
accent = "teal"
crates = ["pamoja-modbus"]

[[track]]
key = "radio"
title = "Radio"
accent = "teal"
lead = "The cheapest link first."
tags = [
  { text = "MQTT", ships = true, crate = "pamoja-mqtt" },
  { text = "satellite", ships = false, crate = "pamoja-satellite" },
]

[backing]
lead = "Free software; hardware costs."
preview = "Not open yet."
goal = "100 kits"
uplink_title = "Sponsor the uplink"
uplink_lead = "The link is the recurring cost."
note_title = "How the link gets paid for"
note = "By an NGO, <em>never</em> the family."
form_note = "A preview."

[[milestone]]
state = "now"
title = "Design the kit"
detail = "With partners."

[[tier]]
name = "Spark"
amount = 15
accent = "teal"
headline = "One node"
items = ["A microcontroller", "One sensor"]

[[uplink]]
name = "Carrier"
per = "partner"
accent = "amber"
role = "vendor"
headline = "Operators"
items = ["Donate airtime"]
"#;

    const CONSOLES: &str = "const SPECS = {\n  farm: {\n    id: 'x',\n  },\n};\n";

    #[test]
    fn parses_and_checks_the_data() {
        let home = Home::parse(SAMPLE).unwrap();
        assert_eq!(home.scenario_keys(), ["farm"]);
        assert_eq!(home.tiers[0].amount, 15);
        assert_eq!(home.milestones[0].state, "now");
        assert!(home.uplinks[0].amount.is_none());
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
