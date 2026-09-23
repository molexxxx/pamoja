# @pamoja/can

CAN 2.0 and CAN FD frames with 11- and 29-bit identifiers, J1939 decode and compose, and a node on a bus, simulated or a Linux interface through SocketCAN. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/can.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html)

## Install

```sh
npm install @pamoja/can
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/can.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/can.ts):

```typescript
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-can`](https://crates.io/crates/pamoja-can) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html), [docs.rs](https://docs.rs/pamoja-can), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-can) |
| TypeScript | [`@pamoja/can`](https://www.npmjs.com/package/@pamoja/can) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-can) |
| Python | [`pamoja-can`](https://pypi.org/project/pamoja-can/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-can) |
| C# | [`Pamoja.Can`](https://www.nuget.org/packages/Pamoja.Can) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-can) |

## Documentation

- [`@pamoja/can` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html), every class, function, and type this package exports.
- [The CAN and J1939 guide](https://pamoja.molex.cloud/docs/guides/can.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
