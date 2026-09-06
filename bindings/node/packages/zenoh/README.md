# @pamoja/zenoh

Zenoh key expressions: validity, canonical form, and wildcard matching. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/zenoh.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/zenoh
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/zenoh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/zenoh.ts):

```typescript
import { keyexpr } from '@pamoja/zenoh'

// A key expression names a set of keys. `*` stands for exactly one chunk, so this selects
// the battery of any node, and not a battery nested deeper.
const anyNode = 'fleet/*/battery'
for (const key of ['fleet/n7/battery', 'fleet/n7/rack/battery']) {
  console.log(`${anyNode} covers ${key}: ${keyexpr.matches(anyNode, key)}`)
}

// `**` stands for any number of chunks, including none, which is what a subscription
// covering a whole subtree wants.
console.log(`fleet/** covers a nested key: ${keyexpr.matches('fleet/**', 'fleet/n7/rack/battery')}`)
console.log(
  `fleet/**/battery covers fleet/battery: ${keyexpr.matches('fleet/**/battery', 'fleet/battery')}`,
)

// Two expressions that select the same keys have one canonical form. Comparing or routing
// on the written form would treat these as different subscriptions.
const written = 'fleet/**/**/battery'
const canonical = keyexpr.canonize(written)
console.log(`${written} is canonical: ${keyexpr.isCanon(written)}, and canonizes to ${canonical}`)

// A malformed expression is rejected rather than canonized into something plausible.
const malformed = 'fleet//battery'
console.log(
  `${malformed} is valid: ${keyexpr.isValid(malformed)},` +
    ` canonizes to ${keyexpr.canonize(malformed)}`,
)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-zenoh`](https://crates.io/crates/pamoja-zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html), [docs.rs](https://docs.rs/pamoja-zenoh), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-zenoh) |
| TypeScript | [`@pamoja/zenoh`](https://www.npmjs.com/package/@pamoja/zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-zenoh) |
| Python | [`pamoja-zenoh`](https://pypi.org/project/pamoja-zenoh/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-zenoh) |
| C# | [`Pamoja.Zenoh`](https://www.nuget.org/packages/Pamoja.Zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-zenoh) |

## Documentation

- [`@pamoja/zenoh` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html), every class, function, and type this package exports.
- [The Zenoh keys guide](https://pamoja.molex.cloud/docs/guides/zenoh.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
