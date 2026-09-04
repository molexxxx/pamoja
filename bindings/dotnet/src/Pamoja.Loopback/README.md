# Pamoja.Loopback

An in-process transport with topic matching and a fault injector, for testing with no broker. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Loopback
```

```csharp
using Pamoja.Loopback;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Core`. `dotnet add package Pamoja` is the whole framework in one package.

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-loopback`](https://crates.io/crates/pamoja-loopback) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html), [docs.rs](https://docs.rs/pamoja-loopback) |
| TypeScript | [`@pamoja/loopback`](https://www.npmjs.com/package/@pamoja/loopback) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html) |
| Python | [`pamoja-loopback`](https://pypi.org/project/pamoja-loopback/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/loopback.html) |
| C# | [`Pamoja.Loopback`](https://www.nuget.org/packages/Pamoja.Loopback) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html) |

## Documentation

- [`Pamoja.Loopback` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html), every type in this namespace.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
