//! The architecture drawing, `docs/assets/architecture.svg`, rendered from the capability
//! map so it names every chapter and crate the map does and cannot drift from them.
//!
//! The drawing answers one question: how a call reaches a crate. The three bindings sit
//! over the compiled engine, which carries every capability; a Rust program links the
//! crates themselves, which is written on the block of crates rather than drawn as a
//! fourth door. The block groups the capabilities by chapter; each box names its crates,
//! sets the ones that build on `pamoja-core`, read from their manifests, in amber over
//! the core itself, and ends with the package that installs the chapter on each registry.
//! It paints its own ground, so it holds on the site, on GitHub in either theme, and on
//! a registry page. A second file lays the same drawing out for a phone, where the wide
//! one would shrink past reading.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use toml_edit::DocumentMut;

use crate::catalog::{escape, Catalog, Language};
use crate::packages::pascal;
use crate::theme::{rgba, PALETTE};

/// Where the drawing is written, relative to the repository root.
pub const PATH: &str = "docs/assets/architecture.svg";
/// The same drawing laid out for a narrow screen.
pub const NARROW_PATH: &str = "docs/assets/architecture-narrow.svg";

const SANS: &str = "Inter, 'Segoe UI', system-ui, -apple-system, sans-serif";
const MONO: &str = "'JetBrains Mono', Consolas, Menlo, monospace";

// The three bindings: the language, how its packages are named, and what carries a
// call, the last in a long and a short form.
const DOORS: [(&str, &str, &str, &str); 3] = [
    ("TypeScript", "@pamoja/<name>", "over napi-rs", "napi-rs"),
    ("Python", "pamoja-<name>", "over PyO3", "PyO3"),
    (
        "C#",
        "Pamoja.<Name>",
        "over cbindgen and P/Invoke",
        "cbindgen, P/Invoke",
    ),
];
// What a Rust program does instead: it links the crates themselves.
const RUST: &str = "Rust: cargo add pamoja-<name>, the crates themselves";
const GAP: f64 = 14.0;
const INSET: f64 = 14.0;
const ROW_H: f64 = 18.0;
const ARROW: f64 = 44.0;
const DOOR_H: f64 = 74.0;
// Where a box's crate names start, under its title, and the room its four package
// names take at the foot: a hairline, four lines, and the padding under them.
const CHIPS_Y: f64 = 42.0;
const NAME_H: f64 = 14.0;
const NAMES_H: f64 = 10.0 + 4.0 * NAME_H + 8.0;

/// Render the drawing as (path, contents), once for a wide screen and once for a phone.
///
/// # Arguments
///
/// * `catalog` - the capability map; every chapter becomes a box and every capability
///   crate a name inside it.
/// * `root` - the repository root, whose crate manifests say which crates build on the
///   core.
///
/// # Returns
///
/// `docs/assets/architecture.svg` and `docs/assets/architecture-narrow.svg` with their
/// contents.
///
/// # Errors
///
/// When a capability crate's manifest cannot be read or parsed.
pub fn render(catalog: &Catalog, root: &Path) -> Result<Vec<(String, String)>, String> {
    let on_core = built_on_core(catalog, root)?;
    Ok(vec![
        (PATH.to_owned(), draw(catalog, &on_core, false)),
        (NARROW_PATH.to_owned(), draw(catalog, &on_core, true)),
    ])
}

// The capability crates whose manifest names `pamoja-core` as a dependency, in either
// of the forms Cargo accepts.
fn built_on_core(catalog: &Catalog, root: &Path) -> Result<BTreeSet<String>, String> {
    let mut out = BTreeSet::new();
    for krate in catalog.capabilities.iter().flat_map(|c| c.crates.iter()) {
        let path = root.join("crates").join(krate).join("Cargo.toml");
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        let manifest: DocumentMut = text
            .parse()
            .map_err(|err| format!("parsing {}: {err}", path.display()))?;
        let depends = manifest
            .get("dependencies")
            .and_then(|deps| deps.as_table_like())
            .is_some_and(|deps| deps.contains_key("pamoja-core"));
        if depends {
            out.insert(krate.clone());
        }
    }
    Ok(out)
}

// The measurements that differ between the two layouts.
struct Shape {
    width: f64,
    margin: f64,
    columns: usize,
    title: f64,
    body: f64,
    small: f64,
}

impl Shape {
    fn of(narrow: bool) -> Shape {
        if narrow {
            Shape {
                width: 440.0,
                margin: 20.0,
                columns: 2,
                title: 13.0,
                body: 11.0,
                small: 10.0,
            }
        } else {
            Shape {
                width: 1004.0,
                margin: 24.0,
                columns: 5,
                title: 15.0,
                body: 12.0,
                small: 11.0,
            }
        }
    }

    fn inner(&self) -> f64 {
        self.width - 2.0 * self.margin
    }
}

// One chapter as drawn: its title, its crates, each with whether it builds on the core,
// and the package that installs the chapter on npm, PyPI, NuGet, and as a feature of the
// pamoja crate.
struct Chapter<'a> {
    title: &'a str,
    crates: Vec<(String, bool)>,
    names: [String; 4],
}

fn draw(catalog: &Catalog, on_core: &BTreeSet<String>, narrow: bool) -> String {
    let p = &PALETTE;
    let s = Shape::of(narrow);
    let domains: BTreeSet<&str> = catalog
        .domains()
        .into_iter()
        .map(|(chapter, _)| chapter.key.as_str())
        .collect();
    let chapters: Vec<Chapter> = catalog
        .chapters
        .iter()
        .map(|chapter| {
            // A domain installs by the chapter's key; a chapter of one capability installs
            // as that capability's own package.
            let names = if domains.contains(chapter.key.as_str()) {
                let key = &chapter.key;
                [
                    format!("@pamoja/{key}"),
                    format!("pamoja-{key}"),
                    format!("Pamoja.{}", pascal(key)),
                    format!("pamoja -F {key}"),
                ]
            } else {
                let own = catalog
                    .in_chapter(&chapter.key)
                    .find(|capability| !capability.crates.is_empty())
                    .expect("a chapter holds a crate");
                [
                    Language::by_key("node").package(own),
                    Language::by_key("python").package(own),
                    Language::by_key("dotnet").package(own),
                    format!("pamoja -F {}", own.key),
                ]
            };
            Chapter {
                title: &chapter.title,
                crates: catalog
                    .in_chapter(&chapter.key)
                    .flat_map(|capability| capability.crates.iter())
                    .map(|krate| {
                        let name = krate.strip_prefix("pamoja-").unwrap_or(krate).to_owned();
                        (name, on_core.contains(krate))
                    })
                    .collect(),
                names,
            }
        })
        .collect();
    let longest = chapters
        .iter()
        .flat_map(|c| c.crates.iter().map(|(name, _)| name.len()))
        .max()
        .unwrap_or(0);
    let chip_columns = if longest > 10 { 1 } else { 2 };
    // Each row of boxes is as tall as its fullest box, so a row of one-crate chapters
    // does not carry the height of the transports.
    let row_heights: Vec<f64> = chapters
        .chunks(s.columns)
        .map(|row| {
            let rows = row
                .iter()
                .map(|c| c.crates.len().div_ceil(chip_columns))
                .max()
                .unwrap_or(1)
                .max(1);
            CHIPS_Y + rows as f64 * ROW_H + NAMES_H
        })
        .collect();
    let row_tops: Vec<f64> = row_heights
        .iter()
        .scan(0.0, |top, height| {
            let at = *top;
            *top += height + GAP;
            Some(at)
        })
        .collect();
    let grid_rows = row_heights.len().max(1);
    let gaps = (s.columns as f64 - 1.0) * GAP;
    let box_w = (s.inner() - 2.0 * INSET - gaps) / s.columns as f64;

    let door_gap = if narrow { 10.0 } else { 20.0 };
    let door_w = (s.inner() - 2.0 * door_gap) / 3.0;
    let door_y = s.margin;
    let engine_y = door_y + DOOR_H + ARROW;
    let engine_h = if narrow { 88.0 } else { 64.0 };
    let block_y = engine_y + engine_h + ARROW;
    let block_head = if narrow { 56.0 } else { 36.0 };
    let grid_y = block_y + block_head;
    let grid_h = row_heights.iter().sum::<f64>() + (grid_rows as f64 - 1.0) * GAP;
    let core_y = grid_y + grid_h + GAP;
    let core_h = if narrow { 92.0 } else { 76.0 };
    let block_h = block_head + grid_h + GAP + core_h + INSET;
    let foot_y = block_y + block_h + 26.0;
    let foot_lines = if narrow { 3.0 } else { 1.0 };
    let height = foot_y + (foot_lines - 1.0) * 16.0 + s.margin;

    let teal_fill = rgba(p.teal, 0.14);
    let teal_line = rgba(p.teal, 0.55);
    let amber_fill = rgba(p.amber, 0.14);
    let amber_line = rgba(p.amber, 0.55);
    let line = rgba(p.cream, 0.12);
    let mut out = String::new();
    out.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title">
  <title id="title">How a call reaches a crate: the three bindings over the compiled engine, a Rust program straight to the crates, every capability by chapter, the crates that build on pamoja-core, and the three ways to install.</title>
  <defs>
    <linearGradient id="bridge" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="{teal}" stop-opacity="0.16"/>
      <stop offset="1" stop-color="{amber}" stop-opacity="0.16"/>
    </linearGradient>
    <marker id="head" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="8" markerHeight="8" orient="auto">
      <path d="M0,0.5 L7,4 L0,7.5" fill="none" stroke="{muted}" stroke-width="1.5"/>
    </marker>
  </defs>
  <rect width="{width}" height="{height}" rx="16" fill="{navy}"/>
"##,
        width = s.width,
        teal = p.teal,
        amber = p.amber,
        muted = p.muted,
        navy = p.navy_0,
    ));

    // The three bindings, each reaching the engine.
    for (index, (language, packages, long, short)) in DOORS.iter().enumerate() {
        let x = s.margin + index as f64 * (door_w + door_gap);
        let bridge = if narrow { short } else { long };
        out.push_str(&format!(
            r##"  <rect x="{x}" y="{door_y}" width="{door_w}" height="{DOOR_H}" rx="10" fill="{teal_fill}" stroke="{teal_line}"/>
"##
        ));
        let tx = x + 12.0;
        out.push_str(&text(
            tx,
            door_y + 25.0,
            SANS,
            s.title,
            600,
            p.cream,
            "",
            language,
        ));
        out.push_str(&text(
            tx,
            door_y + 45.0,
            MONO,
            s.body,
            400,
            p.text,
            "",
            packages,
        ));
        out.push_str(&text(
            tx,
            door_y + 62.0,
            SANS,
            s.small,
            400,
            p.muted,
            "",
            bridge,
        ));
        out.push_str(&arrow(x + door_w / 2.0, door_y + DOOR_H, engine_y, p.muted));
    }

    // The engine: one compiled library carrying every capability, under all three.
    out.push_str(&format!(
        r##"  <rect x="{x}" y="{engine_y}" width="{w}" height="{engine_h}" rx="10" fill="url(#bridge)" stroke="{stroke}"/>
"##,
        x = s.margin,
        w = s.inner(),
        stroke = rgba(p.cream, 0.2),
    ));
    let tx = s.margin + 12.0;
    out.push_str(&text(
        tx,
        engine_y + 25.0,
        SANS,
        s.title,
        600,
        p.cream,
        "",
        "Compiled engine",
    ));
    out.push_str(&text(
        tx,
        engine_y + 46.0,
        SANS,
        s.body,
        400,
        p.text,
        "",
        "pamoja-ffi over the C ABI: one library carrying every capability",
    ));
    if narrow {
        out.push_str(&text(
            tx,
            engine_y + 64.0,
            MONO,
            s.small,
            400,
            p.muted,
            "",
            "@pamoja/native, pamoja-native, Pamoja.Native",
        ));
        out.push_str(&text(
            tx,
            engine_y + 80.0,
            SANS,
            s.small,
            400,
            p.muted,
            "",
            "A package narrows the API, not the download.",
        ));
    } else {
        let rx = s.width - s.margin - 12.0;
        out.push_str(&text(
            rx,
            engine_y + 25.0,
            MONO,
            s.body,
            400,
            p.muted,
            "end",
            "@pamoja/native, pamoja-native, Pamoja.Native",
        ));
        out.push_str(&text(
            rx,
            engine_y + 46.0,
            SANS,
            s.small,
            400,
            p.muted,
            "end",
            "a package narrows the API, not the download",
        ));
    }
    out.push_str(&arrow(s.width / 2.0, engine_y + engine_h, block_y, p.muted));

    // The block of crates: its head names the chapters and says what a Rust program
    // does, which is link these directly.
    out.push_str(&format!(
        r##"  <rect x="{x}" y="{block_y}" width="{w}" height="{block_h}" rx="12" fill="{fill}" stroke="{line}"/>
  <text x="{tx}" y="{ty}" font-family="{SANS}" font-size="12" font-weight="600" fill="{muted}" letter-spacing="1.2">CAPABILITIES BY CHAPTER</text>
"##,
        x = s.margin,
        w = s.inner(),
        fill = rgba(p.cream, 0.03),
        tx = s.margin + INSET,
        ty = block_y + 23.0,
        muted = p.muted,
    ));
    if narrow {
        out.push_str(&text(
            s.margin + INSET,
            block_y + 43.0,
            MONO,
            s.small,
            500,
            p.amber,
            "",
            RUST,
        ));
    } else {
        out.push_str(&text(
            s.width - s.margin - INSET,
            block_y + 23.0,
            MONO,
            s.small,
            500,
            p.amber,
            "end",
            RUST,
        ));
    }
    let chip_w = (box_w - 28.0) / chip_columns as f64;
    for (index, chapter) in chapters.iter().enumerate() {
        let column = index % s.columns;
        let row = index / s.columns;
        let x = s.margin + INSET + column as f64 * (box_w + GAP);
        let y = grid_y + row_tops[row];
        let box_h = row_heights[row];
        out.push_str(&format!(
            r##"  <rect x="{x}" y="{y}" width="{box_w}" height="{box_h}" rx="10" fill="{fill}" stroke="{line}"/>
"##,
            fill = p.navy_2,
        ));
        out.push_str(&text(
            x + 14.0,
            y + 22.0,
            SANS,
            13.0,
            600,
            p.cream,
            "",
            chapter.title,
        ));
        for (slot, (name, on_core)) in chapter.crates.iter().enumerate() {
            let cx = x + 14.0 + (slot % chip_columns) as f64 * chip_w;
            let cy = y + CHIPS_Y + (slot / chip_columns) as f64 * ROW_H;
            let colour = if *on_core { p.amber } else { p.text };
            out.push_str(&text(cx, cy, MONO, 11.5, 500, colour, "", name));
        }
        // The chapter's package on each registry, under a hairline.
        let rule_y = y + box_h - NAMES_H + 4.0;
        out.push_str(&format!(
            r##"  <line x1="{x1}" y1="{rule_y}" x2="{x2}" y2="{rule_y}" stroke="{line}"/>
"##,
            x1 = x + 14.0,
            x2 = x + box_w - 14.0,
        ));
        for (slot, name) in chapter.names.iter().enumerate() {
            let ny = rule_y + 16.0 + slot as f64 * NAME_H;
            out.push_str(&text(x + 14.0, ny, MONO, 10.0, 400, p.muted, "", name));
        }
    }

    // The core, the foundation of the block: the traits, and the key to the amber names.
    let core_x = s.margin + INSET;
    let core_w = s.inner() - 2.0 * INSET;
    out.push_str(&format!(
        r##"  <rect x="{core_x}" y="{core_y}" width="{core_w}" height="{core_h}" rx="10" fill="{amber_fill}" stroke="{amber_line}"/>
"##
    ));
    let tx = core_x + 14.0;
    out.push_str(&text(
        tx,
        core_y + 25.0,
        MONO,
        s.title,
        600,
        p.cream,
        "",
        "pamoja-core",
    ));
    let key = "The names in amber build on it; the rest are pure logic with no dependency.";
    if narrow {
        out.push_str(&text(
            tx,
            core_y + 46.0,
            SANS,
            s.body,
            400,
            p.text,
            "",
            "Transport, Device, Sensor, Actuator, Store, and the event bus",
        ));
        out.push_str(&text(
            tx,
            core_y + 63.0,
            SANS,
            s.small,
            400,
            p.muted,
            "",
            "no_std, so it runs on a microcontroller",
        ));
        out.push_str(&text(
            tx,
            core_y + 80.0,
            SANS,
            s.small,
            400,
            p.amber,
            "",
            key,
        ));
    } else {
        out.push_str(&text(
            tx,
            core_y + 46.0,
            SANS,
            s.body,
            400,
            p.text,
            "",
            "Transport, Device, Sensor, Actuator, Store, and the event bus; no_std, so it runs on a microcontroller",
        ));
        out.push_str(&text(
            tx,
            core_y + 64.0,
            SANS,
            s.small,
            400,
            p.amber,
            "",
            key,
        ));
    }

    // Under the block: the install that takes everything.
    let foot: Vec<String> = if narrow {
        vec![
            "A chapter's package brings its capabilities with it. Everything".to_owned(),
            "at once: npm install pamoja, pip install pamoja,".to_owned(),
            "dotnet add package Pamoja, or cargo add pamoja.".to_owned(),
        ]
    } else {
        vec![
            "A chapter's package brings its capabilities with it. Everything at once: npm install pamoja, pip install pamoja, dotnet add package Pamoja, or cargo add pamoja.".to_owned(),
        ]
    };
    for (index, line) in foot.iter().enumerate() {
        out.push_str(&text(
            s.margin,
            foot_y + index as f64 * 16.0,
            SANS,
            s.small,
            400,
            p.muted,
            "",
            line,
        ));
    }
    out.push_str("</svg>\n");
    out
}

// One line of text. `anchor` is empty for a left-aligned line and "end" for one that
// ends at `x`.
#[allow(clippy::too_many_arguments)]
fn text(
    x: f64,
    y: f64,
    family: &str,
    size: f64,
    weight: u16,
    fill: &str,
    anchor: &str,
    body: &str,
) -> String {
    let anchor = if anchor.is_empty() {
        String::new()
    } else {
        format!(" text-anchor=\"{anchor}\"")
    };
    format!(
        "  <text x=\"{x}\" y=\"{y}\" font-family=\"{family}\" font-size=\"{size}\" font-weight=\"{weight}\" fill=\"{fill}\"{anchor}>{}</text>\n",
        escape(body)
    )
}

// A vertical arrow from `top` to `bottom` at `x`, headed at the bottom.
fn arrow(x: f64, top: f64, bottom: f64, colour: &str) -> String {
    format!(
        r##"  <line x1="{x}" y1="{top}" x2="{x}" y2="{bottom}" stroke="{colour}" stroke-width="1.5" marker-end="url(#head)"/>
"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::repo_root;

    #[test]
    fn every_chapter_domain_and_capability_crate_is_drawn_in_both_layouts() {
        let catalog = Catalog::load(&repo_root()).unwrap();
        let files = render(&catalog, &repo_root()).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, PATH);
        assert_eq!(files[1].0, NARROW_PATH);
        for (_, drawing) in &files {
            for chapter in &catalog.chapters {
                assert!(
                    drawing.contains(&format!(">{}<", escape(&chapter.title))),
                    "{} is missing",
                    chapter.title
                );
            }
            for (chapter, _) in catalog.domains() {
                for name in [
                    format!(">@pamoja/{}<", chapter.key),
                    format!(">Pamoja.{}<", pascal(&chapter.key)),
                    format!(">pamoja -F {}<", chapter.key),
                ] {
                    assert!(drawing.contains(&name), "{name} is missing");
                }
            }
            assert!(
                drawing.contains(">@pamoja/security<"),
                "a lone chapter installs as its capability"
            );
            assert!(drawing.contains(">pamoja -F security<"));
            for capability in &catalog.capabilities {
                for krate in &capability.crates {
                    let name = krate.strip_prefix("pamoja-").unwrap();
                    assert!(drawing.contains(&format!(">{name}<")), "{krate} is missing");
                }
            }
            for engine in ["pamoja-core", "pamoja-ffi"] {
                assert!(catalog.engine.iter().any(|krate| krate == engine));
                assert!(drawing.contains(engine));
            }
        }
    }

    #[test]
    fn the_drawing_paints_its_ground_in_the_palette_and_names_every_language() {
        let catalog = Catalog::load(&repo_root()).unwrap();
        for (_, drawing) in render(&catalog, &repo_root()).unwrap() {
            assert!(drawing.starts_with("<svg "));
            assert!(drawing.contains(&format!("fill=\"{}\"", PALETTE.navy_0)));
            for (language, packages, _, _) in DOORS {
                assert!(drawing.contains(&format!(">{language}<")));
                assert!(drawing.contains(&escape(packages)));
            }
            assert!(drawing.contains(&escape(RUST)));
            assert!(drawing.contains("dotnet add package Pamoja"));
            assert!(!drawing.contains("<name>"), "the placeholders are escaped");
        }
    }

    #[test]
    fn the_narrow_layout_is_a_phone_wide_column() {
        let catalog = Catalog::load(&repo_root()).unwrap();
        let on_core = built_on_core(&catalog, &repo_root()).unwrap();
        let narrow = draw(&catalog, &on_core, true);
        let wide = draw(&catalog, &on_core, false);
        assert!(narrow.contains("width=\"440\""));
        assert!(wide.contains("width=\"1004\""));
        assert!(narrow.len() > wide.len() / 2);
    }

    #[test]
    fn the_crates_built_on_the_core_are_read_from_their_manifests_and_set_in_amber() {
        let catalog = Catalog::load(&repo_root()).unwrap();
        let on_core = built_on_core(&catalog, &repo_root()).unwrap();
        assert!(
            on_core.contains("pamoja-mqtt"),
            "a transport implements the core's trait"
        );
        assert!(on_core.contains("pamoja-security"));
        assert!(
            !on_core.contains("pamoja-modbus"),
            "a pure codec depends on nothing"
        );
        assert!(!on_core.contains("pamoja-sensors"));
        let drawing = draw(&catalog, &on_core, false);
        let amber = format!("fill=\"{}\">mqtt<", PALETTE.amber);
        let plain = format!("fill=\"{}\">modbus<", PALETTE.text);
        assert!(drawing.contains(&amber));
        assert!(drawing.contains(&plain));
    }
}
