# pamoja-telemetry

Observability that ships only what is worth the bytes as link cost rises, while counting everything. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/telemetry.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html)

## Install

```sh
pip install pamoja-telemetry
```

```python
from pamoja import telemetry
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/telemetry.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/telemetry.py):

```python
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-telemetry`](https://crates.io/crates/pamoja-telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html), [docs.rs](https://docs.rs/pamoja-telemetry), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-telemetry) |
| TypeScript | [`@pamoja/telemetry`](https://www.npmjs.com/package/@pamoja/telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-telemetry) |
| Python | [`pamoja-telemetry`](https://pypi.org/project/pamoja-telemetry/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-telemetry) |
| C# | [`Pamoja.Telemetry`](https://www.nuget.org/packages/Pamoja.Telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-telemetry) |

## Documentation

- [`pamoja.telemetry` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html), every class and function in this module.
- [The Telemetry guide](https://pamoja.molex.cloud/docs/guides/telemetry.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
