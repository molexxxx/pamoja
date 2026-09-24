# @pamoja/mavlink

MAVLink v1 and v2 framing, signing, named message fields and enum values, and the mission, command, and offboard protocols. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mavlink.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html)

## Install

```sh
npm install @pamoja/mavlink
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/mavlink.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mavlink.ts):

```typescript
import {
  type MavlinkFrame,
  type MavlinkHeader,
  MavlinkMessage,
  MavlinkParser,
  enumEntry,
  enumNames,
  enumValue,
  fromObject,
  message,
  schemaFor,
} from '@pamoja/mavlink'

const VEHICLE = 1
const AUTOPILOT = 1
const STATION = 255
const PLANNER = 190

// An enum field travels as a number, and the dialect names each number. Printing the name
// keeps a reader from looking up what 2 or 81 means.
const name = (enumeration: string, value: number): string =>
  enumEntry(enumeration, value) ?? String(value)
const header = (systemId: number, componentId: number, sequence: number): MavlinkHeader => ({
  systemId,
  componentId,
  sequence,
})

// Every node broadcasts a heartbeat to say what it is and that it is alive. The frame wraps
// the payload in a header and a checksum seeded with the message's own value.
const heartbeatShape = schemaFor('HEARTBEAT')
const announce = fromObject(heartbeatShape, {
  type: enumValue('MAV_TYPE_GCS'),
  autopilot: enumValue('MAV_AUTOPILOT_INVALID'),
  system_status: enumValue('MAV_STATE_ACTIVE'),
  mavlink_version: 3,
})
const sent = announce.toFrame(header(STATION, PLANNER, 0))
console.log(`sent      ${heartbeatShape.name} in ${sent.bytes.length} bytes`)

// The vehicle answers with its own heartbeat, which reaches the station behind some noise
// and a copy with its last byte flipped in flight.
const vehicle = fromObject(heartbeatShape, {
  type: enumValue('MAV_TYPE_QUADROTOR'),
  autopilot: enumValue('MAV_AUTOPILOT_ARDUPILOTMEGA'),
  base_mode:
    enumValue('MAV_MODE_FLAG_CUSTOM_MODE_ENABLED') |
    enumValue('MAV_MODE_FLAG_STABILIZE_ENABLED') |
    enumValue('MAV_MODE_FLAG_MANUAL_INPUT_ENABLED'),
  system_status: enumValue('MAV_STATE_STANDBY'),
  mavlink_version: 3,
})
const good = vehicle.toFrame(header(VEHICLE, AUTOPILOT, 0))
const garbled = Buffer.from(good.bytes)
garbled[garbled.length - 1] ^= 0xff
const delivered = Buffer.concat([Buffer.from('???'), garbled, good.bytes])

// The parser skips whatever does not start a frame and drops a frame whose checksum fails,
// so only the good copy comes out.
const frames = new MavlinkParser().push(delivered)
console.log(
  `parsed    ${frames.length} frame out of ${delivered.length} bytes,` +
    ' past the noise and the garbled copy',
)
const heard = MavlinkMessage.decode(heartbeatShape, frames[0]!.payload)
console.log(
  `heard     ${name('MAV_TYPE', heard.get('type'))} on` +
    ` ${name('MAV_AUTOPILOT', heard.get('autopilot'))},` +
    ` in ${name('MAV_STATE', heard.get('system_status'))}`,
)

// The base mode is a bitmask, so it names a set of flags rather than one value.
const baseMode = heard.get('base_mode')
console.log(`flags     ${enumNames('MAV_MODE_FLAG', baseMode).join(' | ')}`)
if ((baseMode & enumValue('MAV_MODE_FLAG_SAFETY_ARMED')) === 0) {
  console.log('disarmed  MAV_MODE_FLAG_SAFETY_ARMED is not among them')
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mavlink`](https://crates.io/crates/pamoja-mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html), [docs.rs](https://docs.rs/pamoja-mavlink), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-mavlink) |
| TypeScript | [`@pamoja/mavlink`](https://www.npmjs.com/package/@pamoja/mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-mavlink) |
| Python | [`pamoja-mavlink`](https://pypi.org/project/pamoja-mavlink/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-mavlink) |
| C# | [`Pamoja.Mavlink`](https://www.nuget.org/packages/Pamoja.Mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-mavlink) |

## Documentation

- [`@pamoja/mavlink` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html), every class, function, and type this package exports.
- [The MAVLink guide](https://pamoja.molex.cloud/docs/guides/mavlink.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
