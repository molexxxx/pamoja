//! Link checks: nothing the site links to may be missing.
//!
//! The same check runs twice. While the site is rendered it runs over the pages in memory,
//! where a link into one of the four generated reference trees is taken on trust because
//! those trees are built by their own tools afterwards. `cargo xtask site --verify` runs it
//! again over the finished directory with nothing trusted, so a reference URL that a
//! generator stopped producing fails the build rather than the reader. Only the site's own
//! pages are walked; the generated trees' internal links are their generators' business,
//! but every link from a page of ours into them must resolve, fragment included.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::layout;

/// Where a link points, site-relative.
#[derive(Debug, PartialEq, Eq)]
pub struct Target {
    /// The file, without a leading slash (`docs/guides/modbus.html`).
    pub path: String,
    /// The `#fragment`, without the hash.
    pub fragment: Option<String>,
}

/// The files a check runs over: a set in memory, or a directory on disk.
pub trait Corpus {
    /// Whether `path` exists.
    fn exists(&self, path: &str) -> bool;
    /// The HTML at `path`, when it is an HTML file that exists.
    fn html(&self, path: &str) -> Option<String>;
    /// Every HTML page whose links are checked.
    fn pages(&self) -> Vec<String>;
}

/// The rendered site before it is written.
pub struct Rendered<'a> {
    files: BTreeMap<&'a str, &'a [u8]>,
}

impl<'a> Rendered<'a> {
    /// Wrap the render output.
    pub fn new(files: &'a [(String, Vec<u8>)]) -> Rendered<'a> {
        Rendered {
            files: files
                .iter()
                .map(|(path, body)| (path.as_str(), body.as_slice()))
                .collect(),
        }
    }
}

impl Corpus for Rendered<'_> {
    fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    fn html(&self, path: &str) -> Option<String> {
        if !path.ends_with(".html") {
            return None;
        }
        self.files
            .get(path)
            .map(|body| String::from_utf8_lossy(body).into_owned())
    }

    fn pages(&self) -> Vec<String> {
        self.files
            .keys()
            .filter(|path| path.ends_with(".html"))
            .map(|path| (*path).to_owned())
            .collect()
    }
}

/// A built site on disk.
pub struct OnDisk {
    dir: PathBuf,
}

impl OnDisk {
    /// Check the site under `dir`.
    pub fn new(dir: &Path) -> OnDisk {
        OnDisk {
            dir: dir.to_path_buf(),
        }
    }
}

impl Corpus for OnDisk {
    fn exists(&self, path: &str) -> bool {
        self.dir.join(path).is_file()
    }

    fn html(&self, path: &str) -> Option<String> {
        if !path.ends_with(".html") {
            return None;
        }
        fs::read_to_string(self.dir.join(path)).ok()
    }

    fn pages(&self) -> Vec<String> {
        let mut out = Vec::new();
        walk(&self.dir, &self.dir, &mut out);
        out.sort();
        out
    }
}

// Every HTML file under `dir`, site-relative, leaving the generated reference trees alone.
fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
        let relative = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            if !GENERATED.contains(&relative.as_str()) {
                walk(base, &path, out);
            }
        } else if relative.ends_with(".html") {
            out.push(relative);
        }
    }
}

/// The four reference trees, built by their own generators after the pages are rendered.
pub const GENERATED: [&str; 4] = [
    "docs/reference/rust",
    "docs/reference/node",
    "docs/reference/python",
    "docs/reference/dotnet",
];

/// What is deployed beside the site rather than rendered with it: the dashboard, which
/// pages.yml builds from its own crate and copies in.
const BESIDE: [&str; 1] = ["dashboard"];

/// Check every link of every page in `corpus`.
///
/// # Arguments
///
/// * `corpus` - the pages and files to check.
/// * `trusted` - path prefixes a link may point into without the target existing.
///
/// # Returns
///
/// Nothing when every link resolves.
///
/// # Errors
///
/// Every broken link, one per line: a target that does not exist, or a fragment the target
/// page has no id for.
pub fn check(corpus: &dyn Corpus, trusted: &[&str]) -> Result<(), String> {
    let mut problems = Vec::new();
    let mut ids: BTreeMap<String, Option<BTreeSet<String>>> = BTreeMap::new();
    for page in corpus.pages() {
        let Some(html) = corpus.html(&page) else {
            continue;
        };
        for link in links_in(&html) {
            let Some(target) = resolve(&page, &link) else {
                continue;
            };
            if trusted
                .iter()
                .chain(BESIDE.iter())
                .any(|prefix| target.path.starts_with(&format!("{prefix}/")))
            {
                continue;
            }
            if !corpus.exists(&target.path) {
                problems.push(format!("{page}: `{link}` points at nothing"));
                continue;
            }
            let Some(fragment) = &target.fragment else {
                continue;
            };
            let known = ids
                .entry(target.path.clone())
                .or_insert_with(|| corpus.html(&target.path).map(|html| ids_in(&html)));
            match known {
                Some(known) if !known.contains(fragment) => problems.push(format!(
                    "{page}: `{link}` names a fragment {} has no id for",
                    target.path
                )),
                _ => {}
            }
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} broken link(s):\n  {}",
            problems.len(),
            problems.join("\n  ")
        ))
    }
}

/// Every `href` and `src` value in `html`, in order.
pub fn links_in(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    for attribute in ["href=\"", "src=\""] {
        let mut rest = html;
        while let Some(at) = rest.find(attribute) {
            let value = &rest[at + attribute.len()..];
            let Some(end) = value.find('"') else {
                break;
            };
            out.push(value[..end].to_owned());
            rest = &value[end..];
        }
    }
    out
}

/// Every `id="..."` value in `html`.
pub fn ids_in(html: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = html;
    while let Some(at) = rest.find(" id=\"") {
        let value = &rest[at + 5..];
        let Some(end) = value.find('"') else {
            break;
        };
        out.insert(value[..end].to_owned());
        rest = &value[end..];
    }
    out
}

/// Where a link on the page at `from` points, or `None` for a link that leaves the site.
/// An absolute link to the site's own origin is followed like a root-relative one, since
/// the committed regions that GitHub renders too are written that way.
///
/// # Arguments
///
/// * `from` - the page the link is on, site-relative.
/// * `link` - the `href` as written.
///
/// # Returns
///
/// The site-relative target, a directory link resolved to its `index.html`.
pub fn resolve(from: &str, link: &str) -> Option<Target> {
    let link = match link.strip_prefix(layout::ORIGIN) {
        Some("") => "/",
        Some(rest) if rest.starts_with('/') || rest.starts_with('#') => rest,
        _ => link,
    };
    if link.contains("://")
        || link.starts_with("//")
        || link.starts_with("mailto:")
        || link.starts_with("javascript:")
        || link.starts_with("data:")
    {
        return None;
    }
    let (path, fragment) = match link.find('#') {
        Some(at) => (&link[..at], Some(link[at + 1..].to_owned())),
        None => (link, None),
    };
    let path = path.split('?').next().unwrap_or_default();
    let path = if path.is_empty() {
        from.to_owned()
    } else if let Some(absolute) = path.strip_prefix('/') {
        absolute.to_owned()
    } else {
        let base = from.rsplit_once('/').map_or("", |(dir, _)| dir);
        let joined = if base.is_empty() {
            path.to_owned()
        } else {
            format!("{base}/{path}")
        };
        normalize(&joined)
    };
    let path = if path.is_empty() || path.ends_with('/') {
        format!("{path}index.html")
    } else {
        path
    };
    Some(Target { path, fragment })
}

fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(path: &str, fragment: Option<&str>) -> Option<Target> {
        Some(Target {
            path: path.to_owned(),
            fragment: fragment.map(str::to_owned),
        })
    }

    #[test]
    fn links_resolve_against_the_page_they_are_on() {
        let from = "docs/guides/modbus.html";
        assert_eq!(
            resolve(from, "../install.html"),
            target("docs/install.html", None)
        );
        assert_eq!(resolve(from, "#rust"), target(from, Some("rust")));
        assert_eq!(
            resolve(from, "../reference/rust/pamoja_modbus/index.html"),
            target("docs/reference/rust/pamoja_modbus/index.html", None)
        );
        assert_eq!(resolve(from, "../../"), target("index.html", None));
        assert_eq!(
            resolve(from, "/docs/index.html#a"),
            target("docs/index.html", Some("a"))
        );
        assert_eq!(
            resolve(from, "can.html?x=1#rust"),
            target("docs/guides/can.html", Some("rust"))
        );
        assert_eq!(resolve(from, "https://docs.rs/pamoja"), None);
        assert_eq!(
            resolve(from, "https://pamoja.molex.cloud/docs/install.html#node"),
            target("docs/install.html", Some("node"))
        );
        assert_eq!(
            resolve(from, "https://pamoja.molex.cloud"),
            target("index.html", None)
        );
        assert_eq!(
            resolve(from, "https://pamoja.molex.cloud/dashboard/"),
            target("dashboard/index.html", None)
        );
        assert_eq!(resolve(from, "https://pamoja.molex.cloud.example/"), None);
        assert_eq!(resolve(from, "mailto:x@y.z"), None);
    }

    #[test]
    fn attributes_and_ids_are_found() {
        let html = "<a href=\"a.html\">x</a><img src=\"m.svg\"><h2 id=\"one\">1</h2><div id=\"two\"></div>";
        assert_eq!(links_in(html), ["a.html", "m.svg"]);
        assert_eq!(
            ids_in(html),
            BTreeSet::from(["one".to_owned(), "two".to_owned()])
        );
    }

    #[test]
    fn broken_targets_and_fragments_are_reported_and_trusted_prefixes_are_not() {
        let files = vec![
            (
                "docs/a.html".to_owned(),
                b"<a href=\"b.html#here\">ok</a><a href=\"b.html#gone\">bad</a><a href=\"c.html\">missing</a><a href=\"reference/rust/x/index.html\">trusted</a><a href=\"../site.css\">asset</a>".to_vec(),
            ),
            ("docs/b.html".to_owned(), b"<h1 id=\"here\">B</h1>".to_vec()),
            ("site.css".to_owned(), b"body{}".to_vec()),
        ];
        let corpus = Rendered::new(&files);
        let err = check(&corpus, &["docs/reference/rust"]).unwrap_err();
        assert!(err.contains("2 broken link(s)"), "{err}");
        assert!(err.contains("`b.html#gone` names a fragment docs/b.html has no id for"));
        assert!(err.contains("`c.html` points at nothing"));
        assert!(!err.contains("trusted") && !err.contains("site.css"));

        let strict = check(&corpus, &[]).unwrap_err();
        assert!(strict.contains("3 broken link(s)"), "{strict}");
    }
}
