//! The documentation site, rendered from `docs/`.
//!
//! `cargo xtask site` renders every Markdown page under `docs/` into `target/site`, with the
//! navigation, the search index, the stylesheets and the marks beside them, and checks every
//! link before writing anything. The four API references are generated into the same tree
//! by their own tools afterwards, and `cargo xtask site --verify` then checks the finished
//! directory with nothing taken on trust. The pages are the committed Markdown, generated
//! regions included, so `cargo xtask docs --check` still gates what they say; this only
//! decides how they look.

mod assets;
mod check;
mod highlight;
mod layout;
mod markdown;
mod nav;
mod pages;
mod search;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::catalog::Catalog;
use crate::{docs, version};

use nav::Nav;

/// What a page is, which decides how its body is shaped.
pub enum Kind {
    /// A page read top to bottom.
    Article,
    /// A capability guide, whose four language sections fold into tabs.
    Guide,
}

/// One rendered page.
pub struct Page {
    /// The page, site-relative (`docs/guides/modbus.html`).
    pub url: String,
    /// The Markdown it came from, repository-relative.
    pub source: String,
    /// The text of the first heading.
    pub title: String,
    /// The first paragraph, for the description the head carries.
    pub description: String,
    /// What the page is.
    pub kind: Kind,
    /// The article body.
    pub body: String,
    /// The headings the table of contents lists.
    pub toc: Vec<markdown::Heading>,
    /// The page split at its second-level headings, for the search index.
    pub sections: Vec<markdown::Section>,
}

/// The whole site, loaded and ready to render.
pub struct Site {
    root: PathBuf,
    version: String,
    nav: Nav,
    pages: Vec<Page>,
}

/// Run the `site` task.
///
/// # Arguments
///
/// * `args` - `[--out <dir>]` renders into `dir` (default `target/site`); `--verify [<dir>]`
///   checks every link of a finished site on disk instead.
///
/// # Returns
///
/// Success when the site was written, or when the verification found every link resolving.
pub fn run(args: &[String]) -> ExitCode {
    let root = docs::repo_root();
    let default_out = root.join("target/site");
    let result = if args.first().map(String::as_str) == Some("--verify") {
        let dir = args.get(1).map_or(default_out, PathBuf::from);
        verify(&dir)
    } else {
        let out = match args.iter().position(|arg| arg == "--out") {
            Some(at) => match args.get(at + 1) {
                Some(dir) => PathBuf::from(dir),
                None => {
                    eprintln!("xtask site: --out needs a directory");
                    return ExitCode::FAILURE;
                }
            },
            None => default_out,
        };
        Site::load(&root).and_then(|site| site.write(&out))
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask site: {message}");
            ExitCode::FAILURE
        }
    }
}

impl Site {
    /// Read the map, the version, and every page.
    ///
    /// # Arguments
    ///
    /// * `root` - the repository root.
    ///
    /// # Returns
    ///
    /// The site, ready to render.
    ///
    /// # Errors
    ///
    /// When the map or a page cannot be read, or a page is missing from the navigation.
    pub fn load(root: &Path) -> Result<Site, String> {
        let catalog = Catalog::load(root)?;
        let nav = Nav::from(&catalog);
        let pages = pages::load(root, &nav)?;
        Ok(Site {
            root: root.to_path_buf(),
            version: version::current()?,
            nav,
            pages,
        })
    }

    /// Render every file of the site and check every link between them.
    ///
    /// # Returns
    ///
    /// The files as (site path, contents), sorted by path.
    ///
    /// # Errors
    ///
    /// When a static file is missing, or a link points at a page, file, or fragment that is
    /// not produced. Links into the four generated reference trees are taken on trust here.
    pub fn render(&self) -> Result<Vec<(String, Vec<u8>)>, String> {
        let chrome = layout::Chrome {
            version: &self.version,
            nav: &self.nav,
        };
        let mut files: Vec<(String, Vec<u8>)> = self
            .pages
            .iter()
            .map(|page| {
                (
                    page.url.clone(),
                    layout::document(&chrome, page).into_bytes(),
                )
            })
            .collect();
        files.push((
            "404.html".to_owned(),
            layout::not_found(&chrome).into_bytes(),
        ));
        files.push((
            "search.json".to_owned(),
            search::index(&self.pages, &self.nav).into_bytes(),
        ));
        files.extend(assets::files(&self.root)?);
        files.sort_by(|a, b| a.0.cmp(&b.0));
        check::check(&check::Rendered::new(&files), &check::GENERATED)?;
        Ok(files)
    }

    /// Render the site and write it under `out`, reporting what was written.
    ///
    /// # Arguments
    ///
    /// * `out` - the directory to write into; created if needed, and existing files with
    ///   the same paths are overwritten.
    ///
    /// # Errors
    ///
    /// When rendering fails or a file cannot be written.
    pub fn write(&self, out: &Path) -> Result<(), String> {
        let files = self.render()?;
        let mut total = 0usize;
        for (path, body) in &files {
            let target = out.join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("creating {}: {err}", parent.display()))?;
            }
            fs::write(&target, body)
                .map_err(|err| format!("writing {}: {err}", target.display()))?;
            total += body.len();
        }
        let mut largest: Vec<&(String, Vec<u8>)> = files.iter().collect();
        largest.sort_by_key(|(_, body)| std::cmp::Reverse(body.len()));
        println!(
            "site: wrote {} files ({} KB) to {}",
            files.len(),
            total / 1024,
            out.display()
        );
        for (path, body) in largest.iter().take(5) {
            println!("  {:>6} KB  {path}", body.len() / 1024);
        }
        Ok(())
    }
}

/// Check every link of a finished site on disk, trusting nothing.
///
/// # Arguments
///
/// * `dir` - the site root, with the generated references in place.
///
/// # Errors
///
/// Every broken link, one per line.
pub fn verify(dir: &Path) -> Result<(), String> {
    if !dir.join("docs/index.html").is_file() {
        return Err(format!(
            "{} holds no site; run `cargo xtask site` first",
            dir.display()
        ));
    }
    check::check(&check::OnDisk::new(dir), &[])?;
    println!("site: every link under {} resolves", dir.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn site() -> Site {
        Site::load(&docs::repo_root()).expect("the site loads")
    }

    /// The URLs the site has published; each keeps resolving.
    const PUBLISHED: [&str; 12] = [
        "docs/index.html",
        "docs/install.html",
        "docs/hardware.html",
        "docs/reference/rust.html",
        "docs/reference/node.html",
        "docs/reference/python.html",
        "docs/reference/dotnet.html",
        "docs/about/why.html",
        "docs/about/architecture.html",
        "docs/about/standards.html",
        "docs/about/building.html",
        "docs/about/releasing.html",
    ];

    #[test]
    fn every_published_url_is_produced() {
        let site = site();
        let files = site
            .render()
            .expect("the site renders and its links resolve");
        let paths: BTreeSet<&str> = files.iter().map(|(path, _)| path.as_str()).collect();
        for url in PUBLISHED {
            assert!(paths.contains(url), "{url} is no longer produced");
        }
        let catalog = Catalog::load(&site.root).unwrap();
        for capability in &catalog.capabilities {
            let guide = capability
                .guide
                .as_deref()
                .expect("every capability has a guide");
            let url = format!("docs/{}.html", guide.trim_end_matches(".md"));
            assert!(paths.contains(url.as_str()), "{url} is no longer produced");
        }
        for asset in [
            "docs/assets/pamoja-logo.svg",
            "docs/assets/pamoja-icon.svg",
            "assets/pamoja-logo.svg",
            "site.css",
            "theme.css",
            "js/site.js",
            "search.json",
            "404.html",
        ] {
            assert!(paths.contains(asset), "{asset} is no longer produced");
        }
    }

    #[test]
    fn every_page_is_in_the_navigation_and_every_navigation_entry_is_a_page() {
        let site = site();
        let urls: BTreeSet<&str> = site.pages.iter().map(|page| page.url.as_str()).collect();
        for item in site.nav.items() {
            assert!(
                urls.contains(item.url.as_str()),
                "{} leads nowhere",
                item.url
            );
        }
        assert_eq!(urls.len(), site.nav.items().count());
    }

    #[test]
    fn the_hardware_page_keeps_its_part_anchors_and_the_guides_their_tabs() {
        let site = site();
        let hardware = site
            .pages
            .iter()
            .find(|page| page.url == "docs/hardware.html")
            .unwrap();
        let ids = check::ids_in(&hardware.body);
        for id in ["bme280", "ds18b20", "ina219", "ads1115", "pca9685"] {
            assert!(ids.contains(id), "hardware.html lost #{id}");
        }
        let guides = site
            .pages
            .iter()
            .filter(|page| matches!(page.kind, Kind::Guide))
            .count();
        assert_eq!(
            guides,
            Catalog::load(&site.root).unwrap().capabilities.len()
        );
        assert!(site
            .pages
            .iter()
            .filter(|page| matches!(page.kind, Kind::Guide))
            .all(|page| page.body.contains("class=\"lang-tabs\"")));
    }

    /// Whether `text` holds a `#rrggbb`-style colour literal: a hash, three to eight hex
    /// digits, and then something that is not a word character.
    fn has_colour_literal(text: &str) -> bool {
        let bytes = text.as_bytes();
        let mut at = 0;
        while let Some(offset) = text[at..].find('#') {
            let start = at + offset + 1;
            let digits = bytes[start..]
                .iter()
                .take_while(|byte| byte.is_ascii_hexdigit())
                .count();
            let terminated = bytes
                .get(start + digits)
                .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'-' && *byte != b'_');
            if matches!(digits, 3 | 4 | 6 | 8) && terminated {
                return true;
            }
            at = start;
        }
        false
    }

    #[test]
    fn the_token_sheet_is_the_only_place_colours_live() {
        let css = fs::read_to_string(docs::repo_root().join("web/site.css")).unwrap();
        assert!(
            !has_colour_literal(&css),
            "web/site.css names a colour instead of a token from theme.css"
        );
        assert!(has_colour_literal("color: #fff;"));
        assert!(has_colour_literal("border: 1px solid #16263f"));
        assert!(!has_colour_literal(
            "a[href^=\"#\"] { color: var(--teal); }"
        ));
    }

    #[test]
    fn the_search_index_stays_small() {
        let site = site();
        let index = search::index(&site.pages, &site.nav);
        assert!(index.len() < 250 * 1024, "{} bytes", index.len());
        assert!(index.contains("docs/guides/modbus.html#what-the-example-does"));
    }
}
