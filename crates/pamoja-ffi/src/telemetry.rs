//! The C ABI for device-side telemetry.
//!
//! These functions wrap [`pamoja_telemetry`] for callers that reach the SDK
//! through the flat C boundary: a reporter that ships the events worth their
//! bytes, counts every event it sees whether it ships or not, and moves its own
//! bar as the link gets more expensive.
//!
//! A reporter holds counters across calls, so it crosses as an opaque handle.
//! Only the level of an event crosses with it, because the level is the whole of
//! what the reporter decides on: the code and the optional value belong to the
//! caller, which keeps them alongside.

use pamoja_telemetry::{Event, Level, LinkCost, Reporter};

/// The number of severity levels, which is the width of a snapshot.
pub const PAMOJA_TELEMETRY_LEVEL_COUNT: usize = 5;

/// How urgent an event is.
///
/// A reporter ships an event whose level is at or above its threshold and drops
/// anything below it, so the order of these values is what the filter compares.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PamojaTelemetryLevel {
    /// Fine-grained detail, useful only when chasing a specific problem.
    Trace = 0,
    /// Diagnostic detail for development.
    Debug = 1,
    /// A normal, noteworthy event.
    Info = 2,
    /// Something unexpected that the node recovered from.
    Warn = 3,
    /// A failure that needs attention.
    Error = 4,
}

/// What the link back to the network currently costs.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaLinkCost {
    /// Bytes are effectively free, such as on wired power and ethernet.
    Free = 0,
    /// Bytes are paid for, such as on a cellular plan.
    Metered = 1,
    /// Bytes are scarce, such as on a satellite or long-range radio link.
    Expensive = 2,
    /// Nothing can be shipped at all.
    Offline = 3,
}

/// A count of everything a reporter has seen, cheap enough to ship anywhere.
///
/// This is what a node sends in place of the event stream when the link cannot
/// carry the detail: the shape of what happened survives even though the
/// individual events did not.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaTelemetrySnapshot {
    /// How many events were seen at each level, indexed by
    /// [`PamojaTelemetryLevel`].
    pub by_level: [u32; PAMOJA_TELEMETRY_LEVEL_COUNT],
    /// How many events passed the filter and were shipped.
    pub emitted: u32,
    /// How many events the filter dropped.
    pub dropped: u32,
}

/// An opaque handle to one reporter and its counters.
///
/// Create it with [`pamoja_reporter_new`], feed it with
/// [`pamoja_reporter_record`], and release it with [`pamoja_reporter_free`].
pub struct PamojaReporter {
    reporter: Reporter,
}

/// Returns the level a link cost calls for.
///
/// # Arguments
///
/// * `cost` - what the link currently costs.
///
/// # Returns
///
/// The lowest level still worth its bytes at that cost.
#[no_mangle]
pub extern "C" fn pamoja_link_cost_threshold(cost: PamojaLinkCost) -> PamojaTelemetryLevel {
    level(rust_cost(cost).threshold())
}

/// Creates a reporter that ships events at or above a level.
///
/// # Arguments
///
/// * `threshold` - the lowest level to ship.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_reporter_free`].
#[no_mangle]
pub extern "C" fn pamoja_reporter_new(threshold: PamojaTelemetryLevel) -> *mut PamojaReporter {
    Box::into_raw(Box::new(PamojaReporter {
        reporter: Reporter::new(rust_level(threshold)),
    }))
}

/// Returns the level a reporter is currently shipping from.
///
/// # Arguments
///
/// * `reporter` - the reporter.
///
/// # Returns
///
/// The threshold, or [`PamojaTelemetryLevel::Trace`] if `reporter` is null.
///
/// # Safety
///
/// `reporter` must be a live handle from [`pamoja_reporter_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_reporter_threshold(
    reporter: *const PamojaReporter,
) -> PamojaTelemetryLevel {
    if reporter.is_null() {
        return PamojaTelemetryLevel::Trace;
    }
    level((*reporter).reporter.threshold())
}

/// Moves the level a reporter ships from.
///
/// # Arguments
///
/// * `reporter` - the reporter.
/// * `threshold` - the new lowest level to ship.
///
/// # Safety
///
/// `reporter` must be a live handle from [`pamoja_reporter_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_reporter_set_threshold(
    reporter: *mut PamojaReporter,
    threshold: PamojaTelemetryLevel,
) {
    if reporter.is_null() {
        return;
    }
    (*reporter).reporter.set_threshold(rust_level(threshold));
}

/// Moves the threshold to match what the link now costs.
///
/// # Arguments
///
/// * `reporter` - the reporter.
/// * `cost` - what the link currently costs.
///
/// # Safety
///
/// `reporter` must be a live handle from [`pamoja_reporter_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_reporter_adapt_to(
    reporter: *mut PamojaReporter,
    cost: PamojaLinkCost,
) {
    if reporter.is_null() {
        return;
    }
    (*reporter).reporter.adapt_to(rust_cost(cost));
}

/// Records an event and reports whether it is worth shipping.
///
/// Only the level crosses the boundary, because the level is the whole of what
/// the reporter decides on. The code and the optional value stay with the caller,
/// which is free to ship its own event when this returns `true`.
///
/// # Arguments
///
/// * `reporter` - the reporter.
/// * `level` - the severity of the event that occurred.
///
/// # Returns
///
/// `true` if the event passed the threshold and should be shipped, or `false` if
/// it was counted and dropped, or `reporter` is null.
///
/// # Safety
///
/// `reporter` must be a live handle from [`pamoja_reporter_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_reporter_record(
    reporter: *mut PamojaReporter,
    level: PamojaTelemetryLevel,
) -> bool {
    if reporter.is_null() {
        return false;
    }
    // The code is a borrowed static string on the Rust side and the reporter never
    // reads it, so nothing is lost by leaving it empty here.
    (*reporter)
        .reporter
        .record(Event::new(rust_level(level), ""))
        .is_some()
}

/// Returns how many events a reporter has seen at a level, shipped or not.
///
/// # Arguments
///
/// * `reporter` - the reporter.
/// * `level` - the level to count.
///
/// # Returns
///
/// The count, or 0 if `reporter` is null.
///
/// # Safety
///
/// `reporter` must be a live handle from [`pamoja_reporter_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_reporter_count(
    reporter: *const PamojaReporter,
    level: PamojaTelemetryLevel,
) -> u32 {
    if reporter.is_null() {
        return 0;
    }
    (*reporter).reporter.count(rust_level(level))
}

/// Returns how many events a reporter has seen across every level.
///
/// # Arguments
///
/// * `reporter` - the reporter.
///
/// # Returns
///
/// The total, or 0 if `reporter` is null.
///
/// # Safety
///
/// `reporter` must be a live handle from [`pamoja_reporter_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_reporter_total(reporter: *const PamojaReporter) -> u32 {
    if reporter.is_null() {
        return 0;
    }
    (*reporter).reporter.total()
}

/// Returns how many events passed the threshold and were shipped.
///
/// # Arguments
///
/// * `reporter` - the reporter.
///
/// # Returns
///
/// The emitted count, or 0 if `reporter` is null.
///
/// # Safety
///
/// `reporter` must be a live handle from [`pamoja_reporter_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_reporter_emitted(reporter: *const PamojaReporter) -> u32 {
    if reporter.is_null() {
        return 0;
    }
    (*reporter).reporter.emitted()
}

/// Returns how many events the threshold dropped.
///
/// # Arguments
///
/// * `reporter` - the reporter.
///
/// # Returns
///
/// The dropped count, or 0 if `reporter` is null.
///
/// # Safety
///
/// `reporter` must be a live handle from [`pamoja_reporter_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_reporter_dropped(reporter: *const PamojaReporter) -> u32 {
    if reporter.is_null() {
        return 0;
    }
    (*reporter).reporter.dropped()
}

/// Takes a snapshot of the counters to ship in place of the event stream.
///
/// # Arguments
///
/// * `reporter` - the reporter.
///
/// # Returns
///
/// The snapshot, or an all-zero snapshot if `reporter` is null.
///
/// # Safety
///
/// `reporter` must be a live handle from [`pamoja_reporter_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_reporter_snapshot(
    reporter: *const PamojaReporter,
) -> PamojaTelemetrySnapshot {
    if reporter.is_null() {
        return PamojaTelemetrySnapshot {
            by_level: [0; PAMOJA_TELEMETRY_LEVEL_COUNT],
            emitted: 0,
            dropped: 0,
        };
    }
    let snapshot = (*reporter).reporter.snapshot();
    PamojaTelemetrySnapshot {
        by_level: snapshot.by_level,
        emitted: snapshot.emitted,
        dropped: snapshot.dropped,
    }
}

/// Releases a reporter handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `reporter` must be a handle from [`pamoja_reporter_new`] that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_reporter_free(reporter: *mut PamojaReporter) {
    if !reporter.is_null() {
        drop(Box::from_raw(reporter));
    }
}

/// Maps a Rust level onto the value that crosses the boundary.
fn level(level: Level) -> PamojaTelemetryLevel {
    match level {
        Level::Trace => PamojaTelemetryLevel::Trace,
        Level::Debug => PamojaTelemetryLevel::Debug,
        Level::Info => PamojaTelemetryLevel::Info,
        Level::Warn => PamojaTelemetryLevel::Warn,
        Level::Error => PamojaTelemetryLevel::Error,
    }
}

/// Maps a boundary level back onto the Rust one.
fn rust_level(level: PamojaTelemetryLevel) -> Level {
    match level {
        PamojaTelemetryLevel::Trace => Level::Trace,
        PamojaTelemetryLevel::Debug => Level::Debug,
        PamojaTelemetryLevel::Info => Level::Info,
        PamojaTelemetryLevel::Warn => Level::Warn,
        PamojaTelemetryLevel::Error => Level::Error,
    }
}

/// Maps a boundary link cost back onto the Rust one.
fn rust_cost(cost: PamojaLinkCost) -> LinkCost {
    match cost {
        PamojaLinkCost::Free => LinkCost::Free,
        PamojaLinkCost::Metered => LinkCost::Metered,
        PamojaLinkCost::Expensive => LinkCost::Expensive,
        PamojaLinkCost::Offline => LinkCost::Offline,
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use super::*;

    #[test]
    fn a_costly_link_raises_the_bar() {
        unsafe {
            let reporter = pamoja_reporter_new(PamojaTelemetryLevel::Trace);

            pamoja_reporter_adapt_to(reporter, PamojaLinkCost::Metered);
            assert!(!pamoja_reporter_record(
                reporter,
                PamojaTelemetryLevel::Debug
            ));
            assert!(pamoja_reporter_record(reporter, PamojaTelemetryLevel::Warn));

            assert_eq!(pamoja_reporter_total(reporter), 2);
            assert_eq!(pamoja_reporter_emitted(reporter), 1);
            assert_eq!(pamoja_reporter_dropped(reporter), 1);

            pamoja_reporter_free(reporter);
        }
    }

    #[test]
    fn dropped_events_are_still_counted() {
        unsafe {
            let reporter = pamoja_reporter_new(PamojaTelemetryLevel::Error);

            for _ in 0..3 {
                pamoja_reporter_record(reporter, PamojaTelemetryLevel::Info);
            }
            pamoja_reporter_record(reporter, PamojaTelemetryLevel::Error);

            let snapshot = pamoja_reporter_snapshot(reporter);
            assert_eq!(snapshot.by_level[PamojaTelemetryLevel::Info as usize], 3);
            assert_eq!(snapshot.by_level[PamojaTelemetryLevel::Error as usize], 1);
            assert_eq!(snapshot.emitted, 1);
            assert_eq!(snapshot.dropped, 3);
            assert_eq!(
                pamoja_reporter_count(reporter, PamojaTelemetryLevel::Info),
                3
            );

            pamoja_reporter_free(reporter);
        }
    }

    #[test]
    fn each_link_cost_sets_its_own_bar() {
        assert_eq!(
            pamoja_link_cost_threshold(PamojaLinkCost::Free),
            PamojaTelemetryLevel::Trace
        );
        assert_eq!(
            pamoja_link_cost_threshold(PamojaLinkCost::Metered),
            PamojaTelemetryLevel::Info
        );
        assert_eq!(
            pamoja_link_cost_threshold(PamojaLinkCost::Expensive),
            PamojaTelemetryLevel::Warn
        );
        assert_eq!(
            pamoja_link_cost_threshold(PamojaLinkCost::Offline),
            PamojaTelemetryLevel::Error
        );
    }

    #[test]
    fn a_null_reporter_is_inert() {
        unsafe {
            assert!(!pamoja_reporter_record(
                ptr::null_mut(),
                PamojaTelemetryLevel::Error
            ));
            assert_eq!(pamoja_reporter_total(ptr::null()), 0);
            assert_eq!(pamoja_reporter_snapshot(ptr::null()).emitted, 0);
            pamoja_reporter_free(ptr::null_mut());
        }
    }
}
