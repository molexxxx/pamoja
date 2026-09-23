# @pamoja/session

X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/session.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html)

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
console.log('agreed    both sides derived a key without sending one')

// The pump id is authenticated but not encrypted, so a router still reads it while any
// change to it fails the tag.
const pump = Buffer.from('pump-3')
const reading = Buffer.from('flow=41.2')
const sealed = uplink.seal(reading, pump)
const hidden = sealed.ciphertext.equals(reading) ? 'still' : 'no longer'
console.log(`sealed    counter ${sealed.counter}, and what goes on the wire is ${hidden} the reading`)
console.log(`opened    ${downlink.open(sealed, pump).toString()}`)

// The anti-replay window refuses a counter it has already accepted, so a frame captured
// off the air and sent again is not delivered a second time.
try {
  downlink.open(sealed, pump)
  console.log('a replayed frame was accepted, which should never happen')
} catch (error) {
  console.log(`replay    refused: ${(error as Error).message}`)
}

// A router that rewrites the pump id breaks the tag, so the gateway refuses the frame
// rather than file the reading under the wrong pump. A frame that fails to open leaves its
// counter unused.
const later = uplink.seal(Buffer.from('flow=41.3'), pump)
try {
  downlink.open(later, Buffer.from('pump-4'))
  console.log('a rewritten pump id was accepted, which should never happen')
} catch (error) {
  console.log(`altered   refused: ${(error as Error).message}`)
}

// Radio frames can arrive out of order. The window accepts any counter it has not seen
// among the 64 below the newest, so the frame that was held up still opens.
const newest = uplink.seal(Buffer.from('flow=41.5'), pump)
const first = downlink.open(newest, pump).toString()
const second = downlink.open(later, pump).toString()
console.log(`late      counter ${newest.counter} opened first, then counter ${later.counter}: ${first}, then ${second}`)

// The gateway answers on the same session. Its frames carry the other direction in their
// nonce, so a reply can never be taken for, or replayed as, one from the node.
const order = downlink.seal(Buffer.from('valve=close'), pump)
console.log(`reply     ${uplink.open(order, pump).toString()}, sealed by the gateway and opened by the node`)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-session`](https://crates.io/crates/pamoja-session) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html), [docs.rs](https://docs.rs/pamoja-session), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-session) |
| TypeScript | [`@pamoja/session`](https://www.npmjs.com/package/@pamoja/session) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-session) |
| Python | [`pamoja-session`](https://pypi.org/project/pamoja-session/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-session) |
| C# | [`Pamoja.Session`](https://www.nuget.org/packages/Pamoja.Session) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-session) |

## Documentation

- [`@pamoja/session` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html), every class, function, and type this package exports.
- [The Secured session guide](https://pamoja.molex.cloud/docs/guides/session.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
