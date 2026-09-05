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

A network admits a device over the air. It grants an address and builds the
signed accept that carries it; a device holding only its root key verifies that
frame and activates on it; then both ends exchange a reading on session keys
neither of them sent, and a tampered accept is refused.

No frame is pasted in. The library builds the accept out of the fields the
grant holds, so the 17 bytes it reports are one header byte over a single
encrypted AES block: the nonce the network drew, the network identifier, the
address, the downlink settings and the CMAC that signs them. The device reads
`0x26012E43` back out of that block rather than being configured with it, and
it is created with all-zero identifiers, because a join accept names no device.
The root key alone decides whether an accept is this device's.

It proves:

- A device holding nothing but the root key verifies the accept and reads the
  address `0x26012E43` out of it, decrypted from the frame rather than
  configured on the device.
- Neither side transmits a session key. The device derives its pair from the
  accept it decrypts, the network derives its pair from the grant, and a frame
  the device encrypts reads back at the network as `level=high`.
- That uplink exercises both derived keys: the message integrity code verifies
  under the network session key and the payload decrypts under the application
  key, because the frame goes to a port above zero.
- One byte flipped inside the accept fails the integrity check, so a device
  does not activate on a join it cannot attribute to its own network.

## Rust

<!-- snippet: examples/tests/guides/lorawan.rs#example -->
From [`examples/tests/guides/lorawan.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/lorawan.rs):

```rust
use pamoja_lorawan::{Device, JoinGrant, Uplink};

// The root key is provisioned into the device at the factory and known to the network
// server. It is the only secret either side starts with; any 16 bytes stand in here.
let app_key = [7u8; 16];

// The device asks to join with a nonce it has not used before, which is what stops an
// old accept being replayed at it.
let dev_nonce = 1;
let node = Device::new([0; 8], [0; 8], app_key);

// The network grants the join. It draws its own nonce, names the network the device is
// joining, and assigns the address the device will answer to from then on.
let app_nonce = 2;
let net_id = 19;
let dev_addr = 0x2601_2E43;
let grant = JoinGrant::new(app_nonce, net_id, dev_addr);
let accept = grant.accept(&app_key, dev_nonce);
println!(
    "granted   address {dev_addr:#010X} in a {}-byte accept",
    accept.as_bytes().len()
);

// The device verifies it against the root key. A join accept carries no device
// identifier, so only that key decides whether it is for this device.
let joined = node
    .accept_join(accept.as_bytes(), dev_nonce)
    .expect("the accept verifies under the root key");
println!(
    "joined    the device took address {:#010X}",
    joined.dev_addr()
);

// Neither side transmits a session key. Both derive the same pair from the root key
// and the two nonces, so the network reads what the device sends without ever having
// been told how.
let network = grant.session(&app_key, dev_nonce);
let reading = Uplink::new(1, 1, b"level=high");
let uplink = joined
    .session()
    .encode_uplink(&reading)
    .expect("a payload that fits one frame");
let received = network
    .decode(uplink.as_bytes(), 1)
    .expect("the message integrity code verifies under the derived key");
println!(
    "uplink    the network read {}",
    String::from_utf8_lossy(received.payload())
);

// A single byte changed in the air fails that check, so no one else can admit the
// device or put words in its mouth.
let mut forged = accept.as_bytes().to_vec();
forged[1] ^= 0xFF;
match node.accept_join(&forged, dev_nonce) {
    Ok(_) => println!("a forged accept was taken, which should never happen"),
    Err(error) => println!("forged    accept refused: {error}"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/lorawan.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/lorawan.py#example -->
From [`bindings/python/guides/lorawan.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/lorawan.py):

```python
from pamoja.core import PamojaError
from pamoja.lorawan import device, grant

# The root key is provisioned into the device at the factory and known to the network
# server. It is the only secret either side starts with; any 16 bytes stand in here.
app_key = bytes([7]) * 16

# The device asks to join with a nonce it has not used before, which is what stops an old
# accept being replayed at it.
dev_nonce = 1
node = device(bytes(8), bytes(8), app_key)

# The network grants the join. It draws its own nonce, names the network the device is
# joining, and assigns the address the device will answer to from then on.
dev_addr = 0x26012E43
offer = grant(app_nonce=2, net_id=19, dev_addr=dev_addr)
accept = offer.accept(app_key, dev_nonce)
print(f"granted   address 0x{dev_addr:08X} in a {len(accept)}-byte accept")

# The device verifies it against the root key. A join accept carries no device identifier,
# so only that key decides whether it is for this device.
joined = node.accept_join(accept, dev_nonce)
print(f"joined    the device took address 0x{joined.dev_addr:08X}")

# Neither side transmits a session key. Both derive the same pair from the root key and the
# two nonces, so the network reads what the device sends without ever having been told how.
network = offer.session(app_key, dev_nonce)
uplink = joined.session().encode_uplink(1, 1, b"level=high")
received = network.decode(uplink, 1)
print(f"uplink    the network read {received.payload.decode()}")

# A single byte changed in the air fails that check, so no one else can admit the device or
# put words in its mouth.
forged = bytearray(accept)
forged[1] ^= 0xFF
try:
    node.accept_join(bytes(forged), dev_nonce)
    print("a forged accept was taken, which should never happen")
except PamojaError as error:
    print(f"forged    accept refused: {error}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs):

```csharp
// The root key is provisioned into the device at the factory and known to the
// network server. It is the only secret either side starts with; any 16 bytes
// stand in here.
byte[] appKey = new byte[16];
Array.Fill(appKey, (byte)7);

// The device asks to join with a nonce it has not used before, which is what stops
// an old accept being replayed at it.
const ushort DevNonce = 1;
using var node = new LorawanDevice(new byte[8], new byte[8], appKey);

// The network grants the join. It draws its own nonce, names the network the
// device is joining, and assigns the address it will answer to from then on.
const uint DevAddr = 0x26012E43;
var offer = new LorawanGrant(appNonce: 2, netId: 19, devAddr: DevAddr);
byte[] accept = offer.Accept(appKey, DevNonce);
Console.WriteLine($"granted   address 0x{DevAddr:X8} in a {accept.Length}-byte accept");

// The device verifies it against the root key. A join accept carries no device
// identifier, so only that key decides whether it is for this device.
using LorawanJoinAccept joined = node.AcceptJoin(accept, DevNonce);
Console.WriteLine($"joined    the device took address 0x{joined.DevAddr:X8}");

// Neither side transmits a session key. Both derive the same pair from the root
// key and the two nonces, so the network reads what the device sends without ever
// having been told how.
using LorawanSession network = offer.Session(appKey, DevNonce);
using LorawanSession activated = joined.Session();
byte[] uplink = activated.EncodeUplink(1, 1, "level=high"u8);
LorawanRxData received = network.Decode(uplink, 1);
Console.WriteLine(
    $"uplink    the network read {System.Text.Encoding.UTF8.GetString(received.Payload)}");

// A single byte changed in the air fails that check, so no one else can admit the
// device or put words in its mouth.
byte[] forged = [.. accept];
forged[1] ^= 0xFF;
try
{
    node.AcceptJoin(forged, DevNonce).Dispose();
    Console.WriteLine("a forged accept was taken, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"forged    accept refused: {error.Message}");
}
```
<!-- end -->

## Reference

<!-- table: reference lorawan -->
- Rust: [`pamoja-lorawan`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html)
- TypeScript: [`@pamoja/lorawan`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html)
- Python: [`pamoja.lorawan`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html)
- C#: [`Pamoja.Lorawan`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.html)
<!-- end -->
