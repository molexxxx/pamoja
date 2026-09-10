//! The functional block diagram, drawn as the page rather than as a picture.
//!
//! The same facts the SVG in `crates/xtask/src/diagram.rs` draws for the README, laid out
//! here in the sheet's own elements so it takes the page's tokens, reflows at every width,
//! follows the light and dark sheet without a second file, and is text a reader can select,
//! search, and follow.

use crate::catalog::escape;
use crate::diagram::Block;

/// What each binding is called and what carries it.
const DOORS: [(&str, &str, &str); 3] = [
    ("TypeScript", "@pamoja/&lt;name&gt;", "over napi-rs"),
    ("Python", "pamoja-&lt;name&gt;", "over PyO3"),
    ("C#", "Pamoja.&lt;Name&gt;", "over cbindgen and P/Invoke"),
];

/// What a Rust program does instead of calling through the engine.
const RUST: &str = "Rust: cargo add pamoja-&lt;name&gt;, the crates themselves";

/// The sentence under the drawing.
const FOOT: &str = "A chapter's package brings its capabilities with it. Everything at once: <code>npm install pamoja</code>, <code>pip install pamoja</code>, <code>dotnet add package Pamoja</code>, or <code>cargo add pamoja</code>.";

/// What the whole drawing says, for a reader who cannot see it.
const WHAT: &str = "How a call reaches a crate: three bindings over one compiled engine, a Rust program straight to the crates, every capability by chapter, and every crate over pamoja-core.";

/// Render the block diagram.
///
/// # Arguments
///
/// * `blocks` - one entry per chapter of the capability map.
///
/// # Returns
///
/// The diagram, as one `figure`-ready element.
pub fn diagram(blocks: &[Block]) -> String {
    let doors: String = DOORS
        .iter()
        .map(|(language, package, over)| {
            format!(
                "<div class=\"bd-door\"><b>{language}</b><code>{package}</code><span>{over}</span></div>"
            )
        })
        .collect();

    let cells: String = blocks
        .iter()
        .map(|block| {
            let crates: String = block
                .crates
                .iter()
                .map(|(name, on_core)| {
                    let mark = if *on_core { " class=\"on-core\"" } else { "" };
                    format!("<li{mark}>{}</li>", escape(name))
                })
                .collect();
            let names: String = block
                .names
                .iter()
                .map(|name| format!("<li>{}</li>", escape(name)))
                .collect();
            format!(
                "<div class=\"bd-cell\">\n\
                 <p class=\"bd-cell-title\">{}</p>\n\
                 <ul class=\"bd-crates\">{crates}</ul>\n\
                 <ul class=\"bd-names\">{names}</ul>\n\
                 </div>\n",
                escape(&block.title)
            )
        })
        .collect();

    format!(
        "<div class=\"bd\" role=\"img\" aria-label=\"{WHAT}\">\n\
         <div class=\"bd-doors\">{doors}</div>\n\
         <p class=\"bd-flow\" aria-hidden=\"true\"><span></span></p>\n\
         <div class=\"bd-engine\">\n\
         <div><b>Compiled engine</b><p>pamoja-ffi over the C ABI: one library carrying every capability</p></div>\n\
         <div class=\"bd-engine-end\"><code>@pamoja/native, pamoja-native, Pamoja.Native</code><span>A package narrows the API, not the download.</span></div>\n\
         </div>\n\
         <p class=\"bd-flow\" aria-hidden=\"true\"><span></span></p>\n\
         <div class=\"bd-caps\">\n\
         <p class=\"bd-caps-head\"><b>Capabilities by chapter</b><code>{RUST}</code></p>\n\
         <div class=\"bd-grid\">\n{cells}</div>\n\
         <div class=\"bd-core\">\n\
         <b>pamoja-core</b>\n\
         <p>Transport, Device, Sensor, Actuator, Store, and the event bus; <code>no_std</code>, so it runs on a microcontroller</p>\n\
         <p class=\"bd-key\">The names in the vendor ink build on it; the rest are pure logic with no dependency.</p>\n\
         </div>\n\
         </div>\n\
         <p class=\"bd-foot\">{FOOT}</p>\n\
         </div>\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::diagram::blocks;
    use crate::docs::repo_root;

    #[test]
    fn every_chapter_and_every_crate_is_an_element() {
        let root = repo_root();
        let catalog = Catalog::load(&root).unwrap();
        let blocks = blocks(&catalog, &root).unwrap();
        let html = diagram(&blocks);
        for chapter in &catalog.chapters {
            assert!(
                html.contains(&format!(
                    "<p class=\"bd-cell-title\">{}</p>",
                    escape(&chapter.title)
                )),
                "{} is missing",
                chapter.title
            );
        }
        for capability in &catalog.capabilities {
            for krate in &capability.crates {
                let name = krate.strip_prefix("pamoja-").unwrap();
                assert!(
                    html.contains(&format!(">{name}</li>")),
                    "{krate} is missing"
                );
            }
        }
        assert!(
            html.contains("<li class=\"on-core\">"),
            "the crates that build on the core are marked"
        );
        assert!(
            !html.contains("<svg"),
            "the drawing is the page, not a picture"
        );
        assert!(!html.contains("<name>"), "the placeholders are escaped");
    }
}
