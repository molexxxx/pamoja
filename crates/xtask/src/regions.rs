//! Generated regions inside hand-written Markdown. A `<!-- table: ... -->` block is
//! rendered from the capability map and a `<!-- snippet: path#anchor -->` block is
//! spliced from a marked region of a test file; each is closed by `<!-- end -->`.
//! The text between the markers is replaced on every run, so an edit made there by
//! hand is undone by `cargo xtask docs` and reported by `cargo xtask docs --check`.

use std::fs;
use std::path::Path;

/// The marker that closes a generated region.
pub const END: &str = "<!-- end -->";

/// Whether a Markdown file holds any generated region.
pub fn has_regions(text: &str) -> bool {
    text.contains("<!-- table:") || text.contains("<!-- snippet:")
}

/// Re-render every region in `text`, calling `render` with the directive inside
/// the opening marker (`table: chapters`, `snippet: path#anchor`).
///
/// # Errors
///
/// Returns the reason when a region is not closed or a render fails.
pub fn process(
    text: &str,
    render: &mut dyn FnMut(&str) -> Result<String, String>,
) -> Result<String, String> {
    let mut out = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        out.push(line.to_owned());
        let Some(directive) = directive_of(line) else {
            continue;
        };
        let rendered = render(directive).map_err(|err| format!("`{directive}`: {err}"))?;
        out.push(rendered.trim_end().to_owned());
        loop {
            match lines.next() {
                Some(inner) if inner.trim() == END => {
                    out.push(inner.to_owned());
                    break;
                }
                Some(_) => {}
                None => return Err(format!("`{directive}` has no closing {END}")),
            }
        }
    }
    let mut joined = out.join("\n");
    if text.ends_with('\n') {
        joined.push('\n');
    }
    Ok(joined)
}

/// The directive of an opening marker line, if the line is one.
fn directive_of(line: &str) -> Option<&str> {
    let inner = line
        .trim()
        .strip_prefix("<!--")?
        .strip_suffix("-->")?
        .trim();
    (inner.starts_with("table:") || inner.starts_with("snippet:")).then_some(inner)
}

/// Splice the anchored region `spec` (`path#anchor`, the path relative to the
/// repository root) as a fenced code block that names its source file.
///
/// # Errors
///
/// Returns the reason when the spec is malformed, the file is missing, or the
/// anchor is not found.
pub fn snippet(root: &Path, spec: &str) -> Result<String, String> {
    let (path, anchor) = spec
        .split_once('#')
        .ok_or_else(|| format!("snippet `{spec}` is not of the form path#anchor"))?;
    let file = root.join(path);
    let source =
        fs::read_to_string(&file).map_err(|err| format!("reading {}: {err}", file.display()))?;
    let code = extract(&source, anchor)
        .ok_or_else(|| format!("{path} has no `ANCHOR: {anchor}` region"))?;
    let language = match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust",
        Some("ts") => "typescript",
        Some("py") => "python",
        Some("cs") => "csharp",
        Some("js") => "javascript",
        Some(other) => other,
        None => "",
    };
    Ok(format!(
        "From [`{path}`](https://github.com/molexxxx/pamoja/blob/main/{path}):\n\n```{language}\n{code}\n```"
    ))
}

/// The lines between `ANCHOR: name` and `ANCHOR_END: name`, other anchor markers
/// dropped, common indentation removed, and blank edges trimmed.
pub fn extract(source: &str, anchor: &str) -> Option<String> {
    let mut lines = source.lines();
    lines
        .by_ref()
        .find(|line| is_marker(line, "ANCHOR:", anchor))?;
    let mut body: Vec<&str> = Vec::new();
    let mut closed = false;
    for line in lines {
        if is_marker(line, "ANCHOR_END:", anchor) {
            closed = true;
            break;
        }
        if line.contains("ANCHOR:") || line.contains("ANCHOR_END:") {
            continue;
        }
        body.push(line);
    }
    if !closed {
        return None;
    }
    while body.first().is_some_and(|line| line.trim().is_empty()) {
        body.remove(0);
    }
    while body.last().is_some_and(|line| line.trim().is_empty()) {
        body.pop();
    }
    let indent = body
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    let dedented: Vec<&str> = body
        .iter()
        .map(|line| {
            if line.len() >= indent {
                &line[indent..]
            } else {
                line.trim_start()
            }
        })
        .collect();
    Some(dedented.join("\n"))
}

/// Whether a line is the `keyword name` marker for `anchor`, in any comment style.
fn is_marker(line: &str, keyword: &str, anchor: &str) -> bool {
    let Some(rest) = line.find(keyword).map(|at| &line[at + keyword.len()..]) else {
        return false;
    };
    rest.split_whitespace().next() == Some(anchor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_the_text_between_the_markers() {
        let text = "intro\n<!-- table: chapters -->\nstale\nlines\n<!-- end -->\noutro\n";
        let mut seen = Vec::new();
        let processed = process(text, &mut |directive| {
            seen.push(directive.to_owned());
            Ok("fresh\n".to_owned())
        })
        .unwrap();
        assert_eq!(seen, ["table: chapters"]);
        assert_eq!(
            processed,
            "intro\n<!-- table: chapters -->\nfresh\n<!-- end -->\noutro\n"
        );
    }

    #[test]
    fn leaves_text_without_regions_alone_and_rejects_an_unclosed_one() {
        let plain = "# Title\n\nno regions here\n";
        let same = process(plain, &mut |_| Err("never called".to_owned())).unwrap();
        assert_eq!(same, plain);
        assert!(!has_regions(plain));

        let unclosed = "<!-- snippet: a.rs#x -->\nbody\n";
        let err = process(unclosed, &mut |_| Ok(String::new())).unwrap_err();
        assert!(err.contains("no closing"));
    }

    #[test]
    fn extracts_a_dedented_anchor_region() {
        let source = concat!(
            "fn test() {\n",
            "    // ANCHOR: example\n",
            "    let reading = 21.5;\n",
            "\n",
            "    // ANCHOR: inner\n",
            "    assert_eq!(reading, 21.5);\n",
            "    // ANCHOR_END: inner\n",
            "    // ANCHOR_END: example\n",
            "}\n",
        );
        assert_eq!(
            extract(source, "example").unwrap(),
            "let reading = 21.5;\n\nassert_eq!(reading, 21.5);"
        );
        assert_eq!(
            extract(source, "inner").unwrap(),
            "assert_eq!(reading, 21.5);"
        );
        assert!(extract(source, "missing").is_none());
        assert!(extract("# ANCHOR: open\nx = 1\n", "open").is_none());
    }

    #[test]
    fn a_python_marker_matches_by_name_only() {
        assert!(is_marker("    # ANCHOR: run", "ANCHOR:", "run"));
        assert!(!is_marker("    # ANCHOR: runner", "ANCHOR:", "run"));
        assert!(is_marker("// ANCHOR_END: run", "ANCHOR_END:", "run"));
    }
}
