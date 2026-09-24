# @pamoja/radios

The Semtech SX126x and SX127x LoRa radios and the SX1302 and SX1303 gateway concentrators: their commands, registers, and decoders, the amplifier setting a regional EIRP ceiling allows, a duty-cycle guard, and simulated chips that stand in for a module. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
import { LoraRegion, linkBudget, planFor } from '@pamoja/lora'
import { DutyCycle, SimulatedLoraChip, sx126x } from '@pamoja/radios'

async function onTheBench() {
  // An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz, through a
  // 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP there, and the
  // antenna and pigtail decide how hard the amplifier may drive under that cap.
  const eu868 = planFor(LoraRegion.Eu868)
  const frequency = 868_100_000
  const link = eu868.linkSettings(3)!
  const whip = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
  const ceiling = eu868.maxEirpDbm(frequency)
  const power = sx126x.txPowerUnderCeiling(sx126x.Amplifier.HighPower, whip, ceiling)
  console.log(`power     ${power.settingDbm} dBm under a ${ceiling} dBm EIRP ceiling`)

  // A simulated SX1262 stands in for the chip on the node's board, driven by the same code
  // that drives a real one, and it reports what that code told it.
  const chip = SimulatedLoraChip.sx126x({ amplifier: sx126x.Amplifier.HighPower })
  const radio = chip.radio()
  await radio.configure({ frequencyHz: frequency, link, outputDbm: power.settingDbm })
  const tuned = chip.tuning()
  console.log(
    `tuned     ${(tuned.frequencyHz / 1e6).toFixed(1)} MHz, SF${tuned.link.spreadingFactor} ` +
      `at ${tuned.link.bandwidthHz / 1000} kHz, ${tuned.outputDbm} dBm`,
  )

  // The reading goes out, and the airtime comes back for the duty-cycle guard. The sub-band
  // that holds 868.1 MHz allows 1% of the time, so the frame buys ninety-nine times as long
  // in silence before the next.
  const reading = Buffer.from('level=0.42')
  const airtime = await radio.transmit(reading)
  console.log(`sent      ${chip.sent()[0].payload.length} bytes, ${airtime} us on air`)
  const guard = new DutyCycle(eu868.dutyCyclePermille(frequency)!)
  guard.transmitted(0, link, reading.length)
  console.log(`silence   the next frame starts ${guard.waitUs(0)} us after this one did`)

  // A gateway's answer arrives from the edge of range, 2.5 dB under the noise.
  chip.hear(Buffer.from('ack'), -109, -2.5)
  const heard = await radio.receive(1_000_000)
  if (heard.outcome === 'Frame') {
    console.log(
      `received  ${heard.payload!.toString()} at ${heard.rssiDbm!.toFixed(2)} dBm, ` +
        `SNR ${heard.snrDb!.toFixed(2)} dB`,
    )
  }

  // With nothing on the air the reception times out, and a frame whose CRC fails is dropped
  // rather than handed over.
  const quiet = (await radio.receive(1_000_000)).outcome
  chip.hearCorrupt(-121, -12)
  const broken = (await radio.receive(1_000_000)).outcome
  console.log(`then      ${quiet}, then ${broken}`)
  radio.close()
  return { power, tuned, airtime, link, reading, guard, chip, quiet, broken }
}
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
