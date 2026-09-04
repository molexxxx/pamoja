# @pamoja/kit

Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
npm install @pamoja/kit
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/kit.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/kit.ts):

```typescript
import assert from 'node:assert/strict'

import { Calibration, Median, Thermostat } from '@pamoja/kit'

// A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA is
// full, so the span is 16 mA and mid-scale is 12 mA, not 10.
const level = Calibration.twoPoint(4, 0, 20, 100)
assert.equal(level.apply(12), 50)
assert.equal(level.apply(4), 0)

// The live zero is what makes a broken loop detectable: 0 mA is off the bottom of the
// scale rather than an empty tank. A median window drops that sample outright, where an
// average would blend a quarter of the range into every reading after it.
assert.equal(level.apply(0), -25)
const filtered = new Median()
let percent = 0
for (const milliamps of [12, 12, 0, 12, 12]) {
  percent = level.apply(filtered.update(milliamps))
  assert.equal(percent, 50)
}

// A refill pump runs when the level falls below the deadband, which is the direction
// heating names; nothing about it is specific to temperature. The deadband stops a level
// sitting on the threshold from chattering the contactor.
const pump = Thermostat.heating(50, 10)
assert.equal(pump.update(percent), false)
assert.equal(pump.update(38), true)
assert.equal(pump.update(45), true)
assert.equal(pump.update(62), false)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-kit`](https://crates.io/crates/pamoja-kit) | [docs.rs](https://docs.rs/pamoja-kit), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html) |
| TypeScript | [`@pamoja/kit`](https://www.npmjs.com/package/@pamoja/kit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html) |
| Python | [`pamoja-kit`](https://pypi.org/project/pamoja-kit/) | [`pamoja.kit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html) |
| C# | [`Pamoja.Kit`](https://www.nuget.org/packages/Pamoja.Kit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.Kit.html) |

## Documentation

- [The Helpers guide](https://pamoja.molex.cloud/docs/guides/kit.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
