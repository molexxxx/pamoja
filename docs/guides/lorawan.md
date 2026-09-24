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
  and decrypted from the first window, whose data rate the frame's length is
  held to.
- The 1678 bytes the device saves before sleeping bring a fresh device back on a
  clock that has started over, and its next reading goes out as uplink 1
  without joining again.

The third example puts a sensor where no gateway can hear it, and a relay on a
rooftop that can. TS011-1.0.1 calls this a relay: an ordinary end device that
also wakes every scan period to listen for the devices around it, forwards what
they send on port 226, and passes the network's answer back.

It proves:

- The relay joins the network exactly as any device does, and the network then
  tells it which devices to carry, sending the key that lets it verify their
  wake-up frames in a MAC command riding on the relay's own downlink.
- A sensor in relay mode sends nothing straight to a gateway. Its uplink goes
  out behind a wake-on-radio frame whose 259-symbol preamble spans the relay's
  whole one-second scan, because it does not yet know when that scan happens.
- The relay's acknowledgment says when it scanned and how fast it forwards, so
  from then on the sensor aims at the scan and sends the shortest preamble the
  two clocks allow.
- The forwarded reading reaches the network as the sensor's own frame, with the
  relay's address and the signal it heard alongside, and the answer comes back
  through the relay into the third receive window a relayed device keeps open,
  18 seconds after its uplink.

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

In Rust, `pamoja-lorawan` is `no_std`. `Device` and `JoinGrant` are the two ends of
the join, and `Session` encodes and decodes data frames built as `Uplink` and
`Downlink`. `device::EndDevice` runs a whole node over a `ChannelPlan` from
`pamoja-lora`, with a `Settings` of the radio's power range and a seed. It takes the
caller's time in microseconds and the SNR each frame was heard at, and answers with a
`Transmission` to put on the air or a `Heard` to act on; a refusal is a `DeviceError`.
`relay::Relay` wraps an end device so it carries others under TS011-1.0.1, and the
network side of a whole site is `Network` in `pamoja-gateway`.

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
use pamoja_lorawan::device::{DeviceError, EndDevice, Heard, ReceiveWindow, Settings};
use pamoja_lorawan::{Device, Downlink, JoinGrant};

let app_key = [7u8; 16];
let dev_eui = 0x70B3_D57E_D005_1234u64.to_be_bytes();
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

// The network acknowledges it in the first window and sends a setting back on the same
// port. Naming the window holds the frame to the length that window's data rate carries.
let answer = network
    .session(&app_key, 1)
    .encode_downlink(&Downlink::new(0, 2, b"set=19.0").with_ack())?;
if let Heard::Data(delivery) = node.heard_in(ReceiveWindow::Rx1, answer.as_bytes(), 7)? {
    let reading_was = if delivery.acknowledged() {
        "acknowledged"
    } else {
        "not acknowledged"
    };
    println!(
        "downlink  the reading was {reading_was}, and port {} says {}",
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

A sensor carried by the relay next door:

<!-- snippet: examples/guides/lorawan.rs#relay -->
From [`examples/guides/lorawan.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/lorawan.rs):

```rust
use pamoja_gateway::network::{Event, Network, Registration};
use pamoja_gateway::udp::Rxpk;
use pamoja_lora::region::Region;
use pamoja_lorawan::device::{EndDevice, Heard, ReceiveWindow, Settings};
use pamoja_lorawan::relay::{
    CadPeriodicity, CadToRx, Relay, RelayConfig, RelayHeard, RelaySettings, Wake, XtalAccuracy,
};
use pamoja_lorawan::Device;

// One site: a gateway on a hill, a relay on a rooftop in range of it, and a sensor in a
// cellar the gateway cannot hear at all.
let app_key = [7u8; 16];
let relay_eui = 0x70B3_D57E_D005_0001u64.to_be_bytes();
let sensor_eui = 0x70B3_D57E_D005_0002u64.to_be_bytes();
let plan = Region::Eu868.plan();
let settings = Settings::new(2, 14)
    .with_tuning_range(863_000_000, 870_000_000)
    .with_seed(1);
let mut site = Network::new(plan, 0x00_002A).with_first_dev_addr(0x2601_0001);
site.register(Registration::new(relay_eui, [0; 8], app_key));
site.register(Registration::new(sensor_eui, [0; 8], app_key));
let heard_at = |transmission: &pamoja_lorawan::device::Transmission, at_us: u32| {
    Rxpk::new(
        transmission.frequency_hz,
        transmission.link,
        transmission.frame.as_bytes().to_vec(),
    )
    .with_timestamp_us(at_us)
};

// The relay is an end device that also listens for others, so it joins the ordinary way.
let device = EndDevice::new(plan, Device::new(relay_eui, [0; 8], app_key), settings)?;
let mut rooftop = Relay::new(
    device,
    RelaySettings::new(XtalAccuracy::Ppm20, CadToRx::Symbols4),
);
let join = rooftop.device_mut().join(1, 1_000_000)?;
let Event::Joined { accept, .. } = site.uplink(&heard_at(&join, 1_000_000))? else {
    panic!("the relay is registered");
};
rooftop.heard_in(ReceiveWindow::Rx1, &accept.payload, 7)?;
let relay_addr = rooftop.device().dev_addr().expect("an address");
println!("relay     joined as {relay_addr:#010X}");

// The sensor joins too. Its own uplinks never reach the gateway, but its join does,
// because the cellar door is open while it is installed.
let mut cellar = EndDevice::new(plan, Device::new(sensor_eui, [0; 8], app_key), settings)?;
let sensor_join = cellar.join(2, 20_000_000)?;
let Event::Joined { accept, .. } = site.uplink(&heard_at(&sensor_join, 20_000_000))? else {
    panic!("the sensor is registered");
};
cellar.heard_in(ReceiveWindow::Rx1, &accept.payload, 7)?;
let sensor_addr = cellar.dev_addr().expect("an address");

// The network hands the relay the key that lets it verify the sensor's wake-up frames,
// in a command riding on the relay's own downlink.
let empty = rooftop.device_mut().send_empty(40_000_000)?;
let Event::Data { slot, .. } = site.uplink(&heard_at(&empty, 40_000_000))? else {
    panic!("the relay's own uplink");
};
let trust = site.trust_command(sensor_addr, 0, 63, 0)?;
let configure = site.command(relay_addr, slot, &[trust])?;
rooftop.heard_in(ReceiveWindow::Rx1, &configure.payload, 7)?;
println!("trusted   the relay now forwards for {sensor_addr:#010X}");

// It scans once a second on the region's wake-on-radio channel.
rooftop.start(RelayConfig::new(
    CadPeriodicity::Ms1000,
    rooftop.region_channel(0).expect("a channel"),
))?;
let scan = rooftop.next_scan(60_000_000).expect("a scan");
println!(
    "scan      {:.1} MHz at DR{} every second",
    f64::from(scan.carrier.frequency_hz) / 1e6,
    scan.carrier.data_rate,
);

// The sensor turns relay mode on. Its uplink now goes out behind a frame whose preamble
// spans a whole scan period, because it does not yet know when the relay listens.
cellar.use_relay(true);
let reading = cellar.send(2, b"21.5", false, 61_000_000)?;
let exchange = reading.relay.expect("a wake-on-radio exchange");
println!(
    "wake      {} bytes with a {}-symbol preamble, {} ms before the uplink",
    exchange.wake_up.frame().len(),
    exchange.wake_up.link.preamble_symbols(),
    (exchange.uplink_start_us - exchange.wake_up.start_us) / 1000,
);

// The relay hears it, knows the device, and answers with when it scanned, so every frame
// after this one carries only the preamble the two clocks could have drifted apart.
let Wake::Uplink {
    acknowledgment: Some(ack),
    listen: Some(listen),
    ..
} = rooftop.heard_wor(
    &scan,
    exchange.wake_up.frame(),
    -90,
    4,
    scan.start_us + 500_000,
)?
else {
    panic!("the relay knows this device");
};
let said = cellar.heard_wor_ack(&ack.frame)?;
println!(
    "ack       the relay scans every {} ms and forwards at DR{}",
    said.cad_periodicity.period_us() / 1000,
    said.relay_data_rate,
);

// The uplink follows, and the relay wraps it in one of its own on port 226.
let due_us =
    rooftop.heard_uplink(reading.frame.as_bytes(), -88, 6, listen.start_us + 100_000)?;
let forwarded = rooftop.forward(due_us)?;
let Event::Data {
    dev_addr,
    payload,
    slot,
    relay: Some(relayed),
    ..
} = site.uplink(&heard_at(&forwarded, due_us as u32))?
else {
    panic!("a forwarded uplink");
};
println!(
    "forwarded {} from {dev_addr:#010X}, heard by {:#010X} at {} dBm",
    String::from_utf8_lossy(&payload),
    relayed.relay,
    relayed.metadata.rssi_dbm,
);

// The answer goes back the same way: the network answers the sensor, the relay unwraps it
// and sends it on, and the sensor hears it in the window it keeps for a relay.
let answer = site.answer(sensor_addr, slot, 2, b"set=19.0")?;
let RelayHeard::Downlink { downlink, .. } =
    rooftop.heard_in(ReceiveWindow::Rx1, &answer.payload, 7)?
else {
    panic!("a downlink for the sensor");
};
if let Heard::Data(delivery) = cellar.heard_in(ReceiveWindow::Rxr, downlink.frame(), 7)? {
    println!(
        "downlink  port {} says {}, {} s after the uplink",
        delivery.port().unwrap_or(0),
        String::from_utf8_lossy(delivery.payload()),
        exchange.rxr.delay_us / 1_000_000,
    );
}
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/lorawan` holds the join as `device(devEui, joinEui, appKey)`
on one side and a grant object passed to `grantAccept` and `grantSession` on the
other. A `Session`'s `encodeUplink`, `encodeDownlink`, and `decode` take the frame
counter, the port, and the payload directly. `EndDevice.overTheAir(plan, devEui,
joinEui, appKey, settings)` runs a node: `send` takes the time before the confirmed
flag, and `heard(frame, snrDb, window?)` returns a `Heard` tagged by `kind`. A
refusal throws a `DeviceError` whose `code` names it, and `isDeviceError(error,
'Busy')` tests for one. Keys and EUIs are `Uint8Array`s; addresses and counters are
numbers.

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

// The network acknowledges it in the first window and sends a setting back on the same
// port. Naming the window holds the frame to the length that window's data rate carries.
const answer = lorawan.grantSession(grant, rootKey, 1).encodeDownlink(0, 2, Buffer.from('set=19.0'), { ack: true })
const downlink = sensor.heard(answer, 7, lorawan.ReceiveWindow.Rx1)
if (downlink.kind === 'Data') {
  const { acknowledged, port, payload } = downlink.delivery
  const readingWas = acknowledged ? 'acknowledged' : 'not acknowledged'
  console.log(`downlink  the reading was ${readingWas}, and port ${port ?? 0} says ${payload.toString()}`)
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

A sensor carried by the relay next door:

<!-- snippet: bindings/node/guides/lorawan.ts#relay -->
From [`bindings/node/guides/lorawan.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/lorawan.ts):

```typescript
import { Network } from '@pamoja/gateway'
import { CadPeriodicity, CadToRx, Relay, XtalAccuracy } from '@pamoja/lorawan'

// One site: a gateway on a hill, a relay on a rooftop in range of it, and a sensor in a
// cellar the gateway cannot hear at all.
const relayEui = Buffer.from('70b3d57ed0050001', 'hex')
const sensorEui = Buffer.from('70b3d57ed0050002', 'hex')
const band = planFor(LoraRegion.Eu868)
const radio = { minOutputDbm: 2, maxOutputDbm: 14, lowestHz: 863_000_000, highestHz: 870_000_000, seed: 1 }
const site = new Network(band, 0x00002a, null, 0x26010001)
site.register(relayEui, joinEui, rootKey)
site.register(sensorEui, joinEui, rootKey)
const heardAt = (transmission: lorawan.Transmission, atUs: number) => ({
  frequencyHz: transmission.frequencyHz,
  payload: transmission.frame,
  link: transmission.link,
  timestampUs: atUs,
})
const address = (value: number) => `0x${value.toString(16).toUpperCase().padStart(8, '0')}`

// The relay is an end device that also listens for others, so it joins the ordinary way.
const rooftop = Relay.overTheAir(band, relayEui, joinEui, rootKey, radio, XtalAccuracy.Ppm20, CadToRx.Symbols4)
const relayJoin = rooftop.join(1, 1_000_000)
const relayAccept = site.uplink(heardAt(relayJoin, 1_000_000))
rooftop.heardIn(lorawan.ReceiveWindow.Rx1, relayAccept.accept!.payload, 7)
console.log(`relay     joined as ${address(rooftop.devAddr!)}`)

// The sensor joins too. Its own uplinks never reach the gateway, but its join does, because
// the cellar door is open while it is installed.
const cellar = lorawan.EndDevice.overTheAir(band, sensorEui, joinEui, rootKey, radio)
const sensorJoin = cellar.join(2, 20_000_000)
const sensorAccept = site.uplink(heardAt(sensorJoin, 20_000_000))
cellar.heard(sensorAccept.accept!.payload, 7, lorawan.ReceiveWindow.Rx1)
const sensorAddr = cellar.devAddr!

// The network hands the relay the key that lets it verify the sensor's wake-up frames, in a
// command riding on the relay's own downlink.
const relayEmpty = rooftop.sendEmpty(40_000_000)
const relayCarried = site.uplink(heardAt(relayEmpty, 40_000_000))
const trust = site.trustCommand(sensorAddr, 0, 63, 0)
const configure = site.command(rooftop.devAddr!, relayCarried.slot!, [trust])
rooftop.heardIn(lorawan.ReceiveWindow.Rx1, configure.payload, 7)
console.log(`trusted   the relay now forwards for ${address(sensorAddr)}`)

// It scans once a second on the region's wake-on-radio channel.
rooftop.start(CadPeriodicity.Ms1000, 0)
const scan = rooftop.nextScan(60_000_000)!
console.log(`scan      ${mhz(scan.carrier.frequencyHz)} MHz at DR${scan.carrier.dataRate} every second`)

// The sensor turns relay mode on. Its uplink now goes out behind a frame whose preamble spans
// a whole scan period, because it does not yet know when the relay listens.
cellar.useRelay(true)
const relayedReading = cellar.send(2, '21.5', 61_000_000)
const exchange = relayedReading.relay!
console.log(
  `wake      ${exchange.wakeUp.frame.length} bytes with a ${exchange.wakeUp.link.preambleSymbols}-symbol ` +
    `preamble, ${Math.trunc((exchange.uplinkStartUs - exchange.wakeUp.startUs) / 1000)} ms before the uplink`,
)

// The relay hears it, knows the device, and answers with when it scanned, so every frame after
// this one carries only the preamble the two clocks could have drifted apart.
const woke = rooftop.heardWor(scan, exchange.wakeUp.frame, -90, 4, scan.startUs + 500_000)
if (woke.kind !== 'Uplink') {
  throw new Error('the relay knows this device')
}
const said = cellar.heardWorAck(woke.acknowledgment!.frame)
console.log(`ack       the relay scans every ${periodMs(said.cadPeriodicity)} ms and forwards at DR${said.relayDataRate}`)

// The uplink follows, and the relay wraps it in one of its own on port 226.
const dueUs = rooftop.heardUplink(relayedReading.frame, -88, 6, woke.listen!.startUs + 100_000)
const forwarded = rooftop.forward(dueUs)
const relayed = site.uplink(heardAt(forwarded, dueUs))
console.log(
  `forwarded ${relayed.payload!.toString()} from ${address(relayed.devAddr)}, ` +
    `heard by ${address(relayed.relay!.relay)} at ${relayed.relay!.rssiDbm} dBm`,
)

// The answer goes back the same way: the network answers the sensor, the relay unwraps it and
// sends it on, and the sensor hears it in the window it keeps for a relay.
const relayedAnswer = site.answer(sensorAddr, relayed.slot!, 2, Buffer.from('set=19.0'))
const passed = rooftop.heardIn(lorawan.ReceiveWindow.Rx1, relayedAnswer.payload, 7)
if (passed.kind !== 'Downlink') {
  throw new Error('a downlink for the sensor')
}
const delivered = cellar.heard(passed.downlink.frame, 7, lorawan.ReceiveWindow.Rxr)
if (delivered.kind === 'Data') {
  console.log(
    `downlink  port ${delivered.delivery.port ?? 0} says ${delivered.delivery.payload.toString()}, ` +
      `${Math.trunc(exchange.rxr.delayUs / 1_000_000)} s after the uplink`,
  )
}
```
<!-- end -->

## Python

In Python, `pamoja.lorawan` gives `device(...)` and `grant(app_nonce=, net_id=,
dev_addr=)` for the join, and a `Session` with `encode_uplink(fcnt, port,
payload)`, `encode_downlink(fcnt, port, payload, ack=True)`, and `decode(frame,
fcnt)`. `end_device(...)` builds a whole node from a plan, the credentials, and
`DeviceSettings(min_dbm, max_dbm, seed=...)`. A refusal raises
`LorawanDeviceError`, whose `kind` names it, such as `busy`, with `until_us` or
`max` alongside where they apply. Keys, EUIs, and frames are `bytes`.

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
from pamoja.lorawan import DeviceError, DeviceSettings, ReceiveWindow, end_device

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

# The network acknowledges it in the first window and sends a setting back on the same
# port. Naming the window holds the frame to the length that window's data rate carries.
answer = network.session(root_key, 1).encode_downlink(0, 2, b"set=19.0", ack=True)
downlink = node.heard(answer, 7, ReceiveWindow.RX1)
if downlink.kind == "data":
    delivery = downlink.delivery
    reading_was = "acknowledged" if delivery.acknowledged else "not acknowledged"
    print(
        f"downlink  the reading was {reading_was}, and port {delivery.port or 0} "
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

A sensor carried by the relay next door:

<!-- snippet: bindings/python/guides/lorawan.py#relay -->
From [`bindings/python/guides/lorawan.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/lorawan.py):

```python
from pamoja.gateway import Network, Rxpk
from pamoja.lorawan import Relay

# One site: a gateway on a hill, a relay on a rooftop in range of it, and a sensor in a
# cellar the gateway cannot hear at all.
relay_eui = bytes.fromhex("70b3d57ed0050001")
sensor_eui = bytes.fromhex("70b3d57ed0050002")
band = plan_for("EU868")
radio = DeviceSettings(2, 14, lowest_hz=863_000_000, highest_hz=870_000_000, seed=1)
site = Network(band, 0x00002A, first_dev_addr=0x26010001)
site.register(relay_eui, join_eui, root_key)
site.register(sensor_eui, join_eui, root_key)


def heard_at(transmission, at_us):
    """Hand the network what a gateway heard of one transmission."""
    return Rxpk(
        transmission.frequency_hz,
        transmission.frame,
        link=transmission.link,
        timestamp_us=at_us,
    )


# The relay is an end device that also listens for others, so it joins the ordinary way.
rooftop = Relay.over_the_air(
    band, device(relay_eui, join_eui, root_key), radio, "ppm20", "symbols4"
)
relay_join = rooftop.join(1, 1_000_000)
relay_accept = site.uplink(heard_at(relay_join, 1_000_000))
rooftop.heard_in("rx1", relay_accept.accept.payload, 7)
print(f"relay     joined as 0x{rooftop.dev_addr:08X}")

# The sensor joins too. Its own uplinks never reach the gateway, but its join does, because
# the cellar door is open while it is installed.
cellar = end_device(band, sensor_eui, join_eui, root_key, radio)
sensor_join = cellar.join(2, 20_000_000)
sensor_accept = site.uplink(heard_at(sensor_join, 20_000_000))
cellar.heard(sensor_accept.accept.payload, 7, ReceiveWindow.RX1)
sensor_addr = cellar.dev_addr

# The network hands the relay the key that lets it verify the sensor's wake-up frames, in a
# command riding on the relay's own downlink.
relay_empty = rooftop.send_empty(40_000_000)
relay_carried = site.uplink(heard_at(relay_empty, 40_000_000))
trust = site.trust_command(sensor_addr, 0)
configure = site.command(rooftop.dev_addr, relay_carried.slot, [trust])
rooftop.heard_in("rx1", configure.payload, 7)
print(f"trusted   the relay now forwards for 0x{sensor_addr:08X}")

# It scans once a second on the region's wake-on-radio channel.
rooftop.start("ms1000", 0)
scan = rooftop.next_scan(60_000_000)
print(f"scan      {scan.carrier.frequency_hz / 1e6:.1f} MHz at DR{scan.carrier.data_rate} every second")

# The sensor turns relay mode on. Its uplink now goes out behind a frame whose preamble spans
# a whole scan period, because it does not yet know when the relay listens.
cellar.use_relay(True)
relayed_reading = cellar.send(2, b"21.5", 61_000_000)
exchange = relayed_reading.relay
print(
    f"wake      {len(exchange.wake_up.frame)} bytes with a "
    f"{exchange.wake_up.link.preamble_symbols}-symbol preamble, "
    f"{(exchange.uplink_start_us - exchange.wake_up.start_us) // 1000} ms before the uplink"
)

# The relay hears it, knows the device, and answers with when it scanned, so every frame after
# this one carries only the preamble the two clocks could have drifted apart.
woke = rooftop.heard_wor(scan, exchange.wake_up.frame, -90, 4, scan.start_us + 500_000)
said = cellar.heard_wor_ack(woke.acknowledgment.frame)
print(
    f"ack       the relay scans every {int(said.cad_periodicity[2:])} ms and forwards at "
    f"DR{said.relay_data_rate}"
)

# The uplink follows, and the relay wraps it in one of its own on port 226.
due_us = rooftop.heard_uplink(relayed_reading.frame, -88, 6, woke.listen.start_us + 100_000)
forwarded = rooftop.forward(due_us)
relayed = site.uplink(heard_at(forwarded, due_us))
print(
    f"forwarded {relayed.payload.decode()} from 0x{relayed.dev_addr:08X}, "
    f"heard by 0x{relayed.relay.relay:08X} at {relayed.relay.rssi_dbm} dBm"
)

# The answer goes back the same way: the network answers the sensor, the relay unwraps it and
# sends it on, and the sensor hears it in the window it keeps for a relay.
relayed_answer = site.answer(sensor_addr, relayed.slot, 2, b"set=19.0")
passed = rooftop.heard_in("rx1", relayed_answer.payload, 7)
delivered = cellar.heard(passed.downlink.frame, 7, ReceiveWindow.RXR)
print(
    f"downlink  port {delivered.delivery.port} says {delivered.delivery.payload.decode()}, "
    f"{exchange.rxr.delay_us // 1_000_000} s after the uplink"
)
```
<!-- end -->

## C#

In C#, `Pamoja.Lorawan` holds `LorawanDevice` and `LorawanGrant` for the join and
`LorawanSession` for data frames, each disposable where it holds native state.
`LorawanEndDevice.OverTheAir(plan, credentials, settings)` runs a node with a
`LorawanDeviceSettings`; `Heard` returns a `LorawanHeard` to match with
`is LorawanHeard.Data data`, and a refusal throws `LorawanDeviceException` with its
`Kind`. `LorawanRelayNode` carries others. Keys and identifiers are checked for
their width before the engine reads them.

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

// The network acknowledges it in the first window and sends a setting back on the same
// port. Naming the window holds the frame to the length that window's data rate carries.
using LorawanSession networkSession = network.Session(rootKey, 1);
byte[] answer = networkSession.EncodeDownlink(0, 2, "set=19.0"u8, new LorawanOptions { Ack = true });
if (node.Heard(answer, 7, LorawanReceiveWindow.Rx1) is LorawanHeard.Data data)
{
    LorawanDelivery delivery = data.Delivery;
    string readingWas = delivery.Acknowledged ? "acknowledged" : "not acknowledged";
    Console.WriteLine(
        $"downlink  the reading was {readingWas}, and port {delivery.Port ?? 0} says {Encoding.UTF8.GetString(delivery.Payload)}");
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

A sensor carried by the relay next door:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs#relay -->
From [`bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs):

```csharp
// One site: a gateway on a hill, a relay on a rooftop in range of it, and a sensor
// in a cellar the gateway cannot hear at all.
byte[] rootKey = new byte[16];
Array.Fill(rootKey, (byte)7);
byte[] joinEui = new byte[8];
byte[] relayEui = Convert.FromHexString("70B3D57ED0050001");
byte[] sensorEui = Convert.FromHexString("70B3D57ED0050002");
using LoraChannelPlan band = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
var radio = new LorawanDeviceSettings(2, 14)
{
    LowestHz = 863_000_000,
    HighestHz = 870_000_000,
    Seed = 1,
};
using var site = new GatewayNetwork(band, 0x00002A, firstDevAddr: 0x26010001);
site.Register(relayEui, joinEui, rootKey);
site.Register(sensorEui, joinEui, rootKey);
static string Mhz(uint hz) => (hz / 1e6).ToString("F1", CultureInfo.InvariantCulture);
static GatewayRxpk HeardAt(LorawanTransmission transmission, uint atMicros) =>
    new(transmission.FrequencyHz, transmission.Frame)
    {
        Link = transmission.Link,
        TimestampMicros = atMicros,
    };

// The relay is an end device that also listens for others, so it joins the ordinary way.
using var relayCredentials = new LorawanDevice(relayEui, joinEui, rootKey);
using LorawanRelayNode rooftop = LorawanRelayNode.OverTheAir(
    band, relayCredentials, radio, LorawanXtalAccuracy.Ppm20, LorawanCadToRx.Symbols4);
LorawanTransmission relayJoin = rooftop.Join(1, 1_000_000);
GatewayNetworkEvent relayAccept = site.Uplink(HeardAt(relayJoin, 1_000_000));
rooftop.HeardIn(LorawanReceiveWindow.Rx1, relayAccept.Accept!.Payload, 7);
Console.WriteLine($"relay     joined as 0x{rooftop.DevAddr!.Value:X8}");

// The sensor joins too. Its own uplinks never reach the gateway, but its join does,
// because the cellar door is open while it is installed.
using var sensorCredentials = new LorawanDevice(sensorEui, joinEui, rootKey);
using LorawanEndDevice cellar = LorawanEndDevice.OverTheAir(band, sensorCredentials, radio);
LorawanTransmission sensorJoin = cellar.Join(2, 20_000_000);
GatewayNetworkEvent sensorAccept = site.Uplink(HeardAt(sensorJoin, 20_000_000));
cellar.Heard(sensorAccept.Accept!.Payload, 7, LorawanReceiveWindow.Rx1);
uint sensorAddr = cellar.DevAddr!.Value;

// The network hands the relay the key that lets it verify the sensor's wake-up frames,
// in a command riding on the relay's own downlink.
LorawanTransmission relayEmpty = rooftop.SendEmpty(40_000_000);
GatewayNetworkEvent relayCarried = site.Uplink(HeardAt(relayEmpty, 40_000_000));
LorawanMacCommand trust = site.TrustCommand(sensorAddr, 0);
GatewayTxpk configure = site.Command(rooftop.DevAddr!.Value, relayCarried.Slot!, [trust]);
rooftop.HeardIn(LorawanReceiveWindow.Rx1, configure.Payload, 7);
Console.WriteLine($"trusted   the relay now forwards for 0x{sensorAddr:X8}");

// It scans once a second on the region's wake-on-radio channel.
rooftop.Start(LorawanCadPeriodicity.Ms1000, 0);
LorawanScan scan = rooftop.NextScan(60_000_000)!;
Console.WriteLine(
    $"scan      {Mhz(scan.Carrier.FrequencyHz)} MHz at DR{scan.Carrier.DataRate} every second");

// The sensor turns relay mode on. Its uplink now goes out behind a frame whose preamble
// spans a whole scan period, because it does not yet know when the relay listens.
cellar.UseRelay(true);
LorawanTransmission reading = cellar.Send(2, "21.5"u8, 61_000_000);
LorawanRelayExchange exchange = reading.Relay!;
Console.WriteLine(
    $"wake      {exchange.WakeUp.Frame.Length} bytes with a " +
    $"{exchange.WakeUp.Link.PreambleSymbols}-symbol preamble, " +
    $"{(exchange.UplinkStartMicros - exchange.WakeUp.StartMicros) / 1000} ms before the uplink");

// The relay hears it, knows the device, and answers with when it scanned, so every
// frame after this one carries only the preamble the two clocks could have drifted apart.
var woke = (LorawanWake.Uplink)rooftop.HeardWor(
    scan, exchange.WakeUp.Frame, -90, 4, scan.StartMicros + 500_000);
LorawanRelayStatus said = cellar.HeardWorAck(woke.Acknowledgment!.Frame);
Console.WriteLine(
    $"ack       the relay scans every {PeriodMillis(said.CadPeriodicity)} ms and " +
    $"forwards at DR{said.RelayDataRate}");

// The uplink follows, and the relay wraps it in one of its own on port 226.
ulong dueUs = rooftop.HeardUplink(reading.Frame, -88, 6, woke.Listen!.StartMicros + 100_000);
LorawanTransmission forwarded = rooftop.Forward(dueUs);
GatewayNetworkEvent relayed = site.Uplink(HeardAt(forwarded, (uint)dueUs));
Console.WriteLine(
    $"forwarded {Encoding.UTF8.GetString(relayed.Payload!)} from 0x{relayed.DevAddr:X8}, " +
    $"heard by 0x{relayed.Relay!.Relay:X8} at {relayed.Relay!.RssiDbm} dBm");

// The answer goes back the same way: the network answers the sensor, the relay unwraps
// it and sends it on, and the sensor hears it in the window it keeps for a relay.
GatewayTxpk answer = site.Answer(sensorAddr, relayed.Slot!, 2, "set=19.0"u8);
var passed = (LorawanRelayHeard.Downlink)rooftop.HeardIn(
    LorawanReceiveWindow.Rx1, answer.Payload, 7);
var delivered = (LorawanHeard.Data)cellar.Heard(
    passed.Forwarded.Frame, 7, LorawanReceiveWindow.Rxr);
Console.WriteLine(
    $"downlink  port {delivered.Delivery.Port} says " +
    $"{Encoding.UTF8.GetString(delivered.Delivery.Payload)}, " +
    $"{exchange.Rxr.DelayMicros / 1_000_000} s after the uplink");
```
<!-- end -->

## Values at a glance

**What each end holds.** Only the root key is secret before the join, and the two
session keys are derived at each end, never sent:

| Value | Size | Where it comes from |
| --- | --- | --- |
| DevEUI | 8 bytes | the device's own identifier, printed on it |
| JoinEUI | 8 bytes | the join server's identifier; zeros where there is none |
| AppKey | 16 bytes | the root key, provisioned into the device and known to the network |
| DevNonce | 2 bytes | a number the device has not used before, in each join request |
| AppNonce | 3 bytes | the network's own number, in each accept |
| NetID | 3 bytes | the network the device joins |
| DevAddr | 4 bytes | the address the network assigns, read out of the accept |
| NwkSKey and AppSKey | 16 bytes each | derived from the root key and both nonces |

**The timings,** from RP002-1.0.5 and LoRaWAN 1.0.3:

| Timing | Value |
| --- | --- |
| first receive window | 1 s after the uplink ends |
| second receive window | 2 s after it |
| join accept windows | 5 s and 6 s after the request |
| how early or late a window may open | 20 microseconds |
| a confirmed uplink's retry, unanswered | 1 to 3 s, drawn at random |
| a relayed device's third window | up to 18 s after its uplink |

**The counters:**

| Counter | Value | What it does |
| --- | --- | --- |
| the largest gap a receiver follows | 16,384 | a frame further ahead is refused, so a captured one cannot be replayed early |
| uplinks before asking the network to answer | 64 | the adaptive data rate check |
| more before stepping the data rate down | 32 | and between each step after that |

**The ports:**

| Port | Carries |
| --- | --- |
| 0 | MAC commands only, encrypted with the network session key |
| 1 to 223 | the application's data, encrypted with the application session key |
| 224 | the MAC layer test protocol |
| 226 | a relay's frames to and from its network, under TS011-1.0.1 |

**What an end device refuses,** named `DeviceError::Busy` in Rust, `code` `Busy`
in TypeScript, `kind` `busy` in Python, and `LorawanDeviceErrorKind.Busy` in C#:

| Refusal | Means |
| --- | --- |
| NotJoined | there is no session yet; join first |
| NoCredentials | a device activated by personalization has nothing to join with |
| Busy | a transmission still waits on its receive windows |
| NothingPending | nothing waits on its windows or is due to repeat |
| Wait | the air is not free until the time given with it |
| NoChannel | no enabled channel carries the data rate |
| DataRate | the data rate is not a LoRa one the device can use here |
| PayloadTooLong | the payload does not fit a frame at this data rate; the most that does is given |
| CounterExhausted | the uplink counter is spent, and the device has to join again |
| Frame | the frame did not decode |
| Foreign | the frame is addressed to another device |
| Replayed | the frame repeats or precedes the last downlink the device took |
| CounterGap | the frame counter jumped 16,384 or more ahead, which only LoRaWAN 1.0.3 refuses |
| Refused | a join accept carries settings the region does not allow |
| State | a saved state could not be resumed |
| TooManyChannels | the plan defines more channels than a device keeps |

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| ask to join | `Device::new(dev_eui, join_eui, app_key)`, `join_request(dev_nonce)` |
| grant a join | `JoinGrant::new(app_nonce, net_id, dev_addr)`, `accept(&app_key, dev_nonce)`, `session(&app_key, dev_nonce)` |
| take an accept | `accept_join(frame, dev_nonce)`, then `dev_addr()`, `session()` |
| exchange data | `encode_uplink(&Uplink::new(fcnt, port, payload))`, `encode_downlink(&Downlink::new(...).with_ack())`, `decode(frame, fcnt)` |
| run a node | `EndDevice::new(plan, device, Settings::new(min_dbm, max_dbm).with_seed(seed))` |
| join and send | `join(dev_nonce, now_us)`, `send(port, payload, confirmed, now_us)`, `send_empty(now_us)` |
| hear the network | `heard(frame, snr_db)`, `heard_in(window, frame, snr_db)` |
| survive a power cut | `save(now_us)`, `resume(&saved, now_us)` |
| carry others | `Relay::new(device, RelaySettings::new(..))`, `start(RelayConfig::new(..))`, `forward(now_us)` |

### TypeScript

| To | Call |
| --- | --- |
| ask to join | `device(devEui, joinEui, appKey)`, `joinRequest(devNonce)` |
| grant a join | `grantAccept(grant, appKey, devNonce)`, `grantSession(grant, appKey, devNonce)` |
| take an accept | `acceptJoin(frame, devNonce)`, then `devAddr`, `session()` |
| exchange data | `encodeUplink(fcnt, port, payload)`, `encodeDownlink(fcnt, port, payload, { ack })`, `decode(frame, fcnt)` |
| run a node | `EndDevice.overTheAir(plan, devEui, joinEui, appKey, settings)` |
| join and send | `join(devNonce, nowUs)`, `send(port, payload, nowUs, confirmed)`, `sendEmpty(nowUs)` |
| hear the network | `heard(frame, snrDb, window?)`, `nothingHeard(nowUs)` |
| survive a power cut | `save(nowUs)`, `resume(saved, nowUs)` |
| carry others | `Relay.overTheAir(...)`, `start(...)`, `forward(nowUs)` |

### Python

| To | Call |
| --- | --- |
| ask to join | `device(dev_eui, join_eui, app_key)`, `join_request(dev_nonce)` |
| grant a join | `grant(app_nonce=, net_id=, dev_addr=)`, `accept(app_key, dev_nonce)`, `session(app_key, dev_nonce)` |
| take an accept | `accept_join(frame, dev_nonce)`, then `dev_addr`, `session()` |
| exchange data | `encode_uplink(fcnt, port, payload)`, `encode_downlink(fcnt, port, payload, ack=True)`, `decode(frame, fcnt)` |
| run a node | `end_device(plan, dev_eui, join_eui, app_key, DeviceSettings(min_dbm, max_dbm, seed=...))` |
| join and send | `join(dev_nonce, now_us)`, `send(port, payload, now_us, confirmed=True)`, `send_empty(now_us)` |
| hear the network | `heard(frame, snr_db, window)`, `nothing_heard(now_us)` |
| survive a power cut | `save(now_us)`, `resume(saved, now_us)` |
| carry others | `Relay.over_the_air(...)`, `start(...)`, `forward(now_us)` |

### C#

| To | Call |
| --- | --- |
| ask to join | `new LorawanDevice(devEui, joinEui, appKey)`, `JoinRequest(devNonce)` |
| grant a join | `new LorawanGrant(appNonce, netId, devAddr)`, `Accept(appKey, devNonce)`, `Session(appKey, devNonce)` |
| take an accept | `AcceptJoin(frame, devNonce)`, then `DevAddr`, `Session()` |
| exchange data | `EncodeUplink(fcnt, port, payload)`, `EncodeDownlink(fcnt, port, payload, options)`, `Decode(frame, fcnt)` |
| run a node | `LorawanEndDevice.OverTheAir(plan, credentials, new LorawanDeviceSettings(min, max) { Seed = ... })` |
| join and send | `Join(devNonce, nowUs)`, `Send(port, payload, nowUs, confirmed)`, `SendEmpty(nowUs)` |
| hear the network | `Heard(frame, snrDb, window)`, `NothingHeard(nowUs)` |
| survive a power cut | `Save(nowUs)`, `Resume(saved, nowUs)` |
| carry others | `LorawanRelayNode.OverTheAir(...)`, `Start(...)`, `Forward(nowUs)` |

<!-- languages end -->

## When it goes wrong

A join or a frame that fails its check is refused whole, and the refusal says
why. The mistakes that cost an afternoon:

- **Every accept is refused.** The root key differs between the device and the
  network, or the nonce passed to `accept_join` is not the one the request
  carried. An accept names no device, so those two alone decide.
- **A second reading is refused as busy.** The first still waits on its receive
  windows. Hand the device what each window heard, or tell it nothing was heard,
  and it is free again.
- **After a power cut the network ignores the device.** A device that joins
  again is fine, but one that starts its counters over on an old session has
  every frame taken as a replay. Save the state before sleeping and resume it, as
  the example does; the saved state carries the counters.
- **A downlink is refused as replayed.** The device has already taken one at or
  after that counter. The network numbers its downlinks upward; a server that
  restarted its count has to start a new session.
- **A payload is refused as too long.** Each data rate carries its own most
  payload, 51 bytes at DR0 in EU863-870. The refusal gives the most that fits;
  send less, or let the device move to a faster rate.
- **A send is refused with a wait.** The duty cycle has not freed the air yet.
  The refusal carries the time it will be free, in the caller's microseconds.
- **The network decodes nothing a device sends.** `decode` takes the full 32-bit
  counter it expects for the frame, and its low 16 bits must match what the frame
  carries. A server keeps the count for each device and passes the next one.
- **A relayed device misses its answer.** It arrives in the third window a
  relayed device keeps, up to 18 seconds after the uplink, not in the usual two.
  Name that window when handing the frame to the device.

## Where next

<!-- table: next lorawan -->
- [LoRaWAN gateways](gateway.md): What a LoRaWAN gateway speaks.
- [Secured session](session.md): X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack.
- [Signed updates](update.md): Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own.
- Beside it: [Radios and antennas](../radio.md), [Firmware over the air](fuota.md).
- Also in Radio and reach: [LoRa airtime and range](lora.md), [LoRa radios](radios.md), [Mesh frames](mesh.md), [Routing](routing.md).
<!-- end -->

## Reference

<!-- table: reference lorawan -->
- Rust: [`pamoja-lorawan`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-lorawan)
- TypeScript: [`@pamoja/lorawan`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-lorawan)
- Python: [`pamoja.lorawan`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-lorawan)
- C#: [`Pamoja.Lorawan`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-lorawan)
<!-- end -->
