//! The packages of the Node binding, rendered from the capability map: a manifest, a
//! TypeScript project, and a README for `@pamoja/core` and for each `@pamoja/<key>`
//! capability package, the `pamoja` bundle that depends on all of them, the README of
//! `@pamoja/native`, and the workspace's project references. A package's dependencies
//! are derived from its own imports, so a facade that starts using another package
//! declares it on the next `cargo xtask docs`.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::catalog::{dotnet_name, node_reference_url, Capability, Catalog, Chapter, SITE};
use crate::regions;

/// The repository URL every manifest points at.
const REPOSITORY: &str = "git+https://github.com/molexxxx/pamoja.git";

/// The package that carries the compiled engine and the generated contract. It is
/// not a TypeScript project, so it is a dependency but never a project reference.
const NATIVE: &str = "native";

/// A JSON value whose object keys keep the order they were given, so a manifest reads
/// the way npm writes one and an `exports` map keeps `types` ahead of `default`.
enum Json {
    Str(String),
    Bool(bool),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn str(text: impl Into<String>) -> Json {
        Json::Str(text.into())
    }

    fn strings<I, S>(items: I) -> Json
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Json::Array(
            items
                .into_iter()
                .map(|item| Json::Str(item.into()))
                .collect(),
        )
    }

    fn object(fields: Vec<(&str, Json)>) -> Json {
        Json::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    fn render(&self, out: &mut String, indent: usize) {
        match self {
            Json::Str(text) => {
                out.push('"');
                for c in text.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        c => out.push(c),
                    }
                }
                out.push('"');
            }
            Json::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
            Json::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push_str("[\n");
                for (index, item) in items.iter().enumerate() {
                    out.push_str(&" ".repeat(indent + 2));
                    item.render(out, indent + 2);
                    if index + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&" ".repeat(indent));
                out.push(']');
            }
            Json::Object(fields) => {
                if fields.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push_str("{\n");
                for (index, (key, value)) in fields.iter().enumerate() {
                    out.push_str(&" ".repeat(indent + 2));
                    out.push('"');
                    out.push_str(key);
                    out.push_str("\": ");
                    value.render(out, indent + 2);
                    if index + 1 < fields.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&" ".repeat(indent));
                out.push('}');
            }
        }
    }
}

/// Render every generated file of the Node workspace as (path, contents).
///
/// # Errors
///
/// Returns the reason when a package's source cannot be read.
pub fn render_node(
    root: &Path,
    catalog: &Catalog,
    version: &str,
) -> Result<Vec<(String, String)>, String> {
    let workspace = root.join("bindings/node");
    let mut files = Vec::new();

    let core_deps = package_imports(&workspace.join("packages/core"))?;
    files.extend(package_files(
        "core",
        &core_deps,
        &manifest(
            "core",
            version,
            "The pamoja engine's surface for Node: the runtime version and the transport every link shares, the counterpart of the pamoja-core crate.",
            &format!("{SITE}/"),
            &["pamoja", "iot", "robotics", "core"],
            &core_deps,
            &["transport"],
        ),
        core_readme(),
    ));

    let mut keys: Vec<&str> = Vec::new();
    for capability in &catalog.capabilities {
        let key = capability.node.as_str();
        if key == "core" {
            continue;
        }
        keys.push(key);
        let mut deps = package_imports(&workspace.join("packages").join(key))?;
        deps.remove(key);
        files.extend(package_files(
            key,
            &deps,
            &manifest(
                key,
                version,
                &format!("{}.", capability.summary),
                &homepage(capability),
                &["pamoja", "iot", "robotics", key],
                &deps,
                &[],
            ),
            capability_readme(root, catalog, capability, key)?,
        ));
    }

    for (chapter, members) in catalog.domains() {
        let deps: BTreeSet<String> = members
            .iter()
            .map(|capability| capability.node.clone())
            .collect();
        files.extend(package_files(
            &chapter.key,
            &deps,
            &manifest(
                &chapter.key,
                version,
                &format!("{}: {}", chapter.title, chapter.intent),
                &format!("{SITE}/install.html"),
                &["pamoja", "iot", "robotics", &chapter.key],
                &deps,
                &[],
            ),
            domain_readme(chapter, &members),
        ));
        files.push((
            format!("bindings/node/packages/{}/src/index.ts", chapter.key),
            domain_entry(chapter, &members),
        ));
    }

    let all: BTreeSet<String> = keys
        .iter()
        .map(|key| (*key).to_owned())
        .chain(["core".to_owned(), NATIVE.to_owned()])
        .collect();
    files.extend(package_files(
        "pamoja",
        &all,
        &manifest(
            "pamoja",
            version,
            "The whole pamoja framework in one package: every capability of one memory-safe Rust core, behind an idiomatic TypeScript facade, for IoT, robotics, and drones.",
            &format!("{SITE}/"),
            &["pamoja", "iot", "robotics", "drones", "mqtt", "embedded"],
            &all,
            &[],
        ),
        bundle_readme(catalog),
    ));

    files.push((
        format!("bindings/node/packages/{NATIVE}/README.md"),
        native_readme(),
    ));

    let mut references = vec![reference("packages/core")];
    references.extend(keys.iter().map(|key| reference(&format!("packages/{key}"))));
    references.extend(
        catalog
            .domains()
            .iter()
            .map(|(chapter, _)| reference(&format!("packages/{}", chapter.key))),
    );
    references.push(reference("packages/pamoja"));
    files.push((
        "bindings/node/tsconfig.json".to_owned(),
        pretty(&Json::object(vec![
            ("files", Json::Array(Vec::new())),
            ("references", Json::Array(references)),
        ])),
    ));

    Ok(files)
}

/// The three generated files of one TypeScript package.
fn package_files(
    key: &str,
    deps: &BTreeSet<String>,
    manifest: &Json,
    readme: String,
) -> Vec<(String, String)> {
    vec![
        (
            format!("bindings/node/packages/{key}/package.json"),
            pretty(manifest),
        ),
        (
            format!("bindings/node/packages/{key}/tsconfig.json"),
            pretty(&tsconfig(deps)),
        ),
        (format!("bindings/node/packages/{key}/README.md"), readme),
    ]
}

/// A package manifest. The bundle is the bare `pamoja`; every other package is scoped.
/// `subpaths` are the extra entry points a package exports beside its root, each a
/// module of the same name under `dist/`.
fn manifest(
    key: &str,
    version: &str,
    description: &str,
    homepage: &str,
    keywords: &[&str],
    deps: &BTreeSet<String>,
    subpaths: &[&str],
) -> Json {
    let name = if key == "pamoja" {
        "pamoja".to_owned()
    } else {
        format!("@pamoja/{key}")
    };
    let mut exports = vec![(".".to_owned(), entry("index"))];
    for subpath in subpaths {
        exports.push((format!("./{subpath}"), entry(subpath)));
    }
    Json::object(vec![
        ("name", Json::str(name)),
        ("version", Json::str(version)),
        ("description", Json::str(description)),
        ("license", Json::str("MIT")),
        (
            "publishConfig",
            Json::object(vec![("access", Json::str("public"))]),
        ),
        (
            "repository",
            Json::object(vec![
                ("type", Json::str("git")),
                ("url", Json::str(REPOSITORY)),
                (
                    "directory",
                    Json::str(format!("bindings/node/packages/{key}")),
                ),
            ]),
        ),
        ("homepage", Json::str(homepage)),
        ("keywords", Json::strings(keywords.iter().copied())),
        ("main", Json::str("dist/index.js")),
        ("types", Json::str("dist/index.d.ts")),
        ("exports", Json::Object(exports)),
        ("files", Json::strings(["dist/"])),
        ("engines", Json::object(vec![("node", Json::str(">= 16"))])),
        ("dependencies", pins(deps, version)),
    ])
}

/// One `exports` entry: the declaration first, so TypeScript matches it before `default`.
fn entry(module: &str) -> Json {
    Json::object(vec![
        ("types", Json::str(format!("./dist/{module}.d.ts"))),
        ("default", Json::str(format!("./dist/{module}.js"))),
    ])
}

/// One project reference.
fn reference(path: &str) -> Json {
    Json::object(vec![("path", Json::str(path))])
}

/// The `@pamoja/<name>` packages the TypeScript sources under a package's `src/` import.
fn package_imports(package: &Path) -> Result<BTreeSet<String>, String> {
    let src = package.join("src");
    let entries = fs::read_dir(&src).map_err(|err| format!("reading {}: {err}", src.display()))?;
    let mut names = BTreeSet::new();
    for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
        if path.extension().and_then(|ext| ext.to_str()) != Some("ts") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        names.extend(node_imports(&text));
    }
    Ok(names)
}

/// The `@pamoja/<name>` packages a TypeScript source imports from.
fn node_imports(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut rest = source;
    while let Some(at) = rest.find("'@pamoja/") {
        let after = &rest[at + "'@pamoja/".len()..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !name.is_empty() {
            names.insert(name);
        }
        rest = after;
    }
    names
}

/// The dependency map of a package: every name pinned to the workspace version.
fn pins(names: &BTreeSet<String>, version: &str) -> Json {
    Json::Object(
        names
            .iter()
            .map(|name| (format!("@pamoja/{name}"), Json::str(version)))
            .collect(),
    )
}

/// A package's TypeScript project: the shared options, and a reference to each
/// TypeScript package it depends on so `tsc -b` builds them first.
fn tsconfig(deps: &BTreeSet<String>) -> Json {
    let references: Vec<Json> = deps
        .iter()
        .filter(|dep| dep.as_str() != NATIVE)
        .map(|dep| reference(&format!("../{dep}")))
        .collect();
    Json::object(vec![
        ("extends", Json::str("../../tsconfig.base.json")),
        (
            "compilerOptions",
            Json::object(vec![
                ("rootDir", Json::str("src")),
                ("outDir", Json::str("dist")),
                ("composite", Json::Bool(true)),
            ]),
        ),
        ("include", Json::strings(["src/**/*.ts"])),
        ("references", Json::Array(references)),
    ])
}

/// The guide's URL when the capability has one, else the site's front page.
fn homepage(capability: &Capability) -> String {
    match &capability.guide {
        Some(guide) => format!("{SITE}/{}.html", guide.strip_suffix(".md").unwrap_or(guide)),
        None => format!("{SITE}/"),
    }
}

/// The README of one capability package: what it is, how to install it, the guide's
/// example when the guide exists, and where the documentation is.
/// The entry point of a domain package: every capability of the domain re-exported flat,
/// and again under the capability's own name. A name two capabilities of the domain both
/// export is ambiguous, so `export *` leaves it out and the namespaced form reaches it.
fn domain_entry(chapter: &Chapter, members: &[&Capability]) -> String {
    let names: Vec<&str> = members.iter().map(|c| c.node.as_str()).collect();
    let installs = names
        .iter()
        .map(|name| format!("`@pamoja/{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = format!(
        "/**\n * {}: {}\n *\n * Installing this package installs {installs}, and re-exports each under its own\n * name, so a name two of them share stays unambiguous.\n *\n * @packageDocumentation\n */\n\n",
        chapter.title, chapter.intent
    );
    for name in &names {
        out.push_str(&format!(
            "export * as {} from '@pamoja/{name}'\n",
            camel(name)
        ));
    }
    out
}

/// The README of a domain package.
fn domain_readme(chapter: &Chapter, members: &[&Capability]) -> String {
    let mut out = format!(
        "# @pamoja/{}\n\n{}\n\nOne install for the {} capabilities of this domain. Each is also its own package, and\n`pamoja` is the whole framework in one.\n\n```sh\nnpm install @pamoja/{}\n```\n\n| Capability | Package | What it covers |\n| --- | --- | --- |\n",
        chapter.key,
        chapter.intent,
        members.len(),
        chapter.key
    );
    for capability in members {
        out.push_str(&format!(
            "| [{}]({SITE}/guides/{}.html) | `@pamoja/{}` | {} |\n",
            capability.title, capability.key, capability.node, capability.summary
        ));
    }
    out.push_str(&format!(
        "\nThe guides, with a worked TypeScript example for each, are at [{SITE}]({SITE}/).\n\n## License\n\nMIT\n"
    ));
    out
}

// A domain key as a JavaScript identifier: `field-io` is not one, `fieldIo` is.
fn camel(key: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for ch in key.chars() {
        if ch == '-' || ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn capability_readme(
    root: &Path,
    catalog: &Catalog,
    capability: &Capability,
    key: &str,
) -> Result<String, String> {
    let mut out = format!(
        "# @pamoja/{key}\n\n{}. One capability of [pamoja](https://github.com/molexxxx/pamoja), \
         one memory-safe Rust core with bindings for TypeScript, Python, and C#.\n\n\
         ## Install\n\n```sh\nnpm install @pamoja/{key}\n```\n\n\
         This pulls in `@pamoja/native`, the compiled engine, and nothing else. \
         `npm install pamoja` is the whole framework in one package.\n",
        capability.summary
    );

    let snippet = format!("bindings/node/guides/{}.ts", capability.key);
    if root.join(&snippet).is_file() {
        let example = regions::snippet(root, &format!("{snippet}#example"))?;
        out.push_str("\n## Example\n\nThe test that runs in CI, spliced here as it ran.\n\n");
        out.push_str(&example);
        out.push('\n');
    }

    out.push_str(&format!(
        "\n## The same capability in every language\n\n{}\n",
        catalog.cross_language(capability)
    ));

    out.push_str("\n## Documentation\n\n");
    if capability.guide.is_some() {
        out.push_str(&format!(
            "- [The {} guide]({}), with the same example in Rust, Python, and C#.\n",
            capability.title,
            homepage(capability)
        ));
    }
    out.push_str(&format!(
        "- [Every capability]({SITE}/), and the [install page]({SITE}/install.html).\n\n\
         ## License\n\nMIT\n"
    ));
    Ok(out)
}

/// The README of the `pamoja` bundle.
fn bundle_readme(catalog: &Catalog) -> String {
    let mut out = String::from(
        "# pamoja\n\n\
         The whole pamoja framework in one package: every capability of one memory-safe Rust \
         core, behind an idiomatic TypeScript facade, for IoT, robotics, and drones. Each \
         capability is also its own package, so an application that needs one thing can depend \
         on `@pamoja/mqtt` alone; this package depends on all of them and re-exports them.\n\n\
         ## Install\n\n```sh\nnpm install pamoja\n```\n\n\
         ## What it bundles\n\n| Package | What it covers |\n| --- | --- |\n",
    );
    for capability in catalog.ordered() {
        out.push_str(&format!(
            "| `@pamoja/{}` | {} |\n",
            capability.node, capability.summary
        ));
    }
    out.push_str(&format!(
        "\nAll of them run on `@pamoja/native`, the compiled engine, which is one binary \
         whichever packages you install.\n\n\
         ## Documentation\n\n\
         - [The guides]({SITE}/), one page per capability with the same example in Rust, TypeScript, Python, and C#.\n\
         - [The TypeScript reference]({SITE}/reference/node/index.html), generated from every package.\n\n\
         ## License\n\nMIT\n"
    ));
    out
}

/// The README of `@pamoja/core`.
fn core_readme() -> String {
    format!(
        "# @pamoja/core\n\n\
         The pamoja engine's surface for Node: the runtime version and the transport every \
         link shares. This is the counterpart of the `pamoja-core` crate, and like it, it is \
         small; the compiled engine lives in `@pamoja/native`, which this package depends on.\n\n\
         ## Install\n\n```sh\nnpm install @pamoja/core\n```\n\n\
         Each capability is its own package (`@pamoja/mqtt`, `@pamoja/security`, and so on) \
         and `npm install pamoja` is the whole framework in one package.\n\n\
         ## Documentation\n\n\
         - [The reference for this package]({}), generated from its source.\n\
         - [The guides]({SITE}/) and the [install page]({SITE}/install.html).\n\n\
         ## License\n\nMIT\n",
        node_reference_url("core")
    )
}

/// The README of `@pamoja/native`.
fn native_readme() -> String {
    format!(
        "# @pamoja/native\n\n\
         The compiled pamoja engine for Node, prebuilt for Linux (x64, arm64), macOS (x64, \
         arm64), and Windows (x64), and the generated napi-rs contract every `@pamoja` package \
         builds on. It is one binary that carries every capability; the capability packages are \
         facades over it, so picking packages narrows the API you depend on, not the size of \
         the engine.\n\n\
         You do not install this package directly. Every `@pamoja/<capability>` package and the \
         `pamoja` bundle depend on it. `index.d.ts` types the contract for anything a facade \
         does not cover.\n\n\
         ## Documentation\n\n\
         - [The guides]({SITE}/) and the [TypeScript reference]({SITE}/reference/node/index.html).\n\n\
         ## License\n\nMIT\n"
    )
}

/// Render every generated file of the Python packages as (path, contents): a
/// `pyproject.toml`, a README, and a `py.typed` marker for `pamoja-core` and each
/// `pamoja-<key>` capability package, the `pamoja` metapackage that depends on all of
/// them, and the README of `pamoja-native`, the maturin project under `packages/native`.
///
/// # Errors
///
/// Returns the reason when a package's source cannot be read.
pub fn render_python(
    root: &Path,
    catalog: &Catalog,
    version: &str,
) -> Result<Vec<(String, String)>, String> {
    let packages = root.join("bindings/python/packages");
    let mut files = Vec::new();

    let core_deps = python_package_imports(&packages.join("core/pamoja/core"), "core")?;
    files.extend(python_package_files(
        "core",
        version,
        "The pamoja engine's surface for Python: the runtime version, the error every native call raises, and the transport every link shares, the counterpart of the pamoja-core crate.",
        &format!("{SITE}/"),
        &["pamoja", "iot", "robotics", "core"],
        &core_deps,
        python_core_readme(),
    ));

    let mut keys: Vec<&str> = Vec::new();
    for capability in catalog.ordered() {
        let key = capability.python.as_str();
        if key == "core" {
            continue;
        }
        keys.push(key);
        let deps = python_package_imports(&packages.join(key).join("pamoja").join(key), key)?;
        files.extend(python_package_files(
            key,
            version,
            &format!("{}.", capability.summary),
            &homepage(capability),
            &["pamoja", "iot", "robotics", key],
            &deps,
            python_capability_readme(root, catalog, capability, key)?,
        ));
    }

    for (chapter, members) in catalog.domains() {
        files.extend(python_domain_files(chapter, &members, version));
    }

    let all: BTreeSet<String> = keys
        .iter()
        .map(|key| (*key).to_owned())
        .chain(["core".to_owned(), NATIVE.to_owned()])
        .collect();
    files.push((
        "bindings/python/packages/pamoja/pyproject.toml".to_owned(),
        pyproject(
            "pamoja",
            version,
            "The whole pamoja framework in one package: every capability of one memory-safe Rust core, behind an idiomatic Python facade, for IoT, robotics, and drones.",
            &format!("{SITE}/"),
            &["pamoja", "iot", "robotics", "drones", "mqtt", "embedded"],
            &all,
            true,
        ),
    ));
    files.push((
        "bindings/python/packages/pamoja/README.md".to_owned(),
        python_bundle_readme(catalog),
    ));
    files.push((
        "bindings/python/packages/native/README.md".to_owned(),
        python_native_readme(),
    ));

    Ok(files)
}

/// The three generated files of one pure Python package.
fn python_package_files(
    key: &str,
    version: &str,
    description: &str,
    homepage: &str,
    keywords: &[&str],
    deps: &BTreeSet<String>,
    readme: String,
) -> Vec<(String, String)> {
    vec![
        (
            format!("bindings/python/packages/{key}/pyproject.toml"),
            pyproject(key, version, description, homepage, keywords, deps, false),
        ),
        (format!("bindings/python/packages/{key}/README.md"), readme),
        (
            format!("bindings/python/packages/{key}/pamoja/{key}/py.typed"),
            String::new(),
        ),
    ]
}

/// A pure Python project manifest built by hatchling. A capability package ships its
/// `pamoja/<key>` namespace portion; the metapackage ships nothing and only depends.
fn pyproject(
    key: &str,
    version: &str,
    description: &str,
    homepage: &str,
    keywords: &[&str],
    deps: &BTreeSet<String>,
    metapackage: bool,
) -> String {
    let name = if key == "pamoja" {
        "pamoja".to_owned()
    } else {
        format!("pamoja-{key}")
    };
    let keywords: Vec<String> = keywords.iter().map(|k| format!("\"{k}\"")).collect();
    let dependencies: Vec<String> = deps
        .iter()
        .map(|dep| format!("    \"pamoja-{dep}=={version}\","))
        .collect();
    let build = if metapackage {
        "[tool.hatch.build.targets.wheel]\nbypass-selection = true\n"
    } else {
        "[tool.hatch.build.targets.wheel]\npackages = [\"pamoja\"]\n"
    };
    format!(
        "[build-system]\n\
         requires = [\"hatchling>=1.27\"]\n\
         build-backend = \"hatchling.build\"\n\n\
         [project]\n\
         name = \"{name}\"\n\
         version = \"{version}\"\n\
         description = \"{description}\"\n\
         readme = \"README.md\"\n\
         license = {{ text = \"MIT\" }}\n\
         requires-python = \">=3.10\"\n\
         authors = [{{ name = \"molexxxx\" }}]\n\
         keywords = [{}]\n\
         classifiers = [\n\
         \x20   \"Programming Language :: Python :: 3\",\n\
         \x20   \"License :: OSI Approved :: MIT License\",\n\
         \x20   \"Operating System :: OS Independent\",\n\
         \x20   \"Typing :: Typed\",\n\
         ]\n\
         dependencies = [\n{}\n]\n\n\
         [project.urls]\n\
         Repository = \"https://github.com/molexxxx/pamoja\"\n\
         Documentation = \"{homepage}\"\n\n\
         {build}",
        keywords.join(", "),
        dependencies.join("\n"),
    )
}

/// The `pamoja` distributions a namespace portion imports from: `native` for the
/// generated contract and a capability key for each sibling module, never itself.
fn python_package_imports(portion: &Path, own: &str) -> Result<BTreeSet<String>, String> {
    let entries =
        fs::read_dir(portion).map_err(|err| format!("reading {}: {err}", portion.display()))?;
    let mut names = BTreeSet::new();
    for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
        if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        names.extend(python_imports(&text));
    }
    names.remove(own);
    Ok(names)
}

/// The `pamoja` distributions a Python source imports from.
fn python_imports(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in source.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("from pamoja.") {
            let module: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            names.insert(module);
        } else if let Some(rest) = line.strip_prefix("from pamoja import ") {
            for name in rest.split(',') {
                let name = name.trim().trim_matches(|c| c == '(' || c == ')').trim();
                if !name.is_empty() {
                    names.insert(name.to_owned());
                }
            }
        } else if let Some(rest) = line.strip_prefix("import pamoja.") {
            let module: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            names.insert(module);
        }
    }
    names
        .into_iter()
        .map(|name| {
            if name == "_native" || name == "raw" {
                NATIVE.to_owned()
            } else {
                name
            }
        })
        .collect()
}

/// The README of one Python capability package.
fn python_capability_readme(
    root: &Path,
    catalog: &Catalog,
    capability: &Capability,
    key: &str,
) -> Result<String, String> {
    let mut out = format!(
        "# pamoja-{key}\n\n{}. One capability of [pamoja](https://github.com/molexxxx/pamoja), \
         one memory-safe Rust core with bindings for TypeScript, Python, and C#.\n\n\
         ## Install\n\n```sh\npip install pamoja-{key}\n```\n\n```python\nfrom pamoja import {key}\n```\n\n\
         This pulls in `pamoja-native`, the compiled engine, and nothing else. \
         `pip install pamoja` is the whole framework in one package.\n",
        capability.summary
    );

    let snippet = format!("bindings/python/guides/{}.py", capability.key);
    if root.join(&snippet).is_file() {
        let example = regions::snippet(root, &format!("{snippet}#example"))?;
        out.push_str("\n## Example\n\nThe script the test suite runs, spliced here as it ran.\n\n");
        out.push_str(&example);
        out.push('\n');
    }

    out.push_str(&format!(
        "\n## The same capability in every language\n\n{}\n",
        catalog.cross_language(capability)
    ));

    out.push_str("\n## Documentation\n\n");
    if capability.guide.is_some() {
        out.push_str(&format!(
            "- [The {} guide]({}), with the same example in Rust, TypeScript, and C#.\n",
            capability.title,
            homepage(capability)
        ));
    }
    out.push_str(&format!(
        "- [Every capability]({SITE}/), and the [install page]({SITE}/install.html).\n\n\
         ## License\n\nMIT\n"
    ));
    Ok(out)
}

/// The README of the `pamoja` metapackage.
fn python_bundle_readme(catalog: &Catalog) -> String {
    let mut out = String::from(
        "# pamoja\n\n\
         The whole pamoja framework in one package: every capability of one memory-safe Rust \
         core, behind an idiomatic Python facade, for IoT, robotics, and drones. Each \
         capability is also its own distribution, so an application that needs one thing can \
         depend on `pamoja-mqtt` alone; this package depends on all of them.\n\n\
         ## Install\n\n```sh\npip install pamoja\n```\n\n```python\nfrom pamoja import mqtt, security\n```\n\n\
         ## What it installs\n\n| Distribution | Module | What it covers |\n| --- | --- | --- |\n",
    );
    for capability in &catalog.capabilities {
        out.push_str(&format!(
            "| `pamoja-{0}` | `pamoja.{0}` | {1} |\n",
            capability.python, capability.summary
        ));
    }
    out.push_str(&format!(
        "\nAll of them run on `pamoja-native`, the compiled engine, which is one extension \
         whichever distributions you install.\n\n\
         ## Documentation\n\n\
         - [The guides]({SITE}/), one page per capability with the same example in Rust, TypeScript, Python, and C#.\n\
         - [The Python reference]({SITE}/reference/python/pamoja.html), generated from every module.\n\n\
         ## License\n\nMIT\n"
    ));
    out
}

/// The README of `pamoja-core`.
fn python_core_readme() -> String {
    format!(
        "# pamoja-core\n\n\
         The pamoja engine's surface for Python: the runtime version, the error every native \
         call raises, and the transport every link shares. This is the counterpart of the \
         `pamoja-core` crate, and like it, it is small; the compiled engine is `pamoja-native`, \
         which this package depends on.\n\n\
         ## Install\n\n```sh\npip install pamoja-core\n```\n\n```python\nfrom pamoja.core import version, PamojaError, Transport\n```\n\n\
         Each capability is its own distribution (`pamoja-mqtt` gives `pamoja.mqtt`, and so on) \
         and `pip install pamoja` is the whole framework in one package.\n\n\
         ## Documentation\n\n\
         - [The reference for `pamoja.core`]({SITE}/reference/python/pamoja/core.html), generated from its source.\n\
         - [The guides]({SITE}/) and the [install page]({SITE}/install.html).\n\n\
         ## License\n\nMIT\n"
    )
}

/// The README of `pamoja-native`, the maturin project at the binding's root.
fn python_native_readme() -> String {
    format!(
        "# pamoja-native\n\n\
         The compiled pamoja engine for Python, built with PyO3 and maturin, with wheels for \
         Linux (x64, arm64), macOS (x64, arm64), and Windows (x64), and the generated contract \
         every `pamoja` package builds on. It is one extension module, `pamoja._native`, that \
         carries every capability; the capability distributions are facades over it, so \
         picking distributions narrows the API you depend on, not the size of the engine.\n\n\
         You do not install this distribution directly. Every `pamoja-<capability>` \
         distribution and the `pamoja` metapackage depend on it. `pamoja.raw` re-exports the \
         contract for anything a facade does not cover, and `pamoja/_native/__init__.pyi` types it.\n\n\
         ## Documentation\n\n\
         - [The guides]({SITE}/) and the [Python reference]({SITE}/reference/python/pamoja.html).\n\n\
         ## License\n\nMIT\n"
    )
}

/// Render every generated file of the .NET packages as (path, contents): a project
/// file and a README for `Pamoja.Core` and each `Pamoja.<Name>` capability package,
/// the `Pamoja` metapackage that depends on all of them, and the README of
/// `Pamoja.Native`, whose project file carries the native runtimes and is hand-written.
///
/// # Errors
///
/// Returns the reason when a package's sources cannot be read.
pub fn render_dotnet(root: &Path, catalog: &Catalog) -> Result<Vec<(String, String)>, String> {
    let src = root.join("bindings/dotnet/src");
    let mut files = Vec::new();

    let core_deps = dotnet_package_usings(&src.join("Pamoja.Core"), "Core")?;
    files.push((
        "bindings/dotnet/src/Pamoja.Core/Pamoja.Core.csproj".to_owned(),
        csproj(
            "Core",
            "The pamoja engine's surface for .NET: the runtime version and the transport every link implements, the counterpart of the pamoja-core crate.",
            &format!("{SITE}/"),
            &["pamoja", "iot", "robotics", "core"],
            &core_deps,
            false,
        ),
    ));
    files.push((
        "bindings/dotnet/src/Pamoja.Core/README.md".to_owned(),
        dotnet_core_readme(),
    ));

    let mut names: Vec<String> = Vec::new();
    for capability in &catalog.capabilities {
        if capability.dotnet_package() == "Pamoja.Core" {
            continue;
        }
        let name = dotnet_name(&capability.key);
        let deps = dotnet_package_usings(&src.join(format!("Pamoja.{name}")), &name)?;
        files.push((
            format!("bindings/dotnet/src/Pamoja.{name}/Pamoja.{name}.csproj"),
            csproj(
                &name,
                &format!("{}.", capability.summary),
                &homepage(capability),
                &["pamoja", "iot", "robotics", &capability.key],
                &deps,
                false,
            ),
        ));
        files.push((
            format!("bindings/dotnet/src/Pamoja.{name}/README.md"),
            dotnet_capability_readme(root, catalog, &deps, capability, &name)?,
        ));
        names.push(name);
    }

    for (chapter, members) in catalog.domains() {
        files.extend(dotnet_domain_files(chapter, &members));
    }

    let all: BTreeSet<String> = names
        .iter()
        .cloned()
        .chain(["Core".to_owned(), "Native".to_owned()])
        .collect();
    files.push((
        "bindings/dotnet/src/Pamoja/Pamoja.csproj".to_owned(),
        csproj(
            "",
            "The whole pamoja framework in one package: every capability of one memory-safe Rust core, behind an idiomatic C# facade, for IoT, robotics, and drones.",
            &format!("{SITE}/"),
            &["pamoja", "iot", "robotics", "drones", "mqtt", "embedded"],
            &all,
            true,
        ),
    ));
    files.push((
        "bindings/dotnet/src/Pamoja/README.md".to_owned(),
        dotnet_bundle_readme(catalog),
    ));
    files.push((
        "bindings/dotnet/src/Pamoja.Native/README.md".to_owned(),
        dotnet_native_readme(),
    ));

    Ok(files)
}

/// A project file. `name` is the part after `Pamoja.`, or empty for the metapackage,
/// which ships no assembly and only depends.
fn csproj(
    name: &str,
    description: &str,
    homepage: &str,
    tags: &[&str],
    deps: &BTreeSet<String>,
    metapackage: bool,
) -> String {
    let id = if name.is_empty() {
        "Pamoja".to_owned()
    } else {
        format!("Pamoja.{name}")
    };
    let mut properties = format!(
        "    <PackageId>{id}</PackageId>\n\
         \x20   <AssemblyName>{id}</AssemblyName>\n\
         \x20   <RootNamespace>{id}</RootNamespace>\n\
         \x20   <Description>{description}</Description>\n\
         \x20   <PackageTags>{}</PackageTags>\n\
         \x20   <PackageProjectUrl>{homepage}</PackageProjectUrl>\n\
         \x20   <PackageReadmeFile>README.md</PackageReadmeFile>\n",
        tags.join(";")
    );
    if metapackage {
        properties.push_str(
            "    <IncludeBuildOutput>false</IncludeBuildOutput>\n\
             \x20   <GenerateDocumentationFile>false</GenerateDocumentationFile>\n\
             \x20   <NoWarn>$(NoWarn);NU5128</NoWarn>\n",
        );
    } else {
        properties.push_str(
            "    <GenerateDocumentationFile>true</GenerateDocumentationFile>\n\
             \x20   <IncludeSymbols>true</IncludeSymbols>\n\
             \x20   <SymbolPackageFormat>snupkg</SymbolPackageFormat>\n",
        );
    }
    let references: Vec<String> = deps
        .iter()
        .map(|dep| {
            format!("    <ProjectReference Include=\"../Pamoja.{dep}/Pamoja.{dep}.csproj\" />")
        })
        .collect();
    format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n\n\
         \x20 <PropertyGroup>\n{properties}  </PropertyGroup>\n\n\
         \x20 <ItemGroup>\n\
         \x20   <None Include=\"README.md\" Pack=\"true\" PackagePath=\"\\\" />\n\
         \x20 </ItemGroup>\n\n\
         \x20 <ItemGroup>\n{}\n  </ItemGroup>\n\n\
         </Project>\n",
        references.join("\n")
    )
}

/// The `Pamoja.<X>` packages the C# sources of a project use, never itself.
fn dotnet_package_usings(project: &Path, own: &str) -> Result<BTreeSet<String>, String> {
    let entries =
        fs::read_dir(project).map_err(|err| format!("reading {}: {err}", project.display()))?;
    let mut names = BTreeSet::new();
    for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
        if path.extension().and_then(|ext| ext.to_str()) != Some("cs") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        names.extend(dotnet_usings(&text));
    }
    names.remove(own);
    Ok(names)
}

/// The `Pamoja.<X>` packages a C# source names in its `using` directives.
fn dotnet_usings(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("using Pamoja."))
        .filter_map(|rest| rest.strip_suffix(';'))
        .map(|name| name.split('.').next().unwrap_or(name).to_owned())
        .collect()
}

/// The README of one .NET capability package.
fn dotnet_capability_readme(
    root: &Path,
    catalog: &Catalog,
    deps: &BTreeSet<String>,
    capability: &Capability,
    name: &str,
) -> Result<String, String> {
    let mut out = format!(
        "# Pamoja.{name}\n\n{}. One capability of [pamoja](https://github.com/molexxxx/pamoja), \
         one memory-safe Rust core with bindings for TypeScript, Python, and C#.\n\n\
         ## Install\n\n```sh\ndotnet add package Pamoja.{name}\n```\n\n```csharp\nusing Pamoja.{name};\n```\n\n\
         This pulls in `Pamoja.Native`, the compiled engine{}. \
         `dotnet add package Pamoja` is the whole framework in one package.\n",
        capability.summary,
        extras(deps)
    );

    // The class is suffixed so it cannot shadow the type it demonstrates: a Guides.Modbus
    // would hide the Pamoja.Modbus.Modbus the example calls.
    let snippet = format!("bindings/dotnet/samples/Pamoja.Guides/{name}Guide.cs");
    if root.join(&snippet).is_file() {
        let example = regions::snippet(root, &format!("{snippet}#example"))?;
        out.push_str(
            "\n## Example\n\nThe guide project's example, spliced here as it ran in CI.\n\n",
        );
        out.push_str(&example);
        out.push('\n');
    }

    out.push_str(&format!(
        "\n## The same capability in every language\n\n{}\n",
        catalog.cross_language(capability)
    ));

    out.push_str("\n## Documentation\n\n");
    if capability.guide.is_some() {
        out.push_str(&format!(
            "- [The {} guide]({}), with the same example in Rust, TypeScript, and Python.\n",
            capability.title,
            homepage(capability)
        ));
    }
    out.push_str(&format!(
        "- [Every capability]({SITE}/), and the [install page]({SITE}/install.html).\n\n\
         ## License\n\nMIT\n"
    ));
    Ok(out)
}

// The sibling packages a facade needs besides the engine, named in its README so the
// install line is not a surprise. Most capabilities need none.
fn extras(deps: &BTreeSet<String>) -> String {
    let mut names: Vec<String> = deps
        .iter()
        .filter(|dep| dep.as_str() != "Native")
        .map(|dep| format!("`Pamoja.{dep}`"))
        .collect();
    names.sort();
    match names.len() {
        0 => String::new(),
        1 => format!(", and {}", names[0]),
        _ => format!(
            ", and {} and {}",
            names[..names.len() - 1].join(", "),
            names[names.len() - 1]
        ),
    }
}

/// The README of the `Pamoja` metapackage.
fn dotnet_bundle_readme(catalog: &Catalog) -> String {
    let mut out = String::from(
        "# Pamoja\n\n\
         The whole pamoja framework in one package: every capability of one memory-safe Rust \
         core, behind an idiomatic C# facade, for IoT, robotics, and drones. Each capability \
         is also its own package, so an application that needs one thing can depend on \
         `Pamoja.Mqtt` alone; this package depends on all of them.\n\n\
         ## Install\n\n```sh\ndotnet add package Pamoja\n```\n\n\
         ## What it installs\n\n| Package | What it covers |\n| --- | --- |\n",
    );
    for capability in catalog.ordered() {
        out.push_str(&format!(
            "| `{}` | {} |\n",
            capability.dotnet_package(),
            capability.summary
        ));
    }
    out.push_str(&format!(
        "\nAll of them run on `Pamoja.Native`, the compiled engine, which is one library \
         whichever packages you install.\n\n\
         ## Documentation\n\n\
         - [The guides]({SITE}/), one page per capability with the same example in Rust, TypeScript, Python, and C#.\n\
         - [The C# reference]({SITE}/reference/dotnet/index.html), generated from every package.\n\n\
         ## License\n\nMIT\n"
    ));
    out
}

/// The README of `Pamoja.Core`.
fn dotnet_core_readme() -> String {
    format!(
        "# Pamoja.Core\n\n\
         The pamoja engine's surface for .NET: the runtime version and the transport every \
         link implements. This is the counterpart of the `pamoja-core` crate, and like it, it \
         is small. It is a capability like the others rather than a foundation: only the \
         transport packages depend on it, because they are the ones that return a transport. \
         The compiled engine, which every package depends on, is `Pamoja.Native`.\n\n\
         ## Install\n\n```sh\ndotnet add package Pamoja.Core\n```\n\n```csharp\nusing Pamoja.Core;\n```\n\n\
         Each capability is its own package (`Pamoja.Mqtt`, `Pamoja.Security`, and so on) and \
         `dotnet add package Pamoja` is the whole framework in one package.\n\n\
         ## Documentation\n\n\
         - [The reference for `Pamoja.Core`]({SITE}/reference/dotnet/api/Pamoja.Core.html), generated from its source.\n\
         - [The guides]({SITE}/) and the [install page]({SITE}/install.html).\n\n\
         ## License\n\nMIT\n"
    )
}

/// The README of `Pamoja.Native`.
fn dotnet_native_readme() -> String {
    format!(
        "# Pamoja.Native\n\n\
         The compiled pamoja engine for .NET, bundled for `win-x64`, `linux-x64`, `linux-arm64`, \
         `osx-x64`, and `osx-arm64`, and the P/Invoke contract every `Pamoja` package builds on: \
         `Pamoja.Native.Interop.NativeMethods` mirrors the generated C header one-to-one. It also \
         carries the marshalling a facade needs to use that contract: the safe handle type, the \
         status helpers, owned strings, and `PamojaException`, which every failed native call \
         raises and which sits in the root `Pamoja` namespace so a facade sees it without a \
         using. It is one library that carries every capability; the capability packages are \
         facades over it, so picking packages narrows the API you depend on, not the size of \
         the engine.\n\n\
         You do not install this package directly. Every `Pamoja.<Capability>` package and the \
         `Pamoja` metapackage depend on it. The interop layer stays available for anything a \
         facade does not cover.\n\n\
         ## Documentation\n\n\
         - [The guides]({SITE}/) and the [C# reference]({SITE}/reference/dotnet/index.html).\n\n\
         ## License\n\nMIT\n"
    )
}

/// Two-space JSON with a trailing newline, the way npm writes a manifest.
/// A domain's Python distribution: `pamoja-<key>`, shipping the `pamoja.<key>` module that
/// re-exports the domain's capability modules under their own names, for the same reason the
/// Node package does.
fn python_domain_files(
    chapter: &Chapter,
    members: &[&Capability],
    version: &str,
) -> Vec<(String, String)> {
    let module = chapter.key.replace('-', "_");
    let names: Vec<&str> = members.iter().map(|c| c.python.as_str()).collect();
    let deps: BTreeSet<String> = names.iter().map(|name| (*name).to_owned()).collect();
    let installs = names
        .iter()
        .map(|name| format!("``pamoja.{name}``"))
        .collect::<Vec<_>>()
        .join(", ");

    let mut init = format!(
        "\"\"\"{}: {}\n\nInstalling this distribution installs {installs}, and re-exports each under its\nown name, so a name two of them share stays unambiguous.\n\"\"\"\n\nfrom pamoja import {}\n\n__all__ = [{}]\n",
        chapter.title,
        chapter.intent,
        names.join(", "),
        names
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    init.push('\n');

    let mut readme = format!(
        "# pamoja-{}\n\n{}\n\nOne install for the {} capabilities of this domain. Each is also its own\ndistribution, and `pamoja` is the whole framework in one.\n\n```sh\npip install pamoja-{}\n```\n\n```python\nfrom pamoja.{module} import {}\n```\n\n| Capability | Module | What it covers |\n| --- | --- | --- |\n",
        chapter.key,
        chapter.intent,
        members.len(),
        chapter.key,
        names[0]
    );
    for capability in members {
        readme.push_str(&format!(
            "| [{}]({SITE}/guides/{}.html) | `pamoja.{}` | {} |\n",
            capability.title, capability.key, capability.python, capability.summary
        ));
    }
    readme.push_str(&format!(
        "\nThe guides, with a worked Python example for each, are at [{SITE}]({SITE}/).\n\n## License\n\nMIT\n"
    ));

    vec![
        (
            format!("bindings/python/packages/{}/pyproject.toml", chapter.key),
            pyproject(
                &chapter.key,
                version,
                &format!("{}: {}", chapter.title, chapter.intent),
                &format!("{SITE}/install.html"),
                &["pamoja", "iot", "robotics", &chapter.key],
                &deps,
                false,
            ),
        ),
        (
            format!("bindings/python/packages/{}/README.md", chapter.key),
            readme,
        ),
        (
            format!(
                "bindings/python/packages/{}/pamoja/{module}/__init__.py",
                chapter.key
            ),
            init,
        ),
        (
            format!(
                "bindings/python/packages/{}/pamoja/{module}/py.typed",
                chapter.key
            ),
            String::new(),
        ),
    ]
}

/// A domain's NuGet package: `Pamoja.<Name>`, which references the domain's capability
/// packages and ships no assembly of its own. C# cannot re-export a namespace, so a
/// consumer writes the capability's `using` as they would anyway.
fn dotnet_domain_files(chapter: &Chapter, members: &[&Capability]) -> Vec<(String, String)> {
    let name = pascal(&chapter.key);
    let mut references = String::new();
    for capability in members {
        let package = capability.dotnet_package();
        references.push_str(&format!(
            "    <ProjectReference Include=\"../{package}/{package}.csproj\" />\n"
        ));
    }
    let csproj = format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n\n  <PropertyGroup>\n    <PackageId>Pamoja.{name}</PackageId>\n    <Description>{}: {}</Description>\n    <PackageTags>pamoja;iot;robotics;{}</PackageTags>\n    <PackageProjectUrl>{SITE}/install.html</PackageProjectUrl>\n    <PackageReadmeFile>README.md</PackageReadmeFile>\n    <IncludeBuildOutput>false</IncludeBuildOutput>\n    <GenerateDocumentationFile>false</GenerateDocumentationFile>\n    <NoWarn>$(NoWarn);NU5128</NoWarn>\n  </PropertyGroup>\n\n  <ItemGroup>\n    <None Include=\"README.md\" Pack=\"true\" PackagePath=\"\\\" />\n  </ItemGroup>\n\n  <ItemGroup>\n{references}  </ItemGroup>\n\n</Project>\n",
        chapter.title, chapter.intent, chapter.key
    );

    let mut readme = format!(
        "# Pamoja.{name}\n\n{}\n\nOne reference for the {} capabilities of this domain. Each is also its own package,\nand `Pamoja` is the whole framework in one.\n\n```sh\ndotnet add package Pamoja.{name}\n```\n\nThis package ships no assembly: it brings in the packages below, and each keeps its own\nnamespace, so a type is named the way it is when the package is referenced directly.\n\n| Capability | Package | What it covers |\n| --- | --- | --- |\n",
        chapter.intent,
        members.len()
    );
    for capability in members {
        readme.push_str(&format!(
            "| [{}]({SITE}/guides/{}.html) | `{}` | {} |\n",
            capability.title,
            capability.key,
            capability.dotnet_package(),
            capability.summary
        ));
    }
    readme.push_str(&format!(
        "\nThe guides, with a worked C# example for each, are at [{SITE}]({SITE}/).\n\n## License\n\nMIT\n"
    ));

    vec![
        (
            format!("bindings/dotnet/src/Pamoja.{name}/Pamoja.{name}.csproj"),
            csproj,
        ),
        (
            format!("bindings/dotnet/src/Pamoja.{name}/README.md"),
            readme,
        ),
    ]
}

// A kebab-case key as a .NET package-name segment: `field-io` becomes `FieldIo`.
fn pascal(key: &str) -> String {
    key.split('-').map(dotnet_name).collect::<Vec<_>>().concat()
}

fn pretty(value: &Json) -> String {
    let mut text = String::new();
    value.render(&mut text, 0);
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_packages_a_source_imports() {
        let source = concat!(
            "import { DeviceIdentity } from '@pamoja/security'\n",
            "import type { AuditEntry } from '@pamoja/native'\n",
            "export { Transport } from '@pamoja/core/transport'\n",
        );
        let names: Vec<String> = node_imports(source).into_iter().collect();
        assert_eq!(names, ["core", "native", "security"]);
    }

    #[test]
    fn pins_every_dependency_and_references_only_typescript_packages() {
        let deps: BTreeSet<String> = ["native", "security"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let pinned = pretty(&pins(&deps, "0.2.0"));
        assert!(pinned.contains("\"@pamoja/native\": \"0.2.0\""));
        assert!(pinned.contains("\"@pamoja/security\": \"0.2.0\""));
        let project = pretty(&tsconfig(&deps));
        assert!(project.contains("\"path\": \"../security\""));
        assert!(!project.contains("../native"));
    }

    #[test]
    fn the_bundle_is_the_bare_name_and_the_rest_are_scoped() {
        let deps = BTreeSet::new();
        let bundle = pretty(&manifest("pamoja", "0.2.0", "d", "h", &[], &deps, &[]));
        assert!(bundle.starts_with("{\n  \"name\": \"pamoja\",\n  \"version\": \"0.2.0\","));
        assert!(!bundle.contains("./transport"));
        let core = pretty(&manifest(
            "core",
            "0.2.0",
            "d",
            "h",
            &[],
            &deps,
            &["transport"],
        ));
        assert!(core.contains("\"name\": \"@pamoja/core\""));
        assert!(core.contains(
            "\"./transport\": {\n      \"types\": \"./dist/transport.d.ts\",\n      \"default\": \"./dist/transport.js\"\n    }"
        ));
    }

    #[test]
    fn json_renders_the_way_npm_writes_it() {
        let value = Json::object(vec![
            ("a", Json::strings(["x"])),
            ("b", Json::Array(Vec::new())),
            ("c", Json::object(Vec::new())),
            ("d", Json::Bool(true)),
        ]);
        assert_eq!(
            pretty(&value),
            "{\n  \"a\": [\n    \"x\"\n  ],\n  \"b\": [],\n  \"c\": {},\n  \"d\": true\n}\n"
        );
    }
}
