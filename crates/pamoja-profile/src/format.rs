//! The format a manifest or a rule file is written in, and the wording of what the parser
//! refuses in one.

/// The version of the profile manifest and rule file formats this build reads and writes.
pub const FORMAT: u32 = 1;

/// Checks the `$schema` a file names against the format this build reads.
///
/// A file may leave `$schema` out, name the published schema, or name a copy of it by the
/// same file name, such as `./profile-1.json` for an editor working offline.
///
/// # Arguments
///
/// * `schema` - the `$schema` the file names, if any.
/// * `file` - the kind of file, `"profile"` or `"rules"`, which is the schema's file stem.
/// * `published` - the address of the schema this build writes.
///
/// # Returns
///
/// Nothing when the file is written in the format this build reads.
///
/// # Errors
///
/// The reason, naming the format the file is written in or the schema it names.
pub(crate) fn check_schema(
    schema: Option<&str>,
    file: &str,
    published: &str,
) -> Result<(), String> {
    let Some(schema) = schema else {
        return Ok(());
    };
    let name = schema.rsplit('/').next().unwrap_or(schema);
    let version = name
        .strip_prefix(file)
        .and_then(|rest| rest.strip_prefix('-'))
        .and_then(|rest| rest.strip_suffix(".json"))
        .and_then(|digits| digits.parse::<u32>().ok());
    match version {
        Some(FORMAT) => Ok(()),
        Some(other) => Err(format!(
            "this file is written in {file} format {other}, and this build of pamoja reads format {FORMAT}"
        )),
        None => Err(format!(
            "`$schema` is `{schema}`, which is not a pamoja {file} schema such as `{published}`"
        )),
    }
}

/// Rewrites a parse error so a misspelled field or value names the one it was probably
/// meant to be.
///
/// # Arguments
///
/// * `error` - the error the JSON parser returned.
///
/// # Returns
///
/// The parser's own message, with the nearest known name added when a field or a value
/// was not one the file allows and one of the allowed names is a letter or two away.
#[cfg(feature = "json")]
pub(crate) fn explain(error: &serde_json::Error) -> String {
    let text = error.to_string();
    let Some((what, rest)) = ["unknown field `", "unknown variant `"]
        .into_iter()
        .find_map(|prefix| text.strip_prefix(prefix).map(|rest| (prefix, rest)))
    else {
        return text;
    };
    let Some((given, rest)) = rest.split_once('`') else {
        return text;
    };
    let allowed = rest.split('`').skip(1).step_by(2);
    let Some(nearest) = nearest(given, allowed) else {
        return text;
    };
    let at = text
        .rfind(" at line ")
        .map(|index| &text[index..])
        .unwrap_or_default();
    format!("{what}{given}`, did you mean `{nearest}`?{at}")
}

/// Picks the allowed name a given one is most likely a misspelling of.
///
/// # Arguments
///
/// * `given` - the name the file used.
/// * `allowed` - the names the file may use there.
///
/// # Returns
///
/// The closest allowed name, or `None` when none is within two edits and closer than half
/// the given name's length.
pub(crate) fn nearest<'a>(
    given: &str,
    allowed: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let limit = (given.chars().count() / 2).clamp(1, 2);
    allowed
        .into_iter()
        .map(|name| (distance(given, name), name))
        .filter(|(edits, _)| *edits <= limit)
        .min_by_key(|(edits, _)| *edits)
        .map(|(_, name)| name)
}

/// Counts the single-character insertions, deletions, and substitutions between two names.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, left) in a.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, right) in b.iter().enumerate() {
            let substitute = previous[j] + usize::from(left != *right);
            current.push(substitute.min(previous[j + 1] + 1).min(current[j] + 1));
        }
        previous = current;
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLISHED: &str = "https://pamoja.molex.cloud/schema/profile-1.json";

    #[test]
    fn a_schema_names_the_format_the_file_is_written_in() {
        assert_eq!(check_schema(None, "profile", PUBLISHED), Ok(()));
        assert_eq!(check_schema(Some(PUBLISHED), "profile", PUBLISHED), Ok(()));
        assert_eq!(
            check_schema(Some("./profile-1.json"), "profile", PUBLISHED),
            Ok(())
        );
        let newer = check_schema(
            Some("https://pamoja.molex.cloud/schema/profile-2.json"),
            "profile",
            PUBLISHED,
        )
        .unwrap_err();
        assert!(newer.contains("profile format 2"), "{newer}");
        let other = check_schema(
            Some("https://pamoja.molex.cloud/schema/rules-1.json"),
            "profile",
            PUBLISHED,
        )
        .unwrap_err();
        assert!(other.contains("not a pamoja profile schema"), "{other}");
    }

    #[test]
    fn a_misspelling_is_matched_to_the_name_it_was_meant_to_be() {
        let fields = ["active_secs", "saver_secs", "saver_below", "hysteresis"];
        assert_eq!(nearest("saver_bellow", fields), Some("saver_below"));
        assert_eq!(nearest("hysterisis", fields), Some("hysteresis"));
        assert_eq!(nearest("colour", fields), None);
        assert_eq!(nearest("ab", ["ac", "xy"]), Some("ac"));
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("kitten", "sitting"), 3);
    }

    #[cfg(feature = "json")]
    #[test]
    fn a_parse_error_names_the_field_a_typo_was_meant_to_be() {
        #[derive(Debug, serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        #[allow(dead_code)]
        struct Power {
            saver_below: f32,
            critical_below: f32,
        }
        let error = serde_json::from_str::<Power>(r#"{ "saver_bellow": 0.3 }"#).unwrap_err();
        assert_eq!(
            explain(&error),
            "unknown field `saver_bellow`, did you mean `saver_below`? at line 1 column 16"
        );
        let error = serde_json::from_str::<Power>(r#"{ "colour": 1 }"#).unwrap_err();
        assert!(explain(&error).starts_with("unknown field `colour`, expected"));
    }
}
