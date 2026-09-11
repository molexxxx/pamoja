//! The buttons the README and the crate pages link, drawn on the sheet the site is printed
//! on.
//!
//! A registry renders Markdown without a stylesheet, so a button has to be an image. These
//! are flat rectangles in the site's own inks: one filled in the vendor green for the way
//! in, the same shape on the sheet with a hairline for the other ways, and a chip with a
//! square mark for a reference. Each carries both schemes in its own `<style>`, so a reader
//! on a dark registry page gets the dark sheet. `cargo xtask docs` writes them to
//! `.github/badges/`.

use crate::theme::{rgba, DARK, LIGHT};

/// The height of every button, which the README asks for as `height="44"`.
const BOX: f64 = 44.0;
/// The height of the drawn face inside that box.
const FACE: f64 = 32.0;
/// The height of a reference chip's face.
const CHIP: f64 = 26.0;
/// The corner every rectangle on the sheet takes.
const RADIUS: f64 = 2.0;

const ACTION_SIZE: f64 = 12.0;
const ACTION_TRACK: f64 = 0.9;
const ACTION_PAD: f64 = 14.0;
const ACTION_GAP: f64 = 8.0;
const ARROW_W: f64 = 9.0;

const CHIP_SIZE: f64 = 11.0;
const CHIP_TRACK: f64 = 0.8;
const CHIP_PAD: f64 = 11.0;
const MARK: f64 = 5.0;
const MARK_GAP: f64 = 8.0;

/// The height of a badge's face, which is the chip's, so a row of badges and a row of
/// chips sit on one baseline.
const BADGE: f64 = CHIP;
const BADGE_SIZE: f64 = 10.5;
const BADGE_TRACK: f64 = 0.7;
const BADGE_PAD: f64 = 9.0;

const FONT: &str =
    "Segoe UI, Inter, -apple-system, BlinkMacSystemFont, Helvetica, Arial, sans-serif";

/// The mark an action ends on, which says the link leaves the page, drawn rather than typed
/// so it keeps one stroke weight with every other mark on the sheet. A chip does not carry
/// it: a row of them is already a list of references, and nine arrows is noise.
fn arrow(x: f64, mid: f64, class: &str) -> String {
    let left = x - ARROW_W / 2.0;
    let right = x + ARROW_W / 2.0;
    let top = mid - ARROW_W / 2.0;
    let bottom = mid + ARROW_W / 2.0;
    let hook = left + 1.0;
    let side = ARROW_W - 1.0;
    format!(
        "  <path d=\"M{left} {bottom}L{right} {top}M{hook} {top}h{side}v{side}\" class=\"{class}\" fill=\"none\" stroke-width=\"1.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>\n"
    )
}

/// What a button is: the way out of the page, another way out, or something to look up.
enum Kind {
    /// The filled face, and the only one on a page.
    Primary,
    /// The same rectangle on the sheet, for the other places to go.
    Secondary,
    /// A chip with a square mark, for something to look up.
    Reference,
}

/// A rated value, the way a datasheet prints one: the parameter on the tint, its value on
/// the sheet beside it, both inside one hairline.
struct Badge {
    file: &'static str,
    label: &'static str,
    /// The value, or `None` to take the workspace version with a leading `v`.
    value: Option<&'static str>,
}

/// The badges the README carries: where the release can be had, and under what license.
///
/// A version is written from the workspace version rather than read back from a registry,
/// so it is whatever `cargo xtask docs` last wrote and cannot drift from the tree it was
/// generated in. Nothing here claims a build passed, since an image cannot know that and a
/// stale claim is worse than none.
const BADGES: &[Badge] = &[
    Badge {
        file: "badge-crates.svg",
        label: "crates.io",
        value: None,
    },
    Badge {
        file: "badge-npm.svg",
        label: "npm",
        value: None,
    },
    Badge {
        file: "badge-pypi.svg",
        label: "PyPI",
        value: None,
    },
    Badge {
        file: "badge-nuget.svg",
        label: "NuGet",
        value: None,
    },
    Badge {
        file: "badge-license.svg",
        label: "license",
        value: Some("MIT"),
    },
];

/// One button: the file it is written to, its label, and which of the three kinds it is.
struct Button {
    file: &'static str,
    label: &'static str,
    kind: Kind,
}

/// Every button the documentation links.
const BUTTONS: &[Button] = &[
    Button {
        file: "btn-website.svg",
        label: "website",
        kind: Kind::Primary,
    },
    Button {
        file: "btn-guide.svg",
        label: "read the guide",
        kind: Kind::Primary,
    },
    Button {
        file: "btn-docs.svg",
        label: "documentation",
        kind: Kind::Secondary,
    },
    Button {
        file: "btn-examples.svg",
        label: "examples & guides",
        kind: Kind::Secondary,
    },
    Button {
        file: "btn-hardware.svg",
        label: "hardware",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-ref-rust.svg",
        label: "Rust reference",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-ref-node.svg",
        label: "TypeScript reference",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-ref-python.svg",
        label: "Python reference",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-ref-dotnet.svg",
        label: ".NET reference",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-api.svg",
        label: "API reference",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-reference.svg",
        label: "open the reference",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-dashboard.svg",
        label: "dashboard demo",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-docsrs.svg",
        label: "docs.rs",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-cratesio.svg",
        label: "crates.io",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-npm.svg",
        label: "npm",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-pypi.svg",
        label: "PyPI",
        kind: Kind::Reference,
    },
    Button {
        file: "btn-nuget.svg",
        label: "NuGet",
        kind: Kind::Reference,
    },
];

/// Render every button as (path, contents).
///
/// # Returns
///
/// One SVG per button, under `.github/badges/`.
pub fn render(version: &str) -> Vec<(String, String)> {
    let released = format!("v{version}");
    BUTTONS
        .iter()
        .map(|button| {
            let svg = match button.kind {
                Kind::Primary => action(button.label, true),
                Kind::Secondary => action(button.label, false),
                Kind::Reference => reference(button.label),
            };
            (format!(".github/badges/{}", button.file), svg)
        })
        .chain(BADGES.iter().map(|badge| {
            let value = badge.value.unwrap_or(&released);
            (
                format!(".github/badges/{}", badge.file),
                rated(badge.label, value),
            )
        }))
        .collect()
}

// A rated value: the parameter on the tint, a hairline between, the value on the sheet.
// The left cell keeps the outer radius on its own two corners and squares off against the
// divider, which is what makes the pair read as one part rather than two shapes.
fn rated(label: &str, value: &str) -> String {
    let name = escape(&label.to_uppercase());
    let reading = escape(value);
    let name_span = advance(&label.to_uppercase(), BADGE_SIZE, BADGE_TRACK) * CAPS;
    let reading_span = advance(value, BADGE_SIZE, BADGE_TRACK);
    let left = (name_span + BADGE_PAD * 2.0).round();
    let width = (left + reading_span + BADGE_PAD * 2.0).round();
    let mid = BOX / 2.0;
    let top = (BOX - BADGE) / 2.0 + 0.5;
    let height = BADGE - 1.0;
    let radius = RADIUS;
    let run = left - radius - 0.5;
    let side = height - radius * 2.0;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{BOX}" viewBox="0 0 {width} {BOX}" role="img" aria-label="{plain}: {reading}">
{style}
  <path d="M{corner} {top}h{run}v{height}h-{run}a{radius} {radius} 0 0 1 -{radius} -{radius}v-{side}a{radius} {radius} 0 0 1 {radius} -{radius}z" class="tint"/>
  <rect x="0.5" y="{top}" width="{inner}" height="{height}" rx="{radius}" class="rule" fill="none" stroke-width="1"/>
  <path d="M{left} {top}v{height}" class="rule" stroke-width="1"/>
  <text x="{BADGE_PAD}" y="{mid}" dominant-baseline="central" font-family="{FONT}" font-size="{BADGE_SIZE}" font-weight="700" letter-spacing="{BADGE_TRACK}" class="ink">{name}</text>
  <text x="{value_x}" y="{mid}" dominant-baseline="central" font-family="{FONT}" font-size="{BADGE_SIZE}" font-weight="700" letter-spacing="{BADGE_TRACK}" class="accent">{reading}</text>
</svg>
"##,
        plain = escape(label),
        style = sheet(),
        corner = 0.5 + radius,
        inner = width - 1.0,
        value_x = left + BADGE_PAD,
    )
}

// A label as it can appear inside an SVG, which is XML and takes none of these raw. The
// width is measured before this runs: an ampersand is one glyph on the button however
// many characters XML needs to carry it.
fn escape(label: &str) -> String {
    label
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// The palette a button carries with it, in both schemes. A registry page has no stylesheet
// to give it, and an image cannot read the page it sits on, so the media query travels
// inside the file.
fn sheet() -> String {
    format!(
        r##"  <style>
    .paper {{ fill: {light_paper}; }}
    .rule {{ stroke: {light_rule}; }}
    .band {{ fill: {light_band}; }}
    .ink {{ fill: {light_ink}; }}
    .accent {{ fill: {light_accent}; }}
    .accent-rule {{ stroke: {light_accent_rule}; }}
    .tint {{ fill: {light_tint}; }}
    .s-accent {{ stroke: {light_accent}; }}
    .on-band {{ fill: #ffffff; }}
    .s-on-band {{ stroke: #ffffff; }}
    @media (prefers-color-scheme: dark) {{
      .paper {{ fill: {dark_paper}; }}
      .rule {{ stroke: {dark_rule}; }}
      .band {{ fill: {dark_band}; }}
      .ink {{ fill: {dark_ink}; }}
      .accent {{ fill: {dark_accent}; }}
      .accent-rule {{ stroke: {dark_accent_rule}; }}
      .tint {{ fill: {dark_tint}; }}
      .s-accent {{ stroke: {dark_accent}; }}
    }}
  </style>"##,
        light_paper = LIGHT.paper,
        light_rule = LIGHT.rule,
        light_band = LIGHT.band,
        light_ink = LIGHT.ink,
        light_accent = LIGHT.accent,
        light_accent_rule = rgba(LIGHT.accent, 0.45),
        light_tint = rgba(LIGHT.accent, 0.1),
        dark_paper = DARK.paper,
        dark_rule = DARK.rule,
        dark_band = DARK.band,
        dark_ink = DARK.ink,
        dark_accent = DARK.accent,
        dark_accent_rule = rgba(DARK.accent, 0.45),
        dark_tint = rgba(DARK.accent, 0.1),
    )
}

// An action: a rectangle on the sheet with an uppercase label and the arrow that says the
// link leaves. The way in is filled in the vendor ink; the others are the sheet itself
// behind a hairline, so a row keeps one filled shape.
fn action(label: &str, filled: bool) -> String {
    let text = escape(&label.to_uppercase());
    let span = advance(&label.to_uppercase(), ACTION_SIZE, ACTION_TRACK) * CAPS;
    let width = (span + ACTION_PAD * 2.0 + ACTION_GAP + ARROW_W).round();
    let mid = BOX / 2.0;
    let face = if filled {
        format!(
            r##"  <rect x="0.5" y="{y}" width="{inner}" height="{height}" rx="{RADIUS}" class="band"/>"##,
            y = (BOX - FACE) / 2.0 + 0.5,
            inner = width - 1.0,
            height = FACE - 1.0,
        )
    } else {
        format!(
            r##"  <rect x="0.5" y="{y}" width="{inner}" height="{height}" rx="{RADIUS}" class="paper rule" stroke-width="1"/>"##,
            y = (BOX - FACE) / 2.0 + 0.5,
            inner = width - 1.0,
            height = FACE - 1.0,
        )
    };
    let ink = if filled { "on-band" } else { "ink" };
    let mark = if filled { "s-on-band" } else { "s-accent" };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{BOX}" viewBox="0 0 {width} {BOX}" role="img" aria-label="{plain}">
{style}
{face}
  <text x="{ACTION_PAD}" y="{mid}" dominant-baseline="central" font-family="{FONT}" font-size="{ACTION_SIZE}" font-weight="700" letter-spacing="{ACTION_TRACK}" class="{ink}">{text}</text>
{}</svg>
"##,
        arrow(width - ACTION_PAD - ARROW_W / 2.0, mid, mark),
        plain = escape(label),
        style = sheet(),
    )
}

// A reference chip: the vendor ink at a tenth of its strength behind a hairline of the same
// ink, with a square mark and the label in it, which is how the site draws the button that
// opens a generated reference.
fn reference(label: &str) -> String {
    let text = escape(&label.to_uppercase());
    let span = advance(&label.to_uppercase(), CHIP_SIZE, CHIP_TRACK) * CAPS;
    let width = (span + CHIP_PAD * 2.0 + MARK + MARK_GAP).round();
    let mid = BOX / 2.0;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{BOX}" viewBox="0 0 {width} {BOX}" role="img" aria-label="{plain}">
{style}
  <rect x="0.5" y="{y}" width="{inner}" height="{height}" rx="{RADIUS}" class="tint accent-rule" stroke-width="1"/>
  <rect x="{CHIP_PAD}" y="{mark_y}" width="{MARK}" height="{MARK}" class="accent"/>
  <text x="{label_x}" y="{mid}" dominant-baseline="central" font-family="{FONT}" font-size="{CHIP_SIZE}" font-weight="700" letter-spacing="{CHIP_TRACK}" class="accent">{text}</text>
</svg>
"##,
        plain = escape(label),
        style = sheet(),
        y = (BOX - CHIP) / 2.0 + 0.5,
        inner = width - 1.0,
        height = CHIP - 1.0,
        mark_y = mid - MARK / 2.0,
        label_x = CHIP_PAD + MARK + MARK_GAP,
    )
}

/// Segoe UI Semibold's advances, as fractions of the font size. The stack falls back to
/// Inter and then to the system's own humanist sans, which are close enough at these sizes
/// that a label still clears the shape it sits in.
const ADVANCES: &[(&str, f64)] = &[
    (".,:;", 0.241),
    ("'", 0.258),
    ("ijl", 0.261),
    (" ", 0.275),
    ("|", 0.278),
    ("I", 0.292),
    ("!", 0.304),
    ("()", 0.332),
    ("f", 0.345),
    ("t", 0.361),
    ("r", 0.370),
    ("J", 0.396),
    ("1-", 0.402),
    ("/", 0.414),
    ("s", 0.431),
    ("z", 0.464),
    ("c", 0.470),
    ("L", 0.489),
    ("x", 0.501),
    ("F", 0.502),
    ("v", 0.507),
    ("y", 0.508),
    ("E", 0.518),
    ("a", 0.522),
    ("k", 0.525),
    ("e", 0.531),
    ("7", 0.536),
    ("S", 0.544),
    ("T", 0.552),
    ("02358", 0.555),
    ("69", 0.558),
    ("4", 0.576),
    ("Y", 0.577),
    ("h", 0.582),
    ("nu", 0.583),
    ("P", 0.584),
    ("Z", 0.587),
    ("#", 0.591),
    ("o", 0.597),
    ("bdgpq", 0.603),
    ("B", 0.604),
    ("K", 0.611),
    ("X", 0.619),
    ("C", 0.621),
    ("R", 0.623),
    ("V", 0.642),
    ("A", 0.671),
    ("G", 0.697),
    ("U", 0.703),
    ("&", 0.715),
    ("D", 0.717),
    ("H", 0.735),
    ("wOQ", 0.756),
    ("N", 0.767),
    ("m", 0.886),
    ("M", 0.924),
    ("@", 0.955),
    ("W", 0.966),
];

/// What a character not in the table is worth, which is about what a lowercase letter is.
const DEFAULT_ADVANCE: f64 = 0.58;

// The rendered width of a label, in the absence of a font engine. The shape is text on a
// filled ground, so this only has to be close: too narrow crowds the label, too wide leaves
// the button looking empty.
/// How much wider a run of capitals is than the same string in the mixed case the table
/// was measured from. The labels are set in capitals, so every width is scaled by this.
const CAPS: f64 = 1.07;

fn advance(text: &str, size: f64, tracking: f64) -> f64 {
    text.chars()
        .map(|ch| {
            let ratio = ADVANCES
                .iter()
                .find(|(set, _)| set.contains(ch))
                .map_or(DEFAULT_ADVANCE, |(_, ratio)| *ratio);
            ratio * size + tracking
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn width(svg: &str) -> f64 {
        svg.split("width=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .and_then(|value| value.parse().ok())
            .expect("the root element carries a width")
    }

    #[test]
    fn renders_one_file_per_button_and_badge() {
        let files = render("0.1.0");
        assert_eq!(files.len(), BUTTONS.len() + BADGES.len());
        assert!(files
            .iter()
            .all(|(path, _)| path.starts_with(".github/badges/") && path.ends_with(".svg")));
    }

    #[test]
    fn a_badge_without_a_value_of_its_own_carries_the_workspace_version() {
        let files = render("9.8.7");
        let versioned: Vec<&String> = files
            .iter()
            .filter(|(path, _)| path.contains("badge-") && !path.contains("license"))
            .map(|(_, svg)| svg)
            .collect();
        assert_eq!(versioned.len(), 4);
        assert!(versioned.iter().all(|svg| svg.contains("v9.8.7")));

        let license = files
            .iter()
            .find(|(path, _)| path.ends_with("badge-license.svg"))
            .expect("a license badge");
        assert!(license.1.contains(">MIT<"), "{}", license.1);
        assert!(!license.1.contains("9.8.7"));
    }

    #[test]
    fn a_badge_reads_as_one_part_on_one_baseline_with_a_chip() {
        let (_, svg) = render("0.1.0")
            .into_iter()
            .find(|(path, _)| path.ends_with("badge-crates.svg"))
            .expect("a crates badge");
        // One outer hairline, one divider, and the tint behind the parameter only.
        assert_eq!(svg.matches("class=\"rule\"").count(), 2);
        assert_eq!(svg.matches("class=\"tint\"").count(), 1);
        // The face is the chip's, so the two align when a row of each sits together.
        let top = (BOX - BADGE) / 2.0 + 0.5;
        assert!(
            svg.contains(&format!("height=\"{}\"", BADGE - 1.0)),
            "{svg}"
        );
        assert!(svg.contains(&format!("y=\"{top}\"")), "{svg}");
    }

    #[test]
    fn a_longer_label_gets_a_wider_button() {
        assert!(width(&action("examples & guides", false)) > width(&action("website", true)));
        assert!(width(&reference("TypeScript reference")) > width(&reference("npm")));
    }

    #[test]
    fn a_label_is_measured_before_it_is_escaped() {
        assert!(
            width(&action("examples & guides", true)) < width(&action("examples and guides", true))
        );
    }

    #[test]
    fn a_label_clears_the_shape_it_sits_in() {
        for (button, (_, svg)) in BUTTONS.iter().zip(render("0.1.0")) {
            let label = button.label.to_uppercase();
            let taken = match button.kind {
                Kind::Reference => {
                    advance(&label, CHIP_SIZE, CHIP_TRACK) * CAPS + CHIP_PAD + MARK + MARK_GAP
                }
                _ => advance(&label, ACTION_SIZE, ACTION_TRACK) * CAPS + ACTION_PAD + ACTION_GAP,
            };
            let clearance = width(&svg) - taken;
            assert!(
                clearance >= 10.0,
                "{} sits {clearance} from the edge",
                button.label
            );
        }
    }

    #[test]
    fn the_way_in_is_filled_and_the_others_are_the_sheet() {
        let filled = action("website", true);
        assert!(filled.contains("class=\"band\"") && !filled.contains("class=\"paper rule\""));
        let outline = action("documentation", false);
        assert!(outline.contains("class=\"paper rule\"") && !outline.contains("class=\"band\""));
        let chip = reference("npm");
        assert!(chip.contains("class=\"tint accent-rule\"") && !chip.contains("class=\"band\""));
    }

    #[test]
    fn a_page_carries_one_filled_button() {
        let filled: Vec<_> = BUTTONS
            .iter()
            .filter(|button| matches!(button.kind, Kind::Primary))
            .map(|button| button.file)
            .collect();
        assert_eq!(filled, ["btn-website.svg", "btn-guide.svg"]);
    }

    #[test]
    fn every_button_carries_both_sheets_and_no_other_palette() {
        for (path, svg) in render("0.1.0") {
            assert!(
                svg.contains(LIGHT.accent) && svg.contains(DARK.accent),
                "{path} carries only one sheet"
            );
            assert!(
                svg.contains("prefers-color-scheme: dark"),
                "{path} cannot follow a dark registry page"
            );
            // The navy, coral, and sky of the world these buttons used to be drawn in.
            for gone in ["#0e1b2e", "#16263f", "#f26a4b", "#36b6dd"] {
                assert!(!svg.contains(gone), "{path} still carries {gone}");
            }
        }
    }

    #[test]
    fn every_kind_shares_one_box_so_a_row_lines_up() {
        let height = |svg: &str| -> String {
            svg.split("height=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .map(str::to_owned)
                .expect("the root element carries a height")
        };
        assert_eq!(height(&action("website", true)), height(&reference("npm")));
        assert!(render("0.1.0")
            .iter()
            .all(|(_, svg)| height(svg) == BOX.to_string()));
    }

    #[test]
    fn only_an_action_says_the_link_leaves_the_page() {
        assert!(action("website", true).contains("class=\"s-on-band\""));
        assert!(action("documentation", false).contains("class=\"s-accent\""));
        assert!(!reference("npm").contains("stroke-linecap"));
        assert!(render("0.1.0")
            .iter()
            .all(|(_, svg)| svg.contains("role=\"img\"")));
    }

    #[test]
    fn a_label_that_is_not_xml_is_escaped_into_it() {
        let svg = action("examples & guides", false);
        assert!(svg.contains("EXAMPLES &amp; GUIDES"));
        assert!(!svg.contains("EXAMPLES & GUIDES"));
    }
}
