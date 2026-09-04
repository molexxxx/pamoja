//! The link buttons the READMEs and the site use, drawn from the logo's palette.
//!
//! GitHub, crates.io, npm, PyPI, and NuGet all render a README against a background this
//! project does not control, and half of them render it twice, once per theme. A filled
//! button holds on both; an outlined one disappears on one of them. So every button is
//! filled, and the two kinds are told apart by which fill they carry rather than by weight:
//! a warm one for the places worth going, and a cool one for the registries, which are
//! reference links rather than invitations.
//!
//! The gradients are the logo's: amber through coral for the warm buttons, and the teal it
//! resolves to for the cool ones. `cargo xtask docs` writes them and `--check` fails when
//! they drift, so the palette lives in exactly one place.

/// Amber, the logo's inner glow.
const AMBER: &str = "#FFB627";
/// Coral, the logo's outer glow.
const CORAL: &str = "#F26A4B";
/// Teal, where the logo's wordmark gradient resolves.
const TEAL: &str = "#1FA995";
/// A deeper teal, so the cool buttons have somewhere to travel.
const DEEP_TEAL: &str = "#12736A";
/// Near-black with the palette's warmth in it, for text on a warm fill.
const ON_WARM: &str = "#2A1606";
/// The cream from the logo's core, for text on a cool fill.
const ON_COOL: &str = "#FFF3D6";

/// One button: the file it is written to, its label, and whether it leads somewhere
/// (warm) or names a registry (cool).
struct Button {
    file: &'static str,
    label: &'static str,
    warm: bool,
}

/// Every button the documentation links. The warm ones are the three places a reader is
/// being invited to go; the rest name a registry.
const BUTTONS: &[Button] = &[
    Button {
        file: "btn-website.svg",
        label: "website",
        warm: true,
    },
    Button {
        file: "btn-docs.svg",
        label: "documentation",
        warm: true,
    },
    Button {
        file: "btn-dashboard.svg",
        label: "dashboard demo",
        warm: true,
    },
    Button {
        file: "btn-cratesio.svg",
        label: "crates.io",
        warm: false,
    },
    Button {
        file: "btn-docsrs.svg",
        label: "docs.rs",
        warm: false,
    },
    Button {
        file: "btn-npm.svg",
        label: "npm",
        warm: false,
    },
    Button {
        file: "btn-pypi.svg",
        label: "PyPI",
        warm: false,
    },
    Button {
        file: "btn-nuget.svg",
        label: "NuGet",
        warm: false,
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
            (
                format!(".github/badges/{}", button.file),
                svg(button.label, button.warm),
            )
        })
        .collect()
}

// The height of every button, and the type it carries.
const HEIGHT: f64 = 34.0;
const FONT_SIZE: f64 = 13.0;
const TRACKING: f64 = 0.3;
const PADDING: f64 = 15.0;

// The trailing mark on every label, which says the link leaves the page.
const ARROW: &str = "\u{2197}";

// One button. The width follows the label, so a long one is not cramped and a short one
// carries no dead space.
fn svg(label: &str, warm: bool) -> String {
    let text = format!("{label} {ARROW}");
    let width = (advance(&text) + PADDING * 2.0).round();
    let (from, via, to, ink) = if warm {
        (AMBER, CORAL, CORAL, ON_WARM)
    } else {
        (TEAL, TEAL, DEEP_TEAL, ON_COOL)
    };
    let mid = width / 2.0;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{HEIGHT}" viewBox="0 0 {width} {HEIGHT}" role="img" aria-label="{text}">
  <defs>
    <linearGradient id="fill" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="{from}"/>
      <stop offset="0.55" stop-color="{via}"/>
      <stop offset="1" stop-color="{to}"/>
    </linearGradient>
    <linearGradient id="sheen" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.28"/>
      <stop offset="1" stop-color="#ffffff" stop-opacity="0"/>
    </linearGradient>
  </defs>
  <rect x="0.5" y="0.5" width="{inner_w}" height="{inner_h}" rx="8" fill="url(#fill)"/>
  <rect x="1.5" y="1.5" width="{sheen_w}" height="15" rx="7" fill="url(#sheen)"/>
  <text x="{mid}" y="22" text-anchor="middle" font-family="Segoe UI, Inter, -apple-system, BlinkMacSystemFont, Helvetica, Arial, sans-serif" font-size="{FONT_SIZE}" font-weight="700" letter-spacing="{TRACKING}" fill="{ink}">{text}</text>
</svg>
"##,
        inner_w = width - 1.0,
        inner_h = HEIGHT - 1.0,
        sheen_w = width - 3.0,
    )
}

// The rendered width of a label, in the absence of a font engine. The button is centred
// text on a filled shape, so this only has to be close: too narrow crowds the label, too
// wide leaves the button looking empty. The ratios are for a bold humanist sans at the
// size above, with the tracking added per character.
fn advance(text: &str) -> f64 {
    text.chars()
        .map(|ch| {
            let ratio = match ch {
                'i' | 'j' | 'l' | 'I' | '.' | ',' | '\'' => 0.30,
                'f' | 'r' | 't' | '(' | ')' | '[' | ']' | '/' | ' ' => 0.40,
                'm' | 'M' | 'W' | 'w' => 0.90,
                'A'..='Z' => 0.68,
                _ => 0.58,
            };
            ratio * FONT_SIZE + TRACKING
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
        let short = svg("npm", false);
        let long = svg("dashboard demo", true);
        let width = |svg: &str| -> f64 {
            svg.split("width=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .and_then(|value| value.parse().ok())
                .expect("the root element carries a width")
        };
        assert!(width(&long) > width(&short));
    }

    #[test]
    fn a_warm_button_carries_the_logos_amber_and_a_cool_one_its_teal() {
        assert!(svg("website", true).contains(AMBER));
        assert!(svg("npm", false).contains(TEAL));
        assert!(!svg("npm", false).contains(AMBER));
    }

    #[test]
    fn every_label_says_the_link_leaves_the_page() {
        assert!(render()
            .iter()
            .all(|(_, svg)| svg.contains(ARROW) && svg.contains("role=\"img\"")));
    }
}
