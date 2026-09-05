# Pamoja.Mesh

Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mesh.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Mesh
```

```csharp
using Pamoja.Mesh;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs):

```csharp
// A river gauge floods a level reading to every node in range. The header is fixed
// and big-endian: version, source, destination, sequence id, hop limit, then the
// payload and a checksum over everything but the hop limit.
const uint RiverGauge = 305_419_896;
MeshFrame reading = Mesh.BroadcastFrame(RiverGauge, 1, "level=high"u8);
Console.WriteLine($"sent      {reading.Bytes.Length} bytes to every node in range");
Console.WriteLine($"addressed to broadcast: {reading.Dst == Mesh.Broadcast}");

// A neighbour hears it. Every node in range rebroadcasts, so the same packet
// arrives several times over; the source and sequence id decide which copy is
// the first.
MeshFrame received = Mesh.Parse(reading.Bytes);
Console.WriteLine($"payload   {System.Text.Encoding.UTF8.GetString(received.Payload)}");

using SeenPackets seen = new(64);
bool first = seen.Record(received.Src, received.Id);
bool again = seen.Record(received.Src, received.Id);
Console.WriteLine($"first copy relayed: {first}, second copy relayed: {again}");

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay
// forwards the frame without recomputing it and the check stays end to end.
MeshFrame forwarded = Mesh.Relayed(received.Bytes)!;
Console.WriteLine($"relayed   hop limit {forwarded.HopLimit}");
MeshFrame onward = Mesh.Parse(forwarded.Bytes);
Console.WriteLine($"onward    {System.Text.Encoding.UTF8.GetString(onward.Payload)}");

// A frame that has run out of hops is not relayed again, which ends the flood.
MeshFrame? spent = Mesh.Relayed(Mesh.BroadcastFrame(RiverGauge, 1, "level=high"u8, 0).Bytes);
Console.WriteLine(spent is null
    ? "spent     hop limit reached, the flood stops here"
    : "a spent frame was relayed, which should never happen");

// A payload byte the air mangled fails the checksum rather than reaching the
// application as a plausible reading. The header is a fixed width, so the first
// byte past it is the first byte of the reading itself.
byte[] mangled = [.. reading.Bytes];
mangled[Mesh.HeaderLen] ^= 0xFF;
try
{
    Mesh.Parse(mangled);
    Console.WriteLine("a mangled frame was accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"mangled   rejected: {error.Message}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mesh`](https://crates.io/crates/pamoja-mesh) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mesh/index.html), [docs.rs](https://docs.rs/pamoja-mesh) |
| TypeScript | [`@pamoja/mesh`](https://www.npmjs.com/package/@pamoja/mesh) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html) |
| Python | [`pamoja-mesh`](https://pypi.org/project/pamoja-mesh/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mesh.html) |
| C# | [`Pamoja.Mesh`](https://www.nuget.org/packages/Pamoja.Mesh) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.html) |

## Documentation

- [`Pamoja.Mesh` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.html), every type in this namespace.
- [The Mesh frames guide](https://pamoja.molex.cloud/docs/guides/mesh.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
