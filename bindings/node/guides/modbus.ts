// The Modbus RTU guide example: a gateway at a village water pump polls an energy meter and a
// relay module on one RS485 line; see docs/guides/modbus.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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
  // ANCHOR_END: example
  .then(check)

function check(seen: Awaited<ReturnType<typeof main>>): void {
  assert.deepEqual(seen.registers, [2301, 418, 0])
  assert.deepEqual([seen.out, seen.back], [8, 11])
  assert.equal(seen.low, true)
  assert.deepEqual(seen.states, [true, false, false, false])
  assert.equal(seen.relays.coil(0), false)
  assert.ok(seen.refused instanceof ModbusClientError)
  assert.equal(seen.refused.kind, 'Exception')
  assert.equal(seen.silent?.kind, 'Timeout')
  assert.equal(seen.silent?.unit, 19)
}
