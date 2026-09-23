# pamoja-hal

The embedded-hal traits every driver takes, a bit-banged 1-Wire bus, simulated parts and scripted buses that stand in for hardware, the Linux backends over i2c-dev, spidev, and the GPIO character device, and one I2C bus a program and its drivers share. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/hal.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/hal.html)

## Install

```sh
pip install pamoja-hal
```

```python
from pamoja import hal
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/hal.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/hal.py):

```python
import time

from pamoja.sensors import bme280

BME280 = bme280.ADDRESS_PRIMARY


class ScriptedBus:
    """A bus with the two calls this conversation needs, in the shape of smbus2.

    On a gateway ``SMBus(1)`` gives the real one and nothing below changes. This one
    answers from a script of what a BME280 sends, in the order the datasheet lists,
    and refuses any transfer that is not the next one.
    """

    def __init__(self, script):
        self.script = script
        self.transfers = 0

    def write_byte_data(self, address, register, value):
        step = self._next(address, register)
        if step.get("write") != value:
            raise ValueError(f"unexpected write of {value:#04x}")

    def read_i2c_block_data(self, address, register, length):
        step = self._next(address, register)
        reply = step.get("reply")
        if reply is None or len(reply) != length:
            raise ValueError(f"unexpected read of {length} bytes")
        return bytes(reply)

    @property
    def done(self):
        return self.transfers == len(self.script)

    def _next(self, address, register):
        step = self.script[self.transfers] if self.transfers < len(self.script) else None
        if address != BME280 or step is None or step["register"] != register:
            raise ValueError(f"unexpected transfer at register {register:#04x}")
        self.transfers += 1
        return step


CALIBRATION_A = bytes([
    0x45, 0x6F, 0x6F, 0x68, 0x32, 0x00, 0x46, 0x91, 0x6A, 0xD6, 0xD0, 0x0B, 0x4E, 0x1E,
    0x88, 0xFF, 0xF9, 0xFF, 0xAC, 0x26, 0x0A, 0xD8, 0xBD, 0x10, 0x00, 0x4B,
])
CALIBRATION_B = bytes([0x62, 0x01, 0x00, 0x15, 0x23, 0x03, 0x1E])
BURST = bytes([0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x75, 0x30])


def main():
    bus = ScriptedBus([
        {"register": 0xE0, "write": 0xB6},
        {"register": 0xF3, "reply": [0x00]},
        {"register": 0xD0, "reply": [bme280.CHIP_ID]},
        {"register": 0x88, "reply": CALIBRATION_A},
        {"register": 0xE1, "reply": CALIBRATION_B},
        {"register": 0xF5, "write": 0x00},
        {"register": 0xF2, "write": 0x01},
        {"register": 0xF4, "write": 0x24},
        {"register": 0xF4, "write": 0x25},
        {"register": 0xF3, "reply": [0x00]},
        {"register": 0xF7, "reply": BURST},
    ])

    # The datasheet's start-up: the soft reset word, its 2 ms start-up time, and the
    # status register, whose low bit clears once the calibration image has loaded.
    bus.write_byte_data(BME280, 0xE0, 0xB6)
    time.sleep(0.002)
    bus.read_i2c_block_data(BME280, 0xF3, 1)

    # The chip id says it is a BME280, and the two calibration blocks are read once.
    chip_id = bus.read_i2c_block_data(BME280, 0xD0, 1)[0]
    temp_press = bus.read_i2c_block_data(BME280, 0x88, 26)
    humidity = bus.read_i2c_block_data(BME280, 0xE1, 7)
    calibration = bme280.calibration(temp_press, humidity)
    print(f"calibration  read once: {str(chip_id == bme280.CHIP_ID).lower()}")

    # config, ctrl_hum, then ctrl_meas, in that order because ctrl_hum only takes
    # effect after the ctrl_meas write: every measurement at oversampling x1, asleep.
    bus.write_byte_data(BME280, 0xF5, 0x00)
    bus.write_byte_data(BME280, 0xF2, 0x01)
    bus.write_byte_data(BME280, 0xF4, 0x24)

    # One forced measurement: the mode bits, the datasheet's 9.3 ms maximum for these
    # settings, the status read that confirms the part is idle, and the burst read.
    bus.write_byte_data(BME280, 0xF4, 0x25)
    time.sleep(0.010)
    bus.read_i2c_block_data(BME280, 0xF3, 1)
    burst = bus.read_i2c_block_data(BME280, 0xF7, 8)
    measurement = calibration.compensate(burst)
    celsius = measurement.celsius
    hectopascals = measurement.hectopascals
    humidity_percent = measurement.relative_humidity_percent
    print(f"measured     {celsius:.2f} C, {hectopascals:.2f} hPa, {humidity_percent:.2f} %")

    # The script is spent: every transfer the datasheet lists was made, and no other.
    print(f"bus          {bus.transfers} transfers, unexpected: {str(not bus.done).lower()}")
    return measurement, bus.transfers, bus.done


measurement, transfers, done = main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-hal`](https://crates.io/crates/pamoja-hal) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_hal/index.html), [docs.rs](https://docs.rs/pamoja-hal), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-hal) |
| TypeScript | [`@pamoja/hal`](https://www.npmjs.com/package/@pamoja/hal) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_hal.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-hal) |
| Python | [`pamoja-hal`](https://pypi.org/project/pamoja-hal/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/hal.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-hal) |
| C# | [`Pamoja.Hal`](https://www.nuget.org/packages/Pamoja.Hal) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Hal.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-hal) |

## Documentation

- [`pamoja.hal` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/hal.html), every class and function in this module.
- [The Buses guide](https://pamoja.molex.cloud/docs/guides/hal.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
