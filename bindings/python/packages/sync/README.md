# pamoja-sync

Offline-first queues in memory or on disk, bounded or not, the on-disk one surviving power loss, and the drain that forwards them in order when a link returns. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sync.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html)

## Install

```sh
pip install pamoja-sync
```

```python
from pamoja import sync
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/sync.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sync.py):

```python
import asyncio
import tempfile

from pamoja.core import PamojaError, Transport
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store

TOPIC = "apiary/hive-3/weight"


async def main(folder: str) -> None:
    # The scale logs its weight to a queue on its SD card, bounded so a long outage
    # cannot fill the card. The directory is the queue, so the scale can lose power at
    # any moment and lose nothing it logged.
    outbox = Store.file(folder, 3)
    for weight in ("41.2", "41.5", "40.9"):
        await outbox.append(weight)
    logged = await outbox.len()
    print(f"hive      logged {logged} weights with no link, the most its store holds")

    # A full store refuses the next weight rather than dropping one it already holds.
    try:
        await outbox.append("41.1")
    except PamojaError as error:
        print(f"hive      was refused a 4th: {error}")

    # The scale reboots. Its queue is the directory, so it comes back whole and in
    # order.
    outbox = Store.file(folder, 3)
    held = await outbox.len()
    oldest = await outbox.peek_text()
    print(f"hive      restarted and still holds {held}, oldest first: {oldest}")

    # The cellular uplink carries one weight, then drops. A weight leaves the queue only
    # once a link has taken it, so what the uplink never took stays, in order.
    cellular = LoopbackBroker()
    uplink = Transport.degraded(cellular.rung(), up=1, down=10)
    await uplink.connect()
    forwarded = 0
    try:
        await outbox.drain_to(uplink, TOPIC)
    except PamojaError as error:
        forwarded = held - await outbox.len()
        print(f"uplink    forwarded {forwarded}, then failed: {error}")
    left = await outbox.len()
    next_weight = await outbox.peek_text()
    print(f"hive      still holds {left}, oldest first: {next_weight}")

    # The beekeeper's gateway comes within reach, and the scale drains the rest onto
    # it.
    visit = LoopbackBroker()
    gateway = visit.link()
    await gateway.connect()
    await gateway.subscribe(TOPIC)
    to_gateway = visit.rung()
    await to_gateway.connect()
    await outbox.drain_to(to_gateway, TOPIC)
    took = [(await gateway.recv()).text for _ in range(left)]
    print(f"gateway   took {', '.join(took)} when the beekeeper came by")
    empty = await outbox.len()
    print(f"hive      holds {empty} once the backlog is through")

    return (logged, held, forwarded, left, empty), oldest, next_weight, took


with tempfile.TemporaryDirectory(prefix="pamoja-hive-") as folder:
    counts, oldest, next_weight, took = asyncio.run(main(folder))
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sync`](https://crates.io/crates/pamoja-sync) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html), [docs.rs](https://docs.rs/pamoja-sync), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sync) |
| TypeScript | [`@pamoja/sync`](https://www.npmjs.com/package/@pamoja/sync) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sync) |
| Python | [`pamoja-sync`](https://pypi.org/project/pamoja-sync/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sync) |
| C# | [`Pamoja.Sync`](https://www.nuget.org/packages/Pamoja.Sync) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sync) |

## Documentation

- [`pamoja.sync` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html), every class and function in this module.
- [The Store and forward guide](https://pamoja.molex.cloud/docs/guides/sync.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
