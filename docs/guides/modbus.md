# Modbus RTU

Modbus RTU is what an RS485 run to a field device usually speaks: energy meters, relay modules,
pump drives, soil and water probes. A frame is a one-byte unit address, a function code, its
data, and a CRC-16/MODBUS, with no delimiter at all: 3.5 characters of silence on the line are
what end a frame. One gateway, the client, asks and every other device on the line, a server,
only ever answers.

pamoja covers both ends of that conversation, in all four languages. `ModbusClient` runs whole
transactions over the serial port from the [serial framing guide](serial.md), with the timing
the Modbus over Serial Line specification sets: the silence before each request, the response
timeout, and the turnaround after a broadcast. It checks every reply's CRC, unit, and function
before a value is read out of it. `ModbusServer` is a device, a unit address and the four
tables it serves, answering each frame the way the specification says a device does, silence
included. `ModbusLine` puts several of them on the far end of a simulated port, so a polling
loop is written and tested with nothing plugged in, and runs unchanged on a real line. Beneath
both sit the frame builders and parsers, `no_std` and allocation-free, for a microcontroller
that brings its own UART.

## What the example does

It is the gateway at a village water pump. One RS485 line, at 19200 baud with even parity, the
default the specification sets, carries two devices. An energy meter at unit 17 keeps its
measurements in input registers from 0: volts in tenths, amps in hundredths, then a fault word.
A four-relay module at unit 18 switches the pump on relay 0 and reports the tank's low-level
float switch on discrete input 0. With nothing plugged in, both are servers on a simulated line.

The gateway polls the meter and prints what that poll cost the line. It reads the float switch,
finds the tank low, and starts the pump. Then it broadcasts every relay off, which each device
carries out and none answers, and reads the relays back. Two failures follow, the two a new
installation meets first: asking the meter for its measurements with the wrong function, which
it refuses with an exception, and polling a unit that is not on the line, which never answers.

It proves:

- 19200 8E1 is eleven bits a character, so the silence that ends a frame, 3.5 characters, is
  2005 µs.
- A poll of three registers is eight bytes out and eleven back, 12.89 ms of line time with the
  silence before it.
- The values come back in the order the meter holds them, and scale to 230.1 V and 4.18 A.
- A write reaches the relay module and reads back, and a broadcast write reaches every device,
  draws no reply, and costs the 100 ms turnaround instead.
- A device that is asked for a table it does not have answers with exception 0x02, illegal data
  address, which comes back as an error that names the unit, the function, and the exception.
- A unit that is not there costs the one-second response timeout, which a simulated line
  counts rather than waits, with the 2 ms of silence before the request: 1002 ms.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example modbus" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example modbus</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- modbus" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- modbus</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/modbus.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/modbus.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- modbus" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- modbus</code></div>
</div>
<!-- end -->

## Rust

In Rust the frames are `Pdu`, `Adu`, `Response`, and `Request` in `pamoja-modbus`, all `no_std`
and allocation-free. `Server` comes with the `alloc` feature, and `Client` and `Line` with the
`port` feature, which brings in `pamoja-hal` and its `std` layer; the `pamoja` crate turns both
on with its `std` feature. `Client::new` takes a `pamoja_hal::port::SerialPort`, and every
transaction returns `Result<_, ClientError>`, whose `Exception` variant carries the device's
`Exception`. A `Line` is a `Peer`, so `SerialPort::simulated(settings, line)` puts it on the far
end of a port, and `attach` hands back an `Arc<Mutex<Server>>` for looking at a device later.

<!-- snippet: examples/guides/modbus.rs#example -->
From [`examples/guides/modbus.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/modbus.rs):

```rust
use pamoja_hal::port::{Parity, SerialPort, Settings};
use pamoja_modbus::{Client, Line, Server, BROADCAST};

fn word(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

// The line: 19200 baud, even parity, one stop bit, the default the Modbus specification
// sets. Eleven bits a character, and 3.5 of them of silence mark where a frame ends.
let settings = Settings::new(19_200).with_parity(Parity::Even);
let gap = Client::frame_gap(settings).as_micros() as u64;
println!(
    "line         {settings}, {} bits a character, t3.5 is {gap} us",
    settings.bits_per_character()
);

// Each device's manual gives its unit address and where its values live. The meter keeps
// its measurements in input registers from 0: volts in tenths, amps in hundredths, then a
// fault word. The relay module has four relays as coils 0 to 3, and the tank's low-level
// float switch as discrete input 0, on while the water is below it.
const METER: u8 = 17;
const PUMP: u8 = 18;
let mut line = Line::new();
line.attach(Server::new(METER)?.with_input_registers(0, &[2301, 418, 0]));
let relays = line.attach(
    Server::new(PUMP)?
        .with_coils(0, &[false; 4])
        .with_discrete_inputs(0, &[true]),
);

// The devices sit on a simulated line. On a gateway the port is
// SerialPort::open("/dev/ttyUSB0", settings), and nothing after this statement changes.
let port = SerialPort::simulated(settings, line);
let mut client = Client::new(port.clone());

// Poll the meter with function 0x04 for three input registers, and scale each one as its
// manual says.
let registers = client.read_input_registers(METER, 0, 3)?;
println!(
    "meter        {:.1} V, {:.2} A, faults {}",
    f32::from(registers[0]) / 10.0,
    f32::from(registers[1]) / 100.0,
    registers[2]
);

// What that poll cost the line: the request, the reply, and the silence before the request.
let (out, back) = (port.written(), port.received());
let line_time = settings.transfer_micros(out) + settings.transfer_micros(back) + gap;
println!(
    "poll         {out} bytes out, {back} back, {:.2} ms of line time",
    line_time as f64 / 1_000.0
);

// Read the float switch, and start the pump on relay 0 when the tank is low.
let low = client.read_discrete_inputs(PUMP, 0, 1)?[0];
println!("tank         low-level switch {}", word(low));
if low {
    client.write_single_coil(PUMP, 0, true)?;
}
let states = client.read_coils(PUMP, 0, 4)?;
let words: Vec<&str> = states.iter().map(|&on| word(on)).collect();
println!("relays       {}", words.join(" "));

// A broadcast, to unit 0, reaches every device on the line and none answers: here every
// relay off at once. The client waits out the turnaround so each device has carried it out
// before the next request.
client.write_multiple_coils(BROADCAST, 0, &[false; 4])?;
println!(
    "broadcast    every relay off, no reply, {} ms turnaround",
    client.turnaround().as_millis()
);
let after: Vec<&str> = client
    .read_coils(PUMP, 0, 4)?
    .iter()
    .map(|&on| word(on))
    .collect();
println!("relays       {}", after.join(" "));

// The meter keeps its measurements in input registers. Asking for them as holding
// registers, function 0x03, is the usual mistake with a new device, and the meter refuses
// it with an exception instead of answering.
let refused = client.read_holding_registers(METER, 0, 3);
if let Err(error) = &refused {
    println!("refused      {error}");
}

// A unit that is not on the line never answers. The client gives up after its response
// timeout, one second unless told otherwise, which a simulated line counts instead of
// sleeping through.
let before = port.waited_micros();
let silent = client.read_input_registers(19, 0, 1);
if let Err(error) = &silent {
    println!(
        "silent       {error}, {} ms counted and not slept",
        (port.waited_micros() - before) / 1_000
    );
}
```
<!-- end -->

## TypeScript

In TypeScript everything is in `@pamoja/modbus`: `ModbusClient`, `ModbusServer`, `ModbusLine`,
the frame builders, and `parseFrame`. The client takes a `SerialPort` from `@pamoja/hal`, and
each transaction returns a promise that runs on a worker thread, so a real line never blocks
the event loop. A failed one rejects with a `ModbusClientError`, whose `kind` says why and whose
`exception` holds the device's code, one of the `ExceptionCode` values. `ModbusLine.port`
returns an ordinary `SerialPort`, and the servers stay the program's own objects, so a test
reads a device's tables after the client has written them.

<!-- snippet: bindings/node/guides/modbus.ts#example -->
From [`bindings/node/guides/modbus.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/modbus.ts):

```typescript
import { Parity, SerialPort } from '@pamoja/hal'
import {
  BROADCAST,
  ModbusClient,
  ModbusClientError,
  ModbusLine,
  ModbusServer,
} from '@pamoja/modbus'

function word(on: boolean): string {
  return on ? 'on' : 'off'
}

async function main() {
  // The line: 19200 baud, even parity, one stop bit, the default the Modbus specification
  // sets. Eleven bits a character, and 3.5 of them of silence mark where a frame ends.
  const settings = { baud: 19200, parity: Parity.Even }
  const gap = Math.floor(ModbusClient.frameGapNanos(settings) / 1000)
  console.log(
    `line         ${SerialPort.describe(settings)}, ` +
      `${SerialPort.bitsPerCharacter(settings)} bits a character, t3.5 is ${gap} us`,
  )

  // Each device's manual gives its unit address and where its values live. The meter keeps
  // its measurements in input registers from 0: volts in tenths, amps in hundredths, then a
  // fault word. The relay module has four relays as coils 0 to 3, and the tank's low-level
  // float switch as discrete input 0, on while the water is below it.
  const METER = 17
  const PUMP = 18
  const meter = new ModbusServer(METER)
  meter.setInputRegisters(0, [2301, 418, 0])
  const relays = new ModbusServer(PUMP)
  relays.setCoils(0, [false, false, false, false])
  relays.setDiscreteInputs(0, [true])
  const line = new ModbusLine()
  line.attach(meter)
  line.attach(relays)

  // The devices sit on a simulated line. On a gateway the port is
  // SerialPort.open('/dev/ttyUSB0', settings), and nothing after this statement changes.
  const port = line.port(settings)
  const client = new ModbusClient(port)

  // Poll the meter with function 0x04 for three input registers, and scale each one as its
  // manual says.
  const registers = await client.readInputRegisters(METER, 0, 3)
  console.log(
    `meter        ${(registers[0] / 10).toFixed(1)} V, ${(registers[1] / 100).toFixed(2)} A, ` +
      `faults ${registers[2]}`,
  )

  // What that poll cost the line: the request, the reply, and the silence before the request.
  const out = port.written
  const back = port.received
  const lineTime =
    SerialPort.transferMicros(settings, out) + SerialPort.transferMicros(settings, back) + gap
  console.log(
    `poll         ${out} bytes out, ${back} back, ${(lineTime / 1000).toFixed(2)} ms of line time`,
  )

  // Read the float switch, and start the pump on relay 0 when the tank is low.
  const [low] = await client.readDiscreteInputs(PUMP, 0, 1)
  console.log(`tank         low-level switch ${word(low)}`)
  if (low) {
    await client.writeSingleCoil(PUMP, 0, true)
  }
  const states = await client.readCoils(PUMP, 0, 4)
  console.log(`relays       ${states.map(word).join(' ')}`)

  // A broadcast, to unit 0, reaches every device on the line and none answers: here every
  // relay off at once. The client waits out the turnaround so each device has carried it out
  // before the next request.
  await client.writeMultipleCoils(BROADCAST, 0, [false, false, false, false])
  console.log(`broadcast    every relay off, no reply, ${client.turnaroundMs} ms turnaround`)
  const after = await client.readCoils(PUMP, 0, 4)
  console.log(`relays       ${after.map(word).join(' ')}`)

  // The meter keeps its measurements in input registers. Asking for them as holding
  // registers, function 0x03, is the usual mistake with a new device, and the meter refuses it
  // with an exception instead of answering.
  let refused: ModbusClientError | undefined
  try {
    await client.readHoldingRegisters(METER, 0, 3)
  } catch (error) {
    refused = error as ModbusClientError
    console.log(`refused      ${refused.message}`)
  }

  // A unit that is not on the line never answers. The client gives up after its response
  // timeout, one second unless told otherwise, which a simulated line counts instead of
  // sleeping through.
  const before = port.waitedMicros
  let silent: ModbusClientError | undefined
  try {
    await client.readInputRegisters(19, 0, 1)
  } catch (error) {
    silent = error as ModbusClientError
    const waited = Math.floor((port.waitedMicros - before) / 1000)
    console.log(`silent       ${silent.message}, ${waited} ms counted and not slept`)
  }

  return { registers, out, back, low, states, relays, refused, silent }
}

main()
```
<!-- end -->

## Python

In Python everything is in `pamoja.modbus`. `ModbusClient` takes a `SerialPort` from
`pamoja.hal`, with its timeouts in seconds, and releases the interpreter for the whole of each
transaction. A failed one raises `ModbusClientError`, a `PamojaError` whose `kind` is a string,
`exception` for a refusal and `timeout` for silence, and whose `exception` attribute holds the
code, comparable with the `Exception_` enum. `ModbusLine().attach(server)` returns the line, so
devices chain onto it.

<!-- snippet: bindings/python/guides/modbus.py#example -->
From [`bindings/python/guides/modbus.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/modbus.py):

```python
from pamoja.hal import Parity, SerialSettings
from pamoja.modbus import BROADCAST, ModbusClient, ModbusClientError, ModbusLine, ModbusServer


def word(on: bool) -> str:
    """A state as the output prints it."""
    return "on" if on else "off"


# The line: 19200 baud, even parity, one stop bit, the default the Modbus specification sets.
# Eleven bits a character, and 3.5 of them of silence mark where a frame ends.
settings = SerialSettings(19_200, Parity.EVEN)
gap = ModbusClient.frame_gap_nanos(settings) // 1_000
print(f"line         {settings}, {settings.bits_per_character} bits a character, t3.5 is {gap} us")

# Each device's manual gives its unit address and where its values live. The meter keeps its
# measurements in input registers from 0: volts in tenths, amps in hundredths, then a fault
# word. The relay module has four relays as coils 0 to 3, and the tank's low-level float switch
# as discrete input 0, on while the water is below it.
METER = 17
PUMP = 18
meter = ModbusServer(METER)
meter.set_input_registers(0, [2301, 418, 0])
relays = ModbusServer(PUMP)
relays.set_coils(0, [False] * 4)
relays.set_discrete_inputs(0, [True])
line = ModbusLine().attach(meter).attach(relays)

# The devices sit on a simulated line. On a gateway the port is
# SerialPort.open("/dev/ttyUSB0", settings), and nothing after this statement changes.
port = line.port(settings)
client = ModbusClient(port)

# Poll the meter with function 0x04 for three input registers, and scale each one as its manual
# says.
registers = client.read_input_registers(METER, 0, 3)
print(f"meter        {registers[0] / 10:.1f} V, {registers[1] / 100:.2f} A, faults {registers[2]}")

# What that poll cost the line: the request, the reply, and the silence before the request.
out, back = port.written, port.received
line_time = settings.transfer_micros(out) + settings.transfer_micros(back) + gap
print(f"poll         {out} bytes out, {back} back, {line_time / 1_000:.2f} ms of line time")

# Read the float switch, and start the pump on relay 0 when the tank is low.
[low] = client.read_discrete_inputs(PUMP, 0, 1)
print(f"tank         low-level switch {word(low)}")
if low:
    client.write_single_coil(PUMP, 0, True)
states = client.read_coils(PUMP, 0, 4)
print(f"relays       {' '.join(word(on) for on in states)}")

# A broadcast, to unit 0, reaches every device on the line and none answers: here every relay
# off at once. The client waits out the turnaround so each device has carried it out before the
# next request.
client.write_multiple_coils(BROADCAST, 0, [False] * 4)
print(f"broadcast    every relay off, no reply, {round(client.turnaround * 1_000)} ms turnaround")
after = client.read_coils(PUMP, 0, 4)
print(f"relays       {' '.join(word(on) for on in after)}")

# The meter keeps its measurements in input registers. Asking for them as holding registers,
# function 0x03, is the usual mistake with a new device, and the meter refuses it with an
# exception instead of answering.
refused = None
try:
    client.read_holding_registers(METER, 0, 3)
except ModbusClientError as error:
    refused = error
    print(f"refused      {error}")

# A unit that is not on the line never answers. The client gives up after its response timeout,
# one second unless told otherwise, which a simulated line counts instead of sleeping through.
before = port.waited_micros
silent = None
try:
    client.read_input_registers(19, 0, 1)
except ModbusClientError as error:
    silent = error
    waited = (port.waited_micros - before) // 1_000
    print(f"silent       {error}, {waited} ms counted and not slept")
```
<!-- end -->

## C#

In C# everything is in `Pamoja.Modbus`. `ModbusClient` takes a `SerialPort` from `Pamoja.Hal`,
and its `ResponseTimeout` and `Turnaround` are `TimeSpan` properties, so an object initializer
sets them. Each transaction blocks until its reply is in, and a failed one throws
`ModbusClientException`, whose `Kind` is a `ModbusClientErrorKind` and whose `ExceptionCode` is
a `ModbusExceptionCode`. Servers, lines, clients, and ports hold native handles and are
`IDisposable`; a line holds its own share of each server on it, and a client of its port.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs):

```csharp
static string Word(bool on) => on ? "on" : "off";

// The line: 19200 baud, even parity, one stop bit, the default the Modbus
// specification sets. Eleven bits a character, and 3.5 of them of silence mark where
// a frame ends.
var settings = new SerialSettings(19_200, Parity.Even);
ulong gap = ModbusClient.FrameGapNanos(settings) / 1_000;
Console.WriteLine($"line         {settings}, {settings.BitsPerCharacter} bits a character, t3.5 is {gap} us");

// Each device's manual gives its unit address and where its values live. The meter
// keeps its measurements in input registers from 0: volts in tenths, amps in
// hundredths, then a fault word. The relay module has four relays as coils 0 to 3, and
// the tank's low-level float switch as discrete input 0, on while the water is below it.
const byte Meter = 17;
const byte Pump = 18;
using var meter = new ModbusServer(Meter);
meter.SetInputRegisters(0, [2301, 418, 0]);
using var relays = new ModbusServer(Pump);
relays.SetCoils(0, new bool[4]);
relays.SetDiscreteInputs(0, [true]);
using var line = new ModbusLine().Attach(meter).Attach(relays);

// The devices sit on a simulated line. On a gateway the port is
// SerialPort.Open("/dev/ttyUSB0", settings), and nothing after this statement changes.
using SerialPort port = line.Port(settings);
using var client = new ModbusClient(port);

// Poll the meter with function 0x04 for three input registers, and scale each one as
// its manual says.
ushort[] registers = client.ReadInputRegisters(Meter, 0, 3);
Console.WriteLine(Invariant(
    $"meter        {registers[0] / 10.0:F1} V, {registers[1] / 100.0:F2} A, faults {registers[2]}"));

// What that poll cost the line: the request, the reply, and the silence before the
// request.
long bytesOut = port.Written;
long bytesBack = port.Received;
ulong lineTime = settings.TransferMicros((int)bytesOut) + settings.TransferMicros((int)bytesBack) + gap;
Console.WriteLine(Invariant(
    $"poll         {bytesOut} bytes out, {bytesBack} back, {lineTime / 1_000.0:F2} ms of line time"));

// Read the float switch, and start the pump on relay 0 when the tank is low.
bool low = client.ReadDiscreteInputs(Pump, 0, 1)[0];
Console.WriteLine($"tank         low-level switch {Word(low)}");
if (low)
{
    client.WriteSingleCoil(Pump, 0, true);
}

bool[] states = client.ReadCoils(Pump, 0, 4);
Console.WriteLine($"relays       {string.Join(' ', states.Select(Word))}");

// A broadcast, to unit 0, reaches every device on the line and none answers: here
// every relay off at once. The client waits out the turnaround so each device has
// carried it out before the next request.
client.WriteMultipleCoils(ModbusClient.Broadcast, 0, new bool[4]);
Console.WriteLine($"broadcast    every relay off, no reply, {client.Turnaround.TotalMilliseconds} ms turnaround");
bool[] after = client.ReadCoils(Pump, 0, 4);
Console.WriteLine($"relays       {string.Join(' ', after.Select(Word))}");

// The meter keeps its measurements in input registers. Asking for them as holding
// registers, function 0x03, is the usual mistake with a new device, and the meter
// refuses it with an exception instead of answering.
ModbusClientException? refused = null;
try
{
    client.ReadHoldingRegisters(Meter, 0, 3);
}
catch (ModbusClientException error)
{
    refused = error;
    Console.WriteLine($"refused      {error.Message}");
}

// A unit that is not on the line never answers. The client gives up after its response
// timeout, one second unless told otherwise, which a simulated line counts instead of
// sleeping through.
ulong before = port.WaitedMicros;
ModbusClientException? silent = null;
try
{
    client.ReadInputRegisters(19, 0, 1);
}
catch (ModbusClientException error)
{
    silent = error;
    Console.WriteLine($"silent       {error.Message}, {(port.WaitedMicros - before) / 1_000} ms counted and not slept");
}
```
<!-- end -->

## On a board

The same client on a Raspberry Pi, through a USB RS485 adapter: the program asks every unit
address from 1 to 247 for holding register 0, and lists each device that answers. A device
answers with the value, or with an exception if it has no register 0, and either way it is
there. It is the first thing to run on a new line.

| Wire | From | To |
| --- | --- | --- |
| A, which the specification calls D0 | the adapter's A | every device's A |
| B, D1 | the adapter's B | every device's B |
| common | the adapter's ground | every device's ground or common terminal |
| termination | a resistor across A and B: 120 Ω to match the cable, or the 150 Ω the specification names | the two devices at the ends of the line, and nowhere else |

Each device keeps its own power supply. Short lines at 9600 or 19200 baud often work without
the termination resistors; a long one does not. Set the program's format to the one the
devices use: every device on one line has to share it.

A scan with a 100 ms response timeout takes about 25 seconds, most of it waiting on addresses
nobody answers. A device that cannot read the request, because the pair is swapped or the format
differs, does not answer at all, so a scan that finds nothing points at A and B and at the
format before anything else.

### Rust

<!-- snippet: examples/boards/raspberry-pi/src/bin/modbus.rs#example -->
From [`examples/boards/raspberry-pi/src/bin/modbus.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/bin/modbus.rs):

```rust
use pamoja_hal::port::{Parity, SerialPort, Settings};
use pamoja_modbus::{Client, ClientError};

/// A USB RS485 adapter. The kernel names the first one it finds `ttyUSB0`, or `ttyACM0` for an
/// adapter that presents itself as a modem.
const PORT: &str = "/dev/ttyUSB0";

fn main() -> Result<(), Box<dyn Error>> {
    // The format every device on the line uses, from their manuals: 19200 8E1 is the default
    // the specification sets, and many meters ship at 9600 8N1 instead.
    let settings = Settings::new(19_200).with_parity(Parity::Even);
    let port = SerialPort::open(PORT, settings)?;

    // A device that is there answers within a few milliseconds, so a short response timeout
    // keeps the scan of all 247 addresses under half a minute.
    let mut client = Client::new(port).with_response_timeout(Duration::from_millis(100));

    let mut found = 0;
    for unit in 1..=247 {
        // Holding register 0 is a question any device can answer, with its value or with an
        // exception, and either proves the device is there.
        match client.read_holding_registers(unit, 0, 1) {
            Ok(values) => {
                println!("unit {unit:3}  holding register 0 is {}", values[0]);
                found += 1;
            }
            Err(ClientError::Exception { exception, .. }) => {
                let code = exception.code();
                println!("unit {unit:3}  there, and refused register 0 with exception {code:#04x}");
                found += 1;
            }
            Err(ClientError::Timeout { received: 0, .. }) => {}
            Err(error) => println!("unit {unit:3}  {error}: check the format and the wiring"),
        }
    }
    println!("{found} units answered on {PORT} at {settings}");
    Ok(())
}
```
<!-- end -->

```sh
cd examples/boards/raspberry-pi
cargo run --release --bin modbus
```

### TypeScript

<!-- snippet: bindings/node/boards/raspberry-pi/modbus.ts#example -->
From [`bindings/node/boards/raspberry-pi/modbus.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/boards/raspberry-pi/modbus.ts):

```typescript
import { Parity, SerialPort } from '@pamoja/hal'
import { ModbusClient, ModbusClientError } from '@pamoja/modbus'

// A USB RS485 adapter. The kernel names the first one it finds ttyUSB0, or ttyACM0 for an
// adapter that presents itself as a modem.
const PORT = '/dev/ttyUSB0'

async function main(): Promise<void> {
  // The format every device on the line uses, from their manuals: 19200 8E1 is the default the
  // specification sets, and many meters ship at 9600 8N1 instead.
  const settings = { baud: 19200, parity: Parity.Even }
  const port = SerialPort.open(PORT, settings)

  // A device that is there answers within a few milliseconds, so a short response timeout
  // keeps the scan of all 247 addresses under half a minute.
  const client = new ModbusClient(port, { responseTimeoutMs: 100 })

  let found = 0
  for (let unit = 1; unit <= 247; unit += 1) {
    const label = `unit ${String(unit).padStart(3)}`
    // Holding register 0 is a question any device can answer, with its value or with an
    // exception, and either proves the device is there.
    try {
      const [value] = await client.readHoldingRegisters(unit, 0, 1)
      console.log(`${label}  holding register 0 is ${value}`)
      found += 1
    } catch (error) {
      if (!(error instanceof ModbusClientError)) {
        throw error
      }
      if (error.kind === 'Exception') {
        const code = (error.exception ?? 0).toString(16).padStart(2, '0')
        console.log(`${label}  there, and refused register 0 with exception 0x${code}`)
        found += 1
      } else if (!(error.kind === 'Timeout' && error.received === 0)) {
        console.log(`${label}  ${error.message}: check the format and the wiring`)
      }
    }
  }
  console.log(`${found} units answered on ${PORT} at ${SerialPort.describe(settings)}`)
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
```
<!-- end -->

```sh
npm --prefix bindings/node run boards
node bindings/node/build/boards/raspberry-pi/modbus.js
```

### Python

<!-- snippet: bindings/python/boards/raspberry_pi/modbus.py#example -->
From [`bindings/python/boards/raspberry_pi/modbus.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/boards/raspberry_pi/modbus.py):

```python
from pamoja.hal import Parity, SerialPort, SerialSettings
from pamoja.modbus import ModbusClient, ModbusClientError

# A USB RS485 adapter. The kernel names the first one it finds ttyUSB0, or ttyACM0 for an
# adapter that presents itself as a modem.
PORT = "/dev/ttyUSB0"


def main() -> None:
    # The format every device on the line uses, from their manuals: 19200 8E1 is the default
    # the specification sets, and many meters ship at 9600 8N1 instead.
    settings = SerialSettings(19_200, Parity.EVEN)
    port = SerialPort.open(PORT, settings)

    # A device that is there answers within a few milliseconds, so a short response timeout
    # keeps the scan of all 247 addresses under half a minute.
    client = ModbusClient(port, response_timeout=0.1)

    found = 0
    for unit in range(1, 248):
        # Holding register 0 is a question any device can answer, with its value or with an
        # exception, and either proves the device is there.
        try:
            [value] = client.read_holding_registers(unit, 0, 1)
            print(f"unit {unit:3}  holding register 0 is {value}")
            found += 1
        except ModbusClientError as error:
            if error.kind == "exception":
                print(f"unit {unit:3}  there, and refused register 0 with exception {error.exception:#04x}")
                found += 1
            elif not (error.kind == "timeout" and error.received == 0):
                print(f"unit {unit:3}  {error}: check the format and the wiring")
    print(f"{found} units answered on {PORT} at {settings}")


if __name__ == "__main__":
    main()
```
<!-- end -->

```sh
python bindings/python/boards/raspberry_pi/modbus.py
```

### C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Modbus.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Modbus.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Modbus.cs):

```csharp
using Pamoja.Hal;
using Pamoja.Modbus;

namespace Boards.RaspberryPi;

/// <summary>
/// A Modbus line scan: asks every unit address on an RS485 line, through a USB adapter, which
/// devices are there. Wire the adapter's A and B to every device's A and B, join the grounds,
/// and set the line's format below to the one the devices use.
/// </summary>
public static class ModbusScan
{
    // A USB RS485 adapter. The kernel names the first one it finds ttyUSB0, or ttyACM0 for an
    // adapter that presents itself as a modem.
    private const string Port = "/dev/ttyUSB0";

    /// <summary>Asks units 1 to 247 in turn and reports every one that answers.</summary>
    public static void Run()
    {
        // The format every device on the line uses, from their manuals: 19200 8E1 is the
        // default the specification sets, and many meters ship at 9600 8N1 instead.
        var settings = new SerialSettings(19_200, Parity.Even);
        using SerialPort port = SerialPort.Open(Port, settings);

        // A device that is there answers within a few milliseconds, so a short response timeout
        // keeps the scan of all 247 addresses under half a minute.
        using var client = new ModbusClient(port) { ResponseTimeout = TimeSpan.FromMilliseconds(100) };

        int found = 0;
        for (int unit = 1; unit <= 247; unit++)
        {
            // Holding register 0 is a question any device can answer, with its value or with
            // an exception, and either proves the device is there.
            try
            {
                ushort value = client.ReadHoldingRegisters((byte)unit, 0, 1)[0];
                Console.WriteLine($"unit {unit,3}  holding register 0 is {value}");
                found++;
            }
            catch (ModbusClientException error)
            {
                if (error.Kind == ModbusClientErrorKind.Exception)
                {
                    Console.WriteLine($"unit {unit,3}  there, and refused register 0 with exception 0x{(byte)error.ExceptionCode!:x2}");
                    found++;
                }
                else if (!(error.Kind == ModbusClientErrorKind.Timeout && error.Received == 0))
                {
                    Console.WriteLine($"unit {unit,3}  {error.Message}: check the format and the wiring");
                }
            }
        }

        Console.WriteLine($"{found} units answered on {Port} at {settings}");
    }
}
```
<!-- end -->

```sh
dotnet run --project bindings/dotnet/samples/Pamoja.Boards -- raspberry-pi/modbus
```

## Values at a glance

**The four tables a device serves**, from section 4.3 of the application protocol
specification. Each holds up to 65536 entries, and a device holds only the ones its manual
lists:

| Table | Each entry | Access | Read with | Write with |
| --- | --- | --- | --- | --- |
| Coils | one bit | read and write | `0x01` | `0x05` one, `0x0F` many |
| Discrete inputs | one bit | read only | `0x02` | none |
| Input registers | a 16-bit word | read only | `0x04` | none |
| Holding registers | a 16-bit word | read and write | `0x03` | `0x06` one, `0x10` many |

A manual numbers each table's entries from 1, and a request addresses entry X as X - 1, section
4.4. Some manuals also prefix the table's digit: 0 for coils, 1 for discrete inputs, 3 for input
registers, and 4 for holding registers, so holding register 108 is written 40108 and is address
107 in a request. Every call in pamoja takes the address.

**The eight functions**, with the limits the specification sets in section 6:

| Function | Code | How many | Request | Reply |
| --- | --- | --- | --- | --- |
| read coils | `0x01` | 1 to 2000 | 8 bytes | 5 + ⌈n / 8⌉ bytes |
| read discrete inputs | `0x02` | 1 to 2000 | 8 bytes | 5 + ⌈n / 8⌉ bytes |
| read holding registers | `0x03` | 1 to 125 | 8 bytes | 5 + 2n bytes |
| read input registers | `0x04` | 1 to 125 | 8 bytes | 5 + 2n bytes |
| write single coil | `0x05` | 1, sent as `0xFF00` for on and `0x0000` for off | 8 bytes | 8 bytes, the request echoed |
| write single register | `0x06` | 1 | 8 bytes | 8 bytes, the request echoed |
| write multiple coils | `0x0F` | 1 to 1968 | 9 + ⌈n / 8⌉ bytes | 8 bytes |
| write multiple registers | `0x10` | 1 to 123 | 9 + 2n bytes | 8 bytes |

An exception reply is 5 bytes whatever was asked: the unit, the function with its high bit set,
the exception code, and the CRC.

**Unit addresses**, from section 2.2 of the serial line specification:

| Address | What it is |
| --- | --- |
| 0 | broadcast: every device carries out a write, and none answers; a read cannot be broadcast |
| 1 to 247 | one device each; a server takes one of these |
| 248 to 255 | reserved; a client still sends to them, since some devices answer on one |

**Timing on the line.** RTU sends eleven bits a character: a start bit, eight data bits, a
parity bit, and a stop bit, or a second stop bit when there is no parity, section 2.5.1. A frame
ends at 3.5 characters of silence, and a device drops a frame with a gap of more than 1.5
characters inside it, section 2.5.1.1; above 19200 baud both are fixed. The line time of a poll
is the request, the reply, and the silence before the request, computed by the library:

| Format | One character | t1.5 | t3.5 | Read 3 registers | Write 1 register |
| --- | --- | --- | --- | --- | --- |
| 9600 8E1 | 1145.83 µs | 1718.75 µs | 4010.42 µs | 25.78 ms | 22.34 ms |
| 9600 8N2 | 1145.83 µs | 1718.75 µs | 4010.42 µs | 25.78 ms | 22.34 ms |
| 19200 8E1 | 572.92 µs | 859.38 µs | 2005.21 µs | 12.89 ms | 11.17 ms |
| 38400 8E1 | 286.46 µs | 750 µs | 1750 µs | 7.19 ms | 6.33 ms |
| 115200 8E1 | 95.49 µs | 750 µs | 1750 µs | 3.57 ms | 3.28 ms |

The device's own time to answer comes on top. A client reads each reply to the length its
request implies, rather than timing the gaps inside it, since a USB adapter hands the host
bytes in bursts that hide them; a device built on a microcontroller, reading its own UART, does
time t1.5.

**The client's two waits**, from section 2.4.1:

| Wait | What it is for | pamoja's default | The specification's range |
| --- | --- | --- | --- |
| response timeout | how long a unicast request waits for its whole reply | 1 s | 1 s to several seconds at 9600 baud |
| turnaround | the quiet after a broadcast, so every device has carried it out | 100 ms | 100 ms to 200 ms |

**Exceptions**, from section 7 of the application protocol specification. A device checks a
request against the first three in this order, so the first rule it breaks names the exception,
and the rest report what happened once it had begun:

| Code | Name | A device sends it when |
| --- | --- | --- |
| `0x01` | illegal function | it does not serve the function |
| `0x03` | illegal data value | a quantity is out of range, a byte count does not match, or a coil is written with anything but on or off |
| `0x02` | illegal data address | an address in the span is not one it has |
| `0x04` | server device failure | it failed while carrying the request out |
| `0x05` | acknowledge | it accepted a long job and is still working on it |
| `0x06` | server device busy | it is busy with a long job; ask again later |
| `0x08` | memory parity error | its extended file memory failed a consistency check |
| `0x0A` | gateway path unavailable | a gateway could not route the request |
| `0x0B` | gateway target device failed to respond | a gateway got no answer from the device behind it |

A `ModbusServer` sends the first three, in that order.

**The client's settings in each language:**

### Rust

| Setting | How | Default |
| --- | --- | --- |
| response timeout | `.with_response_timeout(Duration)`, or `set_response_timeout` | `RESPONSE_TIMEOUT`, 1 s |
| turnaround | `.with_turnaround(Duration)`, or `set_turnaround` | `TURNAROUND`, 100 ms |

### TypeScript

| Setting | How | Default |
| --- | --- | --- |
| response timeout | `{ responseTimeoutMs }`, or `client.responseTimeoutMs = ms` | 1000 |
| turnaround | `{ turnaroundMs }`, or `client.turnaroundMs = ms` | 100 |

### Python

| Setting | How | Default |
| --- | --- | --- |
| response timeout | `response_timeout=seconds`, or `client.response_timeout = seconds` | 1.0 |
| turnaround | `turnaround=seconds`, or `client.turnaround = seconds` | 0.1 |

### C#

| Setting | How | Default |
| --- | --- | --- |
| response timeout | `ResponseTimeout = TimeSpan` | 1 s |
| turnaround | `Turnaround = TimeSpan` | 100 ms |

<!-- languages end -->

**The same calls in each language:**

| To | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| make a device | `Server::new(unit)?` | `new ModbusServer(unit)` | `ModbusServer(unit)` | `new ModbusServer(unit)` |
| fill a table | `.with_input_registers(start, &values)` | `setInputRegisters(start, values)` | `set_input_registers(start, values)` | `SetInputRegisters(start, values)` |
| read an entry back | `holding_register(address)` | `holdingRegister(address)` | `holding_register(address)` | `HoldingRegister(address)` |
| put devices on a line | `line.attach(server)` | `line.attach(server)` | `line.attach(server)` | `line.Attach(server)` |
| a port on that line | `SerialPort::simulated(settings, line)` | `line.port(settings)` | `line.port(settings)` | `line.Port(settings)` |
| a client | `Client::new(port)` | `new ModbusClient(port)` | `ModbusClient(port)` | `new ModbusClient(port)` |
| read registers | `read_input_registers(unit, start, n)?` | `await readInputRegisters(unit, start, n)` | `read_input_registers(unit, start, n)` | `ReadInputRegisters(unit, start, n)` |
| write a coil | `write_single_coil(unit, address, on)?` | `await writeSingleCoil(unit, address, on)` | `write_single_coil(unit, address, on)` | `WriteSingleCoil(unit, address, on)` |
| broadcast | unit `BROADCAST` | unit `BROADCAST` | unit `BROADCAST` | unit `ModbusClient.Broadcast` |
| the frame gap | `Client::frame_gap(settings)` | `ModbusClient.frameGapNanos(settings)` | `ModbusClient.frame_gap_nanos(settings)` | `ModbusClient.FrameGapNanos(settings)` |
| answer a frame by hand | `server.answer(&frame)` | `server.answer(frame)` | `server.answer(frame)` | `server.Answer(frame)` |

**Wiring a line**, from section 3.4 of the serial line specification:

| Rule | What the specification says |
| --- | --- |
| devices | 32 on a line without a repeater, always; more where each device's manual allows it |
| length | 1000 m end to end at up to 9600 baud on AWG26 or thicker; drops off the trunk no longer than 20 m |
| termination | at each end of the trunk and nowhere else: 150 Ω at 0.5 W, or 120 Ω at 0.25 W in series with 1 nF when the line is biased |
| biasing | at one point for the whole line, only if a device needs it: 450 Ω to 650 Ω from B to 5 V and from A to the common |
| ground | the common joined to protective ground at one point, usually at the gateway |

## When it goes wrong

What the client and the server refuse, and what they say:

| What happened | The message | What to check |
| --- | --- | --- |
| the device refused | `unit 17 refused function 0x03 with exception 0x02, illegal data address` | the function and the address in the device's manual |
| nothing answered | `unit 19 did not answer within the response timeout` | the unit address, the format, and A and B |
| a reply stopped partway | `unit 17 sent 4 bytes of a reply and then went quiet` | the response timeout, and noise on the line |
| the reply failed its CRC | `the reply is not a valid frame: modbus CRC mismatch: expected 0x…, found 0x…` | noise, the termination, and an adapter that echoes |
| another unit answered | `unit 18 answered a request for unit 17` | a late reply to an earlier request; lengthen the response timeout |
| the reply was for another function | `the reply names function 0x04, not the 0x03 asked for` | a late reply to an earlier request, or a second client on the line |
| the reply did not match the request | `unit 17 answered with values the request did not ask for` | the device's firmware |
| a read was broadcast | `a read cannot be broadcast, since no device answers a broadcast` | ask one unit |
| a request too large for one frame | `the request cannot be sent: the number of values is outside what one modbus frame carries` | at most 125 registers a read and 123 a write |
| a server at a unit it cannot take | `unit 0 is not a device address: 0 is broadcast, and 248 to 255 are reserved` | a unit from 1 to 247 |

How each language hands those over:

| Language | A transaction that fails | Its kind | The device's exception |
| --- | --- | --- | --- |
| Rust | `Err(ClientError)` | the variant: `Exception`, `Timeout`, `Frame`, and the rest | `ClientError::Exception { exception, .. }` |
| TypeScript | a rejected promise with a `ModbusClientError` | `kind`: `'Exception'`, `'Timeout'`, `'Frame'`, and the rest | `exception`, an `ExceptionCode` |
| Python | `ModbusClientError`, a `PamojaError` | `kind`: `"exception"`, `"timeout"`, `"frame"`, and the rest | `exception`, an `int` that compares with `Exception_` |
| C# | `ModbusClientException`, a `PamojaException` | `Kind`, a `ModbusClientErrorKind` | `ExceptionCode`, a `ModbusExceptionCode` |

The mistakes that cost an afternoon:

- **Nothing answers anywhere.** A and B are swapped, since manufacturers do not agree on which is
  which, or the line runs at another format than the program's. A device that cannot read a
  request stays silent rather than complaining. Swap A and B and scan again, then try the other
  common format.
- **Exception 0x02 on a register the manual lists.** The manual numbers from 1, or prefixes the
  table: register 40108 is address 107. Or the value is in the other register table: a meter's
  measurements are usually input registers, function `0x04`, not holding registers.
- **Every reply fails its CRC.** The adapter hands back its own request ahead of each reply,
  which some do when their receiver stays on while they send. Use an adapter that turns its
  receiver off while it transmits.
- **It works on the bench and fails in the field.** A long line needs its two termination
  resistors and one ground point, and its errors show up as the odd CRC failure before they
  stop it altogether.
- **A device misses every other poll.** It needs more time between requests than the 3.5
  characters the client leaves. Wait between polls, as its manual asks.
- **A broadcast write, then an error from the next request.** A slow device is still carrying
  the broadcast out; lengthen the turnaround.
- **One unit's replies fail their CRC, and the rest are fine.** Two devices share that address and
  answer together. Take one off the line and change the other's address with its own software.
- **A 32-bit value reads as nonsense.** Many meters spread one value across two registers, and
  devices disagree on which comes first. The manual says; read both registers in one request so
  they are from the same moment.

## Where next

<!-- table: next modbus -->
- [Serial framing](serial.md): SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets.
- [Device profiles](profile.md): Named, ready-to-run device profiles from plain data or a JSON manifest, run as a node in every language.
- [Buses](hal.md): The embedded-hal traits every driver takes, a bit-banged 1-Wire bus, simulated parts and scripted buses that stand in for hardware, the Linux backends over i2c-dev, spidev, and the GPIO character device, one I2C bus and one serial port a program and its drivers share, and delays that sleep or only count.
- Beside it: [Buses and links](../buses.md).
- Also in Field I/O: [CAN and J1939](can.md), [I2C, SPI, and GPIO](gpio.md).
<!-- end -->

## Reference

<!-- table: reference modbus -->
- Rust: [`pamoja-modbus`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-modbus)
- TypeScript: [`@pamoja/modbus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-modbus)
- Python: [`pamoja.modbus`](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-modbus)
- C#: [`Pamoja.Modbus`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-modbus)
<!-- end -->
