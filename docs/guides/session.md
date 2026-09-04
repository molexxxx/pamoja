# Secured session

Two devices that already hold each other's public key agree a session key without
either of them sending it, then exchange messages that are encrypted, authenticated,
and accepted only once. The primitives are X25519 key agreement from RFC 7748,
HKDF-SHA256 from RFC 5869 to bind the key to that pair of devices and that session,
and ChaCha20-Poly1305 from RFC 8439, which is the cheaper choice on hardware with no
AES acceleration. There is no TLS stack, no certificate chain, and no handshake to
run: establishing a session is deterministic given the two keys and a salt, and every
operation works on buffers the caller owns, so the same code runs on a microcontroller.

Key agreement gives a private channel, not an identified one. The peer's public key
has to be authenticated out of band, by pinning it at provisioning time or by having
it signed with the peer's device identity, or the channel is confidential but open to
a man in the middle.

## What the example does

It provisions a node and its gateway with the X25519 key pair RFC 7748 section 6.1
publishes, and checks the public key the seed derives against the one the
specification fixes. Both sides then establish a session from a salt that travels in
the clear, the node seals a flow reading with the pump id as associated data, and the
gateway opens it. Finally the same frame is offered a second time.

It proves:

- The seed derives the public key the specification publishes, so an implementation
  that is wrong but self-consistent still fails.
- Both sides reach the same key from opposite roles, neither of them having sent it.
- The reading is encrypted: what leaves the node is not the plaintext, and the
  gateway recovers it exactly.
- A frame the gateway has already accepted is refused when it arrives again, so a
  message captured off the air cannot be delivered twice.

## Rust

<!-- snippet: examples/tests/guides/session.rs#example -->
From [`examples/tests/guides/session.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/session.rs):

```rust
use pamoja_session::{AgreementKey, Role, Session};

// Each device is provisioned with a 32-byte seed and publishes the key it derives. These
// are the X25519 pair RFC 7748 section 6.1 publishes, so the derivation is pinned to the
// specification rather than checked against itself.
let node = AgreementKey::from_seed(&[
    0x77, 0x07, 0x6D, 0x0A, 0x73, 0x18, 0xA5, 0x7D, 0x3C, 0x16, 0xC1, 0x72, 0x51, 0xB2, 0x66,
    0x45, 0xDF, 0x4C, 0x2F, 0x87, 0xEB, 0xC0, 0x99, 0x2A, 0xB1, 0x77, 0xFB, 0xA5, 0x1D, 0xB9,
    0x2C, 0x2A,
]);
let gateway = AgreementKey::from_seed(&[
    0x5D, 0xAB, 0x08, 0x7E, 0x62, 0x4A, 0x8A, 0x4B, 0x79, 0xE1, 0x7F, 0x8B, 0x83, 0x80, 0x0E,
    0xE6, 0x6F, 0x3B, 0xB1, 0x29, 0x26, 0x18, 0xB6, 0xFD, 0x1C, 0x2F, 0x8B, 0x27, 0xFF, 0x88,
    0xE0, 0xEB,
]);
assert_eq!(
    node.public().to_bytes(),
    [
        0x85, 0x20, 0xF0, 0x09, 0x89, 0x30, 0xA7, 0x54, 0x74, 0x8B, 0x7D, 0xDC, 0xB4, 0x3E,
        0xF7, 0x5A, 0x0D, 0xBF, 0x3A, 0x0D, 0x26, 0x38, 0x1A, 0xF4, 0xEB, 0xA4, 0xA9, 0x8E,
        0xAA, 0x9B, 0x4E, 0x6A,
    ]
);

// Neither side sends the session key. Both derive it from the shared secret, a salt that
// travels in the clear, and both public keys. The roles have to be opposite.
let salt = [0x09u8; 16];
let mut uplink = Session::establish(&node, &gateway.public(), &salt, Role::Initiator);
let mut downlink = Session::establish(&gateway, &node.public(), &salt, Role::Responder);

// The pump id is authenticated but not encrypted, so a router still reads it while any
// change to it fails the tag. Sealing replaces the plaintext in the buffer it is given.
let mut frame = *b"flow=41.2";
let sealed = uplink.seal(&mut frame, b"pump-3");
assert_ne!(&frame, b"flow=41.2");

let mut captured = frame;
downlink
    .open(&sealed, &mut frame, b"pump-3")
    .expect("authentic and fresh");
assert_eq!(&frame, b"flow=41.2");

// The anti-replay window refuses a counter it has already accepted, so a frame captured
// off the air and sent again is not delivered a second time.
assert!(downlink.open(&sealed, &mut captured, b"pump-3").is_err());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/session.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/session.py#example -->
From [`bindings/python/guides/session.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/session.py):

```python
from pamoja.core import PamojaError
from pamoja.session import AgreementKey, Role, Session

# Each device is provisioned with a 32-byte seed and publishes the key it derives. These
# are the X25519 pair RFC 7748 section 6.1 publishes, so the derivation is pinned to the
# specification rather than checked against itself.
node = AgreementKey(
    bytes.fromhex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
)
gateway = AgreementKey(
    bytes.fromhex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")
)
assert node.public_key.hex() == (
    "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"
)

# Neither side sends the session key. Both derive it from the shared secret, a salt that
# travels in the clear, and both public keys. The roles have to be opposite.
salt = bytes([0x09]) * 16
uplink = Session(node, gateway.public_key, salt, Role.INITIATOR)
downlink = Session(gateway, node.public_key, salt, Role.RESPONDER)

# The pump id is authenticated but not encrypted, so a router still reads it while any
# change to it fails the tag.
sealed = uplink.seal(b"flow=41.2", b"pump-3")
assert sealed.ciphertext != b"flow=41.2"
assert downlink.open(sealed, b"pump-3") == b"flow=41.2"

# The anti-replay window refuses a counter it has already accepted, so a frame captured
# off the air and sent again is not delivered a second time.
try:
    downlink.open(sealed, b"pump-3")
except PamojaError:
    pass
else:
    raise AssertionError("a replayed message should be refused")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SessionGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SessionGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SessionGuide.cs):

```csharp
// Each device is provisioned with a 32-byte seed and publishes the key it
// derives. These are the X25519 pair RFC 7748 section 6.1 publishes, so the
// derivation is pinned to the specification rather than checked against itself.
using var node = new AgreementKey(Convert.FromHexString(
    "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a"));
using var gateway = new AgreementKey(Convert.FromHexString(
    "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb"));
Expect(
    Convert.ToHexString(node.PublicKey).ToLowerInvariant()
        == "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a",
    "the public key is the one the vector publishes");

// Neither side sends the session key. Both derive it from the shared secret, a
// salt that travels in the clear, and both public keys. The roles are opposite.
byte[] salt = new byte[16];
Array.Fill(salt, (byte)0x09);
using var uplink = new Session(node, gateway.PublicKey, salt, SessionRole.Initiator);
using var downlink = new Session(gateway, node.PublicKey, salt, SessionRole.Responder);

// The pump id is authenticated but not encrypted, so a router still reads it
// while any change to it fails the tag.
SealedMessage reading = uplink.Seal("flow=41.2"u8, "pump-3"u8);
Expect(
    !reading.Ciphertext.SequenceEqual("flow=41.2"u8.ToArray()),
    "the reading does not travel in the clear");
Expect(
    downlink.Open(reading, "pump-3"u8).SequenceEqual("flow=41.2"u8.ToArray()),
    "the gateway recovers the reading");

// The anti-replay window refuses a counter it has already accepted, so a frame
// captured off the air and sent again is not delivered a second time.
bool refused = false;
try
{
    downlink.Open(reading, "pump-3"u8);
}
catch (PamojaException)
{
    refused = true;
}
Expect(refused, "the same message is refused a second time");
```
<!-- end -->

## Reference

<!-- table: reference session -->
- Rust: [`pamoja-session`](https://docs.rs/pamoja-session) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html))
- TypeScript: [`@pamoja/session`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html)
- Python: [`pamoja.session`](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html)
- C#: [`Session`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.Session.html), [`SessionRole`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.SessionRole.html), [`AgreementKey`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.AgreementKey.html), [`SealedMessage`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.SealedMessage.html)
<!-- end -->
