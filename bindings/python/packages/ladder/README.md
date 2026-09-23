# pamoja-ladder

Cheapest reachable link first, buffering to a store when every link is down. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/ladder.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/ladder.html)

## Install

```sh
pip install pamoja-ladder
```

```python
from pamoja import ladder
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/ladder.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/ladder.py):

```python
import asyncio

from pamoja.ladder import Delivery, Ladder
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store

REPORT = "vessel/7/report"
ORDERS = "vessel/7/orders"
QUIET_MS = 50


async def main() -> None:
    # Three networks a vessel can reach: the harbor's wifi, the coast's cellular
    # network, and a satellite. Each is a broker with an office ashore listening on it,
    # so which one carried a report is read off that office rather than assumed.
    harbor = LoopbackBroker()
    coast = LoopbackBroker()
    sky = LoopbackBroker()
    harbor_office = harbor.link()
    coast_office = coast.link()
    sky_office = sky.link()
    for office in (harbor_office, coast_office, sky_office):
        await office.connect()
        await office.subscribe(REPORT)

    # Rungs go on cheapest first. The satellite only sends, so it goes on as an uplink,
    # which the ladder never subscribes or listens on.
    ladder = Ladder(Store.memory())
    await ladder.rung(harbor.rung())
    await ladder.rung(coast.rung())
    await ladder.uplink(sky.rung())
    await ladder.connect()
    await ladder.subscribe(ORDERS)

    # In the harbor, the cheapest link takes the report.
    first = await ladder.send(REPORT, "report 1")
    print(f"harbor    carried {(await harbor_office.recv()).text}")

    # Past the breakwater the wifi is out of reach and the report falls through to the
    # coast, and further out to the satellite.
    harbor.reachable = False
    await ladder.send(REPORT, "report 2")
    print(f"coast     carried {(await coast_office.recv()).text}, with the harbor out of reach")
    coast.reachable = False
    await ladder.send(REPORT, "report 3")
    print(f"sky       carried {(await sky_office.recv()).text}, with the coast out of reach too")

    # In a storm nothing is in reach, and the report waits in the store rather than
    # being lost.
    sky.reachable = False
    stormy = await ladder.send(REPORT, "report 4")
    waiting = await ladder.buffered()
    print(f"vessel    buffered report 4 with every link out of reach, {waiting} waiting")

    # A flush with every link still out of reach forwards nothing, because a record
    # leaves the store only once a link has taken it.
    idle = await ladder.flush()
    still = await ladder.buffered()
    print(f"vessel    flushed {idle} while every link was out of reach, {still} still waiting")

    # Back in reach of the coast, a flush sends the backlog, oldest first.
    coast.reachable = True
    forwarded = await ladder.flush()
    late = await coast_office.recv()
    left = await ladder.buffered()
    print(f"coast     carried {late.text} on a flush of {forwarded}, {left} waiting")

    # Orders from shore come back over whichever listening link is in reach.
    await coast_office.send(ORDERS, "return to port")
    order = await ladder.recv()
    print(f"vessel    took {order.text} over the coast network")

    # A ladder does one thing at a time, so a vessel that listens and reports waits
    # for orders with a limit and reports between waits.
    try:
        await asyncio.wait_for(ladder.recv(), QUIET_MS / 1000)
    except asyncio.TimeoutError:
        print(f"vessel    heard nothing more from shore within {QUIET_MS} ms")
    await ladder.send(REPORT, "report 5")
    last = await coast_office.recv()
    print(f"coast     carried {last.text} between waits")

    return first, stormy, (waiting, still, left), order.text, last.text


first, stormy, counts, order, last = asyncio.run(main())
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-ladder`](https://crates.io/crates/pamoja-ladder) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html), [docs.rs](https://docs.rs/pamoja-ladder), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-ladder) |
| TypeScript | [`@pamoja/ladder`](https://www.npmjs.com/package/@pamoja/ladder) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-ladder) |
| Python | [`pamoja-ladder`](https://pypi.org/project/pamoja-ladder/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/ladder.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-ladder) |
| C# | [`Pamoja.Ladder`](https://www.nuget.org/packages/Pamoja.Ladder) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-ladder) |

## Documentation

- [`pamoja.ladder` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/ladder.html), every class and function in this module.
- [The Transport ladder guide](https://pamoja.molex.cloud/docs/guides/ladder.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
