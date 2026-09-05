//! The site's navigation, derived from the capability map.
//!
//! The pages come first (introduction, install, hardware), then a group per chapter holding
//! its guides in the order the map gives them, then the four references and the pages about
//! the project. The same order gives every page its previous and next neighbour.

use crate::catalog::Catalog;

use super::highlight::escape;

/// The navigation: groups of links in reading order.
pub struct Nav {
    /// The groups, in the order the sidebar shows them.
    pub groups: Vec<Group>,
}

/// A run of links under one label, or under none for the pages that open the site.
pub struct Group {
    /// The label above the links, or none.
    pub title: Option<String>,
    /// The links, in order.
    pub items: Vec<Item>,
}

/// One link in the navigation.
#[derive(Debug, PartialEq, Eq)]
pub struct Item {
    /// The text of the link.
    pub title: String,
    /// The page, site-relative (`docs/guides/modbus.html`).
    pub url: String,
}

impl Nav {
    /// Build the navigation from the capability map.
    ///
    /// # Arguments
    ///
    /// * `catalog` - the map, which supplies the chapters and their guides.
    ///
    /// # Returns
    ///
    /// The navigation every page renders.
    pub fn from(catalog: &Catalog) -> Nav {
        let mut groups = vec![Group {
            title: None,
            items: vec![
                item("Introduction", "docs/index.html"),
                item("Install", "docs/install.html"),
                item("Hardware", "docs/hardware.html"),
            ],
        }];
        for chapter in &catalog.chapters {
            let items: Vec<Item> = catalog
                .in_chapter(&chapter.key)
                .filter_map(|capability| {
                    capability.guide.as_ref().map(|guide| {
                        let page = guide.strip_suffix(".md").unwrap_or(guide);
                        item(&capability.title, &format!("docs/{page}.html"))
                    })
                })
                .collect();
            if !items.is_empty() {
                groups.push(Group {
                    title: Some(chapter.title.clone()),
                    items,
                });
            }
        }
        groups.push(Group {
            title: Some("Reference".to_owned()),
            items: vec![
                item("Rust", "docs/reference/rust.html"),
                item("TypeScript", "docs/reference/node.html"),
                item("Python", "docs/reference/python.html"),
                item("C#", "docs/reference/dotnet.html"),
            ],
        });
        groups.push(Group {
            title: Some("About".to_owned()),
            items: vec![
                item("Why it exists", "docs/about/why.html"),
                item("Architecture", "docs/about/architecture.html"),
                item("Standards and conformance", "docs/about/standards.html"),
                item("Building", "docs/about/building.html"),
                item("Releasing", "docs/about/releasing.html"),
            ],
        });
        Nav { groups }
    }

    /// Every link in reading order.
    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.groups.iter().flat_map(|group| group.items.iter())
    }

    /// The link for a page, if the page is in the navigation.
    pub fn item(&self, url: &str) -> Option<&Item> {
        self.items().find(|item| item.url == url)
    }

    /// The group a page sits in, for the label the header shows and the search index keeps.
    pub fn group_of(&self, url: &str) -> Option<&Group> {
        self.groups
            .iter()
            .find(|group| group.items.iter().any(|item| item.url == url))
    }

    /// The pages before and after `url` in reading order.
    pub fn neighbours(&self, url: &str) -> (Option<&Item>, Option<&Item>) {
        let items: Vec<&Item> = self.items().collect();
        let Some(at) = items.iter().position(|item| item.url == url) else {
            return (None, None);
        };
        let previous = at.checked_sub(1).map(|i| items[i]);
        let next = items.get(at + 1).copied();
        (previous, next)
    }

    /// The sidebar as HTML, with the current page marked.
    ///
    /// # Arguments
    ///
    /// * `current` - the page being rendered, site-relative.
    /// * `root` - the prefix that reaches the site root from that page (`../`, `../../`).
    ///
    /// # Returns
    ///
    /// A `<nav>` element.
    pub fn sidebar(&self, current: &str, root: &str) -> String {
        let mut out = String::from("<nav class=\"side-nav\" aria-label=\"Documentation\">\n");
        for group in &self.groups {
            let open = group.items.iter().any(|item| item.url == current);
            if let Some(title) = &group.title {
                out.push_str(&format!(
                    "<details class=\"side-group\"{}><summary>{}</summary>\n<ul>\n",
                    if open { " open" } else { "" },
                    escape(title)
                ));
            } else {
                out.push_str("<ul class=\"side-top\">\n");
            }
            for item in &group.items {
                if item.url == current {
                    out.push_str(&format!(
                        "<li><a href=\"{root}{}\" class=\"current\" aria-current=\"page\">{}</a></li>\n",
                        item.url,
                        escape(&item.title)
                    ));
                } else {
                    out.push_str(&format!(
                        "<li><a href=\"{root}{}\">{}</a></li>\n",
                        item.url,
                        escape(&item.title)
                    ));
                }
            }
            out.push_str("</ul>\n");
            if group.title.is_some() {
                out.push_str("</details>\n");
            }
        }
        out.push_str("</nav>\n");
        out
    }
}

fn item(title: &str, url: &str) -> Item {
    Item {
        title: title.to_owned(),
        url: url.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[[chapter]]
key = "field-io"
title = "Field I/O"
intent = "The wires."

[[chapter]]
key = "empty"
title = "Nothing yet"
intent = "No guides."

[[capability]]
key = "modbus"
chapter = "field-io"
title = "Modbus RTU"
summary = "Modbus"
crates = ["pamoja-modbus"]
node = "modbus"
python = "modbus"
dotnet = ["Modbus"]
guide = "guides/modbus.md"

[[capability]]
key = "can"
chapter = "field-io"
title = "CAN"
summary = "CAN"
crates = ["pamoja-can"]
node = "can"
python = "can"
dotnet = ["Can"]
guide = "guides/can.md"
"#;

    #[test]
    fn chapters_with_guides_become_groups_in_reading_order() {
        let nav = Nav::from(&Catalog::parse(SAMPLE).unwrap());
        let titles: Vec<Option<&str>> = nav
            .groups
            .iter()
            .map(|group| group.title.as_deref())
            .collect();
        assert_eq!(
            titles,
            [None, Some("Field I/O"), Some("Reference"), Some("About")]
        );
        let urls: Vec<&str> = nav.items().map(|item| item.url.as_str()).collect();
        assert_eq!(
            &urls[..5],
            [
                "docs/index.html",
                "docs/install.html",
                "docs/hardware.html",
                "docs/guides/modbus.html",
                "docs/guides/can.html",
            ]
        );
    }

    #[test]
    fn neighbours_follow_reading_order_across_groups() {
        let nav = Nav::from(&Catalog::parse(SAMPLE).unwrap());
        let (previous, next) = nav.neighbours("docs/guides/can.html");
        assert_eq!(
            previous.map(|i| i.url.as_str()),
            Some("docs/guides/modbus.html")
        );
        assert_eq!(
            next.map(|i| i.url.as_str()),
            Some("docs/reference/rust.html")
        );
        assert!(nav.neighbours("docs/index.html").0.is_none());
        assert!(nav.neighbours("docs/about/releasing.html").1.is_none());
        assert_eq!(nav.neighbours("docs/nowhere.html"), (None, None));
        assert_eq!(
            nav.group_of("docs/guides/can.html")
                .and_then(|g| g.title.as_deref()),
            Some("Field I/O")
        );
    }

    #[test]
    fn the_sidebar_marks_the_current_page_and_opens_its_group() {
        let nav = Nav::from(&Catalog::parse(SAMPLE).unwrap());
        let html = nav.sidebar("docs/guides/can.html", "../../");
        assert!(html.contains("<details class=\"side-group\" open><summary>Field I/O</summary>"));
        assert!(html.contains("<details class=\"side-group\"><summary>Reference</summary>"));
        assert!(html.contains(
            "<a href=\"../../docs/guides/can.html\" class=\"current\" aria-current=\"page\">CAN</a>"
        ));
        assert!(html.contains("<a href=\"../../docs/index.html\">Introduction</a>"));
    }
}
