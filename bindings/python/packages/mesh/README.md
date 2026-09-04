# pamoja-mesh

Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
from pamoja.mesh import BROADCAST, SeenPackets, broadcast, crc16, parse, relayed

# A river gauge floods a reading to every node in range. The header is fixed and
# big-endian: version, source, destination, sequence id, hop limit, then the payload
# and a checksum over everything but the hop limit.
reading = broadcast(0x12345678, 1, b"level=high")
assert reading.dst == BROADCAST
assert reading.bytes.hex() == "0112345678ffffffff0001036c6576656c3d686967683335"

# The checksum is CRC-16/CCITT-FALSE, whose published check value fixes the polynomial
# and the starting value.
assert crc16(b"123456789") == 0x29B1

# A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
# several times over; the source and sequence id decide which copy is the first.
received = parse(reading.bytes)
assert received.payload == b"level=high"
seen = SeenPackets(64)
assert seen.record(received.src, received.id)
assert not seen.record(received.src, received.id)

# Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
# frame without recomputing it and the check stays end to end.
forwarded = relayed(received.bytes)
assert forwarded.hop_limit == received.hop_limit - 1
assert parse(forwarded.bytes).payload == received.payload
assert relayed(broadcast(0x12345678, 1, b"level=high", 0).bytes) is None

# A payload byte the air mangled fails the checksum rather than reaching the application
# as a plausible reading.
mangled = bytearray(reading.bytes)
mangled[12] ^= 0xFF
try:
    parse(bytes(mangled))
except PamojaError:
    pass
else:
    raise AssertionError("a frame mangled on the air should be rejected")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mesh`](https://crates.io/crates/pamoja-mesh) | [docs.rs](https://docs.rs/pamoja-mesh), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mesh/index.html) |
| TypeScript | [`@pamoja/mesh`](https://www.npmjs.com/package/@pamoja/mesh) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html) |
| Python | [`pamoja-mesh`](https://pypi.org/project/pamoja-mesh/) | [`pamoja.mesh`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mesh.html) |
| C# | [`Pamoja.Mesh`](https://www.nuget.org/packages/Pamoja.Mesh) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.Mesh.html) |

## Documentation

- [The Mesh frames guide](https://pamoja.molex.cloud/docs/guides/mesh.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
