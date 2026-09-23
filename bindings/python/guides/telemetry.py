"""The telemetry guide example; see docs/guides/telemetry.md."""

# ANCHOR: example
from pamoja.telemetry import Event, Level, LinkCost, Reporter


def fate(event: Event | None, kept: str) -> str:
    """What a node does with an event the reporter hands back: on a link it sends it, and
    with no link it keeps it for when one returns."""
    return "counted only" if event is None else kept


# On the site's own network nothing is held back.
reporter = Reporter(Level.TRACE)
reporter.adapt_to(LinkCost.FREE)
tick = reporter.record(Event(Level.DEBUG, "loop.tick"))
print(f"free      nothing is held back: loop.tick {fate(tick, 'sent')}")

# On a metered link the bar rises to Info. Routine detail stops going out; a reading and a
# warning still do, and a warning carries the measurement that raised it.
reporter.adapt_to(LinkCost.METERED)
tick = reporter.record(Event(Level.DEBUG, "loop.tick"))
reading = reporter.record(Event(Level.INFO, "reading.ok", 4.8))
print(
    f"metered   nothing below {reporter.threshold.value} is sent: "
    f"loop.tick {fate(tick, 'sent')}, reading.ok {fate(reading, 'sent')}"
)
warned = reporter.record(Event(Level.WARN, "battery.low", 0.18))
print(f"metered   {warned.code} sent, carrying {warned.value:.2f}")

# On satellite the bar is Warn: the same reading is no longer worth its bytes, and a
# failure still is.
reporter.adapt_to(LinkCost.EXPENSIVE)
reading = reporter.record(Event(Level.INFO, "reading.ok", 4.9))
lost = reporter.record(Event(Level.ERROR, "link.lost"))
print(
    f"satellite nothing below {reporter.threshold.value} is sent: "
    f"reading.ok {fate(reading, 'sent')}, link.lost {fate(lost, 'sent')}"
)

# With no link at all only errors are kept, for the link's return.
reporter.adapt_to(LinkCost.OFFLINE)
low = reporter.record(Event(Level.WARN, "battery.low", 0.17))
lost = reporter.record(Event(Level.ERROR, "link.lost"))
print(
    f"offline   nothing below {reporter.threshold.value} is kept: "
    f"battery.low {fate(low, 'kept')}, link.lost {fate(lost, 'kept')}"
)

# Only the stream was thinned, not the counts, so every event is still accounted for, and
# the snapshot is what the node ships in place of them.
snapshot = reporter.snapshot()
print(
    f"counts    of {reporter.total} events, {snapshot.emitted} passed the bar and "
    f"{snapshot.dropped} were counted only"
)
print(
    f"levels    trace {snapshot.trace}, debug {snapshot.debug}, info {snapshot.info}, "
    f"warn {snapshot.warn}, error {snapshot.error}"
)
# ANCHOR_END: example

assert reporter.threshold == Level.ERROR
assert warned.code == "battery.low"
assert snapshot.emitted == 5
assert snapshot.dropped == 3
assert reporter.total == 8
