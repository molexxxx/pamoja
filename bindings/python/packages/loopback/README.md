# pamoja-loopback

An in-process transport with topic matching and a fault injector, for testing with no broker. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/loopback.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/loopback.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-loopback
```

```python
from pamoja import loopback
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/loopback.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/loopback.py):

```python
import asyncio

from pamoja.core import PamojaError
from pamoja.loopback import LoopbackBroker


async def main() -> None:
    # One broker and two links off it, all in this process. Nothing binds a port and
    # nothing has to be running for the traffic below to flow, which is what makes this
    # the link to develop a node against before it has a real one.
    broker = LoopbackBroker()
    publisher = broker.link()
    subscriber = broker.link()
    await publisher.connect()
    await subscriber.connect()

    # A `+` stands for exactly one level, so this takes the mixer's temperature but not the
    # raw reading a level below it.
    await subscriber.subscribe("line/+/temp")
    await publisher.send("line/mixer/temp/raw", b"2150")
    await publisher.send("line/mixer/temp", b"21.5")

    message = await subscriber.recv()
    print(f"line/+/temp took {message.payload.decode()} from {message.topic}")

    # A `#` covers every level that remains, so a second link takes the whole subtree,
    # including the reading the single-level filter passed over.
    watcher = broker.link()
    await watcher.connect()
    await watcher.subscribe("line/#")
    await publisher.send("line/mixer/temp/raw", b"2150")

    deep = await watcher.recv()
    print(f"line/#     took {deep.payload.decode()} from {deep.topic}")

    # A link that has been disconnected reports the failure instead of dropping the
    # reading, which is the case a test wants to reach without unplugging anything.
    await publisher.disconnect()
    try:
        await publisher.send("line/mixer/temp", b"21.6")
        print("a disconnected link took a reading, which should never happen")
    except PamojaError as error:
        print(f"disconnected refused the reading: {error}")

    return message, deep


message, deep = asyncio.run(main())
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-loopback`](https://crates.io/crates/pamoja-loopback) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html), [docs.rs](https://docs.rs/pamoja-loopback) |
| TypeScript | [`@pamoja/loopback`](https://www.npmjs.com/package/@pamoja/loopback) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html) |
| Python | [`pamoja-loopback`](https://pypi.org/project/pamoja-loopback/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/loopback.html) |
| C# | [`Pamoja.Loopback`](https://www.nuget.org/packages/Pamoja.Loopback) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html) |

## Documentation

- [`pamoja.loopback` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/loopback.html), every class and function in this module.
- [The Loopback guide](https://pamoja.molex.cloud/docs/guides/loopback.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
