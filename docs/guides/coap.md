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

It reports a reading non-confirmably to an address with nothing listening, then
sends a command confirmably to the same address with the retransmission budget cut
to a few milliseconds, and checks what comes back.

It proves:

- Connecting a CoAP endpoint binds a local socket and nothing else: it reports
  itself connected with no peer anywhere.
- A non-confirmable send succeeds without an acknowledgement, which is the mode
  for a reading whose loss costs nothing.
- A confirmable send that is never acknowledged raises rather than passing
  silently, so a command is not assumed to have landed.
- Disconnecting releases the socket and the endpoint reports itself closed.

## Rust

<!-- snippet: examples/tests/guides/coap.rs#example -->
From [`examples/tests/guides/coap.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/coap.rs):

```rust
use std::time::Duration;

use pamoja_coap::{CoapConfig, CoapTransport, Reliability};
use pamoja_core::Transport;

// CoAP runs over UDP and opens no session, so connecting only binds a local socket.
// Nothing is listening on the far side here, and nothing needs to be.
let mut reporter = CoapTransport::new(
    CoapConfig::new("127.0.0.1", 5683).reliability(Reliability::NonConfirmable),
);
assert!(!reporter.is_connected());
reporter.connect().await.unwrap();
assert!(reporter.is_connected());

// Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which
// is what a battery-powered node sends when one missed reading costs nothing.
reporter
    .send("sensors/1/temperature", b"21.5")
    .await
    .unwrap();

// Confirmable delivery retransmits until an ACK arrives. RFC 7252 fixes the defaults
// at a two-second wait and four retransmissions; both are cut short here.
let mut commander = CoapTransport::new(
    CoapConfig::new("127.0.0.1", 5683)
        .reliability(Reliability::Confirmable)
        .ack_timeout(Duration::from_millis(20))
        .max_retransmits(1),
);
commander.connect().await.unwrap();
assert!(commander.send("actuators/valve", b"open").await.is_err());

reporter.disconnect().await.unwrap();
assert!(!reporter.is_connected());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/coap.ts#example -->
From [`bindings/node/guides/coap.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/coap.ts):

```typescript
import assert from 'node:assert/strict'

import { CoapClient, Reliability } from '@pamoja/coap'

async function main() {
  // CoAP runs over UDP and opens no session, so connecting only binds a local socket.
  // Nothing is listening on the far side here, and nothing needs to be.
  const reporter = new CoapClient({
    host: '127.0.0.1',
    port: 5683,
    reliability: Reliability.NonConfirmable,
  })
  assert.equal(await reporter.isConnected(), false)
  await reporter.connect()
  assert.equal(await reporter.isConnected(), true)

  // Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which is
  // what a battery-powered node sends when one missed reading costs nothing.
  await reporter.send('sensors/1/temperature', Buffer.from('21.5'))

  // Confirmable delivery retransmits until an ACK arrives. RFC 7252 fixes the defaults at a
  // two-second wait and four retransmissions; both are cut short here.
  const commander = new CoapClient({
    host: '127.0.0.1',
    port: 5683,
    reliability: Reliability.Confirmable,
    ackTimeoutMs: 20,
    maxRetransmits: 1,
  })
  await commander.connect()
  await assert.rejects(() => commander.send('actuators/valve', Buffer.from('open')))

  await reporter.disconnect()
  assert.equal(await reporter.isConnected(), false)
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
    # Nothing is listening on the far side here, and nothing needs to be.
    reporter = CoapClient(
        host="127.0.0.1", port=5683, reliability=Reliability.NON_CONFIRMABLE
    )
    assert not await reporter.is_connected()
    await reporter.connect()
    assert await reporter.is_connected()

    # Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which
    # is what a battery-powered node sends when one missed reading costs nothing.
    await reporter.send("sensors/1/temperature", b"21.5")

    # Confirmable delivery retransmits until an ACK arrives. RFC 7252 fixes the defaults
    # at a two-second wait and four retransmissions; both are cut short here.
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
    except PamojaError:
        pass
    else:
        raise AssertionError("an unacknowledged command should be reported, not dropped")

    await reporter.disconnect()
    assert not await reporter.is_connected()


asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs):

```csharp
// CoAP runs over UDP and opens no session, so connecting only binds a local
// socket. Nothing is listening on the far side here, and nothing needs to be.
using var reporter = new CoapClient(new CoapClientOptions
{
    Host = "127.0.0.1",
    Port = 5683,
    Reliability = Reliability.NonConfirmable,
});
Expect(!await reporter.IsConnectedAsync(), "a fresh endpoint holds no socket");
await reporter.ConnectAsync();
Expect(await reporter.IsConnectedAsync(), "connecting binds the local socket");

// Non-confirmable delivery is at most once: the datagram leaves unacknowledged,
// which is what a battery-powered node sends when one missed reading costs
// nothing.
await reporter.SendAsync("sensors/1/temperature", "21.5"u8.ToArray());

// Confirmable delivery retransmits until an ACK arrives. RFC 7252 fixes the
// defaults at a two-second wait and four retransmissions; both are cut short here.
using var commander = new CoapClient(new CoapClientOptions
{
    Host = "127.0.0.1",
    Port = 5683,
    Reliability = Reliability.Confirmable,
    AckTimeoutMs = 20,
    MaxRetransmits = 1,
});
await commander.ConnectAsync();

bool unacknowledged = false;
try
{
    await commander.SendAsync("actuators/valve", "open"u8.ToArray());
}
catch (PamojaException)
{
    unacknowledged = true;
}

Expect(unacknowledged, "an unacknowledged command is reported, not dropped");

await reporter.DisconnectAsync();
Expect(!await reporter.IsConnectedAsync(), "disconnecting releases the socket");
```
<!-- end -->

## Reference

<!-- table: reference coap -->
- Rust: [`pamoja-coap`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_coap/index.html)
- TypeScript: [`@pamoja/coap`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html)
- Python: [`pamoja.coap`](https://pamoja.molex.cloud/docs/reference/python/pamoja/coap.html)
- C#: [`Pamoja.Coap`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Coap.html)
<!-- end -->
