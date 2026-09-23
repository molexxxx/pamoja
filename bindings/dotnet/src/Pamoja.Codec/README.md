# Pamoja.Codec

CBOR, JSON, and raw codecs behind one trait, and batch packing for metered links: delta and varint for integers, and a quantizer for f32 readings. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/codec.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html)

## Install

```sh
dotnet add package Pamoja.Codec
```

```csharp
using Pamoja.Codec;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/CodecGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CodecGuide.cs):

```csharp
// The gauge reports over LoRaWAN in the US915 plan, at the slowest data rate
// because it reaches farthest. An uplink there carries only a few bytes of payload.
using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Us915);
int budget = plan.MaxPayload(0)!.Value.Application;
string Fits(int bytes) => bytes <= budget ? "fits one uplink" : "too big for one uplink";
Console.WriteLine($"uplink    carries {budget} bytes at the slowest US915 data rate");

// One reading as the JSON a web service would take. CBOR carries the same
// document in fewer bytes, but every key name still rides along with every
// reading.
byte[] json = Encoding.UTF8.GetBytes("""{"depth_cm":142.5,"air_c":-6.5,"battery_mv":3712}""");
byte[] cbor = Codec.JsonToCbor(json);
Console.WriteLine($"json      {json.Length} bytes, {Fits(json.Length)}");
Console.WriteLine($"cbor      {cbor.Length} bytes, {Fits(cbor.Length)}");
string restored = Encoding.UTF8.GetString(Codec.CborToJson(cbor));
Console.WriteLine($"cbor      reads back as {restored}");

// A batch the gauge and the server agree on needs no key names. Six hourly
// depths, kept to the millimeter, pack to a count, the first depth, and five
// small steps.
var quantizer = new Quantizer(10.0f);
float[] depths = [142.5f, 143.8f, 145.2f, 146.0f, 145.7f, 145.5f];
byte[] depthBatch = quantizer.Encode(depths);
int depthBytes = depthBatch.Length;
Console.WriteLine($"depths    {depths.Length} readings in {depthBytes} bytes, {Fits(depthBytes)}");
IEnumerable<string> depthsBack = quantizer.Decode(depthBatch)
    .Select(depth => depth.ToString("F1", CultureInfo.InvariantCulture));
Console.WriteLine($"depths    read back as {string.Join(", ", depthsBack)}");

// Battery millivolts are whole numbers already, so they pack with no scale, and
// a falling voltage packs as small as a rising one.
long[] battery = [3712, 3709, 3705, 3702, 3698, 3695];
byte[] batteryBatch = Codec.PackSamples(battery);
int batteryBytes = batteryBatch.Length;
Console.WriteLine($"battery   {battery.Length} readings in {batteryBytes} bytes, {Fits(batteryBytes)}");
Console.WriteLine($"battery   reads back as {string.Join(", ", Codec.UnpackSamples(batteryBatch))}");

// Heavy snowfall can swallow the sensor's echo, leaving no depth at all. The
// quantizer refuses the batch rather than send the gap as a depth.
try
{
    quantizer.Encode([145.5f, float.NaN]);
}
catch (PamojaException error)
{
    Console.WriteLine($"depths    refused a batch with a missing depth: {error.Message}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-codec`](https://crates.io/crates/pamoja-codec) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html), [docs.rs](https://docs.rs/pamoja-codec), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-codec) |
| TypeScript | [`@pamoja/codec`](https://www.npmjs.com/package/@pamoja/codec) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-codec) |
| Python | [`pamoja-codec`](https://pypi.org/project/pamoja-codec/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-codec) |
| C# | [`Pamoja.Codec`](https://www.nuget.org/packages/Pamoja.Codec) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-codec) |

## Documentation

- [`Pamoja.Codec` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html), every type in this namespace.
- [The Codecs guide](https://pamoja.molex.cloud/docs/guides/codec.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
