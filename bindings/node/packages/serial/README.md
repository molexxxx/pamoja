# @pamoja/serial

SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/serial.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/serial
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/serial.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/serial.ts):

```typescript
import {
  COBS_DELIMITER_BYTE,
  SLIP_END_BYTE,
  SLIP_ESC_BYTE,
  SlipDecoder,
  cobs,
  slip,
} from '@pamoja/serial'

// A UART carries bytes, not packets, so a framing has to mark where one packet ends.
// SLIP reserves two byte values for that, and the package names both: the end byte closes
// a frame, the escape byte carries a value that would otherwise look like one.
const payload = Buffer.concat([Buffer.from('lvl='), Buffer.from([SLIP_END_BYTE, SLIP_ESC_BYTE])])
const framed = slip.encode(payload)
console.log(`slip      ${payload.length} payload bytes framed as ${framed.length}`)

// Decoding gives the payload back unchanged, reserved bytes and all.
const restored = slip.decode(framed)
console.log(`slip      decoded back to ${restored.length} bytes`)

// COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
// run led by its own length, so a frame never grows by more than a byte per 254. Zero is
// the delimiter, and never appears inside a frame.
const packet = Buffer.concat([Buffer.from('lvl='), Buffer.from([COBS_DELIMITER_BYTE]), Buffer.from('7')])
const cobsFramed = cobs.encode(packet)
console.log(`cobs      ${packet.length} payload bytes framed as ${cobsFramed.length}`)

// A read from a port returns whatever arrived, which is rarely one whole frame. This
// chunk holds two good frames with a truncated one between them; the decoder hands over
// the good ones and discards only the bad frame.
const decoder = new SlipDecoder()
const chunk = Buffer.concat([
  Buffer.from('ok'),
  Buffer.from([SLIP_END_BYTE]),
  Buffer.from([SLIP_ESC_BYTE]), // a frame that ends before its escape pair completes
  Buffer.from([SLIP_END_BYTE]),
  Buffer.from('go'),
  Buffer.from([SLIP_END_BYTE]),
])
const frames = decoder.feed(chunk)
for (const frame of frames) {
  console.log(`received  ${frame.toString()}`)
}
console.log(`discarded ${decoder.discarded} frame the stream mangled`)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-serial`](https://crates.io/crates/pamoja-serial) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html), [docs.rs](https://docs.rs/pamoja-serial), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-serial) |
| TypeScript | [`@pamoja/serial`](https://www.npmjs.com/package/@pamoja/serial) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-serial) |
| Python | [`pamoja-serial`](https://pypi.org/project/pamoja-serial/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-serial) |
| C# | [`Pamoja.Serial`](https://www.nuget.org/packages/Pamoja.Serial) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-serial) |

## Documentation

- [`@pamoja/serial` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html), every class, function, and type this package exports.
- [The Serial framing guide](https://pamoja.molex.cloud/docs/guides/serial.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
