# @pamoja/field-io

The wires a gateway actually has: framed serial packets, an RS485 request and the reply it draws, a CAN frame, and the address a chip answers on.

One install for the 4 capabilities of this domain. Each is also its own package, and
`pamoja` is the whole framework in one.

```sh
npm install @pamoja/field-io
```

| Capability | Package | What it covers |
| --- | --- | --- |
| [Serial framing](https://pamoja.molex.cloud/docs/guides/serial.html) | `@pamoja/serial` | SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets |
| [Modbus RTU](https://pamoja.molex.cloud/docs/guides/modbus.html) | `@pamoja/modbus` | Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices |
| [CAN and J1939](https://pamoja.molex.cloud/docs/guides/can.html) | `@pamoja/can` | CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose |
| [I2C, SPI, and GPIO](https://pamoja.molex.cloud/docs/guides/gpio.html) | `@pamoja/gpio` | I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins |

The guides, with a worked TypeScript example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
