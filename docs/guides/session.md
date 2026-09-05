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

It provisions a node and its gateway, establishes a session at each end from a
salt that travels in the clear, seals a flow reading with the pump id as
associated data, and opens it at the far end. Then the same frame is offered a
second time.

The seeds are the only key material written out on the page. A real one comes

from the factory or a secure element and never leaves the device; any 32 bytes

stand in here.
The public keys are derived from them, the salt is drawn from the system random
source at run time rather than fixed here, and the counter and tag that
authenticate the frame come back from `seal`, so a caller never composes a
nonce. The key agreement itself is pinned to the X25519 vector RFC 7748
publishes in `pamoja-session`'s own tests, which is where a published constant
belongs.

Sealing in Rust rewrites the buffer it is given, so that example copies the
ciphertext before the gateway opens it and replays that copy; the bindings
return the ciphertext beside the counter and tag and leave the plaintext they
were given untouched. The refusal comes from the counter window rather than
from a buffer that has already been overwritten.

It proves:

- The gateway opens what the node sealed, so both ends reached the same key from
  opposite roles without either of them sending it.
- What leaves the node is not the reading: the ciphertext differs from
  `flow=41.2`.
- Those nine bytes come back exactly, in Rust out of the same buffer that held
  the ciphertext a moment earlier.
- A frame the gateway has already accepted is refused when it arrives again, so a
  message captured off the air cannot be delivered twice.

## Rust

<!-- snippet: examples/tests/guides/session.rs#example -->
From [`examples/tests/guides/session.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/session.rs):

```rust
use pamoja_session::{AgreementKey, Role, Session};

// Each device is provisioned with a 32-byte seed and publishes the key it derives. A
// real seed comes from the factory or a secure element; any 32 bytes stand in here.
let node = AgreementKey::from_seed(&[7u8; 32]);
let gateway = AgreementKey::from_seed(&[9u8; 32]);

// Neither side sends the session key. Both derive it from the shared secret, a salt
// that travels in the clear, and both public keys, with opposite roles.
//
// The salt must be fresh for every session: reusing one derives the same key from the
// same pair of devices twice. The initiator draws it and sends it in the clear, so the
// responder uses the salt it received rather than one of its own.
let mut salt = [0u8; 16];
getrandom::fill(&mut salt).expect("the system random source");
let mut uplink = Session::establish(&node, &gateway.public(), &salt, Role::Initiator);
let mut downlink = Session::establish(&gateway, &node.public(), &salt, Role::Responder);
println!("both sides derived a key without sending one");

// The pump id is authenticated but not encrypted, so a router still reads it while any
// change to it fails the tag. Sealing replaces the plaintext in the buffer it is given.
let mut frame = *b"flow=41.2";
let sealed = uplink.seal(&mut frame, b"pump-3");
println!(
    "sealed    the reading is no longer readable: {}",
    frame != *b"flow=41.2"
);

// The gateway opens it back into the same buffer.
let mut replayed = frame;
downlink
    .open(&sealed, &mut frame, b"pump-3")
    .expect("authentic and fresh");
println!("opened    {}", String::from_utf8_lossy(&frame));

// The anti-replay window refuses a counter it has already accepted, so a frame
// captured off the air and sent again is not delivered a second time.
match downlink.open(&sealed, &mut replayed, b"pump-3") {
    Ok(()) => println!("a replayed frame was accepted, which should never happen"),
    Err(error) => println!("replay    refused: {error}"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/session.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/session.py#example -->
From [`bindings/python/guides/session.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/session.py):

```python
import os

from pamoja.core import PamojaError
from pamoja.session import AgreementKey, Role, Session

# Each device is provisioned with a 32-byte seed and publishes the key it derives. A real
# seed comes from the factory or a secure element; any 32 bytes stand in here.
node = AgreementKey(bytes([7]) * 32)
gateway = AgreementKey(bytes([9]) * 32)

# Neither side sends the session key. Both derive it from the shared secret, a salt that
# travels in the clear, and both public keys, with opposite roles.
#
# The salt must be fresh for every session: reusing one derives the same key from the same
# pair of devices twice. The initiator draws it and sends it in the clear, so the responder
# uses the salt it received rather than one of its own.
salt = os.urandom(16)
uplink = Session(node, gateway.public_key, salt, Role.INITIATOR)
downlink = Session(gateway, node.public_key, salt, Role.RESPONDER)
print("both sides derived a key without sending one")

# The pump id is authenticated but not encrypted, so a router still reads it while any
# change to it fails the tag.
sealed = uplink.seal(b"flow=41.2", b"pump-3")
print(f"sealed    the reading is no longer readable: {sealed.ciphertext != b'flow=41.2'}")
print(f"opened    {downlink.open(sealed, b'pump-3').decode()}")

# The anti-replay window refuses a counter it has already accepted, so a frame captured
# off the air and sent again is not delivered a second time.
try:
    downlink.open(sealed, b"pump-3")
    print("a replayed frame was accepted, which should never happen")
except PamojaError as error:
    print(f"replay    refused: {error}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SessionGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SessionGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SessionGuide.cs):

```csharp
// Each device is provisioned with a 32-byte seed and publishes the key it derives.
// A real seed comes from the factory or a secure element; any 32 bytes stand in.
byte[] nodeSeed = new byte[32];
Array.Fill(nodeSeed, (byte)7);
byte[] gatewaySeed = new byte[32];
Array.Fill(gatewaySeed, (byte)9);
using var node = new AgreementKey(nodeSeed);
using var gateway = new AgreementKey(gatewaySeed);

// Neither side sends the session key. Both derive it from the shared secret, a
// salt that travels in the clear, and both public keys, with opposite roles.
//
// The salt must be fresh for every session: reusing one derives the same key from
// the same pair of devices twice. The initiator draws it and sends it in the
// clear, so the responder uses the salt it received rather than one of its own.
byte[] salt = RandomNumberGenerator.GetBytes(16);
using var uplink = new Session(node, gateway.PublicKey, salt, SessionRole.Initiator);
using var downlink = new Session(gateway, node.PublicKey, salt, SessionRole.Responder);
Console.WriteLine("both sides derived a key without sending one");

// The pump id is authenticated but not encrypted, so a router still reads it while
// any change to it fails the tag.
SealedMessage reading = uplink.Seal("flow=41.2"u8, "pump-3"u8);
bool hidden = !reading.Ciphertext.SequenceEqual("flow=41.2"u8.ToArray());
Console.WriteLine($"sealed    the reading is no longer readable: {hidden}");
byte[] opened = downlink.Open(reading, "pump-3"u8);
Console.WriteLine($"opened    {System.Text.Encoding.UTF8.GetString(opened)}");

// The anti-replay window refuses a counter it has already accepted, so a frame
// captured off the air and sent again is not delivered a second time.
try
{
    downlink.Open(reading, "pump-3"u8);
    Console.WriteLine("a replayed frame was accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"replay    refused: {error.Message}");
}
```
<!-- end -->

## Reference

<!-- table: reference session -->
- Rust: [`pamoja-session`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html)
- TypeScript: [`@pamoja/session`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html)
- Python: [`pamoja.session`](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html)
- C#: [`Pamoja.Session`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html)
<!-- end -->
