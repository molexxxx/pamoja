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
    /// The Markdown kinds render anywhere, the registries included: `chapters` (the
    /// capability map by chapter), `crates` (every crate with its reference links, or
    /// `crates engine` for the engine and the bundle alone), `reference <capability>`
    /// (the per-language reference links of one guide), `binding <node|python|dotnet>`
    /// (the capability table of one binding README), `domains <language>` (the install
    /// line per domain), and `references` (the four languages with their reference
    /// pages). The HTML kinds are for the site's own pages: `packages <language>` (one
    /// row per capability with its install line, its reference, its worked example, and
    /// the other registries), `install <language>` (the same for the six domains), and
    /// `reference-link <language>` (the head of a reference page, with the other three
    /// languages beside it).
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
            ("crates", None) => Ok(self.crates_table(crate_descriptions, false)),
            ("crates", Some("engine")) => Ok(self.crates_table(crate_descriptions, true)),
            ("packages", Some(language @ ("rust" | "node" | "python" | "dotnet"))) => {
                Ok(self.packages_block(language))
            }
            ("install", Some(language @ ("rust" | "node" | "python" | "dotnet"))) => {
                Ok(self.install_block(language))
            }
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

    fn crates_table(&self, descriptions: &BTreeMap<String, String>, engine_only: bool) -> String {
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
        if !engine_only {
            for chapter in &self.chapters {
                for capability in self.in_chapter(&chapter.key) {
                    for krate in &capability.crates {
                        rows.push((chapter.title.clone(), krate.clone()));
                    }
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

    /// One row per capability for the site's reference and install pages: the title linking
    /// the guide, the install line with a copy button, the name a program uses and where its
    /// reference is, the worked example, the registry page, and the same capability on the
    /// other three registries. Grouped under a heading per chapter, the engine's own surface
    /// first, so the page reads the way the guides are arranged.
    fn packages_block(&self, language: &str) -> String {
        let lang = Language::by_key(language);
        let mut out = String::new();
        let mut last = String::new();
        for capability in self.ordered() {
            let chapter = if capability.node == "core" {
                "Engine".to_owned()
            } else {
                self.chapter_title(&capability.chapter)
            };
            if chapter != last {
                if !last.is_empty() {
                    out.push_str("</div>\n\n");
                }
                out.push_str(&format!("### {chapter}\n\n<div class=\"pkgs\">\n"));
                last = chapter;
            }
            out.push_str(&package_row(lang, capability));
        }
        out.push_str("</div>");
        out
    }

    /// The six domains as install rows for the site: the install line with a copy button,
    /// the domain linked to its registry page where it is a package, and the capabilities
    /// it brings in, each linking its guide.
    fn install_block(&self, language: &str) -> String {
        let lang = Language::by_key(language);
        let mut out = String::from("<div class=\"domains\">\n");
        for (chapter, members) in self.domains() {
            let title = match lang.domain_package(&chapter.key) {
                Some(package) => format!(
                    "<a href=\"{}\">{}</a>",
                    lang.registry_url(&package),
                    escape(&chapter.title)
                ),
                None => escape(&chapter.title),
            };
            let names: Vec<String> = members
                .iter()
                .map(|member| match guide_url(member) {
                    Some(url) => format!(
                        "<a href=\"{}\">{}</a>",
                        site_relative(&url),
                        escape(&member.title)
                    ),
                    None => escape(&member.title),
                })
                .collect();
            out.push_str(&format!(
                "<div class=\"domain\">\n<div class=\"domain-what\"><strong>{title}</strong><p>{}</p></div>\n{}\n</div>\n",
                names.join(", "),
                command(&lang.domain_install(&chapter.key))
            ));
        }
        out.push_str("</div>");
        out
    }

    fn chapter_title(&self, key: &str) -> String {
        self.chapters
            .iter()
            .find(|chapter| chapter.key == key)
            .map(|chapter| chapter.title.clone())
            .unwrap_or_default()
    }
}

/// One language's packaging: how a capability is named and installed there, and which
/// registry holds it.
struct Language {
    key: &'static str,
    name: &'static str,
    registry: &'static str,
    /// The fragment of the guide section that shows the language's example.
    anchor: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        key: "rust",
        name: "Rust",
        registry: "crates.io",
        anchor: "rust",
    },
    Language {
        key: "node",
        name: "TypeScript",
        registry: "npm",
        anchor: "typescript",
    },
    Language {
        key: "python",
        name: "Python",
        registry: "PyPI",
        anchor: "python",
    },
    Language {
        key: "dotnet",
        name: "C#",
        registry: "NuGet",
        anchor: "c",
    },
];

impl Language {
    fn by_key(key: &str) -> &'static Language {
        LANGUAGES
            .iter()
            .find(|language| language.key == key)
            .expect("one of the four languages")
    }

    /// The package that carries every capability in this language.
    fn bundle(&self) -> &'static str {
        match self.key {
            "dotnet" => "Pamoja",
            _ => "pamoja",
        }
    }

    /// What one unit of the generated reference is called in this language.
    fn unit(&self) -> &'static str {
        match self.key {
            "rust" => "crate",
            "python" => "module",
            _ => "package",
        }
    }

    /// The tool that generates this language's reference.
    fn generator(&self) -> &'static str {
        match self.key {
            "rust" => "rustdoc",
            "node" => "typedoc",
            "python" => "pdoc",
            _ => "DocFX",
        }
    }

    /// The package a capability is in this language.
    fn package(&self, capability: &Capability) -> String {
        match self.key {
            "rust" => capability
                .crates
                .first()
                .cloned()
                .unwrap_or_else(|| "pamoja-core".to_owned()),
            "node" => node_package(capability),
            "python" => format!("pamoja-{}", capability.python),
            _ => capability.dotnet_package(),
        }
    }

    /// What a reader types to install `package`.
    fn install(&self, package: &str) -> String {
        match self.key {
            "rust" => format!("cargo add {package}"),
            "node" => format!("npm install {package}"),
            "python" => format!("pip install {package}"),
            _ => format!("dotnet add package {package}"),
        }
    }

    /// The registry page of `package`.
    fn registry_url(&self, package: &str) -> String {
        match self.key {
            "rust" => format!("https://crates.io/crates/{package}"),
            "node" => format!("https://www.npmjs.com/package/{package}"),
            "python" => format!("https://pypi.org/project/{package}/"),
            _ => format!("https://www.nuget.org/packages/{package}"),
        }
    }

    /// The name a program uses for the capability, and its page in the generated
    /// reference, site-relative.
    fn import(&self, capability: &Capability) -> (String, String) {
        match self.key {
            "rust" => {
                let krate = self.package(capability);
                (krate.clone(), site_relative(&rustdoc_url(&krate)))
            }
            "node" => (
                node_package(capability),
                site_relative(&node_reference_url(&capability.node)),
            ),
            "python" => (
                format!("pamoja.{}", capability.python),
                site_relative(&python_reference_url(&capability.python)),
            ),
            _ => {
                let package = capability.dotnet_package();
                let href = site_relative(&dotnet_reference_url(&package));
                (package, href)
            }
        }
    }

    /// The package a domain is in this language, or none where it is a feature instead.
    fn domain_package(&self, chapter: &str) -> Option<String> {
        match self.key {
            "rust" => None,
            "node" => Some(format!("@pamoja/{chapter}")),
            "python" => Some(format!("pamoja-{chapter}")),
            _ => Some(format!(
                "Pamoja.{}",
                chapter
                    .split('-')
                    .map(dotnet_name)
                    .collect::<Vec<_>>()
                    .concat()
            )),
        }
    }

    /// What a reader types to install a domain.
    fn domain_install(&self, chapter: &str) -> String {
        match self.domain_package(chapter) {
            Some(package) => self.install(&package),
            None => format!("cargo add pamoja --features {chapter}"),
        }
    }
}

// One capability's row: what it is, how to install it, where to read about it, and where
// else it lives.
fn package_row(lang: &Language, capability: &Capability) -> String {
    let package = lang.package(capability);
    let (import, reference) = lang.import(capability);
    let example =
        guide_url(capability).map(|url| format!("{}#{}", site_relative(&url), lang.anchor));
    let title = match &example {
        Some(href) => format!("<a href=\"{href}\">{}</a>", escape(&capability.title)),
        None => escape(&capability.title),
    };
    let mut links = vec![format!(
        "<li><a href=\"{reference}\"><code>{}</code></a></li>",
        escape(&import)
    )];
    if lang.key == "rust" {
        links.push(format!(
            "<li><a href=\"https://docs.rs/{package}\">docs.rs</a></li>"
        ));
    }
    if let Some(href) = &example {
        links.push(format!("<li><a href=\"{href}\">worked example</a></li>"));
    }
    links.push(format!(
        "<li><a href=\"{}\">{}</a></li>",
        lang.registry_url(&package),
        lang.registry
    ));
    let others: Vec<String> = LANGUAGES
        .iter()
        .filter(|other| other.key != lang.key)
        .map(|other| {
            let package = other.package(capability);
            format!(
                "<a href=\"{}\" title=\"{}\">{}</a>",
                other.registry_url(&package),
                escape(&package),
                other.name
            )
        })
        .collect();
    format!(
        "<div class=\"pkg\">\n<div class=\"pkg-what\">{title}<p>{}</p></div>\n{}\n<ul class=\"pkg-links\">{}</ul>\n<p class=\"pkg-else\"><span>Also in</span> {}</p>\n</div>\n",
        escape(&capability.summary),
        command(&lang.install(&package)),
        links.join(""),
        others.join(" ")
    )
}

// An install line with the button that copies it.
fn command(text: &str) -> String {
    let text = escape(text);
    format!(
        "<div class=\"pkg-get\"><code class=\"cmd\">{text}</code><button class=\"copy\" type=\"button\" data-copy=\"{text}\" aria-label=\"Copy the install command\">copy</button></div>"
    )
}

/// A site URL as the site's own pages link it, so the link check covers it and a local copy
/// of the site resolves it too.
fn site_relative(url: &str) -> String {
    url.strip_prefix("https://pamoja.molex.cloud")
        .map_or_else(|| url.to_owned(), str::to_owned)
}

/// Escape text for an HTML text node or a double-quoted attribute.
pub(crate) fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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

/// The four languages on the front page and in the root README: what a reader installs,
/// and the page on this site that lists every package and opens each one's API pages.
/// That page is the one way into a generated reference; the trees' own roots hand off to
/// it. Relative, since only the site front page carries it, unless `absolute` is set, which
/// the root README needs since a registry renders it away from the site.
fn references(absolute: bool) -> String {
    let mut out = String::from("| Language | Install | Reference |\n| --- | --- | --- |\n");
    for language in &LANGUAGES {
        let href = if absolute {
            format!("{SITE}/reference/{}.html", language.key)
        } else {
            format!("reference/{}.md", language.key)
        };
        out.push_str(&format!(
            "| {} | `{}` | [{} reference]({href}), every {} with its API pages, generated by {} |\n",
            language.name,
            language.install(language.bundle()),
            language.name,
            language.unit(),
            language.generator()
        ));
    }
    out.trim_end().to_owned()
}

/// The head of one language's reference page, the same shape on all four: what the rows
/// below open, and the other three languages. The page is the only door into the generated
/// reference; each tree's own root redirects here, so there is no second front page to
/// keep in step.
fn reference_link(language: &str) -> String {
    let lang = Language::by_key(language);
    let what = match language {
        "rust" => "Every crate, generated by rustdoc from this commit.",
        "node" => "Every <code>@pamoja</code> package, generated by typedoc from this commit.",
        "python" => "Every <code>pamoja</code> module, generated by pdoc from this commit.",
        _ => "Every <code>Pamoja</code> package, generated by DocFX from this commit.",
    };
    let switcher: Vec<String> = LANGUAGES
        .iter()
        .map(|other| {
            if other.key == language {
                format!("<span aria-current=\"page\">{}</span>", other.name)
            } else {
                format!("<a href=\"{}.html\">{}</a>", other.key, other.name)
            }
        })
        .collect();
    format!(
        "<div class=\"door\">\n<p>{what} Each row below opens a {}'s API pages, and the same capability in the other three languages is one step away.</p>\n<nav class=\"door-langs\" aria-label=\"The other languages\">{}</nav>\n</div>",
        lang.unit(),
        switcher.join("\n")
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
pub fn rustdoc_url(krate: &str) -> String {
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
    fn renders_the_package_rows_for_the_site() {
        let catalog = Catalog::parse(SAMPLE).unwrap();
        let descriptions = BTreeMap::new();

        let python = catalog.render("packages python", &descriptions).unwrap();
        assert!(python.starts_with("### Engine\n\n<div class=\"pkgs\">\n<div class=\"pkg\">\n<div class=\"pkg-what\">Transports<p>The transport surface</p></div>"), "{python}");
        assert!(python.contains("### Field I/O\n\n<div class=\"pkgs\">\n<div class=\"pkg\">\n<div class=\"pkg-what\"><a href=\"/docs/guides/modbus.html#python\">Modbus RTU</a>"));
        assert!(python.contains("<code class=\"cmd\">pip install pamoja-modbus</code><button class=\"copy\" type=\"button\" data-copy=\"pip install pamoja-modbus\""));
        assert!(python.contains("<li><a href=\"/docs/reference/python/pamoja/modbus.html\"><code>pamoja.modbus</code></a></li>"));
        assert!(python.contains("<li><a href=\"/docs/guides/modbus.html#python\">worked example</a></li><li><a href=\"https://pypi.org/project/pamoja-modbus/\">PyPI</a></li>"));
        assert!(python.contains("<span>Also in</span> <a href=\"https://crates.io/crates/pamoja-modbus\" title=\"pamoja-modbus\">Rust</a> <a href=\"https://www.npmjs.com/package/@pamoja/modbus\" title=\"@pamoja/modbus\">TypeScript</a> <a href=\"https://www.nuget.org/packages/Pamoja.Modbus\" title=\"Pamoja.Modbus\">C#</a>"));
        assert!(python.ends_with("</div>\n</div>"));

        let rust = catalog.render("packages rust", &descriptions).unwrap();
        assert!(
            rust.contains("<code class=\"cmd\">cargo add pamoja-core</code>"),
            "the engine surface is the core crate"
        );
        assert!(rust.contains("<li><a href=\"/docs/reference/rust/pamoja_modbus/index.html\"><code>pamoja-modbus</code></a></li><li><a href=\"https://docs.rs/pamoja-modbus\">docs.rs</a></li>"));

        let dotnet = catalog.render("packages dotnet", &descriptions).unwrap();
        assert!(dotnet.contains("<code class=\"cmd\">dotnet add package Pamoja.Modbus</code>"));
        assert!(dotnet.contains("<a href=\"/docs/guides/modbus.html#c\">Modbus RTU</a>"));
    }

    #[test]
    fn renders_the_domain_install_rows_and_the_reference_door() {
        let two = format!(
            "{SAMPLE}\n[[capability]]\nkey = \"can\"\nchapter = \"field-io\"\ntitle = \"CAN\"\nsummary = \"CAN frames\"\ncrates = [\"pamoja-can\"]\nnode = \"can\"\npython = \"can\"\ndotnet = [\"Can\"]\nguide = \"guides/can.md\"\n"
        );
        let catalog = Catalog::parse(&two).unwrap();
        let descriptions = BTreeMap::new();

        let node = catalog.render("install node", &descriptions).unwrap();
        assert!(node.starts_with("<div class=\"domains\">\n<div class=\"domain\">\n<div class=\"domain-what\"><strong>"), "{node}");
        assert!(
            node.contains(
                "<div class=\"pkg-get\"><code class=\"cmd\">npm install @pamoja/field-io</code>"
            ),
            "{node}"
        );
        assert!(node.contains("<strong><a href=\"https://www.npmjs.com/package/@pamoja/field-io\">Field I/O</a></strong><p><a href=\"/docs/guides/modbus.html\">Modbus RTU</a>, Transports, <a href=\"/docs/guides/can.html\">CAN</a></p>"));

        let rust = catalog.render("install rust", &descriptions).unwrap();
        assert!(rust.contains("<code class=\"cmd\">cargo add pamoja --features field-io</code>"));
        assert!(
            rust.contains("<strong>Field I/O</strong>"),
            "a feature has no registry page"
        );

        let dotnet = catalog.render("install dotnet", &descriptions).unwrap();
        assert!(dotnet.contains("dotnet add package Pamoja.FieldIo"));

        let door = catalog
            .render("reference-link python", &descriptions)
            .unwrap();
        assert!(door.starts_with("<div class=\"door\">\n<p>Every <code>pamoja</code> module, generated by pdoc from this commit. Each row below opens a module's API pages"), "{door}");
        assert!(
            !door.contains("pamoja.html"),
            "the generated root is not linked"
        );

        let references = catalog
            .render("references absolute", &descriptions)
            .unwrap();
        assert!(references.starts_with("| Language | Install | Reference |\n| --- | --- | --- |\n| Rust | `cargo add pamoja` | [Rust reference](https://pamoja.molex.cloud/docs/reference/rust.html), every crate with its API pages, generated by rustdoc |"), "{references}");
        assert!(references.contains("| C# | `dotnet add package Pamoja` | [C# reference](https://pamoja.molex.cloud/docs/reference/dotnet.html), every package with its API pages, generated by DocFX |"));
        assert!(
            !references.contains("pamoja/index.html"),
            "the generated roots are not linked"
        );
        let relative = catalog.render("references", &descriptions).unwrap();
        assert!(relative.contains("[Python reference](reference/python.md)"));
        assert!(door.contains("<a href=\"rust.html\">Rust</a>\n<a href=\"node.html\">TypeScript</a>\n<span aria-current=\"page\">Python</span>\n<a href=\"dotnet.html\">C#</a>"));

        let engine = catalog
            .render(
                "crates engine",
                &BTreeMap::from([("pamoja-core".to_owned(), "The device model".to_owned())]),
            )
            .unwrap();
        assert!(engine.contains("pamoja-core") && !engine.contains("pamoja-modbus"));
    }

    #[test]
    fn finds_declared_dotnet_types() {
        let source = "public sealed class Modbus : IDisposable { }\npublic readonly record struct Pose(double X);\npublic enum Qos { AtMostOnce }\ninternal interface IHandle<T> { }";
        assert_eq!(declared_types(source), ["Modbus", "Pose", "Qos", "IHandle"]);
    }
}
