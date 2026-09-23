# pamoja-hal

The embedded-hal traits every driver takes, a bit-banged 1-Wire bus, simulated parts and scripted buses that stand in for hardware, the Linux backends over i2c-dev, spidev, and the GPIO character device, one I2C bus a program and its drivers share, and delays that sleep or only count. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
from pamoja.core import PamojaError
from pamoja.hal import I2cBus, I2cStep
from pamoja.sensors import Bme280, Bme280Config, Bme280CtrlMeas, Bme280Measurement, bme280

BME280 = bme280.ADDRESS_PRIMARY


def shown(reading: Bme280Measurement) -> str:
    """A reading the way every line below prints it."""
    return (
        f"{reading.celsius:.2f} C, {reading.hectopascals:.2f} hPa, "
        f"{reading.relative_humidity_percent:.2f} %"
    )


def x(code: int) -> str:
    """An oversampling setting the way a datasheet writes it."""
    return f"x{bme280.oversampling_factor(code)}"


# A bus with one part on it: a BME280 that is not there. It holds a real part's calibration
# and one measurement that part took, and it answers from its registers, so the driver runs
# its whole datasheet sequence against it. On a Raspberry Pi the bus is
# I2cBus.open("/dev/i2c-1") and nothing after this line changes.
bus = I2cBus.simulated([bme280.sim.part(BME280)])
sensor = Bme280(bus, BME280)

# Reset, identify, calibrate, configure. The datasheet wants ctrl_hum written before
# ctrl_meas, and the part left asleep until a measurement is forced. The part keeps what the
# driver wrote, so the configuration reads back off the bus.
sensor.init()
part = bus.part(BME280)
humidity = bme280.ctrl_hum_from_bits(part.register(bme280.REGISTER_CTRL_HUM))
ctrl = bme280.ctrl_meas_from_bits(part.register(bme280.REGISTER_CTRL_MEAS))
asleep = ctrl.mode == bme280.Mode.SLEEP
print(
    f"configured   humidity {x(humidity)}, temperature {x(ctrl.temperature)}, "
    f"pressure {x(ctrl.pressure)}, asleep: {str(asleep).lower()}"
)

# One forced measurement. The driver waits the datasheet's longest measurement time for
# these settings before it reads, and a simulated bus counts that wait rather than sleeping
# through it.
reading = sensor.measure()
print(f"measured     {shown(reading)}")
print(f"waited       {bus.waited_micros / 1000:.2f} ms across {bus.transfers} transfers")
waited = bus.waited_micros

# A part reports whatever it is asked to. Putting one in the first one's place is how a
# program meets a reading it would otherwise wait on the weather for, here a cold store at
# four degrees, and the driver carries on without noticing.
bus.attach(bme280.sim.reporting(BME280, 4.0, 1013.25, 80.0))
cold = sensor.measure()
print(f"cold store   {shown(cold)}")

# Nothing answers at the part's other address, and the driver says so rather than
# returning a reading.
try:
    Bme280(bus, bme280.ADDRESS_SECONDARY).init()
    print("absent       a part answered")
except PamojaError as error:
    print(f"absent       {error}")

# The other half of the bus layer. A script plays one conversation and refuses anything
# else, which proves a driver follows the datasheet rather than merely working: the reset,
# the status once the calibration has loaded, the chip id, the two calibration blocks, the
# three configuration writes in the order the part requires, then one forced measurement.
x1 = bme280.Oversampling.X1
settings = Bme280CtrlMeas(temperature=x1, pressure=x1, mode=bme280.Mode.SLEEP)
forced = Bme280CtrlMeas(temperature=x1, pressure=x1, mode=bme280.Mode.FORCED)
idle = bytes([bme280.sim.STATUS_IDLE])
script = I2cBus.scripted([
    I2cStep.write(BME280, bytes([bme280.REGISTER_RESET, bme280.RESET_WORD])),
    I2cStep.write_read(BME280, bytes([bme280.REGISTER_STATUS]), idle),
    I2cStep.write_read(BME280, bytes([bme280.REGISTER_CHIP_ID]), bytes([bme280.CHIP_ID])),
    I2cStep.write_read(
        BME280, bytes([bme280.REGISTER_CALIB_TEMP_PRESS]), bme280.sim.calibration()
    ),
    I2cStep.write_read(
        BME280, bytes([bme280.REGISTER_CALIB_HUMIDITY]), bme280.sim.calibration_humidity()
    ),
    I2cStep.write(BME280, bytes([bme280.REGISTER_CONFIG, bme280.config_bits(Bme280Config())])),
    I2cStep.write(BME280, bytes([bme280.REGISTER_CTRL_HUM, bme280.ctrl_hum_bits(x1)])),
    I2cStep.write(BME280, bytes([bme280.REGISTER_CTRL_MEAS, bme280.ctrl_meas_bits(settings)])),
    I2cStep.write(BME280, bytes([bme280.REGISTER_CTRL_MEAS, bme280.ctrl_meas_bits(forced)])),
    I2cStep.write_read(BME280, bytes([bme280.REGISTER_STATUS]), idle),
    I2cStep.write_read(BME280, bytes([bme280.REGISTER_DATA]), bme280.sim.burst()),
])
checked = Bme280(script, BME280).measure()
print(
    f"datasheet    {checked.celsius:.2f} C after {script.transfers} transfers, "
    f"{script.remaining} steps left"
)
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
