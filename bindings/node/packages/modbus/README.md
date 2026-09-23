# @pamoja/modbus

Modbus RTU for RS485 field devices: a client that polls them over a serial port with the line's timing, simulated devices that answer as real ones do, and the frames with their CRC-16/MODBUS. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/modbus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html)

## Install

```sh
npm install @pamoja/modbus
```

This pulls in `@pamoja/native`, the compiled engine, and `@pamoja/hal`. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/modbus.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/modbus.ts):

```typescript
import { Parity, SerialPort } from '@pamoja/hal'
import {
  BROADCAST,
  ModbusClient,
  ModbusClientError,
  ModbusLine,
  ModbusServer,
} from '@pamoja/modbus'

function word(on: boolean): string {
  return on ? 'on' : 'off'
}

async function main() {
  // The line: 19200 baud, even parity, one stop bit, the default the Modbus specification
  // sets. Eleven bits a character, and 3.5 of them of silence mark where a frame ends.
  const settings = { baud: 19200, parity: Parity.Even }
  const gap = Math.floor(ModbusClient.frameGapNanos(settings) / 1000)
  console.log(
    `line         ${SerialPort.describe(settings)}, ` +
      `${SerialPort.bitsPerCharacter(settings)} bits a character, t3.5 is ${gap} us`,
  )

  // Each device's manual gives its unit address and where its values live. The meter keeps
  // its measurements in input registers from 0: volts in tenths, amps in hundredths, then a
  // fault word. The relay module has four relays as coils 0 to 3, and the tank's low-level
  // float switch as discrete input 0, on while the water is below it.
  const METER = 17
  const PUMP = 18
  const meter = new ModbusServer(METER)
  meter.setInputRegisters(0, [2301, 418, 0])
  const relays = new ModbusServer(PUMP)
  relays.setCoils(0, [false, false, false, false])
  relays.setDiscreteInputs(0, [true])
  const line = new ModbusLine()
  line.attach(meter)
  line.attach(relays)

  // The devices sit on a simulated line. On a gateway the port is
  // SerialPort.open('/dev/ttyUSB0', settings), and nothing after this statement changes.
  const port = line.port(settings)
  const client = new ModbusClient(port)

  // Poll the meter with function 0x04 for three input registers, and scale each one as its
  // manual says.
  const registers = await client.readInputRegisters(METER, 0, 3)
  console.log(
    `meter        ${(registers[0] / 10).toFixed(1)} V, ${(registers[1] / 100).toFixed(2)} A, ` +
      `faults ${registers[2]}`,
  )

  // What that poll cost the line: the request, the reply, and the silence before the request.
  const out = port.written
  const back = port.received
  const lineTime =
    SerialPort.transferMicros(settings, out) + SerialPort.transferMicros(settings, back) + gap
  console.log(
    `poll         ${out} bytes out, ${back} back, ${(lineTime / 1000).toFixed(2)} ms of line time`,
  )

  // Read the float switch, and start the pump on relay 0 when the tank is low.
  const [low] = await client.readDiscreteInputs(PUMP, 0, 1)
  console.log(`tank         low-level switch ${word(low)}`)
  if (low) {
    await client.writeSingleCoil(PUMP, 0, true)
  }
  const states = await client.readCoils(PUMP, 0, 4)
  console.log(`relays       ${states.map(word).join(' ')}`)

  // A broadcast, to unit 0, reaches every device on the line and none answers: here every
  // relay off at once. The client waits out the turnaround so each device has carried it out
  // before the next request.
  await client.writeMultipleCoils(BROADCAST, 0, [false, false, false, false])
  console.log(`broadcast    every relay off, no reply, ${client.turnaroundMs} ms turnaround`)
  const after = await client.readCoils(PUMP, 0, 4)
  console.log(`relays       ${after.map(word).join(' ')}`)

  // The meter keeps its measurements in input registers. Asking for them as holding
  // registers, function 0x03, is the usual mistake with a new device, and the meter refuses it
  // with an exception instead of answering.
  let refused: ModbusClientError | undefined
  try {
    await client.readHoldingRegisters(METER, 0, 3)
  } catch (error) {
    refused = error as ModbusClientError
    console.log(`refused      ${refused.message}`)
  }

  // A unit that is not on the line never answers. The client gives up after its response
  // timeout, one second unless told otherwise, which a simulated line counts instead of
  // sleeping through.
  const before = port.waitedMicros
  let silent: ModbusClientError | undefined
  try {
    await client.readInputRegisters(19, 0, 1)
  } catch (error) {
    silent = error as ModbusClientError
    const waited = Math.floor((port.waitedMicros - before) / 1000)
    console.log(`silent       ${silent.message}, ${waited} ms counted and not slept`)
  }

  return { registers, out, back, low, states, relays, refused, silent }
}

main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-modbus`](https://crates.io/crates/pamoja-modbus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html), [docs.rs](https://docs.rs/pamoja-modbus), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-modbus) |
| TypeScript | [`@pamoja/modbus`](https://www.npmjs.com/package/@pamoja/modbus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-modbus) |
| Python | [`pamoja-modbus`](https://pypi.org/project/pamoja-modbus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-modbus) |
| C# | [`Pamoja.Modbus`](https://www.nuget.org/packages/Pamoja.Modbus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-modbus) |

## Documentation

- [`@pamoja/modbus` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html), every class, function, and type this package exports.
- [The Modbus RTU guide](https://pamoja.molex.cloud/docs/guides/modbus.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
