# pamoja-profile

Named, ready-to-run device profiles from plain data or a JSON manifest. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/profile.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html)

## Install

```sh
pip install pamoja-profile
```

```python
from pamoja import profile
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/profile.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/profile.py):

```python
from pathlib import Path

from pamoja.loopback import LoopbackBroker
from pamoja.profile import Node, Profile

# A profile is a file. This one ships in the catalog under profiles/: it holds a brooder at
# 32 C by switching a heat lamp, says what it reads, and says how a dashboard draws it.
text = Path("profiles/brooder-heater.json").read_text(encoding="utf-8")
profile = Profile.from_json(text)
print(
    f"profile   {profile.name} reads {profile.reads.quantity} in {profile.reads.unit} "
    f"and reports on {profile.topic}"
)


async def example() -> bool:
    # A node is the profile and the parts that make it run: a sensor, an output, and a link.
    # A morning of readings stands in for the probe, and a dashboard listens on the same
    # broker.
    broker = LoopbackBroker()
    link = broker.link()
    dashboard = broker.link()
    await link.connect()
    await dashboard.connect()
    await dashboard.subscribe(profile.topic)
    morning = [27.5, 31.8, 32.6, 32.1, 31.4]
    lamp = []
    node = Node(profile, read=lambda: morning.pop(0), drive=lamp.append, link=link)

    # Each tick reads, decides, switches the lamp, and publishes the reading. The lamp comes
    # on at 31.5 C or below and goes off at 32.5 C or above, and in between it stays as it
    # was; a reading more than 4 C from 32 raises an alert as well.
    was = False
    for _ in range(5):
        tick = await node.tick()
        on = tick.reaction.actuator is True
        if on:
            change = "lamp stays on" if was else "lamp on"
        else:
            change = "lamp off" if was else "lamp stays off"
        alert = f", alert {tick.reaction.alert.kind}" if tick.reaction.alert else ""
        print(f"{f'{tick.reading:g} C':<10}{change}{alert}")
        was = on

    # The dashboard heard every reading the node published.
    heard = [f"{(await dashboard.recv()).number:g}" for _ in range(5)]
    print(f"heard     {', '.join(heard)} on {profile.topic}")

    # Between ticks the node waits as long as its battery allows: often on a healthy charge,
    # sparingly on a low one. run() does this until stopped, waiting each interval.
    for charge in [0.8, 0.3, 0.1]:
        mode, seconds = node.schedule(charge)
        print(f"battery   at {charge * 100:.0f}% it runs {mode} and waits {seconds:g} s")

    # The same file says how a dashboard draws the node.
    element = profile.presentation.elements[0]
    low, high = element.band
    print(
        f"draws     {element.key} in {element.unit} on a {element.viz}, "
        f"safe from {low:g} to {high:g}"
    )
    return lamp[-1]


lamp_on = asyncio.run(example())
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-profile`](https://crates.io/crates/pamoja-profile) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [docs.rs](https://docs.rs/pamoja-profile), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-profile) |
| TypeScript | [`@pamoja/profile`](https://www.npmjs.com/package/@pamoja/profile) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-profile) |
| Python | [`pamoja-profile`](https://pypi.org/project/pamoja-profile/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-profile) |
| C# | [`Pamoja.Profile`](https://www.nuget.org/packages/Pamoja.Profile) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-profile) |

## Documentation

- [`pamoja.profile` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), every class and function in this module.
- [The Device profiles guide](https://pamoja.molex.cloud/docs/guides/profile.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
