# pamoja-sensors

Datasheet-anchored drivers for eleven parts, from every language: the BME280, BMP280, DS18B20, HDC1080, INA219, INA226, ADS1115, OPT3001, SCD4x, SHT3x, and TMP117. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sensors.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/sensors.html)

## Install

```sh
pip install pamoja-sensors
```

```python
from pamoja import sensors
```

This pulls in `pamoja-native`, the compiled engine, and `pamoja-hal`. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/sensors.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sensors.py):

```python
import os
import shutil
import tempfile
from pathlib import Path

from pamoja.core import PamojaError
from pamoja.hal import I2cBus
from pamoja.sensors import (
    Ads1115,
    Ads1115Mux,
    Ads1115Pga,
    Bmp280,
    Ds18b20Thermometer,
    Hdc1080,
    Ina219,
    Ina226,
    Opt3001,
    Scd4x,
    Sht3x,
    Tmp117,
    ads1115,
    bmp280,
    ds18b20,
    hdc1080,
    ina219,
    ina226,
    opt3001,
    scd4x,
    sht3x,
    tmp117,
)

# The address plan. Every part answers at the address its pins choose, and two parts on one
# address garble each other, so a bench of nine is planned around the two that cannot move:
# the HDC1080 and the SCD41 have one address each.
AIR = sht3x.ADDRESS_A
PRESSURE = bmp280.ADDRESS_SECONDARY
LIGHT = opt3001.ADDRESS_SCL
SOIL = ads1115.ADDRESS_SDA
ENCLOSURE = tmp117.ADDRESS_ADD0_VPLUS
PANEL = ina219.address(ina219.PIN_GROUND, ina219.PIN_SUPPLY)
BATTERY = ina226.address(ina226.PIN_SUPPLY, ina226.PIN_SUPPLY)

# The bench with nothing plugged in: each part answers the way its datasheet says, with the
# reading it is given here, a warm and humid afternoon. On a Raspberry Pi the bus is
# I2cBus.open("/dev/i2c-1") and nothing after this statement changes.
bus = I2cBus.simulated([
    sht3x.sim.reporting(AIR, 24.1, 62.0),
    bmp280.sim.reporting(PRESSURE, 24.1, 1003.2),
    scd4x.sim.reporting(1_180, 24.1, 62.0),
    opt3001.sim.reporting(LIGHT, 4_200.0),
    ads1115.sim.reporting(SOIL, Ads1115Pga.FSR_4_096, 2.35),
    tmp117.sim.reporting(ENCLOSURE, 31.25),
    hdc1080.sim.reporting(31.25, 38.0),
    ina219.sim.reporting(PANEL, 100, 3_200_000, 18_400, 1_250_000),
    ina226.sim.reporting(BATTERY, 2, 20_000_000, 12_800_000, -350_000),
])

# One driver per part. Each holds its own share of the bus, runs its datasheet's whole
# conversation on the first measurement, and releases the interpreter while the part answers.
air_now = Sht3x(bus, AIR).measure()
print(f"air          {air_now.celsius:.2f} C, {air_now.relative_humidity:.2f} %")

weather = Bmp280(bus, PRESSURE).measure()
print(f"pressure     {weather.hectopascals:.1f} hPa")

co2 = Scd4x(bus).measure()
print(f"co2          {co2.co2_ppm} ppm")

sun = Opt3001(bus, LIGHT).measure()
print(f"light        {sun.lux:.0f} lux")

# A capacitive probe's voltage falls as the soil wets and nears 3 V in dry soil, past the
# ADS1115's default range of 2.048 V, so the driver is given the 4.096 V range.
probe = Ads1115(bus, SOIL, mux=Ads1115Mux.AIN0_GND, pga=Ads1115Pga.FSR_4_096).sample()
print(f"soil         {probe.volts:.3f} V")

box = Tmp117(bus, ENCLOSURE).measure()
box_air = Hdc1080(bus).measure()
print(f"enclosure    {box.celsius:.2f} C, {box_air.relative_humidity:.1f} %")

# Two current monitors, each calibrated for its own shunt. The fans and the pump draw more than
# the panel gives, so the battery makes up the rest and its current reads negative: current
# through a shunt is signed by its direction.
charge = Ina219(bus, PANEL, shunt_milliohms=100, max_microamps=3_200_000).measure()
print(
    f"panel        {charge.bus_millivolts / 1e3:.2f} V, "
    f"{charge.current_microamps / 1e6:.2f} A, {charge.power_microwatts / 1e6:.2f} W"
)
drain = Ina226(bus, BATTERY, shunt_milliohms=2, max_microamps=20_000_000).measure()
print(
    f"battery      {drain.bus_volts:.2f} V, {drain.current_amps:.2f} A, "
    f"{drain.power_watts:.2f} W"
)

# Every driver waited as its datasheet asks, the OPT3001's 800 ms integration and the SCD41's
# command times most of all. The simulated bus counted the waits and slept none.
print(f"waited       {bus.waited_micros / 1e6:.2f} s across {bus.transfers} transfers")

# The same probe read at the default range. Past 2.048 V the converter pins at its top code,
# and the sample says so rather than passing the edge of the range off as a reading.
bus.attach(ads1115.sim.reporting(SOIL, Ads1115Pga.FSR_2_048, 2.35))
pinned = Ads1115(bus, SOIL, mux=Ads1115Mux.AIN0_GND).sample()
print(f"default gain {pinned.volts:.3f} V, clipped: {str(pinned.clipped).lower()}")

# A driver aimed at the wrong address meets whatever answers there. The TMP117 reads its
# device id before anything else, so pointed at the soil probe's converter it refuses rather
# than reporting that part's registers as a temperature.
try:
    Tmp117(bus, SOIL).init()
    print("wrong part   accepted, which should never happen")
except PamojaError as error:
    print(f"wrong part   {error}")

# A DS18B20 on a lead into a pot, read the way a Linux board reads one: the kernel's 1-Wire
# driver serves each probe as a file under /sys/bus/w1/devices. The program writes that file
# itself, with the text the kernel prints, so it runs anywhere.
devices = Path(tempfile.gettempdir()) / f"pamoja-bench-{os.getpid()}"
directory = devices / f"{ds18b20.FAMILY_CODE:02x}-000005e2fdc3"
directory.mkdir(parents=True, exist_ok=True)
scratchpad = ds18b20.build_scratchpad(19.5, 12, 30, 5)
(directory / "w1_slave").write_text(ds18b20.w1_slave_text(scratchpad), newline="")
found = []
for thermometer in Ds18b20Thermometer.discover(str(devices)):
    reading = thermometer.read()
    print(
        f"soil probe   {reading.celsius:.4f} C, "
        f"alarms at {reading.alarm_low} and {reading.alarm_high} C"
    )
    found.append(reading)
shutil.rmtree(devices)
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
