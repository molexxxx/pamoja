# pamoja-core

The pamoja engine's surface for Python: the runtime version, the error every native call raises, and the transport every link shares. This is the counterpart of the `pamoja-core` crate, and like it, it is small; the compiled engine is `pamoja-native`, which this package depends on.

## Install

```sh
pip install pamoja-core
```

```python
from pamoja.core import version, PamojaError, Transport
```

Each capability is its own distribution (`pamoja-mqtt` gives `pamoja.mqtt`, and so on) and `pip install pamoja` is the whole framework in one package.

## Documentation

- [The reference for `pamoja.core`](https://pamoja.molex.cloud/docs/reference/python/pamoja/core.html), generated from its source.
- [The guides](https://pamoja.molex.cloud/docs/) and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
