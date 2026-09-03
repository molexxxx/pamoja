# pamoja-native

The compiled pamoja engine for Python, built with PyO3 and maturin, with wheels for Linux (x64, arm64), macOS (x64, arm64), and Windows (x64), and the generated contract every `pamoja` package builds on. It is one extension module, `pamoja._native`, that carries every capability; the capability distributions are facades over it, so picking distributions narrows the API you depend on, not the size of the engine.

You do not install this distribution directly. Every `pamoja-<capability>` distribution and the `pamoja` metapackage depend on it. `pamoja.raw` re-exports the contract for anything a facade does not cover, and `pamoja/_native/__init__.pyi` types it.

## Documentation

- [The guides](https://pamoja.molex.cloud/docs/) and the [Python reference](https://pamoja.molex.cloud/docs/reference/python/pamoja.html).

## License

MIT
