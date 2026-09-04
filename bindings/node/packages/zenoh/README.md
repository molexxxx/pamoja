# @pamoja/zenoh

Zenoh key expressions: validity, canonical form, and wildcard matching. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
npm install @pamoja/zenoh
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/zenoh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/zenoh.ts):

```typescript
import assert from 'node:assert/strict'

import { keyexpr } from '@pamoja/zenoh'

// A key expression names a set of keys. `*` stands for exactly one chunk, so this
// selects the battery of any node, and not a battery nested deeper.
assert.ok(keyexpr.isValid('fleet/*/battery'))
assert.ok(keyexpr.matches('fleet/*/battery', 'fleet/n7/battery'))
assert.ok(!keyexpr.matches('fleet/*/battery', 'fleet/n7/rack/battery'))

// `**` stands for any number of chunks, including none, which is what a subscription
// covering a whole subtree wants.
assert.ok(keyexpr.matches('fleet/**', 'fleet/n7/rack/battery'))
assert.ok(keyexpr.matches('fleet/**/battery', 'fleet/battery'))

// Two expressions that select the same keys have one canonical form. Comparing or
// routing on the written form would treat these as different subscriptions.
assert.ok(!keyexpr.isCanon('fleet/**/**/battery'))
assert.equal(keyexpr.canonize('fleet/**/**/battery'), 'fleet/**/battery')
assert.ok(keyexpr.isCanon('fleet/**/battery'))

// A malformed expression is rejected rather than canonized into something plausible.
assert.ok(!keyexpr.isValid('fleet//battery'))
assert.equal(keyexpr.canonize('fleet//battery'), null)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-zenoh`](https://crates.io/crates/pamoja-zenoh) | [docs.rs](https://docs.rs/pamoja-zenoh), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html) |
| TypeScript | [`@pamoja/zenoh`](https://www.npmjs.com/package/@pamoja/zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html) |
| Python | [`pamoja-zenoh`](https://pypi.org/project/pamoja-zenoh/) | [`pamoja.zenoh`](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html) |
| C# | [`Pamoja.Zenoh`](https://www.nuget.org/packages/Pamoja.Zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.KeyExpression.html) |

## Documentation

- [The Zenoh keys guide](https://pamoja.molex.cloud/docs/guides/zenoh.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
