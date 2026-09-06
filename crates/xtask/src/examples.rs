//! The examples page: every runnable program under `examples/`, with what it shows and the
//! line that runs it, and every guide's example in the four languages, with what it proves
//! and the file that runs in CI. Rendered into the `<!-- table: examples -->` region of
//! `docs/examples.md`, so `cargo xtask docs --check` fails when a program or a guide
//! changes without the page following.

use std::fs;
use std::path::Path;

use crate::catalog::{command, escape, Catalog, SITE};
use crate::docs;

/// The repository, for the links to each file.
const REPO: &str = "https://github.com/molexxxx/pamoja";

/// The languages a guide's runners are written in, by the runner file's extension.
const RUNNERS: [(&str, &str, &str); 4] = [
    (".rs", "rust", "Rust"),
    (".ts", "node", "TypeScript"),
    (".py", "python", "Python"),
    (".cs", "dotnet", "C#"),
];

/// Render the region: the programs, then the guides' examples by chapter.
///
/// # Arguments
///
/// * `root` - the repository root.
/// * `catalog` - the capability map, for the chapters and their guides.
///
/// # Returns
///
/// The Markdown that replaces the `<!-- table: examples -->` region.
///
/// # Errors
///
/// When an example or a guide cannot be read or parsed.
pub fn table(root: &Path, catalog: &Catalog) -> Result<String, String> {
    let mut out = String::from(
        "## Programs\n\nEach one is a complete program with a `main`, written to be read top to bottom and run with nothing plugged in. The line beside it runs it.\n\n<div class=\"pkgs\">\n",
    );
    for program in programs(root)? {
        out.push_str(&program_card(&program));
    }
    out.push_str("</div>\n\n## Guide examples\n\nEvery guide carries the same example in Rust, TypeScript, Python, and C#, spliced from the file that runs it in CI. The buttons open those files; the guide explains them.\n");
    for chapter in &catalog.chapters {
        let mut cards = String::new();
        for capability in catalog.in_chapter(&chapter.key) {
            let Some(guide) = &capability.guide else {
                continue;
            };
            let text = fs::read_to_string(root.join("docs").join(guide))
                .map_err(|err| format!("reading docs/{guide}: {err}"))?;
            cards.push_str(&guide_card(&capability.title, guide, &text));
        }
        if !cards.is_empty() {
            out.push_str(&format!(
                "\n### {}\n\n<div class=\"pkgs\">\n{cards}</div>\n",
                escape(&chapter.title)
            ));
        }
    }
    Ok(out.trim_end().to_owned())
}

/// One program under `examples/`: its name, what its module doc says first, and how to
/// run it.
struct Program {
    name: String,
    summary: String,
    run: String,
}

// The programs, in name order, from their module docs.
fn programs(root: &Path) -> Result<Vec<Program>, String> {
    let dir = root.join("examples");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .map_err(|err| format!("reading {}: {err}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .filter_map(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let path = dir.join(format!("{name}.rs"));
            let source = fs::read_to_string(&path)
                .map_err(|err| format!("reading {}: {err}", path.display()))?;
            let file = syn::parse_file(&source)
                .map_err(|err| format!("parsing {}: {err}", path.display()))?;
            let doc = docs::doc_of(&file.attrs);
            let (summary, run) = summary_and_run(&doc, &name);
            Ok(Program { name, summary, run })
        })
        .collect()
}

/// The first paragraph of a program's doc, and the line that runs it: the command the doc
/// gives after "Run with:", or the default `cargo run` line for the example.
///
/// # Arguments
///
/// * `doc` - the module doc as Markdown.
/// * `name` - the example's name.
///
/// # Returns
///
/// The summary and the command.
pub fn summary_and_run(doc: &str, name: &str) -> (String, String) {
    let summary = doc
        .split("\n\n")
        .map(str::trim)
        .find(|paragraph| !paragraph.is_empty())
        .unwrap_or_default()
        .replace('\n', " ");
    let run = doc
        .lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix("Run with:")?;
            let start = rest.find('`')? + 1;
            let end = rest[start..].find('`')? + start;
            Some(rest[start..end].to_owned())
        })
        .unwrap_or_else(|| format!("cargo run -p pamoja-examples --example {name}"));
    (summary, run)
}

fn program_card(program: &Program) -> String {
    format!(
        "<div class=\"pkg stack\" id=\"example-{name}\">\n<div class=\"pkg-head\">\n<div class=\"pkg-what\"><a class=\"pkg-title\" href=\"{REPO}/blob/main/examples/{name}.rs\">{name}</a><code class=\"pkg-import\">examples/{name}.rs</code><p>{}</p></div>\n{}\n</div>\n</div>\n",
        markdown_inline(&program.summary),
        command(&program.run),
        name = program.name
    )
}

/// One guide's card: what its example proves, and the four files that run it.
///
/// # Arguments
///
/// * `title` - the capability's title.
/// * `guide` - the guide's path under `docs/`.
/// * `text` - the guide's Markdown.
///
/// # Returns
///
/// The card's HTML.
pub fn guide_card(title: &str, guide: &str, text: &str) -> String {
    let page = guide.trim_end_matches(".md");
    let proves: String = proves(text)
        .iter()
        .map(|line| format!("<li>{}</li>", markdown_inline(line)))
        .collect();
    let proves = if proves.is_empty() {
        String::new()
    } else {
        format!("<ul class=\"pkg-proves\">{proves}</ul>")
    };
    let mut buttons: Vec<String> = runners(text)
        .into_iter()
        .map(|(path, key, language)| {
            format!(
                "<a class=\"pkg-btn {key}\" href=\"{REPO}/blob/main/{path}\">{language} <code>{}</code></a>",
                escape(path.rsplit('/').next().unwrap_or(&path))
            )
        })
        .collect();
    buttons.push(format!(
        "<a class=\"pkg-btn\" href=\"{SITE}/{page}.html\">Guide</a>"
    ));
    format!(
        "<div class=\"pkg\" id=\"guide-{}\">\n<div class=\"pkg-head\">\n<div class=\"pkg-what\"><a class=\"pkg-title\" href=\"{SITE}/{page}.html\">{}</a>{proves}</div>\n</div>\n<div class=\"pkg-foot\">\n<div class=\"pkg-btns\">{}</div>\n</div>\n</div>\n",
        page.rsplit('/').next().unwrap_or(page),
        escape(title),
        buttons.join("")
    )
}

/// The bullets under a guide's "It proves:" line.
///
/// # Arguments
///
/// * `text` - the guide's Markdown.
///
/// # Returns
///
/// Each bullet as one line, continuation lines joined.
pub fn proves(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = text.lines().skip_while(|line| line.trim() != "It proves:");
    if lines.next().is_none() {
        return out;
    }
    for line in lines {
        if let Some(bullet) = line.strip_prefix("- ") {
            out.push(bullet.trim().to_owned());
        } else if line.starts_with("  ") && !out.is_empty() {
            let last = out.last_mut().expect("a bullet to continue");
            last.push(' ');
            last.push_str(line.trim());
        } else if !line.trim().is_empty() || !out.is_empty() {
            break;
        }
    }
    out
}

/// The files a guide splices its examples from, one per language, in language order.
///
/// # Arguments
///
/// * `text` - the guide's Markdown.
///
/// # Returns
///
/// (path, language key, language name) per file, each file once.
pub fn runners(text: &str) -> Vec<(String, &'static str, &'static str)> {
    let mut paths: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("<!-- snippet: ") {
        let spec = &rest[at + "<!-- snippet: ".len()..];
        let Some(end) = spec.find(" -->") else {
            break;
        };
        let path = spec[..end].split('#').next().unwrap_or_default().to_owned();
        if !paths.contains(&path) {
            paths.push(path);
        }
        rest = &spec[end..];
    }
    let mut out = Vec::new();
    for (extension, key, language) in RUNNERS {
        if let Some(path) = paths.iter().find(|path| path.ends_with(extension)) {
            out.push((path.clone(), key, language));
        }
    }
    out
}

// Backticks in a doc's first paragraph or a guide's bullet become code, and the rest is
// escaped; the pages carry these inside HTML, where Markdown does not reach.
fn markdown_inline(text: &str) -> String {
    let mut out = String::new();
    for (index, part) in text.split('`').enumerate() {
        if index % 2 == 1 {
            out.push_str(&format!("<code>{}</code>", escape(part)));
        } else {
            out.push_str(&escape(part));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_gives_its_first_paragraph_and_its_run_line() {
        let doc = "Metered-link encoding: pack a batch.\n\nOn a long-range radio every byte costs.\n\nRun with: `cargo run -p pamoja-examples --example batched_telemetry`\n";
        let (summary, run) = summary_and_run(doc, "batched_telemetry");
        assert_eq!(summary, "Metered-link encoding: pack a batch.");
        assert_eq!(
            run,
            "cargo run -p pamoja-examples --example batched_telemetry"
        );
        let (_, run) = summary_and_run("Two\nlines.\n", "sitl");
        assert_eq!(run, "cargo run -p pamoja-examples --example sitl");
    }

    #[test]
    fn a_guide_gives_what_it_proves_and_the_files_that_run_it() {
        let text = "# Modbus RTU\n\nIt proves:\n\n- A request is eight bytes: the address,\n  the code, and the checksum.\n- A reply validates its checksum.\n\n## Rust\n\n<!-- snippet: examples/tests/guides/modbus.rs#example -->\n```rust\n```\n<!-- end -->\n\n<!-- snippet: examples/tests/guides/modbus.rs#frame -->\n<!-- snippet: bindings/node/guides/modbus.ts#example -->\n<!-- snippet: bindings/python/guides/modbus.py#example -->\n<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs#example -->\n";
        assert_eq!(
            proves(text),
            [
                "A request is eight bytes: the address, the code, and the checksum.",
                "A reply validates its checksum."
            ]
        );
        let files = runners(text);
        assert_eq!(files.len(), 4);
        assert_eq!(files[0].0, "examples/tests/guides/modbus.rs");
        assert_eq!(files[3].2, "C#");
        let card = guide_card("Modbus RTU", "guides/modbus.md", text);
        assert!(card.starts_with("<div class=\"pkg\" id=\"guide-modbus\">"));
        assert!(card.contains("<a class=\"pkg-title\" href=\"https://pamoja.molex.cloud/docs/guides/modbus.html\">Modbus RTU</a><ul class=\"pkg-proves\"><li>A request is eight bytes"));
        assert!(card.contains("<a class=\"pkg-btn node\" href=\"https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/modbus.ts\">TypeScript <code>modbus.ts</code></a>"));
        assert!(card.ends_with("<a class=\"pkg-btn\" href=\"https://pamoja.molex.cloud/docs/guides/modbus.html\">Guide</a></div>\n</div>\n</div>\n"));
        assert!(proves("no such line").is_empty());
    }

    #[test]
    fn the_page_lists_every_program_and_every_guide() {
        let root = docs::repo_root();
        let catalog = Catalog::load(&root).unwrap();
        let page = table(&root, &catalog).unwrap();
        assert!(page.starts_with("## Programs\n"));
        assert!(page.contains("<a class=\"pkg-title\" href=\"https://github.com/molexxxx/pamoja/blob/main/examples/batched_telemetry.rs\">batched_telemetry</a>"));
        assert!(page.contains("cargo run -p pamoja-examples --example batched_telemetry"));
        assert!(page.contains("### Field I/O") && page.contains("id=\"guide-modbus\""));
        assert!(page.contains("<code>modbus.py</code>"));
    }
}
