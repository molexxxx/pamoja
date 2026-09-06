# Pamoja.Zenoh

Zenoh key expressions: validity, canonical form, and wildcard matching. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/zenoh.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Zenoh
```

```csharp
using Pamoja.Zenoh;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs):

```csharp
// A key expression names a set of keys. `*` stands for exactly one chunk, so this
// selects the battery of any node, and not a battery nested deeper.
const string AnyNode = "fleet/*/battery";
foreach (string key in new[] { "fleet/n7/battery", "fleet/n7/rack/battery" })
{
    Console.WriteLine($"{AnyNode} covers {key}: {KeyExpression.Matches(AnyNode, key)}");
}

// `**` stands for any number of chunks, including none, which is what a
// subscription covering a whole subtree wants.
Console.WriteLine(
    "fleet/** covers a nested key: "
    + KeyExpression.Matches("fleet/**", "fleet/n7/rack/battery"));
Console.WriteLine(
    "fleet/**/battery covers fleet/battery: "
    + KeyExpression.Matches("fleet/**/battery", "fleet/battery"));

// Two expressions that select the same keys have one canonical form. Comparing or
// routing on the written form would treat these as different subscriptions.
const string Written = "fleet/**/**/battery";
string? canonical = KeyExpression.Canonize(Written);
Console.WriteLine(
    $"{Written} is canonical: {KeyExpression.IsCanon(Written)},"
    + $" and canonizes to {canonical}");

// A malformed expression is rejected rather than canonized into something
// plausible.
const string Malformed = "fleet//battery";
Console.WriteLine(
    $"{Malformed} is valid: {KeyExpression.IsValid(Malformed)},"
    + $" canonizes to {KeyExpression.Canonize(Malformed) ?? "nothing"}");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-zenoh`](https://crates.io/crates/pamoja-zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html), [docs.rs](https://docs.rs/pamoja-zenoh), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-zenoh) |
| TypeScript | [`@pamoja/zenoh`](https://www.npmjs.com/package/@pamoja/zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-zenoh) |
| Python | [`pamoja-zenoh`](https://pypi.org/project/pamoja-zenoh/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-zenoh) |
| C# | [`Pamoja.Zenoh`](https://www.nuget.org/packages/Pamoja.Zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-zenoh) |

## Documentation

- [`Pamoja.Zenoh` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html), every type in this namespace.
- [The Zenoh keys guide](https://pamoja.molex.cloud/docs/guides/zenoh.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
