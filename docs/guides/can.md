# CAN and J1939

CAN is the two-wire bus the moving parts of a machine talk over: motor controllers, servo
drives, battery management, and the engines, gensets, and farm equipment that speak J1939 on
top of it. It has no master. Every node sees every frame on the wire, the identifier at the
head of a frame settles who wins when two start together, and each node keeps the frames it
wants and lets the rest go by.

pamoja covers that from the bytes up, in all four languages. The frames are classic CAN with
up to eight bytes and CAN FD with up to 64, under 11-bit or 29-bit identifiers, with CAN FD's
length encoding. J1939 composes and decodes the priority, parameter group, and addresses it
packs into a 29-bit identifier, and reads and writes the signals inside its eight-byte
payload. `CanBus` is one node's place on a bus: on a Linux board, a SocketCAN socket on an
interface such as `can0`; anywhere, a bus simulated inside the program. A node keeps only the
frames its filters pass, by exact identifier or by J1939 parameter group. The frames and
J1939 are `no_std` and allocation-free, for a microcontroller that brings its own controller.

## What the example does

It is a standby generator's J1939 bus with four nodes on it. The engine controller, at
address 0, broadcasts engine speed. A monitoring gateway keeps engine speed and nothing else.
A service laptop plugged into the diagnostic port keeps everything. A coolant level sensor
beside them speaks plain CAN on an 11-bit identifier. With nothing plugged in, all four sit
on one simulated bus.

The example composes the engine-speed identifier from its fields and builds a reading, with
only the speed written and every other signal marked not available. The engine sends two
readings, and the sensor one frame between them. The gateway reads what its filter kept and
decodes each speed; the laptop reads everything, the sensor's frame among it. Then a request,
which J1939 addresses to one node rather than broadcasting, shows where the destination
lives in the identifier. The engine goes quiet and the gateway's receive times out. Last, a
32-byte CAN FD frame, and a classic frame refusing a ninth byte.

It proves:

- Priority 3, parameter group 61444, and source 0 compose the identifier 0x0CF00400.
- Engine speed sits in bytes 4 and 5 of that group at 0.125 rpm per bit, and the other six
  bytes carry the value that means not available.
- Every node hears every frame but its own: the laptop hears all three.
- A filter for parameter group 61444 keeps both readings from any priority or source, and
  drops the sensor's frame, since an 11-bit identifier never matches a 29-bit filter.
- An 11-bit identifier is not a J1939 message.
- A request to node 0 carries 0 as its destination inside the identifier.
- A receive on a quiet bus returns nothing after its timeout, and the 500 ms is counted
  without the program waiting.
- CAN FD carries 32 bytes at data length code 13, while a classic frame refuses a ninth byte.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example can" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example can</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- can" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- can</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/can.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/can.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- can" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- can</code></div>
</div>
<!-- end -->

## Rust

In Rust the frames and J1939 are `Frame`, `CanId`, `J1939Id`, and `Signals` in `pamoja-can`,
all `no_std` and allocation-free; `Frame::new`, `Frame::fd`, and `Frame::remote` build the
three kinds, and a payload that does not fit is an `Err(CanError)`. The bus is
`pamoja_can::bus`, from the `bus` feature: `CanBus::simulated` makes a bus, `join` puts
another node on it, and `send` and `receive` return `Result<_, BusError>`, a receive giving
`Option<Frame>`. `CanBus::open` needs the `linux` feature on a Linux board. A `CanBus` clones
into the same node, so one thread can receive while another sends; `join` is how a program
makes a second node.

<!-- snippet: examples/guides/can.rs#example -->
From [`examples/guides/can.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/can.rs):

```rust
use std::time::Duration;

use pamoja_can::bus::{CanBus, Filter};
use pamoja_can::{priority, CanError, CanId, Frame, J1939Id, Signals, NOT_AVAILABLE};

// The nodes by the address each answers to, and the two parameter groups in play.
const ENGINE: u8 = 0;
const GATEWAY: u8 = 1;
const ENGINE_CONTROLLER_1: u32 = 61_444; // carries engine speed
const REQUEST: u32 = 59_904; // asks another node for a parameter group

// Where engine speed sits inside that group, and the scale the standard fixes for it.
const ENGINE_SPEED_AT: usize = 3;
const RPM_PER_BIT: f64 = 0.125;

// J1939 keeps its addressing inside the 29-bit identifier: a priority, the parameter group,
// and the sender's address. A broadcast names no destination.
let speed_id = J1939Id::broadcast(priority::CONTROL, ENGINE_CONTROLLER_1, ENGINE);
println!(
    "engine speed 0x{:08X}: pgn {} at priority {}, from node {ENGINE} to every node",
    speed_id.to_id().raw(),
    speed_id.pgn(),
    speed_id.priority()
);

// A reading starts with every signal marked not available, and the engine writes only its
// speed.
let reading = |rpm: f64| -> Result<Frame, CanError> {
    let mut signals = Signals::new();
    signals.set_u16(ENGINE_SPEED_AT, (rpm / RPM_PER_BIT) as u16);
    Frame::new(speed_id.to_id(), signals.as_bytes())
};
let rpm_of = |frame: &Frame| {
    let raw = frame
        .signals()
        .and_then(|signals| signals.u16(ENGINE_SPEED_AT));
    raw.map(|raw| f64::from(raw) * RPM_PER_BIT)
};
let first = reading(1500.0)?;
let unreported = first
    .data()
    .iter()
    .filter(|&&byte| byte == NOT_AVAILABLE)
    .count();
println!(
    "payload      {:.1} rpm in bytes {} and {}, the other {unreported} not available",
    rpm_of(&first).unwrap_or_default(),
    ENGINE_SPEED_AT + 1,
    ENGINE_SPEED_AT + 2
);

// Four nodes on one bus with nothing plugged in. On a Linux board each is
// CanBus::open("can0"), and nothing after this statement changes.
let engine = CanBus::simulated();
let gateway = engine.join()?;
let laptop = engine.join()?;
let sensor = engine.join()?;

// The gateway keeps engine speed and nothing else; the laptop keeps everything.
gateway.set_filters(&[Filter::pgn(ENGINE_CONTROLLER_1)])?;

// Two engine readings, and between them the coolant sensor, which speaks plain CAN: its
// level in percent on the 11-bit identifier 0x120.
engine.send(&first)?;
sensor.send(&Frame::new(CanId::standard(0x120), &[87])?)?;
engine.send(&reading(1512.5)?)?;

// Every node hears every frame but its own, and keeps what its filters pass.
while let Some(frame) = gateway.receive(Duration::from_millis(10))? {
    let from = J1939Id::from_id(frame.id()).map(|id| id.source());
    println!(
        "gateway      {:.1} rpm from node {}",
        rpm_of(&frame).unwrap_or_default(),
        from.unwrap_or_default()
    );
}
let on_the_bus = engine.sent() + sensor.sent();
println!(
    "gateway      kept {} of the {on_the_bus} frames on the bus",
    gateway.received()
);
let mut heard = Vec::new();
while let Some(frame) = laptop.receive(Duration::from_millis(10))? {
    heard.push(frame);
}
if let Some(plain) = heard
    .iter()
    .find(|frame| J1939Id::from_id(frame.id()).is_none())
{
    println!(
        "laptop       heard {}, among them 0x{:03X}, an 11-bit identifier and no J1939 message",
        heard.len(),
        plain.id().raw()
    );
}

// A request is addressed rather than broadcast: below the PDU1 limit, eight bits of the
// identifier name the node it is for.
let request_id = J1939Id::from_parts(priority::DEFAULT, REQUEST, GATEWAY, ENGINE);
println!(
    "request      pgn {} from node {} to node {}",
    request_id.pgn(),
    request_id.source(),
    request_id.destination().unwrap_or_default()
);

// The engine goes quiet. A receive waits for a frame up to its timeout; on a simulated bus
// it returns at once and counts the wait instead of sleeping through it.
let before = gateway.waited_micros();
let quiet = gateway.receive(Duration::from_millis(500))?;
println!(
    "silent       {} frames in {} ms, counted and not slept",
    usize::from(quiet.is_some()),
    (gateway.waited_micros() - before) / 1_000
);

// Above eight bytes CAN FD encodes a length in steps, and a classic frame refuses a ninth
// byte.
let wide = Frame::fd(speed_id.to_id(), &[0; 32])?;
println!(
    "fd           32 bytes travel at data length code {}",
    wide.dlc()
);
if let Err(error) = Frame::new(speed_id.to_id(), &[0; 9]) {
    println!("classic      refused nine bytes: {error}");
}
```
<!-- end -->

## TypeScript

In TypeScript everything is in `@pamoja/can`. A frame is a plain object, `{ id, extended, fd,
remote, len, dlc, data }`, built with `frame`, `fdFrame`, and `remoteFrame`; J1939 is
`broadcastJ1939`, `composeJ1939`, and `decodeJ1939`, and a payload's signals are `signals()`
and `signalsFrom(data)`. `CanBus` is a node: `send` and `receive` return promises that run on
a worker thread, and `receive(timeoutMs)` resolves with a frame or `null`. `filterPgn` and
`filterExact` build the filters `setFilters` takes.

<!-- snippet: bindings/node/guides/can.ts#example -->
From [`bindings/node/guides/can.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/can.ts):

```typescript
import {
  CanBus,
  type CanFrame,
  NOT_AVAILABLE,
  broadcastJ1939,
  composeJ1939,
  decodeJ1939,
  fdFrame,
  filterPgn,
  frame,
  priority,
  signals,
  signalsFrom,
} from '@pamoja/can'

// The nodes by the address each answers to, and the two parameter groups in play.
const ENGINE = 0
const GATEWAY = 1
const ENGINE_CONTROLLER_1 = 61_444 // carries engine speed
const REQUEST = 59_904 // asks another node for a parameter group

// Where engine speed sits inside that group, and the scale the standard fixes for it.
const ENGINE_SPEED_AT = 3
const RPM_PER_BIT = 0.125

// A reading starts with every signal marked not available, and the engine writes only its
// speed.
function reading(speedId: number, rpm: number): CanFrame {
  const reported = signals()
  reported.setU16(ENGINE_SPEED_AT, rpm / RPM_PER_BIT)
  return frame(speedId, reported.bytes, true)
}

function rpmOf(received: CanFrame): number {
  return (signalsFrom(received.data).u16(ENGINE_SPEED_AT) ?? 0) * RPM_PER_BIT
}

function hex(value: number, digits: number): string {
  return `0x${value.toString(16).toUpperCase().padStart(digits, '0')}`
}

async function main() {
  // J1939 keeps its addressing inside the 29-bit identifier: a priority, the parameter group,
  // and the sender's address. A broadcast names no destination.
  const speedId = broadcastJ1939(priority.control, ENGINE_CONTROLLER_1, ENGINE)
  const speed = decodeJ1939(speedId)!
  console.log(
    `engine speed ${hex(speedId, 8)}: pgn ${speed.pgn} at priority ${speed.priority}, ` +
      `from node ${ENGINE} to every node`,
  )

  const first = reading(speedId, 1500)
  const unreported = [...first.data].filter((byte) => byte === NOT_AVAILABLE).length
  console.log(
    `payload      ${rpmOf(first).toFixed(1)} rpm in bytes ${ENGINE_SPEED_AT + 1} and ` +
      `${ENGINE_SPEED_AT + 2}, the other ${unreported} not available`,
  )

  // Four nodes on one bus with nothing plugged in. On a Linux board each is
  // CanBus.open('can0'), and nothing after this statement changes.
  const engine = CanBus.simulated()
  const gateway = engine.join()
  const laptop = engine.join()
  const sensor = engine.join()

  // The gateway keeps engine speed and nothing else; the laptop keeps everything.
  gateway.setFilters([filterPgn(ENGINE_CONTROLLER_1)])

  // Two engine readings, and between them the coolant sensor, which speaks plain CAN: its
  // level in percent on the 11-bit identifier 0x120.
  await engine.send(first)
  await sensor.send(frame(0x120, Buffer.from([87])))
  await engine.send(reading(speedId, 1512.5))

  // Every node hears every frame but its own, and keeps what its filters pass.
  let kept: CanFrame | null
  while ((kept = await gateway.receive(10)) !== null) {
    const from = decodeJ1939(kept.id, kept.extended)?.source ?? 0
    console.log(`gateway      ${rpmOf(kept).toFixed(1)} rpm from node ${from}`)
  }
  const onTheBus = engine.sent + sensor.sent
  console.log(`gateway      kept ${gateway.received} of the ${onTheBus} frames on the bus`)
  const heard: CanFrame[] = []
  let next: CanFrame | null
  while ((next = await laptop.receive(10)) !== null) {
    heard.push(next)
  }
  const plain = heard.find((received) => decodeJ1939(received.id, received.extended) === null)
  if (plain !== undefined) {
    console.log(
      `laptop       heard ${heard.length}, among them ${hex(plain.id, 3)}, ` +
        'an 11-bit identifier and no J1939 message',
    )
  }

  // A request is addressed rather than broadcast: below the PDU1 limit, eight bits of the
  // identifier name the node it is for.
  const request = decodeJ1939(composeJ1939(priority.default, REQUEST, GATEWAY, ENGINE))!
  console.log(
    `request      pgn ${request.pgn} from node ${request.source} to node ${request.destination}`,
  )

  // The engine goes quiet. A receive waits for a frame up to its timeout; on a simulated bus
  // it resolves at once and counts the wait instead of sleeping through it.
  const before = gateway.waitedMicros
  const quiet = await gateway.receive(500)
  const waited = Math.floor((gateway.waitedMicros - before) / 1000)
  console.log(
    `silent       ${quiet === null ? 0 : 1} frames in ${waited} ms, counted and not slept`,
  )

  // Above eight bytes CAN FD encodes a length in steps, and a classic frame refuses a ninth
  // byte.
  const wide = fdFrame(speedId, new Uint8Array(32), true)
  console.log(`fd           32 bytes travel at data length code ${wide.dlc}`)
  try {
    frame(speedId, new Uint8Array(9), true)
  } catch (error) {
    console.log(`classic      refused nine bytes: ${(error as Error).message}`)
  }

  return { speedId, unreported, gateway, heard, request, quiet, wide }
}

main()
```
<!-- end -->

## Python

In Python everything is in `pamoja.can`. `frame`, `fd_frame`, and `remote_frame` build a
read-only `CanFrame`, and `broadcast_j1939`, `compose_j1939`, and `decode_j1939` handle the
identifier. `CanBus` is a node whose `receive` takes a timeout in seconds and returns a frame
or `None`; both it and `send` release the interpreter while the bus is busy. Filters are
`CanFilter.pgn(group)` and `CanFilter.exact(identifier)`, and a failure raises `PamojaError`
from `pamoja.core`.

<!-- snippet: bindings/python/guides/can.py#example -->
From [`bindings/python/guides/can.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/can.py):

```python
from pamoja.can import (
    NOT_AVAILABLE,
    CanBus,
    CanFilter,
    CanFrame,
    Priority,
    broadcast_j1939,
    compose_j1939,
    decode_j1939,
    fd_frame,
    frame,
    signals,
    signals_from,
)
from pamoja.core import PamojaError

# The nodes by the address each answers to, and the two parameter groups in play.
ENGINE = 0
GATEWAY = 1
ENGINE_CONTROLLER_1 = 61_444  # carries engine speed
REQUEST = 59_904  # asks another node for a parameter group

# Where engine speed sits inside that group, and the scale the standard fixes for it.
ENGINE_SPEED_AT = 3
RPM_PER_BIT = 0.125


def reading(speed_id: int, rpm: float) -> CanFrame:
    """A reading: every signal marked not available but the engine's speed."""
    reported = signals()
    reported.set_u16(ENGINE_SPEED_AT, int(rpm / RPM_PER_BIT))
    return frame(speed_id, reported.bytes, extended=True)


def rpm_of(received: CanFrame) -> float:
    """The engine speed a reading carries."""
    return (signals_from(received.data).u16(ENGINE_SPEED_AT) or 0) * RPM_PER_BIT


# J1939 keeps its addressing inside the 29-bit identifier: a priority, the parameter group, and
# the sender's address. A broadcast names no destination.
speed_id = broadcast_j1939(Priority.CONTROL, ENGINE_CONTROLLER_1, ENGINE)
speed = decode_j1939(speed_id)
print(
    f"engine speed 0x{speed_id:08X}: pgn {speed.pgn} at priority {speed.priority}, "
    f"from node {ENGINE} to every node"
)

first = reading(speed_id, 1500)
unreported = sum(1 for byte in first.data if byte == NOT_AVAILABLE)
print(
    f"payload      {rpm_of(first):.1f} rpm in bytes {ENGINE_SPEED_AT + 1} and "
    f"{ENGINE_SPEED_AT + 2}, the other {unreported} not available"
)

# Four nodes on one bus with nothing plugged in. On a Linux board each is
# CanBus.open("can0"), and nothing after this statement changes.
engine = CanBus.simulated()
gateway = engine.join()
laptop = engine.join()
sensor = engine.join()

# The gateway keeps engine speed and nothing else; the laptop keeps everything.
gateway.set_filters([CanFilter.pgn(ENGINE_CONTROLLER_1)])

# Two engine readings, and between them the coolant sensor, which speaks plain CAN: its level
# in percent on the 11-bit identifier 0x120.
engine.send(first)
sensor.send(frame(0x120, bytes([87])))
engine.send(reading(speed_id, 1512.5))

# Every node hears every frame but its own, and keeps what its filters pass.
while (kept := gateway.receive(timeout=0.01)) is not None:
    source = decode_j1939(kept.id, kept.extended).source
    print(f"gateway      {rpm_of(kept):.1f} rpm from node {source}")
on_the_bus = engine.sent + sensor.sent
print(f"gateway      kept {gateway.received} of the {on_the_bus} frames on the bus")
heard = []
while (received := laptop.receive(timeout=0.01)) is not None:
    heard.append(received)
plain = next((f for f in heard if decode_j1939(f.id, f.extended) is None), None)
if plain is not None:
    print(
        f"laptop       heard {len(heard)}, among them 0x{plain.id:03X}, "
        "an 11-bit identifier and no J1939 message"
    )

# A request is addressed rather than broadcast: below the PDU1 limit, eight bits of the
# identifier name the node it is for.
request = decode_j1939(compose_j1939(Priority.DEFAULT, REQUEST, GATEWAY, ENGINE))
print(f"request      pgn {request.pgn} from node {request.source} to node {request.destination}")

# The engine goes quiet. A receive waits for a frame up to its timeout; on a simulated bus it
# returns at once and counts the wait instead of sleeping through it.
before = gateway.waited_micros
quiet = gateway.receive(timeout=0.5)
waited = (gateway.waited_micros - before) // 1_000
print(f"silent       {0 if quiet is None else 1} frames in {waited} ms, counted and not slept")

# Above eight bytes CAN FD encodes a length in steps, and a classic frame refuses a ninth byte.
wide = fd_frame(speed_id, bytes(32), extended=True)
print(f"fd           32 bytes travel at data length code {wide.dlc}")
try:
    frame(speed_id, bytes(9), extended=True)
except PamojaError as error:
    print(f"classic      refused nine bytes: {error}")
```
<!-- end -->

## C#

In C# everything is in `Pamoja.Can`. The static `Can` class builds frames and handles J1939,
`CanFrame` is a value, and `Signals` is a struct over the eight payload bytes. `CanBus` is a
node, disposable since it holds a native handle: `Receive` takes a `TimeSpan` and returns a
`CanFrame` or `null`, and `SetFilters` takes `CanFilter` values, a record struct with `Pgn`
and `Exact` factories. `Open` throws `PlatformNotSupportedException` anywhere but Linux, and
every other failure throws `PamojaException` with the reason.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs):

```csharp
// The nodes by the address each answers to, and the two parameter groups in play.
const byte Engine = 0;
const byte Gateway = 1;
const uint EngineController1 = 61_444; // carries engine speed
const uint Request = 59_904; // asks another node for a parameter group

// Where engine speed sits inside that group, and the scale the standard fixes for it.
const int EngineSpeedAt = 3;
const double RpmPerBit = 0.125;

// J1939 keeps its addressing inside the 29-bit identifier: a priority, the parameter
// group, and the sender's address. A broadcast names no destination.
uint speedId = Can.BroadcastJ1939(J1939Priority.Control, EngineController1, Engine);
J1939Message speed = Can.DecodeJ1939(speedId)!;
Console.WriteLine(
    $"engine speed 0x{speedId:X8}: pgn {speed.Pgn} at priority {speed.Priority}, from node {Engine} to every node");

// A reading starts with every signal marked not available, and the engine writes only
// its speed.
CanFrame Reading(double rpm)
{
    Signals reported = Signals.New();
    reported.SetU16(EngineSpeedAt, (ushort)(rpm / RpmPerBit));
    return Can.Frame(speedId, reported.ToArray(), extended: true);
}

static double RpmOf(CanFrame received) =>
    (Signals.From(received.Data).U16(EngineSpeedAt) ?? 0) * RpmPerBit;

CanFrame first = Reading(1500);
int unreported = first.Data.Count(value => value == Signals.NotAvailable);
Console.WriteLine(Invariant(
    $"payload      {RpmOf(first):F1} rpm in bytes {EngineSpeedAt + 1} and {EngineSpeedAt + 2}, the other {unreported} not available"));

// Four nodes on one bus with nothing plugged in. On a Linux board each is
// CanBus.Open("can0"), and nothing after this statement changes.
using CanBus engine = CanBus.Simulated();
using CanBus gateway = engine.Join();
using CanBus laptop = engine.Join();
using CanBus sensor = engine.Join();

// The gateway keeps engine speed and nothing else; the laptop keeps everything.
gateway.SetFilters(CanFilter.Pgn(EngineController1));

// Two engine readings, and between them the coolant sensor, which speaks plain CAN: its
// level in percent on the 11-bit identifier 0x120.
engine.Send(first);
sensor.Send(Can.Frame(0x120, [87]));
engine.Send(Reading(1512.5));

// Every node hears every frame but its own, and keeps what its filters pass.
while (gateway.Receive(TimeSpan.FromMilliseconds(10)) is { } kept)
{
    byte from = Can.DecodeJ1939(kept.Id, kept.Extended)?.Source ?? 0;
    Console.WriteLine(Invariant($"gateway      {RpmOf(kept):F1} rpm from node {from}"));
}

long onTheBus = engine.Sent + sensor.Sent;
Console.WriteLine($"gateway      kept {gateway.Received} of the {onTheBus} frames on the bus");
var heard = new List<CanFrame>();
while (laptop.Receive(TimeSpan.FromMilliseconds(10)) is { } received)
{
    heard.Add(received);
}

if (heard.Find(received => Can.DecodeJ1939(received.Id, received.Extended) is null) is { } plain)
{
    Console.WriteLine(
        $"laptop       heard {heard.Count}, among them 0x{plain.Id:X3}, an 11-bit identifier and no J1939 message");
}

// A request is addressed rather than broadcast: below the PDU1 limit, eight bits of the
// identifier name the node it is for.
J1939Message request = Can.DecodeJ1939(
    Can.ComposeJ1939((byte)J1939Priority.Normal, Request, Gateway, Engine))!;
Console.WriteLine($"request      pgn {request.Pgn} from node {request.Source} to node {request.Destination}");

// The engine goes quiet. A receive waits for a frame up to its timeout; on a simulated
// bus it returns at once and counts the wait instead of sleeping through it.
ulong before = gateway.WaitedMicros;
CanFrame? quiet = gateway.Receive(TimeSpan.FromMilliseconds(500));
Console.WriteLine(
    $"silent       {(quiet is null ? 0 : 1)} frames in {(gateway.WaitedMicros - before) / 1_000} ms, counted and not slept");

// Above eight bytes CAN FD encodes a length in steps, and a classic frame refuses a
// ninth byte.
CanFrame wide = Can.FdFrame(speedId, new byte[32], extended: true);
Console.WriteLine($"fd           32 bytes travel at data length code {wide.Dlc}");
try
{
    Can.Frame(speedId, new byte[9], extended: true);
}
catch (PamojaException error)
{
    Console.WriteLine($"classic      refused nine bytes: {error.Message}");
}
```
<!-- end -->

## On a board

The same bus on a Raspberry Pi, through an MCP2515 CAN controller on the SPI bus: the
program listens to a real bus for ten seconds and names each J1939 message it hears, engine
speed decoded. It only listens and sends nothing, so it is safe on a running machine's bus,
and it is the first thing to run on one.

The MCP2515 is a controller; a transceiver beside it drives the pair. Pi CAN hats carry both,
with the controller's interrupt on a GPIO and its crystal printed on the can. Many cheap
modules run their transceiver, and with it the whole board, at 5 V, which the Pi's 3.3 V
pins cannot take; choose one whose SPI side runs at 3.3 V.

| Wire | From | To |
| --- | --- | --- |
| SPI | the Pi's MOSI, MISO, SCLK, and CE0 | the MCP2515's SI, SO, SCK, and CS |
| interrupt | the MCP2515's INT | the GPIO the overlay names |
| CAN_H and CAN_L | the transceiver | the bus's CAN_H and CAN_L, never crossed |
| termination | 120 Ω across CAN_H and CAN_L | the two ends of the bus, and nowhere else |

The kernel drives the MCP2515 through a device tree overlay, which `/boot/firmware/config.txt`
loads with the crystal's frequency and the interrupt's GPIO, from the
[overlay documentation](https://github.com/raspberrypi/linux/blob/rpi-6.12.y/arch/arm/boot/dts/overlays/README):

```text
dtparam=spi=on
dtoverlay=mcp2515-can0,oscillator=<the crystal, in hertz>,interrupt=<the GPIO>
```

After a reboot, the [kernel's SocketCAN documentation](https://docs.kernel.org/networking/can.html)
brings the interface up at the bus's bit rate, which every node on it already shares:

```sh
sudo ip link set can0 up type can bitrate 250000
ip -details link show can0
```

With nothing else on the bus, the monitor prints that it is quiet once a second. Nothing it
prints means the bit rate or the crystal is wrong, the pair is crossed, or the bus lacks its
termination.

### Rust

<!-- snippet: examples/boards/raspberry-pi/src/bin/can.rs#example -->
From [`examples/boards/raspberry-pi/src/bin/can.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/bin/can.rs):

```rust
use pamoja_can::bus::CanBus;
use pamoja_can::J1939Id;

/// The interface the MCP2515 overlay makes.
const INTERFACE: &str = "can0";

/// Engine speed's parameter group, where the speed sits in it, and its scale.
const ENGINE_CONTROLLER_1: u32 = 61_444;
const ENGINE_SPEED_AT: usize = 3;
const RPM_PER_BIT: f64 = 0.125;

fn main() -> Result<(), Box<dyn Error>> {
    // The monitor only listens and sends nothing, so it is safe on a running machine's bus.
    let bus = CanBus::open(INTERFACE)?;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        let Some(frame) = bus.receive(Duration::from_secs(1))? else {
            println!("quiet for a second: check the bit rate, the wiring, and the termination");
            continue;
        };
        let at = started.elapsed().as_secs_f64();
        match J1939Id::from_id(frame.id()) {
            Some(id) if id.pgn() == ENGINE_CONTROLLER_1 => {
                let raw = frame
                    .signals()
                    .and_then(|signals| signals.u16(ENGINE_SPEED_AT));
                let rpm = raw.map_or(0.0, |raw| f64::from(raw) * RPM_PER_BIT);
                println!(
                    "{at:7.3} s  pgn {} from {}: {rpm:.1} rpm",
                    id.pgn(),
                    id.source()
                );
            }
            Some(id) => println!(
                "{at:7.3} s  pgn {} from {}, {} bytes",
                id.pgn(),
                id.source(),
                frame.len()
            ),
            None => println!(
                "{at:7.3} s  0x{:03X}, {} bytes, an 11-bit identifier",
                frame.id().raw(),
                frame.len()
            ),
        }
    }
    println!("{} frames in ten seconds on {INTERFACE}", bus.received());
    Ok(())
}
```
<!-- end -->

```sh
cd examples/boards/raspberry-pi
cargo run --release --bin can
```

### TypeScript

<!-- snippet: bindings/node/boards/raspberry-pi/can.ts#example -->
From [`bindings/node/boards/raspberry-pi/can.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/boards/raspberry-pi/can.ts):

```typescript
import { CanBus, decodeJ1939, signalsFrom } from '@pamoja/can'

// The interface the MCP2515 overlay makes.
const INTERFACE = 'can0'

// Engine speed's parameter group, where the speed sits in it, and its scale.
const ENGINE_CONTROLLER_1 = 61_444
const ENGINE_SPEED_AT = 3
const RPM_PER_BIT = 0.125

async function main(): Promise<void> {
  // The monitor only listens and sends nothing, so it is safe on a running machine's bus.
  const bus = CanBus.open(INTERFACE)
  const started = performance.now()
  while (performance.now() - started < 10_000) {
    const frame = await bus.receive(1000)
    if (frame === null) {
      console.log('quiet for a second: check the bit rate, the wiring, and the termination')
      continue
    }
    const at = ((performance.now() - started) / 1000).toFixed(3).padStart(7)
    const id = decodeJ1939(frame.id, frame.extended)
    if (id !== null && id.pgn === ENGINE_CONTROLLER_1) {
      const rpm = (signalsFrom(frame.data).u16(ENGINE_SPEED_AT) ?? 0) * RPM_PER_BIT
      console.log(`${at} s  pgn ${id.pgn} from ${id.source}: ${rpm.toFixed(1)} rpm`)
    } else if (id !== null) {
      console.log(`${at} s  pgn ${id.pgn} from ${id.source}, ${frame.len} bytes`)
    } else {
      const hex = frame.id.toString(16).toUpperCase().padStart(3, '0')
      console.log(`${at} s  0x${hex}, ${frame.len} bytes, an 11-bit identifier`)
    }
  }
  console.log(`${bus.received} frames in ten seconds on ${INTERFACE}`)
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
```
<!-- end -->

```sh
npm --prefix bindings/node run boards
node bindings/node/build/boards/raspberry-pi/can.js
```

### Python

<!-- snippet: bindings/python/boards/raspberry_pi/can.py#example -->
From [`bindings/python/boards/raspberry_pi/can.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/boards/raspberry_pi/can.py):

```python
import time

from pamoja.can import CanBus, decode_j1939, signals_from

# The interface the MCP2515 overlay makes.
INTERFACE = "can0"

# Engine speed's parameter group, where the speed sits in it, and its scale.
ENGINE_CONTROLLER_1 = 61_444
ENGINE_SPEED_AT = 3
RPM_PER_BIT = 0.125


def main() -> None:
    # The monitor only listens and sends nothing, so it is safe on a running machine's bus.
    bus = CanBus.open(INTERFACE)
    started = time.perf_counter()
    while time.perf_counter() - started < 10:
        frame = bus.receive(timeout=1)
        if frame is None:
            print("quiet for a second: check the bit rate, the wiring, and the termination")
            continue
        at = time.perf_counter() - started
        message = decode_j1939(frame.id, frame.extended)
        if message is not None and message.pgn == ENGINE_CONTROLLER_1:
            rpm = (signals_from(frame.data).u16(ENGINE_SPEED_AT) or 0) * RPM_PER_BIT
            print(f"{at:7.3f} s  pgn {message.pgn} from {message.source}: {rpm:.1f} rpm")
        elif message is not None:
            print(f"{at:7.3f} s  pgn {message.pgn} from {message.source}, {frame.len} bytes")
        else:
            print(f"{at:7.3f} s  0x{frame.id:03X}, {frame.len} bytes, an 11-bit identifier")
    print(f"{bus.received} frames in ten seconds on {INTERFACE}")


if __name__ == "__main__":
    main()
```
<!-- end -->

```sh
python bindings/python/boards/raspberry_pi/can.py
```

### C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Can.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Can.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Can.cs):

```csharp
using System.Diagnostics;
using System.Globalization;

using Pamoja.Can;

namespace Boards.RaspberryPi;

/// <summary>
/// A CAN bus monitor: listens on the Pi's CAN interface, through an MCP2515, and names each J1939
/// message it hears. Load the controller's overlay and bring the interface up at the bus's bit
/// rate first.
/// </summary>
public static class CanMonitor
{
    // The interface the MCP2515 overlay makes.
    private const string Interface = "can0";

    // Engine speed's parameter group, where the speed sits in it, and its scale.
    private const uint EngineController1 = 61_444;
    private const int EngineSpeedAt = 3;
    private const double RpmPerBit = 0.125;

    /// <summary>Listens for ten seconds and names each frame.</summary>
    public static void Run()
    {
        // The monitor only listens and sends nothing, so it is safe on a running machine's bus.
        using CanBus bus = CanBus.Open(Interface);
        var started = Stopwatch.StartNew();
        while (started.Elapsed < TimeSpan.FromSeconds(10))
        {
            CanFrame? frame = bus.Receive(TimeSpan.FromSeconds(1));
            if (frame is null)
            {
                Console.WriteLine("quiet for a second: check the bit rate, the wiring, and the termination");
                continue;
            }

            string at = started.Elapsed.TotalSeconds.ToString("F3", CultureInfo.InvariantCulture).PadLeft(7);
            J1939Message? message = Can.DecodeJ1939(frame.Id, frame.Extended);
            if (message is { Pgn: EngineController1 })
            {
                double rpm = (Signals.From(frame.Data).U16(EngineSpeedAt) ?? 0) * RpmPerBit;
                Console.WriteLine(string.Create(
                    CultureInfo.InvariantCulture,
                    $"{at} s  pgn {message.Pgn} from {message.Source}: {rpm:F1} rpm"));
            }
            else if (message is not null)
            {
                Console.WriteLine($"{at} s  pgn {message.Pgn} from {message.Source}, {frame.Length} bytes");
            }
            else
            {
                Console.WriteLine($"{at} s  0x{frame.Id:X3}, {frame.Length} bytes, an 11-bit identifier");
            }
        }

        Console.WriteLine($"{bus.Received} frames in ten seconds on {Interface}");
    }
}
```
<!-- end -->

```sh
dotnet run --project bindings/dotnet/samples/Pamoja.Boards -- raspberry-pi/can
```

## Values at a glance

**The three kinds of frame:**

| Kind | Data | Built with | On a SocketCAN socket |
| --- | --- | --- | --- |
| classic | 0 to 8 bytes | `Frame::new`, `frame`, `Can.Frame` | a 16-byte `can_frame` |
| CAN FD | 0 to 8 bytes, then 12, 16, 20, 24, 32, 48, or 64 | `Frame::fd`, `fdFrame`, `fd_frame`, `Can.FdFrame` | a 72-byte `canfd_frame` |
| remote | none; it asks for 0 to 8 | `Frame::remote`, `remoteFrame`, `remote_frame`, `Can.RemoteFrame` | a `can_frame` with its remote flag set |

Either kind of data frame takes an 11-bit standard or a 29-bit extended identifier; J1939
uses only the extended one.

**The data length code**, four bits on the wire. Up to eight it is the length; above, CAN FD
steps:

| Code | Bytes |
| --- | --- |
| 0 to 8 | the same as the code |
| 9 | 12 |
| 10 | 16 |
| 11 | 20 |
| 12 | 24 |
| 13 | 32 |
| 14 | 48 |
| 15 | 64 |

**Inside a J1939 identifier**, 29 bits, most significant first:

| Bits | Field | What it holds |
| --- | --- | --- |
| 26 to 28 | priority | 0 highest to 7 lowest; 3 for control, 6 by default |
| 25 and 24 | extended data page and data page | the top two bits of the parameter group |
| 16 to 23 | PDU format | the parameter group; below 240 it is addressed, at 240 and above a broadcast |
| 8 to 15 | PDU specific | for an addressed group, the destination; for a broadcast, the low byte of the group |
| 0 to 7 | source address | the node that sent it |

So an addressed group's number never includes a destination, and a broadcast's number is
all 18 bits; `J1939Id::from_parts` fills whichever the group needs.

**Filters.** SocketCAN keeps a frame when `received_id & mask == id & mask`. pamoja's
`Filter` always matches the frame format as well, so a filter for an extended identifier
never passes a standard frame that shares its low bits:

| Filter | Keeps |
| --- | --- |
| `Filter::exact(id)` | that one identifier |
| `Filter::pgn(group)` | one J1939 parameter group, at any priority, from any source, and for an addressed group, to any destination |
| `Filter::new(id, mask)` | the identifiers whose masked bits equal `id`'s |
| no filter, or `clear_filters` | every frame, as a node does when it joins |
| an empty list | nothing |

A node keeps a frame that passes any one of its filters, and setting filters replaces the
ones before.

**What a SocketCAN socket does by default**, from the kernel's documentation, and what a
node here does the same:

| Behavior | SocketCAN | A simulated node |
| --- | --- | --- |
| keeps every frame until filtered | yes | yes |
| hears other sockets on the same interface | yes, the local loopback | yes, every node on the bus |
| hears its own frames | no | no |
| takes CAN FD frames | when asked; pamoja asks | yes |
| receives error frames | no | none are made |

**The kinds of bus:**

| Kind | Made with | What a send does | What a receive gets |
| --- | --- | --- | --- |
| Device | `open(interface)`, Linux only | goes out on the wire, and to the other sockets on the interface | what the bus delivered, waiting up to the timeout |
| Simulated | `simulated()` | reaches every other node on the bus | what the other nodes sent, at once, or nothing with the timeout counted |

`join()` adds a node of the same kind: another socket on the same interface, or another node
on the same simulated bus.

**The same calls in each language:**

| To | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| open an interface | `CanBus::open("can0")?` | `CanBus.open('can0')` | `CanBus.open("can0")` | `CanBus.Open("can0")` |
| make a bus | `CanBus::simulated()` | `CanBus.simulated()` | `CanBus.simulated()` | `CanBus.Simulated()` |
| another node | `bus.join()?` | `bus.join()` | `bus.join()` | `bus.Join()` |
| send | `bus.send(&frame)?` | `await bus.send(frame)` | `bus.send(frame)` | `bus.Send(frame)` |
| receive | `bus.receive(timeout)?` | `await bus.receive(timeoutMs)` | `bus.receive(timeout=seconds)` | `bus.Receive(timeout)` |
| keep one group | `bus.set_filters(&[Filter::pgn(group)])?` | `bus.setFilters([filterPgn(group)])` | `bus.set_filters([CanFilter.pgn(group)])` | `bus.SetFilters(CanFilter.Pgn(group))` |
| a broadcast identifier | `J1939Id::broadcast(p, group, source).to_id()` | `broadcastJ1939(p, group, source)` | `broadcast_j1939(p, group, source)` | `Can.BroadcastJ1939(p, group, source)` |
| read it back | `J1939Id::from_id(id)` | `decodeJ1939(id, extended)` | `decode_j1939(id, extended)` | `Can.DecodeJ1939(id, extended)` |
| write a signal | `signals.set_u16(at, value)` | `signals.setU16(at, value)` | `signals.set_u16(at, value)` | `signals.SetU16(at, value)` |

## When it goes wrong

What the bus and the frames refuse, and what they say:

| What happened | The message | What to check |
| --- | --- | --- |
| the platform has no SocketCAN | `a CAN interface is opened through SocketCAN, which only Linux has here` | open a real interface on a Linux board, and a simulated bus anywhere |
| the interface does not exist | `can0: No such device (os error 19)` | the overlay, the crystal and interrupt it names, and a reboot |
| the interface is down | `can0: Network is down (os error 100)` | `ip link set can0 up type can bitrate ...` |
| a CAN FD frame on a classic interface | `can0: Invalid argument (os error 22)` | the interface's mode, or send a classic frame |
| too much data for the frame | `the data is longer than the frame carries: 8 bytes for classic CAN, 64 for CAN FD` | a classic frame's eight bytes |
| a CAN FD length that does not exist | `CAN FD carries 0 to 8 bytes, then only 12, 16, 20, 24, 32, 48, or 64` | pad the payload to the next length |

How each language hands those over:

| Language | An interface that will not open | A send or receive that fails | A frame that does not fit |
| --- | --- | --- | --- |
| Rust | `Err(OpenError)` | `Err(BusError)` | `Err(CanError)` |
| TypeScript | a thrown `Error` | a rejected promise | a thrown `Error` |
| Python | `PamojaError` | `PamojaError` | `PamojaError` |
| C# | `PlatformNotSupportedException` off Linux, `PamojaException` otherwise | `PamojaException` | `PamojaException` |

The mistakes that cost an afternoon:

- **Nothing arrives at all.** Every node on a bus shares one bit rate, and the interface's has
  to match it; so does the crystal frequency the overlay was given, since the controller's
  timing is derived from it.
- **A lone node's frames go nowhere.** A CAN frame needs at least one other node to
  acknowledge it, so a node alone on a bus retransmits and gathers errors. Test with a second
  node, or a virtual interface, `vcan0`.
- **The interface stops mid-run.** Too many errors take a controller bus-off, and it sends and
  receives nothing more until restarted: `ip link set can0 type can restart-ms 100` recovers
  on its own.
- **The bus works on the bench and not on the machine.** It needs its two 120 Ω terminators,
  one at each end. With the power off, the resistance between CAN_H and CAN_L reads about 60 Ω
  when both are there.
- **CAN_H and CAN_L crossed.** Nothing decodes. The pair is not interchangeable.
- **A program never sees what it sent.** A node does not hear its own frames, on SocketCAN as
  here. Listen on a second node.
- **A filter drops frames it should keep.** Setting filters replaces the ones before, and an
  empty list keeps nothing; a J1939 filter is for 29-bit frames only.
- **A 5 V module on the header.** The Pi's pins are 3.3 V, and a board whose SPI side runs at
  5 V damages them.

## Where next

<!-- table: next can -->
- [Buses](hal.md): The embedded-hal traits every driver takes, a bit-banged 1-Wire bus, simulated parts and scripted buses that stand in for hardware, the Linux backends over i2c-dev, spidev, and the GPIO character device, one I2C bus and one serial port a program and its drivers share, and delays that sleep or only count.
- [Device profiles](profile.md): Named, ready-to-run device profiles from plain data or a JSON manifest, run as a node in every language.
- [Telemetry](telemetry.md): Observability that ships only what is worth the bytes as link cost rises, while counting everything.
- Beside it: [Buses and links](../buses.md).
- Also in Field I/O: [Serial framing](serial.md), [Modbus RTU](modbus.md), [I2C, SPI, and GPIO](gpio.md).
<!-- end -->

## Reference

<!-- table: reference can -->
- Rust: [`pamoja-can`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-can)
- TypeScript: [`@pamoja/can`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-can)
- Python: [`pamoja.can`](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-can)
- C#: [`Pamoja.Can`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-can)
<!-- end -->
