//! One palette for every documentation site.
//!
//! The site is the pages `cargo xtask site` renders plus four generators' output: rustdoc,
//! typedoc, pdoc, and DocFX for the four references. Each generator ships its own theme, so
//! without intervention a reader crosses four visual identities in one click. No single
//! generator covers four languages well, since each understands its own type system, so
//! the fix is one palette with a thin adapter per tool, written in the variable names that
//! tool exposes, and the same palette as custom properties for the site's own stylesheet.
//! The palette is the showcase's, defined once here and emitted by `cargo xtask docs`, so
//! the sites cannot drift from each other or from the showcase.

use std::fs;
use std::path::Path;

/// The showcase's palette, which the documentation shares.
pub(crate) struct Palette {
    pub navy_0: &'static str,
    pub navy_1: &'static str,
    pub navy_2: &'static str,
    pub amber: &'static str,
    pub coral: &'static str,
    pub teal: &'static str,
    pub sky: &'static str,
    pub forest: &'static str,
    pub cream: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
}

pub(crate) const PALETTE: Palette = Palette {
    navy_0: "#0a1322",
    navy_1: "#0e1b2e",
    navy_2: "#16263f",
    amber: "#ffb627",
    coral: "#f26a4b",
    teal: "#1fa995",
    sky: "#36b6dd",
    forest: "#46c97e",
    cream: "#fbf3e4",
    text: "#e7eef8",
    muted: "#95a7be",
};

/// The site's typefaces, served from the site itself (`web/fonts/`), so no page reaches
/// a font host. Absolute, since the generated references load it from any depth.
pub(crate) const FONTS: &str = "/fonts/fonts.css";
/// The site's tokens, which the bar over a generated reference is drawn in.
const TOKENS: &str = "/theme.css";
/// The bar's stylesheet, and the script that draws it.
const BAR: &str = "/reference.css";
const BAR_SCRIPT: &str = "/js/reference.js";
const SANS: &str = "'Inter', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif";
const DISPLAY: &str = "'Sora', 'Inter', system-ui, sans-serif";
const MONO: &str = "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";

/// Render every adapter as (path, contents).
///
/// # Arguments
///
/// * `root` - the repository root, holding the bar's stylesheet and script under `web/`.
///
/// # Returns
///
/// The site's own tokens, then one file per generator: rustdoc's header fragment, typedoc's
/// custom stylesheet, pdoc's custom stylesheet, and DocFX's template stylesheet. Each names
/// the site's files with a stamp of their contents, so a browser that cached the last
/// deploy's copy fetches the new one.
///
/// # Errors
///
/// When the bar's stylesheet or script cannot be read.
pub fn render(root: &Path) -> Result<Vec<(String, String)>, String> {
    let mut stamped = tokens().into_bytes();
    for file in [
        "web/reference.css",
        "web/js/reference.js",
        "web/fonts/fonts.css",
    ] {
        let path = root.join(file);
        stamped
            .extend(fs::read(&path).map_err(|err| format!("reading {}: {err}", path.display()))?);
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in stamped {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    Ok(render_stamped(&format!(
        "{:08x}",
        (hash >> 32) ^ (hash & 0xffff_ffff)
    )))
}

/// The adapters with a given stamp on the site's files.
fn render_stamped(stamp: &str) -> Vec<(String, String)> {
    vec![
        ("web/theme.css".to_owned(), tokens()),
        (
            "docs/theme/rustdoc.html".to_owned(),
            format!("{}<style>\n{}</style>\n", rustdoc(stamp), scrollbars()),
        ),
        (
            "docs/theme/typedoc.css".to_owned(),
            typedoc(stamp) + &scrollbars(),
        ),
        (
            "docs/theme/pdoc/custom.css".to_owned(),
            pdoc(stamp) + &scrollbars(),
        ),
        (
            "bindings/dotnet/docs/templates/pamoja/public/main.css".to_owned(),
            docfx(stamp) + &scrollbars(),
        ),
    ]
}

/// A palette colour with an alpha, as CSS. The palette is hex; the lines and glass the
/// site draws are the cream at a fraction of its strength, which needs `rgba()`.
///
/// # Arguments
///
/// * `hex` - a `#rrggbb` colour from the palette.
/// * `alpha` - the opacity, 0 to 1.
///
/// # Returns
///
/// `rgba(r, g, b, alpha)`.
///
/// # Panics
///
/// When `hex` is not six hex digits behind a `#`, which only a palette edit could cause.
pub(crate) fn rgba(hex: &str, alpha: f32) -> String {
    let digits = hex.strip_prefix('#').expect("a # colour");
    assert_eq!(digits.len(), 6, "{hex} is not #rrggbb");
    let channel = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).expect("hex digits");
    format!(
        "rgba({}, {}, {}, {alpha})",
        channel(0),
        channel(2),
        channel(4)
    )
}

// The site's own stylesheet reads the palette from these custom properties and nothing
// else, so the pages `cargo xtask site` renders share one source of colour with the
// showcase and the four references.
fn tokens() -> String {
    let p = &PALETTE;
    format!(
        "/* Generated by `cargo xtask docs` from crates/xtask/src/theme.rs: the showcase palette as custom properties. Edit the palette there, not this file. */
:root {{
  --navy-0: {navy0};
  --navy-1: {navy1};
  --navy-2: {navy2};
  --amber: {amber};
  --coral: {coral};
  --teal: {teal};
  --sky: {sky};
  --forest: {forest};
  --cream: {cream};
  --text: {text};
  --muted: {muted};
  --line: {line};
  --line-strong: {line_strong};
  --glass: {glass};
  --on-warm: #2a1606;
  --sans: {sans};
  --display: {display};
  --mono: {mono};
}}
",
        navy0 = p.navy_0,
        navy1 = p.navy_1,
        navy2 = p.navy_2,
        amber = p.amber,
        coral = p.coral,
        teal = p.teal,
        sky = p.sky,
        forest = p.forest,
        cream = p.cream,
        text = p.text,
        muted = p.muted,
        line = rgba(p.cream, 0.1),
        line_strong = rgba(p.cream, 0.22),
        glass = rgba(p.navy_0, 0.72),
        sans = SANS,
        display = DISPLAY,
        mono = MONO,
    )
}

// Thin scrollbars in the palette, on every page and in every generated reference, so the
// browser's default bar does not sit on the dark ground like a strip of daylight. The
// standard properties are set for the browsers that have them and the WebKit pseudo
// elements for the rest; a browser that has both uses the standard ones.
fn scrollbars() -> String {
    let thumb = rgba(PALETTE.cream, 0.18);
    let hover = rgba(PALETTE.cream, 0.32);
    format!(
        "\n* {{ scrollbar-width: thin; }}\nhtml {{ scrollbar-color: {thumb} transparent; }}\n::-webkit-scrollbar {{ width: 8px; height: 8px; }}\n::-webkit-scrollbar-track {{ background: transparent; }}\n::-webkit-scrollbar-thumb {{ background: {thumb}; border-radius: 8px; }}\n::-webkit-scrollbar-thumb:hover {{ background: {hover}; }}\n"
    )
}

// The shared header every stylesheet starts with: the provenance, the typefaces, the
// site's tokens, and the bar `web/js/reference.js` draws over the generator's chrome.
fn banner(tool: &str, stamp: &str) -> String {
    format!(
        "/* Generated by `cargo xtask docs` from crates/xtask/src/theme.rs: the showcase palette in {tool}'s own variables. Edit the palette there, not this file. */\n@import url('{FONTS}?v={stamp}');\n@import url('{TOKENS}?v={stamp}');\n@import url('{BAR}?v={stamp}');\n\n"
    )
}

// rustdoc: the Rust reference. It takes a fragment for the document head, so this is a
// stylesheet link and a style block rather than a stylesheet on its own.
fn rustdoc(stamp: &str) -> String {
    let p = &PALETTE;
    format!(
        "<!-- Generated by `cargo xtask docs` from crates/xtask/src/theme.rs: the showcase palette in rustdoc's own variables. Edit the palette there, not this file. -->
<link rel=\"stylesheet\" href=\"{FONTS}?v={stamp}\">
<link rel=\"stylesheet\" href=\"{TOKENS}?v={stamp}\">
<link rel=\"stylesheet\" href=\"{BAR}?v={stamp}\">
<script src=\"{BAR_SCRIPT}?v={stamp}\" defer></script>
<style>
:root[data-theme=\"dark\"] {{
  --main-background-color: {navy1};
  --main-color: {text};
  --link-color: {teal};
  --sidebar-background-color: {navy0};
  --code-block-background-color: {navy0};
}}
:root[data-theme=\"light\"] {{
  --link-color: {teal};
}}
body {{ font-family: {sans}; }}
h1, h2, h3, h4 {{ font-family: {display}; }}
code, pre, .code-header {{ font-family: {mono}; }}
:root[data-theme=\"dark\"] a:hover {{ color: {amber}; }}
</style>
",
        navy0 = p.navy_0,
        navy1 = p.navy_1,
        amber = p.amber,
        teal = p.teal,
        text = p.text,
        sans = SANS,
        display = DISPLAY,
        mono = MONO,
    )
}

// typedoc: the TypeScript reference. Its variables are set per colour scheme on the root
// element; the dark set is the showcase, and the light set keeps the showcase's accents.
fn typedoc(stamp: &str) -> String {
    let p = &PALETTE;
    format!(
        "{}\
@media (prefers-color-scheme: dark) {{
  :root:not([data-theme=\"light\"]) {{
    --color-background: {navy1};
    --color-background-secondary: {navy0};
    --color-background-active: {navy2};
    --color-text: {text};
    --color-text-aside: {muted};
    --color-link: {teal};
    --color-accent: {navy2};
    --color-active-menu-item: {navy2};
    --color-focus-outline: {amber};
  }}
}}
:root[data-theme=\"dark\"] {{
  --color-background: {navy1};
  --color-background-secondary: {navy0};
  --color-background-active: {navy2};
  --color-text: {text};
  --color-text-aside: {muted};
  --color-link: {teal};
  --color-accent: {navy2};
  --color-active-menu-item: {navy2};
  --color-focus-outline: {amber};
}}
:root {{ --color-link: {teal}; --color-focus-outline: {amber}; }}
body {{ font-family: {sans}; }}
h1, h2, h3, h4, .tsd-page-title h1 {{ font-family: {display}; letter-spacing: -0.015em; }}
.tsd-toolbar-contents .title::before {{
  content: \"\";
  display: inline-block;
  width: 24px;
  height: 24px;
  margin-right: 0.5rem;
  vertical-align: -5px;
  background: url(\"../../../assets/pamoja-icon.svg\") no-repeat center / contain;
}}
code, pre, .tsd-signature {{ font-family: {mono}; }}
a:hover {{ color: {amber}; }}
",
        banner("typedoc", stamp),
        navy0 = p.navy_0,
        navy1 = p.navy_1,
        navy2 = p.navy_2,
        amber = p.amber,
        teal = p.teal,
        text = p.text,
        muted = p.muted,
        sans = SANS,
        display = DISPLAY,
        mono = MONO,
    )
}

// pdoc: the Python reference. It picks up a `custom.css` from the template directory and
// includes it after its own stylesheets, so this is a plain stylesheet like the others.
fn pdoc(stamp: &str) -> String {
    let p = &PALETTE;
    format!(
        "{}\
:root {{ --pdoc-background: {navy1}; }}
.pdoc {{
  --text: {text};
  --muted: {muted};
  --link: {teal};
  --link-hover: {amber};
  --code: {navy0};
  --active: {navy2};
  --accent: {navy2};
  --accent2: rgba(251, 243, 228, 0.22);
  --nav-hover: {navy2};
  --name: {cream};
  --def: {teal};
  --annotation: {muted};
}}
body, .pdoc {{ font-family: {sans}; }}
.pdoc h1, .pdoc h2, .pdoc h3 {{ font-family: {display}; letter-spacing: -0.015em; }}
.pdoc code, .pdoc pre {{ font-family: {mono}; }}
",
        banner("pdoc", stamp),
        navy0 = p.navy_0,
        navy1 = p.navy_1,
        navy2 = p.navy_2,
        amber = p.amber,
        teal = p.teal,
        cream = p.cream,
        text = p.text,
        muted = p.muted,
        sans = SANS,
        display = DISPLAY,
        mono = MONO,
    )
}

// DocFX: the C# reference. The modern template is Bootstrap, and ships `public/main.css`
// as the place a site puts its overrides; a template folder listed after `modern` layers
// this file over it.
fn docfx(stamp: &str) -> String {
    let p = &PALETTE;
    format!(
        "{}\
[data-bs-theme=\"dark\"] {{
  --bs-body-bg: {navy1};
  --bs-body-color: {text};
  --bs-secondary-color: {muted};
  --bs-tertiary-bg: {navy0};
  --bs-secondary-bg: {navy2};
  --bs-link-color: {teal};
  --bs-link-color-rgb: 31, 169, 149;
  --bs-link-hover-color: {amber};
  --bs-link-hover-color-rgb: 255, 182, 39;
  --bs-border-color: rgba(251, 243, 228, 0.16);
  --bs-code-color: {cream};
  --bs-warning-border-subtle: {coral};
}}
:root {{
  --bs-link-color: {teal};
  --bs-link-color-rgb: 31, 169, 149;
  --bs-link-hover-color: {amber};
  --bs-link-hover-color-rgb: 255, 182, 39;
  --bs-font-sans-serif: {sans};
  --bs-font-monospace: {mono};
}}
h1, h2, h3, h4, .navbar-brand {{ font-family: {display}; letter-spacing: -0.015em; }}
.navbar-brand #logo {{ height: 30px; width: auto; margin-right: 0.35rem; }}
[data-bs-theme=\"dark\"] pre, [data-bs-theme=\"dark\"] code {{ background: {navy0}; }}
[data-bs-theme=\"dark\"] .navbar, [data-bs-theme=\"dark\"] .toc {{ background: {navy0} !important; }}
",
        banner("DocFX", stamp),
        navy0 = p.navy_0,
        navy1 = p.navy_1,
        navy2 = p.navy_2,
        amber = p.amber,
        teal = p.teal,
        cream = p.cream,
        coral = p.coral,
        text = p.text,
        muted = p.muted,
        sans = SANS,
        display = DISPLAY,
        mono = MONO,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_adapter_carries_the_palette_and_names_its_generator() {
        for (path, body) in render_stamped("f00dcafe") {
            assert!(
                body.contains("crates/xtask/src/theme.rs"),
                "{path} lacks its provenance"
            );
            assert!(body.contains(PALETTE.teal), "{path} lacks the accent");
            assert!(body.contains(PALETTE.navy_1), "{path} lacks the ground");
        }
    }

    #[test]
    fn the_head_fragment_is_a_fragment_and_every_generator_stylesheet_imports_the_fonts() {
        let files = render_stamped("f00dcafe");
        let rustdoc = &files
            .iter()
            .find(|(p, _)| p.ends_with("rustdoc.html"))
            .unwrap()
            .1;
        assert!(rustdoc.starts_with("<!--") && !rustdoc.contains("<html"));
        // The site's own pages link the typefaces from the page head instead, so the token
        // sheet stays tokens.
        for (path, body) in files
            .iter()
            .filter(|(p, _)| p.ends_with(".css") && p != "web/theme.css")
        {
            assert!(
                body.contains("/fonts/fonts.css") && !body.contains("googleapis"),
                "{path} does not load the typefaces from the site"
            );
        }
    }

    #[test]
    fn every_generated_reference_carries_the_site_bar() {
        for (path, body) in render_stamped("f00dcafe")
            .iter()
            .filter(|(p, _)| p != "web/theme.css")
        {
            assert!(
                body.contains(BAR) && body.contains(TOKENS),
                "{path} does not load the site bar"
            );
        }
        let rustdoc = &render_stamped("f00dcafe")[1].1;
        assert!(
            rustdoc.contains(BAR_SCRIPT),
            "rustdoc does not load the bar script"
        );
    }

    #[test]
    fn every_generator_gets_thin_scrollbars_in_the_palette() {
        for (path, body) in render_stamped("f00dcafe")
            .iter()
            .filter(|(p, _)| p != "web/theme.css")
        {
            assert!(
                body.contains("scrollbar-width: thin")
                    && body.contains("::-webkit-scrollbar-thumb"),
                "{path} keeps the default scrollbars"
            );
        }
    }

    #[test]
    fn the_token_sheet_names_every_colour_and_the_alpha_helper_reads_the_palette() {
        let tokens = &render_stamped("f00dcafe")[0];
        assert_eq!(tokens.0, "web/theme.css");
        for name in [
            "--navy-0",
            "--navy-1",
            "--navy-2",
            "--amber",
            "--coral",
            "--teal",
            "--cream",
            "--text",
            "--muted",
            "--line",
            "--line-strong",
            "--sans",
            "--display",
            "--mono",
        ] {
            assert!(
                tokens.1.contains(&format!("{name}: ")),
                "theme.css lacks {name}"
            );
        }
        assert!(!tokens.1.contains("fonts.css"));
        assert!(tokens.1.contains("--sky: #36b6dd;") && tokens.1.contains("--forest: #46c97e;"));
        assert_eq!(rgba("#fbf3e4", 0.1), "rgba(251, 243, 228, 0.1)");
        assert_eq!(rgba(PALETTE.navy_0, 0.5), "rgba(10, 19, 34, 0.5)");
    }
}
