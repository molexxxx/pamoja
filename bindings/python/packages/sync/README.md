# pamoja-sync

Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
pip install pamoja-sync
```

```python
from pamoja import sync
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sync`](https://crates.io/crates/pamoja-sync) | [docs.rs](https://docs.rs/pamoja-sync), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html) |
| TypeScript | [`@pamoja/sync`](https://www.npmjs.com/package/@pamoja/sync) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html) |
| Python | [`pamoja-sync`](https://pypi.org/project/pamoja-sync/) | [`pamoja.sync`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html) |
| C# | [`Pamoja.Sync`](https://www.nuget.org/packages/Pamoja.Sync) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.Store.html) |

## Documentation

- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
