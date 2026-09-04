# @pamoja/profile

Named, ready-to-run device profiles from plain data or a JSON manifest. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/profile.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/profile
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
import assert from 'node:assert/strict'

import { AlertKind, ControlKind, Profile } from '@pamoja/profile'

// A profile is plain data, so a fleet ships one as a file rather than as code. The two
// power thresholds are optional and fall back to the documented defaults.
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
assert.equal(profile.name, 'brooder-heater')
assert.equal(profile.topic, 'poultry/brooder/temperature')
assert.equal(profile.control.kind, ControlKind.Setpoint)
assert.equal(profile.control.setpoint, 32.0)
assert.equal(profile.control.cooling, false)
assert.equal(profile.power.activeSecs, 120)
assert.equal(profile.power.saverBelow, 0.5)

// The manifest is the whole control loop. At 27.5 C the reading is below the deadband, so
// the lamp switches on, and it is more than 4 C from target, so the chicks are cold.
const reaction = profile.controller().evaluate(27.5)
assert.equal(reaction.actuator, true)
assert.equal(reaction.alert?.kind, AlertKind.OutOfRange)
assert.equal(reaction.alert?.reading, 27.5)

// Serializing writes the defaulted fields out in full, so a profile edited on a device and
// shared back carries no value the next reader has to infer.
const shared = profile.toJson()
assert.ok(shared.includes('"saver_below"'))
assert.equal(Profile.fromJson(shared).control.setpoint, 32.0)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-profile`](https://crates.io/crates/pamoja-profile) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [docs.rs](https://docs.rs/pamoja-profile) |
| TypeScript | [`@pamoja/profile`](https://www.npmjs.com/package/@pamoja/profile) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html) |
| Python | [`pamoja-profile`](https://pypi.org/project/pamoja-profile/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html) |
| C# | [`Pamoja.Profile`](https://www.nuget.org/packages/Pamoja.Profile) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html) |

## Documentation

- [`@pamoja/profile` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), every class, function, and type this package exports.
- [The Device profiles guide](https://pamoja.molex.cloud/docs/guides/profile.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
