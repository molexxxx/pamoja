//! Syntax highlighting at build time.
//!
//! Every code block on the site is coloured when the site is rendered, so a page carries
//! plain spans and the browser runs nothing to colour them. syntect drives the grammars.
//! The theme it is handed is not a colour scheme but a set of sentinels, one per class, so
//! each token comes out tagged with which of a handful of classes it belongs to, and
//! `site.css` maps those classes to the palette. That keeps the HTML small (one span per
//! run of tokens sharing a class, no nesting) and keeps colour in the stylesheet, where the
//! rest of the site's colour lives.
//!
//! The syntaxes are syntect's defaults with two-face's extras on top, which is what brings
//! TypeScript and TOML; the default set has neither.

use std::str::FromStr;
use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings,
};
use syntect::parsing::SyntaxSet;

/// The classes a token can carry, each with the scope selectors that place a token in it.
/// syntect applies every matching selector in order of specificity, so `keyword.operator`
/// wins over the broader `keyword` wherever both match.
const CLASSES: &[(&str, &str)] = &[
    (
        "kw",
        "keyword, storage.type, storage.modifier, variable.language",
    ),
    ("op", "keyword.operator"),
    (
        "ty",
        "entity.name.type, entity.name.class, entity.name.struct, entity.name.enum, \
         entity.name.trait, entity.name.namespace, entity.name.interface, support.type, \
         support.class, entity.other.inherited-class",
    ),
    (
        "fn",
        "entity.name.function, support.function, variable.function, meta.function-call",
    ),
    ("mac", "support.macro, entity.name.macro"),
    ("str", "string"),
    (
        "num",
        "constant.numeric, constant.language, constant.character, constant.other",
    ),
    ("cmt", "comment, punctuation.definition.comment"),
    (
        "attr",
        "meta.attribute, entity.other.attribute-name, meta.annotation, variable.annotation",
    ),
];

struct Engine {
    syntaxes: SyntaxSet,
    theme: Theme,
}

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine {
        syntaxes: two_face::syntax::extra_newlines(),
        theme: sentinel_theme(),
    })
}

// A theme whose only job is to say which class a token has: the red channel of the
// foreground carries the class index plus one, and zero means plain text.
fn sentinel_theme() -> Theme {
    let scopes = CLASSES
        .iter()
        .enumerate()
        .map(|(index, (_, selectors))| ThemeItem {
            scope: ScopeSelectors::from_str(selectors).expect("a valid scope selector list"),
            style: StyleModifier {
                foreground: Some(sentinel(index + 1)),
                background: None,
                font_style: None,
            },
        })
        .collect();
    Theme {
        name: None,
        author: None,
        settings: ThemeSettings {
            foreground: Some(sentinel(0)),
            ..ThemeSettings::default()
        },
        scopes,
    }
}

fn sentinel(index: usize) -> Color {
    Color {
        r: u8::try_from(index).expect("fewer than 256 classes"),
        g: 0,
        b: 0,
        a: 255,
    }
}

/// The token syntect looks a language up by, from the name a code fence uses.
fn token(lang: &str) -> &str {
    match lang {
        "csharp" | "cs" | "c#" => "cs",
        "shell" | "bash" | "console" | "zsh" => "sh",
        "js" => "javascript",
        "ts" => "typescript",
        "yml" => "yaml",
        other => other,
    }
}

/// Highlight `code` written in `lang` as escaped HTML with one `<span class="hl-...">` per
/// run of tokens that share a class; plain tokens carry no span.
///
/// # Arguments
///
/// * `code` - the source text, without a trailing fence.
/// * `lang` - the fence's language name; empty, `text`, or a name no grammar covers comes
///   back escaped and otherwise untouched.
///
/// # Returns
///
/// The HTML for the inside of a `<code>` element.
pub fn highlight(code: &str, lang: &str) -> String {
    let engine = engine();
    let Some(syntax) = (!lang.is_empty() && lang != "text")
        .then(|| engine.syntaxes.find_syntax_by_token(token(lang)))
        .flatten()
    else {
        return escape(code);
    };

    let mut lines = HighlightLines::new(syntax, &engine.theme);
    let mut out = String::with_capacity(code.len() * 2);
    for line in with_endings(code) {
        match lines.highlight_line(line, &engine.syntaxes) {
            Ok(tokens) => {
                let mut current: Option<usize> = None;
                let mut run = String::new();
                for (style, text) in tokens {
                    let class = usize::from(style.foreground.r).checked_sub(1);
                    if class != current && !run.is_empty() {
                        flush(&mut out, current, &run);
                        run.clear();
                    }
                    current = class;
                    // The line ending stays outside any span, so a comment or string that
                    // runs to the end of a line closes before the break.
                    match text.strip_suffix('\n') {
                        Some(body) => {
                            run.push_str(body);
                            flush(&mut out, current, &run);
                            run.clear();
                            out.push('\n');
                        }
                        None => run.push_str(text),
                    }
                }
                flush(&mut out, current, &run);
            }
            Err(_) => out.push_str(&escape(line)),
        }
    }
    out
}

fn flush(out: &mut String, class: Option<usize>, text: &str) {
    if text.is_empty() {
        return;
    }
    match class.and_then(|index| CLASSES.get(index)) {
        Some((name, _)) => {
            out.push_str("<span class=\"hl-");
            out.push_str(name);
            out.push_str("\">");
            out.push_str(&escape(text));
            out.push_str("</span>");
        }
        None => out.push_str(&escape(text)),
    }
}

// Every line with its line ending kept, the way the newline grammars expect their input.
fn with_endings(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let end = rest.find('\n').map_or(rest.len(), |at| at + 1);
        let (line, tail) = rest.split_at(end);
        rest = tail;
        Some(line)
    })
}

/// Escape text for an HTML text node or a double-quoted attribute.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_the_guides_use_has_a_grammar() {
        let syntaxes = &engine().syntaxes;
        for lang in [
            "rust",
            "typescript",
            "python",
            "csharp",
            "sh",
            "bash",
            "toml",
            "json",
            "javascript",
        ] {
            assert!(
                syntaxes.find_syntax_by_token(token(lang)).is_some(),
                "no grammar for {lang}"
            );
        }
    }

    #[test]
    fn tokens_are_tagged_by_class_and_plain_text_is_left_alone() {
        let html = highlight("let count = 1; // one\n", "rust");
        assert!(html.contains("<span class=\"hl-kw\">let</span>"), "{html}");
        assert!(html.contains("<span class=\"hl-num\">1</span>"), "{html}");
        assert!(
            html.contains("<span class=\"hl-cmt\">// one</span>"),
            "{html}"
        );
        assert!(html.contains(" count "), "{html}");

        let ts = highlight("const name = \"pump-3\"\n", "typescript");
        assert!(ts.contains("<span class=\"hl-kw\">const</span>"), "{ts}");
        assert!(ts.contains("hl-str"), "{ts}");
    }

    #[test]
    fn unknown_and_plain_languages_are_only_escaped() {
        assert_eq!(highlight("a < b & c", ""), "a &lt; b &amp; c");
        assert_eq!(highlight("a < b", "text"), "a &lt; b");
        assert_eq!(highlight("<x>", "no-such-language"), "&lt;x&gt;");
    }

    #[test]
    fn the_line_splitter_keeps_endings() {
        let lines: Vec<&str> = with_endings("one\ntwo\nthree").collect();
        assert_eq!(lines, ["one\n", "two\n", "three"]);
        assert_eq!(with_endings("").count(), 0);
    }
}
