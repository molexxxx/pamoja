//! The documentation site, rendered from `docs/`.
//!
//! `cargo xtask site` renders the front page and every Markdown page under `docs/` into
//! `target/site`, with the navigation, the search index, the stylesheets, the typefaces and
//! the marks beside them, and checks every link before writing anything. The four API references are generated into the same tree
//! by their own tools afterwards, and `cargo xtask site --verify` then checks the finished
//! directory with nothing taken on trust. The pages are the committed Markdown, generated
//! regions included, so `cargo xtask docs --check` still gates what they say; this only
//! decides how they look.

mod assets;
mod check;
mod highlight;
mod home;
mod layout;
mod markdown;
mod nav;
mod pages;
mod search;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::catalog::Catalog;
use crate::{docs, version};

use home::Home;
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

/// The root pages of the four generated reference trees, each handed off to the page here
/// that lists that language's packages, so a tree has no front door of its own. `site`
/// runs after the generators and overwrites what they put at these paths; pdoc names its
/// root after the package and also writes an index that points at it.
const HANDOFFS: [(&str, &str, &str); 5] = [
    ("docs/reference/rust/index.html", "rust", "Rust"),
    ("docs/reference/node/index.html", "node", "TypeScript"),
    ("docs/reference/python/index.html", "python", "Python"),
    ("docs/reference/python/pamoja.html", "python", "Python"),
    ("docs/reference/dotnet/index.html", "dotnet", "C#"),
];

/// The whole site, loaded and ready to render.
pub struct Site {
    root: PathBuf,
    version: String,
    catalog: Catalog,
    lib_crates: Vec<String>,
    descriptions: BTreeMap<String, String>,
    home: Home,
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
    /// When the map, the front page's data, or a page cannot be read, when a page is missing
    /// from the navigation, or when the front page's data disagrees with the workspace.
    pub fn load(root: &Path) -> Result<Site, String> {
        let catalog = Catalog::load(root)?;
        let lib_crates = docs::lib_crates()?;
        let descriptions = lib_crates
            .iter()
            .filter_map(|krate| docs::crate_description(krate).map(|text| (krate.clone(), text)))
            .collect();
        let home = Home::load(root)?;
        let consoles = fs::read_to_string(root.join("web/js/consoles.js"))
            .map_err(|err| format!("reading web/js/consoles.js: {err}"))?;
        home.check(&lib_crates, &consoles)?;
        let nav = Nav::from(&catalog);
        let pages = pages::load(root, &nav)?;
        Ok(Site {
            root: root.to_path_buf(),
            version: version::current()?,
            catalog,
            lib_crates,
            descriptions,
            home,
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
        let body = self.home.render(
            &self.root,
            &self.catalog,
            &self.lib_crates,
            &self.descriptions,
        )?;
        files.push((
            "index.html".to_owned(),
            layout::home(&chrome, &body).into_bytes(),
        ));
        files.push((
            "404.html".to_owned(),
            layout::not_found(&chrome).into_bytes(),
        ));
        for (path, key, name) in HANDOFFS {
            files.push((
                path.to_owned(),
                layout::redirect(path, &format!("../{key}.html"), name).into_bytes(),
            ));
        }
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
    const PUBLISHED: [&str; 13] = [
        "index.html",
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
            "home.css",
            "theme.css",
            "fonts/fonts.css",
            "fonts/Inter.woff2",
            "js/site.js",
            "js/home.js",
            "js/consoles.js",
            "search.json",
            "404.html",
            ".nojekyll",
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
        for sheet in ["web/site.css", "web/home.css"] {
            let css = fs::read_to_string(docs::repo_root().join(sheet)).unwrap();
            assert!(
                !has_colour_literal(&css),
                "{sheet} names a colour instead of a token from theme.css"
            );
        }
        assert!(has_colour_literal("color: #fff;"));
        assert!(has_colour_literal("border: 1px solid #16263f"));
        assert!(!has_colour_literal(
            "a[href^=\"#\"] { color: var(--teal); }"
        ));
    }

    #[test]
    fn the_generated_trees_hand_off_to_the_reference_pages() {
        let files = site().render().unwrap();
        for (path, key, _) in HANDOFFS {
            let body = files
                .iter()
                .find(|(produced, _)| produced == path)
                .map(|(_, body)| String::from_utf8_lossy(body).into_owned())
                .unwrap_or_else(|| panic!("{path} is not produced"));
            assert!(
                body.contains(&format!("content=\"0; url=../{key}.html\"")),
                "{path} does not hand off to {key}.html"
            );
        }
    }

    /// The front page and everything a browser fetches to show it, without the typefaces,
    /// gzipped as a server would send it, must fit a slow link.
    #[test]
    fn the_front_page_fits_its_budget() {
        use std::io::Write as _;
        let files = site().render().unwrap();
        let mut total = 0usize;
        for path in [
            "index.html",
            "site.css",
            "home.css",
            "theme.css",
            "fonts/fonts.css",
            "js/site.js",
            "js/home.js",
            "js/consoles.js",
            "assets/pamoja-icon.svg",
        ] {
            let body = &files
                .iter()
                .find(|(produced, _)| produced == path)
                .unwrap_or_else(|| panic!("{path} is not produced"))
                .1;
            let mut encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
            encoder.write_all(body).unwrap();
            total += encoder.finish().unwrap().len();
        }
        assert!(
            total < 200 * 1024,
            "the front page costs {total} bytes gzipped"
        );
    }

    #[test]
    fn every_scenario_has_a_console_and_the_front_page_shows_them_all() {
        let site = site();
        let files = site.render().unwrap();
        let index = String::from_utf8_lossy(
            &files
                .iter()
                .find(|(path, _)| path == "index.html")
                .unwrap()
                .1,
        )
        .into_owned();
        for key in site.home.scenario_keys() {
            assert!(
                index.contains(&format!("data-diorama=\"{key}\"")),
                "the front page has no stage for {key}"
            );
        }
        assert!(index.contains("class=\"bento-card span-big\""));
        assert!(
            index.contains("id=\"quick-python\""),
            "the first example is spliced"
        );
    }

    #[test]
    fn the_search_index_stays_small() {
        let site = site();
        let index = search::index(&site.pages, &site.nav);
        assert!(index.len() < 250 * 1024, "{} bytes", index.len());
        assert!(index.contains("docs/guides/modbus.html#what-the-example-does"));
    }
}
