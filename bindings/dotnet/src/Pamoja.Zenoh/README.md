# Pamoja.Zenoh

Zenoh key expressions: validity, canonical form, matching, and whether two expressions share or cover keys. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/zenoh.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html)

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
// A key expression names a set of keys. Chunks sit between slashes, `*` stands for
// exactly one chunk, whatever it holds, and `**` for any number of them, including none.
static string Selects(string pattern, string key)
{
    string verdict = KeyExpression.Matches(pattern, key) ? "covers" : "misses";
    return $"{verdict,-10}{pattern} {verdict} {key}";
}
Console.WriteLine(Selects("farm/*/power", "farm/t7/power"));
Console.WriteLine(Selects("farm/*/power", "farm/substation/power"));
Console.WriteLine(
    $"{Selects("farm/*/power", "farm/row2/t14/power")}, since * is exactly one chunk");
Console.WriteLine(Selects("farm/**/power", "farm/row2/t14/power"));
Console.WriteLine(
    $"{Selects("farm/**/alarm", "farm/alarm")}, where ** is no chunk at all");

// `$*` stands for any run of characters inside one chunk, so it selects on part of a
// name.
Console.WriteLine(Selects("farm/t$*/power", "farm/t7/power"));
Console.WriteLine(Selects("farm/t$*/power", "farm/substation/power"));

// One set of keys has one canonical spelling, and a Zenoh session accepts no other.
foreach (string written in new[] { "farm/*/**/power", "farm/**/*/power", "farm/**/**/power" })
{
    string? canonical = KeyExpression.Canonize(written);
    if (KeyExpression.IsCanon(written))
    {
        Console.WriteLine($"canonical {written}, as written");
    }
    else
    {
        Console.WriteLine($"rewritten {written} is spelled {canonical}");
    }
}

// Joining places one expression beneath another, and canonizes the seam between them.
foreach ((string prefix, string suffix) in new[] { ("farm/t7", "power"), ("farm/**", "*/power") })
{
    Console.WriteLine(
        $"joined    {prefix} and {suffix} make {KeyExpression.Join(prefix, suffix)}");
}

// A malformed expression is refused rather than repaired into something plausible.
foreach ((string written, string why) in new[]
{
    ("farm//power", "a chunk is empty"),
    ("farm/t7*/power", "* stands alone in its chunk, or after $"),
    ("farm/t7/power?", "? and # are reserved"),
})
{
    if (!KeyExpression.IsValid(written) && KeyExpression.Canonize(written) is null)
    {
        Console.WriteLine($"malformed {written}, since {why}");
    }
}
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
