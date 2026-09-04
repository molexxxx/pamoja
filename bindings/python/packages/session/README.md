# pamoja-session

X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
pip install pamoja-session
```

```python
from pamoja import session
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-session`](https://crates.io/crates/pamoja-session) | [docs.rs](https://docs.rs/pamoja-session), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html) |
| TypeScript | [`@pamoja/session`](https://www.npmjs.com/package/@pamoja/session) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html) |
| Python | [`pamoja-session`](https://pypi.org/project/pamoja-session/) | [`pamoja.session`](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html) |
| C# | [`Pamoja.Session`](https://www.nuget.org/packages/Pamoja.Session) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.Session.html) |

## Documentation

- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
