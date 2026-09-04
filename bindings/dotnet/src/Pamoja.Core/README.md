# Pamoja.Core

The pamoja engine's surface for .NET: the runtime version, the exception every native call can raise, and the transport every link shares. This is the counterpart of the `pamoja-core` crate, and like it, it is small; the compiled engine is `Pamoja.Native`, which this package depends on.

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
