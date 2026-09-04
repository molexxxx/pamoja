# Pamoja.Zenoh

Zenoh key expressions: validity, canonical form, and wildcard matching. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
Expect(KeyExpression.IsValid("fleet/*/battery"), "the pattern is well formed");
Expect(
    KeyExpression.Matches("fleet/*/battery", "fleet/n7/battery"),
    "one chunk stands in for the node");
Expect(
    !KeyExpression.Matches("fleet/*/battery", "fleet/n7/rack/battery"),
    "but not for two");

// `**` stands for any number of chunks, including none, which is what a
// subscription covering a whole subtree wants.
Expect(
    KeyExpression.Matches("fleet/**", "fleet/n7/rack/battery"),
    "the subtree wildcard reaches any depth");
Expect(
    KeyExpression.Matches("fleet/**/battery", "fleet/battery"),
    "including no chunks at all");

// Two expressions that select the same keys have one canonical form. Comparing or
// routing on the written form would treat these as different subscriptions.
Expect(!KeyExpression.IsCanon("fleet/**/**/battery"), "a repeated wildcard is not canonical");
Expect(
    KeyExpression.Canonize("fleet/**/**/battery") == "fleet/**/battery",
    "and collapses to the form that selects the same keys");
Expect(KeyExpression.IsCanon("fleet/**/battery"), "which is canonical");

// A malformed expression is rejected rather than canonized into something plausible.
Expect(!KeyExpression.IsValid("fleet//battery"), "an empty chunk is malformed");
Expect(KeyExpression.Canonize("fleet//battery") is null, "and has no canonical form");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-zenoh`](https://crates.io/crates/pamoja-zenoh) | [docs.rs](https://docs.rs/pamoja-zenoh), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html) |
| TypeScript | [`@pamoja/zenoh`](https://www.npmjs.com/package/@pamoja/zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html) |
| Python | [`pamoja-zenoh`](https://pypi.org/project/pamoja-zenoh/) | [`pamoja.zenoh`](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html) |
| C# | [`Pamoja.Zenoh`](https://www.nuget.org/packages/Pamoja.Zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.KeyExpression.html) |

## Documentation

- [The Zenoh keys guide](https://pamoja.molex.cloud/docs/guides/zenoh.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
