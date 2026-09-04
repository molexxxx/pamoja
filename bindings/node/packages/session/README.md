# @pamoja/session

X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
npm install @pamoja/session
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/session.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/session.ts):

```typescript
import assert from 'node:assert/strict'

import { AgreementKey, Role, Session } from '@pamoja/session'

// Each device is provisioned with a 32-byte seed and publishes the key it derives. These are
// the X25519 pair RFC 7748 section 6.1 publishes, so the derivation is pinned to the
// specification rather than checked against itself.
const node = new AgreementKey(
  Buffer.from('77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a', 'hex')
)
const gateway = new AgreementKey(
  Buffer.from('5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb', 'hex')
)
assert.equal(
  node.publicKey().toString('hex'),
  '8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a'
)

// Neither side sends the session key. Both derive it from the shared secret, a salt that
// travels in the clear, and both public keys. The roles have to be opposite.
const salt = Buffer.alloc(16, 0x09)
const uplink = new Session(node, gateway.publicKey(), salt, Role.Initiator)
const downlink = new Session(gateway, node.publicKey(), salt, Role.Responder)

// The pump id is authenticated but not encrypted, so a router still reads it while any change
// to it fails the tag.
const label = Buffer.from('pump-3')
const sealed = uplink.seal(Buffer.from('flow=41.2'), label)
assert.notEqual(sealed.ciphertext.toString(), 'flow=41.2')
assert.equal(downlink.open(sealed, label).toString(), 'flow=41.2')

// The anti-replay window refuses a counter it has already accepted, so a frame captured off
// the air and sent again is not delivered a second time.
assert.throws(() => downlink.open(sealed, label))
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-session`](https://crates.io/crates/pamoja-session) | [docs.rs](https://docs.rs/pamoja-session), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html) |
| TypeScript | [`@pamoja/session`](https://www.npmjs.com/package/@pamoja/session) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html) |
| Python | [`pamoja-session`](https://pypi.org/project/pamoja-session/) | [`pamoja.session`](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html) |
| C# | [`Pamoja.Session`](https://www.nuget.org/packages/Pamoja.Session) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.Session.html) |

## Documentation

- [The Secured session guide](https://pamoja.molex.cloud/docs/guides/session.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
