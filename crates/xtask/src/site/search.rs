//! The search index: every page, with each of its sections.
//!
//! The header's search box fetches `search.json` on first focus and ranks entries in the
//! browser, so the index stays a flat list a few hundred pages long rather than an inverted
//! index, and a page's sections are found by their own headings. A page carries its URL,
//! title, and group once, with its sections beneath it, and the search box expands that into
//! one entry per page and per section, so the index does not repeat them for every section.

use serde_json::{json, Value};

use super::nav::Nav;
use super::Page;

/// How much of a section's prose an entry carries, enough for a result to show a line.
const EXCERPT: usize = 240;

/// The index as JSON.
///
/// # Arguments
///
/// * `pages` - every page of the site.
/// * `nav` - the navigation, which names the group each page belongs to.
///
/// # Returns
///
/// A JSON array with one `{u, p, s, b, x}` per page: the URL, the page title, the group, the
/// excerpt, and the sections, each an `[id, heading, excerpt]` triple.
pub fn index(pages: &[Page], nav: &Nav) -> String {
    let entries: Vec<Value> = pages
        .iter()
        .map(|page| {
            let group = nav
                .group_of(&page.url)
                .and_then(|group| group.title.clone())
                .unwrap_or_else(|| "Documentation".to_owned());
            let sections: Vec<Value> = page
                .sections
                .iter()
                .filter_map(|section| {
                    let id = section.id.as_ref()?;
                    Some(json!([id, section.heading, excerpt(&section.text)]))
                })
                .collect();
            json!({
                "u": page.url,
                "p": page.title,
                "s": group,
                "b": excerpt(&page.description),
                "x": sections,
            })
        })
        .collect();
    serde_json::to_string(&entries).expect("a JSON array of strings")
}

fn excerpt(text: &str) -> String {
    if text.chars().count() <= EXCERPT {
        return text.to_owned();
    }
    let cut: String = text.chars().take(EXCERPT).collect();
    let end = cut.rfind(' ').unwrap_or(cut.len());
    format!("{}\u{2026}", &cut[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn every_page_carries_every_named_section() {
        let catalog = Catalog::parse("").unwrap();
        let nav = Nav::from(&catalog);
        let page = super::super::pages::page(
            "docs/install.md",
            "docs/install.html",
            "# Install\n\nOne line.\n\n## Rust\n\nCargo.\n\n## Node\n\nnpm.\n",
        );
        let entries: Vec<Value> = serde_json::from_str(&index(&[page], &nav)).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["u"], "docs/install.html");
        assert_eq!(entries[0]["b"], "One line.");
        assert_eq!(entries[0]["s"], "Documentation");
        assert_eq!(
            entries[0]["x"],
            json!([["rust", "Rust", "Cargo."], ["node", "Node", "npm."]])
        );
    }

    #[test]
    fn long_prose_is_cut_at_a_word() {
        let long = "word ".repeat(100);
        let cut = excerpt(&long);
        assert!(cut.chars().count() <= EXCERPT + 1);
        assert!(cut.ends_with("word\u{2026}"));
    }
}
