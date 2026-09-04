# pamoja-telemetry

Observability that ships only what is worth the bytes as link cost rises, while counting everything. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-telemetry`](https://crates.io/crates/pamoja-telemetry) | [docs.rs](https://docs.rs/pamoja-telemetry), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html) |
| TypeScript | [`@pamoja/telemetry`](https://www.npmjs.com/package/@pamoja/telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html) |
| Python | [`pamoja-telemetry`](https://pypi.org/project/pamoja-telemetry/) | [`pamoja.telemetry`](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html) |
| C# | [`Pamoja.Telemetry`](https://www.nuget.org/packages/Pamoja.Telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.Reporter.html) |

## Documentation

- [The Telemetry guide](https://pamoja.molex.cloud/docs/guides/telemetry.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
