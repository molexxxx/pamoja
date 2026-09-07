# Pamoja.Sensing

The parts wired to a board: a thermometer that checks its own bytes, a servo pulse, and a stepper walking its coils.

One reference for the 2 capabilities of this domain. Each is also its own package,
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

The guides, with a worked C# example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
