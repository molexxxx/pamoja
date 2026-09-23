# Your own link

The links pamoja ships are the common ones: MQTT, CoAP, Zenoh, the in-process
broker. The link a node actually has may be none of them. A cellular modem with the
vendor's own SDK, a proprietary radio with its own framing, a serial line to a
gateway, a cloud client that only speaks the cloud's protocol. None of that should
keep the node off a ladder or out of a profile.

The transport contract is small on purpose: connect, send, subscribe, and, for a
link that delivers, receive. In Rust a link implements `Transport` and `Receive`
for its own type. In the three bindings it is an object with those methods, handed
to `Transport.fromHandlers`, `Transport.from_handlers`, or `Transport.FromHandlers`.
Either way the result is a transport like any other: it goes on a ladder as a rung,
sits under a fault injector, or is driven directly, and nothing downstream can tell
it from a shipped one.

A link without a receive is an uplink. A LoRa radio that only reports, a satellite
messenger, a one-way beacon: it still sends in its turn on a ladder, and the ladder
never subscribes or listens on it.

Three rules carry most of the weight. A failure is an error, not a return value, so
a send the link cannot make fails, and the ladder passes the message down to the
next link or into its store. A receive waits until a message arrives, and returning
nothing means the link has ended, not that it is quiet. And a link is connected again
after it ends, so its connect and subscribe run once for every connect, not once for
the life of the object.

## What the example does

It is a moored buoy with two links pamoja has never heard of. A cellular modem,
standing in for a vendor's SDK, sends readings and delivers commands, and a satellite
messenger only sends. Both go on one ladder, the modem first because it is cheaper.
The example plays the vendor's side: it hands the modem a command, takes its signal
away, and drops its session.

It proves:

- A link is written against the contract, not against a pamoja type: nothing in
  either class names a broker or a protocol.
- A reading sent through the ladder reaches the first link with its topic and bytes
  intact.
- A subscription reaches every link that listens and no other. The satellite's
  subscribe would fail, and the ladder never calls it.
- A command the modem hands over comes back through the ladder's own receive.
- A send the modem refuses goes out over the satellite instead, so a link's failure
  is the ladder's cue to move on rather than the node's problem.
- A receive that fails is reported once, in the link's own words, `the modem lost its
  session`, and the link has ended after it. With no other link listening, the ladder
  then reports `resource is closed`.
- Connecting again brings the modem back, the ladder places its filter on it a second
  time, and the modem delivers again.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example link" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example link</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- link" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- link</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/link.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/link.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- link" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- link</code></div>
</div>
<!-- end -->

## Rust

In Rust, a link is a type that implements `Transport`: `connect`, `send`, and `subscribe`, each
an `async fn` on `&mut self` returning `pamoja_core::Result`. A link that delivers also
implements `Receive`, whose `recv` returns `Ok(Some(message))` for each message, `Ok(None)` once
the link has ended, and an error when it fails. `TransportLadder::rung` takes a link that
implements both, and `uplink` one that only sends. A failure is an `Error`: `Error::Transport`
carrying the link's own words for a refusal the ladder should pass over, and `Error::Closed` for
a link that is down, which the ladder sets aside until the next `connect`. After a receive fails,
report the error once and then `Ok(None)` until `connect` runs again, as the modem's `ended` flag
does; that is what the bindings do for a handler that fails, and what a ladder expects. A receive
must also be cancel-safe. A ladder waits on every link at once and drops the waits that lose, so
a message has to leave the link in the same step that takes it, as the modem's leaves its inbox.

<!-- snippet: examples/guides/link.rs#parts -->
From [`examples/guides/link.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/link.rs):

```rust
/// What the modem vendor's SDK holds: what it was given to send, the filters it
/// listens on, what it has to hand over, and whether it has signal.
#[derive(Default)]
struct Sdk {
    sent: Vec<Message>,
    filters: Vec<String>,
    inbox: VecDeque<Result<Message>>,
    no_signal: bool,
}

/// A cellular modem reached through its vendor's SDK. Nothing in it names a broker
/// or a protocol: it needs only the operations the contract asks for, and
/// `Receive` is what makes it a link that delivers.
struct Modem {
    sdk: Arc<Mutex<Sdk>>,
    arrived: Arc<Notify>,
    connected: bool,
    ended: bool,
}

impl Transport for Modem {
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        self.ended = false;
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let mut sdk = self.sdk.lock().expect("the SDK");
        if sdk.no_signal {
            return Err(Error::Transport("no signal".to_owned()));
        }
        sdk.sent.push(Message::new(topic, payload));
        Ok(())
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        self.sdk
            .lock()
            .expect("the SDK")
            .filters
            .push(topic.to_owned());
        Ok(())
    }
}

impl Receive for Modem {
    async fn recv(&mut self) -> Result<Option<Message>> {
        if !self.connected {
            return Err(Error::Closed);
        }
        if self.ended {
            return Ok(None);
        }
        loop {
            let next = self.sdk.lock().expect("the SDK").inbox.pop_front();
            if let Some(next) = next {
                self.ended = next.is_err();
                return next.map(Some);
            }
            self.arrived.notified().await;
        }
    }
}

/// A satellite messenger: it sends and never receives, so it implements
/// `Transport` alone and goes on a ladder as an uplink.
struct Satellite {
    sent: Arc<Mutex<Vec<Message>>>,
}

impl Transport for Satellite {
    async fn connect(&mut self) -> Result<()> {
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        self.sent
            .lock()
            .expect("the messenger")
            .push(Message::new(topic, payload));
        Ok(())
    }

    async fn subscribe(&mut self, _topic: &str) -> Result<()> {
        Err(Error::Transport(
            "a satellite messenger only sends".to_owned(),
        ))
    }
}
```
<!-- end -->

<!-- snippet: examples/guides/link.rs#example -->
From [`examples/guides/link.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/link.rs):

```rust
use pamoja_ladder::TransportLadder;
use pamoja_sync::MemoryStore;

// The SDK's state is shared with the example, which plays the vendor's side: it
// hands the modem a command, takes its signal away, and drops its session.
let sdk = Arc::new(Mutex::new(Sdk::default()));
let arrived = Arc::new(Notify::new());
let beamed = Arc::new(Mutex::new(Vec::new()));
let hand_over = |next: Result<Message>| {
    sdk.lock().expect("the SDK").inbox.push_back(next);
    arrived.notify_one();
};
let modem = Modem {
    sdk: Arc::clone(&sdk),
    arrived: Arc::clone(&arrived),
    connected: false,
    ended: false,
};
let satellite = Satellite {
    sent: Arc::clone(&beamed),
};

// The modem is the cheaper link and goes first. The messenger only sends, so it
// goes on as an uplink, which a ladder never listens on or subscribes.
let mut ladder = TransportLadder::new(MemoryStore::new())
    .rung(modem)
    .uplink(satellite);
ladder.connect().await?;

// A reading goes out over the first link that takes it.
ladder.send_text("waves/height", "1.8").await?;
let carried = sdk.lock().expect("the SDK").sent[0].clone();
println!("modem     carried {} {}", carried.topic, carried.text()?);

// A subscription reaches every link that listens, and only those: the messenger,
// whose subscribe would refuse, is never asked.
ladder.subscribe("commands/#").await?;
let filter = sdk.lock().expect("the SDK").filters[0].clone();
println!("modem     listens on {filter}, and the satellite was never asked");

// What the modem hands over comes back through the ladder.
hand_over(Ok(Message::new("commands/interval", b"600")));
let command = ladder.recv().await?.expect("a command");
println!("buoy      took {} {}", command.topic, command.text()?);

// A link that refuses a send passes the reading down to the next link.
sdk.lock().expect("the SDK").no_signal = true;
ladder.send_text("waves/height", "2.4").await?;
let relayed = beamed.lock().expect("the messenger")[0].clone();
println!(
    "satellite carried {} {} while the modem had no signal",
    relayed.topic,
    relayed.text()?
);

// A link that fails while listening says why, once, and has ended after that.
// With no other link listening, the ladder then has nothing to wait on.
hand_over(Err(Error::Transport(
    "the modem lost its session".to_owned(),
)));
let lost = ladder.recv().await.expect_err("the session was lost");
println!("buoy      lost the modem: {lost}");
let idle = ladder.recv().await.expect_err("no link listens");
println!("buoy      has no link left to listen on: {idle}");

// Connecting again brings the modem back, and the ladder places its filter on it
// again, so a link's subscribe runs once for every connect.
ladder.connect().await?;
let filters = sdk.lock().expect("the SDK").filters.clone();
println!(
    "modem     reconnected, and the ladder placed {} on it again",
    filters[1]
);
hand_over(Ok(Message::new("commands/interval", b"900")));
let later = ladder.recv().await?.expect("a command");
println!("buoy      took {} {}", later.topic, later.text()?);
```
<!-- end -->

## TypeScript

In TypeScript, a link is an object with `connect()`, `send(topic, payload)`, and
`subscribe(topic)`, and for a link that delivers, `recv()`; the `TransportHandlers` interface in
`@pamoja/core` spells the shape out. Each method may return a promise or nothing, and each is
called on the object, so a class works as it is. `send` is given the payload as a `Buffer`.
`recv` resolves with `{ topic, payload }`, the payload a `Buffer` or a string, or with `null` once
the link has ended. pamoja calls it again as soon as it settles, from the moment the link
connects, and queues what it delivers. A method that throws or rejects fails the call that
reached it, with its message after `transport error:`, and a `recv` that throws ends the link
until the next connect. `Transport.fromHandlers(object)` makes the transport.

<!-- snippet: bindings/node/guides/link.ts#example -->
From [`bindings/node/guides/link.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/link.ts):

```typescript
import { Transport, type DeliveredMessage, type TransportHandlers } from '@pamoja/core'
import { Ladder } from '@pamoja/ladder'
import { Store } from '@pamoja/sync'

// A cellular modem reached through its vendor's SDK, which this class stands in for.
// Nothing in it names a broker or a protocol: it needs only the operations the
// contract asks for, and `recv` is what makes it a link that delivers.
class Modem implements TransportHandlers {
  sent: { topic: string; text: string }[] = []
  filters: string[] = []
  noSignal = false
  private inbox: (DeliveredMessage | Error)[] = []
  private waiting: ((next: DeliveredMessage | Error) => void)[] = []

  connect(): void {}

  send(topic: string, payload: Buffer): void {
    if (this.noSignal) throw new Error('no signal')
    this.sent.push({ topic, text: payload.toString() })
  }

  subscribe(topic: string): void {
    this.filters.push(topic)
  }

  async recv(): Promise<DeliveredMessage> {
    const next =
      this.inbox.shift() ?? (await new Promise<DeliveredMessage | Error>((hand) => this.waiting.push(hand)))
    if (next instanceof Error) throw next
    return next
  }

  // The vendor's side: a message from the network, or the loss of the session,
  // handed to the wait that is open or kept for the next one.
  handOver(next: DeliveredMessage | Error): void {
    const waiter = this.waiting.shift()
    if (waiter) waiter(next)
    else this.inbox.push(next)
  }
}

// A satellite messenger sends and never receives. With no `recv` it is an uplink,
// which a ladder never listens on or subscribes.
class Satellite implements TransportHandlers {
  sent: { topic: string; text: string }[] = []

  connect(): void {}

  send(topic: string, payload: Buffer): void {
    this.sent.push({ topic, text: payload.toString() })
  }

  subscribe(): void {
    throw new Error('a satellite messenger only sends')
  }
}

async function main() {
  // The modem is the cheaper link and goes first. The messenger has no `recv`, so it
  // goes on as an uplink.
  const modem = new Modem()
  const satellite = new Satellite()
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.fromHandlers(modem))
  await ladder.rung(Transport.fromHandlers(satellite))
  await ladder.connect()

  // A reading goes out over the first link that takes it.
  await ladder.send('waves/height', '1.8')
  const carried = modem.sent[0]
  console.log(`modem     carried ${carried.topic} ${carried.text}`)

  // A subscription reaches every link that listens, and only those: the messenger,
  // whose subscribe would throw, is never asked.
  await ladder.subscribe('commands/#')
  const filter = modem.filters[0]
  console.log(`modem     listens on ${filter}, and the satellite was never asked`)

  // What the modem hands over comes back through the ladder.
  modem.handOver({ topic: 'commands/interval', payload: '600' })
  const command = (await ladder.recv())!
  console.log(`buoy      took ${command.topic} ${command.text!}`)

  // A link that refuses a send passes the reading down to the next link.
  modem.noSignal = true
  await ladder.send('waves/height', '2.4')
  const relayed = satellite.sent[0]
  console.log(`satellite carried ${relayed.topic} ${relayed.text} while the modem had no signal`)

  // A link that fails while listening says why, once, and has ended after that. With
  // no other link listening, the ladder then has nothing to wait on.
  modem.handOver(new Error('the modem lost its session'))
  let lost = ''
  try {
    await ladder.recv()
  } catch (error) {
    lost = (error as Error).message
  }
  console.log(`buoy      lost the modem: ${lost}`)
  let idle = ''
  try {
    await ladder.recv()
  } catch (error) {
    idle = (error as Error).message
  }
  console.log(`buoy      has no link left to listen on: ${idle}`)

  // Connecting again brings the modem back, and the ladder places its filter on it
  // again, so a link's subscribe runs once for every connect.
  await ladder.connect()
  console.log(`modem     reconnected, and the ladder placed ${modem.filters[1]} on it again`)
  modem.handOver({ topic: 'commands/interval', payload: '900' })
  const later = (await ladder.recv())!
  console.log(`buoy      took ${later.topic} ${later.text!}`)

  return { carried, filters: modem.filters, command, relayed, lost, idle, later }
}

main()
```
<!-- end -->

## Python

In Python, a link is an object with `connect()`, `send(topic, payload)`, `subscribe(topic)`, and
for one that delivers, `recv()`; `TransportHandlers` and `ReceivingTransportHandlers` in
`pamoja.core` give the shape to a type checker. Each may be a coroutine function or a plain one,
and each runs on the event loop the ladder was driven from. `send` is given the payload as
`bytes`. `recv` returns a `Message`, a `(topic, payload)` pair with the payload as text or bytes,
or `None` once the link has ended. An exception fails the call that reached it, with the
exception's message after `transport error:`, and one raised by `recv` ends the link until the
next connect. `Transport.from_handlers(obj)` makes the transport.

<!-- snippet: bindings/python/guides/link.py#example -->
From [`bindings/python/guides/link.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/link.py):

```python
import asyncio
from typing import Union

from pamoja.core import Message, PamojaError, Transport
from pamoja.ladder import Ladder
from pamoja.sync import Store


class Modem:
    """A cellular modem reached through its vendor's SDK, which this class stands in for.

    Nothing in it names a broker or a protocol: it needs only the operations the
    contract asks for, and ``recv`` is what makes it a link that delivers.
    """

    def __init__(self) -> None:
        self.sent: list[Message] = []
        self.filters: list[str] = []
        self.no_signal = False
        self.inbox: asyncio.Queue[Union[Message, Exception]] = asyncio.Queue()

    def connect(self) -> None:
        pass

    def send(self, topic: str, payload: bytes) -> None:
        if self.no_signal:
            raise ConnectionError("no signal")
        self.sent.append(Message(topic, payload))

    def subscribe(self, topic: str) -> None:
        self.filters.append(topic)

    async def recv(self) -> Message:
        arrived = await self.inbox.get()
        if isinstance(arrived, Exception):
            raise arrived
        return arrived

    def hand_over(self, arrived: Union[Message, Exception]) -> None:
        """The vendor's side: a message from the network, or the loss of the session."""
        self.inbox.put_nowait(arrived)


class Satellite:
    """A satellite messenger sends and never receives.

    With no ``recv`` it is an uplink, which a ladder never listens on or subscribes.
    """

    def __init__(self) -> None:
        self.sent: list[Message] = []

    def connect(self) -> None:
        pass

    def send(self, topic: str, payload: bytes) -> None:
        self.sent.append(Message(topic, payload))

    def subscribe(self, topic: str) -> None:
        raise ConnectionError("a satellite messenger only sends")


async def main() -> None:
    # The modem is the cheaper link and goes first. The messenger has no `recv`, so it
    # goes on as an uplink.
    modem = Modem()
    satellite = Satellite()
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.from_handlers(modem))
    await ladder.rung(Transport.from_handlers(satellite))
    await ladder.connect()

    # A reading goes out over the first link that takes it.
    await ladder.send("waves/height", "1.8")
    carried = modem.sent[0]
    print(f"modem     carried {carried.topic} {carried.text}")

    # A subscription reaches every link that listens, and only those: the messenger,
    # whose subscribe would raise, is never asked.
    await ladder.subscribe("commands/#")
    placed = modem.filters[0]
    print(f"modem     listens on {placed}, and the satellite was never asked")

    # What the modem hands over comes back through the ladder.
    modem.hand_over(Message("commands/interval", "600"))
    command = await ladder.recv()
    print(f"buoy      took {command.topic} {command.text}")

    # A link that refuses a send passes the reading down to the next link.
    modem.no_signal = True
    await ladder.send("waves/height", "2.4")
    relayed = satellite.sent[0]
    print(f"satellite carried {relayed.topic} {relayed.text} while the modem had no signal")

    # A link that fails while listening says why, once, and has ended after that. With
    # no other link listening, the ladder then has nothing to wait on.
    modem.hand_over(ConnectionResetError("the modem lost its session"))
    try:
        await ladder.recv()
    except PamojaError as error:
        lost = str(error)
    print(f"buoy      lost the modem: {lost}")
    try:
        await ladder.recv()
    except PamojaError as error:
        idle = str(error)
    print(f"buoy      has no link left to listen on: {idle}")

    # Connecting again brings the modem back, and the ladder places its filter on it
    # again, so a link's subscribe runs once for every connect.
    await ladder.connect()
    print(f"modem     reconnected, and the ladder placed {modem.filters[1]} on it again")
    modem.hand_over(Message("commands/interval", "900"))
    later = await ladder.recv()
    print(f"buoy      took {later.topic} {later.text}")

    return carried, modem.filters, command, relayed, lost, idle, later


seen = asyncio.run(main())
```
<!-- end -->

## C#

In C#, a link implements `ITransportHandlers`: `ConnectAsync`, `SendAsync(topic, payload)`, and
`SubscribeAsync(topic)`, each returning a `Task`. A link that delivers implements
`IReceivingTransportHandlers`, whose `ReceiveAsync` returns a `TransportMessage`, or `null` once
the link has ended; `new TransportMessage(topic, text)` makes one from text. pamoja calls the
first three one at a time on a thread of its own, and `ReceiveAsync` on another thread beside
them, so state the receive shares with the rest must be safe to use from two threads at once, as
a `Channel` is. An exception, thrown or carried by the task, fails the call that reached it with
the exception's message after `transport error:`, and one from `ReceiveAsync` ends the link
until the next connect. `Transport.FromHandlers(link)` makes the transport.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs#parts -->
From [`bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs):

```csharp
/// <summary>
/// A cellular modem reached through its vendor's SDK, which this class stands in
/// for. Nothing in it names a broker or a protocol: it needs only the operations
/// the contract asks for, and <see cref="ReceiveAsync"/> is what makes it a link
/// that delivers.
/// </summary>
private sealed class Modem : IReceivingTransportHandlers
{
    private readonly Channel<object> _inbox = Channel.CreateUnbounded<object>();

    public List<TransportMessage> Sent { get; } = new();

    public List<string> Filters { get; } = new();

    public bool NoSignal { get; set; }

    public Task ConnectAsync() => Task.CompletedTask;

    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        if (NoSignal)
        {
            return Task.FromException(new IOException("no signal"));
        }

        Sent.Add(new TransportMessage(topic, payload.ToArray()));
        return Task.CompletedTask;
    }

    public Task SubscribeAsync(string topic)
    {
        Filters.Add(topic);
        return Task.CompletedTask;
    }

    public async Task<TransportMessage?> ReceiveAsync() =>
        await _inbox.Reader.ReadAsync() switch
        {
            Exception lost => throw lost,
            var arrived => (TransportMessage)arrived,
        };

    /// <summary>
    /// The vendor's side: a message from the network, or the loss of the session.
    /// </summary>
    /// <param name="arrived">What the SDK hands over.</param>
    public void HandOver(object arrived) => _inbox.Writer.TryWrite(arrived);
}

/// <summary>
/// A satellite messenger sends and never receives. Without
/// <see cref="IReceivingTransportHandlers"/> it is an uplink, which a ladder never
/// listens on or subscribes.
/// </summary>
private sealed class Satellite : ITransportHandlers
{
    public List<TransportMessage> Sent { get; } = new();

    public Task ConnectAsync() => Task.CompletedTask;

    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        Sent.Add(new TransportMessage(topic, payload.ToArray()));
        return Task.CompletedTask;
    }

    public Task SubscribeAsync(string topic) =>
        Task.FromException(new NotSupportedException("a satellite messenger only sends"));
}
```
<!-- end -->

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs):

```csharp
// The modem is the cheaper link and goes first. The messenger has no
// ReceiveAsync, so it goes on as an uplink.
var modem = new Modem();
var satellite = new Satellite();
using var ladder = new Ladder(Store.Memory());
ladder.Rung(Transport.FromHandlers(modem));
ladder.Rung(Transport.FromHandlers(satellite));
await ladder.ConnectAsync();

// A reading goes out over the first link that takes it.
await ladder.SendAsync("waves/height", "1.8");
TransportMessage carried = modem.Sent[0];
Console.WriteLine($"modem     carried {carried.Topic} {carried.Text}");

// A subscription reaches every link that listens, and only those: the
// messenger, whose subscribe would fail, is never asked.
await ladder.SubscribeAsync("commands/#");
string placed = modem.Filters[0];
Console.WriteLine($"modem     listens on {placed}, and the satellite was never asked");

// What the modem hands over comes back through the ladder.
modem.HandOver(new TransportMessage("commands/interval", "600"));
TransportMessage command = (await ladder.ReceiveAsync())!;
Console.WriteLine($"buoy      took {command.Topic} {command.Text}");

// A link that refuses a send passes the reading down to the next link.
modem.NoSignal = true;
await ladder.SendAsync("waves/height", "2.4");
TransportMessage relayed = satellite.Sent[0];
Console.WriteLine(
    $"satellite carried {relayed.Topic} {relayed.Text} while the modem had no signal");

// A link that fails while listening says why, once, and has ended after that.
// With no other link listening, the ladder then has nothing to wait on.
modem.HandOver(new IOException("the modem lost its session"));
string lost = string.Empty;
try
{
    await ladder.ReceiveAsync();
}
catch (PamojaException error)
{
    lost = error.Message;
}

Console.WriteLine($"buoy      lost the modem: {lost}");
string idle = string.Empty;
try
{
    await ladder.ReceiveAsync();
}
catch (PamojaException error)
{
    idle = error.Message;
}

Console.WriteLine($"buoy      has no link left to listen on: {idle}");

// Connecting again brings the modem back, and the ladder places its filter on
// it again, so a link's subscribe runs once for every connect.
await ladder.ConnectAsync();
Console.WriteLine(
    $"modem     reconnected, and the ladder placed {modem.Filters[1]} on it again");
modem.HandOver(new TransportMessage("commands/interval", "900"));
TransportMessage later = (await ladder.ReceiveAsync())!;
Console.WriteLine($"buoy      took {later.Topic} {later.Text}");
```
<!-- end -->

## Values at a glance

**The shape of a link in each language:**

| Language | A link | A link that delivers | The transport |
| --- | --- | --- | --- |
| Rust | `impl Transport` | `impl Transport + Receive` | the value itself |
| TypeScript | `TransportHandlers` | the same, with `recv` | `Transport.fromHandlers(obj)` |
| Python | `TransportHandlers` | `ReceivingTransportHandlers` | `Transport.from_handlers(obj)` |
| C# | `ITransportHandlers` | `IReceivingTransportHandlers` | `Transport.FromHandlers(obj)` |

**The methods,** each an `async fn` on `&mut self` in Rust, and each free to be plain or
asynchronous in TypeScript and Python:

| What | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| connect | `connect()` | `connect()` | `connect()` | `ConnectAsync()` |
| send | `send(topic, &[u8])` | `send(topic, Buffer)` | `send(topic, bytes)` | `SendAsync(topic, ReadOnlyMemory<byte>)` |
| subscribe | `subscribe(filter)` | `subscribe(filter)` | `subscribe(filter)` | `SubscribeAsync(filter)` |
| receive | `recv()` | `recv()` | `recv()` | `ReceiveAsync()` |
| a message | `Message::new(topic, payload)` | `{ topic, payload }` | `Message(topic, payload)` or `(topic, payload)` | `new TransportMessage(topic, payload)` |
| the link has ended | `Ok(None)` | `null` | `None` | `null` |
| a failure | `Err(Error::Transport(..))` | throw, or reject | raise | throw, or fault the task |

A message's payload may be text as well as bytes in TypeScript, Python, and C#.

**What a ladder does with what a link does:**

| The link | The ladder |
| --- | --- |
| fails to connect | leaves it down, and tries it again on the next `connect` |
| refuses a send with an error | passes the message to the next link, or to its store |
| answers a send with `Closed` | treats it as down until the next `connect`, and passes the message on |
| has no receive | never subscribes it or listens on it |
| delivers a message | hands it to the next receive |
| reports that it has ended | stops listening on it until the next `connect` |
| fails a receive | returns the failure from its own receive, and listens to the link again on the next |
| connects again | places every filter it holds on it again |

## When it goes wrong

What the bindings say about a link:

| What happened | The message | What to check |
| --- | --- | --- |
| a method threw, rejected, or raised | `transport error:` followed by the method's own message | the vendor's error, which the message carries |
| a handler object without a required method | `a transport handler needs a send method` in TypeScript and Python | connect, send, and subscribe are all required; C# checks at compile time |
| a call before `connect` | `resource is closed` | connect first |
| a TypeScript `recv` gave something other than a message | `transport error: Value is none of these types` and where | return `{ topic, payload }`, the payload a `Buffer` or a string |
| a Python `recv` returned something else | `transport error: recv must return a Message, a (topic, payload) pair, or None` | the value `recv` returns |
| a ladder whose only listening link ended or failed | `resource is closed` | connect the ladder again |
| a transport used after it went on a ladder | `this transport was already added to a ladder or a wrapper` | build another with the same handlers |

The mistakes that cost an afternoon:

- **A link that is only quiet reads as ended.** A receive that returns `null`, `None`, or
  `Ok(None)` tells pamoja the link has ended, and nothing asks it again until the next connect. A
  receive with nothing yet waits for something, as the modem's does.
- **A link goes silent after one bad frame.** A receive that fails ends the link until the next
  connect. A link that should shrug off a corrupt frame skips it inside `recv` and waits for the
  next one, and fails only when the link itself has failed.
- **Subscriptions pile up in the vendor's SDK.** A ladder places its filters again every time the
  link connects. A link whose SDK keeps subscriptions across reconnects treats a filter it already
  holds as done.
- **A C# link's receive corrupts shared state.** `ReceiveAsync` runs beside the other methods, on
  its own thread. Share state with it through something built for that, a `Channel` or a lock, as
  the example's modem does.
- **Memory climbs on a link nobody reads.** What a binding's `recv` delivers waits in a queue with
  no limit until something receives it. A link that delivers belongs on a ladder or a transport
  that something is reading.
- **In Rust, a message vanishes between the ladder's waits.** A ladder waits on every listening
  link at once and drops the waits that lose. A `recv` that takes a message and then awaits
  anything before returning loses the message when its wait is dropped. Take the message and
  return it in the same step, as the modem does.
- **An uplink is subscribed after all.** A ladder never subscribes an uplink, but code that drives
  one directly can. Have its subscribe refuse, as the satellite's does, so the mistake is loud.

## Where next

<!-- table: next link -->
- [Engine surface](transport.md): The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version.
- [Transport ladder](ladder.md): Cheapest reachable link first, buffering to a store when every link is down.
- [Secured session](session.md): X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack.
- Also in Transports and testing: [MQTT](mqtt.md), [CoAP](coap.md), [Loopback](loopback.md), [Store and forward](sync.md), [Event bus](bus.md), [Simulators](sim.md).
<!-- end -->

## Reference

<!-- table: reference link -->
- Rust: the `Transport` and `Receive` traits in [`pamoja-core`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-link)
- TypeScript: [`@pamoja/core`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_core.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-link)
- Python: [`pamoja.core`](https://pamoja.molex.cloud/docs/reference/python/pamoja/core.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-link)
- C#: [`Pamoja.Core`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-link)
<!-- end -->
