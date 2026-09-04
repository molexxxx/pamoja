# Device identity

A reading that settles a bill or trips an alarm has to be traceable to the device
that produced it. pamoja gives each device an ed25519 key pair derived from a
32-byte seed it is provisioned with: the seed stays on the device and signs, and
the 32-byte public key travels with the readings and verifies them. Signing is
deterministic and consumes no entropy, so a microcontroller with no random-number
peripheral signs its own telemetry without a source of randomness to get wrong.

## What the example does

It provisions a device with the seed from RFC 8032 test vector 2 and checks the
signature it produces over that vector's message against the one the RFC
publishes. It then signs a meter reading and verifies it the way a gateway would,
holding nothing but the public key. Finally it changes one digit of the reading
and offers the signature under a second device's key, and confirms both are
rejected.

It proves:

- The seed derives the key pair the specification fixes and signs to the exact 64
  bytes RFC 8032 publishes, so an implementation that is wrong but self-consistent
  still fails.
- The fingerprint is the first eight bytes of the public key in hex, a label for
  logs and displays rather than a substitute for the key itself.
- Signing is deterministic: the same reading signed twice gives the same bytes.
- A reading altered after signing does not verify, and neither does a signature
  checked against a different device's key.

## Rust

<!-- snippet: examples/tests/guides/security.rs#example -->
From [`examples/tests/guides/security.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/security.rs):

```rust
use pamoja_security::{DeviceIdentity, PublicIdentity};

// The seed is provisioned into the device and never leaves it. This one is RFC 8032 test
// vector 2, so the key it derives and the signature below are published constants rather
// than values checked against themselves.
let device = DeviceIdentity::from_seed(&[
    0x4c, 0xcd, 0x08, 0x9b, 0x28, 0xff, 0x96, 0xda, 0x9d, 0xb6, 0xc3, 0x46, 0xec, 0x11, 0x4e,
    0x0f, 0x5b, 0x8a, 0x31, 0x9f, 0x35, 0xab, 0xa6, 0x24, 0xda, 0x8c, 0xf6, 0xed, 0x4f, 0xb8,
    0xa6, 0xfb,
]);
assert_eq!(
    device.sign(&[0x72]).to_bytes(),
    [
        0x92, 0xa0, 0x09, 0xa9, 0xf0, 0xd4, 0xca, 0xb8, 0x72, 0x0e, 0x82, 0x0b, 0x5f, 0x64,
        0x25, 0x40, 0xa2, 0xb2, 0x7b, 0x54, 0x16, 0x50, 0x3f, 0x8f, 0xb3, 0x76, 0x22, 0x23,
        0xeb, 0xdb, 0x69, 0xda, 0x08, 0x5a, 0xc1, 0xe4, 0x3e, 0x15, 0x99, 0x6e, 0x45, 0x8f,
        0x36, 0x13, 0xd0, 0xf1, 0x1d, 0x8c, 0x38, 0x7b, 0x2e, 0xae, 0xb4, 0x30, 0x2a, 0xee,
        0xb0, 0x0d, 0x29, 0x16, 0x12, 0xbb, 0x0c, 0x00,
    ]
);

// Only the 32-byte public key travels to the gateway.
let gateway = PublicIdentity::from_bytes(&device.public().to_bytes()).expect("a valid key");
assert_eq!(gateway.fingerprint(), "3d4017c3e843895a");

// Signing is deterministic, so the same reading always yields the same 64 bytes; there is
// no randomness to get wrong on a microcontroller.
let reading = b"meter-4 1182.750 kWh";
let signature = device.sign(reading);
assert_eq!(device.sign(reading), signature);
assert!(gateway.verify(reading, &signature).is_ok());

// A digit changed in transit fails, and so does a signature offered under another device's
// key.
assert!(gateway.verify(b"meter-4 1082.750 kWh", &signature).is_err());
let impostor = DeviceIdentity::from_seed(&[0x5a; 32]);
assert!(impostor.public().verify(reading, &signature).is_err());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/security.ts#example -->
From [`bindings/node/guides/security.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/security.ts):

```typescript
import assert from 'node:assert/strict'

import { DeviceIdentity, fingerprint, verify } from '@pamoja/security'

// The seed is provisioned into the device and never leaves it. This one is RFC 8032 test
// vector 2, so the key it derives and the signature below are published constants rather
// than values checked against themselves.
const device = DeviceIdentity.fromSeed(
  Buffer.from('4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb', 'hex')
)
assert.equal(
  device.sign(Buffer.from([0x72])).toString('hex'),
  '92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da' +
    '085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00'
)

// Only the 32-byte public key travels to the gateway.
const gatewayKey = device.publicKey()
assert.equal(fingerprint(gatewayKey), '3d4017c3e843895a')

// Signing is deterministic, so the same reading always yields the same 64 bytes; there is
// no randomness to get wrong on a microcontroller.
const reading = 'meter-4 1182.750 kWh'
const signature = device.sign(reading)
assert.deepEqual(device.sign(reading), signature)
assert.ok(verify(gatewayKey, reading, signature))

// A digit changed in transit fails, and so does a signature offered under another device's
// key.
assert.ok(!verify(gatewayKey, 'meter-4 1082.750 kWh', signature))
const impostor = DeviceIdentity.fromSeed(Buffer.alloc(32, 0x5a))
assert.ok(!verify(impostor.publicKey(), reading, signature))
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/security.py#example -->
From [`bindings/python/guides/security.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/security.py):

```python
from pamoja.security import DeviceIdentity, fingerprint, verify

# The seed is provisioned into the device and never leaves it. This one is RFC 8032 test
# vector 2, so the key it derives and the signature below are published constants rather
# than values checked against themselves.
device = DeviceIdentity.from_seed(
    bytes.fromhex("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb")
)
assert device.sign(bytes([0x72])) == bytes.fromhex(
    "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da"
    "085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"
)

# Only the 32-byte public key travels to the gateway.
gateway_key = device.public_key
assert fingerprint(gateway_key) == "3d4017c3e843895a"

# Signing is deterministic, so the same reading always yields the same 64 bytes; there is
# no randomness to get wrong on a microcontroller.
reading = "meter-4 1182.750 kWh"
signature = device.sign(reading)
assert device.sign(reading) == signature
assert verify(gateway_key, reading, signature) is True

# A digit changed in transit fails, and so does a signature offered under another device's
# key.
assert verify(gateway_key, "meter-4 1082.750 kWh", signature) is False
impostor = DeviceIdentity.from_seed(bytes([0x5A]) * 32)
assert verify(impostor.public_key, reading, signature) is False
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SecurityGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SecurityGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SecurityGuide.cs):

```csharp
// The seed is provisioned into the device and never leaves it. This one is
// RFC 8032 test vector 2, so the key it derives and the signature below are
// published constants rather than values checked against themselves.
using var device = new DeviceIdentity(Convert.FromHexString(
    "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"));
byte[] message = [0x72];
byte[] published = Convert.FromHexString(
    "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da"
    + "085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00");
Expect(
    device.Sign(message).SequenceEqual(published),
    "the signature is the one the vector publishes");

// Only the 32-byte public key travels to the gateway.
byte[] gatewayKey = device.PublicKey;
Expect(
    DeviceIdentity.FingerprintOf(gatewayKey) == "3d4017c3e843895a",
    "the fingerprint labels the key the vector fixes");

// Signing is deterministic, so the same reading always yields the same 64 bytes;
// there is no randomness to get wrong on a microcontroller.
const string reading = "meter-4 1182.750 kWh";
byte[] signature = device.Sign(reading);
Expect(device.Sign(reading).SequenceEqual(signature), "signing is deterministic");
Expect(DeviceIdentity.Verify(gatewayKey, reading, signature), "the reading is authentic");

// A digit changed in transit fails, and so does a signature offered under another
// device's key.
Expect(
    !DeviceIdentity.Verify(gatewayKey, "meter-4 1082.750 kWh", signature),
    "an altered reading does not verify");
byte[] impostorSeed = new byte[DeviceIdentity.KeyLength];
Array.Fill(impostorSeed, (byte)0x5A);
using var impostor = new DeviceIdentity(impostorSeed);
Expect(
    !DeviceIdentity.Verify(impostor.PublicKey, reading, signature),
    "another device's key does not verify it either");
```
<!-- end -->

## Reference

<!-- table: reference security -->
- Rust: [`pamoja-security`](https://docs.rs/pamoja-security) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_security/index.html))
- TypeScript: [`@pamoja/security`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_security.html)
- Python: [`pamoja.security`](https://pamoja.molex.cloud/docs/reference/python/pamoja/security.html)
- C#: [`DeviceIdentity`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Security.DeviceIdentity.html)
<!-- end -->
