#![cfg_attr(not(any(test, feature = "runtime")), no_std)]

//! Zenoh key-expression logic and transport for the pamoja SDK.
//!
//! Zenoh addresses data by key expressions: `/`-joined chunks with a small, exact wildcard
//! language. Before any session is opened, a node has to know whether a key is well-formed, what
//! its one canonical spelling is, whether a subscription pattern matches a published key, and how
//! two patterns relate. That is pure string logic with no I/O, and getting it wrong silently drops
//! or misroutes messages, so it lives here as checked logic anchored to the Zenoh specification
//! and tested against Zenoh's own implementation.
//!
//! See [`keyexpr`] for the rules and the operations:
//!
//! - validity: a key expression is `/`-joined non-empty chunks with no leading, trailing, or
//!   doubled `/`, where `*` and `**` are whole-chunk wildcards and `$*` is a sub-chunk wildcard. A
//!   chunk that starts with `@` is verbatim, and no wildcard selects it.
//! - canonical form: two expressions that select the same keys share one spelling, so equality is
//!   a string comparison; [`canonize`](keyexpr::canonize) produces it, and a Zenoh session accepts
//!   nothing else. [`join`](keyexpr::join) builds a key beneath a prefix in that form.
//! - matching: [`matches`](keyexpr::matches) tests whether a concrete key is selected by a pattern,
//!   the routing question a subscriber asks of every publication.
//! - relations: [`intersects`](keyexpr::intersects) tests whether two patterns share a key, and
//!   [`includes`](keyexpr::includes) whether one covers every key of another, the questions a
//!   router and a bridge ask of two subscriptions.
//!
//! With the `runtime` feature on, `ZenohTransport` adds the live half: it opens a Zenoh session
//! and implements the core `Transport`, so Zenoh becomes the efficient
//! edge-to-edge and fleet transport behind the same surface as every other link.
//!
//! # Examples
//!
//! ```
//! use pamoja_zenoh::keyexpr::{canonize, includes, intersects, matches};
//!
//! // A subscription with a single-chunk wildcard selects a matching publication.
//! assert!(matches("room275/*/temperature", "room275/device1/temperature"));
//! assert!(!matches("room275/*/temperature", "room275/temperature"));
//!
//! // `**/*` is valid but not canonical; its one canonical spelling puts the `*` first.
//! assert_eq!(canonize("robot/sensor/**/*").as_deref(), Some("robot/sensor/*/**"));
//!
//! // Two patterns overlap when some key belongs to both, and one covers the other when every
//! // key of the second belongs to the first.
//! assert!(intersects("room275/*/temperature", "room275/device1/**"));
//! assert!(includes("room275/**", "room275/*/temperature"));
//! ```

extern crate alloc;

pub mod keyexpr;

#[cfg(feature = "runtime")]
mod transport;

#[cfg(feature = "runtime")]
pub use transport::{Message, ZenohConfig, ZenohTransport};
