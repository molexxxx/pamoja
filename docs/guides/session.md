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

It provisions a node and its gateway, establishes a session at each end from a salt
that travels in the clear, and sends flow readings from a pump with the pump id as
associated data. One arrives and opens, the same frame is offered a second time, one
has its pump id rewritten on the way, one arrives late, after a newer one, and the
gateway sends an order back to the node.

The seeds are the only key material written out on the page. A real one comes from
the factory or a secure element and never leaves the device; any 32 bytes stand in
here. The public keys are derived from them, the salt is drawn from the system random
source at run time rather than fixed here, and the counter and tag that authenticate
each frame come back from `seal`, so a caller never composes a nonce. The key agreement
itself is pinned to the X25519 vector RFC 7748 publishes in `pamoja-session`'s own
tests, which is where a published constant belongs.

Sealing in Rust rewrites the buffer it is given, so that example copies a ciphertext
before it opens the copy it wants to replay or tamper with; the bindings return the
ciphertext beside the counter and tag and leave the plaintext they were given
untouched.

It proves:

- The gateway opens what the node sealed, so both ends reached the same key from
  opposite roles without either of them sending it.
- What leaves the node is not the reading: the ciphertext differs from `flow=41.2`.
- A frame the gateway has already accepted is refused when it arrives again, so a
  message captured off the air cannot be delivered twice.
- A frame whose pump id was rewritten fails authentication, so the reading cannot be
  filed under another pump; and because a failed frame does not use up its counter,
  the genuine one still opens afterward.
- Counter 2 opening before counter 1 is fine: the window accepts a late frame it has
  not seen yet.
- The gateway's reply opens at the node. Each direction has its own nonces under the
  one key, so a reply is never taken for, or replayed as, a message from the node.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example session" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example session</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- session" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- session</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/session.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/session.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- session" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- session</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-session` is `no_std` and works on buffers the caller owns.
`AgreementKey::from_seed` turns a 32-byte seed into the device's key-agreement
secret, and `public()` gives the `AgreementPublicKey` to hand to the peer, which
crosses as `to_bytes()` and `AgreementPublicKey::from_bytes`.
`Session::establish(&local, &peer, &salt, role)` derives the session at each end.
`seal(&mut buf, aad)` encrypts in place and returns a `Sealed` with the `counter` and
16-byte `tag` to send beside the ciphertext, and `open(&sealed, &mut buf, aad)`
decrypts in place or returns a `SessionError`: `Inauthentic` or `Replayed`. A
`Session` is deliberately not `Clone`, because two copies would reuse counters and so
reuse nonces.

<!-- snippet: examples/guides/session.rs#example -->
From [`examples/guides/session.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/session.rs):

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
println!("agreed    both sides derived a key without sending one");

// The pump id is authenticated but not encrypted, so a router still reads it while any
// change to it fails the tag. Sealing replaces the plaintext in the buffer it is given.
let mut frame = *b"flow=41.2";
let sealed = uplink.seal(&mut frame, b"pump-3");
let hidden = if frame != *b"flow=41.2" {
    "no longer"
} else {
    "still"
};
println!(
    "sealed    counter {}, and what goes on the wire is {hidden} the reading",
    sealed.counter
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

// A router that rewrites the pump id breaks the tag, so the gateway refuses the frame
// rather than file the reading under the wrong pump. A frame that fails to open leaves
// its counter unused.
let mut later = *b"flow=41.3";
let later_sealed = uplink.seal(&mut later, b"pump-3");
let mut rewritten = later;
match downlink.open(&later_sealed, &mut rewritten, b"pump-4") {
    Ok(()) => println!("a rewritten pump id was accepted, which should never happen"),
    Err(error) => println!("altered   refused: {error}"),
}

// Radio frames can arrive out of order. The window accepts any counter it has not seen
// among the 64 below the newest, so the frame that was held up still opens.
let mut newest = *b"flow=41.5";
let newest_sealed = uplink.seal(&mut newest, b"pump-3");
downlink
    .open(&newest_sealed, &mut newest, b"pump-3")
    .expect("the newest frame");
downlink
    .open(&later_sealed, &mut later, b"pump-3")
    .expect("a late frame inside the window");
println!(
    "late      counter {} opened first, then counter {}: {}, then {}",
    newest_sealed.counter,
    later_sealed.counter,
    String::from_utf8_lossy(&newest),
    String::from_utf8_lossy(&later)
);

// The gateway answers on the same session. Its frames carry the other direction in
// their nonce, so a reply can never be taken for, or replayed as, one from the node.
let mut order = *b"valve=close";
let order_sealed = downlink.seal(&mut order, b"pump-3");
uplink
    .open(&order_sealed, &mut order, b"pump-3")
    .expect("the gateway's reply");
println!(
    "reply     {}, sealed by the gateway and opened by the node",
    String::from_utf8_lossy(&order)
);
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/session` exports `AgreementKey`, built from a 32-byte seed,
whose `publicKey()` gives the bytes to hand the peer. `new Session(local, peerPublicKey,
salt, Role.Initiator)` derives the session. `seal(plaintext, aad?)` returns a
`SealedMessage` of `ciphertext`, `counter`, and `tag`, leaving the plaintext as it was,
and `open(sealed, aad?)` returns the plaintext or throws `session message failed
authentication` or `session message is a replay`. The associated data is empty unless
given. `hmacSha256` and `hkdfSha256` are there too, for a caller building its own
derivation.

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
<!-- end -->

## Python

In Python, `pamoja.session` exports `AgreementKey(seed)`, whose `public_key` property
is the bytes to hand the peer, and `Session(local, peer_public_key, salt, role)`.
`seal(plaintext, aad=b"")` returns a `SealedMessage` with `ciphertext`, `counter`, and
`tag`, and `open(sealed, aad=b"")` returns the plaintext bytes or raises `PamojaError`
with the reason. `hmac_sha256` and `hkdf_sha256` are there too.

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
print("agreed    both sides derived a key without sending one")

# The pump id is authenticated but not encrypted, so a router still reads it while any
# change to it fails the tag.
sealed = uplink.seal(b"flow=41.2", b"pump-3")
hidden = "still" if sealed.ciphertext == b"flow=41.2" else "no longer"
print(f"sealed    counter {sealed.counter}, and what goes on the wire is {hidden} the reading")
print(f"opened    {downlink.open(sealed, b'pump-3').decode()}")

# The anti-replay window refuses a counter it has already accepted, so a frame captured
# off the air and sent again is not delivered a second time.
try:
    downlink.open(sealed, b"pump-3")
    print("a replayed frame was accepted, which should never happen")
except PamojaError as error:
    print(f"replay    refused: {error}")

# A router that rewrites the pump id breaks the tag, so the gateway refuses the frame rather
# than file the reading under the wrong pump. A frame that fails to open leaves its counter
# unused.
later = uplink.seal(b"flow=41.3", b"pump-3")
try:
    downlink.open(later, b"pump-4")
    print("a rewritten pump id was accepted, which should never happen")
except PamojaError as error:
    print(f"altered   refused: {error}")

# Radio frames can arrive out of order. The window accepts any counter it has not seen
# among the 64 below the newest, so the frame that was held up still opens.
newest = uplink.seal(b"flow=41.5", b"pump-3")
first = downlink.open(newest, b"pump-3").decode()
second = downlink.open(later, b"pump-3").decode()
print(f"late      counter {newest.counter} opened first, then counter {later.counter}: {first}, then {second}")

# The gateway answers on the same session. Its frames carry the other direction in their
# nonce, so a reply can never be taken for, or replayed as, one from the node.
order = downlink.seal(b"valve=close", b"pump-3")
print(f"reply     {uplink.open(order, b'pump-3').decode()}, sealed by the gateway and opened by the node")
```
<!-- end -->

## C#

In C#, `Pamoja.Session` holds `AgreementKey`, whose `PublicKey` is the bytes to hand
the peer, and `new Session(local, peerPublicKey, salt, SessionRole.Initiator)`.
`Seal(plaintext, aad)` returns a `SealedMessage` record of `Counter`, `Tag`, and
`Ciphertext`, and `Open(message, aad)` returns the plaintext or throws
`PamojaException` with the reason. A key or a public key of the wrong length throws
`ArgumentException` before the native call. A key and a session hold native handles
and belong in a `using`; `Session.HmacSha256` and `Session.HkdfSha256` are static.

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
Console.WriteLine("agreed    both sides derived a key without sending one");

// The pump id is authenticated but not encrypted, so a router still reads it while
// any change to it fails the tag.
SealedMessage reading = uplink.Seal("flow=41.2"u8, "pump-3"u8);
string hidden = reading.Ciphertext.SequenceEqual("flow=41.2"u8.ToArray()) ? "still" : "no longer";
Console.WriteLine($"sealed    counter {reading.Counter}, and what goes on the wire is {hidden} the reading");
byte[] opened = downlink.Open(reading, "pump-3"u8);
Console.WriteLine($"opened    {Encoding.UTF8.GetString(opened)}");

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

// A router that rewrites the pump id breaks the tag, so the gateway refuses the
// frame rather than file the reading under the wrong pump. A frame that fails to
// open leaves its counter unused.
SealedMessage later = uplink.Seal("flow=41.3"u8, "pump-3"u8);
try
{
    downlink.Open(later, "pump-4"u8);
    Console.WriteLine("a rewritten pump id was accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"altered   refused: {error.Message}");
}

// Radio frames can arrive out of order. The window accepts any counter it has not
// seen among the 64 below the newest, so the frame that was held up still opens.
SealedMessage newest = uplink.Seal("flow=41.5"u8, "pump-3"u8);
string first = Encoding.UTF8.GetString(downlink.Open(newest, "pump-3"u8));
string second = Encoding.UTF8.GetString(downlink.Open(later, "pump-3"u8));
Console.WriteLine($"late      counter {newest.Counter} opened first, then counter {later.Counter}: {first}, then {second}");

// The gateway answers on the same session. Its frames carry the other direction in
// their nonce, so a reply can never be taken for, or replayed as, one from the node.
SealedMessage order = downlink.Seal("valve=close"u8, "pump-3"u8);
string answer = Encoding.UTF8.GetString(uplink.Open(order, "pump-3"u8));
Console.WriteLine($"reply     {answer}, sealed by the gateway and opened by the node");
```
<!-- end -->

## Values at a glance

**What crosses the wire** with each message, besides the associated data the receiver
already knows:

| Part | Size | What it is |
| --- | --- | --- |
| ciphertext | the plaintext's length | the ChaCha20 encryption of the message |
| counter | 8 bytes | the message's number in its direction, from 0, which the receiver needs to rebuild the nonce |
| tag | 16 bytes | the Poly1305 tag over the ciphertext and the associated data |

**What the session derives,** from the X25519 shared secret, the salt, and both public
keys, initiator first:

| Derived | Size | Used for |
| --- | --- | --- |
| key | 32 bytes | ChaCha20-Poly1305 in both directions |
| nonce prefix | 3 bytes | the part of every nonce that is fixed for the session |

Each 12-byte nonce is a direction byte, 0 for messages the initiator sends and 1 for
the responder's, then the prefix, then the counter, big-endian. The two directions
therefore never share a nonce under the one key.

**What the receiver accepts:**

| The frame | What happens |
| --- | --- |
| authentic, with a counter never seen | opens, and the window records the counter |
| a counter already opened | refused as a replay |
| a counter 64 or more below the newest opened | refused as a replay, since the window no longer tracks it |
| an older counter inside the window, not yet opened | opens |
| altered ciphertext, tag, counter, or associated data | refused as inauthentic, and the counter stays unused |
| sealed under a different key, salt, or role | refused as inauthentic |

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| make the device's key | `AgreementKey::from_seed(&seed)` |
| give the peer the public half | `key.public()`, `to_bytes()`, `AgreementPublicKey::from_bytes(&bytes)` |
| start a session | `Session::establish(&local, &peer, &salt, Role::Initiator)` |
| send | `session.seal(&mut buf, aad)` gives `Sealed { counter, tag }`; `buf` becomes the ciphertext |
| receive | `session.open(&sealed, &mut buf, aad)`; `buf` becomes the plaintext |

### TypeScript

| To | Call |
| --- | --- |
| make the device's key | `new AgreementKey(seed)` |
| give the peer the public half | `key.publicKey()` |
| start a session | `new Session(local, peerPublicKey, salt, Role.Initiator)` |
| send | `session.seal(plaintext, aad?)` gives `{ ciphertext, counter, tag }` |
| receive | `session.open(sealed, aad?)` gives the plaintext |

### Python

| To | Call |
| --- | --- |
| make the device's key | `AgreementKey(seed)` |
| give the peer the public half | `key.public_key` |
| start a session | `Session(local, peer_public_key, salt, Role.INITIATOR)` |
| send | `session.seal(plaintext, aad=b"")` gives a `SealedMessage` |
| receive | `session.open(sealed, aad=b"")` gives the plaintext |

### C#

| To | Call |
| --- | --- |
| make the device's key | `new AgreementKey(seed)` |
| give the peer the public half | `key.PublicKey` |
| start a session | `new Session(local, peerPublicKey, salt, SessionRole.Initiator)` |
| send | `session.Seal(plaintext, aad)` gives a `SealedMessage` |
| receive | `session.Open(message, aad)` gives the plaintext |

<!-- languages end -->

## When it goes wrong

What the session refuses:

| What happened | The message | Where |
| --- | --- | --- |
| a frame that does not authenticate | `session message failed authentication` | every language: `SessionError::Inauthentic` in Rust |
| a frame already opened, or too old for the window | `session message is a replay` | every language: `SessionError::Replayed` in Rust |
| a seed that is not 32 bytes | `seed must be exactly 32 bytes` | the bindings; Rust takes a `[u8; 32]` |
| a peer key that is not 32 bytes | `peerPublicKey must be exactly 32 bytes` | TypeScript and C#, as `ArgumentException`; Python says `peer_public_key` |
| a tag that is not 16 bytes | `tag must be exactly 16 bytes` | TypeScript and Python; C# says `message.Tag` and throws `ArgumentException` |

The mistakes that cost an afternoon:

- **The same salt twice.** A salt reused with the same two devices derives the same key
  and nonce prefix, and both sessions count from 0, so their first frames share a nonce.
  Under ChaCha20-Poly1305 that exposes both plaintexts and lets a forger in. Draw a fresh
  salt for every session, from a random source or a counter kept in storage that survives
  a power cut, and never a constant.
- **Both ends chose the initiator.** The roles order the public keys and the directions,
  so two initiators derive different nonces for each other's frames and nothing opens.
  One side initiates, the other responds.
- **A peer key taken on trust.** Agreement proves the channel is private, not who is on
  the other end. Pin the peer's public key at provisioning, or check it against a
  signature from its device identity, before establishing a session with it.
- **The associated data differs.** The receiver has to pass exactly what the sender
  authenticated. A gateway that builds the pump id from a different field fails every
  frame as inauthentic.
- **Old frames are refused after a burst of loss.** The window covers the newest counter
  opened and the 63 below it. A frame that falls further behind is refused as a replay,
  because the session can no longer prove it is not one.
- **A session copied between threads.** In Rust a session cannot be cloned; in the
  bindings, share one session object rather than rebuilding it from the same salt, which
  is the reused-salt mistake again.

## Where next

<!-- table: next session -->
- [Device identity](security.md): ed25519 device identity.
- [Mesh frames](mesh.md): Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once.
- [LoRaWAN](lorawan.md): LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join.
- Also in Trust and operation: [Audit log](audit.md), [Signed updates](update.md), [Power](power.md), [Telemetry](telemetry.md).
<!-- end -->

## Reference

<!-- table: reference session -->
- Rust: [`pamoja-session`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-session)
- TypeScript: [`@pamoja/session`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-session)
- Python: [`pamoja.session`](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-session)
- C#: [`Pamoja.Session`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-session)
<!-- end -->
