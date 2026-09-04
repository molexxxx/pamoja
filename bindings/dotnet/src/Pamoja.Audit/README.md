# Pamoja.Audit

A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
dotnet add package Pamoja.Audit
```

```csharp
using Pamoja.Audit;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Core`, and nothing else. `dotnet add package Pamoja` is the whole framework in one package.

## Documentation

- [The reference for `Pamoja.Audit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html), generated from its source.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
