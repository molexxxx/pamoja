//! Generated Node bindings for Zenoh key expressions.
//!
//! These mirror the `pamoja-zenoh` Rust API: the naming rules a Zenoh network
//! addresses data by. A key expression is a slash-separated path that may carry
//! the `*` and `**` wildcards, so one subscriber names a whole subtree of a
//! fleet rather than each node in it.
//!
//! Only the naming rules cross. Running a Zenoh session needs the std-only
//! zenoh stack, which would land in every npm install, so it stays behind the
//! Rust crate's `runtime` feature. The facade groups these into one `keyexpr`
//! object, which is why the generated names carry that prefix.

use napi_derive::napi;
use pamoja_zenoh::keyexpr;

/// Reports whether a key expression is well formed.
#[napi]
pub fn keyexpr_is_valid(key: String) -> bool {
    keyexpr::is_valid(&key)
}

/// Reports whether a key expression is already in its canonical form.
#[napi]
pub fn keyexpr_is_canon(key: String) -> bool {
    keyexpr::is_canon(&key)
}

/// Rewrites a key expression into its canonical form, or `null` if it is
/// malformed.
///
/// Two expressions that select the same data have one canonical form, so
/// canonizing before comparing or routing avoids treating `a/**/**/b` and
/// `a/**/b` as different.
#[napi]
pub fn keyexpr_canonize(key: String) -> Option<String> {
    keyexpr::canonize(&key)
}

/// Reports whether a pattern selects a key.
///
/// @param pattern - the expression that may carry wildcards.
/// @param key - the concrete key to test against it.
#[napi]
pub fn keyexpr_matches(pattern: String, key: String) -> bool {
    keyexpr::matches(&pattern, &key)
}
