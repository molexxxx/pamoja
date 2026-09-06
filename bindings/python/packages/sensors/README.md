# pamoja-sensors

Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/sensors.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sensors.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-sensors
```

```python
from pamoja import sensors
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/sensors.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sensors.py):

```python
from pamoja.core import PamojaError
from pamoja.sensors import ds18b20, ina219

# Stand-ins for the two parts. On a running node the thermometer's nine bytes come off
# the 1-Wire bus and the monitor's registers off I2C; here the library builds what each
# would send, so the program runs with nothing plugged in.
thermometer = ds18b20.build_scratchpad(25.0625, 12, 75, -10)

# The monitor is set up for 1 mA per count across a 2 milliohm shunt, the load its
# datasheet's worked design example describes: 11.98 V, 10 A, and 119.8 W.
CURRENT_LSB = 1_000
bus = ina219.bus_register(11_980)
current = ina219.current_register(10_000_000, CURRENT_LSB)
power = ina219.power_register(119_800_000, CURRENT_LSB)

# Everything below is the node's own code. The thermometer checksums every read, so a
# reading is verified before it is believed.
reading = ds18b20.parse_scratchpad(thermometer)
print(f"temperature  {reading.celsius:.4f} C")
print(f"resolution   {reading.resolution_bits} bits")
print(f"alarms       {reading.alarm_high} / {reading.alarm_low} C")

# The monitor computes nothing until it has been told what shunt it is across.
print(f"calibration  0x{ina219.calibration(CURRENT_LSB, 2):04X}")
print(f"bus          {ina219.bus_millivolts(bus)} mV")
print(f"current      {ina219.current_microamps(current, CURRENT_LSB) // 1_000} mA")
print(f"power        {ina219.power_microwatts(power, CURRENT_LSB) // 1_000} mW")

# A bit flipped on a long 1-Wire run fails the checksum, so the node repeats the read
# instead of logging a temperature a couple of degrees off.
corrupted = bytearray(thermometer)
corrupted[0] ^= 1
try:
    ds18b20.parse_scratchpad(bytes(corrupted))
    print("corrupt read accepted, which should never happen")
except PamojaError as error:
    print(f"corrupt read rejected: {error}")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sensors`](https://crates.io/crates/pamoja-sensors) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html), [docs.rs](https://docs.rs/pamoja-sensors), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sensors) |
| TypeScript | [`@pamoja/sensors`](https://www.npmjs.com/package/@pamoja/sensors) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sensors) |
| Python | [`pamoja-sensors`](https://pypi.org/project/pamoja-sensors/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sensors.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sensors) |
| C# | [`Pamoja.Sensors`](https://www.nuget.org/packages/Pamoja.Sensors) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sensors) |

## Documentation

- [`pamoja.sensors` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sensors.html), every class and function in this module.
- [The Sensor drivers guide](https://pamoja.molex.cloud/docs/guides/sensors.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
