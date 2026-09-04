//! The workspace version: `cargo xtask version <x.y.z>` rewrites every manifest
//! that carries it and refreshes the files derived from them; `cargo xtask
//! version --check [expected]` reads them all back and fails on any drift.
//!
//! Every file is found by pattern rather than listed, so a new crate, binding,
//! or platform package is covered the moment it exists.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use toml_edit::{DocumentMut, Item, TableLike, Value};

/// The prefix shared by every crate in the workspace; a dependency with this
/// prefix is pinned to the workspace version.
const CRATE_PREFIX: &str = "pamoja-";

/// The loader napi-rs generates carries the version it was built for in two
/// places per platform package: the comparison and the mismatch message. The
/// text on either side of the version at each.
const LOADER_SITES: [(&str, &str); 2] = [
    ("bindingPackageVersion !== '", "'"),
    ("expected ", " but got"),
];

/// Run `cargo xtask version [<x.y.z> | --check [expected]]`.
///
/// With no argument the workspace version is printed. With a version, every
/// manifest is rewritten to it, the cargo and npm lockfiles are refreshed, the
/// generated Node loader is updated, and the check runs. With `--check`, every
/// version-bearing file is read and compared with the workspace version, and
/// with `expected` also with that.
pub fn run(args: &[String]) -> ExitCode {
    let result = match args.first().map(String::as_str) {
        None => current().map(|version| println!("{version}")),
        Some("--check") => check(args.get(1).map(String::as_str)),
        Some(flag) if flag.starts_with('-') => Err(format!(
            "unknown flag {flag}; usage: version [<x.y.z> | --check [expected]]"
        )),
        Some(version) => bump(version),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask version: {message}");
            ExitCode::FAILURE
        }
    }
}

/// One version read from a file, named well enough to act on a mismatch.
struct Reading {
    file: PathBuf,
    what: String,
    version: String,
}

/// The workspace version, from `[workspace.package]` in the root manifest.
pub(crate) fn current() -> Result<String, String> {
    let path = repo_root().join("Cargo.toml");
    let doc = parse_toml(&path)?;
    doc.get("workspace")
        .and_then(Item::as_table_like)
        .and_then(|workspace| workspace.get("package"))
        .and_then(Item::as_table_like)
        .and_then(|package| package.get("version"))
        .and_then(Item::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "Cargo.toml has no workspace.package.version".to_owned())
}

/// Compare every version-bearing file with the workspace version, listing each
/// disagreement.
fn check(expected: Option<&str>) -> Result<(), String> {
    let version = current()?;
    let mut problems = Vec::new();

    if let Some(expected) = expected {
        if expected != version {
            problems.push(format!(
                "Cargo.toml: workspace.package.version is {version}, expected {expected}"
            ));
        }
    }

    for reading in readings()? {
        if reading.version != version {
            problems.push(format!(
                "{}: {} is {}",
                display(&reading.file),
                reading.what,
                reading.version
            ));
        }
    }

    let changelog = repo_root().join("CHANGELOG.md");
    if !read(&changelog)?.contains(&format!("## [{version}]")) {
        problems.push(format!("CHANGELOG.md: no `## [{version}]` entry"));
    }

    if problems.is_empty() {
        println!("xtask version: every manifest, lockfile, and generated file is at {version}");
        Ok(())
    } else {
        Err(format!(
            "{} place(s) disagree with the workspace version {version}:\n  {}",
            problems.len(),
            problems.join("\n  ")
        ))
    }
}

/// Rewrite every manifest to `new`, refresh the derived files, and check.
fn bump(new: &str) -> Result<(), String> {
    validate(new)?;
    let root = repo_root();
    println!("xtask version: {} -> {new}", current()?);

    edit_toml(&root.join("Cargo.toml"), |doc| {
        let workspace = doc
            .get_mut("workspace")
            .and_then(Item::as_table_like_mut)
            .ok_or("no [workspace] table")?;
        let package = workspace
            .get_mut("package")
            .and_then(Item::as_table_like_mut)
            .ok_or("no [workspace.package] table")?;
        set_version(package, new)?;
        if let Some(deps) = workspace
            .get_mut("dependencies")
            .and_then(Item::as_table_like_mut)
        {
            set_dependency_versions(deps, new)?;
        }
        Ok(())
    })?;

    for manifest in crate_manifests(&root)? {
        edit_toml(&manifest, |doc| {
            if let Some(package) = doc.get_mut("package").and_then(Item::as_table_like_mut) {
                if package.get("version").and_then(Item::as_str).is_some() {
                    set_version(package, new)?;
                }
            }
            for_each_dependency_table(doc, &mut |_, deps| set_dependency_versions(deps, new))
        })?;
    }

    for pyproject in pyprojects(&root) {
        edit_toml(&pyproject, |doc| {
            let project = doc
                .get_mut("project")
                .and_then(Item::as_table_like_mut)
                .ok_or("no [project] table")?;
            set_version(project, new)?;
            if let Some(deps) = project.get_mut("dependencies").and_then(Item::as_array_mut) {
                for entry in deps.iter_mut() {
                    let Some(spec) = entry.as_str() else {
                        continue;
                    };
                    if let Some((name, _)) = python_pin(spec) {
                        replace_value(entry, &format!("{name}=={new}"));
                    }
                }
            }
            Ok(())
        })?;
    }

    for package_json in package_manifests(&root) {
        rewrite(&package_json, "\"version\": \"", "\"", new, 1)?;
        let text = read(&package_json)?;
        write(&package_json, &with_package_pins(&text, new))?;
    }

    for props in binding_files(&root, "Directory.Build.props") {
        rewrite(&props, "<Version>", "</Version>", new, usize::MAX)?;
    }

    for loader in loader_files(&root) {
        for (before, after) in LOADER_SITES {
            rewrite(&loader, before, after, new, usize::MAX)?;
        }
    }

    for lockfile in cargo_lockfiles(&root) {
        let manifest = lockfile.with_file_name("Cargo.toml");
        println!("xtask version: refreshing {}", display(&lockfile));
        let mut cmd = Command::new("cargo");
        cmd.args(["update", "--workspace", "--manifest-path"])
            .arg(&manifest);
        if !super::run(&mut cmd) {
            return Err(format!("cargo update failed for {}", display(&lockfile)));
        }
    }

    for lockfile in binding_files(&root, "package-lock.json") {
        let dir = lockfile.parent().ok_or("package-lock.json has no parent")?;
        println!("xtask version: refreshing {}", display(&lockfile));
        let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
        let mut cmd = Command::new(npm);
        cmd.args([
            "install",
            "--package-lock-only",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
        ])
        .current_dir(dir);
        if !super::run(&mut cmd) {
            return Err(format!("npm install failed for {}", display(&lockfile)));
        }
    }

    check(Some(new))
}

/// Read every version-bearing file in the repository.
fn readings() -> Result<Vec<Reading>, String> {
    let root = repo_root();
    let mut readings = Vec::new();

    let root_manifest = root.join("Cargo.toml");
    let doc = parse_toml(&root_manifest)?;
    if let Some(deps) = doc
        .get("workspace")
        .and_then(Item::as_table_like)
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(Item::as_table_like)
    {
        read_dependency_versions(
            &root_manifest,
            "workspace.dependencies",
            deps,
            false,
            &mut readings,
        )?;
    }

    for manifest in crate_manifests(&root)? {
        let doc = parse_toml(&manifest)?;
        let package = doc.get("package").and_then(Item::as_table_like);
        if let Some(version) = package
            .and_then(|package| package.get("version"))
            .and_then(Item::as_str)
        {
            readings.push(reading(&manifest, "package.version", version));
        }
        let publishable = package
            .and_then(|package| package.get("publish"))
            .map(|publish| match publish {
                Item::Value(Value::Boolean(flag)) => *flag.value(),
                Item::Value(Value::Array(registries)) => !registries.is_empty(),
                _ => true,
            })
            .unwrap_or(true);
        for (section, deps) in dependency_tables(&doc) {
            let must_pin = publishable && !section.contains("dev-dependencies");
            read_dependency_versions(&manifest, &section, deps, must_pin, &mut readings)?;
        }
    }

    for lockfile in cargo_lockfiles(&root) {
        let doc = parse_toml(&lockfile)?;
        let packages = doc
            .get("package")
            .and_then(Item::as_array_of_tables)
            .ok_or_else(|| format!("{} has no [[package]] entries", display(&lockfile)))?;
        for package in packages {
            if package.get("source").is_some() {
                continue;
            }
            let name = package.get("name").and_then(Item::as_str).unwrap_or("?");
            let version = package.get("version").and_then(Item::as_str).unwrap_or("?");
            readings.push(reading(&lockfile, &format!("package {name}"), version));
        }
    }

    for pyproject in pyprojects(&root) {
        let doc = parse_toml(&pyproject)?;
        let project = doc.get("project").and_then(Item::as_table_like);
        let version = project
            .and_then(|project| project.get("version"))
            .and_then(Item::as_str)
            .ok_or_else(|| format!("{} has no project.version", display(&pyproject)))?;
        readings.push(reading(&pyproject, "project.version", version));
        let deps = project
            .and_then(|project| project.get("dependencies"))
            .and_then(Item::as_array);
        for spec in deps.into_iter().flat_map(|deps| deps.iter()) {
            if let Some((name, pinned)) = spec.as_str().and_then(python_pin) {
                readings.push(reading(&pyproject, &format!("dependency {name}"), pinned));
            }
        }
    }

    for package_json in package_manifests(&root) {
        let version = json_version(&package_json, &[])?;
        readings.push(reading(&package_json, "version", &version));
        for (name, pinned) in package_pins(&package_json)? {
            readings.push(reading(
                &package_json,
                &format!("dependency {name}"),
                &pinned,
            ));
        }
    }

    for lockfile in binding_files(&root, "package-lock.json") {
        let version = json_version(&lockfile, &[])?;
        readings.push(reading(&lockfile, "version", &version));
        let version = json_version(&lockfile, &["packages", ""])?;
        readings.push(reading(&lockfile, "packages[\"\"].version", &version));
    }

    for props in binding_files(&root, "Directory.Build.props") {
        let versions = find_between(&read(&props)?, "<Version>", "</Version>");
        if versions.is_empty() {
            return Err(format!("{} has no <Version> element", display(&props)));
        }
        for version in versions {
            readings.push(reading(&props, "<Version>", &version));
        }
    }

    for loader in loader_files(&root) {
        let text = read(&loader)?;
        let mut versions: Vec<String> = LOADER_SITES
            .iter()
            .flat_map(|(before, after)| find_between(&text, before, after))
            .collect();
        versions.sort();
        versions.dedup();
        if versions.is_empty() {
            return Err(format!(
                "{} names no version in its platform checks",
                display(&loader)
            ));
        }
        for version in versions {
            readings.push(reading(
                &loader,
                "the version the loader was generated for",
                &version,
            ));
        }
    }

    Ok(readings)
}

/// Record the pinned version of every workspace dependency in one table.
///
/// With `must_pin`, an unpinned workspace dependency is an error: crates.io
/// rejects a publish whose path dependency carries no version.
fn read_dependency_versions(
    file: &Path,
    section: &str,
    deps: &dyn TableLike,
    must_pin: bool,
    readings: &mut Vec<Reading>,
) -> Result<(), String> {
    for (key, item) in deps.iter() {
        let name = item
            .as_table_like()
            .and_then(|dep| dep.get("package"))
            .and_then(Item::as_str)
            .unwrap_or(key);
        if !name.starts_with(CRATE_PREFIX) {
            continue;
        }
        let what = format!("{section}.{key}.version");
        if let Some(version) = item.as_str() {
            readings.push(reading(file, &what, version));
            continue;
        }
        let Some(dep) = item.as_table_like() else {
            continue;
        };
        if dep.get("workspace").and_then(Item::as_bool) == Some(true) {
            continue;
        }
        match dep.get("version").and_then(Item::as_str) {
            Some(version) => readings.push(reading(file, &what, version)),
            None if must_pin => {
                return Err(format!(
                    "{}: {section}.{key} has no version, so the crate cannot be published",
                    display(file)
                ))
            }
            None => {}
        }
    }
    Ok(())
}

/// Set `version` in a package or project table, keeping its formatting.
fn set_version(table: &mut dyn TableLike, new: &str) -> Result<(), String> {
    let slot = table
        .get_mut("version")
        .and_then(Item::as_value_mut)
        .ok_or("no version key")?;
    replace_value(slot, new);
    Ok(())
}

/// Set the pinned version of every workspace dependency in one table.
fn set_dependency_versions(deps: &mut dyn TableLike, new: &str) -> Result<(), String> {
    for (key, item) in deps.iter_mut() {
        let name = item
            .as_table_like()
            .and_then(|dep| dep.get("package"))
            .and_then(Item::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| key.get().to_owned());
        if !name.starts_with(CRATE_PREFIX) {
            continue;
        }
        if item.as_str().is_some() {
            if let Some(slot) = item.as_value_mut() {
                replace_value(slot, new);
            }
            continue;
        }
        if let Some(slot) = item
            .as_table_like_mut()
            .and_then(|dep| dep.get_mut("version"))
            .and_then(Item::as_value_mut)
        {
            replace_value(slot, new);
        }
    }
    Ok(())
}

/// Replace a string value in place, keeping the whitespace and comments around it.
fn replace_value(slot: &mut Value, new: &str) {
    let decor = slot.decor().clone();
    *slot = Value::from(new);
    *slot.decor_mut() = decor;
}

/// The dependency tables of a manifest: the three plain sections and the same
/// three under each `[target.<cfg>]`, named as they appear in the file.
fn dependency_tables(doc: &DocumentMut) -> Vec<(String, &dyn TableLike)> {
    const SECTIONS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut tables = Vec::new();
    for section in SECTIONS {
        if let Some(table) = doc.get(section).and_then(Item::as_table_like) {
            tables.push((section.to_owned(), table));
        }
    }
    if let Some(targets) = doc.get("target").and_then(Item::as_table_like) {
        for (target, item) in targets.iter() {
            let Some(target_table) = item.as_table_like() else {
                continue;
            };
            for section in SECTIONS {
                if let Some(table) = target_table.get(section).and_then(Item::as_table_like) {
                    tables.push((format!("target.{target}.{section}"), table));
                }
            }
        }
    }
    tables
}

/// Visit every dependency table of a manifest mutably.
fn for_each_dependency_table(
    doc: &mut DocumentMut,
    visit: &mut dyn FnMut(&str, &mut dyn TableLike) -> Result<(), String>,
) -> Result<(), String> {
    const SECTIONS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    for section in SECTIONS {
        if let Some(table) = doc.get_mut(section).and_then(Item::as_table_like_mut) {
            visit(section, table)?;
        }
    }
    if let Some(targets) = doc.get_mut("target").and_then(Item::as_table_like_mut) {
        for (target, item) in targets.iter_mut() {
            let Some(target_table) = item.as_table_like_mut() else {
                continue;
            };
            for section in SECTIONS {
                if let Some(table) = target_table
                    .get_mut(section)
                    .and_then(Item::as_table_like_mut)
                {
                    visit(&format!("target.{}.{section}", target.get()), table)?;
                }
            }
        }
    }
    Ok(())
}

/// Every crate manifest: the workspace members under `crates/` and `examples/`,
/// and the standalone binding crates under `bindings/`.
fn crate_manifests(root: &Path) -> Result<Vec<PathBuf>, String> {
    let bindings = subdirectories(&root.join("bindings"))?;
    let mut binding_packages = Vec::new();
    for binding in &bindings {
        binding_packages.extend(subdirectories(&binding.join("packages"))?);
    }
    let mut manifests: Vec<PathBuf> = subdirectories(&root.join("crates"))?
        .into_iter()
        .chain(bindings)
        .chain(binding_packages)
        .chain([root.join("examples")])
        .map(|dir| dir.join("Cargo.toml"))
        .filter(|path| path.is_file())
        .collect();
    manifests.sort();
    Ok(manifests)
}

/// Every `Cargo.lock`: the workspace's and one per standalone binding crate, at a
/// binding's root or under one of its packages.
fn cargo_lockfiles(root: &Path) -> Vec<PathBuf> {
    let mut lockfiles = vec![root.join("Cargo.lock")];
    lockfiles.extend(binding_files(root, "Cargo.lock"));
    for binding in subdirectories(&root.join("bindings")).unwrap_or_default() {
        for package in subdirectories(&binding.join("packages")).unwrap_or_default() {
            lockfiles.push(package.join("Cargo.lock"));
        }
    }
    lockfiles
        .into_iter()
        .filter(|path| path.is_file())
        .collect()
}

/// Every npm manifest: a binding's workspace root, each package under its
/// `packages/`, and the platform packages under any of those.
fn package_manifests(root: &Path) -> Vec<PathBuf> {
    let mut manifests = binding_files(root, "package.json");
    for binding in subdirectories(&root.join("bindings")).unwrap_or_default() {
        for package in subdirectories(&binding.join("packages")).unwrap_or_default() {
            let manifest = package.join("package.json");
            if manifest.is_file() {
                manifests.push(manifest);
            }
            for platform in subdirectories(&package.join("npm")).unwrap_or_default() {
                let manifest = platform.join("package.json");
                if manifest.is_file() {
                    manifests.push(manifest);
                }
            }
        }
    }
    manifests.sort();
    manifests
}

/// Every Python project manifest: a binding's own and each package under its `packages/`.
fn pyprojects(root: &Path) -> Vec<PathBuf> {
    let mut manifests = binding_files(root, "pyproject.toml");
    for binding in subdirectories(&root.join("bindings")).unwrap_or_default() {
        for package in subdirectories(&binding.join("packages")).unwrap_or_default() {
            let manifest = package.join("pyproject.toml");
            if manifest.is_file() {
                manifests.push(manifest);
            }
        }
    }
    manifests.sort();
    manifests
}

/// A PyPI requirement that pins one of this repository's own distributions, as
/// (name, version): `pamoja-native==0.1.14` gives `("pamoja-native", "0.1.14")`.
fn python_pin(spec: &str) -> Option<(&str, &str)> {
    let (name, version) = spec.split_once("==")?;
    let own = name == "pamoja" || name.starts_with(CRATE_PREFIX);
    own.then_some((name, version))
}

/// Every generated napi-rs loader: `index.js` under a binding or under one of its
/// packages.
fn loader_files(root: &Path) -> Vec<PathBuf> {
    let mut loaders = binding_files(root, "index.js");
    for binding in subdirectories(&root.join("bindings")).unwrap_or_default() {
        for package in subdirectories(&binding.join("packages")).unwrap_or_default() {
            let loader = package.join("index.js");
            if loader.is_file() {
                loaders.push(loader);
            }
        }
    }
    loaders.sort();
    loaders
}

/// The dependency sections of an npm manifest whose entries pin sibling packages.
const PIN_SECTIONS: [&str; 3] = ["dependencies", "optionalDependencies", "peerDependencies"];

/// Whether an npm dependency name is one of this repository's own packages.
fn is_own_package(name: &str) -> bool {
    name == "pamoja" || name.starts_with("@pamoja/")
}

/// The pinned versions of this repository's own packages in an npm manifest, as
/// (name, version).
fn package_pins(file: &Path) -> Result<Vec<(String, String)>, String> {
    let document: serde_json::Value = serde_json::from_str(&read(file)?)
        .map_err(|err| format!("{} is not JSON: {err}", display(file)))?;
    let mut pins = Vec::new();
    for section in PIN_SECTIONS {
        let Some(entries) = document[section].as_object() else {
            continue;
        };
        for (name, version) in entries {
            if is_own_package(name) {
                let version = version.as_str().unwrap_or("?").to_owned();
                pins.push((name.clone(), version));
            }
        }
    }
    Ok(pins)
}

/// Rewrite the pinned version of every own-package dependency in npm manifest text,
/// line by line so the file's layout survives.
fn with_package_pins(text: &str, new: &str) -> String {
    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            let Some((key, rest)) = line.split_once("\": \"") else {
                return line.to_owned();
            };
            let name = key.trim_start().trim_start_matches('"');
            if !is_own_package(name) {
                return line.to_owned();
            }
            let Some(end) = rest.find('"') else {
                return line.to_owned();
            };
            format!("{key}\": \"{new}{}", &rest[end..])
        })
        .collect();
    let mut joined = lines.join("\n");
    if text.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// The file called `name` directly under each binding directory, where present.
fn binding_files(root: &Path, name: &str) -> Vec<PathBuf> {
    subdirectories(&root.join("bindings"))
        .unwrap_or_default()
        .into_iter()
        .map(|dir| dir.join(name))
        .filter(|path| path.is_file())
        .collect()
}

/// The subdirectories of `dir`, sorted; an absent directory has none.
fn subdirectories(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|err| format!("reading {}: {err}", display(dir)))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// The `version` string at `path` within a JSON file, `path` being the object
/// keys to descend before the `version` key.
fn json_version(file: &Path, path: &[&str]) -> Result<String, String> {
    let document: serde_json::Value = serde_json::from_str(&read(file)?)
        .map_err(|err| format!("{} is not JSON: {err}", display(file)))?;
    let mut node = &document;
    for key in path {
        node = &node[*key];
    }
    node["version"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{} has no version at {}", display(file), path.join(".")))
}

/// Replace the text between `before` and the next `after` in a file, at most
/// `limit` times.
fn rewrite(file: &Path, before: &str, after: &str, new: &str, limit: usize) -> Result<(), String> {
    let text = read(file)?;
    let (rewritten, count) = replace_between(&text, before, after, new, limit);
    if count == 0 {
        return Err(format!(
            "{} has no `{before}...{after}` to rewrite",
            display(file)
        ));
    }
    write(file, &rewritten)
}

/// Replace the text between each `before` and the `after` that follows it,
/// returning the new text and how many replacements were made.
fn replace_between(
    text: &str,
    before: &str,
    after: &str,
    new: &str,
    limit: usize,
) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut count = 0;
    while count < limit {
        let Some(start) = rest.find(before) else {
            break;
        };
        let value_start = start + before.len();
        let Some(len) = rest[value_start..].find(after) else {
            break;
        };
        out.push_str(&rest[..value_start]);
        out.push_str(new);
        rest = &rest[value_start + len..];
        count += 1;
    }
    out.push_str(rest);
    (out, count)
}

/// Every text between a `before` and the `after` that follows it.
fn find_between(text: &str, before: &str, after: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(before) {
        let value_start = start + before.len();
        let Some(len) = rest[value_start..].find(after) else {
            break;
        };
        found.push(rest[value_start..value_start + len].to_owned());
        rest = &rest[value_start + len..];
    }
    found
}

/// Require `x.y.z`, optionally followed by a `-pre` suffix.
fn validate(version: &str) -> Result<(), String> {
    let core = version.split_once('-').map_or(version, |(core, _)| core);
    let parts: Vec<&str> = core.split('.').collect();
    let numeric = parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()));
    if numeric {
        Ok(())
    } else {
        Err(format!("{version} is not a version of the form x.y.z"))
    }
}

/// Parse a TOML file keeping its formatting.
fn parse_toml(file: &Path) -> Result<DocumentMut, String> {
    read(file)?
        .parse()
        .map_err(|err| format!("{} is not valid TOML: {err}", display(file)))
}

/// Edit a TOML file in place, keeping its formatting.
fn edit_toml(
    file: &Path,
    edit: impl FnOnce(&mut DocumentMut) -> Result<(), String>,
) -> Result<(), String> {
    let mut doc = parse_toml(file)?;
    edit(&mut doc).map_err(|message| format!("{}: {message}", display(file)))?;
    write(file, &doc.to_string())
}

fn reading(file: &Path, what: &str, version: &str) -> Reading {
    Reading {
        file: file.to_path_buf(),
        what: what.to_owned(),
        version: version.to_owned(),
    }
}

fn read(file: &Path) -> Result<String, String> {
    fs::read_to_string(file).map_err(|err| format!("reading {}: {err}", display(file)))
}

fn write(file: &Path, text: &str) -> Result<(), String> {
    fs::write(file, text).map_err(|err| format!("writing {}: {err}", display(file)))
}

/// A path relative to the repository root with forward slashes, for messages.
fn display(path: &Path) -> String {
    let relative = path.strip_prefix(repo_root()).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root is two levels above the xtask crate")
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_between_markers_up_to_the_limit() {
        let text = "expected 0.1.14 but got x; expected 0.1.14 but got y";
        let (all, count) = replace_between(text, "expected ", " but got", "0.2.0", usize::MAX);
        assert_eq!(count, 2);
        assert_eq!(all, "expected 0.2.0 but got x; expected 0.2.0 but got y");
        let (first, count) = replace_between(text, "expected ", " but got", "0.2.0", 1);
        assert_eq!(count, 1);
        assert_eq!(first, "expected 0.2.0 but got x; expected 0.1.14 but got y");
    }

    #[test]
    fn finds_every_value_between_markers() {
        let text = "<Version>1.2.3</Version> <Version>1.2.4</Version>";
        assert_eq!(
            find_between(text, "<Version>", "</Version>"),
            ["1.2.3", "1.2.4"]
        );
        assert!(find_between("nothing here", "<Version>", "</Version>").is_empty());
    }

    #[test]
    fn validates_the_version_shape() {
        assert!(validate("0.1.15").is_ok());
        assert!(validate("1.0.0-rc.1").is_ok());
        assert!(validate("0.1").is_err());
        assert!(validate("v0.1.15").is_err());
        assert!(validate("0.1.x").is_err());
    }

    #[test]
    fn rewrites_manifest_versions_and_keeps_formatting() {
        let mut doc: DocumentMut = concat!(
            "[package]\n",
            "name = \"pamoja-demo\"\n",
            "version = \"0.1.14\" # the workspace version\n",
            "\n",
            "[dependencies]\n",
            "pamoja-core = { path = \"../pamoja-core\", version = \"0.1.14\" }\n",
            "pamoja-kit = { workspace = true }\n",
            "renamed = { package = \"pamoja-codec\", path = \"../pamoja-codec\", version = \"0.1.14\" }\n",
            "serde = { version = \"1\", features = [\"derive\"] }\n",
            "\n",
            "[dev-dependencies]\n",
            "pamoja-sync = { path = \"../pamoja-sync\" }\n",
            "\n",
            "[target.'cfg(unix)'.dependencies]\n",
            "pamoja-gpio = \"0.1.14\"\n",
        )
        .parse()
        .unwrap();

        let package = doc
            .get_mut("package")
            .and_then(Item::as_table_like_mut)
            .unwrap();
        set_version(package, "0.1.15").unwrap();
        for_each_dependency_table(&mut doc, &mut |_, deps| {
            set_dependency_versions(deps, "0.1.15")
        })
        .unwrap();

        let text = doc.to_string();
        assert!(text.contains("version = \"0.1.15\" # the workspace version\n"));
        assert!(
            text.contains("pamoja-core = { path = \"../pamoja-core\", version = \"0.1.15\" }\n")
        );
        assert!(text.contains("pamoja-kit = { workspace = true }\n"));
        assert!(text.contains("path = \"../pamoja-codec\", version = \"0.1.15\" }\n"));
        assert!(text.contains("serde = { version = \"1\", features = [\"derive\"] }\n"));
        assert!(text.contains("pamoja-sync = { path = \"../pamoja-sync\" }\n"));
        assert!(text.contains("pamoja-gpio = \"0.1.15\"\n"));
        assert!(!text.contains("0.1.14"));

        let mut readings = Vec::new();
        for (section, deps) in dependency_tables(&doc) {
            read_dependency_versions(Path::new("demo"), &section, deps, false, &mut readings)
                .unwrap();
        }
        let named: Vec<(String, String)> = readings
            .into_iter()
            .map(|reading| (reading.what, reading.version))
            .collect();
        assert_eq!(
            named,
            [
                (
                    "dependencies.pamoja-core.version".to_owned(),
                    "0.1.15".to_owned()
                ),
                (
                    "dependencies.renamed.version".to_owned(),
                    "0.1.15".to_owned()
                ),
                (
                    "target.cfg(unix).dependencies.pamoja-gpio.version".to_owned(),
                    "0.1.15".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn an_unpinned_dependency_fails_a_publishable_crate() {
        let doc: DocumentMut = "[dependencies]\npamoja-core = { path = \"../pamoja-core\" }\n"
            .parse()
            .unwrap();
        let mut readings = Vec::new();
        let deps = doc
            .get("dependencies")
            .and_then(Item::as_table_like)
            .unwrap();
        let err =
            read_dependency_versions(Path::new("demo"), "dependencies", deps, true, &mut readings)
                .unwrap_err();
        assert!(err.contains("dependencies.pamoja-core has no version"));
        read_dependency_versions(
            Path::new("demo"),
            "dependencies",
            deps,
            false,
            &mut readings,
        )
        .unwrap();
        assert!(readings.is_empty());
    }

    #[test]
    fn the_checked_out_tree_is_consistent() {
        let version = current().unwrap();
        for reading in readings().unwrap() {
            assert_eq!(
                reading.version,
                version,
                "{}: {}",
                display(&reading.file),
                reading.what
            );
        }
    }
}
