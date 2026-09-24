//! The shape every guide page keeps, checked by `cargo xtask docs --check`.
//!
//! A guide walks one capability in all four languages, and a reader moving between guides
//! expects the same sections in the same order: what the example does and what it proves,
//! how to run it, a section for each language that opens with a paragraph before its code,
//! the values at a glance, what goes wrong, where next, and the reference. The check also
//! reads the calls each language names in "Values at a glance", in a column headed with the
//! language or in its tab, and fails when one names something that language's sources never
//! mention, which is how a renamed call, or a Python name written into the TypeScript table,
//! is caught before a reader copies it.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const SECTIONS: [&str; 10] = [
    "What the example does",
    "Run it",
    "Rust",
    "TypeScript",
    "Python",
    "C#",
    "Values at a glance",
    "When it goes wrong",
    "Where next",
    "Reference",
];

const LANGUAGES: [&str; 4] = ["Rust", "TypeScript", "Python", "C#"];

/// The words each language's sources contain, which a guide's calls are checked against.
pub(crate) struct Corpora {
    words: [HashSet<String>; 4],
}

impl Corpora {
    /// Reads the sources of the Rust crates and of each binding.
    ///
    /// # Arguments
    ///
    /// * `root` - the repository root.
    ///
    /// # Returns
    ///
    /// The words of each language's sources.
    ///
    /// # Errors
    ///
    /// Returns a message naming a source that could not be read.
    pub(crate) fn load(root: &Path) -> Result<Self, String> {
        let mut rust = Vec::new();
        walk(&root.join("crates"), &["rs"], &mut rust)?;
        rust.retain(|path| !path.starts_with(root.join("crates").join("xtask")));
        let mut typescript = Vec::new();
        walk(
            &root.join("bindings/node/packages"),
            &["ts"],
            &mut typescript,
        )?;
        let mut python = Vec::new();
        walk(
            &root.join("bindings/python/packages"),
            &["py", "pyi"],
            &mut python,
        )?;
        let mut dotnet = Vec::new();
        walk(&root.join("bindings/dotnet/src"), &["cs"], &mut dotnet)?;
        Ok(Self {
            words: [
                words_of(&rust)?,
                words_of(&typescript)?,
                words_of(&python)?,
                words_of(&dotnet)?,
            ],
        })
    }

    fn mentions(&self, language: &str, word: &str) -> bool {
        LANGUAGES
            .iter()
            .position(|known| *known == language)
            .is_some_and(|index| self.words[index].contains(word))
    }
}

/// Checks every guide under `docs/guides` for the template sections and for calls that name
/// nothing in their language's sources.
///
/// # Arguments
///
/// * `root` - the repository root.
///
/// # Returns
///
/// `Ok(())` when every guide keeps the template.
///
/// # Errors
///
/// Returns every problem found, one per line, each naming its guide.
pub(crate) fn check(root: &Path) -> Result<(), String> {
    let mut guides = Vec::new();
    crate::docs::collect_markdown(&root.join("docs/guides"), &mut guides)?;
    let corpora = Corpora::load(root)?;
    let mut problems = Vec::new();
    for guide in guides {
        let text =
            fs::read_to_string(&guide).map_err(|e| format!("reading {}: {e}", guide.display()))?;
        let name = guide
            .strip_prefix(root)
            .unwrap_or(&guide)
            .to_string_lossy()
            .replace('\\', "/");
        for problem in sections(&text).into_iter().chain(values(&text, &corpora)) {
            problems.push(format!("{name}: {problem}"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

// The level-two headings outside code fences, with the index of the line each is on.
fn headings(lines: &[&str]) -> Vec<(usize, String)> {
    let mut fenced = false;
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        } else if !fenced {
            if let Some(heading) = line.strip_prefix("## ") {
                found.push((index, heading.trim().to_owned()));
            }
        }
    }
    found
}

fn sections(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let found = headings(&lines);
    let mut problems = Vec::new();
    let mut after = 0;
    for section in SECTIONS {
        match found.iter().position(|(_, heading)| heading == section) {
            None => problems.push(format!("has no `## {section}` section")),
            Some(at) if at < after => problems.push(format!(
                "has `## {section}` out of order; a guide runs {}",
                SECTIONS.join(", ")
            )),
            Some(at) => after = at,
        }
    }
    let body = |section: &str| -> Option<&[&str]> {
        let at = found.iter().position(|(_, heading)| heading == section)?;
        let start = found[at].0 + 1;
        let end = found.get(at + 1).map_or(lines.len(), |(line, _)| *line);
        Some(&lines[start..end])
    };
    if let Some(what) = body("What the example does") {
        if !what.iter().any(|line| line.trim() == "It proves:") {
            problems.push("says what the example does but has no `It proves:` list".to_owned());
        }
    }
    for language in LANGUAGES {
        let Some(section) = body(language) else {
            continue;
        };
        let opening = section.iter().find(|line| !line.trim().is_empty());
        if opening.is_none_or(|line| line.starts_with("<!--") || line.starts_with("```")) {
            problems.push(format!(
                "opens the {language} section with code; open it with a paragraph on the \
                 package, what the ideas are called there, and how a refusal reaches the caller"
            ));
        }
    }
    problems
}

fn values(text: &str, corpora: &Corpora) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let found = headings(&lines);
    let Some(at) = found
        .iter()
        .position(|(_, heading)| heading == "Values at a glance")
    else {
        return Vec::new();
    };
    let start = found[at].0 + 1;
    let end = found.get(at + 1).map_or(lines.len(), |(line, _)| *line);

    let mut problems = Vec::new();
    let mut tab: Option<&str> = None;
    let mut columns: Option<Vec<String>> = None;
    for line in &lines[start..end] {
        if let Some(heading) = line.strip_prefix("### ") {
            tab = LANGUAGES
                .iter()
                .copied()
                .find(|language| *language == heading.trim());
            columns = None;
            continue;
        }
        if line.trim() == crate::site::pages::LANGUAGES_END {
            tab = None;
            continue;
        }
        if !line.starts_with('|') {
            columns = None;
            continue;
        }
        let cells = cells(line);
        if cells
            .iter()
            .all(|cell| !cell.is_empty() && cell.chars().all(|c| matches!(c, '-' | ':' | ' ')))
        {
            continue;
        }
        let Some(header) = &columns else {
            columns = Some(cells);
            continue;
        };
        for (column, cell) in cells.iter().enumerate() {
            let language = tab.or_else(|| {
                header
                    .get(column)
                    .and_then(|name| LANGUAGES.iter().copied().find(|known| known == name))
            });
            let Some(language) = language else {
                continue;
            };
            for span in code_spans(cell) {
                for token in api_tokens(span) {
                    if !corpora.mentions(language, &token) {
                        problems.push(format!(
                            "the {language} calls in Values at a glance name `{token}`, in \
                             `{span}`, which no {language} source mentions"
                        ));
                    }
                }
            }
        }
    }
    problems
}

// A table row's cells, split on the pipes a cell does not escape.
fn cells(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for ch in inner.chars() {
        if ch == '|' && !escaped {
            cells.push(current.trim().to_owned());
            current.clear();
        } else {
            current.push(ch);
        }
        escaped = ch == '\\';
    }
    cells.push(current.trim().to_owned());
    cells
}

fn code_spans(cell: &str) -> Vec<&str> {
    cell.split('`').skip(1).step_by(2).collect()
}

// The words of a code span that read as a call or a type rather than a placeholder: one
// followed by a parenthesis, one on either side of a `.` or `::`, and one that starts with a
// capital. Quoted text is left out, so a name passed as a string is not read as a call.
fn api_tokens(span: &str) -> Vec<String> {
    let mut masked = String::with_capacity(span.len());
    let mut quote = None;
    for ch in span.chars() {
        match quote {
            Some(open) => {
                if ch == open {
                    quote = None;
                }
                masked.push(' ');
            }
            None if ch == '"' || ch == '\'' => {
                quote = Some(ch);
                masked.push(' ');
            }
            None => masked.push(ch),
        }
    }
    let chars: Vec<char> = masked.chars().collect();
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if !is_word(chars[index]) {
            index += 1;
            continue;
        }
        let start = index;
        while index < chars.len() && is_word(chars[index]) {
            index += 1;
        }
        if chars[start].is_ascii_digit() {
            continue;
        }
        let token: String = chars[start..index].iter().collect();
        let before = start.checked_sub(1).map(|at| chars[at]);
        let path_before = start >= 2 && chars[start - 2] == ':' && chars[start - 1] == ':';
        let after = chars.get(index).copied();
        let path_after = chars.get(index) == Some(&':') && chars.get(index + 1) == Some(&':');
        let is_api = after == Some('(')
            || before == Some('.')
            || after == Some('.')
            || path_before
            || path_after
            || token.starts_with(|c: char| c.is_ascii_uppercase());
        if is_api {
            tokens.push(token);
        }
    }
    tokens
}

fn walk(dir: &Path, extensions: &[&str], into: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| format!("reading {}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("reading {}: {e}", dir.display()))?
            .path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if !matches!(
                name,
                "target" | "node_modules" | "dist" | "bin" | "obj" | ".venv" | "__pycache__"
            ) {
                walk(&path, extensions, into)?;
            }
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            into.push(path);
        }
    }
    Ok(())
}

fn words_of(paths: &[PathBuf]) -> Result<HashSet<String>, String> {
    let mut words = HashSet::new();
    for path in paths {
        let text =
            fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
        for word in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            if !word.is_empty() {
                words.insert(word.to_owned());
            }
        }
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUIDE: &str = "# Keys\n\nIntro.\n\n## What the example does\n\nIt runs.\n\nIt proves:\n\n- a thing\n\n## Run it\n\nrun\n\n## Rust\n\nIn Rust, a crate.\n\n<!-- snippet: a.rs#example -->\n<!-- end -->\n\n## TypeScript\n\nIn TypeScript, a package.\n\n## Python\n\nIn Python, a module.\n\n## C#\n\nIn C#, a class.\n\n## Values at a glance\n\n| What | Rust | TypeScript |\n| --- | --- | --- |\n| check | `keyexpr::is_valid(ke)` | `keyexpr.isValid(ke)` |\n\n### Python\n\n| To | Call |\n| --- | --- |\n| check | `is_valid(ke)` |\n\n<!-- languages end -->\n\n## When it goes wrong\n\nIt breaks.\n\n## Where next\n\n## Reference\n";

    fn corpora(rust: &[&str], typescript: &[&str], python: &[&str], dotnet: &[&str]) -> Corpora {
        let set = |words: &[&str]| words.iter().map(|word| (*word).to_owned()).collect();
        Corpora {
            words: [set(rust), set(typescript), set(python), set(dotnet)],
        }
    }

    #[test]
    fn a_guide_that_keeps_the_template_passes() {
        assert!(sections(GUIDE).is_empty(), "{:?}", sections(GUIDE));
        let known = corpora(
            &["keyexpr", "is_valid"],
            &["keyexpr", "isValid"],
            &["is_valid"],
            &[],
        );
        assert!(values(GUIDE, &known).is_empty());
    }

    #[test]
    fn a_missing_or_misplaced_section_is_named() {
        let missing = GUIDE.replace("## When it goes wrong\n\nIt breaks.\n\n", "");
        assert_eq!(
            sections(&missing),
            ["has no `## When it goes wrong` section"]
        );
        let swapped = GUIDE
            .replace("## Python\n", "## PLACEHOLDER\n")
            .replace("## TypeScript\n", "## Python\n")
            .replace("## PLACEHOLDER\n", "## TypeScript\n");
        assert!(sections(&swapped)[0].contains("`## Python` out of order"));
        let proofless = GUIDE.replace("It proves:\n", "");
        assert_eq!(
            sections(&proofless),
            ["says what the example does but has no `It proves:` list"]
        );
    }

    #[test]
    fn a_language_section_opens_with_a_paragraph() {
        let bare = GUIDE.replace("In Rust, a crate.\n\n", "");
        let problems = sections(&bare);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("opens the Rust section with code"));
    }

    #[test]
    fn a_call_its_language_never_mentions_is_refused() {
        let known = corpora(
            &["keyexpr", "is_valid"],
            &["keyexpr", "is_valid"],
            &["is_valid"],
            &[],
        );
        let problems = values(GUIDE, &known);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("TypeScript calls in Values at a glance name `isValid`"));
    }

    #[test]
    fn a_call_reads_apart_from_its_placeholders_and_strings() {
        assert_eq!(
            api_tokens("keyexpr::matches(pattern, key)"),
            ["keyexpr", "matches"]
        );
        assert_eq!(
            api_tokens("new CommandProtocol(command, retries)"),
            ["CommandProtocol"]
        );
        assert_eq!(api_tokens("enumValue('MAV_TYPE_GCS')"), ["enumValue"]);
        assert_eq!(
            api_tokens("ZenohTransport::new(ZenohConfig::new().connect_to(endpoint))"),
            ["ZenohTransport", "new", "ZenohConfig", "new", "connect_to"]
        );
        assert!(api_tokens("0x2A").is_empty());
    }

    #[test]
    fn a_row_splits_on_the_pipes_it_does_not_escape() {
        assert_eq!(cells("| a | `b \\| c` | d |"), ["a", "`b \\| c`", "d"]);
    }
}
