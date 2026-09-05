# @pamoja/session

X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/session.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/session
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/session.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/session.ts):

```typescript
import { AgreementKey, Role, Session } from '@pamoja/session'

// Each device is provisioned with a 32-byte seed and publishes the key it derives. A real
// seed comes from the factory or a secure element; any 32 bytes stand in here.
const node = new AgreementKey(Buffer.alloc(32, 7))
const gateway = new AgreementKey(Buffer.alloc(32, 9))

// Neither side sends the session key. Both derive it from the shared secret, a salt that
// travels in the clear, and both public keys, with opposite roles.
//
// The salt must be fresh for every session: reusing one derives the same key from the same
// pair of devices twice. The initiator draws it and sends it in the clear, so the responder
// uses the salt it received rather than one of its own.
const salt = randomBytes(16)
const uplink = new Session(node, gateway.publicKey(), salt, Role.Initiator)
const downlink = new Session(gateway, node.publicKey(), salt, Role.Responder)
console.log('both sides derived a key without sending one')

// The pump id is authenticated but not encrypted, so a router still reads it while any
// change to it fails the tag.
const reading = Buffer.from('flow=41.2')
const sealed = uplink.seal(reading, Buffer.from('pump-3'))
console.log(`sealed    the reading is no longer readable: ${!sealed.ciphertext.equals(reading)}`)
console.log(`opened    ${downlink.open(sealed, Buffer.from('pump-3')).toString()}`)

// The anti-replay window refuses a counter it has already accepted, so a frame captured
// off the air and sent again is not delivered a second time.
try {
  downlink.open(sealed, Buffer.from('pump-3'))
  console.log('a replayed frame was accepted, which should never happen')
} catch (error) {
  console.log(`replay    refused: ${(error as Error).message}`)
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-session`](https://crates.io/crates/pamoja-session) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html), [docs.rs](https://docs.rs/pamoja-session) |
| TypeScript | [`@pamoja/session`](https://www.npmjs.com/package/@pamoja/session) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html) |
| Python | [`pamoja-session`](https://pypi.org/project/pamoja-session/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html) |
| C# | [`Pamoja.Session`](https://www.nuget.org/packages/Pamoja.Session) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html) |

## Documentation

- [`@pamoja/session` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html), every class, function, and type this package exports.
- [The Secured session guide](https://pamoja.molex.cloud/docs/guides/session.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
