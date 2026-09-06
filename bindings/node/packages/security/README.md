# @pamoja/security

ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_security.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/security.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/security
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/security.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/security.ts):

```typescript
import { DeviceIdentity, fingerprint, verify } from '@pamoja/security'

// The seed is provisioned into the device once and never leaves it. A real one comes from
// the factory or a secure element; any 32 bytes stand in here.
const device = DeviceIdentity.fromSeed(Buffer.alloc(32, 7))

// Only the 32-byte public key travels to the gateway. Its fingerprint is the short form an
// operator reads off a screen to tell one device from another.
const gatewayKey = device.publicKey()
console.log(`device     ${fingerprint(gatewayKey)}`)

// Signing is deterministic, so the same reading always produces the same 64 bytes and there
// is no randomness to get wrong on a microcontroller.
const reading = 'meter-4 1182.750 kWh'
const signature = device.sign(reading)
if (verify(gatewayKey, reading, signature)) {
  console.log(`accepted   ${reading}`)
} else {
  console.log('rejected   a reading the device really did sign, which should never happen')
}

// A digit changed in transit no longer matches what was signed.
const edited = 'meter-4 1082.750 kWh'
if (verify(gatewayKey, edited, signature)) {
  console.log('accepted   an edited reading, which should never happen')
} else {
  console.log(`rejected   ${edited}`)
}

// Nor does the same reading offered under another device's key.
const impostor = DeviceIdentity.fromSeed(Buffer.alloc(32, 90))
if (verify(impostor.publicKey(), reading, signature)) {
  console.log('accepted   an impostor, which should never happen')
} else {
  console.log("rejected   a signature offered under another device's key")
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-security`](https://crates.io/crates/pamoja-security) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_security/index.html), [docs.rs](https://docs.rs/pamoja-security), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-security) |
| TypeScript | [`@pamoja/security`](https://www.npmjs.com/package/@pamoja/security) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_security.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-security) |
| Python | [`pamoja-security`](https://pypi.org/project/pamoja-security/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/security.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-security) |
| C# | [`Pamoja.Security`](https://www.nuget.org/packages/Pamoja.Security) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Security.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-security) |

## Documentation

- [`@pamoja/security` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_security.html), every class, function, and type this package exports.
- [The Device identity guide](https://pamoja.molex.cloud/docs/guides/security.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
