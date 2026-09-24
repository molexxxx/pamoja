# @pamoja/power

Duty cycling and an energy-aware governor that stretches work as the battery drains and holds its mode against a wandering charge. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/power.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html)

## Install

```sh
npm install @pamoja/power
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/power.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/power.ts):

```typescript
import { DutyCycle, PowerMode, PowerPlan } from '@pamoja/power'

// A solar node samples every minute while the charge is healthy, stretches to ten minutes
// to conserve, and to an hour once the battery is nearly flat. Durations cross the binding
// as microseconds.
const plan = new PowerPlan(60_000_000, 600_000_000, 3_600_000_000)

// The default thresholds enter saver mode below 50% charge and critical below 20%.
for (const charge of [0.8, 0.35, 0.12]) {
  const every = plan.intervalUs(charge) / 1_000_000
  console.log(
    `at ${(charge * 100).toFixed(0)}% charge: ${plan.mode(charge)}, sampling every ${every}s`,
  )
}

// A panel that is delivering buys back one mode. The interval for a charge knows nothing of
// the panel, so the cadence comes from the mode the panel bought.
const charging = plan.modeWhileCharging(0.12, true)
const chargingEvery = plan.intervalForUs(charging) / 1_000_000
console.log(`at 12% charge while charging: ${charging}, sampling every ${chargingEvery}s`)

// A charge worked out from a fuel gauge that did not answer is not a number. The plan takes
// it as critical, so a node that cannot tell what it has left does the least until it can.
const unknown = Number.NaN
const unknownEvery = plan.intervalUs(unknown) / 1_000_000
console.log(
  `with no reading from the gauge: ${plan.mode(unknown)}, sampling every ${unknownEvery}s`,
)

// The thresholds say how long the battery must carry the node without sun. Winter nights
// are long, so a winter plan starts saving sooner and goes critical sooner.
const winter = plan.withThresholds(0.7, 0.3)
const saver = (winter.saverBelow * 100).toFixed(0)
const critical = (winter.criticalBelow * 100).toFixed(0)
console.log(`the winter plan saves below ${saver}% and goes critical below ${critical}%`)
const cold = winter.mode(0.6)
const mild = plan.mode(0.6)
console.log(`at 60% charge: ${cold} in winter, ${mild} by default`)

// A fuel gauge wanders a point or two between readings, so a charge sitting at a threshold
// would change the cadence on every cycle. `nextMode` takes the mode the node is in: it drops
// as soon as the charge falls below a threshold, and climbs back only once the charge is the
// plan's hysteresis margin clear of it.
const wandering = [0.49, 0.51, 0.5, 0.53, 0.48, 0.52]
const walk = (governor: PowerPlan): string => {
  let mode: PowerMode = PowerMode.Active
  const modes: string[] = []
  for (const charge of wandering) {
    mode = governor.nextMode(mode, charge)
    modes.push(mode)
  }
  return modes.join(', ')
}
const flapping = walk(plan.withHysteresis(0))
console.log(`a charge wandering around 50% with no margin: ${flapping}`)
const margin = (plan.hysteresis * 100).toFixed(0)
const settled = walk(plan)
console.log(`and with the ${margin} point margin: ${settled}`)
const back = plan.nextMode(PowerMode.Saver, 0.56)
console.log(`at 56% the charge has cleared the margin: ${back}`)

// The work is the same two seconds whichever mode the node is in; stretching the cycle is
// what saves the energy. The duty fraction is the proxy for average draw, so the hourly
// cadence costs a sixtieth of what the one-minute cadence does.
const awakeUs = 2_000_000
const healthy = new DutyCycle(awakeUs, plan.intervalUs(0.8) - awakeUs)
const flat = new DutyCycle(awakeUs, plan.intervalUs(0.12) - awakeUs)
console.log(`awake ${(healthy.fraction * 100).toFixed(2)}% of the time when healthy`)
console.log(`awake ${(flat.fraction * 100).toFixed(3)}% of the time when flat`)

// A node that lives on its panel can stay awake for the share of the time the harvest pays
// for. Asleep it draws next to nothing, so that share is the harvest over what it draws
// awake, and the duty cycle turns it into time.
const minuteUs = 60_000_000
const awakeMw = 120
const cloudy = DutyCycle.fromFraction(minuteUs, 12 / awakeMw)
console.log(`a 12 mW harvest pays for ${cloudy.activeUs / 1000}ms awake in each minute`)

// The share is clamped, so a harvest above the draw keeps the node awake throughout, and a
// harvest the meter could not read keeps it asleep until one can be.
const sunny = DutyCycle.fromFraction(minuteUs, 150 / awakeMw)
const unread = DutyCycle.fromFraction(minuteUs, Number.NaN)
console.log(`a 150 mW harvest keeps it awake all ${sunny.activeUs / 1000}ms`)
console.log(`an unread harvest keeps it asleep all ${unread.sleepUs / 1000}ms`)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-power`](https://crates.io/crates/pamoja-power) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_power/index.html), [docs.rs](https://docs.rs/pamoja-power), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-power) |
| TypeScript | [`@pamoja/power`](https://www.npmjs.com/package/@pamoja/power) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-power) |
| Python | [`pamoja-power`](https://pypi.org/project/pamoja-power/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/power.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-power) |
| C# | [`Pamoja.Power`](https://www.nuget.org/packages/Pamoja.Power) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-power) |

## Documentation

- [`@pamoja/power` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html), every class, function, and type this package exports.
- [The Power guide](https://pamoja.molex.cloud/docs/guides/power.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
