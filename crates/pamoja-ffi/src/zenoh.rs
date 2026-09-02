//! The C ABI for Zenoh key expressions.
//!
//! These wrap [`pamoja_zenoh::keyexpr`] for callers that reach the SDK through
//! the flat C boundary. A key expression is how a Zenoh network addresses data:
//! a slash-separated path that may carry the `*` and `**` wildcards, so one
//! subscriber can name a whole subtree of a fleet rather than each node in it.
//!
//! Only the naming rules cross. Running a Zenoh session needs the std-only
//! zenoh stack, which stays behind the crate's `runtime` feature and out of the
//! shipped libraries, so a caller who wants a live session uses the Rust crate.

use std::ffi::c_char;
use std::ptr;

use pamoja_zenoh::keyexpr;

use crate::{read_str, PamojaString};

/// Reports whether a key expression is well formed.
///
/// # Arguments
///
/// * `key` - the expression to check, as null-terminated UTF-8.
///
/// # Returns
///
/// `true` when the expression is valid, or `false` if it is malformed or `key`
/// is null.
///
/// # Safety
///
/// `key` must be a valid null-terminated UTF-8 string for the duration of the
/// call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_keyexpr_is_valid(key: *const c_char) -> bool {
    match read_str(key, "key") {
        Some(key) => keyexpr::is_valid(key),
        None => false,
    }
}

/// Reports whether a key expression is already in its canonical form.
///
/// # Arguments
///
/// * `key` - the expression to check, as null-terminated UTF-8.
///
/// # Returns
///
/// `true` when the expression is canonical, or `false` if it is not or `key` is
/// null.
///
/// # Safety
///
/// `key` must be a valid null-terminated UTF-8 string for the duration of the
/// call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_keyexpr_is_canon(key: *const c_char) -> bool {
    match read_str(key, "key") {
        Some(key) => keyexpr::is_canon(key),
        None => false,
    }
}

/// Rewrites a key expression into its canonical form.
///
/// Two expressions that select the same data have one canonical form, so
/// canonizing before comparing or routing avoids treating `a/**/**/b` and
/// `a/**/b` as different.
///
/// # Arguments
///
/// * `key` - the expression to canonize, as null-terminated UTF-8.
///
/// # Returns
///
/// A string the caller must release with
/// [`pamoja_string_free`](crate::pamoja_string_free), or null if the expression
/// is malformed or `key` is null.
///
/// # Safety
///
/// `key` must be a valid null-terminated UTF-8 string for the duration of the
/// call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_keyexpr_canonize(key: *const c_char) -> *mut PamojaString {
    let Some(key) = read_str(key, "key") else {
        return ptr::null_mut();
    };
    match keyexpr::canonize(key) {
        Some(canonical) => PamojaString::into_raw(canonical),
        None => {
            crate::set_last_error(format!("`{key}` is not a valid key expression"));
            ptr::null_mut()
        }
    }
}

/// Reports whether a pattern selects a key.
///
/// # Arguments
///
/// * `pattern` - the expression that may carry wildcards, as null-terminated
///   UTF-8.
/// * `key` - the concrete key to test against it, as null-terminated UTF-8.
///
/// # Returns
///
/// `true` when the pattern selects the key, or `false` if it does not or either
/// argument is null.
///
/// # Safety
///
/// Both arguments must be valid null-terminated UTF-8 strings for the duration
/// of the call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_keyexpr_matches(
    pattern: *const c_char,
    key: *const c_char,
) -> bool {
    let (Some(pattern), Some(key)) = (read_str(pattern, "pattern"), read_str(key, "key")) else {
        return false;
    };
    keyexpr::matches(pattern, key)
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;

    #[test]
    fn a_wildcard_pattern_selects_a_node_beneath_it() {
        let pattern = CString::new("fleet/*/battery").expect("static");
        let key = CString::new("fleet/n7/battery").expect("static");
        assert!(unsafe { pamoja_keyexpr_matches(pattern.as_ptr(), key.as_ptr()) });
    }

    #[test]
    fn a_redundant_double_wildcard_canonizes_away() {
        let key = CString::new("fleet/**/**/battery").expect("static");
        let canonical = unsafe { pamoja_keyexpr_canonize(key.as_ptr()) };
        assert!(!canonical.is_null());
        let text = unsafe { std::ffi::CStr::from_ptr(crate::pamoja_string_data(canonical)) };
        assert_eq!(text.to_str().expect("utf-8"), "fleet/**/battery");
        unsafe { crate::pamoja_string_free(canonical) };
    }

    #[test]
    fn a_null_key_is_rejected_rather_than_dereferenced() {
        assert!(!unsafe { pamoja_keyexpr_is_valid(ptr::null()) });
        assert!(!unsafe { pamoja_keyexpr_is_canon(ptr::null()) });
        assert!(unsafe { pamoja_keyexpr_canonize(ptr::null()) }.is_null());
        assert!(!unsafe { pamoja_keyexpr_matches(ptr::null(), ptr::null()) });
    }
}
