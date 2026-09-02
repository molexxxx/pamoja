//! Generated Node bindings for device-side telemetry.
//!
//! These mirror the `pamoja-telemetry` Rust API: a reporter that ships the events
//! worth their bytes, counts every event it sees whether it ships or not, and
//! moves its own bar as the link gets more expensive.
//!
//! Only the level of an event crosses, because the level is the whole of what the
//! reporter decides on. The facade keeps the code and the optional value beside
//! it and hands back the caller's own event when one should ship.

use napi_derive::napi;
use pamoja_telemetry::{Event, Level as CoreLevel, LinkCost as CoreCost, Reporter as CoreReporter};

/// How urgent an event is.
#[napi(string_enum)]
pub enum Level {
    /// Fine-grained detail, useful only when chasing a specific problem.
    Trace,
    /// Diagnostic detail for development.
    Debug,
    /// A normal, noteworthy event.
    Info,
    /// Something unexpected that the node recovered from.
    Warn,
    /// A failure that needs attention.
    Error,
}

/// What the link back to the network currently costs.
#[napi(string_enum)]
pub enum LinkCost {
    /// Bytes are effectively free, such as on wired power and ethernet.
    Free,
    /// Bytes are paid for, such as on a cellular plan.
    Metered,
    /// Bytes are scarce, such as on a satellite or long-range radio link.
    Expensive,
    /// Nothing can be shipped at all.
    Offline,
}

/// A count of everything a reporter has seen, cheap enough to ship anywhere.
#[napi(object)]
pub struct Snapshot {
    /// How many events were seen at trace level.
    pub trace: u32,
    /// How many events were seen at debug level.
    pub debug: u32,
    /// How many events were seen at info level.
    pub info: u32,
    /// How many events were seen at warn level.
    pub warn: u32,
    /// How many events were seen at error level.
    pub error: u32,
    /// How many events passed the filter and were shipped.
    pub emitted: u32,
    /// How many events the filter dropped.
    pub dropped: u32,
}

/// Returns the level a link cost calls for.
#[napi]
pub fn link_cost_threshold(cost: LinkCost) -> Level {
    level(core_cost(cost).threshold())
}

/// Records telemetry events, ships the ones worth their bytes, and counts them
/// all.
#[napi]
pub struct Reporter {
    inner: CoreReporter,
}

#[napi]
impl Reporter {
    /// Creates a reporter that ships events at or above `threshold`.
    #[napi(constructor)]
    pub fn new(threshold: Level) -> Self {
        Self {
            inner: CoreReporter::new(core_level(threshold)),
        }
    }

    /// The level this reporter is currently shipping from.
    #[napi(getter)]
    pub fn threshold(&self) -> Level {
        level(self.inner.threshold())
    }

    /// Moves the level this reporter ships from.
    #[napi(setter)]
    pub fn set_threshold(&mut self, threshold: Level) {
        self.inner.set_threshold(core_level(threshold));
    }

    /// Moves the threshold to match what the link now costs.
    #[napi]
    pub fn adapt_to(&mut self, cost: LinkCost) {
        self.inner.adapt_to(core_cost(cost));
    }

    /// Records an event at `level` and returns whether it should be shipped.
    ///
    /// The event is counted either way, so the aggregate picture survives even
    /// when the link is too costly to carry the detail.
    #[napi]
    pub fn record(&mut self, level: Level) -> bool {
        // The core event carries a borrowed static code the reporter never reads,
        // so nothing is lost by leaving it empty here.
        self.inner
            .record(Event::new(core_level(level), ""))
            .is_some()
    }

    /// Returns how many events have been seen at `level`, shipped or not.
    #[napi]
    pub fn count(&self, level: Level) -> u32 {
        self.inner.count(core_level(level))
    }

    /// How many events have been seen across every level.
    #[napi(getter)]
    pub fn total(&self) -> u32 {
        self.inner.total()
    }

    /// How many events passed the threshold and were shipped.
    #[napi(getter)]
    pub fn emitted(&self) -> u32 {
        self.inner.emitted()
    }

    /// How many events the threshold dropped.
    #[napi(getter)]
    pub fn dropped(&self) -> u32 {
        self.inner.dropped()
    }

    /// Takes a snapshot of the counters to ship in place of the event stream.
    #[napi]
    pub fn snapshot(&self) -> Snapshot {
        let snapshot = self.inner.snapshot();
        Snapshot {
            trace: snapshot.by_level[CoreLevel::Trace as usize],
            debug: snapshot.by_level[CoreLevel::Debug as usize],
            info: snapshot.by_level[CoreLevel::Info as usize],
            warn: snapshot.by_level[CoreLevel::Warn as usize],
            error: snapshot.by_level[CoreLevel::Error as usize],
            emitted: snapshot.emitted,
            dropped: snapshot.dropped,
        }
    }
}

/// Maps a core level onto the value that crosses to JavaScript.
fn level(level: CoreLevel) -> Level {
    match level {
        CoreLevel::Trace => Level::Trace,
        CoreLevel::Debug => Level::Debug,
        CoreLevel::Info => Level::Info,
        CoreLevel::Warn => Level::Warn,
        CoreLevel::Error => Level::Error,
    }
}

/// Maps a JavaScript level back onto the core one.
fn core_level(level: Level) -> CoreLevel {
    match level {
        Level::Trace => CoreLevel::Trace,
        Level::Debug => CoreLevel::Debug,
        Level::Info => CoreLevel::Info,
        Level::Warn => CoreLevel::Warn,
        Level::Error => CoreLevel::Error,
    }
}

/// Maps a JavaScript link cost back onto the core one.
fn core_cost(cost: LinkCost) -> CoreCost {
    match cost {
        LinkCost::Free => CoreCost::Free,
        LinkCost::Metered => CoreCost::Metered,
        LinkCost::Expensive => CoreCost::Expensive,
        LinkCost::Offline => CoreCost::Offline,
    }
}
