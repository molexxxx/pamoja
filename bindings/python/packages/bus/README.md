# pamoja-bus

An in-memory typed publish and subscribe event bus. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/bus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-bus
```

```python
from pamoja import bus
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/bus.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/bus.py):

```python
import asyncio

from pamoja.bus import EventBus


async def main() -> None:
    # A sampler announces a reading and whatever cares about readings picks it up,
    # with neither side holding a reference to the other.
    hub = EventBus(8)
    sampler = await hub.subscribe()
    logger = await hub.subscribe()

    await hub.publish(b"battery.low")
    assert await sampler.next_event() == b"battery.low"
    assert await logger.next_event() == b"battery.low"

    # An endpoint taken later starts from the next event, so it never sees what went
    # out before it existed.
    late = await hub.subscribe()
    await hub.publish(b"link.up")
    assert await late.next_event() == b"link.up"
    assert await sampler.next_event() == b"link.up"

    # The buffer is per endpoint and bounded, so an endpoint further behind than the
    # capacity drops what it missed and resumes with the most recent events.
    slow = EventBus(2)
    reader = await slow.subscribe()
    for count in range(5):
        await slow.publish(bytes([count]))
    assert await reader.next_event() == b"\x03"
    assert await reader.next_event() == b"\x04"


asyncio.run(main())
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-bus`](https://crates.io/crates/pamoja-bus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html), [docs.rs](https://docs.rs/pamoja-bus) |
| TypeScript | [`@pamoja/bus`](https://www.npmjs.com/package/@pamoja/bus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html) |
| Python | [`pamoja-bus`](https://pypi.org/project/pamoja-bus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html) |
| C# | [`Pamoja.Bus`](https://www.nuget.org/packages/Pamoja.Bus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html) |

## Documentation

- [`pamoja.bus` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html), every class and function in this module.
- [The Event bus guide](https://pamoja.molex.cloud/docs/guides/bus.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
