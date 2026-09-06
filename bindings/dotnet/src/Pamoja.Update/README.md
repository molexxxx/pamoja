# Pamoja.Update

Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/update.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Update
```

```csharp
using Pamoja.Update;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Codec` and `Pamoja.Security`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/UpdateGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/UpdateGuide.cs):

```csharp
// The publisher's key signs releases; devices in the field are anchored to its
// public half and will take firmware from nobody else.
byte[] seed = new byte[32];
Array.Fill(seed, (byte)7);
using var publisher = new DeviceIdentity(seed);
byte[] vendor = Enumerable.Repeat((byte)0x0A, 16).ToArray();
byte[] deviceClass = Enumerable.Repeat((byte)0x0B, 16).ToArray();

// The release. A manifest says who the image is for, which slot it belongs in, how
// big it is and what it hashes to; nothing about the image is taken on trust.
byte[] image = Encoding.ASCII.GetBytes("firmware for a flow meter, version two");
var manifest = new Manifest(
    Sequence: 2,
    VendorId: vendor,
    ClassId: deviceClass,
    Storage: 1,
    Digest: Update.ImageDigest(image),
    Size: (uint)image.Length);
byte[] envelope = Update.SignManifest(manifest, publisher);
Console.WriteLine(
    $"published sequence {manifest.Sequence} in a {envelope.Length}-byte envelope");

// On the device. It checks the envelope against the key it was anchored to before
// it accepts a single byte of the image.
Manifest opened = Update.VerifyEnvelope(envelope, publisher.PublicKey);
Console.WriteLine($"accepted  a release for slot {opened.Storage}");

// It left the factory running sequence 1 from slot 0, so the release goes to the
// spare slot and the image it is running stays where it is.
using var fleet = new Updater(vendor, deviceClass, publisher.PublicKey, 2, 4096);
fleet.Provision(0, 1);
fleet.Begin(envelope);
for (int at = 0; at < image.Length; at += 16)
{
    fleet.Write(image.AsSpan(at, Math.Min(16, image.Length - at)));
}

Console.WriteLine($"staged    {fleet.CurrentProgress().Written} of {image.Length} bytes");
byte slot = fleet.Finish();
Console.WriteLine($"written   to slot {slot}, leaving the running image alone");

// The first boot into a new image is a trial. It reverts on the next boot unless
// the device confirms it came up, which is what makes a bad release survivable.
Console.WriteLine($"booting   {fleet.OnBoot().Action}");
fleet.Confirm();
Console.WriteLine($"confirmed slot {slot} is now {fleet.Record(slot).State}");

// The same release signed by a key this device is not anchored to gets nowhere.
byte[] impostorSeed = new byte[32];
Array.Fill(impostorSeed, (byte)90);
using var impostor = new DeviceIdentity(impostorSeed);
try
{
    fleet.Stage(Update.SignManifest(manifest, impostor), image);
    Console.WriteLine("a forged release was accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"forged    refused: {error.Message}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-update`](https://crates.io/crates/pamoja-update) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_update/index.html), [docs.rs](https://docs.rs/pamoja-update), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-update) |
| TypeScript | [`@pamoja/update`](https://www.npmjs.com/package/@pamoja/update) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-update) |
| Python | [`pamoja-update`](https://pypi.org/project/pamoja-update/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/update.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-update) |
| C# | [`Pamoja.Update`](https://www.nuget.org/packages/Pamoja.Update) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-update) |

## Documentation

- [`Pamoja.Update` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.html), every type in this namespace.
- [The Signed updates guide](https://pamoja.molex.cloud/docs/guides/update.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
