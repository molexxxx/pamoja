"""The telemetry guide example; see docs/guides/telemetry.md."""

# ANCHOR: example
from pamoja.telemetry import Event, Level, LinkCost, Reporter, link_cost_threshold

# The node is willing to record everything, then finds out it is reporting over a metered
# link, which puts the bar at INFO.
reporter = Reporter(Level.TRACE)
reporter.adapt_to(LinkCost.METERED)
print(f"on a metered link, nothing below {reporter.threshold.value} is sent")

# Routine detail stops going out. A reading and the warning that follows it still do, and
# a shipped event comes back with the measurement that triggered it.
tick = reporter.record(Event(Level.DEBUG, "loop.tick"))
reading = reporter.record(Event(Level.INFO, "reading.ok", 4.8))
print(f"loop.tick sent: {tick is not None}")
print(f"reading.ok sent: {reading is not None}")
warned = reporter.record(Event(Level.WARN, "battery.low", 0.18))
print(f"sent      {warned.code} carrying {warned.value}")

# The node falls back to satellite, which raises the bar to WARN. The same reading is no
# longer worth its bytes; a failure still is.
reporter.adapt_to(LinkCost.EXPENSIVE)
dearer = reporter.record(Event(Level.INFO, "reading.ok", 4.9))
lost = reporter.record(Event(Level.ERROR, "link.lost"))
print(f"on satellite, reading.ok sent: {dearer is not None}")
print(f"on satellite, link.lost sent: {lost is not None}")

# Only the stream was thinned, not the counts, so every event is still accounted for and
# the snapshot is what the node ships in place of them.
counts = reporter.snapshot()
print(f"of {reporter.total} events, {counts.emitted} went out and {counts.dropped} were counted only")
# ANCHOR_END: example

assert reporter.threshold == Level.WARN
assert tick is None
assert reading is not None
assert warned.code == "battery.low"
assert warned.value == 0.18
assert dearer is None
assert lost is not None
assert counts.info == 2
assert counts.emitted == 3
assert counts.dropped == 2
assert reporter.total == 5

# Offline is the last rung: a node with no link at all still keeps its failures.
assert link_cost_threshold(LinkCost.OFFLINE) == Level.ERROR
