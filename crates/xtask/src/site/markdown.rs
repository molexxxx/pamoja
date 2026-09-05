//! Markdown to HTML for the site's pages.
//!
//! pulldown-cmark parses; this walks the event stream and shapes what a page needs beyond
//! the bare HTML. Every heading gets an id (an explicit `{#id}` from the Markdown, else a
//! slug of its text made unique the way mdBook made it, so the anchors the site has always
//! had keep resolving) and an anchor link, and is recorded for the page's table of contents
//! and the search index. A relative link to a `.md` page is rewritten to the `.html` it
//! becomes. A fenced code block is highlighted at build time and wrapped in a figure that
//! names its language and carries a copy button. A table is wrapped so a wide one scrolls on
//! its own rather than the page. Raw HTML passes through, which the generated regions rely on.

use std::collections::BTreeMap;

use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::highlight::{self, escape};

/// A page rendered from Markdown.
pub struct Rendered {
    /// The article body.
    pub html: String,
    /// The text of the first `h1`, or empty when the page has none.
    pub title: String,
    /// The plain text of the first paragraph, for the page's `<meta name="description">`.
    pub description: String,
    /// Every heading, in order, with the id it renders with.
    pub headings: Vec<Heading>,
    /// The page split at its `h2` headings, for the search index.
    pub sections: Vec<Section>,
}

/// One heading of a page.
pub struct Heading {
    /// 1 for `h1` through 6 for `h6`.
    pub level: u8,
    /// The id the heading element carries.
    pub id: String,
    /// The heading's plain text.
    pub text: String,
}

/// A run of a page between two `h2` headings, as plain text.
pub struct Section {
    /// The id of the heading that opens the section, or none for the text above the first.
    pub id: Option<String>,
    /// The heading's text, or empty for the text above the first.
    pub heading: String,
    /// The section's prose, whitespace collapsed, with code blocks left out.
    pub text: String,
}

/// Render a page.
///
/// # Arguments
///
/// * `markdown` - the page source.
///
/// # Returns
///
/// The HTML and everything the layout, the table of contents, and the search index take
/// from the page.
pub fn render(markdown: &str) -> Rendered {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_HEADING_ATTRIBUTES | Options::ENABLE_STRIKETHROUGH;
    let mut walk = Walk::default();
    for event in Parser::new_ext(markdown, options) {
        walk.step(event);
    }
    walk.finish()
}

#[derive(Default)]
struct Walk<'a> {
    out: Vec<Event<'a>>,
    used: BTreeMap<String, usize>,
    heading: Option<OpenHeading<'a>>,
    code: Option<(String, String)>,
    paragraphs: usize,
    in_paragraph: bool,
    title: String,
    description: String,
    headings: Vec<Heading>,
    sections: Vec<Section>,
}

struct OpenHeading<'a> {
    level: HeadingLevel,
    id: Option<String>,
    events: Vec<Event<'a>>,
    text: String,
}

impl<'a> Walk<'a> {
    fn step(&mut self, event: Event<'a>) {
        if let Some(open) = &mut self.heading {
            match event {
                Event::End(TagEnd::Heading(_)) => self.close_heading(),
                Event::Text(ref text) | Event::Code(ref text) => {
                    open.text.push_str(text);
                    open.events.push(event);
                }
                other => open.events.push(other),
            }
            return;
        }
        if let Some((_, body)) = &mut self.code {
            match event {
                Event::End(TagEnd::CodeBlock) => {
                    let (lang, body) = self.code.take().expect("an open code block");
                    self.out.push(Event::Html(figure(&lang, &body).into()));
                }
                Event::Text(text) => body.push_str(&text),
                _ => {}
            }
            return;
        }

        match event {
            Event::Start(Tag::Heading { level, id, .. }) => {
                self.heading = Some(OpenHeading {
                    level,
                    id: id.map(|id| id.to_string()),
                    events: Vec::new(),
                    text: String::new(),
                });
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .next()
                        .unwrap_or_default()
                        .to_ascii_lowercase(),
                    CodeBlockKind::Indented => String::new(),
                };
                self.code = Some((lang, String::new()));
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                self.out.push(Event::Start(Tag::Link {
                    link_type,
                    dest_url: rewrite_link(&dest_url).into(),
                    title,
                    id,
                }));
            }
            Event::Start(Tag::Table(alignment)) => {
                self.out
                    .push(Event::Html("<div class=\"table-scroll\">\n".into()));
                self.out.push(Event::Start(Tag::Table(alignment)));
            }
            Event::End(TagEnd::Table) => {
                self.out.push(Event::End(TagEnd::Table));
                self.out.push(Event::Html("</div>\n".into()));
            }
            Event::Start(Tag::Paragraph) => {
                self.in_paragraph = true;
                self.paragraphs += 1;
                self.out.push(event);
            }
            Event::End(TagEnd::Paragraph) => {
                self.in_paragraph = false;
                self.out.push(event);
            }
            Event::Text(ref text) | Event::Code(ref text) => {
                if self.in_paragraph && self.paragraphs == 1 {
                    self.description.push_str(text);
                }
                self.section().push_text(text);
                self.out.push(event);
            }
            Event::SoftBreak | Event::HardBreak => {
                if self.in_paragraph && self.paragraphs == 1 {
                    self.description.push(' ');
                }
                self.section().text.push(' ');
                self.out.push(event);
            }
            other => self.out.push(other),
        }
    }

    fn close_heading(&mut self) {
        let open = self.heading.take().expect("an open heading");
        let id = self.unique(open.id.unwrap_or_else(|| slug(&open.text)));
        let level = level_of(open.level);
        if level == 1 && self.title.is_empty() {
            self.title.clone_from(&open.text);
        }
        if level == 2 {
            self.sections.push(Section {
                id: Some(id.clone()),
                heading: open.text.clone(),
                text: String::new(),
            });
        }
        self.headings.push(Heading {
            level,
            id: id.clone(),
            text: open.text,
        });
        self.out.push(Event::Start(Tag::Heading {
            level: open.level,
            id: Some(id.clone().into()),
            classes: Vec::new(),
            attrs: Vec::new(),
        }));
        self.out.extend(open.events);
        self.out.push(Event::InlineHtml(
            format!(
                "<a class=\"anchor\" href=\"#{}\" aria-label=\"Link to this section\">#</a>",
                escape(&id)
            )
            .into(),
        ));
        self.out.push(Event::End(TagEnd::Heading(open.level)));
    }

    // The section the text being walked belongs to, opening the unnamed one above the first
    // heading when nothing has yet.
    fn section(&mut self) -> &mut Section {
        if self.sections.is_empty() {
            self.sections.push(Section {
                id: None,
                heading: String::new(),
                text: String::new(),
            });
        }
        self.sections.last_mut().expect("a section")
    }

    // The id, or the id with `-1`, `-2`, ... when the page has used it already.
    fn unique(&mut self, id: String) -> String {
        let count = self.used.entry(id.clone()).or_insert(0);
        let unique = match *count {
            0 => id.clone(),
            n => format!("{id}-{n}"),
        };
        *count += 1;
        unique
    }

    fn finish(self) -> Rendered {
        let mut html = String::new();
        html::push_html(&mut html, self.out.into_iter());
        Rendered {
            html: mark_sources(&html),
            title: self.title,
            description: collapse(&self.description),
            headings: self.headings,
            sections: self
                .sections
                .into_iter()
                .map(|section| Section {
                    text: collapse(&section.text),
                    ..section
                })
                .collect(),
        }
    }
}

impl Section {
    fn push_text(&mut self, text: &str) {
        if !self.text.is_empty() && !self.text.ends_with(' ') {
            self.text.push(' ');
        }
        self.text.push_str(text);
    }
}

fn level_of(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// The id a heading gets from its text: lowercase, spaces as hyphens, anything that is not
/// a letter, digit, hyphen, or underscore dropped, the way mdBook derived them.
pub fn slug(text: &str) -> String {
    text.chars()
        .filter_map(|ch| {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

/// A relative link to a Markdown page, rewritten to the page it renders as; every other
/// link (absolute, anchored, or to a file that is not a page) is returned as it was.
pub fn rewrite_link(dest: &str) -> String {
    if dest.contains("://")
        || dest.starts_with('#')
        || dest.starts_with('/')
        || dest.starts_with("mailto:")
    {
        return dest.to_owned();
    }
    let (path, fragment) = match dest.find('#') {
        Some(at) => dest.split_at(at),
        None => (dest, ""),
    };
    let Some(stem) = path.strip_suffix(".md") else {
        return dest.to_owned();
    };
    let page = match stem.strip_suffix("README") {
        Some(dir) => format!("{dir}index.html"),
        None => format!("{stem}.html"),
    };
    format!("{page}{fragment}")
}

// The language a fence names, as a reader knows it.
fn label(lang: &str) -> &str {
    match lang {
        "rust" => "Rust",
        "typescript" | "ts" => "TypeScript",
        "javascript" | "js" => "JavaScript",
        "python" | "py" => "Python",
        "csharp" | "cs" => "C#",
        "sh" | "shell" | "bash" | "console" => "Shell",
        "toml" => "TOML",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "html" => "HTML",
        "css" => "CSS",
        "" | "text" => "",
        other => other,
    }
}

// A highlighted code block as a figure: the language, a copy button, and the code.
fn figure(lang: &str, code: &str) -> String {
    let code = code.strip_suffix('\n').unwrap_or(code);
    let body = highlight::highlight(code, lang);
    let caption = match label(lang) {
        "" => String::new(),
        name => format!("<span class=\"code-lang\">{}</span>", escape(name)),
    };
    format!(
        "<figure class=\"code\" data-lang=\"{lang}\"><figcaption>{caption}<button class=\"copy\" type=\"button\" aria-label=\"Copy this code\">copy</button></figcaption><pre><code>{body}</code></pre></figure>\n",
        lang = escape(lang)
    )
}

// The line naming the test a spliced snippet comes from, so the stylesheet can set it as a
// caption on the figure that follows rather than a paragraph of its own.
fn mark_sources(html: &str) -> String {
    html.replace(
        "<p>From <a href=\"https://github.com/molexxxx/pamoja/blob/main/",
        "<p class=\"source\">From <a href=\"https://github.com/molexxxx/pamoja/blob/main/",
    )
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_get_unique_ids_anchors_and_a_record() {
        let page =
            render("# Title\n\nIntro.\n\n## Field I/O\n\n## Field I/O\n\n### Part {#bme280}\n");
        assert_eq!(page.title, "Title");
        assert_eq!(page.description, "Intro.");
        let ids: Vec<&str> = page.headings.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, ["title", "field-io", "field-io-1", "bme280"]);
        assert!(page
            .html
            .contains("<h2 id=\"field-io\">Field I/O<a class=\"anchor\" href=\"#field-io\""));
        assert!(page
            .html
            .contains("<h3 id=\"bme280\">Part<a class=\"anchor\""));
        assert_eq!(slug("C#"), "c");
        assert_eq!(slug("What the example does"), "what-the-example-does");
    }

    #[test]
    fn relative_markdown_links_become_pages() {
        assert_eq!(rewrite_link("install.md"), "install.html");
        assert_eq!(rewrite_link("../README.md"), "../index.html");
        assert_eq!(rewrite_link("README.md#start"), "index.html#start");
        assert_eq!(
            rewrite_link("guides/modbus.md#rust"),
            "guides/modbus.html#rust"
        );
        for kept in [
            "https://docs.rs/pamoja",
            "#arguments",
            "/docs/index.html",
            "assets/pamoja-logo.svg",
            "reference/rust/pamoja/index.html",
        ] {
            assert_eq!(rewrite_link(kept), kept);
        }
        let page = render("See [install](install.md) and [docs.rs](https://docs.rs/x).");
        assert!(page.html.contains("href=\"install.html\""));
        assert!(page.html.contains("href=\"https://docs.rs/x\""));
    }

    #[test]
    fn code_blocks_become_highlighted_figures() {
        let page = render("```rust\nlet x = 1;\n```\n\n```\nplain <text>\n```\n");
        assert!(page.html.contains(
            "<figure class=\"code\" data-lang=\"rust\"><figcaption><span class=\"code-lang\">Rust</span><button class=\"copy\""
        ));
        assert!(page.html.contains("<span class=\"hl-kw\">let</span>"));
        assert!(page
            .html
            .contains("<figure class=\"code\" data-lang=\"\"><figcaption><button"));
        assert!(page.html.contains("plain &lt;text&gt;</code>"));
        assert!(page.sections.is_empty(), "code is not indexed");
    }

    #[test]
    fn tables_scroll_and_raw_html_passes_through() {
        let page = render("| a | b |\n| --- | --- |\n| 1 | 2 |\n\n<div class=\"pkgs\">x</div>\n");
        assert!(page
            .html
            .starts_with("<div class=\"table-scroll\">\n<table>"));
        assert!(page.html.contains("</table>\n</div>"));
        assert!(page.html.contains("<div class=\"pkgs\">x</div>"));
    }

    #[test]
    fn sections_split_at_h2_and_carry_their_prose() {
        let page =
            render("# T\n\nAbove.\n\n## One\n\nFirst `code` here.\nMore.\n\n## Two\n\nSecond.\n");
        assert_eq!(page.sections.len(), 3);
        assert_eq!(page.sections[0].id, None);
        assert_eq!(
            page.sections[0].text, "Above.",
            "the title is indexed on its own"
        );
        assert_eq!(page.sections[1].heading, "One");
        assert_eq!(page.sections[1].text, "First code here. More.");
        assert_eq!(page.sections[2].id.as_deref(), Some("two"));
    }

    #[test]
    fn a_snippet_source_line_is_marked() {
        let page = render(
            "From [`x.rs`](https://github.com/molexxxx/pamoja/blob/main/x.rs):\n\n```rust\nlet a = 1;\n```\n",
        );
        assert!(page.html.starts_with("<p class=\"source\">From <a href="));
    }
}
