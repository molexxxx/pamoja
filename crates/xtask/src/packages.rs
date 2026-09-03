//! The packages of the Node binding, rendered from the capability map: a manifest, a
//! TypeScript project, and a README for `@pamoja/core` and for each `@pamoja/<key>`
//! capability package, the `pamoja` bundle that depends on all of them, the README of
//! `@pamoja/native`, and the workspace's project references. A package's dependencies
//! are derived from its own imports, so a facade that starts using another package
//! declares it on the next `cargo xtask docs`.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::catalog::{node_reference_url, Capability, Catalog, SITE};
use crate::regions;

/// The repository URL every manifest points at.
const REPOSITORY: &str = "git+https://github.com/molexxxx/pamoja.git";

/// The package that carries the compiled engine and the generated contract. It is
/// not a TypeScript project, so it is a dependency but never a project reference.
const NATIVE: &str = "native";

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
            capability_readme(root, capability, key)?,
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

    let mut references: Vec<Value> = vec![json!({ "path": "packages/core" })];
    references.extend(
        keys.iter()
            .map(|key| json!({ "path": format!("packages/{key}") })),
    );
    references.push(json!({ "path": "packages/pamoja" }));
    files.push((
        "bindings/node/tsconfig.json".to_owned(),
        pretty(&json!({ "files": [], "references": references })),
    ));

    Ok(files)
}

/// The three generated files of one TypeScript package.
fn package_files(
    key: &str,
    deps: &BTreeSet<String>,
    manifest: &Value,
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
) -> Value {
    let name = if key == "pamoja" {
        "pamoja".to_owned()
    } else {
        format!("@pamoja/{key}")
    };
    let mut exports = Map::new();
    exports.insert(
        ".".to_owned(),
        json!({ "types": "./dist/index.d.ts", "default": "./dist/index.js" }),
    );
    for subpath in subpaths {
        exports.insert(
            format!("./{subpath}"),
            json!({
                "types": format!("./dist/{subpath}.d.ts"),
                "default": format!("./dist/{subpath}.js")
            }),
        );
    }
    json!({
        "name": name,
        "version": version,
        "description": description,
        "license": "MIT",
        "publishConfig": { "access": "public" },
        "repository": {
            "type": "git",
            "url": REPOSITORY,
            "directory": format!("bindings/node/packages/{key}")
        },
        "homepage": homepage,
        "keywords": keywords,
        "main": "dist/index.js",
        "types": "dist/index.d.ts",
        "exports": Value::Object(exports),
        "files": ["dist/"],
        "engines": { "node": ">= 16" },
        "dependencies": pins(deps, version)
    })
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
fn pins(names: &BTreeSet<String>, version: &str) -> Value {
    let mut map = Map::new();
    for name in names {
        map.insert(format!("@pamoja/{name}"), json!(version));
    }
    Value::Object(map)
}

/// A package's TypeScript project: the shared options, and a reference to each
/// TypeScript package it depends on so `tsc -b` builds them first.
fn tsconfig(deps: &BTreeSet<String>) -> Value {
    let references: Vec<Value> = deps
        .iter()
        .filter(|dep| dep.as_str() != NATIVE)
        .map(|dep| json!({ "path": format!("../{dep}") }))
        .collect();
    json!({
        "extends": "../../tsconfig.base.json",
        "compilerOptions": { "rootDir": "src", "outDir": "dist", "composite": true },
        "include": ["src/**/*.ts"],
        "references": references
    })
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
fn capability_readme(root: &Path, capability: &Capability, key: &str) -> Result<String, String> {
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

    out.push_str("\n## Documentation\n\n");
    if capability.guide.is_some() {
        out.push_str(&format!(
            "- [The {} guide]({}), with the same example in Rust, Python, and C#.\n",
            capability.title,
            homepage(capability)
        ));
    }
    out.push_str(&format!(
        "- [The reference for this package]({}), generated from its source.\n\
         - [Every capability]({SITE}/), and the [install page]({SITE}/install.html).\n\n\
         ## License\n\nMIT\n",
        node_reference_url(key)
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
    for capability in &catalog.capabilities {
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

/// Two-space JSON with a trailing newline, the way npm writes a manifest.
fn pretty(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("a JSON value serializes");
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
        let pinned = pins(&deps, "0.2.0");
        assert_eq!(pinned["@pamoja/native"], "0.2.0");
        assert_eq!(pinned["@pamoja/security"], "0.2.0");
        let project = tsconfig(&deps);
        assert_eq!(project["references"].as_array().unwrap().len(), 1);
        assert_eq!(project["references"][0]["path"], "../security");
    }

    #[test]
    fn the_bundle_is_the_bare_name_and_the_rest_are_scoped() {
        let deps = BTreeSet::new();
        let bundle = manifest("pamoja", "0.2.0", "d", "h", &[], &deps, &[]);
        assert_eq!(bundle["name"], "pamoja");
        let core = manifest("core", "0.2.0", "d", "h", &[], &deps, &["transport"]);
        assert_eq!(core["name"], "@pamoja/core");
        assert_eq!(
            core["exports"]["./transport"]["default"],
            "./dist/transport.js"
        );
        assert!(bundle["exports"]["./transport"].is_null());
    }
}
