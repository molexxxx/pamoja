// The CAN and J1939 guide example: a standby generator's bus, with the engine controller
// broadcasting its speed, a monitoring gateway keeping only that, a service laptop hearing
// everything, and a coolant sensor speaking plain CAN; see docs/guides/can.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import {
  CanBus,
  type CanFrame,
  NOT_AVAILABLE,
  broadcastJ1939,
  composeJ1939,
  decodeJ1939,
  fdFrame,
  filterPgn,
  frame,
  priority,
  signals,
  signalsFrom,
} from '@pamoja/can'

// The nodes by the address each answers to, and the two parameter groups in play.
const ENGINE = 0
const GATEWAY = 1
const ENGINE_CONTROLLER_1 = 61_444 // carries engine speed
const REQUEST = 59_904 // asks another node for a parameter group

// Where engine speed sits inside that group, and the scale the standard fixes for it.
const ENGINE_SPEED_AT = 3
const RPM_PER_BIT = 0.125

// A reading starts with every signal marked not available, and the engine writes only its
// speed.
function reading(speedId: number, rpm: number): CanFrame {
  const reported = signals()
  reported.setU16(ENGINE_SPEED_AT, rpm / RPM_PER_BIT)
  return frame(speedId, reported.bytes, true)
}

function rpmOf(received: CanFrame): number {
  return (signalsFrom(received.data).u16(ENGINE_SPEED_AT) ?? 0) * RPM_PER_BIT
}

function hex(value: number, digits: number): string {
  return `0x${value.toString(16).toUpperCase().padStart(digits, '0')}`
}

async function main() {
  // J1939 keeps its addressing inside the 29-bit identifier: a priority, the parameter group,
  // and the sender's address. A broadcast names no destination.
  const speedId = broadcastJ1939(priority.control, ENGINE_CONTROLLER_1, ENGINE)
  const speed = decodeJ1939(speedId)!
  console.log(
    `engine speed ${hex(speedId, 8)}: pgn ${speed.pgn} at priority ${speed.priority}, ` +
      `from node ${ENGINE} to every node`,
  )

  const first = reading(speedId, 1500)
  const unreported = [...first.data].filter((byte) => byte === NOT_AVAILABLE).length
  console.log(
    `payload      ${rpmOf(first).toFixed(1)} rpm in bytes ${ENGINE_SPEED_AT + 1} and ` +
      `${ENGINE_SPEED_AT + 2}, the other ${unreported} not available`,
  )

  // Four nodes on one bus with nothing plugged in. On a Linux board each is
  // CanBus.open('can0'), and nothing after this statement changes.
  const engine = CanBus.simulated()
  const gateway = engine.join()
  const laptop = engine.join()
  const sensor = engine.join()

  // The gateway keeps engine speed and nothing else; the laptop keeps everything.
  gateway.setFilters([filterPgn(ENGINE_CONTROLLER_1)])

  // Two engine readings, and between them the coolant sensor, which speaks plain CAN: its
  // level in percent on the 11-bit identifier 0x120.
  await engine.send(first)
  await sensor.send(frame(0x120, Buffer.from([87])))
  await engine.send(reading(speedId, 1512.5))

  // Every node hears every frame but its own, and keeps what its filters pass.
  let kept: CanFrame | null
  while ((kept = await gateway.receive(10)) !== null) {
    const from = decodeJ1939(kept.id, kept.extended)?.source ?? 0
    console.log(`gateway      ${rpmOf(kept).toFixed(1)} rpm from node ${from}`)
  }
  const onTheBus = engine.sent + sensor.sent
  console.log(`gateway      kept ${gateway.received} of the ${onTheBus} frames on the bus`)
  const heard: CanFrame[] = []
  let next: CanFrame | null
  while ((next = await laptop.receive(10)) !== null) {
    heard.push(next)
  }
  const plain = heard.find((received) => decodeJ1939(received.id, received.extended) === null)
  if (plain !== undefined) {
    console.log(
      `laptop       heard ${heard.length}, among them ${hex(plain.id, 3)}, ` +
        'an 11-bit identifier and no J1939 message',
    )
  }

  // A request is addressed rather than broadcast: below the PDU1 limit, eight bits of the
  // identifier name the node it is for.
  const request = decodeJ1939(composeJ1939(priority.default, REQUEST, GATEWAY, ENGINE))!
  console.log(
    `request      pgn ${request.pgn} from node ${request.source} to node ${request.destination}`,
  )

  // The engine goes quiet. A receive waits for a frame up to its timeout; on a simulated bus
  // it resolves at once and counts the wait instead of sleeping through it.
  const before = gateway.waitedMicros
  const quiet = await gateway.receive(500)
  const waited = Math.floor((gateway.waitedMicros - before) / 1000)
  console.log(
    `silent       ${quiet === null ? 0 : 1} frames in ${waited} ms, counted and not slept`,
  )

  // Above eight bytes CAN FD encodes a length in steps, and a classic frame refuses a ninth
  // byte.
  const wide = fdFrame(speedId, new Uint8Array(32), true)
  console.log(`fd           32 bytes travel at data length code ${wide.dlc}`)
  try {
    frame(speedId, new Uint8Array(9), true)
  } catch (error) {
    console.log(`classic      refused nine bytes: ${(error as Error).message}`)
  }

  return { speedId, unreported, gateway, heard, request, quiet, wide }
}

main()
  // ANCHOR_END: example
  .then(check)

function check(seen: Awaited<ReturnType<typeof main>>): void {
  assert.equal(seen.speedId, 0x0cf00400)
  assert.equal(seen.unreported, 6)
  assert.equal(seen.gateway.received, 2)
  assert.equal(seen.heard.length, 3)
  assert.equal(seen.request.destination, ENGINE)
  assert.equal(seen.quiet, null)
  assert.equal(seen.wide.dlc, 13)
  assert.throws(() => frame(seen.speedId, new Uint8Array(9), true))
}
