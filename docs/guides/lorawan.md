# LoRaWAN

LoRaWAN is the MAC layer a long-range node speaks above a LoRa radio. The band
is public and the range is measured in kilometres, so every frame carries a
message integrity code keyed to the network, proving it is authentic and intact,
and a payload encrypted to the application, readable only by its owner. Both are
AES-128. A device reaches a network with session keys provisioned into it or by
joining over the air, and pamoja runs both ends of that join: the device asking
to be admitted, and the network admitting it. It does not drive the radio or run
a gateway, so the framing is the same whether a concentrator, a network server,
or nothing at all sits underneath it.

## What the example does

It replays a join accept captured off a live EU868 network, from both sides. The
network builds and signs that frame out of the address and radio settings it
grants; a device holding only the root key verifies the frame and activates on
it; and each side then derives the session keys, which are checked against the
pair an independent implementation published for the same capture.

It proves:

- The accept a network signs is byte for byte the frame that was captured, so
  the encryption of the granted settings and the CMAC over them are pinned to a
  third party's numbers rather than to this implementation.
- A device that holds nothing but the root key verifies that frame and reads the
  address `0x26012E43` out of it.
- Neither side transmits a session key, and the keys both sides derive are the
  published pair: a frame the device encrypts is read back by a session built
  from those keys.
- A single byte changed in the accept fails the MIC, so a device does not
  activate on a join it cannot attribute to its own network.

## Rust

<!-- snippet: examples/tests/guides/lorawan.rs#example -->
From [`examples/tests/guides/lorawan.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/lorawan.rs):

```rust
use pamoja_lorawan::{Device, JoinGrant, Session, Uplink};

// A join accept captured off a live EU868 network, the root key it was signed under, and
// the session keys an independent implementation derived from it. Published at
// https://github.com/anthonykirby/lora-packet/issues/10
let captured = [
    0x20, 0x4D, 0xD8, 0x5A, 0xE6, 0x08, 0xB8, 0x7F, 0xC4, 0x88, 0x99, 0x70, 0xB7, 0xD2, 0x04,
    0x2C, 0x9E, 0x72, 0x95, 0x9B, 0x00, 0x57, 0xAE, 0xD6, 0x09, 0x4B, 0x16, 0x00, 0x3D, 0xF1,
    0x2D, 0xE1, 0x45,
];
let app_key = [
    0xB6, 0xB5, 0x3F, 0x4A, 0x16, 0x8A, 0x7A, 0x88, 0xBD, 0xF7, 0xEA, 0x13, 0x5C, 0xE9, 0xCF,
    0xCA,
];
let dev_nonce = 0xCC85;

// The network half: the address and radio settings this network grants, encrypted and
// signed under the root key, are the frame that was captured.
let cflist = [
    0x18, 0x4F, 0x84, 0xE8, 0x56, 0x84, 0xB8, 0x5E, 0x84, 0x88, 0x66, 0x84, 0x58, 0x6E, 0x84,
    0x00,
];
let offer = JoinGrant::new(0x00E5_063A, 0x13, 0x2601_2E43)
    .with_dl_settings(0x03)
    .with_rx_delay(0x01)
    .with_cflist(cflist);
assert_eq!(offer.accept(&app_key, dev_nonce).as_bytes(), &captured[..]);

// The device half. A join accept carries no EUI, so only the root key decides whether it
// verifies.
let node = Device::new([0; 8], [0; 8], app_key);
let accepted = node
    .accept_join(&captured, dev_nonce)
    .expect("the captured accept verifies");
assert_eq!(accepted.dev_addr(), 0x2601_2E43);

// Neither side transmits a session key; both derive it from the two nonces. What the
// device derived is read back by a session holding the keys published with the capture.
let nwk_skey = [
    0x2C, 0x96, 0xF7, 0x02, 0x81, 0x84, 0xBB, 0x0B, 0xE8, 0xAA, 0x49, 0x27, 0x52, 0x90, 0xD4,
    0xFC,
];
let app_skey = [
    0xF3, 0xA5, 0xC8, 0xF0, 0x23, 0x2A, 0x38, 0xC1, 0x44, 0x02, 0x9C, 0x16, 0x58, 0x65, 0x80,
    0x2C,
];
let gateway = Session::new(0x2601_2E43, nwk_skey, app_skey);
let probe = Uplink::new(1, 1, b"real");
let uplink = accepted
    .session()
    .encode_uplink(&probe)
    .expect("a payload that fits one frame");
let rx = gateway
    .decode(uplink.as_bytes(), 1)
    .expect("the MIC verifies under the derived key");
assert_eq!(rx.payload(), b"real");

// A single byte changed in the air fails the MIC, so no one else can admit the device.
let mut forged = captured;
forged[1] ^= 0xFF;
assert!(node.accept_join(&forged, dev_nonce).is_err());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/lorawan.ts#example -->
From [`bindings/node/guides/lorawan.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/lorawan.ts):

```typescript
import assert from 'node:assert/strict'

import { device, grantAccept, session } from '@pamoja/lorawan'

// A join accept captured off a live EU868 network, the root key it was signed under, and
// the session keys an independent implementation derived from it. Published at
// https://github.com/anthonykirby/lora-packet/issues/10
const captured = Buffer.from(
  '204dd85ae608b87fc4889970b7d2042c9e72959b0057aed6094b16003df12de145',
  'hex'
)
const appKey = Buffer.from('b6b53f4a168a7a88bdf7ea135ce9cfca', 'hex')
const devNonce = 0xcc85

// The network half: the address and radio settings this network grants, encrypted and
// signed under the root key, are the frame that was captured.
const offer = {
  appNonce: 0x00e5063a,
  netId: 0x13,
  devAddr: 0x26012e43,
  dlSettings: 0x03,
  rxDelay: 0x01,
  cflist: Buffer.from('184f84e85684b85e84886684586e8400', 'hex'),
}
assert.deepEqual(grantAccept(offer, appKey, devNonce), captured)

// The device half. A join accept carries no EUI, so only the root key decides whether it
// verifies.
const node = device(Buffer.alloc(8), Buffer.alloc(8), appKey)
const accepted = node.acceptJoin(captured, devNonce)
assert.equal(accepted.devAddr, 0x26012e43)

// Neither side transmits a session key; both derive it from the two nonces. What the
// device derived is read back by a session holding the keys published with the capture.
const keys = Buffer.from(
  '2c96f7028184bb0be8aa49275290d4fcf3a5c8f0232a38c144029c165865802c',
  'hex'
)
const gateway = session(0x26012e43, keys.subarray(0, 16), keys.subarray(16))
const uplink = accepted.session().encodeUplink(1, 1, Buffer.from('real'))
assert.equal(gateway.decode(uplink, 1).payload.toString(), 'real')

// A single byte changed in the air fails the MIC, so no one else can admit the device.
const forged = Buffer.from(captured)
forged[1] ^= 0xff
assert.throws(() => node.acceptJoin(forged, devNonce))
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/lorawan.py#example -->
From [`bindings/python/guides/lorawan.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/lorawan.py):

```python
from pamoja.core import PamojaError
from pamoja.lorawan import device, grant, session

# A join accept captured off a live EU868 network, the root key it was signed under, and
# the session keys an independent implementation derived from it. Published at
# https://github.com/anthonykirby/lora-packet/issues/10
captured = bytes.fromhex(
    "204dd85ae608b87fc4889970b7d2042c9e72959b0057aed6094b16003df12de145"
)
app_key = bytes.fromhex("b6b53f4a168a7a88bdf7ea135ce9cfca")
dev_nonce = 0xCC85

# The network half: the address and radio settings this network grants, encrypted and
# signed under the root key, are the frame that was captured.
offer = grant(
    app_nonce=0x00E5063A, net_id=0x13, dev_addr=0x26012E43, dl_settings=0x03,
    rx_delay=0x01, cflist=bytes.fromhex("184f84e85684b85e84886684586e8400"),
)
assert offer.accept(app_key, dev_nonce) == captured

# The device half. A join accept carries no EUI, so only the root key decides whether it
# verifies.
node = device(bytes(8), bytes(8), app_key)
accepted = node.accept_join(captured, dev_nonce)
assert accepted.dev_addr == 0x26012E43

# Neither side transmits a session key; both derive it from the two nonces. What the
# device derived is read back by a session holding the keys published with the capture.
keys = bytes.fromhex("2c96f7028184bb0be8aa49275290d4fcf3a5c8f0232a38c144029c165865802c")
gateway = session(0x26012E43, keys[:16], keys[16:])
uplink = accepted.session().encode_uplink(1, 1, b"real")
assert gateway.decode(uplink, 1).payload == b"real"

# A single byte changed in the air fails the MIC, so no one else can admit the device.
forged = bytearray(captured)
forged[1] ^= 0xFF
try:
    node.accept_join(bytes(forged), dev_nonce)
except PamojaError:
    pass
else:
    raise AssertionError("a join accept nobody signed should not activate a session")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs):

```csharp
// A join accept captured off a live EU868 network, the root key it was signed
// under, and the session keys an independent implementation derived from it.
// Published at https://github.com/anthonykirby/lora-packet/issues/10
byte[] captured = Convert.FromHexString(
    "204dd85ae608b87fc4889970b7d2042c9e72959b0057aed6094b16003df12de145");
byte[] appKey = Convert.FromHexString("b6b53f4a168a7a88bdf7ea135ce9cfca");
const ushort devNonce = 0xCC85;

// The network half: the address and radio settings this network grants, encrypted
// and signed under the root key, are the frame that was captured.
var offer = new LorawanGrant(
    appNonce: 0x00E5063A,
    netId: 0x13,
    devAddr: 0x26012E43,
    dlSettings: 0x03,
    rxDelay: 0x01,
    cflist: Convert.FromHexString("184f84e85684b85e84886684586e8400"));
Expect(
    offer.Accept(appKey, devNonce).SequenceEqual(captured),
    "the join accept this network signs is the frame that was captured");

// The device half. A join accept carries no EUI, so only the root key decides
// whether it verifies.
using var node = new LorawanDevice(new byte[8], new byte[8], appKey);
using LorawanJoinAccept accepted = node.AcceptJoin(captured, devNonce);
Expect(accepted.DevAddr == 0x26012E43, "the device takes the address it was granted");

// Neither side transmits a session key; both derive it from the two nonces. What
// the device derived is read back by a session holding the published keys.
byte[] keys = Convert.FromHexString(
    "2c96f7028184bb0be8aa49275290d4fcf3a5c8f0232a38c144029c165865802c");
using var gateway = new LorawanSession(0x26012E43, keys.AsSpan(0, 16), keys.AsSpan(16));
using LorawanSession activated = accepted.Session();
byte[] uplink = activated.EncodeUplink(1, 1, "real"u8);
Expect(
    gateway.Decode(uplink, 1).Payload.AsSpan().SequenceEqual("real"u8),
    "the network reads what the device it just admitted wrote");

// A single byte changed in the air fails the MIC, so no one else can admit the
// device.
byte[] forged = [.. captured];
forged[1] ^= 0xFF;
bool refused = false;
try
{
    using LorawanJoinAccept _ = node.AcceptJoin(forged, devNonce);
}
catch (PamojaException)
{
    refused = true;
}
Expect(refused, "a join accept nobody signed does not activate a session");
```
<!-- end -->

## Reference

<!-- table: reference lorawan -->
- Rust: [`pamoja-lorawan`](https://docs.rs/pamoja-lorawan) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html))
- TypeScript: [`@pamoja/lorawan`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html)
- Python: [`pamoja.lorawan`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html)
- C#: [`Lorawan`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.Lorawan.html), [`LorawanDevice`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.LorawanDevice.html), [`LorawanSession`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.LorawanSession.html), [`LorawanHeader`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.LorawanHeader.html), [`LorawanJoinRequest`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.LorawanJoinRequest.html), [`LorawanJoinAccept`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.LorawanJoinAccept.html), [`LorawanGrant`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.LorawanGrant.html), [`LorawanOptions`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.LorawanOptions.html), [`LorawanRxData`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.LorawanRxData.html), [`LorawanMessageType`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.LorawanMessageType.html), [`LorawanDirection`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.LorawanDirection.html)
<!-- end -->
