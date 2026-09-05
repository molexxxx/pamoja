# Device identity

A reading that settles a bill or trips an alarm has to be traceable to the device
that produced it. pamoja gives each device an ed25519 key pair derived from a
32-byte seed it is provisioned with: the seed stays on the device and signs, and
the 32-byte public key travels with the readings and verifies them. Signing is
deterministic and consumes no entropy, so a microcontroller with no random-number
peripheral signs its own telemetry without a source of randomness to get wrong.

## What the example does

It provisions a device with a seed, signs a meter reading, and verifies it the
way a gateway would, holding nothing but the 32-byte public key. The fingerprint
it prints is the short label an operator reads off a screen to tell one device
from another. Finally it changes one digit of the reading and offers the
signature under a second device's key, and confirms both are rejected.

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

## Rust

<!-- snippet: examples/tests/guides/security.rs#example -->
From [`examples/tests/guides/security.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/security.rs):

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
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/security.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/security.py#example -->
From [`bindings/python/guides/security.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/security.py):

```python
from pamoja.security import DeviceIdentity, fingerprint, verify

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
```
<!-- end -->

## C#

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
```
<!-- end -->

## Reference

<!-- table: reference security -->
- Rust: [`pamoja-security`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_security/index.html)
- TypeScript: [`@pamoja/security`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_security.html)
- Python: [`pamoja.security`](https://pamoja.molex.cloud/docs/reference/python/pamoja/security.html)
- C#: [`Pamoja.Security`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Security.html)
<!-- end -->
