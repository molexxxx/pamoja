//! The Zenoh key-expression language: validity, canonical form, matching, and the relations
//! between two expressions.
//!
//! A key expression is a `/`-joined list of non-empty chunks. A chunk is either a literal, the
//! single-chunk wildcard `*` (one non-empty chunk), the multi-chunk wildcard `**` (zero or more
//! chunks), or a literal carrying the sub-chunk wildcard `$*` (any run of characters, including
//! none, within one chunk). A concrete key carries no wildcards. Leading, trailing, and doubled
//! `/` are forbidden, as are the bare characters `*`, `$`, `?`, and `#` outside the wildcard forms.
//!
//! A chunk that starts with `@` is verbatim: no wildcard selects it, and only an identical chunk
//! matches it. That seals the keys beneath it off from every expression that does not name it, which
//! is how Zenoh keeps its administration space, `@/...`, out of a subscription on `**`.
//!
//! The rules follow the Zenoh key-expression specification, including its canonical-form rules:
//! `**/**` collapses to `**`, `**/*` reorders to `*/**`, `$*$*` collapses to `$*`, and a chunk that
//! is exactly `$*` becomes `*`. A Zenoh session accepts only the canonical form, so [`is_canon`] is
//! the check a key passes before it goes on the wire, and [`is_valid`] the weaker one that
//! [`canonize`] can repair.
//!
//! Two expressions relate the way two sets do. They [`intersects`] when at least one key belongs
//! to both, which is the question a router asks before forwarding a publication to a subscriber,
//! and one [`includes`] the other when every key of the second belongs to the first, which is how a
//! bridge knows a new subscription adds nothing to one it already holds.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

/// Returns whether a string is a well-formed key expression.
///
/// A well-formed expression may still need [`canonize`] before a Zenoh session accepts it; see
/// [`is_canon`].
///
/// # Arguments
///
/// * `ke` - the candidate key expression.
///
/// # Returns
///
/// `true` if `ke` is `/`-joined non-empty chunks with no leading, trailing, or doubled `/`, where
/// each chunk is `*`, `**`, or a literal in which `*` appears only as part of `$*` and `$` only
/// before `*`, with no `?` or `#`.
///
/// # Examples
///
/// ```
/// use pamoja_zenoh::keyexpr::is_valid;
///
/// assert!(is_valid("fleet/*/battery"));
/// assert!(is_valid("fleet/**/**/battery")); // well formed, though not yet canonical
/// assert!(!is_valid("fleet//battery")); // an empty chunk
/// assert!(!is_valid("fleet/n7*")); // `*` inside a chunk is legal only as `$*`
/// ```
pub fn is_valid(ke: &str) -> bool {
    if ke.is_empty() || ke.starts_with('/') || ke.ends_with('/') {
        return false;
    }
    ke.split('/').all(chunk_valid)
}

/// Returns whether a key expression is valid and in canonical form.
///
/// The canonical form is the only one a Zenoh session accepts: the Zenoh API refuses any other
/// spelling, and a router that receives one drops the message and closes the connection.
///
/// # Arguments
///
/// * `ke` - the candidate key expression.
///
/// # Returns
///
/// `true` if `ke` equals its own [`canonize`] output, so two expressions selecting the same keys
/// compare equal as strings.
///
/// # Examples
///
/// ```
/// use pamoja_zenoh::keyexpr::is_canon;
///
/// assert!(is_canon("fleet/*/**"));
/// assert!(!is_canon("fleet/**/*")); // the same keys, spelled the other way round
/// ```
pub fn is_canon(ke: &str) -> bool {
    canonize(ke).as_deref() == Some(ke)
}

/// Returns the canonical form of a key expression, or `None` if it is invalid.
///
/// # Arguments
///
/// * `ke` - the key expression to canonicalize.
///
/// # Returns
///
/// `Some(canonical)` for a valid `ke`, applying the canonical-form rules (`**/**` to `**`, `**/*`
/// to `*/**`, `$*$*` to `$*`, and a `$*` chunk to `*`); `None` if `ke` is not a valid key
/// expression.
///
/// # Examples
///
/// ```
/// use pamoja_zenoh::keyexpr::canonize;
///
/// assert_eq!(canonize("robot/sensor/**/*").as_deref(), Some("robot/sensor/*/**"));
/// assert_eq!(canonize("a/**/**/b").as_deref(), Some("a/**/b"));
/// assert_eq!(canonize("a//b"), None); // a doubled slash is not a valid key expression
/// ```
pub fn canonize(ke: &str) -> Option<String> {
    if !is_valid(ke) {
        return None;
    }
    let canon_chunks: Vec<String> = ke.split('/').map(canon_chunk).collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < canon_chunks.len() {
        if is_wildcard(canon_chunks[i].as_str()) {
            let mut stars = 0;
            let mut has_multi = false;
            while i < canon_chunks.len() && is_wildcard(canon_chunks[i].as_str()) {
                if canon_chunks[i].as_str() == "*" {
                    stars += 1;
                } else {
                    has_multi = true;
                }
                i += 1;
            }
            out.extend((0..stars).map(|_| String::from("*")));
            if has_multi {
                out.push(String::from("**"));
            }
        } else {
            out.push(canon_chunks[i].clone());
            i += 1;
        }
    }
    Some(out.join("/"))
}

/// Joins two key expressions with a `/` and returns the canonical form of the result.
///
/// This is how a key is built under a prefix, such as a node's own subtree of a fleet, without
/// spelling the separator by hand or leaving a non-canonical pair of wildcards at the seam.
///
/// # Arguments
///
/// * `prefix` - the leading key expression.
/// * `suffix` - the key expression to place beneath it.
///
/// # Returns
///
/// `Some(joined)` in canonical form; `None` if either side is empty or the joined expression is
/// not valid.
///
/// # Examples
///
/// ```
/// use pamoja_zenoh::keyexpr::join;
///
/// assert_eq!(join("fleet/n7", "battery").as_deref(), Some("fleet/n7/battery"));
/// assert_eq!(join("fleet/**", "*").as_deref(), Some("fleet/*/**")); // canonized at the seam
/// assert_eq!(join("fleet/", "battery"), None); // the seam would be an empty chunk
/// ```
pub fn join(prefix: &str, suffix: &str) -> Option<String> {
    canonize(&format!("{prefix}/{suffix}"))
}

/// Returns whether a concrete key is selected by a pattern key expression.
///
/// # Arguments
///
/// * `pattern` - the key expression to test against; it may contain wildcards.
/// * `key` - the concrete key being routed; it must be valid and carry no wildcards.
///
/// # Returns
///
/// `true` if `key` is one of the keys `pattern` selects. Returns `false` if `pattern` is not a
/// valid key expression, or if `key` is not a valid concrete key. To compare two expressions that
/// both carry wildcards, use [`intersects`] or [`includes`].
///
/// # Examples
///
/// ```
/// use pamoja_zenoh::keyexpr::matches;
///
/// assert!(matches("room275/*/temperature", "room275/device1/temperature"));
/// assert!(!matches("room275/*/temperature", "room275/temperature")); // `*` needs one chunk
/// assert!(matches("organizationA/**/temperature", "organizationA/temperature")); // `**` allows none
/// assert!(matches("thermometer$*/temperature", "thermometer1/temperature"));
/// assert!(!matches("**", "@/router/status")); // no wildcard selects a verbatim chunk
/// ```
pub fn matches(pattern: &str, key: &str) -> bool {
    is_valid(key) && !key.contains('*') && includes(pattern, key)
}

/// Returns whether two key expressions share at least one key.
///
/// This is the relation Zenoh routes by: a publication on one expression reaches a subscriber on
/// another exactly when the two intersect. It is symmetric.
///
/// # Arguments
///
/// * `a` - one key expression; it may contain wildcards.
/// * `b` - the other key expression; it may contain wildcards.
///
/// # Returns
///
/// `true` if some concrete key is selected by both; `false` if none is, or if either expression is
/// not valid.
///
/// # Examples
///
/// ```
/// use pamoja_zenoh::keyexpr::intersects;
///
/// assert!(intersects("fleet/*/battery", "fleet/n7/**")); // both select fleet/n7/battery
/// assert!(!intersects("fleet/*/battery", "fleet/*/rack/**"));
/// assert!(!intersects("fleet/@v1/**", "fleet/*/**")); // a verbatim chunk is sealed off
/// ```
pub fn intersects(a: &str, b: &str) -> bool {
    let (Some(a), Some(b)) = (canonize(a), canonize(b)) else {
        return false;
    };
    let a: Vec<&str> = a.split('/').collect();
    let b: Vec<&str> = b.split('/').collect();
    chunks_intersect(&a, &b)
}

/// Returns whether one key expression selects every key another one selects.
///
/// # Arguments
///
/// * `a` - the key expression that may be the wider one.
/// * `b` - the key expression tested for being covered by `a`.
///
/// # Returns
///
/// `true` if every concrete key `b` selects is also selected by `a`, so a subscription on `a`
/// already receives everything one on `b` would; `false` otherwise, or if either expression is
/// not valid. An expression includes itself, and inclusion implies [`intersects`].
///
/// # Examples
///
/// ```
/// use pamoja_zenoh::keyexpr::includes;
///
/// assert!(includes("fleet/**", "fleet/*/battery"));
/// assert!(!includes("fleet/*/battery", "fleet/**"));
/// assert!(includes("fleet/n$*", "fleet/n7")); // the sub-chunk wildcard selects within a chunk
/// ```
pub fn includes(a: &str, b: &str) -> bool {
    let (Some(a), Some(b)) = (canonize(a), canonize(b)) else {
        return false;
    };
    let a: Vec<&str> = a.split('/').collect();
    let b: Vec<&str> = b.split('/').collect();
    chunks_include(&a, &b)
}

fn is_wildcard(chunk: &str) -> bool {
    chunk == "*" || chunk == "**"
}

fn is_verbatim(chunk: &str) -> bool {
    chunk.starts_with('@')
}

fn chunk_valid(chunk: &str) -> bool {
    if chunk.is_empty() {
        return false;
    }
    if is_wildcard(chunk) {
        return true;
    }
    let bytes = chunk.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'?' | b'#' | b'/' => return false,
            b'$' => {
                if i + 1 >= bytes.len() || bytes[i + 1] != b'*' {
                    return false;
                }
                i += 2;
            }
            b'*' => return false,
            _ => i += 1,
        }
    }
    true
}

fn canon_chunk(chunk: &str) -> String {
    if is_wildcard(chunk) {
        return chunk.to_string();
    }
    let mut s = chunk.to_string();
    while s.contains("$*$*") {
        s = s.replace("$*$*", "$*");
    }
    if s == "$*" {
        return String::from("*");
    }
    s
}

fn chunks_intersect(a: &[&str], b: &[&str]) -> bool {
    let width = b.len() + 1;
    let mut below = vec![false; width];
    let mut row = vec![false; width];
    below[b.len()] = true;
    for j in (0..b.len()).rev() {
        below[j] = b[j] == "**" && below[j + 1];
    }
    for i in (0..a.len()).rev() {
        row[b.len()] = a[i] == "**" && below[b.len()];
        for j in (0..b.len()).rev() {
            row[j] = if a[i] == "**" {
                below[j] || (!is_verbatim(b[j]) && row[j + 1])
            } else if b[j] == "**" {
                row[j + 1] || (!is_verbatim(a[i]) && below[j])
            } else {
                chunk_intersects(a[i], b[j]) && below[j + 1]
            };
        }
        core::mem::swap(&mut row, &mut below);
    }
    below[0]
}

fn chunks_include(a: &[&str], b: &[&str]) -> bool {
    let width = b.len() + 1;
    let mut below = vec![false; width];
    let mut row = vec![false; width];
    below[b.len()] = true;
    for i in (0..a.len()).rev() {
        row[b.len()] = a[i] == "**" && below[b.len()];
        for j in (0..b.len()).rev() {
            row[j] = if a[i] == "**" {
                below[j] || (!is_verbatim(b[j]) && row[j + 1])
            } else {
                b[j] != "**" && chunk_includes(a[i], b[j]) && below[j + 1]
            };
        }
        core::mem::swap(&mut row, &mut below);
    }
    below[0]
}

fn chunk_intersects(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if is_verbatim(a) || is_verbatim(b) {
        return false;
    }
    a == "*" || b == "*" || globs_intersect(a, b)
}

fn chunk_includes(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if is_verbatim(a) || is_verbatim(b) {
        return false;
    }
    if a == "*" {
        return true;
    }
    b != "*" && a.contains("$*") && glob_includes(a, b)
}

fn glob_tokens(chunk: &str) -> Vec<Option<u8>> {
    let bytes = chunk.as_bytes();
    let mut tokens = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            tokens.push(None);
            i += 2;
        } else {
            tokens.push(Some(bytes[i]));
            i += 1;
        }
    }
    tokens
}

fn globs_intersect(a: &str, b: &str) -> bool {
    let a = glob_tokens(a);
    let b = glob_tokens(b);
    let width = b.len() + 1;
    let mut below = vec![false; width];
    let mut row = vec![false; width];
    below[b.len()] = true;
    for j in (0..b.len()).rev() {
        below[j] = b[j].is_none() && below[j + 1];
    }
    for i in (0..a.len()).rev() {
        row[b.len()] = a[i].is_none() && below[b.len()];
        for j in (0..b.len()).rev() {
            row[j] = match (a[i], b[j]) {
                (None, _) => below[j] || row[j + 1],
                (_, None) => row[j + 1] || below[j],
                (Some(x), Some(y)) => x == y && below[j + 1],
            };
        }
        core::mem::swap(&mut row, &mut below);
    }
    below[0]
}

fn glob_includes(a: &str, b: &str) -> bool {
    let mut segments: Vec<&str> = a.split("$*").collect();
    let Some(last) = segments.pop() else {
        return false;
    };
    let Some(rest) = b.strip_prefix(segments[0]) else {
        return false;
    };
    let Some(mut rest) = rest.strip_suffix(last) else {
        return false;
    };
    for needle in segments[1..].iter().filter(|needle| !needle.is_empty()) {
        match rest.find(needle) {
            Some(at) => rest = &rest[at + needle.len()..],
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validity_follows_the_chunk_rules() {
        assert!(is_valid("a/b/c"));
        assert!(is_valid("a/*/c"));
        assert!(is_valid("a/**/c"));
        assert!(is_valid("thermometer$*/temperature"));
        assert!(is_valid("@/router/status"));
        assert!(!is_valid("")); // empty
        assert!(!is_valid("/a")); // leading slash
        assert!(!is_valid("a/")); // trailing slash
        assert!(!is_valid("a//b")); // doubled slash
        assert!(!is_valid("a/b*")); // bare `*` inside a chunk
        assert!(!is_valid("a/$x")); // `$` not before `*`
        assert!(!is_valid("a/**b")); // `**` only as a whole chunk
        assert!(!is_valid("a/b?")); // `?` is reserved
        assert!(!is_valid("a/#")); // and so is `#`
    }

    #[test]
    fn matching_against_concrete_keys() {
        assert!(matches(
            "room275/*/temperature",
            "room275/device1/temperature"
        ));
        assert!(!matches("room275/*/temperature", "room275/temperature"));
        assert!(!matches("room275/*/temperature", "room275/a/b/temperature"));

        assert!(matches(
            "organizationA/**/temperature",
            "organizationA/temperature"
        ));
        assert!(matches(
            "organizationA/**/temperature",
            "organizationA/b8/r275/temperature"
        ));

        assert!(matches("**", "anything/at/all"));
        assert!(matches("demo/**", "demo/a/b/c"));
    }

    #[test]
    fn sub_chunk_wildcard_matches_within_a_chunk() {
        assert!(matches(
            "thermometer$*/temperature",
            "thermometer1/temperature"
        ));
        assert!(matches(
            "thermometer$*/temperature",
            "thermometerA/temperature"
        ));
        assert!(matches(
            "thermometer$*/temperature",
            "thermometer/temperature"
        )); // `$*` may be empty
        assert!(!matches(
            "thermometer$*/temperature",
            "xthermometer1/temperature"
        ));
        assert!(matches("a$*b$*c", "aXXbYYc"));
        assert!(!matches("a$*b$*c", "aXXc")); // the middle `b` is missing
    }

    #[test]
    fn a_pattern_does_not_match_a_key_with_wildcards() {
        assert!(!matches("a/*", "a/*"));
        assert!(!matches("a/b", "a/*"));
    }

    #[test]
    fn no_wildcard_selects_a_verbatim_chunk() {
        assert!(!matches("**", "@/router/status"));
        assert!(!matches("*/router/status", "@/router/status"));
        assert!(!matches("fleet/*/battery", "fleet/@v1/battery"));
        assert!(!matches("fleet/**/battery", "fleet/@v1/battery"));
        assert!(!matches("fleet/$*/battery", "fleet/@v1/battery"));
        assert!(matches("@/**", "@/router/status"));
        assert!(matches("fleet/@v1/*", "fleet/@v1/battery"));
        assert!(matches("fleet/**/@v1/battery", "fleet/n7/@v1/battery"));
        // `$*` inside a verbatim chunk is part of its name, not a wildcard.
        assert!(!matches("@v$*", "@v1"));
        // A chunk is verbatim only when `@` leads it.
        assert!(matches("fleet/*", "fleet/n@7"));
    }

    #[test]
    fn canonical_form_examples() {
        assert_eq!(
            canonize("robot/sensor/**/*").as_deref(),
            Some("robot/sensor/*/**")
        );
        assert_eq!(canonize("a/**/**/b").as_deref(), Some("a/**/b"));
        assert_eq!(canonize("a/x$*$*y/$*").as_deref(), Some("a/x$*y/*"));
        assert_eq!(canonize("**/*/*").as_deref(), Some("*/*/**"));

        assert!(is_canon("robot/sensor/*/**"));
        assert!(!is_canon("robot/sensor/**/*"));
        assert!(!is_canon("a//b")); // invalid is never canonical
    }

    // The canonical-form statements of the Key Expressions RFC, as zenoh-keyexpr's own canon.rs
    // checks them.
    #[test]
    fn canonical_form_matches_the_rfc_statements() {
        let cases = [
            ("hello/foo$*$*/bar", "hello/foo$*/bar"),
            ("hello/**/**/bye", "hello/**/bye"),
            ("hello/**/**", "hello/**"),
            ("hello/$*/bye", "hello/*/bye"),
            ("hello/$*$*/bye", "hello/*/bye"),
            ("$*/hello/$*/bye", "*/hello/*/bye"),
            ("$*$*$*/hello/$*/bye/$*", "*/hello/*/bye/*"),
            ("$*$*$*/hello/$*$*/bye/$*$*", "*/hello/*/bye/*"),
            ("hello/**/*", "hello/*/**"),
        ];
        for (written, canonical) in cases {
            assert_eq!(canonize(written).as_deref(), Some(canonical), "{written}");
        }
        for invalid in [
            "/a/b/",
            "/a/b",
            "a/b/",
            "a/b/*$*",
            "a/b/$**",
            "a/b/**$*",
            "a/b/*$**",
            "a/b/*$***",
            "a/b/**$**",
            "a/b/**$***",
        ] {
            assert_eq!(canonize(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn join_places_a_key_beneath_a_prefix() {
        assert_eq!(
            join("fleet/n7", "battery").as_deref(),
            Some("fleet/n7/battery")
        );
        assert_eq!(join("fleet/**", "**").as_deref(), Some("fleet/**"));
        assert_eq!(join("fleet/**", "*").as_deref(), Some("fleet/*/**"));
        assert_eq!(join("fleet", ""), None);
        assert_eq!(join("", "battery"), None);
        assert_eq!(join("fleet/", "battery"), None);
        assert_eq!(join("fleet", "bat?tery"), None);
    }

    // The examples of the Key Expressions RFC, in the eclipse-zenoh roadmap repository.
    #[test]
    fn relations_match_the_rfc_examples() {
        for key in ["a/c/b", "a/hi/b"] {
            assert!(includes("a/*/b", key), "a/*/b includes {key}");
        }
        for other in ["*/a/b", "*/*/*"] {
            assert!(intersects("a/*/b", other), "a/*/b intersects {other}");
        }
        for other in ["a/*/c", "b/*/a", "a/hi/there/b", "a/hi/*/b"] {
            assert!(
                !intersects("a/*/b", other),
                "a/*/b is disjoint with {other}"
            );
        }

        for other in [
            "a/b",
            "a/**/b/b",
            "a/*/b",
            "a/*/*/b",
            "a/*/**/b",
            "a/**/c/**/b",
        ] {
            assert!(includes("a/**/b", other), "a/**/b includes {other}");
        }
        for other in ["**/b", "a/**"] {
            assert!(intersects("a/**/b", other), "a/**/b intersects {other}");
        }
        assert!(!intersects("a/**/b", "a/**/b/c"));

        assert!(includes("a/c$*/b", "a/cool/b"));
        for other in ["a/*/b", "a/$*c/b"] {
            assert!(intersects("a/c$*/b", other), "a/c$*/b intersects {other}");
        }
        assert!(!intersects("a/c$*/b", "a/uncool/b"));

        let sealed = ["my-api/@v1/**", "my-api/@v2/**", "my-api/@$*/**"];
        let open = ["my-api/*/**", "my-api/**"];
        for (i, a) in sealed.iter().enumerate() {
            for b in sealed[i + 1..].iter().chain(&open) {
                assert!(!intersects(a, b), "{a} and {b} share no key");
            }
        }
        assert!(includes("my-api/**", "my-api/*/**"));
    }

    // Every intersection vector in zenoh-keyexpr's src/key_expr/tests.rs.
    #[test]
    fn intersection_matches_the_zenoh_vectors() {
        let cases = [
            ("a", "a", true),
            ("a/b", "a/b", true),
            ("*", "abc", true),
            ("*", "xxx", true),
            ("ab$*", "abcd", true),
            ("ab$*d", "abcd", true),
            ("ab$*", "ab", true),
            ("ab/*", "ab", false),
            ("a/*/c/*/e", "a/b/c/d/e", true),
            ("a/$*b/c/$*d/e", "a/xb/c/xd/e", true),
            ("a/*/c/*/e", "a/c/e", false),
            ("a/*/c/*/e", "a/b/c/d/x/e", false),
            ("ab$*cd", "abxxcxxd", false),
            ("ab$*cd", "abxxcxxcd", true),
            ("ab$*cd", "abxxcxxcdx", false),
            ("**", "abc", true),
            ("**", "a/b/c", true),
            ("ab/**", "ab", true),
            ("**/xyz", "a/b/xyz/d/e/f/xyz", true),
            ("**/xyz$*xyz", "a/b/xyz/d/e/f/xyz", false),
            ("**/xyz$*xyz", "a/b/xyzdefxyz", true),
            ("a/**/c/**/e", "a/b/b/b/c/d/d/d/e", true),
            ("a/**/c/**/e", "a/c/e", true),
            ("a/**/c/*/e/*", "a/b/b/b/c/d/d/c/d/e/f", true),
            ("a/**/c/*/e/*", "a/b/b/b/c/d/d/c/d/d/e/f", false),
            ("x/abc", "x/abc", true),
            ("x/abc", "abc", false),
            ("x/*", "x/abc", true),
            ("x/*", "abc", false),
            ("*", "x/abc", false),
            ("x/*", "x/abc$*", true),
            ("x/$*abc", "x/abc$*", true),
            ("x/a$*", "x/abc$*", true),
            ("x/a$*de", "x/abc$*de", true),
            ("x/a$*d$*e", "x/a$*e", true),
            ("x/a$*d$*e", "x/a$*c$*e", true),
            ("x/a$*d$*e", "x/ade", true),
            ("x/c$*", "x/abc$*", false),
            ("x/$*d", "x/$*e", false),
            ("@a", "@a", true),
            ("@a", "@ab", false),
            ("@a", "@a/b", false),
            ("@a", "@a/*", false),
            ("@a", "@a/*/**", false),
            ("@a", "@a$*/**", false),
            ("@a", "@a/**", true),
            ("**/xyz$*xyz", "@a/b/xyzdefxyz", false),
            ("@a/**/c/**/e", "@a/b/b/b/c/d/d/d/e", true),
            ("@a/**/c/**/e", "@a/@b/b/b/c/d/d/d/e", false),
            ("@a/**/@c/**/e", "@a/b/b/b/@c/d/d/d/e", true),
            ("@a/**/e", "@a/b/b/d/d/d/e", true),
            ("@a/**/e", "@a/b/b/b/d/d/d/e", true),
            ("@a/**/e", "@a/b/b/c/d/d/d/e", true),
            ("@a/**/e", "@a/b/b/@c/b/d/d/d/e", false),
            ("@a/*", "@a/@b", false),
            ("@a/**", "@a/@b", false),
            ("@a/**/@b", "@a/@b", true),
            ("@a/@b/**", "@a/@b", true),
            ("@a/**/@c/**/@b", "@a/**/@c/@b", true),
            ("@a/**/@c/**/@b", "@a/@c/**/@b", true),
            ("@a/**/@c/@b", "@a/@c/**/@b", true),
            ("@a/**/@b", "@a/**/@c/**/@b", false),
            ("@a", "**/@a", true),
        ];
        for (a, b, want) in cases {
            assert_eq!(intersects(a, b), want, "{a} intersects {b}");
            assert_eq!(intersects(b, a), want, "{b} intersects {a}");
        }
    }

    // Every inclusion vector in zenoh-keyexpr's src/key_expr/tests.rs.
    #[test]
    fn inclusion_matches_the_zenoh_vectors() {
        let cases = [
            ("a", "a", true),
            ("a/b", "a/b", true),
            ("*", "abc", true),
            ("*", "xxx", true),
            ("ab$*", "abcd", true),
            ("ab$*d", "abcd", true),
            ("ab$*", "ab", true),
            ("ab/*", "ab", false),
            ("a/*/c/*/e", "a/b/c/d/e", true),
            ("a/$*b/c/$*d/e", "a/xb/c/xd/e", true),
            ("a/*/c/*/e", "a/c/e", false),
            ("a/*/c/*/e", "a/b/c/d/x/e", false),
            ("ab$*cd", "abxxcxxd", false),
            ("ab$*c$*d", "abxxcxxd", true),
            ("ab$*cd", "abxxcxxcd", true),
            ("ab$*cd", "abxxcxxcdx", false),
            ("**", "abc", true),
            ("**", "a/b/c", true),
            ("ab/**", "ab", true),
            ("**/xyz", "a/b/xyz/d/e/f/xyz", true),
            ("**/xyz$*xyz", "a/b/xyz/d/e/f/xyz", false),
            ("**/xyz$*xyz", "a/b/xyzdefxyz", true),
            ("a/**/c/**/e", "a/b/b/b/c/d/d/d/e", true),
            ("a/**/c/**/e", "a/c/e", true),
            ("a/**/c/*/e/*", "a/b/b/b/c/d/d/c/d/e/f", true),
            ("a/**/c/*/e/*", "a/b/b/b/c/d/d/c/d/d/e/f", false),
            ("x/abc", "x/abc", true),
            ("x/abc", "abc", false),
            ("x/*", "x/abc", true),
            ("x/*", "abc", false),
            ("*", "x/abc", false),
            ("x/*", "x/abc$*", true),
            ("x/$*abc", "x/abc$*", false),
            ("x/a$*", "x/abc$*", true),
            ("x/abc$*", "x/a$*", false),
            ("x/a$*de", "x/abc$*de", true),
            ("x/a$*e", "x/a$*d$*e", true),
            ("x/a$*d$*e", "x/a$*e", false),
            ("x/a$*d$*e", "x/a$*c$*e", false),
            ("x/a$*d$*e", "x/ade", true),
            ("x/c$*", "x/abc$*", false),
            ("x/$*c$*", "x/abc$*", true),
            ("x/$*d", "x/$*e", false),
            ("@a", "@a", true),
            ("@a", "@ab", false),
            ("@a", "@a/b", false),
            ("@a", "@a/*", false),
            ("@a", "@a/*/**", false),
            ("@a$*/**", "@a", false),
            ("@a", "@a/**", false),
            ("@a/**", "@a", true),
            ("**/xyz$*xyz", "@a/b/xyzdefxyz", false),
            ("@a/**/c/**/e", "@a/b/b/b/c/d/d/d/e", true),
            ("@a/*", "@a/@b", false),
            ("@a/**", "@a/@b", false),
            ("@a/**/@b", "@a/@b", true),
            ("@a/@b/**", "@a/@b", true),
        ];
        for (a, b, want) in cases {
            assert_eq!(includes(a, b), want, "{a} includes {b}");
        }
    }

    #[test]
    fn relations_refuse_a_malformed_expression() {
        assert!(!intersects("a//b", "**"));
        assert!(!intersects("**", "a/b?"));
        assert!(!includes("**", "a//b"));
        assert!(!includes("a/", "a"));
    }

    #[test]
    fn relations_read_any_spelling_of_the_same_set() {
        assert!(includes("a/**/**/b", "a/x/b"));
        assert!(intersects("a/**/*", "a/*/**"));
        assert!(includes("a/**/*", "a/*/**") && includes("a/*/**", "a/**/*"));
        assert!(includes("@v$*$*", "@v$*")); // one verbatim chunk, once canonized
    }

    // Checks the three answers against each other on every key a small alphabet spells: a
    // shared key proves an intersection, and inclusion has to hold key by key.
    #[test]
    fn relations_agree_with_the_keys_they_select() {
        let keys = spell(&["a", "b", "ab", "ba", "@a"], 3);
        let patterns = spell(&["a", "@a", "*", "**", "a$*", "$*a", "$*b$*", "a$*b"], 3);
        let selected: Vec<Vec<bool>> = patterns
            .iter()
            .map(|pattern| keys.iter().map(|key| matches(pattern, key)).collect())
            .collect();
        for (a, a_keys) in patterns.iter().zip(&selected) {
            for (b, b_keys) in patterns.iter().zip(&selected) {
                let shared = a_keys.iter().zip(b_keys).any(|(x, y)| *x && *y);
                if shared {
                    assert!(intersects(a, b), "{a} and {b} share a key");
                }
                if includes(a, b) {
                    assert!(intersects(a, b), "{a} includes {b}, so they intersect");
                    let covered = a_keys.iter().zip(b_keys).all(|(x, y)| *x || !*y);
                    assert!(covered, "{a} includes {b}, key by key");
                }
                assert_eq!(intersects(a, b), intersects(b, a), "{a} and {b}");
            }
        }
    }

    fn spell(forms: &[&str], depth: usize) -> Vec<String> {
        let mut spelled: Vec<String> = forms.iter().map(|form| form.to_string()).collect();
        let mut last = spelled.clone();
        for _ in 1..depth {
            last = last
                .iter()
                .flat_map(|head| forms.iter().map(move |form| format!("{head}/{form}")))
                .collect();
            spelled.extend(last.iter().cloned());
        }
        spelled
    }

    // Asks Zenoh's own implementation the same questions, on every expression three chunks of
    // these forms spell, well formed or not. Zenoh 1.10.1's canonizer indexes past the end of a
    // string that ends within two bytes of a doubled `$*`, so the doubled form is spelled with
    // two characters after it.
    #[cfg(feature = "runtime")]
    #[test]
    fn answers_agree_with_zenoh() {
        use zenoh::key_expr::{keyexpr, OwnedKeyExpr};

        let written = spell(
            &[
                "a", "@a", "*", "**", "$*", "$*$*", "a$*$*bc", "@a$*", "a*", "$a", "a?", "#",
            ],
            3,
        );
        for text in &written {
            let theirs = OwnedKeyExpr::autocanonize(text.clone())
                .ok()
                .map(|ke| ke.to_string());
            assert_eq!(canonize(text), theirs, "the canonical form of {text}");
            assert_eq!(
                is_canon(text),
                keyexpr::new(text.as_str()).is_ok(),
                "whether {text} is canonical"
            );
        }

        let canonical: Vec<String> = spell(
            &["a", "ab", "@a", "*", "**", "a$*", "$*b", "a$*b", "$*a$*"],
            3,
        )
        .into_iter()
        .filter(|text| is_canon(text))
        .collect();
        for a in &canonical {
            let theirs_a = keyexpr::new(a.as_str()).expect("a canonical expression");
            for b in &canonical {
                let theirs_b = keyexpr::new(b.as_str()).expect("a canonical expression");
                assert_eq!(
                    intersects(a, b),
                    theirs_a.intersects(theirs_b),
                    "{a} intersects {b}"
                );
                assert_eq!(
                    includes(a, b),
                    theirs_a.includes(theirs_b),
                    "{a} includes {b}"
                );
            }
        }
    }
}
