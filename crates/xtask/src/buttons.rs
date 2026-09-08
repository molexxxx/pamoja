//! The link buttons the READMEs and the site use, drawn from the logo's palette.
//!
//! GitHub, crates.io, npm, PyPI, and NuGet all render a README against a background this
//! project does not control, and half of them render it twice, once per theme. A filled
//! shape holds on both; an outlined one disappears on one of them. So every button carries
//! its own opaque fill, and the hierarchy has to come from somewhere other than weight.
//!
//! It comes from two kinds. An action is a warm face lifted off the page by a shadow its
//! own fill justifies: the three or four places a reader is actually being sent. A
//! reference is a chip that sits flush on the page, dark enough to hold on white, told
//! apart from its neighbours by a single dot of color: the language references, the
//! registries, the hardware catalog. One is lit and one is not, which reads at a glance
//! and survives being scaled down to a crate README.
//!
//! Both kinds are drawn inside the same box, so one `height` attribute in the markup
//! renders the difference the drawing carries rather than making the markup carry it, and
//! a row that mixes them sits on a common center line.
//!
//! The gradients are the logo's: amber through coral for an action, and the hues the
//! wordmark travels through for the reference dots. `cargo xtask docs` writes them and
//! `--check` fails when they drift, so the palette lives in exactly one place.

use crate::theme::{rgba, PALETTE};

/// Amber, the logo's inner glow.
const AMBER: &str = PALETTE.amber;
/// Coral, the logo's outer glow.
const CORAL: &str = PALETTE.coral;
/// A lighter amber, so a short label still shows the face traveling.
const LIT: &str = "#ffc85a";
/// Near-black with the palette's warmth in it, for text on a warm fill.
const ON_WARM: &str = "#2a1606";
/// The shadow directly under an action, which grounds it where the wide one only glows.
const CONTACT: &str = "#7a2c14";

/// The box every button is drawn in. An action fills most of it and a chip sits centered
/// inside it, so both scale together from a single height.
const BOX: f64 = 44.0;
/// The face of an action, with the rest of the box left for its shadow.
const FACE: f64 = 36.0;
/// The face of a reference chip.
const CHIP: f64 = 26.0;

const ACTION_SIZE: f64 = 14.5;
const ACTION_TRACK: f64 = 0.2;
const ACTION_PAD: f64 = 21.0;
const CHIP_SIZE: f64 = 11.5;
const CHIP_TRACK: f64 = 0.4;
/// Where the dot sits, and how far the label clears it.
const DOT_X: f64 = 12.0;
const DOT_GAP: f64 = 10.0;
const CHIP_PAD: f64 = 12.0;

const FONT: &str =
    "Segoe UI, Inter, -apple-system, BlinkMacSystemFont, Helvetica, Arial, sans-serif";

/// The trailing mark on an action, which says the link leaves the page. A chip does not
/// carry it: a row of them is already a list of references, and nine arrows is noise.
const ARROW: &str = "\u{2197}";

/// What a button is: somewhere to go, or something to look up.
enum Kind {
    /// A warm, lifted face.
    Action,
    /// A flush chip, carrying one hue as its dot.
    Reference(&'static str),
}

/// One button: the file it is written to, its label, and which of the two kinds it is.
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
        kind: Kind::Action,
    },
    Button {
        file: "btn-docs.svg",
        label: "documentation",
        kind: Kind::Action,
    },
    Button {
        file: "btn-examples.svg",
        label: "examples & guides",
        kind: Kind::Action,
    },
    Button {
        file: "btn-guide.svg",
        label: "read the guide",
        kind: Kind::Action,
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
                Kind::Action => action(button.label),
                Kind::Reference(accent) => reference(button.label, accent),
            };
            (format!(".github/badges/{}", button.file), svg)
        })
        .collect()
}

// A label as it can appear inside an SVG, which is XML and takes none of these raw.
fn escape(label: &str) -> String {
    label
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// An action: the warm face, lit, with a wide shadow in its own hue for the lift and a
// tight one under it for the contact.
fn action(label: &str) -> String {
    let text = escape(label);
    let width = (advance(&text, ACTION_SIZE, ACTION_TRACK) + ACTION_PAD * 2.0 + 15.0).round();
    let top = 2.0;
    let mid = top + FACE / 2.0;
    let arrow = ACTION_SIZE - 1.5;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{BOX}" viewBox="0 0 {width} {BOX}" role="img" aria-label="{text}">
  <defs>
    <linearGradient id="face" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="{LIT}"/>
      <stop offset="0.42" stop-color="{AMBER}"/>
      <stop offset="1" stop-color="{CORAL}"/>
      <animateTransform attributeName="gradientTransform" type="translate" values="-0.10 0;0.10 0;-0.10 0" dur="11s" repeatCount="indefinite"/>
    </linearGradient>
    <linearGradient id="edge" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.55"/>
      <stop offset="0.42" stop-color="#ffffff" stop-opacity="0.04"/>
      <stop offset="1" stop-color="{navy}" stop-opacity="0.22"/>
    </linearGradient>
    <filter id="lift" x="-40%" y="-40%" width="180%" height="200%">
      <feDropShadow dx="0" dy="4" stdDeviation="6.5" flood-color="{CORAL}" flood-opacity="0.34"/>
      <feDropShadow dx="0" dy="1" stdDeviation="1.2" flood-color="{CONTACT}" flood-opacity="0.22"/>
    </filter>
  </defs>
  <g filter="url(#lift)">
    <rect x="0.5" y="{face_y}" width="{inner}" height="{face_h}" rx="10" fill="url(#face)"/>
    <rect x="0.5" y="{face_y}" width="{inner}" height="{face_h}" rx="10" fill="none" stroke="url(#edge)" stroke-width="1"/>
  </g>
  <text x="{mid_x}" y="{mid}" dominant-baseline="central" text-anchor="middle" font-family="{FONT}" font-size="{ACTION_SIZE}" font-weight="700" letter-spacing="{ACTION_TRACK}" fill="{ON_WARM}">{text}<tspan font-size="{arrow}" fill-opacity="0.55"> {ARROW}</tspan></text>
</svg>
"##,
        navy = PALETTE.navy_0,
        face_y = top + 0.5,
        face_h = FACE - 1.0,
        inner = width - 1.0,
        mid_x = width / 2.0,
    )
}

// A reference chip: the dark face, flush, with one hue as a dot and a halo behind it so
// the color reads without outlining the whole shape in it.
fn reference(label: &str, accent: &str) -> String {
    let text = escape(label);
    let width = (advance(&text, CHIP_SIZE, CHIP_TRACK) + DOT_X + DOT_GAP + CHIP_PAD).round();
    let top = (BOX - CHIP) / 2.0;
    let mid = BOX / 2.0;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{BOX}" viewBox="0 0 {width} {BOX}" role="img" aria-label="{text}">
  <defs>
    <linearGradient id="face" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="{navy_2}"/>
      <stop offset="1" stop-color="{navy_1}"/>
    </linearGradient>
  </defs>
  <rect x="0.5" y="{face_y}" width="{inner}" height="{face_h}" rx="8" fill="url(#face)" stroke="{hairline}" stroke-width="1"/>
  <circle cx="{DOT_X}" cy="{mid}" r="5.5" fill="{halo}"/>
  <circle cx="{DOT_X}" cy="{mid}" r="2.4" fill="{accent}"/>
  <text x="{label_x}" y="{mid}" dominant-baseline="central" font-family="{FONT}" font-size="{CHIP_SIZE}" font-weight="600" letter-spacing="{CHIP_TRACK}" fill="{ink}">{text}</text>
</svg>
"##,
        navy_1 = PALETTE.navy_1,
        navy_2 = PALETTE.navy_2,
        face_y = top + 0.5,
        face_h = CHIP - 1.0,
        inner = width - 1.0,
        hairline = rgba(PALETTE.cream, 0.14),
        halo = rgba(accent, 0.2),
        label_x = DOT_X + DOT_GAP,
        ink = PALETTE.text,
    )
}

// The rendered width of a label, in the absence of a font engine. The shape is text on a
// filled ground, so this only has to be close: too narrow crowds the label, too wide
// leaves the button looking empty. The ratios are for a humanist sans at these sizes,
// with the tracking added per character.
fn advance(text: &str, size: f64, tracking: f64) -> f64 {
    text.chars()
        .map(|ch| {
            let ratio = match ch {
                'i' | 'j' | 'l' | 'I' | '.' | ',' | '\'' | '!' | '|' => 0.29,
                'f' | 'r' | 't' | '(' | ')' | '[' | ']' | '/' | ' ' => 0.40,
                'm' | 'M' | 'W' | 'w' => 0.90,
                'A'..='Z' => 0.68,
                _ => 0.56,
            };
            ratio * size + tracking
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let width = |svg: &str| -> f64 {
            svg.split("width=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .and_then(|value| value.parse().ok())
                .expect("the root element carries a width")
        };
        assert!(width(&action("examples & guides")) > width(&action("website")));
        assert!(width(&reference("TypeScript reference", AMBER)) > width(&reference("npm", AMBER)));
    }

    #[test]
    fn an_action_is_lit_and_a_reference_is_not() {
        let act = action("website");
        assert!(act.contains(AMBER) && act.contains("feDropShadow"));
        let chip = reference("npm", PALETTE.sky);
        assert!(chip.contains(PALETTE.navy_2) && !chip.contains("feDropShadow"));
    }

    #[test]
    fn a_reference_carries_its_own_hue_and_no_other() {
        let python = reference("Python reference", PALETTE.forest);
        assert!(python.contains(PALETTE.forest));
        assert!(!python.contains(PALETTE.sky) && !python.contains(CORAL));
    }

    #[test]
    fn both_kinds_share_one_box_so_a_row_lines_up() {
        let height = |svg: &str| -> String {
            svg.split("height=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .map(str::to_owned)
                .expect("the root element carries a height")
        };
        assert_eq!(height(&action("website")), height(&reference("npm", AMBER)));
        assert!(render()
            .iter()
            .all(|(_, svg)| height(svg) == BOX.to_string()));
    }

    #[test]
    fn only_an_action_says_the_link_leaves_the_page() {
        assert!(action("website").contains(ARROW));
        assert!(!reference("npm", AMBER).contains(ARROW));
        assert!(render().iter().all(|(_, svg)| svg.contains("role=\"img\"")));
    }

    #[test]
    fn a_label_that_is_not_xml_is_escaped_into_it() {
        let svg = action("examples & guides");
        assert!(svg.contains("examples &amp; guides"));
        assert!(!svg.contains("examples & guides"));
    }
}
