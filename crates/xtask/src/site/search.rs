//! The search index: one entry per page and per section of every page.
//!
//! The header's search box fetches `search.json` on first focus and ranks entries in the
//! browser, so the index stays a flat list a few hundred entries long rather than an inverted
//! index, and a page's sections are found by their own headings.

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
/// A JSON array of `{u, p, h, s, b}`: the URL (with a fragment for a section), the page
/// title, the section heading (empty for the page itself), the group, and the excerpt.
pub fn index(pages: &[Page], nav: &Nav) -> String {
    let mut entries: Vec<Value> = Vec::new();
    for page in pages {
        let group = nav
            .group_of(&page.url)
            .and_then(|group| group.title.clone())
            .unwrap_or_else(|| "Documentation".to_owned());
        entries.push(json!({
            "u": page.url,
            "p": page.title,
            "h": "",
            "s": group,
            "b": excerpt(&page.description),
        }));
        for section in &page.sections {
            let Some(id) = &section.id else {
                continue;
            };
            entries.push(json!({
                "u": format!("{}#{id}", page.url),
                "p": page.title,
                "h": section.heading,
                "s": group,
                "b": excerpt(&section.text),
            }));
        }
    }
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
    fn every_page_and_every_named_section_is_an_entry() {
        let catalog = Catalog::parse("").unwrap();
        let nav = Nav::from(&catalog);
        let page = super::super::pages::page(
            "docs/install.md",
            "docs/install.html",
            "# Install\n\nOne line.\n\n## Rust\n\nCargo.\n\n## Node\n\nnpm.\n",
        );
        let entries: Vec<Value> = serde_json::from_str(&index(&[page], &nav)).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0]["u"], "docs/install.html");
        assert_eq!(entries[0]["b"], "One line.");
        assert_eq!(entries[1]["u"], "docs/install.html#rust");
        assert_eq!(entries[1]["h"], "Rust");
        assert_eq!(entries[1]["s"], "Documentation");
        assert_eq!(entries[2]["b"], "npm.");
    }

    #[test]
    fn long_prose_is_cut_at_a_word() {
        let long = "word ".repeat(100);
        let cut = excerpt(&long);
        assert!(cut.chars().count() <= EXCERPT + 1);
        assert!(cut.ends_with("word\u{2026}"));
    }
}
