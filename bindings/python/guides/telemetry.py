"""The telemetry guide example; see docs/guides/telemetry.md."""

# ANCHOR: example
from pamoja.telemetry import Event, Level, LinkCost, Reporter, link_cost_threshold

# The node is willing to record everything, then finds out it is reporting over a
# metered link, which puts the bar at INFO.
reporter = Reporter(Level.TRACE)
reporter.adapt_to(LinkCost.METERED)
assert reporter.threshold == Level.INFO

# Routine detail stops going out. A reading and the warning that follows it still do,
# and a shipped event comes back with the measurement that triggered it.
assert reporter.record(Event(Level.DEBUG, "loop.tick")) is None
assert reporter.record(Event(Level.INFO, "reading.ok", 4.8)) is not None
warned = reporter.record(Event(Level.WARN, "battery.low", 0.18))
assert warned is not None
assert warned.code == "battery.low"
assert warned.value == 0.18

# The node falls back to satellite, which raises the bar to WARN. The same reading is
# no longer worth its bytes; a failure still is.
reporter.adapt_to(LinkCost.EXPENSIVE)
assert reporter.record(Event(Level.INFO, "reading.ok", 4.9)) is None
assert reporter.record(Event(Level.ERROR, "link.lost")) is not None

# Only the stream was thinned, not the counts, so all five events are still accounted
# for and the snapshot is what the node ships in place of them.
counts = reporter.snapshot()
assert counts.info == 2
assert counts.emitted == 3
assert counts.dropped == 2
assert reporter.total == 5

# Offline is the last rung: a node with no link at all still keeps its failures.
assert link_cost_threshold(LinkCost.OFFLINE) == Level.ERROR
# ANCHOR_END: example
