//! Local-first dashboard for a pamoja node.
//!
//! A node serves its own dashboard over its own WiFi hotspot, so a clinic worker, a
//! farmer, or a water committee sees their own data with no internet at all, in their
//! own language, on whatever cheap phone they have. This crate is the host side of
//! that: it turns the state a node already holds into a small, language-neutral
//! snapshot and serves a hand-built, localized page that renders it.
//!
//! The design rests on one split. The device emits only a [`State`] - stable keys,
//! stable codes, raw values, and canonical units, identical in every locale - and the
//! page does all rendering, formatting, and translation at the surface. That keeps the
//! device's job tiny enough for constrained hardware and the page's job rich enough to
//! be beautiful, and it means localization is a property of the page, not a fork of
//! the data.
//!
//! The pieces:
//!
//! - [`State`] is the language-neutral fleet snapshot served at `GET /state`:
//!   [`Org`]s of [`Group`]s of [`Sensor`]s, each group on its own [`Link`].
//! - [`StateSource`] is the one seam between the dashboard and its data; a real
//!   gateway and the `Mock` both implement it.
//! - `Mock` serves a deterministic `Scenario` so the whole dashboard runs and is
//!   debugged with no hardware.
//! - [`Server`] serves the page, the snapshot, and a live event stream over plain TCP.
//!
//! # Capability tiers
//!
//! One design serves hardware from a Raspberry Pi to a microcontroller, chosen with a
//! compile-time tier feature. The `/state` contract is identical across all of them, so a
//! page written for one tier reads another tier's data:
//!
//! - Tiers A and B (`tier-a`, the default, and `tier-b`) embed the full localized app: the
//!   hand-built visuals, history, authenticated control, and the seed locales. A Tier B build
//!   trims flash by embedding only the locales it needs (the `locale-*` features, English
//!   always included); the page learns the embedded set from `GET /locales` and offers only
//!   those languages.
//! - Tier C (`tier-c`) embeds only a single self-contained floor page for the smallest
//!   hardware. It renders the status table with the smallest possible script, and when
//!   scripting is off entirely it falls back to `GET /lite`, a server-rendered,
//!   meta-refreshing table with no script at all. It is plain, but it is legible and it
//!   works on any browser.
//!
//! Build a non-default tier with `--no-default-features`, for example
//! `--no-default-features --features "serve,tier-c"`. Each tier's gzipped page-load budget
//! is enforced by `cargo xtask dashboard footprint`.
//!
//! # Examples
//!
//! A device turns the state it holds into the language-neutral snapshot the page fetches:
//!
//! ```
//! use pamoja_dashboard::{State, Status};
//!
//! let state = State {
//!     orgs: Vec::new(),
//!     status: Status::Alarm,
//!     uptime_secs: Some(3600),
//!     demo: false,
//! };
//!
//! let json = state.to_json().expect("serialize");
//! assert!(json.contains("\"status\":\"alarm\""));
//! ```
//!
//! The hardware-free `Mock` fleet (the `mock` feature) implements [`StateSource`] the same
//! way a real node does, so the whole dashboard runs and is debugged with no hardware.

mod assets;
mod command;
mod source;
mod state;

#[cfg(feature = "mock")]
mod mock;

#[cfg(feature = "serve")]
mod auth;
#[cfg(feature = "serve")]
mod catalog;
#[cfg(feature = "serve")]
mod fleet;
#[cfg(feature = "serve")]
mod lite;
#[cfg(feature = "serve")]
mod serve;

// Capability tier is a compile-time choice. The full bundle ships unless `tier-c` is set, so
// the documented real build (`--no-default-features --features serve`) keeps the full page;
// `tier-c` embeds only the minimal floor page. Selecting `tier-c` alongside a full tier is
// ambiguous about which page to embed, so it is rejected with a clear pointer.
#[cfg(all(feature = "tier-c", any(feature = "tier-a", feature = "tier-b")))]
compile_error!(
    "select one dashboard tier: build a non-default tier with --no-default-features, \
     e.g. --no-default-features --features \"serve,tier-c\""
);

pub use assets::Assets;
pub use command::{Command, CommandError};
// The presentation vocabulary a profile uses to declare custom dashboard elements, so a
// gateway can build a catalog and pin a reading's graphic from this one crate.
pub use pamoja_profile::{ElementSpec, Presentation, Scope, Theme, Viz};
pub use source::StateSource;
pub use state::{
    EventLevel, EventRecord, Group, Link, LinkKind, Mode, Org, Reading, Sensor, State, Status,
    Trend,
};

#[cfg(feature = "mock")]
pub use mock::{Mock, Scenario};

#[cfg(feature = "serve")]
pub use auth::{Auth, AuthError, Challenge};
#[cfg(feature = "serve")]
pub use catalog::Catalog;
#[cfg(feature = "serve")]
pub use fleet::{Fleet, FleetBuilder};
#[cfg(feature = "serve")]
pub use serve::Server;
