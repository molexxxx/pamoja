# pamoja-mesh

Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/mesh.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mesh.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-mesh
```

```python
from pamoja import mesh
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/mesh.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mesh.py):

```python
from pamoja.core import PamojaError
from pamoja.mesh import BROADCAST, SeenPackets, broadcast, parse, relayed

# A river gauge floods a level reading to every node in range. The header is fixed and
# big-endian: version, source, destination, sequence id, hop limit, then the payload and a
# checksum over everything but the hop limit.
RIVER_GAUGE = 305419896
reading = broadcast(RIVER_GAUGE, 1, b"level=high")
print(f"sent      {len(reading.bytes)} bytes to every node in range")
print(f"addressed to broadcast: {reading.dst == BROADCAST}")

# A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
# several times over; the source and sequence id decide which copy is the first.
received = parse(reading.bytes)
print(f"payload   {received.payload.decode()}")

seen = SeenPackets(64)
first = seen.record(received.src, received.id)
again = seen.record(received.src, received.id)
print(f"first copy relayed: {first}, second copy relayed: {again}")

# Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
# frame without recomputing it and the check stays end to end.
forwarded = relayed(received.bytes)
print(f"relayed   hop limit {forwarded.hop_limit}")
onward = parse(forwarded.bytes)
print(f"onward    {onward.payload.decode()}")

# A frame that has run out of hops is not relayed again, which is what ends the flood.
spent = relayed(broadcast(RIVER_GAUGE, 1, b"level=high", 0).bytes)
if spent is None:
    print("spent     hop limit reached, the flood stops here")
else:
    print("a spent frame was relayed, which should never happen")

# A payload byte the air mangled fails the checksum rather than reaching the application
# as a plausible reading.
mangled = bytearray(reading.bytes)
mangled[12] ^= 0xFF
try:
    parse(bytes(mangled))
    print("a mangled frame was accepted, which should never happen")
except PamojaError as error:
    print(f"mangled   rejected: {error}")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mesh`](https://crates.io/crates/pamoja-mesh) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mesh/index.html), [docs.rs](https://docs.rs/pamoja-mesh) |
| TypeScript | [`@pamoja/mesh`](https://www.npmjs.com/package/@pamoja/mesh) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html) |
| Python | [`pamoja-mesh`](https://pypi.org/project/pamoja-mesh/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mesh.html) |
| C# | [`Pamoja.Mesh`](https://www.nuget.org/packages/Pamoja.Mesh) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.html) |

## Documentation

- [`pamoja.mesh` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mesh.html), every class and function in this module.
- [The Mesh frames guide](https://pamoja.molex.cloud/docs/guides/mesh.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
