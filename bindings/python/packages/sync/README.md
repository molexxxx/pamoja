# pamoja-sync

Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sync.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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

from pamoja.core import PamojaError
from pamoja.sync import Store


async def main() -> None:
    # A node with nowhere to send buffers its readings. This queue is held in memory, so it
    # lasts as long as the process; Store.file(dir) is the same queue on disk, which is what
    # a node uses to survive a reboot with its backlog intact.
    outbox = Store.memory()
    for reading in (b"20.1", b"20.4", b"20.2"):
        await outbox.append(reading)
    print(f"queued    {await outbox.len()} readings with no link")

    # Peek reads the oldest record without taking it, so a send that fails part-way leaves
    # the queue exactly as it was.
    oldest = await outbox.peek()
    print(f"oldest    {oldest.decode()} and still {await outbox.len()} held")

    # The link returns and the queue drains oldest first, in the order the readings were
    # taken rather than the order they happen to come back off a buffer.
    drained = []
    while (record := await outbox.pop()) is not None:
        drained.append(record.decode())
    print(f"drained   {', '.join(drained)}")

    # A bounded queue refuses the append that would overflow it. A full store is
    # backpressure the caller is told about, not a reading dropped behind its back.
    bounded = Store.memory(capacity=2)
    await bounded.append(b"20.1")
    await bounded.append(b"20.4")
    try:
        await bounded.append(b"20.2")
        print("a full queue took a third reading, which should never happen")
    except PamojaError as error:
        print(f"full      refused the third reading: {error}")

    return oldest, drained, await outbox.len(), await bounded.len()


oldest, drained, left, held = asyncio.run(main())
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sync`](https://crates.io/crates/pamoja-sync) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html), [docs.rs](https://docs.rs/pamoja-sync) |
| TypeScript | [`@pamoja/sync`](https://www.npmjs.com/package/@pamoja/sync) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html) |
| Python | [`pamoja-sync`](https://pypi.org/project/pamoja-sync/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html) |
| C# | [`Pamoja.Sync`](https://www.nuget.org/packages/Pamoja.Sync) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html) |

## Documentation

- [`pamoja.sync` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html), every class and function in this module.
- [The Store and forward guide](https://pamoja.molex.cloud/docs/guides/sync.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
