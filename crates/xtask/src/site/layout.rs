//! The page shell: everything around an article.
//!
//! One header and footer for every page, and for a documentation page the three-column
//! frame around it: the site navigation on the left, the article, and the page's own table
//! of contents on the right, with the previous and next page under the article. Hand-built
//! strings rather than a template engine, like every other renderer in this crate: there are
//! two layouts, and the typing keeps a broken shell a compile error rather than a page.

use crate::theme;

use super::highlight::escape;
use super::markdown::Heading;
use super::nav::Nav;
use super::{Kind, Page};

/// The repository, for the edit links.
const REPO: &str = "https://github.com/molexxxx/pamoja";

/// What every page's shell needs beyond the page itself.
pub struct Chrome<'a> {
    /// The workspace version the footer names.
    pub version: &'a str,
    /// The navigation the sidebar renders.
    pub nav: &'a Nav,
}

/// The prefix that reaches the site root from a page (`../` for `docs/index.html`).
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
    let root = root_of(&page.url);
    let group = chrome
        .nav
        .group_of(&page.url)
        .and_then(|group| group.title.as_deref())
        .unwrap_or("Documentation");
    let (previous, next) = chrome.nav.neighbours(&page.url);

    let mut out = head(&root, &page.title, &page.description);
    out.push_str("<body>\n");
    out.push_str("<a class=\"skip\" href=\"#content\">Skip to content</a>\n");
    out.push_str(&header(&root, true));
    out.push_str("<div class=\"docs\">\n<aside class=\"side\" id=\"side\">\n");
    out.push_str(&chrome.nav.sidebar(&page.url, &root));
    out.push_str("</aside>\n<main class=\"content\" id=\"content\">\n");
    out.push_str(&format!(
        "<p class=\"crumbs\"><span>{}</span></p>\n",
        escape(group)
    ));
    out.push_str(match page.kind {
        Kind::Guide => "<article class=\"article article-guide\">\n",
        Kind::Article => "<article class=\"article\">\n",
    });
    out.push_str(&page.body);
    out.push_str("</article>\n");
    out.push_str(&pager(&root, previous, next));
    out.push_str(&format!(
        "<p class=\"edit\"><a href=\"{REPO}/edit/main/{}\">Edit this page on GitHub</a></p>\n",
        escape(&page.source)
    ));
    out.push_str("</main>\n");
    out.push_str(&toc(&page.toc));
    out.push_str("</div>\n");
    out.push_str(&footer(&root, chrome.version));
    out.push_str(&format!(
        "<script src=\"{root}js/site.js\" defer></script>\n</body>\n</html>\n"
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
    let mut out = head("/", "Not found", "There is no page at this address.");
    out.push_str("<body>\n");
    out.push_str(&header("/", false));
    out.push_str(
        "<main class=\"content lone\" id=\"content\">\n<article class=\"article\">\n\
         <h1>There is no page here</h1>\n\
         <p>The address may have changed, or the link that brought you here may be stale. \
         The documentation is one step away.</p>\n\
         <ul>\n\
         <li><a href=\"/docs/index.html\">The documentation</a>, with a guide per capability</li>\n\
         <li><a href=\"/docs/install.html\">Install</a>, and what a narrow build costs</li>\n\
         <li><a href=\"/docs/hardware.html\">Hardware</a>, the parts the drivers were written against</li>\n\
         <li><a href=\"/docs/reference/rust.html\">The API references</a> for every language</li>\n\
         </ul>\n</article>\n</main>\n",
    );
    out.push_str(&footer("/", chrome.version));
    out.push_str("<script src=\"/js/site.js\" defer></script>\n</body>\n</html>\n");
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
///
/// # Returns
///
/// The complete document, which redirects at once and still reads as a page.
pub fn redirect(url: &str, target: &str, name: &str) -> String {
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
         <link rel=\"stylesheet\" href=\"{root}theme.css\">\n\
         <link rel=\"stylesheet\" href=\"{root}site.css\">\n\
         </head>\n<body>\n\
         <main class=\"content lone\">\n<article class=\"article\">\n\
         <h1>{name} reference</h1>\n\
         <p>The {name} reference is listed <a href=\"{target}\">one page up</a>: every package with its install line and its API pages.</p>\n\
         </article>\n</main>\n</body>\n</html>\n",
        name = escape(name),
    )
}

fn head(root: &str, title: &str, description: &str) -> String {
    let full = if title == "pamoja" {
        "pamoja documentation".to_owned()
    } else {
        format!("{title} - pamoja")
    };
    format!(
        "<!doctype html>\n<html lang=\"en\" class=\"no-js\" data-root=\"{root}\">\n<head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{}</title>\n\
         <meta name=\"description\" content=\"{}\">\n\
         <meta name=\"theme-color\" content=\"{}\">\n\
         <link rel=\"icon\" href=\"{root}assets/pamoja-icon.svg\">\n\
         <link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\n\
         <link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>\n\
         <link rel=\"stylesheet\" href=\"{}\">\n\
         <link rel=\"stylesheet\" href=\"{root}theme.css\">\n\
         <link rel=\"stylesheet\" href=\"{root}site.css\">\n\
         <script>document.documentElement.classList.replace('no-js','js');\
try{{var h=location.hash.slice(1);document.documentElement.dataset.lang=/^(rust|typescript|python|c)$/.test(h)?h:(localStorage.getItem('pamoja:lang')||'rust')}}catch(e){{document.documentElement.dataset.lang='rust'}}</script>\n\
         </head>\n",
        escape(&full),
        escape(description),
        theme::PALETTE.navy_1,
        theme::FONTS,
    )
}

// The header every page shares: the mark, the site's doors, and the search box. The menu
// button opens the sidebar on a narrow screen and is only rendered where there is one. The
// mark leads to the documentation's front page; the site root is still the showcase, which
// is not rendered here.
fn header(root: &str, with_menu: bool) -> String {
    let menu = if with_menu {
        "<button class=\"menu-toggle\" type=\"button\" aria-controls=\"side\" aria-expanded=\"false\">\
         <span class=\"menu-bars\" aria-hidden=\"true\"></span>Menu</button>\n"
    } else {
        ""
    };
    format!(
        "<header class=\"top\">\n\
         {menu}\
         <a class=\"brand\" href=\"{root}docs/index.html\" aria-label=\"pamoja documentation\">{}<span class=\"brand-word\">pamoja</span></a>\n\
         <nav class=\"top-nav\" aria-label=\"Site\">\n\
         <a href=\"{root}docs/index.html\">Docs</a>\n\
         <a href=\"{root}docs/hardware.html\">Hardware</a>\n\
         <a href=\"{root}docs/reference/rust.html\">Reference</a>\n\
         <a href=\"https://pamoja.molex.cloud/dashboard/\">Dashboard</a>\n\
         <a href=\"{REPO}\" class=\"top-github\">GitHub</a>\n\
         </nav>\n\
         <div class=\"search\" role=\"search\">\n\
         <input class=\"search-input\" type=\"search\" placeholder=\"Search\" aria-label=\"Search the documentation\" autocomplete=\"off\" spellcheck=\"false\">\n\
         <div class=\"search-results\" role=\"listbox\" aria-label=\"Search results\" hidden></div>\n\
         </div>\n\
         </header>\n",
        mark()
    )
}

// The mark: the mesh from the logo, turning slowly. SMIL rather than script, so it moves
// without JavaScript and stops for a reader who asked for reduced motion.
fn mark() -> &'static str {
    r##"<svg class="brand-mark" viewBox="0 0 240 240" width="30" height="30" aria-hidden="true"><defs><radialGradient id="mark-core" cx="0.5" cy="0.42" r="0.62"><stop offset="0" stop-color="#FFF3D6"/><stop offset="0.45" stop-color="#FFB627"/><stop offset="1" stop-color="#F26A4B"/></radialGradient></defs><g><animateTransform attributeName="transform" type="rotate" from="0 120 120" to="360 120 120" dur="60s" repeatCount="indefinite"/><g fill="none" stroke-width="10"><polygon points="120,40 188,80 188,160 120,200 52,160 52,80" stroke="#FBF3E4" stroke-opacity="0.28"/><g stroke="#FBF3E4" stroke-opacity="0.22"><line x1="120" y1="120" x2="120" y2="40"/><line x1="120" y1="120" x2="188" y2="80"/><line x1="120" y1="120" x2="188" y2="160"/><line x1="120" y1="120" x2="120" y2="200"/><line x1="120" y1="120" x2="52" y2="160"/><line x1="120" y1="120" x2="52" y2="80"/></g></g><g><circle cx="120" cy="40" r="13" fill="#FFB627"/><circle cx="188" cy="80" r="13" fill="#F26A4B"/><circle cx="188" cy="160" r="13" fill="#1FA995"/><circle cx="120" cy="200" r="13" fill="#FFB627"/><circle cx="52" cy="160" r="13" fill="#F26A4B"/><circle cx="52" cy="80" r="13" fill="#1FA995"/></g></g><circle cx="120" cy="120" r="22" fill="url(#mark-core)"/></svg>"##
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

fn pager(
    root: &str,
    previous: Option<&super::nav::Item>,
    next: Option<&super::nav::Item>,
) -> String {
    if previous.is_none() && next.is_none() {
        return String::new();
    }
    let mut out = String::from("<nav class=\"pager\" aria-label=\"Previous and next page\">\n");
    match previous {
        Some(item) => out.push_str(&format!(
            "<a class=\"pager-prev\" href=\"{root}{}\" rel=\"prev\"><span>Previous</span>{}</a>\n",
            item.url,
            escape(&item.title)
        )),
        None => out.push_str("<span></span>\n"),
    }
    if let Some(item) = next {
        out.push_str(&format!(
            "<a class=\"pager-next\" href=\"{root}{}\" rel=\"next\"><span>Next</span>{}</a>\n",
            item.url,
            escape(&item.title)
        ));
    }
    out.push_str("</nav>\n");
    out
}

fn footer(root: &str, version: &str) -> String {
    format!(
        "<footer class=\"foot\">\n\
         <div class=\"foot-brand\"><a href=\"{root}docs/index.html\" class=\"brand-word\">pamoja</a>\
         <p>One memory-safe Rust core with bindings for TypeScript, Python, and C#, for IoT, robotics, and drones.</p></div>\n\
         <nav class=\"foot-links\" aria-label=\"Registries\">\n\
         <a href=\"{REPO}\">GitHub</a>\n\
         <a href=\"https://crates.io/crates/pamoja\">crates.io</a>\n\
         <a href=\"https://www.npmjs.com/package/pamoja\">npm</a>\n\
         <a href=\"https://pypi.org/project/pamoja/\">PyPI</a>\n\
         <a href=\"https://www.nuget.org/packages/Pamoja\">NuGet</a>\n\
         </nav>\n\
         <p class=\"foot-fine\">Version {} · MIT licensed</p>\n\
         </footer>\n",
        escape(version)
    )
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
