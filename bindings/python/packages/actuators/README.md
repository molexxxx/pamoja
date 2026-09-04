# pamoja-actuators

PCA9685 PWM and servo pulses, and stepper coil sequencing. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
pip install pamoja-actuators
```

```python
from pamoja import actuators
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-actuators`](https://crates.io/crates/pamoja-actuators) | [docs.rs](https://docs.rs/pamoja-actuators), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html) |
| TypeScript | [`@pamoja/actuators`](https://www.npmjs.com/package/@pamoja/actuators) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html) |
| Python | [`pamoja-actuators`](https://pypi.org/project/pamoja-actuators/) | [`pamoja.actuators`](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html) |
| C# | [`Pamoja.Actuators`](https://www.nuget.org/packages/Pamoja.Actuators) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.Pca9685.html) |

## Documentation

- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
