//! One palette for every page of the site, in both schemes.
//!
//! The site is the pages `cargo xtask site` renders plus four generators' output: rustdoc,
//! typedoc, pdoc, and DocFX for the four references. Each generator ships its own theme, so
//! without intervention a reader crosses four visual identities in one click. No single
//! generator covers four languages well, since each understands its own type system, so
//! the fix is one palette with a thin adapter per tool, written in the variable names that
//! tool exposes, and the same palette as custom properties for the site's own stylesheet.
//! The palette is defined once here and emitted by `cargo xtask docs`, so the references
//! cannot drift from each other or from the site.
//!
//! Each scheme is written twice: once under `prefers-color-scheme`, so a reader who never
//! touches the control gets the sheet their system asks for, and once under `data-theme` on
//! the root element, which the control in the band sets and every generator here already
//! understands.

use std::fs;
use std::path::Path;

/// One scheme's inks. The sheet is white stock with a deep green vendor ink; the dark
/// scheme is the same sheet printed in reverse, and lightens the ink that carries a link
/// so it still reads against the ground.
pub(crate) struct Scheme {
    pub paper: &'static str,
    pub paper_tint: &'static str,
    pub ink: &'static str,
    pub ink_soft: &'static str,
    pub rule: &'static str,
    /// The heavy rule that frames a table or a figure. On the printed sheet that is the
    /// ink itself; on the reversed sheet a full-strength light keyline glares, so the dark
    /// scheme softens it toward the ground.
    pub frame: &'static str,
    pub band: &'static str,
    pub band_deep: &'static str,
    pub accent: &'static str,
    pub accent_deep: &'static str,
    pub caution: &'static str,
    pub caution_tint: &'static str,
    pub alarm: &'static str,
    pub code_string: &'static str,
    pub code_literal: &'static str,
}

/// The sheet as printed.
pub(crate) const LIGHT: Scheme = Scheme {
    paper: "#fbfaf6",
    paper_tint: "#f1efe7",
    ink: "#141414",
    ink_soft: "#585856",
    rule: "#d7d4cc",
    frame: "#141414",
    band: "#0f5f56",
    band_deep: "#0a463f",
    accent: "#0f5f56",
    accent_deep: "#0a463f",
    caution: "#8a5a00",
    caution_tint: "#fff4d6",
    alarm: "#a5321a",
    code_string: "#1f5c8b",
    code_literal: "#8a3a12",
};

/// The same sheet in reverse.
pub(crate) const DARK: Scheme = Scheme {
    paper: "#191a17",
    paper_tint: "#22231f",
    ink: "#eae8e1",
    ink_soft: "#a8a69d",
    rule: "#3a3b35",
    frame: "#82857b",
    band: "#0d4d46",
    band_deep: "#0a3b36",
    accent: "#5ec6b0",
    accent_deep: "#8ad9c7",
    caution: "#e8b44a",
    caution_tint: "#2b2415",
    alarm: "#ef8368",
    code_string: "#8dc0e0",
    code_literal: "#e8a87c",
};

/// The site's typefaces, served from the site itself (`web/fonts/`), so no page reaches
/// a font host. Absolute, since the generated references load it from any depth.
pub(crate) const FONTS: &str = "/fonts/fonts.css";
/// The site's tokens, which the bar over a generated reference is drawn in.
const TOKENS: &str = "/theme.css";
/// The bar's stylesheet, and the script that draws it.
const BAR: &str = "/reference.css";
const BAR_SCRIPT: &str = "/js/reference.js";
const SANS: &str = "'Archivo', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif";
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

/// A color with an alpha, as CSS. The schemes are hex; the rules and shading the site
/// draws are an ink at a fraction of its strength, which needs `rgba()`.
///
/// # Arguments
///
/// * `hex` - a `#rrggbb` color from a scheme.
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
    let digits = hex.strip_prefix('#').expect("a # color");
    assert_eq!(digits.len(), 6, "{hex} is not #rrggbb");
    let channel = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).expect("hex digits");
    format!(
        "rgba({}, {}, {}, {alpha})",
        channel(0),
        channel(2),
        channel(4)
    )
}

// One scheme as the custom properties the site's own stylesheets read.
fn properties(s: &Scheme) -> String {
    format!(
        "  --paper: {paper};
  --paper-tint: {paper_tint};
  --ink: {ink};
  --ink-soft: {ink_soft};
  --rule: {rule};
  --rule-soft: {rule_soft};
  --frame: {frame};
  --band: {band};
  --band-deep: {band_deep};
  --band-tint: {band_tint};
  --on-band: #ffffff;
  --accent: {accent};
  --accent-deep: {accent_deep};
  --caution: {caution};
  --caution-tint: {caution_tint};
  --alarm: {alarm};
  --code-string: {code_string};
  --code-literal: {code_literal};
",
        paper = s.paper,
        paper_tint = s.paper_tint,
        ink = s.ink,
        ink_soft = s.ink_soft,
        rule = s.rule,
        rule_soft = rgba(s.ink, 0.08),
        frame = s.frame,
        band = s.band,
        band_deep = s.band_deep,
        band_tint = rgba(s.accent, 0.1),
        accent = s.accent,
        accent_deep = s.accent_deep,
        caution = s.caution,
        caution_tint = s.caution_tint,
        alarm = s.alarm,
        code_string = s.code_string,
        code_literal = s.code_literal,
    )
}

// The site's own stylesheets read the schemes from these custom properties and nothing
// else, so the pages `cargo xtask site` renders share one source of color with the four
// references. The light sheet is the sheet; the dark one is a choice the control on the
// page writes as `data-theme`, which every generator here already reads for its own theme.
fn tokens() -> String {
    format!(
        "/* Generated by `cargo xtask docs` from crates/xtask/src/theme.rs: the site palette as custom properties. Edit the palette there, not this file. */
:root {{
  color-scheme: light dark;
{light}  --sans: {sans};
  --mono: {mono};
}}
@media (prefers-color-scheme: dark) {{
  :root:not([data-theme=\"light\"]) {{
    color-scheme: dark;
{dark}  }}
}}
:root[data-theme=\"dark\"], :root[data-theme=\"ayu\"] {{
  color-scheme: dark;
{dark}}}
:root[data-theme=\"light\"] {{
  color-scheme: light;
{light}}}
",
        light = properties(&LIGHT),
        dark = properties(&DARK),
        sans = SANS,
        mono = MONO,
    )
}

// Thin scrollbars in the palette, on every page and in every generated reference, so the
// browser's default bar does not sit on the sheet as a strip of chrome. The standard
// properties are set for the browsers that have them and the WebKit pseudo elements for
// the rest; a browser that has both uses the standard ones. Both read the tokens, so a
// bar follows the scheme with everything else.
fn scrollbars() -> String {
    "\n* { scrollbar-width: thin; }\nhtml { scrollbar-color: color-mix(in srgb, var(--ink) 26%, transparent) transparent; }\n::-webkit-scrollbar { width: 8px; height: 8px; }\n::-webkit-scrollbar-track { background: transparent; }\n::-webkit-scrollbar-thumb { background: color-mix(in srgb, var(--ink) 26%, transparent); border-radius: 8px; }\n::-webkit-scrollbar-thumb:hover { background: color-mix(in srgb, var(--ink) 45%, transparent); }\n".to_owned()
}

// The shared header every stylesheet starts with: the provenance, the typefaces, the
// site's tokens, and the bar `web/js/reference.js` draws over the generator's chrome.
fn banner(tool: &str, stamp: &str) -> String {
    format!(
        "/* Generated by `cargo xtask docs` from crates/xtask/src/theme.rs: the site palette in {tool}'s own variables. Edit the palette there, not this file. */\n@import url('{FONTS}?v={stamp}');\n@import url('{TOKENS}?v={stamp}');\n@import url('{BAR}?v={stamp}');\n\n"
    )
}

// Every generator names its own variables, and every one of them is set from the tokens
// rather than from a color, so a reference follows the scheme the reader chose on the site.
const SHARED: &str = "body { font-family: var(--sans); }
h1, h2, h3, h4 { font-family: var(--sans); letter-spacing: -0.01em; }
code, pre { font-family: var(--mono); }
";

// rustdoc: the Rust reference. It takes a fragment for the document head, so this is a
// stylesheet link and a style block rather than a stylesheet on its own. rustdoc writes
// its own `data-theme`, which the token sheet reads, so its switcher moves the whole page.
fn rustdoc(stamp: &str) -> String {
    format!(
        "<!-- Generated by `cargo xtask docs` from crates/xtask/src/theme.rs: the site palette in rustdoc's own variables. Edit the palette there, not this file. -->
<link rel=\"stylesheet\" href=\"{FONTS}?v={stamp}\">
<link rel=\"stylesheet\" href=\"{TOKENS}?v={stamp}\">
<link rel=\"stylesheet\" href=\"{BAR}?v={stamp}\">
<script src=\"{BAR_SCRIPT}?v={stamp}\" defer></script>
<style>
:root, :root[data-theme=\"light\"], :root[data-theme=\"dark\"], :root[data-theme=\"ayu\"] {{
  --main-background-color: var(--paper);
  --main-color: var(--ink);
  --link-color: var(--accent);
  --sidebar-background-color: var(--paper-tint);
  --sidebar-background-color-hover: var(--band-tint);
  --code-block-background-color: var(--paper-tint);
  --headings-border-bottom-color: var(--rule);
  --border-color: var(--rule);
  --scrollbar-thumb-background-color: color-mix(in srgb, var(--ink) 26%, transparent);
  --search-input-focused-border-color: var(--accent);
  --copy-path-button-color: var(--ink-soft);
  --copy-path-img-hover-filter: none;
  --code-highlight-kw-color: var(--accent);
  --code-highlight-kw-2-color: var(--accent);
  --code-highlight-lifetime-color: var(--code-literal);
  --code-highlight-prelude-color: var(--ink);
  --code-highlight-prelude-val-color: var(--code-literal);
  --code-highlight-number-color: var(--code-literal);
  --code-highlight-string-color: var(--code-string);
  --code-highlight-literal-color: var(--code-literal);
  --code-highlight-attribute-color: var(--ink-soft);
  --code-highlight-self-color: var(--accent);
  --code-highlight-macro-color: var(--code-literal);
  --code-highlight-question-mark-color: var(--alarm);
  --code-highlight-comment-color: var(--ink-soft);
  --code-highlight-doc-comment-color: var(--ink-soft);
}}
{SHARED}a:hover {{ color: var(--accent-deep); }}
.code-header {{ font-family: var(--mono); }}
</style>
"
    )
}

// typedoc: the TypeScript reference. Its variables are set on the root element, and it
// writes `data-theme` itself, so both schemes come from the tokens.
fn typedoc(stamp: &str) -> String {
    format!(
        "{}\
:root, :root[data-theme=\"light\"], :root[data-theme=\"dark\"] {{
  --color-background: var(--paper);
  --color-background-secondary: var(--paper-tint);
  --color-background-active: var(--band-tint);
  --color-text: var(--ink);
  --color-text-aside: var(--ink-soft);
  --color-link: var(--accent);
  --color-accent: var(--rule);
  --color-active-menu-item: var(--band-tint);
  --color-focus-outline: var(--accent);
  --color-ts-project: var(--accent);
  --color-ts-class: var(--accent);
  --color-ts-interface: var(--accent);
  --color-ts-function: var(--accent);
}}
{SHARED}.tsd-page-title h1 {{ font-family: var(--sans); }}
.tsd-signature {{ font-family: var(--mono); }}
.tsd-toolbar-contents .title::before {{
  content: \"\";
  display: inline-block;
  width: 24px;
  height: 24px;
  margin-right: 0.5rem;
  vertical-align: -5px;
  background: url(\"../../../assets/pamoja-icon.svg\") no-repeat center / contain;
}}
a:hover {{ color: var(--accent-deep); }}
",
        banner("typedoc", stamp),
    )
}

// pdoc: the Python reference. It picks up a `custom.css` from the template directory and
// includes it after its own stylesheets, so this is a plain stylesheet like the others.
// pdoc has no switcher of its own, so it follows the system and the site's control.
fn pdoc(stamp: &str) -> String {
    format!(
        "{}\
:root {{ --pdoc-background: var(--paper); }}
.pdoc {{
  --text: var(--ink);
  --muted: var(--ink-soft);
  --link: var(--accent);
  --link-hover: var(--accent-deep);
  --code: var(--paper-tint);
  --active: var(--band-tint);
  --accent: var(--paper-tint);
  --accent2: var(--rule);
  --nav-hover: var(--band-tint);
  --name: var(--ink);
  --def: var(--accent);
  --annotation: var(--ink-soft);
}}
html, body {{ background: var(--paper); color: var(--ink); }}
{SHARED}.pdoc h1, .pdoc h2, .pdoc h3 {{ font-family: var(--sans); letter-spacing: -0.01em; }}
.pdoc code, .pdoc pre {{ font-family: var(--mono); }}
",
        banner("pdoc", stamp),
    )
}

// DocFX: the C# reference. The modern template is Bootstrap, and ships `public/main.css`
// as the place a site puts its overrides; a template folder listed after `modern` layers
// this file over it. DocFX writes `data-bs-theme` rather than `data-theme`, so its own
// switcher is mirrored onto the tokens here.
fn docfx(stamp: &str) -> String {
    format!(
        "{}\
:root, [data-bs-theme=\"light\"], [data-bs-theme=\"dark\"] {{
  --bs-body-bg: var(--paper);
  --bs-body-color: var(--ink);
  --bs-secondary-color: var(--ink-soft);
  --bs-tertiary-bg: var(--paper-tint);
  --bs-secondary-bg: var(--paper-tint);
  --bs-emphasis-color: var(--ink);
  --bs-link-color: var(--accent);
  --bs-link-hover-color: var(--accent-deep);
  --bs-border-color: var(--rule);
  --bs-code-color: var(--ink);
  --bs-warning-border-subtle: var(--caution);
  --bs-font-sans-serif: var(--sans);
  --bs-font-monospace: var(--mono);
}}
[data-bs-theme=\"dark\"] {{
{dark}}}
[data-bs-theme=\"light\"] {{
{light}}}
{SHARED}.navbar-brand {{ font-family: var(--sans); letter-spacing: -0.01em; }}
.navbar-brand #logo {{ height: 30px; width: auto; margin-right: 0.35rem; }}
pre, code {{ background: var(--paper-tint); }}
.navbar, .toc {{ background: var(--paper) !important; border-bottom: 1px solid var(--rule); }}
",
        banner("DocFX", stamp),
        dark = properties(&DARK),
        light = properties(&LIGHT),
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
            assert!(
                body.contains("var(--accent)") || body.contains(LIGHT.accent),
                "{path} lacks the vendor ink"
            );
            assert!(
                body.contains("var(--paper)") || body.contains(LIGHT.paper),
                "{path} lacks the sheet"
            );
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
    fn the_token_sheet_names_every_color_and_the_alpha_helper_reads_the_palette() {
        let tokens = &render_stamped("f00dcafe")[0];
        assert_eq!(tokens.0, "web/theme.css");
        for name in [
            "--paper",
            "--paper-tint",
            "--ink",
            "--ink-soft",
            "--rule",
            "--rule-soft",
            "--band",
            "--band-deep",
            "--band-tint",
            "--caution",
            "--caution-tint",
            "--alarm",
            "--code-string",
            "--code-literal",
            "--sans",
            "--mono",
        ] {
            assert!(
                tokens.1.contains(&format!("{name}: ")),
                "theme.css lacks {name}"
            );
        }
        assert!(!tokens.1.contains("fonts.css"));
        assert!(
            tokens.1.contains(&format!("--band: {};", LIGHT.band))
                && tokens.1.contains(&format!("--paper: {};", LIGHT.paper))
        );
        assert!(
            tokens.1.contains("prefers-color-scheme: dark"),
            "a reader who never touches the control gets the sheet their system asks for"
        );
        assert!(
            tokens.1.contains("[data-theme=\"dark\"]") && tokens.1.contains(DARK.paper),
            "the dark sheet is there for a reader who asks for it"
        );
        assert!(
            tokens.1.contains("[data-theme=\"light\"]"),
            "a reader can come back to the light sheet"
        );
        assert_eq!(rgba("#fbf3e4", 0.1), "rgba(251, 243, 228, 0.1)");
    }
}
