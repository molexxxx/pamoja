# pamoja-codec

CBOR, JSON, and raw codecs behind one trait, and batch packing for metered links: delta and varint for integers, and a quantizer for f32 readings. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/codec.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html)

## Install

```sh
pip install pamoja-codec
```

```python
from pamoja import codec
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/codec.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/codec.py):

```python
import json
import math

from pamoja.codec import Quantizer, from_cbor, pack_samples, to_cbor, unpack_samples
from pamoja.core import PamojaError
from pamoja.lora import plan_for

# The gauge reports over LoRaWAN in the US915 plan, at the slowest data rate because it
# reaches farthest. An uplink there carries only a few bytes of payload.
budget = plan_for("US915").max_payload(0).application


def fits(size: int) -> str:
    return "fits one uplink" if size <= budget else "too big for one uplink"


print(f"uplink    carries {budget} bytes at the slowest US915 data rate")

# One reading as the JSON a web service would take. CBOR carries the same document in
# fewer bytes, but every key name still rides along with every reading.
reading = {"depth_cm": 142.5, "air_c": -6.5, "battery_mv": 3712}
as_json = json.dumps(reading, separators=(",", ":")).encode()
cbor = to_cbor(reading)
print(f"json      {len(as_json)} bytes, {fits(len(as_json))}")
print(f"cbor      {len(cbor)} bytes, {fits(len(cbor))}")
print(f"cbor      reads back as {json.dumps(from_cbor(cbor), separators=(',', ':'))}")

# A batch the gauge and the server agree on needs no key names. Six hourly depths, kept to
# the millimeter, pack to a count, the first depth, and five small steps.
quantizer = Quantizer(10)
depths = [142.5, 143.8, 145.2, 146.0, 145.7, 145.5]
depth_batch = quantizer.encode(depths)
depth_bytes = len(depth_batch)
print(f"depths    {len(depths)} readings in {depth_bytes} bytes, {fits(depth_bytes)}")
depths_back = [f"{depth:.1f}" for depth in quantizer.decode(depth_batch)]
print(f"depths    read back as {', '.join(depths_back)}")

# Battery millivolts are whole numbers already, so they pack with no scale, and a falling
# voltage packs as small as a rising one.
battery = [3712, 3709, 3705, 3702, 3698, 3695]
battery_batch = pack_samples(battery)
battery_bytes = len(battery_batch)
print(f"battery   {len(battery)} readings in {battery_bytes} bytes, {fits(battery_bytes)}")
print(f"battery   reads back as {', '.join(map(str, unpack_samples(battery_batch)))}")

# Heavy snowfall can swallow the sensor's echo, leaving no depth at all. The quantizer
# refuses the batch rather than send the gap as a depth.
try:
    quantizer.encode([145.5, math.nan])
except PamojaError as error:
    print(f"depths    refused a batch with a missing depth: {error}")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-codec`](https://crates.io/crates/pamoja-codec) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html), [docs.rs](https://docs.rs/pamoja-codec), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-codec) |
| TypeScript | [`@pamoja/codec`](https://www.npmjs.com/package/@pamoja/codec) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-codec) |
| Python | [`pamoja-codec`](https://pypi.org/project/pamoja-codec/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-codec) |
| C# | [`Pamoja.Codec`](https://www.nuget.org/packages/Pamoja.Codec) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-codec) |

## Documentation

- [`pamoja.codec` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html), every class and function in this module.
- [The Codecs guide](https://pamoja.molex.cloud/docs/guides/codec.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
