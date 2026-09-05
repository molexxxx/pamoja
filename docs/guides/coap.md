# CoAP

CoAP is the protocol for the constrained end of a network: request and response
over UDP, a four-byte header, and no connection to hold open. That suits a node
that wakes, reports, and sleeps, and a link where the cost of a TCP handshake is
measured against the battery.

Because there is no session, reliability is per message rather than per
connection. A non-confirmable message is sent once and forgotten. A confirmable
message waits for an acknowledgement and retransmits until one arrives or the
attempts run out, and RFC 7252 fixes the defaults for that: two seconds for the
first wait, doubling, and four retransmissions. A node picks the mode per client
and gets the guarantee it paid for.

pamoja's client is that behind the same transport surface as every other link, so
what publishes over MQTT publishes over CoAP without knowing the difference. None
of it needs a server: binding a socket is a local act, and an unacknowledged
confirmable request is a defined outcome the example below checks.

## What the example does

It stands up two endpoints pointed at `127.0.0.1:5683`, the plaintext CoAP
port, one non-confirmable and one confirmable. The first reports a temperature
reading, the second sends a valve command, and each prints how its send ends.
Nothing is listening on that port, so the reading leaves unacknowledged and the
command retransmits until its attempts run out.

Reliability is a property of the client, not an argument to the send, so a node
that reports readings and also takes commands holds an endpoint for each
guarantee. Neither send writes a CoAP header: the transport allocates the
message id and the token, and splits `sensors/1/temperature` into one path
option per segment. The 20-millisecond wait and the single retransmission are
overrides. A fresh config starts with the RFC defaults above, which would spend
over a minute retransmitting before reporting the command unacknowledged.

It proves:

- Connecting a CoAP endpoint binds a local socket and nothing else: it reports
  itself connected with nothing on the far side.
- A non-confirmable send succeeds without an acknowledgement, which is the mode
  for a reading whose loss costs nothing.
- A confirmable send to that same address fails once its retransmissions run
  out. Only the mode differs between the two sends, so the guarantee decides the
  outcome rather than the destination.
- The failure leaves the endpoint usable: the command sends again and fails
  again, rather than wedging the client.
- Disconnecting releases the socket and the endpoint reports itself closed.

## Rust

<!-- snippet: examples/tests/guides/coap.rs#example -->
From [`examples/tests/guides/coap.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/coap.rs):

```rust
use std::time::Duration;

use pamoja_coap::{CoapConfig, CoapTransport, Reliability};
use pamoja_core::Transport;

// CoAP runs over UDP and opens no session, so connecting only binds a local socket.
// Nothing is listening on the far side here, and for a non-confirmable send nothing
// needs to be.
let mut reporter = CoapTransport::new(
    CoapConfig::new("127.0.0.1", 5683).reliability(Reliability::NonConfirmable),
);
reporter.connect().await.expect("a local socket");
println!("reporter  connected: {}", reporter.is_connected());

// Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which
// is what a battery-powered node sends when one missed reading costs nothing.
reporter
    .send("sensors/1/temperature", b"21.5")
    .await
    .expect("the datagram leaves");
println!("reporter  sent 21.5 and did not wait for an answer");

// A command is different: it has to arrive. Confirmable delivery retransmits until an
// acknowledgement comes back. RFC 7252 fixes the defaults at a two-second wait and
// four retransmissions; both are cut short here so the guide does not sit waiting.
let mut commander = CoapTransport::new(
    CoapConfig::new("127.0.0.1", 5683)
        .reliability(Reliability::Confirmable)
        .ack_timeout(Duration::from_millis(20))
        .max_retransmits(1),
);
commander.connect().await.expect("a local socket");
match commander.send("actuators/valve", b"open").await {
    Ok(()) => println!("commander the valve acknowledged the command"),
    Err(error) => println!("commander gave up unacknowledged: {error}"),
}

reporter.disconnect().await.expect("a clean close");
println!("reporter  disconnected: {}", !reporter.is_connected());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/coap.ts#example -->
From [`bindings/node/guides/coap.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/coap.ts):

```typescript
import { CoapClient, Reliability } from '@pamoja/coap'

async function main(): Promise<void> {
  // CoAP runs over UDP and opens no session, so connecting only binds a local socket.
  // Nothing is listening on the far side here, and for a non-confirmable send nothing
  // needs to be.
  const reporter = new CoapClient({
    host: '127.0.0.1',
    port: 5683,
    reliability: Reliability.NonConfirmable,
  })
  await reporter.connect()
  console.log(`reporter  connected: ${await reporter.isConnected()}`)

  // Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which is
  // what a battery-powered node sends when one missed reading costs nothing.
  await reporter.send('sensors/1/temperature', Buffer.from('21.5'))
  console.log('reporter  sent 21.5 and did not wait for an answer')

  // A command is different: it has to arrive. Confirmable delivery retransmits until an
  // acknowledgement comes back. RFC 7252 fixes the defaults at a two-second wait and four
  // retransmissions; both are cut short here so the guide does not sit waiting.
  const commander = new CoapClient({
    host: '127.0.0.1',
    port: 5683,
    reliability: Reliability.Confirmable,
    ackTimeoutMs: 20,
    maxRetransmits: 1,
  })
  await commander.connect()
  try {
    await commander.send('actuators/valve', Buffer.from('open'))
    console.log('commander the valve acknowledged the command')
  } catch (error) {
    console.log(`commander gave up unacknowledged: ${(error as Error).message}`)
  }

  await reporter.disconnect()
  console.log(`reporter  disconnected: ${!(await reporter.isConnected())}`)
}

main()
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/coap.py#example -->
From [`bindings/python/guides/coap.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/coap.py):

```python
import asyncio

from pamoja.coap import CoapClient, Reliability
from pamoja.core import PamojaError


async def main() -> None:
    # CoAP runs over UDP and opens no session, so connecting only binds a local socket.
    # Nothing is listening on the far side here, and for a non-confirmable send nothing
    # needs to be.
    reporter = CoapClient(
        host="127.0.0.1", port=5683, reliability=Reliability.NON_CONFIRMABLE
    )
    await reporter.connect()
    print(f"reporter  connected: {await reporter.is_connected()}")

    # Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which
    # is what a battery-powered node sends when one missed reading costs nothing.
    await reporter.send("sensors/1/temperature", b"21.5")
    print("reporter  sent 21.5 and did not wait for an answer")

    # A command is different: it has to arrive. Confirmable delivery retransmits until an
    # acknowledgement comes back. RFC 7252 fixes the defaults at a two-second wait and
    # four retransmissions; both are cut short here so the guide does not sit waiting.
    commander = CoapClient(
        host="127.0.0.1",
        port=5683,
        reliability=Reliability.CONFIRMABLE,
        ack_timeout_ms=20,
        max_retransmits=1,
    )
    await commander.connect()
    try:
        await commander.send("actuators/valve", b"open")
        print("commander the valve acknowledged the command")
    except PamojaError as error:
        print(f"commander gave up unacknowledged: {error}")

    await reporter.disconnect()
    print(f"reporter  disconnected: {not await reporter.is_connected()}")


asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs):

```csharp
// CoAP runs over UDP and opens no session, so connecting only binds a local
// socket. Nothing is listening on the far side here, and for a non-confirmable
// send nothing needs to be.
using var reporter = new CoapClient(new CoapClientOptions
{
    Host = "127.0.0.1",
    Port = 5683,
    Reliability = Reliability.NonConfirmable,
});
await reporter.ConnectAsync();
Console.WriteLine($"reporter  connected: {await reporter.IsConnectedAsync()}");

// Non-confirmable delivery is at most once: the datagram leaves unacknowledged,
// which is what a battery-powered node sends when a missed reading costs nothing.
await reporter.SendAsync("sensors/1/temperature", "21.5"u8.ToArray());
Console.WriteLine("reporter  sent 21.5 and did not wait for an answer");

// A command is different: it has to arrive. Confirmable delivery retransmits until
// an acknowledgement comes back. RFC 7252 fixes the defaults at a two-second wait
// and four retransmissions; both are cut short here so the guide does not sit
// waiting.
using var commander = new CoapClient(new CoapClientOptions
{
    Host = "127.0.0.1",
    Port = 5683,
    Reliability = Reliability.Confirmable,
    AckTimeoutMs = 20,
    MaxRetransmits = 1,
});
await commander.ConnectAsync();
try
{
    await commander.SendAsync("actuators/valve", "open"u8.ToArray());
    Console.WriteLine("commander the valve acknowledged the command");
}
catch (PamojaException error)
{
    Console.WriteLine($"commander gave up unacknowledged: {error.Message}");
}

await reporter.DisconnectAsync();
Console.WriteLine($"reporter  disconnected: {!await reporter.IsConnectedAsync()}");
```
<!-- end -->

## Reference

<!-- table: reference coap -->
- Rust: [`pamoja-coap`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_coap/index.html)
- TypeScript: [`@pamoja/coap`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html)
- Python: [`pamoja.coap`](https://pamoja.molex.cloud/docs/reference/python/pamoja/coap.html)
- C#: [`Pamoja.Coap`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Coap.html)
<!-- end -->
