# @pamoja/core

Node.js bindings for the [pamoja](https://github.com/molexxxx/pamoja)
device SDK core, built with [napi-rs](https://napi.rs).

The generated surface is intentionally thin. A hand-written, idiomatic layer is
added on top of it so JavaScript and TypeScript callers get a native-feeling API
while all behavior stays in the Rust core.

## What is here

| Import | Covers |
| --- | --- |
| `@pamoja/core/mqtt` | an MQTT client with async iteration over incoming messages |
| `@pamoja/core/security` | device identity: sign a reading, verify one, label a key |
| `@pamoja/core/codec` | JSON to CBOR and back, and packing samples for a metered link |
| `@pamoja/core/kit` | the helper math: smoothing, PID, thermostat, depletion, geofencing |

`@pamoja/core` re-exports all four, and the generated low-level contract stays
available at `@pamoja/core/raw` for anything the facade does not surface.

```js
const { DeviceIdentity, Smoother, toCbor } = require("@pamoja/core");

const smoother = new Smoother(0.3);
const reading = smoother.update(21.7);

const device = DeviceIdentity.fromSeed(seed);
const payload = toCbor({ c: reading });
const signature = device.sign(payload);
```

## Build

```
npm install
npm run build
npm test
```

`npm test` runs the smoke test and then the cross-language conformance suite,
which asserts the same vectors every other binding does.

`npm run build` compiles the Rust core into a native Node addon and emits
`index.js` and `index.d.ts`. Both are generated artifacts, but they are
committed and drift-checked in CI, so they can never fall behind the Rust
source. `index.js` also carries the package version, so a version bump means
rebuilding and committing it.
