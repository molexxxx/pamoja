# pamoja-core (Python)

Python bindings for the [pamoja](https://github.com/molexxxx/pamoja) device
SDK core, built with [PyO3](https://pyo3.rs) and [maturin](https://www.maturin.rs).

The generated surface is intentionally thin. A hand-written, idiomatic layer is
added on top of it so Python callers get a native-feeling async API - awaitable
methods, `async for` over incoming messages, `async with` lifecycle, and
exceptions for errors - while all behavior stays in the Rust core.

The generated low-level contract remains available at `pamoja.raw`.

## Install

```
pip install pamoja-core
```

## Build from source

```
python -m venv .venv
.venv/bin/pip install maturin pytest
.venv/bin/maturin develop
.venv/bin/python -m pytest
```

`maturin develop` compiles the Rust core into a native extension (`pamoja._core`)
and installs the `pamoja` package into the active environment.

`cargo run --bin stub_gen` regenerates the committed type stub
`python/pamoja/_core.pyi`. It is a generated artifact, drift-checked in CI so it
can never fall behind the Rust source.

## Usage

```python
import asyncio
from pamoja import MqttClient

async def main():
    async with MqttClient(client_id="sensor-1", host="localhost", port=1883) as client:
        await client.subscribe("sensors/+/temperature")
        await client.publish("sensors/1/temperature", "21.5")
        async for message in client:
            print(message.topic, message.payload.decode())

asyncio.run(main())
```

Beyond the transport, the same import carries device identity, the wire codecs,
the helper math, and the field I/O a gateway needs, each in its own module
(`pamoja.security`, `pamoja.codec`, `pamoja.kit`, `pamoja.serial`,
`pamoja.modbus`, `pamoja.can`, `pamoja.gpio`) and all reachable from `pamoja`.
The generated low-level contract stays available at `pamoja.raw`.

```python
from pamoja import DeviceIdentity, Smoother, to_cbor

smoother = Smoother(0.3)
reading = smoother.update(21.7)

device = DeviceIdentity.from_seed(seed)
payload = to_cbor({"c": reading})
signature = device.sign(payload)
```

Talking to the wires themselves looks the same way:

```python
from pamoja import modbus, serial

# Ask an RS485 energy meter for three holding registers.
port.write(modbus.read_holding_registers(0x11, 0x006B, 3))

# Reassemble whole packets from the chunks the port delivers.
decoder = serial.SlipDecoder()
for frame in decoder.feed(port.read(256)):
    handle(frame)
```

`pytest` runs the smoke tests and the cross-language conformance suite, which
asserts the same vectors every other binding does.
