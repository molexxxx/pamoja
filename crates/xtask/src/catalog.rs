//! The capability map in `docs/capabilities.toml`: the chapters the guides follow,
//! what each capability covers in every language, and the checks that keep the map
//! honest against the crates, the binding exports, and the .NET types. The map
//! renders the tables in the READMEs and the site through [`Catalog::render`].

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use toml_edit::{DocumentMut, Item};

/// Where the site is published; the tables link into it with absolute URLs so the
/// registry pages, which do not resolve relative links, reach the guides too.
pub const SITE: &str = "https://pamoja.molex.cloud/docs";

/// A group of capabilities that share a chapter of the guides.
pub struct Chapter {
    pub key: String,
    pub title: String,
    pub intent: String,
}

/// One capability: what it is called, what it covers, and where it lives in each language.
pub struct Capability {
    pub key: String,
    pub chapter: String,
    pub title: String,
    pub summary: String,
    pub crates: Vec<String>,
    pub node: String,
    pub python: String,
    pub dotnet: Vec<String>,
    pub guide: Option<String>,
}

impl Capability {
    /// The NuGet package the capability lives in: its own `Pamoja.<Name>`, or
    /// `Pamoja.Core` for the surface the engine carries itself.
    pub fn dotnet_package(&self) -> String {
        if self.node == "core" {
            "Pamoja.Core".to_owned()
        } else {
            format!("Pamoja.{}", dotnet_name(&self.key))
        }
    }
}

/// The whole map: chapters in order, capabilities in order, the engine crates, and the
/// crate that bundles every capability behind a feature each.
pub struct Catalog {
    pub chapters: Vec<Chapter>,
    pub capabilities: Vec<Capability>,
    pub engine: Vec<String>,
    pub bundle: Option<String>,
}

impl Catalog {
    /// Read `docs/capabilities.toml` under `root`.
    ///
    /// # Errors
    ///
    /// Returns the reason when the file is missing or malformed.
    pub fn load(root: &Path) -> Result<Catalog, String> {
        let path = root.join("docs/capabilities.toml");
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        Catalog::parse(&text)
    }

    /// Parse the map from its TOML text.
    ///
    /// # Errors
    ///
    /// Returns the reason when a required field is missing or has the wrong type.
    pub fn parse(text: &str) -> Result<Catalog, String> {
        let doc: DocumentMut = text
            .parse()
            .map_err(|err| format!("capabilities.toml is not valid TOML: {err}"))?;

        let mut chapters = Vec::new();
        for table in tables(&doc, "chapter")? {
            chapters.push(Chapter {
                key: string(table, "key", "chapter")?,
                title: string(table, "title", "chapter")?,
                intent: string(table, "intent", "chapter")?,
            });
        }

        let mut capabilities = Vec::new();
        for table in tables(&doc, "capability")? {
            let key = string(table, "key", "capability")?;
            let context = format!("capability {key}");
            capabilities.push(Capability {
                chapter: string(table, "chapter", &context)?,
                title: string(table, "title", &context)?,
                summary: string(table, "summary", &context)?,
                crates: strings(table, "crates", &context)?,
                node: string(table, "node", &context)?,
                python: string(table, "python", &context)?,
                dotnet: strings(table, "dotnet", &context)?,
                guide: table.get("guide").and_then(Item::as_str).map(str::to_owned),
                key,
            });
        }

        let engine = doc
            .get("engine")
            .and_then(Item::as_table_like)
            .map(|engine| strings_of(engine, "crates", "engine"))
            .transpose()?
            .unwrap_or_default();

        let bundle = doc
            .get("bundle")
            .and_then(Item::as_table_like)
            .map(|bundle| string(bundle, "crate", "bundle"))
            .transpose()?;

        Ok(Catalog {
            chapters,
            capabilities,
            engine,
            bundle,
        })
    }

    /// Every capability in table order: the engine's own surface first, then the
    /// chapters in map order.
    pub fn ordered(&self) -> Vec<&Capability> {
        let mut out: Vec<&Capability> = self
            .capabilities
            .iter()
            .filter(|capability| capability.node == "core")
            .collect();
        for chapter in &self.chapters {
            out.extend(
                self.in_chapter(&chapter.key)
                    .filter(|capability| capability.node != "core"),
            );
        }
        out
    }

    /// The chapters worth naming as a set, with every capability they hold. A chapter
    /// qualifies when more than one of its capabilities has a crate of its own, and the
    /// engine's own surface comes with the chapter it belongs to, so installing a domain
    /// gives the whole chapter as the guides present it.
    pub fn domains(&self) -> Vec<(&Chapter, Vec<&Capability>)> {
        self.chapters
            .iter()
            .map(|chapter| {
                let members: Vec<&Capability> = self.in_chapter(&chapter.key).collect();
                (chapter, members)
            })
            .filter(|(_, members)| {
                members
                    .iter()
                    .filter(|capability| !capability.crates.is_empty())
                    .count()
                    > 1
            })
            .collect()
    }

    /// The capabilities of one chapter, in map order.
    pub fn in_chapter<'a>(&'a self, chapter: &'a str) -> impl Iterator<Item = &'a Capability> {
        self.capabilities
            .iter()
            .filter(move |capability| capability.chapter == chapter)
    }

    /// The capability with `key`.
    pub fn capability(&self, key: &str) -> Option<&Capability> {
        self.capabilities
            .iter()
            .find(|capability| capability.key == key)
    }

    /// The same capability in the other three languages: where to install it from and
    /// where its reference is. Every capability page in every registry carries this, so a
    /// reader who arrives on the crates.io page can reach the npm one without going back
    /// through the site.
    ///
    /// # Arguments
    ///
    /// * `capability` - the capability the page is about.
    ///
    /// # Returns
    ///
    /// A Markdown table, without a trailing newline.
    pub fn cross_language(&self, capability: &Capability) -> String {
        let mut out = String::from("| Language | Package | Reference |\n| --- | --- | --- |\n");
        let rust = if capability.crates.is_empty() {
            format!(
                "| Rust | [`pamoja-core`](https://crates.io/crates/pamoja-core) | [reference]({}), [docs.rs](https://docs.rs/pamoja-core) |\n",
                rustdoc_url("pamoja-core")
            )
        } else {
            capability
                .crates
                .iter()
                .map(|krate| {
                    format!(
                        "| Rust | [`{krate}`](https://crates.io/crates/{krate}) | [reference]({}), [docs.rs](https://docs.rs/{krate}) |\n",
                        rustdoc_url(krate)
                    )
                })
                .collect()
        };
        out.push_str(&rust);
        out.push_str(&format!(
            "| TypeScript | [`{0}`](https://www.npmjs.com/package/{0}) | [reference]({1}) |\n",
            node_package(capability),
            node_reference_url(&capability.node)
        ));
        out.push_str(&format!(
            "| Python | [`pamoja-{0}`](https://pypi.org/project/pamoja-{0}/) | [reference]({1}) |\n",
            capability.python,
            python_reference_url(&capability.python)
        ));
        let package = capability.dotnet_package();
        out.push_str(&format!(
            "| C# | [`{package}`](https://www.nuget.org/packages/{package}) | [reference]({}) |\n",
            dotnet_reference_url(&package)
        ));
        out.trim_end().to_owned()
    }

    /// Check the map against the repository: every library crate claimed once, the
    /// node keys matching the packages, the python keys matching the modules, every dotnet
    /// name declared, and every guide present. With `require_guides`, a capability
    /// without a guide is an error too.
    ///
    /// # Errors
    ///
    /// Returns every disagreement found, one per line.
    pub fn check(
        &self,
        root: &Path,
        lib_crates: &[String],
        require_guides: bool,
    ) -> Result<(), String> {
        let mut problems = Vec::new();

        let mut chapter_keys = BTreeSet::new();
        for chapter in &self.chapters {
            if !chapter_keys.insert(chapter.key.as_str()) {
                problems.push(format!("chapter {} is declared twice", chapter.key));
            }
        }
        let mut capability_keys = BTreeSet::new();
        for capability in &self.capabilities {
            if !capability_keys.insert(capability.key.as_str()) {
                problems.push(format!("capability {} is declared twice", capability.key));
            }
            if !chapter_keys.contains(capability.chapter.as_str()) {
                problems.push(format!(
                    "capability {} names the unknown chapter {}",
                    capability.key, capability.chapter
                ));
            }
        }

        let mut claimed: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for capability in &self.capabilities {
            for krate in &capability.crates {
                claimed
                    .entry(krate.as_str())
                    .or_default()
                    .push(capability.key.as_str());
            }
        }
        for krate in &self.engine {
            claimed.entry(krate.as_str()).or_default().push("engine");
        }
        if let Some(name) = &self.bundle {
            claimed.entry(name.as_str()).or_default().push("bundle");
            problems.extend(self.bundle_problems(root, name));
        }
        for krate in lib_crates {
            match claimed.get(krate.as_str()) {
                None => problems.push(format!(
                    "crate {krate} is claimed by no capability and is not in [engine]"
                )),
                Some(owners) if owners.len() > 1 => problems.push(format!(
                    "crate {krate} is claimed more than once: {}",
                    owners.join(", ")
                )),
                Some(_) => {}
            }
        }
        for krate in claimed.keys() {
            if !lib_crates.iter().any(|known| known == krate) {
                problems.push(format!("{krate} is claimed but is not a library crate"));
            }
        }

        // A domain has a package of its own in each binding, alongside the capabilities.
        let domain_keys: BTreeSet<&str> = self
            .domains()
            .into_iter()
            .map(|(chapter, _)| chapter.key.as_str())
            .collect();

        match node_packages(root) {
            Ok(packages) => {
                let keys: BTreeSet<&str> = self
                    .capabilities
                    .iter()
                    .map(|capability| capability.node.as_str())
                    .filter(|key| *key != "core")
                    .chain(domain_keys.iter().copied())
                    .collect();
                for package in &packages {
                    if !keys.contains(package.as_str()) {
                        problems.push(format!(
                            "bindings/node/packages/{package} exists, which no capability claims"
                        ));
                    }
                }
                for key in &keys {
                    if !packages.contains(*key) {
                        problems.push(format!(
                            "node = \"{key}\" has no package under bindings/node/packages"
                        ));
                    }
                }
            }
            Err(err) => problems.push(err),
        }

        match python_packages(root) {
            Ok(packages) => {
                let keys: BTreeSet<&str> = self
                    .capabilities
                    .iter()
                    .map(|capability| capability.python.as_str())
                    .filter(|key| *key != "core")
                    .chain(domain_keys.iter().copied())
                    .collect();
                for package in &packages {
                    if !keys.contains(package.as_str()) {
                        problems.push(format!(
                            "bindings/python/packages/{package} exists, which no capability claims"
                        ));
                    }
                }
                for key in &keys {
                    if !packages.contains(*key) {
                        problems.push(format!(
                            "python = \"{key}\" has no package under bindings/python/packages"
                        ));
                    }
                }
            }
            Err(err) => problems.push(err),
        }

        match dotnet_types(root) {
            Ok(types) => {
                for capability in &self.capabilities {
                    for name in &capability.dotnet {
                        if !types.contains(name) {
                            problems.push(format!(
                                "capability {}: {name} is not a type declared under bindings/dotnet/src",
                                capability.key
                            ));
                        }
                    }
                }
            }
            Err(err) => problems.push(err),
        }

        match dotnet_packages(root) {
            Ok(packages) => {
                let names: BTreeSet<String> =
                    self.capabilities
                        .iter()
                        .filter(|capability| capability.node != "core")
                        .map(|capability| dotnet_name(&capability.key))
                        .chain(domain_keys.iter().map(|key| {
                            key.split('-').map(dotnet_name).collect::<Vec<_>>().concat()
                        }))
                        .collect();
                for package in &packages {
                    if !names.contains(package) {
                        problems.push(format!(
                            "bindings/dotnet/src/Pamoja.{package} exists, which no capability claims"
                        ));
                    }
                }
                for name in &names {
                    if !packages.contains(name) {
                        problems.push(format!(
                            "capability package Pamoja.{name} has no project under bindings/dotnet/src"
                        ));
                    }
                }
            }
            Err(err) => problems.push(err),
        }

        for capability in &self.capabilities {
            match &capability.guide {
                Some(guide) if !root.join("docs").join(guide).is_file() => problems.push(format!(
                    "capability {}: docs/{guide} does not exist",
                    capability.key
                )),
                None if require_guides => {
                    problems.push(format!("capability {} has no guide", capability.key))
                }
                _ => {}
            }
        }

        if problems.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "docs/capabilities.toml disagrees with the repository:\n  {}",
                problems.join("\n  ")
            ))
        }
    }

    // The bundle crate's manifest against the map: one feature per capability, named by
    // the capability key, enabling that capability's crates and no crate another
    // capability claims, and in the default set.
    fn bundle_problems(&self, root: &Path, name: &str) -> Vec<String> {
        let path = root.join("crates").join(name).join("Cargo.toml");
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => return vec![format!("reading {}: {err}", path.display())],
        };
        let doc: DocumentMut = match text.parse() {
            Ok(doc) => doc,
            Err(err) => return vec![format!("{} is not valid TOML: {err}", path.display())],
        };
        let Some(features) = doc.get("features").and_then(Item::as_table_like) else {
            return vec![format!("crates/{name}/Cargo.toml has no [features] table")];
        };
        let feature = |key: &str| -> Option<Vec<String>> {
            features.get(key).and_then(Item::as_array).map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .map(str::to_owned)
                    .collect()
            })
        };
        let default = feature("default").unwrap_or_default();

        // The crates another capability claims: a feature may pull in a shared engine crate,
        // but never one this map attributes to a different capability.
        let claimed_elsewhere = |key: &str| -> BTreeSet<&str> {
            self.capabilities
                .iter()
                .filter(|other| other.key != key)
                .flat_map(|other| other.crates.iter().map(String::as_str))
                .collect()
        };

        let mut problems = Vec::new();

        // A chapter with more than one capability gets a feature of its own, so a build can
        // name a domain instead of listing its parts.
        for chapter in &self.chapters {
            let members: Vec<&str> = self
                .in_chapter(&chapter.key)
                .filter(|capability| !capability.crates.is_empty())
                .map(|capability| capability.key.as_str())
                .collect();
            if members.len() < 2 {
                continue;
            }
            match feature(&chapter.key) {
                None => problems.push(format!(
                    "crates/{name}/Cargo.toml has no `{}` feature for the chapter of the same name",
                    chapter.key
                )),
                Some(enabled) => {
                    let listed: BTreeSet<&str> = enabled.iter().map(String::as_str).collect();
                    let expected: BTreeSet<&str> = members.iter().copied().collect();
                    for missing in expected.difference(&listed) {
                        problems.push(format!(
                            "crates/{name}/Cargo.toml: feature `{}` does not enable `{missing}`",
                            chapter.key
                        ));
                    }
                    for extra in listed.difference(&expected) {
                        problems.push(format!(
                            "crates/{name}/Cargo.toml: feature `{}` enables `{extra}`, which is not in that chapter",
                            chapter.key
                        ));
                    }
                }
            }
        }

        for capability in &self.capabilities {
            if capability.crates.is_empty() {
                continue;
            }
            let key = &capability.key;
            match feature(key) {
                None => problems.push(format!("crates/{name}/Cargo.toml has no `{key}` feature")),
                Some(enabled) => {
                    for krate in &capability.crates {
                        let dep = format!("dep:{krate}");
                        if !enabled.contains(&dep) {
                            problems.push(format!(
                                "crates/{name}/Cargo.toml: feature `{key}` does not enable {dep}"
                            ));
                        }
                    }
                    let others = claimed_elsewhere(key);
                    for entry in &enabled {
                        let Some(krate) = entry.strip_prefix("dep:") else {
                            continue;
                        };
                        if others.contains(krate) {
                            problems.push(format!(
                                "crates/{name}/Cargo.toml: feature `{key}` also enables {entry}, which another capability claims"
                            ));
                        }
                    }
                }
            }
            if !default.contains(key) {
                problems.push(format!(
                    "crates/{name}/Cargo.toml: `{key}` is not in the default feature set"
                ));
            }
        }
        problems
    }

    /// Render one generated table for a `<!-- table: <kind> [arg] -->` region.
    ///
    /// The kinds are `chapters` (the capability map by chapter), `guides` (the guide
    /// list for the site's front page), `crates` (every crate with its reference
    /// links), `reference <capability>` (the per-language reference links of one
    /// guide), and `binding <node|python|dotnet>` (the capability table of one
    /// binding README).
    ///
    /// # Errors
    ///
    /// Returns the reason when the kind or its argument is unknown.
    pub fn render(
        &self,
        directive: &str,
        crate_descriptions: &BTreeMap<String, String>,
    ) -> Result<String, String> {
        let mut words = directive.split_whitespace();
        let kind = words.next().unwrap_or_default();
        let arg = words.next();
        match (kind, arg) {
            ("chapters", None) => Ok(self.chapters_table()),
            ("guides", None) => Ok(self.guide_list()),
            ("crates", None) => Ok(self.crates_table(crate_descriptions)),
            ("reference", Some(key)) => self
                .capability(key)
                .map(|capability| self.reference_links(capability))
                .ok_or_else(|| format!("reference table names the unknown capability {key}")),
            ("binding", Some(language @ ("node" | "python" | "dotnet"))) => {
                Ok(self.binding_table(language))
            }
            ("domains", Some(language @ ("rust" | "node" | "python" | "dotnet"))) => {
                Ok(self.domains_block(language))
            }
            ("references", None) => Ok(references(false)),
            ("references", Some("absolute")) => Ok(references(true)),
            ("reference-link", Some(language @ ("rust" | "node" | "python" | "dotnet"))) => {
                Ok(reference_link(language))
            }
            _ => Err(format!("unknown table `{directive}`")),
        }
    }

    fn chapters_table(&self) -> String {
        let mut out = String::from("| Chapter | Guides | Crates |\n| --- | --- | --- |\n");
        for chapter in &self.chapters {
            let guides: Vec<String> = self
                .in_chapter(&chapter.key)
                .map(|capability| match guide_url(capability) {
                    Some(url) => format!("[{}]({url})", capability.title),
                    None => capability.title.clone(),
                })
                .collect();
            let crates: Vec<String> = self
                .in_chapter(&chapter.key)
                .flat_map(|capability| capability.crates.iter())
                .map(|krate| crate_link(krate))
                .collect();
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                chapter.title,
                guides.join(", "),
                crates.join(", ")
            ));
        }
        let engine: Vec<String> = self.engine.iter().map(|krate| crate_link(krate)).collect();
        out.push_str(&format!(
            "| Engine | the traits every capability implements, the C ABI, and the dashboard | {} |",
            engine.join(", ")
        ));
        if let Some(bundle) = &self.bundle {
            out.push_str(&format!(
                "\n| Everything | `cargo add {bundle}`: every capability above, behind a feature each | {} |",
                crate_link(bundle)
            ));
        }
        out
    }

    fn guide_list(&self) -> String {
        let mut sections = Vec::new();
        for chapter in &self.chapters {
            let mut section = format!("### {}\n\n{}\n\n", chapter.title, chapter.intent);
            for capability in self.in_chapter(&chapter.key) {
                match &capability.guide {
                    Some(guide) => section.push_str(&format!(
                        "- [{}]({guide}) - {}\n",
                        capability.title, capability.summary
                    )),
                    None => section.push_str(&format!(
                        "- {} - {}\n",
                        capability.title, capability.summary
                    )),
                }
            }
            sections.push(section.trim_end().to_owned());
        }
        sections.join("\n\n")
    }

    fn crates_table(&self, descriptions: &BTreeMap<String, String>) -> String {
        let mut out = String::from(
            "| Chapter | Crate | What it does |
| --- | --- | --- |
",
        );
        let mut rows: Vec<(String, String)> = Vec::new();
        if let Some(bundle) = &self.bundle {
            rows.push(("Everything".to_owned(), bundle.clone()));
        }
        for krate in &self.engine {
            rows.push(("Engine".to_owned(), krate.clone()));
        }
        for chapter in &self.chapters {
            for capability in self.in_chapter(&chapter.key) {
                for krate in &capability.crates {
                    rows.push((chapter.title.clone(), krate.clone()));
                }
            }
        }
        // The chapter is named once per run, so the table reads as a handful of groups.
        let mut last = String::new();
        for (chapter, krate) in rows {
            let description = descriptions.get(&krate).cloned().unwrap_or_default();
            let shown = if chapter == last {
                String::new()
            } else {
                last.clone_from(&chapter);
                format!("**{chapter}**")
            };
            out.push_str(&format!(
                "| {shown} | {} | {description} |
",
                crate_link(&krate)
            ));
        }
        out.trim_end().to_owned()
    }

    fn reference_links(&self, capability: &Capability) -> String {
        let mut lines = Vec::new();
        if !capability.crates.is_empty() {
            let crates: Vec<String> = capability
                .crates
                .iter()
                .map(|krate| format!("[`{krate}`]({})", rustdoc_url(krate)))
                .collect();
            lines.push(format!("- Rust: {}", crates.join(", ")));
        } else {
            lines.push(format!(
                "- Rust: the `Transport` trait in [`pamoja-core`]({})",
                rustdoc_url("pamoja-core")
            ));
        }
        lines.push(format!(
            "- TypeScript: [`{}`]({})",
            node_package(capability),
            node_reference_url(&capability.node)
        ));
        lines.push(format!(
            "- Python: [`pamoja.{0}`]({1})",
            capability.python,
            python_reference_url(&capability.python)
        ));
        let package = capability.dotnet_package();
        lines.push(format!(
            "- C#: [`{package}`]({})",
            dotnet_reference_url(&package)
        ));
        lines.join("\n")
    }

    // The install line for each domain, in the language's own mechanism: a feature in
    // Rust, which decides what compiles, and the capability packages elsewhere, where
    // naming them keeps the manifest an honest record of what the code uses.
    fn domains_block(&self, language: &str) -> String {
        let rows: Vec<(String, String)> = self
            .domains()
            .into_iter()
            .map(|(chapter, _members)| {
                let command = match language {
                    "rust" => format!("cargo add pamoja --features {}", chapter.key),
                    "node" => format!("npm install @pamoja/{}", chapter.key),
                    "python" => format!("pip install pamoja-{}", chapter.key),
                    _ => format!(
                        "dotnet add package Pamoja.{}",
                        chapter
                            .key
                            .split('-')
                            .map(dotnet_name)
                            .collect::<Vec<_>>()
                            .concat()
                    ),
                };
                (command, chapter.title.clone())
            })
            .collect();
        let width = rows
            .iter()
            .map(|(command, _)| command.len())
            .max()
            .unwrap_or(0);
        let mut out = String::from(
            "```sh
",
        );
        for (command, title) in rows {
            out.push_str(&format!(
                "{command:<width$}  # {title}
"
            ));
        }
        out.push_str("```");
        out
    }

    fn binding_table(&self, language: &str) -> String {
        let import_heading = match language {
            "node" => "Import",
            "python" => "Module",
            _ => "Package",
        };
        let mut out = format!(
            "| Group | Capability | {import_heading} | What it covers |
| --- | --- | --- | --- |
"
        );
        // The chapter is named once per group, so thirty rows read as a handful of domains
        // rather than a flat list.
        let mut last = "";
        for capability in self.ordered() {
            let import = match language {
                "node" => format!(
                    "[`{}`]({})",
                    node_package(capability),
                    node_reference_url(&capability.node)
                ),
                "python" => format!(
                    "[`pamoja.{0}`]({1})",
                    capability.python,
                    python_reference_url(&capability.python)
                ),
                _ => {
                    let package = capability.dotnet_package();
                    format!("[`{package}`]({})", dotnet_reference_url(&package))
                }
            };
            let title = match guide_url(capability) {
                Some(url) => format!("[{}]({url})", capability.title),
                None => capability.title.clone(),
            };
            // The engine's own surface is hoisted above the chapters, so it is labelled for
            // what it is rather than borrowing the chapter it happens to sit in.
            let chapter = if capability.node == "core" {
                "**Engine**".to_owned()
            } else if capability.chapter == last {
                String::new()
            } else {
                last = &capability.chapter;
                self.chapters
                    .iter()
                    .find(|chapter| chapter.key == capability.chapter)
                    .map(|chapter| format!("**{}**", chapter.title))
                    .unwrap_or_default()
            };
            out.push_str(&format!(
                "| {chapter} | {title} | {import} | {} |
",
                capability.summary
            ));
        }
        out.trim_end().to_owned()
    }

    /// Render `docs/SUMMARY.md`, the site's navigation: the front page and install
    /// page, a part per chapter holding its guides, then the references and the
    /// pages about the project.
    pub fn summary(&self) -> String {
        let mut out =
            String::from("# Summary\n\n[Introduction](README.md)\n[Install](install.md)\n");
        for chapter in &self.chapters {
            let guides: Vec<&Capability> = self
                .in_chapter(&chapter.key)
                .filter(|capability| capability.guide.is_some())
                .collect();
            if guides.is_empty() {
                continue;
            }
            out.push_str(&format!("\n# {}\n\n", chapter.title));
            for capability in guides {
                if let Some(guide) = &capability.guide {
                    out.push_str(&format!("- [{}]({guide})\n", capability.title));
                }
            }
        }
        out.push_str(
            "\n# Reference\n\n\
             - [Rust](reference/rust.md)\n\
             - [TypeScript](reference/node.md)\n\
             - [Python](reference/python.md)\n\
             - [C#](reference/dotnet.md)\n\
             \n# About\n\n\
             - [Why it exists](about/why.md)\n\
             - [Architecture](about/architecture.md)\n\
             - [Standards and conformance](about/standards.md)\n\
             - [Building](about/building.md)\n\
             - [Releasing](about/releasing.md)\n",
        );
        out
    }
}

/// The absolute URL of a capability's guide on the site, when it has one.
fn guide_url(capability: &Capability) -> Option<String> {
    capability.guide.as_ref().map(|guide| {
        let page = guide.strip_suffix(".md").unwrap_or(guide);
        format!("{SITE}/{page}.html")
    })
}

/// A crate name linked to its rustdoc on the site, which is where every other reference
/// on every page points; docs.rs stays the per-version copy, named on the Rust reference.
fn crate_link(krate: &str) -> String {
    format!("[`{krate}`]({})", rustdoc_url(krate))
}

/// The four bindings on the front page: what a reader installs, the page on this site that
/// says what the binding covers, and the generated reference site itself. Relative, since
/// only the site front page carries it, unless `absolute` is set, which the root README
/// needs since a registry renders it away from the site.
fn references(absolute: bool) -> String {
    let rows = [
        (
            "Rust",
            "cargo add pamoja",
            "reference/rust.md",
            "reference/rust/pamoja/index.html",
            "rustdoc",
        ),
        (
            "TypeScript",
            "npm install pamoja",
            "reference/node.md",
            "reference/node/index.html",
            "typedoc",
        ),
        (
            "Python",
            "pip install pamoja",
            "reference/python.md",
            "reference/python/pamoja.html",
            "pdoc",
        ),
        (
            "C#",
            "dotnet add package Pamoja",
            "reference/dotnet.md",
            "reference/dotnet/index.html",
            "DocFX",
        ),
    ];
    let mut out = String::from(
        "| Language | Install | What it covers | Full API reference |\n| --- | --- | --- | --- |\n",
    );
    for (language, install, page, site, generator) in rows {
        let (page, site) = if absolute {
            (
                format!("{SITE}/{}.html", page.trim_end_matches(".md")),
                format!("{SITE}/{site}"),
            )
        } else {
            (page.to_owned(), site.to_owned())
        };
        out.push_str(&format!(
            "| {language} | `{install}` | [every package]({page}) | [{language} reference]({site}), generated by {generator} |\n"
        ));
    }
    out.trim_end().to_owned()
}

/// The button that opens one language's generated reference, the same shape on all four
/// pages. Each generator lands its output at a different entry point, and rustdoc emits no
/// index above the crates, so the Rust button opens the bundle crate, which re-exports
/// every other.
fn reference_link(language: &str) -> String {
    let (href, binding, subtitle, what) = match language {
        "rust" => (
            "rust/pamoja/index.html",
            "Rust",
            "Rust API reference",
            "Every crate, generated by rustdoc from this commit",
        ),
        "node" => (
            "node/index.html",
            "TypeScript",
            "TypeScript binding reference",
            "Every <code>@pamoja</code> package, generated by typedoc",
        ),
        "python" => (
            "python/pamoja.html",
            "Python",
            "Python binding reference",
            "Every <code>pamoja</code> module, generated by pdoc",
        ),
        _ => (
            "dotnet/index.html",
            "C#",
            "C# binding reference",
            "Every <code>Pamoja</code> package, generated by DocFX",
        ),
    };
    format!(
        "<p align=\"center\"><strong>{subtitle}</strong></p>

<p align=\"center\">
  <a href=\"{href}\"><img height=\"38\" alt=\"Open the {binding} API reference\" src=\"https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg\"></a>
</p>

<p align=\"center\">
  {what}. It is a site of its own; the tables on this page name what it documents,
  and every name below opens its page there.
</p>"
    )
}

/// The URL of a module's page in the Python reference on the site.
pub fn python_reference_url(module: &str) -> String {
    format!("{SITE}/reference/python/pamoja/{module}.html")
}

/// The URL of a package's namespace page in the C# reference on the site. The namespace
/// page lists every type the package defines, so it is the one link the package needs.
pub fn dotnet_reference_url(package: &str) -> String {
    format!("{SITE}/reference/dotnet/api/{package}.html")
}

/// The URL of a crate's rustdoc on the site.
fn rustdoc_url(krate: &str) -> String {
    format!(
        "{SITE}/reference/rust/{}/index.html",
        krate.replace('-', "_")
    )
}

/// The `[[name]]` tables of a document.
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

fn strings(
    table: &dyn toml_edit::TableLike,
    key: &str,
    context: &str,
) -> Result<Vec<String>, String> {
    strings_of(table, key, context)
}

fn strings_of(
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

/// The subpath exports of the Node package, without `.` and `./raw`.
fn node_packages(root: &Path) -> Result<BTreeSet<String>, String> {
    let dir = root.join("bindings/node/packages");
    let entries = fs::read_dir(&dir).map_err(|err| format!("reading {}: {err}", dir.display()))?;
    Ok(entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("src/index.ts").is_file())
        .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
        .filter(|name| name != "core" && name != "native" && name != "pamoja")
        .collect())
}

/// The URL of a capability package's page in the generated TypeScript reference.
pub fn node_reference_url(key: &str) -> String {
    format!("{SITE}/reference/node/modules/_pamoja_{key}.html")
}

/// The npm package a capability lives in: its own `@pamoja/<key>`, or `@pamoja/core`
/// for the transport surface the engine carries itself.
pub fn node_package(capability: &Capability) -> String {
    format!("@pamoja/{}", capability.node)
}

/// The facade modules of the Python package, without `__init__` and `raw`.
fn python_packages(root: &Path) -> Result<BTreeSet<String>, String> {
    let dir = root.join("bindings/python/packages");
    let entries = fs::read_dir(&dir).map_err(|err| format!("reading {}: {err}", dir.display()))?;
    Ok(entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_owned();
            // A domain's directory keeps the map's key, `field-io`, while its module is the
            // identifier `field_io`, since a hyphen cannot appear in a Python module name.
            let module = name.replace('-', "_");
            path.join("pamoja")
                .join(&module)
                .join("__init__.py")
                .is_file()
                .then_some(name)
        })
        .filter(|name| name != "core" && name != "pamoja")
        .collect())
}

/// Every type declared in the .NET binding's sources.
fn dotnet_types(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut types = BTreeSet::new();
    for project in dotnet_project_dirs(root)? {
        let entries = fs::read_dir(&project)
            .map_err(|err| format!("reading {}: {err}", project.display()))?;
        for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
            if path.extension().and_then(|ext| ext.to_str()) != Some("cs") {
                continue;
            }
            let text = fs::read_to_string(&path)
                .map_err(|err| format!("reading {}: {err}", path.display()))?;
            types.extend(declared_types(&text));
        }
    }
    Ok(types)
}

/// Every project directory under `bindings/dotnet/src`.
fn dotnet_project_dirs(root: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let dir = root.join("bindings/dotnet/src");
    let entries = fs::read_dir(&dir).map_err(|err| format!("reading {}: {err}", dir.display()))?;
    let mut dirs: Vec<std::path::PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// The capability packages under `bindings/dotnet/src`: every `Pamoja.<Name>` project
/// other than the engine's own (`Core`, `Native`) and the metapackage.
fn dotnet_packages(root: &Path) -> Result<BTreeSet<String>, String> {
    Ok(dotnet_project_dirs(root)?
        .into_iter()
        .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
        .filter_map(|name| name.strip_prefix("Pamoja.").map(str::to_owned))
        .filter(|name| name != "Core" && name != "Native")
        .collect())
}

/// The .NET package a capability key becomes: the key with its first letter raised.
pub fn dotnet_name(key: &str) -> String {
    let mut chars = key.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// The names declared by `class`, `record`, `struct`, `enum`, and `interface` in C# source.
fn declared_types(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut tokens = source.split_whitespace().peekable();
    while let Some(token) = tokens.next() {
        if !matches!(token, "class" | "record" | "struct" | "enum" | "interface") {
            continue;
        }
        let Some(next) = tokens.peek() else {
            break;
        };
        // `record struct Name` names its kind twice.
        if *next == "struct" || *next == "class" {
            continue;
        }
        let name: String = next
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            names.push(name);
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[[chapter]]
key = "field-io"
title = "Field I/O"
intent = "The wires a gateway has."

[[capability]]
key = "modbus"
chapter = "field-io"
title = "Modbus RTU"
summary = "Modbus RTU requests and replies"
crates = ["pamoja-modbus"]
node = "modbus"
python = "modbus"
dotnet = ["Modbus", "ModbusFrame"]
guide = "guides/modbus.md"

[[capability]]
key = "transport"
chapter = "field-io"
title = "Transports"
summary = "The transport surface"
crates = []
node = "core"
python = "core"
dotnet = ["Transport"]

[engine]
crates = ["pamoja-core"]

[bundle]
crate = "pamoja"
"#;

    #[test]
    fn parses_the_map() {
        let catalog = Catalog::parse(SAMPLE).unwrap();
        assert_eq!(catalog.chapters.len(), 1);
        assert_eq!(catalog.capabilities.len(), 2);
        assert_eq!(catalog.engine, ["pamoja-core"]);
        assert_eq!(catalog.bundle.as_deref(), Some("pamoja"));
        let modbus = catalog.capability("modbus").unwrap();
        assert_eq!(modbus.dotnet, ["Modbus", "ModbusFrame"]);
        assert_eq!(modbus.guide.as_deref(), Some("guides/modbus.md"));
        assert!(catalog.capability("transport").unwrap().guide.is_none());
    }

    #[test]
    fn renders_the_tables() {
        let catalog = Catalog::parse(SAMPLE).unwrap();
        let descriptions = BTreeMap::from([
            ("pamoja-modbus".to_owned(), "Modbus RTU framing".to_owned()),
            ("pamoja-core".to_owned(), "The device model".to_owned()),
            ("pamoja".to_owned(), "Everything in one crate".to_owned()),
        ]);

        let chapters = catalog.render("chapters", &descriptions).unwrap();
        assert!(chapters.contains("| Field I/O | [Modbus RTU](https://pamoja.molex.cloud/docs/guides/modbus.html), Transports | [`pamoja-modbus`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html) |"));
        assert!(chapters.contains("| Engine |"));
        assert!(chapters.ends_with("| Everything | `cargo add pamoja`: every capability above, behind a feature each | [`pamoja`](https://pamoja.molex.cloud/docs/reference/rust/pamoja/index.html) |"));

        let guides = catalog.render("guides", &descriptions).unwrap();
        assert!(guides.starts_with("### Field I/O\n\nThe wires a gateway has.\n\n- [Modbus RTU](guides/modbus.md) - Modbus RTU requests and replies\n- Transports - The transport surface"));

        let crates = catalog.render("crates", &descriptions).unwrap();
        assert!(crates.contains("| **Engine** | [`pamoja-core`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html) | The device model |"));
        assert!(crates.starts_with("| Chapter | Crate | What it does |\n| --- | --- | --- |\n| **Everything** | [`pamoja`](https://pamoja.molex.cloud/docs/reference/rust/pamoja/index.html) | Everything in one crate |"));

        let reference = catalog.render("reference modbus", &descriptions).unwrap();
        assert!(reference.contains("- TypeScript: [`@pamoja/modbus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html)"));
        assert!(reference.contains("- Rust: [`pamoja-modbus`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html)"));
        assert!(reference.contains("- C#: [`Pamoja.Modbus`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html)"));

        let binding = catalog.render("binding python", &descriptions).unwrap();
        assert!(binding.contains("| **Field I/O** | [Modbus RTU](https://pamoja.molex.cloud/docs/guides/modbus.html) | [`pamoja.modbus`](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html) | Modbus RTU requests and replies |"));
        assert!(binding.contains("| **Engine** | Transports | [`pamoja.core`](https://pamoja.molex.cloud/docs/reference/python/pamoja/core.html) | The transport surface |"));

        assert!(catalog.render("reference nothing", &descriptions).is_err());
        assert!(catalog.render("binding lua", &descriptions).is_err());
    }

    #[test]
    fn renders_the_summary_with_a_part_per_chapter_that_has_guides() {
        let catalog = Catalog::parse(SAMPLE).unwrap();
        let summary = catalog.summary();
        assert!(summary.contains("\n# Field I/O\n\n- [Modbus RTU](guides/modbus.md)\n"));
        assert!(!summary.contains("Transports"));
        assert!(summary.contains("- [Releasing](about/releasing.md)\n"));
    }

    #[test]
    fn finds_declared_dotnet_types() {
        let source = "public sealed class Modbus : IDisposable { }\npublic readonly record struct Pose(double X);\npublic enum Qos { AtMostOnce }\ninternal interface IHandle<T> { }";
        assert_eq!(declared_types(source), ["Modbus", "Pose", "Qos", "IHandle"]);
    }
}
