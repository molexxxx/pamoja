//! Generated Python bindings for Zenoh key expressions.
//!
//! These mirror the `pamoja-zenoh` Rust API: the naming rules a Zenoh network
//! addresses data by. A key expression is a slash-separated path that may carry
//! the `*` and `**` wildcards, so one subscriber names a whole subtree of a
//! fleet rather than each node in it.
//!
//! Only the naming rules cross. Running a Zenoh session needs the std-only
//! zenoh stack, which would land in every wheel, so it stays behind the Rust
//! crate's `runtime` feature.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use pamoja_zenoh::keyexpr;

/// Reports whether a key expression is well formed.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn keyexpr_is_valid(key: &str) -> bool {
    keyexpr::is_valid(key)
}

/// Reports whether a key expression is already in its canonical form.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn keyexpr_is_canon(key: &str) -> bool {
    keyexpr::is_canon(key)
}

/// Rewrites a key expression into its canonical form, or `None` if it is
/// malformed.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn keyexpr_canonize(key: &str) -> Option<String> {
    keyexpr::canonize(key)
}

/// Reports whether a pattern selects a key.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn keyexpr_matches(pattern: &str, key: &str) -> bool {
    keyexpr::matches(pattern, key)
}
