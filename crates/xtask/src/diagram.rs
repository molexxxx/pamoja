//! The architecture drawing, `docs/assets/architecture.svg`, rendered from the capability
//! map so it names every chapter and crate the map does and cannot drift from them.
//!
//! The drawing answers one question: how a call in each language reaches a crate. The
//! three bindings sit over the compiled engine, which carries every capability; Rust
//! reaches the crates directly and compiles only the ones it names; every capability
//! crate that speaks the core's traits is marked as built on `pamoja-core`, read from its
//! manifest, and the rest are pure logic with no dependency at all. It paints its own
//! ground, so it holds on the site, on GitHub in either theme, and on a registry page. A
//! second file lays the same drawing out for a phone, where the wide one would shrink
//! past reading.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use toml_edit::DocumentMut;

use crate::catalog::{escape, Catalog};
use crate::theme::{rgba, PALETTE};

/// Where the drawing is written, relative to the repository root.
pub const PATH: &str = "docs/assets/architecture.svg";
/// The same drawing laid out for a narrow screen.
pub const NARROW_PATH: &str = "docs/assets/architecture-narrow.svg";

const SANS: &str = "Inter, 'Segoe UI', system-ui, -apple-system, sans-serif";
const MONO: &str = "'JetBrains Mono', Consolas, Menlo, monospace";

// The doors: each language, how its packages are named, and what carries a call, the
// last in a long and a short form.
const DOORS: [(&str, &str, &str, &str); 4] = [
    ("TypeScript", "@pamoja/<name>", "over napi-rs", "napi-rs"),
    ("Python", "pamoja-<name>", "over PyO3", "PyO3"),
    (
        "C#",
        "Pamoja.<Name>",
        "over cbindgen and P/Invoke",
        "cbindgen, P/Invoke",
    ),
    (
        "Rust",
        "pamoja-<name>",
        "over cargo, the crates themselves",
        "cargo, only the crates it names",
    ),
];
const GAP: f64 = 14.0;
const INSET: f64 = 14.0;
const BLOCK_HEAD: f64 = 34.0;
const ROW_H: f64 = 18.0;
const ARROW: f64 = 44.0;
const DOOR_H: f64 = 74.0;

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
    door_w: f64,
    door_gap: f64,
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
                door_w: (400.0 - 20.0) / 3.0,
                door_gap: 10.0,
                title: 13.0,
                body: 11.0,
                small: 10.0,
            }
        } else {
            Shape {
                width: 1004.0,
                margin: 24.0,
                columns: 5,
                door_w: 200.0,
                door_gap: 20.0,
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

fn draw(catalog: &Catalog, on_core: &BTreeSet<String>, narrow: bool) -> String {
    let p = &PALETTE;
    let s = Shape::of(narrow);
    let chapters: Vec<(&str, Vec<(String, bool)>)> = catalog
        .chapters
        .iter()
        .map(|chapter| {
            let names = catalog
                .in_chapter(&chapter.key)
                .flat_map(|capability| capability.crates.iter())
                .map(|krate| {
                    let name = krate.strip_prefix("pamoja-").unwrap_or(krate).to_owned();
                    (name, on_core.contains(krate))
                })
                .collect();
            (chapter.title.as_str(), names)
        })
        .collect();
    let longest = chapters
        .iter()
        .flat_map(|(_, names)| names.iter().map(|(name, _)| name.len()))
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
                .map(|(_, names)| names.len().div_ceil(chip_columns))
                .max()
                .unwrap_or(1)
                .max(1);
            30.0 + rows as f64 * ROW_H + 12.0
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

    let door_y = s.margin;
    let engine_y = door_y + DOOR_H + ARROW;
    let engine_h = if narrow { 88.0 } else { 64.0 };
    let block_y = if narrow {
        engine_y + engine_h + ARROW + DOOR_H + ARROW
    } else {
        engine_y + engine_h + ARROW
    };
    let grid_y = block_y + BLOCK_HEAD;
    let grid_h = row_heights.iter().sum::<f64>() + (grid_rows as f64 - 1.0) * GAP;
    let core_y = grid_y + grid_h + GAP;
    let core_h = if narrow { 92.0 } else { 76.0 };
    let block_h = BLOCK_HEAD + grid_h + GAP + core_h + INSET;
    let legend_y = block_y + block_h + 26.0;
    let height = legend_y + if narrow { 16.0 } else { 0.0 } + s.margin;

    let teal_fill = rgba(p.teal, 0.14);
    let teal_line = rgba(p.teal, 0.55);
    let amber_fill = rgba(p.amber, 0.14);
    let amber_line = rgba(p.amber, 0.55);
    let line = rgba(p.cream, 0.12);
    let mut out = String::new();
    out.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title">
  <title id="title">How a call reaches a crate: the three bindings over the compiled engine, Rust straight to the crates, every capability crate over pamoja-core.</title>
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

    // The three binding doors in a row, and the Rust door: beside them on a wide screen,
    // beside the engine's arrow on a phone.
    let rust_x = if narrow {
        s.margin + 150.0
    } else {
        s.width - s.margin - s.door_w
    };
    let rust_w = if narrow { 250.0 } else { s.door_w };
    let rust_y = if narrow {
        engine_y + engine_h + ARROW
    } else {
        door_y
    };
    for (index, (language, packages, long, short)) in DOORS.iter().enumerate() {
        let rust = index == 3;
        let (x, y, w) = if rust {
            (rust_x, rust_y, rust_w)
        } else {
            (
                s.margin + index as f64 * (s.door_w + s.door_gap),
                door_y,
                s.door_w,
            )
        };
        let (fill, stroke) = if rust {
            (&amber_fill, &amber_line)
        } else {
            (&teal_fill, &teal_line)
        };
        let bridge = if narrow { short } else { long };
        out.push_str(&format!(
            r##"  <rect x="{x}" y="{y}" width="{w}" height="{DOOR_H}" rx="10" fill="{fill}" stroke="{stroke}"/>
"##
        ));
        out.push_str(&text(
            x + 12.0,
            y + 25.0,
            SANS,
            s.title,
            600,
            p.cream,
            "",
            language,
        ));
        out.push_str(&text(
            x + 12.0,
            y + 45.0,
            MONO,
            s.body,
            400,
            p.text,
            "",
            packages,
        ));
        out.push_str(&text(
            x + 12.0,
            y + 62.0,
            SANS,
            s.small,
            400,
            p.muted,
            "",
            bridge,
        ));
    }

    // The bindings reach the engine; Rust reaches the crates.
    for index in 0..3 {
        let x = s.margin + index as f64 * (s.door_w + s.door_gap) + s.door_w / 2.0;
        out.push_str(&arrow(x, door_y + DOOR_H, engine_y, p.muted));
    }
    let rust_arrow_x = rust_x + rust_w / 2.0;
    out.push_str(&arrow(rust_arrow_x, rust_y + DOOR_H, block_y, p.muted));
    if !narrow {
        out.push_str(&text(
            rust_arrow_x - 10.0,
            engine_y + engine_h / 2.0 + 4.0,
            SANS,
            s.small,
            400,
            p.muted,
            "end",
            "compiles only the crates it names",
        ));
    }

    // The engine: one compiled library carrying every capability, under all three bindings.
    let engine_w = 3.0 * s.door_w + 2.0 * s.door_gap;
    out.push_str(&format!(
        r##"  <rect x="{x}" y="{engine_y}" width="{engine_w}" height="{engine_h}" rx="10" fill="url(#bridge)" stroke="{stroke}"/>
"##,
        x = s.margin,
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
        let rx = s.margin + engine_w - 12.0;
        out.push_str(&text(
            rx,
            engine_y + 25.0,
            MONO,
            s.body,
            400,
            p.muted,
            "end",
            "@pamoja/native",
        ));
        out.push_str(&text(
            rx,
            engine_y + 46.0,
            MONO,
            s.body,
            400,
            p.muted,
            "end",
            "pamoja-native, Pamoja.Native",
        ));
    }
    let engine_x = if narrow {
        s.margin + 70.0
    } else {
        s.margin + engine_w / 2.0
    };
    out.push_str(&arrow(engine_x, engine_y + engine_h, block_y, p.muted));
    if !narrow {
        out.push_str(&text(
            engine_x + 10.0,
            engine_y + engine_h + ARROW / 2.0 + 4.0,
            SANS,
            s.small,
            400,
            p.muted,
            "",
            "a package narrows the API, not the download",
        ));
    }

    // The capability crates as one block, a box per chapter with the crates it holds.
    out.push_str(&format!(
        r##"  <rect x="{x}" y="{block_y}" width="{w}" height="{block_h}" rx="12" fill="{fill}" stroke="{line}"/>
  <text x="{tx}" y="{ty}" font-family="{SANS}" font-size="12" font-weight="600" fill="{muted}" letter-spacing="1.2">CAPABILITY CRATES</text>
"##,
        x = s.margin,
        w = s.inner(),
        fill = rgba(p.cream, 0.03),
        tx = s.margin + INSET,
        ty = block_y + 22.0,
        muted = p.muted,
    ));
    let chip_w = (box_w - 28.0) / chip_columns as f64;
    for (index, (title, names)) in chapters.iter().enumerate() {
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
            title,
        ));
        for (slot, (name, on_core)) in names.iter().enumerate() {
            let cx = x + 14.0 + (slot % chip_columns) as f64 * chip_w;
            let cy = y + 42.0 + (slot / chip_columns) as f64 * ROW_H;
            if *on_core {
                out.push_str(&dot(cx - 8.0, cy - 4.0, p.amber));
            }
            out.push_str(&text(cx, cy, MONO, 11.5, 400, p.text, "", name));
        }
    }

    // The core, the foundation of the block: the traits, and which crates build on it.
    let core_x = s.margin + INSET;
    let core_w = s.inner() - 2.0 * INSET;
    out.push_str(&format!(
        r##"  <rect x="{core_x}" y="{core_y}" width="{core_w}" height="{core_h}" rx="10" fill="{amber_fill}" stroke="{amber_line}"/>
"##
    ));
    let tx = core_x + 14.0;
    let key_y = core_y + core_h - 14.0;
    let marked = "marks a crate built on it; the rest depend on nothing at all.";
    out.push_str(&dot(tx + 2.0, key_y - 4.0, p.amber));
    out.push_str(&text(
        tx + 12.0,
        key_y,
        SANS,
        s.small,
        400,
        p.muted,
        "",
        marked,
    ));
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
            s.margin,
            legend_y,
            SANS,
            s.small,
            400,
            p.muted,
            "",
            "Every name in a chapter is a crate, pamoja-<name>, and the same",
        ));
        out.push_str(&text(
            s.margin,
            legend_y + 16.0,
            SANS,
            s.small,
            400,
            p.muted,
            "",
            "capability is a package on npm, PyPI, and NuGet.",
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
            s.margin,
            legend_y,
            SANS,
            s.small,
            400,
            p.muted,
            "",
            "Every name in a chapter is a crate, pamoja-<name>, and the same capability is a package on npm, PyPI, and NuGet.",
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

// The mark on a crate that builds on the core.
fn dot(x: f64, y: f64, colour: &str) -> String {
    format!("  <circle cx=\"{x}\" cy=\"{y}\" r=\"2.5\" fill=\"{colour}\"/>\n")
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
    fn every_chapter_and_capability_crate_is_drawn_in_both_layouts() {
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
    fn the_drawing_paints_its_ground_in_the_palette_and_names_every_door() {
        let catalog = Catalog::load(&repo_root()).unwrap();
        for (_, drawing) in render(&catalog, &repo_root()).unwrap() {
            assert!(drawing.starts_with("<svg "));
            assert!(drawing.contains(&format!("fill=\"{}\"", PALETTE.navy_0)));
            for (language, packages, _, _) in DOORS {
                assert!(drawing.contains(&format!(">{language}<")));
                assert!(drawing.contains(&escape(packages)));
            }
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
    fn the_crates_built_on_the_core_are_read_from_their_manifests() {
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
        let marks = draw(&catalog, &on_core, false).matches("<circle").count();
        assert_eq!(
            marks,
            on_core.len() + 1,
            "one dot per crate on the core, plus the key"
        );
    }
}
