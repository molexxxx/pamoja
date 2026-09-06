//! The static files the pages load: the stylesheets, the scripts, the typefaces, and the
//! marks, plus the file that tells Pages not to run Jekyll over the tree.

use std::fs;
use std::path::Path;

/// The files copied into the site as they are, as (source under the repository root,
/// destination under the site root).
const COPIED: [(&str, &str); 6] = [
    ("web/theme.css", "theme.css"),
    ("web/site.css", "site.css"),
    ("web/home.css", "home.css"),
    ("web/js/site.js", "js/site.js"),
    ("web/js/home.js", "js/home.js"),
    ("web/js/consoles.js", "js/consoles.js"),
];

/// The directories copied whole, as (source, destination, the extensions taken, or none
/// for every file).
const DIRECTORIES: [(&str, &str, &[&str]); 3] = [
    ("web/assets", "assets", &["svg"]),
    ("docs/assets", "docs/assets", &["svg"]),
    ("web/fonts", "fonts", &[]),
];

/// Read every static file.
///
/// # Arguments
///
/// * `root` - the repository root.
///
/// # Returns
///
/// The files as (site path, contents).
///
/// # Errors
///
/// When a file is missing or unreadable.
pub fn files(root: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    for (source, destination) in COPIED {
        out.push((destination.to_owned(), read(&root.join(source))?));
    }
    out.push((".nojekyll".to_owned(), read(&root.join("web/.nojekyll"))?));
    for (source, destination, extensions) in DIRECTORIES {
        let dir = root.join(source);
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .map_err(|err| format!("reading {}: {err}", dir.display()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file())
            .filter(|path| {
                extensions.is_empty()
                    || path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| extensions.contains(&ext))
            })
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("{} has no name", path.display()))?;
            out.push((format!("{destination}/{name}"), read(&path)?));
        }
    }
    Ok(out)
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|err| format!("reading {}: {err}", path.display()))
}
