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

# The broker on the site. The guide's CI runs one on localhost; point these at yours and
# nothing else changes.
BROKER = "127.0.0.1"
PORT = 1883


async def main() -> None:
    # The gateway takes every temperature on the site. A `+` stands for exactly one level,
    # so this matches every node's temperature and nothing deeper.
    gateway = MqttClient(
        client_id="site-gateway", host=BROKER, port=PORT, qos=Qos.AT_LEAST_ONCE
    )
    await gateway.connect()
    await gateway.subscribe("sensors/+/temperature")
    print("gateway   subscribed to sensors/+/temperature")

    # A node publishes under that pattern. At-least-once means the broker acknowledges the
    # message, so a node knows its reading was taken rather than hoping.
    node = MqttClient(client_id="node-1", host=BROKER, port=PORT, qos=Qos.AT_LEAST_ONCE)
    await node.connect()
    await node.publish("sensors/1/temperature", "21.5")
    print("node      published 21.5 to sensors/1/temperature")

    # The gateway receives it with the topic attached, which is how it knows which node
    # sent the reading without the payload having to repeat it.
    received = await gateway.recv()
    print(f"gateway   got {received.payload.decode()} on {received.topic}")

    # Disconnecting leaves the client reusable, so a node that loses its link can
    # reconnect the same object when the broker comes back.
    await node.disconnect()
    print(f"node      disconnected, still connected: {await node.is_connected()}")
    await gateway.disconnect()

    # A broker that is not there is reported rather than leaving a client that looks
    # connected, so a retry loop has something to test.
    nowhere = MqttClient(client_id="node-2", host=BROKER, port=1, keep_alive_secs=1)
    try:
        await nowhere.connect()
        print("an unreachable broker accepted a connection, which should never happen")
    except PamojaError as error:
        print(f"unreachable broker refused: {error}")

    return received


received = asyncio.run(main())
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
