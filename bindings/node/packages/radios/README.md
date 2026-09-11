# @pamoja/radios

The Semtech SX126x and SX127x LoRa radios: their commands, registers, and decoders, the amplifier setting a regional EIRP ceiling allows, and a duty-cycle guard. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/radios.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_radios.html)

## Install

```sh
npm install @pamoja/radios
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/radios.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/radios.ts):

```typescript
import { LoraRegion, airtimeUs, linkBudget, planFor } from '@pamoja/lora'
import { DutyCycle, sx126x } from '@pamoja/radios'

const hex = (bytes: Uint8Array): string =>
  Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join(' ')

// An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz, through a
// 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP there, and the antenna
// and pigtail decide how hard the amplifier may drive under that cap.
const eu868 = planFor(LoraRegion.Eu868)
const frequency = 868_100_000
const link = eu868.linkSettings(3)!
const whip = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
const ceiling = eu868.maxEirpDbm(frequency)
const power = sx126x.txPowerUnderCeiling(sx126x.Amplifier.HighPower, whip, ceiling)
console.log(`power     ${power.settingDbm} dBm under a ${ceiling} dBm EIRP ceiling`)

// The commands in the order section 14.2 of the datasheet gives, each sent in its own SPI
// transaction once BUSY is low. The chip gives up on the frame a second after its airtime.
const airtime = airtimeUs(link, 10)
const events = sx126x.Irq.TxDone | sx126x.Irq.Timeout
const commands: [string, Uint8Array][] = [
  ['standby', sx126x.setStandby()],
  ['packet type', sx126x.setPacketTypeLora()],
  ['frequency', sx126x.setRfFrequency(frequency)],
  ['pa config', sx126x.setPaConfig(power)],
  ['tx params', sx126x.setTxParams(power, 40)],
  ['modulation', sx126x.setLoraModulationParams(link)],
  ['packet', sx126x.setLoraPacketParams(link, 10, false)],
  ['irq', sx126x.setDioIrqParams(events, events)],
  ['tx', sx126x.setTx(airtime + 1_000_000)],
]
for (const [name, bytes] of commands) {
  console.log(`${name.padEnd(12)}${hex(bytes)}`)
}

// Once the frame has left, GetIrqStatus answers with TxDone, and the status byte shows the
// chip back in standby.
const irq = sx126x.irq(Buffer.from([0x00, 0x01]))
const sent = (irq & sx126x.Irq.TxDone) !== 0
const timedOut = (irq & sx126x.Irq.Timeout) !== 0
console.log(`sent      tx done ${sent}, timed out ${timedOut}`)
const status = sx126x.status(0x2c)
console.log(`status    ${status.chipMode}, ${status.commandStatus}`)

// A frame that arrives later comes with the signal levels it was heard at.
const heard = sx126x.packetStatus(Buffer.from([0xdb, 0xf6, 0xe0]))
console.log(`received  RSSI ${heard.rssiDbm} dBm, SNR ${heard.snrDb} dB`)

// The sub-band that holds 868.1 MHz allows 1% of the time, so the frame's airtime buys
// ninety-nine times as long in silence before the next.
const guard = new DutyCycle(eu868.dutyCyclePermille(frequency)!)
const held = guard.transmitted(0, link, 10)
console.log(`airtime   ${held} us, next frame after ${guard.waitUs(0)} us`)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-radios`](https://crates.io/crates/pamoja-radios) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_radios/index.html), [docs.rs](https://docs.rs/pamoja-radios), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-radios) |
| TypeScript | [`@pamoja/radios`](https://www.npmjs.com/package/@pamoja/radios) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_radios.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-radios) |
| Python | [`pamoja-radios`](https://pypi.org/project/pamoja-radios/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/radios.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-radios) |
| C# | [`Pamoja.Radios`](https://www.nuget.org/packages/Pamoja.Radios) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Radios.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-radios) |

## Documentation

- [`@pamoja/radios` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_radios.html), every class, function, and type this package exports.
- [The LoRa radios guide](https://pamoja.molex.cloud/docs/guides/radios.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
