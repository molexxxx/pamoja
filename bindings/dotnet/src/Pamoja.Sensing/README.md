# Pamoja.Sensing

The parts wired to a board: a thermometer that checks its own bytes, a servo pulse, a stepper walking its coils, and a part of your own.

One reference for the 3 capabilities of this domain. Each is also its own package,
and `Pamoja` is the whole framework in one.

```sh
dotnet add package Pamoja.Sensing
```

This package ships no assembly: it brings in the packages below, and each keeps its own
namespace, so a type is named the way it is when the package is referenced directly.

| Capability | Package | What it covers |
| --- | --- | --- |
| [Sensor drivers](https://pamoja.molex.cloud/docs/guides/sensors.html) | `Pamoja.Sensors` | Datasheet-anchored decoders for eleven parts: the BME280, BMP280, DS18B20, HDC1080, INA219, INA226, ADS1115, OPT3001, SCD4x, SHT3x, and TMP117 |
| [Actuator drivers](https://pamoja.molex.cloud/docs/guides/actuators.html) | `Pamoja.Actuators` | PCA9685 PWM and servo pulses, and stepper coil sequencing |
| [Your own device](https://pamoja.molex.cloud/docs/guides/device.html) | `Pamoja.Core` | A sensor and an actuator pamoja has never heard of, written against the core traits, run against a rule, and published with nothing plugged in |

The guides, with a worked C# example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
