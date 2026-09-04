# pamoja-serial

SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
pip install pamoja-serial
```

```python
from pamoja import serial
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-serial`](https://crates.io/crates/pamoja-serial) | [docs.rs](https://docs.rs/pamoja-serial), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html) |
| TypeScript | [`@pamoja/serial`](https://www.npmjs.com/package/@pamoja/serial) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html) |
| Python | [`pamoja-serial`](https://pypi.org/project/pamoja-serial/) | [`pamoja.serial`](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html) |
| C# | [`Pamoja.Serial`](https://www.nuget.org/packages/Pamoja.Serial) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.Serial.html) |

## Documentation

- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
