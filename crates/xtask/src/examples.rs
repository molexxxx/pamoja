//! The examples page: every runnable program under `examples/`, with what it shows and the
//! line that runs it, and every guide's example in the four languages, with what it proves
//! and the file that runs in CI. Rendered into the `<!-- table: examples -->` region of
//! `docs/examples.md`, so `cargo xtask docs --check` fails when a program or a guide
//! changes without the page following.

use std::fs;
use std::path::Path;

use crate::catalog::{command, escape, rustdoc_url};
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
pub fn table(root: &Path) -> Result<String, String> {
    let mut out = String::from(
        "## Programs\n\nEach one is a complete program with a `main`, written to be read top to bottom and run with nothing plugged in. The line beside it runs it.\n\n<div class=\"pkgs\">\n",
    );
    for (at, program) in programs_in(root, "examples")?.iter().enumerate() {
        out.push_str(&program_card(program, at + 1));
    }
    out.push_str("</div>\n\n## Community programs\n\nPrograms people have shared, held to the same bar: complete, run in CI with nothing plugged in, and credited in the file. The [community page](community.md#share-an-example) says how to add one.\n\n");
    let community = programs_in(root, "examples/community")?;
    if community.is_empty() {
        out.push_str("<p>None yet. The first one is yours to add.</p>\n");
    } else {
        out.push_str("<div class=\"pkgs\">\n");
        for (at, program) in community.iter().enumerate() {
            out.push_str(&program_card(program, at + 1));
        }
        out.push_str("</div>\n");
    }
    Ok(out.trim_end().to_owned())
}

/// One program: its name, the file it lives in from the repository root, what its module
/// doc says first, how to run it, and the capability crates it reaches for.
struct Program {
    name: String,
    path: String,
    summary: String,
    run: String,
    uses: Vec<String>,
}

// The capability crates a program imports, read from its `use` lines. The examples crate
// itself is the harness, not a capability, so it is left out.
fn uses(source: &str) -> Vec<String> {
    let mut out: Vec<String> = source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("use "))
        .filter_map(|rest| rest.split([':', ';', ' ']).next())
        .filter(|name| name.starts_with("pamoja_") && *name != "pamoja_examples")
        .map(|name| name.replace('_', "-"))
        .collect();
    out.sort();
    out.dedup();
    out
}

// The programs in one directory, in name order, from their module docs; none when the
// directory does not exist yet.
fn programs_in(root: &Path, relative: &str) -> Result<Vec<Program>, String> {
    let dir = root.join(relative);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
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
            Ok(Program {
                path: format!("{relative}/{name}.rs"),
                uses: uses(&source),
                name,
                summary,
                run,
            })
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

fn program_card(program: &Program, number: usize) -> String {
    // A program that reaches for everything would otherwise bury its own row, so the list
    // stops at eight and the file itself carries the rest.
    const SHOWN: usize = 8;
    let listed: String = program
        .uses
        .iter()
        .take(SHOWN)
        .map(|krate| {
            format!(
                "<li><a href=\"{}\"><code>{}</code></a></li>",
                rustdoc_url(krate),
                escape(krate)
            )
        })
        .collect();
    let rest = program.uses.len().saturating_sub(SHOWN);
    let more = if rest == 0 {
        String::new()
    } else {
        format!(
            "<li class=\"uses-more\"><a href=\"{REPO}/blob/main/{}\">and {rest} more</a></li>",
            program.path
        )
    };
    let uses = if listed.is_empty() {
        String::new()
    } else {
        format!("<ul class=\"uses\"><li class=\"uses-head\">Imports</li>{listed}{more}</ul>\n")
    };
    format!(
        "<div class=\"pkg stack program\" id=\"example-{name}\">\n<div class=\"pkg-head\">\n\
         <div class=\"pkg-what\"><p class=\"program-id\">Program {number}</p>\
         <a class=\"pkg-title\" href=\"{REPO}/blob/main/{path}\">{name}</a><code class=\"pkg-import\">{path}</code>\
         <p>{}</p>{uses}</div>\n{}\n</div>\n</div>\n",
        markdown_inline(&program.summary),
        command(&program.run),
        name = program.name,
        path = program.path
    )
}

/// One guide's row: its title and summary, what its example proves behind a disclosure,
/// and the four files that run it.
///
/// # Arguments
///
/// * `title` - the capability's title.
/// * `summary` - the capability's one-line summary.
/// * `guide` - the guide's path under `docs/`.
/// * `text` - the guide's Markdown.
///
/// # Returns
///
/// The bullets under a guide's "It proves:" line.
///
/// # Arguments
///
/// * `text` - the guide's Markdown.
///
/// # Returns
///
/// The files a guide splices its examples from, one per language, in language order.
///
/// # Arguments
///
/// * `text` - the guide's Markdown.
///
/// # Returns
///
/// (path, language key, language name) per file, each file once.
/// The line that runs a guide's example in each language, and the file it runs.
///
/// # Arguments
///
/// * `text` - the guide's Markdown, whose snippet directives name the four files.
///
/// # Returns
///
/// A block of install-line rows, or an empty string when the guide splices no example.
pub fn run_block(text: &str) -> String {
    let files = runners(text);
    let Some(key) = files
        .iter()
        .find(|(path, _, _)| path.ends_with(".rs"))
        .and_then(|(path, _, _)| path.rsplit('/').next())
        .and_then(|name| name.strip_suffix(".rs"))
    else {
        return String::new();
    };
    // One block per language: the name and the copy button share a line, and the command sits
    // under them with the whole width, so nothing squeezes it and every block is the same
    // shape at every width.
    let mut out = String::from("<div class=\"run\">\n");
    for (_, kind, language) in &files {
        let line = match *kind {
            // One guide shares a name with an example that is not a guide, and cargo target
            // names are unique across the package, so that one carries a suffix.
            "rust" => match key {
                "telemetry" => "cargo run -p pamoja-examples --example telemetry_guide".to_owned(),
                _ => format!("cargo run -p pamoja-examples --example {key}"),
            },
            "node" => format!("npm --prefix bindings/node run guides -- {key}"),
            "python" => format!("python bindings/python/guides/{key}.py"),
            _ => format!("dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- {key}"),
        };
        out.push_str(&format!(
            "<div class=\"run-row\">\
             <p class=\"run-head\"><span class=\"run-lang\">{language}</span>\
             <button class=\"copy\" type=\"button\" data-copy=\"{}\" aria-label=\"Copy the command that runs the {language} example\">copy</button></p>\
             <code class=\"run-cmd\">{}</code></div>\n",
            escape(&line),
            escape(&line)
        ));
    }
    out.push_str("</div>");
    out
}

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
        let text = "# Modbus RTU\n\nIt proves:\n\n- A request is eight bytes: the address,\n  the code, and the checksum.\n- A reply validates its checksum.\n\n## Rust\n\n<!-- snippet: examples/guides/modbus.rs#example -->\n```rust\n```\n<!-- end -->\n\n<!-- snippet: examples/guides/modbus.rs#frame -->\n<!-- snippet: bindings/node/guides/modbus.ts#example -->\n<!-- snippet: bindings/python/guides/modbus.py#example -->\n<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs#example -->\n";
        let files = runners(text);
        assert_eq!(files.len(), 4);
        assert_eq!(files[0].0, "examples/guides/modbus.rs");
        assert_eq!(files[3].2, "C#");
        let run = run_block(text);
        assert!(
            run.starts_with("<div class=\"run\">\n<div class=\"run-row\"><p class=\"run-head\"><span class=\"run-lang\">Rust</span>"),
            "{run}"
        );
        assert!(
            run.contains("cargo run -p pamoja-examples --example modbus"),
            "{run}"
        );
        assert!(
            run.contains("npm --prefix bindings/node run guides -- modbus")
                && run.contains("python bindings/python/guides/modbus.py")
                && run.contains(
                    "dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- modbus"
                ),
            "{run}"
        );
        assert!(
            !run.contains("ModbusGuide.cs"),
            "the file each command runs is named by the listing right under the table"
        );
        assert!(run_block("no snippets here").is_empty());
    }

    #[test]
    fn a_shared_program_is_listed_from_its_own_directory() {
        let root = std::env::temp_dir().join(format!("pamoja-community-{}", std::process::id()));
        let dir = root.join("examples/community");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("hello_valve.rs"),
            "//! Opens a valve from a shared profile.\n//!\n//! Run with: `cargo run -p pamoja-examples --example hello_valve`\n\nfn main() {}\n",
        )
        .unwrap();
        let programs = programs_in(&root, "examples/community").unwrap();
        assert_eq!(programs.len(), 1);
        assert_eq!(programs[0].path, "examples/community/hello_valve.rs");
        let card = program_card(&programs[0], 1);
        assert!(card.contains("href=\"https://github.com/molexxxx/pamoja/blob/main/examples/community/hello_valve.rs\">hello_valve</a><code class=\"pkg-import\">examples/community/hello_valve.rs</code><p>Opens a valve from a shared profile.</p>"), "{card}");
        assert!(programs_in(&root, "examples/nowhere").unwrap().is_empty());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_page_lists_every_program() {
        let root = docs::repo_root();
        let page = table(&root).unwrap();
        assert!(page.starts_with("## Programs\n"));
        assert!(page.contains("<a class=\"pkg-title\" href=\"https://github.com/molexxxx/pamoja/blob/main/examples/batched_telemetry.rs\">batched_telemetry</a>"));
        assert!(page.contains("cargo run -p pamoja-examples --example batched_telemetry"));
        assert!(page.contains("## Community programs"));
        assert!(
            !page.contains("id=\"guide-modbus\""),
            "a guide's example is listed on the guide, not a second time here"
        );
    }
}
