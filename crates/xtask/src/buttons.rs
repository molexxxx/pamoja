//! The link buttons the READMEs and the site use, drawn from the logo's palette.
//!
//! GitHub, crates.io, npm, PyPI, and NuGet all render a README against a background this
//! project does not control, and half of them render it twice, once per theme. A filled
//! shape holds on both; an outlined one disappears on one of them. So every button carries
//! its own opaque fill, and the hierarchy has to come from somewhere other than weight.
//!
//! It comes from three kinds, which are the site's own hero controls drawn as images. One
//! warm face per page carries the first place a reader is being sent, and it is the only
//! saturated shape in the block, so a row reads as one accent among the navy rather than a
//! wall of orange. The other places to go are the same pill in the navy the pages are drawn
//! in, told apart by a brighter edge and an amber mark. Everything that is looked up rather
//! than gone to is a smaller chip, flush on the page, told apart from its neighbors by a
//! single dot of color: the language references, the registries, the hardware catalog.
//!
//! All three are drawn inside the same box, so one `height` attribute in the markup renders
//! the difference the drawing carries rather than making the markup carry it, and a row that
//! mixes them sits on a common center line.
//!
//! The gradients are the logo's, and the geometry is the site's `.btn`: a full pill, the
//! same amber-through-coral face, and the same shadow under it. `cargo xtask docs` writes
//! them and `--check` fails when they drift, so the palette lives in exactly one place.

use crate::theme::{rgba, PALETTE};

/// Amber, the logo's inner glow.
const AMBER: &str = PALETTE.amber;
/// Coral, the logo's outer glow.
const CORAL: &str = PALETTE.coral;
/// Near-black with the palette's warmth in it, for text on a warm fill.
const ON_WARM: &str = "#2a1606";
/// The palette's dark warm, which shades the bottom edge of a warm face.
const CONTACT: &str = "#7a2c14";

/// The box every button is drawn in. An action fills most of it and a chip sits centered
/// inside it, so both scale together from a single height.
const BOX: f64 = 44.0;
/// The face of an action, with the rest of the box left for its shadow.
const FACE: f64 = 34.0;
/// The face of a reference chip.
const CHIP: f64 = 26.0;

const ACTION_SIZE: f64 = 13.5;
const ACTION_TRACK: f64 = 0.1;
/// The label's inset, the space it clears the arrow by, the arrow's disc, and its inset.
const ACTION_LEAD: f64 = 17.0;
const ACTION_GAP: f64 = 9.0;
const PUCK: f64 = 17.0;
const ACTION_TAIL: f64 = 11.0;

const CHIP_SIZE: f64 = 11.5;
const CHIP_TRACK: f64 = 0.4;
/// Where the dot sits, how far the label clears it, and what the label clears the edge by.
const DOT_X: f64 = 11.0;
const DOT_GAP: f64 = 9.0;
const CHIP_PAD: f64 = 11.0;

const FONT: &str =
    "Segoe UI, Inter, -apple-system, BlinkMacSystemFont, Helvetica, Arial, sans-serif";

/// The mark an action ends on, which says the link leaves the page. A chip does not carry
/// it: a row of them is already a list of references, and nine arrows is noise.
const ARROW: &str = "\u{2197}";

/// What a button is: the way out of the page, another way out, or something to look up.
enum Kind {
    /// The warm face, and the only one on a page.
    Primary,
    /// The same pill in navy, for the other places to go.
    Secondary,
    /// A flush chip, carrying one hue as its dot.
    Reference(&'static str),
}

/// One button: the file it is written to, its label, and which of the three kinds it is.
struct Button {
    file: &'static str,
    label: &'static str,
    kind: Kind,
}

/// Every button the documentation links.
///
/// The hues are the ecosystems the logo already travels through, used consistently: amber
/// for Rust, sky for TypeScript, forest for Python, coral for .NET, and teal for the
/// things that are pamoja's own rather than a language's.
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
        kind: Kind::Reference(PALETTE.teal),
    },
    Button {
        file: "btn-ref-rust.svg",
        label: "Rust reference",
        kind: Kind::Reference(PALETTE.amber),
    },
    Button {
        file: "btn-ref-node.svg",
        label: "TypeScript reference",
        kind: Kind::Reference(PALETTE.sky),
    },
    Button {
        file: "btn-ref-python.svg",
        label: "Python reference",
        kind: Kind::Reference(PALETTE.forest),
    },
    Button {
        file: "btn-ref-dotnet.svg",
        label: ".NET reference",
        kind: Kind::Reference(PALETTE.coral),
    },
    Button {
        file: "btn-dashboard.svg",
        label: "dashboard demo",
        kind: Kind::Reference(PALETTE.teal),
    },
    Button {
        file: "btn-api.svg",
        label: "API reference",
        kind: Kind::Reference(PALETTE.cream),
    },
    Button {
        file: "btn-reference.svg",
        label: "open the reference",
        kind: Kind::Reference(PALETTE.cream),
    },
    Button {
        file: "btn-cratesio.svg",
        label: "crates.io",
        kind: Kind::Reference(PALETTE.amber),
    },
    Button {
        file: "btn-docsrs.svg",
        label: "docs.rs",
        kind: Kind::Reference(PALETTE.amber),
    },
    Button {
        file: "btn-npm.svg",
        label: "npm",
        kind: Kind::Reference(PALETTE.sky),
    },
    Button {
        file: "btn-pypi.svg",
        label: "PyPI",
        kind: Kind::Reference(PALETTE.forest),
    },
    Button {
        file: "btn-nuget.svg",
        label: "NuGet",
        kind: Kind::Reference(PALETTE.coral),
    },
];

/// Render every button as (path, SVG).
///
/// # Returns
///
/// One entry per button, pathed under `.github/badges`.
pub fn render() -> Vec<(String, String)> {
    BUTTONS
        .iter()
        .map(|button| {
            let svg = match button.kind {
                Kind::Primary => action(button.label, true),
                Kind::Secondary => action(button.label, false),
                Kind::Reference(accent) => reference(button.label, accent),
            };
            (format!(".github/badges/{}", button.file), svg)
        })
        .collect()
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

// An action: the site's pill, with its label set against a disc holding the arrow. Warm is
// the lit face with a shadow in its own hue; the rest are the same shape in navy, which
// keeps a row to one accent and still reads as somewhere to go rather than something to
// look up.
fn action(label: &str, warm: bool) -> String {
    let text = escape(label);
    let span = advance(label, ACTION_SIZE, ACTION_TRACK);
    let width = (span + ACTION_LEAD + ACTION_GAP + PUCK + ACTION_TAIL).round();
    let mid = BOX / 2.0;
    let face = if warm {
        warm_face(width)
    } else {
        glass_face(width)
    };
    let (ink, mark) = if warm {
        (ON_WARM, rgba(ON_WARM, 0.16))
    } else {
        (PALETTE.cream, rgba(AMBER, 0.16))
    };
    let arrow_ink = if warm { ON_WARM } else { AMBER };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{BOX}" viewBox="0 0 {width} {BOX}" role="img" aria-label="{text}">
{face}
  <text x="{label_x}" y="{mid}" dominant-baseline="central" text-anchor="middle" font-family="{FONT}" font-size="{ACTION_SIZE}" font-weight="600" letter-spacing="{ACTION_TRACK}" fill="{ink}">{text}</text>
  <circle cx="{puck_x}" cy="{mid}" r="{puck_r}" fill="{mark}"/>
  <text x="{puck_x}" y="{mid}" dominant-baseline="central" text-anchor="middle" font-family="{FONT}" font-size="11" font-weight="700" fill="{arrow_ink}" fill-opacity="{arrow_alpha}">{ARROW}</text>
</svg>
"##,
        label_x = ((ACTION_LEAD + span / 2.0) * 100.0).round() / 100.0,
        puck_x = width - ACTION_TAIL - PUCK / 2.0,
        puck_r = PUCK / 2.0,
        arrow_alpha = if warm { 0.72 } else { 1.0 },
    )
}

// The warm face: the site's amber-through-coral gradient, molded by a highlight along the
// top and the contact color along the bottom, over a bloom of its own hue that grounds it.
// The bloom is drawn rather than blurred, because a filter reaches past the image box the
// button is cut to and leaves a seam down both edges of the row.
fn warm_face(width: f64) -> String {
    format!(
        r##"  <defs>
    <linearGradient id="face" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="{AMBER}"/>
      <stop offset="1" stop-color="{CORAL}"/>
      <animateTransform attributeName="gradientTransform" type="translate" values="-0.08 0;0.08 0;-0.08 0" dur="9s" repeatCount="indefinite"/>
    </linearGradient>
    <linearGradient id="mold" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.28"/>
      <stop offset="0.55" stop-color="#ffffff" stop-opacity="0"/>
      <stop offset="1" stop-color="{CONTACT}" stop-opacity="0.16"/>
    </linearGradient>
    <linearGradient id="rim" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.55"/>
      <stop offset="0.5" stop-color="#ffffff" stop-opacity="0.06"/>
      <stop offset="1" stop-color="{navy}" stop-opacity="0.26"/>
    </linearGradient>
    <radialGradient id="bloom" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="{CORAL}" stop-opacity="0.5"/>
      <stop offset="0.6" stop-color="{CORAL}" stop-opacity="0.2"/>
      <stop offset="1" stop-color="{CORAL}" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <ellipse cx="{center}" cy="{bloom_y}" rx="{bloom_rx}" ry="6.5" fill="url(#bloom)"/>
  <rect x="0.5" y="{y}" width="{inner}" height="{height}" rx="{radius}" fill="url(#face)"/>
  <rect x="0.5" y="{y}" width="{inner}" height="{height}" rx="{radius}" fill="url(#mold)"/>
  <rect x="0.5" y="{y}" width="{inner}" height="{height}" rx="{radius}" fill="none" stroke="url(#rim)" stroke-width="1"/>"##,
        navy = PALETTE.navy_0,
        center = width / 2.0,
        bloom_y = (BOX + FACE) / 2.0 - 2.0,
        bloom_rx = width / 2.0 - 2.0,
        y = (BOX - FACE) / 2.0 + 0.5,
        inner = width - 1.0,
        height = FACE - 1.0,
        radius = FACE / 2.0,
    )
}

// The navy face: the same pill in the page's own color, with the strong hairline and an
// inner highlight for the glass, and no shadow, so it sits behind the warm one.
fn glass_face(width: f64) -> String {
    format!(
        r##"  <defs>
    <linearGradient id="face" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="{navy_2}"/>
      <stop offset="1" stop-color="{navy_1}"/>
    </linearGradient>
    <linearGradient id="rim" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.16"/>
      <stop offset="1" stop-color="#ffffff" stop-opacity="0.04"/>
    </linearGradient>
  </defs>
  <rect x="0.5" y="{y}" width="{inner}" height="{height}" rx="{radius}" fill="url(#face)" stroke="{edge}" stroke-width="1"/>
  <rect x="1.5" y="{inset_y}" width="{inset_w}" height="{inset_h}" rx="{inset_r}" fill="none" stroke="url(#rim)" stroke-width="1"/>"##,
        navy_1 = PALETTE.navy_1,
        navy_2 = PALETTE.navy_2,
        edge = rgba(PALETTE.cream, 0.22),
        y = (BOX - FACE) / 2.0 + 0.5,
        inner = width - 1.0,
        height = FACE - 1.0,
        radius = FACE / 2.0,
        inset_y = (BOX - FACE) / 2.0 + 1.5,
        inset_w = width - 3.0,
        inset_h = FACE - 3.0,
        inset_r = FACE / 2.0 - 1.0,
    )
}

// A reference chip: the dark face, flush, with one hue as a dot and a halo behind it so
// the color reads without outlining the whole shape in it.
fn reference(label: &str, accent: &str) -> String {
    let text = escape(label);
    let width = (advance(label, CHIP_SIZE, CHIP_TRACK) + DOT_X + DOT_GAP + CHIP_PAD).round();
    let mid = BOX / 2.0;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{BOX}" viewBox="0 0 {width} {BOX}" role="img" aria-label="{text}">
  <defs>
    <linearGradient id="face" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="{navy_2}"/>
      <stop offset="1" stop-color="{navy_1}"/>
    </linearGradient>
  </defs>
  <rect x="0.5" y="{y}" width="{inner}" height="{height}" rx="{radius}" fill="url(#face)" stroke="{hairline}" stroke-width="1"/>
  <circle cx="{DOT_X}" cy="{mid}" r="5.5" fill="{halo}"/>
  <circle cx="{DOT_X}" cy="{mid}" r="2.4" fill="{accent}"/>
  <text x="{label_x}" y="{mid}" dominant-baseline="central" font-family="{FONT}" font-size="{CHIP_SIZE}" font-weight="600" letter-spacing="{CHIP_TRACK}" fill="{ink}">{text}</text>
</svg>
"##,
        navy_1 = PALETTE.navy_1,
        navy_2 = PALETTE.navy_2,
        y = (BOX - CHIP) / 2.0 + 0.5,
        inner = width - 1.0,
        height = CHIP - 1.0,
        radius = CHIP / 2.0,
        hairline = rgba(PALETTE.cream, 0.14),
        halo = rgba(accent, 0.2),
        label_x = DOT_X + DOT_GAP,
        ink = PALETTE.text,
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
    fn renders_one_file_per_button() {
        let files = render();
        assert_eq!(files.len(), BUTTONS.len());
        assert!(files
            .iter()
            .all(|(path, _)| path.starts_with(".github/badges/") && path.ends_with(".svg")));
    }

    #[test]
    fn a_longer_label_gets_a_wider_button() {
        assert!(width(&action("examples & guides", false)) > width(&action("website", true)));
        assert!(width(&reference("TypeScript reference", AMBER)) > width(&reference("npm", AMBER)));
    }

    #[test]
    fn a_label_is_measured_before_it_is_escaped() {
        assert!(
            width(&action("examples & guides", true)) < width(&action("examples and guides", true))
        );
    }

    #[test]
    fn a_label_clears_the_shape_it_sits_in() {
        for (button, (_, svg)) in BUTTONS.iter().zip(render()) {
            let taken = match button.kind {
                Kind::Reference(_) => {
                    advance(button.label, CHIP_SIZE, CHIP_TRACK) + DOT_X + DOT_GAP
                }
                _ => advance(button.label, ACTION_SIZE, ACTION_TRACK) + ACTION_LEAD + ACTION_GAP,
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
    fn one_kind_is_warm_and_the_others_are_not() {
        let warm = action("website", true);
        assert!(warm.contains(AMBER) && warm.contains("url(#bloom)"));
        let glass = action("documentation", false);
        assert!(glass.contains(PALETTE.navy_2) && !glass.contains("url(#bloom)"));
        let chip = reference("npm", PALETTE.sky);
        assert!(chip.contains(PALETTE.navy_2) && !chip.contains("url(#bloom)"));
    }

    #[test]
    fn a_page_carries_one_warm_button() {
        let warm: Vec<_> = BUTTONS
            .iter()
            .filter(|button| matches!(button.kind, Kind::Primary))
            .map(|button| button.file)
            .collect();
        assert_eq!(warm, ["btn-website.svg", "btn-guide.svg"]);
    }

    #[test]
    fn a_reference_carries_its_own_hue_and_no_other() {
        let python = reference("Python reference", PALETTE.forest);
        assert!(python.contains(PALETTE.forest));
        assert!(!python.contains(PALETTE.sky) && !python.contains(CORAL));
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
        assert_eq!(
            height(&action("website", true)),
            height(&reference("npm", AMBER))
        );
        assert!(render()
            .iter()
            .all(|(_, svg)| height(svg) == BOX.to_string()));
    }

    #[test]
    fn only_an_action_says_the_link_leaves_the_page() {
        assert!(action("website", true).contains(ARROW));
        assert!(action("documentation", false).contains(ARROW));
        assert!(!reference("npm", AMBER).contains(ARROW));
        assert!(render().iter().all(|(_, svg)| svg.contains("role=\"img\"")));
    }

    #[test]
    fn a_label_that_is_not_xml_is_escaped_into_it() {
        let svg = action("examples & guides", false);
        assert!(svg.contains("examples &amp; guides"));
        assert!(!svg.contains("examples & guides"));
    }
}
