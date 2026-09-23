# CoAP

CoAP is the protocol for the constrained end of a network: request and response
over UDP, a four-byte header, and no connection to hold open. That suits a node
that wakes, reports, and sleeps, and a link where the cost of a TCP handshake is
measured against the battery.

Because there is no session, reliability is per message rather than per
connection. A non-confirmable message is sent once and forgotten. A confirmable
message waits for an acknowledgment and retransmits until one arrives or the
attempts run out, and RFC 7252 fixes the defaults for that: a first wait drawn
at random between two and three seconds, doubling after each try, and four
retransmissions. A node picks the mode per client and gets the guarantee it
paid for.

A node does not only report. It also watches: RFC 7641 lets a client observe a
resource, so the server answers with its state now and pushes every change
after it, which is how a command reaches a node that is only ever the one
asking. pamoja carries both ends behind the same transport surface as every
other link: `CoapTransport` for the node, and `CoapServer` for the gateway it
reports to.

## What the example does

It runs an orchard's irrigation on this machine. The gateway in the shed is a
CoAP server that takes moisture readings from every row and holds the state of
the irrigation valve. Row 7's sensor reports a reading, observes the valve,
and sees it open when the gateway changes it. It then sends a battery reading
to a path the gateway does not take. Row 8 reports without asking for an
acknowledgment, and row 9, pointed at a port where nothing listens, gives up.

The gateway binds port 0, so the system picks a free port and the rows are
pointed at it, and nothing else on the machine is disturbed. Neither side
writes a CoAP header: the transport picks the message ids and the tokens and
splits `orchard/row-7/moisture` into one path option per segment. Row 9 waits
20 milliseconds and retransmits once; with the RFC defaults it would retransmit
for more than a minute before giving up. Waits that short suit only a machine
talking to itself, which the table of settings below explains.

It proves:

- A confirmable reading returns once the gateway has acknowledged it, and the
  gateway's receive hands it over with the path it was sent to.
- An observation starts with the resource's state now, `closed`, and delivers
  the change the gateway makes, `open`, under the path that was observed, since
  a notification carries only its registration's token.
- A path the gateway does not take is answered 4.04 Not Found, and the
  confirmable send fails with that code rather than counting the reading as
  delivered.
- A non-confirmable reading leaves at once with no acknowledgment to wait for,
  and still reaches a gateway that is there.
- A confirmable send to a port where nothing listens fails once its
  retransmissions run out, with how many it made.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example coap" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example coap</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- coap" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- coap</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/coap.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/coap.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- coap" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- coap</code></div>
</div>
<!-- end -->

## Rust

In Rust, `CoapConfig::new(host, port)` holds a client's settings, refined with `bind`,
`reliability`, `ack_timeout`, and `max_retransmits`, and `CoapTransport::new(config)` makes the
client. `CoapServer::new(bind)` makes the server, and its `publisher()` gives a `CoapPublisher`
whose `publish` sets a resource's state from another task while the server waits in `recv`. Both
implement the `Transport` and `Receive` traits. A client's `send` is a PUT and its `subscribe`
registers an observation; a server's `subscribe` names the paths it takes readings on, and its
`send` sets a resource's state. `local_addr()`, `observers(path)`, `is_connected()`, and
`disconnect()` are their own. A failure is a `pamoja_core::Error`: `Transport` for a refusal or
a send that ran out of retransmissions, and `Closed` before `connect`.

<!-- snippet: examples/guides/coap.rs#example -->
From [`examples/guides/coap.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/coap.rs):

```rust
use std::time::Duration;

use pamoja_coap::{CoapConfig, CoapServer, CoapTransport, Reliability};
use pamoja_core::{Receive, Transport};

// The gateway in the orchard's shed. It takes moisture readings from every row, and
// holds the irrigation valve's state for the rows to observe. Port 0 lets the system
// pick a free port, which the rows are pointed at below.
let mut gateway = CoapServer::new("127.0.0.1:0");
gateway.connect().await?;
gateway.subscribe("orchard/+/moisture").await?;
gateway.send_text("orchard/valve", "closed").await?;
let port = gateway.local_addr().expect("bound").port();
println!("gateway   takes moisture readings on orchard/+/moisture");

// A battery-powered sensor in row 7. Its reading is confirmable, so it waits for the
// gateway's acknowledgment and retransmits until one comes back.
let mut row7 = CoapTransport::new(
    CoapConfig::new("127.0.0.1", port).ack_timeout(Duration::from_millis(200)),
);
row7.connect().await?;
row7.send_text("orchard/row-7/moisture", "31").await?;
println!("row-7     reported 31, and the gateway acknowledged it");
let reading = gateway.recv().await?.expect("a reading");
println!("gateway   took {} from {}", reading.text()?, reading.topic);

// Observing the valve registers the row with the gateway, which answers with the
// valve's state now and notifies every change after it, as RFC 7641 describes.
row7.subscribe("orchard/valve").await?;
let current = row7.recv().await?.expect("the current state");
println!(
    "row-7     observes {}, which reads {}",
    current.topic,
    current.text()?
);
gateway.send_text("orchard/valve", "open").await?;
let observers = gateway.observers("orchard/valve");
println!("gateway   opened the valve for {observers} observer");
let change = row7.recv().await?.expect("the change");
println!("row-7     {} now reads {}", change.topic, change.text()?);

// A path the gateway does not take is answered 4.04, and a confirmable send reports
// that rather than counting the reading as delivered.
match row7.send_text("orchard/row-7/battery", "3.1").await {
    Ok(()) => println!("row-7     the battery reading was taken, which should never happen"),
    Err(error) => println!("row-7     battery refused: {error}"),
}

// Row 8 sends non-confirmable: once and unacknowledged, which costs the least radio
// time and suits a reading whose loss costs nothing.
let mut row8 = CoapTransport::new(
    CoapConfig::new("127.0.0.1", port).reliability(Reliability::NonConfirmable),
);
row8.connect().await?;
row8.send_text("orchard/row-8/moisture", "27").await?;
println!("row-8     sent 27 without waiting for an answer");
let unconfirmed = gateway.recv().await?.expect("a reading");
println!(
    "gateway   took {} from {}",
    unconfirmed.text()?,
    unconfirmed.topic
);

// Row 9 is pointed at port 1, where nothing listens. A confirmable send retransmits on
// a doubling wait and then gives up. RFC 7252's defaults would take more than a minute
// to get there, so this one waits 20 ms and retransmits once.
let mut row9 = CoapTransport::new(
    CoapConfig::new("127.0.0.1", 1)
        .ack_timeout(Duration::from_millis(20))
        .max_retransmits(1),
);
row9.connect().await?;
match row9.send_text("orchard/row-9/moisture", "29").await {
    Ok(()) => println!("row-9     an empty port acknowledged it, which should never happen"),
    Err(error) => println!("row-9     gave up unacknowledged: {error}"),
}
```
<!-- end -->

## TypeScript

In TypeScript, `new CoapClient({ host, port, bind?, reliability?, ackTimeoutMs?,
maxRetransmits? })` and `new CoapServer(bind)` come from `@pamoja/coap`, with
`Reliability.Confirmable` and `Reliability.NonConfirmable`. Their calls return promises:
`connect`, `send(path, textOrBytes)`, `subscribe`, `recv(timeoutMs?)`, and `disconnect`, and the
client's `isConnected()`. The server answers `observers(path)`, `localPort`, and `isConnected` at
once, and its `send` runs while a `recv` is waiting. A message is `{ topic, payload, text?,
number? }`. For a ladder, `Transport.coap(options)` in `@pamoja/core` takes the client's options.

<!-- snippet: bindings/node/guides/coap.ts#example -->
From [`bindings/node/guides/coap.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/coap.ts):

```typescript
import { CoapClient, CoapServer, Reliability } from '@pamoja/coap'

async function main() {
  // The gateway in the orchard's shed. It takes moisture readings from every row, and holds
  // the irrigation valve's state for the rows to observe. Port 0 lets the system pick a free
  // port, which the rows are pointed at below.
  const gateway = new CoapServer('127.0.0.1:0')
  await gateway.connect()
  await gateway.subscribe('orchard/+/moisture')
  await gateway.send('orchard/valve', 'closed')
  const port = gateway.localPort!
  console.log('gateway   takes moisture readings on orchard/+/moisture')

  // A battery-powered sensor in row 7. Its reading is confirmable, so it waits for the
  // gateway's acknowledgment and retransmits until one comes back.
  const row7 = new CoapClient({ host: '127.0.0.1', port, ackTimeoutMs: 200 })
  await row7.connect()
  await row7.send('orchard/row-7/moisture', '31')
  console.log('row-7     reported 31, and the gateway acknowledged it')
  const reading = (await gateway.recv())!
  console.log(`gateway   took ${reading.text!} from ${reading.topic}`)

  // Observing the valve registers the row with the gateway, which answers with the valve's
  // state now and notifies every change after it, as RFC 7641 describes.
  await row7.subscribe('orchard/valve')
  const current = (await row7.recv())!
  console.log(`row-7     observes ${current.topic}, which reads ${current.text!}`)
  await gateway.send('orchard/valve', 'open')
  const observers = gateway.observers('orchard/valve')
  console.log(`gateway   opened the valve for ${observers} observer`)
  const change = (await row7.recv())!
  console.log(`row-7     ${change.topic} now reads ${change.text!}`)

  // A path the gateway does not take is answered 4.04, and a confirmable send reports that
  // rather than counting the reading as delivered.
  try {
    await row7.send('orchard/row-7/battery', '3.1')
    console.log('row-7     the battery reading was taken, which should never happen')
  } catch (error) {
    console.log(`row-7     battery refused: ${(error as Error).message}`)
  }

  // Row 8 sends non-confirmable: once and unacknowledged, which costs the least radio time
  // and suits a reading whose loss costs nothing.
  const row8 = new CoapClient({ host: '127.0.0.1', port, reliability: Reliability.NonConfirmable })
  await row8.connect()
  await row8.send('orchard/row-8/moisture', '27')
  console.log('row-8     sent 27 without waiting for an answer')
  const unconfirmed = (await gateway.recv())!
  console.log(`gateway   took ${unconfirmed.text!} from ${unconfirmed.topic}`)

  // Row 9 is pointed at port 1, where nothing listens. A confirmable send retransmits on a
  // doubling wait and then gives up. RFC 7252's defaults would take more than a minute to
  // get there, so this one waits 20 ms and retransmits once.
  const row9 = new CoapClient({ host: '127.0.0.1', port: 1, ackTimeoutMs: 20, maxRetransmits: 1 })
  await row9.connect()
  try {
    await row9.send('orchard/row-9/moisture', '29')
    console.log('row-9     an empty port acknowledged it, which should never happen')
  } catch (error) {
    console.log(`row-9     gave up unacknowledged: ${(error as Error).message}`)
  }

  for (const link of [row7, row8, row9]) {
    await link.disconnect()
  }
  await gateway.disconnect()
  return { reading, current, change, observers, unconfirmed }
}

main()
```
<!-- end -->

## Python

In Python, `CoapClient(host=..., port=...)` and `CoapServer(bind)` come from `pamoja.coap`, the
client with `bind`, `reliability`, `ack_timeout_ms`, and `max_retransmits` optional and
`Reliability.CONFIRMABLE` or `Reliability.NON_CONFIRMABLE`. The calls are coroutines: `connect`,
`send(path, text_or_bytes)`, `subscribe`, `recv`, and `disconnect`, and the client's
`is_connected()`. The server answers `observers(path)`, `local_port`, and `is_connected` at once,
and its `send` runs while a `recv` is waiting. A message has `topic`, `payload` as bytes, `text`,
and `number`, `asyncio.wait_for` puts a limit on a receive, and a failure raises `PamojaError`.
For a ladder, `Transport.coap(...)` in `pamoja.core` takes the client's keywords.

<!-- snippet: bindings/python/guides/coap.py#example -->
From [`bindings/python/guides/coap.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/coap.py):

```python
import asyncio

from pamoja.coap import CoapClient, CoapServer, Reliability
from pamoja.core import PamojaError


async def main():
    # The gateway in the orchard's shed. It takes moisture readings from every row, and
    # holds the irrigation valve's state for the rows to observe. Port 0 lets the system
    # pick a free port, which the rows are pointed at below.
    gateway = CoapServer("127.0.0.1:0")
    await gateway.connect()
    await gateway.subscribe("orchard/+/moisture")
    await gateway.send("orchard/valve", "closed")
    port = gateway.local_port
    print("gateway   takes moisture readings on orchard/+/moisture")

    # A battery-powered sensor in row 7. Its reading is confirmable, so it waits for the
    # gateway's acknowledgment and retransmits until one comes back.
    row7 = CoapClient(host="127.0.0.1", port=port, ack_timeout_ms=200)
    await row7.connect()
    await row7.send("orchard/row-7/moisture", "31")
    print("row-7     reported 31, and the gateway acknowledged it")
    reading = await gateway.recv()
    print(f"gateway   took {reading.text} from {reading.topic}")

    # Observing the valve registers the row with the gateway, which answers with the
    # valve's state now and notifies every change after it, as RFC 7641 describes.
    await row7.subscribe("orchard/valve")
    current = await row7.recv()
    print(f"row-7     observes {current.topic}, which reads {current.text}")
    await gateway.send("orchard/valve", "open")
    observers = gateway.observers("orchard/valve")
    print(f"gateway   opened the valve for {observers} observer")
    change = await row7.recv()
    print(f"row-7     {change.topic} now reads {change.text}")

    # A path the gateway does not take is answered 4.04, and a confirmable send reports
    # that rather than counting the reading as delivered.
    try:
        await row7.send("orchard/row-7/battery", "3.1")
        print("row-7     the battery reading was taken, which should never happen")
    except PamojaError as error:
        print(f"row-7     battery refused: {error}")

    # Row 8 sends non-confirmable: once and unacknowledged, which costs the least radio
    # time and suits a reading whose loss costs nothing.
    row8 = CoapClient(host="127.0.0.1", port=port, reliability=Reliability.NON_CONFIRMABLE)
    await row8.connect()
    await row8.send("orchard/row-8/moisture", "27")
    print("row-8     sent 27 without waiting for an answer")
    unconfirmed = await gateway.recv()
    print(f"gateway   took {unconfirmed.text} from {unconfirmed.topic}")

    # Row 9 is pointed at port 1, where nothing listens. A confirmable send retransmits on
    # a doubling wait and then gives up. RFC 7252's defaults would take more than a minute
    # to get there, so this one waits 20 ms and retransmits once.
    row9 = CoapClient(host="127.0.0.1", port=1, ack_timeout_ms=20, max_retransmits=1)
    await row9.connect()
    try:
        await row9.send("orchard/row-9/moisture", "29")
        print("row-9     an empty port acknowledged it, which should never happen")
    except PamojaError as error:
        print(f"row-9     gave up unacknowledged: {error}")

    for link in (row7, row8, row9):
        await link.disconnect()
    await gateway.disconnect()
    return reading, current, change, observers, unconfirmed


reading, current, change, observers, unconfirmed = asyncio.run(main())
```
<!-- end -->

## C#

In C#, `new CoapClient(new CoapClientOptions { ... })` and `new CoapServer(bind)` come from
`Pamoja.Coap`, both disposable, the options taking `Host` and `Port` with `Bind`, `Reliability`,
`AckTimeoutMs`, and `MaxRetransmits` optional. The calls are `ConnectAsync`,
`SendAsync(path, textOrBytes)`, `SubscribeAsync`, `ReceiveAsync()`, `ReceiveAsync(limit)`, and
`DisconnectAsync`, and the client's `IsConnectedAsync`. The server answers `Observers(path)`,
`LocalPort`, and `IsConnected` at once, and its `SendAsync` runs beside a `ReceiveAsync` that is
waiting, where the client's calls run one at a time. A message is a `TransportMessage`, and a
failure throws `PamojaException`. For a ladder, `CoapTransport.Open(options)` takes the client's
options.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs):

```csharp
// The gateway in the orchard's shed. It takes moisture readings from every row,
// and holds the irrigation valve's state for the rows to observe. Port 0 lets the
// system pick a free port, which the rows are pointed at below.
using var gateway = new CoapServer("127.0.0.1:0");
await gateway.ConnectAsync();
await gateway.SubscribeAsync("orchard/+/moisture");
await gateway.SendAsync("orchard/valve", "closed");
ushort port = gateway.LocalPort!.Value;
Console.WriteLine("gateway   takes moisture readings on orchard/+/moisture");

// A battery-powered sensor in row 7. Its reading is confirmable, so it waits for
// the gateway's acknowledgment and retransmits until one comes back.
using var row7 = new CoapClient(new CoapClientOptions
{
    Host = "127.0.0.1",
    Port = port,
    AckTimeoutMs = 200,
});
await row7.ConnectAsync();
await row7.SendAsync("orchard/row-7/moisture", "31");
Console.WriteLine("row-7     reported 31, and the gateway acknowledged it");
TransportMessage reading = (await gateway.ReceiveAsync())!;
Console.WriteLine($"gateway   took {reading.Text} from {reading.Topic}");

// Observing the valve registers the row with the gateway, which answers with the
// valve's state now and notifies every change after it, as RFC 7641 describes.
await row7.SubscribeAsync("orchard/valve");
TransportMessage current = (await row7.ReceiveAsync())!;
Console.WriteLine($"row-7     observes {current.Topic}, which reads {current.Text}");
await gateway.SendAsync("orchard/valve", "open");
int observers = gateway.Observers("orchard/valve");
Console.WriteLine($"gateway   opened the valve for {observers} observer");
TransportMessage change = (await row7.ReceiveAsync())!;
Console.WriteLine($"row-7     {change.Topic} now reads {change.Text}");

// A path the gateway does not take is answered 4.04, and a confirmable send
// reports that rather than counting the reading as delivered.
try
{
    await row7.SendAsync("orchard/row-7/battery", "3.1");
    Console.WriteLine("row-7     the battery reading was taken, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"row-7     battery refused: {error.Message}");
}

// Row 8 sends non-confirmable: once and unacknowledged, which costs the least
// radio time and suits a reading whose loss costs nothing.
using var row8 = new CoapClient(new CoapClientOptions
{
    Host = "127.0.0.1",
    Port = port,
    Reliability = Reliability.NonConfirmable,
});
await row8.ConnectAsync();
await row8.SendAsync("orchard/row-8/moisture", "27");
Console.WriteLine("row-8     sent 27 without waiting for an answer");
TransportMessage unconfirmed = (await gateway.ReceiveAsync())!;
Console.WriteLine($"gateway   took {unconfirmed.Text} from {unconfirmed.Topic}");

// Row 9 is pointed at port 1, where nothing listens. A confirmable send
// retransmits on a doubling wait and then gives up. RFC 7252's defaults would take
// more than a minute to get there, so this one waits 20 ms and retransmits once.
using var row9 = new CoapClient(new CoapClientOptions
{
    Host = "127.0.0.1",
    Port = 1,
    AckTimeoutMs = 20,
    MaxRetransmits = 1,
});
await row9.ConnectAsync();
try
{
    await row9.SendAsync("orchard/row-9/moisture", "29");
    Console.WriteLine("row-9     an empty port acknowledged it, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"row-9     gave up unacknowledged: {error.Message}");
}
```
<!-- end -->

## Values at a glance

**The client's settings,** each with its default:

| Setting | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| host and port, required | `CoapConfig::new(host, port)` | `host`, `port` | `host`, `port` | `Host`, `Port` |
| local address, any free port | `.bind(address)` | `bind` | `bind` | `Bind` |
| delivery, confirmable | `.reliability(delivery)` | `reliability` | `reliability` | `Reliability` |
| first wait for an acknowledgment, 2 seconds | `.ack_timeout(duration)` | `ackTimeoutMs` | `ack_timeout_ms` | `AckTimeoutMs` |
| retransmissions, 4 | `.max_retransmits(n)` | `maxRetransmits` | `max_retransmits` | `MaxRetransmits` |

The delivery is `Reliability::Confirmable` or `Reliability::NonConfirmable` in Rust,
`Reliability.Confirmable` or `Reliability.NonConfirmable` in TypeScript and C#, and
`Reliability.CONFIRMABLE` or `Reliability.NON_CONFIRMABLE` in Python. The retransmissions can
come down freely, but RFC 7252 section 4.8.1 forbids a first wait shorter than two seconds on a
network without congestion control, however quickly it answers.

**The calls.** A client and a server share the transport calls, and what each call means
depends on the end:

| Call | Client | Server |
| --- | --- | --- |
| connect | binds a socket and points it at the server | binds the port it listens on |
| send | a PUT of a reading to a path | sets a resource's state and notifies its observers |
| subscribe | observes a resource | takes the readings on the paths a filter matches |
| receive | the next notification | the next reading |
| disconnect | closes the socket | closes the port, keeping the resources and the filters |

| Call | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| connect | `connect().await?` | `await x.connect()` | `await x.connect()` | `await x.ConnectAsync()` |
| send | `send(path, &bytes)` | `await x.send(path, textOrBytes)` | `await x.send(path, text_or_bytes)` | `await x.SendAsync(path, textOrBytes)` |
| subscribe | `subscribe(path).await?` | `await x.subscribe(path)` | `await x.subscribe(path)` | `await x.SubscribeAsync(path)` |
| receive | `recv().await?` | `await x.recv(ms?)` | `await x.recv()` | `await x.ReceiveAsync(limit?)` |
| disconnect | `disconnect().await?` | `await x.disconnect()` | `await x.disconnect()` | `await x.DisconnectAsync()` |

The server also counts a resource's observers, `observers(path)` in Rust, TypeScript, and Python
and `Observers(path)` in C#, and names its port: `local_addr()`, `localPort`, `local_port`, and
`LocalPort`. In Rust, its `publisher()` sets states from another task while `recv` waits.

**The two ways to send,** with RFC 7252's section numbers:

| Delivery | On the wire | The sender learns | Suits |
| --- | --- | --- | --- |
| non-confirmable (4.3) | the request, once | nothing: the send returns as the datagram leaves | a reading the next one replaces |
| confirmable (4.2) | the request, retransmitted until acknowledged | taken, refused with a 4.xx or 5.xx code, reset, or given up on | a reading or a command that has to arrive |

**The retransmission schedule** with the defaults. The first wait is drawn at random so that
nodes that lost the same packet do not retransmit together, and each wait after it doubles
(RFC 7252 sections 4.2 and 4.8):

| Transmission | Waits for an acknowledgment |
| --- | --- |
| the request | 2 to 3 seconds |
| retransmission 1 | 4 to 6 seconds |
| retransmission 2 | 8 to 12 seconds |
| retransmission 3 | 16 to 24 seconds |
| retransmission 4 | 32 to 48 seconds |

A confirmable send with the defaults gives up 62 to 93 seconds after it starts; 93 seconds is
the MAX_TRANSMIT_WAIT of section 4.8.2.

**What the server answers,** with the response codes of RFC 7252's table 6:

| Request | Answer |
| --- | --- |
| a PUT or POST to a path a filter takes | 2.04 Changed; the reading goes to receive, becomes the path's state, and notifies its observers |
| a PUT or POST to any other path | 4.04 Not Found |
| a GET of a path with a state | 2.05 Content, carrying the state |
| a GET registering an observation | 2.05 Content with the state and an Observe number, then every change (RFC 7641) |
| a GET cancelling an observation | 2.05 Content, and the observer is removed |
| a GET of a path with no state | 4.04 Not Found |
| a DELETE, or any other method | 4.05 Method Not Allowed |
| an empty confirmable message, a CoAP ping | a Reset, the pong of section 4.3 |
| a confirmable request it answered in the last 247 seconds | the same answer again, and nothing taken twice (section 4.5) |
| a Reset of a notification | the observer is removed (RFC 7641 section 3.6) |

## When it goes wrong

What the client and the server say:

| What happened | The message | What to check |
| --- | --- | --- |
| nothing acknowledged a confirmable send | `transport error: no acknowledgment after 5 transmissions` | the host, the port, and that the server runs; with the defaults this takes up to 93 seconds |
| the server does not take the path | `transport error: the server answered 4.04 Not Found` | the path, and the server's filters |
| observing a resource with no state yet | `transport error: the server answered 4.04 Not Found` | set the state on the server first |
| the server does not allow the method | `transport error: the server answered 4.05 Method Not Allowed` | the server takes PUT, POST, and GET |
| the server reset the request | `transport error: the server reset the request: it arrived, but the server could not process it` | often a server that restarted and lost the exchange |
| the server would not register the observation | `transport error: the server answered orchard/valve without registering an observation, so no changes will follow` | the server does not offer that resource for observing |
| the server's port is taken | `transport error: could not bind 0.0.0.0:5683: ...` | another program holds the port |
| a call before `connect` | `resource is closed` | connect first |

The mistakes that cost an afternoon:

- **A non-confirmable reading vanishes and nothing says so.** Non-confirmable is fire and forget,
  so a reading sent while the gateway is down is gone. Send what has to arrive confirmable.
- **A node takes a minute and a half to report a dead gateway.** With the defaults a confirmable
  send retransmits for 62 to 93 seconds. RFC 7252 section 4.8.1 lets the number of
  retransmissions come down freely, so one retransmission gives up after 6 to 9 seconds, but it
  forbids a shorter first wait on a network without congestion control, and asks that the nodes
  of one network use the same values. The example's short waits are for a machine talking to
  itself.
- **An observer stops hearing changes after the gateway restarts.** A restarted gateway has no
  observers, and nothing tells the node. Register again now and then: the client reuses the path's
  token, which a server takes as renewing the observation (RFC 7641 section 4.1), and the state is
  delivered again.
- **A change never arrives.** Notifications are non-confirmable, so one can be lost, and one that
  arrives after a newer one is dropped by its number (RFC 7641 section 3.4). An observation
  carries a state, where only the newest value matters, not a stream of events that must each
  arrive. Send events as confirmable requests instead.
- **A path matches on one side and not the other.** Leading and trailing slashes are dropped and
  empty levels skipped, so `/orchard//valve/` and `orchard/valve` are the same path, but case and
  spelling are not forgiven.
- **Every request times out on one network and not another.** CoAP is UDP, conventionally on port
  5683. A network that passes only TCP, or blocks that port, looks like a server that never
  answers.

## Where next

<!-- table: next coap -->
- [Codecs](codec.md): CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links.
- [Transport ladder](ladder.md): Cheapest reachable link first, buffering to a store when every link is down.
- [Store and forward](sync.md): Offline-first queues in memory or on disk, bounded or not, the on-disk one surviving power loss, and the drain that forwards them in order when a link returns.
- Also in Transports and testing: [MQTT](mqtt.md), [Loopback](loopback.md), [Event bus](bus.md), [Engine surface](transport.md), [Your own link](link.md), [Simulators](sim.md).
<!-- end -->

## Reference

<!-- table: reference coap -->
- Rust: [`pamoja-coap`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_coap/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-coap)
- TypeScript: [`@pamoja/coap`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-coap)
- Python: [`pamoja.coap`](https://pamoja.molex.cloud/docs/reference/python/pamoja/coap.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-coap)
- C#: [`Pamoja.Coap`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Coap.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-coap)
<!-- end -->
