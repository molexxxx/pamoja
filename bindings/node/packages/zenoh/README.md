# @pamoja/zenoh

Zenoh key expressions: validity, canonical form, matching, and whether two expressions share or cover keys. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/zenoh.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html)

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

// A key expression names a set of keys. Chunks sit between slashes, `*` stands for exactly one
// chunk, whatever it holds, and `**` for any number of them, including none.
const selects = (pattern: string, key: string): string => {
  const verdict = keyexpr.matches(pattern, key) ? 'covers' : 'misses'
  return `${verdict.padEnd(10)}${pattern} ${verdict} ${key}`
}
console.log(selects('farm/*/power', 'farm/t7/power'))
console.log(selects('farm/*/power', 'farm/substation/power'))
console.log(`${selects('farm/*/power', 'farm/row2/t14/power')}, since * is exactly one chunk`)
console.log(selects('farm/**/power', 'farm/row2/t14/power'))
console.log(`${selects('farm/**/alarm', 'farm/alarm')}, where ** is no chunk at all`)

// `$*` stands for any run of characters inside one chunk, so it selects on part of a name.
console.log(selects('farm/t$*/power', 'farm/t7/power'))
console.log(selects('farm/t$*/power', 'farm/substation/power'))

// One set of keys has one canonical spelling, and a Zenoh session accepts no other.
for (const written of ['farm/*/**/power', 'farm/**/*/power', 'farm/**/**/power']) {
  const canonical = keyexpr.canonize(written)
  if (keyexpr.isCanon(written)) {
    console.log(`canonical ${written}, as written`)
  } else {
    console.log(`rewritten ${written} is spelled ${canonical}`)
  }
}

// Joining places one expression beneath another, and canonizes the seam between them.
for (const [prefix, suffix] of [
  ['farm/t7', 'power'],
  ['farm/**', '*/power'],
]) {
  console.log(`joined    ${prefix} and ${suffix} make ${keyexpr.join(prefix, suffix)}`)
}

// A malformed expression is refused rather than repaired into something plausible.
for (const [written, why] of [
  ['farm//power', 'a chunk is empty'],
  ['farm/t7*/power', '* stands alone in its chunk, or after $'],
  ['farm/t7/power?', '? and # are reserved'],
]) {
  if (!keyexpr.isValid(written) && keyexpr.canonize(written) === null) {
    console.log(`malformed ${written}, since ${why}`)
  }
}
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
