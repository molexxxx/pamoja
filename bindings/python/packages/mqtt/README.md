# pamoja-mqtt

An MQTT client with the topic and wildcard rules, as the core transport. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/mqtt.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mqtt.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-mqtt
```

```python
from pamoja import mqtt
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/mqtt.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mqtt.py):

```python
import asyncio

from pamoja.core import PamojaError
from pamoja.mqtt import MqttClient, Qos


async def main() -> None:
    # MQTT numbers its three delivery guarantees 0, 1 and 2 on the wire; the binding
    # names them, in that order.
    assert [level.value for level in Qos] == ["AtMostOnce", "AtLeastOnce", "ExactlyOnce"]

    # Nothing listens on this port, so the broker is unreachable. Constructing the client
    # touches nothing; only connecting does.
    client = MqttClient(
        client_id="guide-node",
        host="127.0.0.1",
        port=47811,
        keep_alive_secs=1,
        qos=Qos.EXACTLY_ONCE,
    )
    assert await client.is_connected() is False

    # A refused connection surfaces as a transport error and leaves the client as it was,
    # so the same object can be retried once the broker is back.
    try:
        await client.connect()
    except PamojaError as error:
        assert str(error).startswith("transport error")
    else:
        raise AssertionError("connecting to a closed port should raise")

    assert await client.is_connected() is False


asyncio.run(main())
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mqtt`](https://crates.io/crates/pamoja-mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mqtt/index.html), [docs.rs](https://docs.rs/pamoja-mqtt) |
| TypeScript | [`@pamoja/mqtt`](https://www.npmjs.com/package/@pamoja/mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html) |
| Python | [`pamoja-mqtt`](https://pypi.org/project/pamoja-mqtt/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mqtt.html) |
| C# | [`Pamoja.Mqtt`](https://www.nuget.org/packages/Pamoja.Mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html) |

## Documentation

- [`pamoja.mqtt` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mqtt.html), every class and function in this module.
- [The MQTT guide](https://pamoja.molex.cloud/docs/guides/mqtt.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
