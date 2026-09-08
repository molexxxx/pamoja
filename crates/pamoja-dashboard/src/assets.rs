//! The static page assets, served either embedded or live from disk.
//!
//! In production the page is baked into the firmware with `include_bytes!`, so there
//! is no filesystem dependency and the dashboard is part of the image. In development
//! the same directory is read from disk on every request, so editing the app and
//! reloading shows the change with no recompile. The dashboard is a multi-file
//! ES-module zQuery app, so both modes resolve any nested path under the web root.

#[cfg(feature = "serve")]
use std::path::PathBuf;

/// One bundled file: its URL path, its MIME type, and its bytes.
struct Asset {
    path: &'static str,
    content_type: &'static str,
    bytes: &'static [u8],
}

const HTML: &str = "text/html; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "application/javascript; charset=utf-8";
// Only the full bundle embeds the per-locale JSON; the floor tier carries none.
#[cfg(not(feature = "tier-c"))]
const JSON: &str = "application/json; charset=utf-8";

// The smallest tier embeds only the self-contained floor page at `/`: one ultra-minimal,
// gzippable document that renders the status table with the smallest possible script, and
// degrades to the device's server-rendered `/lite` table when scripting is off. The rich app
// modules are not part of this image.
#[cfg(feature = "tier-c")]
const EMBEDDED: &[Asset] = &[Asset {
    path: "/",
    content_type: HTML,
    bytes: include_bytes!("../web/lite.html"),
}];

// The full bundle, embedded at compile time. `/` maps to the page shell. The order does not
// matter; lookups are by exact path.
#[cfg(not(feature = "tier-c"))]
const EMBEDDED: &[Asset] = &[
    Asset {
        path: "/",
        content_type: HTML,
        bytes: include_bytes!("../web/index.html"),
    },
    Asset {
        path: "/global.css",
        content_type: CSS,
        bytes: include_bytes!("../web/global.css"),
    },
    // The floor page is also reachable from the full app (the top bar's "Lite" link), so a
    // viewer on a weak phone can drop to it; it is excluded from the page-load footprint.
    Asset {
        path: "/lite.html",
        content_type: HTML,
        bytes: include_bytes!("../web/lite.html"),
    },
    Asset {
        path: "/zquery.min.js",
        content_type: JS,
        bytes: include_bytes!("../web/zquery.min.js"),
    },
    Asset {
        path: "/app/app.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/app.js"),
    },
    Asset {
        path: "/app/store.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/store.js"),
    },
    Asset {
        path: "/app/routes.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/routes.js"),
    },
    Asset {
        path: "/app/nav.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/nav.js"),
    },
    Asset {
        path: "/app/lib/feed.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/feed.js"),
    },
    Asset {
        path: "/app/lib/edits.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/edits.js"),
    },
    Asset {
        path: "/app/lib/catalog.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/catalog.js"),
    },
    Asset {
        path: "/app/lib/discovery.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/discovery.js"),
    },
    Asset {
        path: "/app/lib/parallax.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/parallax.js"),
    },
    Asset {
        path: "/app/lib/detail.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/detail.js"),
    },
    Asset {
        path: "/app/lib/i18n.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/i18n.js"),
    },
    Asset {
        path: "/app/lib/pair.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/pair.js"),
    },
    Asset {
        path: "/app/lib/crypto/bytes.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/crypto/bytes.js"),
    },
    Asset {
        path: "/app/lib/crypto/sha256.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/crypto/sha256.js"),
    },
    Asset {
        path: "/app/lib/crypto/hmac.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/crypto/hmac.js"),
    },
    Asset {
        path: "/app/lib/crypto/hkdf.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/crypto/hkdf.js"),
    },
    Asset {
        path: "/app/lib/viz/index.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/viz/index.js"),
    },
    Asset {
        path: "/app/lib/viz/util.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/viz/util.js"),
    },
    Asset {
        path: "/app/lib/viz/links.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/viz/links.js"),
    },
    Asset {
        path: "/app/lib/viz/charts.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/viz/charts.js"),
    },
    Asset {
        path: "/app/lib/viz/gauges.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/viz/gauges.js"),
    },
    Asset {
        path: "/app/lib/viz/glyphs.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/lib/viz/glyphs.js"),
    },
    Asset {
        path: "/app/components/top-bar.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/components/top-bar.js"),
    },
    Asset {
        path: "/app/components/dashboard-page.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/components/dashboard-page.js"),
    },
    Asset {
        path: "/app/components/sensor-modal.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/components/sensor-modal.js"),
    },
    Asset {
        path: "/app/components/pairing-modal.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/components/pairing-modal.js"),
    },
    Asset {
        path: "/app/components/manage-modal.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/components/manage-modal.js"),
    },
    Asset {
        path: "/app/components/group-modal.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/components/group-modal.js"),
    },
    Asset {
        path: "/app/components/mesh-modal.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/components/mesh-modal.js"),
    },
    Asset {
        path: "/app/components/network-view.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/components/network-view.js"),
    },
    Asset {
        path: "/app/components/alarm-bar.js",
        content_type: JS,
        bytes: include_bytes!("../web/app/components/alarm-bar.js"),
    },
    // English is always embedded as the fallback locale; the other seed locales are
    // feature-gated so a constrained build embeds only the languages it needs (see
    // `locale_asset`).
    Asset {
        path: "/app/i18n/en.json",
        content_type: JSON,
        bytes: include_bytes!("../web/app/i18n/en.json"),
    },
];

// The optional seed-locale bundles, each baked in only when its feature is on, so Tier B can
// drop the languages a deployment does not need from flash. English is not here; it is always
// embedded in `EMBEDDED`.
#[cfg(not(feature = "tier-c"))]
fn locale_asset(path: &str) -> Option<(&'static str, &'static [u8])> {
    let bytes: &'static [u8] = match path {
        #[cfg(feature = "locale-sw")]
        "/app/i18n/sw.json" => include_bytes!("../web/app/i18n/sw.json"),
        #[cfg(feature = "locale-ar")]
        "/app/i18n/ar.json" => include_bytes!("../web/app/i18n/ar.json"),
        #[cfg(feature = "locale-fr")]
        "/app/i18n/fr.json" => include_bytes!("../web/app/i18n/fr.json"),
        #[cfg(feature = "locale-pt")]
        "/app/i18n/pt.json" => include_bytes!("../web/app/i18n/pt.json"),
        #[cfg(feature = "locale-hi")]
        "/app/i18n/hi.json" => include_bytes!("../web/app/i18n/hi.json"),
        _ => return None,
    };
    Some((JSON, bytes))
}

/// The locale tags this build actually embeds, in menu order (English first).
///
/// The page reads this from `GET /locales` and offers only these languages, so a Tier B build
/// that dropped a locale from flash never shows it in the switcher.
///
/// # Returns
///
/// The embedded locale tags; empty on a floor (`tier-c`) build, which ships no locale bundles.
#[cfg(not(feature = "tier-c"))]
pub(crate) fn embedded_locales() -> Vec<&'static str> {
    let mut tags = vec!["en"];
    #[cfg(feature = "locale-sw")]
    tags.push("sw");
    #[cfg(feature = "locale-ar")]
    tags.push("ar");
    #[cfg(feature = "locale-fr")]
    tags.push("fr");
    #[cfg(feature = "locale-pt")]
    tags.push("pt");
    #[cfg(feature = "locale-hi")]
    tags.push("hi");
    tags
}

/// The locale tags this build embeds; empty on a floor (`tier-c`) build.
///
/// # Returns
///
/// An empty list: the floor page bakes in a single English page and serves no locale bundles.
#[cfg(feature = "tier-c")]
pub(crate) fn embedded_locales() -> Vec<&'static str> {
    Vec::new()
}

// Resolves an embedded asset: the fixed bundle first, then any feature-gated locale bundle.
fn embedded_get(path: &str) -> Option<(&'static str, Vec<u8>)> {
    if let Some(asset) = EMBEDDED.iter().find(|a| a.path == path) {
        return Some((asset.content_type, asset.bytes.to_vec()));
    }
    #[cfg(not(feature = "tier-c"))]
    if let Some((content_type, bytes)) = locale_asset(path) {
        return Some((content_type, bytes.to_vec()));
    }
    None
}

// The MIME type for a file, by extension, for the directory (development) mode.
fn mime_for(path: &str) -> &'static str {
    if path.ends_with(".html") {
        HTML
    } else if path.ends_with(".css") {
        CSS
    } else if path.ends_with(".js") || path.ends_with(".mjs") {
        JS
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".json") {
        "application/json; charset=utf-8"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else if path.ends_with(".woff2") {
        "font/woff2"
    } else {
        "application/octet-stream"
    }
}

/// Where the page assets come from.
#[derive(Clone, Debug)]
pub enum Assets {
    /// Baked into the binary at compile time: the production path.
    Embedded,
    /// Read from a directory on each request: the hot-reloading development path.
    #[cfg(feature = "serve")]
    Dir(PathBuf),
}

impl Assets {
    /// Resolves a request path to a file's MIME type and bytes.
    ///
    /// The request path `"/"` resolves to the page shell. In [`Assets::Dir`] mode any
    /// file under the directory is served (typed by extension), read fresh from disk so
    /// edits show up on reload; a path that escapes the directory resolves to `None`.
    ///
    /// # Arguments
    ///
    /// * `path` - the request path, such as `"/app/app.js"`.
    ///
    /// # Returns
    ///
    /// The MIME type and the file's bytes, or `None` if no asset matches.
    pub fn get(&self, path: &str) -> Option<(&'static str, Vec<u8>)> {
        match self {
            Assets::Embedded => embedded_get(path),
            #[cfg(feature = "serve")]
            Assets::Dir(root) => {
                let relative = if path == "/" {
                    "index.html"
                } else {
                    path.trim_start_matches('/')
                };
                // Refuse anything that tries to climb out of the asset directory.
                if relative.contains("..") {
                    return None;
                }
                let bytes = std::fs::read(root.join(relative)).ok()?;
                Some((mime_for(relative), bytes))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The dashboard's whole point is that it works on a node's own hotspot with no route
    // to the internet, so nothing it serves may fetch from a remote origin. A stylesheet
    // from a font host is the way this goes wrong: it blocks first paint until the
    // connection times out, on exactly the deployment the crate advertises. A URL written
    // in a comment is harmless and does not count; only the forms that load something do.
    #[test]
    fn nothing_served_fetches_from_off_the_device() {
        const LOADS: &[&str] = &[
            "src=\"http",
            "src='http",
            "href=\"http",
            "href='http",
            "@import",
            "url(http",
            "fetch(\"http",
            "fetch('http",
        ];
        for asset in EMBEDDED {
            let text = String::from_utf8_lossy(asset.bytes);
            for form in LOADS {
                assert!(
                    !text.contains(form),
                    "{} loads from off the device with `{form}`",
                    asset.path
                );
            }
        }
    }

    #[test]
    fn embedded_serves_the_shell_for_root() {
        let (content_type, bytes) = Assets::Embedded.get("/").expect("shell present");
        assert_eq!(content_type, HTML);
        assert!(!bytes.is_empty());
    }

    #[cfg(not(feature = "tier-c"))]
    #[test]
    fn embedded_serves_the_app_entry_and_framework() {
        assert!(Assets::Embedded.get("/zquery.min.js").is_some());
        assert!(Assets::Embedded.get("/app/app.js").is_some());
        assert!(Assets::Embedded.get("/global.css").is_some());
    }

    #[cfg(feature = "tier-c")]
    #[test]
    fn tier_c_embeds_only_the_floor_page() {
        // The floor image is the single self-contained page and nothing else: the rich app
        // modules must not be baked in, so the firmware stays tiny.
        let (content_type, _) = Assets::Embedded.get("/").expect("floor page present");
        assert_eq!(content_type, HTML);
        assert!(Assets::Embedded.get("/app/app.js").is_none());
        assert!(Assets::Embedded.get("/zquery.min.js").is_none());
    }

    #[cfg(not(feature = "tier-c"))]
    #[test]
    fn the_full_bundle_also_embeds_the_floor_page() {
        // The full app links to the floor page (the top bar's "Lite" link), so it must be
        // embedded beside the shell, not only in the tier-c image.
        let (content_type, _) = Assets::Embedded
            .get("/lite.html")
            .expect("floor page embedded");
        assert_eq!(content_type, HTML);
    }

    #[cfg(not(feature = "tier-c"))]
    #[test]
    fn embedded_serves_the_lib_modules() {
        // The feature and helper modules live under app/lib (with the visualizations split
        // into app/lib/viz), so the embedded bundle must resolve those nested paths.
        assert!(Assets::Embedded.get("/app/lib/feed.js").is_some());
        assert!(Assets::Embedded.get("/app/lib/catalog.js").is_some());
        assert!(Assets::Embedded.get("/app/lib/discovery.js").is_some());
        assert!(Assets::Embedded.get("/app/lib/viz/index.js").is_some());
    }

    #[cfg(not(feature = "tier-c"))]
    #[test]
    fn embedded_serves_the_pairing_and_crypto_modules() {
        // app.js imports these unconditionally (pairing/control and its pure-JS crypto); a
        // missing one 404s and the page never mounts, so the embedded bundle must carry them.
        for path in [
            "/app/components/pairing-modal.js",
            "/app/lib/pair.js",
            "/app/lib/crypto/bytes.js",
            "/app/lib/crypto/sha256.js",
            "/app/lib/crypto/hmac.js",
            "/app/lib/crypto/hkdf.js",
        ] {
            assert!(Assets::Embedded.get(path).is_some(), "missing {path}");
        }
    }

    #[cfg(not(feature = "tier-c"))]
    #[test]
    fn every_viz_kind_is_known_to_the_page_renderer() {
        // The Rust `Viz` vocabulary and the page's renderer are one contract across languages:
        // every kind a profile can choose must be one `viz/index.js` can draw, or a custom
        // element would silently fall back to a sparkline. This checks the embedded renderer
        // references each kind, so the two cannot drift apart unnoticed.
        let (_, bytes) = Assets::Embedded
            .get("/app/lib/viz/index.js")
            .expect("viz/index.js is embedded");
        let js = String::from_utf8(bytes).expect("viz/index.js is utf8");
        for viz in pamoja_profile::Viz::ALL {
            let token = format!("'{}'", viz.kind());
            assert!(
                js.contains(&token),
                "the page renderer does not know viz kind {}",
                viz.kind()
            );
        }
    }

    #[cfg(all(not(feature = "tier-c"), feature = "all-locales"))]
    #[test]
    fn the_full_build_embeds_all_six_seed_locales() {
        assert_eq!(embedded_locales(), ["en", "sw", "ar", "fr", "pt", "hi"]);
    }

    #[cfg(not(feature = "tier-c"))]
    #[test]
    fn english_is_embedded_and_every_listed_locale_is_served() {
        // English is always the fallback, and whatever a build reports it embeds must actually
        // resolve - so a Tier B subset and its `GET /locales` list can never disagree.
        assert_eq!(embedded_locales().first(), Some(&"en"));
        for locale in embedded_locales() {
            let path = format!("/app/i18n/{locale}.json");
            assert!(
                Assets::Embedded.get(&path).is_some(),
                "listed but not served: {locale}"
            );
        }
    }

    #[cfg(feature = "tier-c")]
    #[test]
    fn tier_c_embeds_no_locale_bundles() {
        // The floor page bakes in a single English page and serves no locale JSON.
        assert!(embedded_locales().is_empty());
        assert!(Assets::Embedded.get("/app/i18n/en.json").is_none());
    }

    #[cfg(not(feature = "tier-c"))]
    #[test]
    fn the_embedded_bundle_fits_the_flash_budget() {
        // A regression tripwire for the firmware image: the embedded bundle (the app plus the
        // locales this build kept) stays small enough to fit a constrained flash partition.
        const FLASH_BUDGET: usize = 600 * 1024;
        let mut total: usize = EMBEDDED.iter().map(|a| a.bytes.len()).sum();
        for locale in embedded_locales() {
            if locale == "en" {
                continue;
            }
            if let Some((_, bytes)) = locale_asset(&format!("/app/i18n/{locale}.json")) {
                total += bytes.len();
            }
        }
        assert!(
            total <= FLASH_BUDGET,
            "embedded bundle is {total} bytes, over the {FLASH_BUDGET}-byte flash budget"
        );
    }

    #[test]
    fn an_unknown_path_resolves_to_nothing() {
        assert!(Assets::Embedded.get("/secret").is_none());
    }

    #[test]
    fn mime_types_follow_the_extension() {
        assert_eq!(mime_for("index.html"), HTML);
        assert_eq!(mime_for("app/app.js"), JS);
        assert_eq!(mime_for("global.css"), CSS);
    }
}
