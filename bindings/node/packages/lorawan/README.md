# @pamoja/lorawan

LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/lorawan.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/lorawan
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/lorawan.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/lorawan.ts):

```typescript
import { device, grantAccept, grantSession } from '@pamoja/lorawan'

// The root key is provisioned into the device at the factory and known to the network
// server. It is the only secret either side starts with; any 16 bytes stand in here.
const appKey = Buffer.alloc(16, 7)

// The device asks to join with a nonce it has not used before, which is what stops an old
// accept being replayed at it.
const devNonce = 1
const node = device(Buffer.alloc(8), Buffer.alloc(8), appKey)

// The network grants the join. It draws its own nonce, names the network the device is
// joining, and assigns the address the device will answer to from then on.
const devAddr = 0x26012e43
const offer = { appNonce: 2, netId: 19, devAddr }
const accept = grantAccept(offer, appKey, devNonce)
console.log(`granted   address 0x${devAddr.toString(16).toUpperCase()} in a ${accept.length}-byte accept`)

// The device verifies it against the root key. A join accept carries no device identifier,
// so only that key decides whether it is for this device.
const joined = node.acceptJoin(accept, devNonce)
console.log(`joined    the device took address 0x${joined.devAddr.toString(16).toUpperCase()}`)

// Neither side transmits a session key. Both derive the same pair from the root key and the
// two nonces, so the network reads what the device sends without ever having been told how.
const network = grantSession(offer, appKey, devNonce)
const uplink = joined.session().encodeUplink(1, 1, Buffer.from('level=high'))
const received = network.decode(uplink, 1)
console.log(`uplink    the network read ${received.payload.toString()}`)

// A single byte changed in the air fails that check, so no one else can admit the device or
// put words in its mouth.
const forged = Buffer.from(accept)
forged[1] ^= 0xff
try {
  node.acceptJoin(forged, devNonce)
  console.log('a forged accept was taken, which should never happen')
} catch (error) {
  console.log(`forged    accept refused: ${(error as Error).message}`)
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-lorawan`](https://crates.io/crates/pamoja-lorawan) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html), [docs.rs](https://docs.rs/pamoja-lorawan), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-lorawan) |
| TypeScript | [`@pamoja/lorawan`](https://www.npmjs.com/package/@pamoja/lorawan) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-lorawan) |
| Python | [`pamoja-lorawan`](https://pypi.org/project/pamoja-lorawan/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-lorawan) |
| C# | [`Pamoja.Lorawan`](https://www.nuget.org/packages/Pamoja.Lorawan) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-lorawan) |

## Documentation

- [`@pamoja/lorawan` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html), every class, function, and type this package exports.
- [The LoRaWAN guide](https://pamoja.molex.cloud/docs/guides/lorawan.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
