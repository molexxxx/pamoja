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
    // Any page that carries the four language sections in order gets the tab strip; the fold
    // returns None for every page that does not, so a page opts in by its own headings.
    let (body, toc) = match language_tabs(&rendered.html) {
        Some(html) => {
            let language_ids: BTreeSet<&str> = LANGUAGES.iter().map(|(_, id)| *id).collect();
            let is_language = |id: &str| {
                let base = match id.rsplit_once('-') {
                    Some((base, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => {
                        base
                    }
                    _ => id,
                };
                language_ids.contains(base)
            };
            let toc = rendered
                .headings
                .into_iter()
                .filter(|heading| {
                    !((heading.level == 2 || heading.level == 3) && is_language(&heading.id))
                })
                .collect();
            (html, toc)
        }
        None => (rendered.html, rendered.headings),
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

/// Fold every run of four language sections in a guide into a tab block. A guide shows its
/// example in the four languages, and a later section, such as the same program on a board,
/// may show another four; each run becomes its own block, and all of them follow the one
/// language the reader picked. The headings keep their ids on the panels, so the anchors
/// the guides have always had still resolve, and a later run's `rust-1` does too.
///
/// # Arguments
///
/// * `html` - the rendered guide.
///
/// # Returns
///
/// The guide with a tab block in place of each run, or `None` when no run of the four
/// headings is present in order.
pub fn language_tabs(html: &str) -> Option<String> {
    let mut folded: Option<String> = None;
    while let Some(next) = ["h2", "h3"]
        .iter()
        .find_map(|level| fold(folded.as_deref().unwrap_or(html), level))
    {
        folded = Some(next);
    }
    folded
}

/// The line a page writes after its last language section when the text that follows is for
/// every language, not the last one alone.
pub const LANGUAGES_END: &str = "<!-- languages end -->";

// The fold at one heading level: the next four language headings in order, each becoming
// a panel. A run of h2 sections ends at the next h2, so a panel may carry subheadings of
// its own; a run of h3 sections ends at the next h2 or h3. Either ends early at
// `LANGUAGES_END`.
fn fold(html: &str, level: &str) -> Option<String> {
    let mut starts = Vec::with_capacity(LANGUAGES.len());
    let mut anchors = Vec::with_capacity(LANGUAGES.len());
    let mut from = 0;
    for (_, id) in LANGUAGES {
        let (at, anchor) = language_heading(html, from, level, id)?;
        from = at + format!("<{level} id=\"{anchor}\">").len();
        starts.push(at);
        anchors.push(anchor);
    }
    let stops: &[&str] = if level == "h2" {
        &["<h2 id=\"", "<h1", LANGUAGES_END]
    } else {
        &["<h2 id=\"", "<h3 id=\"", LANGUAGES_END]
    };
    let end = stops
        .iter()
        .filter_map(|next| html[from..].find(next))
        .min()
        .map_or(html.len(), |at| at + from);

    let mut out = String::with_capacity(html.len() + 1024);
    out.push_str(&html[..starts[0]]);
    out.push_str("<div class=\"langs\">\n<div class=\"lang-tabs\" role=\"tablist\" aria-label=\"Language\">\n");
    for ((label, id), anchor) in LANGUAGES.iter().zip(&anchors) {
        out.push_str(&format!(
            "<button class=\"lang-tab\" role=\"tab\" type=\"button\" id=\"tab-{anchor}\" aria-controls=\"{anchor}\" aria-selected=\"false\" data-lang=\"{id}\">{label}</button>\n"
        ));
    }
    out.push_str("</div>\n");
    for (index, ((_, id), anchor)) in LANGUAGES.iter().zip(&anchors).enumerate() {
        let start = starts[index];
        let stop = starts.get(index + 1).copied().unwrap_or(end);
        let heading = format!("<{level} id=\"{anchor}\">");
        let section =
            html[start..stop].replacen(&heading, &format!("<{level} class=\"lang-heading\">"), 1);
        out.push_str(&format!(
            "<section class=\"lang-panel\" id=\"{anchor}\" role=\"tabpanel\" aria-labelledby=\"tab-{anchor}\" data-lang=\"{id}\" tabindex=\"0\">\n{section}</section>\n"
        ));
    }
    out.push_str("</div>\n");
    out.push_str(&html[end..]);
    Some(out)
}

// The next heading at `level` for a language from `from` on: its id, or that id with the
// `-1`, `-2` a page gives a heading it has used already. The position and the id found.
fn language_heading(html: &str, from: usize, level: &str, id: &str) -> Option<(usize, String)> {
    let prefix = format!("<{level} id=\"{id}");
    let mut search = from;
    while let Some(found) = html[search..].find(&prefix) {
        let at = search + found;
        let rest = &html[at + prefix.len()..];
        let suffix = &rest[..rest.find("\">")?];
        let numbered = suffix
            .strip_prefix('-')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        if suffix.is_empty() || numbered {
            return Some((at, format!("{id}{suffix}")));
        }
        search = at + prefix.len();
    }
    None
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

    #[test]
    fn the_marker_ends_a_tab_block_so_what_follows_is_for_every_language() {
        let guide = "# Pi\n\n## A program\n\n### Rust\n\nr\n\n### TypeScript\n\nt\n\n### Python\n\np\n\n### C#\n\nc\n\n<!-- languages end -->\n\nFor all four.\n\n## Next\n";
        let page = page("docs/boards/pi.md", "docs/boards/pi.html", guide);
        let block = page.body.find("</section>\n</div>\n").unwrap();
        let shared = page.body.find("<p>For all four.</p>").unwrap();
        assert!(
            block < shared,
            "the text after the marker is outside the tabs: {}",
            page.body
        );
        assert!(page.body.contains(LANGUAGES_END));
    }

    #[test]
    fn a_panel_keeps_its_subheadings_and_a_second_run_folds_on_its_own_anchors() {
        let guide = format!(
            "{}\n## On a board\n\nWired up.\n\n### Rust\n\nrust pin\n\n### TypeScript\n\nts pin\n\n### Python\n\npy pin\n\n### C#\n\ncs pin\n",
            GUIDE.replace("cs body\n", "cs body\n\n### Errors\n\nWhat it refuses.\n")
        );
        let page = page("docs/guides/modbus.md", "docs/guides/modbus.html", &guide);
        assert_eq!(
            page.body.matches("<div class=\"lang-tabs\"").count(),
            2,
            "{}",
            page.body
        );
        let first = page.body.find("id=\"c\" role=\"tabpanel\"").unwrap();
        let errors = page.body.find("Errors").unwrap();
        let reference = page.body.find("<h2 id=\"reference\"").unwrap();
        assert!(
            first < errors && errors < reference,
            "a subheading stays in its panel"
        );
        assert!(page.body.contains("<button class=\"lang-tab\" role=\"tab\" type=\"button\" id=\"tab-python-1\" aria-controls=\"python-1\" aria-selected=\"false\" data-lang=\"python\">Python</button>"), "{}", page.body);
        assert!(page.body.contains("<section class=\"lang-panel\" id=\"c-1\" role=\"tabpanel\" aria-labelledby=\"tab-c-1\" data-lang=\"c\""));
        assert!(page.body.contains("cs pin"));
        let toc: Vec<&str> = page.toc.iter().map(|h| h.text.as_str()).collect();
        assert_eq!(
            toc,
            [
                "Modbus RTU",
                "What the example does",
                "Errors",
                "Reference",
                "On a board"
            ]
        );
        assert_eq!(language_heading("<h2 id=\"can\">", 0, "h2", "c"), None);
        assert_eq!(
            language_heading("<h2 id=\"c-12\">", 0, "h2", "c"),
            Some((0, "c-12".to_owned()))
        );
    }
}
