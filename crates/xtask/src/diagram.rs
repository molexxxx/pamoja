//! The architecture drawing, `docs/assets/architecture.svg`, rendered from the capability
//! map so it names every chapter and crate the map does and cannot drift from them.
//!
//! The drawing answers one question: how a call in each language reaches a crate. The
//! three bindings sit over the compiled engine, which carries every capability; Rust
//! reaches the crates directly and compiles only the ones it names; every capability
//! crate stands on `pamoja-core`. It paints its own ground, so it holds on the site, on
//! GitHub in either theme, and on a registry page.

use crate::catalog::{escape, Catalog};
use crate::theme::{rgba, PALETTE};

/// Where the drawing is written, relative to the repository root.
pub const PATH: &str = "docs/assets/architecture.svg";

const WIDTH: f64 = 1004.0;
const MARGIN: f64 = 24.0;
const GAP: f64 = 14.0;
const SANS: &str = "Inter, 'Segoe UI', system-ui, -apple-system, sans-serif";
const MONO: &str = "'JetBrains Mono', Consolas, Menlo, monospace";

// The doors: each language, how its packages are named, and what carries a call.
const DOORS: [(&str, &str, &str); 4] = [
    ("TypeScript", "@pamoja/<name>", "napi-rs"),
    ("Python", "pamoja-<name>", "PyO3"),
    ("C#", "Pamoja.<Name>", "cbindgen and P/Invoke"),
    ("Rust", "pamoja-<name>", "cargo, the crates themselves"),
];
const DOOR_W: f64 = 200.0;
const DOOR_H: f64 = 74.0;
const DOOR_GAP: f64 = 20.0;
const ENGINE_H: f64 = 64.0;
const ARROW: f64 = 44.0;
const INSET: f64 = 14.0;
const BLOCK_HEAD: f64 = 34.0;
const COLUMNS: usize = 5;
const ROW_H: f64 = 18.0;
const CORE_H: f64 = 56.0;

/// Render the drawing as (path, contents).
///
/// # Arguments
///
/// * `catalog` - the capability map; every chapter becomes a box and every capability
///   crate a name inside it.
///
/// # Returns
///
/// `docs/assets/architecture.svg` and its contents.
pub fn render(catalog: &Catalog) -> (String, String) {
    (PATH.to_owned(), svg(catalog))
}

fn svg(catalog: &Catalog) -> String {
    let p = &PALETTE;
    let chapters: Vec<(&str, Vec<String>)> = catalog
        .chapters
        .iter()
        .map(|chapter| {
            let names = catalog
                .in_chapter(&chapter.key)
                .flat_map(|capability| capability.crates.iter())
                .map(|krate| krate.strip_prefix("pamoja-").unwrap_or(krate).to_owned())
                .collect();
            (chapter.title.as_str(), names)
        })
        .collect();
    let longest = chapters
        .iter()
        .flat_map(|(_, names)| names.iter().map(String::len))
        .max()
        .unwrap_or(0);
    let chip_columns = if longest > 10 { 1 } else { 2 };
    let rows_per_box = chapters
        .iter()
        .map(|(_, names)| names.len().div_ceil(chip_columns))
        .max()
        .unwrap_or(1)
        .max(1);
    let box_h = 30.0 + rows_per_box as f64 * ROW_H + 12.0;
    let grid_rows = chapters.len().div_ceil(COLUMNS).max(1);

    let door_y = MARGIN;
    let engine_y = door_y + DOOR_H + ARROW;
    let block_y = engine_y + ENGINE_H + ARROW;
    let grid_y = block_y + BLOCK_HEAD;
    let grid_h = grid_rows as f64 * box_h + (grid_rows as f64 - 1.0) * GAP;
    let block_h = BLOCK_HEAD + grid_h + INSET;
    let core_y = block_y + block_h + GAP;
    let gaps = (COLUMNS as f64 - 1.0) * GAP;
    let box_w = (WIDTH - 2.0 * (MARGIN + INSET) - gaps) / COLUMNS as f64;
    let legend_y = core_y + CORE_H + 26.0;
    let height = legend_y + MARGIN;

    let teal_fill = rgba(p.teal, 0.14);
    let teal_line = rgba(p.teal, 0.55);
    let amber_fill = rgba(p.amber, 0.14);
    let amber_line = rgba(p.amber, 0.55);
    let line = rgba(p.cream, 0.12);
    let mut out = String::new();
    out.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" viewBox="0 0 {WIDTH} {height}" role="img" aria-labelledby="title">
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
  <rect width="{WIDTH}" height="{height}" rx="16" fill="{navy}"/>
"##,
        teal = p.teal,
        amber = p.amber,
        muted = p.muted,
        navy = p.navy_0,
    ));

    // The four doors: three bindings on the left, Rust on the right.
    for (index, (language, packages, bridge)) in DOORS.iter().enumerate() {
        let rust = index == 3;
        let x = if rust {
            WIDTH - MARGIN - DOOR_W
        } else {
            MARGIN + index as f64 * (DOOR_W + DOOR_GAP)
        };
        let (fill, stroke) = if rust {
            (&amber_fill, &amber_line)
        } else {
            (&teal_fill, &teal_line)
        };
        out.push_str(&format!(
            r##"  <rect x="{x}" y="{door_y}" width="{DOOR_W}" height="{DOOR_H}" rx="10" fill="{fill}" stroke="{stroke}"/>
  <text x="{tx}" y="{ty}" font-family="{SANS}" font-size="15" font-weight="600" fill="{cream}">{language}</text>
  <text x="{tx}" y="{py}" font-family="{MONO}" font-size="12" fill="{text}">{packages}</text>
  <text x="{tx}" y="{by}" font-family="{SANS}" font-size="11" fill="{muted}">over {bridge}</text>
"##,
            tx = x + 14.0,
            ty = door_y + 25.0,
            py = door_y + 45.0,
            by = door_y + 62.0,
            packages = escape(packages),
            bridge = escape(bridge),
            cream = p.cream,
            text = p.text,
            muted = p.muted,
        ));
    }

    // The bindings reach the engine; Rust reaches the crates.
    for index in 0..3 {
        let x = MARGIN + index as f64 * (DOOR_W + DOOR_GAP) + DOOR_W / 2.0;
        out.push_str(&arrow(x, door_y + DOOR_H, engine_y, p.muted));
    }
    let rust_x = WIDTH - MARGIN - DOOR_W / 2.0;
    out.push_str(&arrow(rust_x, door_y + DOOR_H, block_y, p.muted));
    out.push_str(&format!(
        r##"  <text x="{x}" y="{y}" font-family="{SANS}" font-size="11" fill="{muted}" text-anchor="end">compiles only the crates it names</text>
"##,
        x = rust_x - 10.0,
        y = engine_y + ENGINE_H / 2.0 + 4.0,
        muted = p.muted,
    ));

    // The engine: one compiled library carrying every capability, under all three bindings.
    let engine_w = 3.0 * DOOR_W + 2.0 * DOOR_GAP;
    out.push_str(&format!(
        r##"  <rect x="{MARGIN}" y="{engine_y}" width="{engine_w}" height="{ENGINE_H}" rx="10" fill="url(#bridge)" stroke="{stroke}"/>
  <text x="{tx}" y="{ty}" font-family="{SANS}" font-size="15" font-weight="600" fill="{cream}">Compiled engine</text>
  <text x="{tx}" y="{sy}" font-family="{SANS}" font-size="12" fill="{text}">pamoja-ffi over the C ABI: one library carrying every capability</text>
  <text x="{rx}" y="{ty}" font-family="{MONO}" font-size="12" fill="{muted}" text-anchor="end">@pamoja/native</text>
  <text x="{rx}" y="{sy}" font-family="{MONO}" font-size="12" fill="{muted}" text-anchor="end">pamoja-native, Pamoja.Native</text>
"##,
        stroke = rgba(p.cream, 0.2),
        tx = MARGIN + 14.0,
        rx = MARGIN + engine_w - 14.0,
        ty = engine_y + 25.0,
        sy = engine_y + 46.0,
        cream = p.cream,
        text = p.text,
        muted = p.muted,
    ));
    let engine_x = MARGIN + engine_w / 2.0;
    out.push_str(&arrow(engine_x, engine_y + ENGINE_H, block_y, p.muted));
    out.push_str(&format!(
        r##"  <text x="{x}" y="{y}" font-family="{SANS}" font-size="11" fill="{muted}">a package narrows the API, not the download</text>
"##,
        x = engine_x + 10.0,
        y = engine_y + ENGINE_H + ARROW / 2.0 + 4.0,
        muted = p.muted,
    ));

    // The capability crates as one block, a box per chapter with the crates it holds.
    let block_w = WIDTH - 2.0 * MARGIN;
    out.push_str(&format!(
        r##"  <rect x="{MARGIN}" y="{block_y}" width="{block_w}" height="{block_h}" rx="12" fill="{fill}" stroke="{line}"/>
  <text x="{tx}" y="{ty}" font-family="{SANS}" font-size="12" font-weight="600" fill="{muted}" letter-spacing="1.2">CAPABILITY CRATES</text>
"##,
        fill = rgba(p.cream, 0.03),
        tx = MARGIN + INSET,
        ty = block_y + 22.0,
        muted = p.muted,
    ));
    let chip_w = (box_w - 28.0) / chip_columns as f64;
    for (index, (title, names)) in chapters.iter().enumerate() {
        let column = index % COLUMNS;
        let row = index / COLUMNS;
        let x = MARGIN + INSET + column as f64 * (box_w + GAP);
        let y = grid_y + row as f64 * (box_h + GAP);
        out.push_str(&format!(
            r##"  <rect x="{x}" y="{y}" width="{box_w}" height="{box_h}" rx="10" fill="{fill}" stroke="{line}"/>
  <text x="{tx}" y="{ty}" font-family="{SANS}" font-size="13" font-weight="600" fill="{cream}">{title}</text>
"##,
            fill = p.navy_2,
            tx = x + 14.0,
            ty = y + 22.0,
            title = escape(title),
            cream = p.cream,
        ));
        for (slot, name) in names.iter().enumerate() {
            let cx = x + 14.0 + (slot % chip_columns) as f64 * chip_w;
            let cy = y + 42.0 + (slot / chip_columns) as f64 * ROW_H;
            out.push_str(&format!(
                r##"  <text x="{cx}" y="{cy}" font-family="{MONO}" font-size="11.5" fill="{text}">{name}</text>
"##,
                text = p.text,
                name = escape(name),
            ));
        }
    }

    // The core every capability crate stands on.
    let core_w = WIDTH - 2.0 * MARGIN;
    out.push_str(&format!(
        r##"  <rect x="{MARGIN}" y="{core_y}" width="{core_w}" height="{CORE_H}" rx="10" fill="{amber_fill}" stroke="{amber_line}"/>
  <text x="{tx}" y="{ty}" font-family="{MONO}" font-size="15" font-weight="600" fill="{cream}">pamoja-core</text>
  <text x="{tx}" y="{sy}" font-family="{SANS}" font-size="12" fill="{text}">Transport, Device, Sensor, Actuator, Store, and the event bus, as traits every capability implements</text>
  <text x="{rx}" y="{ty}" font-family="{SANS}" font-size="12" fill="{muted}" text-anchor="end">no_std, so it runs on a microcontroller</text>
  <text x="{MARGIN}" y="{legend_y}" font-family="{SANS}" font-size="11" fill="{muted}">Every name in a chapter is a crate, pamoja-&lt;name&gt;, and the same capability is a package on npm, PyPI, and NuGet.</text>
</svg>
"##,
        tx = MARGIN + 14.0,
        rx = MARGIN + core_w - 14.0,
        ty = core_y + 25.0,
        sy = core_y + 46.0,
        cream = p.cream,
        text = p.text,
        muted = p.muted,
    ));
    out
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
    fn every_chapter_and_capability_crate_is_drawn() {
        let catalog = Catalog::load(&repo_root()).unwrap();
        let (path, drawing) = render(&catalog);
        assert_eq!(path, PATH);
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

    #[test]
    fn the_drawing_paints_its_ground_in_the_palette_and_names_every_door() {
        let catalog = Catalog::load(&repo_root()).unwrap();
        let (_, drawing) = render(&catalog);
        assert!(drawing.starts_with("<svg "));
        assert!(drawing.contains(&format!("fill=\"{}\"", PALETTE.navy_0)));
        for (language, packages, _) in DOORS {
            assert!(drawing.contains(&format!(">{language}<")));
            assert!(drawing.contains(&escape(packages)));
        }
        assert!(!drawing.contains("<name>"), "the placeholders are escaped");
    }
}
