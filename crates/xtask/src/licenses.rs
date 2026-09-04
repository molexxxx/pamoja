//! The licence text, copied into every package that is published.
//!
//! Each registry is told the licence is MIT, and MIT itself asks that the notice travel
//! with the copies. A `license = "MIT"` line is metadata, not the notice, so the text goes
//! into every publishable package directory as well: cargo, npm, and the Python build all
//! include a `LICENSE*` file from the package root without being asked, and the .NET
//! projects pack it explicitly. `cargo xtask docs --check` fails if a copy drifts from the
//! one at the repository root.

use std::fs;
use std::path::Path;

/// The file every package carries, named as the repository names it.
const NAME: &str = "LICENSE-MIT";

/// Copy the licence into every publishable package.
///
/// # Arguments
///
/// * `root` - the repository root, which holds the one copy the rest are made from.
///
/// # Returns
///
/// One entry per package, as (path, contents).
///
/// # Errors
///
/// If the licence at the repository root cannot be read, or a package directory cannot
/// be listed.
pub fn render(root: &Path) -> Result<Vec<(String, String)>, String> {
    let source = root.join(NAME);
    let text = fs::read_to_string(&source)
        .map_err(|err| format!("reading {}: {err}", source.display()))?;

    let mut out = Vec::new();
    for (directory, publishable) in [
        ("crates", publishable_crate as fn(&Path) -> bool),
        ("bindings/node/packages", published),
        ("bindings/python/packages", published),
        ("bindings/dotnet/src", published),
    ] {
        for package in packages(&root.join(directory))? {
            if !publishable(&package) {
                continue;
            }
            let name = package
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("unreadable directory name under {directory}"))?;
            out.push((format!("{directory}/{name}/{NAME}"), text.clone()));
        }
    }
    Ok(out)
}

// The directories directly under one of the package roots, sorted so the output is stable.
fn packages(directory: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let entries =
        fs::read_dir(directory).map_err(|err| format!("reading {}: {err}", directory.display()))?;
    let mut paths: Vec<std::path::PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    paths.sort();
    Ok(paths)
}

// A crate that says `publish = false` never reaches a registry, so it needs no copy.
fn publishable_crate(package: &Path) -> bool {
    match fs::read_to_string(package.join("Cargo.toml")) {
        Ok(manifest) => !manifest.contains("publish = false"),
        Err(_) => false,
    }
}

// Every binding package directory is published.
fn published(_package: &Path) -> bool {
    true
}
