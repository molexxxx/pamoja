# pamoja-serial

SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
pip install pamoja-serial
```

```python
from pamoja import serial
```

This pulls in `pamoja-native`, the compiled engine, and nothing else. `pip install pamoja` is the whole framework in one package.

## Documentation

- [The reference for `pamoja.serial`](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html), generated from its source.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
