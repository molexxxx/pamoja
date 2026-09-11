//! The page shell: everything around an article.
//!
//! One header and footer for every page, and for a documentation page the three-column
//! frame around it: the site navigation on the left, the article, and the page's own table
//! of contents on the right, with the previous and next page under the article. Everything
//! between the header and the footer sits in one `#page` element, which `site.js` swaps
//! when a reader follows a link, so a page changes without a reload; every link in the
//! shell is root-relative so a swapped page never carries a path from another depth. Every
//! page is still a complete document with its own title, description, canonical URL, and
//! Open Graph card, so a crawler or a reader without scripts sees the same thing.
//!
//! Hand-built strings rather than a template engine, like every other renderer in this
//! crate: there are two layouts, and the typing keeps a broken shell a compile error rather
//! than a page.

use crate::theme;

use super::highlight::escape;
use super::markdown::Heading;
use super::nav::Nav;
use super::{Kind, Page};

/// The repository, for the edit links.
const REPO: &str = "https://github.com/molexxxx/pamoja";

/// Where the site is served from: its own origin, so every link in the shell starts here.
const ROOT: &str = "/";

/// The site's origin, for the canonical URL and the card each page carries.
pub(crate) const ORIGIN: &str = "https://pamoja.molex.cloud";

/// What every page's shell needs beyond the page itself.
pub struct Chrome<'a> {
    /// The workspace version the footer names.
    pub version: &'a str,
    /// The navigation the sidebar renders.
    pub nav: &'a Nav,
    /// The stamp of the stylesheets and scripts, which names each of them in a page.
    pub stamp: &'a str,
}

/// The prefix that reaches the site root from a page (`../` for `docs/index.html`), for
/// the pages that must resolve on their own wherever they are opened.
pub fn root_of(url: &str) -> String {
    "../".repeat(url.matches('/').count())
}

/// A documentation page as a complete HTML document.
///
/// # Arguments
///
/// * `chrome` - the version and navigation the shell carries.
/// * `page` - the rendered article and what the frame around it shows.
///
/// # Returns
///
/// The document, from `<!doctype html>` to `</html>`.
pub fn document(chrome: &Chrome, page: &Page) -> String {
    let (previous, next) = chrome.nav.neighbors(&page.url);

    let full = if page.title == "pamoja" {
        "pamoja documentation".to_owned()
    } else {
        format!("{} - pamoja", page.title)
    };
    let mut out = head(
        &Head {
            title: &full,
            description: &page.description,
            url: &page.url,
            kind: "article",
        },
        chrome.stamp,
    );
    out.push_str("<body>\n");
    out.push_str("<a class=\"skip\" href=\"#content\">Skip to content</a>\n");
    out.push_str(&header(chrome.version, &page.url));
    // A page carrying the block diagram opens out: the diagram needs more width than a
    // reading column, so the sheet widens and the table of contents stands down, the way a
    // datasheet prints its block diagram on a foldout.
    let foldout = page.body.contains("class=\"bd\"");
    out.push_str(&format!(
        "<div id=\"page\">\n<div class=\"docs{}\">\n<aside class=\"side\" id=\"side\">\n",
        if foldout { " docs-foldout" } else { "" }
    ));
    out.push_str(&chrome.nav.sidebar(&page.url, ROOT));
    out.push_str("</aside>\n<main class=\"content\" id=\"content\">\n");
    out.push_str(match page.kind {
        Kind::Guide => "<article class=\"article article-guide\">\n",
        Kind::Article => "<article class=\"article\">\n",
    });
    out.push_str(&page.body);
    out.push_str("</article>\n");
    out.push_str(&pager(previous, next));
    out.push_str(&format!(
        "<p class=\"edit\"><a href=\"{REPO}/edit/main/{}\">Edit this page on GitHub</a></p>\n",
        escape(&page.source)
    ));
    out.push_str("</main>\n");
    if !foldout {
        out.push_str(&toc(&page.toc));
    }
    out.push_str("</div>\n</div>\n");
    out.push_str(&footer(chrome.version));
    out.push_str(&format!(
        "<script src=\"/js/site.js?v={}\" defer></script>\n</body>\n</html>\n",
        chrome.stamp
    ));
    out
}

/// The front page as a complete document: the shared header and footer around the body
/// `home.rs` renders, with the front page's own stylesheet and the scripts its consoles
/// and wall of cards need.
///
/// # Arguments
///
/// * `chrome` - the version and navigation the shell carries.
/// * `body` - the `<main>` element, rendered by the front page.
///
/// # Returns
///
/// The complete document.
pub fn home(chrome: &Chrome, body: &str) -> String {
    let mut out = head(&Head {
        title: "pamoja",
        description: "One memory-safe Rust core with bindings for TypeScript, Python, and C#, for IoT, robotics, and drones, built to run on cheap hardware with weak or no connectivity.",
        url: "index.html",
        kind: "website",
    }, chrome.stamp);
    out = out.replace(
        &format!("<link rel=\"stylesheet\" href=\"/site.css?v={}\">\n", chrome.stamp),
        &format!(
            "<link rel=\"stylesheet\" href=\"/site.css?v={0}\">\n<link rel=\"stylesheet\" href=\"/home.css?v={0}\">\n",
            chrome.stamp
        ),
    );
    out.push_str("<body class=\"is-home\">\n");
    out.push_str("<a class=\"skip\" href=\"#content\">Skip to content</a>\n");
    out.push_str(&header(chrome.version, "index.html"));
    out.push_str("<div id=\"page\">\n");
    out.push_str(&format!(
        "<nav class=\"side home-menu\" id=\"side\" aria-label=\"Site\">\n\
         <div class=\"side-nav\">\n\
         <ul>\n\
         <li><a href=\"/docs/index.html\">Docs</a></li>\n\
         <li><a href=\"/docs/install.html\">Install</a></li>\n\
         <li><a href=\"/docs/hardware.html\">Hardware</a></li>\n\
         <li><a href=\"/docs/buses.html\">Buses and links</a></li>\n\
         <li><a href=\"/docs/radio.html\">Radios and antennas</a></li>\n\
         <li><a href=\"/docs/examples.html\">Examples</a></li>\n\
         <li><a href=\"/docs/profiles.html\">Profiles</a></li>\n\
         <li><a href=\"/docs/reference/index.html\">API reference</a></li>\n\
         <li><a href=\"https://pamoja.molex.cloud/dashboard/\">Dashboard demo</a></li>\n\
         </ul>\n\
         <details class=\"side-group\" open><summary>Project</summary><ul>\n\
         <li><a href=\"{REPO}\">Source on GitHub</a></li>\n\
         <li><a href=\"/docs/community.html\">Contribute</a></li>\n\
         <li><a href=\"{REPO}/issues/new?template=bug.yml\">Report a bug</a></li>\n\
         <li><a href=\"{REPO}/issues/new?template=capability.yml\">Request a capability</a></li>\n\
         <li><a href=\"{REPO}/issues/new?template=docs.yml\">Report a documentation problem</a></li>\n\
         <li><a href=\"{REPO}/issues/new?template=profile.yml\">Share a profile</a></li>\n\
         <li><a href=\"{REPO}/releases\">Releases</a></li>\n\
         </ul></details>\n\
         </div>\n\
         </nav>\n"
    ));
    out.push_str(body);
    out.push_str("</div>\n");
    out.push_str(&footer(chrome.version));
    out.push_str(&format!(
        "<script src=\"/js/site.js?v={0}\" defer></script>\n\
         <script type=\"module\">import {{ init }} from '/js/home.js?v={0}'; init();</script>\n\
         </body>\n</html>\n",
        chrome.stamp
    ));
    out
}

/// The page served for a URL that does not exist.
///
/// Pages serves it for any missing path, so its links are absolute rather than relative to
/// wherever the reader ended up.
///
/// # Arguments
///
/// * `chrome` - the version and navigation the shell carries.
///
/// # Returns
///
/// The complete document.
pub fn not_found(chrome: &Chrome) -> String {
    let mut out = head(
        &Head {
            title: "Not found - pamoja",
            description: "There is no page at this address.",
            url: "404.html",
            kind: "website",
        },
        chrome.stamp,
    );
    out.push_str("<body>\n");
    out.push_str(&header(chrome.version, "404.html"));
    out.push_str(
        "<div id=\"page\">\n<main class=\"content lone\" id=\"content\">\n<article class=\"article\">\n\
         <h1>There is no page here</h1>\n\
         <p>The address may have changed, or the link that brought you here may be stale. \
         The documentation is one step away.</p>\n\
         <ul>\n\
         <li><a href=\"/docs/index.html\">The documentation</a>, with a guide per capability</li>\n\
         <li><a href=\"/docs/install.html\">Install</a>, and what a narrow build costs</li>\n\
         <li><a href=\"/docs/hardware.html\">Hardware</a>, the parts the drivers were written against</li>\n\
         <li><a href=\"/docs/examples.html\">Examples</a>, every one run in CI</li>\n\
         <li><a href=\"/docs/reference/index.html\">The API references</a> for every language</li>\n\
         </ul>\n</article>\n</main>\n</div>\n",
    );
    out.push_str(&footer(chrome.version));
    out.push_str(&format!(
        "<script src=\"/js/site.js?v={}\" defer></script>\n</body>\n</html>\n",
        chrome.stamp
    ));
    out
}

/// A page that hands a reader on to another: the root of a generated reference tree, whose
/// index is the reference page on this site.
///
/// # Arguments
///
/// * `url` - where the page sits, site-relative, so its stylesheets resolve.
/// * `target` - where it sends the reader, relative to itself.
/// * `name` - the language, for the title and the one line of text.
/// * `stamp` - the stamp that names the stylesheets.
///
/// # Returns
///
/// The complete document, which redirects at once and still reads as a page.
pub fn redirect(url: &str, target: &str, name: &str, stamp: &str) -> String {
    let root = root_of(url);
    let target = escape(target);
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta http-equiv=\"refresh\" content=\"0; url={target}\">\n\
         <meta name=\"robots\" content=\"noindex\">\n\
         <link rel=\"canonical\" href=\"{target}\">\n\
         <title>{name} reference - pamoja</title>\n\
         <link rel=\"stylesheet\" href=\"{root}theme.css?v={stamp}\">\n\
         <link rel=\"stylesheet\" href=\"{root}site.css?v={stamp}\">\n\
         </head>\n<body>\n\
         <main class=\"content lone\">\n<article class=\"article\">\n\
         <h1>{name} reference</h1>\n\
         <p>The {name} reference is listed <a href=\"{target}\">one page up</a>: every package with its install line and its API pages.</p>\n\
         </article>\n</main>\n</body>\n</html>\n",
        name = escape(name),
    )
}

/// What the head of a page says about it.
struct Head<'a> {
    /// The full title, as the tab shows it.
    title: &'a str,
    /// The description, for the meta tag and the card.
    description: &'a str,
    /// The page, site-relative, for the canonical URL.
    url: &'a str,
    /// The Open Graph type: `website` for the front page, `article` for the rest.
    kind: &'a str,
}

fn head(page: &Head, stamp: &str) -> String {
    let canonical = if page.url == "index.html" {
        format!("{ORIGIN}/")
    } else {
        format!("{ORIGIN}/{}", page.url)
    };
    format!(
        "<!doctype html>\n<html lang=\"en\" class=\"no-js\" data-root=\"{ROOT}\" data-stamp=\"{stamp}\">\n<head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n\
         <meta name=\"description\" content=\"{description}\">\n\
         <meta name=\"theme-color\" content=\"{}\">\n\
         <link rel=\"canonical\" href=\"{canonical}\">\n\
         <meta property=\"og:site_name\" content=\"pamoja\">\n\
         <meta property=\"og:type\" content=\"{}\">\n\
         <meta property=\"og:title\" content=\"{title}\">\n\
         <meta property=\"og:description\" content=\"{description}\">\n\
         <meta property=\"og:url\" content=\"{canonical}\">\n\
         <meta name=\"twitter:card\" content=\"summary\">\n\
         <link rel=\"icon\" href=\"/assets/pamoja-icon.svg\">\n\
         <link rel=\"preload\" href=\"/fonts/Archivo.woff2\" as=\"font\" type=\"font/woff2\" crossorigin>\n\
         <link rel=\"stylesheet\" href=\"/fonts/fonts.css?v={stamp}\">\n\
         <link rel=\"stylesheet\" href=\"/theme.css?v={stamp}\">\n\
         <link rel=\"stylesheet\" href=\"/site.css?v={stamp}\">\n\
         <script>document.documentElement.classList.replace('no-js','js');\
try{{var h=location.hash.slice(1);document.documentElement.dataset.lang=/^(rust|typescript|python|c)$/.test(h)?h:(localStorage.getItem('pamoja:lang')||'rust')}}catch(e){{document.documentElement.dataset.lang='rust'}}\
try{{var s=localStorage.getItem('pamoja:scheme');if(s==='light'||s==='dark')document.documentElement.dataset.theme=s}}catch(e){{}}</script>\n\
         </head>\n",
        theme::LIGHT.band,
        page.kind,
        title = escape(page.title),
        description = escape(page.description),
        stamp = stamp,
    )
}

// The header every page shares, drawn as a datasheet's band: the mark and the part name,
// the one-line description, the site's doors, the search box, and the icon bar to the
// project on GitHub; under it the document line a sheet carries on every page, the part,
// its revision, and its license. Every page gets the menu toggle: it opens the sidebar on
// a documentation page and the drawer of site links on the front page, and site.js hides
// it where there is neither.
fn header(version: &str, here: &str) -> String {
    let menu = "<button class=\"menu-toggle\" type=\"button\" aria-controls=\"side\" aria-expanded=\"false\">\
         <span class=\"menu-bars\" aria-hidden=\"true\"></span>Menu</button>\n";
    let matched = SECTIONS
        .iter()
        .find(|(_, _, prefix)| here.starts_with(prefix))
        .map(|(label, _, _)| *label);
    let nav: String = DOORS
        .iter()
        .map(|(label, url)| {
            let current = if matched == Some(*label) {
                " class=\"here\" aria-current=\"page\""
            } else {
                ""
            };
            format!("<a href=\"/{url}\"{current}>{label}</a>\n")
        })
        .collect();
    format!(
        "<header class=\"band\">\n\
         <div class=\"band-row\">\n\
         {menu}\
         <a class=\"brand\" href=\"/\" aria-label=\"pamoja home\">{}<span class=\"brand-word\">pamoja</span></a>\n\
         <p class=\"band-desc\">An SDK for IoT, robotics, and drones</p>\n\
         <nav class=\"top-nav\" aria-label=\"Site\">\n{nav}</nav>\n\
         <div class=\"search\" role=\"search\">\n\
         {}\
         <input class=\"search-input\" type=\"search\" placeholder=\"Search the documentation\" aria-label=\"Search the documentation, or press slash\" autocomplete=\"off\" spellcheck=\"false\">\n\
         <kbd class=\"search-key\" aria-hidden=\"true\">/</kbd>\n\
         <div class=\"search-results\" hidden></div>\n\
         </div>\n\
         <button class=\"scheme\" type=\"button\" data-scheme=\"system\" aria-label=\"Color scheme: following your system. Activate for the light sheet.\">\
         <span class=\"scheme-icon\" aria-hidden=\"true\">{}{}{}</span><span class=\"scheme-word\">Auto</span></button>\n\
         <nav class=\"project\" aria-label=\"The project on GitHub\">\n\
         <a href=\"{REPO}\" title=\"Source on GitHub\" aria-label=\"Source on GitHub\">{}</a>\n\
         <a href=\"{REPO}/issues/new?template=bug.yml\" title=\"Report a bug\" aria-label=\"Report a bug\">{}</a>\n\
         <a href=\"{REPO}/issues/new?template=capability.yml\" title=\"Request a capability or a change\" aria-label=\"Request a capability or a change\">{}</a>\n\
         <a href=\"{REPO}/releases\" title=\"Releases and the changelog\" aria-label=\"Releases and the changelog\">{}</a>\n\
         </nav>\n\
         </div>\n\
         <p class=\"docline\"><span>pamoja</span><span>Rev {}</span><span>MIT license</span><span class=\"docline-end\">pamoja.molex.cloud</span></p>\n\
         </header>\n",
        mark(),
        ICON_GLASS,
        ICON_AUTO,
        ICON_SUN,
        ICON_MOON,
        ICON_GITHUB,
        ICON_BUG,
        ICON_REQUEST,
        ICON_TAG,
        escape(version),
    )
}

/// The doors in the band, in reading order.
const DOORS: [(&str, &str); 4] = [
    ("Docs", "docs/index.html"),
    ("Install", "docs/install.html"),
    ("Hardware", "docs/hardware.html"),
    ("Reference", "docs/reference/index.html"),
];

/// Which door the reader is behind, tried in this order: the three single destinations
/// first, then the documentation, which would otherwise claim every page under `docs/`.
const SECTIONS: [(&str, &str, &str); 4] = [
    ("Install", "docs/install.html", "docs/install"),
    ("Hardware", "docs/hardware.html", "docs/hardware"),
    ("Reference", "docs/reference/index.html", "docs/reference"),
    ("Docs", "docs/index.html", "docs/"),
];

/// A magnifying glass, inside the search field.
const ICON_GLASS: &str = "<svg class=\"search-glass\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.6\" stroke-linecap=\"round\" aria-hidden=\"true\"><circle cx=\"7\" cy=\"7\" r=\"4.5\"/><path d=\"M10.4 10.4 14 14\"/></svg>";

/// A half-lit disc: the scheme follows the reader's system.
const ICON_AUTO: &str = "<svg class=\"i-auto\" viewBox=\"0 0 16 16\" width=\"16\" height=\"16\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\" aria-hidden=\"true\"><circle cx=\"8\" cy=\"8\" r=\"6\"/><path d=\"M8 2a6 6 0 0 0 0 12z\" fill=\"currentColor\" stroke=\"none\"/></svg>";

/// A sun: the light sheet.
const ICON_SUN: &str = "<svg class=\"i-sun\" viewBox=\"0 0 16 16\" width=\"16\" height=\"16\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\" aria-hidden=\"true\"><circle cx=\"8\" cy=\"8\" r=\"3.25\"/><path d=\"M8 1v1.6M8 13.4V15M1 8h1.6M13.4 8H15M3.05 3.05l1.13 1.13M11.82 11.82l1.13 1.13M12.95 3.05l-1.13 1.13M4.18 11.82l-1.13 1.13\"/></svg>";

/// A moon: the sheet in reverse.
const ICON_MOON: &str = "<svg class=\"i-moon\" viewBox=\"0 0 16 16\" width=\"16\" height=\"16\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linejoin=\"round\" aria-hidden=\"true\"><path d=\"M13.2 9.7A5.6 5.6 0 0 1 6.3 2.8a5.6 5.6 0 1 0 6.9 6.9z\"/></svg>";

/// The GitHub mark, filled with the current color.
const ICON_GITHUB: &str = "<svg viewBox=\"0 0 16 16\" width=\"18\" height=\"18\" fill=\"currentColor\" aria-hidden=\"true\"><path d=\"M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0 0 16 8c0-4.42-3.58-8-8-8z\"/></svg>";

/// A bug, drawn in strokes.
const ICON_BUG: &str = "<svg viewBox=\"0 0 16 16\" width=\"18\" height=\"18\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\"><path d=\"M5 7.5a3 3 0 0 1 6 0v2.5a3 3 0 0 1-6 0z\"/><path d=\"M6 5.2V4a2 2 0 0 1 4 0v1.2M8 7.5v5.5M2.5 8.5H5M11 8.5h2.5M3.2 12.5 5 11.3M12.8 12.5 11 11.3M3.2 4.5 5 6M12.8 4.5 11 6\"/></svg>";

/// A plus in a circle, for asking the project for something.
const ICON_REQUEST: &str = "<svg viewBox=\"0 0 16 16\" width=\"18\" height=\"18\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\" aria-hidden=\"true\"><circle cx=\"8\" cy=\"8\" r=\"6.25\"/><path d=\"M8 5.2v5.6M5.2 8h5.6\"/></svg>";

/// A tag, for the releases.
const ICON_TAG: &str = "<svg viewBox=\"0 0 16 16\" width=\"18\" height=\"18\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\"><path d=\"M2 2h5.6l6.4 6.4-5.6 5.6L2 7.6z\"/><circle cx=\"5.2\" cy=\"5.2\" r=\"1\" fill=\"currentColor\" stroke=\"none\"/></svg>";

// The mark: the mesh from the logo, turning slowly. SMIL rather than script, so it moves
// without JavaScript and stops for a reader who asked for reduced motion.
fn mark() -> &'static str {
    r##"<svg class="brand-mark" viewBox="0 0 240 240" width="30" height="30" aria-hidden="true"><g><g fill="none" stroke-width="10"><polygon points="120,40 188,80 188,160 120,200 52,160 52,80" stroke="#FBF3E4" stroke-opacity="0.28"/><g stroke="#FBF3E4" stroke-opacity="0.22"><line x1="120" y1="120" x2="120" y2="40"/><line x1="120" y1="120" x2="188" y2="80"/><line x1="120" y1="120" x2="188" y2="160"/><line x1="120" y1="120" x2="120" y2="200"/><line x1="120" y1="120" x2="52" y2="160"/><line x1="120" y1="120" x2="52" y2="80"/></g></g><g><circle cx="120" cy="40" r="13" fill="#FFB627"/><circle cx="188" cy="80" r="13" fill="#F26A4B"/><circle cx="188" cy="160" r="13" fill="#1FA995"/><circle cx="120" cy="200" r="13" fill="#FFB627"/><circle cx="52" cy="160" r="13" fill="#F26A4B"/><circle cx="52" cy="80" r="13" fill="#1FA995"/></g></g><circle cx="120" cy="120" r="22" fill="#FFB627"/></svg>"##
}

// The page's own table of contents: its second- and third-level headings, nested.
fn toc(headings: &[Heading]) -> String {
    let listed: Vec<&Heading> = headings
        .iter()
        .filter(|heading| heading.level == 2 || heading.level == 3)
        .collect();
    if listed.len() < 2 {
        return String::new();
    }
    let mut out = String::from(
        "<aside class=\"toc\">\n<nav aria-label=\"On this page\">\n<p class=\"toc-title\">On this page</p>\n<ul>\n",
    );
    let mut open_sub = false;
    for heading in listed {
        if heading.level == 3 {
            if !open_sub {
                out.push_str("<ul>\n");
                open_sub = true;
            }
        } else if open_sub {
            out.push_str("</ul>\n</li>\n");
            open_sub = false;
        } else if out.ends_with("</a>\n") {
            out.push_str("</li>\n");
        }
        out.push_str(&format!(
            "<li><a href=\"#{}\">{}</a>\n",
            escape(&heading.id),
            escape(&heading.text)
        ));
        if heading.level == 3 {
            out.push_str("</li>\n");
        }
    }
    if open_sub {
        out.push_str("</ul>\n");
    }
    out.push_str("</li>\n</ul>\n</nav>\n</aside>\n");
    out
}

fn pager(previous: Option<&super::nav::Item>, next: Option<&super::nav::Item>) -> String {
    if previous.is_none() && next.is_none() {
        return String::new();
    }
    let mut out = String::from("<nav class=\"pager\" aria-label=\"Previous and next page\">\n");
    match previous {
        Some(item) => out.push_str(&format!(
            "<a class=\"pager-prev\" href=\"/{}\" rel=\"prev\"><span>Previous</span>{}</a>\n",
            item.url,
            escape(&item.title)
        )),
        None => out.push_str("<span></span>\n"),
    }
    if let Some(item) = next {
        out.push_str(&format!(
            "<a class=\"pager-next\" href=\"/{}\" rel=\"next\"><span>Next</span>{}</a>\n",
            item.url,
            escape(&item.title)
        ));
    }
    out.push_str("</nav>\n");
    out
}

fn footer(version: &str) -> String {
    format!(
        "<footer class=\"foot\">\n\
         <div class=\"foot-grid\">\n\
         <div class=\"foot-notice\">\n\
         <h2 class=\"foot-head\">Important notice</h2>\n\
         <p>pamoja is free software under the MIT license: one memory-safe Rust core with bindings for TypeScript, Python, and C#, for IoT, robotics, and drones. \
         Every example on this site is spliced from a test that runs in CI on every change, every table is rendered from the source, and every number is measured by a build or read from a registry. \
         The scenarios are illustrations of what the crates do, not deployments.</p>\n\
         <p>Nothing here is certified for safety-critical use, and the MIT license disclaims every warranty. Copyright 2026 Anthony Wiedman.</p>\n\
         </div>\n\
         <nav class=\"foot-links\" aria-labelledby=\"foot-published\">\n\
         <h2 class=\"foot-head\" id=\"foot-published\">Published</h2>\n\
         <a href=\"{REPO}\">GitHub</a>\n\
         <a href=\"https://crates.io/crates/pamoja\">crates.io</a>\n\
         <a href=\"https://www.npmjs.com/package/pamoja\">npm</a>\n\
         <a href=\"https://pypi.org/project/pamoja/\">PyPI</a>\n\
         <a href=\"https://www.nuget.org/packages/Pamoja\">NuGet</a>\n\
         </nav>\n\
         <nav class=\"foot-links\" aria-labelledby=\"foot-legal\">\n\
         <h2 class=\"foot-head\" id=\"foot-legal\">Legal</h2>\n\
         <a href=\"/docs/about/terms.html\">Terms</a>\n\
         <a href=\"/docs/about/privacy.html\">Privacy</a>\n\
         <a href=\"/docs/about/notices.html\">Notices</a>\n\
         <a href=\"{REPO}/blob/main/LICENSE-MIT\">MIT license</a>\n\
         </nav>\n\
         <nav class=\"foot-links\" aria-labelledby=\"foot-contact\">\n\
         <h2 class=\"foot-head\" id=\"foot-contact\">Contact</h2>\n\
         <a href=\"{REPO}/issues/new/choose\">Open an issue</a>\n\
         <a href=\"{REPO}/security/advisories/new\">Report a vulnerability</a>\n\
         <a href=\"/docs/about/notices.html#contact\">Email the maintainer</a>\n\
         </nav>\n\
         </div>\n\
         <p class=\"foot-fine\"><span>pamoja</span><span>Rev {}</span><span>MIT license</span><span class=\"foot-host\">pamoja.molex.cloud</span></p>\n\
         </footer>\n",
        escape(version)
    )
}

/// The site's `sitemap.xml`, one entry per page.
///
/// # Arguments
///
/// * `urls` - every page, site-relative, `index.html` included.
///
/// # Returns
///
/// The sitemap document.
pub fn sitemap(urls: &[&str]) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for url in urls {
        let location = if *url == "index.html" {
            format!("{ORIGIN}/")
        } else {
            format!("{ORIGIN}/{url}")
        };
        out.push_str(&format!("<url><loc>{}</loc></url>\n", escape(&location)));
    }
    out.push_str("</urlset>\n");
    out
}

/// The site's `robots.txt`: everything may be crawled, and the sitemap is named.
pub fn robots() -> String {
    format!("User-agent: *\nAllow: /\nSitemap: {ORIGIN}/sitemap.xml\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_root_prefix_climbs_one_step_per_directory() {
        assert_eq!(root_of("docs/index.html"), "../");
        assert_eq!(root_of("docs/guides/modbus.html"), "../../");
        assert_eq!(root_of("index.html"), "");
    }

    #[test]
    fn the_head_names_the_page_for_crawlers_and_cards() {
        let html = head(
            &Head {
                title: "Modbus RTU - pamoja",
                description: "Frames & replies.",
                url: "docs/guides/modbus.html",
                kind: "article",
            },
            "0123abcd",
        );
        assert!(html.contains(
            "<link rel=\"canonical\" href=\"https://pamoja.molex.cloud/docs/guides/modbus.html\">"
        ));
        assert!(html.contains("<meta property=\"og:title\" content=\"Modbus RTU - pamoja\">"));
        assert!(
            html.contains("<meta property=\"og:description\" content=\"Frames &amp; replies.\">")
        );
        assert!(html.contains("<meta property=\"og:type\" content=\"article\">"));
        assert!(html.contains("data-root=\"/\""));
        assert!(
            html.contains("data-stamp=\"0123abcd\""),
            "the page carries the assets' stamp"
        );
        assert!(
            html.contains("<link rel=\"stylesheet\" href=\"/site.css?v=0123abcd\">"),
            "the stylesheet is named with the stamp"
        );
        let front = head(
            &Head {
                title: "pamoja",
                description: "x",
                url: "index.html",
                kind: "website",
            },
            "0123abcd",
        );
        assert!(front.contains("<link rel=\"canonical\" href=\"https://pamoja.molex.cloud/\">"));
    }

    #[test]
    fn the_sitemap_lists_every_page_at_its_public_address() {
        let map = sitemap(&["index.html", "docs/index.html", "docs/guides/modbus.html"]);
        assert!(map.contains("<loc>https://pamoja.molex.cloud/</loc>"));
        assert!(map.contains("<loc>https://pamoja.molex.cloud/docs/guides/modbus.html</loc>"));
        assert_eq!(map.matches("<url>").count(), 3);
        assert!(robots().contains("Sitemap: https://pamoja.molex.cloud/sitemap.xml"));
    }

    #[test]
    fn the_table_of_contents_nests_third_level_headings() {
        let headings = vec![
            Heading {
                level: 1,
                id: "t".into(),
                text: "Title".into(),
            },
            Heading {
                level: 2,
                id: "a".into(),
                text: "A".into(),
            },
            Heading {
                level: 3,
                id: "a-1".into(),
                text: "A one".into(),
            },
            Heading {
                level: 2,
                id: "b".into(),
                text: "B".into(),
            },
        ];
        let html = toc(&headings);
        assert!(html.contains("<li><a href=\"#a\">A</a>\n<ul>\n<li><a href=\"#a-1\">A one</a>\n</li>\n</ul>\n</li>\n<li><a href=\"#b\">B</a>\n</li>\n</ul>"), "{html}");
        assert!(!html.contains("Title"));
        assert!(toc(&headings[..2]).is_empty(), "one heading is no table");
    }
}
