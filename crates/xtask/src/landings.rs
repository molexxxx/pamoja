//! The front page of each binding's generated reference.
//!
//! typedoc, pdoc, and DocFX each open on a page of their own, and by default that page
//! is either empty or a copy of the registry README, whose links point back at the page
//! the reader is already on. This renders one landing per binding instead: the mark, what
//! to install, the example that runs in CI, and every package linked into the reference
//! around it. The three are generated from the same description so they say the same
//! thing, in the markup each generator takes.

use crate::catalog::{node_package, Capability, Catalog};
use crate::regions;
use std::path::Path;

/// The site the guides are published at, reached from inside a reference.
const SITE: &str = "https://pamoja.molex.cloud/docs";

/// One binding's landing page.
struct Landing {
    /// The language as a reader names it.
    language: &'static str,
    /// What the reader types to install everything.
    install: &'static str,
    /// The language of the install fence.
    install_lang: &'static str,
    /// A second line, where the language needs an import to be useful.
    import: Option<&'static str>,
    /// The language of the example fence.
    example_lang: &'static str,
    /// The example file and anchor, spliced from the test that runs it.
    example: &'static str,
    /// What a package is called here, as a column heading.
    heading: &'static str,
    /// Whether the page opens with its own title, or the generator supplies one.
    titled: bool,
    /// The name of a capability's package, and where its page is in this reference.
    package: fn(&Capability) -> (String, String),
}

/// Render the landing page of every binding's reference.
///
/// # Arguments
///
/// * `root` - the repository root, for reading the example each landing splices.
/// * `catalog` - the capability map, which supplies the package table.
///
/// # Returns
///
/// One entry per landing, as (path, contents).
///
/// # Errors
///
/// If an example file or its anchor is missing.
pub fn render(root: &Path, catalog: &Catalog) -> Result<Vec<(String, String)>, String> {
    let node = Landing {
        language: "TypeScript",
        titled: false,
        install: "npm install pamoja",
        install_lang: "sh",
        import: None,
        example_lang: "typescript",
        example: "bindings/node/guides/quickstart.ts#example",
        heading: "Import",
        // Absolute, because typedoc treats a relative link in its front page as a file
        // to copy and warns once per row when it cannot find one.
        package: |capability| {
            (
                node_package(capability),
                format!(
                    "{SITE}/reference/node/modules/_pamoja_{}.html",
                    capability.node
                ),
            )
        },
    };
    let python = Landing {
        language: "Python",
        titled: true,
        install: "pip install pamoja",
        install_lang: "sh",
        import: Some("from pamoja import mqtt, security"),
        example_lang: "python",
        example: "bindings/python/guides/quickstart.py#example",
        heading: "Module",
        package: |capability| {
            (
                format!("pamoja.{}", capability.python),
                format!("pamoja/{}.html", capability.python),
            )
        },
    };
    let dotnet = Landing {
        language: "C#",
        titled: true,
        install: "dotnet add package Pamoja",
        install_lang: "sh",
        import: Some("using Pamoja.Mqtt;"),
        example_lang: "csharp",
        example: "bindings/dotnet/samples/Pamoja.Guides/Quickstart.cs#example",
        heading: "Package",
        package: |capability| {
            let package = capability.dotnet_package();
            let href = format!("api/{package}.html");
            (package, href)
        },
    };

    Ok(vec![
        (
            "bindings/node/docs/index.md".to_owned(),
            markdown(root, catalog, &node)?,
        ),
        (
            "bindings/dotnet/docs/index.md".to_owned(),
            markdown(root, catalog, &dotnet)?,
        ),
        (
            "docs/theme/pdoc/landing.html".to_owned(),
            html(root, catalog, &python)?,
        ),
    ])
}

// The opening block: the language and one sentence. Each generator carries the mark in
// its own chrome, so the page does not repeat it.
fn header(landing: &Landing) -> String {
    let title = if landing.titled {
        format!(
            "# pamoja for {}

",
            landing.language
        )
    } else {
        String::new()
    };
    format!(
        "{title}One memory-safe Rust core for IoT, robotics, and drones, behind an idiomatic {0}
facade. This is the generated reference for every package; the guides, with the same
worked example in four languages, are on [the documentation site]({SITE}/).",
        landing.language
    )
}

// The same opening, as HTML, for the generator that renders a template rather than a page.
fn header_html(landing: &Landing) -> String {
    format!(
        "<h1>pamoja for {0}</h1>

<p>One memory-safe Rust core for IoT, robotics, and drones, behind an idiomatic {0}
facade. This is the generated reference for every package; the guides, with the same
worked example in four languages, are on <a href=\"{SITE}/\">the documentation site</a>.</p>",
        landing.language
    )
}

// The links out, which are the same three from every landing.
fn footer(landing: &Landing) -> Vec<(String, String)> {
    vec![
        (
            format!("{SITE}/"),
            format!(
                "The guides: one page per capability, each with a worked {} example",
                landing.language
            ),
        ),
        (
            format!("{SITE}/install.html"),
            "The install page: taking less than all of it, and what that saves".to_owned(),
        ),
        (
            "https://github.com/molexxxx/pamoja".to_owned(),
            "The source, and every other binding".to_owned(),
        ),
    ]
}

// The landing as Markdown, for the two generators that take a Markdown front page.
fn markdown(root: &Path, catalog: &Catalog, landing: &Landing) -> Result<String, String> {
    let mut out = format!("{}\n\n## Install\n\n", header(landing));
    out.push_str(&format!(
        "```{}\n{}\n```\n",
        landing.install_lang, landing.install
    ));
    if let Some(import) = landing.import {
        out.push_str(&format!("\n```{}\n{import}\n```\n", landing.example_lang));
    }
    out.push_str(
        "\nEach capability is also its own package, so an application that needs one \
         thing depends on one thing.\n\n## A first example\n\nA reading taken off a wire \
         on a field node, sent over a link, and checked on the gateway that receives it, \
         with nothing plugged in and nothing running. This runs in CI, and is spliced \
         here from the test that runs it.\n\n",
    );
    out.push_str(&regions::snippet(root, landing.example)?);
    out.push_str("\n\n## Every package\n\n");
    out.push_str(&format!(
        "| Chapter | {} | What it covers |\n| --- | --- | --- |\n",
        landing.heading
    ));
    for (chapter, capability) in rows(catalog) {
        let (name, href) = (landing.package)(capability);
        out.push_str(&format!(
            "| {chapter} | [`{name}`]({href}) | {} |\n",
            capability.summary
        ));
    }
    out.push_str("\n## Elsewhere\n\n");
    for (href, what) in footer(landing) {
        out.push_str(&format!("- [{what}]({href})\n"));
    }
    Ok(out)
}

// The landing as an HTML fragment, for pdoc, which renders a template rather than a page.
fn html(root: &Path, catalog: &Catalog, landing: &Landing) -> Result<String, String> {
    let example = regions::snippet(root, landing.example)?;
    let code = example
        .lines()
        .skip_while(|line| !line.starts_with("```"))
        .skip(1)
        .take_while(|line| !line.starts_with("```"))
        .map(escape)
        .collect::<Vec<String>>()
        .join("\n");

    let mut out = format!(
        "<!-- Generated by `cargo xtask docs`; edit crates/xtask/src/landings.rs. -->
<section class=\"pamoja-landing\">
{}

<h2>Install</h2>

<pre><code>{}</code></pre>
",
        header_html(landing),
        escape(landing.install)
    );
    if let Some(import) = landing.import {
        out.push_str(&format!("\n<pre><code>{}</code></pre>\n", escape(import)));
    }
    out.push_str(
        "\n<p>Each capability is also its own distribution, so an application that needs \
         one thing depends on one thing.</p>\n\n<h2>A first example</h2>\n\n<p>A reading \
         taken off a wire on a field node, sent over a link, and checked on the gateway \
         that receives it, with nothing plugged in and nothing running. This runs in CI, \
         and is spliced here from the test that runs it.</p>\n\n",
    );
    out.push_str(&format!(
        "<pre><code>{code}</code></pre>\n\n<h2>Every package</h2>\n\n"
    ));
    out.push_str(&format!(
        "<table>\n<thead><tr><th>Chapter</th><th>{}</th><th>What it covers</th></tr></thead>\n<tbody>\n",
        landing.heading
    ));
    for (chapter, capability) in rows(catalog) {
        let (name, href) = (landing.package)(capability);
        // The chapter is emphasised with Markdown in the table the other two render.
        let chapter = match chapter.trim_matches('*') {
            "" => String::new(),
            named => format!("<strong>{named}</strong>"),
        };
        out.push_str(&format!(
            "<tr><td>{chapter}</td><td><a href=\"{href}\"><code>{name}</code></a></td><td>{}</td></tr>\n",
            escape(&capability.summary)
        ));
    }
    out.push_str("</tbody>\n</table>\n\n<h2>Elsewhere</h2>\n\n<ul>\n");
    for (href, what) in footer(landing) {
        out.push_str(&format!("<li><a href=\"{href}\">{what}</a></li>\n"));
    }
    out.push_str("</ul>\n</section>\n");
    Ok(out)
}

// Every capability with the chapter it belongs to, named once per group so the table
// reads as a handful of domains rather than thirty flat rows.
fn rows(catalog: &Catalog) -> Vec<(String, &Capability)> {
    let mut out = Vec::new();
    let mut last = "";
    for capability in catalog.ordered() {
        let chapter = if capability.node == "core" {
            "**Engine**".to_owned()
        } else if capability.chapter == last {
            String::new()
        } else {
            last = &capability.chapter;
            catalog
                .chapters
                .iter()
                .find(|chapter| chapter.key == capability.chapter)
                .map(|chapter| format!("**{}**", chapter.title))
                .unwrap_or_default()
        };
        out.push((chapter, capability));
    }
    out
}

// Markdown emphasis has no meaning in the HTML fragment, so the chapter is bold there.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
