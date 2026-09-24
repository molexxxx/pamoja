//! The C ABI for rules between nodes.
//!
//! A rule file is judged here reading by reading, with no link: the host hands over the
//! topic and the reading each message carried, and learns which rules set or cleared and
//! what each calls for, which the host then carries out. That is the deciding half of the
//! Rust `RuleEngine`, which owns a link and actuators and so cannot cross a C ABI.
//!
//! What fired crosses as JSON, an array of objects with the `rule` by name, the `edge`
//! (`set` or `cleared`), the `reading`, and its `actions`, each written exactly as the
//! rule file writes an action: `{ "drive": ..., "on": ... }` or
//! `{ "publish": ..., "payload": ... }`.

use std::ffi::c_char;
use std::ptr;

use pamoja_kit::Edge;
use pamoja_profile::{Fired, RuleEvaluator, Rules};
use serde_json::{json, Value};

use crate::{read_str, set_last_error, PamojaStatus, PamojaString};

/// An opaque handle to a set of rules armed to judge readings.
///
/// Create it with [`pamoja_rule_evaluator_from_json`] and release it with
/// [`pamoja_rule_evaluator_free`].
pub struct PamojaRuleEvaluator {
    inner: RuleEvaluator,
}

/// Loads a rule file and arms its rules, every condition starting cleared.
///
/// # Arguments
///
/// * `text` - the rule file, as null-terminated UTF-8.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_rule_evaluator_free`], or null if the
/// text is not a rule file or holds a rule no engine could run, with the reason
/// available from [`pamoja_last_error_message`](crate::pamoja_last_error_message).
///
/// # Safety
///
/// `text` must be a valid null-terminated UTF-8 string for the duration of the call, or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_rule_evaluator_from_json(
    text: *const c_char,
) -> *mut PamojaRuleEvaluator {
    let Some(text) = read_str(text, "text") else {
        return ptr::null_mut();
    };
    match Rules::from_json(text).and_then(RuleEvaluator::new) {
        Ok(inner) => Box::into_raw(Box::new(PamojaRuleEvaluator { inner })),
        Err(error) => {
            set_last_error(error.to_string());
            ptr::null_mut()
        }
    }
}

/// Writes the rules back out as the file a fleet shares.
///
/// # Arguments
///
/// * `rules` - the evaluator.
///
/// # Returns
///
/// A string the caller must release with
/// [`pamoja_string_free`](crate::pamoja_string_free), or null if `rules` is null.
///
/// # Safety
///
/// `rules` must be a live handle from [`pamoja_rule_evaluator_from_json`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_rule_evaluator_to_json(
    rules: *const PamojaRuleEvaluator,
) -> *mut PamojaString {
    let Some(rules) = handle(rules) else {
        return ptr::null_mut();
    };
    let file = Rules {
        rules: rules.inner.rules().cloned().collect(),
    };
    match file.to_json() {
        Ok(text) => PamojaString::into_raw(text),
        Err(error) => {
            set_last_error(error.to_string());
            ptr::null_mut()
        }
    }
}

/// Judges one reading from one topic against every rule that watches it.
///
/// # Arguments
///
/// * `rules` - the evaluator.
/// * `topic` - the topic the reading arrived on, as null-terminated UTF-8.
/// * `reading` - the reading, already decoded from the message.
///
/// # Returns
///
/// The JSON array of what fired, in the order the rules are listed, which is `[]` when no
/// rule watches the topic or the reading changed nothing. The caller releases it with
/// [`pamoja_string_free`](crate::pamoja_string_free). Null if a pointer is null, or if a
/// rule watches the topic and the reading is not a finite number, with the reason
/// available from [`pamoja_last_error_message`](crate::pamoja_last_error_message).
///
/// # Safety
///
/// `rules` must be a live handle from [`pamoja_rule_evaluator_from_json`], or null, and
/// `topic` must be a valid null-terminated UTF-8 string for the duration of the call, or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_rule_evaluator_evaluate(
    rules: *mut PamojaRuleEvaluator,
    topic: *const c_char,
    reading: f32,
) -> *mut PamojaString {
    let Some(topic) = read_str(topic, "topic") else {
        return ptr::null_mut();
    };
    if rules.is_null() {
        set_last_error("rules must not be null".to_owned());
        return ptr::null_mut();
    }
    match (*rules).inner.evaluate(topic, reading) {
        Ok(fired) => {
            let list: Vec<Value> = fired.iter().map(fired_json).collect();
            PamojaString::into_raw(Value::Array(list).to_string())
        }
        Err(error) => {
            set_last_error(error.to_string());
            ptr::null_mut()
        }
    }
}

/// Reports whether any rule watches a topic, so the host knows whether to decode a
/// message before handing it over.
///
/// # Arguments
///
/// * `rules` - the evaluator.
/// * `topic` - the topic a message arrived on, as null-terminated UTF-8.
///
/// # Returns
///
/// `true` when some rule watches the topic, and `false` otherwise or if a pointer is
/// null.
///
/// # Safety
///
/// `rules` must be a live handle from [`pamoja_rule_evaluator_from_json`], or null, and
/// `topic` must be a valid null-terminated UTF-8 string for the duration of the call, or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_rule_evaluator_watches(
    rules: *const PamojaRuleEvaluator,
    topic: *const c_char,
) -> bool {
    match (handle(rules), read_str(topic, "topic")) {
        (Some(rules), Some(topic)) => rules.inner.watches(topic),
        _ => false,
    }
}

/// Lists the topics the rules watch, which are the topics to subscribe to.
///
/// # Arguments
///
/// * `rules` - the evaluator.
///
/// # Returns
///
/// A JSON array of the topics, each once, in name order, which the caller releases with
/// [`pamoja_string_free`](crate::pamoja_string_free), or null if `rules` is null.
///
/// # Safety
///
/// `rules` must be a live handle from [`pamoja_rule_evaluator_from_json`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_rule_evaluator_topics_json(
    rules: *const PamojaRuleEvaluator,
) -> *mut PamojaString {
    match handle(rules) {
        Some(rules) => PamojaString::into_raw(json!(rules.inner.topics()).to_string()),
        None => ptr::null_mut(),
    }
}

/// Lists the actuators the rules drive, which are the outputs the host has to have.
///
/// # Arguments
///
/// * `rules` - the evaluator.
///
/// # Returns
///
/// A JSON array of the actuator names, each once, in name order, which the caller
/// releases with [`pamoja_string_free`](crate::pamoja_string_free), or null if `rules`
/// is null.
///
/// # Safety
///
/// `rules` must be a live handle from [`pamoja_rule_evaluator_from_json`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_rule_evaluator_actuators_json(
    rules: *const PamojaRuleEvaluator,
) -> *mut PamojaString {
    match handle(rules) {
        Some(rules) => PamojaString::into_raw(json!(rules.inner.actuators()).to_string()),
        None => ptr::null_mut(),
    }
}

/// Reports whether a rule's condition currently holds.
///
/// # Arguments
///
/// * `rules` - the evaluator.
/// * `rule` - the rule's name, as null-terminated UTF-8.
/// * `out_set` - receives whether the condition holds.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with `*out_set` written, or [`PamojaStatus::InvalidArgument`] if
/// a pointer is null or no rule has that name.
///
/// # Safety
///
/// `rules` must be a live handle from [`pamoja_rule_evaluator_from_json`], or null,
/// `rule` must be a valid null-terminated UTF-8 string for the duration of the call, or
/// null, and `out_set` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_rule_evaluator_is_set(
    rules: *const PamojaRuleEvaluator,
    rule: *const c_char,
    out_set: *mut bool,
) -> PamojaStatus {
    if out_set.is_null() {
        set_last_error("out_set must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let (Some(rules), Some(name)) = (handle(rules), read_str(rule, "rule")) else {
        return PamojaStatus::InvalidArgument;
    };
    match rules.inner.is_set(name) {
        Some(set) => {
            *out_set = set;
            PamojaStatus::Ok
        }
        None => {
            set_last_error(format!("no rule is named `{name}`"));
            PamojaStatus::InvalidArgument
        }
    }
}

/// Releases an evaluator handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `rules` must be a handle from [`pamoja_rule_evaluator_from_json`] that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_rule_evaluator_free(rules: *mut PamojaRuleEvaluator) {
    if !rules.is_null() {
        drop(Box::from_raw(rules));
    }
}

/// Writes what one rule did as the JSON object the host reads.
fn fired_json(fired: &Fired) -> Value {
    let edge = match fired.edge {
        Edge::Set => "set",
        Edge::Cleared => "cleared",
    };
    json!({
        "rule": fired.rule,
        "edge": edge,
        "reading": fired.reading,
        "actions": fired.actions,
    })
}

/// Borrows the evaluator behind a handle, recording why when there is none.
///
/// # Safety
///
/// `rules` must be a live handle from [`pamoja_rule_evaluator_from_json`], or null.
unsafe fn handle<'a>(rules: *const PamojaRuleEvaluator) -> Option<&'a PamojaRuleEvaluator> {
    if rules.is_null() {
        set_last_error("rules must not be null".to_owned());
        None
    } else {
        Some(&*rules)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};

    use super::*;
    use crate::{pamoja_last_error_message, pamoja_string_data, pamoja_string_free};

    const FILE: &str = r#"{ "rules": [ {
        "name": "water-when-dry",
        "when": { "topic": "garden/bed-1/moisture", "below": 30.0, "hysteresis": 5.0 },
        "then": [ { "drive": "bed-valve", "on": true } ],
        "otherwise": [ { "drive": "bed-valve", "on": false },
                       { "publish": "garden/bed-1/valve", "payload": "closed" } ]
    } ] }"#;

    /// Reads and releases a string the ABI handed back.
    unsafe fn take(text: *mut PamojaString) -> String {
        assert!(!text.is_null(), "a string came back");
        let read = CStr::from_ptr(pamoja_string_data(text))
            .to_str()
            .unwrap()
            .to_owned();
        pamoja_string_free(text);
        read
    }

    /// The message the last failed call left behind.
    fn last_error() -> String {
        // Safety: the message pointer is valid until the next failing call on this thread.
        unsafe { CStr::from_ptr(pamoja_last_error_message()) }
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn a_rule_file_judges_readings_and_says_what_to_do() {
        let file = CString::new(FILE).unwrap();
        let topic = CString::new("garden/bed-1/moisture").unwrap();
        let elsewhere = CString::new("garden/bed-2/moisture").unwrap();
        let rule = CString::new("water-when-dry").unwrap();
        // Safety: the handle is created and released here and every string is valid.
        unsafe {
            let rules = pamoja_rule_evaluator_from_json(file.as_ptr());
            assert!(!rules.is_null());
            assert!(pamoja_rule_evaluator_watches(rules, topic.as_ptr()));
            assert!(!pamoja_rule_evaluator_watches(rules, elsewhere.as_ptr()));
            assert_eq!(
                take(pamoja_rule_evaluator_topics_json(rules)),
                r#"["garden/bed-1/moisture"]"#
            );
            assert_eq!(
                take(pamoja_rule_evaluator_actuators_json(rules)),
                r#"["bed-valve"]"#
            );

            assert_eq!(
                take(pamoja_rule_evaluator_evaluate(rules, topic.as_ptr(), 31.0)),
                "[]"
            );
            let dry: Value = serde_json::from_str(&take(pamoja_rule_evaluator_evaluate(
                rules,
                topic.as_ptr(),
                28.0,
            )))
            .unwrap();
            assert_eq!(dry[0]["rule"], "water-when-dry");
            assert_eq!(dry[0]["edge"], "set");
            assert_eq!(
                dry[0]["actions"],
                json!([{ "drive": "bed-valve", "on": true }])
            );
            let mut set = false;
            assert_eq!(
                pamoja_rule_evaluator_is_set(rules, rule.as_ptr(), &mut set),
                PamojaStatus::Ok
            );
            assert!(set);

            let wet: Value = serde_json::from_str(&take(pamoja_rule_evaluator_evaluate(
                rules,
                topic.as_ptr(),
                36.0,
            )))
            .unwrap();
            assert_eq!(wet[0]["edge"], "cleared");
            assert_eq!(wet[0]["actions"][1]["payload"], "closed");

            assert!(pamoja_rule_evaluator_evaluate(rules, topic.as_ptr(), f32::NAN).is_null());
            assert!(last_error().contains("not a finite number"));

            let written = take(pamoja_rule_evaluator_to_json(rules));
            assert_eq!(
                Rules::from_json(&written).unwrap(),
                Rules::from_json(FILE).unwrap()
            );
            pamoja_rule_evaluator_free(rules);
        }
    }

    #[test]
    fn a_rule_file_no_engine_could_run_is_refused_with_the_reason() {
        let filter =
            CString::new(FILE.replace("garden/bed-1/moisture", "garden/+/moisture")).unwrap();
        // Safety: the string is valid for the call.
        let rules = unsafe { pamoja_rule_evaluator_from_json(filter.as_ptr()) };
        assert!(rules.is_null());
        assert!(last_error().contains("a rule watches one topic exactly"));
    }

    #[test]
    fn null_handles_are_tolerated() {
        let rule = CString::new("water-when-dry").unwrap();
        // Safety: every call below is documented to accept null.
        unsafe {
            assert!(!pamoja_rule_evaluator_watches(ptr::null(), rule.as_ptr()));
            assert!(pamoja_rule_evaluator_topics_json(ptr::null()).is_null());
            let mut set = false;
            assert_eq!(
                pamoja_rule_evaluator_is_set(ptr::null(), rule.as_ptr(), &mut set),
                PamojaStatus::InvalidArgument
            );
            pamoja_rule_evaluator_free(ptr::null_mut());
        }
    }
}
