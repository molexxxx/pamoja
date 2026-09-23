# pamoja-bus

An in-memory typed publish and subscribe event bus, with publishers that never wait and subscribers that count what they miss. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/bus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html)

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

from pamoja.bus import EventPublisher

QUIET_MS = 50


async def main() -> None:
    # The station's wiring makes one bus and hands each part what it needs: a publisher
    # to announce, an endpoint to listen. No part holds a reference to another, so any
    # of them can be replaced without touching the rest.
    bus = EventPublisher(2)
    power = bus.publisher()
    sampler = bus.publisher()
    heater = bus.subscribe()
    logger = bus.subscribe()

    # One announcement reaches every part that listens, and each reads its own copy.
    reached = power.publish("battery.low")
    print(f"power     handed battery.low to {reached} parts")
    heater_took = await heater.next_text()
    print(f"heater    took {heater_took}")
    logger_took = await logger.next_text()
    print(f"logger    took {logger_took}")

    # Publishing never waits, even while the part's own wait is open, and a part hears
    # what it publishes.
    waiting = heater.next_text()
    heater.publish("heater.off")
    heard = await waiting
    print(f"heater    heard its own {heard}, sent while it waited")

    # A part that joins late sees only what is published after it subscribes. There is
    # no history to replay.
    radio = bus.subscribe()
    power.publish("battery.ok")
    first = await radio.next_text()
    print(f"radio     joined late, so the first event it sees is {first}")

    # Each endpoint buffers two events. The logger, busy writing to flash, falls behind
    # while the sampler publishes five readings: it loses the oldest events, resumes
    # with the newest, and counts what it lost.
    for reading in range(5):
        sampler.publish(f"wind {reading}")
    resumed = await logger.next_text()
    missed = logger.missed
    print(f"logger    missed {missed} and resumes at {resumed}")
    newest = await logger.next_text()
    print(f"logger    then took {newest}")

    # A wait with a limit gives up without taking anything, so a part can do other work
    # between events and lose nothing by it.
    try:
        await asyncio.wait_for(logger.next_text(), QUIET_MS / 1000)
        print("logger    took an event no one published, which should never happen")
    except asyncio.TimeoutError:
        print(f"logger    heard nothing more within {QUIET_MS} ms")

    return reached, heater_took, logger_took, heard, first, missed, resumed, newest


seen = asyncio.run(main())
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-bus`](https://crates.io/crates/pamoja-bus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html), [docs.rs](https://docs.rs/pamoja-bus), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-bus) |
| TypeScript | [`@pamoja/bus`](https://www.npmjs.com/package/@pamoja/bus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-bus) |
| Python | [`pamoja-bus`](https://pypi.org/project/pamoja-bus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-bus) |
| C# | [`Pamoja.Bus`](https://www.nuget.org/packages/Pamoja.Bus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-bus) |

## Documentation

- [`pamoja.bus` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html), every class and function in this module.
- [The Event bus guide](https://pamoja.molex.cloud/docs/guides/bus.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
