//! Generated Python bindings for device-side telemetry.
//!
//! These mirror the `pamoja-telemetry` Rust API: a reporter that ships the
//! events worth their bytes, counts every event it sees whether it ships or not,
//! and moves its own bar as the link gets more expensive.
//!
//! Only the level of an event crosses, because the level is the whole of what
//! the reporter decides on. The facade keeps the code and the optional value
//! beside it and hands back the caller's own event when one should ship.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_telemetry::{Event, Level, LinkCost, Reporter as CoreReporter};

use crate::PamojaError;

/// A count of everything a reporter has seen, cheap enough to ship anywhere.
#[gen_stub_pyclass]
#[pyclass]
pub struct Snapshot {
    /// How many events were seen at trace level.
    #[pyo3(get)]
    trace: u32,
    /// How many events were seen at debug level.
    #[pyo3(get)]
    debug: u32,
    /// How many events were seen at info level.
    #[pyo3(get)]
    info: u32,
    /// How many events were seen at warn level.
    #[pyo3(get)]
    warn: u32,
    /// How many events were seen at error level.
    #[pyo3(get)]
    error: u32,
    /// How many events passed the filter and were shipped.
    #[pyo3(get)]
    emitted: u32,
    /// How many events the filter dropped.
    #[pyo3(get)]
    dropped: u32,
}

/// Returns the level a named link cost calls for.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn link_cost_threshold(cost: &str) -> PyResult<String> {
    Ok(name(core_cost(cost)?.threshold()))
}

/// Records telemetry events, ships the ones worth their bytes, and counts them
/// all.
#[gen_stub_pyclass]
#[pyclass]
pub struct Reporter {
    inner: CoreReporter,
}

#[gen_stub_pymethods]
#[pymethods]
impl Reporter {
    /// Creates a reporter that ships events at or above the named level.
    #[new]
    fn new(threshold: &str) -> PyResult<Self> {
        Ok(Reporter {
            inner: CoreReporter::new(core_level(threshold)?),
        })
    }

    /// The level this reporter is currently shipping from.
    #[getter]
    fn threshold(&self) -> String {
        name(self.inner.threshold())
    }

    /// Moves the level this reporter ships from.
    #[setter]
    fn set_threshold(&mut self, threshold: &str) -> PyResult<()> {
        self.inner.set_threshold(core_level(threshold)?);
        Ok(())
    }

    /// Moves the threshold to match what the link now costs.
    fn adapt_to(&mut self, cost: &str) -> PyResult<()> {
        self.inner.adapt_to(core_cost(cost)?);
        Ok(())
    }

    /// Records an event at a level and reports whether it should be shipped.
    ///
    /// The event is counted either way, so the aggregate picture survives even
    /// when the link is too costly to carry the detail.
    fn record(&mut self, level: &str) -> PyResult<bool> {
        // The core event carries a borrowed static code the reporter never
        // reads, so nothing is lost by leaving it empty here.
        Ok(self
            .inner
            .record(Event::new(core_level(level)?, ""))
            .is_some())
    }

    /// Returns how many events have been seen at a level, shipped or not.
    fn count(&self, level: &str) -> PyResult<u32> {
        Ok(self.inner.count(core_level(level)?))
    }

    /// How many events have been seen across every level.
    #[getter]
    fn total(&self) -> u32 {
        self.inner.total()
    }

    /// How many events passed the threshold and were shipped.
    #[getter]
    fn emitted(&self) -> u32 {
        self.inner.emitted()
    }

    /// How many events the threshold dropped.
    #[getter]
    fn dropped(&self) -> u32 {
        self.inner.dropped()
    }

    /// Takes a snapshot of the counters to ship in place of the event stream.
    fn snapshot(&self) -> Snapshot {
        let snapshot = self.inner.snapshot();
        Snapshot {
            trace: snapshot.by_level[Level::Trace as usize],
            debug: snapshot.by_level[Level::Debug as usize],
            info: snapshot.by_level[Level::Info as usize],
            warn: snapshot.by_level[Level::Warn as usize],
            error: snapshot.by_level[Level::Error as usize],
            emitted: snapshot.emitted,
            dropped: snapshot.dropped,
        }
    }
}

/// Names a level for Python.
fn name(level: Level) -> String {
    match level {
        Level::Trace => "Trace",
        Level::Debug => "Debug",
        Level::Info => "Info",
        Level::Warn => "Warn",
        Level::Error => "Error",
    }
    .to_owned()
}

/// Reads a level back from its name, refusing one that is not a level.
fn core_level(level: &str) -> PyResult<Level> {
    match level {
        "Trace" => Ok(Level::Trace),
        "Debug" => Ok(Level::Debug),
        "Info" => Ok(Level::Info),
        "Warn" => Ok(Level::Warn),
        "Error" => Ok(Level::Error),
        other => Err(PamojaError::new_err(format!("unknown level {other}"))),
    }
}

/// Reads a link cost back from its name, refusing one that is not a cost.
fn core_cost(cost: &str) -> PyResult<LinkCost> {
    match cost {
        "Free" => Ok(LinkCost::Free),
        "Metered" => Ok(LinkCost::Metered),
        "Expensive" => Ok(LinkCost::Expensive),
        "Offline" => Ok(LinkCost::Offline),
        other => Err(PamojaError::new_err(format!("unknown link cost {other}"))),
    }
}
