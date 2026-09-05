//! The documentation pages: every Markdown file under `docs/`, rendered.
//!
//! A guide's four language sections become one tab block, so a reader who works in Python
//! sees the Python example where the Rust one would otherwise sit; the chosen language
//! persists between pages, and a `#python`-style anchor selects a tab. All four stay in the
//! page, so a reader without JavaScript gets them stacked, as before.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::markdown;
use super::nav::Nav;
use super::{Kind, Page};

/// The four languages every guide shows, as (tab label, panel id) pairs; the id is the slug
/// the heading has always had, so a link to `#typescript` still lands.
const LANGUAGES: [(&str, &str); 4] = [
    ("Rust", "rust"),
    ("TypeScript", "typescript"),
    ("Python", "python"),
    ("C#", "c"),
];

/// Render every page under `docs/`.
///
/// # Arguments
///
/// * `root` - the repository root.
/// * `nav` - the navigation, which every page must appear in.
///
/// # Returns
///
/// The pages, in path order.
///
/// # Errors
///
/// When a page cannot be read, or is not in the navigation.
pub fn load(root: &Path, nav: &Nav) -> Result<Vec<Page>, String> {
    let mut sources = Vec::new();
    crate::docs::collect_markdown(&root.join("docs"), &mut sources)?;
    let mut pages = Vec::new();
    for path in sources {
        let source = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let url = markdown::rewrite_link(&source);
        if nav.item(&url).is_none() {
            return Err(format!(
                "{source} is not in the navigation; add it to crates/xtask/src/site/nav.rs"
            ));
        }
        let text = fs::read_to_string(&path).map_err(|err| format!("reading {source}: {err}"))?;
        pages.push(page(&source, &url, &text));
    }
    Ok(pages)
}

/// Render one page from its Markdown.
///
/// # Arguments
///
/// * `source` - the Markdown file, repository-relative.
/// * `url` - the page it becomes, site-relative.
/// * `text` - the Markdown.
///
/// # Returns
///
/// The page, with a guide's language sections folded into tabs.
pub fn page(source: &str, url: &str, text: &str) -> Page {
    let rendered = markdown::render(text);
    let kind = if url.starts_with("docs/guides/") {
        Kind::Guide
    } else {
        Kind::Article
    };
    let (body, toc) = match kind {
        Kind::Guide => match language_tabs(&rendered.html) {
            Some(html) => {
                let language_ids: BTreeSet<&str> = LANGUAGES.iter().map(|(_, id)| *id).collect();
                let toc = rendered
                    .headings
                    .into_iter()
                    .filter(|heading| {
                        !(heading.level == 2 && language_ids.contains(heading.id.as_str()))
                    })
                    .collect();
                (html, toc)
            }
            None => (rendered.html, rendered.headings),
        },
        Kind::Article => (rendered.html, rendered.headings),
    };
    Page {
        url: url.to_owned(),
        source: source.to_owned(),
        title: rendered.title,
        description: rendered.description,
        kind,
        body,
        toc,
        sections: rendered.sections,
    }
}

/// Fold the four consecutive language sections of a guide into a tab block. The headings
/// keep their ids on the panels, so the anchors the guides have always had still resolve.
///
/// # Arguments
///
/// * `html` - the rendered guide.
///
/// # Returns
///
/// The guide with the tab block in place of the four sections, or `None` when the four
/// headings are not all present in order.
pub fn language_tabs(html: &str) -> Option<String> {
    let mut starts = Vec::with_capacity(LANGUAGES.len());
    let mut from = 0;
    for (_, id) in LANGUAGES {
        let marker = format!("<h2 id=\"{id}\">");
        let at = html[from..].find(&marker)? + from;
        starts.push(at);
        from = at + marker.len();
    }
    let end = html[from..]
        .find("<h2 id=\"")
        .map_or(html.len(), |at| at + from);

    let mut out = String::with_capacity(html.len() + 1024);
    out.push_str(&html[..starts[0]]);
    out.push_str("<div class=\"langs\">\n<div class=\"lang-tabs\" role=\"tablist\" aria-label=\"Language\">\n");
    for (label, id) in LANGUAGES {
        out.push_str(&format!(
            "<button class=\"lang-tab\" role=\"tab\" type=\"button\" id=\"tab-{id}\" aria-controls=\"{id}\" aria-selected=\"false\" data-lang=\"{id}\">{label}</button>\n"
        ));
    }
    out.push_str("</div>\n");
    for (index, (_, id)) in LANGUAGES.iter().enumerate() {
        let start = starts[index];
        let stop = starts.get(index + 1).copied().unwrap_or(end);
        let heading = format!("<h2 id=\"{id}\">");
        let section = html[start..stop].replacen(&heading, "<h2 class=\"lang-heading\">", 1);
        out.push_str(&format!(
            "<section class=\"lang-panel\" id=\"{id}\" role=\"tabpanel\" aria-labelledby=\"tab-{id}\" data-lang=\"{id}\" tabindex=\"0\">\n{section}</section>\n"
        ));
    }
    out.push_str("</div>\n");
    out.push_str(&html[end..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUIDE: &str = "# Modbus RTU\n\nIntro.\n\n## What the example does\n\nIt polls.\n\n## Rust\n\nrust body\n\n## TypeScript\n\nts body\n\n## Python\n\npy body\n\n## C#\n\ncs body\n\n## Reference\n\n- links\n";

    #[test]
    fn a_guide_folds_its_languages_into_tabs_and_keeps_the_anchors() {
        let page = page("docs/guides/modbus.md", "docs/guides/modbus.html", GUIDE);
        assert!(matches!(page.kind, Kind::Guide));
        assert!(page
            .body
            .contains("<div class=\"lang-tabs\" role=\"tablist\""));
        assert!(page.body.contains("<button class=\"lang-tab\" role=\"tab\" type=\"button\" id=\"tab-python\" aria-controls=\"python\""));
        assert!(page.body.contains(
            "<section class=\"lang-panel\" id=\"c\" role=\"tabpanel\" aria-labelledby=\"tab-c\""
        ));
        assert!(page
            .body
            .contains("<h2 class=\"lang-heading\">Python<a class=\"anchor\" href=\"#python\""));
        assert!(
            page.body.contains("<h2 id=\"reference\">Reference"),
            "what follows the tabs is untouched"
        );
        let ids = super::super::check::ids_in(&page.body);
        for id in [
            "rust",
            "typescript",
            "python",
            "c",
            "reference",
            "what-the-example-does",
        ] {
            assert!(ids.contains(id), "missing id {id}");
        }
        let toc: Vec<&str> = page.toc.iter().map(|h| h.text.as_str()).collect();
        assert_eq!(toc, ["Modbus RTU", "What the example does", "Reference"]);
    }

    #[test]
    fn a_page_without_the_four_sections_is_left_alone() {
        assert!(language_tabs("<h2 id=\"rust\">Rust</h2><h2 id=\"python\">Python</h2>").is_none());
        let page = page(
            "docs/install.md",
            "docs/install.html",
            "# Install\n\n## Rust\n\nx\n",
        );
        assert!(matches!(page.kind, Kind::Article));
        assert!(!page.body.contains("lang-tabs"));
        assert_eq!(page.toc.len(), 2);
    }

    #[test]
    fn the_tab_block_ends_at_the_next_heading_or_the_page_end() {
        let tail = language_tabs(
            "<h2 id=\"rust\">R</h2><p>a</p><h2 id=\"typescript\">T</h2><h2 id=\"python\">P</h2><h2 id=\"c\">C</h2><p>last</p>",
        )
        .unwrap();
        assert!(tail.ends_with("<p>last</p></section>\n</div>\n"), "{tail}");
    }
}
