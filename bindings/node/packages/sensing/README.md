# @pamoja/sensing

The parts wired to a board: a thermometer that checks its own bytes, a servo pulse, and a stepper walking its coils.

One install for the 2 capabilities of this domain. Each is also its own package, and
`pamoja` is the whole framework in one.

```sh
npm install @pamoja/sensing
```

| Capability | Package | What it covers |
| --- | --- | --- |
| [Sensor drivers](https://pamoja.molex.cloud/docs/guides/sensors.html) | `@pamoja/sensors` | Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115 |
| [Actuator drivers](https://pamoja.molex.cloud/docs/guides/actuators.html) | `@pamoja/actuators` | PCA9685 PWM and servo pulses, and stepper coil sequencing |

The guides, with a worked TypeScript example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
