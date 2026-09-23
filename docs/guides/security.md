# Device identity

A reading that settles a bill or trips an alarm has to be traceable to the device
that produced it. pamoja gives each device an Ed25519 key pair derived from a
32-byte seed it is provisioned with: the seed stays on the device and signs, and
the 32-byte public key travels with the readings and verifies them. Signing is
deterministic and consumes no entropy, so a microcontroller with no random-number
peripheral signs its own telemetry without a source of randomness to get wrong.

A signature travels one of two ways. Detached, it is 64 bytes kept beside the
payload, which suits a record that is stored and checked later. As a signed
message, it goes in front of the payload as one blob, which is what a link
usually wants: one thing to send, and a gateway that gets the payload back only
once it has checked it.

A signature proves who signed and that nothing changed since. It does not hide
the reading, and it does not say when it was signed. The
[secured session](session.md) covers both.

## What the example does

It provisions a device with a seed, signs a meter reading, and verifies it the
way a gateway would, holding nothing but the 32-byte public key. The fingerprint
it prints is the short label an operator reads off a screen to tell one device
from another. It changes one digit of the reading and offers the signature under
a second device's key, and confirms both are rejected. Then it sends the reading
the way a link would, as one signed message, and cuts a byte off the end of it.

A real seed comes from the factory or a secure element and never leaves the
device. Any 32 bytes stand in for one here. Everything else is built by the
library: the key pair falls out of the seed, the gateway's copy is that public
key in its 32-byte wire form, and the fingerprint is derived from the key rather
than assigned. Key derivation and signing are pinned to RFC 8032 test vector 2
in `pamoja-security`'s own tests, which is where a published constant belongs.

It proves:

- A public key taken as 32 bytes verifies the reading the device signed, so a
  gateway needs nothing else from a device to check what it sends.
- Signing is deterministic: the same reading signed twice gives the identical
  signature, so signing needs no entropy.
- A reading altered after signing does not verify, which is what catches a value
  edited between the meter and the bill.
- The same reading and signature offered under a second device's key do not
  verify either, so a signature does not carry over to another identity.
- A signed message is the 64-byte signature followed by the reading, 84 bytes
  for this one, and the gateway reads the reading back out of it only after the
  check passes.
- A message that lost its last byte is refused whole, not handed back short.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example security" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example security</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- security" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- security</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/security.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/security.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- security" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- security</code></div>
</div>
<!-- end -->

## Rust

In Rust, `DeviceIdentity::from_seed(&seed)` takes the seed as a `[u8; 32]`, so a seed of the
wrong length does not compile. `public()` returns the `PublicIdentity` a gateway keeps;
`to_bytes()` turns it into its 32-byte wire form, and `PublicIdentity::from_bytes(&key)` turns it
back, refusing 32 bytes that are not a key. `sign(payload)` returns a `Signature`, 64 bytes
through `to_bytes()` and back through `Signature::from_bytes`, and `verify(payload, &signature)`
returns `Ok(())` or an `Error::Auth`. `sign_message(payload)` returns the signature and the
payload as one `Vec<u8>`, and `verify_message(&message)` returns the payload, borrowed from the
message, once it has checked it. `fingerprint()` gives the 16-character label. The crate is
`no_std` and draws no randomness, so the same calls run on a microcontroller.

<!-- snippet: examples/guides/security.rs#example -->
From [`examples/guides/security.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/security.rs):

```rust
use pamoja_security::{DeviceIdentity, PublicIdentity};

// The seed is provisioned into the device once and never leaves it. A real one comes
// from the factory or a secure element; any 32 bytes stand in here.
let device = DeviceIdentity::from_seed(&[7u8; 32]);

// Only the 32-byte public key travels to the gateway. Its fingerprint is the short
// form an operator reads off a screen to tell one device from another.
let gateway = PublicIdentity::from_bytes(&device.public().to_bytes()).expect("a valid key");
println!("device     {}", gateway.fingerprint());

// Signing is deterministic, so the same reading always produces the same 64 bytes and
// there is no randomness to get wrong on a microcontroller.
let reading = b"meter-4 1182.750 kWh";
let signature = device.sign(reading);
match gateway.verify(reading, &signature) {
    Ok(()) => println!("accepted   {}", String::from_utf8_lossy(reading)),
    Err(error) => println!("rejected   {error}"),
}

// A digit changed in transit no longer matches what was signed.
let edited = b"meter-4 1082.750 kWh";
match gateway.verify(edited, &signature) {
    Ok(()) => println!("accepted   an edited reading, which should never happen"),
    Err(_) => println!("rejected   {}", String::from_utf8_lossy(edited)),
}

// Nor does the same reading offered under another device's key.
let impostor = DeviceIdentity::from_seed(&[90u8; 32]);
match impostor.public().verify(reading, &signature) {
    Ok(()) => println!("accepted   an impostor, which should never happen"),
    Err(_) => println!("rejected   a signature offered under another device's key"),
}

// On a link the signature and the reading usually travel as one message, signature
// first, and the gateway gets the reading back only once it has checked it.
let message = device.sign_message(reading);
let size = message.len();
println!("message    {size} bytes on the wire, the signature and the reading together");
match gateway.verify_message(&message) {
    Ok(carried) => {
        let carried = String::from_utf8_lossy(carried);
        println!("accepted   {carried}, read out of the message");
    }
    Err(error) => println!("rejected   {error}"),
}

// A message that lost its last byte on the way is refused whole.
match gateway.verify_message(&message[..size - 1]) {
    Ok(_) => println!("accepted   a message cut short, which should never happen"),
    Err(_) => println!("rejected   a message that lost its last byte on the way"),
}
```
<!-- end -->

## TypeScript

In TypeScript, `DeviceIdentity.fromSeed(seed)` from `@pamoja/security` takes the seed as a
`Uint8Array` and throws if it is not 32 bytes. `publicKey()` returns the key as a 32-byte
`Buffer`, `fingerprint()` its label, `sign(payload)` a 64-byte signature, and
`signMessage(payload)` the signature and the payload as one `Buffer`. A gateway needs no
identity object: `verify(publicKey, payload, signature)` returns `true` or `false`,
`verifyMessage(publicKey, message)` returns the payload or `null`, and `fingerprint(publicKey)`
labels a key it was sent. A payload is a string, signed as its UTF-8 bytes, or any `Uint8Array`.
The calls throw only for an argument of the wrong length, and every call is synchronous.

<!-- snippet: bindings/node/guides/security.ts#example -->
From [`bindings/node/guides/security.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/security.ts):

```typescript
import { DeviceIdentity, fingerprint, verify, verifyMessage } from '@pamoja/security'

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

// On a link the signature and the reading usually travel as one message, signature first,
// and the gateway gets the reading back only once it has checked it.
const message = device.signMessage(reading)
const size = message.length
console.log(`message    ${size} bytes on the wire, the signature and the reading together`)
const carried = verifyMessage(gatewayKey, message)
if (carried) {
  console.log(`accepted   ${carried.toString()}, read out of the message`)
} else {
  console.log('rejected   a message the device really did sign, which should never happen')
}

// A message that lost its last byte on the way is refused whole.
if (verifyMessage(gatewayKey, message.subarray(0, size - 1))) {
  console.log('accepted   a message cut short, which should never happen')
} else {
  console.log('rejected   a message that lost its last byte on the way')
}
```
<!-- end -->

## Python

In Python, `DeviceIdentity.from_seed(seed)` from `pamoja.security` raises `ValueError` if the
seed is not 32 bytes. `public_key` and `fingerprint` are properties, `sign(payload)` returns 64
bytes, and `sign_message(payload)` the signature and the payload as one `bytes`. A gateway calls
`verify(public_key, payload, signature)`, which returns `True` or `False`,
`verify_message(public_key, message)`, which returns the payload or `None`, and
`fingerprint(public_key)`. A payload is a `str`, signed as its UTF-8 bytes, or any bytes-like
object. The calls raise `ValueError` only for an argument of the wrong length, and every call is
synchronous.

<!-- snippet: bindings/python/guides/security.py#example -->
From [`bindings/python/guides/security.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/security.py):

```python
from pamoja.security import DeviceIdentity, fingerprint, verify, verify_message

# The seed is provisioned into the device once and never leaves it. A real one comes from
# the factory or a secure element; any 32 bytes stand in here.
device = DeviceIdentity.from_seed(bytes([7]) * 32)

# Only the 32-byte public key travels to the gateway. Its fingerprint is the short form an
# operator reads off a screen to tell one device from another.
gateway_key = device.public_key
print(f"device     {fingerprint(gateway_key)}")

# Signing is deterministic, so the same reading always produces the same 64 bytes and there
# is no randomness to get wrong on a microcontroller.
reading = "meter-4 1182.750 kWh"
signature = device.sign(reading)
if verify(gateway_key, reading, signature):
    print(f"accepted   {reading}")
else:
    print("rejected   a reading the device really did sign, which should never happen")

# A digit changed in transit no longer matches what was signed.
edited = "meter-4 1082.750 kWh"
if verify(gateway_key, edited, signature):
    print("accepted   an edited reading, which should never happen")
else:
    print(f"rejected   {edited}")

# Nor does the same reading offered under another device's key.
impostor = DeviceIdentity.from_seed(bytes([90]) * 32)
if verify(impostor.public_key, reading, signature):
    print("accepted   an impostor, which should never happen")
else:
    print("rejected   a signature offered under another device's key")

# On a link the signature and the reading usually travel as one message, signature first,
# and the gateway gets the reading back only once it has checked it.
message = device.sign_message(reading)
print(f"message    {len(message)} bytes on the wire, the signature and the reading together")
carried = verify_message(gateway_key, message)
if carried is not None:
    print(f"accepted   {carried.decode()}, read out of the message")
else:
    print("rejected   a message the device really did sign, which should never happen")

# A message that lost its last byte on the way is refused whole.
if verify_message(gateway_key, message[:-1]) is not None:
    print("accepted   a message cut short, which should never happen")
else:
    print("rejected   a message that lost its last byte on the way")
```
<!-- end -->

## C#

In C#, `new DeviceIdentity(seed)` in `Pamoja.Security` throws `ArgumentException` if the seed is
not `DeviceIdentity.KeyLength`, 32 bytes, and the identity is disposable. `PublicKey` and
`Fingerprint` are properties, `Sign(payload)` returns `SignatureLength`, 64 bytes, and
`SignMessage(payload)` the signature and the payload as one array. A gateway calls the static
`DeviceIdentity.Verify(publicKey, payload, signature)`, which returns `true` or `false`,
`DeviceIdentity.VerifyMessage(publicKey, message)`, which returns the payload or `null`, and
`DeviceIdentity.FingerprintOf(publicKey)`. A payload is a `string`, signed as its UTF-8 bytes,
or a `ReadOnlySpan<byte>`. A key or a signature of the wrong length throws `ArgumentException`
before native code reads it.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SecurityGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SecurityGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SecurityGuide.cs):

```csharp
// The seed is provisioned into the device once and never leaves it. A real one
// comes from the factory or a secure element; any 32 bytes stand in here.
byte[] seed = new byte[DeviceIdentity.KeyLength];
Array.Fill(seed, (byte)7);
using var device = new DeviceIdentity(seed);

// Only the 32-byte public key travels to the gateway. Its fingerprint is the short
// form an operator reads off a screen to tell one device from another.
byte[] gatewayKey = device.PublicKey;
Console.WriteLine($"device     {DeviceIdentity.FingerprintOf(gatewayKey)}");

// Signing is deterministic, so the same reading always produces the same 64 bytes
// and there is no randomness to get wrong on a microcontroller.
const string reading = "meter-4 1182.750 kWh";
byte[] signature = device.Sign(reading);
Console.WriteLine(DeviceIdentity.Verify(gatewayKey, reading, signature)
    ? $"accepted   {reading}"
    : "rejected   a reading the device really did sign, which should never happen");

// A digit changed in transit no longer matches what was signed.
const string edited = "meter-4 1082.750 kWh";
Console.WriteLine(DeviceIdentity.Verify(gatewayKey, edited, signature)
    ? "accepted   an edited reading, which should never happen"
    : $"rejected   {edited}");

// Nor does the same reading offered under another device's key.
byte[] impostorSeed = new byte[DeviceIdentity.KeyLength];
Array.Fill(impostorSeed, (byte)90);
using var impostor = new DeviceIdentity(impostorSeed);
Console.WriteLine(DeviceIdentity.Verify(impostor.PublicKey, reading, signature)
    ? "accepted   an impostor, which should never happen"
    : "rejected   a signature offered under another device's key");

// On a link the signature and the reading usually travel as one message,
// signature first, and the gateway gets the reading back only once it has
// checked it.
byte[] message = device.SignMessage(reading);
int size = message.Length;
Console.WriteLine($"message    {size} bytes on the wire, the signature and the reading together");
byte[]? carried = DeviceIdentity.VerifyMessage(gatewayKey, message);
Console.WriteLine(carried is not null
    ? $"accepted   {Encoding.UTF8.GetString(carried)}, read out of the message"
    : "rejected   a message the device really did sign, which should never happen");

// A message that lost its last byte on the way is refused whole.
byte[]? cut = DeviceIdentity.VerifyMessage(gatewayKey, message.AsSpan(0, size - 1));
Console.WriteLine(cut is not null
    ? "accepted   a message cut short, which should never happen"
    : "rejected   a message that lost its last byte on the way");
```
<!-- end -->

## Values at a glance

**The calls:**

| What | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| an identity | `DeviceIdentity::from_seed(&seed)` | `DeviceIdentity.fromSeed(seed)` | `DeviceIdentity.from_seed(seed)` | `new DeviceIdentity(seed)` |
| its public key | `public().to_bytes()` | `publicKey()` | `public_key` | `PublicKey` |
| its label | `public().fingerprint()` | `fingerprint()` | `fingerprint` | `Fingerprint` |
| sign | `sign(payload)` | `sign(payload)` | `sign(payload)` | `Sign(payload)` |
| sign as one message | `sign_message(payload)` | `signMessage(payload)` | `sign_message(payload)` | `SignMessage(payload)` |

**What a gateway calls**, holding only the public key:

| What | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| verify | `public.verify(payload, &signature)` | `verify(key, payload, signature)` | `verify(key, payload, signature)` | `DeviceIdentity.Verify(key, payload, signature)` |
| verify a message | `public.verify_message(&message)` | `verifyMessage(key, message)` | `verify_message(key, message)` | `DeviceIdentity.VerifyMessage(key, message)` |
| label a key | `public.fingerprint()` | `fingerprint(key)` | `fingerprint(key)` | `DeviceIdentity.FingerprintOf(key)` |
| a failed check | `Err(Error::Auth)` | `false`, or `null` | `False`, or `None` | `false`, or `null` |

In Rust, `public` is the `PublicIdentity` that `PublicIdentity::from_bytes(&key)` builds from the
32 bytes.

**The sizes:**

| Value | Bytes | Kept where |
| --- | --- | --- |
| seed | 32 | on the device only, in secure storage |
| public key | 32 | with every gateway and auditor that trusts the device |
| signature | 64 | beside the payload |
| signed message | 64, plus the payload | on the link |
| fingerprint | 16 hex characters, from the key's first 8 bytes | in logs and on screens |

The signature is Ed25519 as RFC 8032 defines it, with no prehash and no context string, so a
library that follows the RFC verifies what pamoja signs.

## When it goes wrong

A wrong length throws an `Error` in TypeScript, a `ValueError` in Python, and an
`ArgumentException` in C#. What each check says:

| What happened | The message | What to check |
| --- | --- | --- |
| a seed that is not 32 bytes | `seed must be exactly 32 bytes`; in Rust, a compile error, since the seed is a `[u8; 32]` | how the seed was stored and read back |
| a public key that is not 32 bytes | `publicKey must be exactly 32 bytes`, `public_key` in Python | a key still in hex or base64 text |
| a signature that is not 64 bytes | `signature must be exactly 64 bytes` | where the signature was split from its payload |
| 32 bytes that are not a public key | `authentication error: invalid public identity` from a fingerprint, and `false` from `verify` | where the key came from |
| a payload altered, or signed by another device | `false` from `verify`, and `null` from `verifyMessage`; in Rust, `authentication error: signature verification failed` | nothing: the check did its job |
| a signed message shorter than a signature | `null` from `verifyMessage`; in Rust, `authentication error: message is shorter than a signature` | the link or the buffer that cut it |

The mistakes that cost an afternoon:

- **Text that looks the same is not the same bytes.** A signature covers bytes, so `1182.75`
  against `1182.750`, a trailing newline, or CRLF against LF each fail. Sign the bytes you send,
  and verify the bytes you received rather than a copy rebuilt from parsed values: a document
  written out again on the far side can change its key order or how it writes a number.
- **A fingerprint is a label, not a key.** It is the first 8 bytes of the public key, enough for
  a person to tell devices apart and not enough to trust one. Keep the full 32-byte key and
  compare that.
- **A replayed reading still verifies.** A signature proves who signed and what, not when, so a
  reading recorded today and sent again tomorrow passes. Put a counter or a timestamp inside what
  is signed and have the gateway refuse one it has seen, or carry readings in a secured session,
  which refuses replays itself.
- **Anyone on the link can read a signed reading.** A signature is not encryption. For a reading
  the link must not reveal, use a secured session.
- **The seed was not random, or not secret.** Anyone with the seed can sign as the device.
  Generate it at provisioning from the operating system's cryptographic source,
  `crypto.randomBytes(32)`, `secrets.token_bytes(32)`, `RandomNumberGenerator.GetBytes(32)`, or
  the `getrandom` crate, never from a serial number or a clock.
- **Two devices share a seed.** They are then one identity: a gateway cannot tell them apart,
  and retiring one retires both. Give every device its own seed.

## Where next

<!-- table: next security -->
- [Secured session](session.md): X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack.
- [Signed updates](update.md): Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own.
- [Audit log](audit.md): A tamper-evident, hash-chained log.
<!-- end -->

## Reference

<!-- table: reference security -->
- Rust: [`pamoja-security`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_security/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-security)
- TypeScript: [`@pamoja/security`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_security.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-security)
- Python: [`pamoja.security`](https://pamoja.molex.cloud/docs/reference/python/pamoja/security.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-security)
- C#: [`Pamoja.Security`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Security.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-security)
<!-- end -->
