//! ROS 2 topic and service names: validation and the mapping onto middleware names.
//!
//! ROS 2 names are not free-form strings; the rules (from the ROS 2 design) bound what is legal and
//! how a name reaches the middleware. A name is `/`-separated tokens of alphanumerics and
//! underscores; a token never starts with a digit; the name never has an empty token (`//`), a
//! doubled underscore (`__`), or a trailing `/`. A leading `/` makes it fully qualified; a leading
//! `~/` is the private namespace; balanced `{}` are runtime substitutions. On the wire DDS adds a
//! one-character subsystem prefix, `rt` for topics and `rq`/`rr` for the two halves of a service,
//! and the middleware appends `Request` and `Reply` to a service's two topics, as `rmw_fastrtps`
//! and `rmw_cyclonedds` both build them.

use alloc::format;
use alloc::string::String;

/// The ROS 2 subsystem a name belongs to, which fixes its DDS prefix and suffix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    /// A topic; DDS prefix `rt`, no suffix.
    Topic,
    /// The request side of a service; DDS prefix `rq`, suffix `Request`.
    ServiceRequest,
    /// The reply side of a service; DDS prefix `rr`, suffix `Reply`.
    ServiceResponse,
}

impl EntityKind {
    /// Returns the DDS topic prefix for this subsystem.
    ///
    /// # Returns
    ///
    /// `"rt"` for a topic, `"rq"` for a service request, `"rr"` for a service response.
    pub fn prefix(self) -> &'static str {
        match self {
            EntityKind::Topic => "rt",
            EntityKind::ServiceRequest => "rq",
            EntityKind::ServiceResponse => "rr",
        }
    }

    /// Returns what the middleware appends to a name for this subsystem.
    ///
    /// # Returns
    ///
    /// `""` for a topic, `"Request"` for a service request, and `"Reply"` for a service
    /// response.
    pub fn suffix(self) -> &'static str {
        match self {
            EntityKind::Topic => "",
            EntityKind::ServiceRequest => "Request",
            EntityKind::ServiceResponse => "Reply",
        }
    }
}

/// Returns whether a string is a valid ROS 2 topic or service name.
///
/// # Arguments
///
/// * `name` - the candidate name.
///
/// # Returns
///
/// `true` if `name` obeys the ROS 2 name rules: non-empty, no trailing `/`, no `//` or `__`, every
/// token is alphanumerics and underscores not starting with a digit, any `~` is the first character
/// and (if anything follows) is followed by `/`, and any `{}` substitutions are balanced and hold
/// only alphanumerics and underscores.
///
/// # Examples
///
/// ```
/// use pamoja_ros2::name::is_valid_name;
///
/// assert!(is_valid_name("/robot1/camera_left/image_raw"));
/// assert!(is_valid_name("~/setpoint"));
/// assert!(!is_valid_name("/2foo")); // a token may not start with a digit
/// assert!(!is_valid_name("/foo/")); // no trailing slash
/// assert!(!is_valid_name("/foo//bar")); // no empty token
/// ```
pub fn is_valid_name(name: &str) -> bool {
    if name.is_empty() || name.ends_with('/') {
        return false;
    }
    if name.contains("//") || name.contains("__") {
        return false;
    }
    let bytes = name.as_bytes();
    if let Some(pos) = name.find('~') {
        if pos != 0 || (bytes.len() > 1 && bytes[1] != b'/') {
            return false;
        }
    }

    let mut brace_depth: i32 = 0;
    let mut at_token_start = true;
    for &b in bytes {
        match b {
            b'/' => {
                if brace_depth != 0 {
                    return false; // a `/` may not appear inside a substitution
                }
                at_token_start = true;
            }
            b'~' => at_token_start = false, // legal only at index 0, already checked
            b'{' => {
                brace_depth += 1;
                at_token_start = false;
            }
            b'}' => {
                brace_depth -= 1;
                if brace_depth < 0 {
                    return false;
                }
                at_token_start = false;
            }
            b'0'..=b'9' => {
                if at_token_start {
                    return false; // a token may not start with a digit
                }
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => at_token_start = false,
            _ => return false,
        }
    }
    brace_depth == 0
}

/// Returns whether a name is fully qualified: valid, absolute, and free of substitutions.
///
/// # Arguments
///
/// * `name` - the candidate name.
///
/// # Returns
///
/// `true` if `name` is valid, starts with `/`, and contains neither `~` nor `{}`. Only a fully
/// qualified name can be mapped onto the middleware, because the namespace is already resolved.
pub fn is_fully_qualified(name: &str) -> bool {
    name.starts_with('/') && !name.contains('~') && !name.contains('{') && is_valid_name(name)
}

/// Maps a fully qualified ROS 2 name to its DDS topic name.
///
/// # Arguments
///
/// * `fqn` - a fully qualified name (starting with `/`).
/// * `kind` - the subsystem, which selects the DDS prefix.
///
/// # Returns
///
/// `Some(dds_name)` such as `rt/cmd_vel`, formed by putting the subsystem prefix before the name
/// and its suffix after, so a service's request travels on `rq/<name>Request` and its reply on
/// `rr/<name>Reply`; `None` if `fqn` is not fully qualified.
///
/// # Examples
///
/// ```
/// use pamoja_ros2::name::{dds_topic, EntityKind};
///
/// assert_eq!(dds_topic("/foo", EntityKind::Topic).as_deref(), Some("rt/foo"));
/// assert_eq!(
///     dds_topic("/robot1/camera_left/image_raw", EntityKind::Topic).as_deref(),
///     Some("rt/robot1/camera_left/image_raw"),
/// );
/// assert_eq!(
///     dds_topic("/add_two_ints", EntityKind::ServiceRequest).as_deref(),
///     Some("rq/add_two_intsRequest"),
/// );
/// assert_eq!(
///     dds_topic("/add_two_ints", EntityKind::ServiceResponse).as_deref(),
///     Some("rr/add_two_intsReply"),
/// );
/// assert_eq!(dds_topic("relative", EntityKind::Topic), None); // not fully qualified
/// ```
pub fn dds_topic(fqn: &str, kind: EntityKind) -> Option<String> {
    if !is_fully_qualified(fqn) {
        return None;
    }
    Some(format!("{}{}{}", kind.prefix(), fqn, kind.suffix()))
}

/// Mangles a name by replacing each `/` with `%`, as `rmw_zenoh` does in liveliness tokens.
///
/// # Arguments
///
/// * `name` - the name to mangle.
///
/// # Returns
///
/// The name with every `/` replaced by `%`, for example `/chatter` becomes `%chatter`.
///
/// # Examples
///
/// ```
/// use pamoja_ros2::name::percent_mangle;
///
/// assert_eq!(percent_mangle("/chatter"), "%chatter");
/// assert_eq!(percent_mangle("/robot1/chatter"), "%robot1%chatter");
/// ```
pub fn percent_mangle(name: &str) -> String {
    name.replace('/', "%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_well_formed_names() {
        assert!(is_valid_name("chatter"));
        assert!(is_valid_name("/chatter"));
        assert!(is_valid_name("/robot1/camera_left/image_raw"));
        assert!(is_valid_name("~/setpoint"));
        assert!(is_valid_name("_hidden")); // a leading underscore is allowed
        assert!(is_valid_name("/ns/{node}/out")); // a balanced substitution
    }

    #[test]
    fn rejects_malformed_names() {
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("/foo/"));
        assert!(!is_valid_name("/foo//bar"));
        assert!(!is_valid_name("/foo__bar"));
        assert!(!is_valid_name("/2foo")); // token starts with a digit
        assert!(!is_valid_name("/foo bar")); // space is not allowed
        assert!(!is_valid_name("foo~bar")); // tilde only at the start
        assert!(!is_valid_name("~foo")); // tilde must be followed by a slash
        assert!(!is_valid_name("/ns/{node/out")); // unbalanced brace
    }

    #[test]
    fn fully_qualified_requires_absolute_and_plain() {
        assert!(is_fully_qualified("/foo/bar"));
        assert!(!is_fully_qualified("foo")); // relative
        assert!(!is_fully_qualified("~/foo")); // private
        assert!(!is_fully_qualified("/{ns}/foo")); // substitution
    }

    #[test]
    fn maps_to_dds_topic_names() {
        assert_eq!(
            dds_topic("/foo", EntityKind::Topic).as_deref(),
            Some("rt/foo")
        );
        assert_eq!(
            dds_topic("/robot1/camera_left/image_raw", EntityKind::Topic).as_deref(),
            Some("rt/robot1/camera_left/image_raw"),
        );
        assert_eq!(dds_topic("relative", EntityKind::Topic), None);
    }

    #[test]
    fn a_service_travels_on_the_names_the_middleware_builds() {
        // rmw_cyclonedds builds these in rmw_node.cpp with make_fqtopic, and rmw_fastrtps in
        // rmw_service.cpp with _create_topic_name: the prefix, the name, then the suffix.
        assert_eq!(
            dds_topic("/add_two_ints", EntityKind::ServiceRequest).as_deref(),
            Some("rq/add_two_intsRequest"),
        );
        assert_eq!(
            dds_topic("/add_two_ints", EntityKind::ServiceResponse).as_deref(),
            Some("rr/add_two_intsReply"),
        );
        assert_eq!(EntityKind::Topic.suffix(), "");
    }

    #[test]
    fn percent_mangles_slashes() {
        assert_eq!(percent_mangle("/chatter"), "%chatter");
        assert_eq!(percent_mangle("/robot1/chatter"), "%robot1%chatter");
    }
}
