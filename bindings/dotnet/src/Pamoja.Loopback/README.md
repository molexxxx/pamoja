# Pamoja.Loopback

An in-process transport with topic matching and a fault injector, for testing with no broker. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
dotnet add package Pamoja.Loopback
```

```csharp
using Pamoja.Loopback;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Core`, and nothing else. `dotnet add package Pamoja` is the whole framework in one package.

## Documentation

- [The reference for `Pamoja.Loopback`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html), generated from its source.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
