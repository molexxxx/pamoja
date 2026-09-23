//! The standards register in `docs/standards.toml`: every published specification pamoja
//! implements, the document that defines it, and the test that pins the implementation to
//! that document's own vectors. [`Standards::table`] renders the page and
//! [`Standards::sources`] hands every `url` to `cargo xtask links`, so a specification that
//! moves fails the build rather than sitting on the page looking authoritative.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use toml_edit::DocumentMut;

use crate::catalog::{escape, Catalog};
use crate::hardware::{optional, string, tables};

/// The repository, for the link to the test a row cites.
const REPO: &str = "https://github.com/molexxxx/pamoja";

/// What a test pins an implementation to, and how the page says it.
const ANCHORS: [(&str, &str); 4] = [
    ("vector", "Published vector"),
    ("rule", "Specification rule"),
    ("interop", "Live implementation"),
    ("internal", "Round trip only"),
];

/// A heading on the page, tied to a capability chapter.
pub struct Group {
    /// The chapter key from `docs/capabilities.toml`.
    pub chapter: String,
    /// The heading the group renders under.
    pub title: String,
    /// One line saying what the group covers.
    pub intent: String,
}

/// One standard, and what holds the implementation to it.
pub struct Entry {
    /// The anchor id, unique across the file.
    pub key: String,
    /// The group it renders under.
    pub chapter: String,
    /// The document as its publisher writes it, revision included.
    pub designation: String,
    /// Who publishes it.
    pub body: String,
    /// What it covers here, in one line.
    pub subject: String,
    /// Where the authoritative document lives.
    pub url: String,
    /// The file that holds the implementation to the document, from the repository root.
    pub evidence: String,
    /// Text from the line the evidence cites, which places the link on that line wherever
    /// later edits move it.
    pub at: String,
    /// The one-based line `at` was found on, or why it could not be placed. [`Standards::load`]
    /// fills it; an entry parsed from text alone has none.
    pub line: Option<Result<usize, String>>,
    /// One of [`ANCHORS`].
    pub anchor: String,
    /// Why `cargo xtask links` cannot fetch the document. Empty for the rest.
    pub manual_check: String,
    /// What the test asserts, where that needs saying. Empty for the rest.
    pub note: String,
}

/// The whole register: groups in page order, entries in file order.
pub struct Standards {
    /// The headings, in the order they render.
    pub groups: Vec<Group>,
    /// Every entry, in the order it appears in the file.
    pub entries: Vec<Entry>,
}

impl Standards {
    /// Read `docs/standards.toml` under `root`.
    ///
    /// # Arguments
    ///
    /// * `root` - the repository root.
    ///
    /// # Returns
    ///
    /// The parsed register.
    ///
    /// # Errors
    ///
    /// When the file is missing or a required field is absent or the wrong type. A cited line
    /// that cannot be placed is not an error here; [`Standards::check`] reports it.
    pub fn load(root: &Path) -> Result<Standards, String> {
        let path = root.join("docs/standards.toml");
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        let mut standards = Standards::parse(&text)?;
        for entry in &mut standards.entries {
            entry.line = Some(locate(root, &entry.evidence, &entry.at));
        }
        Ok(standards)
    }

    /// Parse the register from its TOML text.
    ///
    /// # Arguments
    ///
    /// * `text` - the contents of `docs/standards.toml`.
    ///
    /// # Returns
    ///
    /// The parsed register.
    ///
    /// # Errors
    ///
    /// When a required field is missing or has the wrong type.
    pub fn parse(text: &str) -> Result<Standards, String> {
        let doc: DocumentMut = text
            .parse()
            .map_err(|err| format!("standards.toml is not valid TOML: {err}"))?;

        let mut groups = Vec::new();
        for table in tables(&doc, "group")? {
            groups.push(Group {
                chapter: string(table, "chapter", "group")?,
                title: string(table, "title", "group")?,
                intent: string(table, "intent", "group")?,
            });
        }

        let mut entries = Vec::new();
        for table in tables(&doc, "entry")? {
            let key = string(table, "key", "entry")?;
            let context = format!("entry {key}");
            entries.push(Entry {
                chapter: string(table, "chapter", &context)?,
                designation: string(table, "designation", &context)?,
                body: string(table, "body", &context)?,
                subject: string(table, "subject", &context)?,
                url: string(table, "url", &context)?,
                evidence: string(table, "evidence", &context)?,
                at: string(table, "at", &context)?,
                line: None,
                anchor: string(table, "anchor", &context)?,
                manual_check: optional(table, "manual_check"),
                note: optional(table, "note"),
                key,
            });
        }

        Ok(Standards { groups, entries })
    }

    /// Every document URL on the page, in file order, for the link checker.
    ///
    /// # Returns
    ///
    /// The key, the designation, the URL, and why a person checks it instead, per entry.
    pub fn sources(&self) -> Vec<(&str, &str, &str, &str)> {
        self.entries
            .iter()
            .map(|e| {
                (
                    e.key.as_str(),
                    e.designation.as_str(),
                    e.url.as_str(),
                    e.manual_check.as_str(),
                )
            })
            .collect()
    }

    /// Render the page body: one section per capability chapter, an index of its standards,
    /// and a ruled row per standard carrying the document and the test that anchors it.
    ///
    /// # Arguments
    ///
    /// * `catalog` - the capability map, so a chapter can name its crates.
    ///
    /// # Returns
    ///
    /// The Markdown that replaces the `<!-- table: standards -->` region.
    pub fn table(&self, catalog: &Catalog) -> String {
        let (total, vectors) = self.counts();
        let mut out = vec![format!(
            "<p class=\"source\">{total} standards registered, {vectors} of them pinned to the document's own published vectors. Counted from <code>docs/standards.toml</code> when this page was rendered.</p>"
        )];
        for group in &self.groups {
            let entries: Vec<&Entry> = self
                .entries
                .iter()
                .filter(|e| e.chapter == group.chapter)
                .collect();
            if entries.is_empty() {
                continue;
            }
            let index: String = entries
                .iter()
                .map(|e| format!("<a href=\"#{}\">{}</a>", e.key, escape(&e.designation)))
                .collect();
            let mut section = format!(
                "## {}\n\n{}\n\n<nav class=\"hw-index\" aria-label=\"{} index\">{index}</nav>\n<div class=\"hw-cards\">\n",
                group.title,
                group.intent,
                escape(&group.title)
            );
            for entry in &entries {
                section.push_str(&row(entry));
            }
            section.push_str("</div>");
            out.push(section);
        }
        let _ = catalog;
        out.join("\n\n")
    }

    /// Check the register against its own rules.
    ///
    /// # Arguments
    ///
    /// * `catalog` - the capability map, whose chapters a group must name.
    ///
    /// # Returns
    ///
    /// Nothing when every entry is well formed.
    ///
    /// # Errors
    ///
    /// When a key repeats, a group names a chapter the catalog does not have, an entry names
    /// a group that is not on the page, an `anchor` is not one of the known kinds, or an
    /// entry's `at` is on no line of its evidence file, or on more than one.
    pub fn check(&self, catalog: &Catalog) -> Result<(), String> {
        let chapters: BTreeSet<&str> = catalog.chapters.iter().map(|c| c.key.as_str()).collect();
        let mut seen = BTreeSet::new();
        for group in &self.groups {
            if !chapters.contains(group.chapter.as_str()) {
                return Err(format!(
                    "standards.toml: group {} names chapter {}, which docs/capabilities.toml does not have",
                    group.title, group.chapter
                ));
            }
        }
        let groups: BTreeSet<&str> = self.groups.iter().map(|g| g.chapter.as_str()).collect();
        let kinds: BTreeSet<&str> = ANCHORS.iter().map(|(key, _)| *key).collect();
        for entry in &self.entries {
            if !seen.insert(entry.key.as_str()) {
                return Err(format!("standards.toml: {} appears twice", entry.key));
            }
            if !groups.contains(entry.chapter.as_str()) {
                return Err(format!(
                    "standards.toml: {} names chapter {}, which has no group",
                    entry.key, entry.chapter
                ));
            }
            if !kinds.contains(entry.anchor.as_str()) {
                return Err(format!(
                    "standards.toml: {} has anchor {}, which is not one of {:?}",
                    entry.key,
                    entry.anchor,
                    kinds.iter().collect::<Vec<_>>()
                ));
            }
            if !entry.url.starts_with("https://") {
                return Err(format!("standards.toml: {} has a non-https url", entry.key));
            }
            if entry.evidence.contains('#') {
                return Err(format!(
                    "standards.toml: {} cites a line by number, which edits move; name the file in evidence and the line's text in at",
                    entry.key
                ));
            }
            if let Some(Err(problem)) = &entry.line {
                return Err(format!("standards.toml: {}: {problem}", entry.key));
            }
        }
        Ok(())
    }

    /// How many entries the register holds, and how many pin a published vector.
    ///
    /// # Returns
    ///
    /// The total, and the count whose `anchor` is `vector`.
    pub fn counts(&self) -> (usize, usize) {
        let vectors = self.entries.iter().filter(|e| e.anchor == "vector").count();
        (self.entries.len(), vectors)
    }
}

// One standard as a ruled row: the designation and who publishes it, what it covers, and the
// two ways out, the document itself and the test that holds the code to it.
fn row(entry: &Entry) -> String {
    let anchor = ANCHORS
        .iter()
        .find(|(key, _)| *key == entry.anchor)
        .map(|(_, label)| *label)
        .unwrap_or("Anchored");
    let note = match entry.note.is_empty() {
        true => String::new(),
        false => format!("<p class=\"hw-summary\">{}</p>", escape(&entry.note)),
    };
    let name = entry
        .evidence
        .rsplit('/')
        .next()
        .unwrap_or(entry.evidence.as_str());
    let (fragment, at) = match &entry.line {
        Some(Ok(line)) => (format!("#L{line}"), format!(" line {line}")),
        _ => (String::new(), String::new()),
    };
    let (view, label) = match entry.evidence.ends_with(".md") {
        true => ("?plain=1", "The page"),
        false => ("", "The test"),
    };
    let caution = match entry.manual_check.is_empty() {
        true => String::new(),
        false => format!(
            "<small class=\"hw-note\">{}</small>",
            escape(&entry.manual_check)
        ),
    };
    format!(
        "<article class=\"hw-card\" aria-labelledby=\"{key}\">\n<header class=\"hw-head\">\n\n### {designation} {{#{key}}}\n\n<p class=\"hw-by\">{body}</p>\n<p class=\"hw-summary\">{subject}</p>\n</header>\n{note}<div class=\"hw-foot\">\n<section class=\"hw-buy\"><h4>Read it</h4><ul class=\"hw-rows\"><li><a class=\"hw-row\" href=\"{url}\"><span class=\"hw-main\"><b>The document</b><small>{designation}</small>{caution}</span><span class=\"hw-go\" aria-hidden=\"true\">&#8599;</span></a></li></ul></section>\n<section class=\"hw-learn\"><h4>{anchor}</h4><ul class=\"hw-rows\"><li><a class=\"hw-row\" href=\"{REPO}/blob/main/{evidence}{view}{fragment}\"><span class=\"hw-main\"><b>{label}</b><small><code>{name}</code>{at}</small></span><span class=\"hw-go\" aria-hidden=\"true\">&#8599;</span></a></li></ul></section>\n</div>\n</article>\n",
        key = entry.key,
        designation = escape(&entry.designation),
        body = escape(&entry.body),
        subject = escape(&entry.subject),
        url = escape(&entry.url),
        evidence = escape(&entry.evidence),
    )
}

// The one line of `file`, from the repository root, that contains `at`: its one-based number,
// or why there is not exactly one.
fn locate(root: &Path, file: &str, at: &str) -> Result<usize, String> {
    let text =
        fs::read_to_string(root.join(file)).map_err(|err| format!("reading {file}: {err}"))?;
    let lines: Vec<usize> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(at))
        .map(|(index, _)| index + 1)
        .collect();
    match lines.as_slice() {
        [line] => Ok(*line),
        [] => Err(format!("no line of {file} contains {at:?}")),
        many => Err(format!(
            "{} lines of {file} contain {at:?}, lines {}; quote more of the one meant",
            many.len(),
            many.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
[[group]]
chapter = "trust"
title = "Cryptography"
intent = "The primitives."

[[entry]]
key = "rfc-4493"
chapter = "trust"
designation = "RFC 4493"
body = "IETF"
subject = "AES-CMAC."
url = "https://www.rfc-editor.org/info/rfc4493"
evidence = "crates/pamoja-lorawan/src/crypto.rs"
at = "fn cmac_of_the_empty_message_matches_rfc_4493"
anchor = "vector"
"#;

    const CRYPTO: &str = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn cmac_of_the_empty_message_matches_rfc_4493() {\n    }\n}\n";

    fn catalog() -> Catalog {
        Catalog::parse("").expect("an empty catalog parses")
    }

    fn with_trust() -> Catalog {
        Catalog::parse("[[chapter]]\nkey = \"trust\"\ntitle = \"Trust\"\nintent = \"Keys.\"\n")
            .expect("a catalog with the chapter parses")
    }

    // A repository holding the register and the one file it cites, so `load` places the
    // cited line the way it does on the real tree.
    fn repository(name: &str, register: &str, crypto: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("pamoja-standards-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join("crates/pamoja-lorawan/src")).unwrap();
        fs::write(root.join("docs/standards.toml"), register).unwrap();
        fs::write(root.join("crates/pamoja-lorawan/src/crypto.rs"), crypto).unwrap();
        root
    }

    #[test]
    fn a_row_links_the_document_and_the_test_that_anchors_it() {
        let root = repository("row", MINIMAL, CRYPTO);
        let standards = Standards::load(&root).expect("loads");
        fs::remove_dir_all(&root).unwrap();
        standards
            .check(&with_trust())
            .expect("the cited line is found once");
        let rendered = standards.table(&catalog());
        assert!(
            rendered.starts_with(
                "<p class=\"source\">1 standards registered, 1 of them pinned to the document's own published vectors."
            ),
            "the page counts its own register: {rendered}"
        );
        assert!(
            rendered.contains("## Cryptography\n\nThe primitives.\n\n<nav class=\"hw-index\""),
            "{rendered}"
        );
        assert!(rendered.contains("### RFC 4493 {#rfc-4493}"), "{rendered}");
        assert!(
            rendered.contains("href=\"https://www.rfc-editor.org/info/rfc4493\""),
            "{rendered}"
        );
        assert!(
            rendered.contains(
                "href=\"https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-lorawan/src/crypto.rs#L4\""
            ),
            "{rendered}"
        );
        assert!(rendered.contains("<h4>Published vector</h4>"), "{rendered}");
        assert!(
            rendered.contains("<b>The test</b><small><code>crypto.rs</code> line 4"),
            "{rendered}"
        );
    }

    #[test]
    fn a_cited_line_follows_its_text_when_edits_move_it() {
        let moved = format!("// Two lines of new docs.\n//\n{CRYPTO}");
        let root = repository("moved", MINIMAL, &moved);
        let standards = Standards::load(&root).expect("loads");
        fs::remove_dir_all(&root).unwrap();
        assert!(matches!(standards.entries[0].line, Some(Ok(6))));
    }

    #[test]
    fn text_on_no_line_or_on_several_is_refused() {
        let root = repository("gone", MINIMAL, "fn renamed() {}\n");
        let gone = Standards::load(&root).expect("loads");
        fs::remove_dir_all(&root).unwrap();
        let problem = gone.check(&with_trust()).unwrap_err();
        assert!(
            problem.contains("rfc-4493: no line of crates/pamoja-lorawan/src/crypto.rs contains"),
            "{problem}"
        );

        let twice = format!("{CRYPTO}{CRYPTO}");
        let root = repository("twice", MINIMAL, &twice);
        let doubled = Standards::load(&root).expect("loads");
        fs::remove_dir_all(&root).unwrap();
        let problem = doubled.check(&with_trust()).unwrap_err();
        assert!(
            problem.contains("2 lines of crates/pamoja-lorawan/src/crypto.rs contain"),
            "{problem}"
        );
        assert!(problem.contains("lines 4, 10"), "{problem}");
    }

    #[test]
    fn a_line_number_in_the_evidence_is_refused() {
        let numbered = MINIMAL.replace("crypto.rs\"", "crypto.rs#L4\"");
        let problem = Standards::parse(&numbered)
            .expect("parses")
            .check(&with_trust())
            .unwrap_err();
        assert!(problem.contains("cites a line by number"), "{problem}");
    }

    #[test]
    fn a_page_is_linked_as_source_so_the_line_anchor_lands() {
        let page = MINIMAL
            .replace("crates/pamoja-lorawan/src/crypto.rs", "docs/radio.md")
            .replace(
                "fn cmac_of_the_empty_message_matches_rfc_4493",
                "CEPT's ERC Recommendation 70-03",
            );
        let root = repository("page", &page, CRYPTO);
        fs::write(
            root.join("docs/radio.md"),
            "# Radio\n\nCEPT's ERC Recommendation 70-03 sets out the bands.\n",
        )
        .unwrap();
        let standards = Standards::load(&root).expect("loads");
        fs::remove_dir_all(&root).unwrap();
        let rendered = standards.table(&catalog());
        assert!(
            rendered.contains(
                "href=\"https://github.com/molexxxx/pamoja/blob/main/docs/radio.md?plain=1#L3\""
            ),
            "{rendered}"
        );
        assert!(
            rendered.contains("<b>The page</b><small><code>radio.md</code> line 3"),
            "{rendered}"
        );
    }

    #[test]
    fn every_url_reaches_the_link_checker() {
        let standards = Standards::parse(MINIMAL).expect("parses");
        let sources = standards.sources();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].2, "https://www.rfc-editor.org/info/rfc4493");
    }

    #[test]
    fn a_repeated_key_or_an_unknown_anchor_is_refused() {
        let twice = format!("{MINIMAL}{}", MINIMAL.replace("[[group]]", "[[ignored]]"));
        let doubled = Standards::parse(&twice).expect("parses");
        assert!(doubled.check(&catalog()).is_err(), "a repeated key");

        let wrong = MINIMAL.replace("anchor = \"vector\"", "anchor = \"vibes\"");
        assert!(
            Standards::parse(&wrong)
                .expect("parses")
                .check(&catalog())
                .is_err(),
            "an unknown anchor kind"
        );
    }

    #[test]
    fn the_register_counts_what_it_holds() {
        let standards = Standards::parse(MINIMAL).expect("parses");
        assert_eq!(standards.counts(), (1, 1));
    }
}
