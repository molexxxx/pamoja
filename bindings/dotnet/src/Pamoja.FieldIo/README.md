# Pamoja.FieldIo

The wires a gateway actually has: framed serial packets, an RS485 request and the reply it draws, a CAN frame, and the address a chip answers on.

One reference for the 4 capabilities of this domain. Each is also its own package,
and `Pamoja` is the whole framework in one.

```sh
dotnet add package Pamoja.FieldIo
```

This package ships no assembly: it brings in the packages below, and each keeps its own
namespace, so a type is named the way it is when the package is referenced directly.

| Capability | Package | What it covers |
| --- | --- | --- |
| [Serial framing](https://pamoja.molex.cloud/docs/guides/serial.html) | `Pamoja.Serial` | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
| [Modbus RTU](https://pamoja.molex.cloud/docs/guides/modbus.html) | `Pamoja.Modbus` | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
| [CAN and J1939](https://pamoja.molex.cloud/docs/guides/can.html) | `Pamoja.Can` | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
| [I2C, SPI, and GPIO](https://pamoja.molex.cloud/docs/guides/gpio.html) | `Pamoja.Gpio` | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |

The guides, with a worked C# example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
