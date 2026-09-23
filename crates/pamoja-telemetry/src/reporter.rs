//! Recording events, filtering them for the link, and summarizing the counts.

use crate::event::{Event, Level};

// The number of [`Level`] variants, the width of the per-level counters.
const LEVEL_COUNT: usize = 5;

/// How costly the current link is, which sets how selective telemetry should be.
///
/// The cost maps to the level [`threshold`](LinkCost::threshold) a
/// [`Reporter`] should use: a free link ships everything, while an expensive or
/// absent link ships only what is worth its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkCost {
    /// A free or local link: ship all detail.
    Free,
    /// A metered link: skip routine detail.
    Metered,
    /// An expensive link, such as satellite: ship only warnings and errors.
    Expensive,
    /// No link: hold back everything but errors worth buffering.
    Offline,
}

impl LinkCost {
    /// Returns the level threshold this link cost calls for.
    ///
    /// # Returns
    ///
    /// The minimum [`Level`] a reporter should ship at this link cost.
    pub fn threshold(self) -> Level {
        match self {
            LinkCost::Free => Level::Trace,
            LinkCost::Metered => Level::Info,
            LinkCost::Expensive => Level::Warn,
            LinkCost::Offline => Level::Error,
        }
    }
}

/// A point-in-time summary of a reporter's counters.
///
/// This is what a node ships periodically in place of the raw event stream: a few
/// integers that capture how many events occurred at each level and how many were
/// shipped versus dropped, cheap to send even on a metered link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// The number of events seen at each level, indexed by the level's order from
    /// [`Trace`](Level::Trace) to [`Error`](Level::Error).
    pub by_level: [u32; LEVEL_COUNT],
    /// How many events passed the filter and were shipped.
    pub emitted: u32,
    /// How many events were dropped by the filter.
    pub dropped: u32,
}

/// Records telemetry events, ships the ones worth their bytes, and counts them all.
///
/// A reporter keeps a level threshold and forwards only events at or above it, while
/// counting every event it sees - dropped or not - so the aggregate picture survives
/// even when the link is too costly to ship detail. Call
/// [`adapt_to`](Reporter::adapt_to) as the link cost changes to raise or lower the
/// bar, and ship a [`snapshot`](Reporter::snapshot) of the counters periodically
/// instead of the full stream.
///
/// # Examples
///
/// ```
/// use pamoja_telemetry::{Event, Level, LinkCost, Reporter};
///
/// let mut reporter = Reporter::new(Level::Trace);
///
/// // On a metered link, routine debug events are dropped but a warning still ships.
/// reporter.adapt_to(LinkCost::Metered);
/// assert!(reporter.record(Event::debug("loop.tick")).is_none());
/// assert!(reporter.record(Event::warn("battery.low")).is_some());
///
/// // Both events were still counted.
/// assert_eq!(reporter.total(), 2);
/// assert_eq!(reporter.dropped(), 1);
/// ```
pub struct Reporter {
    threshold: Level,
    counts: [u32; LEVEL_COUNT],
    emitted: u32,
}

impl Reporter {
    /// Creates a reporter that ships events at or above `threshold`.
    ///
    /// # Arguments
    ///
    /// * `threshold` - the minimum level to ship.
    ///
    /// # Returns
    ///
    /// A reporter with empty counters.
    pub fn new(threshold: Level) -> Self {
        Self {
            threshold,
            counts: [0; LEVEL_COUNT],
            emitted: 0,
        }
    }

    /// Returns the current ship threshold.
    ///
    /// # Returns
    ///
    /// The minimum level currently being shipped.
    pub fn threshold(&self) -> Level {
        self.threshold
    }

    /// Sets the ship threshold directly.
    ///
    /// # Arguments
    ///
    /// * `threshold` - the new minimum level to ship.
    pub fn set_threshold(&mut self, threshold: Level) {
        self.threshold = threshold;
    }

    /// Raises or lowers the threshold to match the current link cost.
    ///
    /// # Arguments
    ///
    /// * `cost` - how costly the link currently is.
    pub fn adapt_to(&mut self, cost: LinkCost) {
        self.threshold = cost.threshold();
    }

    /// Records an event, returning it to ship if it clears the threshold.
    ///
    /// The event is counted whether or not it is shipped, so the aggregate counts
    /// stay complete even while detail is held back. A count that reaches `u32::MAX`
    /// stays there rather than wrapping to zero, so a node that runs for years reports
    /// a full counter, never a small one.
    ///
    /// # Arguments
    ///
    /// * `event` - the event to record.
    ///
    /// # Returns
    ///
    /// `Some(event)` if it should be shipped, or `None` if it was dropped by the
    /// threshold.
    pub fn record(&mut self, event: Event) -> Option<Event> {
        let count = &mut self.counts[event.level as usize];
        *count = count.saturating_add(1);
        if event.level >= self.threshold {
            self.emitted = self.emitted.saturating_add(1);
            Some(event)
        } else {
            None
        }
    }

    /// Returns how many events have been seen at `level`, shipped or not.
    ///
    /// # Arguments
    ///
    /// * `level` - the level to count.
    ///
    /// # Returns
    ///
    /// The number of events recorded at that level.
    pub fn count(&self, level: Level) -> u32 {
        self.counts[level as usize]
    }

    /// Returns the total number of events seen across all levels.
    ///
    /// # Returns
    ///
    /// The total count, held at `u32::MAX` once it gets there.
    pub fn total(&self) -> u32 {
        self.counts
            .iter()
            .fold(0u32, |total, count| total.saturating_add(*count))
    }

    /// Returns how many events passed the threshold and were shipped.
    ///
    /// # Returns
    ///
    /// The emitted count.
    pub fn emitted(&self) -> u32 {
        self.emitted
    }

    /// Returns how many events were dropped by the threshold.
    ///
    /// # Returns
    ///
    /// The dropped count.
    pub fn dropped(&self) -> u32 {
        self.total().saturating_sub(self.emitted)
    }

    /// Returns a snapshot of the counters to ship in place of the raw stream.
    ///
    /// # Returns
    ///
    /// A [`Snapshot`] of the per-level counts and the emitted and dropped totals.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            by_level: self.counts,
            emitted: self.emitted,
            dropped: self.dropped(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_ships_at_or_above_the_threshold() {
        let mut reporter = Reporter::new(Level::Info);
        assert!(reporter.record(Event::debug("d")).is_none());
        assert!(reporter.record(Event::info("i")).is_some());
        assert!(reporter.record(Event::error("e")).is_some());
    }

    #[test]
    fn it_counts_every_event_even_when_dropped() {
        let mut reporter = Reporter::new(Level::Warn);
        reporter.record(Event::debug("d"));
        reporter.record(Event::debug("d"));
        reporter.record(Event::error("e"));
        assert_eq!(reporter.count(Level::Debug), 2);
        assert_eq!(reporter.count(Level::Error), 1);
        assert_eq!(reporter.total(), 3);
        assert_eq!(reporter.emitted(), 1);
        assert_eq!(reporter.dropped(), 2);
    }

    #[test]
    fn link_cost_sets_the_threshold() {
        let mut reporter = Reporter::new(Level::Trace);
        reporter.adapt_to(LinkCost::Metered);
        assert_eq!(reporter.threshold(), Level::Info);
        reporter.adapt_to(LinkCost::Expensive);
        assert_eq!(reporter.threshold(), Level::Warn);
        reporter.adapt_to(LinkCost::Offline);
        assert_eq!(reporter.threshold(), Level::Error);
        reporter.adapt_to(LinkCost::Free);
        assert_eq!(reporter.threshold(), Level::Trace);
    }

    #[test]
    fn a_snapshot_summarizes_the_counters() {
        let mut reporter = Reporter::new(Level::Info);
        reporter.record(Event::trace("t"));
        reporter.record(Event::info("i"));
        reporter.record(Event::warn("w"));
        let snapshot = reporter.snapshot();
        assert_eq!(snapshot.by_level[Level::Trace as usize], 1);
        assert_eq!(snapshot.by_level[Level::Info as usize], 1);
        assert_eq!(snapshot.by_level[Level::Warn as usize], 1);
        assert_eq!(snapshot.emitted, 2); // info and warn
        assert_eq!(snapshot.dropped, 1); // trace
    }

    #[test]
    fn a_full_counter_holds_rather_than_wrapping() {
        let mut reporter = Reporter::new(Level::Info);
        reporter.counts = [0, u32::MAX - 1, u32::MAX - 1, 0, 0];
        reporter.emitted = u32::MAX - 1;
        reporter.record(Event::debug("d"));
        reporter.record(Event::debug("d"));
        reporter.record(Event::info("i"));
        reporter.record(Event::info("i"));
        assert_eq!(reporter.count(Level::Debug), u32::MAX);
        assert_eq!(reporter.count(Level::Info), u32::MAX);
        assert_eq!(reporter.emitted(), u32::MAX);
        assert_eq!(reporter.total(), u32::MAX);
        assert_eq!(reporter.dropped(), 0);
    }
}
