# Serial framing

A UART moves bytes one character at a time, each wrapped in a start bit and a stop bit, and
nothing on the line marks where one message ends and the next begins. A read hands a program
whatever has arrived: half a message, three messages, or the tail of one and the head of the
next. Byte stuffing supplies the boundary. It reserves one byte value to end a frame and
encodes each payload so that value can never appear inside it. pamoja implements the two
framings field hardware speaks, SLIP from RFC 1055 and COBS from Cheshire and Baker, with
streaming decoders that pull whole frames out of whatever a read returns. It also provides
the port the frames travel over, from all four languages.

The framing is pure byte work with no allocation, so the same code runs on a gateway and on
the microcontroller at the other end of the cable. The port, `SerialPort` in the buses package,
is one line shared by a program and every driver on it, with one of five things on the other
end: the kernel's serial device on a Linux board, a line looped back on itself as with TX wired
to RX, one end of a null-modem pair, a simulated device that answers each write, or a script of
the writes a driver is expected to make. Reads on anything but the kernel's device never wait,
and count the time they would have waited instead, so a program that talks over a UART is
written and tested with nothing plugged in.

## What the example does

It puts a weather mast on one serial line: a node at the top that reads the wind, and a gateway
at the foot. With nothing plugged in, the two are the two ends of a null-modem pair at 115200
baud, eight data bits, no parity, one stop bit.

Each reading is a two-byte sequence number, most significant byte first, then its text, eleven
bytes in all. The node frames three readings with COBS and writes them back to back. The
gateway's first read returns all three frames at once, 39 bytes, and the decoder splits them
back into payloads at each delimiter. The example then reads each frame's time on the wire off
the port's settings.

Three scenes follow. The node restarts halfway through a frame, and as it comes back up it
sends a lone delimiter, which closes off the half frame so the gateway drops it rather than
gluing it to the next one. The same reading is framed with SLIP beside COBS to compare the
cost. And the node goes quiet: the gateway's read waits out its half-second timeout, which a
port with nothing plugged in counts rather than sleeps through.

It proves:

- 115200 8N1 is ten bits a character, 86.81 µs each, so a 13-byte frame takes 1.13 ms on the
  wire.
- The sequence numbers are full of zeros, and COBS carries them anyway: each 11-byte payload
  becomes a 13-byte frame, one code byte and one delimiter longer, with no zero inside it.
- A read returns what has arrived, all three frames in one 39-byte read here, and the decoder
  hands back each payload whole, in order.
- A frame cut short by a restart is dropped and counted, once, and the reading sent after it
  comes through intact.
- With no reserved bytes in the payload, SLIP frames the reading in 12 bytes and COBS in 13.
- A read on a silent line returns nothing after its timeout, and the 500 ms is counted without
  the program waiting for it.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example serial" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example serial</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- serial" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- serial</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/serial.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/serial.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- serial" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- serial</code></div>
</div>
<!-- end -->

## Rust

In Rust the framings are the `slip` and `cobs` modules of `pamoja-serial`, both `no_std` and
allocation-free: `encode` and `decode` work in caller buffers sized by `max_encoded_len`, and
`SlipDecoder<N>` and `CobsDecoder<N>` take one byte at a time through `push`, which returns the
payload when a delimiter completes a frame. A malformed frame is an `Err(SerialError)`,
`TruncatedFrame` or `BufferTooSmall`, and the decoder carries on at the next byte. The port is
`pamoja_hal::port::SerialPort`, from the `std` feature, and `SerialPort::open` needs the
`linux` feature on a Linux board. It takes a `Settings`, built with `Settings::new(baud)` and
`with_parity` and `with_stop_bits`, which also gives `character_nanos` and `transfer_micros`.
`write` and `read` return `Result<_, PortError>`, and a simulated device is anything that
implements `Peer`.

<!-- snippet: examples/guides/serial.rs#example -->
From [`examples/guides/serial.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/serial.rs):

```rust
use std::time::Duration;

use pamoja_hal::port::{SerialPort, Settings};
use pamoja_serial::cobs::{self, CobsDecoder};
use pamoja_serial::slip;

// A reading is a two-byte sequence number, most significant byte first, then its text.
fn reading(sequence: u16, text: &str) -> Vec<u8> {
    let mut payload = sequence.to_be_bytes().to_vec();
    payload.extend_from_slice(text.as_bytes());
    payload
}

// The line: 115200 baud, eight data bits, no parity, one stop bit. Ten bits a character.
let settings = Settings::new(115_200);
println!(
    "line         {settings}, {} bits a character, {:.2} us each",
    settings.bits_per_character(),
    settings.character_nanos() as f64 / 1_000.0
);

// The two ends of the cable with nothing plugged in. On a Raspberry Pi the gateway's end
// is SerialPort::open("/dev/serial0", settings) and nothing after this statement changes.
let (gateway, node) = SerialPort::pair(settings);

// A UART carries bytes, and nothing marks where a message ends, so the node frames each
// reading with COBS: zero becomes the one byte that ends a frame and never appears inside
// one, which matters here, since the sequence number is full of zeros.
let texts = ["wind=12.4", "wind=13.1", "wind=11.8"];
let mut sent = 0;
let mut frame = [0u8; cobs::max_encoded_len(32)];
for (sequence, text) in (1..).zip(texts) {
    let framed = cobs::encode(&reading(sequence, text), &mut frame)?;
    node.write(&frame[..framed])?;
    sent += framed;
}
println!(
    "node         {} readings of {} bytes, framed as {sent} bytes",
    texts.len(),
    reading(1, texts[0]).len()
);

// A read returns whatever has arrived, which is rarely one frame: here it is all three.
// The decoder splits the stream back into payloads at each delimiter.
let mut buffer = [0u8; 256];
let got = gateway.read(&mut buffer, Duration::from_millis(100))?;
println!("gateway      {got} bytes in one read");
let mut decoder: CobsDecoder<64> = CobsDecoder::new();
let mut payloads = Vec::new();
for &byte in &buffer[..got] {
    if let Ok(Some(payload)) = decoder.push(byte) {
        payloads.push(payload.to_vec());
    }
}
for payload in &payloads {
    let (sequence, text) = payload.split_at(2);
    let sequence = u16::from_be_bytes([sequence[0], sequence[1]]);
    println!("reading {sequence}    {}", String::from_utf8_lossy(text));
}

// What one frame costs on the wire at this speed, start and stop bits included.
let frame_len = sent / texts.len();
println!(
    "on the wire  {:.2} ms for a {frame_len}-byte frame at {settings}",
    settings.transfer_micros(frame_len) as f64 / 1_000.0
);

// The node restarts partway through a frame. As it comes back up it sends a lone
// delimiter, which closes off the half frame, so the gateway drops it rather than gluing
// it to the next one, and then it sends the reading again.
let framed = cobs::encode(&reading(4, "wind=12.9"), &mut frame)?;
node.write(&frame[..framed / 2])?;
node.write(&[cobs::DELIMITER])?;
node.write(&frame[..framed])?;
let got = gateway.read(&mut buffer, Duration::from_millis(100))?;
let mut dropped = 0;
let mut resent = Vec::new();
for &byte in &buffer[..got] {
    match decoder.push(byte) {
        Ok(Some(payload)) => resent.push(payload.to_vec()),
        Ok(None) => {}
        Err(_) => dropped += 1,
    }
}
println!(
    "restart      {dropped} frame cut short and dropped, then {}",
    String::from_utf8_lossy(&resent[0][2..])
);

// SLIP, the older framing, ends a frame with one reserved byte and escapes that byte and
// its own escape byte inside one. With no reserved bytes in a reading it costs a byte less
// than COBS; a payload full of them costs up to twice its length under SLIP, and never
// more than one byte in 254 over under COBS.
let first = reading(1, texts[0]);
let mut slip_frame = [0u8; slip::max_encoded_len(32)];
let slip_len = slip::encode(&first, &mut slip_frame)?;
let cobs_len = cobs::encode(&first, &mut frame)?;
println!(
    "framing      {} payload bytes: {slip_len} under SLIP, {cobs_len} under COBS",
    first.len()
);

// The node goes quiet. A read waits for the first byte up to its timeout; on a port with
// nothing plugged in it returns at once and counts the wait instead of sleeping through
// it, so a test of a silent node takes no time.
let before = gateway.waited_micros();
let got = gateway.read(&mut buffer, Duration::from_millis(500))?;
println!(
    "silence      {got} bytes in {} ms, counted and not slept",
    (gateway.waited_micros() - before) / 1_000
);
```
<!-- end -->

## TypeScript

In TypeScript the framings are `slip` and `cobs` in `@pamoja/serial`, each with `encode`,
`decode`, and `maxEncodedLen` over `Buffer`s, and `SlipDecoder` and `CobsDecoder`, whose `feed`
takes a whole chunk and returns every payload it completed, counting dropped frames on
`discarded`. The port is `SerialPort` in `@pamoja/hal`, whose settings are a plain object,
`{ baud, parity, stopBits }`, with `SerialPort.describe`, `SerialPort.characterNanos`, and
`SerialPort.transferMicros` to read them. `write` and `read` return promises, run on a worker
thread so a real line never blocks the event loop, and reject with the reason.

<!-- snippet: bindings/node/guides/serial.ts#example -->
From [`bindings/node/guides/serial.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/serial.ts):

```typescript
import { SerialPort } from '@pamoja/hal'
import { COBS_DELIMITER_BYTE, CobsDecoder, cobs, slip } from '@pamoja/serial'

// A reading is a two-byte sequence number, most significant byte first, then its text.
function reading(sequence: number, text: string): Buffer {
  const number = Buffer.alloc(2)
  number.writeUInt16BE(sequence)
  return Buffer.concat([number, Buffer.from(text)])
}

async function main() {
  // The line: 115200 baud, eight data bits, no parity, one stop bit. Ten bits a character.
  const settings = { baud: 115200 }
  const line = SerialPort.describe(settings)
  const characterMicros = SerialPort.characterNanos(settings) / 1000
  console.log(
    `line         ${line}, ${SerialPort.bitsPerCharacter(settings)} bits a character, ` +
      `${characterMicros.toFixed(2)} us each`,
  )

  // The two ends of the cable with nothing plugged in. On a Raspberry Pi the gateway's end is
  // SerialPort.open('/dev/serial0', settings) and nothing after this statement changes.
  const [gateway, node] = SerialPort.pair(settings)

  // A UART carries bytes, and nothing marks where a message ends, so the node frames each
  // reading with COBS: zero becomes the one byte that ends a frame and never appears inside
  // one, which matters here, since the sequence number is full of zeros.
  const texts = ['wind=12.4', 'wind=13.1', 'wind=11.8']
  let sent = 0
  for (const [index, text] of texts.entries()) {
    const frame = cobs.encode(reading(index + 1, text))
    await node.write(frame)
    sent += frame.length
  }
  console.log(
    `node         ${texts.length} readings of ${reading(1, texts[0]).length} bytes, framed as ${sent} bytes`,
  )

  // A read returns whatever has arrived, which is rarely one frame: here it is all three. The
  // decoder splits the stream back into payloads at each delimiter.
  const arrived = await gateway.read(256, 100)
  console.log(`gateway      ${arrived.length} bytes in one read`)
  const decoder = new CobsDecoder()
  const payloads = decoder.feed(arrived)
  for (const payload of payloads) {
    console.log(`reading ${payload.readUInt16BE(0)}    ${payload.subarray(2).toString()}`)
  }

  // What one frame costs on the wire at this speed, start and stop bits included.
  const frameLength = sent / texts.length
  const frameMillis = SerialPort.transferMicros(settings, frameLength) / 1000
  console.log(`on the wire  ${frameMillis.toFixed(2)} ms for a ${frameLength}-byte frame at ${line}`)

  // The node restarts partway through a frame. As it comes back up it sends a lone delimiter,
  // which closes off the half frame, so the gateway drops it rather than gluing it to the next
  // one, and then it sends the reading again.
  const again = cobs.encode(reading(4, 'wind=12.9'))
  await node.write(again.subarray(0, Math.floor(again.length / 2)))
  await node.write(Buffer.from([COBS_DELIMITER_BYTE]))
  await node.write(again)
  const droppedBefore = decoder.discarded
  const resent = decoder.feed(await gateway.read(256, 100))
  const dropped = decoder.discarded - droppedBefore
  console.log(
    `restart      ${dropped} frame cut short and dropped, then ${resent[0].subarray(2).toString()}`,
  )

  // SLIP, the older framing, ends a frame with one reserved byte and escapes that byte and its
  // own escape byte inside one. With no reserved bytes in a reading it costs a byte less than
  // COBS; a payload full of them costs up to twice its length under SLIP, and never more than
  // one byte in 254 over under COBS.
  const first = reading(1, texts[0])
  const slipLength = slip.encode(first).length
  const cobsLength = cobs.encode(first).length
  console.log(
    `framing      ${first.length} payload bytes: ${slipLength} under SLIP, ${cobsLength} under COBS`,
  )

  // The node goes quiet. A read waits for the first byte up to its timeout; on a port with
  // nothing plugged in it resolves at once and counts the wait instead of sleeping through it,
  // so a test of a silent node takes no time.
  const waitedBefore = gateway.waitedMicros
  const quiet = await gateway.read(256, 500)
  const waited = (gateway.waitedMicros - waitedBefore) / 1000
  console.log(`silence      ${quiet.length} bytes in ${waited} ms, counted and not slept`)

  return { settings, payloads, sent, dropped, resent, slipLength, cobsLength, gateway, node, again }
}

main()
```
<!-- end -->

## Python

In Python the framings are `slip` and `cobs` in `pamoja.serial`, over `bytes`, and
`SlipDecoder` and `CobsDecoder` with `feed` and `discarded`. The port is `SerialPort` in
`pamoja.hal`, which takes a frozen `SerialSettings(baud, parity, stop_bits)`; its `str` is the
way a device's manual writes the format, `9600 8E1`. `read` takes a size and a timeout in
seconds. Every call releases the interpreter while the line is busy, and a failure raises
`PamojaError` from `pamoja.core`.

<!-- snippet: bindings/python/guides/serial.py#example -->
From [`bindings/python/guides/serial.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/serial.py):

```python
from pamoja.hal import SerialPort, SerialSettings
from pamoja.serial import COBS_DELIMITER, CobsDecoder, cobs, slip


def reading(sequence: int, text: str) -> bytes:
    """A reading: a two-byte sequence number, most significant byte first, then its text."""
    return sequence.to_bytes(2, "big") + text.encode()


# The line: 115200 baud, eight data bits, no parity, one stop bit. Ten bits a character.
settings = SerialSettings(115_200)
print(
    f"line         {settings}, {settings.bits_per_character} bits a character, "
    f"{settings.character_nanos / 1_000:.2f} us each"
)

# The two ends of the cable with nothing plugged in. On a Raspberry Pi the gateway's end is
# SerialPort.open("/dev/serial0", settings) and nothing after this statement changes.
gateway, node = SerialPort.pair(settings)

# A UART carries bytes, and nothing marks where a message ends, so the node frames each reading
# with COBS: zero becomes the one byte that ends a frame and never appears inside one, which
# matters here, since the sequence number is full of zeros.
texts = ["wind=12.4", "wind=13.1", "wind=11.8"]
sent = 0
for sequence, text in enumerate(texts, start=1):
    frame = cobs.encode(reading(sequence, text))
    node.write(frame)
    sent += len(frame)
print(
    f"node         {len(texts)} readings of {len(reading(1, texts[0]))} bytes, "
    f"framed as {sent} bytes"
)

# A read returns whatever has arrived, which is rarely one frame: here it is all three. The
# decoder splits the stream back into payloads at each delimiter.
arrived = gateway.read(256, timeout=0.1)
print(f"gateway      {len(arrived)} bytes in one read")
decoder = CobsDecoder()
payloads = decoder.feed(arrived)
for payload in payloads:
    print(f"reading {int.from_bytes(payload[:2], 'big')}    {payload[2:].decode()}")

# What one frame costs on the wire at this speed, start and stop bits included.
frame_length = sent // len(texts)
print(
    f"on the wire  {settings.transfer_micros(frame_length) / 1_000:.2f} ms "
    f"for a {frame_length}-byte frame at {settings}"
)

# The node restarts partway through a frame. As it comes back up it sends a lone delimiter,
# which closes off the half frame, so the gateway drops it rather than gluing it to the next
# one, and then it sends the reading again.
again = cobs.encode(reading(4, "wind=12.9"))
node.write(again[: len(again) // 2])
node.write(bytes([COBS_DELIMITER]))
node.write(again)
dropped_before = decoder.discarded
resent = decoder.feed(gateway.read(256, timeout=0.1))
dropped = decoder.discarded - dropped_before
print(f"restart      {dropped} frame cut short and dropped, then {resent[0][2:].decode()}")

# SLIP, the older framing, ends a frame with one reserved byte and escapes that byte and its
# own escape byte inside one. With no reserved bytes in a reading it costs a byte less than
# COBS; a payload full of them costs up to twice its length under SLIP, and never more than
# one byte in 254 over under COBS.
first = reading(1, texts[0])
slip_length = len(slip.encode(first))
cobs_length = len(cobs.encode(first))
print(
    f"framing      {len(first)} payload bytes: {slip_length} under SLIP, "
    f"{cobs_length} under COBS"
)

# The node goes quiet. A read waits for the first byte up to its timeout; on a port with
# nothing plugged in it returns at once and counts the wait instead of sleeping through it, so
# a test of a silent node takes no time.
waited_before = gateway.waited_micros
quiet = gateway.read(256, timeout=0.5)
waited = (gateway.waited_micros - waited_before) // 1_000
print(f"silence      {len(quiet)} bytes in {waited} ms, counted and not slept")
```
<!-- end -->

## C#

In C# the framings are static methods of `Serial` in `Pamoja.Serial`, `SlipEncode`,
`CobsEncode`, and their decoders, and `SlipDecoder` and `CobsDecoder` with `Feed` and
`Discarded`. The port is `SerialPort` in `Pamoja.Hal`, which takes a `SerialSettings` record
struct, `new SerialSettings(9_600, Parity.Even)`, whose `ToString` gives `9600 8E1`. `Read`
takes a `TimeSpan`, and `Pair` returns a tuple of two ends. Ports and decoders hold native
handles and are `IDisposable`, and a failure throws `PamojaException` with the reason.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs):

```csharp
// A reading is a two-byte sequence number, most significant byte first, then its text.
static byte[] Reading(ushort sequence, string text)
{
    byte[] payload = new byte[2 + Encoding.UTF8.GetByteCount(text)];
    BinaryPrimitives.WriteUInt16BigEndian(payload, sequence);
    Encoding.UTF8.GetBytes(text, payload.AsSpan(2));
    return payload;
}

// The line: 115200 baud, eight data bits, no parity, one stop bit. Ten bits a character.
var settings = new SerialSettings(115_200);
Console.WriteLine(Invariant(
    $"line         {settings}, {settings.BitsPerCharacter} bits a character, {settings.CharacterNanos / 1_000.0:F2} us each"));

// The two ends of the cable with nothing plugged in. On a Raspberry Pi the gateway's end
// is SerialPort.Open("/dev/serial0", settings) and nothing after this statement changes.
var (gateway, node) = SerialPort.Pair(settings);
using (gateway)
using (node)
{
    // A UART carries bytes, and nothing marks where a message ends, so the node frames
    // each reading with COBS: zero becomes the one byte that ends a frame and never
    // appears inside one, which matters here, since the sequence number is full of zeros.
    string[] texts = ["wind=12.4", "wind=13.1", "wind=11.8"];
    int sent = 0;
    for (int index = 0; index < texts.Length; index++)
    {
        byte[] frame = Serial.CobsEncode(Reading((ushort)(index + 1), texts[index]));
        node.Write(frame);
        sent += frame.Length;
    }

    Console.WriteLine(
        $"node         {texts.Length} readings of {Reading(1, texts[0]).Length} bytes, framed as {sent} bytes");

    // A read returns whatever has arrived, which is rarely one frame: here it is all
    // three. The decoder splits the stream back into payloads at each delimiter.
    byte[] arrived = gateway.Read(256, TimeSpan.FromMilliseconds(100));
    Console.WriteLine($"gateway      {arrived.Length} bytes in one read");
    using var decoder = new CobsDecoder();
    byte[][] payloads = decoder.Feed(arrived);
    foreach (byte[] payload in payloads)
    {
        ushort sequence = BinaryPrimitives.ReadUInt16BigEndian(payload);
        Console.WriteLine($"reading {sequence}    {Encoding.UTF8.GetString(payload, 2, payload.Length - 2)}");
    }

    // What one frame costs on the wire at this speed, start and stop bits included.
    int frameLength = sent / texts.Length;
    Console.WriteLine(Invariant(
        $"on the wire  {settings.TransferMicros(frameLength) / 1_000.0:F2} ms for a {frameLength}-byte frame at {settings}"));

    // The node restarts partway through a frame. As it comes back up it sends a lone
    // delimiter, which closes off the half frame, so the gateway drops it rather than
    // gluing it to the next one, and then it sends the reading again.
    byte[] again = Serial.CobsEncode(Reading(4, "wind=12.9"));
    node.Write(again.AsSpan(0, again.Length / 2));
    node.Write([Serial.CobsDelimiter]);
    node.Write(again);
    ulong droppedBefore = decoder.Discarded;
    byte[][] resent = decoder.Feed(gateway.Read(256, TimeSpan.FromMilliseconds(100)));
    ulong dropped = decoder.Discarded - droppedBefore;
    Console.WriteLine(
        $"restart      {dropped} frame cut short and dropped, then {Encoding.UTF8.GetString(resent[0], 2, resent[0].Length - 2)}");

    // SLIP, the older framing, ends a frame with one reserved byte and escapes that byte
    // and its own escape byte inside one. With no reserved bytes in a reading it costs a
    // byte less than COBS; a payload full of them costs up to twice its length under
    // SLIP, and never more than one byte in 254 over under COBS.
    byte[] first = Reading(1, texts[0]);
    int slipLength = Serial.SlipEncode(first).Length;
    int cobsLength = Serial.CobsEncode(first).Length;
    Console.WriteLine(
        $"framing      {first.Length} payload bytes: {slipLength} under SLIP, {cobsLength} under COBS");

    // The node goes quiet. A read waits for the first byte up to its timeout; on a port
    // with nothing plugged in it returns at once and counts the wait instead of sleeping
    // through it, so a test of a silent node takes no time.
    ulong waitedBefore = gateway.WaitedMicros;
    byte[] quiet = gateway.Read(256, TimeSpan.FromMilliseconds(500));
    ulong waited = (gateway.WaitedMicros - waitedBefore) / 1_000;
    Console.WriteLine($"silence      {quiet.Length} bytes in {waited} ms, counted and not slept");
```
<!-- end -->

## On a board

The same port on a Raspberry Pi, checked with one jumper wire and nothing else: the program
opens the Pi's own UART, sends five COBS frames out of GPIO14, and reads each one back on
GPIO15.

| Wire | From | To |
| --- | --- | --- |
| one jumper | GPIO14, TX | GPIO15, RX |

By default the UART runs a login shell. The
[Raspberry Pi documentation](https://github.com/raspberrypi/documentation/blob/master/documentation/asciidoc/computers/configuration/interfaces.adoc)
turns that off for hardware projects: in `sudo raspi-config`, choose Interface Options, then
Serial Port, answer No to a login shell over serial and Yes to the serial port hardware, and
reboot. `/dev/serial0` is the primary UART, which is GPIO14 and GPIO15 on every model but the
Pi 5, where it is the three-pin debug header instead; there, `dtoverlay -h uart0-pi5` lists the
pins a UART can take. Every UART on the header is 3.3 V, and a 5 V device on it does damage; a
5 V device needs a level shifter or a 3.3 V USB adapter.

With the jumper in, each frame comes back in a little over a millisecond. The RP2040 page's
program writes its readings as text lines at the same 115200 8N1 from its GP0; with the Pico's
GP0 wired to GPIO15 and the two grounds joined, the same port reads them.

### Rust

<!-- snippet: examples/boards/raspberry-pi/src/bin/uart.rs#example -->
From [`examples/boards/raspberry-pi/src/bin/uart.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/bin/uart.rs):

```rust
use pamoja_hal::port::{SerialPort, Settings};
use pamoja_serial::cobs::{self, CobsDecoder};

/// The primary UART, on GPIO14 and GPIO15 on every model but the Raspberry Pi 5.
const PORT: &str = "/dev/serial0";

fn main() -> Result<(), Box<dyn Error>> {
    let port = SerialPort::open(PORT, Settings::new(115_200))?;
    let mut decoder: CobsDecoder<64> = CobsDecoder::new();
    let mut buffer = [0u8; 64];
    let mut frame = [0u8; cobs::max_encoded_len(32)];

    for sequence in 1u16..=5 {
        let mut payload = sequence.to_be_bytes().to_vec();
        payload.extend_from_slice(b"ping");
        let framed = cobs::encode(&payload, &mut frame)?;
        let started = Instant::now();
        port.write(&frame[..framed])?;

        // The frame comes back on RX as it goes out on TX. Read until the decoder has it, or
        // until the line has been quiet for 100 ms.
        let mut echoed = None;
        while echoed.is_none() {
            let got = port.read(&mut buffer, Duration::from_millis(100))?;
            if got == 0 {
                break;
            }
            for &byte in &buffer[..got] {
                if let Ok(Some(back)) = decoder.push(byte) {
                    echoed = Some(back.to_vec());
                }
            }
        }
        match echoed {
            Some(back) if back == payload => println!(
                "frame {sequence}  {framed} bytes back in {:.2} ms",
                started.elapsed().as_secs_f64() * 1e3
            ),
            Some(_) => {
                println!("frame {sequence}  came back changed: check the speed and the wiring")
            }
            None => println!(
                "frame {sequence}  nothing came back: check the jumper, and that the console is off"
            ),
        }
        thread::sleep(Duration::from_millis(500));
    }
    Ok(())
}
```
<!-- end -->

```sh
cd examples/boards/raspberry-pi
cargo run --release --bin uart
```

### TypeScript

<!-- snippet: bindings/node/boards/raspberry-pi/uart.ts#example -->
From [`bindings/node/boards/raspberry-pi/uart.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/boards/raspberry-pi/uart.ts):

```typescript
import { setTimeout as sleep } from 'node:timers/promises'
import { SerialPort } from '@pamoja/hal'
import { CobsDecoder, cobs } from '@pamoja/serial'

// The primary UART, on GPIO14 and GPIO15 on every model but the Raspberry Pi 5.
const PORT = '/dev/serial0'

async function main(): Promise<void> {
  const port = SerialPort.open(PORT, { baud: 115200 })
  const decoder = new CobsDecoder()

  for (let sequence = 1; sequence <= 5; sequence += 1) {
    const number = Buffer.alloc(2)
    number.writeUInt16BE(sequence)
    const payload = Buffer.concat([number, Buffer.from('ping')])
    const frame = cobs.encode(payload)
    const started = performance.now()
    await port.write(frame)

    // The frame comes back on RX as it goes out on TX. Read until the decoder has it, or
    // until the line has been quiet for 100 ms.
    let echoed: Buffer | undefined
    while (echoed === undefined) {
      const arrived = await port.read(64, 100)
      if (arrived.length === 0) {
        break
      }
      echoed = decoder.feed(arrived)[0]
    }
    if (echoed === undefined) {
      console.log(`frame ${sequence}  nothing came back: check the jumper, and that the console is off`)
    } else if (echoed.equals(payload)) {
      const millis = performance.now() - started
      console.log(`frame ${sequence}  ${frame.length} bytes back in ${millis.toFixed(2)} ms`)
    } else {
      console.log(`frame ${sequence}  came back changed: check the speed and the wiring`)
    }
    await sleep(500)
  }
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
```
<!-- end -->

```sh
npm --prefix bindings/node run boards
node bindings/node/build/boards/raspberry-pi/uart.js
```

### Python

<!-- snippet: bindings/python/boards/raspberry_pi/uart.py#example -->
From [`bindings/python/boards/raspberry_pi/uart.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/boards/raspberry_pi/uart.py):

```python
import time

from pamoja.hal import SerialPort, SerialSettings
from pamoja.serial import CobsDecoder, cobs

# The primary UART, on GPIO14 and GPIO15 on every model but the Raspberry Pi 5.
PORT = "/dev/serial0"


def main() -> None:
    port = SerialPort.open(PORT, SerialSettings(115_200))
    decoder = CobsDecoder()

    for sequence in range(1, 6):
        payload = sequence.to_bytes(2, "big") + b"ping"
        frame = cobs.encode(payload)
        started = time.perf_counter()
        port.write(frame)

        # The frame comes back on RX as it goes out on TX. Read until the decoder has it, or
        # until the line has been quiet for 100 ms.
        echoed = None
        while echoed is None:
            arrived = port.read(64, timeout=0.1)
            if not arrived:
                break
            payloads = decoder.feed(arrived)
            if payloads:
                echoed = payloads[0]
        if echoed is None:
            print(f"frame {sequence}  nothing came back: check the jumper, and that the console is off")
        elif echoed == payload:
            millis = (time.perf_counter() - started) * 1_000
            print(f"frame {sequence}  {len(frame)} bytes back in {millis:.2f} ms")
        else:
            print(f"frame {sequence}  came back changed: check the speed and the wiring")
        time.sleep(0.5)


if __name__ == "__main__":
    main()
```
<!-- end -->

```sh
python bindings/python/boards/raspberry_pi/uart.py
```

### C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Uart.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Uart.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Uart.cs):

```csharp
using System.Buffers.Binary;
using System.Diagnostics;
using System.Globalization;

using Pamoja.Hal;
using Pamoja.Serial;

namespace Boards.RaspberryPi;

/// <summary>
/// A UART self-test: the Pi's own serial port with TX jumpered to RX, sending COBS frames and
/// reading each one straight back. Turn the serial port hardware on and the serial console off
/// (raspi-config, Interface Options, Serial Port), reboot, and put one jumper between GPIO14 and
/// GPIO15.
/// </summary>
public static class Uart
{
    // The primary UART, on GPIO14 and GPIO15 on every model but the Raspberry Pi 5.
    private const string Port = "/dev/serial0";

    /// <summary>Sends five frames and reports each one's return.</summary>
    public static void Run()
    {
        using SerialPort port = SerialPort.Open(Port, new SerialSettings(115_200));
        using var decoder = new CobsDecoder();

        for (ushort sequence = 1; sequence <= 5; sequence++)
        {
            byte[] payload = new byte[2 + "ping"u8.Length];
            BinaryPrimitives.WriteUInt16BigEndian(payload, sequence);
            "ping"u8.CopyTo(payload.AsSpan(2));
            byte[] frame = Serial.CobsEncode(payload);
            var started = Stopwatch.StartNew();
            port.Write(frame);

            // The frame comes back on RX as it goes out on TX. Read until the decoder has it, or
            // until the line has been quiet for 100 ms.
            byte[]? echoed = null;
            while (echoed is null)
            {
                byte[] arrived = port.Read(64, TimeSpan.FromMilliseconds(100));
                if (arrived.Length == 0)
                {
                    break;
                }

                echoed = decoder.Feed(arrived).FirstOrDefault();
            }

            if (echoed is null)
            {
                Console.WriteLine($"frame {sequence}  nothing came back: check the jumper, and that the console is off");
            }
            else if (echoed.SequenceEqual(payload))
            {
                Console.WriteLine(string.Create(
                    CultureInfo.InvariantCulture,
                    $"frame {sequence}  {frame.Length} bytes back in {started.Elapsed.TotalMilliseconds:F2} ms"));
            }
            else
            {
                Console.WriteLine($"frame {sequence}  came back changed: check the speed and the wiring");
            }

            Thread.Sleep(TimeSpan.FromMilliseconds(500));
        }
    }
}
```
<!-- end -->

```sh
dotnet run --project bindings/dotnet/samples/Pamoja.Boards -- raspberry-pi/uart
```

## Values at a glance

**The two framings side by side:**

| | SLIP | COBS |
| --- | --- | --- |
| Defined by | RFC 1055 | Cheshire and Baker, 1999 |
| Ends a frame with | `END`, `0xC0` | the delimiter, `0x00` |
| Reserved inside a frame | `END` becomes `ESC` `ESC_END`, `0xDB 0xDC`; `ESC` becomes `ESC` `ESC_ESC`, `0xDB 0xDD` | no zero appears; each run of up to 254 bytes is led by its length |
| Size for an n-byte payload | n + 1 to 2n + 1 | n + 2 to n + n / 254 + 2 |
| Decoder | `SlipDecoder`, which skips empty frames | `CobsDecoder` |

**What the framing costs**, computed by each `encode`:

| Payload | Bytes | Under SLIP | Under COBS |
| --- | --- | --- | --- |
| the example's reading | 11 | 12 | 13 |
| 64 zero bytes | 64 | 65 | 66 |
| 64 bytes of SLIP's `END` | 64 | 129 | 66 |
| 1000 bytes with no zero and no `END` | 1000 | 1001 | 1005 |

**Line formats and their time on the wire.** A character is a start bit, eight data bits, a
parity bit if there is one, and the stop bits. Both ends have to agree on the speed and all
three, or every byte arrives wrong:

| Format | Bits a character | One character | 8 bytes | 256 bytes |
| --- | --- | --- | --- | --- |
| 9600 8N1 | 10 | 1041.67 µs | 8.33 ms | 266.67 ms |
| 9600 8E1 | 11 | 1145.83 µs | 9.17 ms | 293.33 ms |
| 9600 8N2 | 11 | 1145.83 µs | 9.17 ms | 293.33 ms |
| 19200 8E1 | 11 | 572.92 µs | 4.58 ms | 146.67 ms |
| 115200 8N1 | 10 | 86.81 µs | 0.70 ms | 22.22 ms |
| 115200 8E1 | 11 | 95.49 µs | 0.76 ms | 24.44 ms |

The
[Modbus over Serial Line specification](https://www.modbus.org/file/secure/modbusoverserial.pdf)
requires 9600 and 19200 baud and makes 19200 with even parity the default, in section 3.2 and
its implementation classes; a device with no parity sends two stop bits instead, section 2.5.1.
The Raspberry Pi's serial console runs at 115200.

**What is on the other end of a port:**

| Kind | Made with | What a write does | What a read gets |
| --- | --- | --- | --- |
| Device | `open(path, settings)`, Linux only | leaves through the UART, and returns once sent | what the line delivered, waiting up to the timeout |
| Looped | `looped(settings)` | queues the bytes to come straight back | what was written, at once |
| Paired | `pair(settings)` | queues the bytes for the other end | what the other end wrote, at once |
| Simulated | `simulated(settings, device)`, Rust | hands the bytes to the device | the device's answer, at once |
| Scripted | `scripted(settings, steps)` | has to match the next step, or fails | the replies the script has reached, at once |

On every kind but the device, a read that finds nothing returns at once and adds its timeout to
`waited_micros`.

**Settings in each language:**

### Rust

| Setting | How | Default |
| --- | --- | --- |
| speed | `Settings::new(baud)` | none |
| parity | `.with_parity(Parity::Even)` | `Parity::None` |
| stop bits | `.with_stop_bits(StopBits::Two)` | `StopBits::One` |

### TypeScript

| Setting | Field | Default |
| --- | --- | --- |
| speed | `baud` | none |
| parity | `parity`, from `Parity` | `Parity.None` |
| stop bits | `stopBits`, 1 or 2 | 1 |

### Python

| Setting | Argument | Default |
| --- | --- | --- |
| speed | `baud` | none |
| parity | `parity`, from `Parity` | `Parity.NONE` |
| stop bits | `stop_bits`, 1 or 2 | 1 |

### C#

| Setting | Parameter | Default |
| --- | --- | --- |
| speed | `Baud` | none |
| parity | `Parity` | `Parity.None` |
| stop bits | `StopBits`, 1 or 2 | 1 |

<!-- languages end -->

**The same calls in each language:**

| To | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| open a real port | `SerialPort::open(path, settings)` | `SerialPort.open(path, settings)` | `SerialPort.open(path, settings)` | `SerialPort.Open(path, settings)` |
| two ends of a cable | `SerialPort::pair(settings)` | `SerialPort.pair(settings)` | `SerialPort.pair(settings)` | `SerialPort.Pair(settings)` |
| write | `write(&bytes)` | `await write(bytes)` | `write(data)` | `Write(bytes)` |
| read | `read(&mut buffer, timeout)` | `await read(max, timeoutMs)` | `read(size, timeout=seconds)` | `Read(max, timeout)` |
| drop stale input | `discard_input()` | `discardInput()` | `discard_input()` | `DiscardInput()` |
| time on the wire | `settings.transfer_micros(n)` | `SerialPort.transferMicros(settings, n)` | `settings.transfer_micros(n)` | `settings.TransferMicros(n)` |
| frame with COBS | `cobs::encode(&payload, &mut out)` | `cobs.encode(payload)` | `cobs.encode(payload)` | `Serial.CobsEncode(payload)` |
| split a stream | `decoder.push(byte)` | `decoder.feed(chunk)` | `decoder.feed(chunk)` | `decoder.Feed(chunk)` |
| count dropped frames | the `Err` from `push` | `decoder.discarded` | `decoder.discarded` | `decoder.Discarded` |

**A Raspberry Pi's UARTs**, from the Raspberry Pi documentation. `/dev/serial0` points at the
primary UART, which is one of two kinds, and the kind decides what the line can do:

| Model | `/dev/serial0` is | Parity | Its speed |
| --- | --- | --- | --- |
| Pi 1, Pi 2, Zero | the PL011, `/dev/ttyAMA0` | yes | fixed |
| Pi 3, Pi 4, Zero W, Zero 2 W | the mini UART, `/dev/ttyS0` | no | follows the core clock, which `enable_uart=1` fixes |
| Pi 5 | the debug header's PL011, `/dev/ttyAMA10` | yes | fixed |

On the models with the mini UART, `dtoverlay=disable-bt` in `config.txt` and
`sudo systemctl disable hciuart` make the PL011 the primary UART on GPIO14 and GPIO15 at the
cost of Bluetooth, which is what a line with parity, such as Modbus RTU, needs.

## When it goes wrong

What the port and the decoders refuse, and what they say:

| What happened | The message | What to check |
| --- | --- | --- |
| the platform has no serial device here | `a serial device is opened through the kernel's terminal interface, which only Linux has here` | open a real port on a Linux board, and a paired or looped one anywhere |
| the device file is missing | `/dev/serial0: No such file or directory (os error 2)` | the serial port hardware setting, and the device name for this model |
| the process may not use the port | `/dev/serial0: Permission denied (os error 13)` | `ls -lL /dev/serial0` names the group that owns it; add the account to that group |
| the speed is not a standard rate | `/dev/serial0: not a standard serial rate` | a rate from 1200 to 921600 |
| a script expected another write | `the script expected 3f to be written, not 21` | the bytes the driver sends |
| a frame ended before its data did | `TruncatedFrame`, counted on `discarded` | the node restarting, or noise on the line |
| a frame was longer than the decoder holds | `BufferTooSmall`, counted on `discarded` | the decoder's size against the largest payload |

How each language hands those over:

| Language | A port that fails | A frame the decoder drops |
| --- | --- | --- |
| Rust | `Err(OpenError)` from `open`, `Err(PortError)` from `write` and `read` | `Err(SerialError)` from `push` |
| TypeScript | a thrown `Error` from `open`, a rejected promise from `write` and `read` | counted on `discarded` |
| Python | `PamojaError` | counted on `discarded` |
| C# | `PlatformNotSupportedException` from `Open` off Linux, `PamojaException` otherwise | counted on `Discarded` |

The mistakes that cost an afternoon:

- **Nothing comes back, or the Pi's own boot messages do.** The serial console still owns the
  port. Turn the login shell off and the serial port hardware on, and reboot.
- **Every byte arrives, and every byte is wrong.** The two ends disagree on the format. Check the
  speed, the parity, and the stop bits on both sides; the settings print the way a manual writes
  them, `9600 8E1`, to make that easy.
- **The mini UART garbles at a speed that used to work.** Its baud rate is derived from the core
  clock, which changes with load unless `enable_uart=1` holds it.
- **Parity is set, and ignored.** On a Pi 3, a Pi 4, or a Zero W the primary UART is the mini
  UART, which has no parity bit. Make the PL011 primary with `dtoverlay=disable-bt`, or use a
  USB adapter.
- **A 5 V device on the header.** Every UART on the Pi is 3.3 V, and 5 V damages it. Use a level
  shifter, or a USB adapter at the device's voltage.
- **Half a message arrives, or two messages in one read.** That is how a UART works, not a
  fault. Keep one decoder for the life of the port and feed it every read, so a frame that
  arrives across two reads comes out whole.
- **Two frames glued into one after a node restarts.** A node that restarts mid-frame leaves a
  half frame on the line with no delimiter after it. Have the node send a lone delimiter as it
  starts, as the example's does, so the half frame is closed off and dropped.
- **A stale reply taken for a new one.** A client that asks and waits should call
  `discard_input` first, so bytes left over from before the question are not taken for its
  answer.
- **A Pi 5 program that reads nothing on GPIO14 and GPIO15.** `/dev/serial0` is its debug header
  there; put a UART on those pins with its overlay and open that device.

## Where next

<!-- table: next serial -->
- [Modbus RTU](modbus.md): Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices.
- [MAVLink](mavlink.md): MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols.
- [Buses](hal.md): The embedded-hal traits every driver takes, a bit-banged 1-Wire bus, simulated parts and scripted buses that stand in for hardware, the Linux backends over i2c-dev, spidev, and the GPIO character device, one I2C bus and one serial port a program and its drivers share, and delays that sleep or only count.
- Beside it: [Buses and links](../buses.md).
- Also in Field I/O: [CAN and J1939](can.md), [I2C, SPI, and GPIO](gpio.md).
<!-- end -->

## Reference

<!-- table: reference serial -->
- Rust: [`pamoja-serial`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-serial)
- TypeScript: [`@pamoja/serial`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-serial)
- Python: [`pamoja.serial`](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-serial)
- C#: [`Pamoja.Serial`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-serial)
<!-- end -->
