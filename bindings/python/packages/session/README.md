# pamoja-session

X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
pip install pamoja-session
```

```python
from pamoja import session
```

This pulls in `pamoja-native`, the compiled engine, and nothing else. `pip install pamoja` is the whole framework in one package.

## Documentation

- [The reference for `pamoja.session`](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html), generated from its source.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
