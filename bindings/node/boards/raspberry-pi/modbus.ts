// A Modbus line scan: asks every unit address on an RS485 line, through a USB adapter, which
// devices are there. Wire the adapter's A and B to every device's A and B, join the grounds,
// and set the line's format below to the one the devices use. See docs/guides/modbus.md.

// ANCHOR: example
import { Parity, SerialPort } from '@pamoja/hal'
import { ModbusClient, ModbusClientError } from '@pamoja/modbus'

// A USB RS485 adapter. The kernel names the first one it finds ttyUSB0, or ttyACM0 for an
// adapter that presents itself as a modem.
const PORT = '/dev/ttyUSB0'

async function main(): Promise<void> {
  // The format every device on the line uses, from their manuals: 19200 8E1 is the default the
  // specification sets, and many meters ship at 9600 8N1 instead.
  const settings = { baud: 19200, parity: Parity.Even }
  const port = SerialPort.open(PORT, settings)

  // A device that is there answers within a few milliseconds, so a short response timeout
  // keeps the scan of all 247 addresses under half a minute.
  const client = new ModbusClient(port, { responseTimeoutMs: 100 })

  let found = 0
  for (let unit = 1; unit <= 247; unit += 1) {
    const label = `unit ${String(unit).padStart(3)}`
    // Holding register 0 is a question any device can answer, with its value or with an
    // exception, and either proves the device is there.
    try {
      const [value] = await client.readHoldingRegisters(unit, 0, 1)
      console.log(`${label}  holding register 0 is ${value}`)
      found += 1
    } catch (error) {
      if (!(error instanceof ModbusClientError)) {
        throw error
      }
      if (error.kind === 'Exception') {
        const code = (error.exception ?? 0).toString(16).padStart(2, '0')
        console.log(`${label}  there, and refused register 0 with exception 0x${code}`)
        found += 1
      } else if (!(error.kind === 'Timeout' && error.received === 0)) {
        console.log(`${label}  ${error.message}: check the format and the wiring`)
      }
    }
  }
  console.log(`${found} units answered on ${PORT} at ${SerialPort.describe(settings)}`)
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
// ANCHOR_END: example
