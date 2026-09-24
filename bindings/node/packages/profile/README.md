# @pamoja/profile

Named, ready-to-run device profiles from plain data or a JSON manifest. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/profile.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html)

## Install

```sh
npm install @pamoja/profile
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
import { Profile, Viz } from '@pamoja/profile'

// A profile is plain data, so a fleet ships one as a file rather than as code. This
// manifest names no battery thresholds, so the documented defaults apply.
const manifest = `{
  "name": "brooder-heater",
  "topic": "poultry/brooder/temperature",
  "control": {
    "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
    "cooling": false, "safe_band": 4.0
  },
  "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
}`
const profile = Profile.fromJson(manifest)
console.log(`profile   ${profile.name} reports on ${profile.topic}`)
console.log(
  `defaults  the file names no battery thresholds, so saver starts below ${(profile.power.saverBelow * 100).toFixed(0)}% and critical below ${(profile.power.criticalBelow * 100).toFixed(0)}%`,
)

// The schedule becomes a power plan, which says what mode a charge puts the node in and
// how long it waits between samples there, in microseconds.
const plan = profile.powerPlan()
for (const charge of [0.8, 0.3, 0.1]) {
  console.log(
    `battery   at ${(charge * 100).toFixed(0)}% it runs ${plan.mode(charge)} and samples every ${plan.intervalUs(charge) / 1_000_000} s`,
  )
}

// One controller runs for the life of the node, because it remembers whether the lamp is
// on. The lamp switches on at 31.5 C or below and off at 32.5 C or above, the setpoint
// less and plus the hysteresis, and in between it stays as it was. A reading more than
// 4 C from the setpoint raises an alert as well.
const controller = profile.controller()
let lamp = false
for (const reading of [27.5, 31.8, 32.6, 32.1, 31.4]) {
  const reaction = controller.evaluate(reading)
  const on = reaction.actuator === true
  const change = on ? (lamp ? 'lamp stays on' : 'lamp on') : lamp ? 'lamp off' : 'lamp stays off'
  const alert = reaction.alert ? `, alert ${reaction.alert.kind}` : ''
  console.log(`${`${reading} C`.padEnd(10)}${change}${alert}`)
  lamp = on
}

// Written back out, the manifest names the thresholds the file left to their defaults,
// so the next reader has nothing to infer, and it loads as the same profile.
const shared = profile.toJson()
if (shared.includes('saver_below') && Profile.fromJson(shared).toJson() === shared) {
  console.log('shared    written back out, it names saver_below and loads as the same profile')
}

// The manifest also carries how a dashboard draws the node: one element here, the
// brooder's temperature on a thermometer with the band the chicks are safe in.
const drawn = profile.withPresentation({
  elements: [
    {
      key: 'brooder_temperature',
      unit: 'celsius',
      label: 'Brooder temperature',
      viz: Viz.Thermometer,
      band: [28, 36],
    },
  ],
})
const element = drawn.presentation?.elements[0]
console.log(
  `draws     ${element?.key} in ${element?.unit} on a ${element?.viz}, safe from ${element?.band?.[0]} to ${element?.band?.[1]}`,
)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-profile`](https://crates.io/crates/pamoja-profile) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [docs.rs](https://docs.rs/pamoja-profile), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-profile) |
| TypeScript | [`@pamoja/profile`](https://www.npmjs.com/package/@pamoja/profile) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-profile) |
| Python | [`pamoja-profile`](https://pypi.org/project/pamoja-profile/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-profile) |
| C# | [`Pamoja.Profile`](https://www.nuget.org/packages/Pamoja.Profile) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-profile) |

## Documentation

- [`@pamoja/profile` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), every class, function, and type this package exports.
- [The Device profiles guide](https://pamoja.molex.cloud/docs/guides/profile.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
