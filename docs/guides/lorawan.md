# LoRaWAN

LoRaWAN is the MAC layer a long-range node speaks above a LoRa radio. The band
is public and the range is measured in kilometers, so every frame carries a
message integrity code keyed to the network, proving it is authentic and intact,
and a payload encrypted to the application, readable only by its owner. Both are
AES-128. A device reaches a network with session keys provisioned into it or by
joining over the air, and pamoja runs both ends of that join: the device asking
to be admitted, and the network admitting it. An end device runs the rest of a
node's side on its own: which channel and data rate each frame goes out at, when
and where to listen for the answer, what the network's MAC commands ask of it,
and what to keep across a power cut. It drives no radio itself, so the same
device runs over a concentrator, a radio on a board, or nothing at all.

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

The second example is a whole node on a North American network. The device is
given no radio and no clock, only the time in microseconds at each call, and it
answers with what to put on the air.

It proves:

- A US915 device joins on 902.3 MHz at DR0, SF10, with 370 ms on air and its
  full 20 dBm, and says the accept is due five seconds later on 923.3 MHz, the
  downlink channel RP002-1.0.5 pairs with that join channel.
- A confirmed reading goes out on 906.9 MHz and is answered one second later on
  927.5 MHz. A second reading while the first waits on its windows is refused
  as busy rather than trampling them.
- The network's acknowledgment and the setting it sends back on port 2 are read
  and decrypted.
- The 1589 bytes the device saves before sleeping bring a fresh device back on a
  clock that has started over, and its next reading goes out as uplink 1
  without joining again.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example lorawan" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example lorawan</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- lorawan" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- lorawan</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/lorawan.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/lorawan.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- lorawan" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- lorawan</code></div>
</div>
<!-- end -->

## Rust

A network admitting a device:

<!-- snippet: examples/guides/lorawan.rs#example -->
From [`examples/guides/lorawan.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/lorawan.rs):

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

A node that runs itself:

<!-- snippet: examples/guides/lorawan.rs#device -->
From [`examples/guides/lorawan.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/lorawan.rs):

```rust
use pamoja_lora::region::Region;
use pamoja_lorawan::device::{DeviceError, EndDevice, Heard, Settings};
use pamoja_lorawan::{Device, Downlink, JoinGrant};

let app_key = [7u8; 16];
let dev_eui = [0x70, 0xB3, 0xD5, 0x7E, 0xD0, 0x05, 0x12, 0x34];
let join_eui = [0; 8];

// The device owns no radio and no clock. It takes the time in microseconds and says what
// to put on the air, so the same code runs over an SX1276, an SX1262, or nothing at all.
// This one's radio puts out 2 to 20 dBm, and its seed would come from the radio's noise.
let plan = Region::Us915.plan();
let settings = Settings::new(2, 20).with_seed(1);
let mut node = EndDevice::new(plan, Device::new(dev_eui, join_eui, app_key), settings)?;

// A US915 device joins in passes over the band, one 125 kHz channel from each group of
// eight. The accept is due on the downlink channel that join channel is answered on.
let request = node.join(1, 0)?;
println!(
    "join      {:.1} MHz at DR{}, {} dBm, {} ms on air; the accept is due {} s later on {:.1} MHz",
    f64::from(request.frequency_hz) / 1e6,
    request.data_rate,
    request.output_dbm,
    request.airtime_us / 1000,
    request.rx1.delay_us / 1_000_000,
    f64::from(request.rx1.frequency_hz) / 1e6,
);

// The network answers, and the device takes its address and session from the accept.
let network = JoinGrant::new(2, 19, 0x2601_2E43);
if let Heard::Joined { dev_addr } = node.heard(network.accept(&app_key, 1).as_bytes(), 7)? {
    println!("joined    as {dev_addr:#010X}");
}

// A confirmed reading. While it waits on its windows, the device refuses to send another.
let reading = node.send(2, b"21.5", true, 10_000_000)?;
println!(
    "uplink    {:.1} MHz at DR{}; the answer is due {} s later on {:.1} MHz",
    f64::from(reading.frequency_hz) / 1e6,
    reading.data_rate,
    reading.rx1.delay_us / 1_000_000,
    f64::from(reading.rx1.frequency_hz) / 1e6,
);
if let Err(DeviceError::Busy) = node.send(2, b"21.6", false, 10_000_000) {
    println!("busy      the reading before still waits on its windows");
}

// The network acknowledges it and sends a setting back on the same port.
let answer = network
    .session(&app_key, 1)
    .encode_downlink(&Downlink::new(0, 2, b"set=19.0").with_ack())?;
if let Heard::Data(delivery) = node.heard(answer.as_bytes(), 7)? {
    println!(
        "downlink  acknowledged: {}, port {} says {}",
        delivery.acknowledged(),
        delivery.port().unwrap_or(0),
        String::from_utf8_lossy(delivery.payload()),
    );
}

// Before sleeping, the device saves what it settled with the network. After the power
// cut a fresh device resumes it on a clock that starts over, and sends its next reading
// with no join.
let saved = node.save(12_000_000)?;
let mut woken = EndDevice::new(plan, Device::new(dev_eui, join_eui, app_key), settings)?;
woken.resume(&saved, 0)?;
let next = woken.send(2, b"21.7", false, 5_000_000)?;
println!(
    "resumed   {} saved bytes; the next reading goes out as uplink {} without joining again",
    saved.as_bytes().len(),
    woken.fcnt_up() - 1,
);
```
<!-- end -->

## TypeScript

A network admitting a device:

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

A node that runs itself:

<!-- snippet: bindings/node/guides/lorawan.ts#device -->
From [`bindings/node/guides/lorawan.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/lorawan.ts):

```typescript
import { LoraRegion, planFor } from '@pamoja/lora'
import * as lorawan from '@pamoja/lorawan'

const rootKey = Buffer.alloc(16, 7)
const devEui = Buffer.from('70b3d57ed0051234', 'hex')
const joinEui = Buffer.alloc(8)

// The device owns no radio and no clock. It takes the time in microseconds and says what to
// put on the air, so the same code runs over an SX1276, an SX1262, or nothing at all. This
// one's radio puts out 2 to 20 dBm, and its seed would come from the radio's noise.
const plan = planFor(LoraRegion.Us915)
const settings = { minOutputDbm: 2, maxOutputDbm: 20, seed: 1 }
const mhz = (hz: number) => (hz / 1e6).toFixed(1)
const sensor = lorawan.EndDevice.overTheAir(plan, devEui, joinEui, rootKey, settings)

// A US915 device joins in passes over the band, one 125 kHz channel from each group of eight.
// The accept is due on the downlink channel that join channel is answered on.
const request = sensor.join(1, 0)
console.log(
  `join      ${mhz(request.frequencyHz)} MHz at DR${request.dataRate}, ${request.outputDbm} dBm, ` +
    `${Math.floor(request.airtimeUs / 1000)} ms on air; ` +
    `the accept is due ${request.rx1.delayUs / 1_000_000} s later on ${mhz(request.rx1.frequencyHz)} MHz`,
)

// The network answers, and the device takes its address and session from the accept.
const grant = { appNonce: 2, netId: 19, devAddr: 0x26012e43 }
const heard = sensor.heard(lorawan.grantAccept(grant, rootKey, 1), 7)
if (heard.kind === 'Joined') {
  console.log(`joined    as 0x${heard.devAddr.toString(16).toUpperCase().padStart(8, '0')}`)
}

// A confirmed reading. While it waits on its windows, the device refuses to send another.
const reading = sensor.send(2, '21.5', 10_000_000, true)
console.log(
  `uplink    ${mhz(reading.frequencyHz)} MHz at DR${reading.dataRate}; ` +
    `the answer is due ${reading.rx1.delayUs / 1_000_000} s later on ${mhz(reading.rx1.frequencyHz)} MHz`,
)
try {
  sensor.send(2, '21.6', 10_000_000)
} catch (error) {
  if (lorawan.isDeviceError(error, 'Busy')) {
    console.log('busy      the reading before still waits on its windows')
  }
}

// The network acknowledges it and sends a setting back on the same port.
const answer = lorawan.grantSession(grant, rootKey, 1).encodeDownlink(0, 2, Buffer.from('set=19.0'), { ack: true })
const downlink = sensor.heard(answer, 7)
if (downlink.kind === 'Data') {
  const { acknowledged, port, payload } = downlink.delivery
  console.log(`downlink  acknowledged: ${acknowledged}, port ${port ?? 0} says ${payload.toString()}`)
}

// Before sleeping, the device saves what it settled with the network. After the power cut a
// fresh device resumes it on a clock that starts over, and sends its next reading with no join.
const saved = sensor.save(12_000_000)
const woken = lorawan.EndDevice.overTheAir(plan, devEui, joinEui, rootKey, settings)
woken.resume(saved, 0)
const next = woken.send(2, '21.7', 5_000_000)
console.log(
  `resumed   ${saved.length} saved bytes; ` +
    `the next reading goes out as uplink ${woken.fcntUp - 1} without joining again`,
)
```
<!-- end -->

## Python

A network admitting a device:

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

A node that runs itself:

<!-- snippet: bindings/python/guides/lorawan.py#device -->
From [`bindings/python/guides/lorawan.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/lorawan.py):

```python
from pamoja.lora import plan_for
from pamoja.lorawan import DeviceError, DeviceSettings, end_device

root_key = bytes([7]) * 16
dev_eui = bytes.fromhex("70b3d57ed0051234")
join_eui = bytes(8)

# The device owns no radio and no clock. It takes the time in microseconds and says what to
# put on the air, so the same code runs over an SX1276, an SX1262, or nothing at all. This
# one's radio puts out 2 to 20 dBm, and its seed would come from the radio's noise.
plan = plan_for("US915")
settings = DeviceSettings(2, 20, seed=1)
node = end_device(plan, dev_eui, join_eui, root_key, settings)

# A US915 device joins in passes over the band, one 125 kHz channel from each group of eight.
# The accept is due on the downlink channel that join channel is answered on.
request = node.join(1, 0)
print(
    f"join      {request.frequency_hz / 1e6:.1f} MHz at DR{request.data_rate}, "
    f"{request.output_dbm} dBm, {request.airtime_us // 1000} ms on air; "
    f"the accept is due {request.rx1.delay_us // 1_000_000} s later on "
    f"{request.rx1.frequency_hz / 1e6:.1f} MHz"
)

# The network answers, and the device takes its address and session from the accept.
network = grant(app_nonce=2, net_id=19, dev_addr=0x26012E43)
heard = node.heard(network.accept(root_key, 1), 7)
if heard.kind == "joined":
    print(f"joined    as 0x{heard.dev_addr:08X}")

# A confirmed reading. While it waits on its windows, the device refuses to send another.
reading = node.send(2, b"21.5", 10_000_000, confirmed=True)
print(
    f"uplink    {reading.frequency_hz / 1e6:.1f} MHz at DR{reading.data_rate}; "
    f"the answer is due {reading.rx1.delay_us // 1_000_000} s later on "
    f"{reading.rx1.frequency_hz / 1e6:.1f} MHz"
)
try:
    node.send(2, b"21.6", 10_000_000)
except DeviceError as error:
    if error.kind == "busy":
        print("busy      the reading before still waits on its windows")

# The network acknowledges it and sends a setting back on the same port.
answer = network.session(root_key, 1).encode_downlink(0, 2, b"set=19.0", ack=True)
downlink = node.heard(answer, 7)
if downlink.kind == "data":
    delivery = downlink.delivery
    acknowledged = "true" if delivery.acknowledged else "false"
    print(
        f"downlink  acknowledged: {acknowledged}, port {delivery.port or 0} "
        f"says {delivery.payload.decode()}"
    )

# Before sleeping, the device saves what it settled with the network. After the power cut a
# fresh device resumes it on a clock that starts over, and sends its next reading with no join.
saved = node.save(12_000_000)
woken = end_device(plan, dev_eui, join_eui, root_key, settings)
woken.resume(saved, 0)
following = woken.send(2, b"21.7", 5_000_000)
print(
    f"resumed   {len(saved)} saved bytes; the next reading goes out as uplink "
    f"{woken.fcnt_up - 1} without joining again"
)
```
<!-- end -->

## C#

A network admitting a device:

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

A node that runs itself:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs#device -->
From [`bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs):

```csharp
byte[] rootKey = new byte[16];
Array.Fill(rootKey, (byte)7);
byte[] devEui = Convert.FromHexString("70B3D57ED0051234");
byte[] joinEui = new byte[8];

// The device owns no radio and no clock. It takes the time in microseconds and says
// what to put on the air, so the same code runs over an SX1276, an SX1262, or nothing
// at all. This one's radio puts out 2 to 20 dBm, and its seed would come from the
// radio's noise.
using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Us915);
var settings = new LorawanDeviceSettings(2, 20) { Seed = 1 };
static string Mhz(uint hz) => (hz / 1e6).ToString("F1", CultureInfo.InvariantCulture);
using var credentials = new LorawanDevice(devEui, joinEui, rootKey);
using LorawanEndDevice node = LorawanEndDevice.OverTheAir(plan, credentials, settings);

// A US915 device joins in passes over the band, one 125 kHz channel from each group
// of eight. The accept is due on the downlink channel that join channel is answered on.
LorawanTransmission request = node.Join(1, 0);
Console.WriteLine(
    $"join      {Mhz(request.FrequencyHz)} MHz at DR{request.DataRate}, {request.OutputDbm} dBm, " +
    $"{request.AirtimeMicros / 1000} ms on air; " +
    $"the accept is due {request.Rx1.DelayMicros / 1_000_000} s later on {Mhz(request.Rx1.FrequencyHz)} MHz");

// The network answers, and the device takes its address and session from the accept.
var network = new LorawanGrant(appNonce: 2, netId: 19, devAddr: 0x26012E43);
if (node.Heard(network.Accept(rootKey, 1), 7) is LorawanHeard.Joined joined)
{
    Console.WriteLine($"joined    as 0x{joined.DevAddr:X8}");
}

// A confirmed reading. While it waits on its windows, the device refuses to send another.
LorawanTransmission reading = node.Send(2, "21.5"u8, 10_000_000, confirmed: true);
Console.WriteLine(
    $"uplink    {Mhz(reading.FrequencyHz)} MHz at DR{reading.DataRate}; " +
    $"the answer is due {reading.Rx1.DelayMicros / 1_000_000} s later on {Mhz(reading.Rx1.FrequencyHz)} MHz");
try
{
    node.Send(2, "21.6"u8, 10_000_000);
}
catch (LorawanDeviceException error) when (error.Kind == LorawanDeviceErrorKind.Busy)
{
    Console.WriteLine("busy      the reading before still waits on its windows");
}

// The network acknowledges it and sends a setting back on the same port.
using LorawanSession networkSession = network.Session(rootKey, 1);
byte[] answer = networkSession.EncodeDownlink(0, 2, "set=19.0"u8, new LorawanOptions { Ack = true });
if (node.Heard(answer, 7) is LorawanHeard.Data data)
{
    LorawanDelivery delivery = data.Delivery;
    string acknowledged = delivery.Acknowledged ? "true" : "false";
    Console.WriteLine(
        $"downlink  acknowledged: {acknowledged}, port {delivery.Port ?? 0} says {Encoding.UTF8.GetString(delivery.Payload)}");
}

// Before sleeping, the device saves what it settled with the network. After the power
// cut a fresh device resumes it on a clock that starts over, and sends its next reading
// with no join.
byte[] saved = node.Save(12_000_000);
using LorawanEndDevice woken = LorawanEndDevice.OverTheAir(plan, credentials, settings);
woken.Resume(saved, 0);
LorawanTransmission next = woken.Send(2, "21.7"u8, 5_000_000);
Console.WriteLine(
    $"resumed   {saved.Length} saved bytes; the next reading goes out as uplink {woken.FcntUp - 1} without joining again");
```
<!-- end -->

## Reference

<!-- table: reference lorawan -->
- Rust: [`pamoja-lorawan`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-lorawan)
- TypeScript: [`@pamoja/lorawan`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-lorawan)
- Python: [`pamoja.lorawan`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-lorawan)
- C#: [`Pamoja.Lorawan`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-lorawan)
<!-- end -->
