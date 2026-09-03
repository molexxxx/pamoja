# Pamoja.Sync

Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
dotnet add package Pamoja.Sync
```

```csharp
using Pamoja.Sync;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Core`, and nothing else. `dotnet add package Pamoja` is the whole framework in one package.

## Documentation

- [The reference for `Pamoja.Sync`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html), generated from its source.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
