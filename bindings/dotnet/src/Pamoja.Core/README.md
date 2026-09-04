# Pamoja.Core

The pamoja engine's surface for .NET: the runtime version and the transport every link implements. This is the counterpart of the `pamoja-core` crate, and like it, it is small. It is a capability like the others rather than a foundation: only the transport packages depend on it, because they are the ones that return a transport. The compiled engine, which every package depends on, is `Pamoja.Native`.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Core
```

```csharp
using Pamoja.Core;
```

Each capability is its own package (`Pamoja.Mqtt`, `Pamoja.Security`, and so on) and `dotnet add package Pamoja` is the whole framework in one package.

## Documentation

- [The reference for `Pamoja.Core`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html), generated from its source.
- [The guides](https://pamoja.molex.cloud/docs/) and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
